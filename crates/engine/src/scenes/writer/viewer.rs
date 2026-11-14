//! Visor de páginas (`office-ferramentas.md` §8): página 1 com Página
//! inteira, Aumentar visão, MENOS ZOOM, a régua de zoom com o bichinho e
//! MAIS ZOOM; página 2 com Visão normal, Duas páginas, Todas as páginas e
//! Lado a lado. Por enquanto a faixa só é desenhada (o interruptor troca de
//! página); as vistas e a rolagem da barra de baixo são a tarefa W15.

use super::editor::Editor;
use super::tools::{
    Cx, Pic, SND_SWITCH, SWITCH, SWITCH_RECT, StripButton, ToolId, ToolStrip, WHAT_VIEWER, inside,
    press_static, strip_button,
};
use crate::Frame;
use crate::assets::Assets;

const PAGE1: [StripButton; 4] = [
    strip_button(
        0x08a9,
        None,
        (35, 58, 60, 39),
        (WHAT_VIEWER, 5),
        "Página inteira",
    ),
    strip_button(
        0x08ad,
        None,
        (122, 58, 60, 39),
        (WHAT_VIEWER, 6),
        "Aumentar visão",
    ),
    strip_button(
        0x0891,
        Some(0x0893),
        (209, 64, 61, 28),
        (WHAT_VIEWER, 7),
        "Menos zoom",
    ),
    strip_button(
        0x0895,
        Some(0x0897),
        (434, 63, 44, 29),
        (WHAT_VIEWER, 8),
        "Mais zoom",
    ),
];

/// Régua de zoom ("a a a a a a") e o bichinho que marca o nível.
const RULER: Pic = Pic::opaque(0x0885, 278, 73);
const BUG: Pic = Pic::masked(0x092c, 0x092d, 364, 57);

const PAGE2: [StripButton; 4] = [
    strip_button(
        0x0899,
        Some(0x089b),
        (55, 58, 66, 39),
        (WHAT_VIEWER, 1),
        "Visão normal",
    ),
    strip_button(
        0x089d,
        None,
        (168, 58, 66, 39),
        (WHAT_VIEWER, 2),
        "Duas páginas",
    ),
    strip_button(
        0x08a1,
        None,
        (281, 58, 66, 39),
        (WHAT_VIEWER, 3),
        "Todas as páginas",
    ),
    strip_button(
        0x08a5,
        None,
        (394, 58, 66, 39),
        (WHAT_VIEWER, 4),
        "Lado a lado",
    ),
];

#[derive(Default)]
pub struct ViewerStrip {
    more: bool,
}

impl ToolStrip for ViewerStrip {
    fn id(&self) -> ToolId {
        ToolId::Viewer
    }

    fn press(&mut self, _ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if inside(SWITCH_RECT, x, y) {
            cx.sound(SND_SWITCH);
            self.more = !self.more;
            return true;
        }
        press_static(if self.more { &PAGE2 } else { &PAGE1 }, cx, x, y)
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        if self.more {
            StripButton::draw_all(&PAGE2, f, a);
        } else {
            StripButton::draw_all(&PAGE1, f, a);
            RULER.draw(f, a);
            BUG.draw(f, a);
        }
        SWITCH.draw(f, a);
    }
}
