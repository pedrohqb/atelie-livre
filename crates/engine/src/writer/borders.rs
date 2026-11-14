//! Borda e fundo de página do Creative Writer (rolos BORDA e FUNDO da
//! Preparação da página), desenhados a partir dos `.BOR` ou das cópias
//! gravadas no `.MAX` (ver `docs/formats/bor.md`).
//!
//! Regras medidas no original (Visão normal, 1 pt = 1 pixel; capturas
//! `w26_95_tribe_normal`, `w26_101_paw`, `w26_111_scroll`):
//!
//! - a faixa nominal é `E = 1,5 × espessura` e fica **fora** da área de
//!   texto, sem empurrar o texto; o lado de dentro dela é o retângulo do
//!   texto alargado em [`BAND_GAP_H`] pt na horizontal e [`BAND_GAP_V`] pt
//!   na vertical;
//! - em cada direção cabem `n = round(lado / E)` peças, esticadas para
//!   encher o lado exatamente; a espessura desenhada é a menor das duas
//!   medidas resultantes (Tribe: 461/13 = 35,46 pt; Paw: 461/10 = 46,1 pt) e
//!   os cantos são quadrados dessa espessura;
//! - sem rotação: cada lado usa a sua peça do mapa; peças de lado
//!   retangulares (Vine, 715 × 480) mantêm a proporção;
//! - o fundo vai em tamanho natural com a origem da janela WMF em
//!   `(dx, dy)` pt da página, mais as figuras das suas molduras; as
//!   molduras de tipo 6 são obstáculos ao texto em todas as páginas.

use formats::bor::{self, Background, Border, Slot};
use formats::chnk::OwnedChunk;
use formats::max::{self, DopProps, Frame, ObjRef};
use formats::winres::Image;
use formats::wmf_draw::{Rect, Wmf};

/// Folga horizontal entre o texto e o lado de dentro da faixa, em pontos.
pub const BAND_GAP_H: f64 = 5.0;
/// Folga vertical entre o texto e o lado de dentro da faixa, em pontos.
pub const BAND_GAP_V: f64 = 1.0;
/// Sprm do `DOP` que aponta o fundo da página (`0x56 "IBGF" n`).
pub const SPRM_BACKGROUND: u8 = 0x56;

/// Folha: tamanho e margens em pontos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageGeom {
    pub width: f64,
    pub height: f64,
    pub margin_top: f64,
    pub margin_left: f64,
    pub margin_bottom: f64,
    pub margin_right: f64,
}

impl PageGeom {
    /// Folha `w × h` com as quatro margens iguais.
    pub fn new(width: f64, height: f64, margin: f64) -> PageGeom {
        PageGeom {
            width,
            height,
            margin_top: margin,
            margin_left: margin,
            margin_bottom: margin,
            margin_right: margin,
        }
    }

    pub fn from_dop(d: &DopProps) -> PageGeom {
        PageGeom {
            width: d.page_width as f64,
            height: d.page_height as f64,
            margin_top: d.margin_top as f64,
            margin_left: d.margin_left as f64,
            margin_bottom: d.margin_bottom as f64,
            margin_right: d.margin_right as f64,
        }
    }

    /// Área de texto (pontos).
    pub fn text_rect(&self) -> Rect {
        Rect::new(
            self.margin_left,
            self.margin_top,
            self.width - self.margin_left - self.margin_right,
            self.height - self.margin_top - self.margin_bottom,
        )
    }
}

/// Onde as peças de uma borda caem na folha (pontos).
#[derive(Debug, Clone, PartialEq)]
pub struct BorderLayout {
    /// Espessura desenhada (lado dos cantos).
    pub band: f64,
    /// Lado de dentro e de fora da faixa.
    pub inner: Rect,
    pub outer: Rect,
    /// Peças em cima/embaixo e de cada lado.
    pub across: usize,
    pub down: usize,
    pub tiles: Vec<(Slot, Rect)>,
}

impl BorderLayout {
    /// A faixa cabe na folha (senão o original avisa `0fa5[11]`: a margem
    /// precisa ser larga o bastante para a borda).
    pub fn fits(&self, page: &PageGeom) -> bool {
        self.outer.x >= -0.5
            && self.outer.y >= -0.5
            && self.outer.x + self.outer.w <= page.width + 0.5
            && self.outer.y + self.outer.h <= page.height + 0.5
    }
}

/// Comprimento de uma peça de lado ao longo do lado, dividido pela sua
/// espessura (1 nas peças quadradas).
fn aspect(w: &Wmf, along_x: bool) -> f64 {
    let (pw, ph) = (w.width.max(1) as f64, w.height.max(1) as f64);
    if along_x { pw / ph } else { ph / pw }
}

/// Posição das peças de `border` na folha.
pub fn border_layout(page: &PageGeom, border: &Border) -> BorderLayout {
    let text = page.text_rect();
    let inner = Rect::new(
        text.x - BAND_GAP_H,
        text.y - BAND_GAP_V,
        text.w + 2.0 * BAND_GAP_H,
        text.h + 2.0 * BAND_GAP_V,
    );
    let e = border.band_pt().max(1.0);
    let (ah, av) = (
        aspect(border.piece(Slot::Top), true),
        aspect(border.piece(Slot::Left), false),
    );
    let across = ((inner.w / (e * ah)).round() as usize).max(1);
    let down = ((inner.h / (e * av)).round() as usize).max(1);
    let band = (inner.w / across as f64 / ah).min(inner.h / down as f64 / av);
    let outer = Rect::new(
        inner.x - band,
        inner.y - band,
        inner.w + 2.0 * band,
        inner.h + 2.0 * band,
    );
    let tiles = bor::tile(outer, band, across, down);
    BorderLayout {
        band,
        inner,
        outer,
        across,
        down,
        tiles,
    }
}

/// Retângulo em pontos levado à folha desenhada em `page_px`.
fn to_px(r: Rect, page_px: Rect, page: &PageGeom) -> Rect {
    let (sx, sy) = (page_px.w / page.width, page_px.h / page.height);
    Rect::new(
        page_px.x + r.x * sx,
        page_px.y + r.y * sy,
        r.w * sx,
        r.h * sy,
    )
}

/// Desenha a borda numa folha que ocupa `page_px` (pixels de `img`).
pub fn draw_border(img: &mut Image, page_px: Rect, page: &PageGeom, border: &Border) {
    let layout = border_layout(page, border);
    let tiles: Vec<(Slot, Rect)> = layout
        .tiles
        .iter()
        .map(|(s, r)| (*s, to_px(*r, page_px, page)))
        .collect();
    border.draw_tiles(img, &tiles);
}

/// Fundo com as figuras que ele usa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageBackground {
    pub background: Background,
    pub pictures: Vec<(u16, Wmf)>,
}

impl PageBackground {
    /// Fundo `id` de um `.BOR` já lido (com cópia das figuras usadas).
    pub fn from_bor(file: &bor::BorFile, id: u16) -> Option<PageBackground> {
        let background = file.background(id)?.clone();
        let ids: Vec<u16> = background
            .pict
            .into_iter()
            .chain(background.pictures().map(|p| p.1))
            .collect();
        let pictures = ids
            .into_iter()
            .filter_map(|i| file.picture(i).map(|w| (i, w.clone())))
            .collect();
        Some(PageBackground {
            background,
            pictures,
        })
    }

    pub fn picture(&self, id: u16) -> Option<&Wmf> {
        self.pictures.iter().find(|p| p.0 == id).map(|p| &p.1)
    }

    /// Molduras que desviam o texto (tipo 6), em pontos da folha.
    pub fn obstacles(&self) -> Vec<Obstacle> {
        self.background
            .obstacles()
            .map(Obstacle::from_frame)
            .collect()
    }
}

/// De que lado de um obstáculo do fundo o texto pode correr (`0x29`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSide {
    /// 1: só à esquerda (tira direita do fundo).
    Left,
    /// 2: só à direita (tira esquerda).
    Right,
    /// 3 (ou outro): nenhum texto ao lado, só acima e abaixo.
    None,
}

/// Obstáculo ao texto vindo do fundo (pontos).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obstacle {
    pub rect: Rect,
    pub text_side: TextSide,
}

impl Obstacle {
    pub fn from_frame(f: &Frame) -> Obstacle {
        Obstacle {
            rect: Rect::new(f.x as f64, f.y as f64, f.width as f64, f.height as f64),
            text_side: match f.wrap {
                Some(1) => TextSide::Left,
                Some(2) => TextSide::Right,
                _ => TextSide::None,
            },
        }
    }
}

/// Desenha o fundo numa folha que ocupa `page_px`.
pub fn draw_background(img: &mut Image, page_px: Rect, page: &PageGeom, bg: &PageBackground) {
    bg.background
        .draw(img, page_px, (page.width, page.height), &|id| {
            bg.picture(id)
        });
}

/// Borda e fundo de um documento.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageArt {
    pub border: Option<Border>,
    pub background: Option<PageBackground>,
}

/// Referência ao fundo no `DOP` (`0x56 "IBGF" n`), que o leitor deixa em
/// `extra`.
pub fn background_ref(dop: &DopProps) -> Option<ObjRef> {
    dop.extra
        .iter()
        .find(|s| s.code == SPRM_BACKGROUND && s.arg.len() == 6)
        .map(|s| {
            ObjRef::new(
                &[s.arg[0], s.arg[1], s.arg[2], s.arg[3]],
                u16::from_le_bytes([s.arg[4], s.arg[5]]),
            )
        })
}

fn part<'a>(c: &'a OwnedChunk, tag: &[u8; 4]) -> Option<&'a [u8]> {
    c.parts
        .iter()
        .find(|p| p.tag.0 == *tag)
        .map(|p| p.data.as_slice())
}

impl PageArt {
    /// Lê a borda (`DOP 0x58 → BDR n`) e o fundo (`DOP 0x56 → IBGF n`, que
    /// aponta `PICT` e `FRAM`) das cópias guardadas no documento.
    pub fn from_document(doc: &max::Document) -> PageArt {
        let border = doc.dop.border.and_then(|r| {
            let c = doc.object(r)?;
            let name = c.name.clone().unwrap_or_default();
            Border::from_parts(c.id, &name, part(c, b"WBDR")?, part(c, b"PKBM")).ok()
        });
        let background = background_ref(&doc.dop).and_then(|r| {
            let c = doc.object(r)?;
            let ibgf = bor::Ibgf::parse(part(c, b"IBGF")?).ok()?;
            let frames = ibgf
                .fram
                .and_then(|id| doc.object(ObjRef::new(b"FRAM", id)))
                .and_then(|f| part(f, b"FRAM"))
                .and_then(|d| bor::read_frames(d).ok())
                .unwrap_or_default();
            let background =
                Background::new(c.id, c.name.as_deref().unwrap_or(""), ibgf, frames, None);
            let ids: Vec<u16> = background
                .pict
                .into_iter()
                .chain(background.pictures().map(|p| p.1))
                .collect();
            let pictures = ids
                .into_iter()
                .filter_map(|id| {
                    let p = doc.object(ObjRef::new(b"PICT", id))?;
                    Some((id, Wmf::from_meta(part(p, b"META")?).ok()?))
                })
                .collect();
            Some(PageBackground {
                background,
                pictures,
            })
        });
        PageArt { border, background }
    }
}

/// Folha branca de `page` a `scale` pixels por ponto, com o fundo e a borda.
pub fn render_page(page: &PageGeom, art: &PageArt, scale: f64) -> Image {
    let (w, h) = (
        (page.width * scale).round().max(1.0) as u32,
        (page.height * scale).round().max(1.0) as u32,
    );
    let mut img = Image {
        width: w,
        height: h,
        rgba: [255, 255, 255, 255].repeat((w * h) as usize),
    };
    let px = Rect::new(0.0, 0.0, page.width * scale, page.height * scale);
    if let Some(bg) = &art.background {
        draw_background(&mut img, px, page, bg);
    }
    if let Some(b) = &art.border {
        draw_border(&mut img, px, page, b);
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Borda sintética de peças quadradas de 480 unidades.
    fn square_border(th: u16) -> Border {
        let mut m = b"META".to_vec();
        for v in [0u16, 0, 480, 480] {
            m.extend_from_slice(&v.to_le_bytes());
        }
        m.extend_from_slice(&[1, 0, 9, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        m.extend_from_slice(&3u32.to_le_bytes());
        m.extend_from_slice(&0u16.to_le_bytes());
        let mut w = Vec::new();
        for v in [1, th, 480, 480, 480] {
            w.extend_from_slice(&v.to_le_bytes());
        }
        w.extend_from_slice(&[0; 8]);
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&(m.len() as u32).to_le_bytes());
        w.extend_from_slice(&m);
        Border::from_parts(0, "x", &w, None).unwrap()
    }

    #[test]
    fn tribe_geometry_matches_the_screen() {
        // Tribe: th 24, folha Alto, margens 72 (w26_95: cantos em x 54..88 e
        // y 143..177 com a folha em (22, 107); 13 peças em cima, de 36 pt na
        // vertical)
        let l = border_layout(&PageGeom::new(595.0, 792.0, 72.0), &square_border(24));
        assert_eq!((l.across, l.down), (13, 18));
        assert!((l.band - 461.0 / 13.0).abs() < 1e-9);
        let first = |v: f64| (v - 0.5).ceil();
        assert_eq!(first(22.0 + l.outer.x), 54.0);
        assert_eq!(first(107.0 + l.outer.y), 143.0);
        assert_eq!(first(22.0 + l.inner.x) - 1.0, 88.0);
        assert!(l.fits(&PageGeom::new(595.0, 792.0, 72.0)));
        assert_eq!(l.tiles.len(), 4 + 2 * 13 + 2 * 18);
    }

    #[test]
    fn paw_geometry_matches_the_screen() {
        // Paw: th 32 → 10 peças de 46,1 pt em cima (12 patas de x 43 a 595)
        let l = border_layout(&PageGeom::new(595.0, 792.0, 72.0), &square_border(32));
        assert_eq!(l.across, 10);
        assert!((l.band - 46.1).abs() < 0.01);
        assert_eq!((22.0 + l.outer.x).round(), 43.0);
        assert_eq!((22.0 + l.outer.x + l.outer.w).round(), 596.0);
    }

    #[test]
    fn narrow_margin_does_not_fit() {
        let page = PageGeom::new(595.0, 792.0, 36.0);
        assert!(!border_layout(&page, &square_border(36)).fits(&page));
        assert!(border_layout(&page, &square_border(16)).fits(&page));
    }

    #[test]
    fn background_ref_from_dop_extra() {
        let mut d = DopProps::default();
        // o leitor deixa o 0x56 em `extra`
        assert_eq!(background_ref(&d), None);
        d.extra.push(max::Sprm {
            code: SPRM_BACKGROUND,
            arg: vec![b'I', b'B', b'G', b'F', 2, 0],
        });
        assert_eq!(background_ref(&d), Some(ObjRef::new(b"IBGF", 2)));
    }

    #[test]
    fn obstacle_sides() {
        let mut f = Frame::new(
            max::FrameKind::Other(6),
            ObjRef::new(b"PICT", 0),
            71,
            127,
            12,
            555,
        );
        f.wrap = Some(2);
        let o = Obstacle::from_frame(&f);
        assert_eq!(o.text_side, TextSide::Right);
        assert_eq!(o.rect, Rect::new(71.0, 127.0, 12.0, 555.0));
    }
}
