//! Formas do pincel, medidas 1:1 nos botões do Fine Artist original.

use crate::Image;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// Não pinta.
    Clear,
    /// Pinta com a tinta (cor + padrão).
    Ink,
    /// Pinta de branco (interior dos carimbos).
    White,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    pub w: u32,
    pub h: u32,
    /// Linha a linha, `w * h` células.
    pub cells: Vec<Cell>,
}

/// Larguras das linhas dos pincéis redondos do original (cada linha centrada).
const ROUND_16: [u32; 16] = [6, 10, 12, 14, 14, 16, 16, 16, 16, 16, 16, 14, 14, 12, 10, 6];
const ROUND_12: [u32; 12] = [4, 8, 10, 10, 12, 12, 12, 12, 10, 10, 8, 4];
const ROUND_8: [u32; 8] = [4, 6, 8, 8, 8, 8, 6, 4];
const ROUND_4: [u32; 4] = [2, 4, 4, 2];

impl Shape {
    fn from_fn(w: u32, h: u32, f: impl Fn(u32, u32) -> bool) -> Shape {
        let cells = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| if f(x, y) { Cell::Ink } else { Cell::Clear })
            .collect();
        Shape { w, h, cells }
    }

    /// Redondo de 16, 12, 8 ou 4 pixels; outros diâmetros viram quadrado.
    pub fn round(d: u32) -> Shape {
        let rows: &[u32] = match d {
            16 => &ROUND_16,
            12 => &ROUND_12,
            8 => &ROUND_8,
            4 => &ROUND_4,
            _ => return Shape::square(d),
        };
        Shape::from_fn(d, d, |x, y| {
            let len = rows[y as usize];
            let start = (d - len) / 2;
            x >= start && x < start + len
        })
    }

    /// Disco de diâmetro `d`: os redondos medidos do original quando existem,
    /// senão o círculo calculado.
    pub fn disc(d: u32) -> Shape {
        if matches!(d, 4 | 8 | 12 | 16) {
            return Shape::round(d);
        }
        let r = d as f32 / 2.0;
        Shape::from_fn(d, d, |x, y| {
            let (dx, dy) = (x as f32 + 0.5 - r, y as f32 + 0.5 - r);
            dx * dx + dy * dy <= r * r
        })
    }

    pub fn square(s: u32) -> Shape {
        Shape::from_fn(s, s, |_, _| true)
    }

    pub fn plus() -> Shape {
        Shape::from_fn(3, 3, |x, y| x == 1 || y == 1)
    }

    pub fn cross() -> Shape {
        Shape::from_fn(3, 3, |x, y| x == y || x + y == 2)
    }

    pub fn dot() -> Shape {
        Shape::square(1)
    }

    /// Traço "\" de `n` linhas, duas colunas por linha (a última só uma).
    pub fn diag_down(n: u32) -> Shape {
        Shape::from_fn(n, n, |x, y| x == y || x == y + 1)
    }

    /// Traço "/" de `n` linhas, espelho de `diag_down`.
    pub fn diag_up(n: u32) -> Shape {
        Shape::from_fn(n, n, |x, y| {
            let m = n - 1 - x;
            m == y || m == y + 1
        })
    }

    pub fn hline(n: u32) -> Shape {
        Shape::from_fn(n, 1, |_, _| true)
    }

    pub fn vline(n: u32) -> Shape {
        Shape::from_fn(1, n, |_, _| true)
    }

    /// Carimbo a partir de um bitmap: preto pinta com a tinta, branco interior
    /// pinta de branco, e os brancos ligados à borda (vizinhança 4) não pintam.
    pub fn from_stamp(img: &Image) -> Shape {
        let (w, h) = (img.width, img.height);
        let dark = |i: usize| {
            let p = &img.rgba[i * 4..i * 4 + 3];
            (p[0] as u32 + p[1] as u32 + p[2] as u32) < 384
        };
        let n = (w * h) as usize;
        let mut cells: Vec<Cell> = (0..n)
            .map(|i| if dark(i) { Cell::Ink } else { Cell::White })
            .collect();
        let mut stack: Vec<(u32, u32)> = Vec::new();
        for x in 0..w {
            stack.push((x, 0));
            stack.push((x, h.saturating_sub(1)));
        }
        for y in 0..h {
            stack.push((0, y));
            stack.push((w.saturating_sub(1), y));
        }
        while let Some((x, y)) = stack.pop() {
            if x >= w || y >= h {
                continue;
            }
            let i = (y * w + x) as usize;
            if cells[i] != Cell::White {
                continue;
            }
            cells[i] = Cell::Clear;
            stack.push((x + 1, y));
            stack.push((x, y + 1));
            if x > 0 {
                stack.push((x - 1, y));
            }
            if y > 0 {
                stack.push((x, y - 1));
            }
        }
        Shape { w, h, cells }
    }

    pub fn count_ink(&self) -> usize {
        self.cells.iter().filter(|&&c| c == Cell::Ink).count()
    }
}

/// Formas das páginas 1 e 4 do pincel, na ordem dos botões do original.
/// Outras páginas não são geométricas: vazio.
pub fn page_shapes(page: u8) -> Vec<Shape> {
    match page {
        1 => vec![
            Shape::round(16),
            Shape::round(12),
            Shape::round(8),
            Shape::round(4),
            Shape::plus(),
            Shape::cross(),
            Shape::dot(),
            Shape::square(2),
            Shape::square(4),
            Shape::square(8),
            Shape::square(12),
            Shape::square(16),
        ],
        4 => vec![
            Shape::diag_down(16),
            Shape::diag_down(12),
            Shape::diag_down(8),
            Shape::diag_up(8),
            Shape::diag_up(12),
            Shape::diag_up(16),
            Shape::hline(8),
            Shape::hline(12),
            Shape::hline(16),
            Shape::vline(16),
            Shape::vline(12),
            Shape::vline(8),
        ],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_counts_match_original() {
        assert_eq!(Shape::round(16).count_ink(), 208);
        assert_eq!(Shape::round(12).count_ink(), 112);
        assert_eq!(Shape::round(8).count_ink(), 52);
        assert_eq!(Shape::round(4).count_ink(), 12);
    }

    #[test]
    fn square_16_has_256() {
        assert_eq!(Shape::square(16).count_ink(), 256);
        assert_eq!(Shape::square(2).count_ink(), 4);
    }

    #[test]
    fn diag_16_has_31() {
        assert_eq!(Shape::diag_down(16).count_ink(), 31);
        assert_eq!(Shape::diag_up(16).count_ink(), 31);
        assert_eq!(Shape::diag_down(8).count_ink(), 15);
        // "/" começa no canto superior direito
        let s = Shape::diag_up(8);
        assert_eq!(s.cells[7], Cell::Ink);
        assert_eq!(s.cells[0], Cell::Clear);
    }

    #[test]
    fn small_shapes_match_original() {
        assert_eq!(Shape::plus().count_ink(), 5);
        assert_eq!(Shape::cross().count_ink(), 5);
        assert_eq!(Shape::cross().cells[1], Cell::Clear);
        assert_eq!(Shape::dot().count_ink(), 1);
    }

    #[test]
    fn disc_uses_original_rounds_and_is_round() {
        assert_eq!(Shape::disc(16), Shape::round(16));
        let d = Shape::disc(32);
        assert_eq!((d.w, d.h), (32, 32));
        assert_eq!(d.cells[0], Cell::Clear);
        assert_eq!(d.cells[16 * 32 + 16], Cell::Ink);
        // área perto de π·16²
        assert!((780..=830).contains(&d.count_ink()), "{}", d.count_ink());
    }

    #[test]
    fn pages_have_twelve_shapes() {
        assert_eq!(page_shapes(1).len(), 12);
        assert_eq!(page_shapes(4).len(), 12);
        assert!(page_shapes(2).is_empty());
    }

    #[test]
    fn stamp_mask_exterior_transparent() {
        // anel preto 5×5 com miolo branco: o miolo pinta branco, os cantos de fora não pintam
        let ring = [
            "..#..", //
            ".#.#.", //
            "#...#", //
            ".#.#.", //
            "..#..",
        ];
        let mut rgba = Vec::new();
        for r in ring {
            for ch in r.chars() {
                let v = if ch == '#' { 0 } else { 255 };
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let s = Shape::from_stamp(&Image {
            width: 5,
            height: 5,
            rgba,
        });
        assert_eq!(s.count_ink(), 8);
        assert_eq!(s.cells[0], Cell::Clear);
        assert_eq!(s.cells[12], Cell::White);
        assert_eq!(s.cells[7], Cell::White);
        assert_eq!(s.cells.iter().filter(|&&c| c == Cell::White).count(), 5);
    }
}
