//! Balões com campos de texto, figuras para escolher e botões Volta / Para /
//! Segue: os do cadastro da Entrada (`WHAT 4400` e `4402` de `ARTIST.EXE`) e
//! os avisos "EPA!" do Rex. Complementa `Balloon` (texto e opções).
//!
//! Tudo é disposto como texto formatado ([`crate::rich`]): no `WHAT`, uma
//! figura de id 0 é uma caixa de texto (o parâmetro é a largura em pixels),
//! `0x22e` é a bolinha verde de uma opção com figura (seguida do bicho,
//! `0x237`…`0x242`, figuras `0x45b`, `0x45d`… de `ARTIST.PIC`, e de espaços,
//! o que forma a grade de três colunas do original), `0x22c` marca uma opção
//! de resposta (bolinha preta) e `0x230` é o cabeçalho "EPA!" dos avisos
//! (casinha do Rex e cachorro, figura `0x2a9`), centralizado. Os
//! avisos têm cantos retos e o mesmo fundo pontilhado; os botões ficam em
//! três vagas fixas (Volta, Para, Segue) centralizadas, e um botão que falta
//! deixa a vaga vazia, como no original.

use crate::assets::{Assets, ImageRef, Program};
use crate::balloon::{BOX_EXTRA, PAD, PAD_BOTTOM, WRAP_LESS, draw_frame, plain_items};
use crate::rich::{self, Ctx, Laid, Run};
use crate::scene::{Input, Key, MouseButton};
use crate::{Frame, Image};
use formats::fnt::Font;
use formats::what::{Item, PIC_OPTION, PIC_OPTION_PICTURE, Record};
use std::rc::Rc;

type Rect = (i32, i32, i32, i32);

const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];
/// Botões Volta / Para / Segue (31×14, com a máscara duas imagens adiante),
/// em três vagas com 10 px entre elas.
const BTN_W: i32 = 31;
const BTN_H: i32 = 14;
const BTN_GAP: i32 = 10;
/// Do pé do texto até os botões, e dos botões até o pé da caixa.
const BTN_ABOVE: i32 = 15;
const BTN_BELOW: i32 = 8;
pub const PIC_BACK: u16 = 0x40f;
pub const PIC_STOP: u16 = 0x417;
pub const PIC_NEXT: u16 = 0x413;
/// Bolinha verde vazia e marcada (15×14).
pub const PIC_BALL: u16 = 0x409;
pub const PIC_BALL_CHECKED: u16 = 0x40d;
/// Primeiro bicho (19×11); os outros estão de 2 em 2.
pub const PIC_ANIMAL0: u16 = 0x45b;
/// Cabeçalho "EPA!" dos avisos (195×48: casinha do Rex, "EPA!" e cachorro).
pub const PIC_ALERT: u16 = 0x2a9;
/// Célula da grade: bolinha, espaço e bicho.
const CELL_W: i32 = 40;
const WHAT_FIELD: u16 = 0;
const WHAT_ANIMAL0: u16 = 0x237;
const WHAT_ANIMALS: u16 = 12;
const WHAT_HEAD: u16 = 0x230;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Back,
    Stop,
    Next,
}

impl Button {
    fn pic(self) -> u16 {
        match self {
            Button::Back => PIC_BACK,
            Button::Stop => PIC_STOP,
            Button::Next => PIC_NEXT,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Button::Back => "Volta",
            Button::Stop => "Para",
            Button::Next => "Segue",
        }
    }

    /// Vaga do botão (0, 1, 2).
    fn slot(self) -> i32 {
        match self {
            Button::Back => 0,
            Button::Stop => 1,
            Button::Next => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Text(String),
    /// Caixa de texto do campo `field`, com `width` pixels.
    Field {
        field: usize,
        width: i32,
    },
    /// Figuras para escolher uma, em `cols` colunas.
    Grid {
        pictures: Vec<u16>,
        cols: usize,
    },
    /// Figura decorativa.
    Figure(u16),
    /// Opções de resposta, cada uma com a bolinha preta na frente.
    Options(Vec<String>),
    /// Trecho de um registro `WHAT`, com a formatação, as caixas de texto
    /// (`fields`), as figuras para escolher e as opções (`options`, na ordem).
    Rich {
        items: Vec<Item>,
        options: Vec<String>,
    },
}

impl Part {
    /// Itens equivalentes, para dispor como texto formatado.
    fn items(&self) -> Vec<Item> {
        match self {
            Part::Text(t) => plain_items(t, &[], &[], &[]),
            Part::Field { width, .. } => vec![
                Item::Picture {
                    id: WHAT_FIELD,
                    param: (*width).clamp(0, 0xffff) as u16,
                },
                Item::Newline,
            ],
            Part::Grid { pictures, cols } => {
                let mut v = Vec::new();
                for row in pictures.chunks((*cols).max(1)) {
                    for (c, pic) in row.iter().enumerate() {
                        v.push(Item::Picture {
                            id: PIC_OPTION_PICTURE,
                            param: 0,
                        });
                        v.push(space_item(" "));
                        v.push(Item::Picture {
                            id: what_for(*pic),
                            param: 0,
                        });
                        if c + 1 < row.len() {
                            v.push(space_item("            "));
                        }
                    }
                    v.push(Item::Newline);
                }
                v
            }
            Part::Figure(pic) => {
                let id = if *pic == PIC_ALERT { WHAT_HEAD } else { *pic };
                vec![
                    Item::Format {
                        indent: 0,
                        center: true,
                    },
                    Item::Picture { id, param: 0 },
                    Item::Newline,
                    Item::Format {
                        indent: 0,
                        center: false,
                    },
                ]
            }
            Part::Options(o) => plain_items("", &[], &[], o)
                .into_iter()
                .skip_while(|i| !matches!(i, Item::Format { .. }))
                .collect(),
            Part::Rich { items, .. } => {
                let mut v = items.clone();
                if !matches!(v.last(), Some(Item::Newline)) {
                    v.push(Item::Newline);
                }
                v
            }
        }
    }

    /// Número de caixas de texto.
    fn fields(&self) -> usize {
        match self {
            Part::Field { .. } => 1,
            Part::Rich { items, .. } => items
                .iter()
                .filter(|i| matches!(i, Item::Picture { id: WHAT_FIELD, .. }))
                .count(),
            _ => 0,
        }
    }
}

fn space_item(s: &str) -> Item {
    Item::Text {
        text: s.into(),
        bold: false,
        color: 0xf,
        indent: 0,
        key: false,
        signature: false,
    }
}

/// Figura do `.PIC` correspondente a um id de figura do `WHAT`.
pub fn pic_for(what_id: u16) -> Option<u16> {
    if (WHAT_ANIMAL0..WHAT_ANIMAL0 + WHAT_ANIMALS).contains(&what_id) {
        Some(PIC_ANIMAL0 + 2 * (what_id - WHAT_ANIMAL0))
    } else if what_id == WHAT_HEAD {
        Some(PIC_ALERT)
    } else {
        None
    }
}

/// Id do `WHAT` de uma figura do `.PIC` (bichos e cabeçalho "EPA!").
fn what_for(pic: u16) -> u16 {
    if (PIC_ANIMAL0..PIC_ANIMAL0 + 2 * WHAT_ANIMALS).contains(&pic) {
        WHAT_ANIMAL0 + (pic - PIC_ANIMAL0) / 2
    } else if pic == PIC_ALERT {
        WHAT_HEAD
    } else {
        pic
    }
}

#[derive(Clone)]
pub struct TextBalloon {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub parts: Vec<Part>,
    pub fields: Vec<String>,
    /// Campo com o cursor.
    pub focus: usize,
    /// Figura escolhida na grade.
    pub selected: usize,
    pub buttons: Vec<Button>,
    /// Aviso do Rex: cantos retos.
    pub alert: bool,
    /// O texto do campo com o cursor está todo selecionado: digitar o
    /// substitui, Backspace o apaga.
    pub select_all: bool,
}

struct Layout {
    laid: Laid,
    fields: Vec<(usize, Rect)>,
    cells: Vec<Rect>,
    options: Vec<Rect>,
    buttons: Vec<(Button, Rect)>,
    h: i32,
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

/// A imagem existe no `ARTIST.PIC` (senão vem o marcador de 32×32).
fn has_pic(a: &Assets, id: u16) -> bool {
    has_pic_in(a, Program::Artist, id)
}

/// A imagem existe no `.PIC` do programa `p`.
fn has_pic_in(a: &Assets, p: Program, id: u16) -> bool {
    a.pic_ids(p).binary_search(&id).is_ok()
}

/// Desenha uma figura colorida do `ARTIST.PIC`: com a máscara duas imagens
/// adiante quando ela tem o mesmo tamanho, senão tratando branco como
/// transparente. Sem a figura, não desenha nada.
pub fn blit_pic(f: &mut Frame, a: &Assets, id: u16, x: i32, y: i32, masked: bool) {
    blit_pic_in(f, a, Program::Artist, id, x, y, masked);
}

/// Como [`blit_pic`], com uma figura do `.PIC` do programa `p`.
pub fn blit_pic_in(f: &mut Frame, a: &Assets, p: Program, id: u16, x: i32, y: i32, masked: bool) {
    if !has_pic_in(a, p, id) {
        return;
    }
    let img = a.image(&ImageRef::Pic(p, id));
    let mask = if masked {
        a.pic_next_id(p, id, 2)
            .map(|m| a.image(&ImageRef::Pic(p, m)))
            .filter(|m| m.height == img.height && m.width.abs_diff(img.width) <= 1)
    } else {
        None
    };
    match mask {
        Some(m) => crate::anim::blit_masked(f, &img, &m, x, y),
        None => blit_white_transparent(f, &img, x, y),
    }
}

/// Desenha uma figura do `ARTIST.PIC` inteira (o branco é opaco); sem ela, nada.
pub fn blit_pic_opaque(f: &mut Frame, a: &Assets, id: u16, x: i32, y: i32) {
    if has_pic(a, id) {
        f.blit(&a.image(&ImageRef::Pic(Program::Artist, id)), x, y);
    }
}

fn blit_white_transparent(f: &mut Frame, img: &Image, x: i32, y: i32) {
    for dy in 0..img.height {
        for dx in 0..img.width {
            let i = ((dy * img.width + dx) * 4) as usize;
            let p = &img.rgba[i..i + 4];
            if p[3] != 0 && p[..3] != WHITE {
                f.put(x + dx as i32, y + dy as i32, [p[0], p[1], p[2]]);
            }
        }
    }
}

/// Botão com figura do original ou, sem ela, um retângulo com o rótulo.
pub fn draw_button(f: &mut Frame, a: &Assets, b: Button, x: i32, y: i32) {
    if has_pic(a, b.pic()) {
        blit_pic(f, a, b.pic(), x, y, true);
    } else {
        f.rect(x, y, BTN_W, BTN_H, BLACK);
        f.rect(x + 1, y + 1, BTN_W - 2, BTN_H - 2, WHITE);
        f.text(x + 2, y + 3, &b.label()[..3], BLACK);
    }
}

impl TextBalloon {
    pub fn new(x: i32, y: i32, w: i32, parts: Vec<Part>, buttons: Vec<Button>) -> TextBalloon {
        let n = parts
            .iter()
            .filter_map(|p| match p {
                Part::Field { field, .. } => Some(*field + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0)
            .max(parts.iter().map(Part::fields).sum());
        TextBalloon {
            x,
            y,
            w: w.max(120),
            parts,
            fields: vec![String::new(); n],
            focus: 0,
            selected: 0,
            buttons,
            alert: false,
            select_all: false,
        }
    }

    /// O mesmo balão como aviso (cantos retos).
    pub fn as_alert(mut self) -> TextBalloon {
        self.alert = true;
        self
    }

    /// Texto das opções, se o balão as tiver.
    pub fn options(&self) -> &[String] {
        self.parts
            .iter()
            .find_map(|p| match p {
                Part::Options(o) => Some(o.as_slice()),
                Part::Rich { options, .. } if !options.is_empty() => Some(options.as_slice()),
                _ => None,
            })
            .unwrap_or(&[])
    }

    /// Balão a partir de um registro `WHAT`, com a formatação, as caixas
    /// de texto, a grade de figuras e as opções do original (o `@` vira
    /// `name`).
    pub fn from_record(
        r: &Record,
        x: i32,
        y: i32,
        name: &str,
        buttons: Vec<Button>,
    ) -> TextBalloon {
        let items = r
            .items
            .iter()
            .cloned()
            .map(|i| match i {
                Item::Text {
                    text,
                    bold,
                    color,
                    indent,
                    key,
                    signature,
                } => Item::Text {
                    text: text.replace('@', name),
                    bold,
                    color,
                    indent,
                    key,
                    signature,
                },
                other => other,
            })
            .collect();
        let options = r.options().into_iter().map(|o| o.1).collect();
        TextBalloon::new(
            x,
            y,
            r.width as i32,
            vec![Part::Rich { items, options }],
            buttons,
        )
    }

    /// Tira as linhas (parágrafos) com `needle` do texto do balão (a dica
    /// do disquete do cadastro, por exemplo).
    pub fn drop_lines_containing(&mut self, needle: &str) {
        for p in &mut self.parts {
            match p {
                Part::Text(t) => {
                    *t = t
                        .lines()
                        .filter(|l| !l.contains(needle))
                        .collect::<Vec<_>>()
                        .join("\n")
                        .trim()
                        .to_string();
                }
                Part::Rich { items, .. } => {
                    let mut out = Vec::new();
                    let mut para = Vec::new();
                    for i in items.drain(..) {
                        let end = matches!(i, Item::Newline);
                        para.push(i);
                        if end {
                            let hit = para.iter().any(
                                |i| matches!(i, Item::Text { text, .. } if text.contains(needle)),
                            );
                            if !hit {
                                out.append(&mut para);
                            }
                            para.clear();
                        }
                    }
                    if !para
                        .iter()
                        .any(|i| matches!(i, Item::Text { text, .. } if text.contains(needle)))
                    {
                        out.append(&mut para);
                    }
                    *items = out;
                }
                _ => {}
            }
        }
        self.parts
            .retain(|p| !matches!(p, Part::Text(t) if t.is_empty()));
    }

    /// Texto do balão, sem formatação nem opções (para conferir).
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        let mut in_option = false;
        for i in self
            .parts
            .iter()
            .filter(|p| !matches!(p, Part::Options(_)))
            .flat_map(Part::items)
        {
            match i {
                Item::Text { text, .. } if !in_option => out.push_str(&text),
                Item::Picture { id: PIC_OPTION, .. } => in_option = true,
                Item::Newline => {
                    in_option = false;
                    out.push('\n');
                }
                _ => {}
            }
        }
        out.trim().to_string()
    }

    /// O balão abre com o cabeçalho "EPA!" do Rex.
    pub fn has_alert_header(&self) -> bool {
        self.parts
            .iter()
            .flat_map(Part::items)
            .find_map(|i| match i {
                Item::Picture { id, .. } => Some(id),
                Item::Text { text, .. } if !text.trim().is_empty() => Some(u16::MAX),
                _ => None,
            })
            == Some(WHAT_HEAD)
    }

    fn font(a: &Assets) -> Option<Rc<Font>> {
        a.font(8)
    }

    /// Largura da caixa: a do registro mais 2 (mais 1 nos avisos).
    pub fn box_width(&self) -> i32 {
        self.w + if self.alert { 1 } else { BOX_EXTRA }
    }

    fn items(&self) -> Vec<Item> {
        self.parts.iter().flat_map(Part::items).collect()
    }

    fn layout(&self, a: &Assets, font: Option<&Font>) -> Layout {
        let none = |_: usize| None;
        let ctx = Ctx {
            assets: a,
            program: Program::Artist,
            font,
            name: "",
            glossary: &none,
            own: None,
        };
        let laid = rich::layout(&self.items(), self.w - WRAP_LESS, &ctx);
        let (ox, oy) = (self.x + PAD, self.y + PAD);
        let abs = |r: Rect| (ox + r.0, oy + r.1, r.2, r.3);
        let fields = laid
            .fields
            .iter()
            .enumerate()
            .map(|(k, r)| (k, abs(*r)))
            .collect();
        let cells = laid
            .runs
            .iter()
            .filter_map(|r| match r {
                Run::Option {
                    x,
                    y,
                    id: PIC_OPTION_PICTURE,
                    ..
                } => Some(abs((*x, *y, CELL_W, rich::OPTION_H))),
                _ => None,
            })
            .collect();
        let options = laid
            .options
            .iter()
            .map(|r| {
                let (x, y, w, h) = abs(*r);
                (x, y - rich::PARA / 2, w, h + rich::PARA)
            })
            .collect();
        let mut y = oy + laid.height;
        let mut buttons = Vec::new();
        if !self.buttons.is_empty() {
            y += BTN_ABOVE;
            let total = 3 * BTN_W + 2 * BTN_GAP;
            let x0 = self.x + (self.box_width() - total) / 2;
            for b in &self.buttons {
                let bx = x0 + b.slot() * (BTN_W + BTN_GAP);
                buttons.push((*b, (bx, y, BTN_W, BTN_H)));
            }
            y += BTN_H + BTN_BELOW;
        } else {
            y += PAD_BOTTOM;
        }
        Layout {
            laid,
            fields,
            cells,
            options,
            buttons,
            h: y - self.y,
        }
    }

    pub fn height(&self, a: &Assets) -> i32 {
        self.layout(a, Self::font(a).as_deref()).h
    }

    pub fn contains(&self, a: &Assets, x: i32, y: i32) -> bool {
        inside((self.x, self.y, self.box_width(), self.height(a)), x, y)
    }

    /// Retângulo do botão `b`, se o balão o tiver.
    pub fn button_rect(&self, a: &Assets, b: Button) -> Option<Rect> {
        self.layout(a, Self::font(a).as_deref())
            .buttons
            .into_iter()
            .find(|(k, _)| *k == b)
            .map(|(_, r)| r)
    }

    /// Retângulo da caixa do campo `k`.
    pub fn field_rect(&self, a: &Assets, k: usize) -> Option<Rect> {
        self.layout(a, Self::font(a).as_deref())
            .fields
            .into_iter()
            .find(|(f, _)| *f == k)
            .map(|(_, r)| r)
    }

    /// Retângulo da célula `k` da grade.
    pub fn cell_rect(&self, a: &Assets, k: usize) -> Option<Rect> {
        self.layout(a, Self::font(a).as_deref())
            .cells
            .get(k)
            .copied()
    }

    /// Retângulo da opção `k`, se o balão a tiver.
    pub fn option_rect(&self, a: &Assets, k: usize) -> Option<Rect> {
        self.layout(a, Self::font(a).as_deref())
            .options
            .get(k)
            .copied()
    }

    /// Opção em (x, y), se houver.
    pub fn hit_option(&self, a: &Assets, x: i32, y: i32) -> Option<usize> {
        self.layout(a, Self::font(a).as_deref())
            .options
            .iter()
            .position(|r| inside(*r, x, y))
    }

    /// Botão em (x, y), se houver.
    pub fn hit_button(&self, a: &Assets, x: i32, y: i32) -> Option<Button> {
        self.layout(a, Self::font(a).as_deref())
            .buttons
            .into_iter()
            .find(|(_, r)| inside(*r, x, y))
            .map(|(b, _)| b)
    }

    /// Ligação (dica `21b`/`21d`, Maggie `21e`, glossário) em (x, y).
    pub fn hot_at(&self, a: &Assets, x: i32, y: i32) -> Option<rich::Hot> {
        let lay = self.layout(a, Self::font(a).as_deref());
        lay.laid.hot_at(x - self.x - PAD, y - self.y - PAD)
    }

    /// (x, y) cai na linha do "Precisa de ajuda?" (figura `222` do `WHAT`),
    /// que o assistente do Max trata como ligação.
    pub fn help_at(&self, a: &Assets, x: i32, y: i32) -> bool {
        let lay = self.layout(a, Self::font(a).as_deref());
        let (ox, oy) = (self.x + PAD, self.y + PAD);
        lay.laid.runs.iter().any(|r| match r {
            Run::Picture { y: py, id, .. } if *id == formats::what::PIC_HELP => {
                inside((ox, oy + py, self.w, rich::LINE + 2), x, y)
            }
            _ => false,
        })
    }

    /// Largura máxima do texto que cabe no campo `k`.
    fn field_capacity(&self, a: &Assets, k: usize) -> i32 {
        self.field_rect(a, k).map_or(100, |r| r.2) - 6
    }

    fn text_width(font: Option<&Font>, s: &str) -> i32 {
        rich::text_width(font, s, false)
    }
    /// Trata uma entrada: cliques nos campos, na grade e nos botões; texto e
    /// Backspace no campo com o cursor; Enter é "Segue"; setas trocam o
    /// campo. Devolve o botão acionado.
    pub fn handle(&mut self, a: &Assets, input: &Input) -> Option<Button> {
        match input {
            Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } => {
                let lay = self.layout(a, Self::font(a).as_deref());
                if let Some((b, _)) = lay.buttons.iter().find(|(_, r)| inside(*r, *x, *y)) {
                    return Some(*b);
                }
                if let Some((f, _)) = lay.fields.iter().find(|(_, r)| inside(*r, *x, *y)) {
                    self.focus = *f;
                    self.select_all = false;
                }
                if let Some(k) = lay.cells.iter().position(|r| inside(*r, *x, *y)) {
                    self.selected = k;
                }
                None
            }
            Input::Text(s) => {
                if let Some(field) = self.fields.get(self.focus) {
                    let font = Self::font(a);
                    let cap = self.field_capacity(a, self.focus);
                    let mut t = if self.select_all {
                        String::new()
                    } else {
                        field.clone()
                    };
                    self.select_all = false;
                    for c in s.chars().filter(|c| !c.is_control()) {
                        t.push(c);
                        if Self::text_width(font.as_deref(), &t) > cap {
                            t.pop();
                            break;
                        }
                    }
                    self.fields[self.focus] = t;
                }
                None
            }
            Input::Key { key, ctrl, .. } if !ctrl => match key {
                Key::Backspace => {
                    if let Some(f) = self.fields.get_mut(self.focus) {
                        if self.select_all {
                            f.clear();
                        } else {
                            f.pop();
                        }
                    }
                    self.select_all = false;
                    None
                }
                Key::Enter => self.buttons.contains(&Button::Next).then_some(Button::Next),
                Key::Down => {
                    if !self.fields.is_empty() {
                        self.focus = (self.focus + 1) % self.fields.len();
                        self.select_all = false;
                    }
                    None
                }
                Key::Up => {
                    if !self.fields.is_empty() {
                        self.focus = (self.focus + self.fields.len() - 1) % self.fields.len();
                        self.select_all = false;
                    }
                    None
                }
                _ => None,
            },
            _ => None,
        }
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let font = Self::font(a);
        let font = font.as_deref();
        let lay = self.layout(a, font);
        draw_frame(f, self.x, self.y, self.box_width(), lay.h, !self.alert);
        let fields = &self.fields;
        let (focus, select_all) = (self.focus, self.select_all);
        rich::draw(
            f,
            a,
            font,
            &lay.laid,
            (self.x + PAD, self.y + PAD),
            Some(self.selected),
            &mut |f, k, (bx, by, w, h)| {
                f.rect(bx, by, w, h, BLACK);
                f.rect(bx + 1, by + 1, w - 2, h - 2, WHITE);
                let text = fields.get(k).map_or("", String::as_str);
                let tw = Self::text_width(font, text);
                if k == focus && select_all && !text.is_empty() {
                    // texto selecionado: invertido
                    f.rect(bx + 2, by + 2, tw + 2, h - 4, BLACK);
                    rich::draw_text(f, font, bx + 3, by + 2, text, false, WHITE);
                } else {
                    rich::draw_text(f, font, bx + 3, by + 2, text, false, BLACK);
                }
                if k == focus && !select_all {
                    f.rect(bx + 3 + tw, by + 2, 1, h - 4, BLACK);
                }
            },
        );
        for (b, r) in &lay.buttons {
            draw_button(f, a, *b, r.0, r.1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};

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

    #[test]
    fn fields_take_text_and_buttons_answer() {
        let (_d, a) = empty_assets();
        let mut b = TextBalloon::new(
            100,
            50,
            300,
            vec![
                Part::Text("Nome:".into()),
                Part::Field {
                    field: 0,
                    width: 144,
                },
                Part::Text("Sobrenome:".into()),
                Part::Field {
                    field: 1,
                    width: 144,
                },
            ],
            vec![Button::Back, Button::Stop, Button::Next],
        );
        assert_eq!(b.fields.len(), 2);
        assert_eq!(b.handle(&a, &Input::Text("Ana".into())), None);
        assert_eq!(b.fields[0], "Ana");
        let r1 = b.field_rect(&a, 1).unwrap();
        b.handle(&a, &click(r1.0 + 5, r1.1 + 5));
        assert_eq!(b.focus, 1);
        b.handle(&a, &Input::Text("Teste".into()));
        b.handle(&a, &key(Key::Backspace));
        assert_eq!(b.fields[1], "Test");
        b.handle(&a, &key(Key::Up));
        assert_eq!(b.focus, 0);
        assert_eq!(b.handle(&a, &key(Key::Enter)), Some(Button::Next));
        let rb = b.button_rect(&a, Button::Back).unwrap();
        assert_eq!(b.hit_button(&a, rb.0 + 3, rb.1 + 3), Some(Button::Back));
        assert_eq!(b.handle(&a, &click(rb.0 + 3, rb.1 + 3)), Some(Button::Back));
        // texto não cabe além da largura da caixa
        b.handle(&a, &Input::Text("x".repeat(80)));
        assert!(b.fields[0].len() < 40);
        assert!(b.contains(&a, 110, 60));
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        // a caixa do campo é branca por dentro
        let r0 = b.field_rect(&a, 0).unwrap();
        assert_eq!(
            f.pixel((r0.0 + r0.2 - 3) as u32, (r0.1 + 7) as u32)[..3],
            WHITE
        );
    }

    #[test]
    fn options_are_hit_and_selected_field_is_replaced() {
        let (_d, a) = empty_assets();
        let mut b = TextBalloon::new(
            64,
            76,
            344,
            vec![
                Part::Text("O que você quer fazer?".into()),
                Part::Options(vec!["um".into(), "dois".into()]),
            ],
            vec![Button::Back, Button::Stop],
        )
        .as_alert();
        assert_eq!(b.options(), ["um".to_string(), "dois".to_string()]);
        let r0 = b.option_rect(&a, 0).unwrap();
        let r1 = b.option_rect(&a, 1).unwrap();
        assert!(r1.1 > r0.1);
        assert_eq!(b.hit_option(&a, r0.0 + 5, r0.1 + 2), Some(0));
        assert_eq!(b.hit_option(&a, r1.0 + 5, r1.1 + 2), Some(1));
        assert_eq!(b.hit_option(&a, 0, 0), None);
        // os botões ficam abaixo das opções
        let rb = b.button_rect(&a, Button::Back).unwrap();
        assert!(rb.1 >= r1.1 + r1.3);
        assert_eq!(b.handle(&a, &click(rb.0 + 2, rb.1 + 2)), Some(Button::Back));
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        // aviso: fundo branco por dentro
        assert_eq!(f.pixel(70, 80)[..3], WHITE);

        // campo pré-selecionado: digitar substitui; Backspace apaga tudo
        let mut n = TextBalloon::new(
            112,
            78,
            259,
            vec![Part::Field {
                field: 0,
                width: 100,
            }],
            vec![Button::Next],
        );
        n.fields[0] = "Bia".into();
        n.select_all = true;
        n.handle(&a, &Input::Text("Zé".into()));
        assert_eq!(n.fields[0], "Zé");
        assert!(!n.select_all);
        n.select_all = true;
        n.handle(&a, &key(Key::Backspace));
        assert_eq!(n.fields[0], "");
        n.fields[0] = "Bia".into();
        n.select_all = true;
        // clicar no campo tira a seleção; o texto fica
        let r = n.field_rect(&a, 0).unwrap();
        n.handle(&a, &click(r.0 + 3, r.1 + 3));
        assert!(!n.select_all);
        n.handle(&a, &Input::Text("!".into()));
        assert_eq!(n.fields[0], "Bia!");
        n.select_all = true;
        n.draw(&mut f, &a);
    }

    #[test]
    fn grid_selects_by_click() {
        let (_d, a) = empty_assets();
        let mut b = TextBalloon::new(
            10,
            10,
            277,
            vec![Part::Grid {
                pictures: (0..12).map(|k| PIC_ANIMAL0 + 2 * k).collect(),
                cols: 3,
            }],
            vec![Button::Next],
        );
        let r = b.cell_rect(&a, 7).unwrap();
        b.handle(&a, &click(r.0 + 4, r.1 + 4));
        assert_eq!(b.selected, 7);
        assert!(b.cell_rect(&a, 12).is_none());
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
    }

    #[test]
    fn from_record_reads_name_and_animal_balloons() {
        let Some(a) = assets() else { return };
        let recs = a.dialogs(Program::Artist, 0x4400);
        let name = TextBalloon::from_record(&recs[4], 112, 78, "Ana", vec![Button::Next]);
        assert_eq!(name.fields.len(), 2);
        assert_eq!(name.w, 297);
        assert!(name.plain_text().starts_with("Preciso saber"));
        let (r0, r1) = (
            name.field_rect(&a, 0).unwrap(),
            name.field_rect(&a, 1).unwrap(),
        );
        assert_eq!((r0.2, r1.2), (144, 144));
        assert!(r1.1 > r0.1);
        let animal = TextBalloon::from_record(&recs[8], 130, 78, "Ana", vec![Button::Next]);
        assert!(animal.plain_text().starts_with("Oi, Ana!"));
        // 12 bichos em 3 colunas e 4 linhas
        let cells: Vec<Rect> = (0..12).map(|k| animal.cell_rect(&a, k).unwrap()).collect();
        assert!(animal.cell_rect(&a, 12).is_none());
        assert_eq!(cells[0].1, cells[2].1);
        assert!(cells[3].1 > cells[0].1 && cells[3].0 == cells[0].0);
        let bad = a.dialogs(Program::Artist, 0x4402);
        let dup = TextBalloon::from_record(&bad[0], 0, 0, "zé", vec![]);
        assert!(dup.has_alert_header());
        assert!(dup.plain_text().contains("(zé)"));
        let mut f = Frame::new(640, 480);
        animal.draw(&mut f, &a);
        dup.draw(&mut f, &a);
        name.draw(&mut f, &a);
        // o botão Segue é a figura 0x413 (verde)
        let r = name.button_rect(&a, Button::Next).unwrap();
        let mut green = 0;
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                let p = f.pixel(x as u32, y as u32);
                if p[1] > 120 && p[0] < 100 {
                    green += 1;
                }
            }
        }
        assert!(green > 20);
    }
}
