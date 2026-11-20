//! Desenho dos efeitos especiais animados na folha (a lógica e as medidas
//! estão em [`crate::writer::effects`]; `office-complemento.md` §2).
//!
//! Cada trecho seguido com o mesmo efeito numa linha forma um retângulo: x
//! do primeiro ao último caractere (sem os espaços das pontas), y = a caixa
//! da linha. Todos andam juntos pelo relógio do Escritório
//! ([`Editor::effects_ms`](super::editor::Editor)); só a vista da página
//! anima (o Visor e a impressão mostram o estado normal).

use crate::Frame;
use crate::writer::doc::{Document, Effect, Item};
use crate::writer::effects::{
    ants_on, cycle, phase, shake_dx, spark_seed, sparks, vapt_rows, vupt_cols,
};
use crate::writer::layout::{Layout, Line};

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];

/// Um trecho com efeito: `x0..x1` na folha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub effect: Effect,
    pub x0: i32,
    pub x1: i32,
}

/// Os trechos com efeito de uma linha.
pub fn runs(doc: &Document, line: &Line) -> Vec<Run> {
    let Some(par) = doc.paras.get(line.para) else {
        return Vec::new();
    };
    let mut out: Vec<Run> = Vec::new();
    // trecho aberto: efeito, x do começo, fim da última letra (sem espaço)
    let mut open: Option<(Effect, i32, Option<i32>)> = None;
    let close = |open: &mut Option<(Effect, i32, Option<i32>)>, out: &mut Vec<Run>| {
        if let Some((effect, x0, Some(x1))) = open.take() {
            out.push(Run { effect, x0, x1 });
        }
    };
    for b in &line.boxes {
        let eff = match par.items.get(b.index) {
            Some(Item::Char(_, p)) => p.effect,
            _ => None,
        };
        match (eff, &mut open) {
            (Some(e), Some((oe, _, end))) if *oe == e => {
                if !b.space {
                    *end = Some(b.x + b.w);
                }
            }
            (Some(e), _) => {
                close(&mut open, &mut out);
                if !b.space {
                    open = Some((e, b.x, Some(b.x + b.w)));
                }
            }
            (None, _) => close(&mut open, &mut out),
        }
    }
    close(&mut open, &mut out);
    out
}

/// Desenha os efeitos da folha `page` (canto em `(ox, oy)` do quadro `v`,
/// que fica em `screen` na tela) no instante `ms` do relógio.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    doc: &Document,
    l: &Layout,
    v: &mut Frame,
    page: usize,
    ox: i32,
    oy: i32,
    screen: (i32, i32),
    ms: u64,
) {
    let step = phase(ms);
    if step == 0 {
        return;
    }
    let mut n = 0u64;
    for line in l.lines_on(page) {
        let (y, h) = (oy + line.top, line.height);
        if y > v.height() as i32 || y + h < 0 {
            continue;
        }
        for r in runs(doc, line) {
            n += 1;
            let (x, w) = (ox + r.x0, r.x1 - r.x0);
            match r.effect {
                Effect::Blink => invert(v, x, y, w, h),
                Effect::Ants => ants(v, x, y, w, h, screen, step),
                Effect::Sparkle => {
                    let seed = spark_seed(cycle(ms), step, n);
                    for s in sparks(seed, w, h) {
                        let rgb = [s.color[0], s.color[1], s.color[2]];
                        for (dx, dy) in s.shape.pixels() {
                            v.put(x + s.x + dx, y + s.y + dy, rgb);
                        }
                    }
                }
                Effect::Shake => shake(v, x, y, w, h, screen.1, step),
                Effect::Vapt => v.rect(x, y, w, vapt_rows(step, h), WHITE),
                Effect::Vupt => v.rect(x, y, vupt_cols(step, w), h, WHITE),
                Effect::Other(_) => {}
            }
        }
    }
}

fn invert(v: &mut Frame, x: i32, y: i32, w: i32, h: i32) {
    for yy in y.max(0)..(y + h).min(v.height() as i32) {
        for xx in x.max(0)..(x + w).min(v.width() as i32) {
            let p = v.pixel(xx as u32, yy as u32);
            v.put(xx, yy, [255 - p[0], 255 - p[1], 255 - p[2]]);
        }
    }
}

/// Moldura tracejada no contorno do retângulo (coordenadas da tela para o
/// tracejado andar igual em todos os trechos).
fn ants(v: &mut Frame, x: i32, y: i32, w: i32, h: i32, (sx, sy): (i32, i32), step: u8) {
    if w <= 0 || h <= 0 {
        return;
    }
    let mut dot = |px: i32, py: i32| {
        if ants_on(px + sx, py + sy, step) {
            v.put(px, py, BLACK);
        }
    };
    for px in x..x + w {
        dot(px, y);
        dot(px, y + h - 1);
    }
    for py in y + 1..y + h - 1 {
        dot(x, py);
        dot(x + w - 1, py);
    }
}

/// Treme-treme: cada linha de pixels desliza ±1 (o lugar que sobra fica
/// branco).
fn shake(v: &mut Frame, x: i32, y: i32, w: i32, h: i32, sy: i32, step: u8) {
    let (fw, fh) = (v.width() as i32, v.height() as i32);
    for py in y.max(0)..(y + h).min(fh) {
        let dx = shake_dx(py + sy, step);
        if dx == 0 {
            continue;
        }
        let row: Vec<[u8; 3]> = (x..x + w)
            .map(|px| {
                if (0..fw).contains(&px) {
                    let p = v.pixel(px as u32, py as u32);
                    [p[0], p[1], p[2]]
                } else {
                    WHITE
                }
            })
            .collect();
        v.rect(x, py, w, 1, WHITE);
        for (k, p) in row.iter().enumerate() {
            v.put(x + k as i32 + dx, py, *p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::{CharChange, CharProps, Pos, Range};
    use crate::writer::layout::SimpleMeasure as FixedMeasure;

    #[test]
    fn runs_join_words_with_the_same_effect_and_drop_edge_spaces() {
        let mut d = Document::from_text("ab cd ef gh", CharProps::default());
        let blink = CharChange::Effect(Some(Effect::Blink));
        d.apply_char(Range::new(Pos::new(0, 0), Pos::new(0, 6)), blink);
        d.apply_char(
            Range::new(Pos::new(0, 6), Pos::new(0, 8)),
            CharChange::Effect(Some(Effect::Vapt)),
        );
        let l = Layout::build(&d, &FixedMeasure);
        let line = &l.lines[0];
        let r = runs(&d, line);
        assert_eq!(r.len(), 2, "{r:?}");
        let bx = |k: usize| line.boxes.iter().find(|b| b.index == k).unwrap();
        assert_eq!(r[0].effect, Effect::Blink);
        // "ab cd" sem o espaço depois de "cd"
        assert_eq!((r[0].x0, r[0].x1), (bx(0).x, bx(4).x + bx(4).w));
        assert_eq!(r[1].effect, Effect::Vapt);
        assert_eq!((r[1].x0, r[1].x1), (bx(6).x, bx(7).x + bx(7).w));
    }

    #[test]
    fn nothing_is_drawn_while_resting_and_blink_inverts() {
        let mut d = Document::from_text("ab", CharProps::default());
        d.apply_char(d.all(), CharChange::Effect(Some(Effect::Blink)));
        let l = Layout::build(&d, &FixedMeasure);
        let mut v = Frame::new(600, 300);
        v.fill(WHITE);
        draw(&d, &l, &mut v, 0, 0, 0, (0, 0), 500);
        assert!(v.pixels().chunks(4).all(|p| p[0] == 255));
        draw(&d, &l, &mut v, 0, 0, 0, (0, 0), 1200);
        let line = &l.lines[0];
        let b = line.boxes[0];
        assert_eq!(v.pixel(b.x as u32, line.top as u32)[0], 0);
    }
}
