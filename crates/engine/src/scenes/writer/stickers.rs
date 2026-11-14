//! Seleciona-adesivos (`office-ferramentas.md` §1): rolos de clip-art das
//! bibliotecas do Writer, tira de filme com os adesivos de um rolo, setas
//! de página dos rolos, Cachorrinho e Cavalete de lápis de cera.
//!
//! Hoje o adesivo escolhido vira um objeto solto com o canto no ponto de
//! inserção ([`Editor::insert_sticker`]); colar com um clique na página,
//! o Cachorrinho e o Cavalete são a tarefa W10.

use super::editor::Editor;
use super::tools::{
    Cx, Pic, Rect, SND_CHOOSE, SND_SWITCH, StripButton, ToolId, ToolStrip, WHAT_STICKERS, inside,
    pic, unavailable,
};
use crate::Frame;
use crate::assets::Assets;
use crate::scenes::stickers::{blit_clipped, blit_part};

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const YELLOW: [u8; 3] = [255, 238, 76];

// tira de rolos e filme, como na pintura
pub const ROLLS_PER_PAGE: usize = 7;
const SLOT_X0: i32 = 23;
const SLOT_W: i32 = 52;
const STRIP_Y: i32 = 58;
const FILM_Y: i32 = 99;
const FILM_H: i32 = 368;
const FILM_W: i32 = 42;
const FILM_CELLS: usize = 8;
/// Setas de página dos rolos, com o número da página no meio (fundo `08d9`).
pub const ARROW_UP: Rect = (511, 56, 16, 14);
pub const ARROW_DOWN: Rect = (511, 86, 16, 14);
const ARROWS_BACK: Pic = Pic::masked(0x08d9, 0x08dd, 510, 56);
/// Abrir a tira de filme de um rolo.
const SND_ROLL: u16 = 0x0233;

/// Cachorrinho (procura figuras) e Cavalete de lápis de cera.
const BUTTONS: [StripButton; 2] = [
    StripButton {
        pic: Pic::masked(0x025d, 0x0263, 414, 62),
        w: 37,
        h: 32,
        help: Some((WHAT_STICKERS, 11)),
        name: "Cachorrinho",
    },
    StripButton {
        pic: Pic::masked(0x0881, 0x0883, 459, 57),
        w: 43,
        h: 41,
        help: Some((WHAT_STICKERS, 6)),
        name: "Cavalete de lápis de cera",
    },
];

fn slot_rect(k: usize) -> Rect {
    (SLOT_X0 + SLOT_W * k as i32, STRIP_Y, SLOT_W, 42)
}

/// Geometria da tira de filme sob o rolo da casa `slot`.
pub struct Film {
    x: i32,
}

impl Film {
    pub fn rect(&self) -> Rect {
        (self.x, FILM_Y, FILM_W, FILM_H)
    }

    pub fn cell(&self, j: usize) -> Rect {
        (self.x + 4, 103 + 41 * j as i32, 34, 34)
    }

    fn up(&self) -> Rect {
        (self.x + 6, 452, 12, 12)
    }

    fn down(&self) -> Rect {
        (self.x + 24, 452, 12, 12)
    }
}

#[derive(Default)]
pub struct StickersStrip {
    roll_page: usize,
    roll: usize,
    film_open: bool,
    scroll: usize,
}

impl StickersStrip {
    pub fn film_open(&self) -> bool {
        self.film_open
    }

    pub fn film(&self) -> Film {
        Film {
            x: 33 + SLOT_W * (self.roll % ROLLS_PER_PAGE) as i32,
        }
    }

    fn roll_pages(ed: &Editor) -> usize {
        ed.rolls.len().div_ceil(ROLLS_PER_PAGE).max(1)
    }

    fn draw_film(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        let film = self.film();
        let (fx, fy, fw, fh) = film.rect();
        f.rect(fx, fy, fw, fh, BLACK);
        for y in fy + 1..fy + fh - 1 {
            for x in fx + 1..fx + fw - 1 {
                f.put(x, y, if (x + y) % 2 == 0 { YELLOW } else { WHITE });
            }
        }
        let Some(roll) = ed.rolls.get(self.roll) else {
            return;
        };
        for j in 0..FILM_CELLS {
            let Some(item) = roll.items.get(self.scroll + j) else {
                break;
            };
            let r = film.cell(j);
            f.rect(r.0, r.1, r.2, r.3, BLACK);
            f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, WHITE);
            if let Some(t) = &item.preview {
                let (tw, th) = (t.width as i32, t.height as i32);
                let inner = (r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2);
                let (x, y) = (r.0 + r.2 / 2 - tw / 2, r.1 + r.3 / 2 - th / 2);
                blit_clipped(f, t, t, x, y, inner);
            }
        }
        let (u, d) = (film.up(), film.down());
        f.blit_masked(&a.image(&pic(0x02db)), &a.image(&pic(0x02dd)), u.0, u.1);
        f.blit_masked(&a.image(&pic(0x02df)), &a.image(&pic(0x02e1)), d.0, d.1);
    }
}

impl ToolStrip for StickersStrip {
    fn id(&self) -> ToolId {
        ToolId::Stickers
    }

    fn deactivate(&mut self, _ed: &mut Editor, _cx: &mut Cx) {
        self.film_open = false;
    }

    fn activate(&mut self, _ed: &mut Editor, _cx: &mut Cx, _again: bool) {
        self.film_open = false;
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if self.film_open {
            let film = self.film();
            if inside(film.rect(), x, y) {
                let n = ed.rolls.get(self.roll).map_or(0, |r| r.items.len());
                let max_scroll = n.saturating_sub(FILM_CELLS);
                if inside(film.up(), x, y) {
                    self.scroll = self.scroll.saturating_sub(1);
                } else if inside(film.down(), x, y) {
                    self.scroll = (self.scroll + 1).min(max_scroll);
                } else if let Some(j) = (0..FILM_CELLS).find(|&j| inside(film.cell(j), x, y)) {
                    let item = self.scroll + j;
                    if item < n {
                        cx.sound(SND_CHOOSE);
                        let roll = ed.rolls[self.roll].id;
                        ed.insert_sticker(roll, item);
                        self.film_open = false;
                    }
                }
                return true;
            }
        }
        if inside(ARROW_UP, x, y) {
            cx.sound(SND_SWITCH);
            let pages = Self::roll_pages(ed);
            self.roll_page = (self.roll_page + pages - 1) % pages;
            self.film_open = false;
            return true;
        }
        if inside(ARROW_DOWN, x, y) {
            cx.sound(SND_SWITCH);
            self.roll_page = (self.roll_page + 1) % Self::roll_pages(ed);
            self.film_open = false;
            return true;
        }
        if let Some(k) = (0..ROLLS_PER_PAGE).find(|&k| inside(slot_rect(k), x, y)) {
            let roll = self.roll_page * ROLLS_PER_PAGE + k;
            if roll < ed.rolls.len() {
                if self.film_open && roll == self.roll {
                    self.film_open = false;
                } else {
                    cx.sound(SND_ROLL);
                    cx.help((WHAT_STICKERS, 2));
                    self.roll = roll;
                    self.scroll = 0;
                    self.film_open = true;
                }
            }
            return true;
        }
        if let Some(k) = StripButton::find(&BUTTONS, x, y) {
            unavailable(&BUTTONS[k], cx);
            return true;
        }
        false
    }

    /// Faixa de opções dos adesivos: rolos da página, Cachorrinho, Cavalete e setas.
    fn draw_strip(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        let left = a.image(&pic(0x0245));
        blit_part(f, &left, &a.image(&pic(0x0247)), 9, STRIP_Y, (0, 14));
        let right = a.image(&pic(0x0239));
        let end = SLOT_X0 + SLOT_W * ROLLS_PER_PAGE as i32;
        blit_part(
            f,
            &right,
            &a.image(&pic(0x023b)),
            end - 52,
            STRIP_Y,
            (52, 64),
        );
        for k in 0..ROLLS_PER_PAGE {
            let roll = self.roll_page * ROLLS_PER_PAGE + k;
            let Some(r) = ed.rolls.get(roll) else { break };
            let rect = slot_rect(k);
            f.blit_masked(
                &a.image(&pic(0x0249)),
                &a.image(&pic(0x024b)),
                rect.0,
                rect.1,
            );
            if let Some(icon) = &r.icon {
                let (w, h) = (icon.width as i32, icon.height as i32);
                f.blit(icon, rect.0 + 26 - w / 2, rect.1 + 21 - h / 2);
            }
        }
        StripButton::draw_all(&BUTTONS, f, a);
        draw_page_arrows(f, a, self.roll_page + 1);
    }

    fn draw_front(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        if self.film_open {
            self.draw_film(ed, f, a);
        }
    }
}

/// Setas ▲▼ das faixas de rolos (Adesivos, Sons) com o número da página.
pub fn draw_page_arrows(f: &mut Frame, a: &Assets, page: usize) {
    ARROWS_BACK.draw(f, a);
    f.blit_masked(
        &a.image(&pic(0x03b3)),
        &a.image(&pic(0x03b5)),
        ARROW_UP.0,
        ARROW_UP.1,
    );
    f.blit_masked(
        &a.image(&pic(0x03b7)),
        &a.image(&pic(0x03b9)),
        ARROW_DOWN.0,
        ARROW_DOWN.1,
    );
    f.rect(511, 72, 16, 12, BLACK);
    f.rect(512, 73, 14, 10, WHITE);
    f.text(515, 74, &page.to_string(), BLACK);
}
