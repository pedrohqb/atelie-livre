//! Componentes de interface compartilhados pelas cenas (Fine Artist e, mais
//! adiante, Creative Writer): retângulos e teste de clique, texto cortado com
//! "…", a "caneta" da fonte de 8 pontos, detecção de clique duplo, fila de
//! sons em sequência e desenho de quadros indexados (paleta) no [`Frame`].
//!
//! Antes de escrever outra cópia de qualquer um destes, use os daqui (ver
//! `docs/arquitetura.md`).
//!
//! [`Frame`]: crate::Frame

pub mod canvas;
pub mod double_click;
pub mod sound_queue;
pub mod text;

pub use canvas::{blit_canvas, blit_canvas_fit, pal_rgb};
pub use double_click::{DOUBLE_CLICK_S, DoubleClick};
pub use sound_queue::SoundQueue;
pub use text::{Pen, clip_text};

/// Retângulo de tela: `(x, y, largura, altura)`.
pub type Rect = (i32, i32, i32, i32);

/// O ponto `(x, y)` está dentro de `r` (borda direita e de baixo de fora).
pub fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

/// Xadrez de pontos `rgb` em `r` (nos pixels com `x + y` par): o "apagado"
/// das telas por trás dos diálogos e dos botões desligados.
pub fn checker(f: &mut crate::Frame, r: Rect, rgb: [u8; 3]) {
    for y in r.1..r.1 + r.3 {
        for x in r.0..r.0 + r.2 {
            if (x + y) % 2 == 0 {
                f.put(x, y, rgb);
            }
        }
    }
}

/// Centro de `r` (útil nos testes que clicam em botões).
pub fn center(r: Rect) -> (i32, i32) {
    (r.0 + r.2 / 2, r.1 + r.3 / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_excludes_right_and_bottom_edges() {
        let r = (10, 20, 5, 4);
        assert!(inside(r, 10, 20));
        assert!(inside(r, 14, 23));
        assert!(!inside(r, 15, 23));
        assert!(!inside(r, 14, 24));
        assert!(!inside(r, 9, 20));
        assert_eq!(center(r), (12, 22));
    }

    #[test]
    fn checker_paints_even_pixels() {
        let mut f = crate::Frame::new(4, 4);
        f.fill([9, 9, 9]);
        checker(&mut f, (1, 1, 2, 2), [1, 2, 3]);
        assert_eq!(&f.pixel(1, 1)[..3], &[1, 2, 3]);
        assert_eq!(&f.pixel(2, 1)[..3], &[9, 9, 9]);
        assert_eq!(&f.pixel(2, 2)[..3], &[1, 2, 3]);
        assert_eq!(&f.pixel(0, 0)[..3], &[9, 9, 9]);
    }
}
