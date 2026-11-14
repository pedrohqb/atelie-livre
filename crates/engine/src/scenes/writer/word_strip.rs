//! Ferramenta Palavra (`office-palavra.md`): faixa com Seleciona-letras,
//! Dimensiona-palavras, Seleciona-cores, Seleciona-estilos, Posicionador,
//! Aplicador, Ponto de inserção, FORMA e o interruptor "Mais escolhas".
//!
//! Hoje as listas mudam o estilo de digitação (e o alinhamento do
//! parágrafo); o pincel arrastado sobre as palavras, a página 2 da faixa, o
//! Aplicador, o Ponto de inserção e a FORMA são a tarefa W5/W7.

use super::editor::Editor;
use super::tools::{
    Cx, Rect, SND_CHOOSE, SND_OPTION, StripButton, ToolId, ToolStrip, WHAT_WORD, inside, pic,
    strip_button, unavailable,
};
use crate::Frame;
use crate::assets::Assets;
use crate::paint::COLORS;
use crate::writer::{Align, Size};

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const LIGHT: [u8; 3] = [140, 140, 140];
const RED: [u8; 3] = [250, 0, 57];

/// Listas suspensas da faixa da Palavra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Fonts,
    Sizes,
    Colors,
    Styles,
    Align,
}

/// Página 1 da faixa, na ordem da tela (ids e balões de `office-palavra.md` §2).
pub const STRIP: [StripButton; 9] = [
    strip_button(
        0x0275,
        Some(0x0277),
        (24, 61, 49, 34),
        (WHAT_WORD, 3),
        "Seleciona-letras",
    ),
    strip_button(
        0x0811,
        None,
        (89, 61, 49, 34),
        (WHAT_WORD, 4),
        "Dimensiona-palavras",
    ),
    strip_button(
        0x027b,
        None,
        (154, 61, 49, 34),
        (WHAT_WORD, 5),
        "Seleciona-cores",
    ),
    strip_button(
        0x0279,
        None,
        (219, 61, 49, 34),
        (WHAT_WORD, 7),
        "Seleciona-estilos",
    ),
    strip_button(
        0x0bc3,
        None,
        (284, 61, 49, 34),
        (WHAT_WORD, 8),
        "Posicionador",
    ),
    strip_button(
        0x080b,
        Some(0x080d),
        (349, 58, 33, 39),
        (WHAT_WORD, 9),
        "Aplicador",
    ),
    strip_button(
        0x0831,
        Some(0x0833),
        (398, 61, 29, 34),
        (WHAT_WORD, 10),
        "Ponto de inserção",
    ),
    strip_button(
        0x0269,
        Some(0x026b),
        (443, 57, 46, 41),
        (WHAT_WORD, 11),
        "FORMA",
    ),
    strip_button(
        0x0229,
        Some(0x022b),
        (508, 59, 20, 30),
        (WHAT_WORD, 25),
        "Mais escolhas",
    ),
];

/// Lista aberta por cada botão da página 1 (os outros são ações).
const LISTS: [Option<List>; 9] = [
    Some(List::Fonts),
    Some(List::Sizes),
    Some(List::Colors),
    Some(List::Styles),
    Some(List::Align),
    None,
    None,
    None,
    None,
];

/// Imagens dos itens da lista de alinhamento (Esquerda, Direita, Centro, Esquerda e direita).
const ALIGN_ITEMS: [u16; 4] = [0x0bc9, 0x0bcb, 0x0bcd, 0x0bcf];
const ALIGN_ITEM_H: i32 = 36;
const ALIGN_ITEM_W: i32 = 97;
const ITEM_H: i32 = 14;
const COLOR_ITEM_H: i32 = 8;
const LIST_Y: i32 = 105;
const STYLE_NAMES: [&str; 4] = ["Simples", "Negrito", "Inclinado", "Sublinhado"];

#[derive(Default)]
pub struct WordStrip {
    list: Option<List>,
}

impl WordStrip {
    pub fn list(&self) -> Option<List> {
        self.list
    }

    fn list_rect(list: List) -> Rect {
        let x = STRIP
            .iter()
            .zip(LISTS)
            .find(|(_, l)| *l == Some(list))
            .map_or(24, |(b, _)| b.pic.x);
        let (w, h) = match list {
            List::Fonts => (170, 2 + 3 * ITEM_H),
            List::Sizes => (100, 2 + 6 * ITEM_H),
            List::Colors => (60, 2 + COLORS.len() as i32 * COLOR_ITEM_H),
            List::Styles => (100, 2 + 4 * ITEM_H),
            List::Align => (ALIGN_ITEM_W + 2, 2 + 4 * ALIGN_ITEM_H),
        };
        (x, LIST_Y, w, h)
    }

    fn list_item_count(list: List) -> usize {
        match list {
            List::Fonts => 1,
            List::Sizes => Size::ALL.len(),
            List::Colors => COLORS.len(),
            List::Styles => STYLE_NAMES.len(),
            List::Align => Align::ALL.len(),
        }
    }

    pub fn list_item_rect(list: List, k: usize) -> Rect {
        let (x, y, w, _) = Self::list_rect(list);
        let h = match list {
            List::Colors => COLOR_ITEM_H,
            List::Align => ALIGN_ITEM_H,
            _ => ITEM_H,
        };
        (x + 1, y + 1 + h * k as i32, w - 2, h)
    }

    /// Clique com uma lista aberta: escolhe o item (se for o caso) e fecha.
    fn list_click(&mut self, ed: &mut Editor, cx: &mut Cx, list: List, x: i32, y: i32) {
        self.list = None;
        let Some(k) =
            (0..Self::list_item_count(list)).find(|&k| inside(Self::list_item_rect(list, k), x, y))
        else {
            return;
        };
        cx.sound(SND_CHOOSE);
        let mut style = ed.doc.style;
        match list {
            List::Fonts => {}
            List::Sizes => {
                style.size = Size::ALL[k];
                ed.set_style(style);
            }
            List::Colors => {
                style.color = COLORS[k];
                ed.set_style(style);
            }
            List::Styles => {
                match k {
                    0 => {
                        style.bold = false;
                        style.italic = false;
                        style.underline = false;
                    }
                    1 => style.bold = !style.bold,
                    2 => style.italic = !style.italic,
                    _ => style.underline = !style.underline,
                }
                ed.set_style(style);
            }
            List::Align => {
                ed.history.record(&ed.doc);
                ed.doc.set_align(Align::ALL[k]);
                ed.after_edit();
            }
        }
    }

    fn draw_list(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        let Some(list) = self.list else { return };
        let (x, y, w, h) = Self::list_rect(list);
        f.rect(x, y, w, h, BLACK);
        f.rect(x + 1, y + 1, w - 2, h - 2, WHITE);
        let font = a.font(8);
        let text = |f: &mut Frame, x: i32, y: i32, s: &str| match &font {
            Some(font) => {
                f.text_font(font, x, y, s, BLACK);
            }
            None => f.text(x, y + 3, s, BLACK),
        };
        let style = ed.doc.style;
        match list {
            List::Fonts => {
                let r = Self::list_item_rect(list, 0);
                f.rect(r.0, r.1, r.2, r.3, LIGHT);
                text(f, r.0 + 4, r.1 + 1, "Pablo Graphos");
                text(f, r.0 + 4, r.1 + 1 + ITEM_H, "(os outros tipos de letra");
                text(f, r.0 + 4, r.1 + 1 + 2 * ITEM_H, "ainda não existem)");
            }
            List::Sizes => {
                for (k, s) in Size::ALL.iter().enumerate() {
                    let r = Self::list_item_rect(list, k);
                    if *s == style.size {
                        f.rect(r.0, r.1, r.2, r.3, LIGHT);
                    }
                    text(f, r.0 + 4, r.1 + 1, s.name());
                }
            }
            List::Colors => {
                for (k, c) in COLORS.iter().enumerate() {
                    let r = Self::list_item_rect(list, k);
                    f.rect(r.0 + 2, r.1 + 1, r.2 - 4, r.3 - 2, ed.rgb(*c));
                    if *c == style.color {
                        f.frame_rect(r.0, r.1, r.2, r.3, 1, BLACK);
                    }
                }
            }
            List::Styles => {
                let on = [
                    !style.bold && !style.italic && !style.underline,
                    style.bold,
                    style.italic,
                    style.underline,
                ];
                for (k, name) in STYLE_NAMES.iter().enumerate() {
                    let r = Self::list_item_rect(list, k);
                    if on[k] {
                        f.rect(r.0, r.1, r.2, r.3, LIGHT);
                    }
                    text(f, r.0 + 4, r.1 + 1, name);
                }
            }
            List::Align => {
                let cur = ed.doc.para().align;
                for (k, id) in ALIGN_ITEMS.iter().enumerate() {
                    let r = Self::list_item_rect(list, k);
                    f.blit(&a.image(&pic(*id)), r.0, r.1);
                    if Align::ALL[k] == cur {
                        f.frame_rect(r.0, r.1, r.2, r.3, 1, RED);
                    }
                }
            }
        }
    }
}

impl ToolStrip for WordStrip {
    fn id(&self) -> ToolId {
        ToolId::Word
    }

    fn deactivate(&mut self, _ed: &mut Editor, _cx: &mut Cx) {
        self.list = None;
    }

    fn modal(&self) -> bool {
        self.list.is_some()
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if let Some(list) = self.list {
            self.list_click(ed, cx, list, x, y);
            return true;
        }
        let Some(k) = StripButton::find(&STRIP, x, y) else {
            return false;
        };
        match LISTS[k] {
            Some(list) => {
                cx.sound(SND_OPTION);
                if let Some(h) = STRIP[k].help {
                    cx.help(h);
                }
                self.list = Some(list);
            }
            None => unavailable(&STRIP[k], cx),
        }
        true
    }

    fn key(&mut self, _ed: &mut Editor, _cx: &mut Cx, key: crate::scene::Key, _ctrl: bool) -> bool {
        // Esc fecha a lista sem escolher (sem som)
        if key == crate::scene::Key::Escape && self.list.is_some() {
            self.list = None;
            return true;
        }
        false
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
    }

    fn draw_front(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        self.draw_list(ed, f, a);
    }
}
