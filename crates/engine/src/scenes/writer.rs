//! Escritório do Creative Writer: a página branca com o editor de texto, a
//! barra de ferramentas e a faixa de opções do original (`WRITER.PIC`),
//! as listas suspensas de tamanho, cor, estilo e alinhamento, os adesivos
//! de clip-art dentro do texto, o ovo (desfazer/refazer) e a explosão da
//! tesoura. Posições medidas no original (spec 3a).

use super::stickers::{blit_clipped, blit_part};
use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::clipart::{ClipRoll, load_rolls};
use crate::paint::COLORS;
use crate::scene::{Effect, Input, Key, KidCommon, MouseButton, Scene};
use crate::writer::store::{LAST_NAME, load, save};
use crate::writer::{Align, Document, FontSet, History, Item, Layout, Size, Sticker, Style};
use formats::palette::Rgb;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// Bibliotecas de clip-art do Creative Writer (o Fine Artist tem também `MAG100.ART`).
pub const LIBRARIES: [&str; 2] = ["MAXMAG10.ART", "MAX100.ART"];

type Rect = (i32, i32, i32, i32);

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const DARK: [u8; 3] = [100, 100, 100];
const LIGHT: [u8; 3] = [140, 140, 140];
const ORANGE: [u8; 3] = [255, 140, 50];
const BROWN: [u8; 3] = [185, 101, 36];
const RED: [u8; 3] = [250, 0, 57];
const YELLOW: [u8; 3] = [255, 238, 76];

/// Página branca.
pub const PAGE: Rect = (23, 108, 593, 349);
/// Margens da página, dos quatro lados.
pub const MARGIN: i32 = 24;
/// Largura e altura da área de texto de uma página.
pub const TEXT_W: i32 = PAGE.2 - 2 * MARGIN;
pub const TEXT_H: i32 = PAGE.3 - 2 * MARGIN;
const EXIT: Rect = (580, 18, 56, 88);
const ARROW_UP: Rect = (504, 58, 16, 14);
const ARROW_DOWN: Rect = (504, 90, 16, 14);
const PAGE_PREV: Rect = (584, 462, 12, 12);
const PAGE_NEXT: Rect = (598, 462, 12, 12);
/// Período do piscar do cursor (meio período aceso).
const CARET_PERIOD: f32 = 1.0;
/// Botão da explosão (apagar tudo) na faixa da tesoura.
const EXPLOSION: Rect = (25, 58, 56, 48);

// tira de rolos e filme, como na pintura
const ROLLS_PER_PAGE: usize = 7;
const SLOT_X0: i32 = 23;
const SLOT_W: i32 = 52;
const STRIP_Y: i32 = 58;
const FILM_Y: i32 = 99;
const FILM_H: i32 = 368;
const FILM_W: i32 = 42;
const FILM_CELLS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Text,
    Stickers,
    Scissors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Select(Tool),
    Undo,
    Unavailable(&'static str),
}

struct ToolDef {
    x: i32,
    y: i32,
    w: i32,
    normal: (u16, u16),
    active: (u16, u16),
    action: Action,
}

const TOOLS: [ToolDef; 10] = [
    ToolDef {
        x: 17,
        y: 12,
        w: 39,
        normal: (0x0251, 0x0253),
        active: (0x0252, 0x0253),
        action: Action::Select(Tool::Text),
    },
    ToolDef {
        x: 65,
        y: 12,
        w: 32,
        normal: (0x01ff, 0x0201),
        active: (0x0200, 0x0201),
        action: Action::Select(Tool::Stickers),
    },
    ToolDef {
        x: 110,
        y: 10,
        w: 34,
        normal: (0x0205, 0x0207),
        active: (0x0206, 0x0207),
        action: Action::Unavailable("sons"),
    },
    ToolDef {
        x: 159,
        y: 16,
        w: 46,
        normal: (0x0800, 0x0801),
        active: (0x0800, 0x0801),
        action: Action::Unavailable("página"),
    },
    ToolDef {
        x: 214,
        y: 11,
        w: 43,
        normal: (0x091f, 0x0921),
        active: (0x0920, 0x0921),
        action: Action::Unavailable("inseto"),
    },
    ToolDef {
        x: 266,
        y: 6,
        w: 44,
        normal: (0x0212, 0x0213),
        active: (0x0212, 0x0213),
        action: Action::Undo,
    },
    ToolDef {
        x: 319,
        y: 9,
        w: 44,
        normal: (0x08e8, 0x08e9),
        active: (0x08e8, 0x08e9),
        action: Action::Select(Tool::Scissors),
    },
    ToolDef {
        x: 372,
        y: 9,
        w: 36,
        normal: (0x0219, 0x0223),
        active: (0x0222, 0x0223),
        action: Action::Unavailable("mão"),
    },
    ToolDef {
        x: 417,
        y: 12,
        w: 46,
        normal: (0x0804, 0x0805),
        active: (0x0804, 0x0805),
        action: Action::Unavailable("livro"),
    },
    ToolDef {
        x: 472,
        y: 11,
        w: 42,
        normal: (0x0226, 0x0227),
        active: (0x0226, 0x0227),
        action: Action::Unavailable("imprimir"),
    },
];

/// Listas suspensas da faixa do Texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Fonts,
    Sizes,
    Colors,
    Styles,
    Align,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StripAction {
    Open(List),
    Unavailable(&'static str),
}

struct StripButton {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    img: u16,
    mask: Option<u16>,
    action: StripAction,
}

const STRIP: [StripButton; 9] = [
    StripButton {
        x: 24,
        y: 61,
        w: 49,
        h: 34,
        img: 0x0275,
        mask: Some(0x0277),
        action: StripAction::Open(List::Fonts),
    },
    StripButton {
        x: 89,
        y: 61,
        w: 49,
        h: 34,
        img: 0x0811,
        mask: None,
        action: StripAction::Open(List::Sizes),
    },
    StripButton {
        x: 154,
        y: 61,
        w: 49,
        h: 34,
        img: 0x027b,
        mask: Some(0x027d),
        action: StripAction::Open(List::Colors),
    },
    StripButton {
        x: 219,
        y: 61,
        w: 49,
        h: 34,
        img: 0x0279,
        mask: None,
        action: StripAction::Open(List::Styles),
    },
    StripButton {
        x: 284,
        y: 61,
        w: 49,
        h: 34,
        img: 0x0bc3,
        mask: None,
        action: StripAction::Open(List::Align),
    },
    StripButton {
        x: 349,
        y: 58,
        w: 33,
        h: 39,
        img: 0x080c,
        mask: Some(0x080d),
        action: StripAction::Unavailable("carimbo de linha"),
    },
    StripButton {
        x: 398,
        y: 61,
        w: 29,
        h: 34,
        img: 0x0831,
        mask: Some(0x0833),
        action: StripAction::Unavailable("inserir"),
    },
    StripButton {
        x: 443,
        y: 57,
        w: 46,
        h: 41,
        img: 0x0269,
        mask: Some(0x026b),
        action: StripAction::Unavailable("forma"),
    },
    StripButton {
        x: 508,
        y: 59,
        w: 20,
        h: 30,
        img: 0x0229,
        mask: Some(0x022b),
        action: StripAction::Unavailable("interruptor"),
    },
];

/// Imagens dos itens da lista de alinhamento (Esquerda, Direita, Centro, Esquerda e direita).
const ALIGN_ITEMS: [u16; 4] = [0x0bc9, 0x0bcb, 0x0bcd, 0x0bcf];
const ALIGN_ITEM_H: i32 = 36;
const ALIGN_ITEM_W: i32 = 97;
const ITEM_H: i32 = 14;
const COLOR_ITEM_H: i32 = 8;
const LIST_Y: i32 = 105;
const STYLE_NAMES: [&str; 4] = ["Simples", "Negrito", "Inclinado", "Sublinhado"];

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Writer, id)
}

fn slot_rect(k: usize) -> Rect {
    (SLOT_X0 + SLOT_W * k as i32, STRIP_Y, SLOT_W, 42)
}

/// Geometria da tira de filme sob o rolo da casa `slot`.
struct Film {
    x: i32,
}

impl Film {
    fn rect(&self) -> Rect {
        (self.x, FILM_Y, FILM_W, FILM_H)
    }

    fn cell(&self, j: usize) -> Rect {
        (self.x + 4, 103 + 41 * j as i32, 34, 34)
    }

    fn up(&self) -> Rect {
        (self.x + 6, 452, 12, 12)
    }

    fn down(&self) -> Rect {
        (self.x + 24, 452, 12, 12)
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub struct Writer {
    common: KidCommon,
    texts: Option<PathBuf>,
    doc: Document,
    history: History,
    fonts: FontSet,
    pal: Vec<Rgb>,
    tool: Tool,
    /// Página mostrada (0 = primeira).
    page: usize,
    list: Option<List>,
    rolls: Vec<ClipRoll>,
    roll_page: usize,
    roll: usize,
    film_open: bool,
    scroll: usize,
    /// Desenhos dos adesivos já decodificados, por (rolo, número).
    art: HashMap<(u16, usize), Option<Rc<crate::Image>>>,
    blink: f32,
    changed: bool,
    mouse: (i32, i32),
}

impl Writer {
    pub fn new(texts: Option<PathBuf>, assets: &Assets) -> Writer {
        let pal = assets.palettes(Program::Writer).pal256.clone();
        let rolls = load_rolls(assets)
            .into_iter()
            .filter(|r| LIBRARIES.contains(&r.library))
            .collect();
        let doc = texts
            .as_ref()
            .and_then(|d| load(&d.join(LAST_NAME)))
            .unwrap_or_default();
        let mut w = Writer {
            common: KidCommon::new(None),
            texts,
            doc,
            history: History::default(),
            fonts: FontSet {
                tiny: assets.font(6),
                small: assets.font(8),
                medium: assets.font(16),
            },
            pal,
            tool: Tool::Text,
            page: 0,
            list: None,
            rolls,
            roll_page: 0,
            roll: 0,
            film_open: false,
            scroll: 0,
            art: HashMap::new(),
            blink: 0.0,
            changed: false,
            mouse: (0, 0),
        };
        // os adesivos do arquivo só guardam rolo e número: medidas vêm dos desenhos
        let keys: Vec<(u16, usize)> = w
            .doc
            .paras
            .iter()
            .flat_map(|p| p.items.iter())
            .filter_map(|i| match i {
                Item::Sticker(s) => Some((s.roll, s.index)),
                Item::Char(..) => None,
            })
            .collect();
        let sizes: Vec<_> = keys
            .into_iter()
            .map(|(roll, n)| {
                let size = w.sticker_art(roll, n).map(|i| (i.width, i.height));
                ((roll, n), size)
            })
            .collect();
        w.doc
            .resize_stickers(|roll, n| sizes.iter().find(|(k, _)| *k == (roll, n))?.1);
        w.follow_cursor();
        w
    }

    pub fn doc(&self) -> &Document {
        &self.doc
    }

    pub fn tool(&self) -> Tool {
        self.tool
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn list(&self) -> Option<List> {
        self.list
    }

    pub fn film_open(&self) -> bool {
        self.film_open
    }

    pub fn roll_count(&self) -> usize {
        self.rolls.len()
    }

    fn layout(&self) -> Layout {
        Layout::build(&self.doc, &self.fonts, TEXT_W, TEXT_H)
    }

    /// Desenho do adesivo (em cache); `None` se o rolo ou o item não existem.
    fn sticker_art(&mut self, roll: u16, index: usize) -> Option<Rc<crate::Image>> {
        if let Some(a) = self.art.get(&(roll, index)) {
            return a.clone();
        }
        let img = self
            .rolls
            .iter()
            .find(|r| r.id == roll)
            .and_then(|r| r.items.get(index))
            .and_then(|i| i.art())
            .map(Rc::new);
        self.art.insert((roll, index), img.clone());
        img
    }

    /// Insere no cursor o adesivo `index` do rolo `roll` (sem desenho, um quadrado reserva).
    pub fn insert_sticker(&mut self, roll: u16, index: usize) {
        let s = match self.sticker_art(roll, index) {
            Some(img) => Sticker {
                roll,
                index,
                w: img.width,
                h: img.height,
            },
            None => Sticker::placeholder(roll, index),
        };
        self.history.record(&self.doc);
        self.doc.insert_sticker(s);
        self.after_edit();
    }

    fn after_edit(&mut self) {
        self.changed = true;
        self.blink = 0.0;
        self.follow_cursor();
    }

    /// Mostra a página onde está o cursor.
    fn follow_cursor(&mut self) {
        let l = self.layout();
        let (k, _) = l.cursor_pos(self.doc.cursor);
        self.page = l.lines[k].page;
    }

    fn set_page(&mut self, delta: i32) {
        let pages = self.layout().pages as i32;
        self.page = (self.page as i32 + delta).clamp(0, pages - 1) as usize;
    }

    fn type_text(&mut self, text: &str) {
        let clean: String = text
            .chars()
            .filter(|c| !c.is_control() && formats::cp1252::encode_char(*c).is_some())
            .collect();
        if clean.is_empty() {
            return;
        }
        self.history.record_typing(&self.doc);
        self.doc.insert_text(&clean);
        self.after_edit();
    }

    fn key(&mut self, key: Key) {
        self.blink = 0.0;
        match key {
            Key::Enter => {
                self.history.record(&self.doc);
                self.doc.newline();
                self.after_edit();
            }
            Key::Backspace => {
                self.history.record_typing(&self.doc);
                if self.doc.backspace() {
                    self.after_edit();
                }
            }
            Key::Left => {
                self.history.close();
                self.doc.move_left();
                self.follow_cursor();
            }
            Key::Right => {
                self.history.close();
                self.doc.move_right();
                self.follow_cursor();
            }
            Key::Up | Key::Down => {
                self.history.close();
                let delta = if key == Key::Up { -1 } else { 1 };
                if let Some(c) = self.layout().cursor_vertical(self.doc.cursor, delta) {
                    self.doc.cursor = c;
                }
                self.follow_cursor();
            }
            Key::Home => {
                self.history.close();
                self.doc.home();
            }
            Key::End => {
                self.history.close();
                self.doc.end();
            }
            Key::PageUp => self.set_page(-1),
            Key::PageDown => self.set_page(1),
            _ => {}
        }
    }

    fn set_style(&mut self, style: Style) {
        self.history.close();
        self.doc.style = style;
    }

    fn list_rect(&self, list: List) -> Rect {
        let x = STRIP
            .iter()
            .find(|b| b.action == StripAction::Open(list))
            .map_or(24, |b| b.x);
        let (w, h) = match list {
            List::Fonts => (170, 2 + 3 * ITEM_H),
            List::Sizes => (100, 2 + 6 * ITEM_H),
            List::Colors => (60, 2 + COLORS.len() as i32 * COLOR_ITEM_H),
            List::Styles => (100, 2 + 4 * ITEM_H),
            List::Align => (ALIGN_ITEM_W + 2, 2 + 4 * ALIGN_ITEM_H),
        };
        (x, LIST_Y, w, h)
    }

    fn list_item_count(list: List) -> usize {
        match list {
            List::Fonts => 1,
            List::Sizes => Size::ALL.len(),
            List::Colors => COLORS.len(),
            List::Styles => STYLE_NAMES.len(),
            List::Align => Align::ALL.len(),
        }
    }

    fn list_item_rect(&self, list: List, k: usize) -> Rect {
        let (x, y, w, _) = self.list_rect(list);
        let h = match list {
            List::Colors => COLOR_ITEM_H,
            List::Align => ALIGN_ITEM_H,
            _ => ITEM_H,
        };
        (x + 1, y + 1 + h * k as i32, w - 2, h)
    }

    /// Clique com uma lista aberta: escolhe o item (se for o caso) e fecha.
    fn list_click(&mut self, list: List, x: i32, y: i32) {
        self.list = None;
        let Some(k) =
            (0..Self::list_item_count(list)).find(|&k| inside(self.list_item_rect(list, k), x, y))
        else {
            return;
        };
        let mut style = self.doc.style;
        match list {
            List::Fonts => {}
            List::Sizes => {
                style.size = Size::ALL[k];
                self.set_style(style);
            }
            List::Colors => {
                style.color = COLORS[k];
                self.set_style(style);
            }
            List::Styles => {
                match k {
                    0 => {
                        style.bold = false;
                        style.italic = false;
                        style.underline = false;
                    }
                    1 => style.bold = !style.bold,
                    2 => style.italic = !style.italic,
                    _ => style.underline = !style.underline,
                }
                self.set_style(style);
            }
            List::Align => {
                self.history.record(&self.doc);
                self.doc.set_align(Align::ALL[k]);
                self.after_edit();
            }
        }
    }

    fn film(&self) -> Film {
        Film {
            x: 33 + SLOT_W * (self.roll % ROLLS_PER_PAGE) as i32,
        }
    }

    fn roll_pages(&self) -> usize {
        self.rolls.len().div_ceil(ROLLS_PER_PAGE).max(1)
    }

    /// Clique na faixa de adesivos ou na tira; verdadeiro se foi tratado.
    fn stickers_click(&mut self, x: i32, y: i32) -> bool {
        if self.film_open {
            let film = self.film();
            if inside(film.rect(), x, y) {
                let n = self.rolls.get(self.roll).map_or(0, |r| r.items.len());
                let max_scroll = n.saturating_sub(FILM_CELLS);
                if inside(film.up(), x, y) {
                    self.scroll = self.scroll.saturating_sub(1);
                } else if inside(film.down(), x, y) {
                    self.scroll = (self.scroll + 1).min(max_scroll);
                } else if let Some(j) = (0..FILM_CELLS).find(|&j| inside(film.cell(j), x, y)) {
                    let item = self.scroll + j;
                    if item < n {
                        let roll = self.rolls[self.roll].id;
                        self.insert_sticker(roll, item);
                        self.film_open = false;
                    }
                }
                return true;
            }
        }
        if inside(ARROW_UP, x, y) {
            let pages = self.roll_pages();
            self.roll_page = (self.roll_page + pages - 1) % pages;
            self.film_open = false;
            return true;
        }
        if inside(ARROW_DOWN, x, y) {
            self.roll_page = (self.roll_page + 1) % self.roll_pages();
            self.film_open = false;
            return true;
        }
        if let Some(k) = (0..ROLLS_PER_PAGE).find(|&k| inside(slot_rect(k), x, y)) {
            let roll = self.roll_page * ROLLS_PER_PAGE + k;
            if roll < self.rolls.len() {
                if self.film_open && roll == self.roll {
                    self.film_open = false;
                } else {
                    self.roll = roll;
                    self.scroll = 0;
                    self.film_open = true;
                }
            }
            return true;
        }
        false
    }

    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>) {
        if let Some(list) = self.list {
            self.list_click(list, x, y);
            return;
        }
        if self.tool == Tool::Stickers && self.stickers_click(x, y) {
            return;
        }
        if inside(PAGE, x, y) {
            let l = self.layout();
            let (tx, ty) = (x - PAGE.0 - MARGIN, y - PAGE.1 - MARGIN);
            if let Some(k) = l.line_at(self.page, ty) {
                self.history.close();
                self.doc.cursor = l.cursor_in_line(k, tx);
                self.blink = 0.0;
            }
            return;
        }
        if inside(EXIT, x, y) {
            fx.push(Effect::Quit);
            return;
        }
        if let Some(t) = TOOLS
            .iter()
            .find(|t| inside((t.x - 5, 5, t.w + 11, 50), x, y))
        {
            match t.action {
                Action::Select(tool) => {
                    self.tool = tool;
                    self.film_open = false;
                }
                Action::Undo => {
                    if self.history.toggle(&mut self.doc) {
                        self.doc.clamp_cursor();
                        self.after_edit();
                    }
                }
                Action::Unavailable(name) => fx.push(Effect::Log(format!(
                    "{name}: ferramenta ainda não disponível"
                ))),
            }
            return;
        }
        if inside(PAGE_PREV, x, y) {
            self.set_page(-1);
            return;
        }
        if inside(PAGE_NEXT, x, y) {
            self.set_page(1);
            return;
        }
        match self.tool {
            Tool::Text => {
                if let Some(b) = STRIP.iter().find(|b| inside((b.x, b.y, b.w, b.h), x, y)) {
                    match b.action {
                        StripAction::Open(list) => self.list = Some(list),
                        StripAction::Unavailable(name) => {
                            fx.push(Effect::Log(format!("{name}: ainda não disponível")))
                        }
                    }
                }
            }
            Tool::Scissors => {
                if inside(EXPLOSION, x, y) && !self.doc.is_empty() {
                    self.history.record(&self.doc);
                    self.doc.clear();
                    self.after_edit();
                }
            }
            Tool::Stickers => {}
        }
    }

    fn rgb(&self, index: u8) -> [u8; 3] {
        let p = self
            .pal
            .get(index as usize)
            .copied()
            .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
        [p.r, p.g, p.b]
    }

    fn draw_chrome(&self, f: &mut Frame, a: &Assets) {
        f.fill(DARK);
        // bordas laranja em relevo, como na pintura
        for y0 in [0, 104, 456, 475] {
            for (dy, c) in [(0, BLACK), (1, ORANGE), (2, ORANGE), (3, BROWN), (4, BLACK)] {
                f.rect(0, y0 + dy, 640, 1, c);
            }
        }
        for (x, c) in [(0, BLACK), (1, BROWN), (2, ORANGE), (3, ORANGE), (4, BLACK)] {
            f.rect(x, 0, 1, 480, c);
        }
        for (x, c) in [
            (635, BLACK),
            (636, BROWN),
            (637, ORANGE),
            (638, ORANGE),
            (639, BLACK),
        ] {
            f.rect(x, 0, 1, 480, c);
        }
        // faixa de opções e linha sob a barra de ferramentas
        f.rect(5, 56, 630, 48, LIGHT);
        f.rect(5, 55, 630, 1, BLACK);
        // nicho escuro do McZee
        f.rect(532, 55, 1, 49, BLACK);
        f.rect(533, 56, 47, 48, DARK);
        // página branca com borda preta de 1 px por fora
        f.rect(PAGE.0 - 1, PAGE.1, 1, PAGE.3, BLACK);
        f.rect(PAGE.0 + PAGE.2, PAGE.1, 1, PAGE.3, BLACK);
        f.rect(PAGE.0, PAGE.1, PAGE.2, PAGE.3, WHITE);
        // barra inferior
        for (x0, x1) in [(22, 137), (155, 486), (518, 616)] {
            f.rect(x0, 461, 1, 14, BLACK);
            f.rect(x1, 461, 1, 14, BLACK);
            f.rect(x0 + 1, 461, x1 - x0 - 1, 14, WHITE);
        }
        let pages = format!("Pág. {}", self.page + 1);
        match a.font(8) {
            Some(font) => {
                f.text_font(&font, 69, 461, "Escritório", BLACK);
                let name = "Documento Novo";
                let w = font.width(name) as i32;
                f.text_font(&font, 320 - w / 2, 462, name, BLACK);
                f.text_font(&font, 537, 462, &pages, BLACK);
            }
            None => {
                f.text(44, 464, "Escritório", BLACK);
                f.text(264, 464, "Documento Novo", BLACK);
                f.text(536, 464, &pages, BLACK);
            }
        }
        f.blit(&a.image(&pic(0x03c3)), 27, 463);
        f.blit(&a.image(&pic(0x03c3)), 523, 463);
        f.blit_masked(
            &a.image(&pic(0x02db)),
            &a.image(&pic(0x02dd)),
            PAGE_PREV.0,
            PAGE_PREV.1,
        );
        f.blit_masked(
            &a.image(&pic(0x02df)),
            &a.image(&pic(0x02e1)),
            PAGE_NEXT.0,
            PAGE_NEXT.1,
        );
    }

    fn draw_tools(&self, f: &mut Frame, a: &Assets) {
        for t in &TOOLS {
            let active = t.action == Action::Select(self.tool);
            let (img, mask) = if active { t.active } else { t.normal };
            if active {
                let w = a.image(&pic(img)).width as i32;
                f.rect(t.x - 5, 5, w + 11, 51, LIGHT);
                f.rect(t.x - 6, 5, 1, 50, BLACK);
                f.rect(t.x + w + 6, 5, 1, 50, BLACK);
            }
            f.blit_masked(&a.image(&pic(img)), &a.image(&pic(mask)), t.x, t.y);
        }
        f.blit_masked(&a.image(&pic(0x0235)), &a.image(&pic(0x0237)), 536, 47);
        f.blit_masked(&a.image(&pic(0x023d)), &a.image(&pic(0x023f)), 581, 16);
    }

    fn draw_options(&self, f: &mut Frame, a: &Assets) {
        match self.tool {
            Tool::Text => {
                for b in &STRIP {
                    let img = a.image(&pic(b.img));
                    match b.mask {
                        Some(m) => f.blit_masked(&img, &a.image(&pic(m)), b.x, b.y),
                        None => f.blit(&img, b.x, b.y),
                    }
                }
            }
            Tool::Stickers => self.draw_strip(f, a),
            Tool::Scissors => {
                f.blit(&a.image(&pic(0x0839)), EXPLOSION.0, EXPLOSION.1);
            }
        }
    }

    /// Faixa de opções dos adesivos: rolos da página e setas.
    fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        let left = a.image(&pic(0x0245));
        blit_part(f, &left, &a.image(&pic(0x0247)), 9, STRIP_Y, (0, 14));
        let right = a.image(&pic(0x0239));
        let end = SLOT_X0 + SLOT_W * ROLLS_PER_PAGE as i32;
        blit_part(
            f,
            &right,
            &a.image(&pic(0x023b)),
            end - 52,
            STRIP_Y,
            (52, 64),
        );
        for k in 0..ROLLS_PER_PAGE {
            let roll = self.roll_page * ROLLS_PER_PAGE + k;
            let Some(r) = self.rolls.get(roll) else { break };
            let rect = slot_rect(k);
            f.blit_masked(
                &a.image(&pic(0x0249)),
                &a.image(&pic(0x024b)),
                rect.0,
                rect.1,
            );
            if let Some(icon) = &r.icon {
                let (w, h) = (icon.width as i32, icon.height as i32);
                f.blit(icon, rect.0 + 26 - w / 2, rect.1 + 21 - h / 2);
            }
        }
        f.blit_masked(
            &a.image(&pic(0x03b3)),
            &a.image(&pic(0x03b5)),
            ARROW_UP.0,
            ARROW_UP.1,
        );
        f.blit_masked(
            &a.image(&pic(0x03b7)),
            &a.image(&pic(0x03b9)),
            ARROW_DOWN.0,
            ARROW_DOWN.1,
        );
        f.rect(504, 75, 16, 12, BLACK);
        f.rect(505, 76, 14, 10, WHITE);
        f.text(508, 77, &(self.roll_page + 1).to_string(), BLACK);
    }

    fn draw_film(&self, f: &mut Frame, a: &Assets) {
        let film = self.film();
        let (fx, fy, fw, fh) = film.rect();
        f.rect(fx, fy, fw, fh, BLACK);
        for y in fy + 1..fy + fh - 1 {
            for x in fx + 1..fx + fw - 1 {
                f.put(x, y, if (x + y) % 2 == 0 { YELLOW } else { WHITE });
            }
        }
        let Some(roll) = self.rolls.get(self.roll) else {
            return;
        };
        for j in 0..FILM_CELLS {
            let Some(item) = roll.items.get(self.scroll + j) else {
                break;
            };
            let r = film.cell(j);
            f.rect(r.0, r.1, r.2, r.3, BLACK);
            f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, WHITE);
            if let Some(t) = &item.preview {
                let (tw, th) = (t.width as i32, t.height as i32);
                let inner = (r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2);
                let (x, y) = (r.0 + r.2 / 2 - tw / 2, r.1 + r.3 / 2 - th / 2);
                blit_clipped(f, t, t, x, y, inner);
            }
        }
        let (u, d) = (film.up(), film.down());
        f.blit_masked(&a.image(&pic(0x02db)), &a.image(&pic(0x02dd)), u.0, u.1);
        f.blit_masked(&a.image(&pic(0x02df)), &a.image(&pic(0x02e1)), d.0, d.1);
    }

    /// Texto da página atual, adesivos e cursor.
    fn draw_text(&self, f: &mut Frame) {
        let l = self.layout();
        let (ox, oy) = (PAGE.0 + MARGIN, PAGE.1 + MARGIN);
        let mut put = |x: i32, y: i32, rgb: [u8; 3]| {
            if inside(PAGE, x, y) {
                f.put(x, y, rgb);
            }
        };
        for line in l.lines.iter().filter(|l| l.page == self.page) {
            let para = &self.doc.paras[line.para];
            let ly = oy + line.y;
            for b in &line.boxes {
                let bx = ox + b.x;
                match &para.items[b.item] {
                    Item::Char(c, style) => {
                        if c.is_whitespace() && !style.underline {
                            continue;
                        }
                        let g = self.fonts.glyph(*c, style);
                        let top = ly + line.ascent - g.ascent;
                        let rgb = self.rgb(style.color);
                        for (dx, dy) in &g.pixels {
                            put(bx + dx, top + dy, rgb);
                        }
                    }
                    Item::Sticker(s) => {
                        let top = ly + line.ascent - s.h as i32;
                        match self.art.get(&(s.roll, s.index)).and_then(|a| a.as_ref()) {
                            Some(img) => {
                                for y in 0..img.height as i32 {
                                    for x in 0..img.width as i32 {
                                        let i = ((y as u32 * img.width + x as u32) * 4) as usize;
                                        let p = &img.rgba[i..i + 3];
                                        if (p[0] as u32 + p[1] as u32 + p[2] as u32) < 384 {
                                            put(bx + x, top + y, BLACK);
                                        }
                                    }
                                }
                            }
                            None => {
                                // adesivo sem desenho: quadro com um X
                                let (w, h) = (s.w as i32, s.h as i32);
                                for k in 0..w.max(h) {
                                    put(bx + k.min(w - 1), top + k.min(h - 1), BLACK);
                                    put(bx + (w - 1 - k).max(0), top + k.min(h - 1), BLACK);
                                }
                                for x in 0..w {
                                    put(bx + x, top, BLACK);
                                    put(bx + x, top + h - 1, BLACK);
                                }
                                for y in 0..h {
                                    put(bx, top + y, BLACK);
                                    put(bx + w - 1, top + y, BLACK);
                                }
                            }
                        }
                    }
                }
            }
        }
        // cursor: linha de 2 px na cor atual, piscando
        if self.list.is_none() && self.blink % CARET_PERIOD < CARET_PERIOD / 2.0 {
            let (k, x) = l.cursor_pos(self.doc.cursor);
            let line = &l.lines[k];
            if line.page == self.page {
                let st = self.doc.style_at_cursor();
                let (h, asc) = self.fonts.metrics(st.size);
                let top = oy + line.y + line.ascent - asc;
                let rgb = self.rgb(self.doc.style.color);
                for dy in 0..h {
                    put(ox + x, top + dy, rgb);
                    put(ox + x + 1, top + dy, rgb);
                }
            }
        }
    }

    fn draw_list(&self, f: &mut Frame, a: &Assets) {
        let Some(list) = self.list else { return };
        let (x, y, w, h) = self.list_rect(list);
        f.rect(x, y, w, h, BLACK);
        f.rect(x + 1, y + 1, w - 2, h - 2, WHITE);
        let font = a.font(8);
        let text = |f: &mut Frame, x: i32, y: i32, s: &str| match &font {
            Some(font) => {
                f.text_font(font, x, y, s, BLACK);
            }
            None => f.text(x, y + 3, s, BLACK),
        };
        let style = self.doc.style;
        match list {
            List::Fonts => {
                let r = self.list_item_rect(list, 0);
                f.rect(r.0, r.1, r.2, r.3, LIGHT);
                text(f, r.0 + 4, r.1 + 1, "Pablo Graphos");
                text(f, r.0 + 4, r.1 + 1 + ITEM_H, "(os outros tipos de letra");
                text(f, r.0 + 4, r.1 + 1 + 2 * ITEM_H, "ainda não existem)");
            }
            List::Sizes => {
                for (k, s) in Size::ALL.iter().enumerate() {
                    let r = self.list_item_rect(list, k);
                    if *s == style.size {
                        f.rect(r.0, r.1, r.2, r.3, LIGHT);
                    }
                    text(f, r.0 + 4, r.1 + 1, s.name());
                }
            }
            List::Colors => {
                for (k, c) in COLORS.iter().enumerate() {
                    let r = self.list_item_rect(list, k);
                    f.rect(r.0 + 2, r.1 + 1, r.2 - 4, r.3 - 2, self.rgb(*c));
                    if *c == style.color {
                        f.frame_rect(r.0, r.1, r.2, r.3, 1, BLACK);
                    }
                }
            }
            List::Styles => {
                let on = [
                    !style.bold && !style.italic && !style.underline,
                    style.bold,
                    style.italic,
                    style.underline,
                ];
                for (k, name) in STYLE_NAMES.iter().enumerate() {
                    let r = self.list_item_rect(list, k);
                    if on[k] {
                        f.rect(r.0, r.1, r.2, r.3, LIGHT);
                    }
                    text(f, r.0 + 4, r.1 + 1, name);
                }
            }
            List::Align => {
                let cur = self.doc.para().align;
                for (k, id) in ALIGN_ITEMS.iter().enumerate() {
                    let r = self.list_item_rect(list, k);
                    f.blit(&a.image(&pic(*id)), r.0, r.1);
                    if Align::ALL[k] == cur {
                        f.frame_rect(r.0, r.1, r.2, r.3, 1, RED);
                    }
                }
            }
        }
    }
}

impl Scene for Writer {
    fn update(&mut self, input: &[Input], dt: f32, _assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        for i in input {
            match *i {
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.mouse = (x, y);
                    self.click(x, y, &mut fx);
                }
                Input::MouseMove { x, y } => self.mouse = (x, y),
                Input::Key {
                    key,
                    ctrl: false,
                    alt: false,
                } => self.key(key),
                Input::Text(ref t) => self.type_text(t),
                _ => {}
            }
        }
        if dt.is_finite() {
            self.blink = (self.blink + dt.max(0.0)) % CARET_PERIOD;
            self.history.tick(dt);
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        self.draw_chrome(frame, assets);
        self.draw_tools(frame, assets);
        self.draw_options(frame, assets);
        self.draw_text(frame);
        if self.tool == Tool::Stickers && self.film_open {
            self.draw_film(frame, assets);
        }
        self.draw_list(frame, assets);
    }

    fn leave(&mut self) -> Vec<Effect> {
        let Some(dir) = &self.texts else {
            return vec![Effect::Log(
                "sem pasta de textos (HOME e XDG_DATA_HOME vazios): texto não guardado".into(),
            )];
        };
        let msg = match save(dir, &self.doc, self.changed, now_secs()) {
            Ok(()) => format!("texto guardado em {}", dir.display()),
            Err(e) => e,
        };
        self.changed = false;
        vec![Effect::Log(msg)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::SceneId;
    use crate::scenes::testutil::{assets, empty_assets};
    use crate::writer::doc::Run;
    use crate::{SCREEN_H, SCREEN_W};

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            ctrl: false,
            alt: false,
        }
    }

    fn text(s: &str) -> Input {
        Input::Text(s.into())
    }

    /// Pixels não brancos dentro da página.
    fn inked(w: &Writer, a: &Assets) -> usize {
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, a);
        let mut n = 0;
        for y in PAGE.1..PAGE.1 + PAGE.3 {
            for x in PAGE.0..PAGE.0 + PAGE.2 {
                if f.pixel(x as u32, y as u32) != [255, 255, 255, 255] {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn typing_shows_text_on_the_page() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        // cursor apagado (meio período) para contar só o texto
        w.update(&[], 0.6, &a);
        assert_eq!(inked(&w, &a), 0);
        w.update(&[text("Oi")], 0.0, &a);
        w.update(&[], 0.6, &a);
        assert_eq!(w.doc().para().text(), "Oi");
        assert!(inked(&w, &a) > 20);
        // o cursor aceso acrescenta uma linha de 2 px
        w.update(&[], 0.5, &a);
        assert!(inked(&w, &a) > 20 + 32);
    }

    #[test]
    fn enter_makes_a_new_paragraph_and_egg_undoes() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[text("a"), key(Key::Enter), text("b")], 0.0, &a);
        assert_eq!(w.doc().paras.len(), 2);
        assert_eq!(w.doc().paras[1].text(), "b");
        w.update(&[click(280, 30)], 0.0, &a); // ovo
        assert_eq!(w.doc().paras.len(), 2);
        assert_eq!(w.doc().paras[1].text(), "");
        w.update(&[click(280, 30)], 0.0, &a);
        assert_eq!(w.doc().paras[1].text(), "b");
    }

    #[test]
    fn size_list_changes_the_run_style() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[click(100, 70)], 0.0, &a);
        assert_eq!(w.list(), Some(List::Sizes));
        // primeiro item: Gigante
        let r = w.list_item_rect(List::Sizes, 0);
        w.update(&[click(r.0 + 5, r.1 + 5), text("a")], 0.0, &a);
        assert_eq!(w.list(), None);
        assert_eq!(w.doc().style.size, Size::Giant);
        let runs = w.doc().para().runs();
        assert!(matches!(&runs[0], Run::Text { style, .. } if style.size == Size::Giant));
        // clique fora fecha a lista sem escolher
        w.update(&[click(100, 70), click(300, 300)], 0.0, &a);
        assert_eq!(w.list(), None);
        assert_eq!(w.doc().style.size, Size::Giant);
    }

    #[test]
    fn color_and_style_lists_apply() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[click(170, 70)], 0.0, &a);
        let r = w.list_item_rect(List::Colors, 3);
        w.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert_eq!(w.doc().style.color, COLORS[3]);
        w.update(&[click(230, 70)], 0.0, &a);
        let r = w.list_item_rect(List::Styles, 1);
        w.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert!(w.doc().style.bold);
        w.update(&[click(300, 70)], 0.0, &a);
        let r = w.list_item_rect(List::Align, 2);
        w.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert_eq!(w.doc().para().align, Align::Center);
    }

    #[test]
    fn sticker_without_data_uses_placeholder() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[click(80, 30)], 0.0, &a);
        assert_eq!(w.tool(), Tool::Stickers);
        assert_eq!(w.roll_count(), 0);
        // sem rolos: cliques na faixa e nas setas não fazem nada
        w.update(&[click(40, 70), click(510, 62), click(510, 95)], 0.0, &a);
        assert!(!w.film_open());
        w.insert_sticker(5, 2);
        assert!(matches!(
            w.doc().para().items[0],
            Item::Sticker(s) if s.w == 32 && s.h == 32
        ));
        assert!(inked(&w, &a) > 60);
    }

    #[test]
    fn scissors_explosion_clears_with_undo() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[text("abc"), click(330, 30)], 0.0, &a);
        assert_eq!(w.tool(), Tool::Scissors);
        w.update(&[click(40, 80)], 0.0, &a);
        assert!(w.doc().is_empty());
        w.update(&[click(280, 30)], 0.0, &a);
        assert_eq!(w.doc().para().text(), "abc");
    }

    #[test]
    fn cursor_keys_and_click_move_the_cursor() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[text("ab"), key(Key::Left), text("x")], 0.0, &a);
        assert_eq!(w.doc().para().text(), "axb");
        w.update(
            &[key(Key::Home), text("1"), key(Key::End), text("2")],
            0.0,
            &a,
        );
        assert_eq!(w.doc().para().text(), "1axb2");
        w.update(&[key(Key::Enter), text("c"), key(Key::Up)], 0.0, &a);
        assert_eq!(w.doc().cursor.para, 0);
        w.update(&[key(Key::Down)], 0.0, &a);
        assert_eq!(w.doc().cursor.para, 1);
        // clique no começo da primeira linha
        w.update(&[click(PAGE.0 + MARGIN, PAGE.1 + MARGIN + 2)], 0.0, &a);
        assert_eq!(w.doc().cursor, crate::writer::Cursor { para: 0, index: 0 });
        w.update(&[key(Key::Backspace)], 0.0, &a);
        assert_eq!(w.doc().para().text(), "1axb2");
    }

    #[test]
    fn pages_follow_the_cursor_and_arrows() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        // 16 px por linha (8×8 reserva ×2): 301 px cabem 18 linhas
        let mut inputs = Vec::new();
        for _ in 0..25 {
            inputs.push(text("a"));
            inputs.push(key(Key::Enter));
        }
        w.update(&inputs, 0.0, &a);
        assert_eq!(w.page(), 1);
        w.update(&[key(Key::PageUp)], 0.0, &a);
        assert_eq!(w.page(), 0);
        w.update(&[click(PAGE_NEXT.0 + 2, PAGE_NEXT.1 + 2)], 0.0, &a);
        assert_eq!(w.page(), 1);
        w.update(&[click(PAGE_NEXT.0 + 2, PAGE_NEXT.1 + 2)], 0.0, &a);
        assert_eq!(w.page(), 1);
        w.update(&[click(PAGE_PREV.0 + 2, PAGE_PREV.1 + 2)], 0.0, &a);
        assert_eq!(w.page(), 0);
        w.update(&[text("b")], 0.0, &a);
        assert_eq!(w.page(), 1);
    }

    #[test]
    fn leave_saves_and_reload_restores() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut w = Writer::new(Some(dir.path().to_path_buf()), &a);
        w.update(&[text("Olá"), key(Key::Enter), text("mundo")], 0.0, &a);
        w.insert_sticker(1, 0);
        assert!(matches!(&w.leave()[..], [Effect::Log(s)] if s.contains("guardado em")));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
        let again = Writer::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(again.doc().paras.len(), 2);
        assert_eq!(again.doc().paras[0].text(), "Olá");
        assert_eq!(again.doc().paras[1].text(), "mundo");
        assert!(matches!(
            again.doc().paras[1].items[5],
            Item::Sticker(s) if s.roll == 1 && s.w == 32
        ));
        let mut untouched = Writer::new(Some(dir.path().to_path_buf()), &a);
        untouched.leave();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn leave_without_dir_logs() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        assert!(matches!(&w.leave()[..], [Effect::Log(s)] if s.contains("não guardado")));
    }

    #[test]
    fn exit_door_quits_and_unavailable_tools_log() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        assert!(w.update(&[click(600, 60)], 0.0, &a).contains(&Effect::Quit));
        let fx = w.update(&[click(125, 30)], 0.0, &a);
        assert!(matches!(&fx[..], [Effect::Log(s)] if s.contains("sons")));
        assert_eq!(w.tool(), Tool::Text);
        let _ = SceneId::Writer;
    }

    #[test]
    fn real_assets_chrome_draws_icons_and_text() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new(None, &a);
        assert!(w.roll_count() > 0);
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        // ícone do Texto, em cima do fundo cinza da barra
        assert_ne!(f.pixel(30, 30), [DARK[0], DARK[1], DARK[2], 255]);
        // adesivo real: tamanho do desenho e tinta preta na página
        w.update(&[click(80, 30), click(40, 70)], 0.0, &a);
        assert!(w.film_open());
        let film = w.film();
        let c = film.cell(0);
        w.update(&[click(c.0 + 5, c.1 + 5)], 0.0, &a);
        assert!(!w.film_open());
        assert!(matches!(w.doc().para().items[0], Item::Sticker(s) if s.w != 32 || s.h != 32));
        w.update(&[], 0.6, &a);
        assert!(inked(&w, &a) > 0);
        // texto na "Pablo Graphos" de 16 pontos, num parágrafo só de texto
        w.update(&[key(Key::Enter), text("Olá")], 0.0, &a);
        let l = w.layout();
        assert_eq!(l.lines[1].height, a.font(16).unwrap().height as i32);
    }
}
