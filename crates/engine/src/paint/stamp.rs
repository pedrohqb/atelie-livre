//! Imagens coloridas coladas no quadro (adesivos).

use super::canvas::Canvas;
use super::store::nearest;
use crate::Image;
use formats::palette::Rgb;

/// Bitmap já convertido para índices da paleta; `None` não pinta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    pub w: u32,
    pub h: u32,
    pub px: Vec<Option<u8>>,
}

fn dark(p: &[u8]) -> bool {
    (p[0] as u32 + p[1] as u32 + p[2] as u32) < 384
}

impl Stamp {
    /// Converte `img` pela paleta, pintando só onde `mask` (mesmo tamanho) é escura.
    /// Sem máscara compatível, a imagem toda é opaca.
    pub fn from_images(img: &Image, mask: &Image, pal: &[Rgb]) -> Stamp {
        let use_mask = (mask.width, mask.height) == (img.width, img.height);
        let mut cache = std::collections::HashMap::new();
        let px = img
            .rgba
            .chunks_exact(4)
            .enumerate()
            .map(|(k, p)| {
                if use_mask && !dark(&mask.rgba[k * 4..k * 4 + 3]) {
                    return None;
                }
                let key = (p[0], p[1], p[2]);
                Some(
                    *cache
                        .entry(key)
                        .or_insert_with(|| nearest(pal, key.0, key.1, key.2)),
                )
            })
            .collect();
        Stamp {
            w: img.width,
            h: img.height,
            px,
        }
    }
}

impl Canvas {
    /// Cola o bitmap com o canto superior esquerdo em `(x0, y0)` (coordenadas do
    /// quadro), respeitando o recorte.
    pub fn paste(&mut self, s: &Stamp, x0: i32, y0: i32) {
        for (k, p) in s.px.iter().enumerate() {
            let Some(v) = p else { continue };
            let x = x0 + (k as u32 % s.w) as i32;
            let y = y0 + (k as u32 / s.w) as i32;
            self.set(x, y, *v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: u32, h: u32, px: &[[u8; 3]]) -> Image {
        Image {
            width: w,
            height: h,
            rgba: px.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        }
    }

    fn pal() -> Vec<Rgb> {
        vec![
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            Rgb { r: 0, g: 0, b: 0 },
            Rgb {
                r: 250,
                g: 0,
                b: 57,
            },
        ]
    }

    #[test]
    fn stamp_respects_mask_and_maps_colors() {
        let i = img(3, 1, &[[250, 0, 50], [255, 255, 255], [9, 9, 9]]);
        let m = img(3, 1, &[[0, 0, 0], [0, 0, 0], [255, 255, 255]]);
        let s = Stamp::from_images(&i, &m, &pal());
        assert_eq!(s.px, vec![Some(2), Some(0), None]);
    }

    #[test]
    fn stamp_without_matching_mask_is_opaque() {
        let i = img(2, 1, &[[0, 0, 0], [255, 255, 255]]);
        let m = img(1, 1, &[[0, 0, 0]]);
        assert_eq!(
            Stamp::from_images(&i, &m, &pal()).px,
            vec![Some(1), Some(0)]
        );
    }

    #[test]
    fn paste_clips_and_skips_transparent() {
        let mut c = Canvas::new(3, 3);
        let s = Stamp {
            w: 2,
            h: 2,
            px: vec![Some(5), None, Some(6), Some(7)],
        };
        c.paste(&s, 2, 2);
        assert_eq!(c.get(2, 2), Some(5));
        c.paste(&s, -1, -1);
        assert_eq!(c.get(0, 0), Some(7));
        c.paste(&s, 100, -100);
        assert_eq!(c.px.iter().filter(|&&v| v != 0).count(), 2);
    }

    #[test]
    fn paste_respects_clip() {
        let mut c = Canvas::new(3, 3);
        c.clip = Some((1, 1, 1, 1));
        let s = Stamp {
            w: 2,
            h: 2,
            px: vec![Some(5), Some(5), Some(5), Some(5)],
        };
        c.paste(&s, 0, 0);
        assert_eq!(c.px.iter().filter(|&&v| v == 5).count(), 1);
        assert_eq!(c.get(1, 1), Some(5));
    }
}
