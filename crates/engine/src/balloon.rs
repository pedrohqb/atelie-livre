//! Balões de fala do McZee e da Maggie: caixa amarela pontilhada com borda
//! preta de 2 pixels, texto na "Pablo Graphos" e opções clicáveis com uma
//! bolinha preta na frente (aparência medida no original).

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use formats::fnt::Font;
use formats::what::{
    Item, PIC_GLOSSARY, PIC_OPTION, PIC_OPTION_PICTURE, PIC_TIP, PIC_TIP_MAX, Record,
};
use std::rc::Rc;

const YELLOW: [u8; 3] = [255, 238, 76];
const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];
const GRAY: [u8; 3] = [120, 120, 120];
pub const PAD: i32 = 8;
const LINE_GAP: i32 = 2;
const BULLET: i32 = 14;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Balloon {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub text: String,
    pub options: Vec<String>,
    /// Ligações para dicas (lâmpada `21b`): (registro de destino, texto),
    /// desenhadas entre o texto e as opções.
    pub links: Vec<(usize, String)>,
    /// Figuras do `.PIC` no meio do texto ("clique até ver [ícone]"): cada
    /// uma ocupa, na ordem, uma linha [`FIGURE`] de `text`.
    pub figures: Vec<u16>,
}

/// Linha de `Balloon::text` que é o lugar de uma figura.
pub const FIGURE: char = '\u{1}';
/// Espaço acima e abaixo da figura no balão.
const FIGURE_GAP: i32 = 2;

impl Balloon {
    /// Balão a partir de um registro `WHAT` do original: o texto antes da
    /// primeira opção vira `text` (o `@` vira `name`), as opções ficam na ordem
    /// em que aparecem e a largura é a do original.
    /// Como [`Balloon::from_record`] para o registro `k` de um bloco, com as
    /// palavras do glossário (figura `ffff`) trocadas pelo título do registro
    /// que elas apontam ("Dê um *clique duplo* na imagem").
    pub fn from_block(records: &[Record], k: usize, x: i32, y: i32, name: &str) -> Option<Balloon> {
        let r = records.get(k)?;
        let mut r = r.clone();
        r.items = r
            .items
            .into_iter()
            .map(|i| match i {
                Item::Picture { id, param } if id == formats::what::PIC_GLOSSARY => {
                    match records.get((param & 0x7fff) as usize) {
                        Some(t) => Item::Text {
                            text: t.title.trim().to_string(),
                            bold: true,
                            color: 0xf,
                            indent: 0,
                            key: false,
                            signature: false,
                        },
                        None => Item::Text {
                            text: String::new(),
                            bold: false,
                            color: 0xf,
                            indent: 0,
                            key: false,
                            signature: false,
                        },
                    }
                }
                other => other,
            })
            .collect();
        let mut b = Balloon::from_record_in(&r, x, y, name, Some(k));
        b.links.retain(|(t, _)| *t < records.len());
        Some(b)
    }

    pub fn from_record(r: &Record, x: i32, y: i32, name: &str) -> Balloon {
        Balloon::from_record_in(r, x, y, name, None)
    }

    /// `own` é o índice do registro no bloco, se conhecido: a lâmpada que
    /// aponta para ele — ou para o registro 0, como a do título dos
    /// registros de dica (`21B0000`) — é só enfeite do título, que fica no
    /// texto; as outras lâmpadas viram ligações (`links`).
    fn from_record_in(r: &Record, x: i32, y: i32, name: &str, own: Option<usize>) -> Balloon {
        let is_title = |param: u16| {
            let t = (param & 0x7fff) as usize;
            t == 0 || Some(t) == own
        };
        let mut text = String::new();
        let mut figures = Vec::new();
        let mut in_options = false;
        // dentro de uma linha de dica (lâmpada): o texto vai para `links`
        let mut in_link = false;
        for i in &r.items {
            match i {
                Item::Picture { id, .. } if *id == PIC_OPTION || *id == PIC_OPTION_PICTURE => {
                    in_options = true;
                }
                Item::Picture { id, param } if *id == PIC_TIP || *id == PIC_TIP_MAX => {
                    in_link = !is_title(*param);
                }
                // figura do `.PIC` no texto (ícone de ferramenta, rolo…)
                Item::Picture { id, .. }
                    if !in_options && !in_link && *id >= 0x1000 && *id != PIC_GLOSSARY =>
                {
                    text.push('\n');
                    text.push(FIGURE);
                    text.push('\n');
                    figures.push(*id);
                }
                Item::Text { text: t, .. } if !in_options && !in_link => text.push_str(t),
                Item::Newline if !in_options => {
                    if in_link {
                        in_link = false;
                    } else {
                        text.push('\n');
                    }
                }
                _ => {}
            }
        }
        let text = text.replace('@', name).trim().to_string();
        let options: Vec<(u16, String)> = r.options();
        let links = r
            .links()
            .into_iter()
            .filter(|(t, s)| !s.is_empty() && !is_title(*t))
            .map(|(t, s)| (t as usize, s))
            .collect();
        Balloon {
            x,
            y,
            w: r.width.max(120) as i32,
            text,
            options: options.into_iter().map(|o| o.1).collect(),
            links,
            figures,
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

/// Caixa dos avisos "EPA!": fundo branco com a mesma borda preta.
pub fn draw_box_white(f: &mut Frame, x: i32, y: i32, w: i32, h: i32) {
    f.rect(x, y, w, h, BLACK);
    f.rect(x + 2, y + 2, w - 4, h - 4, WHITE);
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

/// Lâmpada das dicas (14×14 a partir de (x, y)): bulbo amarelo com contorno
/// e base cinza (o ícone do original não está no `.PIC`).
pub fn draw_lamp(f: &mut Frame, x: i32, y: i32) {
    for dy in 0..9 {
        for dx in 0..9 {
            let (ex, ey) = (dx - 4, dy - 4);
            let d = ex * ex + ey * ey;
            if d <= 16 {
                f.put(x + 2 + dx, y + dy, if d >= 11 { BLACK } else { YELLOW });
            }
        }
    }
    f.rect(x + 4, y + 9, 5, 2, BLACK);
    f.rect(x + 5, y + 11, 3, 2, GRAY);
}

/// A linha é o lugar de uma figura.
fn is_figure_line(line: &str) -> bool {
    line.trim() == FIGURE.to_string()
}

/// Linhas já quebradas do balão.
struct Layout {
    lines: Vec<String>,
    /// Altura de cada linha do texto (a das figuras é a da figura).
    line_hs: Vec<i32>,
    /// Soma de `line_hs`.
    text_h: i32,
    links: Vec<Vec<String>>,
    opts: Vec<Vec<String>>,
    h: i32,
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

    fn figure(id: u16) -> ImageRef {
        ImageRef::Pic(Program::Artist, id)
    }

    /// Linhas do texto, linhas de cada ligação, linhas de cada opção e
    /// altura total; sem fonte, usa a 8×8.
    fn layout(&self, a: &Assets, font: Option<&Font>) -> Layout {
        let inner = (self.w - 2 * PAD) as u32;
        let lines = Self::wrap_any(font, &self.text, inner);
        let lh = Self::line_h(font);
        let mut figs = self.figures.iter();
        let line_hs: Vec<i32> = lines
            .iter()
            .map(|l| {
                if !is_figure_line(l) {
                    return lh;
                }
                // figura que o port não tem não ocupa lugar (como na ajuda)
                match figs.next().map(|&id| Self::figure(id)) {
                    Some(r) if a.has_image(&r) => {
                        (a.image(&r).height as i32 + 2 * FIGURE_GAP).max(lh)
                    }
                    _ => 0,
                }
            })
            .collect();
        let text_h = line_hs.iter().sum::<i32>();
        let item_w = inner.saturating_sub((BULLET + 4) as u32);
        let links: Vec<Vec<String>> = self
            .links
            .iter()
            .map(|(_, t)| Self::wrap_any(font, t, item_w))
            .collect();
        let opts: Vec<Vec<String>> = self
            .options
            .iter()
            .map(|o| Self::wrap_any(font, o, item_w))
            .collect();
        let mut h = 2 * PAD + text_h;
        if !links.is_empty() {
            h += 4 + links.iter().map(|o| o.len() as i32 * lh + 4).sum::<i32>();
        }
        if !opts.is_empty() {
            h += 4 + opts.iter().map(|o| o.len() as i32 * lh + 4).sum::<i32>();
        }
        Layout {
            lines,
            line_hs,
            text_h,
            links,
            opts,
            h,
        }
    }

    pub fn height(&self, a: &Assets) -> i32 {
        self.layout(a, Self::font(a).as_deref()).h
    }

    /// Retângulo (x, y, w, h) da ligação `k`.
    pub fn link_rect(&self, a: &Assets, k: usize) -> (i32, i32, i32, i32) {
        let font = Self::font(a);
        let lay = self.layout(a, font.as_deref());
        let lh = Self::line_h(font.as_deref());
        let mut y0 = self.y + PAD + lay.text_h + 4;
        for o in lay.links.iter().take(k) {
            y0 += o.len() as i32 * lh + 4;
        }
        let n = lay.links.get(k).map_or(1, |o| o.len() as i32);
        (self.x + PAD, y0 - 2, self.w - 2 * PAD, n * lh + 4)
    }

    /// Retângulo (x, y, w, h) da opção `k`.
    pub fn option_rect(&self, a: &Assets, k: usize) -> (i32, i32, i32, i32) {
        let font = Self::font(a);
        let lay = self.layout(a, font.as_deref());
        let lh = Self::line_h(font.as_deref());
        let mut y0 = self.y + PAD + lay.text_h + 4;
        if !lay.links.is_empty() {
            y0 += 4 + lay
                .links
                .iter()
                .map(|o| o.len() as i32 * lh + 4)
                .sum::<i32>();
        }
        for o in lay.opts.iter().take(k) {
            y0 += o.len() as i32 * lh + 4;
        }
        let n = lay.opts.get(k).map_or(1, |o| o.len() as i32);
        (self.x + PAD, y0 - 2, self.w - 2 * PAD, n * lh + 4)
    }

    /// Índice da opção clicada em (x, y), se houver.
    pub fn hit(&self, a: &Assets, x: i32, y: i32) -> Option<usize> {
        (0..self.options.len()).find(|&k| {
            let r = self.option_rect(a, k);
            x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
        })
    }

    /// Registro da ligação (dica) clicada em (x, y), se houver.
    pub fn link_hit(&self, a: &Assets, x: i32, y: i32) -> Option<usize> {
        (0..self.links.len())
            .find(|&k| {
                let r = self.link_rect(a, k);
                x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
            })
            .map(|k| self.links[k].0)
    }

    pub fn contains(&self, a: &Assets, x: i32, y: i32) -> bool {
        let h = self.height(a);
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + h
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let font = Self::font(a);
        let lay = self.layout(a, font.as_deref());
        let lh = Self::line_h(font.as_deref());
        draw_box(f, self.x, self.y, self.w, lay.h);
        let mut ty = self.y + PAD;
        let mut figs = self.figures.iter();
        for (line, h) in lay.lines.iter().zip(&lay.line_hs) {
            if !is_figure_line(line) {
                draw_line(f, font.as_deref(), self.x + PAD, ty, line);
            } else if let Some(r) = figs.next().map(|&id| Self::figure(id))
                && a.has_image(&r)
            {
                let img = a.image(&r);
                let fx = self.x + (self.w - img.width as i32) / 2;
                f.blit(&img, fx, ty + FIGURE_GAP);
            }
            ty += h;
        }
        for (k, link) in lay.links.iter().enumerate() {
            let r = self.link_rect(a, k);
            draw_lamp(f, r.0, r.1 + 1 + (lh - 12) / 2);
            for (j, line) in link.iter().enumerate() {
                let ly = r.1 + 2 + j as i32 * lh;
                draw_line(f, font.as_deref(), r.0 + BULLET + 4, ly, line);
            }
        }
        for (k, opt) in lay.opts.iter().enumerate() {
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
            links: Vec::new(),
            figures: Vec::new(),
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

    fn tip_record(param: u16, title: &str, link: u16) -> Record {
        let t = |s: &str| Item::Text {
            text: s.into(),
            bold: false,
            color: 0xf,
            indent: 0,
            key: false,
            signature: false,
        };
        Record {
            title: "tip".into(),
            width: 200,
            height: 0,
            style: 0,
            items: vec![
                Item::Picture { id: PIC_TIP, param },
                t(title),
                Item::Newline,
                t("Corpo da dica."),
                Item::Newline,
                Item::Picture {
                    id: PIC_TIP,
                    param: link,
                },
                t("Outra dica"),
                Item::Newline,
            ],
        }
    }

    #[test]
    fn title_lamp_stays_in_the_text_and_other_lamps_become_links() {
        // `21B0000` no título (registro de dica) e uma ligação para o 2
        let recs = vec![
            tip_record(0, "intro", 0x7fff),
            tip_record(0, "Quer mudar a cor?", 2),
            tip_record(2, "Mesmo registro", 1),
        ];
        let b = Balloon::from_block(&recs, 1, 0, 0, "").unwrap();
        assert!(b.text.starts_with("Quer mudar a cor?"), "{}", b.text);
        assert_eq!(b.links, vec![(2, "Outra dica".to_string())]);
        // a lâmpada que aponta para o próprio registro também é título
        let b = Balloon::from_block(&recs, 2, 0, 0, "").unwrap();
        assert!(b.text.starts_with("Mesmo registro"), "{}", b.text);
        assert_eq!(b.links, vec![(1, "Outra dica".to_string())]);
        // destino fora do bloco é descartado
        let b = Balloon::from_block(&recs, 0, 0, 0, "").unwrap();
        assert!(b.links.is_empty());
    }

    #[test]
    fn inline_picture_takes_its_place_in_the_text() {
        // "Então, clique até ver [ícone]" (`4000[65]`)
        let t = |s: &str| Item::Text {
            text: s.into(),
            bold: false,
            color: 0xf,
            indent: 0,
            key: false,
            signature: false,
        };
        let r = Record {
            title: "until".into(),
            width: 200,
            height: 0,
            style: 0,
            items: vec![
                t("Então, clique até ver"),
                Item::Picture {
                    id: 0x105f,
                    param: 0,
                },
                t("depois"),
            ],
        };
        let b = Balloon::from_record(&r, 0, 0, "");
        assert_eq!(b.figures, vec![0x105f]);
        assert_eq!(b.text, format!("Então, clique até ver\n{FIGURE}\ndepois"));
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
    }

    #[test]
    fn missing_picture_takes_no_room() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        let with = Balloon {
            x: 0,
            y: 0,
            w: 200,
            text: format!("ver\n{FIGURE}\ndepois"),
            options: Vec::new(),
            links: Vec::new(),
            figures: vec![0x1021],
        };
        let without = Balloon {
            text: "ver\ndepois".into(),
            figures: Vec::new(),
            ..with.clone()
        };
        assert_eq!(with.height(&a), without.height(&a));
    }
}
