pub mod schemas;

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::dbf::{ConversionMode, DbfEncoding, DbfHeader, DbfReader};
use self::schemas::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthSystem {
    SIM,
    SINASC,
    SivepSrag,
    SivepSg,
    Dengon,
    SinanNet,
    Other(String),
}

impl HealthSystem {
    pub fn display_name(&self) -> &str {
        match self {
            HealthSystem::SIM => "SIM (Sistema de Informações sobre Mortalidade)",
            HealthSystem::SINASC => "SINASC (Sistema de Informações sobre Nascidos Vivos)",
            HealthSystem::SivepSrag => "SIVEP SRAG (Hospitalizados por SRAG)",
            HealthSystem::SivepSg => "SIVEP SG (Sentinela de Síndrome Gripal)",
            HealthSystem::Dengon => "DENGUE ONLINE (DENGON / Arboviroses)",
            HealthSystem::SinanNet => "SINAN NET (Sistema de Informação de Agravos de Notificação)",
            HealthSystem::Other(s) => s.as_str(),
        }
    }

    pub fn code(&self) -> &str {
        match self {
            HealthSystem::SIM => "SIM",
            HealthSystem::SINASC => "SINASC",
            HealthSystem::SivepSrag => "SIVEP_SRAG",
            HealthSystem::SivepSg => "SIVEP_SG",
            HealthSystem::Dengon => "DENGON",
            HealthSystem::SinanNet => "SINAN_NET",
            HealthSystem::Other(_) => "OUTRO",
        }
    }

    /// Retorna a codificação padrão utilizada por este sistema de saúde (ISO-8859-1 / Windows-1252).
    pub fn standard_encoding(&self) -> DbfEncoding {
        match self {
            HealthSystem::SIM => DbfEncoding::Iso8859_1,
            HealthSystem::SINASC => DbfEncoding::Iso8859_1,
            HealthSystem::SivepSrag => DbfEncoding::Iso8859_1,
            HealthSystem::SivepSg => DbfEncoding::Iso8859_1,
            HealthSystem::Dengon => DbfEncoding::Iso8859_1,
            HealthSystem::SinanNet => DbfEncoding::Iso8859_1,
            HealthSystem::Other(_) => DbfEncoding::Iso8859_1,
        }
    }

    /// Justificativa técnica do encoding padronizado
    pub fn encoding_rationale(&self) -> &'static str {
        match self {
            HealthSystem::SIM => "Codificação padrão para SIM (ISO-8859-1 / Windows-1252)",
            HealthSystem::SINASC => "Codificação padrão para SINASC (ISO-8859-1 / Windows-1252)",
            HealthSystem::SivepSrag => "Codificação padrão para SIVEP SRAG (ISO-8859-1 / Windows-1252)",
            HealthSystem::SivepSg => "Codificação padrão para SIVEP SG (ISO-8859-1 / Windows-1252)",
            HealthSystem::Dengon => "Codificação padrão para DENGON (ISO-8859-1 / Windows-1252)",
            HealthSystem::SinanNet => "Codificação padrão para SINAN NET (ISO-8859-1 / Windows-1252)",
            HealthSystem::Other(_) => "Codificação padrão geral (ISO-8859-1 / Windows-1252)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivacyLevel {
    WithPersonalData,
    WithoutPersonalData,
}

impl PrivacyLevel {
    pub fn display_name(&self) -> &str {
        match self {
            PrivacyLevel::WithPersonalData => "Base Identificada (Com dados pessoais de pacientes)",
            PrivacyLevel::WithoutPersonalData => "Base Anonimizada (Disseminação pública - dados pessoais omitidos)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbfAnalysis {
    pub file_path: String,
    pub display_name: String,
    pub source_archive: Option<String>,
    pub original_dir: String,
    pub is_from_temp_archive: bool,
    pub file_size_bytes: u64,
    pub num_records: u32,
    pub num_columns: usize,
    pub system: HealthSystem,
    pub privacy: PrivacyLevel,
    pub personal_columns_found: Vec<String>,
    pub key_columns_found: Vec<String>,
    pub column_names: Vec<String>,
    pub warnings: Vec<String>,
    // Padronização de Encoding e Correspondência de Schema
    pub standard_encoding: DbfEncoding,
    pub encoding_rationale: String,
    pub match_percentage: f32,
    pub matched_columns_count: usize,
    pub total_schema_columns: usize,
}

pub struct DbfDetector;

impl DbfDetector {
    /// Lista de colunas estritamente de dados pessoais do paciente/cidadão (LGPD).
    /// NOTA: Dados profissionais (médico, CRM, etc.) não configuram identificação pessoal do paciente.
    const PERSONAL_COLUMNS: &'static [&'static str] = &[
        // Nomes do paciente / nascido / mãe / pai / responsável
        "NOME",
        "NOMEPAI",
        "NOMEMAE",
        "NOMEPACIENT",
        "NM_PACIENT",
        "NOME_PAC",
        "NOME_MAE",
        "NM_MAE_PAC",
        "NM_MAE",
        "NOMERNASC",
        "NOMERESPO",
        // Documentos de identificação do paciente
        "CPF",
        "NU_CPF",
        "CNS",
        "NU_CNS",
        "ID_CNS_SUS",
        "NUMSUS",
        "NUMSUSMAE",
        "DOC_IDENT",
        "TEL_CONTAT",
        "NU_DDD_TEL",
        // Endereço residencial detalhado do paciente
        "ENDRES",
        "NUMRES",
        "COMPLRES",
        "CEPRES",
        "CODBAIRES",
        "CODENDRES",
        "NM_LOGR",
        "NM_LOGRADO",
        "NU_NUMERO",
        "NM_COMPLEM",
        "NM_REFEREN",
        // Endereço de ocorrência do nascimento
        "ENDNASC",
        "NUMENDNASC",
        "COMPLNASC",
        "CEPNASC",
    ];

    pub fn analyze_header<P: AsRef<Path>>(
        path: P,
        source_archive: Option<String>,
        original_dir: String,
        is_from_temp_archive: bool,
    ) -> Result<DbfAnalysis, String> {
        let p = path.as_ref();
        let metadata = std::fs::metadata(p)
            .map_err(|e| format!("Falha ao ler metadados de {:?}: {}", p, e))?;
        let file_size_bytes = metadata.len();

        let reader = DbfReader::open(p, ConversionMode::StrictDbfTypes)?;
        let header = &reader.header;

        let analysis = Self::inspect(
            header,
            p.to_string_lossy().to_string(),
            source_archive,
            original_dir,
            is_from_temp_archive,
            file_size_bytes,
        );
        Ok(analysis)
    }

    pub fn inspect(
        header: &DbfHeader,
        file_path: String,
        source_archive: Option<String>,
        original_dir: String,
        is_from_temp_archive: bool,
        file_size_bytes: u64,
    ) -> DbfAnalysis {
        let mut column_names = Vec::with_capacity(header.fields.len());
        let mut col_set = HashSet::new();

        for f in &header.fields {
            column_names.push(f.name.clone());
            col_set.insert(f.name.to_uppercase());
        }

        // 1. Identificar dados pessoais do paciente/cidadão (LGPD)
        let mut personal_found = Vec::new();
        for &col in Self::PERSONAL_COLUMNS {
            if col_set.contains(col) {
                personal_found.push(col.to_string());
            }
        }

        let privacy = if !personal_found.is_empty() {
            PrivacyLevel::WithPersonalData
        } else {
            PrivacyLevel::WithoutPersonalData
        };

        let display_name = Path::new(&file_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.clone());

        // 2. Identificar o sistema da vigilância a partir da análise das colunas e nome do arquivo
        let detection_result = Self::detect_system(&col_set, Some(&display_name));

        let system = detection_result.system;
        let standard_encoding = system.standard_encoding();
        let encoding_rationale = system.encoding_rationale().to_string();

        let mut warnings = Vec::new();
        if let HealthSystem::Other(_) = &system {
            warnings.push("Base não reconhecida automaticamente entre SIM, SINASC, SIVEP, DENGON ou SINAN NET.".to_string());
        }

        if header.num_records == 0 {
            warnings.push("A base DBF não contém nenhum registro cadastrado.".to_string());
        }

        DbfAnalysis {
            file_path,
            display_name,
            source_archive,
            original_dir,
            is_from_temp_archive,
            file_size_bytes,
            num_records: header.num_records,
            num_columns: header.fields.len(),
            system,
            privacy,
            personal_columns_found: personal_found,
            key_columns_found: detection_result.key_columns,
            column_names,
            warnings,
            standard_encoding,
            encoding_rationale,
            match_percentage: detection_result.match_percentage,
            matched_columns_count: detection_result.matched_columns_count,
            total_schema_columns: detection_result.total_schema_columns,
        }
    }

    pub fn detect_system_by_columns(cols: &HashSet<String>) -> SystemDetectionResult {
        Self::detect_system(cols, None)
    }

    /// Identifica o sistema de vigilância comparando detalhadamente as colunas do DBF
    /// com os esquemas oficiais conhecidos (SIM, SINASC, SIVEP SRAG, SIVEP SG, DENGON, SINAN NET).
    pub fn detect_system(cols: &HashSet<String>, filename: Option<&str>) -> SystemDetectionResult {
        if cols.is_empty() {
            return SystemDetectionResult {
                system: HealthSystem::Other("Base sem colunas".to_string()),
                key_columns: Vec::new(),
                match_percentage: 0.0,
                matched_columns_count: 0,
                total_schema_columns: 0,
            };
        }

        struct SystemSpec<'a> {
            system: HealthSystem,
            schema_cols: &'a [&'a str],
            signature_cols: &'a [&'a str],
        }

        let systems = [
            SystemSpec {
                system: HealthSystem::SIM,
                schema_cols: SIM_COLUMNS,
                signature_cols: SIM_SIGNATURE,
            },
            SystemSpec {
                system: HealthSystem::SINASC,
                schema_cols: SINASC_COLUMNS,
                signature_cols: SINASC_SIGNATURE,
            },
            SystemSpec {
                system: HealthSystem::SivepSg,
                schema_cols: SIVEP_SG_COLUMNS,
                signature_cols: SIVEP_SG_SIGNATURE,
            },
            SystemSpec {
                system: HealthSystem::SivepSrag,
                schema_cols: SIVEP_SRAG_COLUMNS,
                signature_cols: SIVEP_SRAG_SIGNATURE,
            },
            SystemSpec {
                system: HealthSystem::Dengon,
                schema_cols: DENGON_COLUMNS,
                signature_cols: DENGON_SIGNATURE,
            },
            SystemSpec {
                system: HealthSystem::SinanNet,
                schema_cols: SINAN_NET_COLUMNS,
                signature_cols: SINAN_NET_SIGNATURE,
            },
        ];

        let mut best_system: Option<HealthSystem> = None;
        let mut best_score = 0.0f32;
        let mut best_matched_count = 0usize;
        let mut best_total_schema = 0usize;
        let mut best_key_cols = Vec::new();
        let mut best_percentage = 0.0f32;

        let filename_stem = filename.map(|f| {
            Path::new(f)
                .file_stem()
                .map(|s| s.to_string_lossy().to_uppercase())
                .unwrap_or_default()
        });

        for spec in &systems {
            let mut matched_count = 0usize;
            let mut signature_count = 0usize;
            let mut found_keys = Vec::new();

            for &sc in spec.signature_cols {
                if cols.contains(sc) {
                    signature_count += 1;
                    found_keys.push(sc.to_string());
                }
            }

            for &c in spec.schema_cols {
                if cols.contains(c) {
                    matched_count += 1;
                }
            }

            let coverage = (matched_count as f32 / cols.len() as f32) * 100.0;
            // Pontuação: peso alto para colunas de assinatura exclusivas + proporção de cobertura
            let mut score = (signature_count as f32 * 20.0) + coverage;

            // Se possui colunas clínicas exclusivas de arboviroses (Dengue/Chikungunya), prioriza DENGON
            if spec.system == HealthSystem::Dengon && signature_count >= 3 {
                score += 500.0;
            }

            // Bônus se o nome do arquivo corresponde a padrões típicos de tabelas do sistema
            if let Some(stem) = &filename_stem {
                if spec.system == HealthSystem::SinanNet
                    && (stem.ends_with("NET") || SINAN_NET_LOOKUP_TABLES.contains(&stem.as_str()))
                {
                    score += 45.0;
                }
            }

            if score > best_score && (signature_count >= 1 || coverage >= 35.0 || score >= 45.0) {
                best_score = score;
                best_system = Some(spec.system.clone());
                best_matched_count = matched_count;
                best_total_schema = spec.schema_cols.len();
                best_key_cols = found_keys;
                best_percentage = coverage;
            }
        }

        if let Some(system) = best_system {
            SystemDetectionResult {
                system,
                key_columns: best_key_cols,
                match_percentage: (best_percentage * 10.0).round() / 10.0,
                matched_columns_count: best_matched_count,
                total_schema_columns: best_total_schema,
            }
        } else {
            SystemDetectionResult {
                system: HealthSystem::Other("Base da Vigilância (Não categorizada)".to_string()),
                key_columns: Vec::new(),
                match_percentage: 0.0,
                matched_columns_count: 0,
                total_schema_columns: 0,
            }
        }
    }
}

pub struct SystemDetectionResult {
    pub system: HealthSystem,
    pub key_columns: Vec<String>,
    pub match_percentage: f32,
    pub matched_columns_count: usize,
    pub total_schema_columns: usize,
}
