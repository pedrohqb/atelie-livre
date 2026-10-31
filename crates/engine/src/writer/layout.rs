//! Diagramação do documento: quebra de linha automática na largura da
//! página, alinhamento, paginação (uma linha nunca fica entre duas páginas)
//! e as caixas de cada caractere e adesivo, de onde saem o cursor, a
//! navegação por linhas e o desenho. Os glifos vêm da "Pablo Graphos"
//! (6, 8 ou 16 pontos, ampliada por vizinho mais próximo nos tamanhos
//! grandes) ou da 8×8 reserva.

use super::doc::{Align, Cursor, Document, Item, Size, Style};
use font8x8::{BASIC_FONTS, LATIN_FONTS, UnicodeFonts};
use formats::fnt::Font;
use std::rc::Rc;

/// Fontes disponíveis; `None` em todas usa a 8×8 reserva.
#[derive(Debug, Clone, Default)]
pub struct FontSet {
    pub tiny: Option<Rc<Font>>,
    pub small: Option<Rc<Font>>,
    pub medium: Option<Rc<Font>>,
}

/// Glifo pronto para ser desenhado: largura de avanço, altura, ascendente e
/// os pixels de tinta (já ampliados), relativos ao topo da caixa do glifo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph {
    pub advance: i32,
    pub height: i32,
    pub ascent: i32,
    pub pixels: Vec<(i32, i32)>,
}

impl FontSet {
    /// Fonte e fator de ampliação para o tamanho; `None` = 8×8 reserva.
    fn face(&self, size: Size) -> (Option<&Font>, u32) {
        let font = match size {
            Size::Tiny => self.tiny.as_deref().or(self.small.as_deref()),
            Size::Small => self.small.as_deref(),
            _ => self.medium.as_deref(),
        };
        match font {
            Some(f) => (Some(f), size.scale()),
            None => (None, size.fallback_scale()),
        }
    }

    /// Altura da linha e ascendente (posição da base) do tamanho.
    pub fn metrics(&self, size: Size) -> (i32, i32) {
        match self.face(size) {
            (Some(f), s) => ((f.height * s) as i32, (f.ascent * s) as i32),
            (None, s) => ((8 * s) as i32, (8 * s) as i32),
        }
    }

    /// Largura de avanço do caractere no estilo (o negrito ocupa 1 px a mais).
    pub fn advance(&self, c: char, style: &Style) -> i32 {
        let base = match self.face(style.size) {
            (Some(f), s) => (f.width(&c.to_string()) * s) as i32,
            (None, s) => (8 * s) as i32,
        };
        base + i32::from(style.bold)
    }

    /// Pixels do caractere no estilo: ampliação, negrito (deslocado 1 px à
    /// direita também), inclinado (1 px a cada 4 linhas) e sublinhado.
    pub fn glyph(&self, c: char, style: &Style) -> Glyph {
        let (font, scale) = self.face(style.size);
        let s = scale as i32;
        let (height, ascent) = self.metrics(style.size);
        let advance = self.advance(c, style);
        let mut on: Vec<(i32, i32)> = Vec::new();
        match font {
            Some(f) => {
                let g = f.glyph(formats::cp1252::encode_char(c).unwrap_or(b'?'));
                let w = g.width as usize;
                for (k, &bit) in g.bits.iter().enumerate() {
                    if bit && w > 0 {
                        on.push(((k % w) as i32, (k / w) as i32));
                    }
                }
            }
            None => {
                let glyph = BASIC_FONTS
                    .get(c)
                    .or_else(|| LATIN_FONTS.get(c))
                    .or_else(|| BASIC_FONTS.get('?'))
                    .expect("glifo '?'");
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..8 {
                        if bits & (1 << col) != 0 {
                            on.push((col, row as i32));
                        }
                    }
                }
            }
        }
        let mut pixels = Vec::with_capacity(on.len() * (s * s) as usize);
        for (gx, gy) in on {
            for dy in 0..s {
                for dx in 0..s {
                    let (x, y) = (gx * s + dx, gy * s + dy);
                    let shear = if style.italic {
                        (height - 1 - y) / 4
                    } else {
                        0
                    };
                    pixels.push((x + shear, y));
                    if style.bold {
                        pixels.push((x + shear + 1, y));
                    }
                }
            }
        }
        if style.underline {
            let y = (ascent + 1).min(height - 1);
            pixels.extend((0..advance).map(|x| (x, y)));
        }
        pixels.sort_unstable();
        pixels.dedup();
        Glyph {
            advance,
            height,
            ascent,
            pixels,
        }
    }
}

/// Caixa de um item na linha: `x` relativo à margem esquerda da página.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemBox {
    pub item: usize,
    pub x: i32,
    pub w: i32,
    pub height: i32,
    pub ascent: i32,
    pub space: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub para: usize,
    /// Itens `start..end` do parágrafo.
    pub start: usize,
    pub end: usize,
    pub page: usize,
    /// Topo da linha, relativo ao topo da área de texto da página.
    pub y: i32,
    pub height: i32,
    pub ascent: i32,
    /// Começo da linha (após o alinhamento), para linhas vazias.
    pub x0: i32,
    pub boxes: Vec<ItemBox>,
    pub last_in_para: bool,
}

impl Line {
    /// Posição x do cursor em `index` (entre `start` e `end`).
    pub fn cursor_x(&self, index: usize) -> i32 {
        if index <= self.start {
            return self.x0;
        }
        match self.boxes.get(index - self.start - 1) {
            Some(b) => b.x + b.w,
            None => self.boxes.last().map_or(self.x0, |b| b.x + b.w),
        }
    }

    /// Índice (entre `start` e `end`) mais perto de `x`.
    pub fn index_at(&self, x: i32) -> usize {
        let mut best = (self.start, (x - self.x0).abs());
        for (k, b) in self.boxes.iter().enumerate() {
            let d = (x - (b.x + b.w)).abs();
            if d < best.1 {
                best = (self.start + k + 1, d);
            }
        }
        best.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub lines: Vec<Line>,
    pub pages: usize,
    pub width: i32,
    pub page_h: i32,
}

/// Linha em construção.
struct Pending {
    start: usize,
    boxes: Vec<ItemBox>,
    /// Quantidade de caixas até o último espaço, inclusive (onde se pode quebrar).
    break_at: Option<usize>,
}

impl Pending {
    fn new(start: usize) -> Pending {
        Pending {
            start,
            boxes: Vec::new(),
            break_at: None,
        }
    }
}

impl Layout {
    /// Diagrama `doc` em linhas de até `width` pixels e páginas de `page_h`.
    pub fn build(doc: &Document, fonts: &FontSet, width: i32, page_h: i32) -> Layout {
        let mut lines = Vec::new();
        let default = fonts.metrics(doc.style.size);
        for (pi, para) in doc.paras.iter().enumerate() {
            let mut cur = Pending::new(0);
            let mut k = 0;
            while k < para.items.len() {
                let item = &para.items[k];
                let (w, height, ascent) = match item {
                    Item::Char(c, st) => {
                        let (h, a) = fonts.metrics(st.size);
                        (fonts.advance(*c, st), h, a)
                    }
                    Item::Sticker(s) => (s.w as i32, s.h as i32, s.h as i32),
                };
                let x = cur.boxes.last().map_or(0, |b| b.x + b.w);
                if x + w > width && !cur.boxes.is_empty() && !item.is_space() {
                    // não cabe: quebra depois do último espaço, senão antes deste item
                    let cut = cur.break_at.unwrap_or(cur.boxes.len());
                    let rest = cur.boxes.split_off(cut);
                    let end = cur.start + cut;
                    let done = std::mem::replace(&mut cur, Pending::new(end));
                    lines.push(finish(pi, para.align, done, end, false, default, width));
                    // os itens depois do corte voltam para a nova linha, realinhados
                    let mut x = 0;
                    for mut b in rest {
                        b.x = x;
                        x += b.w;
                        cur.boxes.push(b);
                    }
                    continue;
                }
                cur.boxes.push(ItemBox {
                    item: k,
                    x,
                    w,
                    height,
                    ascent,
                    space: item.is_space(),
                });
                if item.is_space() {
                    cur.break_at = Some(cur.boxes.len());
                }
                k += 1;
            }
            let end = para.items.len();
            lines.push(finish(pi, para.align, cur, end, true, default, width));
        }
        // paginação
        let (mut page, mut y) = (0usize, 0i32);
        for l in &mut lines {
            if y > 0 && y + l.height > page_h {
                page += 1;
                y = 0;
            }
            l.page = page;
            l.y = y;
            y += l.height;
        }
        Layout {
            lines,
            pages: page + 1,
            width,
            page_h,
        }
    }

    /// Linha onde está o cursor.
    pub fn line_of(&self, cursor: Cursor) -> usize {
        let mut last = 0;
        for (k, l) in self.lines.iter().enumerate() {
            if l.para != cursor.para {
                continue;
            }
            last = k;
            if cursor.index < l.end || (cursor.index == l.end && l.last_in_para) {
                return k;
            }
        }
        last
    }

    /// (linha, x) do cursor.
    pub fn cursor_pos(&self, cursor: Cursor) -> (usize, i32) {
        let k = self.line_of(cursor);
        (k, self.lines[k].cursor_x(cursor.index))
    }

    /// Cursor na linha `k`, o mais perto possível de `x`.
    pub fn cursor_in_line(&self, k: usize, x: i32) -> Cursor {
        let l = &self.lines[k];
        Cursor {
            para: l.para,
            index: l.index_at(x),
        }
    }

    /// Cursor uma linha acima ou abaixo (`delta` = -1 ou 1), mantendo o x.
    pub fn cursor_vertical(&self, cursor: Cursor, delta: i32) -> Option<Cursor> {
        let (k, x) = self.cursor_pos(cursor);
        let target = k as i32 + delta;
        if target < 0 || target as usize >= self.lines.len() {
            return None;
        }
        Some(self.cursor_in_line(target as usize, x))
    }

    /// Linha da página `page` mais próxima de `y` (relativo à área de texto).
    pub fn line_at(&self, page: usize, y: i32) -> Option<usize> {
        let mut best: Option<(usize, i32)> = None;
        for (k, l) in self.lines.iter().enumerate() {
            if l.page != page {
                continue;
            }
            let d = if y < l.y {
                l.y - y
            } else if y >= l.y + l.height {
                y - (l.y + l.height) + 1
            } else {
                0
            };
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((k, d));
            }
        }
        best.map(|(k, _)| k)
    }
}

/// Fecha a linha: altura, ascendente e alinhamento na largura `width`.
fn finish(
    para: usize,
    align: Align,
    p: Pending,
    end: usize,
    last_in_para: bool,
    default: (i32, i32),
    width: i32,
) -> Line {
    let mut boxes = p.boxes;
    let (height, ascent) = if boxes.is_empty() {
        default
    } else {
        let ascent = boxes.iter().map(|b| b.ascent).max().unwrap_or(0);
        let descent = boxes.iter().map(|b| b.height - b.ascent).max().unwrap_or(0);
        (ascent + descent, ascent)
    };
    // os espaços finais ficam fora do alinhamento
    let content = boxes.iter().rposition(|b| !b.space).map_or(0, |k| k + 1);
    let content_w = boxes.get(content.wrapping_sub(1)).map_or(0, |b| b.x + b.w);
    let slack = (width - content_w).max(0);
    let mut x0 = 0;
    match align {
        Align::Left => {}
        Align::Right => x0 = slack,
        Align::Center => x0 = slack / 2,
        Align::Justify => {
            let spaces = boxes[..content].iter().filter(|b| b.space).count() as i32;
            if !last_in_para && spaces > 0 {
                let (each, mut extra) = (slack / spaces, slack % spaces);
                let mut shift = 0;
                for b in &mut boxes[..content] {
                    b.x += shift;
                    if b.space {
                        let add = each + i32::from(extra > 0);
                        extra -= i32::from(extra > 0);
                        b.w += add;
                        shift += add;
                    }
                }
                for b in &mut boxes[content..] {
                    b.x += shift;
                }
            }
        }
    }
    for b in &mut boxes {
        b.x += x0;
    }
    Line {
        para,
        start: p.start,
        end,
        page: 0,
        y: 0,
        height,
        ascent,
        x0,
        boxes,
        last_in_para,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::Sticker;

    fn doc(text: &str) -> Document {
        let mut d = Document::new();
        for (k, p) in text.split('\n').enumerate() {
            if k > 0 {
                d.newline();
            }
            d.insert_text(p);
        }
        d
    }

    fn fb() -> FontSet {
        FontSet::default()
    }

    #[test]
    fn fallback_metrics_scale_with_size() {
        let f = fb();
        assert_eq!(f.metrics(Size::Medium), (16, 16));
        assert_eq!(f.metrics(Size::Giant), (64, 64));
        assert_eq!(f.metrics(Size::Tiny), (8, 8));
        let st = Style::default();
        assert_eq!(f.advance('a', &st), 16);
        let bold = Style { bold: true, ..st };
        assert_eq!(f.advance('a', &bold), 17);
        let g = f.glyph('a', &st);
        assert_eq!((g.advance, g.height, g.ascent), (16, 16, 16));
        assert!(!g.pixels.is_empty());
        assert!(
            g.pixels
                .iter()
                .all(|&(x, y)| (0..16).contains(&x) && (0..16).contains(&y))
        );
    }

    #[test]
    fn styles_change_pixels() {
        let f = fb();
        let st = Style::default();
        let plain = f.glyph('H', &st);
        let bold = f.glyph('H', &Style { bold: true, ..st });
        assert!(bold.pixels.len() > plain.pixels.len());
        let italic = f.glyph('H', &Style { italic: true, ..st });
        // o topo é deslocado para a direita; a base, não
        let top = |g: &Glyph| g.pixels.iter().filter(|p| p.1 == 0).map(|p| p.0).min();
        assert_eq!(top(&italic).unwrap(), top(&plain).unwrap() + 3);
        let under = f.glyph(
            ' ',
            &Style {
                underline: true,
                ..st
            },
        );
        assert_eq!(under.pixels.len(), 16);
        assert!(under.pixels.iter().all(|p| p.1 == 15));
    }

    #[test]
    fn wraps_at_spaces_and_breaks_long_words() {
        // 16 px por letra, 72 px de largura = 4 letras
        let l = Layout::build(&doc("ab cd ef"), &fb(), 72, 1000);
        let texts: Vec<(usize, usize)> = l.lines.iter().map(|l| (l.start, l.end)).collect();
        assert_eq!(texts, [(0, 3), (3, 6), (6, 8)]);
        assert!(l.lines[0].boxes[2].space);
        assert_eq!(l.lines[1].x0, 0);
        assert_eq!(l.lines[1].boxes[0].x, 0);
        let long = Layout::build(&doc("abcdefgh"), &fb(), 72, 1000);
        let texts: Vec<(usize, usize)> = long.lines.iter().map(|l| (l.start, l.end)).collect();
        assert_eq!(texts, [(0, 4), (4, 8)]);
        // uma letra mais larga que a página ainda ocupa uma linha
        let huge = Layout::build(&doc("ab"), &fb(), 10, 1000);
        assert_eq!(huge.lines.len(), 2);
    }

    #[test]
    fn empty_paragraphs_have_height_and_pages_break_between_lines() {
        let l = Layout::build(&doc("a\n\nb"), &fb(), 100, 40);
        assert_eq!(l.lines.len(), 3);
        assert!(l.lines.iter().all(|l| l.height == 16));
        assert_eq!(
            l.lines.iter().map(|l| (l.page, l.y)).collect::<Vec<_>>(),
            [(0, 0), (0, 16), (1, 0)]
        );
        assert_eq!(l.pages, 2);
    }

    #[test]
    fn alignment_shifts_lines() {
        let mut d = doc("ab ");
        d.set_align(Align::Right);
        let l = Layout::build(&d, &fb(), 100, 1000);
        // o espaço final não conta: 100 - 32 = 68
        assert_eq!(l.lines[0].boxes[0].x, 68);
        assert_eq!(l.lines[0].x0, 68);
        d.set_align(Align::Center);
        let l = Layout::build(&d, &fb(), 100, 1000);
        assert_eq!(l.lines[0].boxes[0].x, 34);
        let mut j = doc("a b c d");
        j.set_align(Align::Justify);
        let l = Layout::build(&j, &fb(), 100, 1000);
        // "a b c" (80 px) na primeira linha, esticada a 100; "d" na última, sem esticar
        assert_eq!(l.lines.len(), 2);
        let last = l.lines[0].boxes.iter().filter(|b| !b.space).last().unwrap();
        assert_eq!(last.x + last.w, 100);
        assert_eq!(l.lines[1].boxes[0].x, 0);
    }

    #[test]
    fn stickers_sit_on_the_baseline() {
        let mut d = Document::new();
        d.insert_char('a');
        d.insert_sticker(Sticker {
            roll: 1,
            index: 0,
            w: 20,
            h: 40,
        });
        let l = Layout::build(&d, &fb(), 200, 1000);
        let line = &l.lines[0];
        assert_eq!((line.height, line.ascent), (40, 40));
        assert_eq!(line.boxes[1].w, 20);
        assert_eq!(line.cursor_x(2), 36);
    }

    #[test]
    fn cursor_navigation() {
        let l = Layout::build(&doc("ab cd\nx"), &fb(), 48, 1000);
        // "ab " / "cd" / "x"
        assert_eq!(l.lines.len(), 3);
        let c = Cursor { para: 0, index: 1 };
        assert_eq!(l.cursor_pos(c), (0, 16));
        assert_eq!(l.cursor_pos(Cursor { para: 0, index: 3 }), (1, 0));
        assert_eq!(l.cursor_pos(Cursor { para: 0, index: 5 }), (1, 32));
        assert_eq!(l.cursor_vertical(c, 1), Some(Cursor { para: 0, index: 4 }));
        assert_eq!(
            l.cursor_vertical(Cursor { para: 0, index: 5 }, 1),
            Some(Cursor { para: 1, index: 1 })
        );
        assert_eq!(l.cursor_vertical(c, -1), None);
        assert_eq!(l.cursor_in_line(0, 100).index, 3);
        assert_eq!(l.line_at(0, 20), Some(1));
        assert_eq!(l.line_at(0, 500), Some(2));
        assert_eq!(l.line_at(3, 0), None);
    }
}
