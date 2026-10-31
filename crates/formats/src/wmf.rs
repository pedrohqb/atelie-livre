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
