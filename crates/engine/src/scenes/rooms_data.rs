//! Elementos fixos das salas, medidos em capturas do original (tools/wine/compose.py).
//! Gerado por tools/wine/rooms_gen.py e depois acertado à mão: a Bolsa Mágica
//! e o que sai dela (maleta 0x685b, livro 0x6863, varinha 0x686b, bolsa
//! aberta 0x6873 ou fechada 0x6833) saíram daqui e são desenhados por
//! `room_objects`, porque mudam com os cliques; a ponta do balão 0x0407 do
//! saguão também saiu; o mastro dos andares 3 e 4 usa a imagem colorida
//! 0x6823 (a 0x6828 é a versão de 1 bit).
//!
//! No fim, as salas como `RoomSpec` (`room_spec`) e o `ARTIST_KIT`.

use super::attic;
use super::elevator;
use super::entry_spec::ARTIST_QUIT;
use super::gags;
use super::room::Floor;
use super::room_objects::{self as objects, ARTIST_BAG_PICS, Surprises};
use super::room_spec::{
    Action, Arrival, CharacterSpec, Fallback, Glossary, Hotspot, Painting, RoomKit, RoomSounds,
    RoomSpec, Studio, Tip, hot,
};
use super::room_talk::{self as talk, Helper, Pick, rec};
use super::studio_wall;
use crate::assets::Program;

/// Imagem de `ARTIST.PIC`, máscara (ou 0) e posição na tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
    pub id: u16,
    pub mask: u16,
    pub x: i32,
    pub y: i32,
}

pub const LOBBY_BG: u16 = 0x6a6b;
pub const LOBBY: &[Element] = &[
    // portas do elevador e o mostrador "SOBE" (medidos pelo contorno preto numa
    // captura do original; as cores do saguão mudam a cada visita)
    Element {
        id: 0x6c0b,
        mask: 0x6c11,
        x: 208,
        y: 72,
    },
    Element {
        id: 0x6c23,
        mask: 0x6c29,
        x: 288,
        y: 0,
    },
    Element {
        id: 0x6c13,
        mask: 0x6c21,
        x: 257,
        y: 131,
    },
    Element {
        id: 0x6c01,
        mask: 0x6c09,
        x: 4,
        y: 9,
    },
    Element {
        id: 0x6823,
        mask: 0x0000,
        x: 596,
        y: 0,
    },
    Element {
        id: 0x681b,
        mask: 0x6821,
        x: 144,
        y: 182,
    },
    Element {
        id: 0x6c63,
        mask: 0x6c69,
        x: 95,
        y: 316,
    },
    // aro do mastro no chão (o mesmo dos andares; posição pelo mastro,
    // como no Ateliê: 44 px à esquerda dele), por baixo do skate
    Element {
        id: 0x682b,
        mask: 0x6831,
        x: 552,
        y: 389,
    },
    Element {
        id: 0x6c33,
        mask: 0x0000,
        x: 502,
        y: 411,
    },
    Element {
        id: 0x6c2b,
        mask: 0x6c31,
        x: 480,
        y: 202,
    },
    Element {
        id: 0x683b,
        mask: 0x6841,
        x: 513,
        y: 346,
    },
];

pub const STUDIO_BG: u16 = 0x6c6b;
pub const STUDIO: &[Element] = &[
    Element {
        id: 0x6823,
        mask: 0x0000,
        x: 575,
        y: 1,
    },
    Element {
        id: 0x6c6d,
        mask: 0x6c71,
        x: 264,
        y: 63,
    },
    Element {
        id: 0x681b,
        mask: 0x6821,
        x: 473,
        y: 146,
    },
    Element {
        id: 0x6843,
        mask: 0x6849,
        x: 466,
        y: 407,
    },
    Element {
        id: 0x682b,
        mask: 0x6831,
        x: 531,
        y: 390,
    },
    Element {
        id: 0x683b,
        mask: 0x6841,
        x: 466,
        y: 328,
    },
    Element {
        id: 0x6e7b,
        mask: 0x6e8d,
        x: 606,
        y: 155,
    },
    Element {
        id: 0x6c73,
        mask: 0x6c75,
        x: 79,
        y: 135,
    },
];

pub const PROJECTS_BG: u16 = 0x67fb;
pub const PROJECTS: &[Element] = &[
    Element {
        id: 0x6801,
        mask: 0x6809,
        x: 39,
        y: 13,
    },
    Element {
        id: 0x680b,
        mask: 0x0000,
        x: 254,
        y: 3,
    },
    Element {
        id: 0x6813,
        mask: 0x6819,
        x: 153,
        y: 168,
    },
    Element {
        id: 0x6823,
        mask: 0x6829,
        x: 573,
        y: 2,
    },
    Element {
        id: 0x681b,
        mask: 0x6821,
        x: 477,
        y: 75,
    },
    Element {
        id: 0x682b,
        mask: 0x6831,
        x: 529,
        y: 371,
    },
    Element {
        id: 0x6843,
        mask: 0x6849,
        x: 466,
        y: 407,
    },
    Element {
        id: 0x683b,
        mask: 0x6841,
        x: 479,
        y: 289,
    },
    Element {
        id: 0x6e8f,
        mask: 0x6e91,
        x: 606,
        y: 155,
    },
];

pub const TRICKS_BG: u16 = 0x687b;
pub const TRICKS: &[Element] = &[
    // estátua do porco no pedestal "Básico" (medida na captura do original)
    Element {
        id: 0x6a01,
        mask: 0x6a11,
        x: 151,
        y: 44,
    },
    Element {
        id: 0x6a13,
        mask: 0x0000,
        x: 325,
        y: 89,
    },
    Element {
        id: 0x6823,
        mask: 0x6829,
        x: 539,
        y: 0,
    },
    Element {
        id: 0x681b,
        mask: 0x6821,
        x: 564,
        y: 3,
    },
    Element {
        id: 0x6843,
        mask: 0x6849,
        x: 457,
        y: 409,
    },
    Element {
        id: 0x682b,
        mask: 0x6831,
        x: 495,
        y: 355,
    },
    Element {
        id: 0x683b,
        mask: 0x6841,
        x: 576,
        y: 273,
    },
    Element {
        id: 0x6e93,
        mask: 0x6e95,
        x: 606,
        y: 155,
    },
];

// ---------------------------------------------------------------------------
// Salas do Fine Artist como `RoomSpec` (a mecânica está em `room.rs`; o
// contrato, em `room_spec.rs`).

/// O que é comum às salas do Fine Artist: `WHAT 100d`, sons do
/// `ARTIST.SND`, figuras da bolsa, cursor da varinha, "EPA!" do `1001[2]`.
pub const ARTIST_KIT: RoomKit = RoomKit {
    program: Program::Artist,
    what: talk::ROOMS_WHAT,
    snd: RoomSounds {
        object: talk::snd::OBJECT,
        pick: talk::snd::PICK,
        choice: talk::snd::CHOICE,
        tip: talk::snd::TIP,
        glossary: talk::snd::GLOSSARY,
        quit: talk::snd::QUIT,
        refuse: talk::snd::REFUSE,
        painting: talk::snd::PAINTING,
        mouse_hole: talk::snd::MOUSE_HOLE,
        portfolio: talk::snd::PORTFOLIO,
        board: talk::snd::STUDIO_ARRIVAL,
        mczee: talk::snd::MCZEE,
        bag_open: objects::SND_BAG_OPEN,
        bag_close: objects::SND_BAG_CLOSE,
        suitcase: objects::SND_SUITCASE,
        book: objects::SND_BOOK,
        wand_take: objects::SND_WAND_TAKE,
        wand_hole: objects::SND_WAND_HOLE,
        pole_up: objects::SND_POLE_UP,
        pole_down: objects::SND_POLE_DOWN,
        skate: objects::SND_SKATE,
        elevator_arrive: elevator::SND_ARRIVE,
    },
    tips: talk::ARTIST_TIPS,
    glossary_links: &[rec::GLOSS_3D, rec::HOTKEY_UP, rec::HOTKEY_DOWN],
    hotkey_pic: Some((talk::PIC_HOTKEY, formats::what::PIC_TIP)),
    help_off: rec::HELP_OFF,
    help_on: rec::HELP_ON,
    wand: Some(Tip {
        record: WAND_FLOORS,
        pos: (120, 235),
        w: 168,
        text: "Varinha Mágica\nQuando você clica no mastro de bombeiro ou no buraco do rato com a Varinha Mágica, coisas incríveis acontecem!",
        links: &[],
    }),
    quit: &ARTIST_QUIT,
    bag: &ARTIST_BAG_PICS,
    wand_cursor: objects::WAND_CURSOR,
    elevator_helper: (117, 102),
};

/// Balão "Wand - floors" do `WHAT 100d`.
const WAND_FLOORS: usize = 59;

const MCZEE_SOUND: u16 = talk::snd::MCZEE;
const MAGGIE_SOUND: u16 = talk::snd::MAGGIE;

const NO_SURPRISES: Surprises = Surprises {
    pole: &[],
    pole_anchor: (0, 0),
    hole: &[],
    hole_anchor: (0, 0),
};

/// Saguão: quadro de andares, elevador, seta da Galeria, skate "LÁ FORA".
pub const LOBBY_ROOM: RoomSpec = RoomSpec {
    kit: &ARTIST_KIT,
    background: LOBBY_BG,
    elements: LOBBY,
    hotspots: &[
        // quadro de andares: os nomes e os andares da figura do
        // prédio (a setinha "Galeria" fica no térreo dela)
        hot((98, 38, 90, 18), Action::Floor(Floor::Tricks)),
        hot((98, 58, 90, 18), Action::Floor(Floor::Projects)),
        hot((98, 78, 90, 18), Action::Floor(Floor::Studio)),
        hot((98, 98, 90, 18), Action::Gallery(talk::snd::STUDIO_ARRIVAL)),
        hot((12, 18, 86, 35), Action::Floor(Floor::Tricks)),
        hot((12, 53, 86, 22), Action::Floor(Floor::Projects)),
        hot((12, 75, 86, 21), Action::Floor(Floor::Studio)),
        hot((10, 96, 88, 22), Action::Gallery(talk::snd::STUDIO_ARRIVAL)),
        hot((513, 346, 54, 47), Action::MouseHole),
        // abertura do mastro no chão, à direita do skate
        hot((560, 390, 80, 20), Action::PoleDown),
        hot((584, 410, 56, 16), Action::PoleDown),
        hot((502, 411, 81, 66), Action::Outside),
        hot((480, 202, 36, 71), Action::Elevator),
        hot((210, 130, 260, 270), Action::Elevator),
        // topo do arco da porta
        hot((236, 64, 210, 66), Action::Elevator),
        hot((146, 290, 48, 20), Action::Quit),
        hot((148, 183, 52, 105), Action::Quit),
        hot((95, 316, 95, 52), Action::Gallery(talk::snd::GALLERY_ARROW)),
        hot((594, 0, 26, 390), Action::PoleUp),
    ],
    arrival: Some(Arrival {
        record: 1,
        fallback: Fallback {
            x: 300,
            y: 20,
            w: 300,
            text: "Oi! Eu sou o McZee. O que você quer fazer?",
            options: &[
                "Começar a desenhar por conta própria",
                "Fazer uma história em quadrinhos, um botão ou adesivo, ou montar um Desenho Animado",
                "Aprender alguns truques de desenho",
                "Copiar, apagar ou limpar desenhos",
            ],
        },
        actions: &[
            Action::Tip(rec::LOBBY_STUDIO),
            Action::Tip(rec::LOBBY_PROJECTS),
            Action::Tip(rec::LOBBY_TRICKS),
            Action::Tip(rec::LOBBY_GALLERY),
        ],
        glossary: None,
    }),
    character: Some(CharacterSpec {
        choices: gags::MCZEE_COSTUMES,
        pos: gags::MCZEE_LOBBY,
        helper: Helper::McZee,
        sound: MCZEE_SOUND,
        menu: (119, 221),
    }),
    quit_pos: (214, 179),
    bag: objects::LOBBY_BAG,
    bag_open: false,
    surprises: objects::LOBBY_SURPRISES,
    gags: gags::LOBBY_SCRIPTS,
    tips: &[],
    painting: None,
    studio: None,
    elevator_sound: elevator::SND_DOORS,
    arrival_sound: None,
    elevator_arrival_sound: None,
};

/// Ateliê de Pintura (andar 2): cavalete e quadro da parede com o último
/// desenho; a bolsa já está aberta ao chegar.
pub const STUDIO_ROOM: RoomSpec = RoomSpec {
    kit: &ARTIST_KIT,
    background: STUDIO_BG,
    elements: STUDIO,
    hotspots: &[
        hot((283, 63, 158, 120), Action::Wall),
        hot(studio_wall::LABEL, Action::WallLabel),
        // tela e plaqueta do cavalete, poste de cima e pés
        hot((105, 135, 186, 165), Action::Easel),
        hot((150, 300, 100, 92), Action::Easel),
        hot((466, 328, 54, 47), Action::MouseHole),
        hot((466, 407, 81, 66), Action::Elevator),
        hot((531, 390, 105, 43), Action::PoleDown),
        hot((575, 0, 24, 395), Action::PoleUp),
        hot((474, 250, 46, 20), Action::Quit),
        hot((473, 146, 52, 104), Action::Quit),
    ],
    arrival: Some(Arrival {
        record: 26,
        fallback: Fallback {
            x: 130,
            y: 230,
            w: 200,
            text: "Oba! Este é o Ateliê de Pintura! O que você quer fazer?",
            options: &[
                "Pintar alguma coisa nova",
                "Trabalhar na minha última obra",
                "Trabalhar em um quadro mais antigo",
                "Outra coisa?",
            ],
        },
        actions: &[
            Action::Tip(rec::STUDIO_NEW),
            Action::Tip(rec::STUDIO_LAST),
            Action::Tip(rec::STUDIO_PORTFOLIO),
            Action::Tip(rec::STUDIO_POLE),
        ],
        glossary: None,
    }),
    character: Some(CharacterSpec {
        choices: gags::MCZEE_COSTUMES,
        pos: gags::MCZEE_STUDIO,
        helper: Helper::McZee,
        sound: MCZEE_SOUND,
        menu: (130, 232),
    }),
    quit_pos: (215, 180),
    bag: objects::STUDIO_BAG,
    bag_open: true,
    surprises: objects::STUDIO_SURPRISES,
    gags: gags::STUDIO_SCRIPTS,
    tips: &[],
    painting: None,
    // miolo branco da parede (287,65)-(434,151); texto das plaquetas
    // centrado na parte branca (medido no original)
    studio: Some(Studio {
        easel_tip: rec::STUDIO_NEW,
        wall_tip: rec::STUDIO_LAST,
        wall_pic: (287, 65),
        wall_scale: 4,
        wall_label: (363, 190),
        easel_label: (197, 277),
        label_rect: studio_wall::LABEL,
    }),
    elevator_sound: objects::SND_SKATE,
    arrival_sound: None,
    elevator_arrival_sound: Some(talk::snd::STUDIO_ARRIVAL),
};

/// Oficina de Projetos (andar 3): vaca, cobra e porco; a Maggie.
pub const PROJECTS_ROOM: RoomSpec = RoomSpec {
    kit: &ARTIST_KIT,
    background: PROJECTS_BG,
    elements: PROJECTS,
    hotspots: &[
        hot((479, 289, 54, 47), Action::MouseHole),
        hot((466, 407, 81, 66), Action::Elevator),
        hot((529, 371, 105, 43), Action::PoleDown),
        hot((573, 0, 24, 375), Action::PoleUp),
        hot((478, 185, 50, 20), Action::Quit),
        hot((478, 72, 54, 112), Action::Quit),
        hot((40, 20, 145, 165), Action::Pick(Pick::Comics)),
        hot((255, 0, 135, 120), Action::Pick(Pick::Flipbook)),
        hot((150, 165, 240, 120), Action::Pick(Pick::Sticker)),
    ],
    arrival: Some(Arrival {
        record: 31,
        fallback: Fallback {
            x: 70,
            y: 150,
            w: 300,
            text: "Aqui é a Maggie! Estamos no Andar dos Projetos! Vamos fazer alguma coisa juntos?",
            options: &[
                "É, uma história em quadrinhos",
                "OK, vamos fazer um botão ou um adesivo",
                "Quero fazer um desenho animado",
                "Outra coisa?",
            ],
        },
        actions: &[
            Action::Pick(Pick::Comics),
            Action::Pick(Pick::Sticker),
            Action::Pick(Pick::Flipbook),
            Action::Tip(rec::PROJECTS_POLE),
        ],
        glossary: None,
    }),
    character: Some(CharacterSpec {
        choices: gags::MAGGIE_POSES,
        pos: gags::MAGGIE_POS,
        helper: Helper::Maggie,
        sound: MAGGIE_SOUND,
        menu: (109, 132),
    }),
    quit_pos: (214, 180),
    bag: objects::PROJECTS_BAG,
    bag_open: false,
    surprises: objects::PROJECTS_SURPRISES,
    gags: gags::PROJECTS_SCRIPTS,
    tips: &[],
    painting: None,
    studio: None,
    elevator_sound: objects::SND_SKATE,
    arrival_sound: None,
    elevator_arrival_sound: None,
};

/// Obras-primas do McZee (cor, máscara) que aparecem na moldura do andar 4.
pub const PAINTINGS: &[(u16, u16)] = &[
    (0x6d37, 0x6d39),
    (0x6d3b, 0x6d3d),
    (0x6d3f, 0x6d45),
    (0x6d47, 0x6d4d),
    (0x6d4f, 0x6d51),
    (0x6d53, 0x6d55),
    (0x6d57, 0x6d59),
    (0x6d5b, 0x6d5d),
];

const TRICKS_HOTSPOTS: &[Hotspot] = &[
    hot((8, 8, 110, 134), Action::Painting),
    hot((576, 273, 54, 47), Action::MouseHole),
    hot((457, 409, 81, 66), Action::Elevator),
    hot((495, 355, 105, 43), Action::PoleDown),
    hot((539, 0, 24, 360), Action::PoleUp),
    hot((564, 114, 50, 20), Action::Quit),
    hot((565, 5, 50, 108), Action::Quit),
    hot((325, 100, 200, 245), Action::Pick(Pick::ThreeD)),
    hot((150, 40, 150, 290), Action::Pick(Pick::Basic)),
];

/// Truques de Desenho (andar 4): estátuas, moldura das obras-primas.
pub const TRICKS_ROOM: RoomSpec = RoomSpec {
    kit: &ARTIST_KIT,
    background: TRICKS_BG,
    elements: TRICKS,
    hotspots: TRICKS_HOTSPOTS,
    arrival: Some(Arrival {
        record: 42,
        fallback: Fallback {
            x: 70,
            y: 120,
            w: 260,
            text: "Seja bem-vindo ao andar dos Truques de Desenho! Eu vou te mostrar os meus truques secretos para desenhar. Com que truque você quer que eu comece?",
            options: &[
                "Um truque básico",
                "Fazendo uma coisa parecer tridimensional",
                "Quero fazer outra coisa",
            ],
        },
        actions: &[
            Action::Pick(Pick::Basic),
            Action::Pick(Pick::ThreeD),
            Action::Tip(rec::TRICKS_POLE),
        ],
        glossary: Some(Glossary {
            option: 1,
            word: "tridimensional",
            record: rec::GLOSS_3D,
        }),
    }),
    character: Some(CharacterSpec {
        choices: gags::MAGGIE_POSES,
        pos: gags::MAGGIE_POS,
        helper: Helper::Maggie,
        sound: MAGGIE_SOUND,
        menu: (109, 132),
    }),
    quit_pos: (214, 200),
    bag: objects::TRICKS_BAG,
    bag_open: false,
    surprises: objects::TRICKS_SURPRISES,
    gags: gags::TRICKS_SCRIPTS,
    tips: &[],
    painting: Some(Painting {
        choices: PAINTINGS,
        pos: (1, 4),
    }),
    studio: None,
    elevator_sound: objects::SND_SKATE,
    arrival_sound: None,
    elevator_arrival_sound: None,
};

/// Sótão escuro (só pelo mastro): o aro leva ao saguão; o resto do escuro
/// só toca um som (`attic`).
pub const ATTIC_ROOM: RoomSpec = RoomSpec {
    kit: &ARTIST_KIT,
    background: 0,
    elements: &[],
    hotspots: &[hot(attic::RING, Action::PoleUp)],
    arrival: None,
    character: None,
    quit_pos: (214, 179),
    bag: objects::TRICKS_BAG,
    bag_open: false,
    surprises: NO_SURPRISES,
    gags: &[],
    tips: &[],
    painting: None,
    studio: None,
    elevator_sound: objects::SND_SKATE,
    arrival_sound: Some(attic::SND_ARRIVE),
    elevator_arrival_sound: None,
};

/// Sala do Fine Artist em cada andar.
pub fn room(floor: Floor) -> &'static RoomSpec {
    match floor {
        Floor::Lobby => &LOBBY_ROOM,
        Floor::Studio => &STUDIO_ROOM,
        Floor::Projects => &PROJECTS_ROOM,
        Floor::Tricks => &TRICKS_ROOM,
        Floor::Attic => &ATTIC_ROOM,
    }
}
