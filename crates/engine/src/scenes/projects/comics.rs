//! História em quadrinhos: molduras sobre o quadro e recorte ao quadrinho.
//!
//! O original permite "até quatro painéis de comprimento" (roteiro de ajuda
//! `WHAT` 3e00 do `ARTIST.EXE`). As molduras são linhas pretas de 3 px
//! desenhadas por cima do quadro (overlay): não entram no desenho guardado.

use super::{BLACK, NICHE, RED, STRIP, clear_niche, inside, pic};
use crate::Frame;
use crate::assets::Assets;
use crate::paint::{BLACK as INK_BLACK, CANVAS_H, CANVAS_W, Canvas, ORIGIN};

/// Número máximo de quadrinhos.
pub const MAX_PANELS: usize = 4;
/// Margem até a borda do quadro.
pub const MARGIN: i32 = 6;
/// Calha entre quadrinhos.
pub const GUTTER: i32 = 8;
/// Espessura da moldura.
pub const LINE: i32 = 3;

/// Dígitos 1–4 do `ARTIST.PIC`: (colorido, mono, máscara). O 1 não tem
/// versão colorida.
const DIGITS: [(Option<u16>, u16, u16); MAX_PANELS] = [
    (None, 0x3a78, 0x3a79),
    (Some(0x3a7b), 0x3a7c, 0x3a7d),
    (Some(0x3a7f), 0x3a80, 0x3a81),
    (Some(0x3a83), 0x3a84, 0x3a85),
];
/// Largura de cada casa dos dígitos na faixa.
const SLOT_W: i32 = 48;

/// Casa do dígito `k` (0-based) na faixa de opções, centrada.
pub fn digit_rect(k: usize) -> (i32, i32, i32, i32) {
    let x0 = STRIP.0 + (STRIP.2 - SLOT_W * MAX_PANELS as i32) / 2;
    (x0 + SLOT_W * k as i32, STRIP.1, SLOT_W, STRIP.3)
}

/// Retângulos `(x, y, w, h)` dos quadrinhos (coordenadas do quadro, com a
/// moldura incluída), para `n` quadrinhos: 1 inteiro; 2 lado a lado;
/// 3 colunas; 4 = 2×2.
pub fn layout(n: usize) -> Vec<(i32, i32, i32, i32)> {
    let (cols, rows) = match n.clamp(1, MAX_PANELS) {
        1 => (1, 1),
        2 => (2, 1),
        3 => (3, 1),
        _ => (2, 2),
    };
    let area_w = CANVAS_W as i32 - 2 * MARGIN;
    let area_h = CANVAS_H as i32 - 2 * MARGIN;
    let w = (area_w - GUTTER * (cols - 1)) / cols;
    let h = (area_h - GUTTER * (rows - 1)) / rows;
    let mut out = Vec::with_capacity(n);
    for r in 0..rows {
        for c in 0..cols {
            out.push((MARGIN + c * (w + GUTTER), MARGIN + r * (h + GUTTER), w, h));
        }
    }
    out
}

/// Interior de um quadrinho (sem a moldura): o recorte da pintura.
fn interior(r: (i32, i32, i32, i32)) -> (i32, i32, u32, u32) {
    (
        r.0 + LINE,
        r.1 + LINE,
        (r.2 - 2 * LINE).max(0) as u32,
        (r.3 - 2 * LINE).max(0) as u32,
    )
}

/// Estado da história em quadrinhos.
#[derive(Debug, Clone)]
pub struct Comics {
    /// Número de quadrinhos (1 a 4).
    pub panels: usize,
    /// O painel "quantos quadrinhos?" está aberto na faixa.
    pub asking: bool,
}

impl Default for Comics {
    fn default() -> Comics {
        Comics {
            panels: 1,
            asking: true,
        }
    }
}

impl Comics {
    pub fn set_panels(&mut self, n: usize) {
        self.panels = n.clamp(1, MAX_PANELS);
    }

    pub fn rects(&self) -> Vec<(i32, i32, i32, i32)> {
        layout(self.panels)
    }

    /// Recorte do quadrinho que contém `(cx, cy)`; na calha ou na margem,
    /// recorte vazio (nada pinta).
    pub fn clip_for(&self, cx: i32, cy: i32) -> (i32, i32, u32, u32) {
        self.rects()
            .into_iter()
            .map(interior)
            .find(|&(x, y, w, h)| cx >= x && cy >= y && cx < x + w as i32 && cy < y + h as i32)
            .unwrap_or((0, 0, 0, 0))
    }

    pub fn clip_at(&self, c: &mut Canvas, cx: i32, cy: i32) {
        c.clip = Some(self.clip_for(cx, cy));
    }

    /// Clique: nicho abre/fecha o painel; com o painel aberto, um dígito
    /// escolhe o número de quadrinhos. `None` se não era do projeto.
    pub fn click(&mut self, x: i32, y: i32) -> Option<bool> {
        if inside(NICHE, x, y) {
            self.asking = !self.asking;
            return Some(false);
        }
        if self.asking && inside(STRIP, x, y) {
            if let Some(k) = (0..MAX_PANELS).find(|&k| inside(digit_rect(k), x, y)) {
                self.set_panels(k + 1);
                self.asking = false;
            }
            return Some(false);
        }
        self.asking = false;
        None
    }

    fn blit_digit(
        &self,
        f: &mut Frame,
        a: &Assets,
        k: usize,
        r: (i32, i32, i32, i32),
        color: bool,
    ) {
        let (col, mono, mask) = DIGITS[k];
        let id = if color { col.unwrap_or(mono) } else { mono };
        let img = a.image(&pic(id));
        let m = a.image(&pic(mask));
        let x = r.0 + (r.2 - img.width as i32) / 2;
        let y = r.1 + (r.3 - img.height as i32) / 2;
        f.blit_masked(&img, &m, x, y);
    }

    /// Painel "quantos quadrinhos?": dígitos 1–4, o escolhido em cor.
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        for k in 0..MAX_PANELS {
            let r = digit_rect(k);
            let chosen = k + 1 == self.panels;
            self.blit_digit(f, a, k, r, chosen);
            if chosen {
                f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, RED);
            }
        }
    }

    /// Molduras por cima do quadro e o dígito atual no nicho.
    pub fn draw_overlay(&self, f: &mut Frame, a: &Assets) {
        for (x, y, w, h) in self.rects() {
            f.frame_rect(ORIGIN.0 + x, ORIGIN.1 + y, w, h, LINE, BLACK);
        }
        clear_niche(f);
        self.blit_digit(f, a, self.panels - 1, NICHE, false);
    }

    /// Cópia do quadro com as molduras em preto (para imprimir).
    pub fn compose(&self, c: &Canvas) -> Canvas {
        let mut out = c.clone();
        out.clip = None;
        out.clip_mask = None;
        for (x, y, w, h) in self.rects() {
            for yy in y..y + h {
                for xx in x..x + w {
                    let edge =
                        xx < x + LINE || xx >= x + w - LINE || yy < y + LINE || yy >= y + h - LINE;
                    if edge {
                        out.set(xx, yy, INK_BLACK);
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_fit_in_canvas_with_margin_and_gutter() {
        for n in 1..=MAX_PANELS {
            let rs = layout(n);
            assert_eq!(rs.len(), n);
            for &(x, y, w, h) in &rs {
                assert!(x >= MARGIN && y >= MARGIN);
                assert!(x + w <= CANVAS_W as i32 - MARGIN);
                assert!(y + h <= CANVAS_H as i32 - MARGIN);
            }
        }
        let two = layout(2);
        assert_eq!(two[1].0 - (two[0].0 + two[0].2), GUTTER);
        let four = layout(4);
        assert_eq!(four[2].1 - (four[0].1 + four[0].3), GUTTER);
        assert_eq!(four[0].0, four[2].0);
        assert_eq!(layout(1)[0], (6, 6, 582, 335));
        assert_eq!(layout(9).len(), MAX_PANELS);
    }

    #[test]
    fn clip_is_panel_interior_or_empty_in_gutter() {
        let mut k = Comics::default();
        k.set_panels(4);
        let r = layout(4)[0];
        assert_eq!(
            k.clip_for(r.0 + 10, r.1 + 10),
            (r.0 + LINE, r.1 + LINE, (r.2 - 6) as u32, (r.3 - 6) as u32)
        );
        // calha entre as duas colunas
        assert_eq!(k.clip_for(r.0 + r.2 + 4, r.1 + 10), (0, 0, 0, 0));
        // em cima da própria moldura também não pinta
        assert_eq!(k.clip_for(r.0 + 1, r.1 + 1), (0, 0, 0, 0));
        assert_eq!(k.clip_for(0, 0), (0, 0, 0, 0));
    }

    #[test]
    fn click_digits_and_niche() {
        let mut k = Comics::default();
        assert!(k.asking);
        let r = digit_rect(3);
        assert_eq!(k.click(r.0 + 5, r.1 + 5), Some(false));
        assert_eq!(k.panels, 4);
        assert!(!k.asking);
        // fechado, a faixa segue para as opções da ferramenta
        assert_eq!(k.click(r.0 + 5, r.1 + 5), None);
        assert_eq!(k.click(NICHE.0 + 3, NICHE.1 + 3), Some(false));
        assert!(k.asking);
        // clique fora fecha o painel sem tratar
        assert_eq!(k.click(300, 300), None);
        assert!(!k.asking);
        assert!(digit_rect(0).0 > STRIP.0);
        assert!(digit_rect(3).0 + SLOT_W < STRIP.0 + STRIP.2);
    }

    #[test]
    fn compose_draws_black_frames_only_on_edges() {
        let mut k = Comics::default();
        k.set_panels(2);
        let c = Canvas::new(CANVAS_W, CANVAS_H);
        let out = k.compose(&c);
        let r = layout(2)[0];
        assert_eq!(out.get(r.0, r.1), Some(INK_BLACK));
        assert_eq!(out.get(r.0 + 2, r.1 + 100), Some(INK_BLACK));
        assert_eq!(out.get(r.0 + 3, r.1 + 100), Some(0));
        assert_eq!(out.get(0, 0), Some(0));
        let black = out.px.iter().filter(|&&v| v == INK_BLACK).count();
        let per = 2 * (r.2 * LINE) + 2 * ((r.3 - 2 * LINE) * LINE);
        assert_eq!(black, 2 * per as usize);
    }
}
