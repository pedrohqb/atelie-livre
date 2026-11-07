//! Cavalete de balões de diálogo (botão Oi! da Palavra; o "Comic easel" do
//! `WHAT` 1009 [5]–[8]): prévia do balão, campo de texto e três botões que
//! abrem listas — AA (fonte: normal ou pequena), forma do balão (as seis do
//! cavalete da história em quadrinhos, inclusive "sem balão") e posição das
//! palavras; NÃO e OK à direita. Medido em `shots/paint-text_orig_03_oi.png`.

use super::shaped_easel::Reply;
use super::ui::{self, Field, List, ListHit, Rect, inside};
use crate::Frame;
use crate::assets::Assets;
use crate::help::HelpBalloon;
use crate::paint::objects::words_stamp;
use crate::paint::stamp::Stamp;
use crate::paint::text::{Align, BalloonShape, WordsStyle, balloon_grid, text_size};
use crate::scene::{Input, Key, MouseButton};
use formats::fnt::Font;
use formats::palette::Rgb;

pub const FRAME: Rect = (164, 122, 311, 235);
pub const PREVIEW: Rect = (169, 127, 301, 124);
pub const FIELD: Rect = (169, 256, 301, 48);
const BUTTONS_PANEL: Rect = (169, 309, 209, 43);
const OK_PANEL: Rect = (383, 309, 87, 43);
pub const NO: Rect = (395, 315, 28, 28);
pub const OK: Rect = (431, 315, 26, 27);
/// Botões AA, forma do balão e posição (49×34), com as máscaras.
const BUTTONS: [(u16, u16, i32); 3] = [
    (0x140b, 0x140d, 197),
    (0x140f, 0x1411, 251),
    (0x1413, 0x1415, 305),
];
const BUTTON_Y: i32 = 314;
/// Registros do `WHAT` 1009 dos três botões (estilo, forma, posição).
pub const HELP_BUTTONS: [usize; 3] = [6, 7, 8];
/// Texto mais longo do balão.
const MAX_CHARS: usize = 120;
/// Largura de quebra dos balões novos com texto comprido.
pub const AUTO_WRAP: u32 = 200;

const FONTS: [&str; 2] = ["Normal", "Pequena"];
const ALIGN_TEXT: [&str; 3] = [
    "O texto vai ser alinhado à esquerda se você escolher este.",
    "O texto vai ser alinhado ao centro se você escolher este.",
    "O texto vai ser alinhado à direita se você escolher este.",
];

pub fn button_rect(k: usize) -> Rect {
    (BUTTONS[k].2, BUTTON_Y, 49, 34)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Font,
    Shape,
    Align,
}

/// Fontes do balão: a normal e a pequena.
pub struct Ctx<'a> {
    pub pal: &'a [Rgb],
    pub big: Option<&'a Font>,
    pub small: Option<&'a Font>,
}

impl Ctx<'_> {
    pub fn font(&self, style: &WordsStyle) -> Option<&Font> {
        if style.small {
            self.small.or(self.big)
        } else {
            self.big
        }
    }
}

pub struct BalloonEasel {
    pub field: Field,
    pub style: WordsStyle,
    pub color: u8,
    /// Balão do quadro sendo modificado (`None` = balão novo).
    pub editing: Option<u32>,
    menu: Option<(Menu, List)>,
    preview: Option<Stamp>,
    dirty: bool,
    pub help: Option<HelpBalloon>,
}

impl BalloonEasel {
    pub fn new(text: &str, style: WordsStyle, color: u8, editing: Option<u32>) -> BalloonEasel {
        BalloonEasel {
            field: Field::new(text, MAX_CHARS),
            style,
            color,
            editing,
            menu: None,
            preview: None,
            dirty: true,
            help: None,
        }
    }

    pub fn menu(&self) -> Option<Menu> {
        self.menu.map(|(m, _)| m)
    }

    /// Lista aberta (posição das casas).
    pub fn menu_list(&self) -> Option<&List> {
        self.menu.as_ref().map(|(_, l)| l)
    }

    /// Estilo final: um balão novo com texto comprido ganha largura de quebra.
    pub fn final_style(&self, ctx: &Ctx) -> WordsStyle {
        let mut st = self.style;
        if st.width == 0 && text_size(ctx.font(&st), &self.field.text).0 > AUTO_WRAP {
            st.width = AUTO_WRAP;
        }
        st
    }

    pub fn refresh(&mut self, ctx: &Ctx) {
        if self.dirty || self.preview.is_none() {
            let st = self.final_style(ctx);
            self.preview = Some(words_stamp(
                ctx.font(&st),
                &self.field.text,
                &st,
                self.color,
            ));
            self.dirty = false;
        }
    }

    fn list_for(&self, m: Menu, font: Option<&Font>) -> List {
        let k = m as usize;
        let r = button_rect(k);
        let cx = r.0 + r.2 / 2;
        match m {
            Menu::Font => List::above(cx, BUTTON_Y - 2, 120, 21, FONTS.len(), 4),
            Menu::Shape => List::above(cx, BUTTON_Y - 2, 60, 34, BalloonShape::ALL.len(), 4),
            Menu::Align => {
                let rows = ui::line_h(font) * 3 + 8;
                List::above(cx, BUTTON_Y - 2, 170, rows, 3, 4)
            }
        }
    }

    pub fn input(&mut self, i: &Input, a: &Assets, ctx: &Ctx) -> Reply {
        match *i {
            Input::Click {
                button: MouseButton::Left | MouseButton::Right,
                x,
                y,
            } => self.click(x, y, a),
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
            } => {
                match key {
                    // "não aperte Return nem Enter" (o texto quebra sozinho)
                    Key::Enter => {}
                    Key::Escape if self.menu.is_some() => self.menu = None,
                    Key::Escape => {
                        return Reply {
                            done: Some(false),
                            ..Reply::default()
                        };
                    }
                    k => {
                        if self.field.key(k) {
                            self.dirty = true;
                        }
                    }
                }
                let _ = ctx;
                Reply::default()
            }
            _ => Reply::default(),
        }
    }

    fn click(&mut self, x: i32, y: i32, a: &Assets) -> Reply {
        let mut r = Reply::default();
        if let Some(h) = self.help.take()
            && h.contains(a, x, y)
        {
            self.help = match h.link_at(a, x, y) {
                Some(t) => h.follow(t).or(Some(h)),
                None => Some(h),
            };
            return r;
        }
        if let Some((m, mut l)) = self.menu.take() {
            match l.hit(x, y) {
                Some(ListHit::Item(k)) => {
                    match m {
                        Menu::Font => self.style.small = k == 1,
                        Menu::Shape => self.style.shape = BalloonShape::ALL[k],
                        Menu::Align => self.style.align = Align::ALL[k],
                    }
                    self.dirty = true;
                }
                Some(ListHit::Up) | Some(ListHit::Down) => {
                    l.scroll_by(l.hit(x, y) == Some(ListHit::Down));
                    self.menu = Some((m, l));
                }
                Some(ListHit::Blank) => self.menu = Some((m, l)),
                None => {}
            }
            return r;
        }
        if inside(OK, x, y) {
            r.done = Some(true);
        } else if inside(NO, x, y) {
            r.done = Some(false);
        } else if let Some(k) = (0..3).find(|&k| inside(button_rect(k), x, y)) {
            let m = [Menu::Font, Menu::Shape, Menu::Align][k];
            self.menu = Some((m, self.list_for(m, a.font(8).as_deref())));
            r.sound = Some(super::shaped_easel::SND_MENU);
            r.help = Some(HELP_BUTTONS[k]);
        } else if inside(FIELD, x, y) {
            self.field.click();
        }
        r
    }

    pub fn tick(&mut self, dt: f32) {
        self.field.tick(dt);
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets, ctx: &Ctx) {
        ui::dim_outside(f);
        let (fx, fy, fw, fh) = FRAME;
        f.rect(PREVIEW.0, PREVIEW.1, PREVIEW.2, PREVIEW.3, ui::WHITE);
        for r in [BUTTONS_PANEL, OK_PANEL] {
            f.rect(r.0, r.1, r.2, r.3, ui::GRAY);
        }
        ui::bar_h(f, fx, fy, fw);
        ui::bar_h(f, fx, 251, fw);
        ui::bar_h(f, fx, 304, fw);
        ui::bar_h(f, fx, fy + fh - ui::BAR, fw);
        ui::bar_v(f, fx, fy, fh);
        ui::bar_v(f, fx + fw - ui::BAR, fy, fh);
        ui::bar_v(f, 378, 309, 43);
        if let Some(s) = &self.preview {
            draw_zoomed(f, s, PREVIEW, ctx.pal);
        }
        let ui_font = a.font(8);
        self.field.draw(f, ui_font.as_deref(), FIELD);
        for (k, (img, mask, _)) in BUTTONS.iter().enumerate() {
            let r = button_rect(k);
            ui::blit(f, a, (*img, *mask), r.0, r.1);
        }
        ui::blit(f, a, ui::BTN_NO, NO.0, NO.1);
        ui::blit(f, a, ui::BTN_OK, OK.0, OK.1);
        if let Some((m, l)) = &self.menu {
            let font = ui_font.as_deref();
            let st = &self.style;
            l.draw(f, a, |f, k, r| match m {
                Menu::Font => ui::menu_row(f, a, font, r, FONTS[k], st.small == (k == 1), false),
                Menu::Shape => {
                    let s = BalloonShape::ALL[k];
                    let g = balloon_grid(s, 24, 8);
                    let (x0, y0) = (r.0 + (r.2 - g.w) / 2, r.1 + (r.3 - g.h) / 2);
                    for y in 0..g.h {
                        for x in 0..g.w {
                            if let Some(edge) = g.at(x, y) {
                                f.put(x0 + x, y0 + y, if edge { ui::BLACK } else { ui::WHITE });
                            }
                        }
                    }
                    f.rect(x0 + g.pad_x + 2, y0 + g.pad_y + 3, 20, 2, ui::GRAY);
                    if s == st.shape {
                        f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, [250, 0, 57]);
                    }
                }
                Menu::Align => {
                    let lines = crate::balloon::wrap_opt(font, ALIGN_TEXT[k], (r.2 - 10) as u32);
                    let lh = ui::line_h(font);
                    for (j, line) in lines.iter().take(3).enumerate() {
                        ui::text(f, font, r.0 + 5, r.1 + 4 + j as i32 * lh, line, ui::BLACK);
                    }
                    if Align::ALL[k] == st.align {
                        f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, [250, 0, 57]);
                    }
                }
            });
        }
        if let Some(h) = &self.help {
            h.draw(f, a);
        }
    }
}

/// Desenha o carimbo centrado em `r`, ampliado 2× se couber (senão em
/// tamanho real, ou reduzido para caber).
fn draw_zoomed(f: &mut Frame, s: &Stamp, r: Rect, pal: &[Rgb]) {
    let fits2 = (s.w as i32) * 2 <= r.2 - 8 && (s.h as i32) * 2 <= r.3 - 8;
    if !fits2 {
        return super::shaped_easel::draw_fit(f, s, r, pal);
    }
    let (ox, oy) = (
        r.0 + (r.2 - 2 * s.w as i32) / 2,
        r.1 + (r.3 - 2 * s.h as i32) / 2,
    );
    for y in 0..s.h as i32 {
        for x in 0..s.w as i32 {
            if let Some(v) = s.px[(y * s.w as i32 + x) as usize] {
                let c = pal
                    .get(v as usize)
                    .copied()
                    .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                f.rect(ox + 2 * x, oy + 2 * y, 2, 2, [c.r, c.g, c.b]);
            }
        }
    }
}
