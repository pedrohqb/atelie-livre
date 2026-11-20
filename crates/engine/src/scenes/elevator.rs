//! Elevador do prédio (pelas portas do saguão ou pelo skate "ELEVADOR" dos
//! andares), como no original: parede verde `6dd3`, portas `6e2b` com a
//! escotilha `6e33`, o mostrador de andares `6e01` no alto, o painel de
//! botões `6e13` à direita e o McZee à esquerda, que pergunta "Subindo? Ou
//! descendo?" (`WHAT 100d[64]`). Um clique num andar do painel fecha as
//! portas e a viagem passa pela escotilha (roteiro `ATAG` 0x190 subindo,
//! 0x191 descendo, um por andar, com o mostrador acompanhando); as portas se
//! abrem para o andar e a sala aparece. Clicar no andar em que se está só
//! abre as portas. Sons: 0x2d8 (motor), 0x2d4 (portas), 0x311 (chegada).

use super::gags::{self, Character};
use super::room::Floor;
use super::room_spec::{Lamp, RoomSpec};
use crate::anim::{self, Animator};
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::Balloon;
use crate::rng::XorShift;
use crate::scene::{Effect, Input, MouseButton};
use crate::ui::{Rect, inside};
use crate::{Frame, SCREEN_H, SCREEN_W};

pub const BACKGROUND: u16 = 0x6dd3;
/// Portas (cor, máscara) e onde ficam; os roteiros da escotilha são
/// gravados em relação a este ponto.
pub const DOORS: (u16, u16) = (0x6e2b, 0x6e31);
pub const DOORS_POS: (i32, i32) = (123, 47);
pub const PORTHOLE: (u16, u16) = (0x6e33, 0x6e39);
pub const PORTHOLE_POS: (i32, i32) = (189, 111);
/// Mostrador no alto e o número aceso de cada andar (S, 2, 3, 4).
pub const INDICATOR: u16 = 0x6e01;
pub const INDICATOR_POS: (i32, i32) = (259, 4);
const LIT: [(u16, (i32, i32)); 4] = [
    (0x6e0b, (270, 9)),
    (0x6e3b, (300, 9)),
    (0x6e5b, (330, 9)),
    (0x6e6b, (361, 9)),
];
/// Painel de botões e o botão aceso do andar atual.
pub const PANEL: (u16, u16) = (0x6e13, 0x6e21);
pub const PANEL_POS: (i32, i32) = (530, 81);
const PANEL_LIT: [(u16, (i32, i32)); 4] = [
    (0x6e23, (574, 240)),
    (0x6e53, (585, 203)),
    (0x6e63, (572, 167)),
    (0x6e73, (567, 127)),
];
/// Áreas clicáveis do painel (botão e nome do andar), de S a 4.
const BUTTONS: [Rect; 4] = [
    (556, 234, 62, 40),
    (566, 196, 54, 36),
    (556, 160, 62, 36),
    (552, 118, 62, 42),
];
/// Números do mostrador no alto (S, 2, 3, 4): também são botões de andar.
const INDICATOR_BUTTONS: [Rect; 4] = [
    (266, 6, 30, 32),
    (296, 6, 30, 32),
    (326, 6, 30, 32),
    (357, 6, 32, 32),
];
/// Porta com a escotilha: abre as portas no andar em que se está.
const DOOR: Rect = (100, 40, 400, 440);
/// Luminária "Tchau" (corpo e placa) e a área do clique.
const TCHAU_POS: (i32, i32) = (46, 6);
const TCHAU: Rect = (46, 6, 56, 130);
/// McZee à esquerda (medido no original).
pub const MCZEE_POS: (i32, i32) = (-30, 180);

/// Escotilha: roteiro da subida e da descida (1,4 s cada, um por andar).
pub const RIDE_UP: u16 = 0x190;
pub const RIDE_DOWN: u16 = 0x191;
const RIDE_STEP: f32 = 1.4;
/// Portas abertas antes de a sala aparecer.
const OPEN_TIME: f32 = 1.0;

pub const SND_MOTOR: u16 = 0x2d8;
pub const SND_DOORS: u16 = 0x2d4;
pub const SND_ARRIVE: u16 = 0x311;

const ROOMS_WHAT: u16 = 0x100d;
const ASK: usize = 64;

/// Figura colorida, máscara (se houver; sem ela, opaca) e canto.
pub type Pic = (u16, Option<u16>, (i32, i32));

/// O que muda no elevador de um programa (ids do `.PIC`/`.SND`/`WHAT`
/// dele). A mecânica (um roteiro por andar, mostrador acompanhando,
/// portas que se abrem para o fundo do andar) é a mesma.
#[derive(Debug, Clone, Copy)]
pub struct ElevatorSpec {
    pub program: Program,
    pub background: u16,
    /// Portas (cor, máscara) e canto; os roteiros da escotilha são gravados
    /// em relação a ele.
    pub doors: (u16, u16),
    pub doors_pos: (i32, i32),
    /// Escotilha separada das portas (FA), desenhada por cima delas.
    pub porthole: Option<((u16, u16), (i32, i32))>,
    /// Mostrador no alto e o número aceso de cada andar (S, 2, 3, 4).
    pub indicator: Pic,
    pub lit: [Pic; 4],
    /// Painel (ou seta) com os andares e o botão aceso (máscara duas
    /// figuras adiante).
    pub panel: ((u16, u16), (i32, i32)),
    pub panel_lit: [(u16, (i32, i32)); 4],
    /// Áreas clicáveis: botões do painel e números do mostrador, de S a 4.
    pub buttons: [Rect; 4],
    pub indicator_buttons: [Rect; 4],
    /// Porta: abre no andar em que se está.
    pub door: Rect,
    /// Luminária "Tchau" (com a fumaça enquanto o "EPA!" está aberto) e a
    /// área do clique.
    pub tchau: Lamp,
    pub tchau_rect: Rect,
    pub mczee_pos: (i32, i32),
    /// Fantasias do McZee quando a sala não manda a dela.
    pub costumes: &'static [u16],
    /// Roteiros da escotilha (subindo, descendo), um por andar.
    pub ride: (u16, u16),
    /// Salas do programa: o fundo do andar (do `.PIC` do kit da sala)
    /// aparece pelas portas abertas.
    pub floor_room: fn(Floor) -> &'static RoomSpec,
    /// Moldura das portas que fica à mostra com elas abertas (px).
    pub doorway_rim: i32,
    /// Balão "Subindo? Ou descendo?": bloco, registro e canto.
    pub what: u16,
    pub ask: usize,
    pub ask_pos: (i32, i32),
    pub snd: ElevatorSounds,
}

/// Sons do elevador (`None`: nenhum).
#[derive(Debug, Clone, Copy)]
pub struct ElevatorSounds {
    /// Motor da viagem.
    pub motor: u16,
    /// O motor tem um trecho em laço (bloco `loop`) que se repete até a
    /// chegada: `PlayLoop` na partida e `StopLoop` ao chegar.
    pub motor_loops: bool,
    /// Portas se abrindo.
    pub doors: Option<u16>,
    pub arrive: u16,
    /// Clique num botão do painel, num número do mostrador e na porta.
    pub button: Option<u16>,
    pub indicator_button: Option<u16>,
    pub door: Option<u16>,
}

/// O elevador do Fine Artist.
pub const ARTIST_ELEVATOR: ElevatorSpec = ElevatorSpec {
    program: Program::Artist,
    background: BACKGROUND,
    doors: DOORS,
    doors_pos: DOORS_POS,
    porthole: Some((PORTHOLE, PORTHOLE_POS)),
    indicator: (INDICATOR, None, INDICATOR_POS),
    lit: [
        (LIT[0].0, None, LIT[0].1),
        (LIT[1].0, None, LIT[1].1),
        (LIT[2].0, None, LIT[2].1),
        (LIT[3].0, None, LIT[3].1),
    ],
    panel: (PANEL, PANEL_POS),
    panel_lit: PANEL_LIT,
    buttons: BUTTONS,
    indicator_buttons: INDICATOR_BUTTONS,
    door: DOOR,
    tchau: super::rooms_data::artist_lamp_at(TCHAU_POS.0, TCHAU_POS.1),
    tchau_rect: TCHAU,
    mczee_pos: MCZEE_POS,
    costumes: gags::MCZEE_COSTUMES,
    ride: (RIDE_UP, RIDE_DOWN),
    floor_room: super::rooms_data::room,
    doorway_rim: 24,
    what: ROOMS_WHAT,
    ask: ASK,
    ask_pos: (118, 192),
    snd: ElevatorSounds {
        motor: SND_MOTOR,
        motor_loops: false,
        doors: Some(SND_DOORS),
        arrive: SND_ARRIVE,
        button: None,
        indicator_button: None,
        door: None,
    },
};

/// O que o elevador pede à sala.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// As portas se abriram neste andar.
    Arrived(Floor),
    /// Clique na luminária ou no "Tchau".
    Quit,
    /// Clique no McZee: a sala abre o menu dele.
    Helper,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Waiting,
    /// Passando por um andar (tempo dentro do trecho).
    Riding {
        t: f32,
    },
    Opening {
        t: f32,
    },
}

const ORDER: [Floor; 4] = [Floor::Lobby, Floor::Studio, Floor::Projects, Floor::Tricks];

fn level(f: Floor) -> usize {
    f.number() as usize - 1
}

pub struct Elevator {
    spec: &'static ElevatorSpec,
    /// Andar em que o elevador está (o do mostrador).
    at: Floor,
    target: Floor,
    phase: Phase,
    ride: Option<Animator>,
    balloon: Option<Balloon>,
    /// "Subindo? Ou descendo?", para mostrar de novo.
    ask: Balloon,
    character: Option<Character>,
}

impl Elevator {
    /// Elevador do programa `p` parado em `at`, com o McZee da sala
    /// (`costume`, se for do `.PIC` do programa; sem ele, sorteia).
    pub fn for_program(
        p: Program,
        at: Floor,
        costume: Option<&Character>,
        assets: &Assets,
        user: &str,
    ) -> Elevator {
        let spec = match p {
            Program::Artist => &ARTIST_ELEVATOR,
            Program::Writer => &super::writer_elevator::WRITER_ELEVATOR,
        };
        let image = costume.filter(|c| c.program == p).map(|c| c.image);
        Elevator::with_spec(spec, at, image, assets, user)
    }

    /// Elevador do Fine Artist parado em `at`; `costume` é a fantasia do McZee (ou sorteia).
    pub fn new(at: Floor, costume: Option<u16>, assets: &Assets, user: &str) -> Elevator {
        Elevator::with_spec(&ARTIST_ELEVATOR, at, costume, assets, user)
    }

    fn with_spec(
        spec: &'static ElevatorSpec,
        at: Floor,
        costume: Option<u16>,
        assets: &Assets,
        user: &str,
    ) -> Elevator {
        let mut rng = XorShift::from_clock();
        let character = match costume {
            Some(image) => Some(Character {
                program: spec.program,
                image,
                x: spec.mczee_pos.0,
                y: spec.mczee_pos.1,
            }),
            None => Character::pick_in(spec.program, spec.costumes, spec.mczee_pos, &mut rng),
        };
        let fallback = Balloon {
            x: 0,
            y: 0,
            w: 154,
            text: "Subindo? Ou descendo?\nClique no andar que você quer visitar.".into(),
            options: Vec::new(),
            links: Vec::new(),
            figures: Vec::new(),
            ..Default::default()
        };
        let mut balloon = Balloon::from_block(
            &assets.dialogs(spec.program, spec.what),
            spec.ask,
            0,
            0,
            user,
        )
        .filter(|b| !b.text.is_empty())
        .unwrap_or(fallback);
        (balloon.x, balloon.y) = spec.ask_pos;
        Elevator {
            spec,
            at,
            target: at,
            phase: Phase::Waiting,
            ride: None,
            balloon: Some(balloon.clone()),
            ask: balloon,
            character,
        }
    }

    pub fn at(&self) -> Floor {
        self.at
    }

    pub fn moving(&self) -> bool {
        self.phase != Phase::Waiting
    }

    pub fn balloon_open(&self) -> bool {
        self.balloon.is_some()
    }

    /// Mostra (ou esconde, com a ajuda desligada) o "Subindo? Ou descendo?".
    pub fn set_ask(&mut self, on: bool) {
        self.balloon = on.then(|| self.ask.clone());
    }

    fn start_step(&mut self, assets: &Assets, fx: &mut Vec<Effect>) {
        let id = if level(self.target) > level(self.at) {
            self.spec.ride.0
        } else {
            self.spec.ride.1
        };
        self.ride = anim::load_exe(assets, self.spec.program, id)
            .map(|a| a.with_offset(self.spec.doors_pos));
        if let Some(a) = &mut self.ride {
            fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
        }
        self.phase = Phase::Riding { t: 0.0 };
    }

    fn snd(&self, id: u16) -> Effect {
        Effect::PlaySound(SoundRef::Snd(self.spec.program, id))
    }

    fn open_doors(&mut self, fx: &mut Vec<Effect>) {
        self.ride = None;
        self.phase = Phase::Opening { t: 0.0 };
        fx.extend(self.spec.snd.doors.map(|id| self.snd(id)));
    }

    /// Escolhe o andar: viaja até ele (ou só abre as portas, se já está nele).
    pub fn choose(&mut self, floor: Floor, assets: &Assets, fx: &mut Vec<Effect>) {
        if self.moving() {
            return;
        }
        self.balloon = None;
        self.target = floor;
        if floor == self.at {
            self.open_doors(fx);
        } else {
            let motor = SoundRef::Snd(self.spec.program, self.spec.snd.motor);
            fx.push(if self.spec.snd.motor_loops {
                Effect::PlayLoop(motor)
            } else {
                Effect::PlaySound(motor)
            });
            self.start_step(assets, fx);
        }
    }

    pub fn update(
        &mut self,
        input: &[Input],
        dt: f32,
        assets: &Assets,
        fx: &mut Vec<Effect>,
    ) -> Option<Outcome> {
        match self.phase {
            Phase::Waiting => {}
            Phase::Riding { t } => {
                let t = t + dt;
                if let Some(a) = &mut self.ride {
                    fx.extend(a.update(dt).into_iter().map(Effect::PlaySound));
                }
                if t >= RIDE_STEP {
                    // passou um andar: o mostrador acompanha
                    let next = if level(self.target) > level(self.at) {
                        level(self.at) + 1
                    } else {
                        level(self.at) - 1
                    };
                    self.at = ORDER[next];
                    if self.at == self.target {
                        if self.spec.snd.motor_loops {
                            // o laço do motor acaba: toca a freada
                            fx.push(Effect::StopLoop);
                        }
                        self.open_doors(fx);
                    } else {
                        self.start_step(assets, fx);
                    }
                } else {
                    self.phase = Phase::Riding { t };
                }
                return None;
            }
            Phase::Opening { t } => {
                let t = t + dt;
                if t >= OPEN_TIME {
                    self.phase = Phase::Waiting;
                    fx.push(self.snd(self.spec.snd.arrive));
                    return Some(Outcome::Arrived(self.at));
                }
                self.phase = Phase::Opening { t };
                return None;
            }
        }
        for i in input {
            let Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } = *i
            else {
                continue;
            };
            // qualquer clique fecha o balão do McZee
            self.balloon = None;
            let spec = self.spec;
            if let Some(k) = spec
                .buttons
                .iter()
                .chain(&spec.indicator_buttons)
                .position(|&r| inside(r, x, y))
            {
                let click = if k < 4 {
                    spec.snd.button
                } else {
                    spec.snd.indicator_button
                };
                fx.extend(click.map(|id| self.snd(id)));
                self.choose(ORDER[k % 4], assets, fx);
                return None;
            }
            if inside(spec.tchau_rect, x, y) {
                return Some(Outcome::Quit);
            }
            if self.character.as_ref().is_some_and(|c| c.hit(assets, x, y)) {
                return Some(Outcome::Helper);
            }
            if inside(spec.door, x, y) {
                fx.extend(spec.snd.door.map(|id| self.snd(id)));
                let at = self.at;
                self.choose(at, assets, fx);
                return None;
            }
        }
        None
    }

    fn pic(&self, frame: &mut Frame, assets: &Assets, (img, mask): (u16, u16), (x, y): (i32, i32)) {
        let p = self.spec.program;
        frame.blit_masked(
            &assets.image(&ImageRef::Pic(p, img)),
            &assets.image(&ImageRef::Pic(p, mask)),
            x,
            y,
        );
    }

    /// Figura com máscara própria ou opaca.
    fn figure(&self, frame: &mut Frame, assets: &Assets, (img, mask, pos): Pic) {
        match mask {
            Some(m) => self.pic(frame, assets, (img, m), pos),
            None => frame.blit(
                &assets.image(&ImageRef::Pic(self.spec.program, img)),
                pos.0,
                pos.1,
            ),
        }
    }

    /// Botão aceso: máscara duas imagens adiante no `.PIC`.
    fn lit(&self, frame: &mut Frame, assets: &Assets, img: u16, (x, y): (i32, i32)) {
        let p = self.spec.program;
        let colour = assets.image(&ImageRef::Pic(p, img));
        match assets.pic_next_id(p, img, 2) {
            Some(m) => frame.blit_masked(&colour, &assets.image(&ImageRef::Pic(p, m)), x, y),
            None => frame.blit(&colour, x, y),
        }
    }

    /// Com as portas abertas, o vão mostra o fundo do andar de destino.
    pub fn draw(&self, frame: &mut Frame, assets: &Assets) {
        self.draw_with_quit(frame, assets, false);
    }

    /// Desenha o elevador; com o "EPA!" da sala aberto (`quit_open`), a
    /// luminária solta fumaça.
    pub fn draw_with_quit(&self, frame: &mut Frame, assets: &Assets, quit_open: bool) {
        let spec = self.spec;
        let p = spec.program;
        frame.fill([20, 70, 30]);
        let bg = assets.image(&ImageRef::Pic(p, spec.background));
        frame.blit(
            &bg,
            (SCREEN_W as i32 - bg.width as i32) / 2,
            (SCREEN_H as i32 - bg.height as i32) / 2,
        );
        self.figure(frame, assets, spec.indicator);
        self.figure(frame, assets, spec.lit[level(self.at)]);
        self.pic(frame, assets, spec.panel.0, spec.panel.1);
        let (img, pos) = spec.panel_lit[level(if self.moving() { self.target } else { self.at })];
        self.lit(frame, assets, img, pos);
        spec.tchau.draw(frame, assets, p, quit_open);
        self.pic(frame, assets, spec.doors, spec.doors_pos);
        if matches!(self.phase, Phase::Opening { .. }) {
            self.draw_doorway(frame, assets);
        } else {
            if let Some((img, pos)) = spec.porthole {
                self.pic(frame, assets, img, pos);
            }
            if let Some(a) = &self.ride {
                a.draw(frame, assets);
            }
        }
        if let Some(c) = &self.character {
            c.draw(frame, assets);
        }
        if let Some(b) = &self.balloon {
            b.draw(frame, assets);
        }
    }

    /// Vão das portas abertas: o fundo do andar dentro do contorno das
    /// portas, deixando a moldura (`doorway_rim`) à mostra.
    fn draw_doorway(&self, frame: &mut Frame, assets: &Assets) {
        let spec = self.spec;
        let rim = spec.doorway_rim;
        let room = (spec.floor_room)(self.target);
        let mask = assets.image(&ImageRef::Pic(spec.program, spec.doors.1));
        let bg = assets.image(&ImageRef::Pic(room.kit.program, room.background));
        let (mw, mh) = (mask.width as i32, mask.height as i32);
        let solid = |x: i32, y: i32| {
            if x < 0 || y < 0 || x >= mw {
                return false;
            }
            if y >= mh {
                return true;
            }
            let i = ((y * mw + x) * 4) as usize;
            let m = &mask.rgba[i..i + 3];
            (m[0] as u32 + m[1] as u32 + m[2] as u32) < 384
        };
        let (ox, oy) = (
            (SCREEN_W as i32 - bg.width as i32) / 2,
            (SCREEN_H as i32 - bg.height as i32) / 2,
        );
        for dy in 0..mh {
            for dx in 0..mw {
                if !(solid(dx, dy)
                    && solid(dx - rim, dy)
                    && solid(dx + rim, dy)
                    && solid(dx, dy - rim))
                {
                    continue;
                }
                let (x, y) = (spec.doors_pos.0 + dx, spec.doors_pos.1 + dy);
                let (bx, by) = (x - ox, y - oy);
                let p = if bx >= 0 && by >= 0 && bx < bg.width as i32 && by < bg.height as i32 {
                    let i = ((by * bg.width as i32 + bx) * 4) as usize;
                    [bg.rgba[i], bg.rgba[i + 1], bg.rgba[i + 2]]
                } else {
                    [0, 0, 0]
                };
                frame.put(x, y, p);
            }
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

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(SoundRef::Snd(_, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    fn run(e: &mut Elevator, secs: f32, a: &Assets, fx: &mut Vec<Effect>) -> Option<Outcome> {
        let mut out = None;
        for _ in 0..(secs / 0.1).round() as usize {
            if let Some(o) = e.update(&[], 0.1, a, fx) {
                out = Some(o);
            }
        }
        out
    }

    #[test]
    fn rides_one_floor_per_step_and_opens() {
        let (_d, a) = empty_assets();
        let mut e = Elevator::new(Floor::Studio, None, &a, "ana");
        assert!(e.balloon_open());
        let mut fx = Vec::new();
        // botão do andar 4 (Truques): dois andares acima
        assert_eq!(e.update(&[click(578, 137)], 0.0, &a, &mut fx), None);
        assert!(!e.balloon_open());
        assert!(e.moving());
        assert_eq!(sounds(&fx), [SND_MOTOR]);
        // cliques durante a viagem não fazem nada
        assert_eq!(e.update(&[click(583, 250)], 0.0, &a, &mut fx), None);
        run(&mut e, 1.5, &a, &mut fx);
        assert_eq!(e.at(), Floor::Projects);
        let mut fx = Vec::new();
        assert_eq!(run(&mut e, 1.4, &a, &mut fx), None);
        assert_eq!(e.at(), Floor::Tricks);
        assert_eq!(sounds(&fx), [SND_DOORS]);
        let mut fx = Vec::new();
        assert_eq!(
            run(&mut e, 1.1, &a, &mut fx),
            Some(Outcome::Arrived(Floor::Tricks))
        );
        assert_eq!(sounds(&fx), [SND_ARRIVE]);
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
    }

    #[test]
    fn same_floor_only_opens_and_tchau_asks_to_quit() {
        let (_d, a) = empty_assets();
        let mut e = Elevator::new(Floor::Lobby, Some(0x6d93), &a, "ana");
        let mut fx = Vec::new();
        assert_eq!(
            e.update(&[click(70, 110)], 0.0, &a, &mut fx),
            Some(Outcome::Quit)
        );
        e.update(&[click(583, 250)], 0.0, &a, &mut fx);
        assert_eq!(sounds(&fx), [SND_DOORS]);
        assert_eq!(
            run(&mut e, 1.1, &a, &mut fx),
            Some(Outcome::Arrived(Floor::Lobby))
        );
    }

    /// Números do mostrador são botões de andar; a porta abre no andar em
    /// que se está; o McZee pede o menu dele à sala.
    #[test]
    fn indicator_door_and_mczee() {
        let (_d, a) = empty_assets();
        let mut e = Elevator::new(Floor::Lobby, Some(0x6d93), &a, "ana");
        let mut fx = Vec::new();
        e.update(&[click(343, 22)], 0.0, &a, &mut fx);
        assert!(e.moving());
        assert_eq!(e.target, Floor::Projects);
        let mut e = Elevator::new(Floor::Studio, Some(0x6d93), &a, "ana");
        let mut fx = Vec::new();
        e.update(&[click(320, 230)], 0.0, &a, &mut fx);
        assert_eq!(sounds(&fx), [SND_DOORS]);
        assert_eq!(
            run(&mut e, 1.1, &a, &mut fx),
            Some(Outcome::Arrived(Floor::Studio))
        );
        let mut e = Elevator::new(Floor::Lobby, None, &a, "ana");
        let mut fx = Vec::new();
        // sem dados, o McZee é o quadrado de 32×32 em MCZEE_POS
        assert_eq!(
            e.update(
                &[click(MCZEE_POS.0 + 10, MCZEE_POS.1 + 10)],
                0.0,
                &a,
                &mut fx
            ),
            Some(Outcome::Helper)
        );
        assert_eq!(
            e.update(&[click(70, 40)], 0.0, &a, &mut fx),
            Some(Outcome::Quit)
        );
        e.set_ask(true);
        assert!(e.balloon_open());
        e.set_ask(false);
        assert!(!e.balloon_open());
    }

    /// Com os dados reais: a escotilha passa pela barra preta de 0x190 na
    /// posição medida na gravação (224, 184), e as portas abertas mostram o
    /// fundo do andar.
    #[test]
    fn real_ride_matches_capture() {
        let Some(a) = assets() else { return };
        let mut e = Elevator::new(Floor::Studio, Some(0x6d93), &a, "ana");
        let mut fx = Vec::new();
        e.choose(Floor::Projects, &a, &mut fx);
        let r = e.ride.as_ref().expect("roteiro da escotilha");
        assert_eq!(r.position(), (224, 184));
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "elevator_ride");
        run(&mut e, 1.5, &a, &mut fx);
        e.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "elevator_open");
        // no meio do vão aparece a parede da Oficina, não a porta roxa
        let bg = a.image(&ImageRef::Pic(
            Program::Artist,
            super::super::rooms_data::PROJECTS_BG,
        ));
        let i = ((300 * bg.width + 320) * 4) as usize;
        assert_eq!(f.pixel(320, 300)[..3], bg.rgba[i..i + 3]);
    }
}
