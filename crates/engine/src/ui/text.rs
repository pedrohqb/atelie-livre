//! Texto de interface: corte com "…" e a "caneta" da fonte de 8 pontos do
//! programa (a dos diálogos e listas), com a 8×8 de reserva quando os dados
//! originais não estão presentes (testes).

use crate::Frame;
use crate::assets::Assets;
use formats::fnt::Font;
use std::rc::Rc;

/// Corta o texto com "…" para caber em `max` pixels (medidos por `width`).
/// Se nem um caractere couber, fica só "…".
pub fn clip_text(s: &str, width: impl Fn(&str) -> i32, max: i32) -> String {
    if width(s) <= max {
        return s.to_string();
    }
    let mut out: String = s.to_string();
    while !out.is_empty() && width(&format!("{out}…")) > max {
        out.pop();
    }
    format!("{out}…")
}

/// Fonte de 8 pontos do programa (ou a 8×8 de reserva) para rótulos,
/// campos e listas.
pub struct Pen {
    font: Option<Rc<Font>>,
}

impl Pen {
    pub fn new(a: &Assets) -> Pen {
        Pen { font: a.font(8) }
    }

    pub fn font(&self) -> Option<&Font> {
        self.font.as_deref()
    }

    /// Largura do texto em pixels (8 por caractere na reserva).
    pub fn width(&self, s: &str) -> i32 {
        match &self.font {
            Some(fo) => fo.width(s) as i32,
            None => 8 * s.chars().count() as i32,
        }
    }

    /// Escreve com o topo em `y`; devolve a largura.
    pub fn text(&self, f: &mut Frame, x: i32, y: i32, s: &str, rgb: [u8; 3]) -> i32 {
        match &self.font {
            Some(fo) => f.text_font(fo, x, y, s, rgb),
            None => {
                f.text(x, y, s, rgb);
                8 * s.chars().count() as i32
            }
        }
    }

    /// Escreve centrado em `cx`.
    pub fn centered(&self, f: &mut Frame, cx: i32, y: i32, s: &str, rgb: [u8; 3]) {
        self.text(f, cx - self.width(s) / 2, y, s, rgb);
    }

    /// [`clip_text`] com a largura desta fonte.
    pub fn clip(&self, s: &str, max: i32) -> String {
        clip_text(s, |t| self.width(t), max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_text_adds_ellipsis() {
        let w = |s: &str| s.chars().count() as i32;
        assert_eq!(clip_text("abc", w, 5), "abc");
        assert_eq!(clip_text("abcdef", w, 4), "abc…");
        let w8 = |s: &str| 8 * s.chars().count() as i32;
        assert_eq!(clip_text("abcdefgh", w8, 40), "abcd…");
        // nada cabe: só as reticências
        assert_eq!(clip_text("abc", w8, 4), "…");
    }

    #[test]
    fn pen_without_font_uses_8_pixels_per_char() {
        let (_d, a) = crate::scenes::testutil::empty_assets();
        let p = Pen::new(&a);
        assert_eq!(p.width("abc"), 24);
        assert_eq!(p.clip("abcdefgh", 40), "abcd…");
        let mut f = Frame::new(64, 16);
        assert_eq!(p.text(&mut f, 0, 0, "ab", [0, 0, 0]), 16);
    }
}
