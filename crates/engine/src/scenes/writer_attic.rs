//! Sótão do Creative Writer (tarefa T10; `cw/report/building-salas.md`
//! §7): tela preta, aro `7d00` (cor `7937`) em (347,24) — Ctrl leva ao
//! Escritório —, onze regiões que tocam um som sorteado e acendem por
//! ~0,3 s um dos 36 objetos "de lanterna" (`7d03`…`7d8d`).
//!
//! Escreva aqui o `RoomSpec` do sótão (aro em `hotspots`, `arrival_sound`
//! `287`) e os dados do escuro; os ganchos `attic::draw_in` e
//! `attic::dark_sound_in` recebem o programa. Até lá o sótão do Writer é o
//! do Fine Artist.

use super::room_spec::RoomSpec;
use super::rooms_data;

/// O sótão do Writer.
pub const ATTIC: &RoomSpec = &rooms_data::ATTIC_ROOM;
