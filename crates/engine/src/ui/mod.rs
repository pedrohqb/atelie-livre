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
}
