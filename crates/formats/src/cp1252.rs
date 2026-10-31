//! Decodificação de texto Windows-1252, a página de código dos textos em português.

const HIGH: [char; 32] = [
    '\u{20ac}', '\u{fffd}', '\u{201a}', '\u{0192}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02c6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{fffd}', '\u{017d}', '\u{fffd}',
    '\u{fffd}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02dc}', '\u{2122}', '\u{0161}', '\u{203a}', '\u{0153}', '\u{fffd}', '\u{017e}', '\u{0178}',
];

/// Converte bytes Windows-1252 em texto; bytes indefinidos viram U+FFFD.
pub fn decode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9f => HIGH[(b - 0x80) as usize],
            _ => b as char,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp1252_portuguese() {
        assert_eq!(decode(&[0x6e, 0xe3, 0x6f]), "não");
        assert_eq!(decode(&[0xe7, 0x93, 0x41, 0x94]), "ç“A”");
    }

    #[test]
    fn cp1252_special_range() {
        assert_eq!(decode(&[0x80, 0x85, 0x96, 0x99]), "€…–™");
        assert_eq!(
            decode(&[0x81, 0x8d, 0x8f, 0x90, 0x9d]),
            "\u{fffd}".repeat(5)
        );
    }
}
