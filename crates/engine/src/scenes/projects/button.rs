//! Botão ou adesivo: forma recortada no quadro e folha de impressão.
//!
//! O original pergunta "Redondo ou Quadrado" para o botão e "Retangular,
//! Redondo, Quadrado, Oval alto ou Oval largo" para o adesivo (roteiro de
//! ajuda `WHAT` 3c00 do `ARTIST.EXE`, registros 4 e 26; os bitmaps das
//! formas que ele cita não existem no `ARTIST.PIC`, por isso as formas são
//! geométricas). A forma é overlay por cima do quadro; a pintura fica
//! limitada à máscara de recorte (`Canvas::clip_mask`) e fora dela o quadro
//! é branco. A forma escolhida fica gravada em `ultimo-<tipo>.projeto.txt`.

use super::{BLACK, GRAY, NICHE, STRIP, card, clear_niche, inside, label};
use crate::Frame;
use crate::assets::Assets;
use crate::paint::store::nearest;
use crate::paint::{BLACK as INK_BLACK, CANVAS_H, CANVAS_W, Canvas, History, ORIGIN, WHITE};
use formats::palette::Rgb;

/// Espessura da borda preta.
pub const BORDER: f32 = 3.0;
/// Largura do anel cinza (a "aba" de metal) dos botões.
pub const RING: f32 = 12.0;
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
                let dx = ((px - cx).abs() - (w / 2.0 - r)).max(0.0);
                let dy = ((py - cy).abs() - (h / 2.0 - r)).max(0.0);
                (dx * dx + dy * dy).sqrt() - r
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
    /// Botão redondo: círculo de 300 px.
    Round,
    /// Botão quadrado: 300×300, cantos de raio 30.
    Square,
    /// Adesivo retangular: 360×260, cantos de raio 24.
    Rect,
    /// Adesivo redondo: círculo de 300 px, sem aba.
    StickerRound,
    /// Adesivo quadrado: 300×300, cantos de raio 30, sem aba.
    StickerSquare,
    /// Adesivo oval alto: elipse de 260×318 (proporção 162:198 do original).
    TallOval,
    /// Adesivo oval largo: elipse de 318×260.
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
        match self {
            Kind::Round | Kind::StickerRound => Outline::Rounded {
                w: 300.0,
                h: 300.0,
                r: 150.0,
            },
            Kind::Square | Kind::StickerSquare => Outline::Rounded {
                w: 300.0,
                h: 300.0,
                r: 30.0,
            },
            Kind::Rect => Outline::Rounded {
                w: 360.0,
                h: 260.0,
                r: 24.0,
            },
            Kind::TallOval => Outline::Ellipse { w: 260.0, h: 318.0 },
            Kind::WideOval => Outline::Ellipse { w: 318.0, h: 260.0 },
        }
    }

    /// Cópias por folha de impressão: (colunas, linhas).
    pub fn sheet_grid(self) -> (u32, u32) {
        if self.is_sticker() { (3, 4) } else { (2, 3) }
    }

    /// Caixa `(x, y, w, h)` da forma no quadro, centrada.
    pub fn bbox(self) -> (i32, i32, i32, i32) {
        let (w, h) = self.outline().size();
        let x = ((CANVAS_W as f32 - w) / 2.0).floor() as i32;
        let y = ((CANVAS_H as f32 - h) / 2.0).floor() as i32;
        (x, y, w as i32, h as i32)
    }

    /// Distância com sinal do centro do pixel `(x, y)` à borda da forma
    /// (negativa dentro).
    fn distance(self, x: i32, y: i32) -> f32 {
        let cx = CANVAS_W as f32 / 2.0;
        let cy = CANVAS_H as f32 / 2.0;
        self.outline()
            .distance(cx, cy, x as f32 + 0.5, y as f32 + 0.5)
    }

    /// Largura da moldura (borda + anel) que não se pinta.
    fn rim(self) -> f32 {
        if self.is_sticker() {
            BORDER
        } else {
            BORDER + RING
        }
    }

    /// Pixel da moldura: `Some(true)` borda preta, `Some(false)` anel cinza,
    /// `None` fora da moldura.
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

    /// Borda e anel por cima do quadro e o rótulo da forma no nicho.
    pub fn draw_overlay(&self, f: &mut Frame, _a: &Assets) {
        let kind = self.kind();
        let (bx, by, bw, bh) = kind.bbox();
        for y in by - 1..by + bh + 1 {
            for x in bx - 1..bx + bw + 1 {
                if let Some(border) = kind.rim_at(x, y) {
                    f.put(
                        ORIGIN.0 + x,
                        ORIGIN.1 + y,
                        if border { BLACK } else { GRAY },
                    );
                }
            }
        }
        clear_niche(f);
        let r = (NICHE.0 + 4, NICHE.1 + 6, NICHE.2 - 8, NICHE.3 - 12);
        f.rect(r.0, r.1, r.2, r.3, super::WHITE);
        f.frame_rect(r.0, r.1, r.2, r.3, 1, BLACK);
        kind.glyph(f, r.0 + 4, r.1 + 4, r.2 - 8, r.3 - 8);
    }

    /// Cópia do quadro com a borda e o anel em índices da paleta.
    pub fn compose(&self, c: &Canvas, pal: &[Rgb]) -> Canvas {
        let gray = nearest(pal, GRAY[0], GRAY[1], GRAY[2]);
        let kind = self.kind();
        let mut out = c.clone();
        out.clip = None;
        out.clip_mask = None;
        let (bx, by, bw, bh) = kind.bbox();
        for y in by - 1..by + bh + 1 {
            for x in bx - 1..bx + bw + 1 {
                if let Some(border) = kind.rim_at(x, y) {
                    out.set(x, y, if border { INK_BLACK } else { gray });
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

    #[test]
    fn round_mask_is_a_disc_inside_the_rim() {
        let m = Kind::Round.mask();
        let at = |x: i32, y: i32| m[(y as u32 * CANVAS_W + x as u32) as usize];
        assert!(at(297, 173));
        // 134 px do centro ainda dentro; 136 já é anel
        assert!(at(297 + 134, 173));
        assert!(!at(297 + 136, 173));
        assert!(!at(0, 0));
        assert!(!at(297, 173 - 149));
        let n = m.iter().filter(|&&b| b).count() as f32;
        let r = 150.0 - BORDER - RING;
        assert!((n - std::f32::consts::PI * r * r).abs() < 600.0);
    }

    #[test]
    fn sticker_shapes_have_their_own_outlines() {
        assert_eq!(Kind::StickerRound.bbox(), (147, 23, 300, 300));
        assert_eq!(Kind::StickerSquare.bbox(), (147, 23, 300, 300));
        assert_eq!(Kind::TallOval.bbox(), (167, 14, 260, 318));
        assert_eq!(Kind::WideOval.bbox(), (138, 43, 318, 260));
        let at = |k: Kind, x: i32, y: i32| k.mask()[(y as u32 * CANVAS_W + x as u32) as usize];
        // adesivo redondo: sem anel, pinta até 4 px da borda
        assert!(at(Kind::StickerRound, 297 + 145, 173));
        assert!(!at(Kind::Round, 297 + 145, 173));
        assert!(!at(Kind::StickerRound, 297 + 148, 173));
        // oval alto: o ponto (297, 14+5) está dentro, (297+128, 173) também; o canto não
        assert!(at(Kind::TallOval, 297, 14 + 5));
        assert!(at(Kind::TallOval, 297 + 125, 173));
        assert!(!at(Kind::TallOval, 297 + 110, 14 + 20));
        assert!(!at(Kind::TallOval, 167 + 2, 173));
        // oval largo é o alto deitado
        assert!(at(Kind::WideOval, 297 + 154, 173));
        assert!(!at(Kind::WideOval, 297, 43 + 1));
        assert!(at(Kind::WideOval, 297, 43 + 5));
        assert_eq!(Kind::TallOval.rim_at(167 + 1, 173), Some(true));
        assert_eq!(Kind::TallOval.rim_at(167 + 5, 173), None);
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
    fn square_and_sticker_boxes_are_centered() {
        assert_eq!(Kind::Round.bbox(), (147, 23, 300, 300));
        assert_eq!(Kind::Square.bbox(), (147, 23, 300, 300));
        assert_eq!(Kind::Rect.bbox(), (117, 43, 360, 260));
        let m = Kind::Rect.mask();
        let at = |x: i32, y: i32| m[(y as u32 * CANVAS_W + x as u32) as usize];
        assert!(at(297, 173));
        assert!(at(117 + 5, 173));
        assert!(!at(117 + 1, 173));
        // canto arredondado fica de fora
        assert!(!at(117 + 4, 43 + 4));
        let sq = Kind::Square.mask();
        assert!(!sq[(27 * CANVAS_W + 151) as usize]);
        assert!(sq[(173 * CANVAS_W + 165) as usize]);
    }

    #[test]
    fn rim_has_border_outside_and_ring_inside() {
        assert_eq!(Kind::Round.rim_at(297 + 148, 173), Some(true));
        assert_eq!(Kind::Round.rim_at(297 + 140, 173), Some(false));
        assert_eq!(Kind::Round.rim_at(297 + 130, 173), None);
        assert_eq!(Kind::Round.rim_at(297 + 160, 173), None);
        assert_eq!(Kind::Rect.rim_at(117 + 1, 173), Some(true));
        assert_eq!(Kind::Rect.rim_at(117 + 6, 173), None);
    }

    #[test]
    fn prepare_whitens_outside_and_clips_painting() {
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(5);
        let b = Button::new(false);
        b.prepare(&mut c);
        assert_eq!(c.get(0, 0), Some(WHITE));
        assert_eq!(c.get(297, 173), Some(5));
        c.plot(1, 1, &Ink::solid(INK_BLACK));
        assert_eq!(c.get(1, 1), Some(WHITE));
    }

    #[test]
    fn set_kind_records_history_and_switches() {
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        let mut h = History::default();
        let mut b = Button::new(false);
        b.prepare(&mut c);
        // pinta perto da borda esquerda do círculo: fora do adesivo (x < 117+3)
        c.plot(297 - 134, 173, &Ink::solid(INK_BLACK));
        assert_eq!(c.get(163, 173), Some(INK_BLACK));
        assert!(!b.set_kind(Kind::Round, &mut c, &mut h));
        // o pixel cabe no adesivo: a troca não muda o quadro
        assert!(!b.set_kind(Kind::Rect, &mut c, &mut h));
        assert_eq!(b.kind(), Kind::Rect);
        assert_eq!(c.get(163, 173), Some(INK_BLACK));
        // pintura longe do centro some ao virar botão quadrado? não: cabe. Vira redondo: some.
        c.plot(117 + 10, 173, &Ink::solid(INK_BLACK));
        assert!(b.set_kind(Kind::Round, &mut c, &mut h));
        assert_eq!(c.get(127, 173), Some(WHITE));
        assert!(h.toggle(&mut c, &mut crate::paint::Objects::default()));
        assert_eq!(c.get(127, 173), Some(INK_BLACK));
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
        c.plot(297, 173, &Ink::solid(INK_BLACK));
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
        // bordas pretas: ao menos 6 círculos de perímetro ~942 px
        let black = page.px.iter().filter(|&&v| v == INK_BLACK).count();
        assert!(black > 6 * 900 && black < 6 * 3000, "{black}");
        let gray = page.px.iter().filter(|&&v| v == 2).count();
        assert!(gray > 6 * 9000, "{gray}");
    }

    #[test]
    fn print_sheet_sticker_is_reduced_to_twelve_copies() {
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        let b = Button::new(true);
        b.prepare(&mut c);
        let page = b.print_sheet(&c, &pal());
        // cada cópia de 360×260 cabe numa célula de 275×292: escala 275/360
        let cell_w = PAGE_W / 3;
        let row_y = (PAGE_H / 4 / 2) as i32;
        let mut copies = 0;
        for col in 0..3 {
            let x0 = (col * cell_w) as i32;
            let n = (x0..x0 + cell_w as i32)
                .filter(|&x| page.get(x, row_y) == Some(INK_BLACK))
                .count();
            // as duas bordas de 3 px, reduzidas a ~2 px cada
            if n >= 3 {
                copies += 1;
            }
        }
        assert_eq!(copies, 3);
        let black = page.px.iter().filter(|&&v| v == INK_BLACK).count();
        assert!(black > 12 * 1500 && black < 12 * 5000, "{black}");
    }
}
