//! Ovo desfazer (`office-ferramentas.md` §5): faixa com DESFAZER, a fila de
//! ovos (chocado = pode desfazer, inteiro = pode refazer) e REFAZER.
//!
//! Hoje o histórico tem um nível só: escolher o ovo na barra alterna
//! desfazer/refazer (comportamento anterior do port) e os dois botões da
//! faixa fazem o mesmo. A pilha de 20 ações e o contador de ovos são a
//! tarefa W9.

use super::editor::Editor;
use super::tools::{Cx, Pic, StripButton, ToolId, ToolStrip, WHAT_EGG, strip_button};
use crate::Frame;
use crate::assets::Assets;

const STRIP: [StripButton; 2] = [
    strip_button(
        0x0889,
        Some(0x088b),
        (31, 69, 90, 18),
        (WHAT_EGG, 0),
        "Desfazer",
    ),
    strip_button(
        0x088d,
        Some(0x088f),
        (427, 69, 81, 18),
        (WHAT_EGG, 0),
        "Refazer",
    ),
];

/// Primeiro ovo da fila (ação que pode ser desfeita: chocado; que pode ser
/// refeita: inteiro, `08df`/`08e1`); a fila anda de 13 em 13 px.
const EGG_HATCHED: Pic = Pic::masked(0x08e3, 0x08e5, 144, 68);
/// Som do desfazer; o do refazer (`0217`/`022f`/`0294`) é ambíguo pelo tamanho.
const SND_UNDO: u16 = 0x020e;
const SND_REDO: u16 = 0x0217;

#[derive(Default)]
pub struct EggStrip;

impl EggStrip {
    fn undo(ed: &mut Editor, cx: &mut Cx) {
        let undo = ed.history.can_undo();
        if ed.toggle_undo() {
            cx.sound(if undo { SND_UNDO } else { SND_REDO });
        }
    }
}

impl ToolStrip for EggStrip {
    fn id(&self) -> ToolId {
        ToolId::Egg
    }

    fn activate(&mut self, ed: &mut Editor, _cx: &mut Cx, _again: bool) {
        // provisório (um nível): cada clique no ovo alterna
        ed.toggle_undo();
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        match StripButton::find(&STRIP, x, y) {
            Some(_) => {
                Self::undo(ed, cx);
                true
            }
            None => false,
        }
    }

    fn draw_strip(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
        // um nível só: um ovo chocado quando há o que desfazer (W9: a fila)
        if ed.history.can_undo() {
            EGG_HATCHED.draw(f, a);
        }
    }
}
