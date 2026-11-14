//! Quadros indexados ([`Canvas`], 1 byte por pixel, cor pela paleta) na tela.

use crate::Frame;
use crate::paint::Canvas;
use formats::palette::Rgb;

/// Cor do índice `i` na paleta (preto se faltar).
pub fn pal_rgb(pal: &[Rgb], i: u8) -> [u8; 3] {
    pal.get(i as usize).map_or([0, 0, 0], |p| [p.r, p.g, p.b])
}

/// Desenha o quadro inteiro com o canto em `(x, y)`.
pub fn blit_canvas(f: &mut Frame, c: &Canvas, pal: &[Rgb], x: i32, y: i32) {
    for (k, &i) in c.px.iter().enumerate() {
        f.put(
            x + (k as u32 % c.w) as i32,
            y + (k as u32 / c.w) as i32,
            pal_rgb(pal, i),
        );
    }
}

/// Desenha o quadro esticado (vizinho mais próximo) para ocupar `w`×`h`.
pub fn blit_canvas_fit(f: &mut Frame, c: &Canvas, pal: &[Rgb], r: (i32, i32, i32, i32)) {
    let (x, y, w, h) = r;
    if c.w == 0 || c.h == 0 {
        return;
    }
    for ty in 0..h {
        for tx in 0..w {
            let sx = (tx as u32 * c.w / w as u32).min(c.w - 1);
            let sy = (ty as u32 * c.h / h as u32).min(c.h - 1);
            f.put(x + tx, y + ty, pal_rgb(pal, c.px[(sy * c.w + sx) as usize]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pal() -> Vec<Rgb> {
        vec![Rgb { r: 1, g: 2, b: 3 }, Rgb { r: 9, g: 8, b: 7 }]
    }

    #[test]
    fn pal_rgb_falls_back_to_black() {
        assert_eq!(pal_rgb(&pal(), 1), [9, 8, 7]);
        assert_eq!(pal_rgb(&pal(), 200), [0, 0, 0]);
    }

    #[test]
    fn blit_and_fit_place_the_pixels() {
        let mut c = Canvas::new(2, 2);
        c.px = vec![0, 1, 1, 0];
        let mut f = Frame::new(8, 8);
        blit_canvas(&mut f, &c, &pal(), 3, 4);
        assert_eq!(&f.pixel(4, 4)[..3], &[9, 8, 7]);
        assert_eq!(&f.pixel(3, 4)[..3], &[1, 2, 3]);
        let mut f = Frame::new(8, 8);
        blit_canvas_fit(&mut f, &c, &pal(), (0, 0, 4, 4));
        assert_eq!(&f.pixel(3, 0)[..3], &[9, 8, 7]);
        assert_eq!(&f.pixel(3, 3)[..3], &[1, 2, 3]);
    }
}
