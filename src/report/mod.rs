use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::detector::DbfAnalysis;
use crate::parquet_writer::ConversionSummaryItem;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreProcessingReport {
    pub generated_at: String,
    pub total_inputs: usize,
    pub valid_dbfs: Vec<DbfAnalysis>,
    pub invalid_archives_without_dbf: Vec<String>,
    pub general_warnings: Vec<String>,
}

impl PreProcessingReport {
    pub fn new() -> Self {
        Self {
            generated_at: chrono::Local::now().format("%d/%m/%Y %H:%M:%S").to_string(),
            total_inputs: 0,
            valid_dbfs: Vec::new(),
            invalid_archives_without_dbf: Vec::new(),
            general_warnings: Vec::new(),
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Relatório Pré-Processamento — Análise das Bases DBF\n\n");
        md.push_str(&format!("**Data/Hora da Análise:** {}\n\n", self.generated_at));

        if !self.invalid_archives_without_dbf.is_empty() {
            md.push_str("### ⚠️ Arquivos Compactados Sem DBF (Serão Desconsiderados):\n");
            for item in &self.invalid_archives_without_dbf {
                md.push_str(&format!("- **{}** (nenhum arquivo .dbf encontrado internamente)\n", item));
            }
            md.push('\n');
        }

        if self.valid_dbfs.is_empty() {
            md.push_str("Nenhuma base DBF válida foi identificada para processamento.\n");
            return md;
        }

        md.push_str("### Bases DBF Identificadas:\n\n");
        md.push_str("| Arquivo / Origem | Sistema Identificado | Correspondência | Encoding Padronizado | Classificação LGPD | Registros | Tamanho |\n");
        md.push_str("|---|---|---|---|---|---|---|\n");

        for item in &self.valid_dbfs {
            let size_mb = (item.file_size_bytes as f64) / (1024.0 * 1024.0);
            let origin = if let Some(arch) = &item.source_archive {
                format!("{} (de {})", item.display_name, arch)
            } else {
                item.display_name.clone()
            };

            md.push_str(&format!(
                "| {} | {} | {:.1}% ({}/{}) | {} | {} | {} | {:.2} MB |\n",
                origin,
                item.system.code(),
                item.match_percentage,
                item.matched_columns_count,
                item.num_columns,
                item.standard_encoding.display_name(),
                item.privacy.display_name(),
                item.num_records,
                size_mb
            ));
        }

        md.push_str("\n### Detalhes de Auditoria por Base:\n\n");
        for (i, item) in self.valid_dbfs.iter().enumerate() {
            md.push_str(&format!("#### {}. {}\n", i + 1, item.display_name));
            md.push_str(&format!("- **Caminho:** `{}`\n", item.file_path));
            md.push_str(&format!("- **Sistema:** {}\n", item.system.display_name()));
            md.push_str(&format!(
                "- **Correspondência com o Schema Oficial:** {:.1}% ({} de {} colunas do arquivo pertencem ao dicionário de {} colunas do sistema)\n",
                item.match_percentage, item.matched_columns_count, item.num_columns, item.total_schema_columns
            ));
            md.push_str(&format!(
                "- **Encoding Padronizado:** {} ({})\n",
                item.standard_encoding.display_name(), item.encoding_rationale
            ));
            if !item.key_columns_found.is_empty() {
                md.push_str(&format!(
                    "- **Colunas-chave/assinatura identificadas:** {}\n",
                    item.key_columns_found.join(", ")
                ));
            }
            md.push_str(&format!("- **Privacidade:** {}\n", item.privacy.display_name()));
            if !item.personal_columns_found.is_empty() {
                md.push_str(&format!(
                    "- **Campos com dados pessoais encontrados:** {}\n",
                    item.personal_columns_found.join(", ")
                ));
            } else {
                md.push_str("- **Campos com dados pessoais:** Nenhum campo nominal/endereço detectado (Base Anonimizada).\n");
            }

            if !item.warnings.is_empty() {
                md.push_str(&format!("- **Avisos:** {}\n", item.warnings.join("; ")));
            }
            md.push('\n');
        }

        md
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostProcessingReport {
    pub completed_at: String,
    pub total_duration: String,
    pub total_records_written: u64,
    pub successful_files: Vec<ConversionSummaryItem>,
    pub failed_files: Vec<String>,
    pub ignored_archives_without_dbf: Vec<String>,
}

impl PostProcessingReport {
    pub fn new(duration: Duration) -> Self {
        let secs = duration.as_secs_f64();
        let dur_str = if secs < 60.0 {
            format!("{:.2} segundos", secs)
        } else {
            format!("{:.1} minutos", secs / 60.0)
        };

        Self {
            completed_at: chrono::Local::now().format("%d/%m/%Y %H:%M:%S").to_string(),
            total_duration: dur_str,
            total_records_written: 0,
            successful_files: Vec::new(),
            failed_files: Vec::new(),
            ignored_archives_without_dbf: Vec::new(),
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Relatório Pós-Processamento — Conversão Parquet Concluída\n\n");
        md.push_str(&format!("**Finalizado em:** {}\n", self.completed_at));
        md.push_str(&format!("**Tempo Total de Execução:** {}\n", self.total_duration));
        md.push_str(&format!("**Total de Registros Convertidos:** {}\n\n", self.total_records_written));

        if !self.ignored_archives_without_dbf.is_empty() {
            md.push_str("### ⚠️ Arquivos Compactados Sem DBF Ignorados:\n");
            for f in &self.ignored_archives_without_dbf {
                md.push_str(&format!("- {}\n", f));
            }
            md.push('\n');
        }

        if !self.failed_files.is_empty() {
            md.push_str("### ❌ Falhas Durante a Conversão:\n");
            for f in &self.failed_files {
                md.push_str(&format!("- {}\n", f));
            }
            md.push('\n');
        }

        md.push_str("### Arquivos Parquet Gerados com Sucesso:\n\n");
        md.push_str("| Arquivo de Saída (Parquet) | Registros | Tamanho Parquet | Tempo |\n");
        md.push_str("|---|---|---|---|\n");

        for item in &self.successful_files {
            let size_mb = (item.file_size_bytes as f64) / (1024.0 * 1024.0);
            md.push_str(&format!(
                "| `{}` | {} | {:.2} MB | {:.2}s |\n",
                item.output_path,
                item.records,
                size_mb,
                (item.duration_ms as f64) / 1000.0
            ));
        }

        md
    }
}
