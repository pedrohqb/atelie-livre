//! Imprimir, a torradeira (`office-ferramentas.md` §9): "imprimir depressa"
//! e "fazer escolhas" (máquina de impressão do McZee). Por enquanto a faixa
//! só é desenhada; a impressão é a tarefa W18.

use super::editor::Editor;
use super::tools::{Cx, StripButton, ToolId, ToolStrip, WHAT_PRINT, press_static, strip_button};
use crate::Frame;
use crate::assets::Assets;

const STRIP: [StripButton; 2] = [
    strip_button(
        0x02ef,
        Some(0x02f1),
        (133, 60, 79, 35),
        (WHAT_PRINT, 0),
        "Imprimir depressa",
    ),
    strip_button(
        0x02f3,
        Some(0x02f5),
        (337, 59, 70, 37),
        (WHAT_PRINT, 0),
        "Fazer escolhas",
    ),
];

#[derive(Default)]
pub struct PrintStrip;

impl ToolStrip for PrintStrip {
    fn id(&self) -> ToolId {
        ToolId::Print
    }

    fn press(&mut self, _ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        press_static(&STRIP, cx, x, y)
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
    }
}
