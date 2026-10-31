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

/// Bytes 20 e 21 do cabeçalho, de significado desconhecido. Os desenhos
/// salvos trazem `(8, 0)` nas miniaturas (`THMB`) e `(40, 4)` ou `(8, 4)` na
/// pintura principal (`PKBM 0`); as imagens dos `.PIC` trazem `(1, 0)` em
/// 1 bit e `(0, 0)` em 4 bits. O leitor ignora os dois.
pub type UnknownBytes = (u8, u8);

/// Bytes 20/21 gravados por [`encode`]: os das miniaturas dos desenhos salvos.
pub const DEFAULT_UNKNOWN: UnknownBytes = (8, 0);

/// Grava um `PKBM` little-endian (a ordem dos documentos e bibliotecas do
/// Windows) com compressão PackBits, nas profundidades 1, 2, 4 e 8 bits,
/// linhas arredondadas para um número par de bytes e, se `bitmap.palette`
/// existir, a paleta embutida logo após o cabeçalho (componentes de 16 bits
/// com a cor no byte alto). Índices acima da profundidade são truncados.
pub fn encode(bitmap: &Bitmap) -> Vec<u8> {
    encode_with(bitmap, DEFAULT_UNKNOWN)
}

/// Como [`encode`], escolhendo os bytes 20/21 do cabeçalho.
pub fn encode_with(bitmap: &Bitmap, unknown: UnknownBytes) -> Vec<u8> {
    let depth = bitmap.depth as usize;
    let (w, h) = (bitmap.width as usize, bitmap.height as usize);
    let row_bytes = (w * depth).div_ceil(8).next_multiple_of(2);
    let mut raw = vec![0u8; row_bytes * h];
    let mask = ((1u16 << depth) - 1) as u8;
    for (y, row) in raw.chunks_exact_mut(row_bytes.max(1)).take(h).enumerate() {
        for x in 0..w {
            let v = bitmap.pixels.get(y * w + x).copied().unwrap_or(0) & mask;
            let bit = x * depth;
            let shift = 8 - depth - bit % 8;
            row[bit / 8] |= v << shift;
        }
    }
    let stream = pack_bits(&raw);
    let palette = bitmap.palette.as_deref().unwrap_or(&[]);
    let pal_count = palette.len().min(u16::MAX as usize);
    let data_offset = HEADER_LEN + 6 * pal_count;

    let mut v = Vec::with_capacity(data_offset + stream.len());
    let p16 = |v: &mut Vec<u8>, n: u16| v.extend_from_slice(&n.to_le_bytes());
    let p32 = |v: &mut Vec<u8>, n: u32| v.extend_from_slice(&n.to_le_bytes());
    p16(&mut v, 1); // marca de ordem de bytes
    p16(&mut v, 0);
    v.extend_from_slice(b"PKBM");
    p16(&mut v, 0);
    p16(&mut v, 1); // PackBits
    p16(&mut v, 1);
    p16(&mut v, HEADER_LEN as u16);
    // nos CHNK do Windows: fim dos pixels descomprimidos (offset dos pixels + tamanho)
    p32(&mut v, (data_offset + raw.len()) as u32);
    v.push(unknown.0);
    v.push(unknown.1);
    p16(&mut v, bitmap.width);
    p16(&mut v, bitmap.height);
    p16(&mut v, bitmap.depth as u16);
    p16(&mut v, pal_count as u16);
    p16(&mut v, HEADER_LEN as u16); // offset da paleta (42 mesmo sem paleta, como no original)
    v.extend_from_slice(&[0; 6]);
    p32(&mut v, data_offset as u32);
    debug_assert_eq!(v.len(), HEADER_LEN);
    for c in &palette[..pal_count] {
        p16(&mut v, (c.r as u16) << 8);
        p16(&mut v, (c.g as u16) << 8);
        p16(&mut v, (c.b as u16) << 8);
    }
    v.extend_from_slice(&stream);
    v
}

/// Compressão PackBits de um fluxo contínuo: sequências de 3 ou mais bytes
/// iguais viram repetição (até 128), o resto vira cópia literal (até 128).
pub fn pack_bits(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() / 2 + 2);
    let run_at = |i: usize| -> usize {
        let b = raw[i];
        raw[i..].iter().take(128).take_while(|&&x| x == b).count()
    };
    let mut i = 0;
    while i < raw.len() {
        let run = run_at(i);
        if run >= 3 {
            out.push((257 - run) as u8);
            out.push(raw[i]);
            i += run;
            continue;
        }
        // literal: avança até um trecho repetido de 3+ ou 128 bytes
        let start = i;
        while i < raw.len() && i - start < 128 {
            if raw.len() - i >= 3 && raw[i] == raw[i + 1] && raw[i] == raw[i + 2] {
                break;
            }
            i += 1;
        }
        out.push((i - start - 1) as u8);
        out.extend_from_slice(&raw[start..i]);
    }
    out
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

    fn pattern(w: u16, h: u16, depth: u8, palette: Option<Vec<Rgb>>) -> Bitmap {
        let mask = ((1u16 << depth) - 1) as u8;
        let pixels = (0..w as usize * h as usize)
            .map(|k| {
                // trechos repetidos e trechos variados, para exercitar runs e literais
                let v = if (k / 37) % 3 == 0 {
                    5
                } else {
                    (k * 7 + k / 11) as u8
                };
                v & mask
            })
            .collect();
        Bitmap {
            width: w,
            height: h,
            depth,
            pixels,
            palette,
        }
    }

    #[test]
    fn encode_roundtrips_every_depth() {
        for depth in [1u8, 2, 4, 8] {
            for (w, h) in [(1, 1), (7, 3), (16, 2), (33, 5), (594, 7)] {
                let b = pattern(w, h, depth, None);
                let e = encode(&b);
                assert!(is_pkbm(&e));
                assert_eq!(decode(&e).unwrap(), b, "{w}x{h}@{depth}");
            }
        }
    }

    #[test]
    fn encode_roundtrips_embedded_palette() {
        let pal: Vec<Rgb> = (0..=255u8)
            .map(|i| Rgb {
                r: i,
                g: 255 - i,
                b: i / 2,
            })
            .collect();
        let b = pattern(144, 84, 8, Some(pal));
        let e = encode(&b);
        assert_eq!(u16::from_le_bytes([e[28], e[29]]), 256);
        assert_eq!(u16::from_le_bytes([e[30], e[31]]), 42);
        assert_eq!(u32::from_le_bytes(e[38..42].try_into().unwrap()), 1578);
        assert_eq!(
            u32::from_le_bytes(e[16..20].try_into().unwrap()),
            1578 + 144 * 84
        );
        assert_eq!(decode(&e).unwrap(), b);
    }

    #[test]
    fn encode_header_matches_documents() {
        let b = pattern(594, 347, 8, None);
        let e = encode_with(&b, (40, 4));
        assert_eq!(
            &e[..16],
            b"\x01\x00\x00\x00PKBM\x00\x00\x01\x00\x01\x00\x2a\x00"
        );
        assert_eq!(
            u32::from_le_bytes(e[16..20].try_into().unwrap()),
            42 + 594 * 347
        );
        assert_eq!((e[20], e[21]), (40, 4));
        assert_eq!(decode(&e).unwrap(), b);
    }

    #[test]
    fn encode_pads_rows_to_even_bytes() {
        // 9 pixels de 1 bit: 2 bytes por linha arredondados para 2; 17 pixels: 3 → 4
        let b = pattern(17, 3, 1, None);
        let e = encode(&b);
        let raw = unpack_bits(&mut Reader::new(&e[42..], Format::Pkbm), 12).unwrap();
        assert_eq!(raw.len(), 12);
        assert_eq!(decode(&e).unwrap(), b);
    }

    #[test]
    fn pack_bits_runs_and_literals() {
        assert_eq!(pack_bits(&[]), Vec::<u8>::new());
        assert_eq!(pack_bits(&[7]), vec![0, 7]);
        assert_eq!(pack_bits(&[7, 7]), vec![1, 7, 7]);
        assert_eq!(pack_bits(&[7, 7, 7]), vec![0xfe, 7]);
        assert_eq!(pack_bits(&[1, 2, 3, 3, 3, 4]), vec![1, 1, 2, 0xfe, 3, 0, 4]);
        // repetição longa quebra em 128
        assert_eq!(pack_bits(&[9; 300]), vec![0x81, 9, 0x81, 9, 0xd5, 9]);
        // literal longo quebra em 128
        let lit: Vec<u8> = (0..200u8).collect();
        let e = pack_bits(&lit);
        assert_eq!(e[0], 127);
        assert_eq!(e[129], 71);
        assert_eq!(e.len(), 202);
        let mut r = Reader::new(&e, Format::Pkbm);
        assert_eq!(unpack_bits(&mut r, 200).unwrap(), lit);
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
