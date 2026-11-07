//! Balão de ajuda do McZee na tela de pintura: um registro `WHAT` com a
//! formatação do original — negrito (dupla impressão deslocada 1 px), cor,
//! recuo, figuras embutidas (ícone da ferramenta, ids `1xxx`) e ligações
//! clicáveis (dica `21b`, tecla de atalho `21c`, dica da Maggie `21e`,
//! palavra do glossário `ffff`) que abrem o registro ligado no lugar.

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::balloon::draw_box;
use formats::fnt::Font;
use formats::what::{Item, PIC_GLOSSARY, PIC_HELP, PIC_TIP, PIC_TIP_MAX, Record};
use std::rc::Rc;

type Rect = (i32, i32, i32, i32);

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const PURPLE: [u8; 3] = [153, 51, 204];
const YELLOW: [u8; 3] = [255, 238, 76];
const GRAY: [u8; 3] = [120, 120, 120];
const PAD: i32 = 8;
const LINE_GAP: i32 = 2;
const INDENT: i32 = 12;
/// Tecla de atalho (`21c`).
const PIC_KEY: u16 = 0x21c;
/// Dica da Maggie (`21e`).
const PIC_MAGGIE: u16 = 0x21e;
/// Tamanho dos ícones desenhados à mão (dica, tecla, Maggie).
const ICON: i32 = 14;
/// Altura máxima de uma figura embutida (ícones da barra de ferramentas).
const MAX_PIC_H: i32 = 44;

/// Pedaço já posicionado do balão (coordenadas relativas ao canto do balão).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Text {
        x: i32,
        y: i32,
        text: String,
        bold: bool,
        color: [u8; 3],
        /// Palavra do glossário: sublinhado pontilhado.
        glossary: bool,
        link: Option<usize>,
    },
    Picture {
        x: i32,
        y: i32,
        id: u16,
    },
    Icon {
        x: i32,
        y: i32,
        kind: u16,
        link: Option<usize>,
    },
}

struct Layout {
    pieces: Vec<Piece>,
    height: i32,
    /// Retângulos clicáveis (relativos) e o registro que abrem.
    links: Vec<(Rect, usize)>,
}

/// Distância entre um balão e o sub-balão de uma ligação ao lado.
const GAP_BESIDE: i32 = 6;

#[derive(Clone)]
pub struct HelpBalloon {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    /// Bloco `WHAT` e índice do registro mostrado.
    pub block: u16,
    pub index: usize,
    records: Rc<Vec<Record>>,
    /// Ponta do rabicho (quem fala), acima do balão; `None` = sem rabicho.
    pub tail: Option<(i32, i32)>,
}

fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Artist, id)
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn text_w(font: Option<&Font>, s: &str, bold: bool) -> i32 {
    let w = match font {
        Some(f) => f.width(s) as i32,
        None => 8 * s.chars().count() as i32,
    };
    if bold && w > 0 { w + 1 } else { w }
}

impl HelpBalloon {
    /// Balão do registro `index` do bloco; `None` se o registro não existe
    /// ou está vazio. O balão fica em `(x, y)`, puxado para dentro da tela.
    pub fn new(records: Rc<Vec<Record>>, block: u16, index: usize, x: i32, y: i32) -> Option<Self> {
        let r = records.get(index)?;
        if r.plain().trim().is_empty() {
            return None;
        }
        let w = (r.width as i32).clamp(120, 400);
        Some(HelpBalloon {
            x: x.min(635 - w),
            y,
            w,
            block,
            index,
            records,
            tail: None,
        })
    }

    /// Mesmo balão com outro registro do bloco (ligação), se existir.
    pub fn follow(&self, index: usize) -> Option<HelpBalloon> {
        HelpBalloon::new(self.records.clone(), self.block, index, self.x, self.y).map(|mut h| {
            h.tail = self.tail;
            h
        })
    }

    /// Balão de uma ligação (gema, tecla, glossário) ao lado deste, que
    /// continua aberto: à esquerda, como no original, ou à direita se não
    /// couber.
    pub fn follow_beside(&self, index: usize) -> Option<HelpBalloon> {
        let mut b = self.follow(index)?;
        let left = self.x - b.w - GAP_BESIDE;
        b.x = if left >= 5 {
            left
        } else {
            (self.x + self.w + GAP_BESIDE).min(635 - b.w)
        };
        Some(b)
    }

    /// Identidade do balão (bloco, registro e lugar).
    pub fn key(&self) -> (u16, usize, i32, i32) {
        (self.block, self.index, self.x, self.y)
    }

    pub fn record(&self) -> &Record {
        &self.records[self.index]
    }

    pub fn title(&self) -> &str {
        &self.records[self.index].title
    }

    fn font(a: &Assets) -> Option<Rc<Font>> {
        a.font(8)
    }

    fn line_h(font: Option<&Font>) -> i32 {
        font.map_or(10, |f| f.height as i32 + LINE_GAP)
    }

    fn layout(&self, a: &Assets) -> Layout {
        let font = Self::font(a);
        let font = font.as_deref();
        let lh = Self::line_h(font);
        let inner = self.w - 2 * PAD;
        let mut pieces = Vec::new();
        let mut links = Vec::new();
        // linha em montagem: pedaços com a altura de cada um
        let mut line: Vec<(Piece, i32)> = Vec::new();
        let mut x = 0;
        let mut y = PAD;
        let mut indent_x = 0;
        let mut link: Option<usize> = None;
        let space = text_w(font, " ", false);

        fn flush(
            line: &mut Vec<(Piece, i32)>,
            y: &mut i32,
            lh: i32,
            pad: i32,
            pieces: &mut Vec<Piece>,
            links: &mut Vec<(Rect, usize)>,
            font: Option<&Font>,
        ) {
            let line_h = line.iter().map(|(_, h)| *h).max().unwrap_or(lh).max(lh);
            for (mut p, h) in line.drain(..) {
                let top = *y + (line_h - h) / 2;
                let (px, w, l) = match &mut p {
                    Piece::Text {
                        x,
                        y: py,
                        text,
                        bold,
                        link,
                        ..
                    } => {
                        *py = top;
                        (*x, text_w(font, text, *bold), *link)
                    }
                    Piece::Picture { x, y: py, .. } => {
                        *py = top;
                        (*x, 0, None)
                    }
                    Piece::Icon { x, y: py, link, .. } => {
                        *py = top;
                        (*x, ICON, *link)
                    }
                };
                if let Some(target) = l {
                    links.push(((pad + px, top, w.max(ICON), h), target));
                }
                pieces.push(p);
            }
            *y += line_h;
        }

        for item in &self.record().items {
            match item {
                Item::Newline => {
                    flush(&mut line, &mut y, lh, PAD, &mut pieces, &mut links, font);
                    x = 0;
                    link = None;
                }
                Item::Text {
                    text,
                    bold,
                    color,
                    indent,
                    ..
                } => {
                    indent_x = *indent as i32 * INDENT;
                    if x < indent_x {
                        x = indent_x;
                    }
                    let color = if *color == 9 { PURPLE } else { BLACK };
                    let starts_space = text.starts_with(' ');
                    let mut first = true;
                    for word in text.split_whitespace() {
                        let w = text_w(font, word, *bold);
                        let gap = if x > indent_x && (!first || starts_space) {
                            space
                        } else {
                            0
                        };
                        if x + gap + w > inner && x > indent_x {
                            flush(&mut line, &mut y, lh, PAD, &mut pieces, &mut links, font);
                            x = indent_x;
                        } else {
                            x += gap;
                        }
                        line.push((
                            Piece::Text {
                                x,
                                y: 0,
                                text: word.to_string(),
                                bold: *bold,
                                color,
                                glossary: false,
                                link,
                            },
                            lh,
                        ));
                        x += w;
                        first = false;
                    }
                    if text.ends_with(' ') && !text.trim().is_empty() {
                        x += space;
                    }
                }
                Item::Picture { id, param } => match *id {
                    PIC_GLOSSARY => {
                        let target = *param as usize;
                        let word = self
                            .records
                            .get(target)
                            .map(|r| r.title.trim().to_string())
                            .unwrap_or_default();
                        if word.is_empty() {
                            continue;
                        }
                        let w = text_w(font, &word, false);
                        let gap = if x > indent_x { space } else { 0 };
                        if x + gap + w > inner && x > indent_x {
                            flush(&mut line, &mut y, lh, PAD, &mut pieces, &mut links, font);
                            x = indent_x;
                        } else {
                            x += gap;
                        }
                        line.push((
                            Piece::Text {
                                x,
                                y: 0,
                                text: word,
                                bold: false,
                                color: BLACK,
                                glossary: true,
                                link: Some(target),
                            },
                            lh,
                        ));
                        x += w + space;
                    }
                    PIC_TIP | PIC_TIP_MAX | PIC_KEY | PIC_MAGGIE | PIC_HELP => {
                        let target = (*param & 0x7fff) as usize;
                        let l = (target != self.index && *id != PIC_HELP).then_some(target);
                        link = l;
                        if x + ICON > inner && x > indent_x {
                            flush(&mut line, &mut y, lh, PAD, &mut pieces, &mut links, font);
                            x = indent_x;
                        }
                        line.push((
                            Piece::Icon {
                                x,
                                y: 0,
                                kind: *id,
                                link: l,
                            },
                            ICON,
                        ));
                        x += ICON + space;
                    }
                    id if (0x1000..0x2000).contains(&id) => {
                        let shown =
                            crate::help_icons::icon(id).map_or(id, crate::help_icons::Icon::pic);
                        if !a.has_image(&pic(shown)) {
                            continue;
                        }
                        let img = a.image(&pic(shown));
                        let (w, h) = (img.width as i32, (img.height as i32).min(MAX_PIC_H));
                        if x + w > inner && x > indent_x {
                            flush(&mut line, &mut y, lh, PAD, &mut pieces, &mut links, font);
                            x = indent_x;
                        }
                        line.push((Piece::Picture { x, y: 0, id }, h));
                        x += w + space;
                    }
                    _ => {}
                },
            }
        }
        flush(&mut line, &mut y, lh, PAD, &mut pieces, &mut links, font);
        Layout {
            pieces,
            height: y + PAD,
            links,
        }
    }

    pub fn height(&self, a: &Assets) -> i32 {
        self.layout(a).height
    }

    /// Posição de tela do balão, puxado para cima se não couber.
    fn origin(&self, a: &Assets) -> (i32, i32, i32) {
        let h = self.height(a);
        (self.x, self.y.min(455 - h).max(5), h)
    }

    pub fn contains(&self, a: &Assets, x: i32, y: i32) -> bool {
        let (bx, by, h) = self.origin(a);
        inside((bx, by, self.w, h), x, y)
    }

    /// Registro ligado sob `(x, y)`, se o clique caiu numa ligação.
    pub fn link_at(&self, a: &Assets, x: i32, y: i32) -> Option<usize> {
        let (bx, by, _) = self.origin(a);
        self.layout(a)
            .links
            .iter()
            .find(|(r, _)| inside((bx + r.0, by + r.1, r.2, r.3), x, y))
            .map(|(_, t)| *t)
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let font = Self::font(a);
        let font = font.as_deref();
        let lay = self.layout(a);
        let (bx, by, h) = self.origin(a);
        draw_box(f, bx, by, self.w, h);
        if let Some(tip) = self.tail.filter(|t| t.1 < by - 4) {
            let base = tip.0.clamp(bx + 12, bx + self.w - 12);
            crate::balloon::draw_tail(f, base, by, tip);
        }
        for p in &lay.pieces {
            match p {
                Piece::Text {
                    x,
                    y,
                    text,
                    bold,
                    color,
                    glossary,
                    ..
                } => {
                    let (sx, sy) = (bx + PAD + x, by + y);
                    let w = draw_text(f, font, sx, sy, text, *color);
                    if *bold {
                        draw_text(f, font, sx + 1, sy, text, *color);
                    }
                    if *glossary {
                        let base = sy + font.map_or(8, |fo| fo.height as i32) - 1;
                        for dx in (0..w).step_by(2) {
                            f.put(sx + dx, base, GRAY);
                        }
                    }
                }
                Piece::Picture { x, y, id } => {
                    crate::help_icons::draw(f, a, *id, bx + PAD + x, by + y);
                }
                Piece::Icon { x, y, kind, .. } => draw_icon(f, bx + PAD + x, by + y, *kind),
            }
        }
    }
}

/// Texto na fonte do balão (ou na 8×8 reserva); devolve a largura.
fn draw_text(f: &mut Frame, font: Option<&Font>, x: i32, y: i32, s: &str, c: [u8; 3]) -> i32 {
    match font {
        Some(fo) => f.text_font(fo, x, y, s, c),
        None => {
            f.text(x, y, s, c);
            8 * s.chars().count() as i32
        }
    }
}

/// Ícones das ligações, desenhados à mão (os do original não estão no `.PIC`):
/// pedra roxa (dica), chave (tecla de atalho), estrela (dica da Maggie), "?".
fn draw_icon(f: &mut Frame, x: i32, y: i32, kind: u16) {
    match kind {
        PIC_KEY => {
            // chave deitada: anel à esquerda, haste com dentes à direita
            for dy in 0..7 {
                for dx in 0..7 {
                    let (ex, ey) = (dx - 3, dy - 3);
                    let d = ex * ex + ey * ey;
                    if d <= 9 && d > 2 {
                        f.put(x + dx, y + 4 + dy, BLACK);
                    }
                }
            }
            f.rect(x + 7, y + 7, 7, 2, BLACK);
            f.rect(x + 11, y + 9, 1, 2, BLACK);
            f.rect(x + 13, y + 9, 1, 2, BLACK);
        }
        PIC_MAGGIE => {
            // estrela de quatro pontas amarela com contorno
            for dy in 0..ICON {
                for dx in 0..ICON {
                    let (ex, ey) = ((dx - 7).abs(), (dy - 7).abs());
                    if ex * ey <= 6 && ex + ey <= 7 {
                        f.put(x + dx, y + dy, if ex + ey >= 6 { BLACK } else { YELLOW });
                    }
                }
            }
        }
        PIC_HELP => {
            f.rect(x + 2, y + 1, 10, 12, WHITE);
            f.frame_rect(x + 2, y + 1, 10, 12, 1, BLACK);
            f.text(x + 3, y + 3, "?", BLACK);
        }
        _ => {
            // dica do McZee: pedra roxa lapidada, como no original (o balão
            // simples da Maggie usa a lâmpada de `draw_lamp`)
            for dy in 0..11 {
                let half: i32 = if dy < 3 { 4 + dy } else { (10 - dy) * 6 / 7 };
                for dx in -half..=half {
                    let edge = dx == -half || dx == half || dy == 0 || dy == 10;
                    let c = if edge {
                        BLACK
                    } else if dy == 3 || (dy < 3 && dx.abs() <= 1) {
                        WHITE
                    } else {
                        PURPLE
                    };
                    f.put(x + 7 + dx, y + 2 + dy, c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    fn t(text: &str, bold: bool) -> Item {
        Item::Text {
            text: text.into(),
            bold,
            color: 0xf,
            indent: 0,
            key: false,
            signature: false,
        }
    }

    fn records() -> Rc<Vec<Record>> {
        Rc::new(vec![
            Record {
                title: "Lata de tinta".into(),
                width: 150,
                height: 100,
                style: 2,
                items: vec![
                    Item::Picture {
                        id: 0x1001,
                        param: 0,
                    },
                    t("Lata de tinta", true),
                    Item::Newline,
                    t("Preencha uma", false),
                    Item::Picture {
                        id: PIC_GLOSSARY,
                        param: 2,
                    },
                    t("com cores e padrões que vão além.", false),
                    Item::Newline,
                    Item::Picture {
                        id: PIC_TIP,
                        param: 1 | 0x8000,
                    },
                    t("Sem quebras", false),
                ],
            },
            Record {
                title: "Sem quebras".into(),
                width: 100,
                height: 40,
                style: 2,
                items: vec![t("Dica.", false)],
            },
            Record {
                title: "área".into(),
                width: 100,
                height: 40,
                style: 2,
                items: vec![t("Uma região.", false)],
            },
            Record {
                title: " ".into(),
                width: 200,
                height: 1,
                style: 2,
                items: vec![],
            },
        ])
    }

    #[test]
    fn empty_record_is_none_and_balloon_stays_on_screen() {
        assert!(HelpBalloon::new(records(), 0x1003, 3, 445, 115).is_none());
        assert!(HelpBalloon::new(records(), 0x1003, 9, 445, 115).is_none());
        let b = HelpBalloon::new(records(), 0x1003, 0, 600, 115).unwrap();
        assert_eq!(b.x + b.w, 635);
        assert_eq!(b.title(), "Lata de tinta");
    }

    #[test]
    fn layout_wraps_links_and_draws_bold() {
        let (_d, a) = empty_assets();
        let b = HelpBalloon::new(records(), 0x1003, 0, 445, 115).unwrap();
        let lay = b.layout(&a);
        // sem .PIC a figura 1001 some; texto quebra em várias linhas
        assert!(
            !lay.pieces
                .iter()
                .any(|p| matches!(p, Piece::Picture { .. }))
        );
        assert!(lay.height > 40);
        let ys: std::collections::BTreeSet<i32> = lay
            .pieces
            .iter()
            .filter_map(|p| match p {
                Piece::Text { y, .. } => Some(*y),
                _ => None,
            })
            .collect();
        assert!(ys.len() >= 4, "{ys:?}");
        // ligações: a palavra do glossário e a dica (ícone + texto)
        assert!(lay.links.iter().any(|(_, t)| *t == 2));
        assert!(lay.links.iter().filter(|(_, t)| *t == 1).count() >= 2);
        let (r, _) = lay.links.iter().find(|(_, t)| *t == 2).unwrap();
        let (bx, by, _) = b.origin(&a);
        assert_eq!(b.link_at(&a, bx + r.0 + 1, by + r.1 + 1), Some(2));
        assert_eq!(b.link_at(&a, bx + 1, by + 1), None);
        assert!(b.contains(&a, bx + 1, by + 1));
        assert!(!b.contains(&a, bx - 1, by));
        let next = b.follow(2).unwrap();
        assert_eq!(next.title(), "área");
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        assert_eq!(f.pixel(bx as u32 + 1, by as u32 + 1)[..3], [0, 0, 0]);
    }
}
