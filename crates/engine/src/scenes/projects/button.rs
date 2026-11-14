//! Botão ou adesivo: forma recortada no quadro e folha de impressão.
//!
//! O original pergunta "Redondo ou Quadrado" para o botão e "Retangular,
//! Redondo, Quadrado, Oval alto ou Oval largo" para o adesivo (roteiro de
//! ajuda `WHAT` 3c00 do `ARTIST.EXE`, registros 4 e 26; os bitmaps das
//! formas que ele cita não existem no `ARTIST.PIC`, por isso as formas são
//! geométricas). Tamanho e lugar de cada forma, o contorno preto de 1 px e
//! o quadro cinza em volta foram medidos no original no Wine
//! (`/home/claude/sweep/report/fid-button.md`). A forma é overlay por cima do
//! quadro; a pintura fica limitada à máscara de recorte (`Canvas::clip_mask`)
//! e fora dela o quadro é branco (na tela aparece cinza). A forma escolhida fica gravada em `ultimo-<tipo>.projeto.txt`.

use super::{BLACK, NICHE, STRIP, card, clear_niche, inside, label};
use crate::Frame;
use crate::assets::Assets;
use crate::paint::{BLACK as INK_BLACK, CANVAS_H, CANVAS_W, Canvas, History, ORIGIN, WHITE};
use formats::palette::Rgb;

/// Espessura da borda preta (1 px, como no original).
pub const BORDER: f32 = 1.0;
/// Cor do quadro fora da forma, na tela.
pub const OUTSIDE: [u8; 3] = [100, 100, 100];
/// Folha de impressão: 210×297 mm a 100 ppp.
pub const PAGE_W: u32 = 827;
pub const PAGE_H: u32 = 1169;

/// Contorno de uma forma: retângulo de cantos arredondados ou elipse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outline {
    Rounded { w: f32, h: f32, r: f32 },
    Ellipse { w: f32, h: f32 },
}

impl Outline {
    fn size(self) -> (f32, f32) {
        match self {
            Outline::Rounded { w, h, .. } | Outline::Ellipse { w, h } => (w, h),
        }
    }

    /// Distância com sinal (negativa dentro) do ponto `(px, py)` à borda do
    /// contorno centrado em `(cx, cy)`. Na elipse é aproximada pelo raio
    /// menor, o bastante para bordas de poucos pixels.
    pub fn distance(self, cx: f32, cy: f32, px: f32, py: f32) -> f32 {
        match self {
            Outline::Rounded { w, h, r } => {
                let qx = (px - cx).abs() - (w / 2.0 - r);
                let qy = (py - cy).abs() - (h / 2.0 - r);
                let (ox, oy) = (qx.max(0.0), qy.max(0.0));
                (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
            }
            Outline::Ellipse { w, h } => {
                let (a, b) = (w / 2.0, h / 2.0);
                let (dx, dy) = ((px - cx) / a, (py - cy) / b);
                ((dx * dx + dy * dy).sqrt() - 1.0) * a.min(b)
            }
        }
    }
}

/// As formas: dois botões (com a aba de metal) e cinco adesivos (roteiro
/// `WHAT` 3c00 [4] e [26]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Botão redondo: círculo de 163 px.
    Round,
    /// Botão quadrado: 146×146.
    Square,
    /// Adesivo retangular: 218×164.
    Rect,
    /// Adesivo redondo: círculo de 199 px.
    StickerRound,
    /// Adesivo quadrado: 200×200.
    StickerSquare,
    /// Adesivo oval alto: elipse de 163×199.
    TallOval,
    /// Adesivo oval largo: elipse de 199×163.
    WideOval,
}

impl Kind {
    pub const BUTTONS: [Kind; 2] = [Kind::Round, Kind::Square];
    pub const STICKERS: [Kind; 5] = [
        Kind::Rect,
        Kind::StickerRound,
        Kind::StickerSquare,
        Kind::TallOval,
        Kind::WideOval,
    ];

    pub fn is_sticker(self) -> bool {
        !Kind::BUTTONS.contains(&self)
    }

    /// Rótulo do cartão da faixa (textos do `WHAT` 3c00).
    pub fn label(self) -> &'static str {
        match self {
            Kind::Round => "Redondo",
            Kind::Square => "Quadrado",
            Kind::Rect => "Retangular",
            Kind::StickerRound => "Redondo",
            Kind::StickerSquare => "Quadrado",
            Kind::TallOval => "Oval alto",
            Kind::WideOval => "Oval largo",
        }
    }

    /// Nome gravado no arquivo `*.projeto.txt` (só ASCII).
    pub fn file_name(self) -> &'static str {
        match self {
            Kind::Round => "botao-redondo",
            Kind::Square => "botao-quadrado",
            Kind::Rect => "retangular",
            Kind::StickerRound => "redondo",
            Kind::StickerSquare => "quadrado",
            Kind::TallOval => "oval-alto",
            Kind::WideOval => "oval-largo",
        }
    }

    pub fn from_file_name(s: &str) -> Option<Kind> {
        Kind::BUTTONS
            .iter()
            .chain(Kind::STICKERS.iter())
            .copied()
            .find(|k| k.file_name() == s)
    }

    pub fn outline(self) -> Outline {
        let (_, _, w, h) = self.bbox();
        let (w, h) = (w as f32, h as f32);
        match self {
            Kind::Round | Kind::StickerRound => Outline::Rounded { w, h, r: w / 2.0 },
            Kind::Square | Kind::StickerSquare | Kind::Rect => Outline::Rounded { w, h, r: 0.0 },
            Kind::TallOval | Kind::WideOval => Outline::Ellipse { w, h },
        }
    }

    /// Cópias por folha de impressão: (colunas, linhas).
    pub fn sheet_grid(self) -> (u32, u32) {
        if self.is_sticker() { (3, 4) } else { (2, 3) }
    }

    /// Caixa `(x, y, w, h)` da forma (com o contorno) no quadro: a medida na
    /// tela do original menos a origem do quadro.
    pub fn bbox(self) -> (i32, i32, i32, i32) {
        let (x, y, w, h) = match self {
            Kind::Round => (230, 186, 163, 163),
            Kind::Square => (243, 209, 146, 146),
            Kind::Rect => (216, 196, 218, 164),
            Kind::StickerRound => (224, 180, 199, 199),
            Kind::StickerSquare => (224, 180, 200, 200),
            Kind::TallOval => (232, 180, 163, 199),
            Kind::WideOval => (224, 196, 199, 163),
        };
        (x - ORIGIN.0, y - ORIGIN.1, w, h)
    }

    /// Distância com sinal do centro do pixel `(x, y)` à borda da forma
    /// (negativa dentro).
    fn distance(self, x: i32, y: i32) -> f32 {
        let (bx, by, bw, bh) = self.bbox();
        let cx = bx as f32 + bw as f32 / 2.0;
        let cy = by as f32 + bh as f32 / 2.0;
        self.outline()
            .distance(cx, cy, x as f32 + 0.5, y as f32 + 0.5)
    }

    /// Largura da moldura (o contorno) que não se pinta.
    fn rim(self) -> f32 {
        BORDER
    }

    /// Pixel do contorno: `Some(true)` borda preta, `None` fora dele (o
    /// `Some(false)`, anel, não existe mais: o original não tem aba).
    fn rim_at(self, x: i32, y: i32) -> Option<bool> {
        let d = self.distance(x, y);
        if d >= 0.0 || d < -self.rim() {
            None
        } else {
            Some(d >= -BORDER)
        }
    }

    /// Máscara de recorte: `true` onde se pode pintar (dentro da moldura).
    pub fn mask(self) -> Vec<bool> {
        let rim = self.rim();
        (0..CANVAS_W * CANVAS_H)
            .map(|k| {
                let (x, y) = ((k % CANVAS_W) as i32, (k / CANVAS_W) as i32);
                self.distance(x, y) < -rim
            })
            .collect()
    }

    /// Miniatura do contorno (1 px de traço) que cabe em `w`×`h`, com o
    /// canto superior esquerdo em `(x, y)`.
    pub fn glyph(self, f: &mut Frame, x: i32, y: i32, w: i32, h: i32) {
        let (ow, oh) = self.outline().size();
        let scale = (w as f32 / ow).min(h as f32 / oh);
        let outline = match self.outline() {
            Outline::Rounded { w, h, r } => Outline::Rounded {
                w: w * scale,
                h: h * scale,
                r: r * scale,
            },
            Outline::Ellipse { w, h } => Outline::Ellipse {
                w: w * scale,
                h: h * scale,
            },
        };
        let (cx, cy) = (x as f32 + w as f32 / 2.0, y as f32 + h as f32 / 2.0);
        for py in y..y + h {
            for px in x..x + w {
                let d = outline.distance(cx, cy, px as f32 + 0.5, py as f32 + 0.5);
                if (-1.0..0.0).contains(&d) {
                    f.put(px, py, BLACK);
                }
            }
        }
    }
}

/// Um cartão do painel: uma forma ou a troca entre botão e adesivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    Shape(Kind),
    /// "Adesivo" num botão, "Botão" num adesivo.
    Switch,
}

/// Cartão `k` de `n` na faixa de opções, centrados.
pub fn card_rect(k: usize, n: usize) -> (i32, i32, i32, i32) {
    let n = n.max(1) as i32;
    let gap = 8;
    let w = ((STRIP.2 - gap * (n - 1)) / n).min(96);
    let x0 = STRIP.0 + (STRIP.2 - (w * n + gap * (n - 1))) / 2;
    (x0 + (w + gap) * k as i32, STRIP.1 + 4, w, STRIP.3 - 8)
}

/// Estado do botão/adesivo.
#[derive(Debug, Clone)]
pub struct Button {
    pub kind: Kind,
    /// O painel de escolha da forma está aberto na faixa.
    pub asking: bool,
}

impl Button {
    pub fn new(sticker: bool) -> Button {
        Button {
            kind: if sticker { Kind::Rect } else { Kind::Round },
            asking: true,
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn sticker(&self) -> bool {
        self.kind.is_sticker()
    }

    /// Cartões do painel: as formas do tipo atual e a troca de tipo.
    pub fn cards(&self) -> Vec<Card> {
        let shapes: &[Kind] = if self.sticker() {
            &Kind::STICKERS
        } else {
            &Kind::BUTTONS
        };
        let mut v: Vec<Card> = shapes.iter().map(|&k| Card::Shape(k)).collect();
        v.push(Card::Switch);
        v
    }

    /// Instala a máscara da forma e deixa branco o que está fora dela.
    pub fn prepare(&self, c: &mut Canvas) {
        let mask = self.kind().mask();
        for (p, &m) in c.px.iter_mut().zip(&mask) {
            if !m {
                *p = WHITE;
            }
        }
        c.clip_mask = Some(mask);
    }

    /// Troca a forma; o que ficaria fora da nova forma vira branco (o ovo desfaz).
    pub fn set_kind(&mut self, kind: Kind, c: &mut Canvas, h: &mut History) -> bool {
        if kind == self.kind() {
            return false;
        }
        self.kind = kind;
        let before = c.px.clone();
        self.prepare(c);
        let changed = c.px != before;
        if changed {
            h.record_previous(before);
        }
        changed
    }

    /// Clique: nicho abre/fecha o painel; com o painel aberto, um cartão
    /// escolhe a forma. `None` se não era do projeto.
    pub fn click(&mut self, x: i32, y: i32, c: &mut Canvas, h: &mut History) -> Option<bool> {
        if inside(NICHE, x, y) {
            self.asking = !self.asking;
            return Some(false);
        }
        if self.asking && inside(STRIP, x, y) {
            let cards = self.cards();
            let hit = (0..cards.len()).find(|&k| inside(card_rect(k, cards.len()), x, y));
            let Some(k) = hit else {
                return Some(false);
            };
            return Some(match cards[k] {
                Card::Shape(kind) => {
                    self.asking = false;
                    self.set_kind(kind, c, h)
                }
                // a troca de tipo deixa o painel aberto, para escolher a forma
                Card::Switch => {
                    let kind = if self.sticker() {
                        Kind::Round
                    } else {
                        Kind::Rect
                    };
                    self.set_kind(kind, c, h)
                }
            });
        }
        self.asking = false;
        None
    }

    /// Painel de escolha: um cartão por forma (miniatura e rótulo), o
    /// escolhido marcado, e o cartão de troca entre botão e adesivo.
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        let cards = self.cards();
        for (k, c) in cards.iter().enumerate() {
            let r = card_rect(k, cards.len());
            match c {
                Card::Shape(kind) => {
                    let label_h = a.font(8).map_or(8, |f| f.height as i32);
                    // miniatura em cima, rótulo embaixo
                    let (gw, gh) = (26, r.3 - label_h - 8);
                    card(f, a, (r.0, r.1, r.2, r.3), "", *kind == self.kind());
                    kind.glyph(f, r.0 + (r.2 - gw) / 2, r.1 + 3, gw, gh);
                    label(
                        f,
                        a,
                        (r.0, r.1 + r.3 - label_h - 4, r.2, label_h),
                        kind.label(),
                    );
                }
                Card::Switch => {
                    let text = if self.sticker() { "Botão" } else { "Adesivo" };
                    card(f, a, r, text, false);
                }
            }
        }
    }

    /// Contorno por cima do quadro, o cinza em volta da forma e o rótulo
    /// da forma no nicho.
    pub fn draw_overlay(&self, f: &mut Frame, _a: &Assets) {
        let kind = self.kind();
        for y in 0..CANVAS_H as i32 {
            for x in 0..CANVAS_W as i32 {
                let d = kind.distance(x, y);
                if d >= 0.0 {
                    f.put(ORIGIN.0 + x, ORIGIN.1 + y, OUTSIDE);
                } else if d >= -BORDER {
                    f.put(ORIGIN.0 + x, ORIGIN.1 + y, BLACK);
                }
            }
        }
        clear_niche(f);
        let r = (NICHE.0 + 4, NICHE.1 + 6, NICHE.2 - 8, NICHE.3 - 12);
        f.rect(r.0, r.1, r.2, r.3, super::WHITE);
        f.frame_rect(r.0, r.1, r.2, r.3, 1, BLACK);
        kind.glyph(f, r.0 + 4, r.1 + 4, r.2 - 8, r.3 - 8);
    }

    /// Cópia do quadro com o contorno em índices da paleta.
    pub fn compose(&self, c: &Canvas, _pal: &[Rgb]) -> Canvas {
        let kind = self.kind();
        let mut out = c.clone();
        out.clip = None;
        out.clip_mask = None;
        let (bx, by, bw, bh) = kind.bbox();
        for y in by - 1..by + bh + 1 {
            for x in bx - 1..bx + bw + 1 {
                if kind.rim_at(x, y).is_some() {
                    out.set(x, y, INK_BLACK);
                }
            }
        }
        out
    }

    /// Folha de impressão com as cópias (2×3 botões ou 3×4 adesivos), cada
    /// uma reduzida para caber na sua célula.
    pub fn print_sheet(&self, c: &Canvas, pal: &[Rgb]) -> Canvas {
        let src = self.compose(c, pal);
        let kind = self.kind();
        let (bx, by, bw, bh) = kind.bbox();
        let (cols, rows) = kind.sheet_grid();
        let cell_w = PAGE_W / cols;
        let cell_h = PAGE_H / rows;
        let scale = (cell_w as f32 / bw as f32)
            .min(cell_h as f32 / bh as f32)
            .min(1.0);
        let dw = (bw as f32 * scale) as u32;
        let dh = (bh as f32 * scale) as u32;
        let mut page = Canvas::new(PAGE_W, PAGE_H);
        for r in 0..rows {
            for col in 0..cols {
                let x0 = (col * cell_w + (cell_w - dw) / 2) as i32;
                let y0 = (r * cell_h + (cell_h - dh) / 2) as i32;
                for dy in 0..dh {
                    for dx in 0..dw {
                        let sx = bx + (dx as f32 / scale) as i32;
                        let sy = by + (dy as f32 / scale) as i32;
                        if let Some(v) = src.get(sx, sy) {
                            page.set(x0 + dx as i32, y0 + dy as i32, v);
                        }
                    }
                }
            }
        }
        page
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Ink;

    fn pal() -> Vec<Rgb> {
        let mut p = vec![
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            Rgb { r: 0, g: 0, b: 0 },
            Rgb {
                r: 140,
                g: 140,
                b: 140,
            },
        ];
        p.resize(
            256,
            Rgb {
                r: 250,
                g: 0,
                b: 57,
            },
        );
        p
    }

    /// Centro da caixa da forma no quadro.
    fn center(k: Kind) -> (i32, i32) {
        let (x, y, w, h) = k.bbox();
        (x + w / 2, y + h / 2)
    }

    #[test]
    fn round_mask_is_a_disc_inside_the_outline() {
        let m = Kind::Round.mask();
        let at = |x: i32, y: i32| m[(y as u32 * CANVAS_W + x as u32) as usize];
        let (cx, cy) = center(Kind::Round);
        assert!(at(cx, cy));
        // raio 81,5: 80 px do centro ainda dentro; 81 já é o contorno
        assert!(at(cx + 79, cy));
        assert!(!at(cx + 81, cy));
        assert!(!at(0, 0));
        let n = m.iter().filter(|&&b| b).count() as f32;
        let r = 163.0 / 2.0 - BORDER;
        assert!((n - std::f32::consts::PI * r * r).abs() < 300.0, "{n}");
    }

    #[test]
    fn boxes_are_the_ones_measured_in_the_original() {
        // tela do original menos a origem do quadro (23, 109)
        assert_eq!(Kind::Round.bbox(), (207, 77, 163, 163));
        assert_eq!(Kind::Square.bbox(), (220, 100, 146, 146));
        assert_eq!(Kind::Rect.bbox(), (193, 87, 218, 164));
        assert_eq!(Kind::StickerRound.bbox(), (201, 71, 199, 199));
        assert_eq!(Kind::StickerSquare.bbox(), (201, 71, 200, 200));
        assert_eq!(Kind::TallOval.bbox(), (209, 71, 163, 199));
        assert_eq!(Kind::WideOval.bbox(), (201, 87, 199, 163));
        for k in Kind::STICKERS {
            assert!(k.is_sticker());
            assert_eq!(k.sheet_grid(), (3, 4));
            assert_eq!(Kind::from_file_name(k.file_name()), Some(k));
        }
        for k in Kind::BUTTONS {
            assert!(!k.is_sticker());
            assert_eq!(Kind::from_file_name(k.file_name()), Some(k));
        }
        assert_eq!(Kind::from_file_name("x"), None);
    }

    #[test]
    fn squares_have_sharp_corners_and_a_one_pixel_outline() {
        let (x, y, w, h) = Kind::Rect.bbox();
        assert_eq!(Kind::Rect.rim_at(x, y), Some(true));
        assert_eq!(Kind::Rect.rim_at(x + w - 1, y + h - 1), Some(true));
        assert_eq!(Kind::Rect.rim_at(x + 1, y + 1), None);
        assert_eq!(Kind::Rect.rim_at(x - 1, y + 5), None);
        let m = Kind::Rect.mask();
        assert!(m[((y + 1) as u32 * CANVAS_W + (x + 1) as u32) as usize]);
        // oval alto: o topo do meio está dentro, o canto da caixa não
        let (ox, oy, ow, _) = Kind::TallOval.bbox();
        let tm = Kind::TallOval.mask();
        assert!(tm[((oy + 3) as u32 * CANVAS_W + (ox + ow / 2) as u32) as usize]);
        assert!(!tm[((oy + 3) as u32 * CANVAS_W + (ox + 3) as u32) as usize]);
    }

    #[test]
    fn prepare_whitens_outside_and_clips_painting() {
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(5);
        let b = Button::new(false);
        b.prepare(&mut c);
        assert_eq!(c.get(0, 0), Some(WHITE));
        let (cx, cy) = center(Kind::Round);
        assert_eq!(c.get(cx, cy), Some(5));
        c.plot(1, 1, &Ink::solid(INK_BLACK));
        assert_eq!(c.get(1, 1), Some(WHITE));
    }

    #[test]
    fn set_kind_records_history_and_switches() {
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        let mut h = History::default();
        let mut b = Button::new(false);
        b.prepare(&mut c);
        let (cx, cy) = center(Kind::Round);
        c.plot(cx, cy, &Ink::solid(INK_BLACK));
        assert!(!b.set_kind(Kind::Round, &mut c, &mut h));
        // o pixel do centro cabe no adesivo: a troca não muda o quadro
        assert!(!b.set_kind(Kind::Rect, &mut c, &mut h));
        assert_eq!(b.kind(), Kind::Rect);
        assert_eq!(c.get(cx, cy), Some(INK_BLACK));
        // perto da borda esquerda do retângulo: some ao virar botão redondo
        let (rx, ry, _, rh) = Kind::Rect.bbox();
        let (px, py) = (rx + 5, ry + rh / 2);
        c.plot(px, py, &Ink::solid(INK_BLACK));
        assert!(b.set_kind(Kind::Round, &mut c, &mut h));
        assert_eq!(c.get(px, py), Some(WHITE));
        assert!(h.toggle(&mut c, &mut crate::paint::Objects::default()));
        assert_eq!(c.get(px, py), Some(INK_BLACK));
    }

    #[test]
    fn click_cards_and_niche() {
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        let mut h = History::default();
        let mut b = Button::new(false);
        assert!(b.asking);
        assert_eq!(b.cards().len(), 3);
        let r = card_rect(1, 3);
        assert_eq!(b.click(r.0 + 3, r.1 + 3, &mut c, &mut h), Some(false));
        assert_eq!(b.kind(), Kind::Square);
        assert!(!b.asking);
        assert_eq!(b.click(r.0 + 3, r.1 + 3, &mut c, &mut h), None);
        assert_eq!(
            b.click(NICHE.0 + 1, NICHE.1 + 1, &mut c, &mut h),
            Some(false)
        );
        assert!(b.asking);
        // "Adesivo" troca o tipo e deixa o painel aberto, agora com 6 cartões
        let r = card_rect(2, 3);
        assert_eq!(b.click(r.0 + 3, r.1 + 3, &mut c, &mut h), Some(false));
        assert_eq!(b.kind(), Kind::Rect);
        assert!(b.asking);
        assert_eq!(b.cards().len(), 6);
        let r = card_rect(3, 6);
        assert_eq!(b.click(r.0 + 3, r.1 + 3, &mut c, &mut h), Some(false));
        assert_eq!(b.kind(), Kind::TallOval);
        assert!(!b.asking);
        for n in [3, 6] {
            let last = card_rect(n - 1, n);
            assert!(last.0 + last.2 <= STRIP.0 + STRIP.2);
            assert!(card_rect(0, n).0 >= STRIP.0);
        }
        // de volta a botão: redondo
        b.asking = true;
        let r = card_rect(5, 6);
        assert_eq!(b.click(r.0 + 3, r.1 + 3, &mut c, &mut h), Some(false));
        assert_eq!(b.kind(), Kind::Round);
    }

    #[test]
    fn print_sheet_has_six_button_copies() {
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        let b = Button::new(false);
        b.prepare(&mut c);
        let (cx, cy) = center(Kind::Round);
        c.plot(cx, cy, &Ink::solid(INK_BLACK));
        let page = b.print_sheet(&c, &pal());
        assert_eq!((page.w, page.h), (PAGE_W, PAGE_H));
        // seis centros pretos, cada um numa célula de 413×389
        let mut centers = 0;
        for r in 0..3 {
            for col in 0..2 {
                let x = col * 413 + 413 / 2;
                let y = r * 389 + 389 / 2;
                let hit = (-1..=1)
                    .any(|dx| (-1..=1).any(|dy| page.get(x + dx, y + dy) == Some(INK_BLACK)));
                if hit {
                    centers += 1;
                }
            }
        }
        assert_eq!(centers, 6);
        // contornos pretos de 1 px: 6 círculos de perímetro ~512 px
        let black = page.px.iter().filter(|&&v| v == INK_BLACK).count();
        assert!(black > 6 * 400 && black < 6 * 1500, "{black}");
    }

    #[test]
    fn print_sheet_sticker_is_reduced_to_twelve_copies() {
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        let b = Button::new(true);
        b.prepare(&mut c);
        let page = b.print_sheet(&c, &pal());
        // cada cópia de 218×164 cabe inteira numa célula de 275×292
        let cell_w = PAGE_W / 3;
        let row_y = (PAGE_H / 4 / 2) as i32;
        let mut copies = 0;
        for col in 0..3 {
            let x0 = (col * cell_w) as i32;
            let n = (x0..x0 + cell_w as i32)
                .filter(|&x| page.get(x, row_y) == Some(INK_BLACK))
                .count();
            // as duas bordas de 1 px
            if n >= 2 {
                copies += 1;
            }
        }
        assert_eq!(copies, 3);
        let black = page.px.iter().filter(|&&v| v == INK_BLACK).count();
        assert!(black > 12 * 600 && black < 12 * 1500, "{black}");
    }
}
