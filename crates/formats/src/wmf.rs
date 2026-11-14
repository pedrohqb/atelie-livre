//! Parte `META` dos clip-arts (`PICT` nas bibliotecas `.ART`): um metarquivo do
//! Windows (WMF) que só desenha um bitmap com `StretchDIB`.
//!
//! ```text
//! 4 bytes  "META"
//! 4 × u16  x, y, largura, altura (largura e altura = tamanho do bitmap)
//! WMF      cabeçalho de 18 bytes (tipo 1, tamanho do cabeçalho em palavras = 9)
//!          registros { u32 tamanho em palavras, u16 função, parâmetros }
//! ```
//!
//! O registro `0x0F43` (StretchDIB) tem `u32 rop`, `u16 uso`, oito `i16` de
//! origem/destino e um DIB empacotado (BITMAPINFOHEADER + cores + bits).

use crate::winres::{Image, decode_bitmap};
use crate::{ErrorKind, Format, Reader, Result};

const STRETCH_DIB: u16 = 0x0F43;
const STRETCH_DIB_PARAMS: usize = 22;

/// Decodifica o bitmap do primeiro `StretchDIB` da parte `META`.
pub fn meta_dib(data: &[u8]) -> Result<Image> {
    let mut r = Reader::new(data, Format::WinRes);
    if r.bytes(4)? != b"META" {
        return Err(r.error(ErrorKind::BadMagic { expected: "META" }));
    }
    r.skip(8)?;
    let wmf_start = r.pos();
    r.skip(2)?; // tipo
    let header_words = r.u16_le()? as usize;
    r.seek(wmf_start + header_words * 2)?;
    while r.remaining() >= 6 {
        let start = r.pos();
        let words = r.u32_le()? as usize;
        let func = r.u16_le()?;
        if words < 3 || func == 0 {
            break;
        }
        let len = words
            .checked_mul(2)
            .ok_or_else(|| r.error(ErrorKind::Invalid("record size".into())))?;
        if func == STRETCH_DIB {
            r.seek(start)?;
            let rec = r.bytes(len)?;
            let dib = rec
                .get(6 + STRETCH_DIB_PARAMS..)
                .ok_or_else(|| r.error(ErrorKind::Invalid("StretchDIB curto".into())))?;
            return decode_bitmap(dib);
        }
        r.seek(start + len)?;
    }
    Err(r.error(ErrorKind::Unsupported("META sem StretchDIB".into())))
}

fn record(out: &mut Vec<u8>, func: u16, params: &[u8]) {
    let words = (6 + params.len()).div_ceil(2) as u32;
    out.extend_from_slice(&words.to_le_bytes());
    out.extend_from_slice(&func.to_le_bytes());
    out.extend_from_slice(params);
    if params.len() % 2 == 1 {
        out.push(0);
    }
}

fn u16s(v: &[u16]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// Parte `META` para uma imagem RGBA (as figuras próprias que viram objetos
/// do documento do Writer): `SetWindowOrg(0,0)`, `SetWindowExt(l, a)` e um
/// `StretchDIB` com um DIB de 24 bits, como os metarquivos das bibliotecas
/// (que o original desenha esticando na moldura). O transparente vira
/// branco. `meta_dib` lê de volta a mesma imagem.
pub fn meta_from_image(img: &Image) -> Vec<u8> {
    let (w, h) = (img.width.min(0x7fff), img.height.min(0x7fff));
    let row = (w as usize * 3).next_multiple_of(4);
    let mut dib = Vec::with_capacity(40 + row * h as usize);
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&(w as i32).to_le_bytes());
    dib.extend_from_slice(&(h as i32).to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes());
    dib.extend_from_slice(&24u16.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    dib.extend_from_slice(&((row * h as usize) as u32).to_le_bytes());
    dib.extend_from_slice(&[0; 16]);
    for y in (0..h).rev() {
        let start = dib.len();
        for x in 0..w {
            let i = ((y * img.width + x) * 4) as usize;
            let p = &img.rgba[i..i + 4];
            if p[3] == 0 {
                dib.extend_from_slice(&[255, 255, 255]);
            } else {
                dib.extend_from_slice(&[p[2], p[1], p[0]]);
            }
        }
        dib.resize(start + row, 0);
    }
    let (w16, h16) = (w as u16, h as u16);
    let mut recs = Vec::new();
    record(&mut recs, 0x020b, &u16s(&[0, 0])); // SetWindowOrg (y, x)
    record(&mut recs, 0x020c, &u16s(&[h16, w16])); // SetWindowExt (y, x)
    let mut sd = 0x00CC_0020u32.to_le_bytes().to_vec(); // SRCCOPY
    sd.extend_from_slice(&u16s(&[0, h16, w16, 0, 0, h16, w16, 0, 0]));
    sd.extend_from_slice(&dib);
    record(&mut recs, STRETCH_DIB, &sd);
    let max_rec = 3 + sd.len().div_ceil(2) as u32;
    record(&mut recs, 0, &[]); // fim
    let total_words = (18 + recs.len()) as u32 / 2;
    let mut m = b"META".to_vec();
    m.extend_from_slice(&u16s(&[0, 0, w16, h16]));
    m.extend_from_slice(&u16s(&[1, 9, 0x0300]));
    m.extend_from_slice(&total_words.to_le_bytes());
    m.extend_from_slice(&0u16.to_le_bytes());
    m.extend_from_slice(&max_rec.to_le_bytes());
    m.extend_from_slice(&0u16.to_le_bytes());
    m.extend_from_slice(&recs);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// META mínimo com um DIB de 2×1 a 1 bit (preto, branco).
    fn sample() -> Vec<u8> {
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&1i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&[0; 24]);
        dib.extend_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0]); // cores
        dib.extend_from_slice(&[0b0100_0000, 0, 0, 0]); // linha: preto, branco
        let mut rec = Vec::new();
        rec.extend_from_slice(&0x00CC_0020u32.to_le_bytes());
        rec.extend_from_slice(&[0; 18]);
        rec.extend_from_slice(&dib);
        let mut m = b"META".to_vec();
        m.extend_from_slice(&[0; 8]);
        m.extend_from_slice(&[1, 0, 9, 0, 0, 3]);
        m.extend_from_slice(&[0; 12]);
        // registro qualquer antes (SetBkMode)
        m.extend_from_slice(&4u32.to_le_bytes());
        m.extend_from_slice(&0x0102u16.to_le_bytes());
        m.extend_from_slice(&1u16.to_le_bytes());
        let words = (6 + rec.len()).div_ceil(2) as u32;
        m.extend_from_slice(&words.to_le_bytes());
        m.extend_from_slice(&STRETCH_DIB.to_le_bytes());
        m.extend_from_slice(&rec);
        if m.len() % 2 == 1 {
            m.push(0);
        }
        m.extend_from_slice(&3u32.to_le_bytes());
        m.extend_from_slice(&0u16.to_le_bytes());
        m
    }

    #[test]
    fn image_roundtrip_through_meta() {
        let mut rgba = Vec::new();
        for k in 0..(3 * 2) {
            rgba.extend_from_slice(&[k as u8 * 40, 10, 200, 255]);
        }
        // um pixel transparente vira branco
        rgba[3] = 0;
        let img = Image {
            width: 3,
            height: 2,
            rgba,
        };
        let m = meta_from_image(&img);
        assert_eq!(&m[4..12], &[0, 0, 0, 0, 3, 0, 2, 0]);
        let back = meta_dib(&m).unwrap();
        assert_eq!((back.width, back.height), (3, 2));
        assert_eq!(&back.rgba[0..3], &[255, 255, 255]);
        assert_eq!(&back.rgba[4..8], &img.rgba[4..8]);
        assert_eq!(&back.rgba[20..24], &img.rgba[20..24]);
    }

    #[test]
    fn decodes_stretch_dib() {
        let img = meta_dib(&sample()).unwrap();
        assert_eq!((img.width, img.height), (2, 1));
        assert_eq!(&img.rgba[0..3], &[0, 0, 0]);
        assert_eq!(&img.rgba[4..7], &[255, 255, 255]);
    }

    #[test]
    fn rejects_bad_magic_and_missing_record() {
        assert!(meta_dib(b"NOPE00000000").is_err());
        let mut m = b"META".to_vec();
        m.extend_from_slice(&[0; 8]);
        m.extend_from_slice(&[1, 0, 9, 0, 0, 3]);
        m.extend_from_slice(&[0; 12]);
        m.extend_from_slice(&3u32.to_le_bytes());
        m.extend_from_slice(&0u16.to_le_bytes());
        assert!(meta_dib(&m).is_err());
        assert!(meta_dib(&sample()[..40]).is_err());
    }
}
