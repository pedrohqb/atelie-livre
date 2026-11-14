//! Porta Saída do Escritório (`office.md` §7): o clique abre a porta (um
//! retângulo preto, `office_134_saida`), toca `02d6`+`02fe` e mostra o
//! "Epa!" (`WHAT 0203[0]`): salvar, jogar fora ou continuar. Com nome,
//! "Salvar" pergunta `0202[3]` (as duas versões ou só a nova); sem nome, o
//! título vira campo de nome. Depois vai à sala Escritório. O fluxo fica em
//! [`super::Writer`] (`open_epa`, `dialog_click`, `finish_rename`).

use super::tools::{Pic, Rect};
use crate::Frame;
use crate::assets::Assets;

/// Porta fechada e o retângulo do clique.
pub const DOOR: Pic = Pic::masked(0x023d, 0x023f, 581, 16);
pub const DOOR_RECT: Rect = (581, 16, 55, 88);
/// Porta aberta (vão preto com o batente), no mesmo lugar.
pub const DOOR_OPEN: Pic = Pic::opaque(0x0241, 581, 16);

pub fn draw(f: &mut Frame, a: &Assets, open: bool) {
    if open {
        DOOR_OPEN.draw(f, a);
    } else {
        DOOR.draw(f, a);
    }
}
