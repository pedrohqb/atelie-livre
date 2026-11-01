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
use crate::anim::{self, Animator};
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::Balloon;
use crate::rng::XorShift;
use crate::scene::{Effect, Input, MouseButton};
use crate::{Frame, SCREEN_H, SCREEN_W};

type Rect = (i32, i32, i32, i32);

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

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
/// Luminária "Tchau".
const TCHAU_IMG: (u16, u16) = (0x681b, 0x6821);
const TCHAU_POS: (i32, i32) = (46, 6);
const TCHAU: Rect = (46, 90, 56, 45);
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

/// O que o elevador pede à sala.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// As portas se abriram neste andar.
    Arrived(Floor),
    /// Clique no "Tchau".
    Quit,
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
    /// Andar em que o elevador está (o do mostrador).
    at: Floor,
    target: Floor,
    phase: Phase,
    ride: Option<Animator>,
    balloon: Option<Balloon>,
    character: Option<Character>,
}

impl Elevator {
    /// Elevador parado em `at`; `costume` é a fantasia do McZee (ou sorteia).
    pub fn new(at: Floor, costume: Option<u16>, assets: &Assets, user: &str) -> Elevator {
        let mut rng = XorShift::from_clock();
        let character = match costume {
            Some(image) => Some(Character {
                image,
                x: MCZEE_POS.0,
                y: MCZEE_POS.1,
            }),
            None => Character::pick(gags::MCZEE_COSTUMES, MCZEE_POS, &mut rng),
        };
        let fallback = Balloon {
            x: 0,
            y: 0,
            w: 154,
            text: "Subindo? Ou descendo?\nClique no andar que você quer visitar.".into(),
            options: Vec::new(),
        };
        let mut balloon = Balloon::from_block(
            &assets.dialogs(Program::Artist, ROOMS_WHAT),
            ASK,
            0,
            0,
            user,
        )
        .filter(|b| !b.text.is_empty())
        .unwrap_or(fallback);
        balloon.x = 118;
        balloon.y = 192;
        Elevator {
            at,
            target: at,
            phase: Phase::Waiting,
            ride: None,
            balloon: Some(balloon),
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

    fn start_step(&mut self, assets: &Assets, fx: &mut Vec<Effect>) {
        let id = if level(self.target) > level(self.at) {
            RIDE_UP
        } else {
            RIDE_DOWN
        };
        self.ride = anim::load_exe(assets, Program::Artist, id).map(|a| a.with_offset(DOORS_POS));
        if let Some(a) = &mut self.ride {
            fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
        }
        self.phase = Phase::Riding { t: 0.0 };
    }

    fn open_doors(&mut self, fx: &mut Vec<Effect>) {
        self.ride = None;
        self.phase = Phase::Opening { t: 0.0 };
        fx.push(Effect::PlaySound(SoundRef::Snd(Program::Artist, SND_DOORS)));
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
            fx.push(Effect::PlaySound(SoundRef::Snd(Program::Artist, SND_MOTOR)));
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
                    fx.push(Effect::PlaySound(SoundRef::Snd(
                        Program::Artist,
                        SND_ARRIVE,
                    )));
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
            if let Some(k) = BUTTONS.iter().position(|&r| inside(r, x, y)) {
                self.choose(ORDER[k], assets, fx);
                return None;
            }
            if inside(TCHAU, x, y) {
                return Some(Outcome::Quit);
            }
        }
        None
    }

    fn pic(frame: &mut Frame, assets: &Assets, (img, mask): (u16, u16), (x, y): (i32, i32)) {
        frame.blit_masked(
            &assets.image(&ImageRef::Pic(Program::Artist, img)),
            &assets.image(&ImageRef::Pic(Program::Artist, mask)),
            x,
            y,
        );
    }

    /// Botão aceso: máscara duas imagens adiante no `.PIC`.
    fn lit(frame: &mut Frame, assets: &Assets, img: u16, (x, y): (i32, i32)) {
        let colour = assets.image(&ImageRef::Pic(Program::Artist, img));
        match assets.pic_next_id(Program::Artist, img, 2) {
            Some(m) => frame.blit_masked(
                &colour,
                &assets.image(&ImageRef::Pic(Program::Artist, m)),
                x,
                y,
            ),
            None => frame.blit(&colour, x, y),
        }
    }

    /// Com as portas abertas, o vão mostra o fundo do andar de destino.
    pub fn draw(&self, frame: &mut Frame, assets: &Assets) {
        let floor_bg = self.target.background();
        frame.fill([20, 70, 30]);
        let bg = assets.image(&ImageRef::Pic(Program::Artist, BACKGROUND));
        frame.blit(
            &bg,
            (SCREEN_W as i32 - bg.width as i32) / 2,
            (SCREEN_H as i32 - bg.height as i32) / 2,
        );
        frame.blit(
            &assets.image(&ImageRef::Pic(Program::Artist, INDICATOR)),
            INDICATOR_POS.0,
            INDICATOR_POS.1,
        );
        let (img, pos) = LIT[level(self.at)];
        frame.blit(
            &assets.image(&ImageRef::Pic(Program::Artist, img)),
            pos.0,
            pos.1,
        );
        Self::pic(frame, assets, PANEL, PANEL_POS);
        let (img, pos) = PANEL_LIT[level(if self.moving() { self.target } else { self.at })];
        Self::lit(frame, assets, img, pos);
        Self::pic(frame, assets, TCHAU_IMG, TCHAU_POS);
        Self::pic(frame, assets, DOORS, DOORS_POS);
        if matches!(self.phase, Phase::Opening { .. }) {
            self.draw_doorway(frame, assets, floor_bg);
        } else {
            Self::pic(frame, assets, PORTHOLE, PORTHOLE_POS);
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
    /// portas, deixando a moldura (24 px) à mostra.
    fn draw_doorway(&self, frame: &mut Frame, assets: &Assets, floor_bg: u16) {
        const RIM: i32 = 24;
        let mask = assets.image(&ImageRef::Pic(Program::Artist, DOORS.1));
        let bg = assets.image(&ImageRef::Pic(Program::Artist, floor_bg));
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
                    && solid(dx - RIM, dy)
                    && solid(dx + RIM, dy)
                    && solid(dx, dy - RIM))
                {
                    continue;
                }
                let (x, y) = (DOORS_POS.0 + dx, DOORS_POS.1 + dy);
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
            Floor::Projects.background(),
        ));
        let i = ((300 * bg.width + 320) * 4) as usize;
        assert_eq!(f.pixel(320, 300)[..3], bg.rgba[i..i + 3]);
    }
}
