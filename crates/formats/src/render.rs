//! Conversão de imagens `PKBM` em RGBA, com a regra de escolha de paleta.
//!
//! Ordem de preferência: paleta embutida na imagem; `CTBL` do arquivo `CHNK`
//! (de 16 ou de 256 cores); `LOGPALETTE` 4 e 8 do executável; rampa de cinzas.

use crate::winres::Image;
use crate::{Rgb, chnk, ne, palette, pkbm};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palettes {
    pub pal16: Vec<Rgb>,
    pub pal256: Vec<Rgb>,
}

fn gray(n: usize) -> Vec<Rgb> {
    (0..n)
        .map(|i| {
            let v = (255 - i * 255 / (n - 1)) as u8;
            Rgb { r: v, g: v, b: v }
        })
        .collect()
}

impl Palettes {
    /// Paletas `LOGPALETTE` 4 (16 cores) e 8 (256 cores) de um executável NE.
    ///
    /// As imagens de 4 bits vieram do Mac, onde o índice 0 é branco e o 15 é preto;
    /// a `LOGPALETTE` de 16 cores do Windows tem o preto no índice 1. A paleta de
    /// 16 é reordenada para a ordem do Mac: 0→0, 1..14→2..15, 15→1.
    pub fn from_exe(exe: Option<&[u8]>) -> Palettes {
        let f = exe.and_then(|d| ne::parse(d).ok());
        let read = |id: u16| -> Option<Vec<Rgb>> {
            palette::parse_logpalette(f.as_ref()?.find("LOGPALETTE", id)?.data).ok()
        };
        let pal16 = match read(4) {
            Some(p) if p.len() == 16 => {
                let mut mac = vec![p[0]];
                mac.extend_from_slice(&p[2..16]);
                mac.push(p[1]);
                mac
            }
            Some(p) => p,
            None => gray(16),
        };
        Palettes {
            pal16,
            pal256: read(8).unwrap_or_else(|| gray(256)),
        }
    }

    /// Aplica a `CTBL` do arquivo, se houver: 16 cores trocam `pal16`, 256 trocam `pal256`.
    pub fn for_chnk(&self, f: &chnk::ChunkFile<'_>) -> Palettes {
        let ctbl = f
            .chunks
            .iter()
            .flat_map(|c| &c.parts)
            .find(|p| p.tag.0 == *b"CTBL")
            .and_then(|p| palette::parse_ctbl(p.data).ok());
        let mut out = self.clone();
        match ctbl {
            Some(c) if c.len() == 16 => out.pal16 = c,
            Some(c) if c.len() == 256 => out.pal256 = c,
            _ => {}
        }
        out
    }
}

/// Converte para RGBA opaco; índice fora da paleta vira preto.
pub fn bitmap_to_rgba(b: &pkbm::Bitmap, pal: &Palettes) -> Image {
    let default;
    let colors: &[Rgb] = match &b.palette {
        Some(p) => p,
        None => {
            default = pkbm::default_palette_for(b.depth, &pal.pal16, &pal.pal256);
            &default
        }
    };
    let mut rgba = Vec::with_capacity(b.pixels.len() * 4);
    for &i in &b.pixels {
        let c = colors
            .get(i as usize)
            .copied()
            .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
        rgba.extend_from_slice(&[c.r, c.g, c.b, 255]);
    }
    Image {
        width: b.width as u32,
        height: b.height as u32,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkbm::Bitmap;

    fn rgb(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }

    fn bitmap(depth: u8, pixels: Vec<u8>, palette: Option<Vec<Rgb>>) -> Bitmap {
        Bitmap {
            width: pixels.len() as u16,
            height: 1,
            depth,
            pixels,
            palette,
        }
    }

    #[test]
    fn from_exe_none_gives_gray_ramps() {
        let p = Palettes::from_exe(None);
        assert_eq!(p.pal16.len(), 16);
        assert_eq!(p.pal256.len(), 256);
        assert_eq!(p.pal16[0], rgb(255, 255, 255));
        assert_eq!(p.pal256[255], rgb(0, 0, 0));
    }

    fn exe_with_logpalette(colors16: &[Rgb]) -> Vec<u8> {
        // NE mínimo com um recurso nomeado LOGPALETTE id 4
        let mut pal = vec![0x00, 0x03];
        pal.extend_from_slice(&(colors16.len() as u16).to_le_bytes());
        for c in colors16 {
            pal.extend_from_slice(&[c.r, c.g, c.b, 0]);
        }
        crate::ne::tests::tiny_ne_with(&[(0, Some("LOGPALETTE"), 0x8004, &pal)])
    }

    #[test]
    fn from_exe_reorders_16_colors_to_mac_order() {
        // imagens de 4 bits vêm do Mac (0 = branco, 15 = preto); a LOGPALETTE do
        // Windows tem o preto no índice 1: 0→0, 1..14→2..15, 15→1
        let colors: Vec<Rgb> = (0..16u8).map(|k| rgb(k, k, k)).collect();
        let p = Palettes::from_exe(Some(&exe_with_logpalette(&colors)));
        let order: Vec<u8> = p.pal16.iter().map(|c| c.r).collect();
        assert_eq!(
            order,
            [0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1]
        );
    }

    #[test]
    fn bitmap_uses_embedded_palette_first() {
        let p = Palettes::from_exe(None);
        let img = bitmap_to_rgba(&bitmap(8, vec![0], Some(vec![rgb(1, 2, 3)])), &p);
        assert_eq!(img.rgba, [1, 2, 3, 255]);
    }

    #[test]
    fn bitmap_falls_back_to_depth_palette() {
        let mut p = Palettes::from_exe(None);
        p.pal16[3] = rgb(9, 8, 7);
        assert_eq!(
            bitmap_to_rgba(&bitmap(1, vec![0, 1], None), &p).rgba,
            [255, 255, 255, 255, 0, 0, 0, 255]
        );
        assert_eq!(
            bitmap_to_rgba(&bitmap(4, vec![3], None), &p).rgba,
            [9, 8, 7, 255]
        );
    }

    #[test]
    fn index_outside_palette_is_black() {
        let p = Palettes::from_exe(None);
        let img = bitmap_to_rgba(&bitmap(8, vec![5], Some(vec![rgb(1, 1, 1)])), &p);
        assert_eq!(img.rgba, [0, 0, 0, 255]);
        assert_eq!((img.width, img.height), (1, 1));
    }

    #[test]
    fn for_chnk_replaces_matching_size() {
        let base = Palettes::from_exe(None);
        let mut ctbl = vec![1u8, 0, 6, 0];
        ctbl.extend_from_slice(b"CTBL");
        ctbl.extend_from_slice(&16u16.to_le_bytes());
        for i in 0..16u16 {
            for _ in 0..3 {
                ctbl.extend_from_slice(&((i * 16) << 8).to_le_bytes());
            }
        }
        let f = chnk::ChunkFile {
            creator: crate::Tag(*b"TEST"),
            version: (4, 4),
            byte_order: chnk::ByteOrder::Little,
            chunks: vec![chnk::Chunk {
                tag: crate::Tag(*b"CTBL"),
                id: 0,
                flags: 0,
                name: None,
                parts: vec![chnk::Part {
                    tag: crate::Tag(*b"CTBL"),
                    offset: 0,
                    data: &ctbl,
                }],
            }],
        };
        let p = base.for_chnk(&f);
        assert_eq!(p.pal16[1], rgb(16, 16, 16));
        assert_eq!(p.pal256, base.pal256);
    }
}
