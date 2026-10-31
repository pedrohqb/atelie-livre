//! Funções puras da camada de plataforma (testáveis sem SDL).

use engine::{Image, SCREEN_H, SCREEN_W};

/// Maior fator inteiro de ampliação que cabe na janela (mínimo 1).
pub fn integer_scale(win_w: u32, win_h: u32) -> u32 {
    (win_w / SCREEN_W).min(win_h / SCREEN_H).max(1)
}

/// Converte coordenadas lógicas da SDL em pixels do quadro; fora dele (faixas pretas) → `None`.
pub fn logical_point(x: f32, y: f32) -> Option<(i32, i32)> {
    let inside = x >= 0.0 && y >= 0.0 && x < SCREEN_W as f32 && y < SCREEN_H as f32;
    inside.then_some((x as i32, y as i32))
}

/// Repetição de tecla (tecla segurada) só vale para as teclas de navegação.
pub fn key_allowed(key: engine::scene::Key, repeat: bool) -> bool {
    use engine::scene::Key::*;
    !repeat || matches!(key, Up | Down | Left | Right | PageUp | PageDown)
}

/// Amplia um cursor pelo vizinho mais próximo.
pub fn scale_cursor(img: &Image, factor: u32) -> Image {
    let f = factor.max(1);
    let (w, h) = (img.width * f, img.height * f);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let i = (((y / f) * img.width + x / f) * 4) as usize;
            rgba.extend_from_slice(&img.rgba[i..i + 4]);
        }
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_scale_minimum_one() {
        assert_eq!(integer_scale(300, 200), 1);
        assert_eq!(integer_scale(1920, 1080), 2);
        assert_eq!(integer_scale(2560, 1440), 3);
        assert_eq!(integer_scale(0, 0), 1);
    }

    #[test]
    fn logical_point_rejects_bars() {
        assert_eq!(logical_point(-1.0, 10.0), None);
        assert_eq!(logical_point(640.0, 10.0), None);
        assert_eq!(logical_point(10.0, 480.0), None);
        assert_eq!(logical_point(f32::NAN, 10.0), None);
        assert_eq!(logical_point(639.9, 479.9), Some((639, 479)));
        assert_eq!(logical_point(0.0, 0.0), Some((0, 0)));
    }

    #[test]
    fn repeat_allowed_only_for_navigation_keys() {
        use engine::scene::Key;
        for k in [
            Key::Up,
            Key::Down,
            Key::Left,
            Key::Right,
            Key::PageUp,
            Key::PageDown,
        ] {
            assert!(key_allowed(k, true), "{k:?}");
        }
        for k in [
            Key::Enter,
            Key::Escape,
            Key::F12,
            Key::Q,
            Key::Home,
            Key::End,
            Key::Other,
        ] {
            assert!(!key_allowed(k, true), "{k:?}");
            assert!(key_allowed(k, false));
        }
    }

    #[test]
    fn scale_cursor_doubles() {
        let img = Image {
            width: 2,
            height: 2,
            rgba: vec![1, 1, 1, 255, 2, 2, 2, 255, 3, 3, 3, 255, 4, 4, 4, 0],
        };
        let s = scale_cursor(&img, 2);
        assert_eq!((s.width, s.height), (4, 4));
        let px = |x: usize, y: usize| &s.rgba[(y * 4 + x) * 4..(y * 4 + x) * 4 + 4];
        assert_eq!(px(1, 1), &[1, 1, 1, 255]);
        assert_eq!(px(2, 0), &[2, 2, 2, 255]);
        assert_eq!(px(3, 3), &[4, 4, 4, 0]);
        assert_eq!(scale_cursor(&img, 1), img);
    }
}
