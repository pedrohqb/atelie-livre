//! Seleciona-sons (`office-ferramentas.md` §2): três teatros de sons por
//! página, controle de volume, Cachorrinho, Gravador e setas de página. Por
//! enquanto a faixa só é desenhada; associar sons é a tarefa W11.

use super::editor::Editor;
use super::stickers::draw_page_arrows;
use super::tools::{Cx, StripButton, ToolId, ToolStrip, WHAT_SOUNDS, press_static, strip_button};
use crate::Frame;
use crate::assets::Assets;

const STRIP: [StripButton; 5] = [
    strip_button(
        0x024d,
        Some(0x024f),
        (20, 59, 112, 37),
        (WHAT_SOUNDS, 2),
        "Teatro de sons",
    ),
    strip_button(
        0x024d,
        Some(0x024f),
        (133, 59, 112, 37),
        (WHAT_SOUNDS, 2),
        "Teatro de sons",
    ),
    strip_button(
        0x024d,
        Some(0x024f),
        (246, 59, 112, 37),
        (WHAT_SOUNDS, 2),
        "Teatro de sons",
    ),
    strip_button(
        0x025d,
        Some(0x0263),
        (405, 62, 37, 32),
        (WHAT_SOUNDS, 9),
        "Cachorrinho",
    ),
    strip_button(
        0x0265,
        Some(0x0267),
        (454, 57, 43, 41),
        (WHAT_SOUNDS, 5),
        "Gravador",
    ),
];

#[derive(Default)]
pub struct SoundsStrip;

impl ToolStrip for SoundsStrip {
    fn id(&self) -> ToolId {
        ToolId::Sounds
    }

    fn press(&mut self, _ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        press_static(&STRIP, cx, x, y)
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
        draw_page_arrows(f, a, 1);
    }
}
