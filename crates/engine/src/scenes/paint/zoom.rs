//! Lente de aumento da tela de pintura: o quadro aparece ampliado 3× e quatro
//! setas na barra de baixo rolam a vista (medido no original, ver
//! `docs/telas/pintura-menu-mczee.md`).

use crate::Frame;
use crate::assets::Assets;
use crate::paint::{CANVAS_H, CANVAS_W, ORIGIN};
use crate::text_balloon::blit_pic;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];

/// Fator de ampliação.
pub const FACTOR: i32 = 3;
/// Caixa branca das setas na barra de baixo (x da borda esquerda e da direita).
const BOX: (i32, i32) = (558, 626);
/// Setas ◀ ▶ ▲ ▼: figura e canto superior esquerdo.
const ARROWS: [(Dir, u16, i32); 4] = [
    (Dir::Left, 0x14f3, 565),
    (Dir::Right, 0x151b, 579),
    (Dir::Up, 0x151f, 593),
    (Dir::Down, 0x1523, 607),
];
const ARROW_Y: i32 = 462;
const ARROW_SIZE: i32 = 12;
/// Quanto cada seta rola, em pixels do quadro (metade da vista).
const STEP: i32 = VIEW_W / 2;

/// Tamanho da vista, em pixels do quadro (a última linha fica parcial).
pub const VIEW_W: i32 = CANVAS_W as i32 / FACTOR;
pub const VIEW_H: i32 = CANVAS_H as i32 / FACTOR;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

/// Lente ligada: canto superior esquerdo da vista, em pixels do quadro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zoom {
    pub ox: i32,
    pub oy: i32,
}

impl Zoom {
    /// Vista centrada em `(cx, cy)` (pixels do quadro), presa às bordas.
    pub fn centered(cx: i32, cy: i32) -> Zoom {
        let mut z = Zoom {
            ox: cx - VIEW_W / 2,
            oy: cy - VIEW_H / 2,
        };
        z.clamp();
        z
    }

    fn clamp(&mut self) {
        self.ox = self.ox.clamp(0, CANVAS_W as i32 - VIEW_W);
        // a última linha da vista pode ficar parcial: a vista cobre VIEW_H+1
        self.oy = self.oy.clamp(0, CANVAS_H as i32 - VIEW_H - 1);
    }

    /// Rola a vista; `false` quando já estava na borda (sem som no original).
    pub fn scroll(&mut self, d: Dir) -> bool {
        let before = *self;
        match d {
            Dir::Left => self.ox -= STEP,
            Dir::Right => self.ox += STEP,
            Dir::Up => self.oy -= STEP,
            Dir::Down => self.oy += STEP,
        }
        self.clamp();
        *self != before
    }

    /// Ponto da tela → pixel do quadro sob a lente.
    pub fn to_canvas(&self, x: i32, y: i32) -> (i32, i32) {
        (
            self.ox + (x - ORIGIN.0).div_euclid(FACTOR),
            self.oy + (y - ORIGIN.1).div_euclid(FACTOR),
        )
    }

    /// Quadro ampliado, pixel a pixel (`rgb` dá a cor de um índice).
    pub fn draw_canvas(&self, f: &mut Frame, px: &[u8], w: u32, rgb: impl Fn(u8) -> [u8; 3]) {
        for sy in 0..CANVAS_H as i32 {
            let cy = self.oy + sy / FACTOR;
            for sx in 0..CANVAS_W as i32 {
                let cx = self.ox + sx / FACTOR;
                let k = (cy as u32 * w + cx as u32) as usize;
                if let Some(&i) = px.get(k) {
                    f.put(ORIGIN.0 + sx, ORIGIN.1 + sy, rgb(i));
                }
            }
        }
    }
}

/// Seta sob `(x, y)`.
pub fn arrow_at(x: i32, y: i32) -> Option<Dir> {
    ARROWS.iter().find_map(|&(d, _, ax)| {
        ((ax..ax + ARROW_SIZE).contains(&x) && (ARROW_Y..ARROW_Y + ARROW_SIZE).contains(&y))
            .then_some(d)
    })
}

/// Caixa branca com as quatro setas, na barra de baixo.
pub fn draw_arrows(f: &mut Frame, a: &Assets) {
    let (x0, x1) = BOX;
    f.rect(x0, 461, 1, 14, BLACK);
    f.rect(x1, 461, 1, 14, BLACK);
    f.rect(x0 + 1, 461, x1 - x0 - 1, 14, WHITE);
    for &(_, id, ax) in &ARROWS {
        blit_pic(f, a, id, ax, ARROW_Y, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_view_is_clamped_and_scrolls_to_edges() {
        let z = Zoom::centered(0, 0);
        assert_eq!((z.ox, z.oy), (0, 0));
        let z = Zoom::centered(1000, 1000);
        assert_eq!(z.ox, CANVAS_W as i32 - VIEW_W);
        assert_eq!(z.oy, CANVAS_H as i32 - VIEW_H - 1);
        let mut z = Zoom::centered(0, 0);
        assert!(!z.scroll(Dir::Left));
        assert!(!z.scroll(Dir::Up));
        assert!(z.scroll(Dir::Right));
        assert_eq!(z.ox, STEP);
        assert!(z.scroll(Dir::Down));
        assert_eq!(z.oy, STEP);
    }

    #[test]
    fn screen_points_map_to_the_view() {
        let z = Zoom::centered(300, 170);
        assert_eq!(z.to_canvas(ORIGIN.0, ORIGIN.1), (z.ox, z.oy));
        assert_eq!(
            z.to_canvas(ORIGIN.0 + 5, ORIGIN.1 + 4),
            (z.ox + 1, z.oy + 1)
        );
        assert_eq!(arrow_at(570, 468), Some(Dir::Left));
        assert_eq!(arrow_at(612, 468), Some(Dir::Down));
        assert_eq!(arrow_at(550, 468), None);
    }
}
