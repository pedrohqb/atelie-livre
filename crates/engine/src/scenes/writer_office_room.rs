//! Escritório do Creative Writer — a sala do andar 2, não a área de escrita
//! (tarefa T6; `cw/report/building-salas.md` §3 e a tabela de pontos
//! quentes do `WRITER.EXE`): fundo `680d`, plaqueta "2", quadro `68bb` com
//! a miniatura da primeira página do último documento, prancheta
//! "Documento Novo" `68f7` (clique duplo → `CwScene::NewDocument`),
//! plaquetas (a do quadro troca o título de um documento do usuário; num
//! documento de outro abre o "EPA!" `0805[30]`), balão de chegada
//! `0bb8[26]` e dicas `[27]`/`[28]`/`[0]`/`[12]`/`[66]`.
//!
//! O último documento é o `FILR 0` do `~MAXUSER.INF` do usuário
//! (`writer::prefs`); para um usuário novo, "O Ponto de Fuga" do Max
//! (`SHARED/USERS/MAX/SAMP18.MAX`). A miniatura é o `THMB` do `.MAX`
//! (`formats::max::Document::thumbnail_bitmap`, 108×144), reduzida como no
//! original a 73×98 e encostada no topo da área da página (medido em
//! `building_38_esc_base`: página em (162,107)–(234,204)).

use super::room_objects::BagLayout;
use super::room_spec::{
    Action, Arrival, CharacterSpec, DocWall, Fallback, LastWork, RoomSpec, Studio, Tip, WorkPlace,
    hot,
};
use super::room_talk::Helper;
use super::rooms_data::Element;
use super::writer_gags as g;
use super::writer_rooms::{MCZEE_MENU, MCZEE_MOUTH, QUIT_POS, WRITER_KIT, rec, snd};
use crate::assets::{Assets, Program};
use crate::writer::prefs;
use formats::pkbm;

/// Fundo (parede laranja com losangos, piso de lajotas verdes).
pub const BACKGROUND: u16 = 0x680d;

const fn el(id: u16, mask: u16, x: i32, y: i32) -> Element {
    Element { id, mask, x, y }
}

/// Figuras fixas, na ordem da tabela de pontos quentes.
pub const ELEMENTS: &[Element] = &[
    // plaqueta "2"
    el(0x69a3, 0x69a5, 0, 0),
    el(0x68df, 0x68e5, 591, -9),
    el(0x68fb, 0x68fd, 547, 384),
    // quadro do último documento e a prancheta
    el(0x68bb, 0x68bd, 127, 88),
    el(0x68f7, 0x68f9, 297, 41),
    el(0x691f, 0x6921, 490, 313),
    // skate "ELEVADOR"
    el(0x68c1, 0x68d1, 466, 407),
];

/// Bolsa Mágica e o que sai dela.
pub const BAG: BagLayout = BagLayout {
    closed: (129, 392),
    open: (129, 414),
    suitcase: (293, 360),
    book: (244, 393),
    wand: (353, 417),
};

/// Plaqueta do quadro (com o título) e a da prancheta.
pub const WALL_LABEL: (i32, i32, i32, i32) = (131, 232, 136, 17);
pub const CLIP_LABEL: (i32, i32, i32, i32) = (319, 232, 117, 16);
/// Área da página no quadro (clique; tabela) e a caixa da miniatura.
pub const WALL_PAGE: (i32, i32, i32, i32) = (149, 106, 100, 144);
pub const THUMB_BOX: (i32, i32, i32, i32) = (149, 107, 100, 98);
/// Folha da prancheta.
pub const CLIP_SHEET: (i32, i32, i32, i32) = (325, 86, 104, 138);

/// Texto da plaqueta da prancheta.
pub const NEW_DOCUMENT: &str = "Documento Novo";

/// Último documento do usuário (`FILR 0`): miniatura, título e dono.
pub fn load_last(place: &WorkPlace<'_>, assets: &Assets) -> Option<LastWork> {
    let (doc, file) = prefs::last_document(place.users, place.user, place.data)?;
    let max = std::fs::read(&file)
        .ok()
        .and_then(|b| formats::max::read(&b).ok());
    let pal = assets.palettes(Program::Writer);
    let image = max
        .as_ref()
        .and_then(|m| m.thumbnail_bitmap())
        .map(|b| bitmap_image(&b, &pal.pal16, &pal.pal256));
    let title = match max {
        Some(m) if doc.title.is_empty() => m.add.title,
        _ => doc.title.clone(),
    };
    Some(LastWork {
        image,
        title,
        own: prefs::is_own(&doc, place.user),
        path: file,
    })
}

/// Troca o título do último documento (só os do usuário).
pub fn rename_last(place: &WorkPlace<'_>, w: &LastWork, name: &str) -> Result<(), String> {
    let users = place
        .users
        .ok_or_else(|| "sem pasta de usuários para gravar o título".to_string())?;
    prefs::retitle(users, place.user, &w.path, name)
}

/// Miniatura PackBits do `.MAX` em RGBA (com a paleta dela, ou a padrão do
/// Writer para a profundidade).
pub fn bitmap_image(
    b: &pkbm::Bitmap,
    pal16: &[formats::palette::Rgb],
    pal256: &[formats::palette::Rgb],
) -> crate::Image {
    let pal = b
        .palette
        .clone()
        .unwrap_or_else(|| pkbm::default_palette_for(b.depth, pal16, pal256));
    let mut rgba = Vec::with_capacity(b.pixels.len() * 4);
    for &i in &b.pixels {
        let c = pal
            .get(i as usize)
            .copied()
            .unwrap_or(formats::palette::Rgb {
                r: 255,
                g: 255,
                b: 255,
            });
        rgba.extend_from_slice(&[c.r, c.g, c.b, 255]);
    }
    crate::Image {
        width: b.width as u32,
        height: b.height as u32,
        rgba,
    }
}

const DOC_WALL: DocWall = DocWall {
    load: load_last,
    rename: rename_last,
    thumb_box: THUMB_BOX,
    other_tip: rec::MAX_DOC,
    not_yours: (
        0x0805,
        30,
        (203, 126),
        "Você não pode mudar esse título.\nVocê tentou mudar o título de um documento que não é seu.\nSe você quiser usar esse documento para começar seu próprio documento:\n1 Dê um clique duplo na tela para abri-lo.\n2 Faça suas mudanças.\n3 Salve o documento com outro título.",
    ),
};

/// Dicas das opções do balão de chegada, perto do objeto (corpos medidos em
/// `building_40_esc_o1`…`43_esc_o4`); os mesmos registros, clicando no
/// objeto, saem da boca do McZee (`object_balloon`).
const TIPS: &[Tip] = &[
    Tip {
        record: rec::OFFICE_NEW,
        pos: (300, 248),
        w: 153,
        text: "Para criar um novo documento\n• Dê um clique duplo na prancheta em branco.",
        links: &[],
    },
    Tip {
        record: rec::OFFICE_LAST,
        pos: (253, 138),
        w: 166,
        text: "Esta é a última coisa que você salvou no Escritório.\nPara trabalhar neste documento de novo\n• Dê um clique duplo nele.",
        links: &[],
    },
    Tip {
        record: rec::MAX_DOC,
        pos: (253, 138),
        w: 188,
        text: "Esse documento é do Max. Como ele é exibido!\nPara ver o que o Max fez\n• Dê um clique duplo nele.",
        links: &[],
    },
    Tip {
        record: rec::PORTFOLIO,
        pos: (260, 206),
        w: 162,
        text: "A pasta é o lugar onde são guardados todos os seus documentos (e os das outras pessoas também).\nPara abrir a pasta\n• Clique nela.",
        links: &[],
    },
    Tip {
        record: rec::OFFICE_POLE,
        pos: (347, 94),
        w: 210,
        text: "Você quer fazer um cartão, faixa ou jornal?\n• Clique no mastro de bombeiro para subir até a Oficina de Projetos!\nQuer usar as Máquinas Mágicas do Max?\n• Use o mastro de bombeiro para subir DOIS andares até a Oficina de Idéias!",
        links: &[
            (rec::POLE_DOWN, "Descendo?"),
            (rec::HOTKEY_UP, "Tecla de atalho"),
        ],
    },
];

const OFFICE_ROOM: RoomSpec = RoomSpec {
    kit: &WRITER_KIT,
    background: BACKGROUND,
    elements: ELEMENTS,
    hotspots: &[
        hot(WALL_LABEL, Action::WallLabel),
        hot(WALL_PAGE, Action::Wall),
        hot(CLIP_SHEET, Action::Easel),
        hot(CLIP_LABEL, Action::Easel),
        hot((490, 313, 54, 47), Action::MouseHole),
        hot((466, 407, 81, 66), Action::Elevator),
        hot((547, 384, 93, 43), Action::PoleDown),
        hot((591, 0, 22, 384), Action::PoleUp),
        hot((468, 85, 56, 132), Action::Quit),
    ],
    arrival: Some(Arrival {
        record: rec::OFFICE,
        fallback: Fallback {
            x: MCZEE_MOUTH.0,
            y: MCZEE_MOUTH.1,
            w: 235,
            text: "Oba! Este é o Escritório! O que você quer fazer?",
            options: &[
                "Escrever alguma coisa nova",
                "Trabalhar na minha última obra",
                "Trabalhar em um documento mais antigo",
                "Outra coisa?",
            ],
        },
        // a dica do quadro troca para `[0]` quando o documento é de outro
        actions: &[
            Action::Tip(rec::OFFICE_NEW),
            Action::Tip(rec::OFFICE_LAST),
            Action::Tip(rec::PORTFOLIO),
            Action::Tip(rec::OFFICE_POLE),
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
    surprises: g::OFFICE_SURPRISES,
    gags: g::OFFICE_GAGS,
    tips: TIPS,
    painting: None,
    studio: Some(Studio {
        easel_tip: rec::OFFICE_NEW,
        wall_tip: rec::OFFICE_LAST,
        wall_pic: (THUMB_BOX.0, THUMB_BOX.1),
        wall_scale: 1,
        wall_label: (WALL_LABEL.0 + WALL_LABEL.2 / 2, 234),
        easel_label: (CLIP_LABEL.0 + CLIP_LABEL.2 / 2, 234),
        label_rect: WALL_LABEL,
        name_balloon: (129, 252),
        new_label: NEW_DOCUMENT,
        doc: Some(&DOC_WALL),
    }),
    elevator_sound: snd::SKATE,
    arrival_sound: None,
    elevator_arrival_sound: Some(snd::BOARD),
    lamp: Some(g::lamp_at(468, 85)),
    object_balloon: Some(MCZEE_MOUTH),
    wand_props: &[],
};

/// A sala do andar 2 do Writer.
pub const OFFICE: &RoomSpec = &OFFICE_ROOM;
