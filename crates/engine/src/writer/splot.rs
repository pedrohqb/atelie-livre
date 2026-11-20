//! Palavras com forma (Splots) no documento: desenhar o `SPLE` do `.MAX`
//! (os glifos em polígonos, `docs/formats/sple.md`), montar um `SPLE` a
//! partir do que o Cavalete de palavras com forma produziu e voltar das
//! escolhas gravadas para o cavalete.
//!
//! O cavalete (`scenes::paint::words::shaped_easel`) desenha em índices de
//! paleta; no Writer a paleta é a das 16 cores do `CSCH` ([`palette`]), as
//! mesmas do texto, e os conjuntos da lista de cores são índices dela.

use crate::Image;
use crate::paint::shaped::{Inks, Shadow, ShapedWords, WordShape};
use crate::paint::stamp::Stamp;
use formats::palette::Rgb;
use formats::sple::{self, Choices, Fill, Glyph, NewSple, Sple, letter, style};

/// As 16 cores do `CSCH` (= `writer::doc::CSCH`), na ordem do recurso.
pub fn palette() -> Vec<Rgb> {
    crate::writer::doc::CSCH.to_vec()
}

/// Tintas do cavalete na paleta do [`palette`]: branco 0, preto 15 e os
/// cinzas #8c8c8c (13), #bebebe (12) e #646464 (14) das sombras.
pub const INKS: Inks = Inks {
    white: 0,
    black: 15,
    gray: 13,
    light: 12,
    dark: 14,
    pattern: [0; 8],
};

/// Maior tamanho de um Splot novo na página, em pontos ("Ana" sai com
/// 300 × 119; os verticais com 250 de altura).
pub const MAX_W: u32 = 300;
pub const MAX_H: u32 = 250;

/// Tamanho na página de uma imagem `w × h`: a largura de [`MAX_W`], ou a
/// altura de [`MAX_H`] se passar dela, na proporção da imagem.
pub fn page_size(w: u32, h: u32) -> (u16, u16) {
    let (w, h) = (w.max(1) as f32, h.max(1) as f32);
    let s = (MAX_W as f32 / w).min(MAX_H as f32 / h);
    (
        ((w * s).round() as u16).max(1),
        ((h * s).round() as u16).max(1),
    )
}

// ---------------------------------------------------------------
// Desenho

/// Desenha o Splot num quadro `w × h` (a caixa do cabeçalho esticada no
/// quadro): fundo transparente, glifos em ordem, par-ímpar.
pub fn draw(s: &Sple, w: u32, h: u32) -> Image {
    let (w, h) = (w.max(1), h.max(1));
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let [top, left, bottom, right] = s.bounds().map(f32::from);
    let sx = w as f32 / (right - left).max(1.0);
    let sy = h as f32 / (bottom - top).max(1.0);
    let to = |(x, y): (i16, i16)| ((f32::from(x) - left) * sx, (f32::from(y) - top) * sy);
    let mut put = |x: i32, y: i32, c: [u8; 3]| {
        if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
            let k = ((y as u32 * w + x as u32) * 4) as usize;
            rgba[k..k + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    };
    for g in &s.glyphs {
        let fg = sple::rgb8(g.color);
        let contours: Vec<Vec<(f32, f32)>> = g
            .contours
            .iter()
            .map(|c| c.iter().map(|&p| to(p)).collect())
            .collect();
        let ink = |x: i32, y: i32| match &g.fill {
            Fill::Pattern { back, pattern, .. } => {
                let bit = (pattern[y.rem_euclid(8) as usize] >> (7 - x.rem_euclid(8))) & 1;
                if bit == 1 { fg } else { sple::rgb8(*back) }
            }
            _ => fg,
        };
        fill_even_odd(&contours, h, |x, y| put(x, y, ink(x, y)));
        if g.fill == Fill::Framed {
            for c in &contours {
                for k in 0..c.len() {
                    let (a, b) = (c[k], c[(k + 1) % c.len()]);
                    line(a, b, |x, y| put(x, y, ink(x, y)));
                }
            }
        }
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
}

/// Pinta os pixels cujos centros caem dentro dos contornos (regra par-ímpar).
fn fill_even_odd(contours: &[Vec<(f32, f32)>], h: u32, mut put: impl FnMut(i32, i32)) {
    let mut xs: Vec<f32> = Vec::new();
    for y in 0..h as i32 {
        let yc = y as f32 + 0.5;
        xs.clear();
        for c in contours {
            for k in 0..c.len() {
                let (a, b) = (c[k], c[(k + 1) % c.len()]);
                if (a.1 <= yc) != (b.1 <= yc) {
                    xs.push(a.0 + (yc - a.1) * (b.0 - a.0) / (b.1 - a.1));
                }
            }
        }
        xs.sort_by(f32::total_cmp);
        for pair in xs.chunks_exact(2) {
            let x0 = (pair[0] - 0.5).ceil() as i32;
            let x1 = (pair[1] - 0.5).floor() as i32;
            for x in x0..=x1 {
                put(x, y);
            }
        }
    }
}

/// Reta de um pixel entre dois pontos.
fn line(a: (f32, f32), b: (f32, f32), mut put: impl FnMut(i32, i32)) {
    let n = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().max(1.0) as i32;
    for k in 0..=n {
        let t = k as f32 / n as f32;
        put(
            (a.0 + (b.0 - a.0) * t).floor() as i32,
            (a.1 + (b.1 - a.1) * t).floor() as i32,
        );
    }
}

// ---------------------------------------------------------------
// Montagem a partir do cavalete

/// Contornos (pelas bordas dos pixels, em unidades de pixel) das partes
/// marcadas em `mask` (`w × h`). Pintados com a regra par-ímpar dão de volta
/// exatamente a máscara.
pub fn trace(mask: &[bool], w: usize, h: usize) -> Vec<Vec<(i32, i32)>> {
    use std::collections::HashMap;
    let on = |x: i32, y: i32| {
        x >= 0
            && y >= 0
            && (x as usize) < w
            && (y as usize) < h
            && mask[y as usize * w + x as usize]
    };
    // arestas orientadas com a tinta à direita (y para baixo)
    let mut next: HashMap<(i32, i32), Vec<(i32, i32)>> = HashMap::new();
    let mut add = |a: (i32, i32), b: (i32, i32)| next.entry(a).or_default().push(b);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if !on(x, y) {
                continue;
            }
            if !on(x, y - 1) {
                add((x, y), (x + 1, y));
            }
            if !on(x + 1, y) {
                add((x + 1, y), (x + 1, y + 1));
            }
            if !on(x, y + 1) {
                add((x + 1, y + 1), (x, y + 1));
            }
            if !on(x - 1, y) {
                add((x, y + 1), (x, y));
            }
        }
    }
    let mut starts: Vec<(i32, i32)> = next.keys().copied().collect();
    starts.sort_by_key(|&(x, y)| (y, x));
    let mut out = Vec::new();
    for s in starts {
        while next.get(&s).is_some_and(|v| !v.is_empty()) {
            let mut loop_pts = vec![s];
            let mut at = s;
            while let Some(b) = next.get_mut(&at).and_then(Vec::pop) {
                if b == s {
                    break;
                }
                loop_pts.push(b);
                at = b;
            }
            out.push(simplify(loop_pts));
        }
    }
    out
}

/// Tira os pontos no meio de uma reta.
fn simplify(p: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let n = p.len();
    if n < 3 {
        return p;
    }
    (0..n)
        .filter(|&k| {
            let (a, b, c) = (p[(k + n - 1) % n], p[k], p[(k + 1) % n]);
            (b.0 - a.0) * (c.1 - b.1) != (b.1 - a.1) * (c.0 - b.0)
        })
        .map(|k| p[k])
        .collect()
}

/// Mais pontos que cabem num glifo (`i16` do total).
const MAX_POINTS: usize = 30_000;

/// Glifos da imagem do cavalete: um (ou mais, se for grande) de tipo 2 por
/// cor, em unidades de `k` por pixel. Devolve os glifos e a caixa.
pub fn glyphs_of(stamp: &Stamp, pal: &[Rgb]) -> (Vec<Glyph>, [i16; 4]) {
    let (w, h) = (stamp.w as usize, stamp.h as usize);
    let k = (i32::from(sple::SHAPE_UNITS) / (w.max(h).max(1) as i32)).max(1);
    let mut colors: Vec<u8> = stamp.px.iter().flatten().copied().collect();
    colors.sort_unstable();
    colors.dedup();
    let mut glyphs = Vec::new();
    for c in colors {
        let mask: Vec<bool> = stamp.px.iter().map(|p| *p == Some(c)).collect();
        let rgb = crate::ui::pal_rgb(pal, c);
        let mut group: Vec<Vec<(i16, i16)>> = Vec::new();
        let mut count = 0;
        for contour in trace(&mask, w, h) {
            if count + contour.len() > MAX_POINTS && !group.is_empty() {
                glyphs.push(Glyph::solid(sple::rgb48(rgb), std::mem::take(&mut group)));
                count = 0;
            }
            count += contour.len();
            group.push(
                contour
                    .into_iter()
                    .map(|(x, y)| ((x * k) as i16, (y * k) as i16))
                    .collect(),
            );
        }
        if !group.is_empty() {
            glyphs.push(Glyph::solid(sple::rgb48(rgb), group));
        }
    }
    let bounds = [0, 0, (h as i32 * k) as i16, (w as i32 * k) as i16];
    (glyphs, bounds)
}

/// Monta o `SPLE` do que o cavalete mostra: escolhas, forma (`scrd` = o
/// recurso `SCRD` inteiro da forma, `None` no retângulo), cores do conjunto
/// e o desenho de `stamp`.
pub fn build(
    words: &ShapedWords,
    stamp: &Stamp,
    pal: &[Rgb],
    color_sets: &[Vec<u8>],
    scrd: Option<&[u8]>,
) -> Sple {
    let (glyphs, bounds) = glyphs_of(stamp, pal);
    let set = color_sets.iter().position(|s| *s == words.colors);
    let colors: Vec<sple::Rgb48> = words
        .colors
        .iter()
        .map(|&c| sple::rgb48(crate::ui::pal_rgb(pal, c)))
        .collect();
    let font: Vec<u8> = words
        .font
        .chars()
        .filter_map(formats::cp1252::encode_char)
        .collect();
    let text: Vec<u8> = words
        .text
        .chars()
        .filter_map(formats::cp1252::encode_char)
        .collect();
    let (shape, shape_kind) = match scrd {
        Some(d) if d.len() > 4 => (sple::shape_record(d), d[0]),
        _ => (sple::rect_shape_record(), 0),
    };
    sple::build(&NewSple {
        font: &font,
        text: &text,
        choices: choices_of(words, set.unwrap_or(0)),
        shape,
        shape_kind,
        colors: &colors,
        bounds,
        glyphs,
    })
}

// ---------------------------------------------------------------
// Escolhas ↔ cavalete

/// Escolhas do cavalete, como o `SPLE` e o `TDRL` as guardam.
pub fn choices_of(w: &ShapedWords, color_set: usize) -> Choices {
    let index = |k: Option<usize>| k.unwrap_or(0) as u8;
    let mut st = 0;
    for (on, bit) in [
        (w.vertical, style::VERTICAL),
        (w.fill, style::FILL),
        (w.caps, style::CAPS),
        (w.flip_h, style::FLIP_H),
        (w.flip_v, style::FLIP_V),
    ] {
        if on {
            st |= bit;
        }
    }
    let mut ls = letter::FRAMED;
    if w.bold {
        ls |= letter::BOLD;
    }
    if w.italic {
        ls |= letter::ITALIC;
    }
    Choices {
        angle: w.angle as i16,
        shape: index(WordShape::ALL.iter().position(|s| *s == w.shape)),
        shadow: index(Shadow::ALL.iter().position(|s| *s == w.shadow)),
        pattern: w.pattern,
        color_set: color_set as u8,
        letter_style: ls,
        style: st,
    }
}

/// Aplica as escolhas `c` (de um `SPLE` ou de um "preferido") à palavra;
/// texto, fonte e tamanho ficam.
pub fn apply(w: &mut ShapedWords, c: &Choices, color_sets: &[Vec<u8>]) {
    w.angle = i32::from(c.angle).rem_euclid(360);
    w.shape = WordShape::ALL
        .get(c.shape as usize)
        .copied()
        .unwrap_or_default();
    w.shadow = Shadow::ALL
        .get(c.shadow as usize)
        .copied()
        .unwrap_or_default();
    w.pattern = c.pattern;
    if let Some(set) = color_sets.get(c.color_set as usize) {
        w.colors = set.clone();
    }
    w.bold = c.letter_style & letter::BOLD != 0;
    w.italic = c.letter_style & letter::ITALIC != 0;
    w.vertical = c.style & style::VERTICAL != 0;
    w.fill = c.style & style::FILL != 0;
    w.caps = c.style & style::CAPS != 0;
    w.flip_h = c.style & style::FLIP_H != 0;
    w.flip_v = c.style & style::FLIP_V != 0;
    // o vertical do original é gravado como giro de 270°; o port empilha
    // as letras sem girar
    if w.vertical && w.angle == 270 {
        w.angle = 0;
    }
}

/// A palavra que reabre o cavalete com o Splot `s`.
pub fn words_of(s: &Sple, color_sets: &[Vec<u8>]) -> ShapedWords {
    let mut w = ShapedWords::new(&formats::cp1252::decode(s.text()), 15);
    w.font = formats::cp1252::decode(s.font());
    apply(&mut w, &s.choices(), color_sets);
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(w: u32, h: u32, f: impl Fn(u32, u32) -> Option<u8>) -> Stamp {
        Stamp {
            w,
            h,
            px: (0..w * h).map(|k| f(k % w, k / w)).collect(),
        }
    }

    #[test]
    fn traced_contours_fill_back_to_the_mask() {
        // anel (com buraco), ponto solto e dois quadrados tocando na quina
        let s = stamp(12, 9, |x, y| {
            let ring = (1..6).contains(&x) && (1..6).contains(&y) && !(x == 3 && y == 3);
            let corner = (x == 8 && y == 7) || (x == 9 && y == 8);
            (ring || (x == 10 && y == 1) || corner).then_some(15)
        });
        let pal = palette();
        let (glyphs, bounds) = glyphs_of(&s, &pal);
        assert_eq!(glyphs.len(), 1);
        let sp = sple::build(&NewSple {
            font: b"",
            text: b"",
            choices: Choices::default(),
            shape: sple::rect_shape_record(),
            shape_kind: 0,
            colors: &[[0; 3]],
            bounds,
            glyphs,
        });
        let back = sple::read(&sple::write(&sp)).unwrap();
        let img = draw(&back, 12, 9);
        for y in 0..9 {
            for x in 0..12 {
                let a = img.rgba[((y * 12 + x) * 4 + 3) as usize];
                assert_eq!(a == 255, s.px[(y * 12 + x) as usize].is_some(), "({x},{y})");
            }
        }
    }

    #[test]
    fn colors_become_separate_glyphs() {
        let s = stamp(4, 2, |x, _| Some(if x < 2 { 5 } else { 9 }));
        let (g, _) = glyphs_of(&s, &palette());
        assert_eq!(g.len(), 2);
        assert_eq!(sple::rgb8(g[0].color), [0xfa, 0x00, 0x39]);
        assert_eq!(
            g[0].contours,
            vec![vec![(0, 0), (4000, 0), (4000, 4000), (0, 4000)]]
        );
    }

    #[test]
    fn simplify_keeps_corners_only() {
        let p = vec![(0, 0), (1, 0), (2, 0), (2, 1), (2, 2), (0, 2), (0, 1)];
        assert_eq!(simplify(p), vec![(0, 0), (2, 0), (2, 2), (0, 2)]);
    }

    #[test]
    fn choices_round_trip_through_the_words() {
        let sets: Vec<Vec<u8>> = vec![vec![15], vec![0], vec![5, 11]];
        let mut w = ShapedWords::new("Ana", 15);
        w.shape = WordShape::Heart;
        w.shadow = Shadow::Dusk;
        w.pattern = 3;
        w.colors = vec![5, 11];
        w.bold = true;
        w.caps = true;
        w.flip_v = true;
        w.angle = 340;
        let c = choices_of(&w, 2);
        assert_eq!((c.shape, c.shadow, c.color_set), (11, 3, 2));
        assert_eq!(c.letter_style, letter::FRAMED | letter::BOLD);
        assert_eq!(c.style, style::CAPS | style::FLIP_V);
        let mut back = ShapedWords::new("Ana", 15);
        apply(&mut back, &c, &sets);
        assert_eq!(back, w);
    }

    #[test]
    fn built_splot_keeps_text_font_and_choices() {
        let mut w = ShapedWords::new("Olá", 15);
        w.font = "Impact".into();
        w.shape = WordShape::Wave;
        let s = stamp(30, 10, |x, y| (x > 2 && y > 2).then_some(15));
        let sp = build(&w, &s, &palette(), &[vec![15]], Some(&[3, 0, 1, 0, 9, 9]));
        let back = sple::read(&sple::write(&sp)).unwrap();
        assert_eq!(back.text(), b"Ol\xe1");
        assert_eq!(back.prefix[0x21], 3);
        assert_eq!(back.shape_record(), &[9, 9]);
        let again = words_of(&back, &[vec![15]]);
        assert_eq!(
            (again.text.as_str(), again.font.as_str()),
            ("Olá", "Impact")
        );
        assert_eq!(again.shape, WordShape::Wave);
        assert_eq!(back.bounds(), [0, 0, 2660, 7980]);
    }

    #[test]
    fn page_size_fits_300_by_250() {
        assert_eq!(page_size(600, 238), (300, 119));
        assert_eq!(page_size(84, 250), (84, 250));
        assert_eq!(page_size(10, 100), (25, 250));
    }

    #[test]
    fn framed_and_pattern_glyphs_are_painted() {
        let sq = vec![vec![(0i16, 0i16), (8, 0), (8, 8), (0, 8)]];
        let mut g = Glyph::solid([0xffff, 0, 0], sq);
        g.tag = 0x1605;
        g.fill = Fill::Pattern {
            back: [0, 0, 0xffff],
            flag: 0xffff,
            pattern: [0xf0; 8],
        };
        let sp = sple::build(&NewSple {
            font: b"",
            text: b"",
            choices: Choices::default(),
            shape: sple::rect_shape_record(),
            shape_kind: 0,
            colors: &[[0; 3]],
            bounds: [0, 0, 8, 8],
            glyphs: vec![g],
        });
        let img = draw(&sp, 8, 8);
        assert_eq!(&img.rgba[0..4], &[255, 0, 0, 255]);
        assert_eq!(&img.rgba[7 * 4..7 * 4 + 4], &[0, 0, 255, 255]);
    }
}
