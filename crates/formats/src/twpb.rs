//! Parte `TWPB` dos objetos do documento `.MAX` do Creative Writer
//! (`PICT` e `SPLE`): o contorno que o texto contorna ("text wrap").
//! Ver `docs/formats/twpb.md`.
//!
//! ```text
//! u16  tipo       8 (figuras e a maioria dos Splots) ou 16 (alguns Splots)
//! u16  largura    do objeto como aparece na página
//! u16  altura
//! u16  nl         pontos da borda esquerda
//! u16  nr         pontos da borda direita
//! nl × { i16 x, i16 y }   margem esquerda: pixels vazios desde a esquerda até a tinta
//! nr × { i16 x, i16 y }   margem direita: pixels vazios desde a direita até a tinta
//! ```
//!
//! `y` cresce de cima para baixo e vai até a altura (o último ponto fecha a
//! última linha); linhas vazias no alto e embaixo ficam de fora. Entre dois
//! pontos a margem é a reta que os liga: o original guarda só os vértices.

use crate::{ErrorKind, Format, Reader, Result};

/// Contorno de um objeto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Twpb {
    pub kind: u16,
    pub width: u16,
    pub height: u16,
    /// `(margem, y)` da borda esquerda.
    pub left: Vec<(i16, i16)>,
    /// `(margem, y)` da borda direita.
    pub right: Vec<(i16, i16)>,
}

/// Tipo gravado nas figuras.
pub const KIND_PICTURE: u16 = 8;

pub fn read(data: &[u8]) -> Result<Twpb> {
    let mut r = Reader::new(data, Format::Chnk);
    let kind = r.u16_le()?;
    let width = r.u16_le()?;
    let height = r.u16_le()?;
    let nl = r.u16_le()? as usize;
    let nr = r.u16_le()? as usize;
    let mut sides = [Vec::with_capacity(nl), Vec::with_capacity(nr)];
    for (side, n) in sides.iter_mut().zip([nl, nr]) {
        for _ in 0..n {
            side.push((r.i16_le()?, r.i16_le()?));
        }
        if side.windows(2).any(|w| w[1].1 < w[0].1) {
            return Err(r.error(ErrorKind::Invalid("TWPB: y fora de ordem".into())));
        }
    }
    let [left, right] = sides;
    Ok(Twpb {
        kind,
        width,
        height,
        left,
        right,
    })
}

pub fn write(t: &Twpb) -> Vec<u8> {
    let mut v = Vec::with_capacity(10 + 4 * (t.left.len() + t.right.len()));
    for x in [
        t.kind,
        t.width,
        t.height,
        t.left.len() as u16,
        t.right.len() as u16,
    ] {
        v.extend(x.to_le_bytes());
    }
    for &(x, y) in t.left.iter().chain(&t.right) {
        v.extend(x.to_le_bytes());
        v.extend(y.to_le_bytes());
    }
    v
}

/// Margem na altura `y` pela reta entre os vértices (fora deles, `None`).
pub fn margin_at(side: &[(i16, i16)], y: f32) -> Option<f32> {
    let (first, last) = (side.first()?, side.last()?);
    if y < first.1 as f32 || y > last.1 as f32 {
        return None;
    }
    for w in side.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if (y0 as f32..=y1 as f32).contains(&y) {
            if y1 == y0 {
                return Some(x0.min(x1) as f32);
            }
            let t = (y - y0 as f32) / (y1 - y0) as f32;
            return Some(x0 as f32 + t * (x1 - x0) as f32);
        }
    }
    Some(first.0 as f32)
}

impl Twpb {
    /// Margens `(esquerda, direita)` na linha `y` (centro da linha em `y + 0,5`).
    pub fn margins(&self, y: f32) -> Option<(f32, f32)> {
        Some((margin_at(&self.left, y)?, margin_at(&self.right, y)?))
    }
}

/// Tolerância da redução de pontos, em pixels.
pub const TOLERANCE: f32 = 0.5;
/// Folga em volta da tinta (x, y), em pixels: o original deixa as margens
/// 2 px menores na horizontal e engorda 1 px na vertical.
pub const PAD: (usize, usize) = (2, 1);

/// Contorno de uma máscara `w`×`h` (`true` = tinta), como o Writer calcula
/// ao inserir um objeto: a tinta engordada em [`PAD`], a margem de cada
/// linha com tinta, reduzida aos vértices de uma poligonal que não se afasta
/// mais de [`TOLERANCE`] px. Nos 58 exemplos com figura, 95% das linhas
/// ficam a até 2 px do contorno gravado pelo original.
pub fn from_mask(mask: &[bool], w: u32, h: u32) -> Twpb {
    let (wi, hi) = (w as usize, h as usize);
    let mask = &pad(mask, wi, hi)[..];
    let mut left = Vec::new();
    let mut right = Vec::new();
    for y in 0..hi {
        let row = &mask[y * wi..(y + 1) * wi];
        if let (Some(a), Some(b)) = (row.iter().position(|&p| p), row.iter().rposition(|&p| p)) {
            left.push((a as i16, y as i16));
            right.push(((wi - 1 - b) as i16, y as i16));
        }
    }
    // a última linha com tinta vai até a sua borda de baixo
    for side in [&mut left, &mut right] {
        if let Some(&(x, y)) = side.last() {
            side.push((x, y + 1));
        }
    }
    Twpb {
        kind: KIND_PICTURE,
        width: w as u16,
        height: h as u16,
        left: simplify(&left, TOLERANCE),
        right: simplify(&right, TOLERANCE),
    }
}

/// Máscara engordada [`PAD`] pixels (máximo num retângulo).
fn pad(mask: &[bool], w: usize, h: usize) -> Vec<bool> {
    let (px, py) = PAD;
    let mut rows = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            rows[y * w + x] =
                (x.saturating_sub(px)..(x + px + 1).min(w)).any(|xx| mask[y * w + xx]);
        }
    }
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            (y.saturating_sub(py)..(y + py + 1).min(h)).any(|yy| rows[yy * w + x])
        })
        .collect()
}

/// Douglas–Peucker na direção x (as margens são funções de y).
fn simplify(pts: &[(i16, i16)], tol: f32) -> Vec<(i16, i16)> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let mut keep = vec![false; pts.len()];
    keep[0] = true;
    keep[pts.len() - 1] = true;
    let mut stack = vec![(0usize, pts.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let ((x0, y0), (x1, y1)) = (pts[a], pts[b]);
        let mut worst = (0.0f32, a);
        for (k, &(x, y)) in pts.iter().enumerate().take(b).skip(a + 1) {
            let t = if y1 == y0 {
                0.0
            } else {
                (y - y0) as f32 / (y1 - y0) as f32
            };
            let d = (x as f32 - (x0 as f32 + t * (x1 - x0) as f32)).abs();
            if d > worst.0 {
                worst = (d, k);
            }
        }
        if worst.0 > tol {
            keep[worst.1] = true;
            stack.push((a, worst.1));
            stack.push((worst.1, b));
        }
    }
    pts.iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Triângulo de 9×8: linha y tem tinta de 4−y/2 a 4+y/2.
    fn triangle() -> Vec<bool> {
        let (w, h) = (9usize, 8usize);
        (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as i32, (i / w) as i32);
                (x - 4).abs() <= y / 2
            })
            .collect()
    }

    #[test]
    fn from_mask_keeps_vertices_and_closes_bottom() {
        let t = from_mask(&triangle(), 9, 8);
        assert_eq!((t.kind, t.width, t.height), (8, 9, 8));
        // engordado 2 px dos lados e 1 px embaixo
        assert_eq!(t.left.first(), Some(&(2, 0)));
        assert_eq!(t.left.last(), Some(&(0, 8)));
        assert!(t.left.len() < 9, "{:?}", t.left);
        for y in 0..8 {
            let (l, r) = t.margins(y as f32).unwrap();
            let want = (4.0 - ((y + 1).min(7) / 2) as f32 - 2.0).max(0.0);
            assert!((l - want).abs() <= TOLERANCE + 0.01, "y {y}: {l} {want}");
            assert!((r - want).abs() <= TOLERANCE + 0.01, "y {y}: {r} {want}");
        }
        assert_eq!(t.margins(9.0), None);
    }

    #[test]
    fn empty_rows_are_skipped_and_roundtrip() {
        let mut m = vec![false; 9 * 8];
        m[3 * 9 + 4] = true; // linha 3, x 4: engordado, linhas 2 a 4
        let t = from_mask(&m, 9, 8);
        assert_eq!(t.left, vec![(2, 2), (2, 5)]);
        assert_eq!(t.right, vec![(2, 2), (2, 5)]);
        assert_eq!(read(&write(&t)).unwrap(), t);
        assert!(read(&write(&t)[..12]).is_err());
        let mut bad = t.clone();
        bad.left.swap(0, 1);
        assert!(read(&write(&bad)).is_err());
    }
}
