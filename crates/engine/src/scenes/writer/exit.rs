//! Porta Saída do Escritório (`office.md` §7). No original abre a porta
//! (retângulo preto), toca `02d6`+`02fe` e mostra o "Epa!" (`WHAT 0203[0]`):
//! salvar, jogar fora ou continuar; depois vai à sala Escritório.
//!
//! Hoje a porta só encerra (o texto é guardado em `leave`); o "Epa!", o
//! nome, o `.MAX` e a volta à sala são a tarefa W21.

use super::editor::Editor;
use super::tools::{Cx, Pic, Rect};
use crate::Frame;
use crate::assets::Assets;
use crate::scene::Effect;

/// Porta fechada e o retângulo do clique.
pub const DOOR: Pic = Pic::masked(0x023d, 0x023f, 581, 16);
pub const DOOR_RECT: Rect = (581, 16, 55, 88);

/// Clique na porta.
pub fn click(_ed: &mut Editor, cx: &mut Cx) {
    cx.fx.push(Effect::Quit);
}

pub fn draw(f: &mut Frame, a: &Assets) {
    DOOR.draw(f, a);
}
