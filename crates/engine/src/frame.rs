//! Quadro RGBA em software, desenhado pelas cenas e ampliado pela plataforma.

use crate::Image;
use font8x8::{BASIC_FONTS, LATIN_FONTS, UnicodeFonts};

pub const SCREEN_W: u32 = 640;
pub const SCREEN_H: u32 = 480;

#[derive(Debug, Clone)]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Frame {
    /// Quadro preto opaco.
    pub fn new(width: u32, height: u32) -> Frame {
        let mut f = Frame {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        };
        f.fill([0, 0, 0]);
        f
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// RGBA, linha a linha.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        self.pixels[i..i + 4].try_into().expect("4 bytes")
    }

    pub fn fill(&mut self, rgb: [u8; 3]) {
        for p in self.pixels.chunks_exact_mut(4) {
            p.copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }

    /// Um pixel, recortado nas bordas.
    pub fn put(&mut self, x: i32, y: i32, rgb: [u8; 3]) {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            let i = ((y as u32 * self.width + x as u32) * 4) as usize;
            self.pixels[i..i + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }

    /// Copia a imagem com o canto superior esquerdo em (x, y); pixels de alfa 0 são pulados.
    pub fn blit(&mut self, img: &Image, x: i32, y: i32) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + img.width as i32).min(self.width as i32);
        let y1 = (y + img.height as i32).min(self.height as i32);
        for dy in y0..y1 {
            for dx in x0..x1 {
                let si = (((dy - y) as u32 * img.width + (dx - x) as u32) * 4) as usize;
                let src = &img.rgba[si..si + 4];
                if src[3] != 0 {
                    self.put(dx, dy, [src[0], src[1], src[2]]);
                }
            }
        }
    }

    /// Retângulo cheio, recortado nas bordas.
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgb: [u8; 3]) {
        for yy in y.max(0)..(y + h).min(self.height as i32) {
            for xx in x.max(0)..(x + w).min(self.width as i32) {
                self.put(xx, yy, rgb);
            }
        }
    }

    /// Contorno de `t` pixels por dentro do retângulo.
    pub fn frame_rect(&mut self, x: i32, y: i32, w: i32, h: i32, t: i32, rgb: [u8; 3]) {
        self.rect(x, y, w, t, rgb);
        self.rect(x, y + h - t, w, t, rgb);
        self.rect(x, y, t, h, rgb);
        self.rect(x + w - t, y, t, h, rgb);
    }

    /// Copia `img` só onde `mask` (mesmo tamanho) é escura; sem máscara compatível, `blit`.
    pub fn blit_masked(&mut self, img: &Image, mask: &Image, x: i32, y: i32) {
        if (mask.width, mask.height) != (img.width, img.height) {
            return self.blit(img, x, y);
        }
        for dy in 0..img.height {
            for dx in 0..img.width {
                let i = ((dy * img.width + dx) * 4) as usize;
                let m = &mask.rgba[i..i + 3];
                if (m[0] as u32 + m[1] as u32 + m[2] as u32) < 384 {
                    let p = &img.rgba[i..i + 3];
                    self.put(x + dx as i32, y + dy as i32, [p[0], p[1], p[2]]);
                }
            }
        }
    }

    /// Texto numa fonte de bitmap do Windows, com o topo da linha em `y`.
    /// Devolve a largura desenhada.
    pub fn text_font(
        &mut self,
        font: &formats::fnt::Font,
        x: i32,
        y: i32,
        s: &str,
        rgb: [u8; 3],
    ) -> i32 {
        let mut gx = x;
        for g in font.glyphs(s) {
            for (k, &on) in g.bits.iter().enumerate() {
                if on {
                    let w = g.width as usize;
                    self.put(gx + (k % w) as i32, y + (k / w) as i32, rgb);
                }
            }
            gx += g.width as i32;
        }
        gx - x
    }

    /// Texto com a fonte 8×8; caracteres sem glifo viram `?`.
    pub fn text(&mut self, x: i32, y: i32, s: &str, rgb: [u8; 3]) {
        for (k, c) in s.chars().enumerate() {
            let glyph = BASIC_FONTS
                .get(c)
                .or_else(|| LATIN_FONTS.get(c))
                .or_else(|| BASIC_FONTS.get('?'))
                .expect("'?' glyph");
            let gx = x + 8 * k as i32;
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        self.put(gx + col, y + row as i32, rgb);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_and_frame_rect_clip() {
        let mut f = Frame::new(4, 4);
        f.fill([0, 0, 0]);
        f.rect(-2, -2, 4, 4, [9, 9, 9]);
        assert_eq!(f.pixel(1, 1), [9, 9, 9, 255]);
        assert_eq!(f.pixel(2, 2), [0, 0, 0, 255]);
        f.frame_rect(0, 0, 4, 4, 1, [5, 5, 5]);
        assert_eq!(f.pixel(3, 0), [5, 5, 5, 255]);
        assert_eq!(f.pixel(2, 2), [0, 0, 0, 255]);
    }

    #[test]
    fn blit_masked_copies_only_dark_mask() {
        let mut f = Frame::new(2, 1);
        f.fill([0, 0, 0]);
        let img = Image {
            width: 2,
            height: 1,
            rgba: vec![1, 2, 3, 255, 4, 5, 6, 255],
        };
        let mask = Image {
            width: 2,
            height: 1,
            rgba: vec![0, 0, 0, 255, 255, 255, 255, 255],
        };
        f.blit_masked(&img, &mask, 0, 0);
        assert_eq!(f.pixel(0, 0), [1, 2, 3, 255]);
        assert_eq!(f.pixel(1, 0), [0, 0, 0, 255]);
    }

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Image {
        Image {
            width: w,
            height: h,
            rgba: rgba.repeat((w * h) as usize),
        }
    }

    fn lit(f: &Frame) -> Vec<(u32, u32)> {
        let mut v = Vec::new();
        for y in 0..f.height() {
            for x in 0..f.width() {
                if f.pixel(x, y) != [0, 0, 0, 255] {
                    v.push((x, y));
                }
            }
        }
        v
    }

    #[test]
    fn blit_clips_all_edges() {
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        let img = solid(4, 4, [255, 0, 0, 255]);
        f.blit(&img, -2, -2);
        f.blit(&img, 638, 478);
        f.blit(&img, 700, 0);
        f.blit(&img, 0, -100);
        let l = lit(&f);
        assert_eq!(l.len(), 8);
        assert!(l.contains(&(0, 0)) && l.contains(&(1, 1)) && !l.contains(&(2, 2)));
        assert!(l.contains(&(639, 479)) && l.contains(&(638, 478)));
    }

    #[test]
    fn blit_skips_transparent() {
        let mut f = Frame::new(4, 4);
        f.blit(&solid(2, 2, [9, 9, 9, 0]), 0, 0);
        assert!(lit(&f).is_empty());
    }

    #[test]
    fn fill_sets_every_pixel() {
        let mut f = Frame::new(3, 2);
        f.fill([1, 2, 3]);
        assert!(f.pixels().chunks(4).all(|p| p == [1, 2, 3, 255]));
        assert_eq!(f.pixels().len(), 3 * 2 * 4);
    }

    #[test]
    fn text_draws_glyph() {
        let mut f = Frame::new(32, 16);
        f.text(0, 0, "A", [255, 255, 255]);
        let l = lit(&f);
        assert!(!l.is_empty());
        assert!(l.iter().all(|&(x, y)| x < 8 && y < 8));
        assert!(l.iter().any(|&(_, y)| y == 1));
    }

    #[test]
    fn text_handles_accents_and_unknown() {
        let mut a = Frame::new(16, 8);
        a.text(0, 0, "ã", [255, 255, 255]);
        assert!(!lit(&a).is_empty());
        let mut u = Frame::new(16, 8);
        u.text(0, 0, "\u{4e2d}", [255, 255, 255]);
        let mut q = Frame::new(16, 8);
        q.text(0, 0, "?", [255, 255, 255]);
        assert_eq!(u.pixels(), q.pixels());
        let mut two = Frame::new(16, 8);
        two.text(0, 0, "??", [255, 255, 255]);
        assert!(lit(&two).iter().any(|&(x, _)| x >= 8));
    }
}
