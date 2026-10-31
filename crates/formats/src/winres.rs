//! Recursos padrão do Windows: bitmaps DIB, ícones, cursores e tabelas de strings.

use crate::{ErrorKind, Format, Reader, Result, cp1252};

const MAX_PIXELS: usize = 1 << 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// RGBA de 8 bits por canal, linhas de cima para baixo.
    pub rgba: Vec<u8>,
}

pub fn decode_bitmap(data: &[u8]) -> Result<Image> {
    decode_dib(data, false)
}

/// Ícone: DIB com altura dobrada (imagem XOR seguida da máscara AND de 1 bit).
pub fn decode_icon(data: &[u8]) -> Result<Image> {
    decode_dib(data, true)
}

/// Cursor: hotspot (`u16` x, `u16` y) seguido de um ícone.
pub fn decode_cursor(data: &[u8]) -> Result<(Image, (u16, u16))> {
    let mut r = Reader::new(data, Format::WinRes);
    let hotspot = (r.u16_le()?, r.u16_le()?);
    let img = decode_dib(&data[4..], true).map_err(|mut e| {
        e.offset += 4;
        e
    })?;
    Ok((img, hotspot))
}

fn decode_dib(data: &[u8], icon: bool) -> Result<Image> {
    let mut r = Reader::new(data, Format::WinRes);
    let header = r.u32_le()? as usize;
    let (w, h, bpp, compression, colors_used, entry_len) = match header {
        12 => {
            let w = r.u16_le()? as i64;
            let h = r.u16_le()? as i64;
            r.skip(2)?;
            (w, h, r.u16_le()?, 0, 0, 3)
        }
        40.. => {
            let w = r.u32_le()? as i32 as i64;
            let h = r.u32_le()? as i32 as i64;
            r.skip(2)?;
            let bpp = r.u16_le()?;
            let compression = r.u32_le()?;
            r.skip(12)?;
            let used = r.u32_le()? as usize;
            (w, h, bpp, compression, used, 4)
        }
        n => return Err(r.error(ErrorKind::Unsupported(format!("DIB header size {n}")))),
    };
    r.seek(header)?;
    if compression != 0 {
        return Err(r.error(ErrorKind::Unsupported(format!(
            "DIB compression {compression}"
        ))));
    }
    if !matches!(bpp, 1 | 4 | 8 | 24) {
        return Err(r.error(ErrorKind::Unsupported(format!("{bpp} bits per pixel"))));
    }
    let top_down = h < 0;
    let mut height = h.unsigned_abs() as usize;
    if icon {
        if top_down || height % 2 != 0 {
            return Err(r.error(ErrorKind::Invalid(format!("icon height {h}"))));
        }
        height /= 2;
    }
    if w <= 0 || height == 0 || (w as usize).saturating_mul(height) > MAX_PIXELS {
        return Err(r.error(ErrorKind::Invalid(format!("DIB size {w}x{h}"))));
    }
    let width = w as usize;

    let mut palette = Vec::new();
    if bpp <= 8 {
        let max = 1usize << bpp;
        let count = if colors_used == 0 {
            max
        } else {
            colors_used.min(max)
        };
        for _ in 0..count {
            let e = r.bytes(entry_len)?;
            palette.push([e[2], e[1], e[0]]);
        }
    }

    let stride = (width * bpp as usize).div_ceil(32) * 4;
    let mask_stride = width.div_ceil(32) * 4;
    let needed = stride * height + if icon { mask_stride * height } else { 0 };
    if needed > r.remaining() {
        return Err(r.error(ErrorKind::UnexpectedEof {
            wanted: needed,
            available: r.remaining(),
        }));
    }
    let bits = r.bytes(stride * height)?;
    let mask = if icon {
        Some(r.bytes(mask_stride * height)?)
    } else {
        None
    };

    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let src = if top_down { y } else { height - 1 - y };
        let row = &bits[src * stride..(src + 1) * stride];
        for x in 0..width {
            let rgb = if bpp == 24 {
                [row[x * 3 + 2], row[x * 3 + 1], row[x * 3]]
            } else {
                let bit = x * bpp as usize;
                let idx =
                    (row[bit / 8] >> (8 - bpp as usize - bit % 8)) & ((1u16 << bpp) - 1) as u8;
                *palette.get(idx as usize).ok_or_else(|| {
                    r.error(ErrorKind::Invalid(format!(
                        "color index {idx} outside palette"
                    )))
                })?
            };
            let transparent =
                mask.is_some_and(|m| m[src * mask_stride + x / 8] & (0x80 >> (x % 8)) != 0);
            rgba.extend_from_slice(&rgb);
            rgba.push(if transparent { 0 } else { 255 });
        }
    }
    Ok(Image {
        width: width as u32,
        height: height as u32,
        rgba,
    })
}

/// Tabela de strings do Win16: 16 itens por bloco (`u8` tamanho + Windows-1252);
/// o item `i` do bloco `b` tem id `(b − 1) × 16 + i`. Itens vazios são omitidos.
pub fn parse_string_table(block: u16, data: &[u8]) -> Result<Vec<(u16, String)>> {
    let mut r = Reader::new(data, Format::WinRes);
    if block == 0 {
        return Err(r.error(ErrorKind::Invalid("string table block 0".into())));
    }
    let base = (block - 1).wrapping_mul(16);
    let mut out = Vec::new();
    for i in 0..16u16 {
        if r.remaining() == 0 {
            break;
        }
        let n = r.u8()? as usize;
        if n > 0 {
            out.push((base.wrapping_add(i), cp1252::decode(r.bytes(n)?)));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;

    /// DIB com BITMAPINFOHEADER; `palette` em (r, g, b); `rows` já alinhadas, de baixo para cima.
    fn dib(
        w: i32,
        h: i32,
        bpp: u16,
        compression: u32,
        palette: &[(u8, u8, u8)],
        rows: &[u8],
    ) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&40u32.to_le_bytes());
        v.extend_from_slice(&w.to_le_bytes());
        v.extend_from_slice(&h.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&bpp.to_le_bytes());
        v.extend_from_slice(&compression.to_le_bytes());
        v.extend_from_slice(&[0; 12]);
        v.extend_from_slice(&(palette.len() as u32).to_le_bytes());
        v.extend_from_slice(&[0; 4]);
        for &(r, g, b) in palette {
            v.extend_from_slice(&[b, g, r, 0]);
        }
        v.extend_from_slice(rows);
        v
    }

    const BW: [(u8, u8, u8); 2] = [(0, 0, 0), (255, 255, 255)];

    fn icon_2x2() -> Vec<u8> {
        // XOR de baixo para cima: "10", "01"; AND: "00", "10" (canto superior esquerdo transparente)
        let rows = [0x80, 0, 0, 0, 0x40, 0, 0, 0, 0x00, 0, 0, 0, 0x80, 0, 0, 0];
        dib(2, 4, 1, 0, &BW, &rows)
    }

    #[test]
    fn icon_2x2_1bit_with_mask() {
        let img = decode_icon(&icon_2x2()).unwrap();
        assert_eq!((img.width, img.height), (2, 2));
        assert_eq!(
            img.rgba,
            [
                0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255
            ]
        );
    }

    #[test]
    fn bitmap_4bit_bottom_up() {
        let pal: Vec<(u8, u8, u8)> = (0..16).map(|i| (i * 10, i * 10 + 1, i * 10 + 2)).collect();
        let rows = [0x12, 0, 0, 0, 0x30, 0, 0, 0];
        let img = decode_bitmap(&dib(2, 2, 4, 0, &pal, &rows)).unwrap();
        assert_eq!(
            img.rgba,
            [
                30, 31, 32, 255, 0, 1, 2, 255, 10, 11, 12, 255, 20, 21, 22, 255
            ]
        );
    }

    #[test]
    fn bitmap_core_header_1bit() {
        let mut v = Vec::new();
        v.extend_from_slice(&12u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&[0, 0, 255, 255, 0, 0]); // RGBTRIPLE em b, g, r: vermelho, azul
        v.extend_from_slice(&[0x80, 0, 0, 0]);
        assert_eq!(decode_bitmap(&v).unwrap().rgba, [0, 0, 255, 255]);
    }

    #[test]
    fn bitmap_24bit_bgr() {
        let rows = [3, 2, 1, 0];
        assert_eq!(
            decode_bitmap(&dib(1, 1, 24, 0, &[], &rows)).unwrap().rgba,
            [1, 2, 3, 255]
        );
    }

    #[test]
    fn cursor_hotspot() {
        let mut d = vec![3, 0, 5, 0];
        d.extend_from_slice(&icon_2x2());
        let (img, hs) = decode_cursor(&d).unwrap();
        assert_eq!(hs, (3, 5));
        assert_eq!((img.width, img.height), (2, 2));
    }

    #[test]
    fn compressed_dib_is_unsupported() {
        let d = dib(2, 2, 4, 2, &[(0, 0, 0)], &[0; 8]);
        assert!(matches!(
            decode_bitmap(&d).unwrap_err().kind,
            ErrorKind::Unsupported(_)
        ));
    }

    #[test]
    fn huge_dimensions_are_error() {
        assert!(decode_bitmap(&dib(60000, 60000, 1, 0, &BW, &[0; 8])).is_err());
        assert!(decode_bitmap(&dib(-1, 1, 1, 0, &BW, &[0; 8])).is_err());
    }

    #[test]
    fn string_table_ids_and_accents() {
        let mut d = vec![0, 4, 0x61, 0xe7, 0xe3, 0x6f];
        d.extend_from_slice(&[0; 14]);
        assert_eq!(
            parse_string_table(2, &d).unwrap(),
            [(17, "ação".to_string())]
        );
    }

    #[test]
    fn string_table_tolerates_padding_and_short_tables() {
        let d = [0, 4, b'E', b'r', b'r', b'o'];
        assert_eq!(
            parse_string_table(1, &d).unwrap(),
            [(1, "Erro".to_string())]
        );
        assert!(parse_string_table(1, &[0, 9, b'x']).is_err());
        assert!(parse_string_table(0, &[0]).is_err());
    }

    #[test]
    fn every_truncation_and_corruption_is_handled() {
        let mut cur = vec![3, 0, 5, 0];
        cur.extend_from_slice(&icon_2x2());
        for n in 0..cur.len() {
            let _ = decode_cursor(&cur[..n]);
            let _ = decode_icon(&cur[4..n.max(4)]);
            let _ = decode_bitmap(&cur[4..n.max(4)]);
        }
        for i in 0..cur.len() {
            for x in [0x00, 0xff, 0x7f, 0x80] {
                let mut c = cur.clone();
                c[i] = x;
                let _ = decode_cursor(&c);
                let _ = decode_bitmap(&c[4..]);
            }
        }
    }
}
