use std::collections::HashSet;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::detector::DbfAnalysis;
use crate::parquet_writer::ConversionSummaryItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialColumnInfo {
    pub column_name: String,
    pub present_in_count: usize,
    pub total_files: usize,
    pub present_in_files: Vec<String>,
    pub missing_in_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaCompatibilityReport {
    pub total_files: usize,
    pub total_unique_columns: usize,
    pub common_columns_count: usize,
    /// Colunas presentes apenas em parte dos arquivos (não existem em todos)
    pub partial_columns: Vec<PartialColumnInfo>,
    /// True quando todos têm as mesmas colunas, mas em ordem diferente
    pub different_column_order: bool,
    /// Códigos distintos de sistemas de saúde detectados (ex: ["SIM", "SINASC"])
    pub distinct_systems: Vec<String>,
    /// True se algum arquivo não pertence a nenhum sistema identificado na pré-avaliação
    pub has_unidentified_system: bool,
}

impl SchemaCompatibilityReport {
    pub fn has_missing_columns(&self) -> bool {
        !self.partial_columns.is_empty()
    }

    pub fn has_mixed_systems(&self) -> bool {
        self.distinct_systems.len() > 1
    }

    /// Merge só é permitido quando todos os arquivos pertencem a um mesmo sistema já identificado na pré-avaliação.
    pub fn is_merge_allowed(&self) -> bool {
        !self.has_mixed_systems() && !self.has_unidentified_system
    }

    pub fn has_differences(&self) -> bool {
        self.has_missing_columns()
            || self.different_column_order
            || self.has_mixed_systems()
            || self.has_unidentified_system
    }
}

/// Analisa se os arquivos DBF possuem schemas idênticos ou divergentes (colunas faltantes,
/// colunas exclusivas, ordem diferente ou sistemas distintos).
pub fn analyze_schema_compatibility(dbfs: &[DbfAnalysis]) -> Option<SchemaCompatibilityReport> {
    if dbfs.len() < 2 {
        return None;
    }

    let total_files = dbfs.len();
    let mut all_cols_ordered: Vec<String> = Vec::new();
    let mut seen_cols: HashSet<String> = HashSet::new();
    let mut distinct_systems: Vec<String> = Vec::new();
    let mut has_unidentified_system = false;

    for dbf in dbfs {
        if matches!(dbf.system, crate::detector::HealthSystem::Other(_)) {
            has_unidentified_system = true;
        }
        let sys_code = dbf.system.code().to_string();
        if !distinct_systems.contains(&sys_code) {
            distinct_systems.push(sys_code);
        }
        for col in &dbf.column_names {
            if seen_cols.insert(col.clone()) {
                all_cols_ordered.push(col.clone());
            }
        }
    }

    let file_col_sets: Vec<HashSet<&str>> = dbfs
        .iter()
        .map(|d| d.column_names.iter().map(|s| s.as_str()).collect())
        .collect();

    let mut common_columns_count = 0usize;
    let mut partial_columns: Vec<PartialColumnInfo> = Vec::new();

    for col in &all_cols_ordered {
        let mut present_in_files = Vec::new();
        let mut missing_in_files = Vec::new();

        for (i, set) in file_col_sets.iter().enumerate() {
            let fname = dbfs[i].display_name.clone();
            if set.contains(col.as_str()) {
                present_in_files.push(fname);
            } else {
                missing_in_files.push(fname);
            }
        }

        if missing_in_files.is_empty() {
            common_columns_count += 1;
        } else {
            partial_columns.push(PartialColumnInfo {
                column_name: col.clone(),
                present_in_count: present_in_files.len(),
                total_files,
                present_in_files,
                missing_in_files,
            });
        }
    }

    let first_cols = &dbfs[0].column_names;
    let different_column_order = dbfs[1..].iter().any(|d| &d.column_names != first_cols);

    Some(SchemaCompatibilityReport {
        total_files,
        total_unique_columns: all_cols_ordered.len(),
        common_columns_count,
        partial_columns,
        different_column_order,
        distinct_systems,
        has_unidentified_system,
    })
}

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

    pub fn schema_compatibility(&self) -> Option<SchemaCompatibilityReport> {
        analyze_schema_compatibility(&self.valid_dbfs)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Relatório Pré-Processamento — Análise das Bases DBF\n\n");
        md.push_str(&format!("**Data/Hora da Análise:** {}\n\n", self.generated_at));

        if !self.invalid_archives_without_dbf.is_empty() {
            md.push_str("### ⚠ Arquivos Compactados Sem DBF (Serão Desconsiderados):\n");
            for item in &self.invalid_archives_without_dbf {
                md.push_str(&format!("- **{}** (nenhum arquivo .dbf encontrado internamente)\n", item));
            }
            md.push('\n');
        }

        if let Some(compat) = self.schema_compatibility() {
            if compat.has_differences() {
                md.push_str("### ⚠ Alerta de Diferença de Schemas (Colunas entre Arquivos):\n");
                if compat.has_mixed_systems() {
                    md.push_str(&format!(
                        "- **Sistemas diferentes detectados:** {}\n",
                        compat.distinct_systems.join(", ")
                    ));
                }
                if compat.has_missing_columns() {
                    md.push_str(&format!(
                        "- **Colunas comuns a todos os {} arquivos:** {} de {} colunas únicas.\n",
                        compat.total_files, compat.common_columns_count, compat.total_unique_columns
                    ));
                    md.push_str(&format!(
                        "- **Colunas presentes apenas em parte dos arquivos ({}):** {}\n",
                        compat.partial_columns.len(),
                        compat
                            .partial_columns
                            .iter()
                            .map(|c| format!("`{}` ({}/{})", c.column_name, c.present_in_count, c.total_files))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                } else if compat.different_column_order {
                    md.push_str("- Todos os arquivos possuem as mesmas colunas, mas em **ordem diferente**.\n");
                }
                md.push_str("- **Como a unificação (Merge) trata isso:** utiliza **união por nome (`union_by_name`)** — alinha colunas de mesmo nome mesmo fora de ordem e preenche com `NULL` as colunas ausentes em algum arquivo.\n\n");
            }
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
            md.push_str("### ⚠ Arquivos Compactados Sem DBF Ignorados:\n");
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
