//! Escritório do Creative Writer — a sala do andar 2, não a área de escrita
//! (tarefa T6; `cw/report/building-salas.md` §3): fundo `680d`, quadro do
//! último documento (miniatura vinda do agente office), prancheta
//! "Documento Novo" (clique duplo → `Action::Easel` →
//! `CwScene::NewDocument`), plaquetas, balão `0bb8[26]` e dicas
//! `[27]`/`[28]`/`[0]`/`[12]`/`[66]`.
//!
//! Escreva aqui um `RoomSpec` com `kit: &WRITER_KIT` (`writer_rooms`) e
//! troque [`OFFICE`] por ele. Até lá a sala é o Ateliê do Fine Artist (o
//! clique duplo no cavalete já abre a área de escrita com um documento
//! novo, e o quadro da parede, a área de escrita).

use super::room_spec::RoomSpec;
use super::rooms_data;

/// A sala do andar 2 do Writer.
pub const OFFICE: &RoomSpec = &rooms_data::STUDIO_ROOM;
