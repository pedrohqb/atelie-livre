//! As 72 modalidades do pincel (6 páginas de 12 botões), comparadas uma a uma
//! com o Fine Artist original (tabela em `docs/telas/pincel.md`):
//!
//! - páginas 1 e 4: a forma é carimbada em cada pixel do caminho;
//! - página 2: um carimbo a cada movimento do mouse que se afasta 16 pixels do
//!   último ("quanto mais depressa você arrastar, menos peixes");
//! - página 3: bolhas, rabiscos, spray, rachadura, teia, espiral e as trilhas
//!   de ladrilhos 21×21 alinhados ao quadro (ferrovia, rodovia, pegadas, cano,
//!   rio e toca do esquilo);
//! - página 5: formas elásticas de 1 pixel, mostradas na tela enquanto arrasta
//!   e pintadas ao soltar (reta, radial de 15°, espelhos, cria-lados,
//!   cria-curvas, quadrado/círculo/triângulo de dentro para fora, retângulo,
//!   forma oval e triângulo 2);
//! - página 6: giz, lápis de cera e pastel com textura, aquarela, clarear e
//!   escurecer.
//!
//! Tudo em coordenadas do quadro; quem guarda o desfazer é a cena.

use super::canvas::{Canvas, Ink, ORIGIN, line_points};
use super::geom::{
    centered_square_points, centered_triangle_points, circle_points, decimate, dist,
    ellipse_points, polyline_points, rect_points, smooth_points, snap15, thicken,
};
use super::shape::{Cell, Shape, page_shapes};
use crate::rng::XorShift;
use formats::palette::Rgb;
use std::collections::HashMap;

type Px = (i32, i32);

/// Lado dos ladrilhos das trilhas; a grade começa no canto do quadro.
pub const CELL: i32 = 21;
/// Distância mínima entre dois carimbos da página 2.
pub const STAMP_SPACING: i32 = 16;
/// Espessura das linhas dos efeitos da página 3 (bolhas, rabiscos, rachadura,
/// teia e espiral): 2 pixels, medida no original.
const EFFECT_PEN: i32 = 2;
/// Distância entre as voltas da espiral.
const SPIRAL_PITCH: f64 = 10.0;
/// Raio da espiral ao apertar o botão.
const SPIRAL_START: f64 = 7.0;
/// A espiral cresce a distância de cada movimento, até este tanto por movimento.
const SPIRAL_GROWTH: f64 = 10.0;
/// Bolha menor que isto (movimento curto) não é desenhada.
const BUBBLE_MIN: i32 = 3;
/// Desvio máximo de cada ponto dos rabiscos.
const SCRIBBLE_JITTER: i32 = 3;
/// Spray da página 3: raio e pontos por atualização.
const SPRAY_RADIUS: i32 = 10;
const SPRAY_DOTS: u32 = 6;
/// Distância mínima entre os vértices do cria-curvas.
const CURVE_STEP: i32 = 16;

/// Carimbos da página 2 em `ARTIST.PIC`: peixe, cerejas, globo, nuvem, joaninha,
/// flor, montanha, estrela, árvore cheia, árvore seca, floco e carinha.
pub const STAMPS: [u16; 12] = [
    0x164b, 0x16a9, 0x16ab, 0x16ad, 0x16af, 0x16b1, 0x16b3, 0x16b5, 0x16b7, 0x16b9, 0x16bb, 0x16bd,
];

/// Ladrilhos de uma trilha, na ordem: horizontal, vertical e as curvas
/// sul-leste, sul-oeste, norte-oeste e norte-leste (variantes se alternam).
type TrackIds = [&'static [u16]; 6];

const RAILROAD: TrackIds = [
    &[0x11a3],
    &[0x11a5],
    &[0x11a7],
    &[0x11a9],
    &[0x11ab],
    &[0x11ad],
];
const HIGHWAY: TrackIds = [
    &[0x11b9],
    &[0x11b7],
    &[0x11b1],
    &[0x11af],
    &[0x11b3],
    &[0x11b5],
];
const PIPE: TrackIds = [
    &[0x16bf, 0x16d6],
    &[0x16d7, 0x16d8],
    &[0x16df, 0x16e0],
    &[0x16d9, 0x16da],
    &[0x16db, 0x16dc],
    &[0x16dd, 0x16de],
];
const RIVER: TrackIds = [
    &[0x16e1, 0x16e2, 0x16e9, 0x16ea],
    &[0x16e5, 0x16e6, 0x16ed, 0x16ee],
    &[0x16ef, 0x16f0],
    &[0x16e3, 0x16e4],
    &[0x16e7, 0x16e8],
    &[0x16eb, 0x16ec],
];
const GOPHER: TrackIds = [
    &[0x16f1, 0x16f2, 0x16f9, 0x16fa],
    &[0x16f5, 0x16f6, 0x16fd, 0x16fe],
    &[0x16ff, 0x1700],
    &[0x16f3, 0x16f4],
    &[0x16f7, 0x16f8],
    &[0x16fb, 0x16fc],
];
/// Trilhas da página 3 por botão (6 ferrovia, 7 rodovia, 9 cano, 10 rio, 11 toca).
const TRACKS: [(usize, TrackIds); 5] = [
    (6, RAILROAD),
    (7, HIGHWAY),
    (9, PIPE),
    (10, RIVER),
    (11, GOPHER),
];
/// Pegadas (botão 8 da página 3) nas 8 direções, em sentido horário a partir
/// do leste: L, SE, S, SO, O, NO, N, NE.
const FOOTPRINTS: [u16; 8] = [
    0x112d, 0x1199, 0x11a1, 0x119d, 0x1195, 0x1197, 0x119f, 0x119b,
];

/// Lado de uma casa da grade das trilhas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    N,
    E,
    S,
    W,
}

impl Side {
    fn opposite(self) -> Side {
        match self {
            Side::N => Side::S,
            Side::E => Side::W,
            Side::S => Side::N,
            Side::W => Side::E,
        }
    }

    /// Lado pelo qual se sai da casa com o passo `(dx, dy)` (um deles é 0).
    fn of_step(dx: i32, dy: i32) -> Side {
        if dx > 0 {
            Side::E
        } else if dx < 0 {
            Side::W
        } else if dy > 0 {
            Side::S
        } else {
            Side::N
        }
    }
}

/// Ladrilhos de uma trilha de vizinhança 4.
#[derive(Debug, Clone, Default)]
pub struct TrackSet {
    h: Vec<Shape>,
    v: Vec<Shape>,
    se: Vec<Shape>,
    sw: Vec<Shape>,
    nw: Vec<Shape>,
    ne: Vec<Shape>,
}

impl TrackSet {
    fn load(ids: &TrackIds, load: &mut impl FnMut(u16) -> Shape) -> TrackSet {
        let mut sets: Vec<Vec<Shape>> = ids
            .iter()
            .map(|l| l.iter().map(|&id| load(id)).collect())
            .collect();
        let mut next = || {
            if sets.is_empty() {
                Vec::new()
            } else {
                sets.remove(0)
            }
        };
        TrackSet {
            h: next(),
            v: next(),
            se: next(),
            sw: next(),
            nw: next(),
            ne: next(),
        }
    }

    /// Ladrilho que liga os lados `a` e `b` (iguais = reta naquele eixo); a
    /// variante `n` se alterna entre as que existem.
    pub fn tile(&self, a: Side, b: Side, n: usize) -> Option<&Shape> {
        use Side::*;
        let set = match (a.min(b), a.max(b)) {
            (N, S) | (N, N) | (S, S) => &self.v,
            (E, S) => &self.se,
            (S, W) => &self.sw,
            (N, W) => &self.nw,
            (N, E) => &self.ne,
            _ => &self.h,
        };
        (!set.is_empty()).then(|| &set[n % set.len()])
    }
}

/// Imagens do pincel lidas do `ARTIST.PIC`.
#[derive(Debug, Clone, Default)]
pub struct Tiles {
    pub stamps: Vec<Shape>,
    /// Trilhas na ordem de `TRACKS`.
    pub tracks: Vec<TrackSet>,
    pub feet: Vec<Shape>,
}

impl Tiles {
    /// Monta tudo com `load(id)` (bitmap de `ARTIST.PIC` → forma).
    pub fn load(mut load: impl FnMut(u16) -> Shape) -> Tiles {
        Tiles {
            stamps: STAMPS.iter().map(|&id| load(id)).collect(),
            tracks: TRACKS
                .iter()
                .map(|(_, ids)| TrackSet::load(ids, &mut load))
                .collect(),
            feet: FOOTPRINTS.iter().map(|&id| load(id)).collect(),
        }
    }
}

/// Tintas da página 6.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Medium {
    /// Giz: textura fina, 3/4 dos pixels.
    Chalk,
    /// Lápis de cera: miolo da cor com falhas no tom claro, borda de `rim`
    /// pixels só no tom claro.
    Crayon {
        rim: f32,
    },
    /// Pastel: textura em riscos horizontais.
    Pastel,
    /// Aquarela: metade da cor sobre o que já está pintado.
    Water,
    Lighten,
    Darken,
}

/// Tinta e diâmetro de cada botão da página 6, medidos no original.
const MEDIA: [(Medium, u32); 12] = [
    (Medium::Chalk, 8),
    (Medium::Chalk, 16),
    (Medium::Chalk, 32),
    (Medium::Crayon { rim: 2.0 }, 8),
    (Medium::Crayon { rim: 4.0 }, 16),
    (Medium::Crayon { rim: 4.0 }, 32),
    (Medium::Pastel, 4),
    (Medium::Pastel, 8),
    (Medium::Pastel, 24),
    (Medium::Water, 16),
    (Medium::Lighten, 16),
    (Medium::Darken, 16),
];
/// Quanto o clarear e o escurecer puxam para o branco ou o preto, por traço.
const SHADE: f32 = 0.35;

/// Ruído fixo por pixel de tela (a textura do papel não anda com o pincel).
fn grain(sx: i32, sy: i32, salt: u32, percent: u32) -> bool {
    let mut h = (sx as u32).wrapping_mul(0x9E37_79B1)
        ^ (sy as u32).wrapping_mul(0x85EB_CA77)
        ^ salt.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    h % 100 < percent
}

/// Traço em andamento.
#[derive(Debug, Clone)]
pub enum Stroke {
    /// Páginas 1 e 4: forma carimbada em cada pixel do caminho.
    Trail {
        last: Px,
    },
    /// Página 2: carimbo no movimento que se afasta 16 pixels do último.
    Spaced {
        last: Px,
    },
    Bubbles {
        last: Px,
    },
    Scribble {
        last: Px,
        rng: XorShift,
    },
    Spray {
        pos: Px,
        rng: XorShift,
    },
    Spiral {
        center: Px,
        last: Px,
        r: f64,
    },
    /// Trilha de ladrilhos; `set` é `None` nas pegadas.
    Track {
        set: Option<usize>,
        cell: Px,
        entry: Option<Side>,
        n: usize,
    },
    /// Forma elástica (rachadura, teia e página 5): só pinta ao soltar.
    Elastic {
        anchor: Px,
        pointer: Px,
        verts: Vec<Px>,
        seed: u32,
    },
    /// Página 6; `levels` evita pintar duas vezes o mesmo pixel no traço
    /// (2 = miolo, 1 = borda do lápis de cera).
    Medium {
        last: Px,
        levels: Vec<u8>,
    },
}

impl Stroke {
    pub fn is_elastic(&self) -> bool {
        matches!(self, Stroke::Elastic { .. })
    }
}

/// O pincel: modalidade escolhida, imagens e o estado das figuras de mais de
/// um traço (cria-lados e triângulo 2).
pub struct Pen {
    page: u8,
    k: usize,
    shape: Option<Shape>,
    tiles: Tiles,
    pal: Vec<Rgb>,
    nearest: HashMap<[u8; 3], u8>,
    /// Fim do último lado do cria-lados.
    side_end: Option<Px>,
    /// Primeiro lado do triângulo 2.
    tri: Option<(Px, Px)>,
}

impl Pen {
    pub fn new(tiles: Tiles, pal: Vec<Rgb>) -> Pen {
        let mut p = Pen {
            page: 1,
            k: 0,
            shape: None,
            tiles,
            pal,
            nearest: HashMap::new(),
            side_end: None,
            tri: None,
        };
        p.select(1, 0);
        p
    }

    /// Escolhe o botão `k` da página `page`; termina a figura em andamento.
    pub fn select(&mut self, page: u8, k: usize) {
        self.page = page;
        self.k = k;
        self.shape = match page {
            2 => self.tiles.stamps.get(k).cloned(),
            6 => MEDIA.get(k).map(|&(_, d)| Shape::disc(d)),
            1 | 4 => page_shapes(page).into_iter().nth(k),
            _ => None,
        };
        self.end_figure();
    }

    /// Termina o cria-lados e o triângulo 2 (clicar fora, trocar de ferramenta).
    pub fn end_figure(&mut self) {
        self.side_end = None;
        self.tri = None;
    }

    /// A modalidade só pinta ao soltar o botão.
    pub fn elastic(&self) -> bool {
        self.page == 5 || (self.page == 3 && matches!(self.k, 3 | 4))
    }

    /// Aperta o botão em `p`; pinta o que já aparece no clique e devolve o traço.
    pub fn press(&mut self, c: &mut Canvas, p: Px, ink: &Ink, seed: u32) -> Option<Stroke> {
        if self.elastic() {
            return Some(Stroke::Elastic {
                anchor: p,
                pointer: p,
                verts: vec![p],
                seed,
            });
        }
        let s = match (self.page, self.k) {
            (1 | 2 | 4, _) => {
                c.stamp(self.shape.as_ref()?, p.0, p.1, ink);
                if self.page == 2 {
                    Stroke::Spaced { last: p }
                } else {
                    Stroke::Trail { last: p }
                }
            }
            (3, 0) => Stroke::Bubbles { last: p },
            (3, 1) => {
                plot_all(c, &thicken(&[p], EFFECT_PEN), ink);
                Stroke::Scribble {
                    last: p,
                    rng: XorShift::new(seed),
                }
            }
            (3, 2) => {
                let mut s = Stroke::Spray {
                    pos: p,
                    rng: XorShift::new(seed),
                };
                self.tick(&mut s, c, ink);
                s
            }
            (3, 5) => {
                plot_all(c, &spiral_points(p, 0.0, SPIRAL_START), ink);
                Stroke::Spiral {
                    center: p,
                    last: p,
                    r: SPIRAL_START,
                }
            }
            (3, k) => Stroke::Track {
                set: TRACKS.iter().position(|&(b, _)| b == k),
                cell: (p.0.div_euclid(CELL), p.1.div_euclid(CELL)),
                entry: None,
                n: 0,
            },
            (6, _) => {
                let mut s = Stroke::Medium {
                    last: p,
                    levels: vec![0; c.px.len()],
                };
                self.medium_dab(&mut s, c, p, ink);
                s
            }
            _ => return None,
        };
        Some(s)
    }

    /// O ponteiro foi para `p` com o botão apertado.
    pub fn drag(&mut self, s: &mut Stroke, c: &mut Canvas, p: Px, ink: &Ink) {
        match s {
            Stroke::Trail { last } => {
                if let Some(shape) = self.shape.as_ref() {
                    for q in line_points(*last, p).into_iter().skip(1) {
                        c.stamp(shape, q.0, q.1, ink);
                    }
                }
                *last = p;
            }
            Stroke::Spaced { last } => {
                let (dx, dy) = (p.0 - last.0, p.1 - last.1);
                if dx * dx + dy * dy >= STAMP_SPACING * STAMP_SPACING {
                    if let Some(shape) = self.shape.as_ref() {
                        c.stamp(shape, p.0, p.1, ink);
                    }
                    *last = p;
                }
            }
            Stroke::Bubbles { last } => {
                let r = dist(*last, p);
                if r >= BUBBLE_MIN {
                    plot_all(c, &thicken(&circle_points(p, r), EFFECT_PEN), ink);
                }
                *last = p;
            }
            Stroke::Scribble { last, rng } => {
                if p != *last {
                    let to = (
                        p.0 + rng.offset(SCRIBBLE_JITTER),
                        p.1 + rng.offset(SCRIBBLE_JITTER),
                    );
                    plot_all(c, &thicken(&line_points(*last, to), EFFECT_PEN), ink);
                    *last = to;
                }
            }
            Stroke::Spray { pos, .. } => *pos = p,
            Stroke::Spiral { center, last, r } => {
                let grow = (dist(*last, p) as f64).min(SPIRAL_GROWTH);
                if grow > 0.0 {
                    plot_all(c, &spiral_points(*center, *r, *r + grow), ink);
                    *r += grow;
                }
                *last = p;
            }
            Stroke::Track { .. } => self.track_to(s, c, p, ink),
            Stroke::Elastic { pointer, verts, .. } => {
                *pointer = p;
                if verts.last() != Some(&p) {
                    verts.push(p);
                }
            }
            Stroke::Medium { last, .. } => {
                let from = *last;
                for q in line_points(from, p).into_iter().skip(1) {
                    self.medium_dab(s, c, q, ink);
                }
                if let Stroke::Medium { last, .. } = s {
                    *last = p;
                }
            }
        }
    }

    /// Uma atualização com o botão apertado (o spray borrifa). Verdadeiro se pintou.
    pub fn tick(&mut self, s: &mut Stroke, c: &mut Canvas, ink: &Ink) -> bool {
        let Stroke::Spray { pos, rng } = s else {
            return false;
        };
        let mut n = 0;
        while n < SPRAY_DOTS {
            let (dx, dy) = (rng.offset(SPRAY_RADIUS), rng.offset(SPRAY_RADIUS));
            if dx * dx + dy * dy > SPRAY_RADIUS * SPRAY_RADIUS {
                continue;
            }
            c.plot(pos.0 + dx, pos.1 + dy, ink);
            n += 1;
        }
        true
    }

    /// Solta o botão em `p`: as formas elásticas viram pintura.
    pub fn release(&mut self, mut s: Stroke, c: &mut Canvas, p: Px, ink: &Ink) {
        if let Stroke::Elastic { pointer, verts, .. } = &mut s {
            *pointer = p;
            if verts.last() != Some(&p) {
                verts.push(p);
            }
        }
        let Stroke::Elastic { anchor, .. } = s else {
            return;
        };
        plot_all(c, &self.preview(&s), ink);
        match (self.page, self.k) {
            (5, 4) => self.side_end = Some(p),
            (5, 11) => {
                self.tri = match self.tri {
                    None => Some((anchor, p)),
                    Some(_) => None,
                }
            }
            _ => {}
        }
    }

    /// Pixels da forma elástica, para mostrar na tela enquanto arrasta.
    pub fn preview(&self, s: &Stroke) -> Vec<Px> {
        let Stroke::Elastic {
            anchor: a,
            pointer: p,
            verts,
            seed,
        } = s
        else {
            return Vec::new();
        };
        let (a, p) = (*a, *p);
        match (self.page, self.k) {
            (3, 3) => thicken(&crack_points(a, dist(a, p), *seed), EFFECT_PEN),
            (3, 4) => thicken(
                &web_points(a, dist(a, p) as f64 * 2.0 / 3f64.sqrt()),
                EFFECT_PEN,
            ),
            (5, 1) => line_points(a, snap15(a, p)),
            (5, 2) => {
                let mut v = line_points(a, p);
                v.extend(line_points(a, (2 * a.0 - p.0, p.1)));
                v
            }
            (5, 3) => {
                let mut v = line_points(a, p);
                v.extend(line_points(a, (p.0, 2 * a.1 - p.1)));
                v
            }
            (5, 4) => line_points(self.side_end.unwrap_or(a), p),
            (5, 5) => smooth_points(&decimate(verts, CURVE_STEP), false),
            (5, 6) => centered_square_points(a, p),
            (5, 7) => circle_points(a, dist(a, p)),
            (5, 8) => centered_triangle_points(a, p),
            (5, 9) => rect_points(a, p),
            (5, 10) => ellipse_points(a, p),
            (5, 11) => match self.tri {
                None => line_points(a, p),
                Some((t0, t1)) => polyline_points(&[t1, p, t0], false),
            },
            _ => line_points(a, p),
        }
    }

    /// Anda a trilha até a casa de `p`, desenhando cada casa ao sair dela
    /// (ladrilho escolhido pelos lados de entrada e saída). A casa onde o
    /// traço termina não é desenhada, como no original.
    fn track_to(&mut self, s: &mut Stroke, c: &mut Canvas, p: Px, ink: &Ink) {
        let Stroke::Track {
            set,
            cell,
            entry,
            n,
        } = s
        else {
            return;
        };
        let target = (p.0.div_euclid(CELL), p.1.div_euclid(CELL));
        while *cell != target {
            let (dx, dy) = (target.0 - cell.0, target.1 - cell.1);
            let center = (cell.0 * CELL + CELL / 2, cell.1 * CELL + CELL / 2);
            let step = match set {
                Some(i) => {
                    // trilhas só andam na horizontal e na vertical (vertical primeiro)
                    let step = if dy != 0 {
                        (0, dy.signum())
                    } else {
                        (dx.signum(), 0)
                    };
                    let out = Side::of_step(step.0, step.1);
                    let tile = self
                        .tiles
                        .tracks
                        .get(*i)
                        .and_then(|t| t.tile(entry.unwrap_or(out.opposite()), out, *n));
                    if let Some(tile) = tile {
                        c.stamp(tile, center.0, center.1, ink);
                    }
                    *entry = Some(out.opposite());
                    step
                }
                None => {
                    let step = (dx.signum(), dy.signum());
                    if let Some(foot) = self.tiles.feet.get(foot_index(step)) {
                        c.stamp(foot, center.0, center.1, ink);
                    }
                    step
                }
            };
            *cell = (cell.0 + step.0, cell.1 + step.1);
            *n += 1;
        }
    }

    /// Uma pincelada da página 6 centrada em `p`.
    fn medium_dab(&mut self, s: &mut Stroke, c: &mut Canvas, p: Px, ink: &Ink) {
        let Stroke::Medium { levels, .. } = s else {
            return;
        };
        let Some((medium, d)) = MEDIA.get(self.k).copied() else {
            return;
        };
        let r = d as f32 / 2.0;
        let (x0, y0) = (p.0 - d as i32 / 2, p.1 - d as i32 / 2);
        for y in 0..d as i32 {
            for x in 0..d as i32 {
                let (fx, fy) = (x as f32 + 0.5 - r, y as f32 + 0.5 - r);
                let d2 = fx * fx + fy * fy;
                if d2 > r * r {
                    continue;
                }
                let (cx, cy) = (x0 + x, y0 + y);
                if !c.can_paint(cx, cy) {
                    continue;
                }
                let i = (cy as u32 * c.w + cx as u32) as usize;
                let level = match medium {
                    Medium::Crayon { rim } if d2 > (r - rim) * (r - rim) => 1,
                    _ => 2,
                };
                if levels.get(i).is_none_or(|&l| l >= level) {
                    continue;
                }
                levels[i] = level;
                let (sx, sy) = (cx + ORIGIN.0, cy + ORIGIN.1);
                let under = c.px[i];
                let color = ink.index_at(sx, sy);
                let new = match medium {
                    Medium::Chalk => grain(sx, sy, 1, 76).then_some(color),
                    Medium::Pastel => grain(sx >> 1, sy, 2, 76).then_some(color),
                    Medium::Crayon { .. } if level == 1 => {
                        grain(sx, sy, 5, 78).then(|| self.mix(color, [255; 3], 0.5))
                    }
                    Medium::Crayon { .. } => {
                        if grain(sx, sy, 3, 78) {
                            Some(color)
                        } else {
                            grain(sx, sy, 4, 85).then(|| self.mix(color, [255; 3], 0.5))
                        }
                    }
                    Medium::Water => {
                        let rgb = self.rgb(color);
                        Some(self.mix(under, rgb, 0.5))
                    }
                    Medium::Lighten => Some(self.mix(under, [255; 3], SHADE)),
                    Medium::Darken => Some(self.mix(under, [0; 3], SHADE)),
                };
                if let Some(v) = new {
                    c.px[i] = v;
                }
            }
        }
    }

    fn rgb(&self, idx: u8) -> [u8; 3] {
        self.pal
            .get(idx as usize)
            .map(|c| [c.r, c.g, c.b])
            .unwrap_or([0; 3])
    }

    /// Cor da paleta mais perto de `idx` misturado com `to` na proporção `t`.
    fn mix(&mut self, idx: u8, to: [u8; 3], t: f32) -> u8 {
        if self.pal.len() < 2 {
            return idx;
        }
        let a = self.rgb(idx);
        let m: [u8; 3] =
            std::array::from_fn(|j| (a[j] as f32 * (1.0 - t) + to[j] as f32 * t).round() as u8);
        if let Some(&v) = self.nearest.get(&m) {
            return v;
        }
        let v = self
            .pal
            .iter()
            .enumerate()
            .min_by_key(|(_, c)| {
                let d = |x: u8, y: u8| (x as i32 - y as i32).pow(2);
                d(c.r, m[0]) + d(c.g, m[1]) + d(c.b, m[2])
            })
            .map(|(i, _)| i as u8)
            .unwrap_or(idx);
        self.nearest.insert(m, v);
        v
    }
}

fn plot_all(c: &mut Canvas, pts: &[Px], ink: &Ink) {
    for &(x, y) in pts {
        c.plot(x, y, ink);
    }
}

/// Índice em `FOOTPRINTS` do passo `(dx, dy)` (sinais).
fn foot_index(step: Px) -> usize {
    match step {
        (1, 0) => 0,
        (1, 1) => 1,
        (0, 1) => 2,
        (-1, 1) => 3,
        (-1, 0) => 4,
        (-1, -1) => 5,
        (0, -1) => 6,
        _ => 7,
    }
}

/// Espiral de Arquimedes centrada em `c`, do raio `r0` ao `r1`, com voltas a
/// `SPIRAL_PITCH` pixels uma da outra.
pub fn spiral_points(c: Px, r0: f64, r1: f64) -> Vec<Px> {
    let k = std::f64::consts::TAU / SPIRAL_PITCH;
    let at = |r: f64| {
        let a = r * k;
        (
            c.0 + (r * a.cos()).round() as i32,
            c.1 + (r * a.sin()).round() as i32,
        )
    };
    let mut verts = vec![at(r0)];
    let mut r = r0;
    while r < r1 {
        r = (r + 2.0 / (r * k).max(1.0)).min(r1);
        verts.push(at(r));
    }
    thicken(&polyline_points(&verts, false), EFFECT_PEN)
}

/// Teia de aranha hexagonal (vértices a leste e oeste) de raio `big`: raios e
/// anéis mais juntos no meio, mais anéis nas teias grandes.
pub fn web_points(c: Px, big: f64) -> Vec<Px> {
    let n = 2 + (big / 50.0) as usize;
    let hexagon = |r: f64| -> Vec<Px> {
        (0..6)
            .map(|j| {
                let a = (j as f64 * 60.0).to_radians();
                (
                    c.0 + (r * a.cos()).round() as i32,
                    c.1 + (r * a.sin()).round() as i32,
                )
            })
            .collect()
    };
    let mut out = Vec::new();
    for ring in 1..=n {
        let r = big * (ring as f64 / n as f64).powf(1.3);
        out.extend(polyline_points(&hexagon(r), true));
    }
    for v in hexagon(big) {
        out.extend(line_points(c, v));
    }
    out
}

/// Rachadura: seis braços tortos de comprimento `len` saindo de `c`, cada um
/// com quatro trechos de direção sorteada (fixa pela `seed` do traço).
pub fn crack_points(c: Px, len: i32, seed: u32) -> Vec<Px> {
    let mut rng = XorShift::new(seed);
    let seg = len as f64 / 4.0;
    let mut out = Vec::new();
    for arm in 0..6 {
        let base = arm as f64 * 60.0 + rng.offset(12) as f64;
        let mut at = (c.0 as f64, c.1 as f64);
        let mut verts = vec![c];
        for _ in 0..4 {
            let a = (base + rng.offset(20) as f64).to_radians();
            at = (at.0 + seg * a.cos(), at.1 + seg * a.sin());
            verts.push((at.0.round() as i32, at.1.round() as i32));
        }
        out.extend(polyline_points(&verts, false));
    }
    out
}

/// `true` se a forma tem células que pintam com a tinta.
pub fn has_ink(s: &Shape) -> bool {
    s.cells.contains(&Cell::Ink)
}

#[cfg(test)]
mod tests;
