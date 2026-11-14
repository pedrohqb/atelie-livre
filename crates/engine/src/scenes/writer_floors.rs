//! Oficinas de Projetos (andar 3) e de Idéias (andar 4) do Creative Writer
//! (tarefas T7 e I5; `cw/report/building-salas.md` §4–5, `ideas.md` §2 e
//! a tabela de pontos quentes do `WRITER.EXE`): o Max (som `283`; poses em
//! `writer_gags::MAX_POSES`), os bichos da parede dos Projetos (alce JORNAL
//! `693f`, burro CARTÃO `6943`, rinoceronte FAIXA `6947`, que a Varinha
//! Mágica enfeita), a Máquina de Idéias `68d3` e o Quadro de Figuras
//! `68db`, balões de chegada `0bb8[31]`/`[42]`, dos objetos `[35]`–`[37]`,
//! `[44]`, `[49]` (com "Ignorar" → `[38]`, `[45]`, `[50]`) e do mastro
//! `[69]`/`[70]`/`[71]`.
//!
//! A primeira bolinha de um objeto (ou Ctrl+clique) começa o trabalho:
//!
//! - **Projetos**: o jornal, o cartão e a faixa ainda não existem; até lá
//!   [`PROJECT_START`] abre a área de escrita com um documento novo
//!   (`CwScene::NewDocument`), o *placeholder* combinado.
//! - **Idéias**: a Máquina de Idéias e o Quadro de Figuras são as telas
//!   `CwScene::IdeaMachine` e `CwScene::PictureBoard` (`writer_ideas`).
//!
//! A segunda bolinha abre a pasta (no original, já filtrada pela área:
//! "Máquina de Idéias", `STR#03ac[9]`; a pasta do port ainda não filtra).

use super::room_objects::BagLayout;
use super::room_spec::{Action, Arrival, CharacterSpec, Fallback, RoomSpec, Tip, WandProp, hot};
use super::room_talk::{Helper, Pick, PickSpec, Start};
use super::rooms_data::Element;
use super::writer_gags as g;
use super::writer_rooms::{MAX_MENU, QUIT_POS, WRITER_KIT, rec, snd};
use crate::scene::{CwScene, SceneId};

const fn el(id: u16, mask: u16, x: i32, y: i32) -> Element {
    Element { id, mask, x, y }
}

/// O Max dos Projetos (de pé) e o das Idéias (de pé ou encostado na
/// Máquina de Idéias).
const MAX: CharacterSpec = CharacterSpec {
    choices: &[g::MAX_STANDING.0],
    pos: g::MAX_STANDING.1,
    helper: Helper::Max,
    sound: snd::MAX,
    menu: MAX_MENU,
    poses: &[],
};
const MAX_IDEAS: CharacterSpec = CharacterSpec {
    poses: g::MAX_POSES,
    ..MAX
};

/// Fundo dos Projetos (parede rosa com cantos, piso de madeira).
pub const PROJECTS_BG: u16 = 0x6811;
/// Bichos da parede: alce, burro e rinoceronte (canto da figura).
pub const MOOSE: (i32, i32) = (78, 31);
pub const DONKEY: (i32, i32) = (240, 1);
pub const RHINO: (i32, i32) = (243, 137);

pub const PROJECTS_ELEMENTS: &[Element] = &[
    // plaqueta "3"
    el(0x69a7, 0x69a9, 0, 0),
    el(0x68df, 0x68e5, 573, -22),
    el(0x68fb, 0x68fd, 529, 371),
    el(0x693f, 0x6941, MOOSE.0, MOOSE.1),
    el(0x6943, 0x6945, DONKEY.0, DONKEY.1),
    el(0x6947, 0x6949, RHINO.0, RHINO.1),
    el(0x691f, 0x6921, 479, 299),
    el(0x68c1, 0x68d1, 466, 407),
];

pub const PROJECTS_BAG: BagLayout = BagLayout {
    closed: (87, 332),
    open: (87, 354),
    suitcase: (234, 285),
    book: (192, 335),
    wand: (308, 357),
};

/// Áreas dos bichos (a placa "FAIXA" do rinoceronte não reage).
const MOOSE_RECT: (i32, i32, i32, i32) = (78, 31, 120, 95);
const DONKEY_RECT: (i32, i32, i32, i32) = (240, 1, 143, 115);
const RHINO_RECT: (i32, i32, i32, i32) = (243, 137, 150, 78);

const PROJECTS_ROOM: RoomSpec = RoomSpec {
    kit: &WRITER_KIT,
    background: PROJECTS_BG,
    elements: PROJECTS_ELEMENTS,
    hotspots: &[
        hot(MOOSE_RECT, Action::Pick(Pick::Newspaper)),
        hot(DONKEY_RECT, Action::Pick(Pick::Card)),
        hot(RHINO_RECT, Action::Pick(Pick::Banner)),
        hot((479, 299, 54, 47), Action::MouseHole),
        hot((466, 407, 81, 66), Action::Elevator),
        hot((529, 371, 105, 43), Action::PoleDown),
        hot((573, 0, 22, 371), Action::PoleUp),
        hot((477, 75, 56, 132), Action::Quit),
    ],
    arrival: Some(Arrival {
        record: rec::PROJECTS,
        fallback: Fallback {
            x: 107,
            y: 145,
            w: 200,
            text: "Aqui é o Max! Estamos no Andar dos Projetos! Vamos fazer alguma coisa juntos?",
            options: &[
                "É, um jornal",
                "OK, vamos fazer um cartão",
                "Quero uma faixa",
                "Outra coisa?",
            ],
        },
        // as três primeiras abrem direto o balão do bicho
        actions: &[
            Action::Pick(Pick::Newspaper),
            Action::Pick(Pick::Card),
            Action::Pick(Pick::Banner),
            Action::Tip(rec::PROJECTS_POLE),
        ],
        glossary: None,
    }),
    character: Some(MAX),
    quit_pos: QUIT_POS,
    bag: PROJECTS_BAG,
    bag_open: false,
    surprises: g::PROJECTS_SURPRISES,
    gags: g::PROJECTS_GAGS,
    tips: &[Tip {
        record: rec::PROJECTS_POLE,
        pos: (339, 121),
        w: 200,
        text: "Quer escrever por conta própria?\n• Clique na abertura do mastro de bombeiro para descer até o Escritório.\nPrecisa de inspiração?\n• Clique no mastro de bombeiro para subir até a Oficina de Idéias.",
        links: &[],
    }],
    painting: None,
    studio: None,
    elevator_sound: snd::SKATE,
    arrival_sound: None,
    elevator_arrival_sound: None,
    lamp: Some(g::lamp_at(477, 75)),
    object_balloon: None,
    // acessórios da varinha (1 célula, 3 s, som `293`), presos ao bicho
    wand_props: &[
        WandProp {
            rect: MOOSE_RECT,
            scripts: g::MOOSE_HATS,
            anchor: MOOSE,
        },
        WandProp {
            rect: DONKEY_RECT,
            scripts: g::DONKEY_HATS,
            anchor: DONKEY,
        },
        WandProp {
            rect: RHINO_RECT,
            scripts: g::RHINO_HATS,
            anchor: RHINO,
        },
    ],
};

/// Fundo das Idéias (parede amarela com peixinhos, piso xadrez).
pub const IDEAS_BG: u16 = 0x6813;

pub const IDEAS_ELEMENTS: &[Element] = &[
    // plaqueta "4"
    el(0x69ab, 0x69ad, 0, 0),
    el(0x68df, 0x68e5, 573, 0),
    el(0x68fb, 0x68fd, 529, 393),
    // Máquina de Idéias pequena (com o peixe) e o Quadro de Figuras
    el(0x68d3, 0x68d9, 135, 239),
    el(0x68db, 0x68dd, 326, 41),
    el(0x691f, 0x6921, 472, 347),
    el(0x68c1, 0x68d1, 495, 420),
];

pub const IDEAS_BAG: BagLayout = BagLayout {
    closed: (266, 388),
    open: (266, 410),
    suitcase: (355, 343),
    book: (414, 396),
    wand: (386, 439),
};

const IDEAS_ROOM: RoomSpec = RoomSpec {
    kit: &WRITER_KIT,
    background: IDEAS_BG,
    elements: IDEAS_ELEMENTS,
    hotspots: &[
        hot((135, 239, 184, 185), Action::Pick(Pick::IdeaMachine)),
        hot((326, 41, 222, 237), Action::Pick(Pick::PictureBoard)),
        hot((472, 347, 54, 47), Action::MouseHole),
        hot((495, 420, 81, 60), Action::Elevator),
        hot((529, 393, 105, 27), Action::PoleDown),
        hot((573, 0, 22, 393), Action::PoleUp),
        hot((15, 7, 56, 132), Action::Quit),
    ],
    arrival: Some(Arrival {
        record: rec::IDEAS,
        fallback: Fallback {
            x: 107,
            y: 146,
            w: 261,
            text: "Seja bem-vindo à Oficina de Idéias! Eu inventei uns Combinadores Mágicos capazes de acabar com qualquer síndrome da página em branco.\nO que você quer ver?",
            options: &[
                "Uma sentença maluca",
                "Uma figura para dar inspiração",
                "Quero fazer outra coisa",
            ],
        },
        actions: &[
            Action::Pick(Pick::IdeaMachine),
            Action::Pick(Pick::PictureBoard),
            Action::Tip(rec::IDEAS_POLE),
        ],
        glossary: None,
    }),
    character: Some(MAX_IDEAS),
    quit_pos: QUIT_POS,
    bag: IDEAS_BAG,
    bag_open: false,
    surprises: g::IDEAS_SURPRISES,
    gags: g::IDEAS_GAGS,
    tips: &[Tip {
        record: rec::IDEAS_POLE,
        pos: (402, 199),
        w: 216,
        text: "Quer fazer um cartão, cartaz ou jornal?\n• Clique na abertura do mastro de bombeiro para descer até a Oficina de Projetos.\nQuer escrever por conta própria?\n• Use a abertura do mastro de bombeiro para descer dois andares até o Escritório",
        links: &[(rec::HOTKEY_DOWN, "Tecla de atalho")],
    }],
    painting: None,
    studio: None,
    elevator_sound: snd::SKATE,
    arrival_sound: None,
    elevator_arrival_sound: None,
    lamp: Some(g::lamp_at(15, 7)),
    object_balloon: None,
    wand_props: &[],
};

/// Oficina de Projetos do Writer.
pub const PROJECTS: &RoomSpec = &PROJECTS_ROOM;
/// Oficina de Idéias do Writer.
pub const IDEAS: &RoomSpec = &IDEAS_ROOM;

/// Enquanto o jornal, o cartão e a faixa do Writer não existem, a primeira
/// opção dos bichos abre a área de escrita com um documento novo.
pub const PROJECT_START: Start = Start::Scene(SceneId::Cw(CwScene::NewDocument));

/// Balão de escolha dos objetos do Writer (`WHAT 0bb8`; posições medidas,
/// `building-salas.md` §4–5).
pub fn pick(p: Pick) -> PickSpec {
    let (record, pos, w, text, options, skip) = match p {
        Pick::Newspaper => (
            rec::NEWSPAPER,
            (28, 121),
            218,
            "Você quer:",
            [
                "Começar um novo jornal?",
                "Abrir um jornal que você já criou?",
            ],
            rec::SKIP_PROJECT,
        ),
        Pick::Card => (
            rec::CARD,
            (205, 93),
            213,
            "Oi! Você quer:",
            [
                "Criar um novo cartão?",
                "Trabalhar com um cartão que você já criou?",
            ],
            rec::SKIP_PROJECT,
        ),
        Pick::Banner => (
            rec::BANNER,
            (225, 229),
            211,
            "Oi! Você quer:",
            [
                "Fazer uma nova faixa?",
                "Trabalhar com uma faixa que você já criou?",
            ],
            rec::SKIP_PROJECT,
        ),
        Pick::IdeaMachine => (
            rec::IDEA_MACHINE,
            (82, 231),
            188,
            "Tudo bem! Você quer:",
            [
                "Começar uma coisa nova?",
                "Trabalhar com uma história que você já começou?",
            ],
            rec::SKIP_MACHINE,
        ),
        _ => (
            rec::PICTURE_BOARD,
            (326, 89),
            189,
            "O Quadro de Figuras! Você quer:",
            [
                "Começar do começo",
                "Trabalhar com uma história que você já começou",
            ],
            rec::SKIP_BOARD,
        ),
    };
    let start = match p {
        Pick::IdeaMachine => Start::Scene(SceneId::Cw(CwScene::IdeaMachine)),
        Pick::PictureBoard => Start::Scene(SceneId::Cw(CwScene::PictureBoard)),
        _ => PROJECT_START,
    };
    PickSpec {
        record,
        pos,
        w,
        text,
        options,
        skip,
        start,
    }
}
