//! Máquina de Idéias (close-up da Oficina de Idéias, `ideas.md` §3): três
//! janelas com pedaços de sentença sorteados (`writer::ideas::Machine`),
//! alavanca-peixe que gira as três por 1000 ms, botões redondos que
//! sorteiam uma janela, POE (escrever uma entrada própria, centralizada,
//! Enter termina), TIRA (`0805[7]`/`[9]`) e FEITO (`0805[40]`: escrever a
//! história, guardar no Caderno, fechar ou continuar). Ctrl + clique na
//! alavanca ou num botão redondo sorteia só as entradas do usuário; Shift,
//! só as figuras.

use super::buttons::{Art, PressButton, Presser, hit};
use super::notebook::{NotebookView, Outcome};
use super::{
    Instructions, MAX_RECT, Owner, draw_backdrop, max_balloon, plain_balloon, play, rex_alert,
    room, snd,
};
use crate::Frame;
use crate::assets::Assets;
use crate::balloon::{Balloon, line_height, wrap_opt};
use crate::rng::XorShift;
use crate::scene::{Effect, Input, Key, KidCommon, MouseButton, Scene};
use crate::text_balloon::TextBalloon;
use crate::ui::{Pen, Rect, inside};
use crate::writer::ideas::machine::{Grammar, Machine, Mode, Slot, WINDOWS, WindowRule};
use crate::writer::ideas::notes::NoteText;
use crate::writer::ideas::story::IdeaStory;
use crate::writer::ideas::{SPIN_MS, session, store};

/// Corpo da Máquina (placa "IDÉIAS", visor, janelas) e máscara.
const BODY: (Art, i32, i32) = (Art::masked(0x69bf, 0x7531), 160, 110);
/// Alavanca-peixe em repouso e puxada.
const LEVER: Rect = (423, 148, 72, 114);
const LEVER_UP: Art = Art::masked(0x754b, 0x754d);
const LEVER_DOWN: Art = Art::masked(0x754f, 0x7551);
/// Quanto tempo o peixe fica puxado depois de soltar.
const LEVER_HOLD: f32 = 0.3;
/// Passo do giro (o original sorteia a cada quadro; ≈1/15 s).
const SPIN_STEP: f32 = 1.0 / 15.0;

const DONE: PressButton = PressButton::new(
    333,
    176,
    64,
    27,
    Art::masked(0x753b, 0x753d),
    Art::masked(0x7561, 0x7565),
);
const ROUND: PressButton = PressButton::new(
    373,
    242,
    31,
    31,
    Art::masked(0x753f, 0x7541),
    Art::masked(0x7567, 0x7541),
);
const POE: PressButton = PressButton::new(
    175,
    237,
    24,
    24,
    Art::masked(0x7543, 0x7545),
    Art::masked(0x7569, 0x7545),
);
const TIRA: PressButton = PressButton::new(
    176,
    265,
    28,
    24,
    Art::masked(0x7547, 0x7549),
    Art::masked(0x756d, 0x7549),
);
/// Distância vertical entre as linhas de botões das janelas.
const ROW: i32 = 63;
/// Área útil de cada janela de texto.
pub const WINDOW_RECTS: [Rect; WINDOWS] = [
    (204, 232, 162, 60),
    (204, 296, 162, 58),
    (204, 358, 162, 60),
];

/// Balões: `0bb8[46]` (chegada), `[47]` (instruções); `0805[7]`, `[9]`, `[40]`.
const ARRIVAL: (usize, (i32, i32), i32) = (46, (70, 253), 169);
const ARRIVAL_TEXT: &str = "Pode brincar com a Máquina de Idéias! Tenho certeza de que ela vai te ajudar com alguma expressão maneira para você começar a sua história!";
const INSTRUCTIONS: Instructions = Instructions {
    x: 146,
    y: 94,
    w: 350,
    h: 290,
    title: "Usando a Máquina de Idéias — clique para continuar",
    pic: 0x0c3f,
};
const ALERT_POS: (i32, i32) = (208, 172);
const WHAT_NOW_POS: (i32, i32) = (177, 172);

/// Botões da tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    Lever,
    Done,
    Round(usize),
    Poe(usize),
    Tira(usize),
}

fn buttons() -> Vec<(Btn, PressButton)> {
    let mut v = vec![(Btn::Done, DONE)];
    for w in 0..WINDOWS {
        let dy = ROW * w as i32;
        v.push((Btn::Round(w), ROUND.at(ROUND.x, ROUND.y + dy)));
        v.push((Btn::Poe(w), POE.at(POE.x, POE.y + dy)));
        v.push((Btn::Tira(w), TIRA.at(TIRA.x, TIRA.y + dy)));
    }
    v
}

/// Balão aberto.
enum Talk {
    /// "Pode brincar com a Máquina de Idéias!" (`0bb8[46]`).
    Arrival(Balloon),
    /// "Usando a Máquina de Idéias" (`0bb8[47]`).
    Instructions,
    /// "Tem certeza de que quer jogar fora esta entrada?" (`0805[7]`).
    Remove(usize, TextBalloon),
    /// "Você não pode jogar fora as coisas do Max!" (`0805[9]`).
    MaxEntry(TextBalloon),
    /// "O que você quer fazer em seguida?" (`0805[40]`).
    WhatNow(Balloon),
}

/// Gramática mínima quando falta o `WORKSHOP.CHK` (testes sem os dados).
fn fallback_grammar() -> Grammar {
    let rule = |t: &[&str], format: &str| WindowRule {
        tables: vec![t.iter().map(|s| s.to_string()).collect()],
        format: format.into(),
        figures: vec![],
    };
    Grammar {
        windows: [
            rule(&["O pirata Zeca,"], "%s"),
            rule(&["foi caminhar"], "%s"),
            rule(&["sobre uma dobra do tempo."], "%s"),
        ],
    }
}

pub struct IdeaMachine {
    common: KidCommon,
    owner: Owner,
    machine: Machine,
    rng: XorShift,
    ctrl: bool,
    shift: bool,
    presser: Presser<Btn>,
    /// Giro da alavanca: tempo que falta, tempo até o próximo passo e modo.
    spin: Option<(f32, f32, Mode)>,
    /// O peixe ainda aparece puxado (segundos).
    lever_shown: f32,
    /// POE em andamento: a janela e o texto.
    editing: Option<(usize, NoteText)>,
    talk: Option<Talk>,
    notebook: Option<NotebookView>,
    /// As entradas do usuário mudaram (gravar ao sair).
    dirty: bool,
    started: bool,
}

impl IdeaMachine {
    /// Máquina do usuário: a da sessão, se ele já a abriu; senão uma nova
    /// com as entradas gravadas e a gramática do `WORKSHOP.CHK`.
    pub fn new(users: Option<&std::path::Path>, user: &str, assets: &Assets) -> IdeaMachine {
        let owner = Owner::new(users, user);
        let mut rng = XorShift::from_clock();
        let machine = session::machine(user).unwrap_or_else(|| {
            let grammar = Grammar::load(assets.data_dir()).unwrap_or_else(|_| fallback_grammar());
            let entries = owner
                .dir
                .as_deref()
                .map(store::load_machine_entries)
                .unwrap_or_default();
            Machine::new(grammar, entries, &mut rng)
        });
        IdeaMachine {
            common: KidCommon::new(None),
            owner,
            machine,
            rng,
            ctrl: false,
            shift: false,
            presser: Presser::default(),
            spin: None,
            lever_shown: 0.0,
            editing: None,
            talk: None,
            notebook: None,
            dirty: false,
            started: false,
        }
    }

    /// Sorteio fixo (testes e capturas).
    pub fn with_seed(mut self, seed: u32) -> IdeaMachine {
        self.rng = XorShift::new(seed);
        self
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn spinning(&self) -> bool {
        self.spin.is_some()
    }

    pub fn editing(&self) -> Option<usize> {
        self.editing.as_ref().map(|(w, _)| *w)
    }

    pub fn notebook(&self) -> Option<&NotebookView> {
        self.notebook.as_ref()
    }

    /// Nome do balão aberto (testes e rastro).
    pub fn talk(&self) -> Option<&'static str> {
        self.talk.as_ref().map(|t| match t {
            Talk::Arrival(_) => "chegada",
            Talk::Instructions => "instrucoes",
            Talk::Remove(..) => "tira",
            Talk::MaxEntry(_) => "coisa-do-max",
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
            &["Instruções da Máquina de Idéias"],
        );
        self.talk = Some(Talk::Arrival(b));
    }

    fn mode(&self) -> Mode {
        Mode::from_keys(self.ctrl, self.shift)
    }

    fn finish_editing(&mut self) {
        if let Some((w, t)) = self.editing.take()
            && self.machine.add_user(w, &t.text())
        {
            self.dirty = true;
        }
    }

    fn press(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        if self.spin.is_some() {
            return;
        }
        let list = buttons();
        let target = hit(&list, x, y).or_else(|| inside(LEVER, x, y).then_some(Btn::Lever));
        // um clique fora da janela em edição termina a entrada
        if self.editing.is_some() && target != self.editing.as_ref().map(|(w, _)| Btn::Poe(*w)) {
            self.finish_editing();
        }
        match target {
            Some(b) => self.presser.press(b),
            None if inside(MAX_RECT, x, y) => {
                fx.push(play(snd::OBJECT));
                self.arrival(a);
            }
            None => {}
        }
    }

    fn release(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        let list = buttons();
        let under = hit(&list, x, y).or_else(|| inside(LEVER, x, y).then_some(Btn::Lever));
        let Some(b) = self.presser.release(under) else {
            return;
        };
        match b {
            Btn::Lever => {
                fx.push(play(snd::MACHINE));
                self.spin = Some((SPIN_MS as f32 / 1000.0, 0.0, self.mode()));
                self.lever_shown = LEVER_HOLD;
            }
            Btn::Round(w) => {
                fx.push(play(snd::ROUND));
                let mode = self.mode();
                self.machine.next(w, &mut self.rng, mode);
            }
            Btn::Poe(w) => {
                fx.push(play(snd::POE));
                self.editing = Some((w, NoteText::default()));
            }
            Btn::Tira(w) => {
                fx.push(play(snd::REX));
                fx.push(play(snd::EPA));
                let user = matches!(self.machine.slot(w), Slot::User(_));
                self.talk = Some(if user {
                    Talk::Remove(
                        w,
                        rex_alert(
                            a,
                            7,
                            ALERT_POS,
                            220,
                            "Tem certeza de que quer jogar fora esta entrada?",
                            &["Não, eu quero manter!", "É, quero jogar fora!"],
                        ),
                    )
                } else {
                    Talk::MaxEntry(rex_alert(
                        a,
                        9,
                        ALERT_POS,
                        199,
                        "Você não pode jogar fora as coisas do Max!\nVocê só pode jogar fora as entradas que você mesmo criou.",
                        &[],
                    ))
                });
            }
            Btn::Done => {
                fx.push(play(snd::DONE));
                self.talk = Some(Talk::WhatNow(plain_balloon(
                    a,
                    40,
                    WHAT_NOW_POS,
                    283,
                    "O que você quer fazer em seguida?",
                    &[
                        "Escrever uma história com essa sentença",
                        "Colocar essa sentença no Caderno de Idéias",
                        "Fechar a Máquina de Idéias",
                        "Ops! Quero continuar trabalhando na Máquina de Idéias",
                    ],
                )));
            }
        }
    }

    /// Clique com um balão aberto.
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
            Talk::Instructions => fx.push(play(snd::BALLOON)),
            Talk::Remove(w, b) => match b.hit_option(a, x, y) {
                Some(1) => {
                    fx.push(play(snd::CHOICE));
                    if self.machine.remove_user(w, &mut self.rng).is_ok() {
                        self.dirty = true;
                    }
                }
                Some(_) => fx.push(play(snd::CHOICE)),
                None => self.talk = Some(Talk::Remove(w, b)),
            },
            Talk::MaxEntry(_) => fx.push(play(snd::BALLOON)),
            Talk::WhatNow(b) => match b.hit(a, x, y) {
                Some(0) => {
                    fx.push(play(snd::CHOICE));
                    fx.push(play(snd::EDITOR));
                    let story = IdeaStory::from_sentence(&self.machine.sentence());
                    let id = story.scene();
                    session::set_story(&self.owner.user, story);
                    fx.push(Effect::ChangeScene(id));
                }
                Some(1) => {
                    fx.push(play(snd::CHOICE));
                    fx.push(play(snd::NOTEBOOK));
                    let mut nb = NotebookView::open(&self.owner, a);
                    nb.add_sentence(self.machine.sentence());
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

    fn key(&mut self, key: Key) {
        let Some((_, t)) = self.editing.as_mut() else {
            return;
        };
        match key {
            Key::Enter => self.finish_editing(),
            Key::Backspace => t.backspace(),
            Key::Delete => t.delete(),
            Key::Left => t.left(),
            Key::Right => t.right(),
            _ => {}
        }
    }

    fn step(&mut self, dt: f32) {
        self.lever_shown = (self.lever_shown - dt).max(0.0);
        let Some((left, next, mode)) = self.spin.as_mut() else {
            return;
        };
        *left -= dt;
        *next -= dt;
        let mode = *mode;
        if *next <= 0.0 || *left <= 0.0 {
            *next += SPIN_STEP;
            let done = *left <= 0.0;
            self.machine.spin(&mut self.rng, mode);
            if done {
                self.spin = None;
            }
        }
    }

    fn draw_window(&self, f: &mut Frame, a: &Assets, w: usize) {
        let r = WINDOW_RECTS[w];
        let pen = Pen::new(a);
        let lh = line_height(pen.font());
        let (cx, cy) = (r.0 + r.2 / 2, r.1 + r.3 / 2);
        let text = match &self.editing {
            Some((ew, t)) if *ew == w => Some((t.text(), true)),
            _ => match self.machine.slot(w) {
                Slot::Rebus(c) => {
                    if let Some(img) = a.rebus(*c) {
                        f.blit(&img, cx - img.width as i32 / 2, cy - img.height as i32 / 2);
                    }
                    None
                }
                _ => self
                    .machine
                    .window_text(w)
                    .map(|t| (t.trim().to_string(), false)),
            },
        };
        let Some((text, caret)) = text else {
            return;
        };
        let lines = wrap_opt(pen.font(), &text, (r.2 - 10) as u32);
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        let top = cy - lh * lines.len() as i32 / 2;
        for (k, l) in lines.iter().enumerate() {
            pen.centered(f, cx, top + lh * k as i32, l, [0, 0, 0]);
        }
        if caret {
            let last = lines.last().map(String::as_str).unwrap_or("");
            let x = cx + pen.width(last) / 2 + 1;
            let y = top + lh * (lines.len() as i32 - 1);
            f.rect(x, y, 1, lh, [0, 0, 0]);
        }
    }
}

impl Scene for IdeaMachine {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        if !self.started {
            self.started = true;
            fx.push(play(snd::CLOSE_UP));
            fx.push(play(snd::MACHINE));
            self.arrival(assets);
        }
        self.step(dt);
        for i in input {
            match i {
                Input::Ctrl(on) => self.ctrl = *on,
                Input::Shift(on) => self.shift = *on,
                _ => {}
            }
            if let Some(nb) = self.notebook.as_mut() {
                if let Some(out) = nb.handle(i, &mut fx, assets) {
                    self.notebook = None;
                    if out == Outcome::Ok {
                        fx.push(Effect::Log("caderno gravado".into()));
                    }
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
                    } else {
                        self.press(*x, *y, &mut fx, assets);
                    }
                }
                Input::MouseUp {
                    button: MouseButton::Left,
                    x,
                    y,
                } => self.release(*x, *y, &mut fx, assets),
                Input::Key { key, ctrl, .. } => {
                    self.ctrl = *ctrl;
                    self.key(*key);
                }
                Input::Text(s) => {
                    if let Some((_, t)) = self.editing.as_mut() {
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
        let (body, bx, by) = BODY;
        body.draw(f, a, bx, by);
        let pulled = self.presser.is_down(Btn::Lever) || self.lever_shown > 0.0;
        let lever = if pulled { LEVER_DOWN } else { LEVER_UP };
        lever.draw(f, a, LEVER.0, LEVER.1);
        for (k, b) in buttons() {
            b.draw(f, a, self.presser.is_down(k));
        }
        for w in 0..WINDOWS {
            self.draw_window(f, a, w);
        }
        match &self.talk {
            Some(Talk::Arrival(b)) | Some(Talk::WhatNow(b)) => b.draw(f, a),
            Some(Talk::Instructions) => INSTRUCTIONS.draw(f, a),
            Some(Talk::Remove(_, b)) | Some(Talk::MaxEntry(b)) => b.draw(f, a),
            None => {}
        }
        if let Some(nb) = &self.notebook {
            nb.draw(f, a);
        }
    }

    fn leave(&mut self) -> Vec<Effect> {
        self.finish_editing();
        session::set_machine(&self.owner.user, &self.machine);
        let mut fx = Vec::new();
        if self.dirty
            && let Some(dir) = &self.owner.dir
        {
            match store::save_machine_entries(dir, self.machine.all_user_entries()) {
                Ok(()) => self.dirty = false,
                Err(e) => fx.push(Effect::Log(format!("Máquina de Idéias: {e}"))),
            }
        }
        fx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{CwScene, SceneId};
    use crate::scenes::testutil::{assets, empty_assets};
    use crate::ui::center;

    fn click(m: &mut IdeaMachine, a: &Assets, (x, y): (i32, i32)) -> Vec<Effect> {
        let mut fx = m.update(
            &[Input::Click {
                button: MouseButton::Left,
                x,
                y,
            }],
            0.0,
            a,
        );
        fx.extend(m.update(
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

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(crate::assets::SoundRef::Snd(_, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    fn opened(a: &Assets, user: &str) -> IdeaMachine {
        let mut m = IdeaMachine::new(None, user, a).with_seed(5);
        let fx = m.update(&[], 0.0, a);
        assert_eq!(sounds(&fx), [snd::CLOSE_UP, snd::MACHINE]);
        assert_eq!(m.talk(), Some("chegada"));
        // fecha o balão de chegada clicando fora dele
        click(&mut m, a, (600, 20));
        assert_eq!(m.talk(), None);
        m
    }

    fn round(w: usize) -> (i32, i32) {
        center(ROUND.at(ROUND.x, ROUND.y + ROW * w as i32).rect())
    }

    #[test]
    fn lever_spins_for_a_second_then_stops() {
        let (_d, a) = empty_assets();
        let mut m = opened(&a, "maq1");
        let fx = click(&mut m, &a, center(LEVER));
        assert_eq!(sounds(&fx), [snd::MACHINE]);
        assert!(m.spinning());
        for _ in 0..59 {
            m.update(&[], 1.0 / 60.0, &a);
        }
        assert!(m.spinning());
        m.update(&[], 2.0 / 60.0, &a);
        assert!(!m.spinning());
    }

    #[test]
    fn release_outside_the_button_cancels() {
        let (_d, a) = empty_assets();
        let mut m = opened(&a, "maq2");
        let (x, y) = round(1);
        m.update(
            &[Input::Click {
                button: MouseButton::Left,
                x,
                y,
            }],
            0.0,
            &a,
        );
        let fx = m.update(
            &[Input::MouseUp {
                button: MouseButton::Left,
                x: 10,
                y: 10,
            }],
            0.0,
            &a,
        );
        assert!(sounds(&fx).is_empty());
        let fx = click(&mut m, &a, (x, y));
        assert_eq!(sounds(&fx), [snd::ROUND]);
    }

    #[test]
    fn poe_types_an_entry_and_tira_asks_before_throwing_it_away() {
        let (_d, a) = empty_assets();
        let mut m = opened(&a, "maq3");
        let poe = center(POE.at(POE.x, POE.y + ROW).rect());
        assert_eq!(sounds(&click(&mut m, &a, poe)), [snd::POE]);
        assert_eq!(m.editing(), Some(1));
        m.update(&[Input::Text("dançou".into())], 0.0, &a);
        m.update(
            &[Input::Key {
                key: Key::Enter,
                ctrl: false,
                alt: false,
            }],
            0.0,
            &a,
        );
        assert_eq!(m.editing(), None);
        assert_eq!(m.machine().user_entries(1), ["dançou"]);
        assert_eq!(m.machine().window_text(1), Some("dançou"));
        // TIRA: "Tem certeza…?"; "É, quero jogar fora!"
        let tira = center(TIRA.at(TIRA.x, TIRA.y + ROW).rect());
        assert_eq!(sounds(&click(&mut m, &a, tira)), [snd::REX, snd::EPA]);
        assert_eq!(m.talk(), Some("tira"));
        let Some(Talk::Remove(_, b)) = &m.talk else {
            unreachable!()
        };
        let yes = center(b.option_rect(&a, 1).unwrap());
        click(&mut m, &a, yes);
        assert!(m.machine().user_entries(1).is_empty());
        // coisa do Max: "Você não pode…"
        click(&mut m, &a, tira);
        assert_eq!(m.talk(), Some("coisa-do-max"));
    }

    #[test]
    fn done_writes_a_story_with_the_sentence() {
        let (_d, a) = empty_assets();
        let mut m = opened(&a, "maq4");
        click(&mut m, &a, center(DONE.rect()));
        assert_eq!(m.talk(), Some("feito"));
        let Some(Talk::WhatNow(b)) = &m.talk else {
            unreachable!()
        };
        let write = center(b.option_rect(&a, 0));
        let fx = click(&mut m, &a, write);
        let id = SceneId::Cw(CwScene::IdeaStory(
            crate::writer::ideas::story::IdeaSource::Machine,
        ));
        assert!(fx.contains(&Effect::ChangeScene(id)));
        let story = session::take_story("maq4").expect("história pendente");
        assert_eq!(
            story.doc.text(),
            "O pirata Zeca, foi caminhar sobre uma dobra do tempo."
        );
    }

    #[test]
    fn done_can_close_or_open_the_notebook() {
        let (_d, a) = empty_assets();
        let mut m = opened(&a, "maq5");
        click(&mut m, &a, center(DONE.rect()));
        let Some(Talk::WhatNow(b)) = &m.talk else {
            unreachable!()
        };
        let (close, keep) = (center(b.option_rect(&a, 2)), center(b.option_rect(&a, 1)));
        let fx = click(&mut m, &a, close);
        assert!(fx.contains(&Effect::ChangeScene(room())));
        click(&mut m, &a, center(DONE.rect()));
        click(&mut m, &a, keep);
        let nb = m.notebook().expect("caderno aberto");
        assert_eq!(nb.sentences().len(), 1);
    }

    #[test]
    fn sentence_persists_for_the_session() {
        let (_d, a) = empty_assets();
        let mut m = opened(&a, "maq6");
        click(&mut m, &a, round(0));
        let before = m.machine().sentence();
        m.leave();
        let again = IdeaMachine::new(None, "maq6", &a);
        assert_eq!(again.machine().sentence(), before);
    }

    /// Com os dados reais: a gramática do `WORKSHOP.CHK`, as figuras e os
    /// balões; grava o quadro em `$ATELIE_DUMP/ideas_machine.ppm`.
    #[test]
    fn real_machine_draws() {
        let Some(a) = assets() else { return };
        let mut m = IdeaMachine::new(None, "maqreal", &a).with_seed(3);
        m.update(&[], 0.0, &a);
        assert_eq!(m.machine().grammar().windows[0].figures.len(), 63);
        let Some(Talk::Arrival(b)) = &m.talk else {
            panic!("sem balão de chegada")
        };
        assert!(b.text.starts_with("Pode brincar"), "{}", b.text);
        let mut f = Frame::new(640, 480);
        m.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "ideas_machine");
        // o corpo da máquina está desenhado (placa IDÉIAS em cima)
        assert_ne!(f.pixel(290, 125), f.pixel(5, 5));
    }
}
