//! Peças comuns dos dois cavaletes da Palavra: moldura laranja, campo de
//! texto, botões do `ARTIST.PIC` e listas suspensas com setas.

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::scene::Key;
use formats::fnt::Font;

pub type Rect = (i32, i32, i32, i32);

pub const BLACK: [u8; 3] = [0, 0, 0];
pub const WHITE: [u8; 3] = [255, 255, 255];
pub const GRAY: [u8; 3] = [100, 100, 100];
const ORANGE: [u8; 3] = [255, 140, 50];
const BROWN: [u8; 3] = [185, 101, 36];
/// Fundo do texto selecionado (como no Windows do original).
const SELECTION: [u8; 3] = [0, 0, 160];
/// Espessura das barras da moldura.
pub const BAR: i32 = 5;

/// Botões dos cavaletes (`ARTIST.PIC`): figura e máscara.
pub const BTN_OK: (u16, u16) = (0x026d, 0x026f);
pub const BTN_NO: (u16, u16) = (0x0271, 0x0273);
const ARROW_UP: (u16, u16) = (0x02db, 0x02dd);
const ARROW_DOWN: (u16, u16) = (0x02df, 0x02e1);
const CHECK: u16 = 0x0295;

pub fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Artist, id)
}

/// Figura com máscara (sem a figura nos dados, nada aparece).
pub fn blit(f: &mut Frame, a: &Assets, (img, mask): (u16, u16), x: i32, y: i32) {
    if a.has_image(&pic(img)) {
        f.blit_masked(&a.image(&pic(img)), &a.image(&pic(mask)), x, y);
    }
}

/// Barra horizontal da moldura: preto, laranja, laranja, marrom, preto.
pub fn bar_h(f: &mut Frame, x: i32, y: i32, w: i32) {
    for (k, c) in [BLACK, ORANGE, ORANGE, BROWN, BLACK].iter().enumerate() {
        f.rect(x, y + k as i32, w, 1, *c);
    }
}

/// Barra vertical: preto, marrom, laranja, laranja, preto.
pub fn bar_v(f: &mut Frame, x: i32, y: i32, h: i32) {
    for (k, c) in [BLACK, BROWN, ORANGE, ORANGE, BLACK].iter().enumerate() {
        f.rect(x + k as i32, y, 1, h, *c);
    }
}

/// Escurece em xadrez o que fica fora do cavalete (barras de ferramentas,
/// faixa e colunas), como os botões "apagados" do original.
pub fn dim_outside(f: &mut Frame) {
    let mut dot = |x0: i32, y0: i32, x1: i32, y1: i32| {
        for y in y0..y1 {
            for x in x0..x1 {
                if (x + y) % 2 == 0 {
                    f.put(x, y, GRAY);
                }
            }
        }
    };
    dot(0, 0, 640, 106);
    dot(0, 106, 24, 480);
    dot(616, 106, 640, 480);
    dot(24, 456, 616, 480);
}

/// Texto numa fonte do programa (ou na 8×8 reserva).
pub fn text(f: &mut Frame, font: Option<&Font>, x: i32, y: i32, s: &str, rgb: [u8; 3]) {
    match font {
        Some(ft) => {
            f.text_font(ft, x, y, s, rgb);
        }
        None => f.text(x, y, s, rgb),
    }
}

pub fn text_w(font: Option<&Font>, s: &str) -> i32 {
    match font {
        Some(ft) => ft.width(s) as i32,
        None => 8 * s.chars().count() as i32,
    }
}

pub fn line_h(font: Option<&Font>) -> i32 {
    font.map_or(8, |ft| ft.height as i32)
}

/// Campo de texto de uma linha: o texto começa todo selecionado (digitar
/// troca tudo; Backspace apaga tudo), como no original.
#[derive(Debug, Clone)]
pub struct Field {
    pub text: String,
    pub selected: bool,
    blink: f32,
    max: usize,
}

/// Meio período do piscar do cursor.
const BLINK: f32 = 0.5;

impl Field {
    pub fn new(text: &str, max: usize) -> Field {
        Field {
            text: text.to_string(),
            selected: !text.is_empty(),
            blink: 0.0,
            max,
        }
    }

    /// Acrescenta o texto digitado (só caracteres que a fonte do programa tem).
    pub fn type_text(&mut self, s: &str) -> bool {
        let chars: Vec<char> = s
            .chars()
            .filter(|c| !c.is_control() && formats::cp1252::encode_char(*c).is_some())
            .collect();
        if chars.is_empty() {
            return false;
        }
        if std::mem::take(&mut self.selected) {
            self.text.clear();
        }
        for c in chars {
            if self.text.chars().count() < self.max {
                self.text.push(c);
            }
        }
        self.blink = 0.0;
        true
    }

    /// Backspace (e Delete) apagam a última letra, ou tudo se selecionado.
    pub fn key(&mut self, key: Key) -> bool {
        if !matches!(key, Key::Backspace | Key::Delete) {
            return false;
        }
        if std::mem::take(&mut self.selected) {
            self.text.clear();
        } else {
            self.text.pop();
        }
        self.blink = 0.0;
        true
    }

    /// Um clique no campo tira a seleção (o cursor vai para o fim).
    pub fn click(&mut self) {
        self.selected = false;
        self.blink = 0.0;
    }

    pub fn tick(&mut self, dt: f32) {
        if dt.is_finite() && dt > 0.0 {
            self.blink = (self.blink + dt) % (2.0 * BLINK);
        }
    }

    pub fn draw(&self, f: &mut Frame, font: Option<&Font>, r: Rect) {
        f.rect(r.0, r.1, r.2, r.3, WHITE);
        let (x, y) = (r.0 + 3, r.1 + 3);
        let w = text_w(font, &self.text);
        let h = line_h(font);
        if self.selected {
            f.rect(x - 1, y - 1, w + 2, h + 2, SELECTION);
            text(f, font, x, y, &self.text, WHITE);
        } else {
            text(f, font, x, y, &self.text, BLACK);
            if self.blink < BLINK {
                f.rect(x + w + 1, y, 1, h, BLACK);
            }
        }
    }
}

/// Lista suspensa: uma coluna de casas encostada embaixo em `bottom` (ou
/// em cima em `top`), com a linha de setas em cima quando não cabe tudo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct List {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub row_h: i32,
    pub n: usize,
    pub visible: usize,
    pub scroll: usize,
}

/// Altura da linha das setas.
const ARROWS_H: i32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListHit {
    Item(usize),
    Up,
    Down,
    /// Dentro da lista, fora de casas e setas.
    Blank,
}

impl List {
    /// Coluna de `n` casas centrada em `cx`, terminando em `bottom`, sem
    /// subir acima de `min_y`.
    pub fn above(cx: i32, bottom: i32, w: i32, row_h: i32, n: usize, min_y: i32) -> List {
        let room = bottom - min_y - 2;
        let visible = if n as i32 * row_h <= room {
            n
        } else {
            (((room - ARROWS_H) / row_h).max(1) as usize).min(n)
        };
        let mut l = List {
            x: (cx - w / 2).clamp(2, 638 - w),
            y: 0,
            w,
            row_h,
            n,
            visible,
            scroll: 0,
        };
        l.y = bottom - l.height();
        l
    }

    /// Coluna que desce a partir de `top` até, no máximo, `max_y`.
    pub fn below(x: i32, top: i32, w: i32, row_h: i32, n: usize, max_y: i32) -> List {
        let room = max_y - top - 2;
        let visible = if n as i32 * row_h <= room {
            n
        } else {
            (((room - ARROWS_H) / row_h).max(1) as usize).min(n)
        };
        List {
            x: x.clamp(2, 638 - w),
            y: top,
            w,
            row_h,
            n,
            visible,
            scroll: 0,
        }
    }

    fn arrows(&self) -> bool {
        self.n > self.visible
    }

    pub fn height(&self) -> i32 {
        2 + self.visible as i32 * self.row_h + if self.arrows() { ARROWS_H } else { 0 }
    }

    pub fn rect(&self) -> Rect {
        (self.x, self.y, self.w, self.height())
    }

    /// Mostra a casa `k` (rolando o mínimo).
    pub fn show(&mut self, k: usize) {
        if k < self.scroll {
            self.scroll = k;
        } else if k >= self.scroll + self.visible {
            self.scroll = k + 1 - self.visible;
        }
        self.scroll = self.scroll.min(self.n.saturating_sub(self.visible));
    }

    /// Casa da linha visível `j`.
    pub fn row_rect(&self, j: usize) -> Rect {
        let y0 = self.y + 1 + if self.arrows() { ARROWS_H } else { 0 };
        (
            self.x + 1,
            y0 + j as i32 * self.row_h,
            self.w - 2,
            self.row_h,
        )
    }

    fn up_rect(&self) -> Rect {
        (self.x + 1, self.y + 1, self.w / 2 - 1, ARROWS_H)
    }

    fn down_rect(&self) -> Rect {
        (self.x + self.w / 2, self.y + 1, self.w / 2 - 1, ARROWS_H)
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<ListHit> {
        if !inside(self.rect(), x, y) {
            return None;
        }
        if self.arrows() {
            if inside(self.up_rect(), x, y) {
                return Some(ListHit::Up);
            }
            if inside(self.down_rect(), x, y) {
                return Some(ListHit::Down);
            }
        }
        Some(
            (0..self.visible)
                .find(|&j| inside(self.row_rect(j), x, y))
                .map(|j| ListHit::Item(self.scroll + j))
                .filter(|h| matches!(h, ListHit::Item(k) if *k < self.n))
                .unwrap_or(ListHit::Blank),
        )
    }

    /// Rola uma casa (setas); falso se já está na ponta.
    pub fn scroll_by(&mut self, down: bool) -> bool {
        let max = self.n.saturating_sub(self.visible);
        let next = if down {
            (self.scroll + 1).min(max)
        } else {
            self.scroll.saturating_sub(1)
        };
        std::mem::replace(&mut self.scroll, next) != next
    }

    /// Fundo, setas e as casas visíveis (`item(f, k, casa)`).
    pub fn draw(&self, f: &mut Frame, a: &Assets, mut item: impl FnMut(&mut Frame, usize, Rect)) {
        let (x, y, w, h) = self.rect();
        f.rect(x, y, w, h, WHITE);
        f.frame_rect(x, y, w, h, 1, BLACK);
        if self.arrows() {
            let (ux, uy, uw, _) = self.up_rect();
            let (dx, dy, dw, _) = self.down_rect();
            blit(f, a, ARROW_UP, ux + uw / 2 - 6, uy + 2);
            blit(f, a, ARROW_DOWN, dx + dw / 2 - 6, dy + 2);
            f.rect(x, y + ARROWS_H, w, 1, BLACK);
        }
        for j in 0..self.visible {
            let k = self.scroll + j;
            if k >= self.n {
                break;
            }
            let r = self.row_rect(j);
            item(f, k, r);
            f.rect(r.0, r.1 + r.3 - 1, r.2, 1, BLACK);
        }
    }
}

/// Linha de menu com texto e a marca ✓ quando ligada.
pub fn menu_row(
    f: &mut Frame,
    a: &Assets,
    font: Option<&Font>,
    r: Rect,
    label: &str,
    checked: bool,
    bold: bool,
) {
    let ty = r.1 + (r.3 - line_h(font)) / 2;
    if checked {
        if a.has_image(&pic(CHECK)) {
            f.blit_masked(
                &a.image(&pic(CHECK)),
                &a.image(&pic(CHECK)),
                r.0 + 4,
                r.1 + (r.3 - 12) / 2,
            );
        } else {
            text(f, None, r.0 + 4, ty, "v", BLACK);
        }
    }
    text(f, font, r.0 + 20, ty, label, BLACK);
    if bold {
        text(f, font, r.0 + 21, ty, label, BLACK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_starts_selected_then_types_and_erases() {
        let mut f = Field::new("Ana", 10);
        assert!(f.selected);
        assert!(f.type_text("O"));
        assert_eq!(f.text, "O");
        f.type_text("i!\u{7}中");
        assert_eq!(f.text, "Oi!");
        f.key(Key::Backspace);
        assert_eq!(f.text, "Oi");
        let mut g = Field::new("Ana", 4);
        g.key(Key::Backspace);
        assert_eq!(g.text, "");
        g.type_text("abcdef");
        assert_eq!(g.text, "abcd");
        assert!(!g.key(Key::Left));
        let mut h = Field::new("Ana", 10);
        h.click();
        h.type_text("!");
        assert_eq!(h.text, "Ana!");
    }

    #[test]
    fn list_scrolls_and_hits() {
        let mut l = List::above(100, 300, 60, 20, 30, 40);
        assert!(l.visible < 30);
        assert_eq!(l.y + l.height(), 300);
        assert!(l.y >= 40);
        let r = l.row_rect(0);
        assert_eq!(l.hit(r.0 + 5, r.1 + 5), Some(ListHit::Item(0)));
        assert!(l.scroll_by(true));
        assert_eq!(l.hit(r.0 + 5, r.1 + 5), Some(ListHit::Item(1)));
        assert!(!l.scroll_by(false) || l.scroll == 0);
        l.show(29);
        assert_eq!(l.scroll + l.visible, 30);
        assert!(!l.scroll_by(true));
        assert_eq!(l.hit(l.x + 2, l.y + 4), Some(ListHit::Up));
        assert_eq!(l.hit(0, 0), None);
        let small = List::above(100, 300, 60, 20, 3, 40);
        assert_eq!((small.visible, small.height()), (3, 62));
        let b = List::below(10, 100, 50, 20, 4, 470);
        assert_eq!(b.visible, 4);
        assert_eq!(b.y, 100);
    }
}
