//! Saguão do Creative Writer (tarefa T5; `cw/report/building-salas.md` §1
//! e a tabela de pontos quentes do `WRITER.EXE`,
//! `cw/report/building-hotspots.txt`): fundo `6805`, quadro de andares
//! `68b7` com cinco zonas (Oficina de Idéias, Oficina de Projetos,
//! Escritório, setinha da Biblioteca e a faixa de baixo, que leva ao
//! sótão), mostrador do elevador que só toca `233` (`Action::Sound`), seta
//! "Biblioteca" (`2fb`), skate "LÁ FORA", portas e botão do elevador (`2d4`
//! + `311`), luminária "Tchau" com a fumaça, balão de chegada `0bb8[1]`
//! com as dicas `[2]`–`[5]`.
//!
//! Posições das figuras conferidas com `scratch/locate.py` em
//! `building_18_closed`; corpos dos balões medidos por diferença de
//! capturas (`building_16_after_door`, `17_lobby_opt1`, `21`–`23`).

use super::room::Floor;
use super::room_objects::BagLayout;
use super::room_spec::{Action, Arrival, CharacterSpec, Fallback, RoomSpec, Tip, hot};
use super::room_talk::Helper;
use super::rooms_data::Element;
use super::writer_gags as g;
use super::writer_rooms::{MCZEE_MENU, QUIT_POS, WRITER_KIT, rec, snd};

/// Fundo (parede amarela com faixas, chão cinza).
pub const BACKGROUND: u16 = 0x6805;

/// Figura colorida com a máscara (id do 1 bit + 1), na posição da tabela.
const fn el(id: u16, mask: u16, x: i32, y: i32) -> Element {
    Element { id, mask, x, y }
}

/// Figuras fixas, na ordem da tabela de pontos quentes.
pub const ELEMENTS: &[Element] = &[
    // portas do elevador, botão de chamar, mostrador com a seta
    el(0x69af, 0x69b1, 216, 93),
    el(0x694b, 0x6951, 480, 202),
    el(0x6953, 0x6955, 288, 21),
    // quadro de andares (mapa do prédio)
    el(0x68b7, 0x68b9, 4, 9),
    // skate "LÁ FORA" e seta "Biblioteca"
    el(0x68b3, 0x68b5, 409, 391),
    el(0x6815, 0x68b1, 94, 306),
    // mastro e a abertura dele no chão
    el(0x68df, 0x68e5, 596, 0),
    el(0x68fb, 0x68fd, 552, 393),
    // buraco do rato
    el(0x691f, 0x6921, 497, 347),
];

/// Bolsa Mágica e o que sai dela (`building.md` §8.3).
pub const BAG: BagLayout = BagLayout {
    closed: (97, 388),
    open: (97, 410),
    suitcase: (239, 359),
    book: (297, 402),
    wand: (222, 421),
};

const TIPS: &[Tip] = &[
    Tip {
        record: rec::LOBBY_OFFICE,
        pos: (34, 110),
        w: 165,
        text: "Clique no andar do Escritório para começar a escrever por conta própria.\nVocê pode escrever qualquer coisa aqui, desde uma carta de amor até um best-seller!",
        links: &[],
    },
    Tip {
        record: rec::LOBBY_PROJECTS,
        pos: (30, 89),
        w: 165,
        text: "Clique na Oficina de Projetos para trabalhar com o Max.\nEle está esperando para ajudar você a fazer cartões, faixas e jornais!",
        links: &[],
    },
    Tip {
        record: rec::LOBBY_IDEAS,
        pos: (25, 69),
        w: 162,
        text: "Encalacrado? Não esquenta! Clique aqui para ir até a Oficina de Idéias!\nO Max inventou algumas máquinas maneiras para ajudar você a desbloquear.",
        links: &[],
    },
    Tip {
        record: rec::LOBBY_LIBRARY,
        pos: (94, 147),
        w: 162,
        text: "Clique na seta da Biblioteca para ver todas as coisas que você escreveu.\nAli você também pode copiar, jogar fora ou mover documentos para outros lugares.",
        links: &[],
    },
];

const LOBBY_ROOM: RoomSpec = RoomSpec {
    kit: &WRITER_KIT,
    background: BACKGROUND,
    elements: ELEMENTS,
    hotspots: &[
        // quadro de andares: as faixas da tabela (o andar 4 é a Oficina
        // de Idéias; a faixa de baixo do mapa leva ao sótão)
        hot((25, 24, 159, 33), Action::Floor(Floor::Tricks)),
        hot((30, 56, 153, 21), Action::Floor(Floor::Projects)),
        hot((34, 76, 149, 22), Action::Floor(Floor::Studio)),
        hot((10, 99, 36, 20), Action::Gallery(snd::BOARD)),
        hot((41, 128, 53, 12), Action::Floor(Floor::Attic)),
        // mostrador do elevador: só o som
        hot((288, 21, 104, 71), Action::Sound(snd::DIAL)),
        hot((497, 347, 54, 47), Action::MouseHole),
        hot((409, 391, 81, 66), Action::Outside),
        hot((552, 393, 105, 43), Action::PoleDown),
        hot((596, 0, 22, 393), Action::PoleUp),
        hot((480, 202, 36, 71), Action::Elevator),
        hot((216, 93, 248, 301), Action::Elevator),
        hot((94, 306, 94, 52), Action::Gallery(snd::LIBRARY_ARROW)),
        // luminária com a placa "Tchau"
        hot((147, 162, 56, 132), Action::Quit),
    ],
    arrival: Some(Arrival {
        record: rec::LOBBY,
        fallback: Fallback {
            x: 129,
            y: 233,
            w: 276,
            text: "Oi, @! O que você quer fazer primeiro?",
            options: &[
                "Começar a trabalhar por conta própria",
                "Fazer um cartão, uma faixa ou um jornal",
                "Ter algumas idéias para escrever",
                "Copiar, apagar ou limpar documentos",
            ],
        },
        actions: &[
            Action::Tip(rec::LOBBY_OFFICE),
            Action::Tip(rec::LOBBY_PROJECTS),
            Action::Tip(rec::LOBBY_IDEAS),
            Action::Tip(rec::LOBBY_LIBRARY),
        ],
        glossary: None,
    }),
    character: Some(CharacterSpec {
        choices: g::MCZEE_COSTUMES,
        pos: g::MCZEE_POS,
        helper: Helper::McZee,
        sound: snd::MCZEE,
        menu: MCZEE_MENU,
        poses: &[],
    }),
    quit_pos: QUIT_POS,
    bag: BAG,
    bag_open: false,
    surprises: g::LOBBY_SURPRISES,
    gags: g::LOBBY_GAGS,
    tips: TIPS,
    painting: None,
    studio: None,
    elevator_sound: snd::ELEVATOR_DOORS,
    arrival_sound: None,
    elevator_arrival_sound: None,
    lamp: Some(g::lamp_at(147, 162)),
    object_balloon: None,
    wand_props: &[],
};

/// O saguão do Writer.
pub const LOBBY: &RoomSpec = &LOBBY_ROOM;
