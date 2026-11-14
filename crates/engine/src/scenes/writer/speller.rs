//! Corretor e troca-palavras, a abelha (`office-ferramentas.md` §4): PULAR
//! PARA, CORRIGIR, TROCAR e o Embaralhador, cada um abrindo um painel no
//! lugar da faixa. Por enquanto a faixa só é desenhada; os painéis e os
//! dicionários são as tarefas W16/W17.

use super::editor::Editor;
use super::tools::{Cx, StripButton, ToolId, ToolStrip, WHAT_SPELLER, press_static, strip_button};
use crate::Frame;
use crate::assets::Assets;

const STRIP: [StripButton; 4] = [
    strip_button(
        0x08cd,
        Some(0x08cf),
        (59, 59, 94, 38),
        (WHAT_SPELLER, 1),
        "Pular para",
    ),
    strip_button(
        0x082d,
        Some(0x082f),
        (204, 59, 89, 37),
        (WHAT_SPELLER, 4),
        "Corrigir",
    ),
    strip_button(
        0x0835,
        Some(0x0837),
        (344, 57, 54, 42),
        (WHAT_SPELLER, 5),
        "Trocar",
    ),
    strip_button(
        0x08b6,
        Some(0x08bb),
        (449, 56, 29, 43),
        (WHAT_SPELLER, 9),
        "Embaralhador",
    ),
];

#[derive(Default)]
pub struct SpellerStrip;

impl ToolStrip for SpellerStrip {
    fn id(&self) -> ToolId {
        ToolId::Speller
    }

    fn press(&mut self, _ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        press_static(&STRIP, cx, x, y)
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
    }
}
