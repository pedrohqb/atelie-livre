//! Fontes de bitmap do Windows (recurso `FONT` de um `.FON`), versões 2.0 e 3.0.
//! O Fine Artist usa a "Pablo Graphos" de `SHARED/STUFF/PABLO.FON` nos balões e
//! nas barras.
//!
//! Cabeçalho: `dfPixHeight` em 0x58, `dfAscent` em 0x4A, `dfFirstChar`,
//! `dfLastChar`, `dfDefaultChar` em 0x5F..0x61. A tabela de caracteres começa
//! em 0x76 (v2: `u16` largura + `u16` offset) ou 0x94 (v3: `u16` + `u32`). Cada
//! glifo é guardado em colunas de 8 pixels: todas as linhas da coluna 0, depois
//! todas as da coluna 1, e assim por diante; bit mais alto à esquerda.

use crate::{ErrorKind, Format, Reader, Result, cp1252};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph {
    pub width: u32,
    /// Linha a linha, `width * altura` valores (verdadeiro = tinta).
    pub bits: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    pub name: String,
    pub points: u16,
    pub height: u32,
    pub ascent: u32,
    first: u8,
    default: u8,
    glyphs: Vec<Glyph>,
}

impl Font {
    /// Glifo do byte Windows-1252 `c` (o padrão da fonte se faltar).
    pub fn glyph(&self, c: u8) -> &Glyph {
        let idx = |b: u8| b.checked_sub(self.first).map(|i| i as usize);
        idx(c)
            .and_then(|i| self.glyphs.get(i))
            .or_else(|| {
                idx(self.first.saturating_add(self.default)).and_then(|i| self.glyphs.get(i))
            })
            .or_else(|| self.glyphs.first())
            .expect("fonte sem glifos")
    }

    fn byte(c: char) -> u8 {
        cp1252::encode_char(c).unwrap_or(b'?')
    }

    /// Largura do texto em pixels.
    pub fn width(&self, text: &str) -> u32 {
        text.chars().map(|c| self.glyph(Self::byte(c)).width).sum()
    }

    /// Glifos do texto, em ordem.
    pub fn glyphs<'a>(&'a self, text: &'a str) -> impl Iterator<Item = &'a Glyph> + 'a {
        text.chars().map(|c| self.glyph(Self::byte(c)))
    }
}

pub fn parse(data: &[u8]) -> Result<Font> {
    let mut r = Reader::new(data, Format::WinRes);
    let version = r.u16_le()?;
    let entry = match version {
        0x200 => 4,
        0x300 => 6,
        v => return Err(r.error(ErrorKind::Unsupported(format!("FNT versão {v:#x}")))),
    };
    let u16_at = |o: usize| -> Result<u16> {
        let mut r = Reader::new(data, Format::WinRes);
        r.seek(o)?;
        r.u16_le()
    };
    let u8_at = |o: usize| -> Result<u8> {
        let mut r = Reader::new(data, Format::WinRes);
        r.seek(o)?;
        r.u8()
    };
    let points = u16_at(0x44)?;
    let ascent = u16_at(0x4a)? as u32;
    let height = u16_at(0x58)? as u32;
    let (first, last, default) = (u8_at(0x5f)?, u8_at(0x60)?, u8_at(0x61)?);
    if last < first || height == 0 || height > 256 {
        return Err(r.error(ErrorKind::Invalid("cabeçalho de fonte".into())));
    }
    let face = {
        let mut r = Reader::new(data, Format::WinRes);
        r.seek(0x69)?;
        let off = r.u32_le()? as usize;
        let end = data.get(off..).and_then(|d| d.iter().position(|&b| b == 0));
        end.map(|e| cp1252::decode(&data[off..off + e]))
            .unwrap_or_default()
    };
    let table = if version == 0x200 { 0x76 } else { 0x94 };
    let mut glyphs = Vec::new();
    let mut t = Reader::new(data, Format::WinRes);
    t.seek(table)?;
    for _ in first..=last {
        let width = t.u16_le()? as u32;
        let offset = if entry == 4 {
            t.u16_le()? as usize
        } else {
            t.u32_le()? as usize
        };
        if width > 256 {
            return Err(t.error(ErrorKind::Invalid(format!("largura de glifo {width}"))));
        }
        let cols = width.div_ceil(8) as usize;
        let need = cols * height as usize;
        let raw = data.get(offset..offset + need).ok_or_else(|| {
            t.error(ErrorKind::UnexpectedEof {
                wanted: need,
                available: data.len().saturating_sub(offset),
            })
        })?;
        let mut bits = vec![false; (width * height) as usize];
        for y in 0..height as usize {
            for x in 0..width as usize {
                let b = raw[(x / 8) * height as usize + y];
                bits[y * width as usize + x] = (b >> (7 - x % 8)) & 1 == 1;
            }
        }
        glyphs.push(Glyph { width, bits });
    }
    Ok(Font {
        name: face,
        points,
        height,
        ascent,
        first,
        default,
        glyphs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fonte v2 mínima: caracteres 'A' (0x41) e 'B', 2 linhas de altura.
    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; 0x76 + 3 * 4];
        d[0..2].copy_from_slice(&0x200u16.to_le_bytes());
        d[0x44..0x46].copy_from_slice(&8u16.to_le_bytes());
        d[0x4a..0x4c].copy_from_slice(&2u16.to_le_bytes());
        d[0x58..0x5a].copy_from_slice(&2u16.to_le_bytes());
        d[0x5f] = 0x41;
        d[0x60] = 0x42;
        d[0x61] = 0;
        let bits = d.len() as u16;
        // 'A': largura 3, linhas 101 / 010; 'B': largura 9 (duas colunas)
        d[0x76..0x78].copy_from_slice(&3u16.to_le_bytes());
        d[0x78..0x7a].copy_from_slice(&bits.to_le_bytes());
        d[0x7a..0x7c].copy_from_slice(&9u16.to_le_bytes());
        d[0x7c..0x7e].copy_from_slice(&(bits + 2).to_le_bytes());
        d.extend_from_slice(&[0b1010_0000, 0b0100_0000]);
        d.extend_from_slice(&[0xff, 0x00, 0b1000_0000, 0x00]);
        d
    }

    #[test]
    fn parses_glyphs_column_major() {
        let f = parse(&sample()).unwrap();
        assert_eq!(f.height, 2);
        let a = f.glyph(b'A');
        assert_eq!(a.width, 3);
        assert_eq!(a.bits, vec![true, false, true, false, true, false]);
        let b = f.glyph(b'B');
        assert_eq!(b.width, 9);
        assert!(b.bits[8]); // nona coluna, primeira linha
        assert!(!b.bits[9 + 8]);
        // fora da faixa: glifo padrão (o primeiro)
        assert_eq!(f.glyph(b'z').width, 3);
        assert_eq!(f.width("AB?"), 3 + 9 + 3);
    }

    #[test]
    fn rejects_bad_data() {
        assert!(parse(&[0, 1]).is_err());
        let mut d = sample();
        d.truncate(d.len() - 3);
        assert!(parse(&d).is_err());
    }
}
