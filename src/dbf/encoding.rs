use encoding_rs::WINDOWS_1252;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DbfEncoding {
    #[default]
    Iso8859_1, // ISO-8859-1 / Windows-1252 (Padrão oficial DATASUS no DuckDB)
    Cp850,     // CP850 / IBM850 (dBase III/IV MS-DOS antigo)
    Utf8,      // UTF-8
}

impl DbfEncoding {
    pub fn display_name(&self) -> &str {
        match self {
            DbfEncoding::Iso8859_1 => "ISO-8859-1 / Windows-1252 (Padrão DATASUS / DuckDB)",
            DbfEncoding::Cp850 => "CP850 / IBM850 (MS-DOS legado)",
            DbfEncoding::Utf8 => "UTF-8",
        }
    }

    pub fn decode(&self, bytes: &[u8]) -> String {
        match self {
            DbfEncoding::Iso8859_1 => {
                let (cow, _, _) = WINDOWS_1252.decode(bytes);
                cow.into_owned()
            }
            DbfEncoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            DbfEncoding::Cp850 => decode_cp850(bytes),
        }
    }
}

// Tabela de decodificação CP850 (OEM Code Page 850 - Latim 1 DOS)
const CP850_MAP: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', 'ø', '£', 'Ø', '×', 'ƒ',
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '®', '¬', '½', '¼', '¡', '«', '»',
    '░', '▒', '▓', '│', '┤', 'Á', 'Â', 'À', '©', '╣', '║', '╗', '╝', '¢', '¥', '┐',
    '└', '┴', '┬', '├', '─', '┼', 'ã', 'Ã', '╚', '╔', '╩', '╦', '╠', '═', '╬', '¤',
    'ð', 'Ð', 'Ê', 'Ë', 'È', 'ı', 'Í', 'Î', 'Ï', '┘', '┌', '█', '▄', '¦', 'Ì', '▀',
    'Ó', 'ß', 'Ô', 'Ò', 'õ', 'Õ', 'µ', 'þ', 'Þ', 'Ú', 'Û', 'Ù', 'ý', 'Ý', '¯', '´',
    '\u{00AD}', '±', '‗', '¾', '¶', '§', '÷', '¸', '°', '¨', '·', '¹', '³', '²', '■', ' ',
];

fn decode_cp850(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        if b < 0x80 {
            out.push(b as char);
        } else {
            out.push(CP850_MAP[(b - 0x80) as usize]);
        }
    }
    out
}
