//! Balões com campos de texto, figuras para escolher e botões Volta / Para /
//! Segue: os do cadastro da Entrada (`WHAT 4400` e `4402` de `ARTIST.EXE`).
//! Complementa `Balloon` (texto e opções) sem alterá-lo.
//!
//! No `WHAT`, uma figura de id 0 é uma caixa de texto (o parâmetro é a
//! largura em pixels), `0x22e` marca uma opção com figura (a bolinha verde),
//! os ids `0x237`…`0x242` são os doze bichos (figuras `0x45b`, `0x45d`… de
//! `ARTIST.PIC`) e `0x230` é a cabeça do McZee dos avisos (`0x41b`).

use crate::assets::{Assets, ImageRef, Program};
use crate::balloon::{PAD, balloon_font, draw_box, draw_line, line_height, wrap_opt};
use crate::scene::{Input, Key, MouseButton};
use crate::{Frame, Image};
use formats::fnt::Font;
use formats::what::{Item, PIC_OPTION_PICTURE, Record};
use std::rc::Rc;

type Rect = (i32, i32, i32, i32);

const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];
/// Altura da caixa de texto e espaço que ela ocupa na coluna.
const FIELD_H: i32 = 15;
const FIELD_ROW: i32 = 20;
/// Altura de cada linha da grade de figuras.
const CELL_H: i32 = 16;
/// Botões Volta / Para / Segue (31×14, com a máscara duas imagens adiante).
const BTN_W: i32 = 31;
const BTN_H: i32 = 14;
const BTN_GAP: i32 = 10;
pub const PIC_BACK: u16 = 0x40f;
pub const PIC_STOP: u16 = 0x417;
pub const PIC_NEXT: u16 = 0x413;
/// Bolinha verde vazia e marcada (15×14).
pub const PIC_BALL: u16 = 0x409;
pub const PIC_BALL_CHECKED: u16 = 0x40d;
/// Primeiro bicho (19×11); os outros estão de 2 em 2.
pub const PIC_ANIMAL0: u16 = 0x45b;
/// Cabeça do McZee dos avisos (35×35).
pub const PIC_MCZEE_HEAD: u16 = 0x41b;
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
}

/// Figura do `.PIC` correspondente a um id de figura do `WHAT`.
pub fn pic_for(what_id: u16) -> Option<u16> {
    if (WHAT_ANIMAL0..WHAT_ANIMAL0 + WHAT_ANIMALS).contains(&what_id) {
        Some(PIC_ANIMAL0 + 2 * (what_id - WHAT_ANIMAL0))
    } else if what_id == WHAT_HEAD {
        Some(PIC_MCZEE_HEAD)
    } else {
        None
    }
}

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
}

struct Layout {
    fields: Vec<(usize, Rect)>,
    cells: Vec<Rect>,
    buttons: Vec<(Button, Rect)>,
    h: i32,
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

/// A imagem existe no `ARTIST.PIC` (senão vem o marcador de 32×32).
fn has_pic(a: &Assets, id: u16) -> bool {
    a.pic_ids(Program::Artist).binary_search(&id).is_ok()
}

/// Desenha uma figura colorida do `ARTIST.PIC`: com a máscara duas imagens
/// adiante quando ela tem o mesmo tamanho, senão tratando branco como
/// transparente. Sem a figura, não desenha nada.
pub fn blit_pic(f: &mut Frame, a: &Assets, id: u16, x: i32, y: i32, masked: bool) {
    if !has_pic(a, id) {
        return;
    }
    let img = a.image(&ImageRef::Pic(Program::Artist, id));
    let mask = if masked {
        a.pic_next_id(Program::Artist, id, 2)
            .map(|m| a.image(&ImageRef::Pic(Program::Artist, m)))
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
            .unwrap_or(0);
        TextBalloon {
            x,
            y,
            w: w.max(120),
            parts,
            fields: vec![String::new(); n],
            focus: 0,
            selected: 0,
            buttons,
        }
    }

    /// Balão a partir de um registro `WHAT`: texto (o `@` vira `name`),
    /// caixas de texto, grade de figuras e figuras decorativas, na ordem.
    pub fn from_record(
        r: &Record,
        x: i32,
        y: i32,
        name: &str,
        buttons: Vec<Button>,
    ) -> TextBalloon {
        let mut parts: Vec<Part> = Vec::new();
        let mut text = String::new();
        let mut fields = 0;
        let mut grid: Option<(Vec<u16>, usize, bool)> = None; // figuras, colunas, 1ª linha
        let flush = |text: &mut String, parts: &mut Vec<Part>| {
            let t = text.trim().to_string();
            text.clear();
            if !t.is_empty() {
                parts.push(Part::Text(t));
            }
        };
        let close_grid = |grid: &mut Option<(Vec<u16>, usize, bool)>, parts: &mut Vec<Part>| {
            if let Some((pictures, cols, _)) = grid.take() {
                parts.push(Part::Grid {
                    pictures,
                    cols: cols.max(1),
                });
            }
        };
        for i in &r.items {
            match i {
                Item::Text { text: t, .. } => {
                    if grid.is_none() || !t.trim().is_empty() {
                        if grid.is_some() {
                            close_grid(&mut grid, &mut parts);
                        }
                        text.push_str(t);
                    }
                }
                Item::Newline => {
                    if let Some(g) = grid.as_mut() {
                        g.2 = false;
                    } else {
                        text.push('\n');
                    }
                }
                Item::Picture { id, param } if *id == WHAT_FIELD => {
                    flush(&mut text, &mut parts);
                    parts.push(Part::Field {
                        field: fields,
                        width: (*param as i32).max(40),
                    });
                    fields += 1;
                }
                Item::Picture { id, .. } if *id == PIC_OPTION_PICTURE => {
                    if grid.is_none() {
                        flush(&mut text, &mut parts);
                        grid = Some((Vec::new(), 0, true));
                    }
                    if let Some(g) = grid.as_mut()
                        && g.2
                    {
                        g.1 += 1;
                    }
                }
                Item::Picture { id, .. } => {
                    let Some(pic) = pic_for(*id) else { continue };
                    match grid.as_mut() {
                        Some(g) => g.0.push(pic),
                        None => {
                            flush(&mut text, &mut parts);
                            parts.push(Part::Figure(pic));
                        }
                    }
                }
            }
        }
        close_grid(&mut grid, &mut parts);
        flush(&mut text, &mut parts);
        for p in &mut parts {
            if let Part::Text(t) = p {
                *t = t.replace('@', name);
            }
        }
        TextBalloon::new(x, y, r.width as i32, parts, buttons)
    }

    fn font(a: &Assets) -> Option<Rc<Font>> {
        balloon_font(a)
    }

    fn layout(&self, a: &Assets, font: Option<&Font>) -> Layout {
        let lh = line_height(font);
        let inner = self.w - 2 * PAD;
        let mut y = self.y + PAD;
        let mut lay = Layout {
            fields: Vec::new(),
            cells: Vec::new(),
            buttons: Vec::new(),
            h: 0,
        };
        for p in &self.parts {
            match p {
                Part::Text(t) => {
                    y += wrap_opt(font, t, inner as u32).len() as i32 * lh;
                }
                Part::Field { field, width } => {
                    let w = (*width).min(inner);
                    lay.fields.push((*field, (self.x + PAD, y + 2, w, FIELD_H)));
                    y += FIELD_ROW;
                }
                Part::Grid { pictures, cols } => {
                    let cw = inner / *cols as i32;
                    for (k, _) in pictures.iter().enumerate() {
                        let (col, row) = ((k % cols) as i32, (k / cols) as i32);
                        lay.cells
                            .push((self.x + PAD + col * cw, y + row * CELL_H, cw, CELL_H));
                    }
                    y += pictures.len().div_ceil(*cols) as i32 * CELL_H + 2;
                }
                Part::Figure(id) => {
                    let h = if has_pic(a, *id) {
                        a.image(&ImageRef::Pic(Program::Artist, *id)).height as i32
                    } else {
                        0
                    };
                    y += h + 2;
                }
            }
        }
        if !self.buttons.is_empty() {
            y += 4;
            let n = self.buttons.len() as i32;
            let total = n * BTN_W + (n - 1) * BTN_GAP;
            let mut bx = self.x + (self.w - total) / 2;
            for b in &self.buttons {
                lay.buttons.push((*b, (bx, y, BTN_W, BTN_H)));
                bx += BTN_W + BTN_GAP;
            }
            y += BTN_H;
        }
        lay.h = y + PAD - self.y;
        lay
    }

    pub fn height(&self, a: &Assets) -> i32 {
        self.layout(a, Self::font(a).as_deref()).h
    }

    pub fn contains(&self, a: &Assets, x: i32, y: i32) -> bool {
        inside((self.x, self.y, self.w, self.height(a)), x, y)
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

    /// Botão em (x, y), se houver.
    pub fn hit_button(&self, a: &Assets, x: i32, y: i32) -> Option<Button> {
        self.layout(a, Self::font(a).as_deref())
            .buttons
            .into_iter()
            .find(|(_, r)| inside(*r, x, y))
            .map(|(b, _)| b)
    }

    /// Largura máxima do texto que cabe no campo `k`.
    fn field_capacity(&self, k: usize) -> i32 {
        self.parts
            .iter()
            .find_map(|p| match p {
                Part::Field { field, width } if *field == k => Some(*width),
                _ => None,
            })
            .unwrap_or(100)
            - 6
    }

    fn text_width(font: Option<&Font>, s: &str) -> i32 {
        match font {
            Some(f) => f.width(s) as i32,
            None => 8 * s.chars().count() as i32,
        }
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
                }
                if let Some(k) = lay.cells.iter().position(|r| inside(*r, *x, *y)) {
                    self.selected = k;
                }
                None
            }
            Input::Text(s) => {
                if let Some(field) = self.fields.get(self.focus) {
                    let font = Self::font(a);
                    let cap = self.field_capacity(self.focus);
                    let mut t = field.clone();
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
                        f.pop();
                    }
                    None
                }
                Key::Enter => self.buttons.contains(&Button::Next).then_some(Button::Next),
                Key::Down => {
                    if !self.fields.is_empty() {
                        self.focus = (self.focus + 1) % self.fields.len();
                    }
                    None
                }
                Key::Up => {
                    if !self.fields.is_empty() {
                        self.focus = (self.focus + self.fields.len() - 1) % self.fields.len();
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
        let lay = self.layout(a, font.as_deref());
        let lh = line_height(font.as_deref());
        let inner = self.w - 2 * PAD;
        draw_box(f, self.x, self.y, self.w, lay.h);
        let mut y = self.y + PAD;
        for p in &self.parts {
            match p {
                Part::Text(t) => {
                    for line in wrap_opt(font.as_deref(), t, inner as u32) {
                        draw_line(f, font.as_deref(), self.x + PAD, y, &line);
                        y += lh;
                    }
                }
                Part::Field { field, width } => {
                    let w = (*width).min(inner);
                    let (bx, by) = (self.x + PAD, y + 2);
                    f.rect(bx, by, w, FIELD_H, BLACK);
                    f.rect(bx + 1, by + 1, w - 2, FIELD_H - 2, WHITE);
                    let text = self.fields.get(*field).map_or("", String::as_str);
                    draw_line(f, font.as_deref(), bx + 3, by + 2, text);
                    if *field == self.focus {
                        let cx = bx + 3 + Self::text_width(font.as_deref(), text);
                        f.rect(cx, by + 2, 1, FIELD_H - 4, BLACK);
                    }
                    y += FIELD_ROW;
                }
                Part::Grid { pictures, cols } => {
                    let cw = inner / *cols as i32;
                    for (k, pic) in pictures.iter().enumerate() {
                        let (col, row) = ((k % cols) as i32, (k / cols) as i32);
                        let (cx, cy) = (self.x + PAD + col * cw, y + row * CELL_H);
                        let ball = if k == self.selected {
                            PIC_BALL_CHECKED
                        } else {
                            PIC_BALL
                        };
                        if has_pic(a, ball) {
                            blit_pic(f, a, ball, cx, cy, false);
                        } else {
                            crate::balloon::draw_bullet(f, cx + 2, cy + 2);
                            if k == self.selected {
                                f.rect(cx + 5, cy + 5, 4, 4, WHITE);
                            }
                        }
                        if has_pic(a, *pic) {
                            blit_pic(f, a, *pic, cx + 20, cy + 2, false);
                        } else {
                            f.text(cx + 20, cy + 3, &format!("{}", k + 1), BLACK);
                        }
                    }
                    y += pictures.len().div_ceil(*cols) as i32 * CELL_H + 2;
                }
                Part::Figure(id) => {
                    if has_pic(a, *id) {
                        blit_pic(f, a, *id, self.x + PAD, y, true);
                        y += a.image(&ImageRef::Pic(Program::Artist, *id)).height as i32;
                    }
                    y += 2;
                }
            }
        }
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
        assert!(matches!(&name.parts[0], Part::Text(t) if t.starts_with("Preciso saber")));
        assert!(matches!(
            name.parts[1],
            Part::Field {
                field: 0,
                width: 144
            }
        ));
        let animal = TextBalloon::from_record(&recs[8], 130, 78, "Ana", vec![Button::Next]);
        assert!(matches!(&animal.parts[0], Part::Text(t) if t.starts_with("Oi, Ana!")));
        assert!(
            matches!(&animal.parts[1], Part::Grid { pictures, cols: 3 } if pictures.len() == 12)
        );
        let bad = a.dialogs(Program::Artist, 0x4402);
        let dup = TextBalloon::from_record(&bad[0], 0, 0, "zé", vec![]);
        assert_eq!(dup.parts[0], Part::Figure(PIC_MCZEE_HEAD));
        assert!(matches!(&dup.parts[1], Part::Text(t) if t.contains("(zé)")));
        let mut f = Frame::new(640, 480);
        name.draw(&mut f, &a);
        animal.draw(&mut f, &a);
        dup.draw(&mut f, &a);
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
