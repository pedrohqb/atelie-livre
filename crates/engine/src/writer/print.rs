//! Folhas impressas do Creative Writer (tarefa P15; `cw/report/
//! projects-impressao.md` §"Folhas impressas"): a composição das folhas a
//! partir das páginas já diagramadas, com a borda e o fundo da página.
//!
//! O papel é o do `PRNT` gravado no documento ([`paper_of`]: 612×792 nos
//! exemplos do Max, impressora de 300 dpi; 595×792 nos documentos gravados
//! sem impressora; 792×595 na faixa do Wine), virado se a página só couber
//! assim ([`sheet_for`]). No original a página do documento é a área imprimível dessa
//! folha (por isso as faixas dos exemplos têm 542×767 e a do Wine 577×595);
//! aqui ela vai centrada na folha.
//!
//! - **Documento e jornal** ([`SheetKind::Pages`]): uma folha por página
//!   ("Alto" em pé, "Largo" deitada), com as colunas e divisórias que a
//!   diagramação já pôs na página.
//! - **Cartão** ([`SheetKind::Card`]): as 4 páginas numa folha só, cada uma
//!   num quarto dela, reduzida à metade; em cima a página 1 (Capa) e a 4
//!   (Contracapa) de cabeça para baixo, embaixo a 2 e a 3 em pé (figura
//!   `0c65` do assistente). Dobra-se ao meio na horizontal e de novo na
//!   vertical. Da 5ª página em diante não sai (`3c00[106]`). No cartão
//!   "Largo" a folha é deitada e os quartos também.
//! - **Faixa** ([`SheetKind::Banner`]): uma folha por página, para emendar
//!   lado a lado; a palavra com forma atravessa as folhas (a diagramação já
//!   a desenha em pedaços, um em cada página).
//!
//! As páginas chegam prontas ([`PrintDoc::pages`]: o que o editor desenha,
//! 1 pixel por ponto, com o branco transparente); a borda e o fundo
//! ([`PageArt`]) são redesenhados a partir dos metarquivos na resolução da
//! folha. As folhas saem com [`SCALE`] pixels por ponto (144 dpi), como as
//! do Fine Artist ([`crate::paint::print`]).
//!
//! Guias (opcionais, ligadas por padrão): linhas tracejadas de dobra no
//! cartão (como na figura do assistente) e de corte nas folhas da faixa.
//! Ver `docs/telas/creative-writer-impressao.md`.

use super::assistant::Kind;
use super::borders::{self, PageArt, PageGeom};
use super::convert::MaxExtra;
use super::doc::Document;
use crate::paint::print::{Rgb8, SCALE};
use formats::max;

/// Páginas do cartão que vão para a folha.
pub const CARD_PAGES: usize = 4;
/// Papel sem `PRNT` (o padrão do original sem impressora), em pé.
pub const DEFAULT_PAPER: (u32, u32) = (595, 792);

const WHITE: [u8; 3] = [255, 255, 255];
/// Cor das guias de dobra e de corte.
const GUIDE: [u8; 3] = [160, 160, 160];

/// Como as páginas viram folhas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SheetKind {
    /// Uma folha por página (documento do Escritório, jornal).
    #[default]
    Pages,
    /// As 4 páginas do cartão numa folha, em quartos.
    Card,
    /// Uma folha deitada por página, para emendar lado a lado.
    Banner,
}

impl SheetKind {
    /// Do projeto da Oficina de Projetos (`None` = documento do Escritório).
    pub fn of(kind: Option<Kind>) -> SheetKind {
        match kind {
            Some(Kind::Card) => SheetKind::Card,
            Some(Kind::Banner) => SheetKind::Banner,
            Some(Kind::News) | None => SheetKind::Pages,
        }
    }
}

/// Projeto de um documento: o `WIZA`/`ADD` gravado; sem ele, a marca de
/// faixa do `DOP` (`0x5b`).
pub fn project_kind(doc: &Document, extra: &MaxExtra) -> Option<Kind> {
    Kind::from_wiza(extra.add.project.to_bytes()).or(doc.dop.banner.then_some(Kind::Banner))
}

/// Borda e fundo do documento do editor, das cópias guardadas em `extra`.
/// O fundo é o do modelo ou, se ele não o tiver, o `0x56` que a conversão
/// deixou em `extra.dop_extra`.
pub fn page_art(doc: &Document, extra: &MaxExtra) -> PageArt {
    let to_max = |r: super::object::ObjRef| max::ObjRef::new(&r.tag, r.id);
    let background = doc.dop.background.map(to_max).or_else(|| {
        extra
            .dop_extra
            .iter()
            .find(|s| s.code == borders::SPRM_BACKGROUND && s.arg.len() == 6)
            .map(|s| {
                max::ObjRef::new(
                    &[s.arg[0], s.arg[1], s.arg[2], s.arg[3]],
                    u16::from_le_bytes([s.arg[4], s.arg[5]]),
                )
            })
    });
    let find = |r: max::ObjRef| {
        extra.object(super::object::ObjRef {
            tag: r.tag.0,
            id: r.id,
        })
    };
    PageArt::from_refs(doc.dop.border.map(to_max), background, &find)
}

/// Papel do `PRNT` (`"prr1"`, `u16` dpi x e y, `u16` largura e altura em
/// pixels da impressora), em pontos; `None` se não der para ler.
pub fn paper_of(prnt: Option<&[u8]>) -> Option<(u32, u32)> {
    let b = prnt?;
    if b.len() < 12 || &b[..4] != b"prr1" {
        return None;
    }
    let u = |k: usize| u32::from(u16::from_le_bytes([b[k], b[k + 1]]));
    let (dx, dy, w, h) = (u(4), u(6), u(8), u(10));
    if dx == 0 || dy == 0 {
        return None;
    }
    let (w, h) = (w * 72 / dx, h * 72 / dy);
    ((100..=5000).contains(&w) && (100..=5000).contains(&h)).then_some((w, h))
}

/// Folha para a página `geom` no papel `paper`: o papel como está se a
/// página couber nele; senão virado; e, se nem assim couber, do tamanho da
/// página.
pub fn sheet_for(geom: &PageGeom, paper: (u32, u32)) -> (u32, u32) {
    let (pw, ph) = (geom.width.round() as u32, geom.height.round() as u32);
    let fits = |(w, h): (u32, u32)| pw <= w && ph <= h;
    let turned = (paper.1, paper.0);
    if fits(paper) {
        paper
    } else if fits(turned) {
        turned
    } else {
        let (w, h) = if (pw > ph) == (turned.0 > turned.1) {
            turned
        } else {
            paper
        };
        (w.max(pw), h.max(ph))
    }
}

/// O que imprimir: as páginas desenhadas e a arte da página.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintDoc {
    pub kind: SheetKind,
    /// Tamanho e margens da página (pontos).
    pub geom: PageGeom,
    /// Papel da impressora (pontos; ver [`paper_of`]).
    pub paper: (u32, u32),
    /// Cada página como o editor a desenha (1 pixel por ponto, sem borda nem
    /// fundo); o branco deixa a arte aparecer.
    pub pages: Vec<Rgb8>,
    pub art: PageArt,
}

impl PrintDoc {
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// O documento tem fundo (a máquina pergunta se é para imprimi-lo).
    pub fn has_background(&self) -> bool {
        self.art.background.is_some()
    }

    /// Folhas que sairão com `opts`.
    pub fn plan(&self, opts: &SheetOptions) -> Vec<SheetPlan> {
        plan(self.kind, &self.geom, self.paper, self.pages.len(), opts)
    }

    /// A página `k` em `scale` pixels por ponto: o fundo (se `background`),
    /// a borda e, por cima, o que o editor desenhou.
    pub fn page_image(&self, k: usize, scale: f64, background: bool) -> Rgb8 {
        let art = if background {
            self.art.clone()
        } else {
            PageArt {
                border: self.art.border.clone(),
                background: None,
            }
        };
        let img = borders::render_page(&self.geom, &art, scale);
        let mut out = Rgb8::from_rgba(img.width, img.height, &img.rgba);
        let Some(page) = self.pages.get(k) else {
            return out;
        };
        if page.w == 0 || page.h == 0 {
            return out;
        }
        let sx: Vec<u32> = (0..out.w)
            .map(|x| (((x as f64 + 0.5) / scale) as u32).min(page.w - 1))
            .collect();
        for y in 0..out.h {
            let py = (((y as f64 + 0.5) / scale) as u32).min(page.h - 1);
            let src = &page.px[(py * page.w) as usize..((py + 1) * page.w) as usize];
            let dst = &mut out.px[(y * out.w) as usize..((y + 1) * out.w) as usize];
            for (d, &x) in dst.iter_mut().zip(&sx) {
                let p = src[x as usize];
                if p != WHITE {
                    *d = p;
                }
            }
        }
        out
    }

    /// As folhas prontas, [`SCALE`] pixels por ponto.
    pub fn sheets(&self, opts: &SheetOptions) -> Vec<Rgb8> {
        self.plan(opts)
            .iter()
            .map(|p| self.render_sheet(p, opts, SCALE as f64))
            .collect()
    }

    /// A folha `plan` a `scale` pixels por ponto.
    pub fn render_sheet(&self, plan: &SheetPlan, opts: &SheetOptions, scale: f64) -> Rgb8 {
        let (sw, sh) = plan.size;
        let mut out = Rgb8::new(
            ((sw as f64 * scale).round() as u32).max(1),
            ((sh as f64 * scale).round() as u32).max(1),
        );
        for pl in &plan.places {
            let k = pl.rect.2 as f64 / self.geom.width.max(1.0);
            let img = self.page_image(pl.page, scale * k, opts.background);
            let (x0, y0) = (
                (pl.rect.0 as f64 * scale).round() as i64,
                (pl.rect.1 as f64 * scale).round() as i64,
            );
            blit(&mut out, &img, x0, y0, pl.upside_down);
        }
        if opts.guides {
            for g in &plan.guides {
                dashed(&mut out, *g, scale);
            }
        }
        out
    }
}

/// Escolhas que mudam as folhas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SheetOptions {
    /// Imprimir o fundo (`4c00[1]`).
    pub background: bool,
    /// Só as páginas de `.0` a `.1` (contadas de 1, inclusive; `4c00[6]`).
    pub pages: Option<(usize, usize)>,
    /// Linhas de dobra (cartão) e de corte (faixa).
    pub guides: bool,
}

impl Default for SheetOptions {
    fn default() -> SheetOptions {
        SheetOptions {
            background: true,
            pages: None,
            guides: true,
        }
    }
}

/// Uma página posta numa folha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Índice da página (de 0).
    pub page: usize,
    /// Retângulo na folha, em pontos.
    pub rect: (f32, f32, f32, f32),
    /// Girada 180° (as de cima do cartão).
    pub upside_down: bool,
}

/// Linha-guia (dobra ou corte) numa folha, em pontos: vertical em `x` ou
/// horizontal em `y`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Guide {
    Vertical(f32),
    Horizontal(f32),
}

/// Uma folha: tamanho em pontos, as páginas e as guias.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetPlan {
    pub size: (u32, u32),
    pub places: Vec<Placement>,
    pub guides: Vec<Guide>,
}

/// Páginas escolhidas (de 0), dentro do documento.
fn chosen(n: usize, range: Option<(usize, usize)>) -> std::ops::Range<usize> {
    match range {
        Some((from, to)) => {
            let from = from.max(1).min(n + 1) - 1;
            from..to.min(n).max(from)
        }
        None => 0..n,
    }
}

/// Folhas de um documento de `n` páginas no papel `paper`.
pub fn plan(
    kind: SheetKind,
    geom: &PageGeom,
    paper: (u32, u32),
    n: usize,
    opts: &SheetOptions,
) -> Vec<SheetPlan> {
    let (pw, ph) = (geom.width as f32, geom.height as f32);
    let size = sheet_for(geom, paper);
    let (sw, sh) = (size.0 as f32, size.1 as f32);
    // a página centrada na folha (ou num quarto dela)
    let centered = |x: f32, y: f32, w: f32, h: f32| {
        let k = (w / pw).min(h / ph).min(1.0);
        let (dw, dh) = (pw * k, ph * k);
        (x + (w - dw) / 2.0, y + (h - dh) / 2.0, dw, dh)
    };
    let one = |k: usize, guides: bool| {
        let rect = centered(0.0, 0.0, sw, sh);
        SheetPlan {
            size,
            places: vec![Placement {
                page: k,
                rect,
                upside_down: false,
            }],
            // linhas de corte nas beiradas da página, para emendar
            guides: if guides && rect.0 > 0.5 {
                vec![Guide::Vertical(rect.0), Guide::Vertical(rect.0 + rect.2)]
            } else {
                Vec::new()
            },
        }
    };
    match kind {
        SheetKind::Pages => chosen(n, opts.pages).map(|k| one(k, false)).collect(),
        SheetKind::Banner => chosen(n, opts.pages).map(|k| one(k, true)).collect(),
        SheetKind::Card => {
            // cada quarto, a metade da folha
            let (qw, qh) = (sw / 2.0, sh / 2.0);
            let quarters = [
                (0, 0.0, 0.0, true),
                (3, qw, 0.0, true),
                (1, 0.0, qh, false),
                (2, qw, qh, false),
            ];
            let places = quarters
                .iter()
                .filter(|q| q.0 < n.min(CARD_PAGES))
                .map(|&(page, x, y, upside_down)| Placement {
                    page,
                    rect: centered(x, y, qw, qh),
                    upside_down,
                })
                .collect();
            vec![SheetPlan {
                size,
                places,
                guides: vec![Guide::Horizontal(qh), Guide::Vertical(qw)],
            }]
        }
    }
}

/// Copia `img` para `out` com o canto em `(x0, y0)`; `upside_down` gira 180°.
fn blit(out: &mut Rgb8, img: &Rgb8, x0: i64, y0: i64, upside_down: bool) {
    for y in 0..img.h {
        let oy = y0 + y as i64;
        if oy < 0 || oy >= out.h as i64 {
            continue;
        }
        let sy = if upside_down { img.h - 1 - y } else { y };
        for x in 0..img.w {
            let ox = x0 + x as i64;
            if ox < 0 || ox >= out.w as i64 {
                continue;
            }
            let sx = if upside_down { img.w - 1 - x } else { x };
            out.px[(oy as u32 * out.w + ox as u32) as usize] = img.px[(sy * img.w + sx) as usize];
        }
    }
}

/// Linha tracejada fina (4 pt de traço, 4 pt de vão) por cima da folha.
fn dashed(out: &mut Rgb8, g: Guide, scale: f64) {
    let dash = (4.0 * scale).max(1.0) as u32;
    let on = |t: u32| (t / dash) % 2 == 0;
    match g {
        Guide::Vertical(x) => {
            let x = ((x as f64 * scale) as u32).min(out.w.saturating_sub(1));
            for y in (0..out.h).filter(|&y| on(y)) {
                out.px[(y * out.w + x) as usize] = GUIDE;
            }
        }
        Guide::Horizontal(y) => {
            let y = ((y as f64 * scale) as u32).min(out.h.saturating_sub(1));
            for x in (0..out.w).filter(|&x| on(x)) {
                out.px[(y * out.w + x) as usize] = GUIDE;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: [u8; 3] = [255, 0, 0];
    const BLUE: [u8; 3] = [0, 0, 255];

    /// Documento de `n` páginas de `w`×`h` pontos; a página `k` tem um
    /// quadrado de 10 pt no canto de cima à esquerda do texto (72, 72), com
    /// `k + 1` pontos de largura de cor diferente embaixo dele.
    fn doc(kind: SheetKind, w: u32, h: u32, n: usize) -> PrintDoc {
        let pages = (0..n)
            .map(|k| {
                let mut p = Rgb8::new(w, h);
                for y in 72..82 {
                    for x in 72..82 {
                        p.px[(y * w + x) as usize] = RED;
                    }
                }
                for x in 72..72 + k as u32 + 1 {
                    p.px[(82 * w + x) as usize] = BLUE;
                }
                p
            })
            .collect();
        PrintDoc {
            kind,
            geom: PageGeom::new(w as f64, h as f64, 72.0),
            paper: DEFAULT_PAPER,
            pages,
            art: PageArt::default(),
        }
    }

    fn at(s: &Rgb8, x: f32, y: f32) -> [u8; 3] {
        // ponto da folha (pontos) → pixel
        let (x, y) = ((x * SCALE as f32) as u32, (y * SCALE as f32) as u32);
        s.px[(y * s.w + x) as usize]
    }

    #[test]
    fn pages_one_sheet_each_in_range() {
        let d = doc(SheetKind::Pages, 595, 792, 5);
        let all = d.sheets(&SheetOptions::default());
        assert_eq!(all.len(), 5);
        assert_eq!((all[0].w, all[0].h), (595 * SCALE, 792 * SCALE));
        assert_eq!(at(&all[0], 75.0, 75.0), RED);
        assert_eq!(at(&all[0], 30.0, 30.0), WHITE);
        let some = SheetOptions {
            pages: Some((2, 3)),
            ..SheetOptions::default()
        };
        let p = d.plan(&some);
        assert_eq!(
            p.iter().map(|s| s.places[0].page).collect::<Vec<_>>(),
            [1, 2]
        );
        // fora do documento: corta no que existe
        assert_eq!(chosen(5, Some((4, 99))), 3..5);
        assert_eq!(chosen(5, Some((9, 12))), 5..5);
        assert_eq!(chosen(5, Some((0, 1))), 0..1);
        // "Largo": folha deitada
        let wide = doc(SheetKind::Pages, 792, 595, 1).sheets(&SheetOptions::default());
        assert_eq!((wide[0].w, wide[0].h), (792 * SCALE, 595 * SCALE));
    }

    #[test]
    fn card_quarters_with_top_row_upside_down() {
        let d = doc(SheetKind::Card, 595, 792, 5);
        let plan = d.plan(&SheetOptions::default());
        assert_eq!(plan.len(), 1);
        let p = &plan[0];
        assert_eq!(p.size, (595, 792));
        let pages: Vec<(usize, bool)> = p.places.iter().map(|q| (q.page, q.upside_down)).collect();
        // 1 e 4 em cima, de cabeça para baixo; 2 e 3 embaixo; a 5ª não sai
        assert_eq!(pages, [(0, true), (3, true), (1, false), (2, false)]);
        assert_eq!(p.places[1].rect, (297.5, 0.0, 297.5, 396.0));
        let s = d.render_sheet(p, &SheetOptions::default(), SCALE as f64);
        // página 2 (embaixo à esquerda), em pé: o quadrado em (36, 396 + 36)
        assert_eq!(at(&s, 38.0, 396.0 + 38.0), RED);
        // página 1 (em cima à esquerda), girada: o quadrado vai para perto
        // do canto de baixo à direita do quarto
        assert_eq!(at(&s, 297.5 - 38.0, 396.0 - 38.0), RED);
        assert_eq!(at(&s, 38.0, 38.0), WHITE);
        // a página 4 girada, no quarto de cima à direita
        assert_eq!(at(&s, 595.0 - 38.0, 396.0 - 38.0), RED);
        // guias de dobra no meio
        assert_eq!(at(&s, 1.0, 396.0), GUIDE);
        let no_guides = SheetOptions {
            guides: false,
            ..SheetOptions::default()
        };
        let s = d.render_sheet(p, &no_guides, SCALE as f64);
        assert_eq!(at(&s, 1.0, 396.0), WHITE);
        // cartão de 2 páginas: só os dois quartos
        assert_eq!(
            doc(SheetKind::Card, 595, 792, 2).plan(&no_guides)[0]
                .places
                .len(),
            2
        );
    }

    #[test]
    fn wide_card_lies_down() {
        let d = doc(SheetKind::Card, 792, 595, 4);
        let p = &d.plan(&SheetOptions::default())[0];
        assert_eq!(p.size, (792, 595));
        assert_eq!(p.places[3].rect, (396.0, 297.5, 396.0, 297.5));
    }

    #[test]
    fn banner_one_landscape_sheet_per_page() {
        // a faixa gravada no Wine: página 577×595, PRNT deitado de 792×595
        let mut d = doc(SheetKind::Banner, 577, 595, 3);
        d.paper = (792, 595);
        let plan = d.plan(&SheetOptions::default());
        assert_eq!(plan.len(), 3);
        assert!(plan.iter().all(|p| p.size == (792, 595)));
        let x = (792.0 - 577.0) / 2.0;
        assert_eq!(plan[2].places[0].rect, (x, 0.0, 577.0, 595.0));
        assert_eq!(
            plan[0].guides,
            [Guide::Vertical(x), Guide::Vertical(x + 577.0)]
        );
        let s = d.sheets(&SheetOptions::default());
        assert_eq!((s[1].w, s[1].h), (792 * SCALE, 595 * SCALE));
        assert_eq!(at(&s[1], x + 75.0, 75.0), RED);
        // o terceiro pedaço tem 3 pontos azuis
        assert_eq!(at(&s[2], x + 74.5, 82.5), BLUE);
    }

    #[test]
    fn paper_from_prnt_turns_with_the_page() {
        // exemplos do Max: 300 dpi, 2550×3300 (carta em pé)
        let samp = [
            0x70, 0x72, 0x72, 0x31, 0x2c, 1, 0x2c, 1, 0xf6, 9, 0xe4, 0x0c, 0x4b, 0, 0x46, 0,
        ];
        assert_eq!(paper_of(Some(&samp)), Some((612, 792)));
        assert_eq!(paper_of(Some(&max::DEFAULT_PRINTER)), Some((595, 792)));
        assert_eq!(paper_of(Some(b"xxxx")), None);
        assert_eq!(paper_of(None), None);
        let alto = PageGeom::new(542.0, 767.0, 72.0);
        assert_eq!(sheet_for(&alto, (612, 792)), (612, 792));
        let largo = PageGeom::new(792.0, 595.0, 72.0);
        assert_eq!(sheet_for(&largo, (612, 792)), (792, 612));
        // página maior que o papel: a folha cresce
        assert_eq!(
            sheet_for(&PageGeom::new(700.0, 900.0, 72.0), (612, 792)),
            (700, 900)
        );
        // faixa dos exemplos: folha em pé, página no meio, linhas de corte
        let mut d = doc(SheetKind::Banner, 542, 767, 2);
        d.paper = (612, 792);
        let p = &d.plan(&SheetOptions::default())[0];
        assert_eq!(p.size, (612, 792));
        assert_eq!(p.places[0].rect, (35.0, 12.5, 542.0, 767.0));
        assert_eq!(p.guides.len(), 2);
        // cartão em papel carta: o quarto é mais largo que meia página
        let mut c = doc(SheetKind::Card, 595, 792, 4);
        c.paper = (612, 792);
        let p = &c.plan(&SheetOptions::default())[0];
        assert_eq!(p.places[0].rect, (4.25, 0.0, 297.5, 396.0));
    }

    #[test]
    fn page_image_scales_content_over_white() {
        let d = doc(SheetKind::Pages, 100, 100, 1);
        let img = d.page_image(0, 0.5, true);
        assert_eq!((img.w, img.h), (50, 50));
        assert_eq!(img.px[(37 * 50 + 37) as usize], RED);
        assert_eq!(img.px[0], WHITE);
    }

    #[test]
    fn kinds_from_projects() {
        assert_eq!(SheetKind::of(None), SheetKind::Pages);
        assert_eq!(SheetKind::of(Some(Kind::News)), SheetKind::Pages);
        assert_eq!(SheetKind::of(Some(Kind::Card)), SheetKind::Card);
        assert_eq!(SheetKind::of(Some(Kind::Banner)), SheetKind::Banner);
        let (mut d, mut e) = super::super::convert::blank();
        assert_eq!(project_kind(&d, &e), None);
        d.dop.banner = true;
        assert_eq!(project_kind(&d, &e), Some(Kind::Banner));
        e.add.project = max::Project::Card;
        assert_eq!(project_kind(&d, &e), Some(Kind::Card));
    }
}
