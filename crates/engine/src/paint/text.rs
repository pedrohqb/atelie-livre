//! Texto no quadro (ferramenta Palavra): glifos da fonte de bitmap "Pablo
//! Graphos" — ou da fonte 8×8 reserva, quando os dados não trazem a fonte —
//! plotados pixel a pixel, com o mesmo arranjo de `Frame::text_font`, e o
//! balão de fala branco com contorno preto.

use super::canvas::{BLACK, Canvas, WHITE};
use font8x8::{BASIC_FONTS, LATIN_FONTS, UnicodeFonts};
use formats::fnt::Font;

/// Altura (e largura) de cada caractere da fonte 8×8 reserva.
const FALLBACK: u32 = 8;
/// Folga entre o texto e o contorno do balão.
pub const BALLOON_PAD: i32 = 6;
/// Altura do rabicho do balão, abaixo do contorno.
const TAIL_H: i32 = 7;

/// Largura e altura do texto em pixels (altura da linha mesmo com texto vazio).
pub fn text_size(font: Option<&Font>, text: &str) -> (u32, u32) {
    match font {
        Some(f) => (f.width(text), f.height),
        None => (FALLBACK * text.chars().count() as u32, FALLBACK),
    }
}

/// Chama `put(dx, dy)` para cada pixel de tinta do texto, relativo ao canto
/// superior esquerdo da linha.
pub fn text_pixels(font: Option<&Font>, text: &str, mut put: impl FnMut(i32, i32)) {
    let mut gx = 0i32;
    match font {
        Some(f) => {
            for g in f.glyphs(text) {
                let w = g.width as usize;
                for (k, &on) in g.bits.iter().enumerate() {
                    if on && w > 0 {
                        put(gx + (k % w) as i32, (k / w) as i32);
                    }
                }
                gx += g.width as i32;
            }
        }
        None => {
            for c in text.chars() {
                let glyph = BASIC_FONTS
                    .get(c)
                    .or_else(|| LATIN_FONTS.get(c))
                    .or_else(|| BASIC_FONTS.get('?'))
                    .expect("glifo '?'");
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..8 {
                        if bits & (1 << col) != 0 {
                            put(gx + col, row as i32);
                        }
                    }
                }
                gx += FALLBACK as i32;
            }
        }
    }
}

/// Pixels do balão de fala em volta de um texto de `w`×`h` com canto em `(x, y)`:
/// `put(dx, dy, preto)` para cada pixel (preto = contorno; senão, interior branco).
/// O balão é o retângulo do texto com `BALLOON_PAD` de folga e um rabicho
/// apontando para baixo e para a esquerda.
pub fn balloon_pixels(x: i32, y: i32, w: u32, h: u32, mut put: impl FnMut(i32, i32, bool)) {
    let x0 = x - BALLOON_PAD;
    let y0 = y - BALLOON_PAD;
    let x1 = x + w as i32 + BALLOON_PAD; // exclusivo
    let y1 = y + h as i32 + BALLOON_PAD; // exclusivo
    // base do rabicho na borda de baixo: colunas [tail_l, tail_r]
    let tail_l = x0 + 10;
    let tail_r = (x0 + 17).min(x1 - 2);
    for yy in y0..y1 {
        for xx in x0..x1 {
            let edge = yy == y0 || yy == y1 - 1 || xx == x0 || xx == x1 - 1;
            let opening = yy == y1 - 1 && xx > tail_l && xx < tail_r;
            put(xx, yy, edge && !opening);
        }
    }
    // rabicho: afunila da base até a ponta em (x0 + 4, y1 + TAIL_H - 1)
    let tip = x0 + 4;
    for k in 0..TAIL_H {
        let t = (k + 1) as f32 / TAIL_H as f32;
        let l = tail_l as f32 + (tip as f32 - tail_l as f32) * t;
        let r = tail_r as f32 + (tip as f32 - tail_r as f32) * t;
        let (l, r) = (l.round() as i32, r.round() as i32);
        for xx in l..=r {
            put(xx, y1 + k, xx == l || xx == r);
        }
    }
}

/// Forma do balão de uma palavra (Cavalete de história em quadrinhos).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BalloonShape {
    /// Sem balão: só o texto.
    #[default]
    None,
    /// Retângulo com rabicho (o balão "Oi!" da pintura).
    RectTail,
    Rect,
    /// Oval com rabicho.
    OvalTail,
    Oval,
    /// Pensamento: oval com bolinhas.
    Thought,
}

impl BalloonShape {
    pub const ALL: [BalloonShape; 6] = [
        BalloonShape::RectTail,
        BalloonShape::Rect,
        BalloonShape::OvalTail,
        BalloonShape::Oval,
        BalloonShape::Thought,
        BalloonShape::None,
    ];

    /// Nome gravado no `*.objetos.json`.
    pub fn name(self) -> &'static str {
        match self {
            BalloonShape::None => "nenhum",
            BalloonShape::RectTail => "retangulo-rabicho",
            BalloonShape::Rect => "retangulo",
            BalloonShape::OvalTail => "oval-rabicho",
            BalloonShape::Oval => "oval",
            BalloonShape::Thought => "pensamento",
        }
    }

    pub fn from_name(s: &str) -> Option<BalloonShape> {
        BalloonShape::ALL.into_iter().find(|b| b.name() == s)
    }

    fn is_oval(self) -> bool {
        matches!(
            self,
            BalloonShape::OvalTail | BalloonShape::Oval | BalloonShape::Thought
        )
    }
}

/// Posição das palavras no balão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

impl Align {
    pub const ALL: [Align; 3] = [Align::Left, Align::Center, Align::Right];

    pub fn name(self) -> &'static str {
        match self {
            Align::Left => "esquerda",
            Align::Center => "centro",
            Align::Right => "direita",
        }
    }

    pub fn from_name(s: &str) -> Option<Align> {
        Align::ALL.into_iter().find(|a| a.name() == s)
    }
}

/// Estilo de uma palavra: forma do balão, posição do texto, largura (0 =
/// uma linha só; senão o texto quebra nessa largura) e fonte pequena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WordsStyle {
    pub shape: BalloonShape,
    pub align: Align,
    pub width: u32,
    pub small: bool,
}

impl WordsStyle {
    /// Texto solto, como a opção 1 da Palavra.
    pub fn plain() -> WordsStyle {
        WordsStyle::default()
    }

    /// Balão "Oi!" da pintura.
    pub fn balloon() -> WordsStyle {
        WordsStyle {
            shape: BalloonShape::RectTail,
            ..WordsStyle::default()
        }
    }

    pub fn has_balloon(&self) -> bool {
        self.shape != BalloonShape::None
    }
}

/// Menor largura de quebra, em pixels.
pub const MIN_WRAP: u32 = 24;

/// Quebra `text` em linhas de até `width` pixels, nas palavras; uma palavra
/// maior que a largura fica sozinha na linha. `width` 0 não quebra.
pub fn wrap_words(font: Option<&Font>, text: &str, width: u32) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if line.is_empty() || text_size(font, &candidate).0 <= width {
            line = candidate;
        } else {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    lines.push(line);
    lines
}

/// Grade do balão em volta de um bloco de texto `tw`×`th`: o bloco fica em
/// `(pad_x, pad_y)` de uma imagem `w`×`h`; cada pixel é `None` (fora),
/// `Some(true)` (contorno preto) ou `Some(false)` (interior branco).
pub struct BalloonGrid {
    pub w: i32,
    pub h: i32,
    pub pad_x: i32,
    pub pad_y: i32,
    pub px: Vec<Option<bool>>,
}

impl BalloonGrid {
    pub fn at(&self, x: i32, y: i32) -> Option<bool> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        self.px[(y * self.w + x) as usize]
    }
}

/// Altura do rabicho das formas ovais e das bolinhas do pensamento.
const OVAL_TAIL_H: i32 = 9;
const THOUGHT_H: i32 = 13;

pub fn balloon_grid(shape: BalloonShape, tw: u32, th: u32) -> BalloonGrid {
    let (tw, th) = (tw.max(1) as i32, th.max(1) as i32);
    let blank = |w: i32, h: i32, pad_x: i32, pad_y: i32| BalloonGrid {
        w,
        h,
        pad_x,
        pad_y,
        px: vec![None; (w * h) as usize],
    };
    match shape {
        BalloonShape::None => blank(tw, th, 0, 0),
        BalloonShape::RectTail | BalloonShape::Rect => {
            let pad = BALLOON_PAD;
            let tail = if shape == BalloonShape::RectTail {
                TAIL_H + 1
            } else {
                0
            };
            let mut g = blank(tw + 2 * pad, th + 2 * pad + tail, pad, pad);
            let w = g.w;
            let mut put = |x: i32, y: i32, edge: bool| {
                if x >= 0 && y >= 0 && x < w && y < g.h {
                    g.px[(y * w + x) as usize] = Some(edge);
                }
            };
            if shape == BalloonShape::RectTail {
                balloon_pixels(pad, pad, tw as u32, th as u32, &mut put);
            } else {
                for y in 0..th + 2 * pad {
                    for x in 0..tw + 2 * pad {
                        let edge =
                            x == 0 || y == 0 || x == tw + 2 * pad - 1 || y == th + 2 * pad - 1;
                        put(x, y, edge);
                    }
                }
            }
            g
        }
        _ => oval_grid(shape, tw, th),
    }
}

/// Oval (com rabicho ou bolinhas de pensamento) que contém o bloco de texto.
fn oval_grid(shape: BalloonShape, tw: i32, th: i32) -> BalloonGrid {
    debug_assert!(shape.is_oval());
    // semieixos: os cantos do bloco ficam dentro ((1/1.3)² + (1/1.6)² < 1)
    let a = tw as f32 / 2.0 * 1.3 + 6.0;
    let b = th as f32 / 2.0 * 1.6 + 5.0;
    let pad_x = (a - tw as f32 / 2.0).ceil() as i32;
    let pad_y = (b - th as f32 / 2.0).ceil() as i32;
    let body_w = tw + 2 * pad_x;
    let body_h = th + 2 * pad_y;
    let extra = match shape {
        BalloonShape::OvalTail => OVAL_TAIL_H,
        BalloonShape::Thought => THOUGHT_H,
        _ => 0,
    };
    let (w, h) = (body_w, body_h + extra);
    let (cx, cy) = (body_w as f32 / 2.0, body_h as f32 / 2.0);
    let ell = |x: i32, y: i32, a: f32, b: f32| {
        let dx = (x as f32 + 0.5 - cx) / a;
        let dy = (y as f32 + 0.5 - cy) / b;
        dx * dx + dy * dy <= 1.0
    };
    // rabicho: triângulo da base (na borda de baixo da oval) à ponta
    let tri = {
        let x1 = cx - a * 0.45;
        let x2 = cx - a * 0.2;
        let yb = |x: f32| cy + b * (1.0 - ((x - cx) / a).powi(2)).max(0.0).sqrt();
        let p1 = (x1, yb(x1) - 2.0);
        let p2 = (x2, yb(x2) - 2.0);
        let tip = (cx - a * 0.75, (body_h + OVAL_TAIL_H) as f32 - 0.5);
        move |x: i32, y: i32| {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let side = |u: (f32, f32), v: (f32, f32)| {
                (v.0 - u.0) * (p.1 - u.1) - (v.1 - u.1) * (p.0 - u.0)
            };
            let (s1, s2, s3) = (side(p1, p2), side(p2, tip), side(tip, p1));
            (s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0) || (s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0)
        }
    };
    let bubbles = [
        ((cx - a * 0.55, body_h as f32 + 3.5), 3.5f32),
        ((cx - a * 0.72, body_h as f32 + 10.0), 2.5f32),
    ];
    let in_bubble = |x: i32, y: i32, shrink: f32| {
        bubbles.iter().any(|&((bx, by), r)| {
            let dx = x as f32 + 0.5 - bx;
            let dy = y as f32 + 0.5 - by;
            dx * dx + dy * dy <= (r - shrink).max(0.0).powi(2)
        })
    };
    let mut px = vec![None; (w * h) as usize];
    let in_tri = |x: i32, y: i32| shape == BalloonShape::OvalTail && tri(x, y);
    for y in 0..h {
        for x in 0..w {
            let body = ell(x, y, a, b);
            let core = ell(x, y, a - 1.5, b - 1.5);
            let t = in_tri(x, y);
            let t_edge = t
                && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|&(dx, dy)| !in_tri(x + dx, y + dy));
            let bub = shape == BalloonShape::Thought && in_bubble(x, y, 0.0);
            let bub_core = bub && in_bubble(x, y, 1.2);
            let inside = body || t || bub;
            if !inside {
                continue;
            }
            let edge = (body && !core && !(t && !t_edge))
                || (t && t_edge && !core)
                || (bub && !bub_core && !core);
            px[(y * w + x) as usize] = Some(edge);
        }
    }
    BalloonGrid {
        w,
        h,
        pad_x,
        pad_y,
        px,
    }
}

/// Leiaute de uma palavra: linhas (texto e deslocamento x no bloco), tamanho
/// do bloco de texto, grade do balão e posição do cursor (fim da última linha).
pub struct WordsLayout {
    pub lines: Vec<(String, i32)>,
    pub line_h: i32,
    pub tw: i32,
    pub th: i32,
    pub grid: BalloonGrid,
    /// Canto do cursor de texto, relativo à imagem.
    pub caret: (i32, i32),
}

pub fn words_layout(font: Option<&Font>, text: &str, style: &WordsStyle) -> WordsLayout {
    let lines = wrap_words(font, text, style.width);
    let line_h = text_size(font, "").1 as i32;
    let widths: Vec<i32> = lines.iter().map(|l| text_size(font, l).0 as i32).collect();
    let tw = widths.iter().copied().max().unwrap_or(0).max(1);
    let th = line_h * lines.len() as i32;
    let lines: Vec<(String, i32)> = lines
        .into_iter()
        .zip(&widths)
        .map(|(l, &lw)| {
            let dx = match style.align {
                Align::Left => 0,
                Align::Center => (tw - lw) / 2,
                Align::Right => tw - lw,
            };
            (l, dx)
        })
        .collect();
    let grid = balloon_grid(style.shape, tw as u32, th as u32);
    let last = lines.len() - 1;
    let caret = (
        grid.pad_x + lines[last].1 + widths[last],
        grid.pad_y + last as i32 * line_h,
    );
    WordsLayout {
        lines,
        line_h,
        tw,
        th,
        grid,
        caret,
    }
}

impl Canvas {
    /// Escreve `text` com o canto superior esquerdo em `(x, y)` (coordenadas do
    /// quadro), na cor sólida `color`, recortado ao quadro.
    pub fn draw_text(&mut self, font: Option<&Font>, x: i32, y: i32, text: &str, color: u8) {
        text_pixels(font, text, |dx, dy| self.set(x + dx, y + dy, color));
    }

    /// Balão de fala branco com contorno preto em volta do texto de `text_size`.
    pub fn draw_balloon(&mut self, font: Option<&Font>, x: i32, y: i32, text: &str) {
        let (w, h) = text_size(font, text);
        balloon_pixels(x, y, w, h, |xx, yy, edge| {
            self.set(xx, yy, if edge { BLACK } else { WHITE })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::canvas::WHITE;

    fn count(c: &Canvas, v: u8) -> usize {
        c.px.iter().filter(|&&p| p == v).count()
    }

    #[test]
    fn fallback_text_size_and_pixels() {
        assert_eq!(text_size(None, "Oi"), (16, 8));
        assert_eq!(text_size(None, ""), (0, 8));
        let mut c = Canvas::new(40, 20);
        c.draw_text(None, 2, 3, "A", 5);
        let n = count(&c, 5);
        assert!(n > 0);
        // todos dentro da caixa 8×8 a partir de (2, 3)
        for (k, &p) in c.px.iter().enumerate() {
            if p == 5 {
                let (x, y) = ((k % 40) as i32, (k / 40) as i32);
                assert!((2..10).contains(&x) && (3..11).contains(&y), "({x}, {y})");
            }
        }
    }

    #[test]
    fn text_is_clipped_to_canvas() {
        let mut c = Canvas::new(10, 10);
        c.draw_text(None, -4, -4, "MM", 3);
        c.draw_text(None, 8, 8, "MM", 3);
        c.draw_text(None, 100, 100, "MM", 3);
        assert!(count(&c, 3) > 0);
        assert_eq!(c.px.len(), 100);
    }

    fn count_grid(g: &BalloonGrid, v: Option<bool>) -> usize {
        g.px.iter().filter(|&&p| p == v).count()
    }

    #[test]
    fn wrap_breaks_at_words_and_keeps_long_words() {
        assert_eq!(wrap_words(None, "oi tudo bem", 0), vec!["oi tudo bem"]);
        // 8 px por letra: 10 letras = 80 px
        assert_eq!(
            wrap_words(None, "oi tudo bem com você", 80),
            vec!["oi tudo", "bem com", "você"]
        );
        assert_eq!(
            wrap_words(None, "palavrascompridas x", 40),
            vec!["palavrascompridas", "x"]
        );
        assert_eq!(wrap_words(None, "", 40), vec![""]);
    }

    #[test]
    fn grids_contain_the_text_block_and_have_outlines() {
        for shape in BalloonShape::ALL {
            let g = balloon_grid(shape, 40, 8);
            assert!(
                g.w >= 40 + 2 * g.pad_x && g.h >= 8 + 2 * g.pad_y,
                "{shape:?}"
            );
            if shape == BalloonShape::None {
                assert_eq!(count_grid(&g, None), g.px.len());
                continue;
            }
            // o bloco de texto é todo interior branco
            for y in g.pad_y..g.pad_y + 8 {
                for x in g.pad_x..g.pad_x + 40 {
                    assert_eq!(g.at(x, y), Some(false), "{shape:?} ({x},{y})");
                }
            }
            assert!(count_grid(&g, Some(true)) > 60, "{shape:?}");
            // o canto da imagem das ovais fica de fora
            if shape.is_oval() {
                assert_eq!(g.at(0, 0), None);
            }
        }
        // o rabicho e as bolinhas ficam abaixo do corpo
        let tail = balloon_grid(BalloonShape::OvalTail, 40, 8);
        let plain = balloon_grid(BalloonShape::Oval, 40, 8);
        assert_eq!(tail.h, plain.h + OVAL_TAIL_H);
        assert!((0..tail.w).any(|x| tail.at(x, tail.h - 3) == Some(true)));
        assert_eq!(tail.at(tail.w - 1, tail.h - 1), None);
        let th = balloon_grid(BalloonShape::Thought, 40, 8);
        assert_eq!(th.h, plain.h + THOUGHT_H);
        assert!((0..th.w).any(|x| th.at(x, th.h - 2).is_some()));
        // retângulo sem rabicho: altura só do bloco e das folgas
        let r = balloon_grid(BalloonShape::Rect, 40, 8);
        assert_eq!((r.w, r.h), (40 + 2 * BALLOON_PAD, 8 + 2 * BALLOON_PAD));
        assert_eq!(r.at(0, 0), Some(true));
        assert_eq!(r.at(r.w - 1, r.h - 1), Some(true));
    }

    #[test]
    fn layout_aligns_lines_and_places_caret() {
        let style = WordsStyle {
            shape: BalloonShape::Rect,
            align: Align::Right,
            width: 80,
            small: false,
        };
        let l = words_layout(None, "oi tudo bem", &style);
        assert_eq!(l.lines.len(), 2);
        assert_eq!(l.tw, 56);
        assert_eq!(l.lines[1], ("bem".to_string(), 56 - 24));
        assert_eq!(l.caret, (BALLOON_PAD + 56, BALLOON_PAD + 8));
        let c = words_layout(None, "ab", &WordsStyle::plain());
        assert_eq!(c.caret, (16, 0));
        assert_eq!((c.grid.w, c.grid.h), (16, 8));
        let e = words_layout(None, "", &WordsStyle::balloon());
        assert_eq!(e.caret, (BALLOON_PAD, BALLOON_PAD));
        assert_eq!(
            BalloonShape::from_name("pensamento"),
            Some(BalloonShape::Thought)
        );
        assert_eq!(Align::from_name("centro"), Some(Align::Center));
    }

    #[test]
    fn balloon_is_white_inside_black_outline_with_tail() {
        let mut c = Canvas::new(60, 40);
        c.px.fill(7);
        c.draw_balloon(None, 10, 10, "Oi");
        // texto 16×8 em (10,10): balão de (4,4) a (32,24) exclusivos
        assert_eq!(c.get(4, 4), Some(BLACK));
        assert_eq!(c.get(31, 23), Some(BLACK));
        assert_eq!(c.get(31, 4), Some(BLACK));
        assert_eq!(c.get(5, 5), Some(WHITE));
        assert_eq!(c.get(9, 9), Some(WHITE));
        assert_eq!(c.get(30, 22), Some(WHITE));
        assert_eq!(c.get(3, 3), Some(7));
        assert_eq!(c.get(32, 4), Some(7));
        // abertura do rabicho na borda de baixo e ponta mais abaixo, à esquerda
        assert_eq!(c.get(15, 23), Some(WHITE));
        assert_eq!(c.get(14, 23), Some(BLACK));
        assert_eq!(c.get(15, 24), Some(WHITE));
        assert_eq!(c.get(8, 30), Some(BLACK));
        assert_eq!(c.get(8, 31), Some(7));
        assert_eq!(c.get(31, 30), Some(7));
    }
}
