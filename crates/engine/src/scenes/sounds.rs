//! Ferramenta Seleciona-sons da tela de pintura. A faixa de opções tem três
//! faixas de teatro por página com as categorias (carrosséis `CSRO` de
//! `MAG100.SND` e `MAX100.SND`, na ordem dos ids, como no original: 8
//! categorias em 3 páginas), o controle de volume, o Cachorrinho (sons em
//! arquivos `.wav`), o Gravador e as setas de página. Escolher uma categoria
//! abre a lista de nomes à esquerda; o som escolhido fica "na mão" e se
//! associa ao adesivo ou à palavra clicada (campo `sound` do objeto, gravado
//! no `*.objetos.json`); a Varinha Mágica toca. Geometria medida no original
//! (varredura `paint-sounds`, ver `docs/telas/fine-artist.md`).

pub mod finder;
pub mod recorder;

pub use finder::Finder;
pub use recorder::Recorder;

use crate::assets::{Assets, ItemKind, Program, SoundRef};
use crate::paint::{CANVAS_H, CANVAS_W, ORIGIN};
use crate::scene::Effect;
use crate::ui::Rect;
pub(crate) use crate::ui::inside;
use crate::{Frame, Image};
use formats::chnk;
use std::path::Path;

/// Bibliotecas de sons de `SHARED/STUFF`; os ids dos carrosséis continuam
/// de um arquivo para o outro (Grave 0, Agudo 1, Dinossauro 2…).
const LIBRARIES: [&str; 2] = ["MAG100.SND", "MAX100.SND"];
/// Pontos de som soltos de versões antigas do port (lidos uma vez e
/// presos aos objetos debaixo deles; o arquivo some ao gravar).
pub const SPOTS_NAME: &str = "ultimo.sons.json";

/// Faixas de teatro: contorno de 1 bit `0x024e` com máscara `0x024f`.
const BANNER_PIC: u16 = 0x024e;
const BANNER_MASK: u16 = 0x024f;
const BANNER_W: i32 = 112;
const BANNER_H: i32 = 37;
pub const PER_PAGE: usize = 3;
/// Topo do texto na placa amarela, relativo à faixa.
const PLATE_TEXT_Y: i32 = 20;

/// Figura com máscara num lugar fixo: (figura, máscara, x, y).
pub type Placed = (u16, u16, i32, i32);

/// Onde fica cada coisa da faixa do Seleciona-sons e de que programa vêm as
/// figuras e os sons. A pintura do Fine Artist e o Escritório do Creative
/// Writer têm a mesma ferramenta (as mesmas categorias, lista, Cachorrinho
/// e Gravador) com medidas e figuras diferentes: [`ARTIST`] e
/// `scenes::writer::sounds::LAYOUT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub program: Program,
    /// Faixa de teatro: figura, máscara e se a figura é só o contorno de 1
    /// bit a pintar com as cores medidas (pintura) ou já colorida (Writer).
    pub banner: (u16, u16, bool),
    pub banner_x0: i32,
    pub banner_step: i32,
    pub banner_y: i32,
    /// Controle de volume: área do clique, trilho e cursor (figura,
    /// máscara, x) com o alto do cursor indo de `knob_top` (100) a
    /// `knob_bottom` (0).
    pub volume: Rect,
    pub track: Placed,
    pub knob: (u16, u16, i32),
    pub knob_top: i32,
    pub knob_bottom: i32,
    pub dog: Rect,
    pub dog_pic: Placed,
    pub recorder: Rect,
    pub recorder_pic: (u16, u16),
    pub arrow_up: Rect,
    pub arrow_down: Rect,
    /// Fundo atrás das setas e do número da página (Writer: `08d9`).
    pub arrows_back: Option<Placed>,
    pub page_box: Rect,
    /// Lista de nomes: `x` debaixo da 1ª faixa e o passo de uma faixa para a
    /// outra (0 = sempre no mesmo lugar), topo, (largura, altura, passo)
    /// das linhas e quantas cabem (a de "Sem Som" incluída; com mais, a
    /// última vira as setas de rolagem).
    pub list_x0: i32,
    pub list_step: i32,
    pub list_y: i32,
    pub row: (i32, i32, i32),
    pub list_rows: usize,
    /// Linha das setas da lista: caixa laranja como as outras (pintura) ou
    /// branca, de 16 px, só com o contorno (Writer).
    pub plain_arrows: bool,
    /// Área do documento: o clique ali vai para o objeto debaixo.
    pub canvas: Rect,
}

/// A faixa na tela de pintura (varredura `paint-sounds`).
pub const ARTIST: Layout = Layout {
    program: Program::Artist,
    banner: (BANNER_PIC, BANNER_MASK, true),
    banner_x0: 9,
    banner_step: BANNER_W,
    banner_y: 61,
    volume: (368, 56, 26, 44),
    track: (0x0251, 0x0257, 374, 62),
    knob: (0x0259, 0x025b, 370),
    knob_top: 63,
    knob_bottom: 88,
    dog: (405, 58, 42, 40),
    dog_pic: (0x025d, 0x0263, 409, 63),
    recorder: (456, 59, 43, 41),
    recorder_pic: (0x0265, 0x0267),
    arrow_up: (504, 58, 16, 14),
    arrow_down: (504, 90, 16, 14),
    arrows_back: None,
    page_box: (505, 75, 16, 12),
    list_x0: 19,
    list_step: 0,
    list_y: 97,
    row: (94, 24, 23),
    list_rows: 14,
    plain_arrows: false,
    canvas: (ORIGIN.0, ORIGIN.1, CANVAS_W as i32, CANVAS_H as i32),
};
pub const DOG: Rect = ARTIST.dog;
pub const RECORDER: Rect = ARTIST.recorder;
pub const ARROW_UP: Rect = ARTIST.arrow_up;
pub const ARROW_DOWN: Rect = ARTIST.arrow_down;

/// Sons do `ARTIST.SND` (medidos no original).
pub const SND_CATEGORY: u16 = 0x0233;
pub const SND_PAGE: u16 = 0x021d;
pub const SND_VOLUME: u16 = 0x020b;
pub const SND_VOLUME_TEST: u16 = 0x022c;
pub const SND_DOG: u16 = 0x02f1;
pub const SND_RECORDER: u16 = 0x0224;
pub const SND_CLOSE: u16 = 0x02ed;
pub const SND_CANCEL: u16 = 0x022f;

/// Balões do bloco `WHAT 1004` (Seleciona-sons).
pub const HELP_PICK: usize = 2;
pub const HELP_THEN_CLICK: usize = 3;
pub const HELP_VOLUME: usize = 4;
pub const HELP_RECORDER: usize = 5;
pub const HELP_ATTACHED: usize = 10;
pub const HELP_DOG: usize = 12;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const ORANGE: [u8; 3] = [255, 140, 50];
const LILAC: [u8; 3] = [136, 136, 230];
const GREEN: [u8; 3] = [92, 203, 83];
const PINK: [u8; 3] = [250, 74, 114];
const YELLOW: [u8; 3] = [255, 238, 76];
const PURPLE: [u8; 3] = [147, 44, 163];
const BLUE: [u8; 3] = [97, 65, 219];

/// Cores das regiões do contorno da faixa de teatro: um ponto de cada região
/// (pintada como num livro de colorir) e a cor medida no original.
const BANNER_FILL: [(i32, i32, [u8; 3]); 64] = [
    (6, 1, LILAC),
    (48, 1, ORANGE),
    (89, 1, LILAC),
    (45, 2, ORANGE),
    (68, 2, ORANGE),
    (3, 3, GREEN),
    (6, 3, GREEN),
    (44, 3, ORANGE),
    (50, 3, LILAC),
    (69, 3, ORANGE),
    (107, 3, GREEN),
    (43, 4, ORANGE),
    (70, 4, ORANGE),
    (9, 8, PINK),
    (11, 8, PINK),
    (92, 8, PINK),
    (104, 8, PINK),
    (5, 9, PINK),
    (106, 9, YELLOW),
    (108, 9, PINK),
    (7, 10, YELLOW),
    (42, 11, GREEN),
    (71, 11, GREEN),
    (17, 12, PURPLE),
    (39, 12, PINK),
    (73, 12, PINK),
    (91, 12, PURPLE),
    (11, 13, GREEN),
    (41, 13, PINK),
    (71, 13, PINK),
    (102, 13, GREEN),
    (40, 15, ORANGE),
    (46, 15, ORANGE),
    (70, 15, ORANGE),
    (98, 15, LILAC),
    (33, 16, LILAC),
    (35, 16, LILAC),
    (78, 16, LILAC),
    (97, 16, LILAC),
    (15, 17, LILAC),
    (17, 17, LILAC),
    (28, 17, LILAC),
    (30, 17, LILAC),
    (32, 17, LILAC),
    (34, 17, LILAC),
    (79, 17, LILAC),
    (81, 17, LILAC),
    (83, 17, LILAC),
    (94, 17, LILAC),
    (96, 17, LILAC),
    (98, 17, LILAC),
    (17, 21, YELLOW),
    (8, 25, YELLOW),
    (104, 25, YELLOW),
    (5, 30, PINK),
    (108, 30, PINK),
    (7, 31, YELLOW),
    (104, 31, YELLOW),
    (3, 34, YELLOW),
    (6, 34, YELLOW),
    (13, 34, YELLOW),
    (99, 34, YELLOW),
    (102, 34, YELLOW),
    (109, 34, YELLOW),
];

/// Pontilhado mais escuro na moldura da nota musical.
const BANNER_SHADE: [(i32, i32); 17] = [
    (50, 3),
    (52, 3),
    (54, 3),
    (56, 3),
    (58, 3),
    (60, 3),
    (62, 3),
    (63, 3),
    (65, 4),
    (66, 6),
    (66, 7),
    (66, 8),
    (66, 9),
    (66, 11),
    (66, 12),
    (66, 13),
    (65, 16),
];

pub(crate) fn pic(id: u16) -> crate::assets::ImageRef {
    crate::assets::ImageRef::Pic(Program::Artist, id)
}

pub(crate) fn snd(id: u16) -> Effect {
    Effect::PlaySound(SoundRef::Snd(Program::Artist, id))
}

/// Uma categoria: nome e sons (nome, referência).
#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub sounds: Vec<(String, SoundRef)>,
}

/// O que está "na mão" para associar ao próximo adesivo ou palavra clicado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hand {
    /// "Sem Som": tira o som do objeto clicado.
    Silence,
    Sound {
        name: String,
        sound: SoundRef,
    },
}

impl Hand {
    pub fn sound(&self) -> Option<&SoundRef> {
        match self {
            Hand::Silence => None,
            Hand::Sound { sound, .. } => Some(sound),
        }
    }
}

/// Ponto de som solto de versões antigas do port (coordenadas do quadro).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    pub x: i32,
    pub y: i32,
    pub name: String,
    pub sound: SoundRef,
}

/// Resposta a um clique: efeitos, balão do bloco `WHAT 1004` a abrir e se
/// o clique caiu no quadro (o adesivo ou a palavra debaixo decide).
#[derive(Debug, Default, PartialEq)]
pub struct Reply {
    pub handled: bool,
    pub fx: Vec<Effect>,
    pub help: Option<usize>,
    pub canvas: bool,
}

impl Reply {
    fn done() -> Reply {
        Reply {
            handled: true,
            ..Reply::default()
        }
    }

    fn with_help(mut self, k: usize) -> Reply {
        self.help = Some(k);
        self
    }
}

/// Categorias de um arquivo: (id do carrossel, categoria).
fn file_categories(assets: &Assets, library: &str) -> Vec<(u16, Category)> {
    // sons por id de chunk, na ordem do catálogo (= ordem dos chunks no arquivo)
    let Some(src) = assets
        .catalog()
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(library))
    else {
        return Vec::new();
    };
    let mut by_id = std::collections::HashMap::new();
    for item in &src.items {
        if let ItemKind::Sound(r) = &item.kind {
            // rótulo "SND  n nome" (a etiqueta tem um espaço no fim)
            let mut parts = item.label.split_whitespace();
            let (Some(tag), Some(id)) = (parts.next(), parts.next()) else {
                continue;
            };
            if tag != "SND" {
                continue;
            }
            let name = parts.collect::<Vec<_>>().join(" ");
            if let Ok(id) = id.parse::<u16>() {
                by_id.insert(id, (name, r.clone()));
            }
        }
    }
    let Some(data) = assets.library_bytes(library) else {
        return Vec::new();
    };
    let Ok(f) = chnk::parse(data) else {
        return Vec::new();
    };
    f.chunks
        .iter()
        .filter(|c| &c.tag.0 == b"CSRO")
        .filter_map(|c| {
            let list = c.parts.iter().find(|p| &p.tag.0 == b"CSRO")?;
            let ids = crate::clipart::parse_list(list.data);
            let sounds: Vec<(String, SoundRef)> =
                ids.iter().filter_map(|id| by_id.get(id).cloned()).collect();
            (!sounds.is_empty()).then(|| {
                (
                    c.id,
                    Category {
                        name: c.name.clone().unwrap_or_default(),
                        sounds,
                    },
                )
            })
        })
        .collect()
}

/// Lê as categorias de `MAG100.SND` e `MAX100.SND` na ordem dos ids dos
/// carrosséis; um arquivo ausente só tira as categorias dele.
pub fn load_categories(assets: &Assets) -> Vec<Category> {
    let mut cats: Vec<(u16, Category)> = LIBRARIES
        .iter()
        .flat_map(|lib| file_categories(assets, lib))
        .collect();
    cats.sort_by_key(|(id, _)| *id);
    cats.into_iter().map(|(_, c)| c).collect()
}

/// Faixa de teatro colorida: o contorno de 1 bit com as regiões pintadas
/// pelas cores medidas (`BANNER_FILL`); fora da máscara, transparente.
pub fn banner_image(outline: &Image, mask: &Image) -> Option<Image> {
    let (w, h) = (outline.width as i32, outline.height as i32);
    if (w, h) != (BANNER_W, BANNER_H)
        || (mask.width, mask.height) != (outline.width, outline.height)
    {
        return None;
    }
    let at = |img: &Image, x: i32, y: i32| -> [u8; 4] {
        let i = 4 * (y * w + x) as usize;
        [
            img.rgba[i],
            img.rgba[i + 1],
            img.rgba[i + 2],
            img.rgba[i + 3],
        ]
    };
    // opaco: máscara preta; branco do contorno = região a pintar
    let opaque = |x: i32, y: i32| at(mask, x, y)[0] < 128;
    let blank = |x: i32, y: i32| at(outline, x, y)[0] >= 128;
    let mut px: Vec<Option<[u8; 3]>> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| opaque(x, y).then(|| if blank(x, y) { WHITE } else { BLACK }))
        .collect();
    let mut painted = vec![false; px.len()];
    for &(sx, sy, color) in &BANNER_FILL {
        if !(0..w).contains(&sx) || !(0..h).contains(&sy) {
            continue;
        }
        let mut stack = vec![(sx, sy)];
        while let Some((x, y)) = stack.pop() {
            if !(0..w).contains(&x) || !(0..h).contains(&y) {
                continue;
            }
            let i = (y * w + x) as usize;
            if painted[i] || px[i] != Some(WHITE) {
                continue;
            }
            painted[i] = true;
            px[i] = Some(color);
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
        }
    }
    for &(x, y) in &BANNER_SHADE {
        let i = (y * w + x) as usize;
        if px[i].is_some() {
            px[i] = Some(BLUE);
        }
    }
    let mut rgba = Vec::with_capacity(px.len() * 4);
    for p in px {
        match p {
            Some([r, g, b]) => rgba.extend_from_slice(&[r, g, b, 255]),
            None => rgba.extend_from_slice(&[0, 0, 0, 0]),
        }
    }
    Some(Image {
        width: outline.width,
        height: outline.height,
        rgba,
    })
}

impl Layout {
    /// Altura do cursor do volume (0–100) e vice-versa.
    fn knob_y(&self, volume: u8) -> i32 {
        let (top, bottom) = (self.knob_top, self.knob_bottom);
        bottom - (volume.min(100) as i32 * (bottom - top) + 50) / 100
    }

    fn volume_at(&self, y: i32) -> u8 {
        // o meio do cursor (9 px) segue o ponteiro
        let (top, bottom) = (self.knob_top, self.knob_bottom);
        let at = (y - 4).clamp(top, bottom);
        ((bottom - at) * 100 / (bottom - top)) as u8
    }

    fn pic(&self, id: u16) -> crate::assets::ImageRef {
        crate::assets::ImageRef::Pic(self.program, id)
    }

    fn snd(&self, id: u16) -> Effect {
        Effect::PlaySound(SoundRef::Snd(self.program, id))
    }

    fn banner_x(&self, slot: usize) -> i32 {
        self.banner_x0 + slot as i32 * self.banner_step
    }

    /// Faixa de teatro já colorida (`None` = desenho de reserva).
    fn banner_image(&self, assets: &Assets) -> Option<Image> {
        let (img, mask, paint) = self.banner;
        if !assets.has_image(&self.pic(img)) {
            return None;
        }
        let (img, mask) = (assets.image(&self.pic(img)), assets.image(&self.pic(mask)));
        if paint {
            banner_image(&img, &mask)
        } else {
            Some(masked_image(&img, &mask))
        }
    }
}

/// A figura com o fora da máscara transparente.
fn masked_image(img: &Image, mask: &Image) -> Image {
    let mut out = img.clone();
    if (mask.width, mask.height) == (img.width, img.height) {
        for (p, m) in out.rgba.chunks_exact_mut(4).zip(mask.rgba.chunks_exact(4)) {
            if m[0] >= 128 {
                p[3] = 0;
            }
        }
    }
    out
}

pub struct Sounds {
    layout: Layout,
    cats: Vec<Category>,
    page: usize,
    /// Categoria com a lista de nomes aberta.
    open: Option<usize>,
    scroll: usize,
    hand: Option<Hand>,
    /// Arrastando o cursor do volume.
    dragging: bool,
    banner: Option<Image>,
    pub finder: Option<Finder>,
    pub recorder: Option<Recorder>,
    /// Pontos soltos lidos de `SPOTS_NAME`, a prender nos objetos.
    legacy: Vec<Spot>,
}

impl Sounds {
    /// A ferramenta da tela de pintura.
    pub fn new(assets: &Assets) -> Sounds {
        Sounds::with_layout(ARTIST, assets)
    }

    /// A ferramenta com outra faixa (o Escritório do Writer).
    pub fn with_layout(layout: Layout, assets: &Assets) -> Sounds {
        let banner = layout.banner_image(assets);
        let mut s = Sounds::with_categories(load_categories(assets), banner);
        s.layout = layout;
        s
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    fn with_categories(cats: Vec<Category>, banner: Option<Image>) -> Sounds {
        Sounds {
            layout: ARTIST,
            cats,
            page: 0,
            open: None,
            scroll: 0,
            hand: None,
            dragging: false,
            banner,
            finder: None,
            recorder: None,
            legacy: Vec::new(),
        }
    }

    pub fn categories(&self) -> usize {
        self.cats.len()
    }

    pub fn category_names(&self) -> Vec<&str> {
        self.cats.iter().map(|c| c.name.as_str()).collect()
    }

    pub fn pages(&self) -> usize {
        self.cats.len().div_ceil(PER_PAGE).max(1)
    }

    /// Página das categorias (0, 1, 2…).
    pub fn page(&self) -> usize {
        self.page
    }

    pub fn list_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn hand(&self) -> Option<&Hand> {
        self.hand.as_ref()
    }

    pub fn set_hand(&mut self, hand: Hand) {
        self.hand = Some(hand);
    }

    /// Tira da mão o que estava para ser associado.
    pub fn take_hand(&mut self) -> Option<Hand> {
        self.hand.take()
    }

    /// Algum diálogo (Cachorrinho, Gravador) aberto: os cliques são dele.
    pub fn modal(&self) -> bool {
        self.finder.is_some() || self.recorder.is_some()
    }

    pub fn dragging(&self) -> bool {
        self.dragging
    }

    /// Outra ferramenta ou o próprio Seleciona-sons: larga tudo.
    pub fn reset(&mut self) {
        self.open = None;
        self.hand = None;
        self.dragging = false;
        self.finder = None;
        self.recorder = None;
    }

    /// Recorder em gravação no meio de um `reset`: a gravação é descartada.
    pub fn stop_effects(&self) -> Vec<Effect> {
        self.recorder
            .as_ref()
            .and_then(|r| r.cancel_effect())
            .into_iter()
            .collect()
    }

    pub fn update(&mut self, _dt: f32) {}

    fn banner_rect(&self, slot: usize) -> Rect {
        let l = &self.layout;
        (l.banner_x(slot), l.banner_y - 3, BANNER_W, BANNER_H + 3)
    }

    fn list_len(&self) -> usize {
        self.open
            .and_then(|c| self.cats.get(c))
            .map_or(0, |c| c.sounds.len() + 1)
    }

    /// A lista tem mais linhas do que cabem: a última vira as setas.
    fn list_scrolls(&self) -> bool {
        self.list_len() > self.layout.list_rows
    }

    fn visible_rows(&self) -> usize {
        if self.list_scrolls() {
            self.layout.list_rows - 1
        } else {
            self.list_len()
        }
    }

    /// Linha `k` da lista aberta (debaixo da faixa dela, no Writer).
    fn row_rect(&self, k: usize) -> Rect {
        let l = &self.layout;
        let slot = self.open.map_or(0, |c| c % PER_PAGE) as i32;
        let (w, h, step) = l.row;
        (
            l.list_x0 + l.list_step * slot,
            l.list_y + k as i32 * step,
            w,
            h,
        )
    }

    /// Clique com a ferramenta ativa (sem diálogo aberto).
    pub fn click(&mut self, x: i32, y: i32, assets: &Assets) -> Reply {
        // lista de nomes aberta
        if self.open.is_some() {
            let rows = self.visible_rows();
            if let Some(k) = (0..rows).find(|&k| inside(self.row_rect(k), x, y)) {
                return self.pick(self.scroll + k);
            }
            let last = self.layout.list_rows - 1;
            if self.list_scrolls() && inside(self.row_rect(last), x, y) {
                let r = self.row_rect(last);
                let step = last - 1;
                let max = self.list_len().saturating_sub(last);
                self.scroll = if x < r.0 + r.2 / 2 {
                    self.scroll.saturating_sub(step)
                } else {
                    (self.scroll + step).min(max)
                };
                return self.reply(SND_PAGE);
            }
        }
        let first = self.page * PER_PAGE;
        if let Some(slot) = (0..PER_PAGE)
            .filter(|s| first + s < self.cats.len())
            .find(|&s| inside(self.banner_rect(s), x, y))
        {
            let c = first + slot;
            if self.open == Some(c) {
                self.open = None;
                return Reply::done();
            }
            self.open = Some(c);
            self.scroll = 0;
            return self.reply(SND_CATEGORY).with_help(HELP_PICK);
        }
        let l = self.layout;
        if inside(l.arrow_up, x, y) || inside(l.arrow_down, x, y) {
            let n = self.pages();
            self.page = if inside(l.arrow_up, x, y) {
                (self.page + n - 1) % n
            } else {
                (self.page + 1) % n
            };
            self.open = None;
            return self.reply(SND_PAGE);
        }
        if inside(l.volume, x, y) {
            self.dragging = true;
            let mut r = self.reply(SND_VOLUME).with_help(HELP_VOLUME);
            r.fx.extend(self.set_volume(y, assets));
            return r;
        }
        if inside(l.dog, x, y) {
            self.open = None;
            return self.reply(SND_DOG).with_help(HELP_DOG);
        }
        if inside(l.recorder, x, y) {
            self.open = None;
            return self.reply(SND_RECORDER).with_help(HELP_RECORDER);
        }
        if inside(l.canvas, x, y) {
            self.open = None;
            return Reply {
                handled: true,
                canvas: true,
                ..Reply::default()
            };
        }
        Reply::default()
    }

    /// Linha `k` da lista (0 = "Sem Som"): o som vai para a mão e toca.
    fn pick(&mut self, k: usize) -> Reply {
        let Some(cat) = self.open.and_then(|c| self.cats.get(c)) else {
            return Reply::done();
        };
        let mut r = Reply::done().with_help(HELP_THEN_CLICK);
        if k == 0 {
            self.hand = Some(Hand::Silence);
        } else if let Some((name, sound)) = cat.sounds.get(k - 1).cloned() {
            r.fx.push(Effect::PlaySound(sound.clone()));
            self.hand = Some(Hand::Sound { name, sound });
        }
        self.open = None;
        r
    }

    /// Resposta com um som do programa da faixa.
    fn reply(&self, id: u16) -> Reply {
        Reply {
            handled: true,
            fx: vec![self.layout.snd(id)],
            ..Reply::default()
        }
    }

    /// Volume pela altura `y` do ponteiro.
    fn set_volume(&self, y: i32, assets: &Assets) -> Option<Effect> {
        let v = self.layout.volume_at(y);
        (v != assets.volume()).then(|| {
            assets.set_volume(v);
            Effect::SetVolume(v)
        })
    }

    /// Arrasto do cursor do volume.
    pub fn drag(&mut self, y: i32, assets: &Assets) -> Option<Effect> {
        if !self.dragging {
            return None;
        }
        self.set_volume(y, assets)
    }

    /// Fim do arrasto: um som de amostra na altura nova.
    pub fn release(&mut self) -> Option<Effect> {
        std::mem::take(&mut self.dragging).then(|| self.layout.snd(SND_VOLUME_TEST))
    }

    /// Faixa de opções: faixas de teatro, volume, Cachorrinho, Gravador e setas.
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        let l = &self.layout;
        let font = a.font(8);
        // a placa usa a letra pequena (Pablo 6 pt), como no original
        let small = a.font(6).or_else(|| font.clone());
        let first = self.page * PER_PAGE;
        for (slot, c) in self.cats.iter().skip(first).take(PER_PAGE).enumerate() {
            let (x, y) = (l.banner_x(slot), l.banner_y);
            match &self.banner {
                Some(img) => f.blit(img, x, y),
                None => {
                    f.rect(x, y, BANNER_W, BANNER_H, BLACK);
                    f.rect(x + 1, y + 1, BANNER_W - 2, BANNER_H - 2, LILAC);
                    f.rect(x + 17, y + 21, 78, 13, YELLOW);
                }
            }
            // nome na placa amarela, como no carrossel ("AnimalSelvagem")
            let cx = x + BANNER_W / 2;
            match &small {
                Some(fo) => {
                    let w = fo.width(&c.name) as i32;
                    f.text_font(fo, cx - w / 2, y + PLATE_TEXT_Y, &c.name, BLACK);
                }
                None => f.text(
                    cx - 4 * c.name.chars().count() as i32,
                    y + 24,
                    &c.name,
                    BLACK,
                ),
            }
        }
        if self.cats.is_empty() {
            f.text(l.banner_x0, l.banner_y + 14, "sem MAG100.SND", BLACK);
        }
        let masked = |f: &mut Frame, img: u16, mask: u16, x: i32, y: i32| {
            f.blit_masked(&a.image(&l.pic(img)), &a.image(&l.pic(mask)), x, y);
        };
        if let Some((img, mask, x, y)) = l.arrows_back {
            masked(f, img, mask, x, y);
        }
        masked(f, l.track.0, l.track.1, l.track.2, l.track.3);
        masked(f, l.knob.0, l.knob.1, l.knob.2, l.knob_y(a.volume()));
        masked(f, l.dog_pic.0, l.dog_pic.1, l.dog_pic.2, l.dog_pic.3);
        let (rec, rec_mask) = l.recorder_pic;
        masked(f, rec, rec_mask, l.recorder.0, l.recorder.1);
        masked(f, 0x03b3, 0x03b5, l.arrow_up.0, l.arrow_up.1);
        masked(f, 0x03b7, 0x03b9, l.arrow_down.0, l.arrow_down.1);
        let b = l.page_box;
        f.rect(b.0, b.1, b.2, b.3, BLACK);
        f.rect(b.0 + 1, b.1 + 1, b.2 - 2, b.3 - 2, WHITE);
        let n = (self.page + 1).to_string();
        match &font {
            Some(fo) => {
                let w = fo.width(&n) as i32;
                f.text_font(fo, b.0 + b.2 / 2 - w / 2, b.1 + 1, &n, BLACK);
            }
            None => f.text(b.0 + 4, b.1 + 2, &n, BLACK),
        }
    }

    /// Lista de nomes aberta e diálogos.
    pub fn draw_overlay(&self, f: &mut Frame, a: &Assets) {
        self.draw_list(f, a);
        if let Some(d) = &self.finder {
            d.draw(f, a);
        }
        if let Some(r) = &self.recorder {
            r.draw(f, a);
        }
    }

    /// Só a lista de nomes aberta (os diálogos são desenhados à parte).
    pub fn draw_list(&self, f: &mut Frame, a: &Assets) {
        let Some(cat) = self.open.and_then(|c| self.cats.get(c)) else {
            return;
        };
        let l = &self.layout;
        let font = a.font(8);
        let names: Vec<&str> = std::iter::once("Sem Som")
            .chain(cat.sounds.iter().map(|(n, _)| n.as_str()))
            .skip(self.scroll)
            .take(self.visible_rows())
            .collect();
        let box_at = |f: &mut Frame, k: usize| {
            let r = self.row_rect(k);
            f.rect(r.0, r.1, r.2, r.3, BLACK);
            f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, ORANGE);
            f.rect(r.0 + 3, r.1 + 3, r.2 - 6, r.3 - 6, BLACK);
            f.rect(r.0 + 4, r.1 + 4, r.2 - 8, r.3 - 8, WHITE);
            r
        };
        let text_y = (l.row.1 - 14) / 2;
        for (k, name) in names.iter().enumerate() {
            let r = box_at(f, k);
            match &font {
                Some(fo) => {
                    let shown = crate::ui::clip_text(name, |s| fo.width(s) as i32, r.2 - 12);
                    let w = fo.width(&shown) as i32;
                    f.text_font(fo, r.0 + r.2 / 2 - w / 2, r.1 + text_y, &shown, BLACK);
                }
                None => f.text(r.0 + 6, r.1 + 8, name, BLACK),
            }
        }
        if self.list_scrolls() {
            let r = self.row_rect(l.list_rows - 1);
            let (up, down) = (a.image(&l.pic(0x03b3)), a.image(&l.pic(0x03b7)));
            let (um, dm) = (a.image(&l.pic(0x03b5)), a.image(&l.pic(0x03b9)));
            if l.plain_arrows {
                // Writer: caixa branca de 16 px colada na última linha
                let r = (r.0, r.1, r.2, 16);
                f.rect(r.0, r.1, r.2, r.3, BLACK);
                f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, WHITE);
                f.blit_masked(&up, &um, r.0 + 24, r.1 + 2);
                f.blit_masked(&down, &dm, r.0 + 58, r.1 + 2);
            } else {
                let r = box_at(f, l.list_rows - 1);
                f.blit_masked(&up, &um, r.0 + r.2 / 4 - 8, r.1 + 5);
                f.blit_masked(&down, &dm, r.0 + 3 * r.2 / 4 - 8, r.1 + 5);
            }
        }
    }

    /// Lê os pontos soltos de versões antigas (tolerante: linhas inválidas são
    /// puladas); `take_legacy` os entrega para serem presos aos objetos.
    pub fn load_spots(&mut self, path: &Path) {
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        self.legacy = parse_spots(&text);
    }

    pub fn take_legacy(&mut self) -> Vec<Spot> {
        std::mem::take(&mut self.legacy)
    }

    /// Os sons agora ficam nos objetos: o arquivo antigo de pontos some
    /// (depois de os pontos terem sido presos aos objetos ao abrir).
    pub fn save_spots(&self, path: &Path) -> std::io::Result<()> {
        if self.legacy.is_empty() {
            // limpeza: sem arquivo (ou sem pasta), nada a fazer
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }
}

/// Pontos do formato antigo `[{"x": .., "y": .., "name": "..", "file": "..", "index": ..}]`.
pub fn parse_spots(text: &str) -> Vec<Spot> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with('{') {
            continue;
        }
        let field = |key: &str| -> Option<String> {
            let k = format!("\"{key}\": ");
            let i = line.find(&k)? + k.len();
            let rest = &line[i..];
            let v = if let Some(r) = rest.strip_prefix('"') {
                r[..r.find('"')?].to_string()
            } else {
                rest[..rest.find([',', '}'])?].trim().to_string()
            };
            Some(v)
        };
        let (Some(x), Some(y), Some(name), Some(file), Some(index)) = (
            field("x"),
            field("y"),
            field("name"),
            field("file"),
            field("index"),
        ) else {
            continue;
        };
        let (Ok(x), Ok(y), Ok(index)) = (x.parse(), y.parse(), index.parse::<usize>()) else {
            continue;
        };
        if file.is_empty() {
            continue;
        }
        out.push(Spot {
            x,
            y,
            name,
            sound: SoundRef::Library { file, index },
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(index: usize) -> SoundRef {
        SoundRef::Library {
            file: "MAG100.SND".into(),
            index,
        }
    }

    fn cat(name: &str, n: usize) -> Category {
        Category {
            name: name.into(),
            sounds: (0..n).map(|i| (format!("{name}{i}"), lib(i))).collect(),
        }
    }

    fn fake(n: usize) -> Sounds {
        let names = [
            "Grave",
            "Agudo",
            "Dinossauro",
            "AnimalSelvagem",
            "AnimalDeFazenda",
            "Ação",
            "Maluco",
            "Absurdo",
        ];
        Sounds::with_categories(
            names
                .iter()
                .take(n)
                .enumerate()
                .map(|(i, c)| cat(c, if i == 5 { 23 } else { 13 }))
                .collect(),
            None,
        )
    }

    fn assets() -> (tempfile::TempDir, Assets) {
        crate::scenes::testutil::empty_assets()
    }

    fn row_center(k: usize) -> (i32, i32) {
        let r = fake(0).row_rect(k);
        (r.0 + r.2 / 2, r.1 + r.3 / 2)
    }

    #[test]
    fn eight_categories_in_three_pages_with_wrapping_arrows() {
        let (_d, a) = assets();
        let mut s = fake(8);
        assert_eq!(s.pages(), 3);
        // ▲ na primeira página vai para a última (como no original: 1 → 3)
        let r = s.click(ARROW_UP.0 + 4, ARROW_UP.1 + 4, &a);
        assert_eq!(r.fx, vec![snd(SND_PAGE)]);
        assert_eq!(s.page(), 2);
        s.click(ARROW_DOWN.0 + 4, ARROW_DOWN.1 + 4, &a);
        assert_eq!(s.page(), 0);
        s.click(ARROW_DOWN.0 + 4, ARROW_DOWN.1 + 4, &a);
        assert_eq!(s.page(), 1);
        // terceira faixa da página 2 = sexta categoria (Ação)
        let b = fake(0).banner_rect(2);
        let r = s.click(b.0 + 50, b.1 + 20, &a);
        assert_eq!(r.fx, vec![snd(SND_CATEGORY)]);
        assert_eq!(r.help, Some(HELP_PICK));
        assert_eq!(s.open, Some(5));
        // página 3 só tem duas faixas: a terceira vaga não responde
        s.click(ARROW_UP.0 + 4, ARROW_UP.1 + 4, &a);
        s.click(ARROW_UP.0 + 4, ARROW_UP.1 + 4, &a);
        assert_eq!(s.page(), 2);
        assert!(!s.click(b.0 + 50, b.1 + 20, &a).handled);
    }

    #[test]
    fn pick_puts_the_sound_in_hand_and_silence_too() {
        let (_d, a) = assets();
        let mut s = fake(3);
        let b = fake(0).banner_rect(1);
        s.click(b.0 + 50, b.1 + 20, &a);
        let (x, y) = row_center(2);
        let r = s.click(x, y, &a);
        assert_eq!(r.fx, vec![Effect::PlaySound(lib(1))]);
        assert_eq!(r.help, Some(HELP_THEN_CLICK));
        assert_eq!(
            s.hand(),
            Some(&Hand::Sound {
                name: "Agudo1".into(),
                sound: lib(1)
            })
        );
        assert!(!s.list_open());
        s.click(b.0 + 50, b.1 + 20, &a);
        let (x, y) = row_center(0);
        s.click(x, y, &a);
        assert_eq!(s.hand(), Some(&Hand::Silence));
        // clique no quadro: decide quem está debaixo (não cria ponto solto)
        let r = s.click(ORIGIN.0 + 100, ORIGIN.1 + 100, &a);
        assert!(r.canvas && r.handled);
        assert_eq!(s.take_hand(), Some(Hand::Silence));
        // fora da faixa e do quadro: não é desta ferramenta
        assert!(!s.click(300, 30, &a).handled);
    }

    #[test]
    fn long_list_scrolls_with_its_last_row() {
        let (_d, a) = assets();
        let mut s = fake(8);
        s.page = 1;
        let b = fake(0).banner_rect(2);
        s.click(b.0 + 50, b.1 + 20, &a);
        assert!(s.list_scrolls());
        let r = fake(0).row_rect(ARTIST.list_rows - 1);
        s.click(r.0 + r.2 - 10, r.1 + 10, &a);
        // 24 linhas, 13 visíveis: a rolagem para na última página
        assert_eq!(s.scroll, 11);
        // a primeira linha visível agora é o 11º som
        let (x, y) = row_center(0);
        s.click(x, y, &a);
        assert_eq!(s.hand().and_then(|h| h.sound()), Some(&lib(10)));
        s.click(b.0 + 50, b.1 + 20, &a);
        s.scroll = 11;
        s.click(r.0 + 10, r.1 + 10, &a);
        assert_eq!(s.scroll, 0);
    }

    #[test]
    fn volume_follows_the_knob() {
        let (_d, a) = assets();
        let mut s = fake(1);
        assert_eq!(ARTIST.knob_y(100), ARTIST.knob_top);
        assert_eq!(ARTIST.knob_y(0), ARTIST.knob_bottom);
        let r = s.click(ARTIST.volume.0 + 5, ARTIST.knob_top + 4, &a);
        assert_eq!(r.help, Some(HELP_VOLUME));
        assert!(r.fx.contains(&Effect::SetVolume(100)));
        assert_eq!(a.volume(), 100);
        assert_eq!(
            s.drag(ARTIST.knob_bottom + 30, &a),
            Some(Effect::SetVolume(0))
        );
        assert_eq!(s.drag(ARTIST.knob_bottom + 30, &a), None);
        assert_eq!(s.release(), Some(snd(SND_VOLUME_TEST)));
        assert_eq!(s.drag(ARTIST.knob_top, &a), None);
    }

    #[test]
    fn dog_and_recorder_ask_for_their_dialogs() {
        let (_d, a) = assets();
        let mut s = fake(1);
        let r = s.click(DOG.0 + 10, DOG.1 + 10, &a);
        assert_eq!((r.fx, r.help), (vec![snd(SND_DOG)], Some(HELP_DOG)));
        let r = s.click(RECORDER.0 + 10, RECORDER.1 + 10, &a);
        assert_eq!(
            (r.fx, r.help),
            (vec![snd(SND_RECORDER)], Some(HELP_RECORDER))
        );
    }

    #[test]
    fn legacy_spots_are_read_and_the_old_file_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(SPOTS_NAME);
        std::fs::write(
            &p,
            "[\n  {\"x\": 5, \"y\": 6, \"name\": \"Sol3\", \"file\": \"MAG100.SND\", \"index\": 3},\n lixo\n]\n",
        )
        .unwrap();
        let mut s = fake(1);
        s.load_spots(&p);
        assert_eq!(
            s.take_legacy(),
            vec![Spot {
                x: 5,
                y: 6,
                name: "Sol3".into(),
                sound: SoundRef::Library {
                    file: "MAG100.SND".into(),
                    index: 3
                }
            }]
        );
        s.save_spots(&p).unwrap();
        assert!(!p.exists());
        // sem arquivo, gravar não falha
        s.save_spots(&p).unwrap();
    }

    #[test]
    fn banner_regions_get_their_colors() {
        // contorno 112×37 todo branco dentro da máscara: tudo vira a cor da
        // primeira semente (uma região só)
        let mk = |v: u8| Image {
            width: BANNER_W as u32,
            height: BANNER_H as u32,
            rgba: [v, v, v, 255].repeat((BANNER_W * BANNER_H) as usize),
        };
        let img = banner_image(&mk(255), &mk(0)).unwrap();
        assert_eq!(&img.rgba[..3], &LILAC);
        // fora da máscara: transparente
        let img = banner_image(&mk(255), &mk(255)).unwrap();
        assert_eq!(img.rgba[3], 0);
        let tiny = Image {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        };
        assert!(banner_image(&mk(255), &tiny).is_none());
    }
}
