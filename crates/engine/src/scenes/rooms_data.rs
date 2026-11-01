//! Elementos fixos das salas, medidos em capturas do original (tools/wine/compose.py).
//! Gerado por tools/wine/rooms_gen.py; não editar à mão.

/// Imagem de `ARTIST.PIC`, máscara (ou 0) e posição na tela.
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
        id: 0x685b,
        mask: 0x6861,
        x: 263,
        y: 394,
    },
    Element {
        id: 0x6873,
        mask: 0x6879,
        x: 97,
        y: 410,
    },
    Element {
        id: 0x681b,
        mask: 0x6821,
        x: 144,
        y: 182,
    },
    Element {
        id: 0x6863,
        mask: 0x6869,
        x: 356,
        y: 367,
    },
    Element {
        id: 0x6c63,
        mask: 0x6c69,
        x: 95,
        y: 316,
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
    Element {
        id: 0x686b,
        mask: 0x6871,
        x: 222,
        y: 421,
    },
    Element {
        id: 0x0407,
        mask: 0x0000,
        x: 54,
        y: 323,
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
        id: 0x6873,
        mask: 0x6879,
        x: 129,
        y: 414,
    },
    Element {
        id: 0x6863,
        mask: 0x6869,
        x: 265,
        y: 383,
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
        id: 0x686b,
        mask: 0x6871,
        x: 352,
        y: 410,
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
        id: 0x6833,
        mask: 0x6839,
        x: 146,
        y: 392,
    },
    Element {
        id: 0x6828,
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
        id: 0x6833,
        mask: 0x6839,
        x: 149,
        y: 357,
    },
    Element {
        id: 0x6828,
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
