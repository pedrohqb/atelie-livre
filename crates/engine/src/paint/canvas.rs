//! Quadro de pintura (índices da paleta de 256 cores), tinta, traço e histórico.

use super::shape::{Cell, Shape};

pub const CANVAS_W: u32 = 594;
pub const CANVAS_H: u32 = 347;
/// Canto superior esquerdo do quadro na tela lógica.
pub const ORIGIN: (i32, i32) = (23, 109);
/// Índices da paleta de 256 cores.
pub const WHITE: u8 = 0;
pub const BLACK: u8 = 1;

/// Cor mais padrão 8×8. No pixel de tela `(x, y)`, bit 0 pinta a cor e bit 1 pinta branco.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ink {
    pub color: u8,
    pub pattern: [u8; 8],
}

impl Ink {
    pub fn solid(color: u8) -> Ink {
        Ink {
            color,
            pattern: [0; 8],
        }
    }

    /// Índice resultante no pixel de tela `(sx, sy)`.
    pub fn index_at(&self, sx: i32, sy: i32) -> u8 {
        let row = self.pattern[sy.rem_euclid(8) as usize];
        if (row >> (7 - sx.rem_euclid(8))) & 1 == 0 {
            self.color
        } else {
            WHITE
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

impl Canvas {
    pub fn new(w: u32, h: u32) -> Canvas {
        Canvas {
            w,
            h,
            px: vec![WHITE; (w * h) as usize],
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as u32) < self.w && (y as u32) < self.h)
            .then(|| y as usize * self.w as usize + x as usize)
    }

    pub fn get(&self, x: i32, y: i32) -> Option<u8> {
        self.index(x, y).map(|i| self.px[i])
    }

    /// Pinta um pixel do quadro com a tinta (recortado nas bordas).
    pub fn plot(&mut self, x: i32, y: i32, ink: &Ink) {
        if let Some(i) = self.index(x, y) {
            self.px[i] = ink.index_at(x + ORIGIN.0, y + ORIGIN.1);
        }
    }

    pub fn clear(&mut self) {
        self.px.fill(WHITE);
    }

    /// Carimba a forma centrada em `(cx, cy)`, coordenadas do quadro.
    pub fn stamp(&mut self, s: &Shape, cx: i32, cy: i32, ink: &Ink) {
        let x0 = cx - s.w as i32 / 2;
        let y0 = cy - s.h as i32 / 2;
        for (k, cell) in s.cells.iter().enumerate() {
            let x = x0 + (k as u32 % s.w) as i32;
            let y = y0 + (k as u32 / s.w) as i32;
            match cell {
                Cell::Clear => {}
                Cell::Ink => self.plot(x, y, ink),
                Cell::White => {
                    if let Some(i) = self.index(x, y) {
                        self.px[i] = WHITE;
                    }
                }
            }
        }
    }

    /// Preenche a região de vizinhança 4 com o mesmo índice do pixel `(x, y)`.
    pub fn fill(&mut self, x: i32, y: i32, ink: &Ink) {
        let Some(start) = self.index(x, y) else {
            return;
        };
        let target = self.px[start];
        let mut seen = vec![false; self.px.len()];
        let mut stack = vec![(x, y)];
        seen[start] = true;
        let mut region = Vec::new();
        while let Some((px, py)) = stack.pop() {
            region.push((px, py));
            for (nx, ny) in [(px - 1, py), (px + 1, py), (px, py - 1), (px, py + 1)] {
                if let Some(i) = self.index(nx, ny)
                    && !seen[i]
                    && self.px[i] == target
                {
                    seen[i] = true;
                    stack.push((nx, ny));
                }
            }
        }
        for (px, py) in region {
            self.plot(px, py, ink);
        }
    }
}

/// Pontos da reta de `a` a `b` (Bresenham), incluindo as duas pontas.
pub fn line_points(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = a;
    let dx = (b.0 - x).abs();
    let dy = -(b.1 - y).abs();
    let sx = if x < b.0 { 1 } else { -1 };
    let sy = if y < b.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut out = Vec::with_capacity((dx - dy + 1) as usize);
    loop {
        out.push((x, y));
        if (x, y) == b {
            return out;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Desfazer de um nível: guarda o quadro antes da última ação; desfazer e refazer se alternam.
#[derive(Debug, Clone, Default)]
pub struct History {
    saved: Option<Vec<u8>>,
}

impl History {
    /// Chamar antes de cada ação que muda o quadro.
    pub fn record(&mut self, c: &Canvas) {
        self.saved = Some(c.px.clone());
    }

    /// Troca o quadro pelo guardado (desfaz ou refaz). Falso se não há nada guardado.
    pub fn toggle(&mut self, c: &mut Canvas) -> bool {
        match &mut self.saved {
            Some(s) if s.len() == c.px.len() => {
                std::mem::swap(s, &mut c.px);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(c: &Canvas, v: u8) -> usize {
        c.px.iter().filter(|&&p| p == v).count()
    }

    #[test]
    fn ink_solid_pattern_is_color() {
        let ink = Ink::solid(7);
        for (x, y) in [(0, 0), (5, 3), (-1, -9), (639, 479)] {
            assert_eq!(ink.index_at(x, y), 7);
        }
    }

    #[test]
    fn ink_pattern_bit_one_is_white() {
        // 0x80 na linha 0: só a coluna 0 (mod 8) tem bit 1
        let ink = Ink {
            color: 9,
            pattern: [0x80, 0, 0, 0, 0, 0, 0, 0],
        };
        assert_eq!(ink.index_at(0, 0), WHITE);
        assert_eq!(ink.index_at(8, 16), WHITE);
        assert_eq!(ink.index_at(1, 0), 9);
        assert_eq!(ink.index_at(0, 1), 9);
        assert_eq!(ink.index_at(-8, -8), WHITE);
    }

    #[test]
    fn plot_uses_screen_coordinates_for_pattern() {
        let mut c = Canvas::new(16, 16);
        let ink = Ink {
            color: 9,
            pattern: [0x80, 0, 0, 0, 0, 0, 0, 0],
        };
        // pixel de tela (24, 112) = quadro (1, 3): coluna 0, linha 0 → branco
        c.plot(1, 3, &ink);
        assert_eq!(c.get(1, 3), Some(WHITE));
        c.plot(2, 3, &ink);
        assert_eq!(c.get(2, 3), Some(9));
    }

    #[test]
    fn stamp_clips_at_edges() {
        let mut c = Canvas::new(10, 10);
        c.stamp(&Shape::square(4), 0, 0, &Ink::solid(BLACK));
        assert_eq!(count(&c, BLACK), 4);
        c.stamp(&Shape::square(4), 100, 100, &Ink::solid(BLACK));
        c.stamp(&Shape::square(4), -100, 5, &Ink::solid(BLACK));
        assert_eq!(count(&c, BLACK), 4);
    }

    #[test]
    fn stamp_white_cells_paint_white() {
        let mut c = Canvas::new(4, 4);
        c.px.fill(5);
        let s = Shape {
            w: 2,
            h: 1,
            cells: vec![Cell::White, Cell::Clear],
        };
        c.stamp(&s, 1, 0, &Ink::solid(BLACK));
        assert_eq!(c.get(0, 0), Some(WHITE));
        assert_eq!(c.get(1, 0), Some(5));
    }

    #[test]
    fn line_points_endpoints_and_length() {
        let p = line_points((0, 0), (10, 3));
        assert_eq!(p.first(), Some(&(0, 0)));
        assert_eq!(p.last(), Some(&(10, 3)));
        assert_eq!(p.len(), 11);
        assert_eq!(line_points((5, 5), (5, 5)), vec![(5, 5)]);
        let q = line_points((3, 9), (3, 0));
        assert_eq!(q.len(), 10);
        assert_eq!(q.last(), Some(&(3, 0)));
    }

    #[test]
    fn fill_bounded_region() {
        let mut c = Canvas::new(10, 10);
        for i in 0..10 {
            c.px[5 * 10 + i] = BLACK; // linha horizontal em y = 5
        }
        c.fill(2, 2, &Ink::solid(4));
        assert_eq!(count(&c, 4), 50);
        assert_eq!(c.get(2, 8), Some(WHITE));
    }

    #[test]
    fn fill_does_not_leak_diagonally() {
        let mut c = Canvas::new(3, 3);
        // diagonal preta separa os cantos
        c.px = vec![0, 1, 0, 1, 0, 1, 0, 1, 0];
        c.fill(0, 0, &Ink::solid(4));
        assert_eq!(c.px, vec![4, 1, 0, 1, 0, 1, 0, 1, 0]);
    }

    #[test]
    fn fill_same_color_terminates() {
        let mut c = Canvas::new(50, 50);
        c.fill(10, 10, &Ink::solid(WHITE));
        assert_eq!(count(&c, WHITE), 2500);
        let pat = Ink {
            color: WHITE,
            pattern: [0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa],
        };
        c.fill(0, 0, &pat);
        c.fill(-1, 0, &pat);
        assert_eq!(count(&c, WHITE), 2500);
    }

    #[test]
    fn history_toggle_undo_redo() {
        let mut c = Canvas::new(4, 4);
        let mut h = History::default();
        assert!(!h.toggle(&mut c));
        h.record(&c);
        c.plot(0, 0, &Ink::solid(BLACK));
        assert!(h.toggle(&mut c));
        assert_eq!(c.get(0, 0), Some(WHITE));
        assert!(h.toggle(&mut c));
        assert_eq!(c.get(0, 0), Some(BLACK));
    }
}
