//! Moldura do Escritório (`office.md` §2–§4): bordas laranja, barra de
//! ferramentas com a aba da ativa, faixa de opções, laterais da página,
//! barra de baixo, o McZee no nicho e a porta.
//!
//! Cores medidas no original: barra de ferramentas cinza-escuro (100), aba
//! ativa e faixa cinza-claro (190), laterais da página cinza (140).

use super::editor::Editor;
use super::tools::{Pic, Rect, ToolDef, ToolId};
use crate::Frame;
use crate::assets::Assets;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const DARK: [u8; 3] = [100, 100, 100];
const GRAY: [u8; 3] = [140, 140, 140];
const LIGHT: [u8; 3] = [190, 190, 190];
const ORANGE: [u8; 3] = [255, 140, 50];
const BROWN: [u8; 3] = [185, 101, 36];

/// Faixa de opções (fundo).
pub const STRIP: Rect = (5, 56, 630, 48);

/// Barra de baixo: três caixas brancas (andar, nome do documento, páginas).
const BOTTOM_Y: i32 = 461;
const BOTTOM_H: i32 = 14;
const BOX_FLOOR: (i32, i32) = (29, 168);
const BOX_TITLE: (i32, i32) = (194, 445);
const BOX_PAGES: (i32, i32) = (471, 610);
/// Cabeça vermelha (mapa do prédio) e ícone "todas as páginas".
const FLOOR_ICON: Pic = Pic::masked(0x03c3, 0x03c9, 33, 464);
const PAGES_ICON: Pic = Pic::masked(0x0c37, 0x0c39, 475, 463);
/// Setas ▲▼◀▶ da barra de baixo.
pub const ARROW_UP: Pic = Pic::masked(0x02db, 0x02dd, 543, 463);
pub const ARROW_DOWN: Pic = Pic::masked(0x02df, 0x02e1, 560, 463);
pub const ARROW_LEFT: Pic = Pic::masked(0x0819, 0x081b, 577, 463);
pub const ARROW_RIGHT: Pic = Pic::masked(0x0871, 0x0873, 594, 463);
/// Clique nas setas (12×12).
pub fn arrow_rect(p: Pic) -> Rect {
    (p.x, p.y, 12, 12)
}

/// Nome mostrado na barra de baixo enquanto o documento não tem nome
/// (`STR# 0800[0]`).
pub const DEFAULT_TITLE: &str = "Documento Novo";

/// Bordas laranja em relevo, fundo das barras e laterais da página.
pub fn draw_frame(f: &mut Frame) {
    f.fill(GRAY);
    f.rect(5, 5, 630, 50, DARK);
    let (x, y, w, h) = STRIP;
    f.rect(x, y, w, h, LIGHT);
    f.rect(5, 55, 630, 1, BLACK);
    for y0 in [0, 104, 456, 475] {
        for (dy, c) in [(0, BLACK), (1, ORANGE), (2, ORANGE), (3, BROWN), (4, BLACK)] {
            f.rect(0, y0 + dy, 640, 1, c);
        }
    }
    for (x, c) in [(0, BLACK), (1, BROWN), (2, ORANGE), (3, ORANGE), (4, BLACK)] {
        f.rect(x, 0, 1, 480, c);
    }
    for (x, c) in [
        (635, BLACK),
        (636, BROWN),
        (637, ORANGE),
        (638, ORANGE),
        (639, BLACK),
    ] {
        f.rect(x, 0, 1, 480, c);
    }
    // barra de baixo: fundo escuro entre as caixas
    f.rect(5, BOTTOM_Y - 2, 630, BOTTOM_H + 3, DARK);
    // nicho escuro do McZee
    f.rect(532, 56, 1, 48, BLACK);
    f.rect(533, 56, 47, 48, DARK);
}

/// Barra de ferramentas: aba clara atrás da ferramenta ativa.
pub fn draw_toolbar(f: &mut Frame, a: &Assets, active: ToolId) {
    for t in ToolId::ALL.map(ToolId::def) {
        if t.id == active {
            draw_tab(f, t);
            t.active.draw(f, a);
        } else {
            t.inactive.draw(f, a);
        }
    }
}

fn draw_tab(f: &mut Frame, t: &ToolDef) {
    let (x, y, w, h) = t.hit;
    f.rect(x, y, w, h + 1, LIGHT);
    f.rect(x - 1, y, 1, h, BLACK);
    f.rect(x + w, y, 1, h, BLACK);
}

fn text_in(f: &mut Frame, a: &Assets, (x0, x1): (i32, i32), s: &str) {
    match a.font(8) {
        Some(font) => {
            let w = font.width(s) as i32;
            f.text_font(&font, (x0 + x1) / 2 - w / 2, BOTTOM_Y + 1, s, BLACK);
        }
        None => {
            let w = 8 * s.chars().count() as i32;
            f.text((x0 + x1) / 2 - w / 2, BOTTOM_Y + 3, s, BLACK);
        }
    }
}

/// Barra de baixo: andar ("Escritório"), nome do documento e páginas.
pub fn draw_bottom(f: &mut Frame, a: &Assets, ed: &Editor, title: &str) {
    for (x0, x1) in [BOX_FLOOR, BOX_TITLE, BOX_PAGES] {
        f.rect(x0, BOTTOM_Y, 1, BOTTOM_H, BLACK);
        f.rect(x1, BOTTOM_Y, 1, BOTTOM_H, BLACK);
        f.rect(x0 + 1, BOTTOM_Y, x1 - x0 - 1, BOTTOM_H, WHITE);
    }
    FLOOR_ICON.draw(f, a);
    text_in(f, a, (44, 140), "Escritório");
    text_in(f, a, BOX_TITLE, title);
    PAGES_ICON.draw(f, a);
    text_in(f, a, (488, 538), &format!("Pág. {}", ed.page + 1));
    for p in [ARROW_UP, ARROW_DOWN, ARROW_LEFT, ARROW_RIGHT] {
        p.draw(f, a);
    }
}
