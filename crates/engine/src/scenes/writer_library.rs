//! Biblioteca do Creative Writer — a Galeria do Writer (tarefa T9;
//! `cw/report/building-salas.md` §6): fundo `6807`, quatro molduras com os
//! documentos (miniatura `THMB` do `.MAX`, plaqueta com o título), impressora,
//! lixeira, interruptor e gaveta de unidades, seta "Saguão", balões
//! `0bb8[6]`–`[25]`, `[41]` e avisos do Rex `0805`.
//!
//! É a mesma cena da Galeria do Fine Artist (`gallery_room`) com o
//! [`WRITER_LIBRARY`]; o que é próprio dos documentos (listar, miniaturas,
//! abrir no Escritório, imprimir a 1ª página, gravar a parede) é o
//! [`DOCUMENTS`]. A pasta da sala é a do usuário (`<usuários>/<apelido>`,
//! como o `USERS\<APELIDO>` do original).
//!
//! - **Documentos**: os `.MAX` da pasta do usuário (`writer::store::list`,
//!   do mais recente ao mais antigo) e os exemplos do Max
//!   (`SHARED/USERS/MAX/*.MAX`).
//! - **Parede**: `FILR` 1–4 do `~MAXUSER.INF` do usuário, pelas funções de
//!   `writer::prefs` (as da T14: o lugar 0 é o quadro do Escritório e os
//!   outros chunks são mantidos); usuário novo: os exemplos de
//!   `prefs::DEFAULT_WALL`.
//! - **Abrir**: o documento vira o último (`prefs::remember_last`, `FILR 0`)
//!   e `Effect::OpenDocument` leva ao Escritório com ele (`Writer::open`).
//! - **Imprimir**: a prévia da Galeria com o [`WRITER_PREVIEW`] e a 1ª
//!   página desenhada em texto simples ([`render_first_page`]; a
//!   diagramação completa é de outra tarefa).
//! - **Gaveta**: duas unidades, a pasta do usuário e os exemplos; soltar um
//!   documento numa unidade o copia para lá.

use super::gallery_print::{PreviewSpec, PrintPreview, Sheet, Walker};
use super::gallery_room::{
    DrawerSpec, Drive, GalleryAlerts, GalleryPositions, GalleryRecords, GallerySounds, GallerySpec,
    GalleryTexts, Item, Picture, Works, fill_wall, stamped_name,
};
use super::writer_gags as g;
use super::writer_rooms::{WRITER_KIT, snd};
use crate::Frame;
use crate::assets::{Assets, Program};
use crate::paint::Canvas;
use crate::paint::print::Rgb8;
use crate::paths::find_ci;
use crate::scene::Effect;
use crate::ui::Rect;
use crate::writer::{prefs, store};
use formats::max::Align;
use formats::palette::Rgb;
use std::path::{Path, PathBuf};

/// Molduras na parede `692b` (102×124; a página no quadrado de 100×100 e a
/// plaqueta embaixo), na ordem dos `FILR` 1–4: em cima à esquerda, em cima
/// à direita, embaixo à direita, embaixo à esquerda.
pub const FRAMES: [(i32, i32); 4] = [(159, 9), (296, 9), (296, 143), (159, 143)];
/// Área da página dentro da moldura (cinza `8c8c8c` em volta) e plaqueta.
const PAGE_AREA: Rect = (1, 1, 100, 100);
const PAGE_BG: [u8; 3] = [140, 140, 140];
const PLATE: Rect = (3, 108, 96, 13);

/// Elementos fixos (cor, máscara = duas figuras adiante, x, y): a parede
/// das molduras primeiro, depois o armário, a impressora, a lixeira, a
/// seta, a luminária e o buraco do rato.
const ELEMENTS: [(u16, u16, i32, i32); 7] = [
    (0x692b, 0x692d, 142, 0),   // parede com as quatro molduras
    (0x68e7, 0x68e9, 420, 170), // armário (porta)
    (0x6913, 0x6915, 569, 102), // impressora no quadro da parede
    (0x690f, 0x6911, 501, 351), // lixeira de reciclagem
    (0x68eb, 0x68ed, 547, 217), // seta "Saguão"
    (0x6917, 0x6919, 496, 5),   // luminária "Tchau"
    (0x691f, 0x6921, 552, 273), // buraco do rato
];
const LAMP: Rect = (496, 5, 56, 132);
const PRINTER: Rect = (569, 102, 60, 74);
const TRASH: Rect = (501, 351, 92, 108);
const LOBBY_ARROW: Rect = (547, 217, 91, 46);
const MOUSE_HOLE: Rect = (552, 273, 54, 47);

/// Interruptor (desligado / ligado) e a gaveta de unidades (`86_lib_switch`).
pub const DRAWER: DrawerSpec = DrawerSpec {
    switch_off: (0x6923, 0x6925, 496, 151),
    switch_on: (0x6927, 0x6929, 496, 177),
    switch_area: (496, 151, 36, 102),
    drawer: (0x693b, 0x693d, 275, 169),
    // fileiras de 14 px a cada 21 px; pontas vermelhas de 13 e 14 px
    rows: (296, 180, 162, 14, 21, 5),
    caps: (13, 14),
};

/// Sons da Biblioteca (ids do `WRITER.SND`; `building-salas.md` §6 e a
/// varredura do original).
pub mod sfx {
    /// Moldura (pegar, clicar) e plaqueta.
    pub const PICK: u16 = 0x21c;
    /// Impressora e a abertura da prévia.
    pub const PRINTER: u16 = 0x212;
    pub const PREVIEW: u16 = 0x2f6;
    /// Clique na lixeira.
    pub const TRASH: u16 = 0x2f8;
    /// "EPA!" do Rex (no original, soltar na lixeira só toca este).
    pub const ALERT: u16 = 0x2fe;
    pub const REFUSE: u16 = 0x217;
    /// Luminária "Tchau".
    pub const LAMP: u16 = 0x2f7;
    /// Interruptor da gaveta.
    pub const SWITCH: u16 = 0x308;
    /// Seta "Saguão".
    pub const LOBBY: u16 = 0x2fb;
}

/// Os documentos do Writer na Biblioteca.
pub const DOCUMENTS: Works = Works {
    load: load_documents,
    load_wall,
    save_wall,
    open: open_document,
    print: print_document,
    remove: |it| std::fs::remove_file(&it.path),
    drives,
};

/// A Biblioteca.
pub const WRITER_LIBRARY: GallerySpec = GallerySpec {
    kit: &WRITER_KIT,
    background: Some(0x6807),
    elements: &ELEMENTS,
    frames: &FRAMES,
    frame_img: None,
    frame_size: (102, 124),
    picture: Picture::Fit {
        area: PAGE_AREA,
        bg: PAGE_BG,
        edges: true,
    },
    plate: PLATE,
    quit: LAMP,
    printer: PRINTER,
    trash: TRASH,
    lobby: LOBBY_ARROW,
    mouse_hole: MOUSE_HOLE,
    drawer: Some(DRAWER),
    rec: GalleryRecords {
        welcome: 6,
        welcome2: 7,
        // Reorganizar, Abrir, Jogar fora, Ver outros documentos
        welcome_tips: [8, 9, 11, 12],
        // Imprimir, Copiar para um disquete, Examinar outras unidades, Voltar
        welcome2_actions: &[Some(10), Some(14), Some(13), None],
        print: 10,
        trash: 11,
        work_on: 41,
        wand: 58,
        switch: Some(25),
    },
    alerts: GalleryAlerts {
        block: 0x0805,
        recycle: 23,
        recycle_wall: 22,
        not_yours_wall: 34,
        not_yours: 35,
        plate: Some(30),
        already_there: Some(31),
        cant_copy: Some(14),
    },
    welcome2_drop: &[],
    text: GalleryTexts {
        welcome: (
            "Seja bem-vindo à Biblioteca! O que você quer fazer?",
            &[
                "Reorganizar a parede",
                "Abrir um documento",
                "Jogar documentos fora",
                "Ver outros documentos",
                "Outra coisa...",
            ],
        ),
        welcome2: (
            "O que você quer fazer?",
            &[
                "Imprimir um documento",
                "Copiar para um disquete",
                "Examinar outras unidades ou discos?",
                "Voltar para outras escolhas...",
            ],
        ),
        work_on: (
            "Se você quiser trabalhar neste documento, eu vou levar você para o Escritório na Bolsa Mágica.",
            &[
                "Estou pronto para ir!",
                "Ops! Não quero trabalhar com isto!",
            ],
        ),
        not_yours: "Não, obrigado... Não posso comer esse documento — ele não é seu. Vou tirá-lo da parede e trocar por uma coisa quase tão boa quanto!",
        recycle_wall: (
            "Tem certeza de que você quer que o Rex coma isso? Você quer jogar fora esse documento ou só tirá-lo da parede?",
            &[
                "Jogue fora! Eu nunca mais quero olhar para isso!",
                "Tire da parede, posso querer usar depois",
                "Ops! Deixa pra lá. Deixe na parede",
            ],
        ),
        recycle: (
            "O Rex está se preparando para comer isso... Você tem certeza de que quer que eu coma o documento?",
            &[
                "Jogue fora! Nunca mais quero olhar para isso!",
                "Tira a mão daí! Posso precisar mais tarde.",
            ],
        ),
        plate: "Você não pode mudar esse título. Você tentou mudar o título de um documento que não é seu.",
        already_there: "Já está lá! O documento já está onde você me pediu para colocar.",
        cant_copy: "Sinto muito, não posso copiar isso.",
        sample_log: "os exemplos do Max não podem ir para a lixeira",
    },
    snd: GallerySounds {
        // a Biblioteca não tem fala de chegada (só a do quadro do saguão)
        arrival: None,
        pick: sfx::PICK,
        choice: snd::CHOICE,
        printer: sfx::PRINTER,
        preview: sfx::PREVIEW,
        trash: sfx::TRASH,
        trash_drop: &[sfx::ALERT],
        alert: sfx::ALERT,
        refuse: sfx::REFUSE,
        quit: &[sfx::LAMP, sfx::ALERT],
        mczee: snd::MCZEE,
        lobby: sfx::LOBBY,
        mouse_hole: snd::MOUSE_HOLE,
        switch: sfx::SWITCH,
    },
    pos: GalleryPositions {
        // medidas pela diferença de capturas (só a caixa, sem o rabicho)
        welcome: (129, 234),
        tip: (129, 234),
        tips: &[
            (8, (330, 123)),
            (9, (250, 163)),
            (10, (458, 200)),
            (11, (402, 211)),
            (12, (333, 179)),
            (13, (288, 111)),
            (14, (268, 80)),
        ],
        printer_tip: (129, 233),
        trash_tip: (129, 233),
        switch_tip: (129, 233),
        work_on: (25, 20),
        // `[41]` em cada moldura (TL, TR, BR, BL), pela diferença de capturas
        work_on_at: &[(286, 29), (48, 29), (48, 150), (286, 150)],
        alert: (192, 156),
        quit: (208, 179),
        plate_alert: (202, 126),
    },
    character: (g::MCZEE_COSTUMES, g::MCZEE_POS),
    bag: super::room_objects::BagLayout {
        closed: (137, 364),
        open: (137, 386),
        suitcase: (366, 333),
        book: (281, 336),
        wand: (336, 392),
    },
    surprises: g::LIBRARY_SURPRISES,
    works: &DOCUMENTS,
};

/// Prévia de impressão da Biblioteca: a janela branca de moldura laranja,
/// a folha em pé numa moldura cinza, o McZee `4c03` e os balões do
/// `WRITER.EXE` (medida em `cwlib/pr1.png`, `pr2.png` da varredura).
pub const WRITER_PREVIEW: PreviewSpec = PreviewSpec {
    program: Program::Writer,
    window: (16, 32, 575, 431),
    gray_bar: None,
    sheet_frame: Some((66, 62, 210, 270)),
    page: (76, 70, 190, 254),
    mczee: (0x4c03, 545, 0),
    balloon_right: 581,
    balloon_top: 63,
    alert_pos: (209, 149),
    ready: (
        0x4c00,
        8,
        "OK. Já estou pronto para imprimir!\nAgora é uma boa hora para ver se a sua impressora está ligada.\nQuando você estiver pronto para imprimir, clique em Segue.",
    ),
    printing: (
        0x4c00,
        9,
        "Eu ainda estou imprimindo... um pouquinho de paciência!\nPara cancelar a impressão, aperte Esc.",
    ),
    done: (
        0x4c00,
        11,
        "Terminei de imprimir o documento. Foi divertido!\nPara voltar ao seu trabalho, clique em Pára.",
    ),
    cant_print: (
        0x0805,
        21,
        "Sua impressora está com algum problema.\nNão consigo imprimir o seu documento.",
    ),
    canceled: (0x4c02, 2, "Eu cancelei o trabalho de impressão para você!"),
    // o dinossauro atrás do rato, no canto de baixo (`22_imprimindo.png`)
    walker: Walker::Frames {
        frames: &[0x4c0b, 0x4c0f, 0x4c17],
        at: (20, 430),
        step: 4,
        period: 0.2,
    },
    snd_stop: 0x2ec,
    snd_next: 0x2eb,
    snd_alert: 0x2fe,
};

/// Um documento lido para a Biblioteca.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocInfo {
    pub title: Option<String>,
    pub author: String,
    /// Texto puro (parágrafos separados por `\n`).
    pub text: String,
    /// Alinhamento de cada parágrafo (o que faltar é à esquerda).
    pub aligns: Vec<Align>,
    /// Folha em pontos e margens (cima, esquerda, baixo, direita).
    pub page: (u32, u32),
    pub margins: (i32, i32, i32, i32),
    /// Miniatura do arquivo (`THMB`) em RGB, com largura e altura.
    pub thumb: Option<(u32, u32, Vec<[u8; 3]>)>,
}

impl DocInfo {
    /// Documento só de texto, na folha padrão.
    pub fn plain(text: &str) -> DocInfo {
        DocInfo {
            title: None,
            author: String::new(),
            text: text.to_string(),
            aligns: Vec::new(),
            page: SHEET,
            margins: (MARGIN, MARGIN, MARGIN, MARGIN),
            thumb: None,
        }
    }
}

fn is_ext(p: &Path, ext: &str) -> bool {
    p.extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(ext))
}

/// Lê um `.MAX`: título, autor, texto, alinhamentos, folha e miniatura.
pub fn read_doc(path: &Path) -> Option<DocInfo> {
    let data = std::fs::read(path).ok()?;
    let d = formats::max::read(&data).ok()?;
    let thumb = d.thumbnail_bitmap().map(|b| {
        let pal = b.palette.clone().unwrap_or_else(|| {
            formats::pkbm::default_palette_for(
                b.depth,
                &formats::max::THUMB_PALETTE,
                &formats::max::THUMB_PALETTE,
            )
        });
        let px = b
            .pixels
            .iter()
            .map(|&i| {
                pal.get(i as usize)
                    .map_or([255, 255, 255], |c| [c.r, c.g, c.b])
            })
            .collect();
        (b.width as u32, b.height as u32, px)
    });
    let title = Some(d.add.title.trim().to_string()).filter(|t| !t.is_empty());
    let aligns = d
        .paragraphs()
        .iter()
        .map(|&(start, _)| d.para_props_at(start).align)
        .collect();
    let dop = &d.dop;
    let page = (u32::from(dop.page_width), u32::from(dop.page_height));
    let page = if page.0 >= 72 && page.1 >= 72 && page.0 <= 2000 && page.1 <= 2000 {
        page
    } else {
        SHEET
    };
    let m = |v: u16| i32::from(v).clamp(0, 200);
    Some(DocInfo {
        title,
        author: d.add.author.trim().to_string(),
        // `plain_text` já separa os parágrafos com `\n`, sem as âncoras
        text: d.plain_text(),
        aligns,
        page,
        margins: (
            m(dop.margin_top),
            m(dop.margin_left),
            m(dop.margin_bottom),
            m(dop.margin_right),
        ),
        thumb,
    })
}

/// Exemplos do Max (`SHARED/USERS/MAX/*.MAX`), por nome.
pub fn list_samples(data_dir: &Path) -> Vec<PathBuf> {
    let Some(d) = find_ci(data_dir, &prefs::SAMPLES_DIR.join("/")) else {
        return Vec::new();
    };
    let Ok(rd) = std::fs::read_dir(d) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| is_ext(p, "max"))
        .collect();
    // SAMP2 antes de SAMP10
    v.sort_by_key(|p| {
        let s = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_uppercase())
            .unwrap_or_default();
        let n: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        (n.parse::<u32>().unwrap_or(u32::MAX), s)
    });
    v
}

/// Índice da cor mais próxima de `c` na paleta.
fn nearest(pal: &[Rgb], c: [u8; 3]) -> u8 {
    let d = |p: &Rgb| {
        let (r, g, b) = (
            p.r as i32 - c[0] as i32,
            p.g as i32 - c[1] as i32,
            p.b as i32 - c[2] as i32,
        );
        r * r + g * g + b * b
    };
    pal.iter()
        .take(256)
        .enumerate()
        .min_by_key(|(_, p)| d(p))
        .map_or(0, |(i, _)| i as u8)
}

/// Reduz uma imagem RGB para caber em `max` (proporção mantida) como o
/// `StretchBlt` do original no modo de juntar linhas (`BLACKONWHITE`): cada
/// pixel de destino junta só os pixels de origem que caem nele (um ou
/// dois por eixo, sem sobrepor), ficando com o mais escuro. Nos índices da
/// paleta.
pub fn fit_thumb(w: u32, h: u32, px: &[[u8; 3]], max: (u32, u32), pal: &[Rgb]) -> Canvas {
    if w == 0 || h == 0 {
        return Canvas::new(1, 1);
    }
    let k = (max.0 as f32 / w as f32)
        .min(max.1 as f32 / h as f32)
        .min(1.0);
    let (tw, th) = (
        ((w as f32 * k).round() as u32).max(1),
        ((h as f32 * k).round() as u32).max(1),
    );
    // origem de cada coordenada de destino: [início, fim)
    let span = |t: u32, n: u32, tn: u32| {
        let first = (t * n).div_ceil(tn);
        let end = ((t + 1) * n).div_ceil(tn).min(n);
        (first.min(n - 1), end.max(first + 1).min(n))
    };
    let lum = |p: [u8; 3]| p[0] as u32 * 3 + p[1] as u32 * 6 + p[2] as u32;
    let mut c = Canvas::new(tw, th);
    let mut cache: Vec<([u8; 3], u8)> = Vec::new();
    for ty in 0..th {
        let (sy0, sy1) = span(ty, h, th);
        for tx in 0..tw {
            let (sx0, sx1) = span(tx, w, tw);
            let mut best = [255u8, 255, 255];
            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let p = px[(sy * w + sx) as usize];
                    if lum(p) < lum(best) {
                        best = p;
                    }
                }
            }
            let i = match cache.iter().find(|(k, _)| *k == best) {
                Some(&(_, i)) => i,
                None => {
                    let i = nearest(pal, best);
                    cache.push((best, i));
                    i
                }
            };
            c.px[(ty * tw + tx) as usize] = i;
        }
    }
    c
}

/// Miniatura de um documento sem `THMB`: uma
/// página de 108×144 com a borda e cada letra como um risquinho cinza.
pub fn text_thumb(text: &str) -> (u32, u32, Vec<[u8; 3]>) {
    let (w, h) = (108u32, 144u32);
    let mut px = vec![[255u8, 255, 255]; (w * h) as usize];
    for x in 0..w {
        px[x as usize] = [0, 0, 0];
        px[((h - 1) * w + x) as usize] = [0, 0, 0];
    }
    for y in 0..h {
        px[(y * w) as usize] = [0, 0, 0];
        px[(y * w + w - 1) as usize] = [0, 0, 0];
    }
    const GRAY: [u8; 3] = [96, 96, 96];
    let (left, right, top, line) = (12u32, 96u32, 14u32, 4u32);
    let (mut x, mut y) = (left, top);
    'text: for para in text.split('\n') {
        for word in para.split(' ') {
            let n = word.chars().count() as u32;
            if x > left && x + n > right {
                x = left;
                y += line;
            }
            if y + 2 >= h - 10 {
                break 'text;
            }
            for _ in 0..n {
                if x < right {
                    px[(y * w + x) as usize] = GRAY;
                    px[((y + 1) * w + x) as usize] = GRAY;
                }
                x += 1;
            }
            x += 1;
        }
        x = left;
        y += line;
    }
    (w, h, px)
}

/// Os documentos da criança e os exemplos do Max, com a página reduzida
/// para a moldura.
fn load_documents(dir: Option<&Path>, assets: &Assets, pal: &[Rgb]) -> Vec<Item> {
    let max = (PAGE_AREA.2 as u32, PAGE_AREA.3 as u32);
    let mut items = Vec::new();
    let mut add = |paths: Vec<PathBuf>, sample: bool| {
        for path in paths {
            let Some(doc) = read_doc(&path) else { continue };
            let (w, h, px) = doc.thumb.clone().unwrap_or_else(|| text_thumb(&doc.text));
            let name = doc.title.clone().unwrap_or_else(|| stamped_name(&path));
            items.push(Item::new(
                path,
                name,
                doc.title,
                doc.author,
                sample,
                fit_thumb(w, h, &px, max, pal),
            ));
        }
    };
    add(dir.map(store::list).unwrap_or_default(), false);
    add(list_samples(assets.data_dir()), true);
    items
}

/// Pasta dos usuários e apelido, a partir da pasta do usuário da sala.
fn user_of(dir: Option<&Path>) -> (Option<&Path>, String) {
    let users = dir.and_then(Path::parent);
    let user = dir
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    (users, user)
}

/// As molduras: os `FILR` 1–4 do usuário (`prefs::library`, com os exemplos
/// de um usuário novo); as que não se acham ficam com os outros documentos,
/// na ordem da lista.
fn load_wall(dir: Option<&Path>, assets: &Assets, items: &[Item], n: usize) -> Vec<Option<usize>> {
    let (users, user) = user_of(dir);
    let mut wall = vec![None; n];
    let slots = prefs::library(users, &user, assets.data_dir());
    for (k, (_, file)) in slots.into_iter().enumerate().take(n) {
        wall[k] = file
            .and_then(|f| items.iter().position(|it| it.path == f))
            .filter(|i| !wall.contains(&Some(*i)));
    }
    fill_wall(&mut wall, items);
    wall
}

/// Grava as molduras nos `FILR` 1–4 do `~MAXUSER.INF` do usuário, mantendo
/// o resto do arquivo.
fn save_wall(dir: Option<&Path>, assets: &Assets, wall: &[Option<&Item>]) -> Result<(), String> {
    let (Some(users), user) = user_of(dir) else {
        return Ok(());
    };
    let data = assets.data_dir();
    let mut p = prefs::load(Some(users), &user, data);
    for (k, &slot) in prefs::LIBRARY_SLOTS.iter().enumerate() {
        let doc = wall.get(k).copied().flatten().and_then(|it| {
            let title = it.title.clone().unwrap_or_default();
            prefs::wall_doc(&it.path, &title, Some(users), data)
        });
        match doc {
            Some(d) => prefs::set_slot(&mut p, slot, d),
            None => p.wall.retain(|d| d.slot != slot),
        }
    }
    prefs::save(users, &user, &p).map_err(|e| e.to_string())
}

/// Clique duplo (ou "Estou pronto para ir!"): o documento vira o último
/// (`FILR 0`, o quadro do Escritório) e o Escritório o abre.
fn open_document(
    it: &Item,
    dir: Option<&Path>,
    assets: &Assets,
    _: &[Rgb],
    _: Program,
) -> Vec<Effect> {
    let mut fx = Vec::new();
    if let (Some(users), user) = user_of(dir) {
        let title = it.title.clone().unwrap_or_default();
        if let Err(e) = prefs::remember_last(users, &user, assets.data_dir(), &it.path, &title) {
            fx.push(Effect::Log(format!("biblioteca: {e}")));
        }
    }
    fx.push(Effect::OpenDocument(it.path.clone()));
    fx
}

/// Folha de 595×792 pontos (o `PRNT` do original, 72 dpi).
pub const SHEET: (u32, u32) = (595, 792);
/// Margens da folha impressa (uma polegada) e entrelinha.
const MARGIN: i32 = 72;

/// A 1ª página do documento em texto simples, com a fonte de 16 pontos do
/// programa (sem ela, a 8×8): os parágrafos, com o alinhamento de cada um,
/// dentro das margens, até encher a folha. A diagramação fiel (fontes,
/// tamanhos, figuras, bordas) é de outra tarefa.
pub fn render_first_page(doc: &DocInfo, assets: &Assets) -> Rgb8 {
    let (w, h) = doc.page;
    let (top, left, bottom, right) = doc.margins;
    let mut f = Frame::new(w, h);
    f.fill([255, 255, 255]);
    let font = assets.font(16);
    let width = |s: &str| match &font {
        Some(ft) => ft.width(s) as i32,
        None => s.chars().count() as i32 * 8,
    };
    let lh = font.as_ref().map_or(10, |ft| ft.height as i32 + 4);
    let max_w = (w as i32 - left - right).max(40);
    let last_y = h as i32 - bottom - lh;
    let mut y = top;
    let put = |f: &mut Frame, y: i32, s: &str, align: Align| {
        let x = match align {
            Align::Center => left + (max_w - width(s)) / 2,
            Align::Right => left + max_w - width(s),
            Align::Left | Align::Justify => left,
        };
        match &font {
            Some(ft) => {
                f.text_font(ft, x, y, s, [0, 0, 0]);
            }
            None => f.text(x, y, s, [0, 0, 0]),
        }
    };
    'page: for (k, para) in doc.text.split('\n').enumerate() {
        let align = doc.aligns.get(k).copied().unwrap_or_default();
        let mut line = String::new();
        for word in para.split(' ') {
            let cand = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && width(&cand) > max_w {
                if y > last_y {
                    break 'page;
                }
                put(&mut f, y, &line, align);
                y += lh;
                line = word.to_string();
            } else {
                line = cand;
            }
        }
        if y > last_y {
            break;
        }
        put(&mut f, y, &line, align);
        y += lh;
    }
    Rgb8::from_rgba(w, h, f.pixels())
}

/// Documento solto na impressora: a prévia com a 1ª página.
fn print_document(
    it: &Item,
    _: &[Rgb],
    dir: Option<PathBuf>,
    command: &str,
    assets: &Assets,
) -> Result<PrintPreview, String> {
    let doc = read_doc(&it.path)
        .ok_or_else(|| format!("imprimir: não consegui abrir {}", it.path.display()))?;
    let page = render_first_page(&doc, assets);
    Ok(PrintPreview::with_spec(
        &WRITER_PREVIEW,
        Sheet::Page(page),
        dir,
        command,
        assets,
    ))
}

/// Unidades da gaveta: a pasta do usuário (onde se copia) e os exemplos do
/// Max (só de leitura).
fn drives(dir: Option<&Path>, _: &Assets) -> Vec<Drive> {
    let mut v = Vec::new();
    if let Some(d) = dir {
        v.push(Drive {
            label: "Ateliê Livre".into(),
            dir: Some(d.to_path_buf()),
        });
    }
    v.push(Drive {
        label: "Exemplos do Max".into(),
        dir: None,
    });
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::ImageRef;
    use crate::scenes::testutil::{assets, empty_assets};

    #[test]
    fn text_thumb_is_a_page_with_marks() {
        let (w, h, px) = text_thumb("Era uma vez\num gato");
        assert_eq!((w, h), (108, 144));
        assert_eq!(px[0], [0, 0, 0]);
        let gray = px.iter().filter(|p| **p == [96, 96, 96]).count();
        // "Era uma vez" e "um gato": 15 letras, dois pixels cada
        assert_eq!(gray, 30);
    }

    #[test]
    fn fit_thumb_keeps_proportion() {
        let pal = vec![
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            Rgb { r: 0, g: 0, b: 0 },
        ];
        let mut px = vec![[255u8, 255, 255]; 108 * 144];
        // uma linha de 1 px no meio: cai numa fileira e não some
        for x in 0..108 {
            px[72 * 108 + x] = [0, 0, 0];
        }
        let c = fit_thumb(108, 144, &px, (100, 100), &pal);
        assert_eq!((c.w, c.h), (75, 100));
        assert!((0..c.h).any(|y| (0..c.w).all(|x| c.px[(y * c.w + x) as usize] == 1)));
        // já pequena: fica do tamanho que tem
        let c = fit_thumb(10, 20, &vec![[0, 0, 0]; 200], (100, 100), &pal);
        assert_eq!((c.w, c.h), (10, 20));
    }

    #[test]
    fn first_page_has_ink_inside_the_margins() {
        let (_d, a) = empty_assets();
        let mut doc = DocInfo::plain("Era uma vez um gato que gostava de escrever.\nFim.");
        doc.aligns = vec![Align::Left, Align::Center];
        let s = render_first_page(&doc, &a);
        assert_eq!((s.w, s.h), SHEET);
        let ink: Vec<(u32, u32)> = (0..s.h)
            .flat_map(|y| (0..s.w).map(move |x| (x, y)))
            .filter(|&(x, y)| s.px[(y * s.w + x) as usize] != [255, 255, 255])
            .collect();
        assert!(!ink.is_empty());
        assert!(
            ink.iter()
                .all(|&(x, y)| x >= MARGIN as u32 && y >= MARGIN as u32)
        );
    }

    /// Com os dados reais: as figuras, os balões e os exemplos existem.
    #[test]
    fn library_matches_real_data() {
        let Some(a) = assets() else { return };
        let p = Program::Writer;
        let mut els: Vec<(u16, u16, i32, i32)> = ELEMENTS.to_vec();
        els.extend([DRAWER.switch_off, DRAWER.switch_on, DRAWER.drawer]);
        for (img, mask, _, _) in els {
            assert!(a.has_image(&ImageRef::Pic(p, img)), "{img:#x}");
            assert_eq!(a.pic_next_id(p, img, 2), Some(mask), "{img:#x}");
        }
        let wall = a.image(&ImageRef::Pic(p, 0x692b));
        assert_eq!((wall.width, wall.height), (274, 271));
        let bg = a.image(&ImageRef::Pic(p, 0x6807));
        assert_eq!((bg.width, bg.height), (640, 480));
        let recs = a.dialogs(p, 0x0bb8);
        for (k, n) in [(6, 5), (7, 4), (41, 2), (8, 0), (25, 0), (58, 0)] {
            assert_eq!(recs[k].options().len(), n, "0bb8[{k}]");
        }
        let alerts = a.dialogs(p, 0x0805);
        for (k, n) in [(22, 3), (23, 2), (30, 0), (31, 0), (34, 0), (35, 0)] {
            assert_eq!(alerts[k].options().len(), n, "0805[{k}]");
        }
        let machine = a.dialogs(p, 0x4c00);
        assert!(machine[8].plain().contains("pronto para imprimir"));
        assert!(machine[11].plain().contains("Terminei"));
        assert!(alerts[21].plain().contains("impressora"));
        assert!(a.dialogs(p, 0x4c02)[2].plain().contains("cancelei"));
        assert_eq!(a.image(&ImageRef::Pic(p, 0x4c03)).width, 95);
        // exemplos do Max, com título e miniatura
        let samples = list_samples(a.data_dir());
        assert_eq!(samples.len(), 21);
        for name in &prefs::DEFAULT_WALL[1..] {
            let s = samples
                .iter()
                .find(|p| {
                    p.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(name)
                })
                .unwrap_or_else(|| panic!("{name}"));
            let d = read_doc(s).unwrap();
            assert!(d.title.is_some() && d.thumb.is_some(), "{name}");
        }
    }
}
