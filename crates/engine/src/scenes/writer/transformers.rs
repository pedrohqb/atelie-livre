//! Transformadores, a mão com tinta (`office-ferramentas.md` §7, balões
//! `0fa8`): Mover-modificar, Dimensionar tudo/lateral/vertical, Agente
//! Secreto 009, Coloca-figuras (Maçã) e Flutua-texto. Comportamento em
//! `docs/telas/creative-writer-transformadores.md`.
//!
//! - **Mover-modificar** (padrão ao escolher a ferramenta): arrastar um
//!   adesivo ou palavra com forma o move (`022e`); um clique sem arrastar
//!   leva o adesivo ou desenho para o Cavalete de lápis de cera
//!   ([`CrayonEasel`]); OK troca a figura. A palavra com forma iria para o
//!   cavalete de palavras com forma (W7; por enquanto fica no registro).
//!   Ctrl+arrastar estica por igual (`0fa8[15]`).
//! - **Dimensionar**: arrastar sobre o objeto, com o centro fixo; para fora
//!   aumenta, para dentro diminui (`022a` em laço enquanto arrasta). Com Ctrl
//!   os dois lados mudam por igual. O ponteiro fica até trocar de ferramenta
//!   (`0fa8[10]`): clicar fora dos objetos não põe o ponto de inserção.
//! - **Agente Secreto 009**: painel SENHA; OK embaralha a exibição (ver
//!   [`crate::writer::cipher`]) ou, com a senha (ou a senha-mestra com
//!   Ctrl+Shift), desembaralha. Avisos do Rex `0805[37]`, `[38]`, `[43]`.
//! - **Coloca-figuras**: figuras da Maçã depois das palavras conhecidas
//!   ([`crate::writer::rebus_apply`]); Ctrl+clique tira todas. Sem novidade:
//!   `0805[4]`; nenhuma palavra conhecida: `[3]`; sem a tabela: `[6]`.
//! - **Flutua-texto**: lista com os 4 modos; depois, clicar no objeto.

use super::cursors;
use super::editor::{Editor, PAGE};
use super::tools::{
    Cx, Pic, Rect, SND_CHOOSE, SND_OPTION, StripButton, ToolId, ToolStrip, WHAT_TRANSFORMERS,
    cursor, inside, strip_button,
};
use crate::Frame;
use crate::assets::{Assets, CursorRef, Program, SoundRef};
use crate::scene::{Effect, Input, Key, MouseButton};
use crate::scenes::writer_easel::{CrayonEasel, EaselOutcome};
use crate::writer::cipher::{self, Outcome};
use crate::writer::object::{ObjectKind, Resize, Wrap};
use crate::writer::rebus_apply::{self, Placed};
use formats::maca::Maca;

const W: u16 = WHAT_TRANSFORMERS;

/// Escolher o Mover-modificar.
pub const SND_MOVER: u16 = 0x021f;
/// Começar a arrastar um objeto.
pub const SND_DRAG: u16 = 0x022e;
/// Dimensionar (em laço enquanto arrasta).
pub const SND_SIZE: u16 = 0x022a;
/// OK do painel SENHA que deu certo.
pub const SND_OK: u16 = 0x02ee;
/// Flutua-texto (o mesmo som aparece também como `0295`).
pub const SND_WRAP: u16 = 0x0281;
/// "Epa!" do Rex.
pub const SND_EPA: u16 = 0x02fe;
/// Abrir o cavalete (como no Quadro de Figuras: `0224` + `0267`).
const SND_EASEL: [u16; 2] = [0x0224, 0x0267];

/// Avisos do Rex (`WHAT 0805`).
const REX: u16 = 0x0805;
pub const NO_MATCHES: (u16, usize) = (REX, 3);
pub const ALL_MATCHED: (u16, usize) = (REX, 4);
pub const REBUS_FILE: (u16, usize) = (REX, 6);
/// "Você não pode recortar nem copiar essas palavras!"
pub const ENCRYPTED_CUT_COPY: (u16, usize) = (REX, 13);
/// "Você precisa digitar uma senha" (para desembaralhar).
pub const NEED_PASSWORD: (u16, usize) = (REX, 37);
/// "Sinto muito, a senha não é essa!"
pub const WRONG_PASSWORD: (u16, usize) = (REX, 38);
/// "Você precisa digitar uma senha" (para embaralhar).
pub const NEED_PASSWORD_SCRAMBLE: (u16, usize) = (REX, 43);

const MOVER: usize = 0;
const SIZE_ALL: usize = 1;
const SIZE_WIDTH: usize = 2;
const SIZE_HEIGHT: usize = 3;
const SECRET: usize = 4;
const APPLE: usize = 5;
const WRAP: usize = 6;

const STRIP: [StripButton; 7] = [
    strip_button(
        0x0923,
        Some(0x0929),
        (33, 58, 36, 39),
        (W, 1),
        "Mover-modificar",
    ),
    strip_button(
        0x0857,
        Some(0x0859),
        (94, 54, 56, 48),
        (W, 2),
        "Dimensionar tudo",
    ),
    strip_button(
        0x085b,
        Some(0x085d),
        (150, 54, 37, 48),
        (W, 3),
        "Dimensionar lateral",
    ),
    strip_button(
        0x085f,
        Some(0x0861),
        (187, 54, 66, 48),
        (W, 4),
        "Dimensionar vertical",
    ),
    strip_button(
        0x0807,
        Some(0x0809),
        (278, 59, 37, 38),
        (W, 5),
        "Agente Secreto 009",
    ),
    strip_button(0x0865, None, (340, 61, 67, 34), (W, 6), "Coloca-figuras"),
    strip_button(
        0x08c7,
        Some(0x08cb),
        (432, 56, 74, 43),
        (W, 7),
        "Flutua-texto",
    ),
];

// ---------------------------------------------------------------
// Painel SENHA (`office_110`)

/// Caixa do painel (moldura laranja), pendurada na faixa.
const PANEL: Rect = (113, 106, 322, 63);
/// Placa da senha `08d7` (207×52).
const PLATE: Pic = Pic::opaque(0x08d7, 118, 112);
/// Campo branco da placa.
const FIELD: Rect = (137, 128, 170, 19);
/// Etiqueta "SENHA" (aba de baixo da placa).
const LABEL: Rect = (170, 150, 104, 10);
/// NÃO `0271` e OK `026d` (com as máscaras duas figuras adiante).
const NO: Pic = Pic::masked(0x0271, 0x0273, 345, 124);
const OK: Pic = Pic::masked(0x026d, 0x026f, 388, 125);
const NO_RECT: Rect = (345, 124, 28, 28);
const OK_RECT: Rect = (388, 125, 26, 27);
const ORANGE: [u8; 3] = [255, 140, 50];
const DARK_ORANGE: [u8; 3] = [185, 101, 36];
const GREY: [u8; 3] = [140, 140, 140];
const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];

// ---------------------------------------------------------------
// Lista do Flutua-texto (`office_114`)

/// Moldura da lista.
const LIST: Rect = (221, 98, 217, 221);
/// Os 4 modos: figura (186×42, com o texto e a miniatura) e lugar.
const WRAP_ITEMS: [(u16, i32, Wrap); 4] = [
    (0x0c2b, 105, Wrap::Contour),
    (0x0c2d, 160, Wrap::AllPages),
    (0x0c2f, 215, Wrap::Box),
    (0x0c31, 270, Wrap::TopBottom),
];
const WRAP_X: i32 = 245;
const WRAP_W: i32 = 186;
const WRAP_H: i32 = 42;

/// Distância mínima (pixels) para um clique virar arrasto.
const DRAG_SLOP: i32 = 2;

/// O que um clique na página faz agora.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Mover,
    Size(Resize),
    /// Flutua-texto com o modo escolhido na lista.
    Wrap(Wrap),
}

/// Arrasto de um objeto.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Drag {
    obj: usize,
    /// `None` = mover; senão, esticar.
    size: Option<Resize>,
    /// Onde o botão desceu (tela).
    start: (i32, i32),
    last: (i32, i32),
    /// Centro do objeto na tela, no clique.
    center: (f64, f64),
    /// Retângulo do objeto no clique (pontos da folha).
    rect: (i16, i16, u16, u16),
    /// Já mexeu (guardou o estado no Ovo).
    moved: bool,
}

#[derive(Default)]
pub struct TransformersStrip {
    mode: Mode,
    drag: Option<Drag>,
    /// Painel SENHA aberto, com o que foi digitado.
    secret: Option<String>,
    /// Lista do Flutua-texto aberta.
    wrap_list: bool,
    /// Tabela da Maçã, lida do `WRITER.PIC` no primeiro clique.
    maca: Option<Option<Maca>>,
    /// Cavalete de lápis de cera aberto com a figura do objeto.
    easel: Option<(usize, CrayonEasel)>,
}

impl TransformersStrip {
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Painel SENHA aberto e o que está digitado nele.
    pub fn secret_text(&self) -> Option<&str> {
        self.secret.as_deref()
    }

    pub fn wrap_list_open(&self) -> bool {
        self.wrap_list
    }

    /// Objeto que está no cavalete.
    pub fn easel_object(&self) -> Option<usize> {
        self.easel.as_ref().map(|(k, _)| *k)
    }

    /// Clique sem arrastar no objeto `k` com o Mover-modificar.
    fn modify(&mut self, ed: &mut Editor, cx: &mut Cx, k: usize) {
        let o = &ed.doc().objects[k];
        match o.kind {
            ObjectKind::Sticker | ObjectKind::Drawing => {
                let e = match ed.picture(o) {
                    Some(img) => CrayonEasel::with_image(&img, cx.assets),
                    None => CrayonEasel::blank(cx.assets),
                };
                for s in SND_EASEL {
                    cx.sound(s);
                }
                self.easel = Some((k, e));
            }
            _ => cx
                .log("Mover-modificar: o cavalete de palavras com forma ainda não está disponível"),
        }
    }

    /// Passa um evento ao cavalete aberto; ao fechar com OK, troca a figura.
    fn easel_input(&mut self, ed: &mut Editor, cx: &mut Cx, i: Input) {
        let Some((k, e)) = self.easel.as_mut() else {
            return;
        };
        let k = *k;
        if let Some(out) = e.handle(&i, cx.fx, cx.assets) {
            self.easel = None;
            if let EaselOutcome::Ok(img) = out {
                ed.replace_picture(k, &img);
            }
        }
    }

    fn button(&mut self, ed: &mut Editor, cx: &mut Cx, k: usize) {
        let help = STRIP[k].help.unwrap_or((W, 0));
        match k {
            MOVER => {
                self.mode = Mode::Mover;
                cx.sound(SND_MOVER);
                cx.help(help);
            }
            SIZE_ALL | SIZE_WIDTH | SIZE_HEIGHT => {
                self.mode = Mode::Size(match k {
                    SIZE_ALL => Resize::All,
                    SIZE_WIDTH => Resize::Width,
                    _ => Resize::Height,
                });
                cx.sound(SND_OPTION);
                cx.help(help);
            }
            SECRET => {
                self.secret = Some(String::new());
                cx.sound(SND_OPTION);
                cx.help(help);
            }
            APPLE => {
                cx.sound(SND_OPTION);
                self.apple(ed, cx);
            }
            WRAP => {
                self.wrap_list = true;
                cx.sound(SND_WRAP);
                cx.help(help);
            }
            _ => {}
        }
    }

    /// Clique na Maçã (com Ctrl: tira todas as figuras).
    fn apple(&mut self, ed: &mut Editor, cx: &mut Cx) {
        if cx.ctrl {
            if rebus_apply::has_rebus(ed.doc()) {
                let n = ed.edit(rebus_apply::remove);
                cx.log(format!("Coloca-figuras: {n} figuras tiradas"));
            }
            return;
        }
        let assets = cx.assets;
        let maca = self.maca.get_or_insert_with(|| {
            assets
                .pic_bytes(Program::Writer)
                .and_then(|b| Maca::read(b).ok())
        });
        let Some(maca) = maca.as_ref() else {
            rex(cx, REBUS_FILE);
            return;
        };
        // numa cópia: só vira ação do Ovo se puser alguma figura
        let (mut doc, mut sel) = (ed.doc().clone(), ed.sel);
        match rebus_apply::place(&mut doc, &mut sel, |w| maca.lookup(w)) {
            Placed::Some(n) => {
                ed.edit(move |d, s| {
                    *d = doc;
                    *s = sel;
                });
                cx.help((W, 12));
                cx.log(format!("Coloca-figuras: {n} figuras"));
            }
            Placed::AllMatched => rex(cx, ALL_MATCHED),
            Placed::NoMatches => rex(cx, NO_MATCHES),
        }
    }

    /// OK do painel SENHA.
    fn secret_ok(&mut self, ed: &mut Editor, cx: &mut Cx) {
        let typed = self.secret.clone().unwrap_or_default();
        let was = ed.encrypted();
        match ed.secret_ok(&typed, cx.ctrl && cx.shift) {
            Outcome::Scrambled => {
                cx.sound(SND_OK);
                self.secret = None;
                cx.help((W, 11));
            }
            Outcome::Unscrambled => {
                cx.sound(SND_OK);
                self.secret = None;
                cx.close_help();
            }
            Outcome::Empty => rex(
                cx,
                if was {
                    NEED_PASSWORD
                } else {
                    NEED_PASSWORD_SCRAMBLE
                },
            ),
            Outcome::Wrong => {
                self.secret = Some(String::new());
                rex(cx, WRONG_PASSWORD);
            }
        }
    }

    fn secret_press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) {
        if inside(OK_RECT, x, y) {
            self.secret_ok(ed, cx);
        } else if inside(NO_RECT, x, y) {
            cx.sound(SND_OPTION);
            self.secret = None;
        }
    }

    fn wrap_press(&mut self, cx: &mut Cx, x: i32, y: i32) {
        self.wrap_list = false;
        if let Some(&(_, _, w)) = WRAP_ITEMS
            .iter()
            .find(|&&(_, iy, _)| inside((WRAP_X, iy, WRAP_W, WRAP_H), x, y))
        {
            self.mode = Mode::Wrap(w);
            cx.sound(SND_CHOOSE);
        }
    }

    /// Clique na página com o modo atual; `true` = tratado.
    fn page_press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        let hit = ed
            .object_at(x, y)
            .filter(|&k| ed.doc().objects[k].kind.is_movable());
        let size = match self.mode {
            Mode::Wrap(w) => {
                if let Some(k) = hit
                    && ed.doc().objects[k].wrap != w
                {
                    ed.edit(|d, _| d.objects[k].wrap = w);
                    cx.sound(SND_OPTION);
                }
                return inside(PAGE, x, y);
            }
            Mode::Size(r) => Some(if cx.ctrl { Resize::All } else { r }),
            Mode::Mover => cx.ctrl.then_some(Resize::All),
        };
        let Some(k) = hit else {
            // o ponteiro de dimensionar fica até trocar de ferramenta
            return size.is_some() && inside(PAGE, x, y);
        };
        let (Some(p), o) = (ed.to_page(x, y), &ed.doc().objects[k]) else {
            return false;
        };
        let (cxp, cyp) = (
            f64::from(o.x) + f64::from(o.width) / 2.0,
            f64::from(o.y) + f64::from(o.height) / 2.0,
        );
        self.drag = Some(Drag {
            obj: k,
            size,
            start: (x, y),
            last: (x, y),
            center: (
                f64::from(x) - (f64::from(p.x) - cxp),
                f64::from(y) - (f64::from(p.y) - cyp),
            ),
            rect: (o.x, o.y, o.width, o.height),
            moved: false,
        });
        true
    }
}

/// Aviso do Rex ("Epa!"), com a ajuda ligada ou não.
fn rex(cx: &mut Cx, h: (u16, usize)) {
    cx.sound(SND_EPA);
    cx.help_even_off(h);
}

/// Novo tamanho `(largura, altura)` ao arrastar de `start` a `cur` com o
/// centro do objeto em `center` (tudo na tela); `(w0, h0)` = tamanho no
/// clique. Tudo: na proporção da distância ao centro; lateral e vertical:
/// o dobro do quanto o ponteiro se afastou do centro naquele eixo.
pub fn sized(
    mode: Resize,
    (w0, h0): (u16, u16),
    center: (f64, f64),
    start: (i32, i32),
    cur: (i32, i32),
) -> (i32, i32) {
    let (w0, h0) = (f64::from(w0), f64::from(h0));
    let d = |p: (i32, i32)| (f64::from(p.0) - center.0, f64::from(p.1) - center.1);
    let (s, c) = (d(start), d(cur));
    match mode {
        Resize::All => {
            let k = c.0.hypot(c.1) / s.0.hypot(s.1).max(4.0);
            ((w0 * k).round() as i32, (h0 * k).round() as i32)
        }
        Resize::Width => (
            (w0 + 2.0 * (c.0.abs() - s.0.abs())).round() as i32,
            h0 as i32,
        ),
        Resize::Height => (
            w0 as i32,
            (h0 + 2.0 * (c.1.abs() - s.1.abs())).round() as i32,
        ),
    }
}

impl ToolStrip for TransformersStrip {
    fn id(&self) -> ToolId {
        ToolId::Transformers
    }

    fn activate(&mut self, _ed: &mut Editor, _cx: &mut Cx, again: bool) {
        if !again {
            self.mode = Mode::Mover;
        }
    }

    fn deactivate(&mut self, _ed: &mut Editor, cx: &mut Cx) {
        self.secret = None;
        self.easel = None;
        self.wrap_list = false;
        if self.drag.take().is_some_and(|d| d.size.is_some()) {
            cx.fx.push(Effect::StopLoop);
        }
    }

    fn modal(&self) -> bool {
        self.secret.is_some() || self.wrap_list || self.easel.is_some()
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if self.easel.is_some() {
            let button = MouseButton::Left;
            self.easel_input(ed, cx, Input::Click { button, x, y });
            return true;
        }
        if self.secret.is_some() {
            self.secret_press(ed, cx, x, y);
            return true;
        }
        if self.wrap_list {
            self.wrap_press(cx, x, y);
            return true;
        }
        if let Some(k) = StripButton::find(&STRIP, x, y) {
            self.button(ed, cx, k);
            return true;
        }
        self.page_press(ed, cx, x, y)
    }

    fn motion(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) {
        if self.easel.is_some() {
            self.easel_input(ed, cx, Input::MouseMove { x, y });
            return;
        }
        let Some(mut d) = self.drag else {
            return;
        };
        if !d.moved {
            if (x - d.start.0).abs().max((y - d.start.1).abs()) < DRAG_SLOP {
                return;
            }
            d.moved = true;
            ed.record();
            match d.size {
                Some(_) => cx
                    .fx
                    .push(Effect::PlayLoop(SoundRef::Snd(Program::Writer, SND_SIZE))),
                None => cx.sound(SND_DRAG),
            }
        }
        let (pw, ph) = (
            i32::from(ed.doc().dop.page_width),
            i32::from(ed.doc().dop.page_height),
        );
        let Some(o) = ed.doc_mut().objects.get_mut(d.obj) else {
            self.drag = None;
            return;
        };
        match d.size {
            None => o.move_by(x - d.last.0, y - d.last.1, pw, ph),
            Some(mode) => {
                let (x0, y0, w0, h0) = d.rect;
                let (w, h) = sized(mode, (w0, h0), d.center, d.start, (x, y));
                (o.x, o.y, o.width, o.height) = (x0, y0, w0, h0);
                o.resize(mode, w, h);
                // o centro fica no lugar
                let dx = (i32::from(w0) - i32::from(o.width)) / 2;
                let dy = (i32::from(h0) - i32::from(o.height)) / 2;
                o.x = (i32::from(x0) + dx).clamp(i16::MIN.into(), i16::MAX.into()) as i16;
                o.y = (i32::from(y0) + dy).clamp(i16::MIN.into(), i16::MAX.into()) as i16;
            }
        }
        d.last = (x, y);
        self.drag = Some(d);
    }

    fn release(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) {
        if self.easel.is_some() {
            let button = MouseButton::Left;
            self.easel_input(ed, cx, Input::MouseUp { button, x, y });
            return;
        }
        let Some(d) = self.drag.take() else {
            return;
        };
        if d.moved {
            if d.size.is_some() {
                cx.fx.push(Effect::StopLoop);
            }
            ed.after_edit();
        } else if d.size.is_none() {
            self.modify(ed, cx, d.obj);
        }
    }

    fn key(&mut self, ed: &mut Editor, cx: &mut Cx, key: Key, _ctrl: bool) -> bool {
        if self.easel.is_some() {
            return true;
        }
        if let Some(t) = self.secret.as_mut() {
            match key {
                Key::Enter => self.secret_ok(ed, cx),
                Key::Escape => self.secret = None,
                Key::Backspace => {
                    t.pop();
                }
                _ => {}
            }
            return true;
        }
        if self.wrap_list {
            if key == Key::Escape {
                self.wrap_list = false;
            }
            return true;
        }
        false
    }

    fn text(&mut self, _ed: &mut Editor, _cx: &mut Cx, text: &str) -> bool {
        let Some(t) = self.secret.as_mut() else {
            return self.wrap_list || self.easel.is_some();
        };
        for c in text.chars().filter(|c| !c.is_control()) {
            if t.chars().count() < cipher::MAX_LEN {
                t.push(c);
            }
        }
        true
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
    }

    fn draw_front(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        if let Some(t) = &self.secret {
            draw_panel(f, a, t);
        }
        if let Some((_, e)) = &self.easel {
            e.draw(f, a);
        }
        if self.wrap_list {
            let (x, y, w, h) = LIST;
            f.rect(x, y, w, h, WHITE);
            f.frame_rect(x, y, w, h, 1, BLACK);
            for &(id, iy, _) in &WRAP_ITEMS {
                if iy > WRAP_ITEMS[0].1 {
                    f.rect(x, iy - 7, w, 1, BLACK);
                }
                Pic::opaque(id, WRAP_X, iy).draw(f, a);
            }
        }
    }

    fn cursor(&self, ed: &Editor, x: i32, y: i32) -> Option<CursorRef> {
        if self.modal() || !inside(PAGE, x, y) {
            return None;
        }
        if let Some(d) = &self.drag {
            return Some(cursor(match d.size {
                None => cursors::HAND_FIST,
                Some(r) => size_cursor(r),
            }));
        }
        let movable = ed
            .object_at(x, y)
            .is_some_and(|k| ed.doc().objects[k].kind.is_movable());
        match self.mode {
            Mode::Mover => movable.then(|| cursor(cursors::HAND_OPEN)),
            Mode::Size(r) => Some(cursor(size_cursor(r))),
            Mode::Wrap(_) => Some(cursor(cursors::DOT)),
        }
    }
}

fn size_cursor(r: Resize) -> u16 {
    match r {
        Resize::All => cursors::SIZE_ALL,
        Resize::Width => cursors::SIZE_H,
        Resize::Height => cursors::SIZE_V,
    }
}

/// O painel SENHA com o texto `t` e o ponto de inserção no fim.
fn draw_panel(f: &mut Frame, a: &Assets, t: &str) {
    let (x, y, w, h) = PANEL;
    // preto, laranja (2 claros + 1 escuro), preto; fundo cinza
    f.rect(x, y, w, h, BLACK);
    f.rect(x + 1, y + 2, w - 2, h - 3, DARK_ORANGE);
    f.rect(x + 2, y + 2, w - 4, h - 4, ORANGE);
    f.rect(x + 4, y + 5, w - 8, h - 9, BLACK);
    f.rect(x + 5, y + 6, w - 10, h - 11, GREY);
    // divisória entre a placa e os botões
    f.rect(211, y + 5, 6, h - 9, ORANGE);
    PLATE.draw(f, a);
    f.rect(324, y + 5, 2, h - 9, BLACK);
    f.rect(326, y + 5, 3, h - 9, ORANGE);
    f.rect(326, y + 5, 1, h - 9, DARK_ORANGE);
    f.rect(329, y + 5, 1, h - 9, BLACK);
    NO.draw(f, a);
    OK.draw(f, a);
    let font = a.font(8);
    let (fx, fy, fw, _) = FIELD;
    let width = |s: &str| match font.as_deref() {
        Some(fo) => fo.width(s) as i32,
        None => 8 * s.chars().count() as i32,
    };
    // o que não cabe no campo rola para a esquerda
    let mut shown = t;
    while width(shown) > fw - 8 {
        let mut it = shown.chars();
        it.next();
        shown = it.as_str();
    }
    let tx = fx + 4;
    match font.as_deref() {
        Some(fo) => {
            f.text_font(fo, tx, fy + 2, shown, BLACK);
        }
        None => f.text(tx, fy + 4, shown, BLACK),
    }
    f.rect(tx + width(shown), fy + 2, 1, 14, BLACK);
    let (lx, ly, lw, _) = LABEL;
    let l = "SENHA";
    match font.as_deref() {
        Some(fo) => {
            f.text_font(fo, lx + (lw - width(l)) / 2, ly - 2, l, BLACK);
        }
        None => f.text(lx + (lw - width(l)) / 2, ly, l, BLACK),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;
    use crate::scenes::writer::Writer;
    use crate::writer::doc::Item;
    use crate::writer::object::{FloatObject, ObjRef, ObjectKind};
    use crate::{
        scene::{Input, MouseButton, Scene},
        ui::center,
    };

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn up(x: i32, y: i32) -> Input {
        Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn mv(x: i32, y: i32) -> Input {
        Input::MouseMove { x, y }
    }

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(SoundRef::Snd(Program::Writer, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    fn button(k: usize) -> Input {
        let (x, y) = center(STRIP[k].rect());
        click(x, y)
    }

    /// Writer com a ferramenta escolhida, o texto `t` e um adesivo de
    /// 40×40 no ponto (200, 100) da folha 1.
    fn writer(t: &str) -> (tempfile::TempDir, Assets, Writer) {
        let (d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[Input::Text(t.into())], 0.0, &a);
        let o = FloatObject::new(ObjectKind::Sticker, ObjRef::pict(9), 0, 200, 100, 40, 40);
        w.ed.add_object(o, None);
        let (x, y) = center(ToolId::Transformers.def().hit);
        w.update(&[click(x, y)], 0.0, &a);
        (d, a, w)
    }

    /// Ponto da tela sobre o ponto `(x, y)` da folha 1.
    fn on_page(w: &Writer, x: i32, y: i32) -> (i32, i32) {
        w.ed.to_screen(0, x, y)
    }

    fn obj(w: &Writer) -> (i16, i16, u16, u16) {
        let o = &w.ed.doc().objects[0];
        (o.x, o.y, o.width, o.height)
    }

    #[test]
    fn mover_drags_the_object_and_is_one_undo_step() {
        let (_d, a, mut w) = writer("Oi");
        assert_eq!(w.tools.transformers.mode(), Mode::Mover);
        let (x, y) = on_page(&w, 220, 120);
        let fx = w.update(
            &[click(x, y), mv(x + 10, y + 5), mv(x + 30, y + 15)],
            0.0,
            &a,
        );
        assert_eq!(sounds(&fx), vec![SND_DRAG]);
        let fx = w.update(&[up(x + 30, y + 15)], 0.0, &a);
        assert!(sounds(&fx).is_empty());
        assert_eq!(obj(&w), (230, 115, 40, 40));
        // o texto não ganhou o ponto de inserção no lugar do clique
        assert_eq!(w.ed.sel.caret.index, 2);
        assert!(w.ed.undo_step());
        assert_eq!(obj(&w), (200, 100, 40, 40));
    }

    #[test]
    fn mover_click_without_drag_opens_the_easel() {
        let (_d, a, mut w) = writer("");
        let (x, y) = on_page(&w, 220, 120);
        let eggs = w.ed.undo.undo_len();
        let fx = w.update(&[click(x, y), mv(x + 1, y), up(x + 1, y)], 0.0, &a);
        assert_eq!(obj(&w), (200, 100, 40, 40));
        assert_eq!(w.ed.undo.undo_len(), eggs);
        assert_eq!(sounds(&fx), SND_EASEL.to_vec());
        assert_eq!(w.tools.transformers.easel_object(), Some(0));
        // sem a figura (o adesivo de teste não tem PICT): o cavalete em
        // branco; OK põe o desenho no lugar do adesivo
        // o 1º clique fecha o balão do cavalete; OK em (479,311) 38×38
        let (ox, oy) = (498, 330);
        w.update(&[click(10, 10), up(10, 10)], 0.0, &a);
        assert_eq!(w.tools.transformers.easel_object(), Some(0));
        w.update(&[click(ox, oy), up(ox, oy)], 0.0, &a);
        assert_eq!(w.tools.transformers.easel_object(), None);
        let o = &w.ed.doc().objects[0];
        assert_eq!(o.kind, ObjectKind::Drawing);
        assert_eq!((o.x, o.y), (200, 100));
        assert!(w.ed.extra.object(o.object).is_some());
        assert!(w.ed.undo_step());
        assert_eq!(w.ed.doc().objects[0].kind, ObjectKind::Sticker);
    }

    #[test]
    fn sizers_keep_the_center_and_loop_the_sound() {
        let (_d, a, mut w) = writer("");
        // Dimensionar lateral: 10 px para fora do centro = +20 de largura
        w.update(&[button(SIZE_WIDTH)], 0.0, &a);
        assert_eq!(w.tools.transformers.mode(), Mode::Size(Resize::Width));
        let (x, y) = on_page(&w, 230, 120);
        let fx = w.update(&[click(x, y), mv(x + 10, y + 7)], 0.0, &a);
        assert!(fx.contains(&Effect::PlayLoop(SoundRef::Snd(Program::Writer, SND_SIZE))));
        let fx = w.update(&[up(x + 10, y + 7)], 0.0, &a);
        assert!(fx.contains(&Effect::StopLoop));
        assert_eq!(obj(&w), (190, 100, 60, 40));
        // Dimensionar vertical: para dentro diminui
        w.update(&[button(SIZE_HEIGHT)], 0.0, &a);
        let (x, y) = on_page(&w, 220, 135);
        w.update(&[click(x, y), mv(x, y - 10), up(x, y - 10)], 0.0, &a);
        assert_eq!(obj(&w), (190, 110, 60, 20));
        // Dimensionar tudo: o dobro da distância, o dobro do tamanho
        w.update(&[button(SIZE_ALL)], 0.0, &a);
        let (x, y) = on_page(&w, 230, 120);
        w.update(&[click(x, y), mv(x + 10, y), up(x + 10, y)], 0.0, &a);
        assert_eq!(obj(&w), (160, 100, 120, 40));
        // clicar fora dos objetos não mexe no texto nem no ponto de inserção
        let (x, y) = on_page(&w, 400, 400);
        w.update(&[click(x, y), up(x, y)], 0.0, &a);
        assert_eq!(w.ed.sel.caret.index, 0);
    }

    #[test]
    fn sized_math() {
        let c = (100.0, 100.0);
        assert_eq!(
            sized(Resize::All, (40, 20), c, (110, 100), (120, 100)),
            (80, 40)
        );
        assert_eq!(
            sized(Resize::Width, (40, 20), c, (90, 100), (80, 130)),
            (60, 20)
        );
        assert_eq!(
            sized(Resize::Height, (40, 20), c, (100, 95), (100, 105)),
            (40, 20)
        );
        assert_eq!(
            sized(Resize::Height, (40, 20), c, (100, 95), (100, 90)),
            (40, 30)
        );
    }

    #[test]
    fn ctrl_drag_with_the_mover_sizes_evenly() {
        let (_d, a, mut w) = writer("");
        let (x, y) = on_page(&w, 230, 120);
        w.update(&[Input::Ctrl(true)], 0.0, &a);
        w.update(&[click(x, y), mv(x + 10, y), up(x + 10, y)], 0.0, &a);
        w.update(&[Input::Ctrl(false)], 0.0, &a);
        assert_eq!(obj(&w), (180, 80, 80, 80));
    }

    /// Clique direto na faixa (sem o Writer), com o balão pedido.
    fn press(
        t: &mut TransformersStrip,
        ed: &mut Editor,
        a: &Assets,
        (x, y): (i32, i32),
        ctrl: bool,
        shift: bool,
    ) -> (Vec<u16>, Option<(u16, usize)>) {
        let mut fx = Vec::new();
        let mut cx = Cx::new(&mut fx, a);
        (cx.ctrl, cx.shift) = (ctrl, shift);
        t.press(ed, &mut cx, x, y);
        let help = cx.take_help().map(|h| h.0);
        (sounds(&fx), help)
    }

    fn type_in(t: &mut TransformersStrip, ed: &mut Editor, a: &Assets, s: &str) {
        let mut fx = Vec::new();
        assert!(t.text(ed, &mut Cx::new(&mut fx, a), s));
    }

    #[test]
    fn secret_agent_scrambles_and_unscrambles() {
        let (_d, a) = empty_assets();
        let mut ed = Editor::blank(None, &a);
        ed.type_text("O gato");
        let mut t = TransformersStrip::default();
        let at = |r: Rect| center(r);
        let button = at(STRIP[SECRET].rect());
        assert_eq!(
            press(&mut t, &mut ed, &a, button, false, false).1,
            Some((W, 5))
        );
        assert_eq!(t.secret_text(), Some(""));
        assert!(t.modal());
        // OK sem senha: o Rex
        assert_eq!(
            press(&mut t, &mut ed, &a, at(OK_RECT), false, false),
            (vec![SND_EPA], Some(NEED_PASSWORD_SCRAMBLE))
        );
        assert!(!ed.encrypted());
        type_in(&mut t, &mut ed, &a, "abc");
        assert_eq!(t.secret_text(), Some("abc"));
        assert_eq!(
            press(&mut t, &mut ed, &a, at(OK_RECT), false, false),
            (vec![SND_OK], Some((W, 11)))
        );
        assert!(ed.encrypted());
        assert_eq!(ed.extra.add.password, "abc");
        assert_eq!(t.secret_text(), None);
        // o texto não muda
        assert_eq!(ed.doc().paras[0].text(), "O gato");
        // sem senha, senha errada, senha-mestra sem e com Ctrl+Shift
        press(&mut t, &mut ed, &a, button, false, false);
        assert_eq!(
            press(&mut t, &mut ed, &a, at(OK_RECT), false, false).1,
            Some(NEED_PASSWORD)
        );
        type_in(&mut t, &mut ed, &a, "xyz");
        assert_eq!(
            press(&mut t, &mut ed, &a, at(OK_RECT), false, false).1,
            Some(WRONG_PASSWORD)
        );
        assert_eq!(t.secret_text(), Some(""));
        type_in(&mut t, &mut ed, &a, cipher::MASTER);
        press(&mut t, &mut ed, &a, at(OK_RECT), true, false);
        assert!(ed.encrypted());
        type_in(&mut t, &mut ed, &a, cipher::MASTER);
        assert_eq!(
            press(&mut t, &mut ed, &a, at(OK_RECT), true, true).0,
            vec![SND_OK]
        );
        assert!(!ed.encrypted());
        assert!(ed.extra.add.password.is_empty());
        // NÃO fecha sem mudar nada
        press(&mut t, &mut ed, &a, button, false, false);
        press(&mut t, &mut ed, &a, at(NO_RECT), false, false);
        assert_eq!(t.secret_text(), None);
        assert!(!ed.encrypted());
    }

    #[test]
    fn secret_panel_takes_the_keyboard_and_blocks_copy() {
        let (_d, a, mut w) = writer("O gato");
        w.update(&[button(SECRET), Input::Text("ab".into())], 0.0, &a);
        let k = |key, ctrl| Input::Key {
            key,
            ctrl,
            alt: false,
        };
        w.update(
            &[k(Key::Backspace, false), Input::Text("c".into())],
            0.0,
            &a,
        );
        // digitar foi para o campo, não para o documento
        assert_eq!(w.tools.transformers.secret_text(), Some("ac"));
        assert_eq!(w.doc().paras[0].text(), "O gato");
        let fx = w.update(&[k(Key::Enter, false)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_OK]);
        assert!(w.ed.encrypted());
        // documento embaralhado: Ctrl+C é recusado
        let fx = w.update(&[k(Key::A, true), k(Key::C, true)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_EPA]);
        assert!(w.ed.clipboard.clip.is_none());
        // Esc fecha o painel
        w.update(&[button(SECRET), k(Key::Escape, false)], 0.0, &a);
        assert_eq!(w.tools.transformers.secret_text(), None);
    }

    #[test]
    fn scrambled_display_changes_the_ink_and_covers_objects() {
        let (_d, a, mut w) = writer("O gato e a bicicleta");
        let draw = |w: &Writer| {
            let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
            w.draw(&mut f, &a);
            f
        };
        w.update(&[], 0.6, &a);
        let plain = draw(&w);
        w.update(&[button(SECRET), Input::Text("k".into())], 0.0, &a);
        let (ox, oy) = center(OK_RECT);
        w.update(&[click(ox, oy)], 0.0, &a);
        let secret = draw(&w);
        // o adesivo (sem desenho: só o contorno) virou um quadrado preto
        let (x, y) = on_page(&w, 220, 120);
        assert_ne!(plain.pixel(x as u32, y as u32), [0, 0, 0, 255]);
        assert_eq!(secret.pixel(x as u32, y as u32), [0, 0, 0, 255]);
        // e a linha de texto mudou
        let (x0, y0) = on_page(&w, 0, 0);
        let diff = (y0 + 60..y0 + 90)
            .flat_map(|y| (x0..x0 + 190).map(move |x| (x as u32, y as u32)))
            .filter(|&(x, y)| plain.pixel(x, y) != secret.pixel(x, y))
            .count();
        assert!(diff > 20, "{diff}");
    }

    #[test]
    fn apple_without_the_table_warns() {
        let (_d, a) = empty_assets();
        let mut ed = Editor::blank(None, &a);
        ed.type_text("gato");
        let mut t = TransformersStrip::default();
        assert_eq!(
            press(
                &mut t,
                &mut ed,
                &a,
                center(STRIP[APPLE].rect()),
                false,
                false
            ),
            (vec![SND_OPTION, SND_EPA], Some(REBUS_FILE))
        );
        assert_eq!(ed.doc().paras[0].text(), "gato");
    }

    /// Com o `WRITER.PIC` real: a amostra do original ganha as 3 figuras
    /// (`0x25`, `0x0f`, `0xeb`, `office-arquivos.md` §1), e o 2º clique só
    /// mostra o Epa! `0805[4]`.
    #[test]
    fn apple_with_the_real_table() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut ed = Editor::blank(None, &a);
        ed.type_text("O gato e a bicicleta foram passear no parque.");
        let mut t = TransformersStrip::default();
        let apple = center(STRIP[APPLE].rect());
        let (snd, help) = press(&mut t, &mut ed, &a, apple, false, false);
        assert_eq!((snd, help), (vec![SND_OPTION], Some((W, 12))));
        let figs: Vec<u8> = ed.doc().paras[0]
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Rebus(n) => Some(*n),
                _ => None,
            })
            .collect();
        assert_eq!(figs, [0x25, 0x0f, 0xeb]);
        assert_eq!(ed.undo.undo_len(), 2);
        let (snd, help) = press(&mut t, &mut ed, &a, apple, false, false);
        assert_eq!((snd, help), (vec![SND_OPTION, SND_EPA], Some(ALL_MATCHED)));
        assert_eq!(ed.undo.undo_len(), 2);
        ed.type_text(" abelha");
        // "abelha" não está na tabela: nada novo
        assert_eq!(
            press(&mut t, &mut ed, &a, apple, false, false).1,
            Some(ALL_MATCHED)
        );
        let mut ed = Editor::blank(None, &a);
        ed.type_text("abelha voou");
        assert_eq!(
            press(&mut t, &mut ed, &a, apple, false, false).1,
            Some(NO_MATCHES)
        );
    }

    #[test]
    fn ctrl_apple_removes_all_figures_in_one_step() {
        let (_d, a, mut w) = writer("gato");
        w.ed.edit(|d, _| {
            d.insert_item(crate::writer::doc::Pos::new(0, 4), Item::Rebus(0x25));
        });
        w.update(
            &[Input::Ctrl(true), button(APPLE), Input::Ctrl(false)],
            0.0,
            &a,
        );
        assert!(!rebus_apply::has_rebus(w.doc()));
        assert!(w.ed.undo_step());
        assert!(rebus_apply::has_rebus(w.doc()));
    }

    #[test]
    fn wrap_list_then_click_on_the_object() {
        let (_d, a, mut w) = writer("");
        let fx = w.update(&[button(WRAP)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_WRAP]);
        assert!(w.tools.transformers.wrap_list_open());
        // a 4ª opção: acima e abaixo
        let (_, iy, _) = WRAP_ITEMS[3];
        w.update(&[click(WRAP_X + 20, iy + 20)], 0.0, &a);
        assert!(!w.tools.transformers.wrap_list_open());
        assert_eq!(w.tools.transformers.mode(), Mode::Wrap(Wrap::TopBottom));
        let (x, y) = on_page(&w, 220, 120);
        w.update(&[click(x, y), up(x, y)], 0.0, &a);
        assert_eq!(w.ed.doc().objects[0].wrap, Wrap::TopBottom);
        assert_eq!(obj(&w), (200, 100, 40, 40));
        assert!(w.ed.undo_step());
        assert_eq!(w.ed.doc().objects[0].wrap, Wrap::Contour);
        // clicar fora da lista só fecha
        w.update(&[button(WRAP), click(100, 400)], 0.0, &a);
        assert!(!w.tools.transformers.wrap_list_open());
        assert_eq!(w.tools.transformers.mode(), Mode::Wrap(Wrap::TopBottom));
    }
}
