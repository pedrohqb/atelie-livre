//! Botão ou adesivo: forma recortada no quadro e folha de impressão.
//!
//! O original pergunta "Redondo ou Quadrado" para o botão (roteiro de ajuda
//! `WHAT` 3c00 do `ARTIST.EXE`); o adesivo é a terceira opção. A forma é
//! overlay por cima do quadro; a pintura fica limitada à máscara de recorte
//! (`Canvas::clip_mask`) e fora dela o quadro é branco.

use super::{BLACK, GRAY, NICHE, STRIP, card, clear_niche, inside};
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

/// As três formas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Botão redondo: círculo de 300 px.
    Round,
    /// Botão quadrado: 300×300, cantos de raio 30.
    Square,
    /// Adesivo: 360×260, cantos de raio 24.
    Sticker,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Round, Kind::Square, Kind::Sticker];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Round => "Botão redondo",
            Kind::Square => "Botão quadrado",
            Kind::Sticker => "Adesivo",
        }
    }

    /// Largura, altura e raio dos cantos da forma.
    fn geometry(self) -> (f32, f32, f32) {
        match self {
            Kind::Round => (300.0, 300.0, 150.0),
            Kind::Square => (300.0, 300.0, 30.0),
            Kind::Sticker => (360.0, 260.0, 24.0),
        }
    }

    /// Cópias por folha de impressão: (colunas, linhas).
    pub fn sheet_grid(self) -> (u32, u32) {
        match self {
            Kind::Round | Kind::Square => (2, 3),
            Kind::Sticker => (3, 4),
        }
    }

    /// Caixa `(x, y, w, h)` da forma no quadro, centrada.
    pub fn bbox(self) -> (i32, i32, i32, i32) {
        let (w, h, _) = self.geometry();
        let x = ((CANVAS_W as f32 - w) / 2.0).floor() as i32;
        let y = ((CANVAS_H as f32 - h) / 2.0).floor() as i32;
        (x, y, w as i32, h as i32)
    }

    /// Distância com sinal do centro do pixel `(x, y)` à borda da forma
    /// (negativa dentro).
    fn distance(self, x: i32, y: i32) -> f32 {
        let (w, h, r) = self.geometry();
        let cx = CANVAS_W as f32 / 2.0;
        let cy = CANVAS_H as f32 / 2.0;
        let dx = ((x as f32 + 0.5 - cx).abs() - (w / 2.0 - r)).max(0.0);
        let dy = ((y as f32 + 0.5 - cy).abs() - (h / 2.0 - r)).max(0.0);
        (dx * dx + dy * dy).sqrt() - r
    }

    /// Largura da moldura (borda + anel) que não se pinta.
    fn rim(self) -> f32 {
        match self {
            Kind::Sticker => BORDER,
            _ => BORDER + RING,
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
}

/// Cartão `k` da faixa de opções.
pub fn card_rect(k: usize) -> (i32, i32, i32, i32) {
    const W: i32 = 96;
    const PITCH: i32 = 104;
    let x0 = STRIP.0 + (STRIP.2 - (PITCH * 2 + W)) / 2;
    (x0 + PITCH * k as i32, STRIP.1 + 6, W, STRIP.3 - 12)
}

/// Estado do botão/adesivo.
#[derive(Debug, Clone)]
pub struct Button {
    /// `true` = adesivo retangular; `false` = botão.
    pub sticker: bool,
    /// Botão quadrado (só vale quando não é adesivo).
    pub square: bool,
    /// O painel de escolha da forma está aberto na faixa.
    pub asking: bool,
}

impl Button {
    pub fn new(sticker: bool) -> Button {
        Button {
            sticker,
            square: false,
            asking: true,
        }
    }

    pub fn kind(&self) -> Kind {
        match (self.sticker, self.square) {
            (true, _) => Kind::Sticker,
            (false, true) => Kind::Square,
            (false, false) => Kind::Round,
        }
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
        self.sticker = kind == Kind::Sticker;
        self.square = kind == Kind::Square;
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
            let hit = (0..Kind::ALL.len()).find(|&k| inside(card_rect(k), x, y));
            let Some(k) = hit else {
                return Some(false);
            };
            self.asking = false;
            return Some(self.set_kind(Kind::ALL[k], c, h));
        }
        self.asking = false;
        None
    }

    /// Painel de escolha: três cartões, o escolhido marcado.
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        for (k, kind) in Kind::ALL.iter().enumerate() {
            card(f, a, card_rect(k), kind.label(), *kind == self.kind());
        }
    }

    /// Borda e anel por cima do quadro e o rótulo da forma no nicho.
    pub fn draw_overlay(&self, f: &mut Frame, a: &Assets) {
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
        let label = match kind {
            Kind::Round => "Redondo",
            Kind::Square => "Quadrado",
            Kind::Sticker => "Adesivo",
        };
        clear_niche(f);
        card(
            f,
            a,
            (NICHE.0 + 2, NICHE.1 + 12, NICHE.2 - 4, NICHE.3 - 24),
            label,
            false,
        );
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
    fn square_and_sticker_boxes_are_centered() {
        assert_eq!(Kind::Round.bbox(), (147, 23, 300, 300));
        assert_eq!(Kind::Square.bbox(), (147, 23, 300, 300));
        assert_eq!(Kind::Sticker.bbox(), (117, 43, 360, 260));
        let m = Kind::Sticker.mask();
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
        assert_eq!(Kind::Sticker.rim_at(117 + 1, 173), Some(true));
        assert_eq!(Kind::Sticker.rim_at(117 + 6, 173), None);
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
        assert!(!b.set_kind(Kind::Sticker, &mut c, &mut h));
        assert_eq!(b.kind(), Kind::Sticker);
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
        let r = card_rect(1);
        assert_eq!(b.click(r.0 + 3, r.1 + 3, &mut c, &mut h), Some(false));
        assert_eq!(b.kind(), Kind::Square);
        assert!(!b.asking);
        assert_eq!(b.click(r.0 + 3, r.1 + 3, &mut c, &mut h), None);
        assert_eq!(
            b.click(NICHE.0 + 1, NICHE.1 + 1, &mut c, &mut h),
            Some(false)
        );
        assert!(b.asking);
        let r = card_rect(2);
        assert_eq!(b.click(r.0 + 3, r.1 + 3, &mut c, &mut h), Some(false));
        assert_eq!(b.kind(), Kind::Sticker);
        assert!(card_rect(2).0 + card_rect(2).2 < STRIP.0 + STRIP.2);
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
