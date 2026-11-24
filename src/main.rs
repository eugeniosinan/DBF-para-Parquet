use std::path::{Path, PathBuf};

use dbf_para_parquet::archive::{ArchiveExtractionResult, ArchiveManager};
use dbf_para_parquet::dbf::ConversionMode;
use dbf_para_parquet::detector::DbfDetector;
use dbf_para_parquet::parquet_writer::ParquetConverter;
use dbf_para_parquet::ui::DbfConverterApp;
use eframe::egui::Vec2;

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();

    // Modo CLI se passado "--cli" ou "--inspect"
    if args.len() > 1 && (args[1] == "--cli" || args[1] == "--inspect") {
        let inspect_only = args[1] == "--inspect";
        run_cli(&args[2..], inspect_only);
        return Ok(());
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

fn run_cli(args: &[String], inspect_only: bool) {
    if args.is_empty() {
        println!("Uso CLI:");
        println!("  dbf-para-parquet --inspect <arquivo.dbf|dbc|zip|rar|7z>");
        println!("  dbf-para-parquet --cli <arquivo.dbf|dbc|zip|rar|7z> [saida.parquet]");
        return;
    }

    let input_path = PathBuf::from(&args[0]);
    if !input_path.exists() {
        eprintln!("Arquivo não encontrado: {:?}", input_path);
        return;
    }

    let parent_dir = input_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_string_lossy()
        .to_string();

    let mut dbfs_to_process = Vec::new();
    let mut _temp_dirs = Vec::new();

    if ArchiveManager::is_archive(&input_path) {
        println!("Examinando arquivo compactado: {:?}", input_path);
        match ArchiveManager::process_archive(&input_path) {
            ArchiveExtractionResult::Success {
                archive_name,
                dbf_files,
                _temp_dir,
            } => {
                println!(
                    "Extração concluída: {} arquivo(s) DBF encontrado(s) em {}",
                    dbf_files.len(),
                    archive_name
                );
                for dbf in dbf_files {
                    dbfs_to_process.push((dbf, Some(archive_name.clone()), parent_dir.clone(), true));
                }
                _temp_dirs.push(_temp_dir);
            }
            ArchiveExtractionResult::NoDbfFound {
                archive_name,
                internal_files_sample,
            } => {
                eprintln!(
                    "⚠️ ALERTA: O arquivo compactado '{}' NÃO contém nenhum arquivo .dbf interno!",
                    archive_name
                );
                eprintln!("Arquivos encontrados dentro: {:?}", internal_files_sample);
                return;
            }
            ArchiveExtractionResult::Error {
                archive_name,
                message,
            } => {
                eprintln!("Erro ao descompactar {}: {}", archive_name, message);
                return;
            }
        }
    } else {
        dbfs_to_process.push((input_path.clone(), None, parent_dir, false));
    }

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
                dbf_para_parquet::dbf::DbfEncoding::Iso8859_1
            }
        };

        if inspect_only {
            continue;
        }

        let output_path = if args.len() > 1 && dbfs_to_process.len() == 1 {
            PathBuf::from(&args[1])
        } else {
            let stem = dbf_path.file_stem().unwrap_or_default().to_string_lossy();
            let parent = PathBuf::from(original_dir);
            parent.join(format!("{}.parquet", stem))
        };

        println!("\nIniciando conversão para: {:?}", output_path);
        match ParquetConverter::convert_single(
            dbf_path,
            &output_path,
            ConversionMode::EpidemiologicalDates,
            standard_encoding,
            None,
        ) {
            Ok(summary) => {
                println!(
                    "✅ Sucesso! {} registros gravados em {:?} ({:.2} MB em {} ms)",
                    summary.records,
                    summary.output_path,
                    summary.file_size_bytes as f64 / (1024.0 * 1024.0),
                    summary.duration_ms
                );
                if *is_temp {
                    let _ = std::fs::remove_file(dbf_path);
                }
            }
            Err(e) => {
                eprintln!("❌ Erro na conversão: {}", e);
                if *is_temp {
                    let _ = std::fs::remove_file(dbf_path);
                }
            }
        }
    }
}
