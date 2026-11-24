use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

#[derive(Debug)]
pub enum ArchiveExtractionResult {
    Success {
        archive_name: String,
        dbf_files: Vec<PathBuf>,
        _temp_dir: TempDir,
    },
    NoDbfFound {
        archive_name: String,
        internal_files_sample: Vec<String>,
    },
    Error {
        archive_name: String,
        message: String,
    },
}

pub struct ArchiveManager;

impl ArchiveManager {
    pub fn is_archive<P: AsRef<Path>>(path: P) -> bool {
        let ext = path
            .as_ref()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(ext.as_str(), "zip" | "7z" | "rar" | "dbc")
    }

    pub fn process_archive<P: AsRef<Path>>(archive_path: P) -> ArchiveExtractionResult {
        let path = archive_path.as_ref();
        let archive_name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "zip" => Self::process_zip(path, &archive_name),
            "7z" => Self::process_7z(path, &archive_name),
            "rar" => Self::process_rar(path, &archive_name),
            "dbc" => Self::process_dbc(path, &archive_name),
            _ => ArchiveExtractionResult::Error {
                archive_name,
                message: format!("Formato de arquivo compactado não suportado: .{}", ext),
            },
        }
    }

    fn process_zip(path: &Path, archive_name: &str) -> ArchiveExtractionResult {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                return ArchiveExtractionResult::Error {
                    archive_name: archive_name.to_string(),
                    message: format!("Falha ao abrir arquivo zip: {}", e),
                }
            }
        };

        let mut archive = match zip::ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                return ArchiveExtractionResult::Error {
                    archive_name: archive_name.to_string(),
                    message: format!("Falha ao interpretar arquivo zip: {}", e),
                }
            }
        };

        let mut dbf_indices = Vec::new();
        let mut internal_files = Vec::new();

        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = match file.name() {
                    Ok(n) => n.to_string(),
                    Err(_) => continue,
                };
                let lower = name.to_lowercase();
                if lower.ends_with(".dbf") {
                    dbf_indices.push(i);
                }
                if internal_files.len() < 10 {
                    internal_files.push(name);
                }
            }
        }

        if dbf_indices.is_empty() {
            return ArchiveExtractionResult::NoDbfFound {
                archive_name: archive_name.to_string(),
                internal_files_sample: internal_files,
            };
        }

        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => {
                return ArchiveExtractionResult::Error {
                    archive_name: archive_name.to_string(),
                    message: format!("Erro ao criar diretório temporário para extração: {}", e),
                }
            }
        };

        let mut extracted_dbfs = Vec::new();

        for idx in dbf_indices {
            let mut z_file = match archive.by_index(idx) {
                Ok(f) => f,
                Err(e) => {
                    return ArchiveExtractionResult::Error {
                        archive_name: archive_name.to_string(),
                        message: format!("Erro ao ler entrada no zip: {}", e),
                    }
                }
            };

            let outpath = match z_file.enclosed_name() {
                Some(p) => {
                    let fname = p.file_name().unwrap_or(p.as_os_str());
                    temp_dir.path().join(fname)
                }
                None => continue,
            };

            let mut outfile = match File::create(&outpath) {
                Ok(f) => f,
                Err(e) => {
                    return ArchiveExtractionResult::Error {
                        archive_name: archive_name.to_string(),
                        message: format!("Erro ao criar arquivo descompactado {:?}: {}", outpath, e),
                    }
                }
            };

            if let Err(e) = std::io::copy(&mut z_file, &mut outfile) {
                return ArchiveExtractionResult::Error {
                    archive_name: archive_name.to_string(),
                    message: format!("Erro ao extrair arquivo {:?}: {}", outpath, e),
                };
            }

            extracted_dbfs.push(outpath);
        }

        ArchiveExtractionResult::Success {
            archive_name: archive_name.to_string(),
            dbf_files: extracted_dbfs,
            _temp_dir: temp_dir,
        }
    }

    fn process_7z(path: &Path, archive_name: &str) -> ArchiveExtractionResult {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => {
                return ArchiveExtractionResult::Error {
                    archive_name: archive_name.to_string(),
                    message: format!("Erro ao criar diretório temporário: {}", e),
                }
            }
        };

        match sevenz_rust2::decompress_file(path, temp_dir.path()) {
            Ok(_) => {
                let mut dbfs = Vec::new();
                let mut all_files = Vec::new();

                for entry in walkdir::WalkDir::new(temp_dir.path())
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        let p = entry.path();
                        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if name.to_lowercase().ends_with(".dbf") {
                            dbfs.push(p.to_path_buf());
                        }
                        if all_files.len() < 10 {
                            all_files.push(name);
                        }
                    }
                }

                if dbfs.is_empty() {
                    ArchiveExtractionResult::NoDbfFound {
                        archive_name: archive_name.to_string(),
                        internal_files_sample: all_files,
                    }
                } else {
                    ArchiveExtractionResult::Success {
                        archive_name: archive_name.to_string(),
                        dbf_files: dbfs,
                        _temp_dir: temp_dir,
                    }
                }
            }
            Err(e) => ArchiveExtractionResult::Error {
                archive_name: archive_name.to_string(),
                message: format!("Falha ao descompactar 7z: {}", e),
            },
        }
    }

    fn process_rar(path: &Path, archive_name: &str) -> ArchiveExtractionResult {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => {
                return ArchiveExtractionResult::Error {
                    archive_name: archive_name.to_string(),
                    message: format!("Erro ao criar diretório temporário: {}", e),
                }
            }
        };

        // Usa unrar ou 7z instalado no sistema
        let unrar_status = Command::new("unrar")
            .arg("x")
            .arg("-y")
            .arg(path)
            .arg(temp_dir.path())
            .output();

        let success = match unrar_status {
            Ok(out) if out.status.success() => true,
            _ => {
                // Tenta fallback com 7z
                match Command::new("7z")
                    .arg("x")
                    .arg(format!("-o{}", temp_dir.path().display()))
                    .arg("-y")
                    .arg(path)
                    .output()
                {
                    Ok(out) => out.status.success(),
                    Err(_) => false,
                }
            }
        };

        if !success {
            return ArchiveExtractionResult::Error {
                archive_name: archive_name.to_string(),
                message: "Não foi possível extrair o arquivo RAR (utilitário unrar/7z indisponível ou arquivo corrompido).".to_string(),
            };
        }

        let mut dbfs = Vec::new();
        let mut all_files = Vec::new();

        for entry in walkdir::WalkDir::new(temp_dir.path())
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let p = entry.path();
                let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                if name.to_lowercase().ends_with(".dbf") {
                    dbfs.push(p.to_path_buf());
                }
                if all_files.len() < 10 {
                    all_files.push(name);
                }
            }
        }

        if dbfs.is_empty() {
            ArchiveExtractionResult::NoDbfFound {
                archive_name: archive_name.to_string(),
                internal_files_sample: all_files,
            }
        } else {
            ArchiveExtractionResult::Success {
                archive_name: archive_name.to_string(),
                dbf_files: dbfs,
                _temp_dir: temp_dir,
            }
        }
    }

    fn process_dbc(path: &Path, archive_name: &str) -> ArchiveExtractionResult {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => {
                return ArchiveExtractionResult::Error {
                    archive_name: archive_name.to_string(),
                    message: format!("Erro ao criar diretório temporário para DBC: {}", e),
                }
            }
        };

        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "dados".to_string());

        let dbf_out = temp_dir.path().join(format!("{}.dbf", stem));

        match datasus_dbc::decompress(path, &dbf_out) {
            Ok(_) => ArchiveExtractionResult::Success {
                archive_name: archive_name.to_string(),
                dbf_files: vec![dbf_out],
                _temp_dir: temp_dir,
            },
            Err(e) => ArchiveExtractionResult::Error {
                archive_name: archive_name.to_string(),
                message: format!("Falha ao descompactar arquivo DBC do DATASUS: {:?}", e),
            },
        }
    }
}
