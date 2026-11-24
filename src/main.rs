use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use dbf_para_parquet::archive::{ArchiveExtractionResult, ArchiveManager};
use dbf_para_parquet::dbf::{ConversionMode, DbfEncoding};
use dbf_para_parquet::detector::DbfDetector;
use dbf_para_parquet::parquet_writer::ParquetConverter;
use dbf_para_parquet::ui::DbfConverterApp;
use eframe::egui::Vec2;

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();

    // Modo CLI se passado "--cli", "--inspect", "-h", "--help"
    if args.len() > 1 {
        let first = args[1].as_str();
        if first == "--cli" || first == "--inspect" || first == "-h" || first == "--help" || first == "help" {
            let inspect_only = first == "--inspect";
            let cli_args = if first == "--cli" || first == "--inspect" {
                &args[2..]
            } else {
                &args[1..]
            };
            run_cli(cli_args, inspect_only);
            return Ok(());
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(1080.0, 750.0))
            .with_min_inner_size(Vec2::new(850.0, 580.0))
            .with_title("Conversor de Bases da Vigilância para Parquet — Eugênio Araújo (CIE / SMS Rio)"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Conversor de Bases da Vigilância para Parquet — Eugênio Araújo (CIE / SMS Rio)",
        native_options,
        Box::new(|cc| Ok(Box::new(DbfConverterApp::new(cc)))),
    )
}

fn print_help() {
    println!("Conversor de Bases da Vigilância em Saúde — DBF/DBC para Parquet");
    println!("Desenvolvido por Eugênio Araújo — Centro de Inteligência Epidemiológica (CIE / SMS Rio)\n");
    println!("USO:");
    println!("  dbf-para-parquet [OPÇÕES]");
    println!("  dbf-para-parquet --inspect <arquivo|pasta...> [OPÇÕES]");
    println!("  dbf-para-parquet --cli <arquivo|pasta...> [OPÇÕES]\n");
    println!("MODOS DE EXECUÇÃO:");
    println!("  (sem argumentos)               Inicia a Interface Gráfica (GUI) interativa");
    println!("  --inspect <arquivos/pastas>    Inspeciona schemas, encoding e auditoria LGPD sem converter");
    println!("  --cli <arquivos/pastas>        Executa a conversão diretamente via terminal\n");
    println!("OPÇÕES DA LINHA DE COMANDO:");
    println!("  -o, --output <ARQUIVO>         Caminho do arquivo Parquet gerado (ex: saida.parquet)");
    println!("  -d, --out-dir <PASTA>          Diretório onde salvar os arquivos Parquet gerados");
    println!("  -m, --merge                    Consolida múltiplos DBFs/DBCs em um único arquivo Parquet");
    println!("  --strict                       Modo estrito: respeita tipos originais DBF (desativa Date32)");
    println!("  --encoding <ENCODING>          Força o encoding: auto, iso-8859-1, cp850, utf-8");
    println!("                                 (Padrão: auto - identifica conforme o agravo/sistema DATASUS)");
    println!("  -h, --help                     Exibe esta mensagem de ajuda\n");
    println!("FORMATOS SUPORTADOS:");
    println!("  .dbf, .dbc (DATASUS comprimido), .zip, .rar, .7z ou diretórios inteiros\n");
    println!("EXEMPLOS:");
    println!("  # Inspecionar base ou pacote compactado:");
    println!("  dbf-para-parquet --inspect DORJ2025.dbc\n");
    println!("  # Conversão simples de 1 arquivo:");
    println!("  dbf-para-parquet --cli DORJ2025.dbc -o DORJ2025.parquet\n");
    println!("  # Consolidar múltiplos anos em um único Parquet (Merge):");
    println!("  dbf-para-parquet --cli DOAC2021.dbc DOAC2022.dbc --merge -o DOAC_Consolidado.parquet\n");
    println!("  # Converter todos os arquivos de um diretório salvando em outra pasta:");
    println!("  dbf-para-parquet --cli /dados/sivep/ --out-dir /dados/parquet/\n");
    println!("  # Forçar encoding CP850 e modo estrito de tipos DBF:");
    println!("  dbf-para-parquet --cli entrada.dbf --encoding cp850 --strict -o saida.parquet\n");
}

fn run_cli(args: &[String], inspect_only: bool) {
    if args.is_empty() {
        print_help();
        return;
    }

    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut output_file: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut merge = false;
    let mut strict = false;
    let mut custom_encoding: Option<DbfEncoding> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" | "help" => {
                print_help();
                return;
            }
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(PathBuf::from(&args[i]));
                }
            }
            "-d" | "--out-dir" => {
                i += 1;
                if i < args.len() {
                    out_dir = Some(PathBuf::from(&args[i]));
                }
            }
            "-m" | "--merge" => {
                merge = true;
            }
            "--strict" => {
                strict = true;
            }
            "--encoding" => {
                i += 1;
                if i < args.len() {
                    match args[i].to_lowercase().as_str() {
                        "iso-8859-1" | "iso8859-1" | "latin1" | "windows-1252" => {
                            custom_encoding = Some(DbfEncoding::Iso8859_1);
                        }
                        "cp850" | "ibm850" => {
                            custom_encoding = Some(DbfEncoding::Cp850);
                        }
                        "utf-8" | "utf8" => {
                            custom_encoding = Some(DbfEncoding::Utf8);
                        }
                        "auto" => {
                            custom_encoding = None;
                        }
                        other => {
                            eprintln!("⚠️ Encoding desconhecido '{}'. Usando detecção automática padronizada.", other);
                        }
                    }
                }
            }
            arg => {
                let p = PathBuf::from(arg);
                // Compatibilidade com sintaxe antiga: `--cli entrada.dbf saida.parquet`
                let is_parquet_dest = p.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()) == Some("parquet".to_string())
                    && !p.exists()
                    && output_file.is_none()
                    && !inputs.is_empty();

                if is_parquet_dest {
                    output_file = Some(p);
                } else {
                    inputs.push(p);
                }
            }
        }
        i += 1;
    }

    if inputs.is_empty() {
        eprintln!("❌ Erro: Nenhum arquivo ou diretório de entrada especificado.");
        println!();
        print_help();
        return;
    }

    // Expande diretórios e arquivos de entrada
    let mut files_to_scan = Vec::new();
    for input_path in &inputs {
        if !input_path.exists() {
            eprintln!("⚠️ Caminho de entrada não encontrado: {:?}", input_path);
            continue;
        }

        if input_path.is_dir() {
            for entry in WalkDir::new(input_path).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() {
                    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                    if matches!(ext.as_str(), "dbf" | "dbc" | "zip" | "rar" | "7z") {
                        files_to_scan.push(p.to_path_buf());
                    }
                }
            }
        } else {
            files_to_scan.push(input_path.clone());
        }
    }

    if files_to_scan.is_empty() {
        eprintln!("❌ Nenhum arquivo DBF, DBC, ZIP, RAR ou 7Z encontrado para processamento.");
        return;
    }

    let mut dbfs_to_process = Vec::new();
    let mut temp_dirs = Vec::new();

    for file_path in files_to_scan {
        let parent_dir = file_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_string_lossy()
            .to_string();

        if ArchiveManager::is_archive(&file_path) {
            println!("Examinando arquivo/pacote: {:?}", file_path);
            match ArchiveManager::process_archive(&file_path) {
                ArchiveExtractionResult::Success {
                    archive_name,
                    dbf_files,
                    _temp_dir,
                } => {
                    println!(
                        "  → {} arquivo(s) DBF extraído(s) de {}",
                        dbf_files.len(),
                        archive_name
                    );
                    for dbf in dbf_files {
                        dbfs_to_process.push((dbf, Some(archive_name.clone()), parent_dir.clone(), true));
                    }
                    temp_dirs.push(_temp_dir);
                }
                ArchiveExtractionResult::NoDbfFound {
                    archive_name,
                    internal_files_sample,
                } => {
                    eprintln!(
                        "⚠️ ALERTA: O arquivo '{}' NÃO contém nenhum arquivo .dbf interno!",
                        archive_name
                    );
                    eprintln!("Arquivos encontrados dentro: {:?}", internal_files_sample);
                }
                ArchiveExtractionResult::Error {
                    archive_name,
                    message,
                } => {
                    eprintln!("Erro ao descompactar {}: {}", archive_name, message);
                }
            }
        } else {
            dbfs_to_process.push((file_path.clone(), None, parent_dir, false));
        }
    }

    if dbfs_to_process.is_empty() {
        eprintln!("❌ Nenhum arquivo DBF válido pronto para conversão.");
        return;
    }

    // Análise de cabeçalhos e schemas
    let mut analyzed_items = Vec::new();
    for (dbf_path, archive_name, original_dir, is_temp) in &dbfs_to_process {
        println!("\n--- Análise da Base: {:?} ---", dbf_path.file_name().unwrap_or_default());
        let analysis_res = DbfDetector::analyze_header(dbf_path, archive_name.clone(), original_dir.clone(), *is_temp);
        let standard_encoding = match &analysis_res {
            Ok(analysis) => {
                println!("• Base Identificada: {}", analysis.system.display_name());
                println!(
                    "• Correspondência com Schema Oficial: {:.1}% ({} de {} colunas)",
                    analysis.match_percentage, analysis.matched_columns_count, analysis.num_columns
                );
                println!(
                    "• Encoding Padronizado: {} ({})",
                    analysis.standard_encoding.display_name(),
                    analysis.encoding_rationale
                );
                println!("• Classificação: {}", analysis.privacy.display_name());
                println!("• Total de Registros: {}", analysis.num_records);
                println!("• Total de Colunas: {}", analysis.num_columns);
                if !analysis.key_columns_found.is_empty() {
                    println!("• Colunas de assinatura: {}", analysis.key_columns_found.join(", "));
                }
                if !analysis.personal_columns_found.is_empty() {
                    println!(
                        "• Dados pessoais do paciente detectados: {}",
                        analysis.personal_columns_found.join(", ")
                    );
                } else {
                    println!("• Dados pessoais: Nenhum campo nominal ou de endereço do paciente detectado (Base Anonimizada).");
                }
                if !analysis.warnings.is_empty() {
                    println!("• Avisos: {}", analysis.warnings.join("; "));
                }
                analysis.standard_encoding
            }
            Err(e) => {
                eprintln!("Erro ao analisar cabeçalho DBF: {}", e);
                DbfEncoding::Iso8859_1
            }
        };

        analyzed_items.push((dbf_path.clone(), archive_name.clone(), original_dir.clone(), *is_temp, standard_encoding));
    }

    if inspect_only {
        println!("\n🔍 Inspeção concluída para {} arquivo(s).", analyzed_items.len());
        return;
    }

    let mode = if strict {
        println!("\n⚙️ Modo de Conversão: DBF Estrito (respeitando tipos literais do cabeçalho)");
        ConversionMode::StrictDbfTypes
    } else {
        println!("\n⚙️ Modo de Conversão: Padrão Epidemiológico Saúde (datas convertidas para Date32)");
        ConversionMode::EpidemiologicalDates
    };

    // Caso 1: Modo Unificado (--merge)
    if merge {
        let input_dbf_paths: Vec<PathBuf> = analyzed_items.iter().map(|(p, ..)| p.clone()).collect();
        let first_encoding = custom_encoding.unwrap_or(analyzed_items[0].4);

        let target_output = if let Some(ref out) = output_file {
            out.clone()
        } else if let Some(ref dir) = out_dir {
            dir.join("consolidado.parquet")
        } else {
            PathBuf::from("consolidado.parquet")
        };

        println!("\n📦 [MODO UNIFICADO] Consolidando {} arquivo(s) em: {:?}", input_dbf_paths.len(), target_output);
        println!("• Encoding aplicado: {}", first_encoding.display_name());

        match ParquetConverter::convert_merged(
            &input_dbf_paths,
            &target_output,
            mode,
            first_encoding,
            None,
        ) {
            Ok(summary) => {
                println!(
                    "\n✅ Sucesso Unificado! {} registros consolidados em {:?} ({:.2} MB em {} ms)",
                    summary.records,
                    summary.output_path,
                    summary.file_size_bytes as f64 / (1024.0 * 1024.0),
                    summary.duration_ms
                );
            }
            Err(e) => {
                eprintln!("\n❌ Erro na unificação: {}", e);
            }
        }
        return;
    }

    // Caso 2: Conversão Individual
    println!("\n📄 [MODO INDIVIDUAL] Convertendo {} arquivo(s)...", analyzed_items.len());
    let mut success_count = 0;
    let total_count = analyzed_items.len();

    for (dbf_path, archive_name, original_dir, _, standard_encoding) in analyzed_items {
        let encoding_to_use = custom_encoding.unwrap_or(standard_encoding);

        let target_output = if total_count == 1 && output_file.is_some() {
            output_file.clone().unwrap()
        } else {
            let stem = if let Some(ref arch) = archive_name {
                Path::new(arch).file_stem().unwrap_or_default().to_string_lossy().to_string()
            } else {
                dbf_path.file_stem().unwrap_or_default().to_string_lossy().to_string()
            };

            let base_dir = if let Some(ref dir) = out_dir {
                dir.clone()
            } else {
                PathBuf::from(&original_dir)
            };

            base_dir.join(format!("{}.parquet", stem))
        };

        println!("\nConvertendo {:?} -> {:?}", dbf_path.file_name().unwrap_or_default(), target_output);
        match ParquetConverter::convert_single(
            &dbf_path,
            &target_output,
            mode,
            encoding_to_use,
            None,
        ) {
            Ok(summary) => {
                success_count += 1;
                println!(
                    "  ✅ {} registros gravados ({:.2} MB em {} ms)",
                    summary.records,
                    summary.file_size_bytes as f64 / (1024.0 * 1024.0),
                    summary.duration_ms
                );
            }
            Err(e) => {
                eprintln!("  ❌ Erro: {}", e);
            }
        }
    }

    println!("\nConcluído: {} de {} arquivo(s) convertidos com sucesso.", success_count, total_count);
}
