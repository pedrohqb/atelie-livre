//! Imagens `PKBM` (ver `docs/formats/pkbm.md`).

use crate::{ErrorKind, Format, Reader, Result, Rgb};

const HEADER_LEN: usize = 42;
/// Limite de segurança: nenhuma imagem real passa de 594×347.
const MAX_PIXELS: usize = 1 << 24;
/// Expansão máxima do PackBits: 2 bytes geram até 128.
const MAX_PACKBITS_RATIO: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    pub width: u16,
    pub height: u16,
    pub depth: u8,
    /// Um índice de paleta por pixel, linha a linha, de cima para baixo.
    pub pixels: Vec<u8>,
    /// Paleta embutida na imagem, quando existe.
    pub palette: Option<Vec<Rgb>>,
}

/// Leitor que aplica a ordem de bytes indicada pela marca do cabeçalho.
struct Ordered<'a> {
    r: Reader<'a>,
    big: bool,
}

impl Ordered<'_> {
    fn u16(&mut self) -> Result<u16> {
        if self.big {
            self.r.u16_be()
        } else {
            self.r.u16_le()
        }
    }
    fn u32(&mut self) -> Result<u32> {
        if self.big {
            self.r.u32_be()
        } else {
            self.r.u32_le()
        }
    }
}

/// A etiqueta aparece como `PKBM` ou, quando gravada como u32 little-endian, `MBKP`.
pub fn is_pkbm(data: &[u8]) -> bool {
    data.len() >= 8
        && matches!(&data[4..8], b"PKBM" | b"MBKP")
        && matches!(&data[0..2], [0, 1] | [1, 0])
}

pub fn decode(data: &[u8]) -> Result<Bitmap> {
    let bad_magic =
        || Reader::new(data, Format::Pkbm).error(ErrorKind::BadMagic { expected: "PKBM" });
    if !is_pkbm(data) {
        return Err(bad_magic());
    }
    let mut o = Ordered {
        r: Reader::new(data, Format::Pkbm),
        big: data[0] == 0,
    };
    o.r.seek(10)?;
    let compressed = match o.u16()? {
        0 => false,
        1 => true,
        n => {
            return Err(o
                .r
                .error(ErrorKind::Unsupported(format!("compression {n}"))));
        }
    };
    o.r.seek(22)?;
    let width = o.u16()?;
    let height = o.u16()?;
    let depth = match o.u16()? {
        d @ (1 | 2 | 4 | 8) => d as u8,
        d => return Err(o.r.error(ErrorKind::Unsupported(format!("depth {d}")))),
    };
    let pal_count = o.u16()? as usize;
    let pal_offset = o.u16()? as usize;
    o.r.seek(38)?;
    let data_offset = o.u32()? as usize;
    if data_offset < HEADER_LEN || data_offset > data.len() {
        return Err(o
            .r
            .error(ErrorKind::Invalid(format!("pixel offset {data_offset}"))));
    }

    let palette = if pal_count == 0 {
        None
    } else {
        o.r.seek(pal_offset)?;
        let mut pal = Vec::with_capacity(pal_count.min(256));
        for _ in 0..pal_count {
            let (r, g, b) = (o.u16()?, o.u16()?, o.u16()?);
            pal.push(Rgb {
                r: (r >> 8) as u8,
                g: (g >> 8) as u8,
                b: (b >> 8) as u8,
            });
        }
        Some(pal)
    };

    let (w, h) = (width as usize, height as usize);
    if w * h > MAX_PIXELS {
        return Err(o
            .r
            .error(ErrorKind::Unsupported(format!("image {w}x{h} too large"))));
    }
    let row_bytes = (w * depth as usize).div_ceil(8).next_multiple_of(2);
    let size = row_bytes * h;
    o.r.seek(data_offset)?;
    let raw = if compressed {
        unpack_bits(&mut o.r, size)?
    } else {
        o.r.bytes(size)?.to_vec()
    };

    let mask = ((1u16 << depth) - 1) as u8;
    let mut pixels = Vec::with_capacity(w * h);
    for row in raw.chunks_exact(row_bytes.max(1)).take(h) {
        for x in 0..w {
            let bit = x * depth as usize;
            let shift = 8 - depth as usize - bit % 8;
            pixels.push((row[bit / 8] >> shift) & mask);
        }
    }
    Ok(Bitmap {
        width,
        height,
        depth,
        pixels,
        palette,
    })
}

/// Fluxo PackBits contínuo até completar `size` bytes.
fn unpack_bits(r: &mut Reader<'_>, size: usize) -> Result<Vec<u8>> {
    if size > r.remaining().saturating_mul(MAX_PACKBITS_RATIO) {
        return Err(r.error(ErrorKind::Invalid(format!(
            "{size} bytes cannot come from {} compressed bytes",
            r.remaining()
        ))));
    }
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        let n = r.u8()? as usize;
        let piece = match n {
            0..=127 => n + 1,
            128 => continue,
            _ => 257 - n,
        };
        if out.len() + piece > size {
            return Err(r.error(ErrorKind::Invalid("PackBits run exceeds image size".into())));
        }
        if n < 128 {
            out.extend_from_slice(r.bytes(piece)?);
        } else {
            let b = r.u8()?;
            out.resize(out.len() + piece, b);
        }
    }
    Ok(out)
}

/// Paleta padrão quando a imagem não traz a sua: 1 bit branco/preto,
/// 4 bits a `LOGPALETTE` de 16 cores, 8 bits a de 256.
pub fn default_palette_for(depth: u8, pal16: &[Rgb], pal256: &[Rgb]) -> Vec<Rgb> {
    match depth {
        1 => vec![
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            Rgb { r: 0, g: 0, b: 0 },
        ],
        2 | 4 => pal16.to_vec(),
        _ => pal256.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;

    #[derive(Clone, Copy)]
    enum Order {
        Big,
        Little,
    }

    /// Monta um PKBM; `stream` são os bytes de pixel já no formato final (cru ou PackBits).
    fn build(
        order: Order,
        (w, h, depth): (u16, u16, u16),
        compressed: bool,
        palette: &[(u16, u16, u16)],
        stream: &[u8],
    ) -> Vec<u8> {
        let p16 = |v: u16| match order {
            Order::Big => v.to_be_bytes(),
            Order::Little => v.to_le_bytes(),
        };
        let p32 = |v: u32| match order {
            Order::Big => v.to_be_bytes(),
            Order::Little => v.to_le_bytes(),
        };
        let pal_bytes = palette.len() * 6;
        let data_off = 42 + pal_bytes as u32;
        let mut v = Vec::new();
        v.extend_from_slice(&p16(1));
        v.extend_from_slice(&p16(0));
        v.extend_from_slice(b"PKBM");
        v.extend_from_slice(&p16(0));
        v.extend_from_slice(&p16(compressed as u16));
        v.extend_from_slice(&p16(1));
        v.extend_from_slice(&p16(42));
        v.extend_from_slice(&p32(0));
        v.extend_from_slice(&[0, 0]);
        v.extend_from_slice(&p16(w));
        v.extend_from_slice(&p16(h));
        v.extend_from_slice(&p16(depth));
        v.extend_from_slice(&p16(palette.len() as u16));
        v.extend_from_slice(&p16(if palette.is_empty() { 0 } else { 42 }));
        v.extend_from_slice(&[0; 6]);
        v.extend_from_slice(&p32(data_off));
        assert_eq!(v.len(), 42);
        for &(r, g, b) in palette {
            v.extend_from_slice(&p16(r));
            v.extend_from_slice(&p16(g));
            v.extend_from_slice(&p16(b));
        }
        v.extend_from_slice(stream);
        v
    }

    #[test]
    fn decodes_raw_1bit_big_endian() {
        // 8×2, 1 byte por linha arredondado para 2
        let d = build(
            Order::Big,
            (8, 2, 1),
            false,
            &[],
            &[0b1010_0000, 0, 0xff, 0],
        );
        let b = decode(&d).unwrap();
        assert_eq!((b.width, b.height, b.depth), (8, 2, 1));
        assert_eq!(b.pixels, [1, 0, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1]);
        assert_eq!(b.palette, None);
    }

    #[test]
    fn decodes_packed_4bit_little_endian() {
        // 3×2: linhas [12 30] e [ff f0], como um literal PackBits de 4 bytes
        let d = build(
            Order::Little,
            (3, 2, 4),
            true,
            &[],
            &[0x03, 0x12, 0x30, 0xff, 0xf0],
        );
        assert_eq!(decode(&d).unwrap().pixels, [1, 2, 3, 15, 15, 15]);
    }

    #[test]
    fn decodes_packbits_run() {
        // 4×1 em 8 bits: 0xfd repete o próximo byte 4 vezes
        let d = build(Order::Big, (4, 1, 8), true, &[], &[0xfd, 7]);
        assert_eq!(decode(&d).unwrap().pixels, [7, 7, 7, 7]);
    }

    #[test]
    fn packbits_noop_128_is_skipped() {
        let d = build(Order::Big, (2, 1, 8), true, &[], &[0x80, 0x01, 5, 6]);
        assert_eq!(decode(&d).unwrap().pixels, [5, 6]);
    }

    #[test]
    fn embedded_palette_uses_high_byte() {
        let pal = [(0xff00, 0, 0), (0, 0x8000, 0x10ff)];
        let d = build(Order::Little, (2, 1, 8), false, &pal, &[1, 0]);
        let b = decode(&d).unwrap();
        assert_eq!(b.pixels, [1, 0]);
        assert_eq!(
            b.palette.unwrap(),
            [
                Rgb { r: 255, g: 0, b: 0 },
                Rgb {
                    r: 0,
                    g: 128,
                    b: 16
                }
            ]
        );
    }

    #[test]
    fn run_exceeding_image_is_error() {
        let d = build(Order::Big, (2, 1, 8), true, &[], &[0xfd, 7]);
        assert!(matches!(
            decode(&d).unwrap_err().kind,
            ErrorKind::Invalid(_)
        ));
    }

    #[test]
    fn truncated_stream_is_error() {
        let d = build(Order::Big, (8, 1, 8), true, &[], &[0x07, 1, 2, 3]);
        assert!(decode(&d).is_err());
        let d = build(Order::Big, (8, 1, 8), false, &[], &[1, 2, 3]);
        assert!(decode(&d).is_err());
    }

    #[test]
    fn huge_declared_size_is_error_without_allocating() {
        let d = build(Order::Big, (65535, 65535, 8), true, &[], &[0xff; 18]);
        assert!(decode(&d).is_err());
        let d = build(Order::Big, (65535, 65535, 8), false, &[], &[0; 18]);
        assert!(decode(&d).is_err());
    }

    #[test]
    fn rejects_bad_byte_order_mark_and_depth() {
        let mut d = build(Order::Big, (2, 1, 8), false, &[], &[0, 0]);
        d[1] = 2;
        assert_eq!(
            decode(&d).unwrap_err().kind,
            ErrorKind::BadMagic { expected: "PKBM" }
        );
        let d = build(Order::Big, (2, 1, 3), false, &[], &[0, 0]);
        assert!(matches!(
            decode(&d).unwrap_err().kind,
            ErrorKind::Unsupported(_)
        ));
    }

    #[test]
    fn accepts_byte_swapped_tag() {
        // nas bibliotecas CHNK a etiqueta aparece como "MBKP" (u32 little-endian)
        let mut d = build(Order::Little, (2, 1, 8), false, &[], &[3, 4]);
        d[4..8].copy_from_slice(b"MBKP");
        assert!(is_pkbm(&d));
        assert_eq!(decode(&d).unwrap().pixels, [3, 4]);
    }

    #[test]
    fn detects_signature() {
        assert!(is_pkbm(&build(
            Order::Little,
            (2, 1, 8),
            false,
            &[],
            &[0, 0]
        )));
        assert!(!is_pkbm(b"\x00\x01\x00\x00PKBX"));
        assert!(!is_pkbm(b"PKBM"));
    }

    #[test]
    fn default_palettes() {
        let p16: Vec<Rgb> = (0..16).map(|i| Rgb { r: i, g: 0, b: 0 }).collect();
        let p256: Vec<Rgb> = (0..=255).map(|i| Rgb { r: 0, g: i, b: 0 }).collect();
        let mono = default_palette_for(1, &p16, &p256);
        assert_eq!(
            mono,
            [
                Rgb {
                    r: 255,
                    g: 255,
                    b: 255
                },
                Rgb { r: 0, g: 0, b: 0 }
            ]
        );
        assert_eq!(default_palette_for(4, &p16, &p256), p16);
        assert_eq!(default_palette_for(8, &p16, &p256), p256);
    }

    #[test]
    fn every_truncation_and_corruption_is_handled() {
        let d = build(
            Order::Little,
            (3, 2, 4),
            true,
            &[(1, 2, 3)],
            &[0x03, 0x12, 0x30, 0xff, 0xf0],
        );
        for n in 0..d.len() {
            let _ = decode(&d[..n]);
        }
        for i in 0..d.len() {
            for x in [0x00, 0xff, 0x7f, 0x80] {
                let mut c = d.clone();
                c[i] = x;
                let _ = decode(&c);
            }
        }
    }
}
