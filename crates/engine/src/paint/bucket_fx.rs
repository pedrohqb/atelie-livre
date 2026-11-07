//! Efeitos das dez opções da lata de tinta e os papéis de parede (ver
//! `docs/telas/fine-artist.md`, "Lata de tinta"). Tudo aqui trabalha sobre o
//! [`Canvas`] (índices da paleta de 256 cores) ou sobre um [`Object`]; a
//! tela de pintura só escolhe o efeito e guarda o desfazer.

use super::canvas::{COLORS, Canvas, Ink, ORIGIN, WHITE};
use super::objects::Object;
use super::stamp::Stamp;
use crate::rng::XorShift;
use formats::palette::{Rgb, nearest};
use std::rc::Rc;

/// Lado do ladrilho dos papéis de parede, em pixels.
pub const TILE: usize = 32;

/// Papel de parede de `W-MAG100.CHK`: um ou mais quadros 32×32 (índices da
/// paleta de 256 cores). Com mais de um quadro, a área preenchida se mexe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wallpaper {
    pub name: String,
    pub frames: Vec<Vec<u8>>,
}

impl Wallpaper {
    /// Índice do quadro `frame` no pixel `(x, y)` do quadro de pintura: o
    /// ladrilho começa no canto do quadro (medido no original).
    pub fn index_at(&self, frame: usize, x: i32, y: i32) -> u8 {
        let Some(f) = self.frames.get(frame % self.frames.len().max(1)) else {
            return WHITE;
        };
        let (tx, ty) = (
            x.rem_euclid(TILE as i32) as usize,
            y.rem_euclid(TILE as i32) as usize,
        );
        f.get(ty * TILE + tx).copied().unwrap_or(WHITE)
    }

    pub fn animated(&self) -> bool {
        self.frames.len() > 1
    }
}

/// O que a lata despeja: tinta (cor e padrão) ou um papel de parede.
#[derive(Debug, Clone, Copy)]
pub enum Pour<'a> {
    Ink(Ink),
    Wallpaper(&'a Wallpaper),
}

impl Pour<'_> {
    /// Índice no pixel `(x, y)` do quadro (os padrões seguem a grade da tela).
    pub fn index_at(&self, x: i32, y: i32) -> u8 {
        match self {
            Pour::Ink(ink) => ink.index_at(x + ORIGIN.0, y + ORIGIN.1),
            Pour::Wallpaper(w) => w.index_at(0, x, y),
        }
    }
}

/// Área do quadro: um `bool` por pixel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub w: u32,
    pub h: u32,
    pub mask: Vec<bool>,
}

impl Region {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && (x as u32) < self.w
            && (y as u32) < self.h
            && self.mask[(y as u32 * self.w + x as u32) as usize]
    }

    pub fn len(&self) -> usize {
        self.mask.iter().filter(|&&m| m).count()
    }

    pub fn is_empty(&self) -> bool {
        !self.mask.iter().any(|&m| m)
    }

    fn points(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        let w = self.w;
        self.mask
            .iter()
            .enumerate()
            .filter(|(_, m)| **m)
            .map(move |(i, _)| ((i as u32 % w) as i32, (i as u32 / w) as i32))
    }
}

/// Área de vizinhança 4 com o mesmo índice de `seen` (o que se vê, do tamanho
/// do quadro) que `(x, y)`, sem sair do recorte do quadro.
pub fn region(c: &Canvas, seen: &[u8], x: i32, y: i32) -> Option<Region> {
    if seen.len() != c.px.len() || !c.can_paint(x, y) {
        return None;
    }
    let w = c.w as i32;
    let at = |x: i32, y: i32| (y * w + x) as usize;
    let target = seen[at(x, y)];
    let mut mask = vec![false; c.px.len()];
    mask[at(x, y)] = true;
    let mut stack = vec![(x, y)];
    while let Some((px, py)) = stack.pop() {
        for (nx, ny) in [(px - 1, py), (px + 1, py), (px, py - 1), (px, py + 1)] {
            if c.can_paint(nx, ny) && !mask[at(nx, ny)] && seen[at(nx, ny)] == target {
                mask[at(nx, ny)] = true;
                stack.push((nx, ny));
            }
        }
    }
    Some(Region {
        w: c.w,
        h: c.h,
        mask,
    })
}

/// Pinta a área com o que a lata despeja. Devolve se algo mudou.
pub fn fill(c: &mut Canvas, r: &Region, pour: Pour) -> bool {
    let mut changed = false;
    for (x, y) in r.points() {
        let v = pour.index_at(x, y);
        if c.get(x, y) != Some(v) {
            c.set(x, y, v);
            changed = true;
        }
    }
    changed
}

/// Troca-cores: todos os pixels do quadro com o índice `target` recebem o
/// que a lata despeja.
pub fn swap_color(c: &mut Canvas, target: u8, pour: Pour) -> bool {
    let mut changed = false;
    for y in 0..c.h as i32 {
        for x in 0..c.w as i32 {
            if c.get(x, y) == Some(target) && c.can_paint(x, y) {
                let v = pour.index_at(x, y);
                changed |= v != target;
                c.set(x, y, v);
            }
        }
    }
    changed
}

/// Padrão (de `patterns`) e cor que melhor explicam a janela 8×8 da grade da
/// tela em volta de `(x, y)`. Só valem padrões de duas cores: a cor e branco.
fn pattern_at(c: &Canvas, x: i32, y: i32, patterns: &[[u8; 8]]) -> Option<(usize, u8)> {
    let (sx, sy) = (x + ORIGIN.0, y + ORIGIN.1);
    let (gx, gy) = (sx - sx.rem_euclid(8), sy - sy.rem_euclid(8));
    let window: Vec<(i32, i32, u8)> = (0..8)
        .flat_map(|dy| (0..8).map(move |dx| (gx + dx, gy + dy)))
        .filter_map(|(sx, sy)| c.get(sx - ORIGIN.0, sy - ORIGIN.1).map(|v| (sx, sy, v)))
        .collect();
    let clicked = c.get(x, y)?;
    let color = if clicked != WHITE {
        clicked
    } else {
        window.iter().map(|w| w.2).find(|&v| v != WHITE)?
    };
    let mut best: Option<(usize, usize)> = None;
    for (k, &pattern) in patterns.iter().enumerate() {
        let ink = Ink { color, pattern };
        let hits = window
            .iter()
            .filter(|&&(sx, sy, v)| ink.index_at(sx, sy) == v)
            .count();
        if best.is_none_or(|(_, b)| hits > b) {
            best = Some((k, hits));
        }
    }
    best.filter(|&(_, hits)| hits * 4 >= window.len() * 3)
        .map(|(k, _)| (k, color))
}

/// Troca-padrões: a área ligada a `(x, y)` desenhada com um dos `patterns`
/// (cor e branco, na grade da tela) passa a ser desenhada com o novo padrão,
/// na mesma cor. Um papel de parede substitui o padrão inteiro.
pub fn swap_pattern(c: &mut Canvas, x: i32, y: i32, patterns: &[[u8; 8]], pour: Pour) -> bool {
    let Some((k, color)) = pattern_at(c, x, y, patterns) else {
        return false;
    };
    let old = Ink {
        color,
        pattern: patterns[k],
    };
    let fits = |c: &Canvas, x: i32, y: i32| {
        c.can_paint(x, y) && c.get(x, y) == Some(old.index_at(x + ORIGIN.0, y + ORIGIN.1))
    };
    // uma célula 8×8 da grade conta se tem pelo menos um pixel da cor: assim
    // a área não vaza pelo branco do fundo
    let cell_has_color = |c: &Canvas, x: i32, y: i32| {
        let (sx, sy) = (x + ORIGIN.0, y + ORIGIN.1);
        let (gx, gy) = (sx - sx.rem_euclid(8), sy - sy.rem_euclid(8));
        (0..8)
            .any(|dy| (0..8).any(|dx| c.get(gx + dx - ORIGIN.0, gy + dy - ORIGIN.1) == Some(color)))
    };
    if !fits(c, x, y) {
        return false;
    }
    let w = c.w as i32;
    let mut mask = vec![false; c.px.len()];
    mask[(y * w + x) as usize] = true;
    let mut stack = vec![(x, y)];
    while let Some((px, py)) = stack.pop() {
        for (nx, ny) in [(px - 1, py), (px + 1, py), (px, py - 1), (px, py + 1)] {
            if nx < 0 || ny < 0 || nx >= w || ny >= c.h as i32 {
                continue;
            }
            let i = (ny * w + nx) as usize;
            if !mask[i] && fits(c, nx, ny) && cell_has_color(c, nx, ny) {
                mask[i] = true;
                stack.push((nx, ny));
            }
        }
    }
    let r = Region {
        w: c.w,
        h: c.h,
        mask,
    };
    match pour {
        Pour::Ink(ink) => fill(
            c,
            &r,
            Pour::Ink(Ink {
                color,
                pattern: ink.pattern,
            }),
        ),
        Pour::Wallpaper(_) => fill(c, &r, pour),
    }
}

/// Contorno: os pixels de fora da área a até `width` pixels dela (distância
/// de tabuleiro de xadrez) recebem o que a lata despeja.
pub fn outline(c: &mut Canvas, r: &Region, width: i32, pour: Pour) -> bool {
    let mut ring = vec![false; c.px.len()];
    for (x, y) in r.points() {
        for dy in -width..=width {
            for dx in -width..=width {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0
                    && ny >= 0
                    && (nx as u32) < c.w
                    && (ny as u32) < c.h
                    && !r.contains(nx, ny)
                {
                    ring[(ny as u32 * c.w + nx as u32) as usize] = true;
                }
            }
        }
    }
    fill(
        c,
        &Region {
            w: c.w,
            h: c.h,
            mask: ring,
        },
        pour,
    )
}

/// Matriz de Bayer 4×4, para o pontilhado ordenado da mistura.
const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Mistura-cores: do centro `(cx, cy)` para fora, a área vai da cor `from`
/// (a da tinta) até a cor `to` (a que estava lá), num pontilhado ordenado.
pub fn blend(c: &mut Canvas, r: &Region, (cx, cy): (i32, i32), from: u8, to: u8) -> bool {
    let dist = |x: i32, y: i32| (((x - cx).pow(2) + (y - cy).pow(2)) as f32).sqrt();
    let far = r.points().map(|(x, y)| dist(x, y)).fold(0.0f32, f32::max);
    if far <= 0.0 {
        return false;
    }
    let mut changed = false;
    for (x, y) in r.points() {
        let t = dist(x, y) / far;
        let level = BAYER[y.rem_euclid(4) as usize][x.rem_euclid(4) as usize] as f32 + 0.5;
        let v = if t * 16.0 > level { to } else { from };
        changed |= c.get(x, y) != Some(v);
        c.set(x, y, v);
    }
    changed
}

/// Pontinhos: cada pixel da área copia um vizinho sorteado a até `radius`
/// pixels; perto das bordas a área fica salpicada com as cores ao redor.
pub fn pointillize(c: &mut Canvas, r: &Region, rng: &mut XorShift, radius: i32) -> bool {
    let src = c.px.clone();
    let (w, h) = (c.w as i32, c.h as i32);
    let mut changed = false;
    for (x, y) in r.points() {
        let (nx, ny) = (
            (x + rng.offset(radius)).clamp(0, w - 1),
            (y + rng.offset(radius)).clamp(0, h - 1),
        );
        let v = src[(ny * w + nx) as usize];
        if src[(y * w + x) as usize] != v {
            c.set(x, y, v);
            changed = true;
        }
    }
    changed
}

/// Preenchimento pop: o quadro encolhe pela metade e se repete 2×2.
pub fn pop(c: &mut Canvas) -> bool {
    let src = c.px.clone();
    let (w, h) = (c.w as i32, c.h as i32);
    let (hw, hh) = ((w + 1) / 2, (h + 1) / 2);
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = (((x % hw) * 2).min(w - 1), ((y % hh) * 2).min(h - 1));
            c.set(x, y, src[(sy * w + sx) as usize]);
        }
    }
    c.px != src
}

/// Bolhas: `count` círculos sorteados perto de `(cx, cy)`, com o contorno no
/// que a lata despeja e um brilho branco no alto à esquerda.
pub fn bubbles(
    c: &mut Canvas,
    (cx, cy): (i32, i32),
    rng: &mut XorShift,
    count: usize,
    pour: Pour,
) -> bool {
    let before = c.px.clone();
    for _ in 0..count {
        let r = 4 + rng.below(10) as i32;
        let (bx, by) = (cx + rng.offset(48), cy + rng.offset(48));
        for dy in -r..=r {
            for dx in -r..=r {
                let d2 = dx * dx + dy * dy;
                let (x, y) = (bx + dx, by + dy);
                if d2 <= r * r && d2 > (r - 1) * (r - 1) {
                    c.set(x, y, pour.index_at(x, y));
                } else if r > 5 {
                    // brilho: um arco curto dentro, no alto à esquerda
                    let hi = r - 3;
                    if dx < 0 && dy < 0 && d2 <= hi * hi && d2 > (hi - 1) * (hi - 1) && dx * 3 < dy
                    {
                        c.set(x, y, WHITE);
                    }
                }
            }
        }
    }
    c.px != before
}

/// Passo do troca-cores em seqüência na roda das 38 cores da tira:
/// posição `p` vai para `SPIN_MUL · p + SPIN_ADD` (mod 38). Com estes números
/// o branco vira vinho e o preto vira verde, como no original.
const SPIN_MUL: usize = 25;
const SPIN_ADD: usize = 11;

/// Próxima cor do troca-cores em seqüência para o índice `v`: índices fora da
/// tira usam a cor da tira de mesmo RGB (ou a mais próxima).
pub fn spin_index(v: u8, pal: &[Rgb]) -> u8 {
    let n = COLORS.len();
    let pos = COLORS.iter().position(|&c| c == v).or_else(|| {
        let p = pal.get(v as usize)?;
        let strip: Vec<Rgb> = COLORS
            .iter()
            .map(|&c| {
                pal.get(c as usize)
                    .copied()
                    .unwrap_or(Rgb { r: 0, g: 0, b: 0 })
            })
            .collect();
        Some(nearest(&strip, p.r, p.g, p.b) as usize)
    });
    match pos {
        Some(p) => COLORS[(SPIN_MUL * p + SPIN_ADD) % n],
        None => v,
    }
}

/// Troca-cores em seqüência: todas as cores do quadro andam um passo na roda.
pub fn spin(c: &mut Canvas, pal: &[Rgb]) -> bool {
    let mut map = [0u8; 256];
    for (v, m) in map.iter_mut().enumerate() {
        *m = spin_index(v as u8, pal);
    }
    let before = c.px.clone();
    for y in 0..c.h as i32 {
        for x in 0..c.w as i32 {
            if let Some(v) = c.get(x, y) {
                c.set(x, y, map[v as usize]);
            }
        }
    }
    c.px != before
}

/// Área preenchida com um papel de parede que se mexe: os pixels ainda com o
/// quadro esperado passam ao quadro seguinte; os que foram pintados por cima
/// saem da área.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moving {
    pub wallpaper: usize,
    pub frame: usize,
    pub pixels: Vec<u32>,
}

impl Moving {
    pub fn new(wallpaper: usize, r: &Region) -> Moving {
        Moving {
            wallpaper,
            frame: 0,
            pixels: r
                .mask
                .iter()
                .enumerate()
                .filter(|(_, m)| **m)
                .map(|(i, _)| i as u32)
                .collect(),
        }
    }

    /// Avança um quadro. Devolve `false` quando a área acabou.
    pub fn step(&mut self, c: &mut Canvas, wp: &Wallpaper) -> bool {
        let n = wp.frames.len().max(1);
        let next = (self.frame + 1) % n;
        let w = c.w;
        self.pixels.retain(|&i| {
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            if c.get(x, y) != Some(wp.index_at(self.frame, x, y)) {
                return false;
            }
            c.set(x, y, wp.index_at(next, x, y));
            true
        });
        self.frame = next;
        !self.pixels.is_empty()
    }
}

/// Pixels do objeto como se vêem (escala, giros e inversões aplicados).
fn baked(o: &Object) -> (i32, i32, Vec<Option<u8>>) {
    let (w, h) = (o.w.max(1), o.h.max(1));
    let px = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| o.pixel(x, y))
        .collect();
    (w, h, px)
}

fn set_baked(o: &mut Object, w: i32, h: i32, px: Vec<Option<u8>>) {
    o.image = Rc::new(Stamp {
        w: w as u32,
        h: h as u32,
        px,
    });
    o.w = w;
    o.h = h;
    o.flip_h = false;
    o.flip_v = false;
    o.turns = 0;
}

/// Lata dentro de um adesivo: a área da mesma cor ligada a `(ox, oy)`
/// (relativo ao canto) recebe o que a lata despeja.
pub fn object_fill(o: &mut Object, ox: i32, oy: i32, pour: Pour) -> bool {
    let Some(target) = o.pixel(ox, oy) else {
        return false;
    };
    let (w, h, mut px) = baked(o);
    let mut seen = vec![false; px.len()];
    seen[(oy * w + ox) as usize] = true;
    let mut stack = vec![(ox, oy)];
    let mut changed = false;
    while let Some((x, y)) = stack.pop() {
        let i = (y * w + x) as usize;
        let v = Some(pour.index_at(o.x + x, o.y + y));
        changed |= px[i] != v;
        px[i] = v;
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let j = (ny * w + nx) as usize;
            if !seen[j] && o.pixel(nx, ny) == Some(target) {
                seen[j] = true;
                stack.push((nx, ny));
            }
        }
    }
    if changed {
        set_baked(o, w, h, px);
    }
    changed
}

/// Troca-cores dentro de um adesivo: todos os pixels da cor clicada.
pub fn object_swap_color(o: &mut Object, ox: i32, oy: i32, pour: Pour) -> bool {
    let Some(target) = o.pixel(ox, oy) else {
        return false;
    };
    let (w, h, mut px) = baked(o);
    let mut changed = false;
    for (i, p) in px.iter_mut().enumerate() {
        if *p == Some(target) {
            let (x, y) = (i as i32 % w, i as i32 / w);
            let v = Some(pour.index_at(o.x + x, o.y + y));
            changed |= *p != v;
            *p = v;
        }
    }
    if changed {
        set_baked(o, w, h, px);
    }
    changed
}

/// Contorno de um adesivo: a imagem cresce `width` pixels de cada lado e os
/// pixels transparentes a até `width` pixels da figura recebem a tinta.
pub fn object_outline(o: &mut Object, width: i32, pour: Pour) -> bool {
    let (w, h, px) = baked(o);
    let (nw, nh) = (w + 2 * width, h + 2 * width);
    let (nx0, ny0) = (o.x - width, o.y - width);
    let opaque =
        |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && px[(y * w + x) as usize].is_some();
    let mut out = vec![None; (nw * nh) as usize];
    let mut any = false;
    for y in 0..nh {
        for x in 0..nw {
            let (sx, sy) = (x - width, y - width);
            if opaque(sx, sy) {
                out[(y * nw + x) as usize] = px[(sy * w + sx) as usize];
                continue;
            }
            let near =
                (-width..=width).any(|dy| (-width..=width).any(|dx| opaque(sx + dx, sy + dy)));
            if near {
                out[(y * nw + x) as usize] = Some(pour.index_at(nx0 + x, ny0 + y));
                any = true;
            }
        }
    }
    if any {
        o.x = nx0;
        o.y = ny0;
        set_baked(o, nw, nh, out);
    }
    any
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::canvas::BLACK;
    use crate::paint::objects::Source;

    fn canvas(w: u32, h: u32) -> Canvas {
        Canvas::new(w, h)
    }

    fn count(c: &Canvas, v: u8) -> usize {
        c.px.iter().filter(|&&p| p == v).count()
    }

    /// Quadro 40×40 branco com uma moldura preta de 1 px em (10,10)-(29,29).
    fn boxed() -> Canvas {
        let mut c = canvas(40, 40);
        for k in 10..30 {
            c.set(k, 10, BLACK);
            c.set(k, 29, BLACK);
            c.set(10, k, BLACK);
            c.set(29, k, BLACK);
        }
        c
    }

    fn pal() -> Vec<Rgb> {
        // paleta de teste: cada índice com um RGB distinto
        (0..256)
            .map(|i| Rgb {
                r: i as u8,
                g: (i * 7) as u8,
                b: (i * 13) as u8,
            })
            .collect()
    }

    #[test]
    fn region_stops_at_walls() {
        let c = boxed();
        let r = region(&c, &c.px, 20, 20).unwrap();
        assert_eq!(r.len(), 18 * 18);
        assert!(r.contains(11, 11) && !r.contains(10, 10) && !r.contains(5, 5));
    }

    #[test]
    fn wallpaper_tiles_from_canvas_corner() {
        let mut f0 = vec![2u8; TILE * TILE];
        f0[3 * TILE + 5] = 9;
        let wp = Wallpaper {
            name: "t".into(),
            frames: vec![f0],
        };
        assert_eq!(wp.index_at(0, 5, 3), 9);
        assert_eq!(wp.index_at(0, 5 + 32, 3 + 64), 9);
        assert_eq!(wp.index_at(0, 6, 3), 2);
        let mut c = boxed();
        let r = region(&c, &c.px.clone(), 20, 20).unwrap();
        assert!(fill(&mut c, &r, Pour::Wallpaper(&wp)));
        // nenhum ponto de dentro da caixa cai em (5, 3) módulo 32
        assert_eq!(c.get(20, 20), Some(2));
        assert_eq!(c.get(5, 3), Some(WHITE));
        assert_eq!(c.get(10, 10), Some(BLACK));
    }

    #[test]
    fn swap_color_changes_every_pixel_of_that_color() {
        let mut c = boxed();
        c.set(2, 2, BLACK);
        assert!(swap_color(&mut c, BLACK, Pour::Ink(Ink::solid(7))));
        assert_eq!(count(&c, BLACK), 0);
        assert_eq!(count(&c, 7), 76 + 1);
    }

    #[test]
    fn swap_pattern_keeps_color_and_changes_pattern() {
        let mut c = canvas(64, 64);
        let checker = [0x55u8, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa];
        let solid = [0u8; 8];
        let old = Ink {
            color: 5,
            pattern: checker,
        };
        // retângulo xadrez em (8..40, 8..40) sobre fundo branco
        for y in 8..40 {
            for x in 8..40 {
                c.plot(x, y, &old);
            }
        }
        let patterns = vec![solid, checker];
        let new = Ink {
            color: 9,
            pattern: solid,
        };
        assert!(swap_pattern(&mut c, 20, 20, &patterns, Pour::Ink(new)));
        // o retângulo fica liso na cor antiga (5), o fundo não muda
        assert!((8..40).all(|y| (8..40).all(|x| c.get(x, y) == Some(5))));
        assert_eq!(c.get(2, 2), Some(WHITE));
        assert_eq!(c.get(50, 50), Some(WHITE));
        assert_eq!(count(&c, 9), 0);
    }

    #[test]
    fn outline_rings_the_region() {
        let mut c = canvas(30, 30);
        for y in 10..20 {
            for x in 10..20 {
                c.set(x, y, BLACK);
            }
        }
        let r = region(&c, &c.px.clone(), 15, 15).unwrap();
        assert!(outline(&mut c, &r, 2, Pour::Ink(Ink::solid(6))));
        assert_eq!(c.get(8, 8), Some(6));
        assert_eq!(c.get(21, 15), Some(6));
        assert_eq!(c.get(22, 15), Some(WHITE));
        assert_eq!(c.get(15, 15), Some(BLACK));
        assert_eq!(count(&c, 6), 14 * 14 - 100);
    }

    #[test]
    fn blend_goes_from_ink_at_center_to_old_color_outside() {
        let mut c = canvas(41, 41);
        let r = region(&c, &c.px.clone(), 20, 20).unwrap();
        assert!(blend(&mut c, &r, (20, 20), 6, WHITE));
        assert_eq!(c.get(20, 20), Some(6));
        assert_eq!(c.get(0, 0), Some(WHITE));
        let near = (18..23).flat_map(|y| (18..23).map(move |x| (x, y)));
        let inks = near.filter(|&(x, y)| c.get(x, y) == Some(6)).count();
        assert!(inks >= 20, "{inks}");
        let corner = (0..4).flat_map(|y| (0..4).map(move |x| (x, y)));
        assert!(corner.filter(|&(x, y)| c.get(x, y) == Some(6)).count() <= 2);
    }

    #[test]
    fn pointillize_speckles_only_near_edges() {
        let mut c = canvas(60, 20);
        for y in 0..20 {
            for x in 30..60 {
                c.set(x, y, BLACK);
            }
        }
        let r = region(&c, &c.px.clone(), 5, 5).unwrap();
        let mut rng = XorShift::new(7);
        assert!(pointillize(&mut c, &r, &mut rng, 3));
        // longe da borda, nada muda; perto, aparecem pontinhos pretos
        assert!((0..20).all(|y| (0..26).all(|x| c.get(x, y) == Some(WHITE))));
        let dots = (0..20)
            .flat_map(|y| (27..30).map(move |x| (x, y)))
            .filter(|&(x, y)| c.get(x, y) == Some(BLACK))
            .count();
        assert!(dots > 5 && dots < 60, "{dots}");
        // fora da área (o preto), nada muda
        assert!((0..20).all(|y| (30..60).all(|x| c.get(x, y) == Some(BLACK))));
    }

    #[test]
    fn pop_makes_two_by_two_copies() {
        let mut c = canvas(40, 20);
        c.set(10, 6, BLACK);
        assert!(pop(&mut c));
        for (x, y) in [(5, 3), (25, 3), (5, 13), (25, 13)] {
            assert_eq!(c.get(x, y), Some(BLACK), "({x},{y})");
        }
        assert_eq!(count(&c, BLACK), 4);
    }

    #[test]
    fn bubbles_draw_rings_near_click() {
        let mut c = canvas(200, 200);
        let mut rng = XorShift::new(3);
        assert!(bubbles(
            &mut c,
            (100, 100),
            &mut rng,
            5,
            Pour::Ink(Ink::solid(8))
        ));
        let inked: Vec<(i32, i32)> = (0..200)
            .flat_map(|y| (0..200).map(move |x| (x, y)))
            .filter(|&(x, y)| c.get(x, y) == Some(8))
            .collect();
        assert!(inked.len() > 40);
        assert!(
            inked
                .iter()
                .all(|&(x, y)| (x - 100).abs() <= 62 && (y - 100).abs() <= 62)
        );
    }

    #[test]
    fn spin_is_a_permutation_of_the_strip() {
        let pal = pal();
        let mut seen: Vec<u8> = COLORS.iter().map(|&v| spin_index(v, &pal)).collect();
        seen.sort();
        let mut all = COLORS.to_vec();
        all.sort();
        assert_eq!(seen, all);
        // branco vira vinho (128,0,64) e preto vira verde escuro, como no original
        assert_eq!(spin_index(WHITE, &pal), 27);
        assert_eq!(spin_index(BLACK, &pal), 12);
        let mut c = boxed();
        assert!(spin(&mut c, &pal));
        assert_eq!(c.get(0, 0), Some(27));
        assert_eq!(c.get(10, 10), Some(12));
    }

    #[test]
    fn spin_maps_duplicate_indices_by_color() {
        let mut pal = pal();
        pal[200] = pal[5];
        assert_eq!(spin_index(200, &pal), spin_index(5, &pal));
    }

    #[test]
    fn moving_wallpaper_steps_and_drops_painted_pixels() {
        let wp = Wallpaper {
            name: "m".into(),
            frames: vec![vec![3; TILE * TILE], vec![4; TILE * TILE]],
        };
        let mut c = canvas(10, 10);
        let r = region(&c, &c.px.clone(), 0, 0).unwrap();
        fill(&mut c, &r, Pour::Wallpaper(&wp));
        let mut m = Moving::new(0, &r);
        c.set(0, 0, BLACK);
        assert!(m.step(&mut c, &wp));
        assert_eq!(c.get(5, 5), Some(4));
        assert_eq!(c.get(0, 0), Some(BLACK));
        assert_eq!(m.pixels.len(), 99);
        assert!(m.step(&mut c, &wp));
        assert_eq!(c.get(5, 5), Some(3));
        c.px.fill(BLACK);
        assert!(!m.step(&mut c, &wp));
    }

    fn sticker() -> Object {
        // quadrado 4×4 opaco: borda 1, miolo 2
        let px = (0..16)
            .map(|i| {
                let (x, y) = (i % 4, i / 4);
                Some(if x == 0 || y == 0 || x == 3 || y == 3 {
                    1
                } else {
                    2
                })
            })
            .collect();
        let s = Stamp { w: 4, h: 4, px };
        Object::new(1, Rc::new(s), 10, 10, Source::Png("t.png".into()))
    }

    #[test]
    fn object_fill_and_swap() {
        let mut o = sticker();
        assert!(object_fill(&mut o, 1, 1, Pour::Ink(Ink::solid(6))));
        assert_eq!(o.pixel(2, 2), Some(6));
        assert_eq!(o.pixel(0, 0), Some(1));
        assert!(object_swap_color(&mut o, 0, 0, Pour::Ink(Ink::solid(9))));
        assert_eq!(o.pixel(3, 3), Some(9));
        assert_eq!(o.pixel(1, 1), Some(6));
    }

    #[test]
    fn object_outline_grows_the_sticker() {
        let mut o = sticker();
        assert!(object_outline(&mut o, 2, Pour::Ink(Ink::solid(6))));
        assert_eq!((o.x, o.y, o.w, o.h), (8, 8, 8, 8));
        assert_eq!(o.pixel(0, 0), Some(6));
        assert_eq!(o.pixel(2, 2), Some(1));
        assert_eq!(o.pixel(3, 3), Some(2));
    }
}
