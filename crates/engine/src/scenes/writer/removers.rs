//! Re-movedores, cola e tesoura (`office-ferramentas.md` §6): Recortar,
//! Copiar, Colar, Remove-sons e Remove-figuras (o "borrão" verde), Ponto de
//! inserção e Jogar tudo fora (explosão `0845` em (435,55)).
//!
//! Hoje só a explosão funciona (apaga tudo, desfazível, ao ser clicada);
//! no original ela é escolhida e depois se clica na página. O resto é a
//! tarefa W12 (o McZee ajuda nas três primeiras vezes mesmo desligado:
//! [`Cx::help_even_off`]).

use super::editor::Editor;
use super::tools::{Cx, StripButton, ToolId, ToolStrip, WHAT_REMOVERS, press_static, strip_button};
use crate::Frame;
use crate::assets::Assets;

const W: u16 = WHAT_REMOVERS;

const STRIP: [StripButton; 6] = [
    strip_button(0x0839, Some(0x083b), (58, 54, 56, 48), (W, 1), "Recortar"),
    strip_button(0x083d, Some(0x083f), (114, 54, 37, 48), (W, 2), "Copiar"),
    strip_button(0x0841, Some(0x0843), (151, 54, 44, 48), (W, 3), "Colar"),
    strip_button(
        0x0849,
        Some(0x084b),
        (195, 54, 45, 48),
        (W, 6),
        "Remove-sons",
    ),
    strip_button(
        0x0851,
        Some(0x0853),
        (240, 54, 66, 48),
        (W, 7),
        "Remove-figuras",
    ),
    strip_button(
        0x0831,
        Some(0x0833),
        (356, 61, 29, 34),
        (W, 13),
        "Ponto de inserção",
    ),
];

/// Jogar tudo fora: a explosão.
pub const EXPLOSION: StripButton = strip_button(
    0x0845,
    Some(0x0847),
    (435, 55, 45, 45),
    (W, 9),
    "Jogar tudo fora",
);
const SND_ALL_GONE: u16 = 0x025f;

#[derive(Default)]
pub struct RemoversStrip;

impl ToolStrip for RemoversStrip {
    fn id(&self) -> ToolId {
        ToolId::Removers
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if super::tools::inside(EXPLOSION.rect(), x, y) {
            if let Some(h) = EXPLOSION.help {
                cx.help(h);
            }
            if ed.clear_all() {
                cx.sound(SND_ALL_GONE);
            }
            return true;
        }
        press_static(&STRIP, cx, x, y)
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
        EXPLOSION.pic.draw(f, a);
    }
}
