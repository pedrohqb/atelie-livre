//! Ovo desfazer (`office-ferramentas.md` §5): faixa com DESFAZER, a fila de
//! ovos e REFAZER. A pilha guarda até 20 ações ([`crate::writer::undo`]); a
//! fila mostra um ovo chocado (`08e3`) por ação que pode ser desfeita e um
//! inteiro (`08df`) por ação que pode ser refeita, a partir de (144,68),
//! de 13 em 13 px. Ctrl+Z (em qualquer ferramenta) alterna desfazer e
//! refazer.

use super::editor::Editor;
use super::tools::{Cx, StripButton, ToolId, ToolStrip, WHAT_EGG, pic, strip_button};
use crate::Frame;
use crate::assets::Assets;
use crate::writer::undo::{Egg, UndoStack};

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

/// Sons do desfazer e do refazer (`0217`/`022f`/`0294` têm o mesmo
/// tamanho: o refazer é um deles).
pub const SND_UNDO: u16 = 0x020e;
pub const SND_REDO: u16 = 0x0217;

/// Máscara de cada ovo (duas figuras adiante).
fn egg_mask(e: Egg) -> u16 {
    e.pic() + 2
}

#[derive(Default)]
pub struct EggStrip;

impl ToolStrip for EggStrip {
    fn id(&self) -> ToolId {
        ToolId::Egg
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        match StripButton::find(&STRIP, x, y) {
            Some(0) => {
                if ed.undo_step() {
                    cx.sound(SND_UNDO);
                }
                true
            }
            Some(_) => {
                if ed.redo_step() {
                    cx.sound(SND_REDO);
                }
                true
            }
            None => false,
        }
    }

    fn draw_strip(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
        for (k, e) in ed.undo.eggs().into_iter().enumerate() {
            let (x, y) = UndoStack::<()>::egg_pos(k);
            f.blit_masked(&a.image(&pic(e.pic())), &a.image(&pic(egg_mask(e))), x, y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn egg_masks_are_two_pictures_ahead() {
        assert_eq!(egg_mask(Egg::Hatched), 0x08e5);
        assert_eq!(egg_mask(Egg::Whole), 0x08e1);
    }
}
