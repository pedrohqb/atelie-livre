//! Ferramenta Seleciona-adesivos da tela de pintura: rolos na faixa de opções
//! (o rolo de bichos animados e os 28 rolos de clip-art, 7 por página), tira de
//! filme com as miniaturas, adesivo na mão e animação até colar no quadro.
//! Geometria medida no original (spec 2c e 2d).

use super::sticker_scripts::{SCRIPT_FPS, SCRIPTS, Script};
use crate::assets::{Assets, ImageRef, Program};
use crate::clipart::{ClipRoll, load_rolls};
use crate::paint::stamp::Stamp;
use crate::paint::{Cell, ORIGIN, Shape};
use crate::{Frame, Image};
use formats::palette::Rgb;
use std::rc::Rc;

/// Primeiro rolo: quadros de cada bicho em `ARTIST.PIC` (`N` cor, `N+6` máscara).
pub const ANIMALS: [&[u16]; 16] = [
    &[0x753b, 0x7543, 0x754b],
    &[0x7553, 0x755b, 0x7563],
    &[0x757b, 0x7583],
    &[0x758b, 0x7593],
    &[0x759b, 0x75a3, 0x75ab, 0x75b3],
    &[0x75bb, 0x75c3, 0x75cb, 0x75d3],
    &[0x75db, 0x75e3, 0x75eb, 0x75f3],
    &[0x75fb, 0x7603, 0x760b, 0x7613, 0x761b, 0x7623],
    &[0x762b, 0x7633, 0x763b, 0x7643, 0x764b, 0x7653],
    &[0x765b, 0x7663, 0x766b, 0x7673, 0x767b],
    &[0x7683, 0x768b, 0x7693, 0x769b],
    &[0x76a3, 0x76ab, 0x76b3, 0x76bb],
    &[0x76c3, 0x76cb, 0x76d3, 0x76db, 0x76e3, 0x76eb],
    &[0x76f3, 0x76fb, 0x7703, 0x770b],
    &[0x7713, 0x771b, 0x7723, 0x772b],
    &[0x7733, 0x773b],
];

/// Miniaturas em traço de cada bicho, na tira de filme.
pub const THUMBS: [u16; 16] = [
    0x7743, 0x77ec, 0x77ed, 0x77ee, 0x77ef, 0x77f0, 0x77f1, 0x77f2, 0x77f3, 0x77f4, 0x77f5, 0x77f6,
    0x77f7, 0x77f8, 0x77f9, 0x77fa,
];

/// Animação aproximada (bichos sem roteiro gravado): quadros no lugar.
const FALLBACK_FPS: f32 = 8.0;
const FALLBACK_LOOPS: usize = 2;

pub const ROLLS_PER_PAGE: usize = 7;
const SLOT_X0: i32 = 23;
const SLOT_W: i32 = 52;
const STRIP_Y: i32 = 58;
const ARROW_UP: (i32, i32, i32, i32) = (504, 58, 16, 14);
const ARROW_DOWN: (i32, i32, i32, i32) = (504, 90, 16, 14);
const FILM_Y: i32 = 99;
const FILM_H: i32 = 368;
const FILM_W: i32 = 42;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const GREEN: [u8; 3] = [71, 140, 80];
const YELLOW: [u8; 3] = [255, 238, 76];

type Rect = (i32, i32, i32, i32);

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Artist, id)
}

fn canvas_rect() -> Rect {
    (
        ORIGIN.0,
        ORIGIN.1,
        crate::paint::CANVAS_W as i32,
        crate::paint::CANVAS_H as i32,
    )
}

/// Retângulo do `k`-ésimo rolo da página (moldura de 52 pixels).
fn slot_rect(k: usize) -> Rect {
    (SLOT_X0 + SLOT_W * k as i32, STRIP_Y, SLOT_W, 42)
}

/// Geometria da tira de filme sob o rolo da casa `slot`.
struct Film {
    x: i32,
    green: bool,
}

impl Film {
    fn rect(&self) -> Rect {
        (self.x, FILM_Y, FILM_W, FILM_H)
    }

    fn cells(&self) -> usize {
        if self.green { 9 } else { 8 }
    }

    /// Casa `j` (visível) da tira: verde 28×32 a cada 39 px; amarela 34×34 a cada 41.
    fn cell(&self, j: usize) -> Rect {
        if self.green {
            (self.x + 7, 103 + 39 * j as i32, 28, 32)
        } else {
            (self.x + 4, 103 + 41 * j as i32, 34, 34)
        }
    }

    fn up(&self) -> Rect {
        (self.x + 6, 452, 12, 12)
    }

    fn down(&self) -> Rect {
        (self.x + 24, 452, 12, 12)
    }
}

/// Adesivo colocado no quadro, pronto para virar objeto: imagem, canto
/// superior esquerdo (coordenadas do quadro) e origem (rolo, casa).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub stamp: Stamp,
    pub x: i32,
    pub y: i32,
    pub roll: usize,
    pub item: usize,
}

/// Adesivo escolhido, ainda não colocado.
enum Held {
    Animal(usize),
    /// Clip-art pronto e a casa do rolo de onde veio.
    Clip(Stamp, usize),
}

/// Roteiro de um bicho lido de `A-MAG100.ANI` (`formats::atag`): células
/// (imagem, máscara, duração em ms) e passos (célula, deslocamento).
pub struct AniAnimal {
    /// Imagem, máscara (preto = opaco, já invertida em relação ao `.ANI`) e duração em ms.
    cells: Vec<(Rc<Image>, Rc<Image>, u32)>,
    steps: Vec<(usize, i32, i32)>,
    /// Tamanho da célula 0 (o adesivo em repouso, centrado no clique).
    size0: (i32, i32),
}

impl AniAnimal {
    /// Imagem e posição de tela no instante `t_ms`, com o clique em `(sx, sy)`;
    /// `None` quando o roteiro acabou.
    fn frame_at(&self, t_ms: u32, sx: i32, sy: i32) -> Option<(Rc<Image>, Rc<Image>, i32, i32)> {
        let mut acc = 0u32;
        for &(cell, x, y) in &self.steps {
            let (img, mask, ms) = self.cells.get(cell)?.clone();
            acc = acc.saturating_add(ms.max(1));
            if t_ms < acc {
                let (x0, y0) = (sx - self.size0.0 / 2, sy - self.size0.1 / 2);
                return Some((img, mask, x0 + x, y0 + y));
            }
        }
        None
    }
}

/// Bicho colocado, se animando. `cx, cy`: clique no quadro.
struct Playing {
    animal: usize,
    cx: i32,
    cy: i32,
    t: f32,
    /// Metade da altura do primeiro quadro (base dos quadros sem roteiro).
    base_half: i32,
    /// Imagem de repouso (colada ao terminar) e canto superior esquerdo no quadro.
    rest: Option<(Stamp, i32, i32)>,
}

/// Desenho de clip-art (1 bit) como carimbo: preto pinta preto, o branco interno
/// pinta branco e o branco ligado à borda é transparente.
pub fn line_art_stamp(img: &Image, pal: &[Rgb]) -> Stamp {
    let shape = Shape::from_stamp(img);
    let black = crate::paint::store::nearest(pal, 0, 0, 0);
    let white = crate::paint::store::nearest(pal, 255, 255, 255);
    Stamp {
        w: shape.w,
        h: shape.h,
        px: shape
            .cells
            .iter()
            .map(|c| match c {
                Cell::Clear => None,
                Cell::Ink => Some(black),
                Cell::White => Some(white),
            })
            .collect(),
    }
}

pub struct Stickers {
    clip: Vec<ClipRoll>,
    page: usize,
    roll: usize,
    film_open: bool,
    scroll: usize,
    held: Option<Held>,
    playing: Option<Playing>,
    /// Roteiros do `.ANI` (vazio se a biblioteca faltar: usa os gravados).
    ani: Vec<AniAnimal>,
}

/// Lê os roteiros dos bichos de `A-MAG100.ANI`.
fn load_ani(assets: &Assets) -> Vec<AniAnimal> {
    assets
        .animations("A-MAG100.ANI")
        .into_iter()
        .filter_map(|(_, script, images)| {
            let sc = formats::atag::parse(&script).ok()?;
            let cells: Vec<(Rc<Image>, Rc<Image>, u32)> = sc
                .cells
                .iter()
                .map(|c| {
                    let img = assets.image(images.get(c.image as usize)?);
                    // no .ANI a máscara é branca onde é opaco; aqui preto = opaco
                    let m = assets.image(images.get(c.mask as usize)?);
                    let mut inv = (*m).clone();
                    for px in inv.rgba.chunks_exact_mut(4) {
                        px[0] = 255 - px[0];
                        px[1] = 255 - px[1];
                        px[2] = 255 - px[2];
                    }
                    Some((img, Rc::new(inv), c.duration_ms))
                })
                .collect::<Option<_>>()?;
            let steps: Vec<(usize, i32, i32)> = sc
                .playback()
                .iter()
                .map(|st| (st.cell as usize, st.x as i32, st.y as i32))
                .collect();
            if cells.is_empty() || steps.iter().any(|&(c, _, _)| c >= cells.len()) {
                return None;
            }
            let first = cells[0].0.clone();
            Some(AniAnimal {
                cells,
                steps,
                size0: (first.width as i32, first.height as i32),
            })
        })
        .collect()
}

impl Stickers {
    pub fn new(assets: &Assets) -> Stickers {
        let ani = load_ani(assets);
        Stickers {
            ani: if ani.len() == ANIMALS.len() {
                ani
            } else {
                Vec::new()
            },
            clip: load_rolls(assets),
            page: 0,
            roll: 0,
            film_open: false,
            scroll: 0,
            held: None,
            playing: None,
        }
    }

    pub fn roll_count(&self) -> usize {
        1 + self.clip.len()
    }

    fn page_count(&self) -> usize {
        self.roll_count().div_ceil(ROLLS_PER_PAGE)
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn roll(&self) -> usize {
        self.roll
    }

    pub fn film_open(&self) -> bool {
        self.film_open
    }

    pub fn holding(&self) -> bool {
        self.held.is_some()
    }

    /// Índice do bicho na mão, se for um bicho.
    pub fn held_animal(&self) -> Option<usize> {
        match self.held {
            Some(Held::Animal(a)) => Some(a),
            _ => None,
        }
    }

    pub fn ani_len(&self) -> usize {
        self.ani.len()
    }

    pub fn is_animating(&self) -> bool {
        self.playing.is_some()
    }

    fn item_count(&self, roll: usize) -> usize {
        if roll == 0 {
            ANIMALS.len()
        } else {
            self.clip.get(roll - 1).map_or(0, |r| r.items.len())
        }
    }

    fn film(&self) -> Film {
        let slot = self.roll % ROLLS_PER_PAGE;
        Film {
            x: 33 + SLOT_W * slot as i32,
            green: self.roll == 0,
        }
    }

    /// Fecha a tira e solta o que estiver na mão (troca de ferramenta etc.).
    pub fn reset(&mut self) {
        self.film_open = false;
        self.held = None;
    }

    /// Termina a animação em curso: devolve a imagem de repouso a colocar no
    /// quadro, se o bicho tem uma (a cobra vai embora sem deixar nada).
    pub fn finish(&mut self) -> Option<Placed> {
        let p = self.playing.take()?;
        let (stamp, x, y) = p.rest?;
        Some(Placed {
            stamp,
            x,
            y,
            roll: 0,
            item: p.animal,
        })
    }

    /// Avança a animação; devolve o adesivo a colocar se ela terminou neste passo.
    pub fn update(&mut self, dt: f32) -> Option<Placed> {
        let p = self.playing.as_mut()?;
        p.t += if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let done = self
            .playing
            .as_ref()
            .is_some_and(|p| self.frame_images(p).is_none());
        if done { self.finish() } else { None }
    }

    /// Imagem de repouso do adesivo (rolo, casa), para reabrir um desenho.
    pub fn stamp_for(
        &self,
        roll: usize,
        item: usize,
        assets: &Assets,
        pal: &[Rgb],
    ) -> Option<Stamp> {
        if roll == 0 {
            let frames = ANIMALS.get(item)?;
            return Some(match (self.ani.get(item), &SCRIPTS[item]) {
                (Some(an), _) => Stamp::from_images(&an.cells[0].0, &an.cells[0].1, pal),
                (None, Some(sc)) => {
                    let id = sc.rest.map_or(frames[0], |(id, _, _)| id);
                    Stamp::from_images(&assets.image(&pic(id)), &assets.image(&pic(id + 6)), pal)
                }
                (None, None) => Stamp::from_images(
                    &assets.image(&pic(frames[0])),
                    &assets.image(&pic(frames[0] + 6)),
                    pal,
                ),
            });
        }
        let art = self.clip.get(roll - 1)?.items.get(item)?.art()?;
        Some(line_art_stamp(&art, pal))
    }

    /// Imagens visíveis no instante atual: (id, x, y na tela, `x, y` é o centro
    /// da base, como nos quadros sem roteiro); `None` = acabou.
    fn frame_images(&self, p: &Playing) -> Option<Vec<Frame3>> {
        let (sx, sy) = (ORIGIN.0 + p.cx, ORIGIN.1 + p.cy);
        if let Some(an) = self.ani.get(p.animal) {
            let (img, mask, x, y) = an.frame_at((p.t * 1000.0) as u32, sx, sy)?;
            return Some(vec![Frame3::Lib(img, mask, x, y)]);
        }
        match &SCRIPTS[p.animal] {
            Some(s) => script_images(s, (p.t * SCRIPT_FPS) as usize, sx, sy).map(|v| {
                v.into_iter()
                    .map(|(id, x, y)| Frame3::Pic(id, x, y, false))
                    .collect()
            }),
            None => {
                let frames = ANIMALS[p.animal];
                let k = (p.t * FALLBACK_FPS) as usize;
                if k >= frames.len() * FALLBACK_LOOPS {
                    return None;
                }
                // sem roteiro gravado: quadros no lugar, alinhados pelo centro da
                // base do primeiro quadro (que fica centrado no clique)
                Some(vec![Frame3::Pic(
                    frames[k % frames.len()],
                    sx,
                    sy + p.base_half,
                    true,
                )])
            }
        }
    }

    /// Clique com a ferramenta de adesivos: `None` se não era daqui,
    /// `Some(Some(adesivo))` se um adesivo de clip-art foi colocado.
    pub fn click(
        &mut self,
        x: i32,
        y: i32,
        assets: &Assets,
        pal: &[Rgb],
    ) -> Option<Option<Placed>> {
        if self.film_open {
            let film = self.film();
            if inside(film.rect(), x, y) {
                let n = self.item_count(self.roll);
                let max_scroll = n.saturating_sub(film.cells());
                if inside(film.up(), x, y) {
                    self.scroll = self.scroll.saturating_sub(1);
                } else if inside(film.down(), x, y) {
                    self.scroll = (self.scroll + 1).min(max_scroll);
                } else if let Some(j) = (0..film.cells()).find(|&j| inside(film.cell(j), x, y)) {
                    let item = self.scroll + j;
                    if item < n {
                        self.held = self.make_held(item, pal);
                        if self.held.is_some() {
                            self.film_open = false;
                        }
                    }
                }
                return Some(None);
            }
        }
        if inside(ARROW_UP, x, y) {
            let pages = self.page_count();
            self.page = (self.page + pages - 1) % pages;
            self.reset();
            return Some(None);
        }
        if inside(ARROW_DOWN, x, y) {
            self.page = (self.page + 1) % self.page_count();
            self.reset();
            return Some(None);
        }
        if let Some(k) = (0..ROLLS_PER_PAGE).find(|&k| inside(slot_rect(k), x, y)) {
            let roll = self.page * ROLLS_PER_PAGE + k;
            if roll < self.roll_count() {
                if self.film_open && roll == self.roll {
                    self.film_open = false;
                } else {
                    self.roll = roll;
                    self.scroll = 0;
                    self.film_open = true;
                }
                self.held = None;
            }
            return Some(None);
        }
        if self.held.is_some() && inside(canvas_rect(), x, y) {
            let (cx, cy) = (x - ORIGIN.0, y - ORIGIN.1);
            match self.held.take() {
                Some(Held::Clip(stamp, item)) => {
                    let (x, y) = (cx - stamp.w as i32 / 2, cy - stamp.h as i32 / 2);
                    return Some(Some(Placed {
                        stamp,
                        x,
                        y,
                        roll: self.roll,
                        item,
                    }));
                }
                Some(Held::Animal(animal)) => {
                    let stamp = |id: u16| {
                        Stamp::from_images(
                            &assets.image(&pic(id)),
                            &assets.image(&pic(id + 6)),
                            pal,
                        )
                    };
                    let base_half = assets.image(&pic(ANIMALS[animal][0])).height as i32 / 2;
                    let rest = match (self.ani.get(animal), &SCRIPTS[animal]) {
                        // com o roteiro do .ANI, o adesivo volta à célula 0 no clique
                        (Some(an), _) => {
                            let s = Stamp::from_images(&an.cells[0].0, &an.cells[0].1, pal);
                            let (w, h) = (s.w as i32, s.h as i32);
                            Some((s, cx - w / 2, cy - h / 2))
                        }
                        (None, sc) => match sc {
                            // a cobra (sem repouso) vai embora pela direita
                            Some(sc) => sc
                                .rest
                                .map(|(id, dx, dy)| (stamp(id), cx + dx as i32, cy + dy as i32)),
                            None => {
                                let s = stamp(ANIMALS[animal][0]);
                                let (w, h) = (s.w as i32, s.h as i32);
                                Some((s, cx - w / 2, cy - h / 2))
                            }
                        },
                    };
                    self.playing = Some(Playing {
                        animal,
                        cx,
                        cy,
                        t: 0.0,
                        base_half,
                        rest,
                    });
                }
                None => {}
            }
            return Some(None);
        }
        None
    }

    fn make_held(&self, item: usize, pal: &[Rgb]) -> Option<Held> {
        if self.roll == 0 {
            return Some(Held::Animal(item));
        }
        let art = self.clip.get(self.roll - 1)?.items.get(item)?.art()?;
        Some(Held::Clip(line_art_stamp(&art, pal), item))
    }

    /// Faixa de opções: rolos da página e setas.
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        // tampa esquerda (14 px de 10cf) e direita (12 px de 10ab)
        let left = a.image(&pic(0x10cf));
        let left_mask = a.image(&pic(0x10d1));
        blit_part(f, &left, &left_mask, 9, STRIP_Y, (0, 14));
        let right = a.image(&pic(0x10ab));
        let right_mask = a.image(&pic(0x10ad));
        let end = SLOT_X0 + SLOT_W * ROLLS_PER_PAGE as i32;
        blit_part(f, &right, &right_mask, end - 52, STRIP_Y, (52, 64));
        for k in 0..ROLLS_PER_PAGE {
            let roll = self.page * ROLLS_PER_PAGE + k;
            if roll >= self.roll_count() {
                break;
            }
            let r = slot_rect(k);
            let (frame, mask) = if roll == 0 {
                (0x10d3, 0x10d5)
            } else {
                (0x10a7, 0x10a9)
            };
            f.blit_masked(&a.image(&pic(frame)), &a.image(&pic(mask)), r.0, r.1);
            let icon = if roll == 0 {
                Some(a.image(&pic(THUMBS[0])))
            } else {
                self.clip.get(roll - 1).and_then(|c| c.icon.clone())
            };
            if let Some(icon) = icon {
                let (w, h) = (icon.width as i32, icon.height as i32);
                f.blit(&icon, r.0 + 26 - w / 2, r.1 + 21 - h / 2);
            }
        }
        f.blit_masked(
            &a.image(&pic(0x03b3)),
            &a.image(&pic(0x03b5)),
            ARROW_UP.0,
            ARROW_UP.1,
        );
        f.blit_masked(
            &a.image(&pic(0x03b7)),
            &a.image(&pic(0x03b9)),
            ARROW_DOWN.0,
            ARROW_DOWN.1,
        );
        // adesivos animados e "faça o seu" (ainda sem ação)
        f.blit(&a.image(&pic(0x025d)), 409, 63);
        f.blit(&a.image(&pic(0x123f)), 456, 59);
        f.rect(504, 75, 16, 12, BLACK);
        f.rect(505, 76, 14, 10, WHITE);
        f.text(508, 77, &(self.page + 1).to_string(), BLACK);
    }

    /// Animação, adesivo na mão (seguindo o ponteiro) e tira de filme.
    pub fn draw_overlay(&self, f: &mut Frame, a: &Assets, pal: &[Rgb], mouse: (i32, i32)) {
        let clip = canvas_rect();
        if let Some(p) = &self.playing
            && let Some(images) = self.frame_images(p)
        {
            for fr in images {
                match fr {
                    Frame3::Pic(id, x, y, centered) => {
                        let img = a.image(&pic(id));
                        let (x, y) = if centered {
                            (x - img.width as i32 / 2, y - img.height as i32)
                        } else {
                            (x, y)
                        };
                        blit_clipped(f, &img, &a.image(&pic(id + 6)), x, y, clip);
                    }
                    Frame3::Lib(img, mask, x, y) => {
                        blit_clipped(f, &img, &mask, x, y, clip);
                    }
                }
            }
        }
        if inside(clip, mouse.0, mouse.1) {
            match &self.held {
                Some(Held::Animal(animal)) if self.ani.get(*animal).is_some() => {
                    let an = &self.ani[*animal];
                    let (x, y) = (mouse.0 - an.size0.0 / 2, mouse.1 - an.size0.1 / 2);
                    blit_clipped(f, &an.cells[0].0, &an.cells[0].1, x, y, clip);
                }
                Some(Held::Animal(animal)) => {
                    let (id, dx, dy) = rest_of(*animal).unwrap_or_else(|| {
                        let id = ANIMALS[*animal][0];
                        let img = a.image(&pic(id));
                        (id, -(img.width as i32) / 2, -(img.height as i32) / 2)
                    });
                    let (x, y) = (mouse.0 + dx, mouse.1 + dy);
                    blit_clipped(f, &a.image(&pic(id)), &a.image(&pic(id + 6)), x, y, clip);
                }
                Some(Held::Clip(stamp, _)) => {
                    let x0 = mouse.0 - stamp.w as i32 / 2;
                    let y0 = mouse.1 - stamp.h as i32 / 2;
                    for (k, v) in stamp.px.iter().enumerate() {
                        let Some(v) = v else { continue };
                        let (x, y) = (
                            x0 + (k as u32 % stamp.w) as i32,
                            y0 + (k as u32 / stamp.w) as i32,
                        );
                        if inside(clip, x, y) {
                            let c =
                                pal.get(*v as usize)
                                    .copied()
                                    .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                            f.put(x, y, [c.r, c.g, c.b]);
                        }
                    }
                }
                None => {}
            }
        }
        if self.film_open {
            self.draw_film(f, a);
        }
    }

    fn draw_film(&self, f: &mut Frame, a: &Assets) {
        let film = self.film();
        let (fx, fy, fw, fh) = film.rect();
        f.rect(fx, fy, fw, fh, BLACK);
        if film.green {
            f.rect(fx + 1, fy + 1, fw - 2, fh - 2, GREEN);
            // furos do filme: 2×3 brancos com contorno preto, a cada 7 pixels
            for y in (fy + 2..fy + fh - 4).step_by(7) {
                for hx in [fx + 2, fx + 36] {
                    f.rect(hx, y - 1, 4, 5, BLACK);
                    f.rect(hx + 1, y, 2, 3, WHITE);
                }
            }
        } else {
            // amarelo pontilhado (xadrez com branco), como no original
            for y in fy + 1..fy + fh - 1 {
                for x in fx + 1..fx + fw - 1 {
                    f.put(x, y, if (x + y) % 2 == 0 { YELLOW } else { WHITE });
                }
            }
        }
        let mut cell = |r: Rect, img: Option<std::rc::Rc<Image>>| {
            f.rect(r.0, r.1, r.2, r.3, BLACK);
            f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, WHITE);
            if let Some(t) = img {
                let (tw, th) = (t.width as i32, t.height as i32);
                let inner = (r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2);
                let (x, y) = (r.0 + r.2 / 2 - tw / 2, r.1 + r.3 / 2 - th / 2);
                blit_clipped(f, &t, &t, x, y, inner);
            }
        };
        for j in 0..film.cells() {
            let item = self.scroll + j;
            if item >= self.item_count(self.roll) {
                break;
            }
            let img = if self.roll == 0 {
                Some(a.image(&pic(THUMBS[item])))
            } else {
                self.clip[self.roll - 1].items[item].preview.clone()
            };
            cell(film.cell(j), img);
        }
        let (u, d) = (film.up(), film.down());
        f.blit_masked(&a.image(&pic(0x02db)), &a.image(&pic(0x02dd)), u.0, u.1);
        f.blit_masked(&a.image(&pic(0x02df)), &a.image(&pic(0x02e1)), d.0, d.1);
    }
}

/// Um quadro da animação: imagem de `ARTIST.PIC` (id, x, y, centrado na base)
/// ou imagem de biblioteca com máscara.
enum Frame3 {
    Pic(u16, i32, i32, bool),
    Lib(Rc<Image>, Rc<Image>, i32, i32),
}

/// Repouso gravado do bicho (id, deslocamento a partir do clique).
fn rest_of(animal: usize) -> Option<(u16, i32, i32)> {
    SCRIPTS[animal]
        .as_ref()
        .and_then(|s| s.rest)
        .map(|(id, dx, dy)| (id, dx as i32, dy as i32))
}

/// Imagens do quadro `frame` do roteiro, com o clique em `(sx, sy)` na tela.
fn script_images(s: &Script, frame: usize, sx: i32, sy: i32) -> Option<Vec<(u16, i32, i32)>> {
    let cycle: usize = s.steps.iter().map(|st| st.frames as usize).sum();
    if cycle == 0 {
        return None;
    }
    let (round, mut f) = match s.repeat_shift {
        Some(_) => (frame / cycle, frame % cycle),
        None if frame >= cycle => return None,
        None => (0, frame),
    };
    if round > 64 {
        return None;
    }
    let (shx, shy) = s
        .repeat_shift
        .map(|(x, y)| (x as i32 * round as i32, y as i32 * round as i32))
        .unwrap_or((0, 0));
    if s.repeat_shift.is_some() {
        // a volta inteira saiu do quadro: acabou
        let gone = s
            .steps
            .iter()
            .flat_map(|st| st.images)
            .all(|&(_, dx, _)| sx + shx + dx as i32 > ORIGIN.0 + crate::paint::CANVAS_W as i32);
        if gone || round > 64 {
            return None;
        }
    }
    for st in s.steps {
        if f < st.frames as usize {
            return Some(
                st.images
                    .iter()
                    .map(|&(id, dx, dy)| (id, sx + shx + dx as i32, sy + shy + dy as i32))
                    .collect(),
            );
        }
        f -= st.frames as usize;
    }
    None
}

/// Copia as colunas `cols` de `img` (onde a máscara é escura) para `(x, y)`.
pub(crate) fn blit_part(
    f: &mut Frame,
    img: &Image,
    mask: &Image,
    x: i32,
    y: i32,
    cols: (u32, u32),
) {
    if (mask.width, mask.height) != (img.width, img.height) {
        return;
    }
    for dy in 0..img.height {
        for dx in cols.0..cols.1.min(img.width) {
            let i = ((dy * img.width + dx) * 4) as usize;
            let m = &mask.rgba[i..i + 3];
            if (m[0] as u32 + m[1] as u32 + m[2] as u32) < 384 {
                f.put(
                    x + dx as i32,
                    y + dy as i32,
                    [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]],
                );
            }
        }
    }
}

/// `blit_masked` recortado a um retângulo da tela.
pub(crate) fn blit_clipped(f: &mut Frame, img: &Image, mask: &Image, x: i32, y: i32, clip: Rect) {
    let masked = (mask.width, mask.height) == (img.width, img.height);
    for dy in 0..img.height as i32 {
        for dx in 0..img.width as i32 {
            let (sx, sy) = (x + dx, y + dy);
            if !inside(clip, sx, sy) {
                continue;
            }
            let i = ((dy as u32 * img.width + dx as u32) * 4) as usize;
            let opaque = if masked {
                let m = &mask.rgba[i..i + 3];
                (m[0] as u32 + m[1] as u32 + m[2] as u32) < 384
            } else {
                img.rgba[i + 3] != 0
            };
            if opaque {
                f.put(sx, sy, [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_cover_all_animals_and_end() {
        for (k, s) in SCRIPTS.iter().enumerate() {
            let Some(s) = s else { continue };
            assert!(!s.steps.is_empty(), "bicho {k}");
            // termina em tempo finito, em qualquer ponto de clique
            assert!(script_images(s, 15 * 60, 300, 300).is_none(), "bicho {k}");
            assert!(script_images(s, 0, 300, 300).is_some(), "bicho {k}");
        }
    }

    #[test]
    fn snake_walks_right_and_leaves() {
        let s = SCRIPTS[0].as_ref().unwrap();
        let a = script_images(s, 0, 100, 300).unwrap();
        let b = script_images(s, 18, 100, 300).unwrap();
        assert_eq!(b[0].1 - a[0].1, 127);
        assert!(s.rest.is_none());
    }

    #[test]
    fn line_art_stamp_masks_exterior() {
        let px: Vec<u8> = ["#.#", "#.#", "###"]
            .iter()
            .flat_map(|r| r.chars())
            .flat_map(|c| {
                if c == '#' {
                    [0, 0, 0, 255]
                } else {
                    [255, 255, 255, 255]
                }
            })
            .collect();
        let img = Image {
            width: 3,
            height: 3,
            rgba: px,
        };
        let pal = vec![
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            Rgb { r: 0, g: 0, b: 0 },
        ];
        let s = line_art_stamp(&img, &pal);
        // o "U" é aberto em cima: os brancos ligados à borda ficam transparentes
        assert_eq!(s.px[1], None);
        assert_eq!(s.px[4], None);
        assert_eq!(s.px[0], Some(1));
    }
}
