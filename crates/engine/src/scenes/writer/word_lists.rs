//! Listas suspensas da faixa da Palavra (`office-palavra.md` §4–7), com a
//! geometria medida nas capturas do original (`office_14`..`office_46`):
//! caixa branca com borda preta de 1 px a partir de y 94, itens separados
//! por uma linha preta e o item atual com o ✓ (`029d`) à esquerda.
//!
//! Os rótulos de tamanho, alinhamento, estilos de parágrafo, espaçamento e
//! recuos são figuras do `WRITER.PIC`; estilos, efeitos e Mix são texto (os
//! estilos desenhados no próprio estilo) e as fontes, o nome desenhado na
//! própria fonte, 5 de cada vez com as setas ▲▼ embaixo.

use super::editor::Editor;
use super::tools::{Rect, pic};
use crate::Frame;
use crate::assets::Assets;
use crate::writer::brush::Brush;
use crate::writer::doc::{
    Align, CSCH, CharChange, Effect, FONTS, Indent, ParaChange, ParaStyle, Size, Spacing, TextStyle,
};
use crate::writer::fonts::{Family, RunFont};
use crate::writer::mix::Mix;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];

/// ✓ do item atual (14×12).
pub const CHECK: u16 = 0x029d;
/// Topo (borda) de todas as listas.
pub const TOP: i32 = 94;
/// Passo das listas de texto e de cores.
const PITCH: i32 = 21;
/// Linhas de fonte visíveis no Seleciona-letras.
pub const FONT_ROWS: usize = 5;
/// Faixa das setas do Seleciona-letras (abaixo das 5 fontes).
const FONT_ARROWS_H: i32 = 15;
/// Setas ▲ e ▼ (12×12; `N+2` = apertada).
const ARROW_UP: u16 = 0x02db;
const ARROW_DOWN: u16 = 0x02df;

/// Listas das duas páginas da faixa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum List {
    Fonts,
    Sizes,
    Colors,
    Styles,
    Align,
    Highlight,
    Effects,
    Mix,
    ParaStyles,
    Spacing,
    Indents,
}

/// Rótulo de um item.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Label {
    Text(&'static str),
    Pic(u16),
    Color([u8; 3]),
    Font(&'static str),
    Style(TextStyle),
}

fn rgb(c: formats::Rgb) -> [u8; 3] {
    [c.r, c.g, c.b]
}

/// Itens das listas de bitmaps (cor; preto e branco = `N+1`).
const SIZE_PICS: [u16; 7] = [0x0c1d, 0x0c1f, 0x0c21, 0x0c23, 0x0c25, 0x0c27, 0x0c29];
const ALIGN_PICS: [u16; 4] = [0x0bc9, 0x0bcb, 0x0bcd, 0x0bcf];
const PARA_PICS: [u16; 3] = [0x0bd9, 0x0bdb, 0x0bdd];
const SPACING_PICS: [u16; 3] = [0x0bdf, 0x0be1, 0x0be3];
const INDENT_PICS: [u16; 4] = [0x0bd1, 0x0bd3, 0x0bd5, 0x0bd7];
const NOTHING: &str = "Nada de especial";

impl List {
    pub const ALL: [List; 11] = [
        List::Fonts,
        List::Sizes,
        List::Colors,
        List::Styles,
        List::Align,
        List::Highlight,
        List::Effects,
        List::Mix,
        List::ParaStyles,
        List::Spacing,
        List::Indents,
    ];

    /// Bordas esquerda e direita (medidas).
    fn x_span(self) -> (i32, i32) {
        match self {
            List::Fonts => (24, 175),
            List::Sizes => (89, 240),
            List::Colors => (154, 256),
            List::Styles => (219, 310),
            List::Align => (284, 411),
            List::Highlight => (24, 126),
            List::Effects => (89, 197),
            List::Mix => (154, 262),
            List::ParaStyles => (219, 359),
            List::Spacing => (215, 402),
            List::Indents => (324, 458),
        }
    }

    /// Linhas horizontais: a borda de cima, os separadores e a de baixo
    /// (sem a faixa das setas das fontes).
    fn rules(self) -> Vec<i32> {
        let pitch = |n: i32, p: i32| (0..=n).map(|k| TOP + k * p).collect();
        match self {
            List::Fonts => pitch(FONT_ROWS as i32, PITCH),
            List::Sizes => vec![TOP, 159, 208, 245, 275, 307, 333, 357],
            List::Colors | List::Highlight => pitch(16, PITCH),
            List::Styles => pitch(4, PITCH),
            List::Effects | List::Mix => pitch(7, PITCH),
            List::Align | List::Indents => pitch(4, 49),
            List::ParaStyles => pitch(4, 51),
            List::Spacing => pitch(4, 52),
        }
    }

    /// Número de itens (as fontes: todas, não só as visíveis).
    pub fn items(self) -> usize {
        match self {
            List::Fonts => FONTS.len(),
            List::Sizes => 7,
            List::Colors | List::Highlight => 16,
            List::Styles | List::Align | List::ParaStyles | List::Spacing | List::Indents => 4,
            List::Effects | List::Mix => 7,
        }
    }

    /// Caixa inteira, com a borda.
    pub fn rect(self) -> Rect {
        let (x0, x1) = self.x_span();
        let r = self.rules();
        let bottom = r[r.len() - 1]
            + if self == List::Fonts {
                FONT_ARROWS_H
            } else {
                0
            };
        (x0, TOP, x1 - x0 + 1, bottom - TOP + 1)
    }

    /// Lugar visível `row` (entre as linhas), por dentro.
    pub fn row_rect(self, row: usize) -> Rect {
        let (x0, x1) = self.x_span();
        let r = self.rules();
        let (a, b) = (r[row], r[row + 1]);
        (x0 + 1, a + 1, x1 - x0 - 1, b - a - 1)
    }

    /// Linhas visíveis.
    pub fn rows(self) -> usize {
        self.rules().len() - 1
    }

    /// Setas ▲ e ▼ do Seleciona-letras.
    pub fn font_arrows() -> [Rect; 2] {
        let y = TOP + FONT_ROWS as i32 * PITCH + 2;
        [(62, y, 12, 12), (115, y, 12, 12)]
    }

    /// Mostra o ✓ (o Mix e as cores não são um estado).
    fn checked(self) -> bool {
        !matches!(self, List::Mix | List::Colors | List::Highlight)
    }

    fn label(self, k: usize) -> Label {
        match self {
            List::Fonts => Label::Font(FONTS[k].0),
            List::Sizes => Label::Pic(SIZE_PICS[k]),
            List::Colors | List::Highlight => Label::Color(rgb(CSCH[k])),
            List::Styles => Label::Style(TextStyle::ALL[k]),
            List::Align => Label::Pic(ALIGN_PICS[k]),
            List::Effects => match k {
                0 => Label::Text(NOTHING),
                k => Label::Text(Effect::LIST[k - 1].name()),
            },
            List::Mix => Label::Text(Mix::ALL[k].name()),
            List::ParaStyles => match k {
                0 => Label::Text(NOTHING),
                k => Label::Pic(PARA_PICS[k - 1]),
            },
            List::Spacing => match k {
                0 => Label::Text(NOTHING),
                k => Label::Pic(SPACING_PICS[k - 1]),
            },
            List::Indents => Label::Pic(INDENT_PICS[k]),
        }
    }

    /// O pincel do item `k`.
    pub fn brush(self, k: usize) -> Brush {
        match self {
            List::Fonts => Brush::Font(FONTS[k].0),
            List::Sizes => Brush::Char(CharChange::Size(Size::ALL[k])),
            List::Colors => Brush::Char(CharChange::Color(Some(CSCH[k]))),
            // o branco tira o iluminador (a folha é branca)
            List::Highlight => Brush::Char(CharChange::Highlight(
                Some(CSCH[k]).filter(|&c| c != CSCH[0]),
            )),
            List::Styles => Brush::Char(CharChange::Style(TextStyle::ALL[k])),
            List::Align => Brush::Para(ParaChange::Align(Align::ALL[k])),
            List::Effects => Brush::Char(CharChange::Effect(
                k.checked_sub(1).map(|k| Effect::LIST[k]),
            )),
            List::Mix => Brush::Mix(Mix::ALL[k]),
            List::ParaStyles => Brush::Para(ParaChange::Style(
                [
                    ParaStyle::Normal,
                    ParaStyle::DropCap,
                    ParaStyle::RaisedCap,
                    ParaStyle::Bullets,
                ][k],
            )),
            List::Spacing => Brush::Para(ParaChange::Spacing(
                [
                    Spacing::Normal,
                    Spacing::NewPage,
                    Spacing::ExtraLines,
                    Spacing::ExtraAfter,
                ][k],
            )),
            List::Indents => Brush::Para(ParaChange::Indent(Indent::ALL[k])),
        }
    }

    /// Item com ✓: o do pincel carregado, se for desta lista; senão o
    /// estado do texto no ponto de inserção.
    pub fn current(self, ed: &Editor, brush: Option<Brush>) -> Option<usize> {
        if let Some(b) = brush
            && let Some(k) = (0..self.items()).find(|&k| self.brush(k) == b)
        {
            return Some(k);
        }
        let p = ed.typing_props();
        let doc = ed.doc();
        let para = doc.paras[ed.sel.caret.para.min(doc.paras.len() - 1)].props;
        match self {
            List::Fonts => {
                let name = doc.fonts.name(p.font);
                let fam = Family::resolve(name);
                FONTS.iter().position(|(n, _)| Family::resolve(n) == fam)
            }
            List::Sizes => Size::ALL.iter().position(|&s| s == p.size),
            List::Styles => match (p.bold, p.italic, p.underline) {
                (false, false, false) => Some(0),
                (true, _, _) => Some(1),
                (_, true, _) => Some(2),
                _ => Some(3),
            },
            List::Align => Align::ALL.iter().position(|&a| a == para.align),
            List::Effects => match p.effect {
                None => Some(0),
                Some(e) => Effect::LIST.iter().position(|&x| x == e).map(|k| k + 1),
            },
            List::ParaStyles => Some(para.para_style() as usize),
            List::Spacing => Some(para.spacing() as usize),
            List::Indents => Indent::ALL.iter().position(|&i| i == para.indent),
            List::Colors | List::Highlight | List::Mix => None,
        }
    }

    /// Desenha a lista; `scroll` = primeira fonte visível.
    pub fn draw(self, ed: &Editor, brush: Option<Brush>, scroll: usize, f: &mut Frame, a: &Assets) {
        let (x, y, w, h) = self.rect();
        f.rect(x, y, w, h, BLACK);
        f.rect(x + 1, y + 1, w - 2, h - 2, WHITE);
        let (x0, x1) = self.x_span();
        for &r in &self.rules()[1..] {
            f.rect(x0, r, x1 - x0 + 1, 1, BLACK);
        }
        let cur = if self.checked() {
            self.current(ed, brush)
        } else {
            None
        };
        let first = if self == List::Fonts { scroll } else { 0 };
        for row in 0..self.rows() {
            let k = first + row;
            if k >= self.items() {
                break;
            }
            let (rx, ry, rw, rh) = self.row_rect(row);
            if cur == Some(k) {
                f.blit(&a.image(&pic(CHECK)), rx + 3, ry + (rh - 12) / 2);
            }
            match self.label(k) {
                Label::Pic(id) => {
                    let img = a.image(&pic(id));
                    let (iw, ih) = (img.width as i32, img.height as i32);
                    // os tamanhos ficam no meio do lado direito do ✓; as
                    // outras figuras, logo depois dele
                    let ix = if self == List::Sizes {
                        rx + 16 + (rw - 16 - iw) / 2
                    } else {
                        rx + 23
                    };
                    f.blit(&img, ix, ry + (rh - ih) / 2);
                }
                Label::Color(c) => {
                    // faixa de 14 px com moldura preta
                    f.rect(rx + 2, ry + 2, rw - 4, 16, BLACK);
                    f.rect(rx + 3, ry + 3, rw - 6, 14, c);
                }
                Label::Text(s) => {
                    let tx = if self == List::Mix { rx + 11 } else { rx + 18 };
                    text(ed, f, &label_font(), tx, ry, rh, s);
                }
                Label::Style(st) => {
                    let font = label_font()
                        .bold(st == TextStyle::Bold)
                        .italic(st == TextStyle::Italic)
                        .underline(st == TextStyle::Underline);
                    text(ed, f, &font, rx + 20, ry, rh, st.name());
                }
                Label::Font(name) => {
                    let font = RunFont::new(Family::resolve(name), Size::Minimal);
                    text(ed, f, &font, rx + 18, ry, rh, name);
                }
            }
        }
        if self == List::Fonts {
            let [up, down] = List::font_arrows();
            f.blit(&a.image(&pic(ARROW_UP)), up.0, up.1);
            f.blit(&a.image(&pic(ARROW_DOWN)), down.0, down.1);
        }
    }
}

/// Fonte dos rótulos de texto (a Pablo Graphos, como no original).
fn label_font() -> RunFont {
    RunFont::new(Family::Pablo, Size::Minimal)
}

/// Texto centrado na altura do item.
fn text(ed: &Editor, f: &mut Frame, font: &RunFont, x: i32, y: i32, h: i32, s: &str) {
    let m = ed.fonts().line_metrics(font);
    let th = m.ascent + m.descent;
    ed.fonts()
        .draw(f, x, y + (h - th) / 2, font, s, BLACK, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_matches_the_captures() {
        // tamanhos: Gigante 95..158, Micro 334..356
        assert_eq!(List::Sizes.row_rect(0), (90, 95, 150, 64));
        assert_eq!(List::Sizes.row_rect(6), (90, 334, 150, 23));
        // cores: 16 faixas de passo 21, de y 94 a 430
        let (_, y, _, h) = List::Colors.rect();
        assert_eq!((y, y + h - 1), (94, 430));
        // efeitos e Mix: 7 itens até y 241
        assert_eq!(List::Mix.rect().3, 241 - 94 + 1);
        // fontes: 5 linhas e as setas até y 214
        assert_eq!(List::Fonts.rect().3, 214 - 94 + 1);
        for l in List::ALL {
            let (x, y, w, h) = l.rect();
            assert!(x >= 0 && x + w <= 640 && y + h <= 456, "{l:?}");
            assert!(l.rows() <= l.items());
        }
    }

    #[test]
    fn each_item_has_its_own_brush() {
        for l in List::ALL {
            for a in 0..l.items() {
                for b in a + 1..l.items() {
                    assert_ne!(l.brush(a), l.brush(b), "{l:?} {a} {b}");
                }
            }
        }
        assert_eq!(
            List::Effects.brush(3),
            Brush::Char(CharChange::Effect(Some(Effect::Sparkle)))
        );
        assert_eq!(List::Mix.brush(0), Brush::Mix(Mix::Crazy));
    }
}
