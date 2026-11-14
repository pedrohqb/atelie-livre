//! Prédio do Creative Writer: o [`WRITER_KIT`] (o que é comum às salas do
//! Writer: `WHAT 0bb8`, sons do `WRITER.SND`, figuras da Bolsa Mágica do
//! `WRITER.PIC`, "EPA!" `0805[12]`) e o despacho por andar para os arquivos
//! de cada sala. Dados de `cw/report/building.md` §8 e
//! `building-salas.md`.
//!
//! Cada sala fica no seu arquivo, de uma tarefa só (ver
//! `docs/arquitetura.md`, "Creative Writer: specs por programa"):
//!
//! | andar | arquivo | tarefa |
//! |---|---|---|
//! | saguão (`Floor::Lobby`) | `writer_lobby` | T5 |
//! | Escritório (`Floor::Studio`) | `writer_office_room` | T6 |
//! | Projetos e Ideias (`Floor::Projects`, `Floor::Tricks`) | `writer_floors` | T7 |
//! | sótão (`Floor::Attic`) | `writer_attic` | T10 |
//!
//! Enquanto uma sala não tem os dados do Writer, o arquivo dela devolve a
//! sala do Fine Artist (com o `ARTIST_KIT`): o prédio funciona inteiro, com
//! as figuras do FA, e as saídas levam às telas do Writer (`Room` guarda o
//! programa à parte e passa os ids por `SceneId::in_program`).

use super::entry_spec::QuitSpec;
use super::room::Floor;
use super::room_objects::{BagPics, WAND_CURSOR};
use super::room_spec::{RoomKit, RoomSounds, RoomSpec};
use super::writer_entry::WRITER_QUIT;
use crate::assets::Program;

/// Sons próprios das salas do Writer (o resto é igual ao Fine Artist).
pub mod snd {
    /// Clique no McZee (13868 B; `295` e `2d3` têm o mesmo tamanho).
    pub const MCZEE: u16 = 0x281;
    /// Clique no Max (22060 B).
    pub const MAX: u16 = 0x283;
    /// Bolinha de um balão.
    pub const CHOICE: u16 = 0x2ff;
    /// Buraco do rato sem a varinha.
    pub const MOUSE_HOLE: u16 = 0x2ea;
}

/// Bolsa Mágica e o que sai dela no `WRITER.PIC` (cor, máscara = duas
/// imagens adiante, tamanho; `building.md` §8.3).
pub const WRITER_BAG_PICS: BagPics = BagPics {
    program: Program::Writer,
    closed: (0x68ef, 0x68f5, 113, 78),
    open: (0x68ff, 0x690d, 128, 56),
    suitcase: (0x6937, 0x6939, 99, 74),
    book: (0x692f, 0x6931, 85, 60),
    wand: (0x6933, 0x6935, 54, 54),
};

/// "EPA!" das salas do Writer.
const QUIT: &QuitSpec = &WRITER_QUIT;

/// O que é comum às salas do Writer.
pub const WRITER_KIT: RoomKit = RoomKit {
    program: Program::Writer,
    what: 0x0bb8,
    snd: RoomSounds {
        object: 0x21c,
        pick: 0x2f4,
        choice: snd::CHOICE,
        tip: 0x2dd,
        // não medido no Writer
        glossary: 0x2dd,
        quit: [0x2f7, 0x2fe],
        refuse: 0x217,
        // não há moldura de obras-primas no Writer
        painting: 0,
        mouse_hole: snd::MOUSE_HOLE,
        portfolio: 0x2f1,
        board: 0x304,
        mczee: snd::MCZEE,
        bag_open: 0x2cf,
        bag_close: 0x2ce,
        suitcase: 0x2f1,
        book: [0x2e2, 0x2f6],
        wand_take: 0x307,
        // não medido no Writer: o mesmo do Fine Artist
        wand_hole: 0x2ea,
        pole_up: 0x2f3,
        pole_down: 0x2f2,
        skate: 0x2fc,
        elevator_arrive: 0x311,
    },
    // as dicas ficam em cada sala (`RoomSpec::tips`)
    tips: &[],
    glossary_links: &[],
    // figura da "Tecla de atalho" no `WHAT 0bb8`: a medir (T5–T7)
    hotkey_pic: None,
    help_off: 29,
    help_on: 30,
    // registro do balão da varinha nos andares: a identificar (T13)
    wand: None,
    quit: QUIT,
    bag: &WRITER_BAG_PICS,
    // cursor da varinha do `WRITER.EXE`: a pegar no GROUP_CURSOR (T13)
    wand_cursor: WAND_CURSOR,
    // menu do McZee no elevador ≈ (140,100) (`building-salas.md` §2; T8)
    elevator_helper: (140, 100),
};

/// Sala do Writer em cada andar.
pub fn room(floor: Floor) -> &'static RoomSpec {
    match floor {
        Floor::Lobby => super::writer_lobby::LOBBY,
        Floor::Studio => super::writer_office_room::OFFICE,
        Floor::Projects => super::writer_floors::PROJECTS,
        Floor::Tricks => super::writer_floors::IDEAS,
        Floor::Attic => super::writer_attic::ATTIC,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::ImageRef;
    use crate::scenes::testutil::assets;

    /// Com os dados reais: as figuras da bolsa existem no `WRITER.PIC` com
    /// os tamanhos e as máscaras dadas, e os balões comuns (menu do
    /// personagem e chegada às salas) estão no `WHAT 0bb8`.
    #[test]
    fn writer_kit_matches_real_data() {
        let Some(a) = assets() else { return };
        let p = WRITER_BAG_PICS;
        for (img, mask, w, h) in [p.closed, p.open, p.suitcase, p.book, p.wand] {
            for id in [img, mask] {
                let i = a.image(&ImageRef::Pic(Program::Writer, id));
                assert_eq!((i.width as i32, i.height as i32), (w, h), "{id:#x}");
            }
            assert_eq!(a.pic_next_id(Program::Writer, img, 2), Some(mask));
        }
        let recs = a.dialogs(Program::Writer, WRITER_KIT.what);
        assert_eq!(recs[WRITER_KIT.help_off].options().len(), 3);
        assert_eq!(recs[WRITER_KIT.help_on].options().len(), 2);
        // chegada: saguão, Biblioteca, Escritório, Projetos, Ideias, elevador
        for (k, n) in [(1, 4), (6, 5), (26, 4), (31, 4), (42, 3)] {
            assert_eq!(recs[k].options().len(), n, "0bb8[{k}]");
        }
    }
}
