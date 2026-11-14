//! Transformadores, a mão com tinta (`office-ferramentas.md` §7):
//! Mover-modificar, Dimensionar tudo/lateral/vertical, Agente Secreto 009,
//! Coloca-figuras (Maçã) e Flutua-texto. Por enquanto a faixa só é
//! desenhada; o resto é a tarefa W13.

use super::editor::Editor;
use super::tools::{
    Cx, StripButton, ToolId, ToolStrip, WHAT_TRANSFORMERS, press_static, strip_button,
};
use crate::Frame;
use crate::assets::Assets;

const W: u16 = WHAT_TRANSFORMERS;

const STRIP: [StripButton; 7] = [
    strip_button(
        0x0923,
        Some(0x0929),
        (33, 58, 36, 39),
        (W, 1),
        "Mover-modificar",
    ),
    strip_button(
        0x0857,
        Some(0x0859),
        (94, 54, 56, 48),
        (W, 2),
        "Dimensionar tudo",
    ),
    strip_button(
        0x085b,
        Some(0x085d),
        (150, 54, 37, 48),
        (W, 3),
        "Dimensionar lateral",
    ),
    strip_button(
        0x085f,
        Some(0x0861),
        (187, 54, 66, 48),
        (W, 4),
        "Dimensionar vertical",
    ),
    strip_button(
        0x0807,
        Some(0x0809),
        (278, 59, 37, 38),
        (W, 5),
        "Agente Secreto 009",
    ),
    strip_button(0x0865, None, (340, 61, 67, 34), (W, 6), "Coloca-figuras"),
    strip_button(
        0x08c7,
        Some(0x08cb),
        (432, 56, 74, 43),
        (W, 7),
        "Flutua-texto",
    ),
];

#[derive(Default)]
pub struct TransformersStrip;

impl ToolStrip for TransformersStrip {
    fn id(&self) -> ToolId {
        ToolId::Transformers
    }

    fn press(&mut self, _ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        press_static(&STRIP, cx, x, y)
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
    }
}
