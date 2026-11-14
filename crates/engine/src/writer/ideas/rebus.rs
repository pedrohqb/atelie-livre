//! Figuras-rebus: o caractere `c` (1–255) é a figura `2c − 1` do
//! `WRITER.PIC` (colorida, ≈30×28), com a versão em 1 bit em `2c`.
//!
//! O bitmap `2c` não é uma máscara de transparência: em 252 dos 254 pares
//! ele é exatamente o conjunto dos pixels pretos da figura colorida (a
//! versão monocromática, como os pares "colorido / 1 bit" das salas). Nas
//! capturas do original a figura aparece sobre o branco da página e das
//! janelas da Máquina, pixel a pixel igual à colorida. Para desenhar sobre
//! qualquer fundo (seleção, cor de página) o port usa uma máscara calculada:
//! o branco ligado à borda da figura fica transparente; o branco de dentro
//! (o pelo do gato, por exemplo) continua opaco.

use crate::Image;
use crate::assets::{Assets, ImageRef, Program};
use crate::frame::Frame;
use std::rc::Rc;

const WHITE: [u8; 3] = [255, 255, 255];

/// Ids no `WRITER.PIC` da figura colorida e da versão em 1 bit de `c`
/// (`None` para `c = 0`).
pub fn image_ids(c: u8) -> Option<(u16, u16)> {
    (c > 0).then(|| (2 * c as u16 - 1, 2 * c as u16))
}

/// Cópia de `img` com alfa 0 no branco ligado à borda (vizinhança de 4).
pub fn masked(img: &Image) -> Image {
    let (w, h) = (img.width as usize, img.height as usize);
    let white = |i: usize| img.rgba[i * 4..i * 4 + 3] == WHITE && img.rgba[i * 4 + 3] != 0;
    let mut outside = vec![false; w * h];
    let mut stack: Vec<usize> = Vec::new();
    for x in 0..w {
        stack.extend([x, (h.saturating_sub(1)) * w + x]);
    }
    for y in 0..h {
        stack.extend([y * w, y * w + w.saturating_sub(1)]);
    }
    while let Some(i) = stack.pop() {
        if i >= w * h || outside[i] || !white(i) {
            continue;
        }
        outside[i] = true;
        let (x, y) = (i % w, i / w);
        if x > 0 {
            stack.push(i - 1);
        }
        if x + 1 < w {
            stack.push(i + 1);
        }
        if y > 0 {
            stack.push(i - w);
        }
        if y + 1 < h {
            stack.push(i + w);
        }
    }
    let mut out = img.clone();
    for (i, o) in outside.iter().enumerate() {
        if *o {
            out.rgba[i * 4 + 3] = 0;
        }
    }
    out
}

impl Assets {
    /// Figura-rebus `c`, colorida e com o branco de fora transparente;
    /// `None` se `c = 0` ou se o `WRITER.PIC` não tiver a figura.
    pub fn rebus(&self, c: u8) -> Option<Image> {
        let (color, _) = image_ids(c)?;
        let r = ImageRef::Pic(Program::Writer, color);
        self.has_image(&r).then(|| masked(&self.image(&r)))
    }

    /// Versão em 1 bit da figura-rebus `c` (preto e branco, opaca).
    pub fn rebus_mono(&self, c: u8) -> Option<Rc<Image>> {
        let (_, mono) = image_ids(c)?;
        let r = ImageRef::Pic(Program::Writer, mono);
        self.has_image(&r).then(|| self.image(&r))
    }
}

/// Desenha a figura-rebus `c` com o canto em (x, y), sem o branco de fora;
/// devolve o tamanho desenhado (`None` se a figura não existe).
pub fn draw(frame: &mut Frame, assets: &Assets, c: u8, x: i32, y: i32) -> Option<(u32, u32)> {
    let img = assets.rebus(c)?;
    frame.blit(&img, x, y);
    Some((img.width, img.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_of_known_figures() {
        assert_eq!(image_ids(0x25), Some((0x0049, 0x004a)));
        assert_eq!(image_ids(0x1a), Some((0x0033, 0x0034)));
        assert_eq!(image_ids(0xff), Some((0x01fd, 0x01fe)));
        assert_eq!(image_ids(0), None);
    }

    /// Imagem a partir de linhas: `.` branco, `#` preto, `o` laranja.
    fn img(rows: &[&str]) -> Image {
        let mut rgba = Vec::new();
        for r in rows {
            for ch in r.chars() {
                rgba.extend_from_slice(match ch {
                    '.' => &[255, 255, 255, 255],
                    '#' => &[0, 0, 0, 255],
                    _ => &[255, 140, 50, 255],
                });
            }
        }
        Image {
            width: rows[0].len() as u32,
            height: rows.len() as u32,
            rgba,
        }
    }

    fn alpha(i: &Image) -> String {
        (0..i.height)
            .map(|y| {
                (0..i.width)
                    .map(|x| {
                        if i.rgba[((y * i.width + x) * 4 + 3) as usize] == 0 {
                            ' '
                        } else {
                            'x'
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("|")
    }

    #[test]
    fn white_outside_becomes_transparent_inside_stays() {
        let m = masked(&img(&[
            "......", //
            ".####.", //
            ".#..#.", //
            ".#o.#.", //
            ".####.", //
            "......",
        ]));
        assert_eq!(alpha(&m), "      | xxxx | xxxx | xxxx | xxxx |      ");
        // cores preservadas
        assert_eq!(
            &m.rgba[(3 * 6 + 2) * 4..(3 * 6 + 2) * 4 + 3],
            &[255, 140, 50]
        );
    }

    #[test]
    fn opening_in_the_outline_lets_the_outside_in() {
        let m = masked(&img(&["#.#", "#.#", "###"]));
        assert_eq!(alpha(&m), "x x|x x|xxx");
    }

    #[test]
    fn draw_skips_transparent_white() {
        let mut f = Frame::new(4, 4);
        f.fill([10, 20, 30]);
        f.blit(&masked(&img(&["..", ".#"])), 0, 0);
        assert_eq!(f.pixel(0, 0), [10, 20, 30, 255]);
        assert_eq!(f.pixel(1, 1), [0, 0, 0, 255]);
    }
}
