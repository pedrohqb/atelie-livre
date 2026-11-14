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
use super::room_objects::BagPics;
use super::room_spec::{Persist, RoomKit, RoomSounds, RoomSpec, Tip};
use super::writer_entry::WRITER_QUIT;
use crate::assets::{CursorRef, Program};
use crate::writer::prefs;

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
    /// Clique num objeto com balão (prancheta, quadro, bichos, máquinas).
    pub const OBJECT: u16 = 0x21c;
    /// Abre o balão de escolha dos bichos dos Projetos.
    pub const PICK: u16 = 0x2f4;
    /// Quadro de andares do saguão (fala do McZee) e chegada pelo elevador
    /// ao Escritório.
    pub const BOARD: u16 = 0x304;
    /// Seta "Biblioteca" do saguão (e "Saguão" da Biblioteca).
    pub const LIBRARY_ARROW: u16 = 0x2fb;
    /// Mostrador do elevador no saguão: só o som.
    pub const DIAL: u16 = 0x233;
    /// Portas e botão do elevador no saguão.
    pub const ELEVATOR_DOORS: u16 = 0x2d4;
    /// Skate ("LÁ FORA" no saguão, "ELEVADOR" nos andares).
    pub const SKATE: u16 = 0x2fc;
}

/// Registros do `WHAT 0bb8` usados pelas salas do Writer.
pub mod rec {
    /// "Esse documento é do Max…" (quadro do Escritório com o exemplo).
    pub const MAX_DOC: usize = 0;
    pub const LOBBY: usize = 1;
    /// Dicas do saguão: Escritório, Projetos, Ideias, Biblioteca.
    pub const LOBBY_OFFICE: usize = 2;
    pub const LOBBY_PROJECTS: usize = 3;
    pub const LOBBY_IDEAS: usize = 4;
    pub const LOBBY_LIBRARY: usize = 5;
    /// "A pasta é o lugar onde são guardados…".
    pub const PORTFOLIO: usize = 12;
    /// "Se você quiser, pode digitar outro nome!" (plaqueta) e a dica
    /// "Quer esconder o seu trabalho dos outros?".
    pub const NAME: usize = 18;
    pub const HIDE: usize = 19;
    pub const OFFICE: usize = 26;
    /// Prancheta: "Para criar um novo documento • Dê um clique duplo…".
    pub const OFFICE_NEW: usize = 27;
    /// Quadro: "Esta é a última coisa que você salvou no Escritório…".
    pub const OFFICE_LAST: usize = 28;
    pub const PROJECTS: usize = 31;
    pub const CARD: usize = 35;
    pub const NEWSPAPER: usize = 36;
    pub const BANNER: usize = 37;
    /// "Quer ignorar este balão…? Ctrl" (bichos dos Projetos).
    pub const SKIP_PROJECT: usize = 38;
    pub const IDEAS: usize = 42;
    pub const IDEA_MACHINE: usize = 44;
    pub const SKIP_MACHINE: usize = 45;
    pub const PICTURE_BOARD: usize = 49;
    pub const SKIP_BOARD: usize = 50;
    /// "Varinha Mágica" nos andares (com a "Tecla de atalho" `[57]`).
    pub const WAND_FLOORS: usize = 56;
    pub const HOTKEY_WAND: usize = 57;
    /// "Outra coisa?" do Escritório (mastro, "Descendo?" e "Tecla de atalho").
    pub const OFFICE_POLE: usize = 66;
    pub const HOTKEY_UP: usize = 67;
    pub const POLE_DOWN: usize = 68;
    /// "Outra coisa?" dos Projetos e das Ideias.
    pub const PROJECTS_POLE: usize = 69;
    pub const IDEAS_POLE: usize = 70;
    pub const HOTKEY_DOWN: usize = 71;
    /// "Para ver mais escolhas…" e "Tecla de atalho" Ctrl+O (pasta).
    pub const PORTFOLIO_HELP: usize = 53;
    pub const HOTKEY_PORTFOLIO: usize = 54;
}

/// Balões dos objetos das salas do McZee (prancheta, quadro do
/// Escritório, varinha): saem da boca dele, com o rabicho para cima
/// (corpo medido em `building_44_esc_frame`, `45_esc_clip`,
/// `49_wand_picked`).
pub const MCZEE_MOUTH: (i32, i32) = (129, 233);
/// Menu "Você quer me ver menos?" do McZee e do Max (`19_mczee_click`,
/// `60_p_max`).
pub const MCZEE_MENU: (i32, i32) = (129, 233);
pub const MAX_MENU: (i32, i32) = (107, 132);
/// "EPA! O que você quer fazer?": o mesmo lugar em todas as salas
/// (`building_25_lamp`, `ideas_71_tchau`).
pub const QUIT_POS: (i32, i32) = (208, 179);

/// Dicas comuns às salas do Writer (posições das "Tecla de atalho" a medir:
/// as do Fine Artist).
const KIT_TIPS: &[Tip] = &[
    Tip {
        record: rec::SKIP_PROJECT,
        pos: (169, 203),
        w: 301,
        text: "Quer ignorar este balão e começar um novo projeto imediatamente?\n• Na próxima vez, segure Ctrl quando clicar no projeto que você quer começar.",
        links: &[],
    },
    Tip {
        record: rec::SKIP_MACHINE,
        pos: (169, 203),
        w: 284,
        text: "Você quer ignorar este balão e começar imediatamente do começo?\n• Na próxima vez, segure Ctrl quando clicar na Máquina de Idéias.",
        links: &[],
    },
    Tip {
        record: rec::SKIP_BOARD,
        pos: (169, 203),
        w: 283,
        text: "Você quer ignorar este balão e começar imediatamente do começo?\n• Na próxima vez, segure Ctrl quando clicar na Máquina de Idéias.",
        links: &[],
    },
    Tip {
        record: rec::HOTKEY_UP,
        pos: (195, 204),
        w: 246,
        text: "Quer subir DOIS andares?\nTente segurar a tecla Ctrl enquanto clica no mastro de bombeiro.",
        links: &[],
    },
    Tip {
        record: rec::POLE_DOWN,
        pos: (195, 204),
        w: 237,
        text: "Quer descer pelo mastro de bombeiro?\n• Clique na abertura do mastro de bombeiro do andar em que você está.",
        links: &[],
    },
    Tip {
        record: rec::HOTKEY_DOWN,
        pos: (195, 204),
        w: 272,
        text: "Quer descer DOIS andares?\nSegure a tecla Ctrl enquanto clica na abertura do mastro de bombeiro do andar em que você está.",
        links: &[],
    },
    Tip {
        record: rec::HOTKEY_WAND,
        pos: (195, 204),
        w: 228,
        text: "Quer continuar usando a Varinha Mágica?\nGeralmente a Varinha Mágica desaparece depois que você clica uma vez.\nSe você quiser continuar usando a Varinha Mágica, segure Ctrl quando clicar.",
        links: &[],
    },
    Tip {
        record: rec::NAME,
        pos: (129, 252),
        w: 197,
        text: "Se você quiser, pode digitar outro nome!\nPara apagar alguma coisa\n• Aperte Delete/Backspace.",
        links: &[(rec::HIDE, "Esconder o seu trabalho?")],
    },
    Tip {
        record: rec::HIDE,
        pos: (195, 204),
        w: 264,
        text: "Quer esconder o seu trabalho dos outros?\n• Comece o nome com um ponto de exclamação (!).\nPor exemplo, !Resumo do Livro\nA única pessoa que pode abrir o seu trabalho é você mesmo!",
        links: &[],
    },
];

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
        object: snd::OBJECT,
        pick: snd::PICK,
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
        board: snd::BOARD,
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
        skate: snd::SKATE,
        elevator_arrive: 0x311,
    },
    // as dicas de cada sala ficam nela (`RoomSpec::tips`); aqui, as comuns
    tips: KIT_TIPS,
    glossary_links: &[
        rec::HOTKEY_UP,
        rec::HOTKEY_DOWN,
        rec::HOTKEY_WAND,
        rec::HOTKEY_PORTFOLIO,
    ],
    // a "Tecla de atalho" do `WHAT 0bb8` é a mesma figura `21c` do FA
    hotkey_pic: Some((super::room_talk::PIC_HOTKEY, formats::what::PIC_TIP)),
    help_off: 29,
    help_on: 30,
    wand: Some(Tip {
        record: rec::WAND_FLOORS,
        pos: MCZEE_MOUTH,
        w: 178,
        text: "Varinha Mágica\nQuando você clica no mastro de bombeiro ou no buraco do rato com a Varinha Mágica, acontecem coisas incríveis!",
        links: &[(rec::HOTKEY_WAND, "Tecla de atalho")],
    }),
    quit: QUIT,
    bag: &WRITER_BAG_PICS,
    // varinha do `WRITER.EXE` (`CURSOR` 0x000f, `scenes::writer::cursors`)
    wand_cursor: CursorRef(Program::Writer, super::writer::cursors::WAND),
    // menu do McZee no elevador ≈ (140,100) (`building-salas.md` §2; T8)
    elevator_helper: (140, 100),
    // a bolsa aberta e a ajuda desligada valem entre salas e sessões
    persist: Some(Persist {
        load: prefs::load_state,
        save: prefs::save_state,
    }),
    ctrl_o: true,
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
