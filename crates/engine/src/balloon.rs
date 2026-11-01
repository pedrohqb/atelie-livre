//! Balões de fala do McZee e da Maggie: caixa amarela pontilhada com borda
//! preta de 2 pixels, texto na "Pablo Graphos" e opções clicáveis com uma
//! bolinha preta na frente (aparência medida no original).

use crate::Frame;
use crate::assets::Assets;
use formats::fnt::Font;
use formats::what::{Item, PIC_OPTION, PIC_OPTION_PICTURE, Record};
use std::rc::Rc;

const YELLOW: [u8; 3] = [255, 238, 76];
const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];
pub const PAD: i32 = 8;
const LINE_GAP: i32 = 2;
const BULLET: i32 = 14;

pub struct Balloon {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub text: String,
    pub options: Vec<String>,
}

impl Balloon {
    /// Balão a partir de um registro `WHAT` do original: o texto antes da
    /// primeira opção vira `text` (o `@` vira `name`), as opções ficam na ordem
    /// em que aparecem e a largura é a do original.
    pub fn from_record(r: &Record, x: i32, y: i32, name: &str) -> Balloon {
        let mut text = String::new();
        let mut in_options = false;
        for i in &r.items {
            match i {
                Item::Picture { id, .. } if *id == PIC_OPTION || *id == PIC_OPTION_PICTURE => {
                    in_options = true;
                }
                Item::Text { text: t, .. } if !in_options => text.push_str(t),
                Item::Newline if !in_options => text.push('\n'),
                _ => {}
            }
        }
        let text = text.replace('@', name).trim().to_string();
        let options: Vec<(u16, String)> = r.options();
        Balloon {
            x,
            y,
            w: r.width.max(120) as i32,
            text,
            options: options.into_iter().map(|o| o.1).collect(),
        }
    }
}

/// Quebra o texto em linhas que cabem em `width` pixels.
pub fn wrap(font: &Font, text: &str, width: u32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let cand = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if font.width(&cand) <= width || line.is_empty() {
                line = cand;
            } else {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            }
        }
        lines.push(line);
    }
    lines
}

/// Fonte dos balões: "Pablo Graphos" de 8 pontos, se instalada.
pub fn balloon_font(a: &Assets) -> Option<Rc<Font>> {
    a.font(8)
}

/// Altura de uma linha de texto do balão; sem fonte, a 8×8.
pub fn line_height(font: Option<&Font>) -> i32 {
    font.map_or(10, |f| f.height as i32 + LINE_GAP)
}

/// Quebra o texto na fonte, ou só nas quebras de linha sem ela.
pub fn wrap_opt(font: Option<&Font>, text: &str, width: u32) -> Vec<String> {
    match font {
        Some(f) => wrap(f, text, width),
        None => text.split('\n').map(str::to_string).collect(),
    }
}

/// Caixa do balão: fundo pontilhado amarelo com borda preta e cantos arredondados.
pub fn draw_box(f: &mut Frame, x: i32, y: i32, w: i32, h: i32) {
    // fundo pontilhado: amarelo nas posições (x + y) ímpares de 1 em 3
    f.rect(x, y, w, h, BLACK);
    for yy in y + 2..y + h - 2 {
        for xx in x + 2..x + w - 2 {
            let c = if (xx + 2 * yy) % 3 == 0 {
                YELLOW
            } else {
                WHITE
            };
            f.put(xx, yy, c);
        }
    }
    // cantos arredondados (tira um pixel de cada canto)
    for (cx, cy) in [
        (x, y),
        (x + w - 1, y),
        (x, y + h - 1),
        (x + w - 1, y + h - 1),
    ] {
        f.put(cx, cy, WHITE);
    }
}

/// Bolinha preta com brilho branco (marca das opções), 10×10 a partir de (x, y).
pub fn draw_bullet(f: &mut Frame, bx: i32, by: i32) {
    for dy in 0..10 {
        for dx in 0..10 {
            let (ex, ey) = (dx as f32 - 4.5, dy as f32 - 4.5);
            if ex * ex + ey * ey <= 22.0 {
                f.put(bx + dx, by + dy, BLACK);
            }
        }
    }
    f.put(bx + 6, by + 3, WHITE);
    f.put(bx + 5, by + 3, WHITE);
    f.put(bx + 6, by + 2, WHITE);
    f.put(bx + 6, by + 4, WHITE);
    f.put(bx + 7, by + 3, WHITE);
}

/// Texto numa linha: na fonte do balão ou na 8×8.
pub fn draw_line(f: &mut Frame, font: Option<&Font>, x: i32, y: i32, s: &str) {
    match font {
        Some(fo) => {
            f.text_font(fo, x, y, s, BLACK);
        }
        None => f.text(x, y, s, BLACK),
    }
}

impl Balloon {
    fn font(a: &Assets) -> Option<Rc<Font>> {
        balloon_font(a)
    }

    fn line_h(font: Option<&Font>) -> i32 {
        line_height(font)
    }

    fn wrap_any(font: Option<&Font>, text: &str, width: u32) -> Vec<String> {
        wrap_opt(font, text, width)
    }

    /// Linhas do texto, linhas de cada opção e altura total; sem fonte, usa a 8×8.
    fn layout(&self, font: Option<&Font>) -> (Vec<String>, Vec<Vec<String>>, i32) {
        let inner = (self.w - 2 * PAD) as u32;
        let lines = Self::wrap_any(font, &self.text, inner);
        let opts: Vec<Vec<String>> = self
            .options
            .iter()
            .map(|o| Self::wrap_any(font, o, inner.saturating_sub((BULLET + 4) as u32)))
            .collect();
        let lh = Self::line_h(font);
        let mut h = 2 * PAD + lines.len() as i32 * lh;
        if !opts.is_empty() {
            h += 4 + opts.iter().map(|o| o.len() as i32 * lh + 4).sum::<i32>();
        }
        (lines, opts, h)
    }

    pub fn height(&self, a: &Assets) -> i32 {
        self.layout(Self::font(a).as_deref()).2
    }

    /// Retângulo (x, y, w, h) da opção `k`.
    pub fn option_rect(&self, a: &Assets, k: usize) -> (i32, i32, i32, i32) {
        let font = Self::font(a);
        let (lines, opts, _) = self.layout(font.as_deref());
        let lh = Self::line_h(font.as_deref());
        let mut y0 = self.y + PAD + lines.len() as i32 * lh + 4;
        for o in opts.iter().take(k) {
            y0 += o.len() as i32 * lh + 4;
        }
        let n = opts.get(k).map_or(1, |o| o.len() as i32);
        (self.x + PAD, y0 - 2, self.w - 2 * PAD, n * lh + 4)
    }

    /// Índice da opção clicada em (x, y), se houver.
    pub fn hit(&self, a: &Assets, x: i32, y: i32) -> Option<usize> {
        (0..self.options.len()).find(|&k| {
            let r = self.option_rect(a, k);
            x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
        })
    }

    pub fn contains(&self, a: &Assets, x: i32, y: i32) -> bool {
        let h = self.height(a);
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + h
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let font = Self::font(a);
        let (lines, opts, h) = self.layout(font.as_deref());
        let lh = Self::line_h(font.as_deref());
        draw_box(f, self.x, self.y, self.w, h);
        let mut ty = self.y + PAD;
        for line in &lines {
            draw_line(f, font.as_deref(), self.x + PAD, ty, line);
            ty += lh;
        }
        for (k, opt) in opts.iter().enumerate() {
            let r = self.option_rect(a, k);
            draw_bullet(f, r.0, r.1 + 2 + (lh - 10) / 2);
            for (j, line) in opt.iter().enumerate() {
                let ly = r.1 + 2 + j as i32 * lh;
                draw_line(f, font.as_deref(), r.0 + BULLET + 4, ly, line);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_breaks_on_width() {
        // fonte sintética: cada glifo tem 3 px (ver formats::fnt tests)
        let mut d = vec![0u8; 0x76 + 3 * 4];
        d[0..2].copy_from_slice(&0x200u16.to_le_bytes());
        d[0x4a..0x4c].copy_from_slice(&2u16.to_le_bytes());
        d[0x58..0x5a].copy_from_slice(&2u16.to_le_bytes());
        d[0x5f] = 0x20;
        d[0x60] = 0x21;
        let bits = d.len() as u16;
        for i in 0..3 {
            d[0x76 + 4 * i..0x78 + 4 * i].copy_from_slice(&3u16.to_le_bytes());
            d[0x78 + 4 * i..0x7a + 4 * i].copy_from_slice(&bits.to_le_bytes());
        }
        d.extend_from_slice(&[0, 0]);
        let f = formats::fnt::parse(&d).unwrap();
        // "aa aa aa" = 8 glifos de 3 px = 24; largura 15 → quebra em duas linhas
        let lines = wrap(&f, "aa aa aa", 15);
        assert_eq!(lines, vec!["aa aa", "aa"]);
        assert_eq!(wrap(&f, "x\ny", 100), vec!["x", "y"]);
    }

    #[test]
    fn options_are_hit_in_order() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        let b = Balloon {
            x: 100,
            y: 50,
            w: 200,
            text: "Oi".into(),
            options: vec!["um".into(), "dois".into()],
        };
        let r0 = b.option_rect(&a, 0);
        let r1 = b.option_rect(&a, 1);
        assert!(r1.1 > r0.1);
        assert_eq!(b.hit(&a, r0.0 + 5, r0.1 + 2), Some(0));
        assert_eq!(b.hit(&a, r1.0 + 5, r1.1 + 2), Some(1));
        assert_eq!(b.hit(&a, 0, 0), None);
        assert!(b.contains(&a, 150, 60));
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        assert_eq!(f.pixel(100, 60)[..3], [0, 0, 0]);
    }
}
