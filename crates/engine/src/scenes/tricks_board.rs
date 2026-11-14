//! Tela de apresentação das lições dos Truques de Desenho: o quadro branco
//! com o título ("Truques Básicos" / "Truques de 3-D"), a figura grande
//! numa moldura laranja e a Maggie (às vezes com o Max ou o McZee) mudando
//! de pose a cada balão. É a primeira parte da lição no original; ao
//! escolher treinar ("Sim, vamos experimentar"), a lição continua na tela
//! de pintura ("ateliê de treinamento", [`SceneId::TricksStudio`]).
//!
//! O roteiro é o mesmo [`Wizard`] da pintura (`tricks_basic` /
//! `tricks_3d`): os passos antes de `STUDIO` são desta tela. Geometria,
//! figuras e poses medidas no original no Wine
//! (`/home/claude/sweep/report/fid-tricks.md`, `docs/telas/truques.md`).

use std::rc::Rc;

use super::wizard::{
    Advance, BOARD_PANEL, BTN_BACK, BTN_NEXT, BTN_STOP, BalloonPlace, TrickKind, Wizard,
    WizardEvent, WizardOutcome,
};
use crate::Frame;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::help::HelpBalloon;
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::text_balloon::blit_pic;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const ORANGE: [u8; 3] = [255, 140, 50];
const BROWN: [u8; 3] = [185, 101, 36];

/// Quadro branco de cantos arredondados sobre o fundo preto.
pub const BOARD: (i32, i32, i32, i32) = (4, 18, 632, 458);
const BOARD_RADIUS: i32 = 10;
/// Caixa do título (moldura de fora).
pub const TITLE_BOX: (i32, i32, i32, i32) = (227, 4, 186, 30);
/// Pincel grosso da criança no quadro (passo "trace o espaço negativo").
const STROKE_R: i32 = 3;

/// Som do original para um evento do assistente (medidos no Wine pelo
/// tamanho do `waveOutWrite`): Segue e Volta `02eb` (7424 bytes), bolinha
/// de opção `0254` (2816), clique na Maggie `0226` (22060).
pub fn event_sound(e: WizardEvent) -> Option<Effect> {
    let id = match e {
        WizardEvent::Next | WizardEvent::Back => 0x02eb,
        WizardEvent::Option(_) => 0x0254,
        WizardEvent::McZee => 0x0226,
        _ => return None,
    };
    Some(Effect::PlaySound(SoundRef::Snd(Program::Artist, id)))
}

/// Moldura laranja em relevo (5 px) com o retângulo de fora em `(x, y, w, h)`:
/// de fora para dentro, preto, laranja, laranja, marrom e preto em cima e à
/// direita; preto, marrom, laranja, laranja e preto à esquerda e embaixo.
pub fn bevel(f: &mut Frame, x: i32, y: i32, w: i32, h: i32) {
    let top = [BLACK, ORANGE, ORANGE, BROWN, BLACK];
    let side = [BLACK, BROWN, ORANGE, ORANGE, BLACK];
    for k in 0..5 {
        let (x0, y0, x1, y1) = (x + k, y + k, x + w - 1 - k, y + h - 1 - k);
        f.rect(x0, y0, x1 - x0 + 1, 1, top[k as usize]);
        f.rect(x1, y0, 1, y1 - y0 + 1, top[k as usize]);
        f.rect(x0, y0 + 1, 1, y1 - y0, side[k as usize]);
        f.rect(x0 + 1, y1, x1 - x0, 1, side[k as usize]);
    }
}

/// Barra horizontal em relevo (5 px) de `x` a `x + w`, com o topo em `y`
/// (a divisão entre o título e a área de uma moldura).
pub fn bevel_bar(f: &mut Frame, x: i32, y: i32, w: i32) {
    for (k, c) in [BLACK, ORANGE, ORANGE, BROWN, BLACK]
        .into_iter()
        .enumerate()
    {
        f.rect(x, y + k as i32, w, 1, c);
    }
}

/// Texto centrado em `(x, y, w, h)` na fonte de `points` (ou na do balão).
pub fn centered_text(f: &mut Frame, a: &Assets, points: u16, r: (i32, i32, i32, i32), s: &str) {
    let font = a.font(points).or_else(|| crate::balloon::balloon_font(a));
    match font {
        Some(fo) => {
            let w = fo.width(s) as i32;
            let h = fo.glyph(b'A').bits.len() as i32 / fo.glyph(b'A').width.max(1) as i32;
            f.text_font(&fo, r.0 + (r.2 - w) / 2, r.1 + (r.3 - h) / 2, s, BLACK);
        }
        None => f.text(
            r.0 + (r.2 - 8 * s.chars().count() as i32) / 2,
            r.1 + (r.3 - 8) / 2,
            s,
            BLACK,
        ),
    }
}

/// Moldura "Sua Área de Desenho" na pintura (de fora), com o título em cima.
pub const YOUR_AREA_BOX: (i32, i32, i32, i32) = (26, 115, 398, 338);
/// Moldura "Área de Desenho da Maggie" (de fora), com o título em cima.
pub const MAGGIE_BOX: (i32, i32, i32, i32) = (426, 277, 185, 176);
/// Altura do título das duas molduras (sem a moldura).
const STUDIO_TITLE_H: i32 = 15;

/// O assistente da lição na tela de pintura, a partir do passo `start`: o
/// balão encostado à direita, sob a Maggie da porta.
pub fn studio_wizard(kind: TrickKind, start: usize) -> Wizard {
    let (right, top) = super::wizard::STUDIO_BALLOON;
    kind.wizard()
        .with_places(BalloonPlace::RightAt(right, top), &[])
        .starting_at(start)
}

fn titled_box(f: &mut Frame, a: &Assets, r: (i32, i32, i32, i32), title: &str) {
    let (x, y, w, _) = r;
    f.rect(x + 5, y + 5, w - 10, STUDIO_TITLE_H, WHITE);
    centered_text(f, a, 8, (x + 5, y + 5, w - 10, STUDIO_TITLE_H), title);
    bevel(f, r.0, r.1, r.2, r.3);
    bevel_bar(f, x + 5, y + 5 + STUDIO_TITLE_H, w - 10);
}

/// Na pintura, durante a lição: as molduras "Sua Área de Desenho" e "Área
/// de Desenho da Maggie" (com a figura do passo), como no original. Nada
/// enquanto a criança trabalha por conta própria.
pub fn draw_studio(f: &mut Frame, a: &Assets, w: &Wizard, blink: bool) {
    if w.paused() {
        return;
    }
    titled_box(f, a, YOUR_AREA_BOX, "Sua Área de Desenho");
    let (mx, my, mw, mh) = MAGGIE_BOX;
    f.rect(mx + 5, my + 25, mw - 10, mh - 30, WHITE);
    w.draw_panel(f, a, blink);
    titled_box(f, a, MAGGIE_BOX, "Área de Desenho da Maggie");
}

impl TrickKind {
    /// Título do quadro da apresentação.
    pub fn board_title(self) -> &'static str {
        match self {
            TrickKind::Basic => "Truques Básicos",
            TrickKind::ThreeD => "Truques de 3-D",
        }
    }

    /// Primeiro passo da lição na tela de pintura.
    pub fn studio_step(self) -> usize {
        match self {
            TrickKind::Basic => super::tricks_basic::STUDIO,
            TrickKind::ThreeD => super::tricks_3d::STUDIO,
        }
    }

    fn board_balloons(self) -> &'static [(usize, i32, i32)] {
        match self {
            TrickKind::Basic => super::tricks_basic::BOARD_BALLOONS,
            TrickKind::ThreeD => super::tricks_3d::BOARD_BALLOONS,
        }
    }

    fn board_poses(self) -> &'static [(usize, &'static [(u16, i32, i32)])] {
        match self {
            TrickKind::Basic => super::tricks_basic::BOARD_POSES,
            TrickKind::ThreeD => super::tricks_3d::BOARD_POSES,
        }
    }

    /// O assistente da lição inteira (apresentação e pintura).
    pub fn wizard(self) -> Wizard {
        match self {
            TrickKind::Basic => super::tricks_basic::wizard(),
            TrickKind::ThreeD => super::tricks_3d::wizard(),
        }
    }
}

pub struct TricksBoard {
    kind: TrickKind,
    common: KidCommon,
    wizard: Wizard,
    /// Traços da criança no quadro, em coordenadas do quadro.
    strokes: Vec<((i32, i32), (i32, i32))>,
    drag: Option<(i32, i32)>,
    help: Option<HelpBalloon>,
    t: f32,
    /// Já mandou trocar de cena (não processa mais nada).
    done: bool,
}

impl TricksBoard {
    pub fn new(kind: TrickKind, click: Option<SoundRef>) -> TricksBoard {
        TricksBoard {
            kind,
            common: KidCommon::new(click),
            wizard: kind
                .wizard()
                .with_panel(BOARD_PANEL)
                .with_places(BalloonPlace::At(440, 40), kind.board_balloons()),
            strokes: Vec::new(),
            drag: None,
            help: None,
            t: 0.0,
            done: false,
        }
    }

    pub fn wizard(&self) -> &Wizard {
        &self.wizard
    }

    /// Dica aberta pela lâmpada de um balão.
    pub fn help(&self) -> Option<&HelpBalloon> {
        self.help.as_ref()
    }

    pub fn strokes(&self) -> usize {
        self.strokes.len()
    }

    /// Figuras da Maggie (e companhia) do passo atual.
    pub fn poses(&self) -> &'static [(u16, i32, i32)] {
        let k = self.wizard.step().record.1;
        let all = self.kind.board_poses();
        all.iter()
            .find(|p| p.0 == k)
            .or(all.first())
            .map_or(&[], |p| p.1)
    }

    fn buttons(&self, a: &Assets) -> Vec<(WizardEvent, (i32, i32), (u16, u16))> {
        let w = &self.wizard;
        let Some(b) = w.balloon(a, "") else {
            return Vec::new();
        };
        let y = b.y + b.height(a) + 4;
        let mut x = b.x + 12;
        let mut out = Vec::new();
        if w.stopping() || w.quitting() {
            return out;
        }
        if w.has_back() {
            out.push((WizardEvent::Back, (x, y), BTN_BACK));
            x += 41;
        }
        out.push((WizardEvent::Stop, (x, y), BTN_STOP));
        x += 41;
        if w.has_next() {
            out.push((WizardEvent::Next, (x, y), BTN_NEXT));
        }
        out
    }

    fn maggie_at(&self, a: &Assets, x: i32, y: i32) -> bool {
        self.poses().iter().take(1).any(|&(id, px, py)| {
            let r = ImageRef::Pic(Program::Artist, id);
            a.has_image(&r) && {
                let img = a.image(&r);
                x >= px && y >= py && x < px + img.width as i32 && y < py + img.height as i32
            }
        })
    }

    fn painting(&self) -> bool {
        self.wizard.step().advance == Advance::PaintThenNext
            && !self.wizard.stopping()
            && !self.wizard.quitting()
    }

    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        if let Some(h) = self.help.take() {
            if h.contains(a, x, y) {
                self.help = match h.link_at(a, x, y) {
                    Some(t) => h.follow(t).or(Some(h)),
                    None => Some(h),
                };
            }
            return;
        }
        let Some(b) = self.wizard.balloon(a, "") else {
            return;
        };
        if let Some(target) = b.link_hit(a, x, y) {
            let block = self.wizard.step().record.0;
            let records = Rc::new(a.dialogs(Program::Artist, block));
            self.help = HelpBalloon::new(records, block, target, b.x - 160, b.y + 20);
            return;
        }
        let mut ev = b.hit(a, x, y).map(WizardEvent::Option);
        if ev.is_none() {
            ev = self
                .buttons(a)
                .into_iter()
                .find(|(_, (bx, by), _)| x >= *bx && y >= *by && x < bx + 31 && y < by + 14)
                .map(|(e, _, _)| e);
        }
        let (px, py, pw, ph) = BOARD_PANEL;
        let in_panel = x >= px && y >= py && x < px + pw && y < py + ph;
        if ev.is_none() && in_panel && self.painting() {
            self.drag = Some((x - px, y - py));
            self.strokes.push(((x - px, y - py), (x - px, y - py)));
            return;
        }
        if ev.is_none() {
            ev = self.wizard.panel_hit(x, y);
        }
        if ev.is_none() && self.maggie_at(a, x, y) {
            ev = Some(WizardEvent::McZee);
        }
        if ev.is_none() && (self.wizard.stopping() || self.wizard.quitting()) {
            ev = Some(WizardEvent::McZee);
        }
        if let Some(e) = ev {
            self.event(e, fx);
        }
    }

    fn event(&mut self, e: WizardEvent, fx: &mut Vec<Effect>) {
        fx.extend(event_sound(e));
        let out = self.wizard.on_event(e);
        match out {
            WizardOutcome::Entered(s) => {
                let (block, k) = self.wizard.step().record;
                fx.push(Effect::Log(format!("lição: passo {s} ({block:04x}[{k}])")));
                if s >= self.kind.studio_step() {
                    self.done = true;
                    fx.push(Effect::ChangeScene(SceneId::TricksStudio(self.kind, s)));
                } else if s == 0 {
                    self.strokes.clear();
                }
                self.help = None;
            }
            WizardOutcome::Leave(id) => {
                self.done = true;
                fx.push(Effect::ChangeScene(id));
            }
            WizardOutcome::FreePaint => {
                self.done = true;
                let s = self.kind.studio_step();
                fx.push(Effect::ChangeScene(SceneId::TricksStudio(self.kind, s)));
            }
            _ => {}
        }
    }
}

impl Scene for TricksBoard {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        self.t += dt.max(0.0);
        self.wizard.tick(dt);
        for i in input {
            if self.done {
                break;
            }
            match *i {
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => self.click(x, y, &mut fx, assets),
                Input::MouseMove { x, y } => {
                    if let Some(last) = self.drag {
                        let (px, py, _, _) = BOARD_PANEL;
                        let p = (x - px, y - py);
                        self.strokes.push((last, p));
                        self.drag = Some(p);
                    }
                }
                Input::MouseUp {
                    button: MouseButton::Left,
                    ..
                } => {
                    if self.drag.take().is_some() {
                        self.event(WizardEvent::Painted, &mut fx);
                    }
                }
                _ => {}
            }
        }
        fx
    }

    fn draw(&self, f: &mut Frame, a: &Assets) {
        f.fill(BLACK);
        let (bx, by, bw, bh) = BOARD;
        let r = BOARD_RADIUS;
        for y in by..by + bh {
            let dy = (r - (y - by)).max((y - (by + bh - 1 - r)).max(0));
            let inset = if dy > 0 {
                r - ((r * r - dy * dy) as f32).sqrt() as i32
            } else {
                0
            };
            f.rect(bx + inset, y, bw - 2 * inset, 1, WHITE);
        }
        let (tx, ty, tw, th) = TITLE_BOX;
        f.rect(tx, ty, tw, th, WHITE);
        bevel(f, tx, ty, tw, th);
        centered_text(
            f,
            a,
            16,
            (tx + 5, ty + 5, tw - 10, th - 10),
            self.kind.board_title(),
        );
        // a figura grande: a do passo (ou a do último passo com figura)
        let (px, py, pw, ph) = BOARD_PANEL;
        f.rect(px, py, pw, ph, WHITE);
        if let Some(id) = self.wizard.step().maggie {
            let img = a.image(&ImageRef::Pic(Program::Artist, id));
            f.blit(
                &img,
                px + (pw - img.width as i32) / 2,
                py + (ph - img.height as i32) / 2,
            );
        }
        let blink = (self.t * 3.0) as i32 % 2 == 0;
        self.wizard.draw_panel(f, a, blink);
        for &((x0, y0), (x1, y1)) in &self.strokes {
            let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
            for k in 0..=n {
                let (x, y) = (x0 + (x1 - x0) * k / n, y0 + (y1 - y0) * k / n);
                for dy in -STROKE_R..=STROKE_R {
                    for dx in -STROKE_R..=STROKE_R {
                        let (qx, qy) = (x + dx, y + dy);
                        if dx * dx + dy * dy <= STROKE_R * STROKE_R
                            && qx >= 0
                            && qy >= 0
                            && qx < pw
                            && qy < ph
                        {
                            f.put(px + qx, py + qy, BLACK);
                        }
                    }
                }
            }
        }
        bevel(f, px - 5, py - 5, pw + 10, ph + 10);
        for &(id, x, y) in self.poses() {
            blit_pic(f, a, id, x, y, true);
        }
        if let Some(b) = self.wizard.balloon(a, self.kind.board_title()) {
            b.draw(f, a);
        }
        for (_, (x, y), (img, _)) in self.buttons(a) {
            let r = ImageRef::Pic(Program::Artist, img);
            if a.has_image(&r) {
                f.blit(&a.image(&r), x, y);
            }
        }
        if let Some(h) = &self.help {
            h.draw(f, a);
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

    fn option(b: &mut TricksBoard, a: &Assets, k: usize) -> Vec<Effect> {
        let r = b.wizard.balloon(a, "").unwrap().option_rect(a, k);
        b.update(&[click(r.0 + 6, r.1 + 4)], 0.0, a)
    }

    fn next(b: &mut TricksBoard, a: &Assets) -> Vec<Effect> {
        let (_, (x, y), _) = b
            .buttons(a)
            .into_iter()
            .find(|(e, _, _)| *e == WizardEvent::Next)
            .expect("Segue");
        b.update(&[click(x + 10, y + 6)], 0.0, a)
    }

    #[test]
    fn bevel_matches_the_original_colors() {
        let mut f = Frame::new(40, 40);
        bevel(&mut f, 0, 0, 40, 40);
        // em cima: preto, laranja, laranja, marrom, preto
        let col: Vec<[u8; 3]> = (0..5)
            .map(|y| f.pixel(20, y)[..3].try_into().unwrap())
            .collect();
        assert_eq!(col, [BLACK, ORANGE, ORANGE, BROWN, BLACK]);
        // à esquerda: preto, marrom, laranja, laranja, preto
        let row: Vec<[u8; 3]> = (0..5)
            .map(|x| f.pixel(x, 20)[..3].try_into().unwrap())
            .collect();
        assert_eq!(row, [BLACK, BROWN, ORANGE, ORANGE, BLACK]);
        let row: Vec<[u8; 3]> = (35..40)
            .rev()
            .map(|x| f.pixel(x, 20)[..3].try_into().unwrap())
            .collect();
        assert_eq!(row, [BLACK, ORANGE, ORANGE, BROWN, BLACK]);
    }

    #[test]
    fn draws_without_data() {
        let (_dir, a) = empty_assets();
        let b = TricksBoard::new(TrickKind::Basic, None);
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        assert_eq!(f.pixel(0, 0)[..3], BLACK);
        assert_eq!(f.pixel(100, 100)[..3], WHITE);
    }

    #[test]
    fn basic_board_runs_until_the_studio() {
        let Some(a) = assets() else { return };
        let mut b = TricksBoard::new(TrickKind::Basic, None);
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "tricks_board_0");
        // a Maggie sentada com o pincel, à direita do quadro
        assert_eq!(b.poses(), &[(0x3803, 468, 262)]);
        let fx = option(&mut b, &a, 0);
        assert!(fx.contains(&Effect::PlaySound(SoundRef::Snd(Program::Artist, 0x254))));
        assert_eq!(b.wizard.current(), 1);
        let fx = next(&mut b, &a);
        assert!(fx.contains(&Effect::PlaySound(SoundRef::Snd(Program::Artist, 0x2eb))));
        next(&mut b, &a);
        assert_eq!(b.wizard.current(), 3);
        // o Max aparece com a Maggie no "clique no espaço positivo"
        assert_eq!(b.poses().len(), 2);
        let (px, py, _, _) = BOARD_PANEL;
        b.update(&[click(px + 150, py + 100)], 0.0, &a);
        assert_eq!(b.wizard.current(), 4);
        // traça no próprio quadro
        b.update(
            &[
                click(px + 20, py + 20),
                Input::MouseMove {
                    x: px + 60,
                    y: py + 40,
                },
                Input::MouseUp {
                    button: MouseButton::Left,
                    x: px + 60,
                    y: py + 40,
                },
            ],
            0.0,
            &a,
        );
        assert!(b.strokes() >= 2);
        assert_eq!(b.wizard.current(), 4);
        next(&mut b, &a);
        assert_eq!(b.wizard.current(), 5);
        b.update(&[click(px + 80, py + 136)], 0.0, &a);
        for (x, y, w, h) in super::super::tricks_basic::PIG_CIRCLES {
            b.update(&[click(px + x + w / 2, py + y + h / 2)], 0.0, &a);
        }
        assert_eq!(b.wizard.current(), 7);
        next(&mut b, &a);
        next(&mut b, &a);
        assert_eq!(b.wizard.current(), 9);
        let fx = option(&mut b, &a, 0);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::TricksStudio(
            TrickKind::Basic,
            super::super::tricks_basic::STUDIO
        ))));
    }

    #[test]
    fn going_to_train_skips_the_board() {
        let Some(a) = assets() else { return };
        let mut b = TricksBoard::new(TrickKind::ThreeD, None);
        let fx = option(&mut b, &a, 1);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::TricksStudio(
            TrickKind::ThreeD,
            super::super::tricks_3d::STUDIO
        ))));
        let mut b = TricksBoard::new(TrickKind::ThreeD, None);
        let fx = option(&mut b, &a, 2);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::TricksFloor)));
    }
}
