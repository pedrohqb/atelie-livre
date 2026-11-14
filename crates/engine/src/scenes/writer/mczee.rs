//! McZee do Escritório (`office.md` §5): balões automáticos de cada
//! ferramenta e opção (`WHAT 0fa0`–`0faa` do `WRITER.EXE`), o menu de 8
//! botões do nicho (o mesmo da pintura, `paint::menu::WRITER`) e o
//! ligar/desligar da ajuda.
//!
//! As ferramentas pedem balões com [`Cx::help`](super::tools::Cx::help); o
//! Writer repassa o pedido a [`McZee::show`], que só abre o balão se o McZee
//! estiver ligado.

use super::tools::{HelpRef, SND_CHOOSE, WHAT_GENERAL, inside};
use crate::Frame;
use crate::assets::{Assets, Program};
use crate::balloon::Balloon;
use crate::help::HelpBalloon;
use crate::scene::{Effect, SceneId};
use crate::scenes::paint::menu::{self, Item};
use std::collections::HashMap;
use std::rc::Rc;

/// Nicho do McZee à direita da faixa: clique abre o menu.
pub const NICHE: (i32, i32, i32, i32) = (533, 47, 47, 57);
/// McZee no nicho.
pub const MCZEE: super::tools::Pic = super::tools::Pic::masked(0x0235, 0x0237, 536, 47);
/// Canto de cima dos balões: encostados à direita (borda em x 629) logo
/// abaixo da linha preta da faixa (borda de cima em y 108,
/// `office_03_palavra.png`).
const HELP_RIGHT: i32 = 629;
const HELP_Y: i32 = 108;
/// Ponta do rabicho: embaixo do McZee.
const HELP_TAIL: (i32, i32) = (545, 101);
/// Balões de ligar e desligar o McZee, Varinha Mágica.
const WHAT_ON: usize = 0;
const WHAT_OFF: usize = 1;
const WHAT_WAND: usize = 7;
/// Ditados e piadas do McZee (cara de sol do menu): `WHAT 0201` do Writer.
const WHAT_JOKES: u16 = 0x0201;
const JOKE_POS: (i32, i32, i32) = (218, 197, 200);
/// Abrir o menu do McZee.
const SND_MENU: u16 = 0x02ce;
/// Balão de boas-vindas e som da entrada no Escritório.
pub const WELCOME: HelpRef = (super::tools::WHAT_WORD, 0);
pub const SND_ENTER: u16 = 0x0304;

/// Balão de opções aberto (o que as bolinhas fazem).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Talk {
    /// "OK, vou embora se você quiser mesmo que eu vá."
    Dismiss,
    /// "Oi! Aqui está o McZee…" (volta a ajudar).
    Return,
    /// Piada: qualquer clique fecha.
    Joke,
}

/// O que o menu pediu ao Writer (o resto o McZee resolve sozinho).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    New,
    Open,
    Save,
    Notebook,
}

pub struct McZee {
    /// Ajuda ligada (balões automáticos); "dê o fora" desliga.
    help_on: bool,
    help: Option<HelpBalloon>,
    /// Balão principal que continua aberto ao lado do sub-balão de uma
    /// ligação (`help`), com a identidade do sub a que pertence.
    help_parent: Option<(HelpBalloon, (u16, usize, i32, i32))>,
    menu_open: bool,
    talk: Option<(Balloon, Talk)>,
    /// Varinha Mágica na mão: o próximo clique toca o som da palavra ou adesivo.
    wand: bool,
    dialogs: HashMap<u16, Rc<Vec<formats::what::Record>>>,
}

impl Default for McZee {
    fn default() -> Self {
        McZee {
            help_on: true,
            help: None,
            help_parent: None,
            menu_open: false,
            talk: None,
            wand: false,
            dialogs: HashMap::new(),
        }
    }
}

impl McZee {
    pub fn help_on(&self) -> bool {
        self.help_on
    }

    pub fn menu_open(&self) -> bool {
        self.menu_open
    }

    pub fn wand(&self) -> bool {
        self.wand
    }

    pub fn set_wand(&mut self, on: bool) {
        self.wand = on;
    }

    /// Balão de ajuda aberto (bloco e registro).
    pub fn help(&self) -> Option<HelpRef> {
        self.help.as_ref().map(|h| (h.block, h.index))
    }

    pub fn talk(&self) -> Option<Talk> {
        self.talk.as_ref().map(|(_, t)| *t)
    }

    /// Retângulo da bolinha `k` do balão de opções aberto.
    pub fn talk_option(&self, a: &Assets, k: usize) -> Option<(i32, i32, i32, i32)> {
        let (b, _) = self.talk.as_ref()?;
        (k < b.options.len()).then(|| b.option_rect(a, k))
    }

    fn records(&mut self, block: u16, a: &Assets) -> Rc<Vec<formats::what::Record>> {
        self.dialogs
            .entry(block)
            .or_insert_with(|| Rc::new(a.dialogs(Program::Writer, block)))
            .clone()
    }

    /// Abre o balão `h` (se a ajuda estiver ligada ou `forced`). O registro
    /// `usize::MAX` fecha o balão aberto.
    pub fn show(&mut self, h: HelpRef, forced: bool, a: &Assets) {
        if h.1 == usize::MAX {
            self.close();
            return;
        }
        if !self.help_on && !forced {
            return;
        }
        self.open(h, a);
    }

    fn open(&mut self, (block, index): HelpRef, a: &Assets) {
        let records = self.records(block, a);
        self.help_parent = None;
        self.help = HelpBalloon::new(records, block, index, 0, HELP_Y).map(|b| {
            let mut b = b.in_program(Program::Writer);
            b.x = (HELP_RIGHT - b.w).max(5);
            b.tail = Some(HELP_TAIL);
            b
        });
    }

    /// Fecha os balões de ajuda.
    pub fn close(&mut self) {
        self.help = None;
        self.help_parent = None;
    }

    fn help_parent(&self) -> Option<&HelpBalloon> {
        let (parent, key) = self.help_parent.as_ref()?;
        (self.help.as_ref()?.key() == *key).then_some(parent)
    }

    fn set_sub_help(&mut self, parent: HelpBalloon, sub: HelpBalloon) {
        self.help_parent = Some((parent, sub.key()));
        self.help = Some(sub);
    }

    /// Cliques que vêm antes de tudo: balão de opções, menu aberto e balões
    /// de ajuda (ligações abrem sub-balões ao lado). `Some(..)` = consumido
    /// (com o pedido do menu ao Writer, se houver); clique fora dos balões
    /// os fecha e devolve `None` (segue).
    pub fn front_click(
        &mut self,
        x: i32,
        y: i32,
        fx: &mut Vec<Effect>,
        a: &Assets,
    ) -> Option<Option<Request>> {
        if let Some((b, kind)) = self.talk.take() {
            if b.contains(a, x, y) {
                match b.hit(a, x, y) {
                    Some(k) => self.talk_choice(kind, k, fx),
                    None if kind == Talk::Joke => {}
                    None => self.talk = Some((b, kind)),
                }
                return Some(None);
            }
            if kind == Talk::Joke {
                // a piada fecha com qualquer clique
                return Some(None);
            }
        }
        if self.menu_open && menu::WRITER.contains(x, y) {
            if let Some(item) = menu::WRITER.hit(x, y) {
                self.menu_open = false;
                self.close();
                return Some(self.menu_item(item, fx, a));
            }
            return Some(None);
        }
        self.help_click(x, y, a).then_some(None)
    }

    /// Clique com balão de ajuda aberto (como na pintura): ligação abre o
    /// registro ligado num sub-balão ao lado; fora dos balões fecha e segue.
    fn help_click(&mut self, x: i32, y: i32, a: &Assets) -> bool {
        let parent = self.help_parent().cloned();
        self.help_parent = None;
        let Some(h) = self.help.take() else {
            return false;
        };
        if h.contains(a, x, y) {
            match (h.link_at(a, x, y), parent) {
                (Some(t), Some(p)) => match h.follow(t) {
                    Some(sub) => self.set_sub_help(p, sub),
                    None => self.set_sub_help(p, h),
                },
                (Some(t), None) => match h.follow_beside(t) {
                    Some(sub) => self.set_sub_help(h, sub),
                    None => self.help = Some(h),
                },
                (None, Some(p)) => self.set_sub_help(p, h),
                (None, None) => self.help = Some(h),
            }
            return true;
        }
        if let Some(p) = parent.as_ref()
            && p.contains(a, x, y)
        {
            let p = p.clone();
            match p.link_at(a, x, y).and_then(|t| p.follow_beside(t)) {
                Some(sub) => self.set_sub_help(p, sub),
                None => self.help = Some(p),
            }
            return true;
        }
        false
    }

    /// Clique no nicho: abre ou fecha o menu; com o McZee desligado, ele
    /// pergunta se deve voltar.
    pub fn niche_click(&mut self, fx: &mut Vec<Effect>, a: &Assets) {
        fx.push(super::tools::snd(SND_MENU));
        self.close();
        if self.help_on {
            self.menu_open = !self.menu_open;
        } else {
            self.open_talk(WHAT_GENERAL, WHAT_ON, Talk::Return, a);
        }
    }

    fn open_talk(&mut self, block: u16, k: usize, kind: Talk, a: &Assets) {
        let records = self.records(block, a);
        let name = crate::scenes::room::DEFAULT_USER;
        let fallback = Balloon {
            x: 0,
            y: HELP_Y,
            w: 170,
            text: String::new(),
            options: Vec::new(),
            links: Vec::new(),
            figures: Vec::new(),
            rich: None,
            tail: None,
        };
        let mut b = Balloon::from_block(&records, k, 0, HELP_Y, name)
            .map(|b| b.in_program(Program::Writer))
            .unwrap_or(fallback);
        if b.text.is_empty() && b.options.is_empty() {
            let (text, options) = match kind {
                Talk::Dismiss => (
                    "OK, vou embora se você quiser mesmo que eu vá.",
                    vec!["É, por favor dê o fora", "Não, preciso de mais ajuda!"],
                ),
                Talk::Return => (
                    "Oi! Aqui está o McZee para resolver todos os problemas!",
                    vec!["Sim, por favor!", "Não, obrigado!"],
                ),
                Talk::Joke => ("Por que é que o elefante fica vesgo?", vec![]),
            };
            b.text = text.to_string();
            b.options = options.into_iter().map(str::to_string).collect();
        }
        if kind == Talk::Joke {
            (b.x, b.y, b.w) = JOKE_POS;
            b.options.clear();
        } else {
            b.x = (HELP_RIGHT - b.w).max(5);
        }
        self.close();
        self.talk = Some((b, kind));
    }

    /// Bolinha `k` de um balão de opções.
    fn talk_choice(&mut self, kind: Talk, k: usize, fx: &mut Vec<Effect>) {
        fx.push(super::tools::snd(SND_CHOOSE));
        match (kind, k) {
            (Talk::Dismiss, 0) => {
                self.help_on = false;
                self.close();
                self.menu_open = false;
            }
            (Talk::Return, 0) => self.help_on = true,
            _ => {}
        }
    }

    /// Item escolhido no menu: o que é do McZee resolve aqui, o resto
    /// (novo, pasta, salvar, caderno) volta como pedido ao Writer.
    fn menu_item(&mut self, item: Item, fx: &mut Vec<Effect>, a: &Assets) -> Option<Request> {
        fx.push(super::tools::snd(SND_CHOOSE));
        match item {
            Item::Dismiss => self.open_talk(WHAT_GENERAL, WHAT_OFF, Talk::Dismiss, a),
            Item::New => return Some(Request::New),
            Item::Open => return Some(Request::Open),
            Item::Save => return Some(Request::Save),
            Item::Notebook => return Some(Request::Notebook),
            Item::Wand => {
                self.wand = true;
                self.open((WHAT_GENERAL, WHAT_WAND), a);
            }
            Item::Joke => {
                let n = self.records(WHAT_JOKES, a).len().max(1);
                let k = crate::rng::XorShift::from_clock().below(n);
                self.open_talk(WHAT_JOKES, k, Talk::Joke, a);
            }
            Item::Story => fx.push(Effect::Push(SceneId::BookStory)),
            // a cartola é só do Fine Artist
            Item::Tricks => {}
        }
        None
    }

    /// O ponto está sobre algum balão ou o menu aberto?
    pub fn covers(&self, a: &Assets, x: i32, y: i32) -> bool {
        self.help.as_ref().is_some_and(|h| h.contains(a, x, y))
            || self.help_parent().is_some_and(|h| h.contains(a, x, y))
            || self.talk.as_ref().is_some_and(|(b, _)| b.contains(a, x, y))
            || (self.menu_open && menu::WRITER.contains(x, y))
    }

    pub fn draw_niche(&self, f: &mut Frame, a: &Assets) {
        MCZEE.draw(f, a);
    }

    /// Menu, balões de ajuda e balão de opções, por cima de tudo.
    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        if self.menu_open {
            menu::WRITER.draw(f, a);
        }
        if let Some(p) = self.help_parent() {
            p.draw(f, a);
        }
        if let Some(h) = &self.help {
            h.draw(f, a);
        }
        if let Some((b, _)) = &self.talk {
            b.draw(f, a);
        }
    }
}

/// O clique caiu no nicho do McZee?
pub fn niche_hit(x: i32, y: i32) -> bool {
    inside(NICHE, x, y)
}
