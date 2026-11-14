//! Preparação da página (`office-ferramentas.md` §3): página 1 com o rolo
//! FUNDO, Cria-colunas, Alto ou largo, Margens e Nº de página; página 2 com
//! os três rolos BORDA. Por enquanto a faixa só é desenhada (o interruptor
//! troca de página); o resto é a tarefa W14.

use super::editor::Editor;
use super::tools::{
    Cx, SND_SWITCH, SWITCH, SWITCH_RECT, StripButton, ToolId, ToolStrip, WHAT_PAGE, inside,
    press_static, strip_button,
};
use crate::Frame;
use crate::assets::Assets;

const PAGE1: [StripButton; 5] = [
    strip_button(
        0x0821,
        Some(0x0823),
        (38, 57, 120, 42),
        (WHAT_PAGE, 2),
        "Fundo",
    ),
    strip_button(
        0x086b,
        None,
        (188, 61, 49, 34),
        (WHAT_PAGE, 4),
        "Cria-colunas",
    ),
    strip_button(
        0x08c3,
        None,
        (267, 61, 49, 34),
        (WHAT_PAGE, 6),
        "Alto ou largo",
    ),
    strip_button(0x0869, None, (346, 61, 49, 34), (WHAT_PAGE, 8), "Margens"),
    strip_button(
        0x0875,
        None,
        (425, 61, 49, 34),
        (WHAT_PAGE, 9),
        "Nº de página",
    ),
];

const PAGE2: [StripButton; 3] = [
    strip_button(
        0x0953,
        Some(0x0955),
        (104, 57, 108, 42),
        (WHAT_PAGE, 3),
        "Borda 1",
    ),
    strip_button(
        0x0957,
        Some(0x0959),
        (212, 57, 94, 42),
        (WHAT_PAGE, 3),
        "Borda 2",
    ),
    strip_button(
        0x0825,
        Some(0x0827),
        (306, 57, 106, 42),
        (WHAT_PAGE, 3),
        "Borda 3",
    ),
];

#[derive(Default)]
pub struct PageStrip {
    /// Página 2 da faixa (bordas).
    more: bool,
}

impl PageStrip {
    fn buttons(&self) -> &'static [StripButton] {
        if self.more { &PAGE2 } else { &PAGE1 }
    }
}

impl ToolStrip for PageStrip {
    fn id(&self) -> ToolId {
        ToolId::Page
    }

    fn press(&mut self, _ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if inside(SWITCH_RECT, x, y) {
            cx.sound(SND_SWITCH);
            cx.help((WHAT_PAGE, 1));
            self.more = !self.more;
            return true;
        }
        press_static(self.buttons(), cx, x, y)
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(self.buttons(), f, a);
        SWITCH.draw(f, a);
    }
}
