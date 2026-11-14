//! Tabela de fontes (chunk `FONT`):
//!
//! ```text
//! u16 tamanho do que segue o cabeçalho de 10 bytes, u16 n, i16 -1, u16 4, u16 0
//! n × u16 offset de cada entrada (a partir do byte 10)
//! entradas: u8 tamanho + nome (Pascal), u16 ftc, u16 família
//! ```

use super::err;
use crate::{ErrorKind, Result, cp1252};

/// Uma fonte da tabela.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Font {
    /// Nome que o GDI devolveu ao criar a fonte (no Wine sem as TrueType do
    /// produto aparece o substituto, como "Liberation Sans"). Case-o com a
    /// `STR#` 0x206 e caia num substituto quando não existir.
    pub name: String,
    /// Número usado pelo sprm de caractere `0x0e`.
    pub ftc: u16,
    /// Família do `LOGFONT` (`lfPitchAndFamily >> 4`): 1 romana, 2 sem
    /// serifa, 4 script, 5 decorativa.
    pub family: u16,
}

impl Font {
    pub fn new(name: &str, ftc: u16, family: u16) -> Font {
        Font {
            name: name.into(),
            ftc,
            family,
        }
    }
}

/// Lê a tabela; `base` é o offset da parte no arquivo (para os erros).
pub fn read(b: &[u8], base: usize) -> Result<Vec<Font>> {
    let bad = |at: usize, m: &str| err(base + at, ErrorKind::Invalid(m.into()));
    if b.len() < 10 {
        return Err(bad(0, "FONT too short"));
    }
    let n = u16::from_le_bytes([b[2], b[3]]) as usize;
    let mut fonts = Vec::with_capacity(n.min(256));
    for k in 0..n {
        let at = 10 + 2 * k;
        let o = b
            .get(at..at + 2)
            .map(|x| 10 + u16::from_le_bytes([x[0], x[1]]) as usize)
            .ok_or_else(|| bad(at, "FONT offsets beyond part"))?;
        let len = *b.get(o).ok_or_else(|| bad(at, "FONT entry beyond part"))? as usize;
        let e = b
            .get(o + 1..o + 1 + len + 4)
            .ok_or_else(|| bad(o, "FONT entry beyond part"))?;
        fonts.push(Font {
            name: cp1252::decode(&e[..len]),
            ftc: u16::from_le_bytes([e[len], e[len + 1]]),
            family: u16::from_le_bytes([e[len + 2], e[len + 3]]),
        });
    }
    Ok(fonts)
}

/// Grava a tabela (nomes em Windows-1252, cortados em 255 bytes).
pub fn write(fonts: &[Font]) -> Vec<u8> {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(fonts.len());
    for f in fonts {
        let name: Vec<u8> = f
            .name
            .chars()
            .map(|c| cp1252::encode_char(c).unwrap_or(b'?'))
            .take(255)
            .collect();
        let mut e = vec![name.len() as u8];
        e.extend_from_slice(&name);
        e.extend_from_slice(&f.ftc.to_le_bytes());
        e.extend_from_slice(&f.family.to_le_bytes());
        entries.push(e);
    }
    let table = 2 * fonts.len();
    let size = table + entries.iter().map(Vec::len).sum::<usize>();
    let mut v = Vec::with_capacity(10 + size);
    v.extend_from_slice(&(size as u16).to_le_bytes());
    v.extend_from_slice(&(fonts.len() as u16).to_le_bytes());
    v.extend_from_slice(&(-1i16).to_le_bytes());
    v.extend_from_slice(&4u16.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    let mut o = table;
    for e in &entries {
        v.extend_from_slice(&(o as u16).to_le_bytes());
        o += e.len();
    }
    for e in &entries {
        v.extend_from_slice(e);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn samp18_fonts() {
        let b = hex(
            "46000400ffff040000000800180024003b000b4c75636964612053616e7300000200075374656e63696c01000500124c756369646120426c61636b6c65747465720200040006496d7061637403000200",
        );
        let f = read(&b, 0).unwrap();
        assert_eq!(
            f,
            [
                Font::new("Lucida Sans", 0, 2),
                Font::new("Stencil", 1, 5),
                Font::new("Lucida Blackletter", 2, 4),
                Font::new("Impact", 3, 2),
            ]
        );
        assert_eq!(write(&f), b);
    }

    #[test]
    fn teste1_font_and_errors() {
        let b = hex("12000100ffff0400000002000b4c75636964612053616e7300000100");
        assert_eq!(read(&b, 0).unwrap(), [Font::new("Lucida Sans", 0, 1)]);
        assert_eq!(write(&read(&b, 0).unwrap()), b);
        for n in 0..b.len() {
            if n < b.len() {
                let _ = read(&b[..n], 0);
            }
        }
        assert!(read(&b[..20], 0).is_err());
        assert!(read(&[0; 4], 0).is_err());
        assert_eq!(write(&[]), hex("00000000ffff04000000"));
    }
}
