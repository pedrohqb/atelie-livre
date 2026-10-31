//! Texto no quadro (ferramenta Palavra): glifos da fonte de bitmap "Pablo
//! Graphos" — ou da fonte 8×8 reserva, quando os dados não trazem a fonte —
//! plotados pixel a pixel, com o mesmo arranjo de `Frame::text_font`, e o
//! balão de fala branco com contorno preto.

use super::canvas::{BLACK, Canvas, WHITE};
use font8x8::{BASIC_FONTS, LATIN_FONTS, UnicodeFonts};
use formats::fnt::Font;

/// Altura (e largura) de cada caractere da fonte 8×8 reserva.
const FALLBACK: u32 = 8;
/// Folga entre o texto e o contorno do balão.
pub const BALLOON_PAD: i32 = 6;
/// Altura do rabicho do balão, abaixo do contorno.
const TAIL_H: i32 = 7;

/// Largura e altura do texto em pixels (altura da linha mesmo com texto vazio).
pub fn text_size(font: Option<&Font>, text: &str) -> (u32, u32) {
    match font {
        Some(f) => (f.width(text), f.height),
        None => (FALLBACK * text.chars().count() as u32, FALLBACK),
    }
}

/// Chama `put(dx, dy)` para cada pixel de tinta do texto, relativo ao canto
/// superior esquerdo da linha.
pub fn text_pixels(font: Option<&Font>, text: &str, mut put: impl FnMut(i32, i32)) {
    let mut gx = 0i32;
    match font {
        Some(f) => {
            for g in f.glyphs(text) {
                let w = g.width as usize;
                for (k, &on) in g.bits.iter().enumerate() {
                    if on && w > 0 {
                        put(gx + (k % w) as i32, (k / w) as i32);
                    }
                }
                gx += g.width as i32;
            }
        }
        None => {
            for c in text.chars() {
                let glyph = BASIC_FONTS
                    .get(c)
                    .or_else(|| LATIN_FONTS.get(c))
                    .or_else(|| BASIC_FONTS.get('?'))
                    .expect("glifo '?'");
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..8 {
                        if bits & (1 << col) != 0 {
                            put(gx + col, row as i32);
                        }
                    }
                }
                gx += FALLBACK as i32;
            }
        }
    }
}

/// Pixels do balão de fala em volta de um texto de `w`×`h` com canto em `(x, y)`:
/// `put(dx, dy, preto)` para cada pixel (preto = contorno; senão, interior branco).
/// O balão é o retângulo do texto com `BALLOON_PAD` de folga e um rabicho
/// apontando para baixo e para a esquerda.
pub fn balloon_pixels(x: i32, y: i32, w: u32, h: u32, mut put: impl FnMut(i32, i32, bool)) {
    let x0 = x - BALLOON_PAD;
    let y0 = y - BALLOON_PAD;
    let x1 = x + w as i32 + BALLOON_PAD; // exclusivo
    let y1 = y + h as i32 + BALLOON_PAD; // exclusivo
    // base do rabicho na borda de baixo: colunas [tail_l, tail_r]
    let tail_l = x0 + 10;
    let tail_r = (x0 + 17).min(x1 - 2);
    for yy in y0..y1 {
        for xx in x0..x1 {
            let edge = yy == y0 || yy == y1 - 1 || xx == x0 || xx == x1 - 1;
            let opening = yy == y1 - 1 && xx > tail_l && xx < tail_r;
            put(xx, yy, edge && !opening);
        }
    }
    // rabicho: afunila da base até a ponta em (x0 + 4, y1 + TAIL_H - 1)
    let tip = x0 + 4;
    for k in 0..TAIL_H {
        let t = (k + 1) as f32 / TAIL_H as f32;
        let l = tail_l as f32 + (tip as f32 - tail_l as f32) * t;
        let r = tail_r as f32 + (tip as f32 - tail_r as f32) * t;
        let (l, r) = (l.round() as i32, r.round() as i32);
        for xx in l..=r {
            put(xx, y1 + k, xx == l || xx == r);
        }
    }
}

impl Canvas {
    /// Escreve `text` com o canto superior esquerdo em `(x, y)` (coordenadas do
    /// quadro), na cor sólida `color`, recortado ao quadro.
    pub fn draw_text(&mut self, font: Option<&Font>, x: i32, y: i32, text: &str, color: u8) {
        text_pixels(font, text, |dx, dy| self.set(x + dx, y + dy, color));
    }

    /// Balão de fala branco com contorno preto em volta do texto de `text_size`.
    pub fn draw_balloon(&mut self, font: Option<&Font>, x: i32, y: i32, text: &str) {
        let (w, h) = text_size(font, text);
        balloon_pixels(x, y, w, h, |xx, yy, edge| {
            self.set(xx, yy, if edge { BLACK } else { WHITE })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::canvas::WHITE;

    fn count(c: &Canvas, v: u8) -> usize {
        c.px.iter().filter(|&&p| p == v).count()
    }

    #[test]
    fn fallback_text_size_and_pixels() {
        assert_eq!(text_size(None, "Oi"), (16, 8));
        assert_eq!(text_size(None, ""), (0, 8));
        let mut c = Canvas::new(40, 20);
        c.draw_text(None, 2, 3, "A", 5);
        let n = count(&c, 5);
        assert!(n > 0);
        // todos dentro da caixa 8×8 a partir de (2, 3)
        for (k, &p) in c.px.iter().enumerate() {
            if p == 5 {
                let (x, y) = ((k % 40) as i32, (k / 40) as i32);
                assert!((2..10).contains(&x) && (3..11).contains(&y), "({x}, {y})");
            }
        }
    }

    #[test]
    fn text_is_clipped_to_canvas() {
        let mut c = Canvas::new(10, 10);
        c.draw_text(None, -4, -4, "MM", 3);
        c.draw_text(None, 8, 8, "MM", 3);
        c.draw_text(None, 100, 100, "MM", 3);
        assert!(count(&c, 3) > 0);
        assert_eq!(c.px.len(), 100);
    }

    #[test]
    fn balloon_is_white_inside_black_outline_with_tail() {
        let mut c = Canvas::new(60, 40);
        c.px.fill(7);
        c.draw_balloon(None, 10, 10, "Oi");
        // texto 16×8 em (10,10): balão de (4,4) a (32,24) exclusivos
        assert_eq!(c.get(4, 4), Some(BLACK));
        assert_eq!(c.get(31, 23), Some(BLACK));
        assert_eq!(c.get(31, 4), Some(BLACK));
        assert_eq!(c.get(5, 5), Some(WHITE));
        assert_eq!(c.get(9, 9), Some(WHITE));
        assert_eq!(c.get(30, 22), Some(WHITE));
        assert_eq!(c.get(3, 3), Some(7));
        assert_eq!(c.get(32, 4), Some(7));
        // abertura do rabicho na borda de baixo e ponta mais abaixo, à esquerda
        assert_eq!(c.get(15, 23), Some(WHITE));
        assert_eq!(c.get(14, 23), Some(BLACK));
        assert_eq!(c.get(15, 24), Some(WHITE));
        assert_eq!(c.get(8, 30), Some(BLACK));
        assert_eq!(c.get(8, 31), Some(7));
        assert_eq!(c.get(31, 30), Some(7));
    }
}
