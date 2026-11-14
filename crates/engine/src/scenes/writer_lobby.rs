//! Saguão do Creative Writer (tarefa T5; `cw/report/building-salas.md` §1):
//! fundo `6805`, quadro de andares com cinco zonas (Ideias, Projetos,
//! Escritório, setinha da Biblioteca e a faixa de baixo, que leva ao
//! sótão), mostrador do elevador que só toca `233` (`Action::Sound`), seta
//! "Biblioteca" (`Action::Gallery(0x2fb)`), skate "LÁ FORA", portas e botão
//! do elevador, balão de chegada `0bb8[1]` com as dicas `[2]`–`[5]`.
//!
//! Escreva aqui um `RoomSpec` com `kit: &WRITER_KIT` (`writer_rooms`), as
//! dicas da sala em `tips`, e troque [`LOBBY`] por ele. Até lá o saguão do
//! Writer é o do Fine Artist.

use super::room_spec::RoomSpec;
use super::rooms_data;

/// O saguão do Writer.
pub const LOBBY: &RoomSpec = &rooms_data::LOBBY_ROOM;
