//! Salas do prédio do Fine Artist: saguão, Ateliê de Pintura, Oficina de
//! Projetos, Truques de Desenho e o sótão escuro; e as do Creative Writer
//! (saguão, Escritório, Oficinas de Projetos e de Ideias, sótão), com a
//! mesma mecânica. Tudo o que muda de sala para sala (fundo, elementos,
//! áreas clicáveis, balões, personagem, bolsa…) é um `RoomSpec`
//! (`room_spec`): os do Fine Artist em `rooms_data`, os do Writer em
//! `writer_rooms` e vizinhos. Em
//! cada andar há um personagem parado à esquerda (McZee com uma fantasia
//! sorteada no saguão e no Ateliê; Maggie nos andares 3 e 4) e, de tempos em
//! tempos, uma animação ambiente do andar (`gags`). Os objetos do chão
//! (Bolsa Mágica, maleta, livro, Varinha Mágica) estão em `room_objects`; o
//! elevador, em `elevator`; os balões, em `room_talk`; o sótão, em `attic`.
//!
//! Como no original, as opções do balão de chegada só apontam (abrem uma
//! dica sobre o objeto); os objetos da Oficina e dos Truques abrem um balão
//! de escolha, que Ctrl+clique pula; o cavalete e o quadro da parede do
//! Ateliê mostram a dica no clique simples e abrem a pintura no duplo; o
//! mastro de bombeiro dá a volta por cinco posições (saguão, Ateliê,
//! Projetos, Truques, sótão): o mastro sobe uma, a abertura no chão desce
//! uma e Ctrl anda duas.

use super::attic;
use super::elevator::{Elevator, Outcome};
use super::gags::{Character, GagPlayer};
use super::projects::ProjectKind;
use super::room_objects::{BagPart, MagicBag, Target};
use super::room_spec::{self, Action, LastWork, RoomKit, RoomSpec, RoomState, Rooms, WorkPlace};
use super::room_talk::{self as talk, Helper, Pick, Start};
use super::studio_wall::{self, End, NameField};
use crate::anim::Animator;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::Balloon;
use crate::paint::Canvas;
use crate::paint::store::{LAST_NAME, load_name, load_png};
use crate::rng::XorShift;
use crate::scene::{DEFAULT_CURSOR, Effect, Input, Key, KidCommon, MouseButton, Scene, SceneId};
use crate::text_balloon::TextBalloon;
use crate::ui::DoubleClick;
use crate::ui::inside;
use crate::{Frame, SCREEN_H, SCREEN_W};
use formats::palette::Rgb;
use std::path::PathBuf;

/// Posição no prédio (os dois programas têm os mesmos cinco lugares).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Floor {
    /// Saguão (térreo).
    Lobby,
    /// Andar 2: Ateliê de Pintura (FA) / Escritório (CW).
    Studio,
    /// Andar 3: Oficina de Projetos.
    Projects,
    /// Andar 4: Truques de Desenho (FA) / Oficina de Ideias (CW).
    Tricks,
    /// Sótão escuro acima do andar 4 (só pelo mastro).
    Attic,
}

/// De que é o balão aberto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Talk {
    Arrival,
    Tip,
    Pick(Pick),
    /// Menu do McZee ou da Maggie; `off` = "Que bom que você me chamou de novo".
    Helper {
        off: bool,
    },
    Wand,
    AskButton,
}

/// As cinco posições do mastro, na ordem em que ele sobe.
const CYCLE: [Floor; 5] = [
    Floor::Lobby,
    Floor::Studio,
    Floor::Projects,
    Floor::Tricks,
    Floor::Attic,
];

impl Floor {
    /// O andar `d` posições acima pelo mastro (negativo: abaixo), com volta.
    pub fn step(self, d: i32) -> Floor {
        let i = CYCLE.iter().position(|f| *f == self).unwrap_or(0) as i32;
        CYCLE[(i + d).rem_euclid(CYCLE.len() as i32) as usize]
    }
}

/// Nome usado no lugar do `@` dos balões quando não há usuário escolhido.
pub const DEFAULT_USER: &str = "artista";

impl Floor {
    pub fn number(self) -> u8 {
        match self {
            Floor::Lobby => 1,
            Floor::Studio => 2,
            Floor::Projects => 3,
            Floor::Tricks => 4,
            Floor::Attic => 5,
        }
    }
}

/// Por onde se chegou a um andar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Via {
    Pole,
    Board,
    Elevator,
}

/// Dois cliques no mesmo ponto dentro deste tempo são um clique duplo.
/// Folga (px) entre os dois cliques do clique duplo no cavalete e na parede.
const DOUBLE_SLACK: i32 = 6;

/// Espera antes da primeira animação ambiente e entre as seguintes (s).
/// Animações de passagem: só depois de um tempo sem ninguém clicar (como no
/// original); cada clique ou tecla recomeça a espera.
const GAG_FIRST: (f32, f32) = (20.0, 30.0);
const GAG_BETWEEN: (f32, f32) = (20.0, 40.0);

pub struct Room {
    common: KidCommon,
    /// Programa do prédio: para onde levam as saídas (`SceneId::in_program`).
    program: Program,
    /// As salas do prédio, por andar.
    rooms: Rooms,
    floor: Floor,
    /// Dados da sala do andar atual.
    spec: &'static RoomSpec,
    last_drawing: Option<Canvas>,
    /// Nome do último quadro (plaqueta da parede do Ateliê).
    last_name: Option<String>,
    pal: Vec<Rgb>,
    drawings: Option<PathBuf>,
    balloon: Option<Balloon>,
    /// De que é o balão aberto.
    talk: Talk,
    /// Dica aberta por cima do balão (lâmpada, tecla de atalho, glossário):
    /// o próximo clique a fecha.
    sub: Option<Balloon>,
    /// "EPA!" da luminária e do "Tchau": modal, só as bolinhas respondem.
    quit: Option<TextBalloon>,
    /// Ctrl apertado.
    ctrl: bool,
    /// Relógio do clique duplo (s): cada quadro conta no máximo
    /// [`crate::scene::CLICK_FRAME_MAX`].
    clock: f32,
    /// Último clique simples no cavalete ou na parede: ação, hora e ponto.
    clicks: DoubleClick<Action>,
    /// "Você quer me ver menos? — Sim": sem balões de chegada até religar.
    help_off: bool,
    /// Plaqueta da parede em edição (novo nome do último quadro).
    naming: Option<NameField>,
    rng: XorShift,
    character: Option<Character>,
    /// Obra-prima do McZee na moldura do andar dos Truques (sorteada).
    painting: Option<(u16, u16)>,
    gags: GagPlayer,
    /// Apelido do usuário, o `@` dos balões.
    user: String,
    /// Bolsa Mágica do andar (e a Varinha Mágica).
    bag: MagicBag,
    /// Surpresa da Varinha Mágica em curso (mastro ou buraco do rato).
    surprise: Option<Animator>,
    /// Sótão: o objeto aceso pelo clique no escuro (Writer).
    dark: attic::Dark,
    /// Dentro do elevador.
    elevator: Option<Elevator>,
    /// O cursor atual é o da varinha.
    wand_cursor: bool,
    /// Pasta dos usuários do port (preferências e documentos do Writer).
    users: Option<PathBuf>,
    /// Último trabalho de um programa com documentos (`Studio::doc`), com a
    /// miniatura já reduzida para a moldura.
    last_work: Option<LastWork>,
    /// Aviso "EPA!" sem opções (título de um documento de outra pessoa):
    /// qualquer clique o fecha.
    alert: Option<TextBalloon>,
}

impl Room {
    /// Sala do Fine Artist.
    pub fn new(
        floor: Floor,
        click: Option<SoundRef>,
        drawings: Option<PathBuf>,
        assets: &Assets,
    ) -> Room {
        Room::for_program(Program::Artist, floor, click, drawings, assets)
    }

    /// Sala do prédio do programa `p` (salas de `room_spec::rooms(p)`).
    pub fn for_program(
        program: Program,
        floor: Floor,
        click: Option<SoundRef>,
        drawings: Option<PathBuf>,
        assets: &Assets,
    ) -> Room {
        let rooms = room_spec::rooms(program);
        let spec = rooms(floor);
        let pal = assets.palettes(Program::Artist).pal256.clone();
        // um último quadro todo em branco é como nenhum (parede vazia)
        let last_drawing = drawings
            .as_ref()
            .and_then(|d| load_png(&d.join(LAST_NAME), &pal))
            .filter(|c| c.px.iter().any(|&v| v != crate::paint::WHITE));
        let last_name = drawings.as_deref().and_then(load_name);
        let mut rng = XorShift::from_clock();
        let character = pick_character(spec, &mut rng);
        let gags = GagPlayer::with_rng(spec.gags, GAG_FIRST, GAG_BETWEEN, rng.clone())
            .in_program(spec.kit.program);
        Room {
            common: KidCommon::new(click),
            program,
            rooms,
            spec,
            last_drawing,
            last_name,
            pal,
            drawings,
            balloon: arrival_balloon(spec, assets, DEFAULT_USER),
            talk: Talk::Arrival,
            sub: None,
            quit: None,
            ctrl: false,
            clock: 0.0,
            clicks: DoubleClick::with_slack(DOUBLE_SLACK),
            help_off: false,
            naming: None,
            floor,
            painting: pick_painting(spec, &mut rng),
            rng,
            character,
            gags,
            user: DEFAULT_USER.to_string(),
            bag: new_bag(spec),
            surprise: None,
            dark: attic::Dark::default(),
            elevator: None,
            wand_cursor: false,
            users: None,
            last_work: None,
            alert: None,
        }
    }

    /// Com a pasta dos usuários do port: o estado guardado (bolsa, ajuda,
    /// se o programa o guarda) e o último trabalho da parede (`Studio::doc`).
    pub fn with_users_dir(mut self, dir: Option<PathBuf>, assets: &Assets) -> Room {
        self.users = dir;
        if let (Some(p), Some(d)) = (self.kit().persist, self.users.as_deref()) {
            let st = (p.load)(d, &self.user);
            self.bag.open = st.bag_open;
            if st.help_off && !self.help_off {
                self.help_off = true;
                self.close();
            }
        }
        self.load_last_work(assets);
        self
    }

    fn place<'a>(&'a self, assets: &'a Assets) -> WorkPlace<'a> {
        WorkPlace {
            users: self.users.as_deref(),
            user: &self.user,
            data: assets.data_dir(),
        }
    }

    /// Relê o último trabalho (só nas salas com `Studio::doc`).
    fn load_last_work(&mut self, assets: &Assets) {
        self.last_work =
            self.spec.studio.and_then(|st| st.doc).and_then(|d| {
                (d.load)(&self.place(assets), assets).map(|w| fit_work(w, d.thumb_box))
            });
    }

    /// Grava o estado guardado (bolsa, ajuda), se o programa o guarda.
    fn save_state(&self) {
        if let (Some(p), Some(d)) = (self.kit().persist, self.users.as_deref()) {
            (p.save)(
                d,
                &self.user,
                RoomState {
                    bag_open: self.bag.open,
                    help_off: self.help_off,
                },
            );
        }
    }

    /// Com o apelido do usuário no lugar do `@` dos balões.
    pub fn with_user(mut self, user: &str, assets: &Assets) -> Room {
        self.user = user.to_string();
        self.balloon = arrival_balloon(self.spec, assets, &self.user);
        self.talk = Talk::Arrival;
        self.load_last_work(assets);
        self
    }

    pub fn character(&self) -> Option<&Character> {
        self.character.as_ref()
    }

    /// Há uma animação ambiente em curso.
    pub fn gag_playing(&self) -> bool {
        self.gags.is_playing()
    }

    pub fn balloon_open(&self) -> bool {
        self.balloon.is_some()
    }

    /// O "EPA!" (Sair / Continuar) está aberto.
    pub fn quit_open(&self) -> bool {
        self.quit.is_some()
    }

    pub fn bag(&self) -> &MagicBag {
        &self.bag
    }

    /// Há uma surpresa da Varinha Mágica em curso.
    pub fn surprise_playing(&self) -> bool {
        self.surprise.is_some()
    }

    pub fn in_elevator(&self) -> bool {
        self.elevator.is_some()
    }

    /// A ajuda do McZee / da Maggie está desligada.
    pub fn help_off(&self) -> bool {
        self.help_off
    }

    fn show(&mut self, mut b: Balloon, t: Talk) {
        b.tail = talk::mczee_tail(helper(self.spec), self.elevator.is_some(), &b);
        self.balloon = Some(b);
        self.talk = t;
        self.sub = None;
    }

    fn close(&mut self) {
        self.balloon = None;
        self.sub = None;
    }

    fn kit(&self) -> &'static RoomKit {
        self.spec.kit
    }

    /// Som `id` do `.SND` do programa da sala.
    fn snd(&self, id: u16) -> Effect {
        Effect::PlaySound(self.kit().sound(id))
    }

    /// Troca de tela: o id do Fine Artist levado ao programa do prédio.
    fn go(&self, id: SceneId) -> SceneId {
        id.in_program(self.program)
    }

    /// Dica do registro `k` (a da sala ou a do kit).
    fn tip(&self, k: usize, assets: &Assets) -> Balloon {
        talk::tip_in(self.kit(), self.spec.tip(k), assets, k, &self.user)
    }

    fn show_arrival(&mut self, assets: &Assets) {
        match arrival_balloon(self.spec, assets, &self.user) {
            Some(b) => self.show(b, Talk::Arrival),
            None => self.close(),
        }
    }

    /// Clique na Bolsa Mágica ou no que saiu dela.
    fn use_bag(&mut self, part: BagPart, fx: &mut Vec<Effect>, assets: &Assets) {
        match part {
            BagPart::Bag => {
                // com a varinha na mão, a bolsa a recebe de volta e se abre
                let open = self.bag.holding_wand || !self.bag.open;
                self.bag.holding_wand = false;
                self.bag.open = open;
                let s = &self.kit().snd;
                fx.push(self.snd(if open { s.bag_open } else { s.bag_close }));
                self.save_state();
            }
            BagPart::Suitcase => {
                fx.push(self.snd(self.kit().snd.suitcase));
                fx.push(Effect::Push(self.go(SceneId::Portfolio)));
            }
            BagPart::Book => {
                fx.extend(self.kit().snd.book.map(|id| self.snd(id)));
                fx.push(Effect::Push(self.go(SceneId::BookStory)));
            }
            BagPart::Wand => {
                self.bag.holding_wand = true;
                self.bag.open = false;
                fx.push(self.snd(self.kit().snd.wand_take));
                if let Some(b) = wand_balloon(self.kit(), assets, &self.user) {
                    self.show(b, Talk::Wand);
                }
            }
        }
    }

    /// A varinha tocou o mastro ou o buraco do rato: a surpresa do andar.
    /// Depois dela a bolsa se abre com a varinha dentro.
    fn start_surprise(&mut self, target: Target, fx: &mut Vec<Effect>, assets: &Assets) {
        self.bag.holding_wand = false;
        let s = &self.kit().snd;
        fx.push(self.snd(match target {
            Target::Pole => s.pole_up,
            Target::Hole => s.wand_hole,
        }));
        let p = self.kit().program;
        match self
            .spec
            .surprises
            .start_in(p, target, &mut self.rng, assets)
        {
            Some(mut a) => {
                fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
                self.surprise = Some(a);
            }
            None => self.bag.open = true,
        }
    }

    fn enter_elevator(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        fx.push(self.snd(self.spec.elevator_sound));
        fx.push(self.snd(self.kit().snd.elevator_arrive));
        // o McZee entra junto (a Maggie e o Max ficam)
        let costume = self.character.as_ref().filter(|_| {
            self.spec
                .character
                .is_some_and(|c| c.helper == Helper::McZee)
        });
        let mut e = Elevator::for_program(self.program, self.floor, costume, assets, &self.user);
        e.set_ask(!self.help_off);
        self.elevator = Some(e);
        self.close();
        self.bag.holding_wand = false;
    }

    /// Abre o "EPA!" por cima da sala, com a vinheta.
    fn open_quit(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        fx.extend(self.kit().snd.quit.map(|id| self.snd(id)));
        self.close();
        self.quit = Some(talk::quit_balloon_in(
            self.kit().quit,
            assets,
            self.spec.quit_pos,
        ));
    }

    /// Clique com o "EPA!" aberto: só as bolinhas respondem.
    fn quit_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(q) = &self.quit else { return };
        let s = &self.kit().snd;
        match q.hit_option(assets, x, y) {
            Some(0) => {
                fx.push(self.snd(s.choice));
                fx.push(Effect::Quit);
            }
            Some(_) => {
                fx.push(self.snd(s.choice));
                self.quit = None;
            }
            None => {
                if !q.contains(assets, x, y) {
                    fx.push(self.snd(s.refuse));
                }
            }
        }
    }

    /// Clique no McZee ou na Maggie (na sala ou no elevador): o menu
    /// "Você quer me ver menos?" (ou "Que bom que você me chamou de novo").
    fn open_helper(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        let kit = self.kit();
        let (h, pos, sound) = if self.elevator.is_some() {
            (Helper::McZee, kit.elevator_helper, kit.snd.mczee)
        } else {
            match self.spec.character {
                Some(c) => (c.helper, c.menu, c.sound),
                None => return,
            }
        };
        fx.push(self.snd(sound));
        let off = self.help_off;
        let b = h.balloon_in(kit, assets, &self.user, off, pos);
        self.show(b, Talk::Helper { off });
    }

    /// "Diga de novo" e "Ajudar": o balão de chegada (no elevador, o
    /// "Subindo? Ou descendo?").
    fn repeat_arrival(&mut self, assets: &Assets) {
        match self.elevator.as_mut() {
            Some(e) => {
                self.balloon = None;
                e.set_ask(true);
            }
            None => self.show_arrival(assets),
        }
    }

    /// Começa um trabalho novo do objeto (escolha "criar novo" ou Ctrl+clique).
    fn start_pick(&mut self, p: Pick, fx: &mut Vec<Effect>, assets: &Assets) {
        match p.start() {
            Start::Scene(id) => fx.push(Effect::ChangeScene(id)),
            Start::AskButton => self.act(Action::AskButton, fx, assets),
        }
    }

    /// Opção `k` do balão aberto.
    fn choose(&mut self, k: usize, fx: &mut Vec<Effect>, assets: &Assets) {
        // "Ignorar isto" (última linha do balão de escolha): a dica do Ctrl
        if let Talk::Pick(p) = self.talk
            && k == talk::IGNORE
        {
            self.open_link(p.skip_tip(), fx, assets);
            return;
        }
        fx.push(self.snd(self.kit().snd.choice));
        match self.talk {
            Talk::Arrival => {
                let action = self.spec.arrival_action(k);
                self.act(action, fx, assets);
            }
            Talk::Pick(p) => {
                self.close();
                if k == 0 {
                    self.start_pick(p, fx, assets);
                } else {
                    fx.push(self.snd(self.kit().snd.portfolio));
                    let to = super::writer_floors::folder(p);
                    fx.push(Effect::Push(self.go(to)));
                }
            }
            Talk::Helper { off: false } => match k {
                0 => {
                    self.help_off = true;
                    self.close();
                    self.save_state();
                }
                1 => self.repeat_arrival(assets),
                _ => self.close(),
            },
            Talk::Helper { off: true } => {
                if k == 0 {
                    self.help_off = false;
                    self.save_state();
                    self.repeat_arrival(assets);
                } else {
                    self.close();
                }
            }
            Talk::AskButton => {
                let kind = if k == 0 {
                    ProjectKind::Button
                } else {
                    ProjectKind::Sticker
                };
                self.close();
                fx.push(Effect::ChangeScene(SceneId::Project(kind)));
            }
            Talk::Tip | Talk::Wand => self.close(),
        }
    }

    /// Ligação (dica, tecla de atalho, glossário): abre por cima do balão.
    fn open_link(&mut self, k: usize, fx: &mut Vec<Effect>, assets: &Assets) {
        fx.push(self.snd(self.kit().link_sound(k)));
        self.sub = Some(self.tip(k, assets));
    }

    /// Palavra do glossário numa opção do balão de chegada ("tridimensional"
    /// dos Truques): o registro que ela abre.
    fn glossary_hit(&self, b: &Balloon, x: i32, y: i32, assets: &Assets) -> Option<usize> {
        let g = self.spec.arrival?.glossary?;
        (self.talk == Talk::Arrival
            && b.option_word_rect(assets, g.option, g.word)
                .is_some_and(|r| inside(r, x, y)))
        .then_some(g.record)
    }

    fn act(&mut self, action: Action, fx: &mut Vec<Effect>, assets: &Assets) {
        match action {
            Action::Floor(fl) => {
                fx.push(self.snd(self.kit().snd.board));
                self.set_floor(fl, Via::Board, fx, assets);
            }
            Action::Gallery(id) => {
                fx.push(self.snd(id));
                fx.push(Effect::ChangeScene(self.go(SceneId::Gallery)));
            }
            Action::PoleUp => {
                fx.push(self.snd(self.kit().snd.pole_up));
                let to = self.floor.step(if self.ctrl { 2 } else { 1 });
                self.set_floor(to, Via::Pole, fx, assets);
            }
            Action::PoleDown => {
                fx.push(self.snd(self.kit().snd.pole_down));
                let to = self.floor.step(if self.ctrl { -2 } else { -1 });
                self.set_floor(to, Via::Pole, fx, assets);
            }
            Action::MouseHole => fx.push(self.snd(self.kit().snd.mouse_hole)),
            Action::Elevator => self.enter_elevator(fx, assets),
            Action::Outside => {
                fx.push(self.snd(self.kit().snd.skate));
                fx.push(Effect::ChangeScene(self.go(SceneId::Street)));
            }
            Action::Quit => self.open_quit(fx, assets),
            Action::Close => self.close(),
            Action::AskButton => {
                let mut b = talk::fallback(
                    200,
                    "Legal! O que você quer fazer?",
                    &["Um botão", "Um adesivo"],
                    &[],
                );
                (b.x, b.y) = (183, 239);
                self.show(b, Talk::AskButton);
            }
            Action::Easel | Action::Wall => {
                fx.push(self.snd(self.kit().snd.object));
                if let Some(st) = self.spec.studio {
                    let k = match (action, st.doc, &self.last_work) {
                        (Action::Easel, ..) => st.easel_tip,
                        (_, Some(d), Some(w)) if !w.own => d.other_tip,
                        _ => st.wall_tip,
                    };
                    let mut b = self.tip(k, assets);
                    if let Some((x, y)) = self.spec.object_balloon {
                        (b.x, b.y) = (x, y);
                    }
                    self.show(b, Talk::Tip);
                }
            }
            Action::Pick(p) => {
                fx.push(self.snd(self.kit().snd.object));
                if self.ctrl {
                    self.close();
                    self.start_pick(p, fx, assets);
                } else {
                    fx.push(self.snd(self.kit().snd.pick));
                    let b = p.balloon_in(self.kit(), assets, &self.user);
                    self.show(b, Talk::Pick(p));
                }
            }
            Action::Painting => {
                fx.push(self.snd(self.kit().snd.painting));
                self.painting = pick_painting(self.spec, &mut self.rng);
            }
            Action::WallLabel if self.doc_wall().is_some() => self.doc_label(fx, assets),
            Action::WallLabel => {
                if self.last_drawing.is_some() {
                    fx.push(self.snd(self.kit().snd.object));
                    let name = self.wall_name().unwrap_or_default();
                    self.naming = Some(NameField::new(&name));
                    let b = studio_wall::balloon(assets, &self.user);
                    self.show(b, Talk::Tip);
                }
            }
            Action::Tip(k) => {
                // a dica do quadro da parede, com o documento de outra pessoa
                let k = match (self.spec.studio, &self.last_work) {
                    (Some(st), Some(w)) if k == st.wall_tip && !w.own => {
                        st.doc.map_or(k, |d| d.other_tip)
                    }
                    _ => k,
                };
                let b = self.tip(k, assets);
                self.show(b, Talk::Tip);
            }
            Action::Sound(id) => {
                fx.push(self.snd(id));
                self.close();
            }
        }
    }

    /// Clique duplo no cavalete (quadro novo) ou no quadro da parede (o
    /// último quadro), se este clique completa um.
    fn double_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>) -> bool {
        let Some(action) = self.clicks.take(self.clock, (x, y)) else {
            return false;
        };
        fx.push(self.snd(self.kit().snd.object));
        // quadro do Escritório do Writer: abre o documento da parede
        if let (Action::Wall, Some(_), Some(w)) = (action, self.doc_wall(), &self.last_work) {
            fx.push(Effect::OpenDocument(w.path.clone()));
            return true;
        }
        fx.push(Effect::ChangeScene(self.go(match action {
            Action::Easel => SceneId::PaintNew,
            _ => SceneId::Paint,
        })));
        true
    }

    /// Cursor da varinha enquanto ela está na mão; senão, a mão.
    fn sync_cursor(&mut self, fx: &mut Vec<Effect>) {
        if self.bag.holding_wand != self.wand_cursor {
            self.wand_cursor = self.bag.holding_wand;
            fx.push(Effect::SetCursor(if self.wand_cursor {
                self.kit().wand_cursor
            } else {
                DEFAULT_CURSOR
            }));
        }
    }

    pub fn floor(&self) -> Floor {
        self.floor
    }

    /// Muda de andar; `via` diz por onde (a fala `304` do quadro do saguão
    /// toca no clique; pelo elevador, ao chegar ao Ateliê).
    fn set_floor(&mut self, floor: Floor, via: Via, fx: &mut Vec<Effect>, assets: &Assets) {
        self.floor = floor;
        self.spec = (self.rooms)(floor);
        let spec = self.spec;
        self.close();
        self.quit = None;
        if !self.help_off {
            self.show_arrival(assets);
        }
        self.character = pick_character(spec, &mut self.rng);
        self.painting = pick_painting(spec, &mut self.rng);
        let open = self.bag.open;
        self.bag = new_bag(spec);
        if spec.kit.persist.is_some() {
            self.bag.open = open;
        }
        self.surprise = None;
        self.clicks.reset();
        self.alert = None;
        self.naming = None;
        self.gags = GagPlayer::with_rng(
            spec.gags,
            GAG_FIRST,
            GAG_BETWEEN,
            XorShift::new(self.rng.next_u32()),
        )
        .in_program(spec.kit.program);
        self.load_last_work(assets);
        if let Some(id) = spec.arrival_sound {
            fx.push(self.snd(id));
        }
        if via == Via::Elevator
            && let Some(id) = spec.elevator_arrival_sound
        {
            fx.push(self.snd(id));
        }
    }

    /// Um clique de botão esquerdo em (x, y) fora do elevador; `true` se ele
    /// foi usado (os seguintes do mesmo quadro esperam).
    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) -> bool {
        if self.alert.take().is_some() {
            return true;
        }
        if self.quit.is_some() {
            self.quit_click(x, y, fx, assets);
            return true;
        }
        if self.double_click(x, y, fx) {
            return true;
        }
        if self.sub.take().is_some() {
            return true;
        }
        if let Some(b) = &self.balloon {
            if let Some(k) = self.glossary_hit(b, x, y, assets) {
                self.open_link(k, fx, assets);
                return true;
            }
            if let Some(k) = b.hit(assets, x, y) {
                self.choose(k, fx, assets);
                return true;
            }
            if let Some(k) = b.link_hit(assets, x, y) {
                self.open_link(k, fx, assets);
                return true;
            }
            if b.contains(assets, x, y) {
                return false;
            }
            // o menu do personagem é modal: fora dele, a vinheta de recusa
            if matches!(self.talk, Talk::Helper { .. }) {
                fx.push(self.snd(self.kit().snd.refuse));
                return true;
            }
            // clicar fora do balão o fecha; o da varinha deixa o clique
            // seguir (ela já está na mão), e um objeto com balão abre o dele
            let pass = (self.talk == Talk::Wand && self.bag.holding_wand)
                || self.helper_hit(x, y, assets)
                || self.hotspot_at(x, y).is_some_and(Action::talks);
            self.close();
            if !pass {
                return false;
            }
        }
        if self.elevator.is_some() {
            return false;
        }
        if self.floor != Floor::Attic
            && let Some(part) = self.bag.hit(x, y)
        {
            self.use_bag(part, fx, assets);
            return true;
        }
        if !self.bag.holding_wand && self.helper_hit(x, y, assets) {
            self.open_helper(fx, assets);
            return true;
        }
        let Some(action) = self.hotspot_at(x, y) else {
            if self.floor == Floor::Attic {
                let id = self.dark.click(self.program, x, y, &mut self.rng);
                fx.extend(id.map(|id| self.snd(id)));
                return true;
            }
            return false;
        };
        match (self.bag.holding_wand, action) {
            (true, Action::PoleUp | Action::PoleDown) => {
                self.start_surprise(Target::Pole, fx, assets)
            }
            (true, Action::MouseHole) => self.start_surprise(Target::Hole, fx, assets),
            (true, Action::Pick(_))
                if let Some(prop) = self.spec.wand_props.iter().find(|w| inside(w.rect, x, y)) =>
            {
                self.start_prop(prop, assets, fx)
            }
            (_, action) => {
                if matches!(action, Action::Easel | Action::Wall) {
                    self.clicks.arm(action, self.clock, (x, y));
                }
                self.act(action, fx, assets)
            }
        }
        true
    }

    fn hotspot_at(&self, x: i32, y: i32) -> Option<Action> {
        self.spec
            .hotspots
            .iter()
            .find(|h| inside(h.rect, x, y))
            .map(|h| h.action)
    }

    fn helper_hit(&self, x: i32, y: i32, assets: &Assets) -> bool {
        self.character.as_ref().is_some_and(|c| c.hit(assets, x, y))
    }

    /// Nome na plaqueta da parede: o do último quadro, ou "Quadro Novo" se
    /// ele não tem nome; nada sem último quadro.
    fn wall_name(&self) -> Option<String> {
        if self.doc_wall().is_some() {
            return self.last_work.as_ref().map(|w| w.title.clone());
        }
        self.last_drawing.as_ref().map(|_| {
            self.last_name
                .clone()
                .unwrap_or_else(|| super::paint::rename::DEFAULT_NAME.to_string())
        })
    }

    /// Termina a edição da plaqueta: troca o nome (ou desiste).
    fn finish_naming(&mut self, end: End, fx: &mut Vec<Effect>) {
        let Some(field) = self.naming.take() else {
            return;
        };
        if self.talk == Talk::Tip {
            self.close();
        }
        let name = field.name();
        if let Some(d) = self.doc_wall() {
            self.rename_doc(d, end, &name, fx);
            return;
        }
        if end == End::Cancel || name.is_empty() || Some(&name) == self.last_name.as_ref() {
            return;
        }
        let Some(dir) = &self.drawings else { return };
        match studio_wall::rename_last(dir, &self.pal, &name) {
            Ok(()) => {
                fx.push(Effect::Log(format!(
                    "último quadro agora se chama \"{name}\""
                )));
                self.last_name = Some(name);
            }
            Err(e) => fx.push(Effect::Log(e)),
        }
    }

    /// Quadro da parede com um documento (Writer).
    fn doc_wall(&self) -> Option<&'static room_spec::DocWall> {
        self.spec.studio.and_then(|st| st.doc)
    }

    /// Plaqueta do quadro com um documento: o próprio troca de título; o de
    /// outra pessoa abre o "EPA! Você não pode mudar esse título".
    fn doc_label(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        let (Some(d), Some(w)) = (self.doc_wall(), &self.last_work) else {
            return;
        };
        fx.push(self.snd(self.kit().snd.object));
        if w.own {
            self.naming = Some(NameField::new(&w.title));
            let pos = self
                .spec
                .studio
                .map_or(studio_wall::BALLOON_POS, |st| st.name_balloon);
            let b = studio_wall::balloon_in(self.kit(), assets, &self.user, pos);
            self.show(b, Talk::Tip);
        } else {
            fx.push(self.snd(self.kit().snd.quit[1]));
            self.close();
            let (block, k, pos, text) = d.not_yours;
            self.alert = Some(talk::alert_balloon_in(
                self.kit().program,
                assets,
                (block, k),
                pos,
                text,
            ));
        }
    }

    /// Troca o título do documento da parede (Return ou clique fora).
    fn rename_doc(&mut self, d: &room_spec::DocWall, end: End, name: &str, fx: &mut Vec<Effect>) {
        let Some(w) = self.last_work.clone() else {
            return;
        };
        if end == End::Cancel || name.is_empty() || name == w.title || !w.own {
            return;
        }
        let place = WorkPlace {
            users: self.users.as_deref(),
            user: &self.user,
            data: std::path::Path::new(""),
        };
        match (d.rename)(&place, &w, name) {
            Ok(()) => {
                fx.push(Effect::Log(format!(
                    "último documento agora se chama \"{name}\""
                )));
                if let Some(w) = self.last_work.as_mut() {
                    w.title = name.to_string();
                }
            }
            Err(e) => fx.push(Effect::Log(e)),
        }
    }

    /// A varinha tocou um objeto que ela enfeita (bicho da parede).
    fn start_prop(&mut self, prop: &room_spec::WandProp, assets: &Assets, fx: &mut Vec<Effect>) {
        self.bag.holding_wand = false;
        let id = prop.scripts[self.rng.below(prop.scripts.len().max(1))];
        match crate::anim::load_exe(assets, self.kit().program, id) {
            Some(a) => {
                let mut a = a.with_offset(prop.anchor);
                fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
                self.surprise = Some(a);
            }
            None => self.bag.open = true,
        }
    }

    /// Último quadro reduzido no miolo do quadro da parede.
    fn draw_wall_picture(&self, st: &room_spec::Studio, frame: &mut Frame) {
        if let Some(d) = st.doc {
            if let Some(img) = self.last_work.as_ref().and_then(|w| w.image.as_ref()) {
                let (bx, by, bw, _) = d.thumb_box;
                frame.blit(img, bx + (bw - img.width as i32) / 2, by);
            }
            return;
        }
        let Some(c) = &self.last_drawing else { return };
        let k = st.wall_scale;
        for y in 0..c.h / k {
            for x in 0..c.w / k {
                let i = c.px[((y * k) * c.w + x * k) as usize];
                frame.put(
                    st.wall_pic.0 + x as i32,
                    st.wall_pic.1 + y as i32,
                    crate::ui::pal_rgb(&self.pal, i),
                );
            }
        }
    }

    /// Nome na plaqueta: o do último quadro (ou "Quadro Novo" se ele não
    /// tem nome) na parede; "Quadro Novo" no cavalete.
    fn draw_labels(&self, st: &room_spec::Studio, frame: &mut Frame, assets: &Assets) {
        let wall = if self.naming.is_some() {
            None
        } else {
            self.wall_name()
        };
        let labels = [
            (wall, st.wall_label),
            (Some(st.new_label.to_string()), st.easel_label),
        ];
        let font = assets.font(8);
        for (text, (cx, y)) in labels {
            let Some(t) = text else { continue };
            match &font {
                Some(f) => {
                    let w = f.width(&t) as i32;
                    frame.text_font(f, cx - w / 2, y, &t, [0, 0, 0]);
                }
                None => frame.text(cx - 4 * t.chars().count() as i32, y + 2, &t, [0, 0, 0]),
            }
        }
        if let Some(n) = &self.naming {
            n.draw_in(frame, font.as_deref(), st.label_rect);
        }
    }
}

/// Personagem da sala, com a fantasia ou pose sorteada.
fn pick_character(spec: &RoomSpec, rng: &mut XorShift) -> Option<Character> {
    let c = spec.character?;
    if !c.poses.is_empty() {
        let (image, (x, y)) = c.poses[rng.below(c.poses.len())];
        return Some(Character {
            program: spec.kit.program,
            image,
            x,
            y,
        });
    }
    Character::pick_in(spec.kit.program, c.choices, c.pos, rng)
}

/// Reduz a miniatura do último trabalho à altura da caixa da moldura (sem
/// passar da largura), pelo vizinho mais próximo.
fn fit_work(mut w: LastWork, (_, _, bw, bh): crate::ui::Rect) -> LastWork {
    if let Some(img) = &w.image {
        let (iw, ih) = (img.width as i32, img.height as i32);
        if iw > 0 && ih > 0 {
            let k = (bh as f32 / ih as f32).min(bw as f32 / iw as f32);
            // arredondado para baixo, como o original (108×144 → 73×98)
            let (tw, th) = (
                ((iw as f32 * k) as i32).max(1),
                ((ih as f32 * k) as i32).max(1),
            );
            let mut rgba = Vec::with_capacity((tw * th * 4) as usize);
            for y in 0..th {
                for x in 0..tw {
                    let (sx, sy) = ((x * iw / tw) as u32, (y * ih / th) as u32);
                    let i = ((sy * img.width + sx) * 4) as usize;
                    rgba.extend_from_slice(&img.rgba[i..i + 4]);
                }
            }
            w.image = Some(crate::Image {
                width: tw as u32,
                height: th as u32,
                rgba,
            });
        }
    }
    w
}

/// Obra-prima sorteada para a moldura da sala, se houver moldura.
fn pick_painting(spec: &RoomSpec, rng: &mut XorShift) -> Option<(u16, u16)> {
    spec.painting.map(|p| p.choices[rng.below(p.choices.len())])
}

fn new_bag(spec: &RoomSpec) -> MagicBag {
    MagicBag::with_pics(spec.kit.bag, spec.bag, spec.bag_open)
}

/// Balão de chegada à sala: o do `WHAT` do original quando os dados estão
/// instalados (e têm as opções esperadas), senão o texto reserva. O sótão
/// não tem.
fn arrival_balloon(spec: &RoomSpec, assets: &Assets, user: &str) -> Option<Balloon> {
    let a = spec.arrival?;
    let f = a.fallback;
    let mut fallback = talk::fallback(f.w, f.text, f.options, &[]);
    (fallback.x, fallback.y) = (f.x, f.y);
    let records = assets.dialogs(spec.kit.program, spec.kit.what);
    let mut b = match records.get(a.record) {
        Some(r) if r.options().len() == fallback.options.len() => {
            Balloon::from_block(&records, a.record, f.x, f.y, user).unwrap_or(fallback)
        }
        _ => fallback,
    };
    b.tail = talk::mczee_tail(helper(spec), false, &b);
    Some(b)
}

/// Quem fala na sala (o personagem parado), se há alguém.
fn helper(spec: &RoomSpec) -> Option<Helper> {
    spec.character.map(|c| c.helper)
}

/// Balão da Varinha Mágica nos andares (fala do mastro e do buraco do
/// rato), sem a "Tecla de atalho".
fn wand_balloon(kit: &RoomKit, assets: &Assets, user: &str) -> Option<Balloon> {
    let w = kit.wand?;
    let mut b = Balloon::from_block(&assets.dialogs(kit.program, kit.what), w.record, 0, 0, user)
        .filter(|b| !b.text.is_empty())
        .unwrap_or_else(|| talk::fallback(w.w, w.text, &[], w.links));
    b.options.clear();
    (b.x, b.y) = w.pos;
    Some(b)
}

/// Copia a imagem tratando branco puro como transparente (elementos sem máscara).
fn blit_white_transparent(f: &mut Frame, img: &crate::Image, x: i32, y: i32) {
    for dy in 0..img.height {
        for dx in 0..img.width {
            let i = ((dy * img.width + dx) * 4) as usize;
            let p = &img.rgba[i..i + 4];
            if p[3] != 0 && p[..3] != [255, 255, 255] {
                f.put(x + dx as i32, y + dy as i32, [p[0], p[1], p[2]]);
            }
        }
    }
}

impl Scene for Room {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        self.clock += crate::scene::click_time(dt);
        for i in input {
            if let Input::Ctrl(down) = i {
                self.ctrl = *down;
            }
        }
        if input
            .iter()
            .any(|i| matches!(i, Input::Click { .. } | Input::Key { .. }))
        {
            self.gags.poke();
        }
        fx.extend(
            self.gags
                .update(dt, assets)
                .into_iter()
                .map(Effect::PlaySound),
        );
        self.dark.update(dt);
        if let Some(a) = &mut self.surprise {
            fx.extend(a.update(dt).into_iter().map(Effect::PlaySound));
            if a.is_finished() {
                // a varinha volta para a bolsa, que fica aberta, e o balão
                // de chegada reaparece
                self.surprise = None;
                self.bag.open = true;
                if !self.help_off && self.balloon.is_none() {
                    self.show_arrival(assets);
                }
            }
        }
        if self.elevator.is_some() && self.balloon.is_none() && self.quit.is_none() {
            let outcome = self
                .elevator
                .as_mut()
                .and_then(|e| e.update(input, dt, assets, &mut fx));
            match outcome {
                Some(Outcome::Arrived(fl)) => {
                    self.elevator = None;
                    self.set_floor(fl, Via::Elevator, &mut fx, assets);
                }
                Some(Outcome::Quit) => self.open_quit(&mut fx, assets),
                Some(Outcome::Helper) => self.open_helper(&mut fx, assets),
                None => {}
            }
            self.sync_cursor(&mut fx);
            return fx;
        }
        if self.kit().ctrl_o
            && self.naming.is_none()
            && input.iter().any(|i| {
                matches!(
                    i,
                    Input::Key {
                        key: Key::O,
                        ctrl: true,
                        ..
                    }
                )
            })
        {
            self.close();
            fx.push(self.snd(self.kit().snd.suitcase));
            fx.push(Effect::Push(self.go(SceneId::Portfolio)));
        }
        let mut used = false;
        for i in input {
            if let Some(n) = self.naming.as_mut() {
                if let Some(end) = n.input(i) {
                    self.finish_naming(end, &mut fx);
                    continue;
                }
                // clicar fora da plaqueta guarda o nome digitado
                let label = self
                    .spec
                    .studio
                    .map_or(studio_wall::LABEL, |st| st.label_rect);
                if let Input::Click { x, y, .. } = *i
                    && !inside(label, x, y)
                {
                    self.finish_naming(End::Commit, &mut fx);
                }
            }
            let Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } = *i
            else {
                continue;
            };
            if self.naming.is_some() {
                continue;
            }
            // durante a surpresa da varinha os cliques esperam
            if self.surprise.is_some() {
                continue;
            }
            if used {
                // o resto do quadro espera, mas um clique duplo pode chegar
                // inteiro num quadro só (mouse rápido, quadro lento)
                if self.double_click(x, y, &mut fx) {
                    break;
                }
                continue;
            }
            used = self.click(x, y, &mut fx, assets);
        }
        self.sync_cursor(&mut fx);
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        if let Some(e) = &self.elevator {
            e.draw(frame, assets);
            if let Some(b) = &self.balloon {
                b.draw(frame, assets);
            }
            if let Some(q) = &self.quit {
                q.draw(frame, assets);
            }
            return;
        }
        if self.floor == Floor::Attic {
            attic::draw_in(self.program, &self.dark, frame, assets);
            return;
        }
        let spec = self.spec;
        let p = spec.kit.program;
        frame.fill([0, 0, 0]);
        if spec.background != 0 {
            let bg = assets.image(&ImageRef::Pic(p, spec.background));
            frame.blit(
                &bg,
                (SCREEN_W as i32 - bg.width as i32) / 2,
                (SCREEN_H as i32 - bg.height as i32) / 2,
            );
        }
        for e in spec.elements {
            let img = assets.image(&ImageRef::Pic(p, e.id));
            if e.mask != 0 {
                let mask = assets.image(&ImageRef::Pic(p, e.mask));
                frame.blit_masked(&img, &mask, e.x, e.y);
            } else {
                blit_white_transparent(frame, &img, e.x, e.y);
            }
        }
        if let Some(l) = spec.lamp {
            let (id, (x, y)) = if self.quit.is_some() {
                l.smoke
            } else {
                l.normal
            };
            crate::text_balloon::blit_pic_in(frame, assets, p, id, x, y, true);
        }
        if let Some(st) = &spec.studio {
            self.draw_wall_picture(st, frame);
            self.draw_labels(st, frame, assets);
        }
        self.bag.draw(frame, assets);
        if let (Some((img, mask)), Some(pt)) = (self.painting, spec.painting) {
            frame.blit_masked(
                &assets.image(&ImageRef::Pic(p, img)),
                &assets.image(&ImageRef::Pic(p, mask)),
                pt.pos.0,
                pt.pos.1,
            );
        }
        if let Some(c) = &self.character {
            c.draw(frame, assets);
        }
        // por cima de tudo, menos dos balões
        self.gags.draw(frame, assets);
        if let Some(a) = &self.surprise {
            a.draw(frame, assets);
        }
        if let Some(b) = &self.balloon {
            b.draw(frame, assets);
        }
        if let Some(b) = &self.sub {
            b.draw(frame, assets);
        }
        if let Some(q) = &self.quit {
            q.draw(frame, assets);
        }
        if let Some(b) = &self.alert {
            b.draw(frame, assets);
        }
    }
}

#[cfg(test)]
mod writer_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::rooms_data;
    use crate::scenes::testutil::empty_assets;
    use crate::scenes::{elevator, gags, room_objects as objects};

    /// Menu do McZee dentro do elevador.
    const ELEVATOR_HELPER: (i32, i32) = rooms_data::ARTIST_KIT.elevator_helper;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(SoundRef::Snd(_, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    fn option(r: &Room, a: &Assets, k: usize) -> Input {
        let (bx, by, _, _) = r.balloon.as_ref().unwrap().option_rect(a, k);
        click(bx + 20, by + 3)
    }

    fn quit_option(r: &Room, a: &Assets, k: usize) -> Input {
        let (bx, by, _, _) = r.quit.as_ref().unwrap().option_rect(a, k).unwrap();
        click(bx + 4, by + 3)
    }

    fn link(r: &Room, a: &Assets, k: usize) -> Input {
        let (bx, by, _, _) = r.balloon.as_ref().unwrap().link_rect(a, k);
        click(bx + 20, by + 3)
    }

    fn changes(fx: &[Effect]) -> Vec<SceneId> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::ChangeScene(id) => Some(*id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn lobby_directory_and_pole_tour() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        assert!(r.balloon_open());
        // clique fora do balão (que fica sobre o chão, como no original) fecha;
        // depois os pontos quentes funcionam
        r.update(&[click(520, 150)], 0.0, &a);
        assert!(!r.balloon_open());
        let fx = r.update(&[click(140, 86)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Studio);
        assert!(r.balloon_open());
        // pelo saguão, a fala do McZee no Ateliê
        assert_eq!(sounds(&fx), [talk::snd::STUDIO_ARRIVAL]);
        r.update(&[click(600, 470), click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
        r.update(&[click(600, 470), click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Tricks);
        // o mastro dos Truques sobe ao sótão escuro; o aro, ao saguão
        let fx = r.update(&[click(600, 470), click(550, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Attic);
        assert!(!r.balloon_open());
        assert_eq!(sounds(&fx), [objects::SND_POLE_UP, attic::SND_ARRIVE]);
        let fx = r.update(&[click(100, 300)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Attic);
        assert_eq!(sounds(&fx).len(), 1, "um som no escuro");
        r.update(&[click(410, 50)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        // opção 2 do balão do saguão: só a dica apontando a Oficina
        let fx = r.update(&[option(&r, &a, 1)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        assert_eq!(sounds(&fx), [talk::snd::CHOICE]);
        assert_eq!((r.talk, r.balloon.as_ref().unwrap().x), (Talk::Tip, 30));
        // a figura do prédio no quadro também leva aos andares, com a fala
        r.update(&[click(600, 470)], 0.0, &a);
        let fx = r.update(&[click(40, 70)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
        assert_eq!(sounds(&fx), [talk::snd::STUDIO_ARRIVAL]);
    }

    /// Setinha "Galeria" do quadro (fala `304`) e seta grande (`2fb`).
    #[test]
    fn lobby_gallery_arrows() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.update(&[click(300, 470)], 0.0, &a);
        let fx = r.update(&[click(25, 108)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Gallery]);
        assert_eq!(sounds(&fx), [talk::snd::STUDIO_ARRIVAL]);
        let fx = r.update(&[click(140, 340)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Gallery]);
        assert_eq!(sounds(&fx), [talk::snd::GALLERY_ARROW]);
        // topo do arco da porta também abre o elevador
        r.update(&[click(340, 90)], 0.0, &a);
        assert!(r.in_elevator());
    }

    /// No elevador, o McZee abre o mesmo menu (modal); "Sim" desliga a
    /// ajuda e o "Subindo?" não aparece mais.
    #[test]
    fn elevator_mczee_menu() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.update(&[click(300, 470), click(340, 300)], 0.0, &a);
        assert!(r.in_elevator());
        let (mx, my) = (elevator::MCZEE_POS.0 + 10, elevator::MCZEE_POS.1 + 10);
        let fx = r.update(&[click(mx, my)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::MCZEE]);
        assert_eq!(r.talk, Talk::Helper { off: false });
        assert_eq!(
            r.balloon.as_ref().map(|b| (b.x, b.y)),
            Some(ELEVATOR_HELPER)
        );
        // modal: clique fora só dá a recusa
        let fx = r.update(&[click(400, 400)], 0.0, &a);
        assert!(r.balloon_open());
        assert_eq!(sounds(&fx), [talk::snd::REFUSE]);
        r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert!(r.help_off() && !r.balloon_open());
        // de volta ao saguão pela porta: sem balão de chegada
        r.update(&[click(320, 230)], 0.0, &a);
        for _ in 0..15 {
            r.update(&[], 0.1, &a);
        }
        assert!(!r.in_elevator());
        assert_eq!(r.floor(), Floor::Lobby);
        assert!(!r.balloon_open());
    }

    /// O mastro é um ciclo de cinco posições: sobe uma, a abertura desce
    /// uma, Ctrl anda duas, com volta.
    #[test]
    fn pole_cycles_through_five_floors() {
        assert_eq!(Floor::Tricks.step(1), Floor::Attic);
        assert_eq!(Floor::Attic.step(1), Floor::Lobby);
        assert_eq!(Floor::Lobby.step(-1), Floor::Attic);
        assert_eq!(Floor::Lobby.step(-2), Floor::Tricks);
        assert_eq!(Floor::Projects.step(2), Floor::Attic);
        assert_eq!(Floor::Lobby.step(2), Floor::Projects);
        assert_eq!(Floor::Attic.step(2), Floor::Studio);
        let (_d, a) = empty_assets();
        // abertura do mastro do saguão: sótão; com Ctrl, os Truques
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.update(&[click(300, 450), click(620, 398)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Attic);
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.update(&[click(300, 450)], 0.0, &a);
        r.update(&[Input::Ctrl(true), click(620, 398)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Tricks);
        // Ctrl+mastro dos Projetos: sótão; Ctrl+aro: Ateliê
        r.update(
            &[Input::Ctrl(false), click(600, 470), click(520, 370)],
            0.0,
            &a,
        );
        assert_eq!(r.floor(), Floor::Projects);
        r.update(&[click(600, 470)], 0.0, &a);
        r.update(&[Input::Ctrl(true), click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Attic);
        r.update(&[click(410, 50)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Studio);
    }

    /// Cavalete: clique simples só mostra a dica; o duplo abre um quadro
    /// novo. Parede: dica; duplo abre o último quadro.
    #[test]
    fn easel_and_wall_need_a_double_click() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[click(600, 470)], 0.0, &a);
        let fx = r.update(&[click(190, 200)], 0.0, &a);
        assert!(changes(&fx).is_empty());
        assert!(r.balloon_open());
        assert_eq!(sounds(&fx), [talk::snd::OBJECT]);
        let fx = r.update(&[click(191, 201)], 0.2, &a);
        assert_eq!(changes(&fx), [SceneId::PaintNew]);
        // devagar demais não é clique duplo (o tempo passa em vários quadros:
        // um quadro só conta no máximo `CLICK_FRAME_MAX`)
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[click(600, 470), click(350, 100)], 0.0, &a);
        assert!(r.balloon_open());
        for _ in 0..10 {
            r.update(&[], 0.1, &a);
        }
        let fx = r.update(&[click(350, 100)], 0.0, &a);
        assert!(changes(&fx).is_empty());
        let fx = r.update(&[click(350, 100)], 0.1, &a);
        assert_eq!(changes(&fx), [SceneId::Paint]);
    }

    /// As opções do balão de chegada do Ateliê só apontam (dicas); nenhuma
    /// grava um quadro em branco por cima do último.
    #[test]
    fn studio_arrival_options_only_point() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = a.palettes(Program::Artist).pal256.clone();
        let mut c = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
        c.px.fill(255);
        crate::paint::store::save_drawing(dir.path(), &c, &pal, false, 0).unwrap();
        for k in 0..4 {
            let mut r = Room::new(Floor::Studio, None, Some(dir.path().to_path_buf()), &a);
            let fx = r.update(&[option(&r, &a, k)], 0.0, &a);
            assert!(changes(&fx).is_empty(), "opção {k}");
            assert!(r.balloon_open(), "opção {k}");
            assert_eq!(sounds(&fx), [talk::snd::CHOICE]);
        }
        let last = load_png(&dir.path().join(LAST_NAME), &pal).unwrap();
        assert!(last.px.iter().all(|&v| v == 255));
        // "Outra coisa?": dica do mastro com "Tecla de atalho" e "Descendo?"
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[option(&r, &a, 3)], 0.0, &a);
        assert_eq!(r.balloon.as_ref().unwrap().links.len(), 2);
        let fx = r.update(&[link(&r, &a, 1)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::TIP]);
        assert!(r.sub.is_some());
        // o próximo clique fecha só a dica de cima
        r.update(&[click(5, 5)], 0.0, &a);
        assert!(r.sub.is_none() && r.balloon_open());
    }

    /// Último quadro no quadro da parede, com o nome na plaqueta; o
    /// cavalete fica em branco.
    #[test]
    fn studio_shows_last_drawing_on_the_wall() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut c = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
        c.px.fill(128); // cinza na rampa dos testes
        crate::paint::store::save_drawing_named(
            dir.path(),
            &c,
            &crate::paint::Objects::default(),
            &a.palettes(Program::Artist).pal256,
            true,
            0,
            Some("Fundo Lunar"),
        )
        .unwrap();
        let r = Room::new(Floor::Studio, None, Some(dir.path().to_path_buf()), &a);
        assert_eq!(r.last_name.as_deref(), Some("Fundo Lunar"));
        let mut f = Frame::new(640, 480);
        r.draw(&mut f, &a);
        // o cavalete não muda com o último quadro
        let mut g = Frame::new(640, 480);
        Room::new(Floor::Studio, None, None, &a).draw(&mut g, &a);
        for (x, y) in [(150, 200), (190, 220), (260, 260)] {
            assert_eq!(f.pixel(x, y), g.pixel(x, y));
        }
        assert_ne!(f.pixel(360, 110), g.pixel(360, 110));
    }

    /// Plaqueta da parede: clique abre o campo com o nome selecionado;
    /// digitar e Return trocam o nome do último quadro.
    #[test]
    fn wall_label_renames_the_last_picture() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut c = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
        c.px.fill(3);
        crate::paint::store::save_drawing_named(
            dir.path(),
            &c,
            &crate::paint::Objects::default(),
            &a.palettes(Program::Artist).pal256,
            true,
            0,
            Some("Casa"),
        )
        .unwrap();
        let mut r = Room::new(Floor::Studio, None, Some(dir.path().to_path_buf()), &a);
        r.update(&[click(600, 470)], 0.0, &a);
        r.update(&[click(360, 196)], 0.0, &a);
        assert_eq!(r.naming.as_ref().map(|n| n.text.as_str()), Some("Casa"));
        assert!(r.balloon_open());
        let enter = Input::Key {
            key: crate::scene::Key::Enter,
            ctrl: false,
            alt: false,
        };
        r.update(&[Input::Text("Lua".into()), enter], 0.0, &a);
        assert!(r.naming.is_none() && !r.balloon_open());
        assert_eq!(r.last_name.as_deref(), Some("Lua"));
        assert!(dir.path().join("Lua.png").exists());
        assert_eq!(load_name(dir.path()).as_deref(), Some("Lua"));
    }

    /// Vaca, porco, cobra e estátuas abrem o balão de escolha; "criar novo"
    /// entra, "trabalhar com um que você já criou" abre a pasta, "Ignorar
    /// isto" mostra a dica do Ctrl; Ctrl+clique entra direto.
    #[test]
    fn project_objects_ask_first() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Projects, None, None, &a);
        r.update(&[click(600, 470)], 0.0, &a);
        let fx = r.update(&[click(100, 80)], 0.0, &a);
        assert!(changes(&fx).is_empty());
        assert_eq!(sounds(&fx), [talk::snd::OBJECT, talk::snd::PICK]);
        assert_eq!(r.talk, Talk::Pick(Pick::Comics));
        let fx = r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Project(ProjectKind::Comics)]);
        // porco: "trabalhar com um que você já criou" → pasta
        let mut r = Room::new(Floor::Projects, None, None, &a);
        r.update(&[click(600, 470), click(320, 50)], 0.0, &a);
        assert_eq!(r.talk, Talk::Pick(Pick::Flipbook));
        let fx = r.update(&[option(&r, &a, 1)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::Portfolio)));
        assert!(!r.balloon_open());
        // cobra: "Ignorar isto" (a última linha) → dica [38]
        r.update(&[click(300, 230)], 0.0, &a);
        assert_eq!(r.talk, Talk::Pick(Pick::Sticker));
        let fx = r.update(&[option(&r, &a, talk::IGNORE)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::TIP]);
        assert!(r.sub.is_some());
        // com outro balão aberto, clicar na vaca troca direto de balão
        r.update(&[click(5, 5)], 0.0, &a);
        r.update(&[click(100, 80)], 0.0, &a);
        assert_eq!(r.talk, Talk::Pick(Pick::Comics));
        // Ctrl+clique na vaca: direto ao projeto
        let fx = r.update(&[Input::Ctrl(true), click(100, 80)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Project(ProjectKind::Comics)]);
        // opção 1 do balão de chegada dos Truques: o balão da estátua
        let mut r = Room::new(Floor::Tricks, None, None, &a);
        let fx = r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert!(changes(&fx).is_empty());
        assert_eq!(r.talk, Talk::Pick(Pick::Basic));
        let fx = r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert_eq!(
            changes(&fx),
            [SceneId::Tricks(super::super::wizard::TrickKind::Basic)]
        );
    }

    /// "tridimensional" no balão de chegada dos Truques abre o glossário
    /// sem fechar o balão; o resto da opção abre o balão do Pensador.
    #[test]
    fn tricks_glossary_word_and_frame() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Tricks, None, None, &a);
        let b = r.balloon.clone().unwrap();
        let (wx, wy, _, _) = b.option_word_rect(&a, 1, "tridimensional").unwrap();
        let fx = r.update(&[click(wx + 2, wy + 2)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::GLOSSARY]);
        assert!(r.sub.is_some() && r.balloon_open());
        r.update(&[click(5, 470)], 0.0, &a);
        let (ox, oy, _, _) = b.option_rect(&a, 1);
        r.update(&[click(ox + 20, oy + 3)], 0.0, &a);
        assert_eq!(r.talk, Talk::Pick(Pick::ThreeD));
        // a moldura troca de quadro a cada clique
        r.update(&[click(5, 470)], 0.0, &a);
        let fx = r.update(&[click(60, 60)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::PAINTING]);
        assert!(r.painting.is_some());
        // buraco do rato sem a varinha: só o som
        let fx = r.update(&[click(600, 300)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::MOUSE_HOLE]);
    }

    /// Luminária e "Tchau" abrem o "EPA!" modal: fora das bolinhas, a
    /// vinheta de recusa; "Continuar" fecha; "Sair" sai.
    #[test]
    fn lamp_and_tchau_open_the_modal_epa() {
        let (_d, a) = empty_assets();
        for (fl, lamp) in [
            (Floor::Lobby, (170, 220)),
            (Floor::Studio, (496, 180)),
            (Floor::Projects, (500, 120)),
            (Floor::Tricks, (590, 50)),
        ] {
            let mut r = Room::new(fl, None, None, &a);
            r.update(&[click(300, 470)], 0.0, &a);
            let fx = r.update(&[click(lamp.0, lamp.1)], 0.0, &a);
            assert!(r.quit_open(), "{fl:?}");
            assert_eq!(sounds(&fx), talk::snd::QUIT);
            let fx = r.update(&[click(5, 470)], 0.0, &a);
            assert!(r.quit_open());
            assert_eq!(sounds(&fx), [talk::snd::REFUSE]);
            let fx = r.update(&[quit_option(&r, &a, 1)], 0.0, &a);
            assert!(!r.quit_open());
            assert_eq!(sounds(&fx), [talk::snd::CHOICE]);
        }
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[click(600, 470), click(496, 260)], 0.0, &a);
        let fx = r.update(&[quit_option(&r, &a, 0)], 0.0, &a);
        assert!(fx.contains(&Effect::Quit));
    }

    /// McZee e Maggie: "Você quer me ver menos?"; "Sim" desliga a ajuda (sem
    /// balão de chegada) e o próximo clique pergunta se é para ajudar de novo.
    #[test]
    fn helper_turns_help_off_and_on() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Projects, None, None, &a);
        r.update(&[click(600, 470)], 0.0, &a);
        // sem dados, a Maggie é o quadrado de 32×32 em MAGGIE_POS
        let (mx, my) = (gags::MAGGIE_POS.0 + 10, gags::MAGGIE_POS.1 + 10);
        let fx = r.update(&[click(mx, my)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::MAGGIE]);
        assert_eq!(r.talk, Talk::Helper { off: false });
        r.update(&[option(&r, &a, 1)], 0.0, &a);
        assert_eq!(r.talk, Talk::Arrival, "diga de novo");
        r.update(&[click(600, 470), click(mx, my)], 0.0, &a);
        r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert!(r.help_off() && !r.balloon_open());
        // sem ajuda, o andar seguinte chega sem balão
        r.update(&[click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Tricks);
        assert!(!r.balloon_open());
        r.update(&[click(mx, my)], 0.0, &a);
        assert_eq!(r.talk, Talk::Helper { off: true });
        r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert!(!r.help_off());
        assert_eq!(r.talk, Talk::Arrival);
        // McZee no Ateliê, com a fala dele
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[click(600, 470)], 0.0, &a);
        let (cx, cy) = (gags::MCZEE_STUDIO.0 + 20, gags::MCZEE_STUDIO.1 + 10);
        let fx = r.update(&[click(cx, cy)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::MCZEE]);
        assert_eq!(r.talk, Talk::Helper { off: false });
    }

    /// Com os dados reais: "tridimensional" é uma palavra dentro da opção 2
    /// do balão de chegada dos Truques.
    #[test]
    fn tricks_glossary_word_with_real_data() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let r = Room::new(Floor::Tricks, None, None, &a);
        let b = r.balloon.as_ref().unwrap();
        let w = b.option_word_rect(&a, 1, "tridimensional").unwrap();
        let o = b.option_rect(&a, 1);
        assert!(w.0 >= o.0 && w.1 >= o.1 && w.2 > 30, "{w:?} {o:?}");
    }

    #[test]
    fn draws_without_data() {
        let (_d, a) = empty_assets();
        let mut f = Frame::new(640, 480);
        for fl in CYCLE {
            let mut r = Room::new(fl, None, None, &a);
            // as animações ambientes tentam carregar e falham sem quebrar
            for _ in 0..40 {
                let fx = r.update(&[], 1.0, &a);
                assert!(!fx.iter().any(|e| matches!(e, Effect::PlaySound(_))));
            }
            r.draw(&mut f, &a);
        }
    }

    #[test]
    fn each_floor_has_its_character() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        let c = r.character().unwrap().clone();
        assert!(gags::MCZEE_COSTUMES.contains(&c.image));
        assert_eq!((c.x, c.y), gags::MCZEE_LOBBY);
        r.update(&[click(520, 150), click(140, 66)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
        let c = r.character().unwrap();
        assert!(gags::MAGGIE_POSES.contains(&c.image));
        assert_eq!((c.x, c.y), gags::MAGGIE_POS);
    }

    #[test]
    fn room_gags_play_with_real_data() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        let mut f = Frame::new(640, 480);
        let mut played = 0;
        // nada nos primeiros 15 s sem cliques; depois de 30 s já começou
        for _ in 0..150 {
            r.update(&[], 0.1, &a);
            assert!(!r.gags.is_playing());
        }
        for _ in 0..400 {
            r.update(&[], 0.1, &a);
            if r.gags.is_playing() {
                played += 1;
                r.draw(&mut f, &a);
            }
        }
        assert!(played > 0);
        // McZee fantasiado à esquerda do saguão: pixel não branco na barriga
        r.draw(&mut f, &a);
        assert_ne!(f.pixel(60, 300)[..3], [255, 255, 255]);
    }

    #[test]
    fn clicks_postpone_room_gags() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        // um clique a cada 10 s: nunca fica ocioso o bastante
        for k in 0..600 {
            let input = if k % 100 == 0 {
                vec![click(600, 470)]
            } else {
                vec![]
            };
            r.update(&input, 0.1, &a);
            assert!(!r.gags.is_playing(), "tocou no passo {k}");
        }
    }
    /// Bolsa Mágica: abre e fecha; a maleta abre a pasta por cima da sala, o
    /// livro conta a história e a varinha vira o cursor, com o balão dela.
    #[test]
    fn magic_bag_and_its_contents() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        assert!(!r.bag().open);
        r.update(&[click(300, 450)], 0.0, &a); // fecha o balão de chegada
        let fx = r.update(&[click(150, 425)], 0.0, &a);
        assert!(r.bag().open);
        assert_eq!(sounds(&fx), [objects::SND_BAG_OPEN]);
        let fx = r.update(&[click(310, 430)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::Portfolio)));
        assert_eq!(sounds(&fx), [objects::SND_SUITCASE]);
        let fx = r.update(&[click(400, 400)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::BookStory)));
        let fx = r.update(&[click(250, 445)], 0.0, &a);
        assert!(r.bag().holding_wand);
        assert!(!r.bag().open);
        assert!(r.balloon_open());
        assert!(fx.contains(&Effect::SetCursor(objects::WAND_CURSOR)));
        assert_eq!(sounds(&fx), [objects::SND_WAND_TAKE]);
        // a bolsa recebe a varinha de volta e se abre
        let fx = r.update(&[click(150, 410)], 0.0, &a);
        assert!(!r.bag().holding_wand);
        assert!(r.bag().open);
        assert!(fx.contains(&Effect::SetCursor(DEFAULT_CURSOR)));
        let fx = r.update(&[click(150, 440)], 0.0, &a);
        assert!(!r.bag().open);
        assert_eq!(sounds(&fx), [objects::SND_BAG_CLOSE]);
    }

    /// Sem os roteiros, a varinha no buraco do rato só volta para a bolsa;
    /// o clique atravessa o balão dela, como no original.
    #[test]
    fn wand_without_data_returns_to_the_bag() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Studio, None, None, &a);
        assert!(r.bag().open, "no Ateliê a bolsa já chega aberta");
        r.update(&[click(600, 470)], 0.0, &a);
        r.update(&[click(370, 430)], 0.0, &a);
        assert!(r.bag().holding_wand && r.balloon_open());
        let fx = r.update(&[click(490, 350)], 0.0, &a);
        assert!(!r.balloon_open());
        assert!(!r.bag().holding_wand);
        assert!(r.bag().open);
        assert_eq!(sounds(&fx), [objects::SND_WAND_HOLE]);
        assert!(fx.contains(&Effect::SetCursor(DEFAULT_CURSOR)));
        assert_eq!(r.floor(), Floor::Studio);
    }

    /// Mastro sobe, abertura no chão desce, cada um com o seu som; skates
    /// levam à rua (saguão) e ao elevador (andares).
    #[test]
    fn pole_hole_and_skates() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        let fx = r.update(&[click(300, 450), click(606, 200)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Studio);
        assert_eq!(sounds(&fx), [objects::SND_POLE_UP]);
        let fx = r.update(&[click(300, 30), click(610, 412)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        assert_eq!(sounds(&fx), [objects::SND_POLE_DOWN]);
        let fx = r.update(&[click(300, 450), click(540, 445)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Street)));
        assert_eq!(sounds(&fx), [objects::SND_SKATE]);
        // portas do elevador
        let fx = r.update(&[click(330, 250)], 0.0, &a);
        assert!(r.in_elevator());
        assert_eq!(sounds(&fx), [elevator::SND_DOORS, elevator::SND_ARRIVE]);
        let mut f = Frame::new(640, 480);
        r.draw(&mut f, &a);
        // skate "ELEVADOR" da Oficina, depois o andar 2 no painel
        let mut r = Room::new(Floor::Projects, None, None, &a);
        let fx = r.update(&[click(300, 450), click(500, 440)], 0.0, &a);
        assert!(r.in_elevator());
        assert_eq!(sounds(&fx), [objects::SND_SKATE, elevator::SND_ARRIVE]);
        r.update(&[click(595, 212)], 0.0, &a);
        for _ in 0..30 {
            r.update(&[], 0.1, &a);
        }
        assert!(!r.in_elevator());
        assert_eq!(r.floor(), Floor::Studio);
        assert!(r.balloon_open(), "balão de chegada do Ateliê");
    }

    /// "Tchau" no elevador pergunta antes de sair.
    #[test]
    fn elevator_tchau_asks() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Tricks, None, None, &a);
        r.update(&[click(300, 450), click(490, 440)], 0.0, &a);
        assert!(r.in_elevator());
        r.update(&[click(70, 110)], 0.0, &a);
        assert!(r.quit_open());
        let fx = r.update(&[quit_option(&r, &a, 0)], 0.0, &a);
        assert!(fx.contains(&Effect::Quit));
    }

    /// Com os dados reais: a varinha no mastro do saguão faz um bicho descer
    /// (ou subir) pelo mastro sem mudar de andar; cliques esperam; no fim a
    /// bolsa está aberta com a varinha dentro.
    #[test]
    fn wand_surprise_on_the_pole_with_real_data() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.update(&[click(300, 450), click(150, 425)], 0.0, &a);
        r.update(&[click(250, 445)], 0.0, &a);
        let fx = r.update(&[click(606, 200)], 0.0, &a);
        assert!(r.surprise_playing());
        assert_eq!(r.floor(), Floor::Lobby);
        assert!(sounds(&fx).contains(&objects::SND_POLE_UP));
        let mut f = Frame::new(640, 480);
        r.update(&[], 0.5, &a);
        r.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "lobby_wand_pole");
        // clique durante a surpresa não troca de andar
        r.update(&[click(140, 66)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        for _ in 0..40 {
            r.update(&[], 0.1, &a);
        }
        assert!(!r.surprise_playing());
        assert!(r.bag().open && !r.bag().holding_wand);
        r.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "lobby_bag_open");
        // a bolsa aberta no chão: a maleta vermelha em (300, 430)
        let p = f.pixel(300, 430);
        assert!(p[0] > 150 && p[1] < 80, "{p:?}");
    }

    /// Clique no meio da primeira área da sala com a ação dada.
    fn spot(r: &Room, action: Action) -> Input {
        let h = r
            .spec
            .hotspots
            .iter()
            .find(|h| h.action == action)
            .unwrap_or_else(|| panic!("{action:?}"));
        click(h.rect.0 + h.rect.2 / 2, h.rect.1 + h.rect.3 / 2)
    }

    /// No prédio do Writer as saídas levam às telas do Writer (as salas
    /// usam o próprio `RoomSpec`, do Writer ou, enquanto ele não existe, o
    /// do Fine Artist).
    #[test]
    fn writer_building_leads_to_writer_screens() {
        use crate::scene::CwScene;
        let (_d, a) = empty_assets();
        let mut r = Room::for_program(Program::Writer, Floor::Lobby, None, None, &a);
        r.close();
        let fx = r.update(&[spot(&r, Action::Outside)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Cw(CwScene::Street)]);
        let gallery = r
            .spec
            .hotspots
            .iter()
            .find_map(|h| match h.action {
                Action::Gallery(_) => Some(h.action),
                _ => None,
            })
            .unwrap();
        let fx = r.update(&[spot(&r, gallery)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Cw(CwScene::Library)]);
        // a bolsa: o livro e a maleta abrem as telas do Writer por cima
        let l = r.spec.bag;
        r.bag.open = true;
        let fx = r.update(&[click(l.book.0 + 10, l.book.1 + 10)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::Cw(CwScene::BookStory))));
        let fx = r.update(&[click(l.suitcase.0 + 10, l.suitcase.1 + 10)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::Cw(CwScene::Portfolio))));
        // andar 2: clique duplo no cavalete/prancheta = documento novo; no
        // quadro da parede, a área de escrita
        let mut r = Room::for_program(Program::Writer, Floor::Studio, None, None, &a);
        r.close();
        let easel = spot(&r, Action::Easel);
        r.update(std::slice::from_ref(&easel), 0.0, &a);
        let fx = r.update(&[easel], 0.1, &a);
        assert_eq!(changes(&fx), [SceneId::Cw(CwScene::NewDocument)]);
        let mut r = Room::for_program(Program::Writer, Floor::Studio, None, None, &a);
        r.close();
        let wall = spot(&r, Action::Wall);
        r.update(std::slice::from_ref(&wall), 0.0, &a);
        let fx = r.update(&[wall], 0.1, &a);
        assert_eq!(changes(&fx), [SceneId::Writer]);
        // o mesmo prédio do Fine Artist continua levando às telas dele
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.close();
        let fx = r.update(&[spot(&r, Action::Outside)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Street]);
    }

    /// Fixa o que a sala sorteia (fantasia, pose, obra-prima), para comparar
    /// quadros de duas versões do código.
    fn pin(r: &mut Room) {
        if let Some(c) = r.character.as_mut() {
            c.image = if gags::MCZEE_COSTUMES.contains(&c.image) {
                gags::MCZEE_COSTUMES[0]
            } else {
                gags::MAGGIE_POSES[0]
            };
        }
        if r.painting.is_some() {
            r.painting = Some((0x6d37, 0x6d39));
        }
    }

    /// Estados de uma sala no despejo: nome e cliques até ele.
    type Steps<'a> = &'a [(&'a str, &'a [(i32, i32)])];

    /// Com `ATELIE_DATA` e `ATELIE_DUMP`: grava quadros das salas do Fine
    /// Artist em estados fixos (chegada, sem balão, bolsa aberta, menu do
    /// personagem, "EPA!", balões dos objetos, elevador, sótão), para
    /// conferir que uma refatoração não muda nada (`cmp` dos `.ppm`).
    #[test]
    fn dump_fa_rooms_for_comparison() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        if std::env::var_os("ATELIE_DUMP").is_none() {
            return;
        }
        let mut f = Frame::new(640, 480);
        let shot = |r: &mut Room, f: &mut Frame, name: &str| {
            pin(r);
            r.draw(f, &a);
            crate::scenes::testutil::dump(f, name);
        };
        let steps: &[(Floor, Steps)] = &[
            (
                Floor::Lobby,
                &[
                    ("base", &[(140, 300)]),
                    ("bag", &[(150, 425)]),
                    ("helper", &[(60, 300)]),
                    ("helper_out", &[(400, 300)]),
                    ("quit", &[(600, 470), (170, 220)]),
                    ("tip", &[(300, 430)]),
                ],
            ),
            (
                Floor::Studio,
                &[
                    ("base", &[(600, 470)]),
                    ("easel", &[(191, 201)]),
                    ("wall", &[(350, 100)]),
                    ("bag", &[(600, 470), (180, 420)]),
                ],
            ),
            (
                Floor::Projects,
                &[
                    ("base", &[(600, 470)]),
                    ("comics", &[(100, 100)]),
                    ("flipbook", &[(320, 60)]),
                    ("sticker", &[(250, 220)]),
                    ("helper", &[(600, 470), (40, 380)]),
                ],
            ),
            (
                Floor::Tricks,
                &[
                    ("base", &[(600, 470)]),
                    ("basic", &[(220, 200)]),
                    ("threed", &[(420, 200)]),
                    ("frame", &[(600, 470), (60, 60)]),
                ],
            ),
            (Floor::Attic, &[("dark", &[(100, 300)])]),
        ];
        for (floor, list) in steps {
            let mut r = Room::new(*floor, None, None, &a).with_user("ana", &a);
            shot(&mut r, &mut f, &format!("fa_{floor:?}_arrival"));
            for (name, clicks) in list.iter() {
                for &(x, y) in clicks.iter() {
                    r.update(&[click(x, y)], 0.0, &a);
                }
                shot(&mut r, &mut f, &format!("fa_{floor:?}_{name}"));
            }
        }
        // saguão: arrival options e o elevador
        let mut r = Room::new(Floor::Lobby, None, None, &a).with_user("ana", &a);
        for k in 0..4 {
            let o = option(&r, &a, k);
            r.update(&[o], 0.0, &a);
            shot(&mut r, &mut f, &format!("fa_lobby_option{k}"));
            r.update(&[click(600, 470)], 0.0, &a);
            r.show_arrival(&a);
        }
        r.update(&[click(600, 470), click(340, 300)], 0.0, &a);
        shot(&mut r, &mut f, "fa_elevator");
        r.update(&[click(590, 180)], 0.0, &a);
        for k in 0..30 {
            r.update(&[], 0.1, &a);
            if k % 6 == 0 {
                shot(&mut r, &mut f, &format!("fa_elevator_ride{k}"));
            }
        }
        shot(&mut r, &mut f, "fa_elevator_arrived");
    }
}
