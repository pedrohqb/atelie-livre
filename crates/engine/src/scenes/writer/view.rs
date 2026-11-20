//! Vistas das folhas no Escritório (Visor de páginas, `office-ferramentas.md`
//! §8): o zoom e a arrumação das folhas na vista, sem desenho.
//!
//! Medidas no original (capturas `cw/w_page/*`, documento de duas folhas
//! "Alto" de 595 × 792 pontos):
//!
//! - a vista é x 22..616 × y 107..456 (dentro da moldura preta); cada folha
//!   tem uma moldura preta de 1 px na beira e as folhas empilhadas encostam
//!   umas nas outras (sem vão: a moldura de baixo de uma e a de cima da
//!   seguinte ficam em linhas vizinhas, `g25`, `g33`);
//! - o zoom tem 8 níveis, 25, 33, 50, 67, 75, 100, 125 e 200 %; a folha
//!   reduzida tem `⌊595·z⌋ × ⌊792·z⌋` pixels e fica centrada na janela
//!   (`x = (640 − largura) / 2`) com o topo em y 107 (`zm1`..`zm5`);
//! - o bichinho `092c` anda sobre a régua de x 278 a 399:
//!   `x = 278 + ⌊k · 121 / 7⌋` no nível `k`; num zoom que não é nível
//!   (Página inteira) ele fica no meio dos dois níveis vizinhos (`zfit`:
//!   44 % → x 303);
//! - Página inteira: a folha cabe na altura da vista (350 px, 44 %);
//!   Aumentar visão: a folha enche a largura (100 % na folha Alto);
//! - Duas páginas: pares lado a lado, encostados, centrados, no zoom atual
//!   limitado a caber em 630 px (`vduas`: 52,9 % vindo de 100 %; `u1`: o
//!   cartão em 33 %), com 8 px entre um par e o seguinte;
//! - Todas as páginas: todas na vista, no maior zoom que cabe (`vtodas`:
//!   44 %, x 58..581); Lado a lado: todas numa fileira, com 8 px em cima e
//!   embaixo (`vlado`: 42 %, x 70..569, y 115..448) e 12 px dos lados;
//! - as setas da barra de baixo rolam 36 px por clique (`t0` → `t1`).

use super::editor::{ORIGIN, PAGE, PAGE_GAP};
use crate::ui::Rect;

/// Área da vista no original, dentro da moldura.
pub const AREA: Rect = (22, 107, 595, 350);
/// As folhas menores que a vista ficam centradas na janela.
pub const SCREEN_W: i32 = 640;
/// Níveis de zoom (por cento), da régua `0885`.
pub const LEVELS: [u16; 8] = [25, 33, 50, 67, 75, 100, 125, 200];
/// Nível da Visão normal (1 ponto = 1 pixel).
pub const NORMAL_LEVEL: usize = 5;
/// Vão entre dois pares de folhas na vista Duas páginas.
pub const PAIR_GAP: i32 = 8;
/// A dupla da vista Duas páginas cabe em `640 − 2 × 5` pixels.
pub const PAIR_MARGIN: i32 = 5;
/// Folga em cima e embaixo na vista Lado a lado.
pub const SIDE_MARGIN_V: i32 = 8;
/// Folga dos lados nas vistas Lado a lado e Todas as páginas.
pub const ROW_MARGIN: i32 = 12;
/// Quanto cada clique numa seta da barra de baixo rola, em pixels.
pub const ARROW_STEP: i32 = 36;
/// Régua do bichinho: x do nível 0 e caminho até o último.
pub const BUG_X0: i32 = 278;
pub const BUG_RANGE: i32 = 121;

/// Como as folhas ficam na vista (página 2 da faixa do Visor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Arrange {
    /// Visão normal: uma sobre a outra.
    #[default]
    Normal,
    /// Duas páginas, como um livro aberto.
    Two,
    /// Todas as páginas na tela.
    All,
    /// Lado a lado, numa fileira.
    Side,
}

/// Zoom pedido (página 1 da faixa do Visor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zoom {
    /// Um dos [`LEVELS`].
    Level(usize),
    /// Página inteira: a folha inteira na vista.
    FitPage,
    /// Aumentar visão: a folha (ou o par) enche a largura.
    FitWidth,
}

impl Default for Zoom {
    fn default() -> Zoom {
        Zoom::Level(NORMAL_LEVEL)
    }
}

/// A vista do Escritório: arrumação, zoom e, com a barra de baixo, a
/// visão geral "todas as páginas" (guarda a vista de antes para voltar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct View {
    pub arrange: Arrange,
    pub zoom: Zoom,
    /// Visão geral aberta pelo ícone de páginas da barra de baixo.
    pub overview: bool,
}

/// Onde cada folha cai na tela (sem a rolagem).
#[derive(Debug, Clone, PartialEq)]
pub struct Sheets {
    /// Pixels por ponto.
    pub scale: f64,
    /// Tamanho de cada folha na tela.
    pub w: i32,
    pub h: i32,
    /// Canto de cada folha na tela, sem rolagem.
    pub origins: Vec<(i32, i32)>,
}

impl Sheets {
    pub fn rect(&self, page: usize) -> Option<Rect> {
        self.origins.get(page).map(|&(x, y)| (x, y, self.w, self.h))
    }

    /// Fim (exclusivo) do conteúdo em x e y.
    pub fn extent(&self) -> (i32, i32) {
        self.origins.iter().fold((0, 0), |(mx, my), &(x, y)| {
            (mx.max(x + self.w), my.max(y + self.h))
        })
    }

    /// Rolagem máxima para o conteúdo não sair da vista do Escritório.
    pub fn max_scroll(&self) -> (i32, i32) {
        let (x1, y1) = self.extent();
        (
            (x1 - (PAGE.0 + PAGE.2)).max(0),
            (y1 - (PAGE.1 + PAGE.3)).max(0),
        )
    }
}

fn level_scale(k: usize) -> f64 {
    f64::from(LEVELS[k.min(LEVELS.len() - 1)]) / 100.0
}

/// Tamanho de uma folha de `pw × ph` pontos no zoom `s`.
pub fn sheet_size(pw: i32, ph: i32, s: f64) -> (i32, i32) {
    let px = |v: i32| ((f64::from(v) * s + 1e-9).floor() as i32).max(1);
    (px(pw), px(ph))
}

/// Colunas e zoom da vista Todas as páginas: o maior zoom em que as `n`
/// folhas cabem na vista.
fn all_grid(pw: f64, ph: f64, n: usize) -> (usize, f64) {
    let (vw, vh) = (f64::from(SCREEN_W - 2 * ROW_MARGIN), f64::from(AREA.3));
    (1..=n.max(1))
        .map(|c| {
            let r = n.max(1).div_ceil(c);
            (c, (vh / (r as f64 * ph)).min(vw / (c as f64 * pw)))
        })
        .fold(
            (1, 0.0),
            |best, cur| if cur.1 > best.1 { cur } else { best },
        )
}

impl View {
    /// Arrumação em vigor (a visão geral é sempre Todas as páginas).
    pub fn effective(&self) -> Arrange {
        if self.overview {
            Arrange::All
        } else {
            self.arrange
        }
    }

    /// A Visão normal em 100 %: o desenho de sempre do editor.
    pub fn is_normal(&self) -> bool {
        self.effective() == Arrange::Normal && self.zoom == Zoom::Level(NORMAL_LEVEL)
    }

    /// Zoom pedido, sem os limites de cada arrumação.
    fn wanted(&self, pw: f64, ph: f64) -> f64 {
        let (vw, vh) = (f64::from(AREA.2), f64::from(AREA.3));
        let pair = f64::from(SCREEN_W - 2 * PAIR_MARGIN) / (2.0 * pw);
        match (self.zoom, self.effective()) {
            (Zoom::Level(k), _) => level_scale(k),
            (Zoom::FitPage, Arrange::Two) => (vh / ph).min(pair),
            (Zoom::FitPage, _) => (vh / ph).min(vw / pw),
            (Zoom::FitWidth, Arrange::Two) => pair,
            (Zoom::FitWidth, _) => vw / pw,
        }
    }

    /// Pixels por ponto para `n` folhas de `pw × ph` pontos.
    pub fn scale(&self, pw: i32, ph: i32, n: usize) -> f64 {
        let (pw, ph) = (f64::from(pw.max(1)), f64::from(ph.max(1)));
        match self.effective() {
            Arrange::Normal => self.wanted(pw, ph),
            Arrange::Two => self
                .wanted(pw, ph)
                .min(f64::from(SCREEN_W - 2 * PAIR_MARGIN) / (2.0 * pw)),
            Arrange::All => all_grid(pw, ph, n).1,
            Arrange::Side => (f64::from(AREA.3 - 2 * SIDE_MARGIN_V) / ph)
                .min(f64::from(SCREEN_W - 2 * ROW_MARGIN) / (n.max(1) as f64 * pw)),
        }
    }

    /// Posição das `n` folhas na tela, sem rolagem.
    pub fn sheets(&self, pw: i32, ph: i32, n: usize) -> Sheets {
        let n = n.max(1);
        let scale = self.scale(pw, ph, n);
        let (w, h) = sheet_size(pw, ph, scale);
        let centered = |total: i32| (SCREEN_W - total) / 2;
        let origins = match self.effective() {
            Arrange::Normal => {
                let x = if w >= AREA.2 { ORIGIN.0 } else { centered(w) };
                (0..n)
                    .map(|k| (x, ORIGIN.1 + k as i32 * (h + PAGE_GAP)))
                    .collect()
            }
            Arrange::Two => {
                let x0 = centered(2 * w);
                (0..n)
                    .map(|k| {
                        let (r, c) = ((k / 2) as i32, (k % 2) as i32);
                        (x0 + c * w, AREA.1 + r * (h + PAIR_GAP))
                    })
                    .collect()
            }
            Arrange::All => {
                let (cols, _) = all_grid(f64::from(pw), f64::from(ph), n);
                let rows = n.div_ceil(cols) as i32;
                let (x0, y0) = (
                    centered(cols as i32 * w),
                    AREA.1 + (AREA.3 - rows * h).max(0) / 2,
                );
                (0..n)
                    .map(|k| {
                        let (r, c) = ((k / cols) as i32, (k % cols) as i32);
                        (x0 + c * w, y0 + r * h)
                    })
                    .collect()
            }
            Arrange::Side => {
                let (x0, y0) = (centered(n as i32 * w), AREA.1 + (AREA.3 - h).max(0) / 2);
                (0..n).map(|k| (x0 + k as i32 * w, y0)).collect()
            }
        };
        Sheets {
            scale,
            w,
            h,
            origins,
        }
    }

    /// x do bichinho sobre a régua para o zoom `s`.
    pub fn bug_x(s: f64) -> i32 {
        let last = LEVELS.len() - 1;
        let pos = (0..=last)
            .find(|&k| (level_scale(k) - s).abs() < 0.005)
            .map(|k| 2 * k)
            .unwrap_or_else(|| match (0..last).find(|&k| s < level_scale(k + 1)) {
                Some(k) if s > level_scale(k) => 2 * k + 1,
                Some(_) => 0,
                None => 2 * last,
            });
        BUG_X0 + (pos as i32 * BUG_RANGE) / (2 * last as i32)
    }

    /// MENOS ZOOM (`in_ = false`) ou MAIS ZOOM a partir do zoom `s`; volta
    /// para a Visão normal se estava em Todas ou Lado a lado. `false` = já
    /// estava no fim da régua.
    pub fn step_zoom(&mut self, s: f64, in_: bool) -> bool {
        let next = if in_ {
            (0..LEVELS.len()).find(|&k| level_scale(k) > s + 0.005)
        } else {
            (0..LEVELS.len())
                .rev()
                .find(|&k| level_scale(k) < s - 0.005)
        };
        if matches!(self.arrange, Arrange::All | Arrange::Side) {
            self.arrange = Arrange::Normal;
        }
        match next {
            Some(k) => {
                self.zoom = Zoom::Level(k);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALTO: (i32, i32) = (595, 792);

    fn view(arrange: Arrange, zoom: Zoom) -> View {
        View {
            arrange,
            zoom,
            overview: false,
        }
    }

    #[test]
    fn zoomed_out_sheets_match_the_original() {
        // zm1..zm5: molduras em x 97/121/171/222/246, folhas de 446..148 px
        for (k, x, w) in [
            (4, 97, 446),
            (3, 121, 398),
            (2, 171, 297),
            (1, 222, 196),
            (0, 246, 148),
        ] {
            let s = view(Arrange::Normal, Zoom::Level(k)).sheets(ALTO.0, ALTO.1, 2);
            assert_eq!((s.origins[0], s.w), ((x, 107), w), "nível {k}");
            // as folhas encostam (g25: 107..304, a seguinte em 305)
            assert_eq!(s.origins[1].1, 107 + s.h);
        }
        let s = view(Arrange::Normal, Zoom::Level(0)).sheets(ALTO.0, ALTO.1, 2);
        assert_eq!(s.h, 198);
    }

    #[test]
    fn normal_view_is_the_editor_stack() {
        let v = View::default();
        assert!(v.is_normal());
        let s = v.sheets(ALTO.0, ALTO.1, 3);
        assert_eq!(s.scale, 1.0);
        assert_eq!(
            s.origins,
            vec![
                ORIGIN,
                (ORIGIN.0, ORIGIN.1 + 792),
                (ORIGIN.0, ORIGIN.1 + 1584)
            ]
        );
        assert_eq!(s.max_scroll(), (0, 107 + 3 * 792 - 457));
    }

    #[test]
    fn whole_page_and_magnify() {
        // zfit: 189..450 × 107..456
        let s = view(Arrange::Normal, Zoom::FitPage).sheets(ALTO.0, ALTO.1, 1);
        assert_eq!((s.origins[0], s.w, s.h), ((189, 107), 262, 350));
        assert_eq!(View::bug_x(s.scale), 303);
        // zinc: Aumentar visão = 100 % na folha Alto, bichinho no 100 %
        let s = view(Arrange::Normal, Zoom::FitWidth).sheets(ALTO.0, ALTO.1, 1);
        assert_eq!(s.scale, 1.0);
        assert_eq!(View::bug_x(s.scale), 364);
    }

    #[test]
    fn bug_walks_the_ruler() {
        let xs: Vec<i32> = (0..8).map(|k| View::bug_x(level_scale(k))).collect();
        assert_eq!(xs, vec![278, 295, 312, 329, 347, 364, 381, 399]);
    }

    #[test]
    fn zoom_steps_stop_at_the_ends() {
        let mut v = View::default();
        let mut seen = Vec::new();
        while v.step_zoom(v.scale(595, 792, 1), false) {
            seen.push(v.zoom);
        }
        assert_eq!(seen.len(), 5);
        assert_eq!(v.zoom, Zoom::Level(0));
        // de Página inteira (44 %), Mais zoom vai a 50 %
        v.zoom = Zoom::FitPage;
        assert!(v.step_zoom(v.scale(595, 792, 1), true));
        assert_eq!(v.zoom, Zoom::Level(2));
        v.zoom = Zoom::Level(7);
        assert!(!v.step_zoom(2.0, true));
    }

    #[test]
    fn two_pages_all_pages_and_side_by_side() {
        // vduas: vindo de 100 %, o par cabe em 630 px (5..319 e 320..634)
        let s = view(Arrange::Two, Zoom::Level(NORMAL_LEVEL)).sheets(ALTO.0, ALTO.1, 3);
        assert_eq!((s.origins[0], s.origins[1].0, s.w), ((5, 107), 320, 315));
        assert_eq!(s.origins[2], (5, 107 + s.h + PAIR_GAP));
        // u1: cartão em 33 %, 124..319 e 320..515
        let s = view(Arrange::Two, Zoom::Level(1)).sheets(ALTO.0, ALTO.1, 4);
        assert_eq!((s.origins[0].0, s.origins[1].0, s.w), (124, 320, 196));
        // vtodas: 58..319, 320..581, 107..456
        let s = view(Arrange::All, Zoom::default()).sheets(ALTO.0, ALTO.1, 2);
        assert_eq!(
            (s.origins[0], s.origins[1], s.w, s.h),
            ((58, 107), (320, 107), 262, 350)
        );
        assert_eq!(s.max_scroll(), (0, 0));
        // vlado: 70..319, 320..569, 115..448
        let s = view(Arrange::Side, Zoom::default()).sheets(ALTO.0, ALTO.1, 2);
        assert_eq!(
            (s.origins[0], s.origins[1], s.w, s.h),
            ((70, 115), (320, 115), 250, 334)
        );
    }

    #[test]
    fn all_pages_uses_a_grid_when_needed() {
        let s = view(Arrange::All, Zoom::default()).sheets(ALTO.0, ALTO.1, 7);
        let (x1, y1) = s.extent();
        assert!(s.origins[0].0 >= 0 && x1 <= SCREEN_W);
        assert!(s.origins[0].1 >= AREA.1 && y1 <= AREA.1 + AREA.3);
        assert!(
            s.origins.iter().any(|o| o.1 > s.origins[0].1),
            "mais de uma fileira"
        );
    }

    #[test]
    fn overview_is_all_pages_and_remembers_the_view() {
        let mut v = view(Arrange::Two, Zoom::Level(2));
        v.overview = true;
        assert_eq!(v.effective(), Arrange::All);
        assert!(!v.is_normal());
        v.overview = false;
        assert_eq!(v.effective(), Arrange::Two);
    }
}
