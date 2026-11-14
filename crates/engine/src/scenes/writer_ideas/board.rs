//! Quadro de Figuras (close-up da Oficina de Idéias, `ideas.md` §4): a
//! figura em tamanho natural, centrada e recortada na área do quadro;
//! OUTRA (sorteia), lista suspensa com rolagem e ✓ na atual, campo de
//! título, POE (cavalete vazio; depois do OK digita-se o título), clique na
//! figura (modificar no cavalete), TIRA (`0805[10]`/`[11]`) e FEITO
//! (`0805[41]`). As figuras próprias são gravadas ao sair.

use super::buttons::{Art, PressButton, Presser, hit};
use super::notebook::{NotebookView, Outcome};
use super::{
    Instructions, MAX_RECT, Owner, draw_backdrop, max_balloon, plain_balloon, play, rex_alert,
    room, snd,
};
use crate::Frame;
use crate::assets::Assets;
use crate::balloon::{Balloon, line_height};
use crate::rng::XorShift;
use crate::scene::{Effect, Input, Key, KidCommon, MouseButton, Scene};
use crate::scenes::writer_easel::{CrayonEasel, EaselOutcome};
use crate::text_balloon::TextBalloon;
use crate::ui::{Pen, Rect, inside};
use crate::writer::ideas::board::{Board, load_max_pictures};
use crate::writer::ideas::notes::NoteText;
use crate::writer::ideas::story::IdeaStory;
use crate::writer::ideas::{session, store};

/// Moldura aberta (sanefa, McZee no medalhão, faixa verde).
const FRAME: (Art, i32, i32) = (Art::masked(0x7579, 0x7725), 160, 80);
/// Onde a figura aparece (centrada e recortada).
pub const PICTURE: Rect = (193, 170, 248, 202);
const DONE: PressButton = PressButton::new(
    389,
    128,
    64,
    27,
    Art::masked(0x753b, 0x753d),
    Art::masked(0x7561, 0x7565),
);
const OTHER: PressButton = PressButton::new(
    326,
    376,
    40,
    30,
    Art::masked(0x7571, 0x7575),
    Art::masked(0x7577, 0x7575),
);
const POE: PressButton = PressButton::new(
    373,
    380,
    24,
    24,
    Art::masked(0x7543, 0x7545),
    Art::masked(0x7569, 0x7545),
);
const TIRA: PressButton = PressButton::new(
    405,
    380,
    28,
    24,
    Art::masked(0x7547, 0x7549),
    Art::masked(0x756d, 0x7549),
);
/// Seta ▲ da lista (normal e apertada = silhueta, `N+2`).
const LIST_BUTTON: PressButton =
    PressButton::new(203, 381, 18, 20, Art::opaque(0x02db), Art::opaque(0x02dd));
const LIST_ARROW_POS: (i32, i32) = (206, 386);
/// Campo do título.
pub const TITLE: Rect = (222, 381, 96, 19);

/// Lista suspensa: coluna branca, cabeçalho com ▲▼, 17 linhas de 21 px.
const LIST_X: i32 = 222;
const LIST_W: i32 = 131;
const LIST_TOP: i32 = 8;
const LIST_HEAD: i32 = 16;
const LIST_ROW: i32 = 21;
const LIST_ROWS: usize = 17;
const LIST_UP: (i32, i32) = (263, 10);
const LIST_DOWN: (i32, i32) = (311, 10);

const ARRIVAL: (usize, (i32, i32), i32) = (51, (70, 265), 172);
const ARRIVAL_TEXT: &str = "Percorra as figuras no Quadro de Figuras! Tenho certeza de que você vai achar uma sobre a qual vai querer escrever!";
const INSTRUCTIONS: Instructions = Instructions {
    x: 145,
    y: 64,
    w: 351,
    h: 350,
    title: "Usando o Quadro de Figuras — clique para continuar",
    pic: 0x0c3b,
};
const ALERT_POS: (i32, i32) = (200, 172);
const WHAT_NOW_POS: (i32, i32) = (177, 177);
/// Título de figura sem nome (`STR#0809[2]`).
const UNKNOWN: &str = "Figura Desconhecida";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Btn {
    Done,
    Other,
    Poe,
    Tira,
    List,
}

fn buttons() -> [(Btn, PressButton); 5] {
    [
        (Btn::Done, DONE),
        (Btn::Other, OTHER),
        (Btn::Poe, POE),
        (Btn::Tira, TIRA),
        (Btn::List, LIST_BUTTON),
    ]
}

enum Talk {
    Arrival(Balloon),
    Instructions,
    Remove(usize, TextBalloon),
    MaxPicture(TextBalloon),
    WhatNow(Balloon),
}

/// Para que o cavalete foi aberto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EaselFor {
    New,
    Modify(usize),
}

pub struct PictureBoard {
    common: KidCommon,
    owner: Owner,
    board: Board,
    rng: XorShift,
    presser: Presser<Btn>,
    talk: Option<Talk>,
    /// Lista suspensa aberta, com a primeira linha visível.
    list: Option<usize>,
    easel: Option<(CrayonEasel, EaselFor)>,
    /// Título sendo digitado (depois do POE).
    naming: Option<NoteText>,
    notebook: Option<NotebookView>,
    started: bool,
}

impl PictureBoard {
    /// Quadro do usuário: as figuras próprias gravadas e as do Max; à
    /// mostra, a última vista na sessão (na primeira vez, uma sorteada).
    pub fn new(users: Option<&std::path::Path>, user: &str, assets: &Assets) -> PictureBoard {
        let owner = Owner::new(users, user);
        let mine = owner
            .dir
            .as_deref()
            .map(store::load_board_pictures)
            .unwrap_or_default();
        let board = Board::new(load_max_pictures(assets.data_dir()), mine);
        PictureBoard::with_board(owner, board, XorShift::from_clock())
    }

    fn with_board(owner: Owner, mut board: Board, mut rng: XorShift) -> PictureBoard {
        match session::board(&owner.user).and_then(|k| board.find(&k)) {
            Some(i) => board.select(i),
            None => {
                let n = board.len();
                board.select(rng.below(n));
            }
        }
        PictureBoard {
            common: KidCommon::new(None),
            owner,
            board,
            rng,
            presser: Presser::default(),
            talk: None,
            list: None,
            easel: None,
            naming: None,
            notebook: None,
            started: false,
        }
    }

    /// Sorteio fixo (testes e capturas); escolhe de novo a figura inicial.
    pub fn with_seed(mut self, seed: u32) -> PictureBoard {
        self.rng = XorShift::new(seed);
        if session::board(&self.owner.user).is_none() {
            let n = self.board.len();
            self.board.select(self.rng.below(n));
        }
        self
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn list_open(&self) -> bool {
        self.list.is_some()
    }

    pub fn easel_open(&self) -> bool {
        self.easel.is_some()
    }

    pub fn naming(&self) -> bool {
        self.naming.is_some()
    }

    pub fn notebook(&self) -> Option<&NotebookView> {
        self.notebook.as_ref()
    }

    pub fn talk(&self) -> Option<&'static str> {
        self.talk.as_ref().map(|t| match t {
            Talk::Arrival(_) => "chegada",
            Talk::Instructions => "instrucoes",
            Talk::Remove(..) => "tira",
            Talk::MaxPicture(_) => "coisa-do-max",
            Talk::WhatNow(_) => "feito",
        })
    }

    fn arrival(&mut self, a: &Assets) {
        let (k, pos, w) = ARRIVAL;
        let b = max_balloon(
            a,
            k,
            pos,
            &self.owner.user,
            w,
            ARRIVAL_TEXT,
            &["Instruções do Quadro de Figuras"],
        );
        self.talk = Some(Talk::Arrival(b));
    }

    fn title(&self) -> String {
        let t = self.board.title(self.board.current());
        if t.is_empty() {
            UNKNOWN.into()
        } else {
            t.into()
        }
    }

    fn finish_naming(&mut self) {
        if let Some(t) = self.naming.take() {
            let title = t.text();
            let title = if title.trim().is_empty() {
                UNKNOWN.to_string()
            } else {
                title
            };
            self.board.set_title(self.board.current(), &title);
        }
    }

    fn open_easel(&mut self, purpose: EaselFor, fx: &mut Vec<Effect>, a: &Assets) {
        fx.push(play(snd::EASEL_OPEN));
        fx.push(play(snd::PICTURES));
        let e = match purpose {
            EaselFor::New => CrayonEasel::blank(a),
            EaselFor::Modify(i) => match self.board.image(i) {
                Some(img) => CrayonEasel::with_image(img, a),
                None => CrayonEasel::blank(a),
            },
        };
        self.easel = Some((e, purpose));
    }

    fn easel_closed(&mut self, out: EaselOutcome, purpose: EaselFor) {
        let EaselOutcome::Ok(img) = out else { return };
        match purpose {
            EaselFor::New => {
                self.board.add_user("", img);
                self.naming = Some(NoteText::default());
            }
            EaselFor::Modify(i) => self.board.modify(i, img),
        }
    }

    // --- lista suspensa ----------------------------------------------------------

    fn list_row(k: usize) -> Rect {
        (
            LIST_X,
            LIST_TOP + LIST_HEAD + LIST_ROW * k as i32,
            LIST_W,
            LIST_ROW,
        )
    }

    fn list_rect() -> Rect {
        (
            LIST_X,
            LIST_TOP,
            LIST_W,
            LIST_HEAD + LIST_ROW * LIST_ROWS as i32,
        )
    }

    fn click_list(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>) {
        let Some(first) = self.list else { return };
        let n = self.board.len();
        let max_first = n.saturating_sub(LIST_ROWS);
        if inside((LIST_UP.0 - 4, LIST_TOP, 20, LIST_HEAD), x, y) {
            fx.push(play(snd::ITEM));
            self.list = Some(first.saturating_sub(LIST_ROWS));
            return;
        }
        if inside((LIST_DOWN.0 - 4, LIST_TOP, 20, LIST_HEAD), x, y) {
            fx.push(play(snd::ITEM));
            self.list = Some((first + LIST_ROWS).min(max_first));
            return;
        }
        let row = (0..LIST_ROWS).find(|&k| inside(Self::list_row(k), x, y));
        match row.map(|k| first + k).filter(|&i| i < n) {
            Some(i) => {
                fx.push(play(snd::ITEM));
                self.board.select(i);
                self.list = None;
            }
            None if inside(Self::list_rect(), x, y) => {}
            None => self.list = None,
        }
    }

    // --- cliques ------------------------------------------------------------------

    fn press(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        if self.naming.is_some() && !inside(TITLE, x, y) {
            self.finish_naming();
        }
        match hit(&buttons(), x, y) {
            Some(b) => self.presser.press(b),
            None if inside(PICTURE, x, y) && !self.board.is_empty() => {
                let i = self.board.current();
                self.open_easel(EaselFor::Modify(i), fx, a);
            }
            None if inside(MAX_RECT, x, y) => {
                fx.push(play(snd::OBJECT));
                self.arrival(a);
            }
            None => {}
        }
    }

    fn release(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        let Some(b) = self.presser.release(hit(&buttons(), x, y)) else {
            return;
        };
        match b {
            Btn::Other => {
                fx.push(play(snd::ROUND));
                self.board.other(&mut self.rng);
            }
            Btn::List => {
                fx.push(play(snd::LIST));
                let first = self.board.current().saturating_sub(LIST_ROWS / 2);
                self.list = Some(first.min(self.board.len().saturating_sub(LIST_ROWS)));
            }
            Btn::Poe => {
                fx.push(play(snd::POE));
                self.open_easel(EaselFor::New, fx, a);
            }
            Btn::Tira => {
                fx.push(play(snd::REX));
                fx.push(play(snd::EPA));
                let i = self.board.current();
                self.talk = Some(if self.board.is_user(i) {
                    Talk::Remove(
                        i,
                        rex_alert(
                            a,
                            10,
                            ALERT_POS,
                            210,
                            "Tem certeza?\nVocê quer mesmo jogar fora esta figura?",
                            &["Não, eu quero manter!", "É, quero jogar fora!"],
                        ),
                    )
                } else {
                    Talk::MaxPicture(rex_alert(
                        a,
                        11,
                        ALERT_POS,
                        254,
                        "Você não pode jogar fora as coisas do Max!\nVocê só pode jogar fora as figuras que você mesmo criou.",
                        &[],
                    ))
                });
            }
            Btn::Done => {
                fx.push(play(snd::DONE));
                self.talk = Some(Talk::WhatNow(plain_balloon(
                    a,
                    41,
                    WHAT_NOW_POS,
                    283,
                    "O que você quer fazer agora?",
                    &[
                        "Escrever uma história com essa figura",
                        "Colocar essa figura no Caderno de Idéias",
                        "Fechar o Quadro de Figuras",
                        "Ops! Quero continuar trabalhando no Quadro de Figuras",
                    ],
                )));
            }
        }
    }

    fn click_talk(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        let Some(talk) = self.talk.take() else {
            return;
        };
        match talk {
            Talk::Arrival(b) => {
                if b.hit(a, x, y) == Some(0) {
                    fx.push(play(snd::CHOICE));
                    self.talk = Some(Talk::Instructions);
                } else if b.contains(a, x, y) {
                    fx.push(play(snd::BALLOON));
                    self.talk = Some(Talk::Arrival(b));
                }
            }
            Talk::Instructions | Talk::MaxPicture(_) => fx.push(play(snd::BALLOON)),
            Talk::Remove(i, b) => match b.hit_option(a, x, y) {
                Some(1) => {
                    fx.push(play(snd::CHOICE));
                    let _ = self.board.remove(i);
                }
                Some(_) => fx.push(play(snd::CHOICE)),
                None => self.talk = Some(Talk::Remove(i, b)),
            },
            Talk::WhatNow(b) => match b.hit(a, x, y) {
                Some(0) => {
                    fx.push(play(snd::CHOICE));
                    fx.push(play(snd::EDITOR));
                    let i = self.board.current();
                    if let Some(img) = self.board.image(i) {
                        let story = IdeaStory::from_picture(&self.title(), img.clone());
                        let id = story.scene();
                        session::set_story(&self.owner.user, story);
                        fx.push(Effect::ChangeScene(id));
                    }
                }
                Some(1) => {
                    fx.push(play(snd::CHOICE));
                    fx.push(play(snd::NOTEBOOK));
                    let mut nb = NotebookView::open(&self.owner, a);
                    if let Some(img) = self.board.image(self.board.current()) {
                        nb.add_picture(&self.title(), img.clone());
                    }
                    self.notebook = Some(nb);
                }
                Some(2) => {
                    fx.push(play(snd::CHOICE));
                    fx.push(play(snd::ROOM));
                    fx.push(Effect::ChangeScene(room()));
                }
                Some(_) => fx.push(play(snd::CHOICE)),
                None => self.talk = Some(Talk::WhatNow(b)),
            },
        }
    }

    fn draw_list(&self, f: &mut Frame, a: &Assets, first: usize) {
        let pen = Pen::new(a);
        let lh = line_height(pen.font());
        let (x, y, w, h) = Self::list_rect();
        f.rect(x - 1, y - 1, w + 2, h + 2, [0, 0, 0]);
        f.rect(x, y, w, h, [255, 255, 255]);
        super::buttons::Art::opaque(0x02db).draw(f, a, LIST_UP.0, LIST_UP.1);
        super::buttons::Art::opaque(0x02df).draw(f, a, LIST_DOWN.0, LIST_DOWN.1);
        for k in 0..LIST_ROWS {
            let i = first + k;
            if i >= self.board.len() {
                break;
            }
            let r = Self::list_row(k);
            f.rect(r.0, r.1, r.2, 1, [0, 0, 0]);
            let title = pen.clip(self.board.title(i), r.2 - 22);
            pen.text(f, r.0 + 20, r.1 + (r.3 - lh) / 2, &title, [0, 0, 0]);
            if i == self.board.current() {
                draw_check(f, r.0 + 5, r.1 + 5);
            }
        }
    }
}

/// Marca ✓ da figura atual na lista.
fn draw_check(f: &mut Frame, x: i32, y: i32) {
    for k in 0..4 {
        f.rect(x + k, y + 5 + k, 2, 2, [0, 0, 0]);
    }
    for k in 0..8 {
        f.rect(x + 4 + k, y + 8 - k, 2, 2, [0, 0, 0]);
    }
}

/// Figura centrada em `r`, recortada nas bordas e só onde a moldura
/// (`frame`, com o canto em `at`) é branca: a sanefa e as franjas ficam
/// por cima, como no original.
fn blit_clipped(f: &mut Frame, img: &crate::Image, r: Rect, frame: &crate::Image, at: (i32, i32)) {
    let (w, h) = (img.width as i32, img.height as i32);
    let (x0, y0) = (r.0 + (r.2 - w) / 2, r.1 + (r.3 - h) / 2);
    let white = |x: i32, y: i32| {
        let (fx, fy) = (x - at.0, y - at.1);
        if fx < 0 || fy < 0 || fx >= frame.width as i32 || fy >= frame.height as i32 {
            return true;
        }
        let i = ((fy * frame.width as i32 + fx) * 4) as usize;
        frame.rgba[i..i + 3] == [255, 255, 255]
    };
    for y in r.1.max(y0)..(r.1 + r.3).min(y0 + h) {
        for x in r.0.max(x0)..(r.0 + r.2).min(x0 + w) {
            let i = (((y - y0) * w + (x - x0)) * 4) as usize;
            let p = &img.rgba[i..i + 4];
            if p[3] != 0 && white(x, y) {
                f.put(x, y, [p[0], p[1], p[2]]);
            }
        }
    }
}

impl Scene for PictureBoard {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        if !self.started {
            self.started = true;
            fx.push(play(snd::CLOSE_UP));
            fx.push(play(snd::BOARD));
            self.arrival(assets);
        }
        if let Some(nb) = self.notebook.as_mut() {
            nb.tick(dt);
        }
        for i in input {
            if let Some(nb) = self.notebook.as_mut() {
                if let Some(out) = nb.handle(i, &mut fx, assets) {
                    self.notebook = None;
                    if out == Outcome::Ok {
                        fx.push(Effect::Log("caderno gravado".into()));
                    }
                }
                continue;
            }
            if let Some((e, purpose)) = self.easel.as_mut() {
                let purpose = *purpose;
                if let Some(out) = e.handle(i, &mut fx, assets) {
                    self.easel = None;
                    self.easel_closed(out, purpose);
                }
                continue;
            }
            match i {
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    if self.talk.is_some() {
                        self.click_talk(*x, *y, &mut fx, assets);
                    } else if self.list.is_some() {
                        self.click_list(*x, *y, &mut fx);
                    } else {
                        self.press(*x, *y, &mut fx, assets);
                    }
                }
                Input::MouseUp {
                    button: MouseButton::Left,
                    x,
                    y,
                } => self.release(*x, *y, &mut fx, assets),
                Input::Key { key, .. } => {
                    if let Some(t) = self.naming.as_mut() {
                        match key {
                            Key::Enter => self.finish_naming(),
                            Key::Backspace => t.backspace(),
                            Key::Delete => t.delete(),
                            Key::Left => t.left(),
                            Key::Right => t.right(),
                            _ => {}
                        }
                    }
                }
                Input::Text(s) => {
                    if let Some(t) = self.naming.as_mut() {
                        t.insert(s);
                    }
                }
                _ => {}
            }
        }
        fx
    }

    fn draw(&self, f: &mut Frame, a: &Assets) {
        draw_backdrop(f, a);
        let (art, x, y) = FRAME;
        art.draw(f, a, x, y);
        if let Some(img) = self.board.image(self.board.current()) {
            let frame = a.image(&super::wpic(art.id));
            blit_clipped(f, img, PICTURE, &frame, (x, y));
        }
        for (k, b) in buttons() {
            if k == Btn::List {
                let down = self.presser.is_down(k);
                let art = if down { b.pressed } else { b.normal };
                art.draw(f, a, LIST_ARROW_POS.0, LIST_ARROW_POS.1);
            } else {
                b.draw(f, a, self.presser.is_down(k));
            }
        }
        // campo do título
        let pen = Pen::new(a);
        let lh = line_height(pen.font());
        f.rect(TITLE.0, TITLE.1, TITLE.2, TITLE.3, [255, 255, 255]);
        let (text, caret) = match &self.naming {
            Some(t) => (t.text(), true),
            None => (self.title(), false),
        };
        let shown = pen.clip(&text, TITLE.2 - 6);
        let ty = TITLE.1 + (TITLE.3 - lh) / 2;
        pen.centered(f, TITLE.0 + TITLE.2 / 2, ty, &shown, [0, 0, 0]);
        if caret {
            let x = TITLE.0 + TITLE.2 / 2 + pen.width(&shown) / 2 + 1;
            f.rect(x, ty, 1, lh, [0, 0, 0]);
        }
        if let Some(first) = self.list {
            self.draw_list(f, a, first);
        }
        match &self.talk {
            Some(Talk::Arrival(b)) | Some(Talk::WhatNow(b)) => b.draw(f, a),
            Some(Talk::Instructions) => INSTRUCTIONS.draw(f, a),
            Some(Talk::Remove(_, b)) | Some(Talk::MaxPicture(b)) => b.draw(f, a),
            None => {}
        }
        if let Some((e, _)) = &self.easel {
            e.draw(f, a);
        }
        if let Some(nb) = &self.notebook {
            nb.draw(f, a);
        }
    }

    fn leave(&mut self) -> Vec<Effect> {
        self.finish_naming();
        let mut fx = Vec::new();
        if let Some(k) = self.board.key(self.board.current()) {
            session::set_board(&self.owner.user, k);
        }
        if self.board.dirty()
            && let Some(dir) = &self.owner.dir
        {
            match store::save_board_pictures(dir, self.board.user_pictures_mut()) {
                Ok(()) => self.board.saved(),
                Err(e) => fx.push(Effect::Log(format!("Quadro de Figuras: {e}"))),
            }
        }
        fx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Image;
    use crate::scene::{CwScene, SceneId};
    use crate::scenes::testutil::{assets, empty_assets};
    use crate::ui::center;
    use crate::writer::ideas::board::MaxPicture;

    fn img(w: u32, h: u32) -> Image {
        Image {
            width: w,
            height: h,
            rgba: vec![0; (w * h * 4) as usize],
        }
    }

    fn click(b: &mut PictureBoard, a: &Assets, (x, y): (i32, i32)) -> Vec<Effect> {
        let mut fx = b.update(
            &[Input::Click {
                button: MouseButton::Left,
                x,
                y,
            }],
            0.0,
            a,
        );
        fx.extend(b.update(
            &[Input::MouseUp {
                button: MouseButton::Left,
                x,
                y,
            }],
            0.0,
            a,
        ));
        fx
    }

    fn opened(a: &Assets, user: &str, dir: Option<&std::path::Path>) -> PictureBoard {
        let max = (0..30)
            .map(|k| MaxPicture {
                pict: k,
                title: format!("Figura {k}"),
                image: img(100 + k as u32, 50),
            })
            .collect();
        let owner = Owner {
            user: user.into(),
            dir: dir.map(|d| d.to_path_buf()),
        };
        let mut b = PictureBoard::with_board(owner, Board::new(max, vec![]), XorShift::new(9));
        b.update(&[], 0.0, a);
        assert_eq!(b.talk(), Some("chegada"));
        click(&mut b, a, (600, 20));
        assert_eq!(b.talk(), None);
        b
    }

    #[test]
    fn list_scrolls_and_picks_a_picture() {
        let (_d, a) = empty_assets();
        let mut b = opened(&a, "q1", None);
        click(&mut b, &a, center(LIST_BUTTON.rect()));
        assert!(b.list_open());
        let first = b.list.unwrap();
        click(&mut b, &a, (LIST_DOWN.0, LIST_TOP + 4));
        assert_eq!(b.list, Some((first + LIST_ROWS).min(30 - LIST_ROWS)));
        let row = center(PictureBoard::list_row(2));
        let expect = b.list.unwrap() + 2;
        click(&mut b, &a, row);
        assert!(!b.list_open());
        assert_eq!(b.board().current(), expect);
    }

    #[test]
    fn poe_draws_a_picture_and_asks_for_its_title() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut b = opened(&a, "q2", Some(dir.path()));
        click(&mut b, &a, center(POE.rect()));
        assert!(b.easel_open());
        click(&mut b, &a, (10, 10)); // fecha o balão do cavalete
        click(&mut b, &a, (200, 200)); // um risco
        click(&mut b, &a, (497, 330)); // OK
        assert!(!b.easel_open());
        assert!(b.naming());
        assert_eq!(b.board().current(), 0);
        assert!(b.board().is_user(0));
        b.update(&[Input::Text("Meu rabisco".into())], 0.0, &a);
        b.update(
            &[Input::Key {
                key: Key::Enter,
                ctrl: false,
                alt: false,
            }],
            0.0,
            &a,
        );
        assert_eq!(b.board().title(0), "Meu rabisco");
        b.leave();
        let saved = store::load_board_pictures(dir.path());
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].title, "Meu rabisco");
        assert_eq!(saved[0].image.width, 600);
    }

    #[test]
    fn tira_protects_max_pictures_and_removes_the_users() {
        let (_d, a) = empty_assets();
        let mut b = opened(&a, "q3", None);
        click(&mut b, &a, center(TIRA.rect()));
        assert_eq!(b.talk(), Some("coisa-do-max"));
        click(&mut b, &a, (5, 5));
        b.board.add_user("Minha", img(5, 5));
        click(&mut b, &a, center(TIRA.rect()));
        assert_eq!(b.talk(), Some("tira"));
        let Some(Talk::Remove(_, t)) = &b.talk else {
            unreachable!()
        };
        let yes = center(t.option_rect(&a, 1).unwrap());
        click(&mut b, &a, yes);
        assert_eq!(b.board().len(), 30);
    }

    #[test]
    fn done_writes_a_story_with_the_picture_and_remembers_it() {
        let (_d, a) = empty_assets();
        let mut b = opened(&a, "q4", None);
        click(&mut b, &a, center(OTHER.rect()));
        let shown = b.board().current();
        click(&mut b, &a, center(DONE.rect()));
        let Some(Talk::WhatNow(w)) = &b.talk else {
            panic!("sem balão do FEITO")
        };
        let write = center(w.option_rect(&a, 0));
        let fx = click(&mut b, &a, write);
        let id = SceneId::Cw(CwScene::IdeaStory(
            crate::writer::ideas::story::IdeaSource::Board,
        ));
        assert!(fx.contains(&Effect::ChangeScene(id)));
        let story = session::take_story("q4").unwrap();
        assert_eq!(story.pictures[0].title, format!("Figura {shown}"));
        b.leave();
        let max = (0..30)
            .map(|k| MaxPicture {
                pict: k,
                title: format!("Figura {k}"),
                image: img(10, 10),
            })
            .collect();
        let again = PictureBoard::with_board(
            Owner {
                user: "q4".into(),
                dir: None,
            },
            Board::new(max, vec![]),
            XorShift::new(1),
        );
        assert_eq!(again.board().current(), shown);
    }

    #[test]
    fn done_puts_the_picture_in_the_notebook() {
        let (_d, a) = empty_assets();
        let mut b = opened(&a, "q5", None);
        click(&mut b, &a, center(DONE.rect()));
        let Some(Talk::WhatNow(w)) = &b.talk else {
            unreachable!()
        };
        let keep = center(w.option_rect(&a, 1));
        click(&mut b, &a, keep);
        let nb = b.notebook().expect("caderno");
        assert_eq!(nb.pictures().len(), 1);
    }

    /// Com os dados reais: as 28 figuras do Max e o desenho da tela
    /// (`$ATELIE_DUMP/ideas_board.ppm`).
    #[test]
    fn real_board_draws() {
        let Some(a) = assets() else { return };
        let mut b = PictureBoard::new(None, "quadroreal", &a).with_seed(4);
        assert_eq!(b.board().len(), 28);
        b.update(&[], 0.0, &a);
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "ideas_board");
        b.talk = None;
        b.list = Some(0);
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "ideas_board_list");
    }
}
