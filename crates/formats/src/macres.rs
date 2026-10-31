//! Recursos de estilo Mac guardados nos executáveis: `STR#`, `STR`, `PAT#`.

use crate::{ErrorKind, Format, Reader, Result, cp1252};

/// `u16_le` contagem; cada item: `u8` tamanho, bytes Windows-1252, um byte 0x00.
pub fn parse_str_list(data: &[u8]) -> Result<Vec<String>> {
    let mut r = Reader::new(data, Format::MacRes);
    let count = r.u16_le()? as usize;
    let mut out = Vec::with_capacity(count.min(data.len()));
    for _ in 0..count {
        let n = r.u8()? as usize;
        let s = cp1252::decode(r.bytes(n)?);
        if r.u8()? != 0 {
            return Err(r.error(ErrorKind::Invalid(
                "STR# item without NUL terminator".into(),
            )));
        }
        out.push(s);
    }
    Ok(out)
}

/// Texto até o primeiro 0x00 (ou o recurso inteiro, se não houver).
pub fn parse_cstring(data: &[u8]) -> Result<String> {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    Ok(cp1252::decode(&data[..end]))
}

/// `u16_le` contagem; cada padrão 8×8 tem 8 linhas de `u16_le`, com os bits no byte baixo.
pub fn parse_pattern_list(data: &[u8]) -> Result<Vec<[u8; 8]>> {
    let mut r = Reader::new(data, Format::MacRes);
    let count = r.u16_le()? as usize;
    let bytes = r.bytes(count.saturating_mul(16))?;
    Ok(bytes
        .chunks_exact(16)
        .map(|c| std::array::from_fn(|row| c[row * 2]))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::parse_logpalette;

    #[test]
    fn str_list_layout() {
        let d = [2, 0, 3, b'a', b'b', b'c', 0, 1, 0xe3, 0, 0xaa];
        assert_eq!(parse_str_list(&d).unwrap(), ["abc", "ã"]);
    }

    #[test]
    fn str_list_count_beyond_data_is_error() {
        let d = [5, 0, 1, b'a', 0];
        assert!(parse_str_list(&d).is_err());
    }

    #[test]
    fn str_list_missing_terminator_is_invalid() {
        let d = [1, 0, 1, b'a', b'x'];
        assert!(parse_str_list(&d).is_err());
    }

    #[test]
    fn cstring_stops_at_nul() {
        assert_eq!(parse_cstring(b"Eu n\xe3o\0lixo").unwrap(), "Eu não");
        assert_eq!(parse_cstring(b"sem fim").unwrap(), "sem fim");
    }

    #[test]
    fn pattern_list() {
        // cada linha é uma palavra de 16 bits little-endian; os bits ficam no byte baixo
        let mut d = vec![2, 0];
        d.extend_from_slice(&[0xff, 0x00].repeat(8));
        for row in [0x55u8, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa] {
            d.extend_from_slice(&[row, 0x00]);
        }
        d.push(0);
        assert_eq!(
            parse_pattern_list(&d).unwrap(),
            [[0xff; 8], [0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa]]
        );
        assert!(parse_pattern_list(&[1, 0, 1, 2]).is_err());
    }

    #[test]
    fn every_truncation_is_handled() {
        let d = [2, 0, 3, b'a', b'b', b'c', 0, 1, 0xe3, 0];
        for n in 0..d.len() {
            let _ = parse_str_list(&d[..n]);
            let _ = parse_pattern_list(&d[..n]);
            let _ = parse_logpalette(&d[..n]);
        }
    }
}
