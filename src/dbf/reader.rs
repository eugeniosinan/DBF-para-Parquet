use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::builder::{
    BooleanBuilder, Date32Builder, Float64Builder, Int64Builder, StringBuilder,
};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use chrono::NaiveDate;

use crate::dbf::DbfEncoding;

#[derive(Debug, Clone, PartialEq)]
pub enum DbfFieldType {
    Character, // 'C'
    Numeric,   // 'N'
    Float,     // 'F'
    Date,      // 'D'
    Logical,   // 'L'
    Memo,      // 'M'
    Binary,    // 'B'
    Integer,   // 'I'
    Unknown(char),
}

impl DbfFieldType {
    pub fn from_char(c: char) -> Self {
        match c.to_ascii_uppercase() {
            'C' => DbfFieldType::Character,
            'N' => DbfFieldType::Numeric,
            'F' => DbfFieldType::Float,
            'D' => DbfFieldType::Date,
            'L' => DbfFieldType::Logical,
            'M' => DbfFieldType::Memo,
            'B' => DbfFieldType::Binary,
            'I' => DbfFieldType::Integer,
            other => DbfFieldType::Unknown(other),
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            DbfFieldType::Character => 'C',
            DbfFieldType::Numeric => 'N',
            DbfFieldType::Float => 'F',
            DbfFieldType::Date => 'D',
            DbfFieldType::Logical => 'L',
            DbfFieldType::Memo => 'M',
            DbfFieldType::Binary => 'B',
            DbfFieldType::Integer => 'I',
            DbfFieldType::Unknown(c) => *c,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DbfField {
    pub name: String,
    pub field_type: DbfFieldType,
    pub length: u8,
    pub decimals: u8,
}

#[derive(Debug, Clone)]
pub struct DbfHeader {
    pub version: u8,
    pub last_update: (u16, u8, u8),
    pub num_records: u32,
    pub header_len: u16,
    pub record_len: u16,
    pub fields: Vec<DbfField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionMode {
    /// Respeita estritamente o tipo de cabeçalho do DBF
    StrictDbfTypes,
    /// Trata colunas de data (DDMMYYYY) como Date32 mesmo se vierem como 'C' no cabeçalho
    EpidemiologicalDates,
}

pub struct DbfReader {
    pub path: PathBuf,
    pub header: DbfHeader,
    pub schema: SchemaRef,
    pub mode: ConversionMode,
    pub encoding: DbfEncoding,
    reader: BufReader<File>,
    current_record: u32,
}

impl DbfReader {
    pub fn open<P: AsRef<Path>>(path: P, mode: ConversionMode) -> Result<Self, String> {
        Self::open_with_encoding(path, mode, DbfEncoding::Iso8859_1)
    }

    pub fn open_with_encoding<P: AsRef<Path>>(
        path: P,
        mode: ConversionMode,
        encoding: DbfEncoding,
    ) -> Result<Self, String> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf)
            .map_err(|e| format!("Falha ao abrir arquivo DBF {:?}: {}", path_buf, e))?;
        let mut reader = BufReader::with_capacity(512 * 1024, file);

        let header = Self::read_header(&mut reader)?;
        let schema = Self::build_arrow_schema(&header, mode);

        // Posiciona no início do primeiro registro (logo após o header)
        reader
            .seek(SeekFrom::Start(header.header_len as u64))
            .map_err(|e| format!("Falha ao posicionar ponteiro no DBF: {}", e))?;

        Ok(Self {
            path: path_buf,
            header,
            schema: Arc::new(schema),
            mode,
            encoding,
            reader,
            current_record: 0,
        })
    }

    fn read_header(reader: &mut BufReader<File>) -> Result<DbfHeader, String> {
        let mut h_buf = [0u8; 32];
        reader
            .read_exact(&mut h_buf)
            .map_err(|e| format!("Erro ao ler cabeçalho inicial DBF: {}", e))?;

        let version = h_buf[0];
        let year = 1900 + h_buf[1] as u16 + if h_buf[1] < 70 { 100 } else { 0 };
        let month = h_buf[2];
        let day = h_buf[3];
        let num_records = u32::from_le_bytes([h_buf[4], h_buf[5], h_buf[6], h_buf[7]]);
        let header_len = u16::from_le_bytes([h_buf[8], h_buf[9]]);
        let record_len = u16::from_le_bytes([h_buf[10], h_buf[11]]);

        if header_len < 32 {
            return Err(format!(
                "Tamanho do cabeçalho inválido no DBF: {} bytes",
                header_len
            ));
        }

        let num_field_descs = ((header_len - 32) / 32) as usize;
        let mut fields = Vec::with_capacity(num_field_descs);

        for _ in 0..num_field_descs {
            let mut f_buf = [0u8; 32];
            reader
                .read_exact(&mut f_buf)
                .map_err(|e| format!("Erro ao ler descritor de campo DBF: {}", e))?;

            // 0x0D marca o final da lista de descritores de campo
            if f_buf[0] == 0x0D {
                break;
            }

            let name_raw = &f_buf[0..11];
            let name_len = name_raw.iter().position(|&b| b == 0).unwrap_or(11);
            let name = String::from_utf8_lossy(&name_raw[..name_len])
                .trim()
                .to_uppercase();

            if name.is_empty() {
                continue;
            }

            let field_type_char = f_buf[11] as char;
            let field_type = DbfFieldType::from_char(field_type_char);
            let length = f_buf[16];
            let decimals = f_buf[17];

            fields.push(DbfField {
                name,
                field_type,
                length,
                decimals,
            });
        }

        Ok(DbfHeader {
            version,
            last_update: (year, month, day),
            num_records,
            header_len,
            record_len,
            fields,
        })
    }

    pub fn is_known_epidemiological_date_column(col_name: &str) -> bool {
        let upper = col_name.to_uppercase();
        if crate::detector::schemas::EPIDEMIOLOGICAL_DATE_COLUMNS.contains(&upper.as_str()) {
            return true;
        }
        // Prefixo comum para datas em bases de vigilância (DT_ ou DT)
        upper.starts_with("DT_") || (upper.starts_with("DT") && upper.len() >= 6)
    }

    pub fn build_arrow_schema(header: &DbfHeader, mode: ConversionMode) -> Schema {
        let mut arrow_fields = Vec::with_capacity(header.fields.len());

        for f in &header.fields {
            let data_type = match f.field_type {
                DbfFieldType::Date => DataType::Date32,
                DbfFieldType::Character => {
                    if mode == ConversionMode::EpidemiologicalDates
                        && Self::is_known_epidemiological_date_column(&f.name)
                    {
                        DataType::Date32
                    } else {
                        DataType::Utf8
                    }
                }
                DbfFieldType::Numeric => {
                    if f.decimals == 0 {
                        if f.length <= 9 {
                            DataType::Int32
                        } else {
                            DataType::Int64
                        }
                    } else {
                        DataType::Float64
                    }
                }
                DbfFieldType::Float => DataType::Float64,
                DbfFieldType::Logical => DataType::Boolean,
                DbfFieldType::Integer => DataType::Int32,
                _ => DataType::Utf8,
            };

            arrow_fields.push(Field::new(&f.name, data_type, true));
        }

        Schema::new(arrow_fields)
    }

    /// Lê o próximo batch de registros, retornando None quando atingir o fim dos registros
    pub fn next_batch(&mut self, batch_size: usize) -> Result<Option<RecordBatch>, String> {
        if self.current_record >= self.header.num_records {
            return Ok(None);
        }

        let records_to_read =
            std::cmp::min(batch_size, (self.header.num_records - self.current_record) as usize);
        if records_to_read == 0 {
            return Ok(None);
        }

        let record_len = self.header.record_len as usize;
        let mut rec_buf = vec![0u8; record_len];

        // Inicializa builders para cada coluna conforme o schema
        let mut builders: Vec<Box<dyn ColumnBuilder>> = Vec::with_capacity(self.header.fields.len());
        for (i, field) in self.header.fields.iter().enumerate() {
            let dt = self.schema.field(i).data_type();
            match dt {
                DataType::Utf8 => builders.push(Box::new(StringBuilder::with_capacity(
                    records_to_read,
                    records_to_read * field.length as usize,
                ))),
                DataType::Date32 => {
                    builders.push(Box::new(Date32Builder::with_capacity(records_to_read)))
                }
                DataType::Int32 => {
                    builders.push(Box::new(Int32ColBuilder(arrow_array::builder::Int32Builder::with_capacity(records_to_read))))
                }
                DataType::Int64 => {
                    builders.push(Box::new(Int64Builder::with_capacity(records_to_read)))
                }
                DataType::Float64 => {
                    builders.push(Box::new(Float64Builder::with_capacity(records_to_read)))
                }
                DataType::Boolean => {
                    builders.push(Box::new(BooleanBuilder::with_capacity(records_to_read)))
                }
                _ => builders.push(Box::new(StringBuilder::with_capacity(records_to_read, 16))),
            }
        }

        let mut actual_read = 0;

        for _ in 0..records_to_read {
            match self.reader.read_exact(&mut rec_buf) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(format!("Erro ao ler registro {}: {}", self.current_record, e)),
            }

            self.current_record += 1;

            // Byte 0: flag de exclusão (0x2A '*' = deletado, 0x20 ' ' = normal)
            let is_deleted = rec_buf[0] == 0x2A;
            if is_deleted {
                // Registros deletados no padrão DBF são ignorados
                continue;
            }

            let mut offset = 1;
            for (i, f) in self.header.fields.iter().enumerate() {
                let end = offset + f.length as usize;
                let raw_val = if end <= rec_buf.len() {
                    &rec_buf[offset..end]
                } else {
                    &[]
                };
                offset = end;

                builders[i].append_raw(raw_val, f, self.mode, self.encoding)?;
            }

            actual_read += 1;
        }

        if actual_read == 0 {
            return Ok(None);
        }

        let mut arrays: Vec<ArrayRef> = Vec::with_capacity(builders.len());
        for mut b in builders {
            arrays.push(b.finish());
        }

        let batch = RecordBatch::try_new(self.schema.clone(), arrays)
            .map_err(|e| format!("Erro ao criar RecordBatch: {}", e))?;

        Ok(Some(batch))
    }
}

/// Helper para parsing direto do padrão ISO/dBase nativo YYYYMMDD (usado em campos tipo 'D')
pub fn parse_date_yyyymmdd(s: &str) -> Option<i32> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.contains('_') || trimmed.starts_with("0000") || trimmed == "00000000" {
        return None;
    }

    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.len() == 8 {
        if let (Ok(year), Ok(month), Ok(day)) = (
            digits[0..4].parse::<i32>(),
            digits[4..6].parse::<u32>(),
            digits[6..8].parse::<u32>(),
        ) {
            if (1850..=2100).contains(&year) {
                if let Some(nd) = NaiveDate::from_ymd_opt(year, month, day) {
                    return Some(nd.signed_duration_since(epoch).num_days() as i32);
                }
            }
        }
    }

    None
}

/// Helper para parsing tolerante de datas em campos textuais ('C').
/// Com validação de ano plausível (1850..=2100), os formatos DDMMYYYY e YYYYMMDD
/// tornam-se matematicamente mutuamente exclusivos:
/// - Em YYYYMMDD, os dígitos 5..6 são o mês (01..=12), logo os 4 últimos dígitos nunca formam um ano >= 1850.
/// - Em DDMMYYYY, os dígitos 5..6 são o século (18..=21), logo nunca formam um mês válido (01..=12) em YYYYMMDD.
pub fn parse_date_tolerant(s: &str) -> Option<i32> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.contains('_') || trimmed.starts_with("0000") || trimmed == "00000000" {
        return None;
    }

    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();

    // 1. Extração de dígitos numéricos
    let digits: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();

    // Se tiver 8 dígitos numéricos:
    if digits.len() == 8 {
        // Tenta padrão DDMMYYYY (predominante em bases brasileiras tipo 'C', ex: SIM/SINASC)
        if let (Ok(day), Ok(month), Ok(year)) = (
            digits[0..2].parse::<u32>(),
            digits[2..4].parse::<u32>(),
            digits[4..8].parse::<i32>(),
        ) {
            if (1850..=2100).contains(&year) {
                if let Some(nd) = NaiveDate::from_ymd_opt(year, month, day) {
                    return Some(nd.signed_duration_since(epoch).num_days() as i32);
                }
            }
        }

        // Tenta padrão YYYYMMDD (formato nativo dBase 'D' ou DATASUS alternativo)
        if let (Ok(year), Ok(month), Ok(day)) = (
            digits[0..4].parse::<i32>(),
            digits[4..6].parse::<u32>(),
            digits[6..8].parse::<u32>(),
        ) {
            if (1850..=2100).contains(&year) {
                if let Some(nd) = NaiveDate::from_ymd_opt(year, month, day) {
                    return Some(nd.signed_duration_since(epoch).num_days() as i32);
                }
            }
        }
    }

    // 2. Tenta DD/MM/YYYY
    if let Ok(nd) = NaiveDate::parse_from_str(trimmed, "%d/%m/%Y") {
        return Some(nd.signed_duration_since(epoch).num_days() as i32);
    }

    // 3. Tenta YYYY-MM-DD
    if let Ok(nd) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Some(nd.signed_duration_since(epoch).num_days() as i32);
    }

    None
}

trait ColumnBuilder {
    fn append_raw(
        &mut self,
        raw: &[u8],
        field: &DbfField,
        mode: ConversionMode,
        encoding: DbfEncoding,
    ) -> Result<(), String>;
    fn finish(&mut self) -> ArrayRef;
}

impl ColumnBuilder for StringBuilder {
    fn append_raw(
        &mut self,
        raw: &[u8],
        _field: &DbfField,
        _mode: ConversionMode,
        encoding: DbfEncoding,
    ) -> Result<(), String> {
        let decoded = encoding.decode(raw);
        // Sanitização: converte string vazia ou só com espaços em NULL
        let trimmed = decoded.trim();
        if trimmed.is_empty() {
            self.append_null();
        } else {
            self.append_value(trimmed);
        }
        Ok(())
    }

    fn finish(&mut self) -> ArrayRef {
        Arc::new(self.finish())
    }
}

impl ColumnBuilder for Date32Builder {
    fn append_raw(
        &mut self,
        raw: &[u8],
        field: &DbfField,
        _mode: ConversionMode,
        encoding: DbfEncoding,
    ) -> Result<(), String> {
        let decoded = encoding.decode(raw);
        let parsed = if field.field_type == DbfFieldType::Date {
            // Campos nativos dBase tipo 'D' são sempre YYYYMMDD pela especificação do formato
            parse_date_yyyymmdd(&decoded).or_else(|| parse_date_tolerant(&decoded))
        } else {
            parse_date_tolerant(&decoded)
        };
        match parsed {
            Some(days) => self.append_value(days),
            None => self.append_null(),
        }
        Ok(())
    }

    fn finish(&mut self) -> ArrayRef {
        Arc::new(self.finish())
    }
}

struct Int32ColBuilder(arrow_array::builder::Int32Builder);

impl ColumnBuilder for Int32ColBuilder {
    fn append_raw(
        &mut self,
        raw: &[u8],
        _field: &DbfField,
        _mode: ConversionMode,
        _encoding: DbfEncoding,
    ) -> Result<(), String> {
        let s = String::from_utf8_lossy(raw);
        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed.contains('_') {
            self.0.append_null();
        } else {
            match trimmed.parse::<i32>() {
                Ok(v) => self.0.append_value(v),
                Err(_) => self.0.append_null(),
            }
        }
        Ok(())
    }

    fn finish(&mut self) -> ArrayRef {
        Arc::new(self.0.finish())
    }
}

impl ColumnBuilder for Int64Builder {
    fn append_raw(
        &mut self,
        raw: &[u8],
        _field: &DbfField,
        _mode: ConversionMode,
        _encoding: DbfEncoding,
    ) -> Result<(), String> {
        let s = String::from_utf8_lossy(raw);
        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed.contains('_') {
            self.append_null();
        } else {
            match trimmed.parse::<i64>() {
                Ok(v) => self.append_value(v),
                Err(_) => self.append_null(),
            }
        }
        Ok(())
    }

    fn finish(&mut self) -> ArrayRef {
        Arc::new(self.finish())
    }
}

impl ColumnBuilder for Float64Builder {
    fn append_raw(
        &mut self,
        raw: &[u8],
        _field: &DbfField,
        _mode: ConversionMode,
        _encoding: DbfEncoding,
    ) -> Result<(), String> {
        let s = String::from_utf8_lossy(raw);
        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed.contains('_') {
            self.append_null();
        } else {
            match trimmed.replace(',', ".").parse::<f64>() {
                Ok(v) => self.append_value(v),
                Err(_) => self.append_null(),
            }
        }
        Ok(())
    }

    fn finish(&mut self) -> ArrayRef {
        Arc::new(self.finish())
    }
}

impl ColumnBuilder for BooleanBuilder {
    fn append_raw(
        &mut self,
        raw: &[u8],
        _field: &DbfField,
        _mode: ConversionMode,
        _encoding: DbfEncoding,
    ) -> Result<(), String> {
        if raw.is_empty() {
            self.append_null();
            return Ok(());
        }
        match raw[0] as char {
            'T' | 't' | 'Y' | 'y' | '1' | 'S' | 's' => self.append_value(true),
            'F' | 'f' | 'N' | 'n' | '0' => self.append_value(false),
            _ => self.append_null(),
        }
        Ok(())
    }

    fn finish(&mut self) -> ArrayRef {
        Arc::new(self.finish())
    }
}
