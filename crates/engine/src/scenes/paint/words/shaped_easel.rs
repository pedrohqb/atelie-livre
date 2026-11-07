//! Cavalete de palavras com forma (botão FORMA da Palavra): diálogo modal
//! medido no original (`shots/tools_o_text_forma.png`) — prévia grande em
//! cima, campo de texto, botões + e − (tamanho), giro (◀ ▶) e, embaixo, os
//! seis botões que abrem listas: ABC (tipos de letra), FORMA, AA (estilos),
//! cores, padrões e sombras; NÃO e OK à direita.

use super::ui::{self, Field, List, ListHit, Rect, inside};
use crate::Frame;
use crate::assets::Assets;
use crate::help::HelpBalloon;
use crate::paint::shaped::{
    self, Face, Inks, MAX_CHARS, Shadow, ShapedWords, WordShape, font_label, render,
};
use crate::paint::stamp::Stamp;
use crate::scene::{Input, Key, MouseButton};
use formats::fnt::Font;
use formats::palette::Rgb;

/// Moldura e painéis (x, y, w, h).
pub const FRAME: Rect = (88, 72, 463, 335);
pub const PREVIEW: Rect = (93, 77, 453, 219);
pub const FIELD: Rect = (93, 301, 272, 49);
const SIZE_PANEL: Rect = (370, 301, 75, 49);
const ROT_PANEL: Rect = (450, 301, 96, 49);
const BUTTONS_PANEL: Rect = (93, 355, 352, 47);
const OK_PANEL: Rect = (450, 355, 96, 47);
/// Botões + e − (31×14) e de giro (30×29).
pub const PLUS: Rect = (392, 308, 31, 14);
pub const MINUS: Rect = (392, 329, 31, 14);
pub const ROT_LEFT: Rect = (462, 311, 30, 29);
pub const ROT_RIGHT: Rect = (504, 311, 30, 29);
pub const NO: Rect = (464, 364, 28, 28);
pub const OK: Rect = (506, 365, 26, 27);
/// Botões de baixo (49×34), a cada 57 px.
const BUTTON_X0: i32 = 101;
const BUTTON_STEP: i32 = 57;
const BUTTON_Y: i32 = 361;
const BUTTON_MASK: u16 = 0x0277;
const BUTTON_PICS: [u16; 6] = [0x0275, 0x027d, 0x0279, 0x027b, 0x027f, 0x0281];

/// Sons: abrir uma lista; girar.
pub const SND_MENU: u16 = 0x0233;
pub const SND_ROTATE: u16 = 0x021c;
/// Registros do bloco `WHAT` 1009: balão de girar.
pub const HELP_ROTATE: usize = 3;

/// Giro de um clique, em graus; com o botão segurado, mais um passo a cada `SPIN_STEP` s.
const CLICK_DEG: i32 = 5;
const SPIN_DELAY: f32 = 0.4;
const SPIN_STEP: f32 = 0.05;

/// Rótulos do menu AA (estilos).
const STYLES: [&str; 6] = [
    "Normal",
    "Negrito",
    "Inclinado",
    "Contorno",
    "MAIÚSCULAS",
    "Vertical",
];

/// Botão de baixo `k` (ABC, FORMA, AA, cores, padrões, sombras).
pub fn button_rect(k: usize) -> Rect {
    (BUTTON_X0 + BUTTON_STEP * k as i32, BUTTON_Y, 49, 34)
}

/// Lista aberta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Fonts,
    Shapes,
    Styles,
    Colors,
    Patterns,
    Shadows,
}

impl Menu {
    const ALL: [Menu; 6] = [
        Menu::Fonts,
        Menu::Shapes,
        Menu::Styles,
        Menu::Colors,
        Menu::Patterns,
        Menu::Shadows,
    ];
}

/// O que a pintura precisa para desenhar a palavra.
pub struct Ctx<'a> {
    pub pal: &'a [Rgb],
    pub patterns: &'a [[u8; 8]],
    /// Fonte do programa (Pablo Graphos 16), para a família "do programa".
    pub pablo: Option<&'a Font>,
    /// Fonte pequena dos menus e do campo.
    pub ui_font: Option<&'a Font>,
    pub inks: Inks,
    /// Conjuntos da lista de cores (um índice, ou dois/três alternados).
    pub color_sets: &'a [Vec<u8>],
}

/// Resposta a uma entrada.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Reply {
    /// `Some(true)` = OK, `Some(false)` = NÃO.
    pub done: Option<bool>,
    pub sound: Option<u16>,
    /// Registro do `WHAT` 1009 a mostrar.
    pub help: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct Spin {
    left: bool,
    held: f32,
    acc: f32,
}

pub struct ShapedEasel {
    pub words: ShapedWords,
    pub field: Field,
    /// Palavra do quadro sendo modificada (`None` = palavra nova).
    pub editing: Option<u32>,
    menu: Option<(Menu, List)>,
    spin: Option<Spin>,
    preview: Option<Stamp>,
    dirty: bool,
    pub help: Option<HelpBalloon>,
    fonts: Vec<String>,
}

/// Imagem da palavra no estado do cavalete.
pub fn stamp_of(w: &ShapedWords, ctx: &Ctx) -> Stamp {
    let loaded = shaped::load_face(&w.font);
    let face = match &loaded {
        Some(f) => Face::Outline(f),
        None => Face::Bitmap(ctx.pablo),
    };
    let inks = Inks {
        pattern: ctx
            .patterns
            .get(w.pattern as usize)
            .copied()
            .unwrap_or([0; 8]),
        ..ctx.inks
    };
    render(w, face, &inks)
}

impl ShapedEasel {
    pub fn new(words: ShapedWords, editing: Option<u32>) -> ShapedEasel {
        ShapedEasel {
            field: Field::new(&words.text, MAX_CHARS),
            words,
            editing,
            menu: None,
            spin: None,
            preview: None,
            dirty: true,
            help: None,
            fonts: shaped::font_names(),
        }
    }

    pub fn menu(&self) -> Option<Menu> {
        self.menu.map(|(m, _)| m)
    }

    /// Lista aberta (posição das casas).
    pub fn menu_list(&self) -> Option<&List> {
        self.menu.as_ref().map(|(_, l)| l)
    }

    /// Imagem atual (refeita se algo mudou).
    pub fn refresh(&mut self, ctx: &Ctx) {
        if self.dirty || self.preview.is_none() {
            self.words.text = self.field.text.clone();
            self.preview = Some(stamp_of(&self.words, ctx));
            self.dirty = false;
        }
    }

    fn list_for(&self, m: Menu, ctx: &Ctx) -> List {
        let k = Menu::ALL.iter().position(|x| *x == m).unwrap_or(0);
        let r = button_rect(k);
        let cx = r.0 + r.2 / 2;
        let (w, row_h, n) = match m {
            Menu::Fonts => (190, 18, self.fonts.len()),
            Menu::Shapes => (72, 46, WordShape::ALL.len()),
            Menu::Styles => (150, 21, STYLES.len()),
            Menu::Colors => (68, 22, ctx.color_sets.len()),
            Menu::Patterns => (62, 22, ctx.patterns.len()),
            Menu::Shadows => (170, 21, Shadow::ALL.len()),
        };
        let mut l = List::above(cx, BUTTON_Y - 2, w, row_h, n, 4);
        let current = match m {
            Menu::Fonts => self.fonts.iter().position(|f| *f == self.words.font),
            Menu::Shapes => WordShape::ALL.iter().position(|s| *s == self.words.shape),
            Menu::Colors => ctx.color_sets.iter().position(|c| *c == self.words.colors),
            Menu::Patterns => Some(self.words.pattern as usize),
            Menu::Shadows => Shadow::ALL.iter().position(|s| *s == self.words.shadow),
            Menu::Styles => None,
        };
        if let Some(c) = current {
            l.show(c);
        }
        l
    }

    fn pick(&mut self, m: Menu, k: usize, ctx: &Ctx) {
        let w = &mut self.words;
        match m {
            Menu::Fonts => {
                if let Some(f) = self.fonts.get(k) {
                    w.font = f.clone();
                }
            }
            Menu::Shapes => w.shape = WordShape::ALL[k.min(WordShape::ALL.len() - 1)],
            Menu::Styles => match k {
                0 => w.set_styles(""),
                1 => w.bold = !w.bold,
                2 => w.italic = !w.italic,
                3 => w.outline = !w.outline,
                4 => w.caps = !w.caps,
                _ => w.vertical = !w.vertical,
            },
            Menu::Colors => {
                if let Some(c) = ctx.color_sets.get(k) {
                    w.colors = c.clone();
                }
            }
            Menu::Patterns => w.pattern = k.min(255) as u8,
            Menu::Shadows => w.shadow = Shadow::ALL[k.min(Shadow::ALL.len() - 1)],
        }
        self.dirty = true;
    }

    /// Uma entrada com o cavalete aberto (todas ficam com ele).
    pub fn input(&mut self, i: &Input, a: &Assets, ctx: &Ctx) -> Reply {
        match *i {
            Input::Click {
                button: MouseButton::Left | MouseButton::Right,
                x,
                y,
            } => self.click(x, y, a, ctx),
            Input::MouseUp { .. } => {
                self.spin = None;
                Reply::default()
            }
            Input::Text(ref t) => {
                if self.field.type_text(t) {
                    self.dirty = true;
                }
                Reply::default()
            }
            Input::Key {
                key,
                ctrl: false,
                alt: false,
            } => match key {
                Key::Enter => Reply {
                    done: Some(true),
                    ..Reply::default()
                },
                Key::Escape if self.menu.is_some() => {
                    self.menu = None;
                    Reply::default()
                }
                Key::Escape => Reply {
                    done: Some(false),
                    ..Reply::default()
                },
                k => {
                    if self.field.key(k) {
                        self.dirty = true;
                    }
                    Reply::default()
                }
            },
            _ => Reply::default(),
        }
    }

    fn click(&mut self, x: i32, y: i32, a: &Assets, ctx: &Ctx) -> Reply {
        let mut r = Reply::default();
        if let Some(h) = self.help.take() {
            if h.contains(a, x, y) {
                self.help = match h.link_at(a, x, y) {
                    Some(t) => h.follow(t).or(Some(h)),
                    None => Some(h),
                };
                return r;
            }
        }
        if let Some((m, mut l)) = self.menu.take() {
            match l.hit(x, y) {
                Some(ListHit::Item(k)) => self.pick(m, k, ctx),
                Some(ListHit::Up) => {
                    l.scroll_by(false);
                    self.menu = Some((m, l));
                }
                Some(ListHit::Down) => {
                    l.scroll_by(true);
                    self.menu = Some((m, l));
                }
                Some(ListHit::Blank) => self.menu = Some((m, l)),
                // clique fora fecha a lista
                None => {}
            }
            return r;
        }
        if inside(OK, x, y) {
            r.done = Some(true);
        } else if inside(NO, x, y) {
            r.done = Some(false);
        } else if inside(PLUS, x, y) || inside(MINUS, x, y) {
            if self.words.step_size(inside(PLUS, x, y)) {
                self.dirty = true;
            }
        } else if inside(ROT_LEFT, x, y) || inside(ROT_RIGHT, x, y) {
            let left = inside(ROT_LEFT, x, y);
            self.words.rotate(if left { CLICK_DEG } else { -CLICK_DEG });
            self.spin = Some(Spin {
                left,
                held: 0.0,
                acc: 0.0,
            });
            self.dirty = true;
            r.sound = Some(SND_ROTATE);
        } else if let Some(k) = (0..6).find(|&k| inside(button_rect(k), x, y)) {
            let m = Menu::ALL[k];
            self.menu = Some((m, self.list_for(m, ctx)));
            r.sound = Some(SND_MENU);
        } else if inside(FIELD, x, y) {
            self.field.click();
        }
        r
    }

    /// Passagem do tempo: cursor e giro com o botão segurado.
    pub fn tick(&mut self, dt: f32) {
        self.field.tick(dt);
        let Some(s) = self.spin.as_mut() else {
            return;
        };
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        let before = s.held;
        s.held += dt;
        if s.held < SPIN_DELAY {
            return;
        }
        // só conta o tempo depois da espera
        s.acc += s.held - before.max(SPIN_DELAY);
        let left = s.left;
        while let Some(s) = self.spin.as_mut()
            && s.acc >= SPIN_STEP
        {
            s.acc -= SPIN_STEP;
            self.words.rotate(if left { CLICK_DEG } else { -CLICK_DEG });
            self.dirty = true;
        }
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets, ctx: &Ctx) {
        ui::dim_outside(f);
        let (fx, fy, fw, fh) = FRAME;
        for (r, color) in [
            (PREVIEW, ui::WHITE),
            (SIZE_PANEL, ui::GRAY),
            (ROT_PANEL, ui::GRAY),
            (BUTTONS_PANEL, ui::GRAY),
            (OK_PANEL, ui::GRAY),
        ] {
            f.rect(r.0, r.1, r.2, r.3, color);
        }
        ui::bar_h(f, fx, fy, fw);
        ui::bar_h(f, fx, 296, fw);
        ui::bar_h(f, fx, 350, fw);
        ui::bar_h(f, fx, fy + fh - ui::BAR, fw);
        ui::bar_v(f, fx, fy, fh);
        ui::bar_v(f, fx + fw - ui::BAR, fy, fh);
        ui::bar_v(f, 365, 301, 49);
        ui::bar_v(f, 445, 301, 101);
        if let Some(s) = &self.preview {
            draw_fit(f, s, PREVIEW, ctx.pal);
        }
        self.field.draw(f, ctx.ui_font, FIELD);
        ui::blit(f, a, (0x028d, 0x028f), PLUS.0, PLUS.1);
        ui::blit(f, a, (0x0289, 0x028b), MINUS.0, MINUS.1);
        ui::blit(f, a, (0x0283, 0x0285), ROT_LEFT.0, ROT_LEFT.1);
        ui::blit(f, a, (0x0287, 0x0285), ROT_RIGHT.0, ROT_RIGHT.1);
        for (k, id) in BUTTON_PICS.iter().enumerate() {
            let r = button_rect(k);
            ui::blit(f, a, (*id, BUTTON_MASK), r.0, r.1);
        }
        ui::blit(f, a, ui::BTN_NO, NO.0, NO.1);
        ui::blit(f, a, ui::BTN_OK, OK.0, OK.1);
        if let Some((m, l)) = &self.menu {
            self.draw_menu(f, a, ctx, *m, l);
        }
        if let Some(h) = &self.help {
            h.draw(f, a);
        }
    }

    fn draw_menu(&self, f: &mut Frame, a: &Assets, ctx: &Ctx, m: Menu, l: &List) {
        let rgb = |i: u8| {
            let c = ctx
                .pal
                .get(i as usize)
                .copied()
                .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
            [c.r, c.g, c.b]
        };
        let font = ctx.ui_font;
        let w = &self.words;
        l.draw(f, a, |f, k, r| match m {
            Menu::Fonts => {
                let name = self.fonts.get(k).map(String::as_str).unwrap_or("");
                ui::menu_row(f, a, font, r, font_label(name), *name == w.font, false);
            }
            Menu::Styles => {
                let on = match k {
                    0 => w.styles().is_empty(),
                    1 => w.bold,
                    2 => w.italic,
                    3 => w.outline,
                    4 => w.caps,
                    _ => w.vertical,
                };
                ui::menu_row(f, a, font, r, STYLES[k], on, k == 1);
            }
            Menu::Shadows => {
                let s = Shadow::ALL[k];
                ui::menu_row(f, a, font, r, s.label(), s == w.shadow, false);
            }
            Menu::Shapes => {
                let s = WordShape::ALL[k];
                draw_shape_icon(f, s, (r.0 + 6, r.1 + 6, r.2 - 12, r.3 - 12));
                if s == w.shape {
                    f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, [250, 0, 57]);
                }
            }
            Menu::Colors => {
                let set = &ctx.color_sets[k];
                let inner = (r.0 + 4, r.1 + 3, r.2 - 8, r.3 - 7);
                f.rect(
                    inner.0 - 1,
                    inner.1 - 1,
                    inner.2 + 2,
                    inner.3 + 2,
                    ui::BLACK,
                );
                let n = set.len().max(1) as i32;
                for (j, &c) in set.iter().enumerate() {
                    let x0 = inner.0 + inner.2 * j as i32 / n;
                    let x1 = inner.0 + inner.2 * (j as i32 + 1) / n;
                    f.rect(x0, inner.1, x1 - x0, inner.3, rgb(c));
                }
                if *set == w.colors {
                    f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, [250, 0, 57]);
                }
            }
            Menu::Patterns => {
                let p = ctx.patterns[k];
                let inner = (r.0 + 4, r.1 + 3, r.2 - 8, r.3 - 7);
                f.rect(
                    inner.0 - 1,
                    inner.1 - 1,
                    inner.2 + 2,
                    inner.3 + 2,
                    ui::BLACK,
                );
                for y in 0..inner.3 {
                    for x in 0..inner.2 {
                        let bit = (p[(y % 8) as usize] >> (7 - x % 8)) & 1;
                        f.put(
                            inner.0 + x,
                            inner.1 + y,
                            if bit == 1 { ui::WHITE } else { ui::BLACK },
                        );
                    }
                }
                if k == w.pattern as usize {
                    f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, [250, 0, 57]);
                }
            }
        });
    }
}

/// Silhueta de uma forma (preta) dentro de `r`, como os ícones da lista FORMA.
pub fn draw_shape_icon(f: &mut Frame, s: WordShape, r: Rect) {
    let (x0, y0, w, h) = r;
    match s {
        WordShape::ArcUp | WordShape::Circle => {
            let full = s == WordShape::Circle;
            let rad = if full {
                h as f32 / 2.0
            } else {
                (w as f32 / 2.0).min(h as f32)
            };
            let (cx, cy) = (
                x0 as f32 + w as f32 / 2.0,
                if full {
                    y0 as f32 + h as f32 / 2.0
                } else {
                    (y0 + h) as f32
                },
            );
            let thick = rad * 0.45;
            for y in y0..y0 + h {
                for x in x0..x0 + w {
                    let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                    let d = (dx * dx + dy * dy).sqrt();
                    if d <= rad && d >= rad - thick && (full || dy <= 0.0) {
                        f.put(x, y, ui::BLACK);
                    }
                }
            }
        }
        _ => {
            for x in 0..w {
                let u = (x as f32 + 0.5) / w as f32;
                let (t, b) = s.envelope(u);
                let (t, b) = ((t * h as f32).round() as i32, (b * h as f32).round() as i32);
                f.rect(x0 + x, y0 + t, 1, b - t, ui::BLACK);
            }
        }
    }
}

/// Desenha o carimbo dentro de `r`: em tamanho real se couber; senão
/// reduzido (vizinho mais próximo) para caber, centrado.
pub fn draw_fit(f: &mut Frame, s: &Stamp, r: Rect, pal: &[Rgb]) {
    let (bw, bh) = (r.2 - 8, r.3 - 8);
    let scale = (bw as f32 / s.w as f32)
        .min(bh as f32 / s.h as f32)
        .min(1.0);
    let (dw, dh) = (
        ((s.w as f32 * scale) as i32).max(1),
        ((s.h as f32 * scale) as i32).max(1),
    );
    let (ox, oy) = (r.0 + (r.2 - dw) / 2, r.1 + (r.3 - dh) / 2);
    for y in 0..dh {
        let sy = ((y as f32 + 0.5) / scale) as u32;
        for x in 0..dw {
            let sx = ((x as f32 + 0.5) / scale) as u32;
            if sx >= s.w || sy >= s.h {
                continue;
            }
            if let Some(v) = s.px[(sy * s.w + sx) as usize] {
                let c = pal
                    .get(v as usize)
                    .copied()
                    .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                f.put(ox + x, oy + y, [c.r, c.g, c.b]);
            }
        }
    }
}
