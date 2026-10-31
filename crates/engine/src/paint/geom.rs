//! Formas geométricas da página 5 do pincel (retas, retângulos, elipses e
//! triângulos), como listas de pixels em coordenadas do quadro. Tudo aqui é
//! puro: quem pinta é o `Canvas`, quem pré-visualiza é a cena.
//!
//! A caixa de cada forma tem `a` e `b` como cantos opostos, em qualquer ordem;
//! os dois cantos ficam dentro da forma.

use super::canvas::line_points;

type Px = (i32, i32);

/// Caixa normalizada: `(x0, y0, x1, y1)` com `x0 <= x1` e `y0 <= y1`.
fn bbox(a: Px, b: Px) -> (i32, i32, i32, i32) {
    (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
}

/// Contorno (1 pixel) do retângulo de cantos `a` e `b`.
pub fn rect_points(a: Px, b: Px) -> Vec<Px> {
    let (x0, y0, x1, y1) = bbox(a, b);
    let mut out = Vec::new();
    for x in x0..=x1 {
        out.push((x, y0));
        if y1 != y0 {
            out.push((x, y1));
        }
    }
    for y in y0 + 1..y1 {
        out.push((x0, y));
        if x1 != x0 {
            out.push((x1, y));
        }
    }
    out
}

/// Retângulo cheio de cantos `a` e `b`.
pub fn fill_rect_points(a: Px, b: Px) -> Vec<Px> {
    let (x0, y0, x1, y1) = bbox(a, b);
    (y0..=y1)
        .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
        .collect()
}

/// Contorno (1 pixel) da elipse inscrita na caixa de cantos `a` e `b`
/// (algoritmo do ponto médio para caixas de qualquer paridade).
pub fn ellipse_points(a: Px, b: Px) -> Vec<Px> {
    let (mut x0, mut y0, mut x1, mut y1) = bbox(a, b);
    let aa = (x1 - x0) as i64;
    let bb = (y1 - y0) as i64;
    let b1 = bb & 1;
    let mut dx = 4 * (1 - aa) * bb * bb;
    let mut dy = 4 * (b1 + 1) * aa * aa;
    let mut err = dx + dy + b1 * aa * aa;
    y0 += ((bb + 1) / 2) as i32;
    y1 = y0 - b1 as i32;
    let a8 = 8 * aa * aa;
    let b8 = 8 * bb * bb;
    let mut out = Vec::new();
    loop {
        out.extend([(x1, y0), (x0, y0), (x0, y1), (x1, y1)]);
        let e2 = 2 * err;
        if e2 <= dy {
            y0 += 1;
            y1 -= 1;
            dy += a8;
            err += dy;
        }
        if e2 >= dx || 2 * err > dy {
            x0 += 1;
            x1 -= 1;
            dx += b8;
            err += dx;
        }
        if x0 > x1 {
            break;
        }
    }
    // elipses muito estreitas: completa as pontas verticais
    while ((y0 - y1) as i64) < bb {
        out.extend([(x0 - 1, y0), (x1 + 1, y0), (x0 - 1, y1), (x1 + 1, y1)]);
        y0 += 1;
        y1 -= 1;
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Elipse cheia inscrita na caixa de cantos `a` e `b`.
pub fn fill_ellipse_points(a: Px, b: Px) -> Vec<Px> {
    fill_spans(&ellipse_points(a, b))
}

/// Vértices do triângulo isósceles na caixa de cantos `a` e `b`: base embaixo,
/// ápice no meio de cima.
fn triangle_vertices(a: Px, b: Px) -> [Px; 3] {
    let (x0, y0, x1, y1) = bbox(a, b);
    [(x0 + (x1 - x0) / 2, y0), (x0, y1), (x1, y1)]
}

/// Contorno (1 pixel) do triângulo isósceles na caixa de cantos `a` e `b`.
pub fn triangle_points(a: Px, b: Px) -> Vec<Px> {
    let [top, left, right] = triangle_vertices(a, b);
    let (x0, _, x1, _) = bbox(a, b);
    // a lateral direita é o espelho da esquerda, para o triângulo sair simétrico
    let mut out = line_points(top, left);
    out.extend(
        line_points(top, left)
            .into_iter()
            .map(|(x, y)| (x0 + x1 - x, y)),
    );
    out.extend(line_points(left, right));
    out.sort_unstable();
    out.dedup();
    out
}

/// Triângulo cheio na caixa de cantos `a` e `b`.
pub fn fill_triangle_points(a: Px, b: Px) -> Vec<Px> {
    fill_spans(&triangle_points(a, b))
}

/// Pixels da linha poligonal que passa pelos vértices, na ordem; fechada se
/// `closed`. Um só vértice vira um ponto.
pub fn polyline_points(verts: &[Px], closed: bool) -> Vec<Px> {
    let mut out = Vec::new();
    match verts {
        [] => {}
        [p] => out.push(*p),
        _ => {
            for w in verts.windows(2) {
                out.extend(line_points(w[0], w[1]));
            }
            if closed {
                out.extend(line_points(verts[verts.len() - 1], verts[0]));
            }
        }
    }
    out
}

/// Engrossa uma lista de pixels: cada um vira um quadrado `t × t` centrado nele.
pub fn thicken(points: &[Px], t: i32) -> Vec<Px> {
    let off = t / 2;
    let mut out: Vec<Px> = points
        .iter()
        .flat_map(|&(x, y)| {
            (0..t).flat_map(move |dy| (0..t).map(move |dx| (x - off + dx, y - off + dy)))
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Preenche, linha a linha, do pixel mais à esquerda ao mais à direita do
/// contorno (serve para formas convexas).
fn fill_spans(outline: &[Px]) -> Vec<Px> {
    let Some(y0) = outline.iter().map(|p| p.1).min() else {
        return Vec::new();
    };
    let y1 = outline.iter().map(|p| p.1).max().unwrap_or(y0);
    let mut spans = vec![(i32::MAX, i32::MIN); (y1 - y0 + 1) as usize];
    for &(x, y) in outline {
        let s = &mut spans[(y - y0) as usize];
        s.0 = s.0.min(x);
        s.1 = s.1.max(x);
    }
    spans
        .iter()
        .enumerate()
        .filter(|(_, s)| s.0 <= s.1)
        .flat_map(|(k, &(l, r))| (l..=r).map(move |x| (x, y0 + k as i32)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(p: &[Px]) -> (i32, i32, i32, i32) {
        (
            p.iter().map(|q| q.0).min().unwrap(),
            p.iter().map(|q| q.1).min().unwrap(),
            p.iter().map(|q| q.0).max().unwrap(),
            p.iter().map(|q| q.1).max().unwrap(),
        )
    }

    fn unique(p: &[Px]) -> usize {
        let mut v = p.to_vec();
        v.sort_unstable();
        v.dedup();
        v.len()
    }

    /// Simétrica nos eixos horizontal e vertical da caixa.
    fn symmetric(p: &[Px], a: Px, b: Px) {
        let (x0, y0, x1, y1) = bbox(a, b);
        for &(x, y) in p {
            assert!(
                p.contains(&(x0 + x1 - x, y)),
                "sem espelho horizontal de {x},{y}"
            );
            assert!(
                p.contains(&(x, y0 + y1 - y)),
                "sem espelho vertical de {x},{y}"
            );
        }
    }

    #[test]
    fn rect_outline_and_fill_counts() {
        assert_eq!(unique(&rect_points((0, 0), (3, 3))), 12);
        assert_eq!(unique(&rect_points((3, 3), (0, 0))), 12);
        assert_eq!(unique(&fill_rect_points((0, 0), (3, 3))), 16);
        assert_eq!(unique(&rect_points((5, 5), (5, 5))), 1);
        assert_eq!(unique(&rect_points((0, 0), (4, 0))), 5);
        assert_eq!(unique(&rect_points((0, 0), (0, 4))), 5);
        assert_eq!(bounds(&rect_points((7, -2), (-3, 4))), (-3, -2, 7, 4));
    }

    #[test]
    fn ellipse_is_symmetric_and_bounded() {
        for (a, b) in [
            ((0, 0), (10, 6)),
            ((0, 0), (9, 5)),
            ((4, 9), (-6, 2)),
            ((0, 0), (20, 3)),
        ] {
            let p = ellipse_points(a, b);
            assert_eq!(bounds(&p), bbox(a, b));
            symmetric(&p, a, b);
            let f = fill_ellipse_points(a, b);
            assert_eq!(bounds(&f), bbox(a, b));
            symmetric(&f, a, b);
            assert!(f.len() >= p.len());
        }
    }

    #[test]
    fn ellipse_small_cases() {
        // caixa 1×1: um ponto; 3×3: losango de 4 pontos mais... conferido por contagem
        assert_eq!(ellipse_points((2, 2), (2, 2)), vec![(2, 2)]);
        let p = ellipse_points((0, 0), (2, 2));
        assert_eq!(unique(&p), p.len());
        assert!(
            p.contains(&(1, 0))
                && p.contains(&(0, 1))
                && p.contains(&(2, 1))
                && p.contains(&(1, 2))
        );
        assert!(!p.contains(&(1, 1)));
        // círculo de 5×5: 12 pixels no contorno, 21 cheio
        assert_eq!(unique(&ellipse_points((0, 0), (4, 4))), 12);
        assert_eq!(unique(&fill_ellipse_points((0, 0), (4, 4))), 21);
        // contorno é fechado em vizinhança 8: cada linha da caixa tem pixel
        let p = ellipse_points((0, 0), (30, 12));
        for y in 0..=12 {
            assert!(p.iter().any(|q| q.1 == y), "linha {y} vazia");
        }
        for x in 0..=30 {
            assert!(p.iter().any(|q| q.0 == x), "coluna {x} vazia");
        }
    }

    #[test]
    fn triangle_counts_and_bounds() {
        let p = triangle_points((0, 0), (4, 4));
        assert_eq!(bounds(&p), (0, 0, 4, 4));
        assert!(p.contains(&(2, 0)) && p.contains(&(0, 4)) && p.contains(&(4, 4)));
        // base de 5 e duas laterais de 5 (ápice e cantos compartilhados)
        assert_eq!(unique(&p), 5 + 5 + 5 - 3);
        let f = fill_triangle_points((0, 0), (4, 4));
        assert_eq!(unique(&f), 1 + 3 + 3 + 5 + 5);
        assert!(f.contains(&(2, 2)));
        // espelho horizontal
        for &(x, y) in &p {
            assert!(p.contains(&(4 - x, y)));
        }
        let q = triangle_points((4, 4), (0, 0));
        assert_eq!(unique(&q), unique(&p));
        assert_eq!(unique(&triangle_points((3, 3), (3, 3))), 1);
    }

    #[test]
    fn polyline_and_thicken() {
        assert!(polyline_points(&[], false).is_empty());
        assert_eq!(polyline_points(&[(1, 1)], true), vec![(1, 1)]);
        let open = polyline_points(&[(0, 0), (3, 0), (3, 3)], false);
        assert_eq!(unique(&open), 7);
        let closed = polyline_points(&[(0, 0), (3, 0), (3, 3)], true);
        assert_eq!(unique(&closed), 9);
        assert_eq!(thicken(&[(5, 5)], 3).len(), 9);
        assert_eq!(bounds(&thicken(&[(5, 5)], 3)), (4, 4, 6, 6));
        assert_eq!(thicken(&[(0, 0), (1, 0)], 3).len(), 12);
    }
}
