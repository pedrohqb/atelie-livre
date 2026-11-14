//! Pinturas do MacPaint (tipo `PNTG`; ver `docs/formats/pict.md`).
//!
//! O Creative Writer abre arquivos do Mac sem extensão: se não forem `PICT`,
//! tenta MacPaint, que ele mesmo decodifica (balões `idsOpenMacPaintNot` e
//! `idsEmptyMacPaint` do bloco `WHAT 0801`: figura ilegível ou vazia). O formato é uma página de
//! 576 × 720 pixels de 1 bit (preto = 1): 512 bytes de cabeçalho (versão e
//! 38 padrões 8×8) e as 720 linhas de 72 bytes em PackBits. Cópias vindas
//! do Mac costumam ter na frente o cabeçalho MacBinary de 128 bytes.

use crate::winres::Image;
use crate::{ErrorKind, Format, Reader, Result};

pub const WIDTH: usize = 576;
pub const HEIGHT: usize = 720;
pub const ROW_BYTES: usize = WIDTH / 8;
const HEADER: usize = 512;
const MACBINARY: usize = 128;
const PATTERNS: usize = 38;

/// Página do MacPaint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacPaint {
    /// Versão do cabeçalho (0, 2 ou 3).
    pub version: u32,
    /// Os 38 padrões de preenchimento do documento (zeros na versão 0).
    pub patterns: Vec<[u8; 8]>,
    /// 720 linhas de 72 bytes; bit alto à esquerda, 1 = preto.
    pub bits: Vec<u8>,
}

impl MacPaint {
    /// Pixel preto em (`x`, `y`)?
    pub fn pixel(&self, x: usize, y: usize) -> bool {
        x < WIDTH && y < HEIGHT && self.bits[y * ROW_BYTES + x / 8] & (0x80 >> (x % 8)) != 0
    }

    /// Página toda branca (o `idsEmptyMacPaint` do original).
    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&b| b == 0)
    }

    /// Menor retângulo com pixels pretos: (x0, y0, x1, y1), fim exclusivo.
    /// Útil para importar só o desenho, sem a página branca em volta.
    pub fn ink_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let mut b: Option<(usize, usize, usize, usize)> = None;
        for y in 0..HEIGHT {
            let row = &self.bits[y * ROW_BYTES..(y + 1) * ROW_BYTES];
            let Some(first) = row.iter().position(|&v| v != 0) else {
                continue;
            };
            let last = row.iter().rposition(|&v| v != 0).unwrap_or(first);
            let x0 = first * 8 + row[first].leading_zeros() as usize;
            let x1 = last * 8 + 8 - row[last].trailing_zeros() as usize;
            b = Some(match b {
                None => (x0, y, x1, y + 1),
                Some((a0, b0, a1, _)) => (a0.min(x0), b0, a1.max(x1), y + 1),
            });
        }
        b
    }

    /// Imagem RGBA (preto e branco opacos).
    pub fn to_image(&self) -> Image {
        let mut rgba = Vec::with_capacity(WIDTH * HEIGHT * 4);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let v = if self.pixel(x, y) { 0 } else { 255 };
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        Image {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            rgba,
        }
    }
}

/// Tamanho do cabeçalho MacBinary na frente dos dados (0 ou 128).
pub fn macbinary_offset(data: &[u8]) -> usize {
    let is_macbinary = data.len() > MACBINARY + HEADER
        && data[0] == 0
        && (1..=63).contains(&data[1])
        && &data[65..69] == b"PNTG";
    if is_macbinary { MACBINARY } else { 0 }
}

/// Parece um MacPaint? (Versão 0, 2 ou 3 e a primeira linha decodifica.)
/// Use depois de descartar `PICT`, que é mais fácil de reconhecer.
pub fn is_macpaint(data: &[u8]) -> bool {
    let o = macbinary_offset(data);
    let Some(h) = data.get(o..o + 4) else {
        return false;
    };
    let version = u32::from_be_bytes([h[0], h[1], h[2], h[3]]);
    if !matches!(version, 0 | 2 | 3) || data.len() < o + HEADER + 2 {
        return false;
    }
    let mut r = Reader::new(&data[o + HEADER..], Format::MacPaint);
    unpack(&mut r, ROW_BYTES).is_ok()
}

/// Lê uma página do MacPaint (com ou sem MacBinary). As linhas formam um
/// fluxo PackBits contínuo (os arquivos do MacPaint comprimem linha a
/// linha, mas há gravadores que não respeitam a divisa); se o fluxo acabar
/// antes do fim da página, o resto fica branco.
pub fn read(data: &[u8]) -> Result<MacPaint> {
    let o = macbinary_offset(data);
    let mut r = Reader::new(data, Format::MacPaint);
    r.seek(o)?;
    let version = r.u32_be()?;
    if !matches!(version, 0 | 2 | 3) {
        return Err(r.error(ErrorKind::BadMagic {
            expected: "MacPaint version 0, 2 or 3",
        }));
    }
    let mut patterns = Vec::with_capacity(PATTERNS);
    for _ in 0..PATTERNS {
        let mut p = [0u8; 8];
        p.copy_from_slice(r.bytes(8)?);
        patterns.push(p);
    }
    r.seek(o + HEADER)?;
    if r.remaining() == 0 {
        return Err(r.error(ErrorKind::Invalid("no picture data".into())));
    }
    let mut bits = unpack(&mut r, ROW_BYTES * HEIGHT)?;
    bits.resize(ROW_BYTES * HEIGHT, 0);
    Ok(MacPaint {
        version,
        patterns,
        bits,
    })
}

/// PackBits até `size` bytes ou até o fim dos dados (o que vier antes).
/// Erro se a primeira linha não fecha ou se uma sequência passa do fim.
fn unpack(r: &mut Reader<'_>, size: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(size);
    while out.len() < size && r.remaining() > 0 {
        let n = r.u8()? as usize;
        let piece = match n {
            0..=127 => n + 1,
            128 => continue,
            _ => 257 - n,
        };
        if out.len() + piece > size {
            return Err(r.error(ErrorKind::Invalid("PackBits run past the page".into())));
        }
        if n < 128 {
            out.extend_from_slice(r.bytes(piece)?);
        } else {
            let b = r.u8()?;
            out.resize(out.len() + piece, b);
        }
    }
    if out.len() < ROW_BYTES.min(size) {
        return Err(r.error(ErrorKind::Invalid("not even one row".into())));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkbm::pack_bits;

    fn page(f: impl Fn(usize, usize) -> bool) -> Vec<u8> {
        let mut bits = vec![0u8; ROW_BYTES * HEIGHT];
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if f(x, y) {
                    bits[y * ROW_BYTES + x / 8] |= 0x80 >> (x % 8);
                }
            }
        }
        bits
    }

    fn file(version: u32, bits: &[u8]) -> Vec<u8> {
        let mut v = vec![0u8; HEADER];
        v[..4].copy_from_slice(&version.to_be_bytes());
        v[4..12].copy_from_slice(&[0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55]);
        for row in bits.chunks(ROW_BYTES) {
            v.extend(pack_bits(row));
        }
        v
    }

    #[test]
    fn reads_page_row_by_row() {
        let bits = page(|x, y| x == y || (x / 8 + y / 8) % 2 == 1 && x < 64 && y < 64);
        let m = read(&file(2, &bits)).unwrap();
        assert_eq!(m.version, 2);
        assert_eq!(m.patterns.len(), 38);
        assert_eq!(
            m.patterns[0],
            [0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55]
        );
        assert_eq!(m.bits, bits);
        assert!(m.pixel(100, 100) && !m.pixel(101, 100));
        assert!(m.pixel(0, 0) && m.pixel(8, 0) && !m.pixel(1, 0));
        assert!(!m.is_empty());
        assert_eq!(m.ink_bounds(), Some((0, 0, 576, 576)));
        let img = m.to_image();
        assert_eq!((img.width, img.height), (576, 720));
        assert_eq!(&img.rgba[4..8], &[255, 255, 255, 255]);
        assert_eq!(&img.rgba[8 * 4..8 * 4 + 4], &[0, 0, 0, 255]);
    }

    #[test]
    fn macbinary_header_is_skipped() {
        let bits = page(|x, y| (200..210).contains(&x) && (30..33).contains(&y));
        let mut v = vec![0u8; MACBINARY];
        v[1] = 5;
        v[2..7].copy_from_slice(b"teste");
        v[65..69].copy_from_slice(b"PNTG");
        v[69..73].copy_from_slice(b"MPNT");
        v.extend(file(0, &bits));
        assert_eq!(macbinary_offset(&v), 128);
        assert!(is_macpaint(&v));
        let m = read(&v).unwrap();
        assert_eq!(m.ink_bounds(), Some((200, 30, 210, 33)));
    }

    #[test]
    fn stream_across_rows_and_short_data() {
        // fluxo contínuo (um run atravessa linhas) e dados que acabam cedo
        let mut v = vec![0u8; HEADER];
        v.extend([0x81, 0xff, 0x81, 0xff]); // 2 × 128 bytes pretos
        let m = read(&v).unwrap();
        assert!(m.pixel(0, 0) && m.pixel(575, 2) && m.pixel(319, 3));
        assert!(!m.pixel(320, 3) && !m.pixel(0, 700));
    }

    #[test]
    fn empty_page_and_errors() {
        let m = read(&file(0, &vec![0; ROW_BYTES * HEIGHT])).unwrap();
        assert!(m.is_empty());
        assert_eq!(m.ink_bounds(), None);
        // versão inválida, sem dados, run que passa da página
        let mut bad = file(0, &vec![0; ROW_BYTES * HEIGHT]);
        bad[3] = 7;
        assert!(read(&bad).is_err() && !is_macpaint(&bad));
        assert!(read(&[0u8; HEADER]).is_err());
        // 1 byte literal + 405 runs de 128 = 1 byte além da página
        let mut over = vec![0u8; HEADER];
        over.extend([0x00, 0xaa]);
        for _ in 0..405 {
            over.extend([0x81, 0x00]);
        }
        assert!(read(&over).is_err());
    }
}
