//! Cavalete de lápis de cera do Creative Writer (`ideas.md` §4,
//! `office-ferramentas.md` §1; capturas `ideas_33`/`ideas_34`,
//! `office_54`): janela de moldura laranja (115,100)–(524,379) com a área
//! branca de 400×200 px, barra com lápis, aspirador e ovo, setas de parte e
//! NÃO/OK, e a paleta das 16 cores embaixo. O desenho é de 600×350 (o
//! tamanho que o original grava para uma figura nova); as setas mostram a
//! outra parte. Usado pelo Quadro de Figuras (POE e "modificar a figura")
//! e, mais adiante, pelos adesivos do Escritório (W10).
//!
//! Comparação com o cavalete de adesivos do Fine Artist
//! (`scenes/paint/sticker_easel.rs`): aquele é um conjunto de métodos da
//! tela de pintura (`Paint`), preso à barra e à paleta de 256 cores dela,
//! com lupa e pincéis da pintura; este é um componente independente, com
//! outra geometria, a paleta de 16 cores e só o lápis. O que é comum — o
//! quadro indexado (`paint::Canvas`), o desfazer de um nível
//! (`paint::History`) e o traço (`paint::canvas::line_points`) — é
//! reaproveitado daqui; a paleta do quadro é a das 16 cores mais as cores
//! da figura que veio para ser modificada.

use crate::Frame;
use crate::Image;
use crate::assets::{Assets, Program};
use crate::balloon::Balloon;
use crate::paint::canvas::line_points;
use crate::paint::{Canvas, History, Objects};
use crate::scene::{Effect, Input, MouseButton};
use crate::scenes::writer_ideas::buttons::Art;
use crate::scenes::writer_ideas::{play, snd};
use crate::ui::{Rect, inside};

/// As 16 cores do lápis (as mesmas do Seleciona-cores do Escritório,
/// `office-palavra.md` §7), na ordem da paleta.
pub const COLORS: [[u8; 3]; 16] = [
    [255, 255, 255],
    [255, 238, 76],
    [255, 140, 50],
    [185, 101, 36],
    [250, 74, 114],
    [250, 0, 57],
    [136, 136, 230],
    [97, 65, 219],
    [208, 22, 222],
    [147, 44, 163],
    [92, 203, 83],
    [71, 140, 80],
    [190, 190, 190],
    [140, 140, 140],
    [100, 100, 100],
    [0, 0, 0],
];
/// Cor do lápis ao abrir: preto.
const START_COLOR: usize = 15;
/// Tamanho de uma figura nova.
pub const NEW_W: u32 = 600;
pub const NEW_H: u32 = 350;
/// Moldura (borda preta de fora) e área de desenho.
pub const WINDOW: Rect = (115, 100, 410, 280);
pub const AREA: Rect = (120, 105, 400, 200);
/// Barra cinza e divisões.
const BAR: Rect = (120, 310, 400, 40);
const SEPARATORS: [i32; 2] = [386, 432];
/// Paleta: 16 casas de 25 px a partir de x 120; amostra de 20×14.
const SWATCH_Y: i32 = 358;
const SWATCH_X0: i32 = 120;
const SWATCH_STEP: i32 = 25;
const ORANGE: [u8; 3] = [255, 140, 50];
const BROWN: [u8; 3] = [185, 101, 36];
const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const GRAY: [u8; 3] = [100, 100, 100];
/// Espessura do traço do lápis.
const THICK: i32 = 4;
/// Passo das setas, em pixels do desenho.
const PAN_STEP: i32 = 40;
/// Balão "Este é o Cavalete de lápis de cera" (`WHAT 0fa1[6]`).
const HELP: (u16, usize, (i32, i32)) = (0x0fa1, 6, (163, 148));

/// Botões da barra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Crayon,
    Vacuum,
    Egg,
    Up,
    Down,
    Left,
    Right,
    No,
    Ok,
}

const BUTTONS: [(Tool, Art, (i32, i32), Rect); 9] = [
    (
        Tool::Crayon,
        Art::masked(0x095b, 0x0bb9),
        (163, 316),
        (140, 310, 70, 40),
    ),
    (
        Tool::Vacuum,
        Art::masked(0x0bbb, 0x0bc1),
        (235, 317),
        (215, 310, 75, 40),
    ),
    (
        Tool::Egg,
        Art::masked(0x0295, 0x0297),
        (313, 314),
        (295, 310, 70, 40),
    ),
    (Tool::Up, Art::keyed(0x02db), (405, 313), (403, 311, 16, 14)),
    (
        Tool::Down,
        Art::keyed(0x02df),
        (405, 335),
        (403, 333, 16, 14),
    ),
    (
        Tool::Left,
        Art::keyed(0x0819),
        (393, 324),
        (391, 322, 14, 16),
    ),
    (
        Tool::Right,
        Art::keyed(0x0871),
        (417, 324),
        (417, 322, 14, 16),
    ),
    (
        Tool::No,
        Art::masked(0x0271, 0x0273),
        (446, 316),
        (440, 311, 38, 38),
    ),
    (
        Tool::Ok,
        Art::masked(0x026d, 0x026f),
        (484, 317),
        (479, 311, 38, 38),
    ),
];

/// Como o cavalete fechou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EaselOutcome {
    /// OK, com o desenho.
    Ok(Image),
    /// NÃO: nada muda.
    No,
}

pub struct CrayonEasel {
    canvas: Canvas,
    /// Cor de cada índice do quadro: as 16 do lápis e as da figura.
    pal: Vec<[u8; 3]>,
    history: History,
    color: usize,
    /// Canto do trecho visível (coordenadas do desenho).
    view: (i32, i32),
    /// Último ponto do traço em andamento.
    last: Option<(i32, i32)>,
    help: Option<Balloon>,
}

impl CrayonEasel {
    /// Cavalete vazio (POE do Quadro): 600×350 em branco.
    pub fn blank(a: &Assets) -> CrayonEasel {
        CrayonEasel {
            canvas: Canvas::new(NEW_W, NEW_H),
            pal: COLORS.to_vec(),
            history: History::default(),
            color: START_COLOR,
            view: (0, 0),
            last: None,
            help: help(a),
        }
    }

    /// Cavalete com uma figura para modificar (no tamanho dela).
    pub fn with_image(img: &Image, a: &Assets) -> CrayonEasel {
        let mut e = CrayonEasel::blank(a);
        let (w, h) = (img.width.max(1), img.height.max(1));
        e.canvas = Canvas::new(w, h);
        for y in 0..img.height {
            for x in 0..img.width {
                let i = ((y * img.width + x) * 4) as usize;
                let p = &img.rgba[i..i + 4];
                let rgb = if p[3] == 0 { WHITE } else { [p[0], p[1], p[2]] };
                let k = e.index_of(rgb);
                e.canvas.set(x as i32, y as i32, k);
            }
        }
        e
    }

    /// Índice da cor na paleta do quadro (acrescenta, ou a mais próxima se
    /// a paleta encheu).
    fn index_of(&mut self, rgb: [u8; 3]) -> u8 {
        if let Some(k) = self.pal.iter().position(|c| *c == rgb) {
            return k as u8;
        }
        if self.pal.len() < 256 {
            self.pal.push(rgb);
            return (self.pal.len() - 1) as u8;
        }
        let d = |c: &[u8; 3]| {
            (0..3)
                .map(|i| (c[i] as i32 - rgb[i] as i32).pow(2))
                .sum::<i32>()
        };
        self.pal
            .iter()
            .enumerate()
            .min_by_key(|(_, c)| d(c))
            .map_or(0, |(k, _)| k as u8)
    }

    pub fn color(&self) -> usize {
        self.color
    }

    pub fn view(&self) -> (i32, i32) {
        self.view
    }

    pub fn size(&self) -> (u32, u32) {
        (self.canvas.w, self.canvas.h)
    }

    /// O desenho como imagem RGBA (opaca).
    pub fn image(&self) -> Image {
        let mut rgba = Vec::with_capacity(self.canvas.px.len() * 4);
        for &k in &self.canvas.px {
            let c = self.pal.get(k as usize).copied().unwrap_or(WHITE);
            rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
        Image {
            width: self.canvas.w,
            height: self.canvas.h,
            rgba,
        }
    }

    fn to_canvas(&self, x: i32, y: i32) -> (i32, i32) {
        (x - AREA.0 + self.view.0, y - AREA.1 + self.view.1)
    }

    fn pan(&mut self, dx: i32, dy: i32) {
        let max_x = (self.canvas.w as i32 - AREA.2).max(0);
        let max_y = (self.canvas.h as i32 - AREA.3).max(0);
        self.view = (
            (self.view.0 + dx * PAN_STEP).clamp(0, max_x),
            (self.view.1 + dy * PAN_STEP).clamp(0, max_y),
        );
    }

    fn plot(&mut self, (x, y): (i32, i32)) {
        let c = self.color as u8;
        let r = THICK / 2;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r + 1 {
                    self.canvas.set(x + dx, y + dy, c);
                }
            }
        }
    }

    fn stroke_to(&mut self, p: (i32, i32)) {
        let from = self.last.unwrap_or(p);
        for q in line_points(from, p) {
            self.plot(q);
        }
        self.last = Some(p);
    }

    /// Trata uma entrada; devolve como fechou, se fechou.
    pub fn handle(&mut self, i: &Input, fx: &mut Vec<Effect>, _a: &Assets) -> Option<EaselOutcome> {
        match i {
            Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } => {
                let (x, y) = (*x, *y);
                if self.help.take().is_some() {
                    return None;
                }
                if inside(AREA, x, y) {
                    self.history.record(&self.canvas);
                    let p = self.to_canvas(x, y);
                    self.last = None;
                    self.stroke_to(p);
                    return None;
                }
                if let Some(k) = (0..COLORS.len()).find(|&k| inside(swatch(k), x, y)) {
                    self.color = k;
                    return None;
                }
                let tool = BUTTONS.iter().find(|b| inside(b.3, x, y)).map(|b| b.0)?;
                self.act(tool, fx)
            }
            Input::MouseMove { x, y } if self.last.is_some() => {
                let p = self.to_canvas(*x, *y);
                self.stroke_to(p);
                None
            }
            Input::MouseUp { x, y, .. } if self.last.is_some() => {
                let p = self.to_canvas(*x, *y);
                self.stroke_to(p);
                self.last = None;
                None
            }
            _ => None,
        }
    }

    fn act(&mut self, tool: Tool, fx: &mut Vec<Effect>) -> Option<EaselOutcome> {
        match tool {
            Tool::Crayon => {}
            Tool::Vacuum => {
                self.history.record(&self.canvas);
                self.canvas.clear();
            }
            Tool::Egg => {
                self.history
                    .toggle(&mut self.canvas, &mut Objects::default());
            }
            Tool::Up => self.pan(0, -1),
            Tool::Down => self.pan(0, 1),
            Tool::Left => self.pan(-1, 0),
            Tool::Right => self.pan(1, 0),
            Tool::No => {
                fx.push(play(snd::NO));
                return Some(EaselOutcome::No);
            }
            Tool::Ok => {
                fx.push(play(snd::DONE));
                return Some(EaselOutcome::Ok(self.image()));
            }
        }
        None
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let (x, y, w, h) = WINDOW;
        f.rect(x, y, w, h, BLACK);
        band(f, (x + 1, y + 1, w - 2, h - 2));
        // área de desenho
        f.rect(AREA.0 - 1, AREA.1 - 1, AREA.2 + 2, AREA.3 + 2, BLACK);
        f.rect(AREA.0, AREA.1, AREA.2, AREA.3, [190, 190, 190]);
        for sy in 0..AREA.3 {
            for sx in 0..AREA.2 {
                let (cx, cy) = (sx + self.view.0, sy + self.view.1);
                if let Some(k) = self.canvas.get(cx, cy) {
                    let c = self.pal.get(k as usize).copied().unwrap_or(WHITE);
                    f.put(AREA.0 + sx, AREA.1 + sy, c);
                }
            }
        }
        // faixa laranja entre a área e a barra, barra e divisões
        f.rect(AREA.0 - 1, 305, AREA.2 + 2, 5, BLACK);
        band_h(f, AREA.0, 306, AREA.2);
        f.rect(BAR.0, BAR.1, BAR.2, BAR.3, GRAY);
        for sx in SEPARATORS {
            f.rect(sx, BAR.1, 5, BAR.3, BLACK);
            f.rect(sx + 1, BAR.1, 1, BAR.3, BROWN);
            f.rect(sx + 2, BAR.1, 2, BAR.3, ORANGE);
        }
        f.rect(AREA.0 - 1, 350, AREA.2 + 2, 5, BLACK);
        band_h(f, AREA.0, 351, AREA.2);
        // paleta
        f.rect(AREA.0, 355, AREA.2, 20, WHITE);
        for (k, c) in COLORS.iter().enumerate() {
            let r = swatch(k);
            if k == self.color {
                f.rect(r.0 - 4, r.1 - 4, r.2 + 8, r.3 + 8, BLACK);
            } else {
                f.rect(r.0 - 1, r.1 - 1, r.2 + 2, r.3 + 2, BLACK);
            }
            f.rect(r.0, r.1, r.2, r.3, *c);
        }
        for (_, art, (bx, by), _) in BUTTONS {
            art.draw(f, a, bx, by);
        }
        if let Some(b) = &self.help {
            b.draw(f, a);
        }
    }
}

/// Casa da cor `k` na paleta.
fn swatch(k: usize) -> Rect {
    (SWATCH_X0 + 3 + SWATCH_STEP * k as i32, SWATCH_Y, 20, 14)
}

/// Faixa laranja da moldura (3 px, com a sombra marrom) em volta de `r`.
fn band(f: &mut Frame, r: Rect) {
    let (x, y, w, h) = r;
    f.rect(x, y, w, h, ORANGE);
    f.rect(x, y + 2, 1, h - 2, BROWN);
    f.rect(x + 2, y + 2, w - 4, 1, BROWN);
    f.rect(x + 3, y + 3, w - 6, h - 6, BLACK);
}

fn band_h(f: &mut Frame, x: i32, y: i32, w: i32) {
    f.rect(x, y, w, 2, ORANGE);
    f.rect(x, y + 2, w, 1, BROWN);
}

fn help(a: &Assets) -> Option<Balloon> {
    let (block, k, (x, y)) = HELP;
    let records = a.dialogs(Program::Writer, block);
    Balloon::from_block(&records, k, x, y, "")
        .map(|b| b.in_program(Program::Writer))
        .or_else(|| {
        let mut b = crate::scenes::room_talk::fallback(
            144,
            "Este é o Cavalete de lápis de cera. Use-o para desenhar ou modificar uma figura.\nQuando terminar, clique em OK.",
            &[],
            &[],
        );
        (b.x, b.y) = (x, y);
        Some(b)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};
    use crate::ui::center;

    fn click(e: &mut CrayonEasel, a: &Assets, (x, y): (i32, i32)) -> Option<EaselOutcome> {
        let mut fx = Vec::new();
        let out = e.handle(
            &Input::Click {
                button: MouseButton::Left,
                x,
                y,
            },
            &mut fx,
            a,
        );
        e.handle(
            &Input::MouseUp {
                button: MouseButton::Left,
                x,
                y,
            },
            &mut fx,
            a,
        );
        out
    }

    fn button(t: Tool) -> (i32, i32) {
        center(BUTTONS.iter().find(|b| b.0 == t).unwrap().3)
    }

    fn opened(a: &Assets) -> CrayonEasel {
        let mut e = CrayonEasel::blank(a);
        click(&mut e, a, (10, 10)); // fecha o balão
        e
    }

    #[test]
    fn crayon_draws_in_the_chosen_color_and_ok_returns_600x350() {
        let (_d, a) = empty_assets();
        let mut e = opened(&a);
        click(&mut e, &a, center(swatch(5)));
        assert_eq!(e.color(), 5);
        let mut fx = Vec::new();
        e.handle(
            &Input::Click {
                button: MouseButton::Left,
                x: 150,
                y: 150,
            },
            &mut fx,
            &a,
        );
        e.handle(&Input::MouseMove { x: 200, y: 150 }, &mut fx, &a);
        e.handle(
            &Input::MouseUp {
                button: MouseButton::Left,
                x: 200,
                y: 150,
            },
            &mut fx,
            &a,
        );
        let Some(EaselOutcome::Ok(img)) = click(&mut e, &a, button(Tool::Ok)) else {
            panic!("OK não fechou")
        };
        assert_eq!((img.width, img.height), (NEW_W, NEW_H));
        let at = |x: u32, y: u32| {
            let i = ((y * img.width + x) * 4) as usize;
            [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]]
        };
        assert_eq!(at(175 - 120, 150 - 105), COLORS[5]);
        assert_eq!(at(5, 5), WHITE);
    }

    #[test]
    fn egg_undoes_and_vacuum_clears() {
        let (_d, a) = empty_assets();
        let mut e = opened(&a);
        click(&mut e, &a, (200, 200));
        assert_ne!(e.image().rgba, CrayonEasel::blank(&a).image().rgba);
        click(&mut e, &a, button(Tool::Egg));
        assert_eq!(e.image().rgba, CrayonEasel::blank(&a).image().rgba);
        click(&mut e, &a, (200, 200));
        click(&mut e, &a, button(Tool::Vacuum));
        assert_eq!(e.image().rgba, CrayonEasel::blank(&a).image().rgba);
        assert_eq!(click(&mut e, &a, button(Tool::No)), Some(EaselOutcome::No));
    }

    #[test]
    fn arrows_show_the_other_part_and_pictures_keep_their_colors() {
        let (_d, a) = empty_assets();
        let mut e = opened(&a);
        click(&mut e, &a, button(Tool::Right));
        click(&mut e, &a, button(Tool::Down));
        assert_eq!(e.view(), (PAN_STEP, PAN_STEP));
        for _ in 0..20 {
            click(&mut e, &a, button(Tool::Right));
        }
        assert_eq!(e.view().0, NEW_W as i32 - AREA.2);
        let mut img = Image {
            width: 3,
            height: 1,
            rgba: vec![0; 12],
        };
        img.rgba[0..4].copy_from_slice(&[10, 20, 30, 255]);
        img.rgba[4..8].copy_from_slice(&[0, 0, 0, 255]);
        img.rgba[8..12].copy_from_slice(&[255, 255, 255, 0]);
        let e = CrayonEasel::with_image(&img, &a);
        assert_eq!(e.size(), (3, 1));
        let out = e.image();
        assert_eq!(&out.rgba[0..4], &[10, 20, 30, 255]);
        assert_eq!(&out.rgba[4..8], &[0, 0, 0, 255]);
        assert_eq!(&out.rgba[8..12], &[255, 255, 255, 255]);
    }

    #[test]
    fn real_easel_draws() {
        let Some(a) = assets() else { return };
        let e = CrayonEasel::blank(&a);
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "ideas_easel");
        assert_eq!(&f.pixel(150, 200)[..3], &WHITE);
    }
}
