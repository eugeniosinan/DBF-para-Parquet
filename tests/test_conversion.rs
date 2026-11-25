use std::path::PathBuf;

use dbf_para_parquet::archive::{ArchiveExtractionResult, ArchiveManager};
use dbf_para_parquet::dbf::{parse_date_tolerant, ConversionMode, DbfField, DbfFieldType, DbfHeader};
use dbf_para_parquet::detector::{DbfDetector, HealthSystem, PrivacyLevel};
use dbf_para_parquet::parquet_writer::ParquetConverter;

#[test]
fn test_date_parser_formats() {
    // 01/01/2025 -> DDMMYYYY
    let d1 = parse_date_tolerant("01012025");
    assert!(d1.is_some());

    // 20250101 -> YYYYMMDD
    let d2 = parse_date_tolerant("20250101");
    assert!(d2.is_some());
    assert_eq!(d1, d2);

    // 01/01/2025
    let d3 = parse_date_tolerant("01/01/2025");
    assert_eq!(d1, d3);

    // 2025-01-01
    let d4 = parse_date_tolerant("2025-01-01");
    assert_eq!(d1, d4);

    // Regressão: 20101203 (03/12/2010 em YYYYMMDD) NÃO pode ser lida como 20/10/1203
    let d_iso_2010 = parse_date_tolerant("20101203");
    let d_br_2010 = parse_date_tolerant("03122010");
    let d_hyphen_2010 = parse_date_tolerant("2010-12-03");
    assert!(d_iso_2010.is_some());
    assert_eq!(d_iso_2010, d_br_2010);
    assert_eq!(d_iso_2010, d_hyphen_2010);

    // 20/10/2020 em DDMMYYYY (onde 2010 pareceria um ano em YYYYMMDD, mas mês=20 é inválido)
    assert_eq!(parse_date_tolerant("20102020"), parse_date_tolerant("2020-10-20"));

    // Invalid / blank dates should return None
    assert_eq!(parse_date_tolerant(""), None);
    assert_eq!(parse_date_tolerant("        "), None);
    assert_eq!(parse_date_tolerant("00000000"), None);
    assert_eq!(parse_date_tolerant("________"), None);
}

#[test]
fn test_detector_rules() {
    let mut header = DbfHeader {
        version: 3,
        last_update: (2025, 1, 1),
        num_records: 100,
        header_len: 100,
        record_len: 50,
        fields: vec![
            DbfField {
                name: "NUMERODO".to_string(),
                field_type: DbfFieldType::Character,
                length: 8,
                decimals: 0,
            },
            DbfField {
                name: "DTOBITO".to_string(),
                field_type: DbfFieldType::Character,
                length: 8,
                decimals: 0,
            },
            DbfField {
                name: "NOME".to_string(),
                field_type: DbfFieldType::Character,
                length: 50,
                decimals: 0,
            },
        ],
    };

    let analysis = DbfDetector::inspect(
        &header,
        "sim_teste.dbf".to_string(),
        None,
        ".".to_string(),
        false,
        5000,
    );
    assert_eq!(analysis.system, HealthSystem::SIM);
    assert_eq!(analysis.privacy, PrivacyLevel::WithPersonalData);

    // Remove NOME to test anonymized database detection
    header.fields.pop();
    let analysis_anon = DbfDetector::inspect(
        &header,
        "sim_anon.dbf".to_string(),
        None,
        ".".to_string(),
        false,
        5000,
    );
    assert_eq!(analysis_anon.system, HealthSystem::SIM);
    assert_eq!(analysis_anon.privacy, PrivacyLevel::WithoutPersonalData);
}

#[test]
fn test_archive_inspection_and_conversion() {
    let zip_sim = PathBuf::from("Bases de Exemplo/SIM/DORJ2025_19112026.zip");
    if !zip_sim.exists() {
        return;
    }

    match ArchiveManager::process_archive(&zip_sim) {
        ArchiveExtractionResult::Success { dbf_files, .. } => {
            assert!(!dbf_files.is_empty());
            let dbf = &dbf_files[0];
            let analysis = DbfDetector::analyze_header(
                dbf,
                Some("DORJ2025".to_string()),
                "Bases de Exemplo/SIM".to_string(),
                true,
            ).unwrap();
            assert_eq!(analysis.system, HealthSystem::SIM);
            assert_eq!(analysis.privacy, PrivacyLevel::WithPersonalData);
            assert_eq!(analysis.num_records, 64936);

            let out_parquet = tempfile::NamedTempFile::new().unwrap().into_temp_path().with_extension("parquet");
            let res = ParquetConverter::convert_single(
                dbf,
                &out_parquet,
                ConversionMode::EpidemiologicalDates,
                analysis.standard_encoding,
                None,
            ).unwrap();
            assert_eq!(res.records, 64936);
            assert!(out_parquet.exists());
        }
        _ => panic!("Esperado sucesso na extração do ZIP"),
    }
}

#[test]
fn test_all_systems_column_detection_and_encodings() {
    use std::collections::HashSet;
    use dbf_para_parquet::dbf::DbfEncoding;

    // 1. SIM
    let sim_cols: HashSet<String> = ["NUMERODO", "DTOBITO", "CAUSABAS", "LINHAA", "LINHAB", "IDADE", "SEXO"]
        .iter().map(|s| s.to_string()).collect();
    let res_sim = DbfDetector::detect_system_by_columns(&sim_cols);
    assert_eq!(res_sim.system, HealthSystem::SIM);
    assert_eq!(res_sim.system.standard_encoding(), DbfEncoding::Iso8859_1);
    assert!(res_sim.match_percentage > 90.0);

    // 2. SINASC
    let sinasc_cols: HashSet<String> = ["NUMERODN", "DTNASC", "HORANASC", "APGAR1", "APGAR5", "PESO", "PARTO"]
        .iter().map(|s| s.to_string()).collect();
    let res_sinasc = DbfDetector::detect_system_by_columns(&sinasc_cols);
    assert_eq!(res_sinasc.system, HealthSystem::SINASC);
    assert_eq!(res_sinasc.system.standard_encoding(), DbfEncoding::Iso8859_1);
    assert!(res_sinasc.match_percentage > 90.0);

    // 3. SIVEP SRAG
    let srag_cols: HashSet<String> = ["NU_NOTIFIC", "DT_NOTIFIC", "DT_SIN_PRI", "VACINA_COV", "DOSE_1_COV", "DT_INTERNA", "DT_ENTUTI"]
        .iter().map(|s| s.to_string()).collect();
    let res_srag = DbfDetector::detect_system_by_columns(&srag_cols);
    assert_eq!(res_srag.system, HealthSystem::SivepSrag);
    assert_eq!(res_srag.system.standard_encoding(), DbfEncoding::Iso8859_1);

    // 4. SIVEP SG
    let sg_cols: HashSet<String> = ["NUM_REGIST", "DT_PREENC", "DT_PRISINT", "SE_PRISINT", "DOR_GARGAN", "SIND_DOWN", "IMUNODEPRE"]
        .iter().map(|s| s.to_string()).collect();
    let res_sg = DbfDetector::detect_system_by_columns(&sg_cols);
    assert_eq!(res_sg.system, HealthSystem::SivepSg);
    assert_eq!(res_sg.system.standard_encoding(), DbfEncoding::Iso8859_1);

    // 5. DENGON
    let dengon_cols: HashSet<String> = ["ID_AGRAVO", "DT_NOTIFIC", "DT_SIN_PRI", "DOR_RETRO", "PETEQUIA_N", "LACO", "LEUCOPENIA"]
        .iter().map(|s| s.to_string()).collect();
    let res_dengon = DbfDetector::detect_system_by_columns(&dengon_cols);
    assert_eq!(res_dengon.system, HealthSystem::Dengon);
    assert_eq!(res_dengon.system.standard_encoding(), DbfEncoding::Iso8859_1);

    // 6. SINAN NET
    let sinan_cols: HashSet<String> = [
        "NU_NOTIFIC", "TP_NOT", "ID_AGRAVO", "DT_NOTIFIC", "SEM_NOT", "NU_ANO",
        "SG_UF_NOT", "ID_UNIDADE", "ID_REGIONA", "ID_MUNICIP", "FONETICA_N",
    ].iter().map(|s| s.to_string()).collect();
    let res_sinan = DbfDetector::detect_system_by_columns(&sinan_cols);
    assert_eq!(res_sinan.system, HealthSystem::SinanNet);
    assert_eq!(res_sinan.system.standard_encoding(), DbfEncoding::Iso8859_1);
}

#[test]
fn test_convert_each_system_sample() {
    let test_samples = [
        ("Bases de Exemplo/DENGON/Chikv2025_2026_11_23.zip", HealthSystem::Dengon, 550),
        ("Bases de Exemplo/SIVEP_SENTINELA_SG/SG-2026.zip", HealthSystem::SivepSg, 4015),
        ("Bases de Exemplo/SIVEP_SRAG/SRAG_2024_23112026.zip", HealthSystem::SivepSrag, 7279),
    ];

    for (zip_path_str, expected_system, expected_records) in test_samples {
        let zip_p = PathBuf::from(zip_path_str);
        if !zip_p.exists() {
            continue;
        }

        match ArchiveManager::process_archive(&zip_p) {
            ArchiveExtractionResult::Success { dbf_files, .. } => {
                assert!(!dbf_files.is_empty());
                let dbf = &dbf_files[0];
                let analysis = DbfDetector::analyze_header(
                    dbf,
                    Some(zip_path_str.to_string()),
                    ".".to_string(),
                    true,
                ).unwrap();

                assert_eq!(analysis.system, expected_system);
                assert_eq!(analysis.num_records, expected_records);

                let out_parquet = tempfile::NamedTempFile::new().unwrap().into_temp_path().with_extension("parquet");
                let summary = ParquetConverter::convert_single(
                    dbf,
                    &out_parquet,
                    ConversionMode::EpidemiologicalDates,
                    analysis.standard_encoding,
                    None,
                ).unwrap();

                assert_eq!(summary.records, expected_records as u64);
                assert!(out_parquet.exists());
                assert!(std::fs::metadata(&out_parquet).unwrap().len() > 0);
            }
            _ => panic!("Falha ao extrair {}", zip_path_str),
        }
    }
}

#[test]
fn test_merge_multiple_years() {
    let zip1 = PathBuf::from("Bases de Exemplo/DENGON/Chikv2025_2026_11_23.zip");
    let zip2 = PathBuf::from("Bases de Exemplo/DENGON/Chikv2026_2026_11_23.zip");

    if !zip1.exists() || !zip2.exists() {
        return;
    }

    let ext1 = match ArchiveManager::process_archive(&zip1) {
        ArchiveExtractionResult::Success { dbf_files, _temp_dir, .. } => (dbf_files, _temp_dir),
        _ => panic!("Falha zip1"),
    };
    let ext2 = match ArchiveManager::process_archive(&zip2) {
        ArchiveExtractionResult::Success { dbf_files, _temp_dir, .. } => (dbf_files, _temp_dir),
        _ => panic!("Falha zip2"),
    };

    let files_to_merge = vec![ext1.0[0].clone(), ext2.0[0].clone()];
    let out_parquet = tempfile::NamedTempFile::new().unwrap().into_temp_path().with_extension("parquet");

    let summary = ParquetConverter::convert_merged(
        &files_to_merge,
        &out_parquet,
        ConversionMode::EpidemiologicalDates,
        dbf_para_parquet::dbf::DbfEncoding::Iso8859_1,
        None,
    ).unwrap();

    // 550 + 4287 = 4837
    assert_eq!(summary.records, 4837);
    assert!(out_parquet.exists());
    assert!(std::fs::metadata(&out_parquet).unwrap().len() > 0);
}

#[test]
fn test_sinan_net_detection_and_conversion() {
    let animp_path = PathBuf::from("Bases de Exemplo/SINANNET/ANIMPNET.DBF");
    if animp_path.exists() {
        let analysis = DbfDetector::analyze_header(
            &animp_path,
            None,
            "Bases de Exemplo/SINANNET".to_string(),
            false,
        ).unwrap();

        assert_eq!(analysis.system, HealthSystem::SinanNet);
        assert_eq!(analysis.num_records, 4790);
        assert_eq!(analysis.privacy, PrivacyLevel::WithPersonalData);

        let out_parquet = tempfile::NamedTempFile::new().unwrap().into_temp_path().with_extension("parquet");
        let summary = ParquetConverter::convert_single(
            &animp_path,
            &out_parquet,
            ConversionMode::EpidemiologicalDates,
            analysis.standard_encoding,
            None,
        ).unwrap();

        assert_eq!(summary.records, 4790);
        assert!(out_parquet.exists());
        assert!(std::fs::metadata(&out_parquet).unwrap().len() > 0);
    }

    let bairr_path = PathBuf::from("Bases de Exemplo/SINANNET/BAIRRNET.DBF");
    if bairr_path.exists() {
        let analysis_bairr = DbfDetector::analyze_header(
            &bairr_path,
            None,
            "Bases de Exemplo/SINANNET".to_string(),
            false,
        ).unwrap();

        assert_eq!(analysis_bairr.system, HealthSystem::SinanNet);
        assert_eq!(analysis_bairr.num_records, 168);
        assert_eq!(analysis_bairr.privacy, PrivacyLevel::WithoutPersonalData);
    }
}

#[test]
fn test_dissemin_dbc_detection_and_conversion() {
    let test_cases = vec![
        ("Bases_Dissemin_Exemplo/DOAC2022.dbc", HealthSystem::SIM, 4159),
        ("Bases_Dissemin_Exemplo/DNAC2022.DBC", HealthSystem::SINASC, 14483),
        ("Bases_Dissemin_Exemplo/CHAGBR15.dbc", HealthSystem::SinanNet, 4225),
    ];

    for (dbc_path_str, expected_sys, expected_records) in test_cases {
        let p = PathBuf::from(dbc_path_str);
        if !p.exists() {
            continue;
        }

        assert!(ArchiveManager::is_archive(&p));
        match ArchiveManager::process_archive(&p) {
            ArchiveExtractionResult::Success {
                archive_name,
                dbf_files,
                _temp_dir,
            } => {
                assert_eq!(dbf_files.len(), 1);
                let dbf_file = &dbf_files[0];

                let analysis = DbfDetector::analyze_header(
                    dbf_file,
                    Some(archive_name),
                    "Bases_Dissemin_Exemplo".to_string(),
                    true,
                ).unwrap();

                assert_eq!(analysis.system, expected_sys);
                assert_eq!(analysis.num_records, expected_records);
                // Todas as bases públicas do DISSEMIN devem ser classificadas como anonimizadas
                assert_eq!(analysis.privacy, PrivacyLevel::WithoutPersonalData);

                let out_parquet = tempfile::NamedTempFile::new().unwrap().into_temp_path().with_extension("parquet");
                let summary = ParquetConverter::convert_single(
                    dbf_file,
                    &out_parquet,
                    ConversionMode::EpidemiologicalDates,
                    analysis.standard_encoding,
                    None,
                ).unwrap();

                assert_eq!(summary.records, expected_records as u64);
                assert!(out_parquet.exists());
                assert!(std::fs::metadata(&out_parquet).unwrap().len() > 0);
            }
            _ => panic!("Falha ao extrair DBC {}", dbc_path_str),
        }
    }
}


