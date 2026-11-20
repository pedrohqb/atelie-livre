//! Fontes do Escritório do Creative Writer: as 6 famílias do Seleciona-letras
//! (`STR#` 0x206), os 7 tamanhos do Dimensiona-palavras e os estilos
//! negrito, inclinado e sublinhado, com medição (para a diagramação) e
//! rasterização de trechos com cor.
//!
//! As TrueType do produto (Lucida Sans, Lucida Handwriting, Lucida
//! Blackletter, Impact e Stencil) vinham nos disquetes de instalação e não
//! podem ir com o port. Cada família é resolvida, nesta ordem, para:
//! 1. a fonte original, se estiver instalada no sistema (cópia do usuário);
//! 2. uma substituta livre embutida (`assets/fonts/`, recurso
//!    `embedded-fonts`, ligado por padrão);
//! 3. uma substituta livre instalada no sistema;
//! 4. qualquer fonte sem serifa do sistema.
//!
//! As métricas de linha (ascendente, descendente e entrelinha externa) são
//! sempre as do original, medidas no Writer no Wine com as TrueType do
//! disquete (`docs/telas/writer-fontes.md`): a quebra de página sai igual à
//! do original seja qual for a substituta. A substituta é ampliada na
//! vertical para ter a altura das maiúsculas da original e na horizontal
//! para ter a mesma largura média, e os avanços são arredondados a pixels
//! inteiros por caractere, como no GDI. "Pablo Graphos" é a fonte de bitmap
//! do produto (`SHARED/STUFF/PABLO.FON`), ampliada por fatores inteiros como
//! o GDI faz com fontes de bitmap.
//!
//! A escala é a da Visão normal do original: 1 ponto = 1 pixel.

use super::doc::{CharProps, Document, Size};
use super::layout::{FontKey, Measure};
use crate::frame::Frame;
use crate::paint::ttf;
use ab_glyph::{Font as _, FontArc, FontVec, GlyphId, PxScale, point};
use formats::fnt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// Família do Seleciona-letras.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Family {
    Pablo,
    LucidaSans,
    LucidaHandwriting,
    LucidaBlackletter,
    Impact,
    Stencil,
}

impl Family {
    /// Na ordem do `STR#` 0x206.
    pub const ALL: [Family; 6] = [
        Family::Pablo,
        Family::LucidaSans,
        Family::LucidaHandwriting,
        Family::LucidaBlackletter,
        Family::Impact,
        Family::Stencil,
    ];

    /// Fonte padrão dos documentos (`STR#` 0x800 [1]; ftc 0 do `FONT`).
    pub const DEFAULT: Family = Family::LucidaSans;

    /// Nome original (o que vai na tabela `FONT` do `.MAX`).
    pub fn name(self) -> &'static str {
        match self {
            Family::Pablo => "Pablo Graphos",
            Family::LucidaSans => "Lucida Sans",
            Family::LucidaHandwriting => "Lucida Handwriting",
            Family::LucidaBlackletter => "Lucida Blackletter",
            Family::Impact => "Impact",
            Family::Stencil => "Stencil",
        }
    }

    /// Família pelo nome gravado (sem diferença de maiúsculas; ignora
    /// espaços e zeros no fim; "pablo" é o nome interno da Pablo Graphos).
    pub fn from_name(name: &str) -> Option<Family> {
        let n = name.trim_end_matches(['\0', ' ']).trim();
        if n.eq_ignore_ascii_case("pablo") {
            return Some(Family::Pablo);
        }
        Family::ALL
            .into_iter()
            .find(|f| f.name().eq_ignore_ascii_case(n))
    }

    /// Como [`from_name`](Family::from_name), caindo na fonte padrão quando
    /// o nome não é do Writer (documentos do Mac citam "Times", e os
    /// gravados no Wine sem as fontes, "Liberation Sans").
    pub fn resolve(name: &str) -> Family {
        Family::from_name(name).unwrap_or(Family::DEFAULT)
    }

    fn truetype(self) -> Option<&'static Original> {
        match self {
            Family::Pablo => None,
            Family::LucidaSans => Some(&ORIGINALS[0]),
            Family::LucidaHandwriting => Some(&ORIGINALS[1]),
            Family::LucidaBlackletter => Some(&ORIGINALS[2]),
            Family::Impact => Some(&ORIGINALS[3]),
            Family::Stencil => Some(&ORIGINALS[4]),
        }
    }
}

/// Posição do tamanho na lista do Dimensiona-palavras (Gigante = 0), se
/// `points` é um dos 7.
fn size_index(points: u16) -> Option<usize> {
    Size::ALL.iter().position(|s| s.pt() == points)
}

/// Fonte de um trecho: família, pontos e estilos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunFont {
    pub family: Family,
    pub points: u16,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for RunFont {
    fn default() -> RunFont {
        RunFont::new(Family::DEFAULT, Size::default())
    }
}

impl RunFont {
    pub fn new(family: Family, size: Size) -> RunFont {
        RunFont {
            family,
            points: size.pt(),
            bold: false,
            italic: false,
            underline: false,
        }
    }

    /// Fonte das letras com as propriedades `p` no documento (o nome vem da
    /// tabela `FONT`; nomes que não são do Writer caem na Lucida Sans).
    pub fn of(doc: &Document, p: &CharProps) -> RunFont {
        RunFont {
            underline: p.underline,
            ..RunFont::from_key(&FontKey::of(doc, p))
        }
    }

    /// Fonte de uma medição da diagramação (sem sublinhado, que não muda a
    /// medida).
    pub fn from_key(k: &FontKey) -> RunFont {
        RunFont {
            family: Family::resolve(k.name),
            points: k.size_pt,
            bold: k.bold,
            italic: k.italic,
            underline: false,
        }
    }

    pub fn bold(mut self, on: bool) -> RunFont {
        self.bold = on;
        self
    }

    pub fn italic(mut self, on: bool) -> RunFont {
        self.italic = on;
        self
    }

    pub fn underline(mut self, on: bool) -> RunFont {
        self.underline = on;
        self
    }

    /// Pontos válidos (de 1 a 255; documentos estranhos podem ter 0).
    fn pts(&self) -> u16 {
        self.points.clamp(1, 255)
    }
}

/// Métricas de uma linha numa fonte, em pixels da Visão normal: a base fica
/// `ascent` pixels abaixo do topo; a linha ocupa `ascent + descent +
/// leading` (a entrelinha externa vai embaixo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineMetrics {
    pub ascent: i32,
    pub descent: i32,
    pub leading: i32,
}

impl LineMetrics {
    pub fn height(&self) -> i32 {
        self.ascent + self.descent + self.leading
    }
}

// ---------------------------------------------------------------------------
// Dados das TrueType originais (medidos; ver docs/telas/writer-fontes.md)

/// Métricas da TrueType original de uma família. Os números vêm das tabelas
/// das fontes do disquete e da medição no Writer: só números, nada dos
/// desenhos das letras.
struct Original {
    family: Family,
    /// `head.unitsPerEm`.
    upm: i32,
    /// `OS/2.usWinAscent`, `usWinDescent` e a entrelinha externa do GDI
    /// (`max(0, hhea.lineGap − ((winAscent + winDescent) − (hhea.ascender −
    /// hhea.descender)))`), em unidades da fonte.
    win_ascent: i32,
    win_descent: i32,
    ext_leading: i32,
    /// `post.underlinePosition` (negativo = abaixo da base) e espessura.
    underline_pos: i32,
    underline_thick: i32,
    /// Ascendente, descendente e entrelinha medidos no original em cada
    /// tamanho da lista (Gigante..Micro); o GDI usa a tabela `VDMX` da fonte,
    /// por isso não dá para tirar só das proporções acima.
    lines: [(i32, i32, i32); 7],
    /// Avanço médio sobre [`CALIBRATION`] e altura das maiúsculas ('H'), em
    /// frações do em: para ajustar o tamanho das substitutas.
    avg_advance: f32,
    cap_height: f32,
    /// Letras minúsculas desenhadas como maiúsculas (a Stencil só tem
    /// maiúsculas: 'a' e 'A' são o mesmo glifo).
    caps_only: bool,
    /// Inclinação própria do desenho (a Lucida Handwriting é cursiva, 11,25°),
    /// dada às substitutas retas.
    slant: f32,
    /// Substitutas livres no sistema, em ordem de preferência.
    system: &'static [&'static str],
}

/// Texto usado para comparar larguras (pt-BR, com acentos e maiúsculas).
pub const CALIBRATION: &str = "Era uma vez um menino que gostava de escrever histórias sobre dragões, piratas e naves. O capitão João navegou até a ilha do tesouro! Será que você adivinha? ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789";

const ORIGINALS: [Original; 5] = [
    Original {
        family: Family::LucidaSans,
        upm: 2048,
        win_ascent: 1900,
        win_descent: 432,
        ext_leading: 80,
        underline_pos: -200,
        underline_thick: 100,
        lines: [
            (45, 10, 2),
            (33, 7, 1),
            (22, 5, 1),
            (17, 4, 1),
            (13, 3, 1),
            (12, 3, 0),
            (10, 3, 0),
        ],
        avg_advance: 0.5203,
        cap_height: 0.7227,
        caps_only: false,
        slant: 0.0,
        system: &[
            "DejaVu Sans",
            "Verdana",
            "Open Sans",
            "Noto Sans",
            "Liberation Sans",
            "FreeSans",
        ],
    },
    Original {
        family: Family::LucidaHandwriting,
        upm: 2048,
        win_ascent: 1964,
        win_descent: 666,
        ext_leading: 195,
        underline_pos: -200,
        underline_thick: 100,
        lines: [
            (47, 16, 5),
            (36, 12, 3),
            (23, 8, 2),
            (17, 6, 2),
            (14, 5, 1),
            (12, 4, 1),
            (10, 4, 1),
        ],
        avg_advance: 0.6030,
        cap_height: 0.8105,
        caps_only: false,
        // tan(11,25°)
        slant: 0.199,
        system: &["Merienda", "Courgette", "Kalam", "Comic Neue"],
    },
    Original {
        family: Family::LucidaBlackletter,
        upm: 2048,
        win_ascent: 1962,
        win_descent: 445,
        ext_leading: 197,
        underline_pos: -200,
        underline_thick: 100,
        lines: [
            (47, 10, 5),
            (36, 8, 3),
            (24, 5, 2),
            (17, 4, 2),
            (14, 3, 1),
            (13, 3, 1),
            (11, 3, 1),
        ],
        avg_advance: 0.4771,
        cap_height: 0.7407,
        caps_only: false,
        slant: 0.0,
        system: &["UnifrakturMaguntia", "UnifrakturCook", "New Rocker"],
    },
    Original {
        family: Family::Impact,
        upm: 2048,
        win_ascent: 2066,
        win_descent: 432,
        ext_leading: 0,
        underline_pos: -205,
        underline_thick: 102,
        lines: [
            (50, 10, 0),
            (36, 8, 0),
            (24, 5, 0),
            (18, 4, 0),
            (15, 3, 0),
            (13, 3, 0),
            (11, 2, 0),
        ],
        avg_advance: 0.4293,
        cap_height: 0.7905,
        caps_only: false,
        slant: 0.0,
        system: &["Anton", "Oswald", "League Gothic"],
    },
    Original {
        family: Family::Stencil,
        upm: 2048,
        win_ascent: 1860,
        win_descent: 567,
        ext_leading: 0,
        underline_pos: -275,
        underline_thick: 126,
        lines: [
            (44, 13, 0),
            (33, 10, 0),
            (22, 7, 0),
            (16, 5, 0),
            (13, 4, 0),
            (11, 3, 0),
            (8, 2, 0),
        ],
        avg_advance: 0.5535,
        cap_height: 0.6665,
        caps_only: true,
        slant: 0.0,
        system: &["Stardos Stencil", "Allerta Stencil", "Saira Stencil One"],
    },
];

/// Arredondamento do GDI/FreeType para métricas (26.6, meio para cima).
fn scale_round(v: i32, points: u16, upm: i32) -> i32 {
    let x = v as i64 * points as i64 * 64 / upm as i64;
    if x >= 0 {
        ((x + 32) >> 6) as i32
    } else {
        -((-x + 32) >> 6) as i32
    }
}

impl Original {
    fn line(&self, points: u16) -> LineMetrics {
        if let Some(i) = size_index(points) {
            let (ascent, descent, leading) = self.lines[i];
            return LineMetrics {
                ascent,
                descent,
                leading,
            };
        }
        LineMetrics {
            ascent: scale_round(self.win_ascent, points, self.upm),
            descent: scale_round(self.win_descent, points, self.upm),
            leading: scale_round(self.ext_leading, points, self.upm),
        }
    }

    /// Topo e espessura do sublinhado, relativos à base.
    fn underline(&self, points: u16) -> (i32, i32) {
        let thick = scale_round(self.underline_thick, points, self.upm).max(1);
        let pos = scale_round(-self.underline_pos, points, self.upm);
        (pos - thick / 2, thick)
    }
}

/// Inclinação do inclinado simulado (o GDI inclina a fonte normal; medido
/// ≈ 8 px em 33 linhas na Lucida Sans de 48 pontos).
pub const SYNTHETIC_SLANT: f32 = 0.24;

// ---------------------------------------------------------------------------
// Pablo Graphos (bitmap)

/// Altura de caractere (altura menos entrelinha interna), ascendente, altura
/// da célula e entrelinha externa das três fontes de `PABLO.FON` (6, 8 e 16
/// pontos), usadas quando os dados originais não estão disponíveis para
/// conferir.
pub const PABLO_STRIKES: [(i32, i32, i32, i32); 3] =
    [(9, 9, 12, 1), (11, 11, 13, 1), (16, 17, 21, 2)];

/// Fonte de bitmap e fator inteiro que o mapeador do GDI escolhe para uma
/// altura de caractere de `ppem` pixels, dadas as alturas de caractere das
/// fontes disponíveis: penalidade de 150 por pixel de diferença, mais 600 se
/// passar do pedido, mais 20 por fator de ampliação e 50 por ampliar
/// (pesos do mapeador de fontes do Windows 3.1; o Wine não amplia fontes de
/// bitmap, por isso isto não foi medido).
pub fn pablo_strike(char_heights: &[i32], ppem: i32) -> Option<(usize, i32)> {
    let mut best: Option<(i32, usize, i32)> = None;
    for (i, &ch) in char_heights.iter().enumerate() {
        if ch <= 0 {
            continue;
        }
        for k in 1..=8 {
            let diff = ch * k - ppem;
            let mut pen = 150 * diff.abs() + if diff > 0 { 600 } else { 0 };
            if k > 1 {
                pen += 50 + 20 * (k - 1);
            }
            if best.is_none_or(|b| pen < b.0) {
                best = Some((pen, i, k));
            }
        }
    }
    best.map(|(_, i, k)| (i, k))
}

struct PabloStrike {
    font: Rc<fnt::Font>,
    char_height: i32,
    ext_leading: i32,
}

/// Linhas da Pablo Graphos sem os dados (a mesma escolha de fonte e fator).
fn pablo_line_default(points: u16) -> LineMetrics {
    let heights: Vec<i32> = PABLO_STRIKES.iter().map(|s| s.0).collect();
    let (i, k) = pablo_strike(&heights, points as i32).unwrap_or((1, 1));
    let (_, asc, h, el) = PABLO_STRIKES[i];
    LineMetrics {
        ascent: asc * k,
        descent: (h - asc) * k,
        leading: el * k,
    }
}

/// Métricas de linha da fonte do trecho, sem carregar fontes (são as do
/// original em todas as resoluções de família).
pub fn line_metrics(font: &RunFont) -> LineMetrics {
    match font.family.truetype() {
        Some(o) => o.line(font.pts()),
        None => pablo_line_default(font.pts()),
    }
}

// ---------------------------------------------------------------------------
// Substitutas

/// De onde veio a fonte usada para uma família.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A TrueType original, instalada no sistema.
    Original(PathBuf),
    /// Substituta embutida no programa (nome do arquivo em `assets/fonts/`).
    Embedded(&'static str),
    /// Substituta instalada no sistema (nome da família).
    System(String),
    /// A Pablo Graphos dos dados originais.
    Bitmap,
    /// Nada disponível: a família é desenhada com a da Lucida Sans (ou, sem
    /// nenhuma, não desenha nada).
    Missing,
}

/// Substitutas embutidas: (família, arquivo, dados).
#[cfg(feature = "embedded-fonts")]
const EMBEDDED: [(Family, &str, &[u8]); 5] = [
    (
        Family::LucidaSans,
        "DejaVuSans-cp1252.ttf",
        include_bytes!("../../../../assets/fonts/DejaVuSans-cp1252.ttf"),
    ),
    (
        Family::LucidaHandwriting,
        "Merienda-wght.ttf",
        include_bytes!("../../../../assets/fonts/Merienda-wght.ttf"),
    ),
    (
        Family::LucidaBlackletter,
        "UnifrakturMaguntia-Book.ttf",
        include_bytes!("../../../../assets/fonts/UnifrakturMaguntia-Book.ttf"),
    ),
    (
        Family::Impact,
        "Anton-Regular.ttf",
        include_bytes!("../../../../assets/fonts/Anton-Regular.ttf"),
    ),
    (
        Family::Stencil,
        "StardosStencil-Bold.ttf",
        include_bytes!("../../../../assets/fonts/StardosStencil-Bold.ttf"),
    ),
];

#[cfg(not(feature = "embedded-fonts"))]
const EMBEDDED: [(Family, &str, &[u8]); 0] = [];

/// Uma TrueType pronta para uma família: a fonte e os ajustes para ficar
/// com o tamanho do original.
#[derive(Clone)]
struct Face {
    font: FontArc,
    source: Source,
    /// Pixels por em na horizontal e na vertical, por ponto.
    x_scale: f32,
    y_scale: f32,
    /// Inclinação própria (substituta reta de uma original cursiva).
    slant: f32,
    caps_only: bool,
}

impl Face {
    /// Prepara a fonte para imitar a original `o`. A original instalada fica
    /// como está; uma substituta é ajustada pela largura média e altura das
    /// maiúsculas (limitado a ±30%).
    fn new(font: FontArc, source: Source, o: &Original) -> Face {
        if matches!(source, Source::Original(_)) {
            return Face {
                font,
                source,
                x_scale: 1.0,
                y_scale: 1.0,
                slant: 0.0,
                caps_only: false,
            };
        }
        let (avg, cap) = em_metrics(&font, o.caps_only);
        let fit = |want: f32, got: f32| {
            if got > 0.0 {
                (want / got).clamp(0.7, 1.3)
            } else {
                1.0
            }
        };
        Face {
            x_scale: fit(o.avg_advance, avg),
            y_scale: fit(o.cap_height, cap),
            slant: o.slant,
            caps_only: o.caps_only,
            font,
            source,
        }
    }

    fn glyph(&self, c: char) -> GlyphId {
        let c = if self.caps_only {
            c.to_uppercase().next().unwrap_or(c)
        } else {
            c
        };
        self.font.glyph_id(c)
    }

    /// Escala do ab_glyph (que mede pela altura ascendente − descendente)
    /// para `ppem_x` × `ppem_y` pixels por em.
    fn px_scale(&self, points: u16) -> PxScale {
        let upm = self.font.units_per_em().unwrap_or(1000.0);
        let k = self.font.height_unscaled() / upm;
        let p = points as f32;
        PxScale {
            x: p * self.x_scale * k,
            y: p * self.y_scale * k,
        }
    }

    /// Avanço arredondado do glifo (como o GDI, por caractere).
    fn advance(&self, c: char, points: u16) -> i32 {
        let upm = self.font.units_per_em().unwrap_or(1000.0);
        let adv = self.font.h_advance_unscaled(self.glyph(c));
        (adv * points as f32 * self.x_scale / upm).round() as i32
    }
}

/// Avanço médio sobre [`CALIBRATION`] e altura do 'H', em frações do em.
fn em_metrics(font: &FontArc, caps_only: bool) -> (f32, f32) {
    let upm = font.units_per_em().unwrap_or(1000.0);
    let text: String = if caps_only {
        CALIBRATION.to_uppercase()
    } else {
        CALIBRATION.to_string()
    };
    let n = text.chars().count().max(1) as f32;
    let avg = text
        .chars()
        .map(|c| font.h_advance_unscaled(font.glyph_id(c)))
        .sum::<f32>()
        / upm
        / n;
    // topo da tinta do 'H' desenhado grande (os limites do contorno do
    // ab_glyph são os dos pontos de controle, maiores que a tinta nas curvas)
    let k = font.height_unscaled() / upm;
    let big = 400.0;
    let h = font
        .glyph_id('H')
        .with_scale_and_position(PxScale::from(big * k), point(0.0, 0.0));
    let cap = font.outline_glyph(h).map_or(0.0, |o| {
        let top = o.px_bounds().min.y;
        let mut first = f32::MAX;
        o.draw(|_, y, v| {
            if v >= 0.5 {
                first = first.min(y as f32);
            }
        });
        if first == f32::MAX {
            0.0
        } else {
            -(top + first) / big
        }
    });
    (avg, cap)
}

fn load_path(path: &std::path::Path, index: u32) -> Option<FontArc> {
    let data = std::fs::read(path).ok()?;
    FontVec::try_from_vec_and_index(data, index)
        .ok()
        .map(FontArc::new)
}

/// Fonte da família: a original instalada, a embutida ou uma do sistema.
fn resolve_face(o: &Original, use_system: bool) -> Option<Face> {
    if use_system
        && let Some(f) = ttf::find_any_style(o.family.name())
        && let Some(font) = load_path(&f.path, f.index)
    {
        return Some(Face::new(font, Source::Original(f.path.clone()), o));
    }
    if let Some(&(_, file, data)) = EMBEDDED.iter().find(|e| e.0 == o.family)
        && let Ok(font) = FontArc::try_from_slice(data)
    {
        return Some(Face::new(font, Source::Embedded(file), o));
    }
    if !use_system {
        return None;
    }
    for name in o.system {
        if let Some(f) = ttf::find_any_style(name)
            && let Some(font) = load_path(&f.path, f.index)
        {
            return Some(Face::new(font, Source::System(f.name.clone()), o));
        }
    }
    let f = ttf::default_face()?;
    let font = load_path(&f.path, f.index)?;
    Some(Face::new(font, Source::System(f.name.clone()), o))
}

// ---------------------------------------------------------------------------
// Rasterização

/// Cobertura (0–255) de um trecho desenhado, com a origem da caneta em
/// (0, 0) no topo da linha: o pixel (`x + i`, `y + j`) da linha tem
/// cobertura `cov[j * w + i]`. `x` pode ser negativo (inclinação e
/// sobras à esquerda dos glifos); `advance` é a largura do trecho na linha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextMask {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub cov: Vec<u8>,
    pub advance: i32,
    pub baseline: i32,
}

impl TextMask {
    fn new(x: i32, w: i32, h: i32, advance: i32, baseline: i32) -> TextMask {
        TextMask {
            x,
            y: 0,
            w: w.max(0),
            h: h.max(0),
            cov: vec![0; (w.max(0) * h.max(0)) as usize],
            advance,
            baseline,
        }
    }

    /// Cobertura no pixel (`px`, `py`) relativo à origem da caneta.
    pub fn at(&self, px: i32, py: i32) -> u8 {
        let (i, j) = (px - self.x, py - self.y);
        if i < 0 || j < 0 || i >= self.w || j >= self.h {
            return 0;
        }
        self.cov[(j * self.w + i) as usize]
    }

    fn put_max(&mut self, px: i32, py: i32, v: u8) {
        let (i, j) = (px - self.x, py - self.y);
        if i < 0 || j < 0 || i >= self.w || j >= self.h {
            return;
        }
        let c = &mut self.cov[(j * self.w + i) as usize];
        *c = (*c).max(v);
    }

    /// Caixa da tinta (x0, y0, x1, y1 exclusivos), relativa à origem.
    pub fn ink_bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let mut b: Option<(i32, i32, i32, i32)> = None;
        for j in 0..self.h {
            for i in 0..self.w {
                if self.cov[(j * self.w + i) as usize] >= 128 {
                    let (x, y) = (self.x + i, self.y + j);
                    b = Some(match b {
                        None => (x, y, x + 1, y + 1),
                        Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x + 1), e.max(y + 1)),
                    });
                }
            }
        }
        b
    }

    /// Engrossa (negrito simulado): cada pixel repete `r` pixels à direita.
    fn embolden(&mut self, r: i32) {
        for j in 0..self.h {
            let row = (j * self.w) as usize;
            for i in (0..self.w).rev() {
                let mut v = self.cov[row + i as usize];
                for d in 1..=r {
                    if i - d >= 0 {
                        v = v.max(self.cov[row + (i - d) as usize]);
                    }
                }
                self.cov[row + i as usize] = v;
            }
        }
    }

    /// Inclina: linhas acima da base vão para a direita (`slant` pixels por
    /// linha), as de baixo para a esquerda.
    fn shear(&mut self, slant: f32) {
        if slant == 0.0 {
            return;
        }
        let mut out = vec![0u8; self.cov.len()];
        for j in 0..self.h {
            let shift = ((self.baseline - (self.y + j)) as f32 * slant).round() as i32;
            for i in 0..self.w {
                let src = i - shift;
                if src >= 0 && src < self.w {
                    out[(j * self.w + i) as usize] = self.cov[(j * self.w + src) as usize];
                }
            }
        }
        self.cov = out;
    }

    /// Desenha no quadro com o canto da origem em (`x`, `top`): `smooth`
    /// mistura as bordas com o fundo; sem ele (como nas telas de 1995), só
    /// os pixels com mais de metade de cobertura, na cor cheia.
    pub fn draw(&self, frame: &mut Frame, x: i32, top: i32, rgb: [u8; 3], smooth: bool) {
        for j in 0..self.h {
            for i in 0..self.w {
                let a = self.cov[(j * self.w + i) as usize];
                if a == 0 {
                    continue;
                }
                let (px, py) = (x + self.x + i, top + self.y + j);
                if px < 0 || py < 0 || px >= frame.width() as i32 || py >= frame.height() as i32 {
                    continue;
                }
                if !smooth {
                    if a >= 128 {
                        frame.put(px, py, rgb);
                    }
                    continue;
                }
                let bg = frame.pixel(px as u32, py as u32);
                let mix = |f: u8, b: u8| {
                    ((f as u32 * a as u32 + b as u32 * (255 - a as u32)) / 255) as u8
                };
                frame.put(
                    px,
                    py,
                    [mix(rgb[0], bg[0]), mix(rgb[1], bg[1]), mix(rgb[2], bg[2])],
                );
            }
        }
    }
}

/// As fontes do Writer, prontas para medir e desenhar.
pub struct WriterFonts {
    faces: HashMap<Family, Face>,
    pablo: Vec<PabloStrike>,
    masks: RefCell<HashMap<(RunFont, String), Rc<TextMask>>>,
}

impl std::fmt::Debug for WriterFonts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("WriterFonts");
        for fam in Family::ALL {
            d.field(fam.name(), &self.source(fam));
        }
        d.finish()
    }
}

impl WriterFonts {
    /// Resolve as famílias (original instalada → embutida → sistema) com as
    /// fontes da Pablo Graphos dos dados originais (podem faltar).
    pub fn new(pablo: Vec<Rc<fnt::Font>>) -> WriterFonts {
        WriterFonts::build(pablo, true)
    }

    /// Só as substitutas embutidas, sem olhar o sistema: o mesmo resultado
    /// em qualquer máquina (testes).
    pub fn embedded(pablo: Vec<Rc<fnt::Font>>) -> WriterFonts {
        WriterFonts::build(pablo, false)
    }

    fn build(pablo: Vec<Rc<fnt::Font>>, use_system: bool) -> WriterFonts {
        let faces = ORIGINALS
            .iter()
            .filter_map(|o| resolve_face(o, use_system).map(|f| (o.family, f)))
            .collect();
        let mut pablo: Vec<PabloStrike> = pablo
            .into_iter()
            .map(|font| {
                let (ch, el) = PABLO_STRIKES
                    .iter()
                    .find(|s| s.2 == font.height as i32)
                    .map_or((font.height as i32, 0), |s| (s.0, s.3));
                PabloStrike {
                    font,
                    char_height: ch,
                    ext_leading: el,
                }
            })
            .collect();
        pablo.sort_by_key(|s| s.char_height);
        WriterFonts {
            faces,
            pablo,
            masks: RefCell::new(HashMap::new()),
        }
    }

    /// De onde vem a fonte da família.
    pub fn source(&self, family: Family) -> Source {
        match family {
            Family::Pablo if !self.pablo.is_empty() => Source::Bitmap,
            Family::Pablo => Source::Missing,
            f => self
                .faces
                .get(&f)
                .map_or(Source::Missing, |face| face.source.clone()),
        }
    }

    /// TrueType que desenha a família (a da Lucida Sans quando falta).
    fn face(&self, family: Family) -> Option<&Face> {
        self.faces
            .get(&family)
            .or_else(|| self.faces.get(&Family::LucidaSans))
    }

    fn pablo_pick(&self, points: u16) -> Option<(&PabloStrike, i32)> {
        let heights: Vec<i32> = self.pablo.iter().map(|s| s.char_height).collect();
        let (i, k) = pablo_strike(&heights, points as i32)?;
        Some((&self.pablo[i], k))
    }

    /// Topo e espessura do sublinhado relativos à base.
    fn underline(&self, font: &RunFont) -> (i32, i32) {
        match font.family.truetype() {
            Some(o) => o.underline(font.pts()),
            None => {
                let k = self.pablo_pick(font.pts()).map_or(1, |p| p.1);
                (k, k)
            }
        }
    }

    /// Desenha o trecho numa máscara de cobertura (guardada para reusar).
    pub fn render(&self, font: &RunFont, text: &str) -> Rc<TextMask> {
        let key = (*font, text.to_string());
        if let Some(m) = self.masks.borrow().get(&key) {
            return m.clone();
        }
        let m = Rc::new(self.render_uncached(font, text));
        let mut cache = self.masks.borrow_mut();
        if cache.len() > 4096 {
            cache.clear();
        }
        cache.insert(key, m.clone());
        m
    }

    fn render_uncached(&self, font: &RunFont, text: &str) -> TextMask {
        let lm = self.line_metrics(font);
        let p = font.pts();
        let advance = self.text_width(font, text);
        let face = (font.family != Family::Pablo || self.pablo.is_empty())
            .then(|| self.face(font.family))
            .flatten();
        let own_slant = face.map_or(0.0, |f| f.slant);
        let italic_slant =
            if font.italic && own_slant == 0.0 && font.family != Family::LucidaHandwriting {
                SYNTHETIC_SLANT
            } else {
                0.0
            };
        let slant = own_slant + italic_slant;
        let bold_r = if font.bold { (p as i32 / 24).max(1) } else { 0 };
        // margem para inclinação e glifos que passam da caixa
        let pad = (lm.height() as f32 * (slant + 0.2)).ceil() as i32 + 2;
        let mut m = TextMask::new(
            -pad,
            advance + 2 * pad + bold_r,
            lm.height(),
            advance,
            lm.ascent,
        );
        let mut pen = 0;
        match (face, self.pablo_pick(p)) {
            (Some(face), _) => {
                let scale = face.px_scale(p);
                for c in text.chars() {
                    let g = face
                        .glyph(c)
                        .with_scale_and_position(scale, point(pen as f32, lm.ascent as f32));
                    if let Some(o) = face.font.outline_glyph(g) {
                        let b = o.px_bounds();
                        let (bx, by) = (b.min.x as i32, b.min.y as i32);
                        o.draw(|gx, gy, v| {
                            let a = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                            m.put_max(bx + gx as i32, by + gy as i32, a);
                        });
                    }
                    pen += self.advance(font, c);
                }
            }
            (None, Some((strike, k))) => {
                let top = lm.ascent - strike.font.ascent as i32 * k;
                for c in text.chars() {
                    let g = strike
                        .font
                        .glyph(formats::cp1252::encode_char(c).unwrap_or(b'?'));
                    let w = g.width as i32;
                    for (n, &on) in g.bits.iter().enumerate() {
                        if !on || w == 0 {
                            continue;
                        }
                        let (gx, gy) = (n as i32 % w, n as i32 / w);
                        for dy in 0..k {
                            for dx in 0..k {
                                m.put_max(pen + gx * k + dx, top + gy * k + dy, 255);
                            }
                        }
                    }
                    pen += self.advance(font, c);
                }
            }
            (None, None) => {}
        }
        if bold_r > 0 {
            m.embolden(bold_r);
        }
        m.shear(slant);
        if font.underline && advance > 0 {
            let (off, thick) = self.underline(font);
            for y in lm.ascent + off..lm.ascent + off + thick {
                for x in 0..advance {
                    m.put_max(x, y.min(lm.height() - 1), 255);
                }
            }
        }
        m
    }

    /// Desenha o trecho no quadro com a origem da caneta em (`x`, `top`) (topo
    /// da linha) e devolve o avanço.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        frame: &mut Frame,
        x: i32,
        top: i32,
        font: &RunFont,
        text: &str,
        rgb: [u8; 3],
        smooth: bool,
    ) -> i32 {
        let m = self.render(font, text);
        m.draw(frame, x, top, rgb, smooth);
        m.advance
    }
}

/// Medição com as fontes de verdade (a diagramação usa a mesma coisa pela
/// [`Measure`]).
impl WriterFonts {
    /// Métricas da linha na fonte do trecho.
    pub fn line_metrics(&self, font: &RunFont) -> LineMetrics {
        if font.family == Family::Pablo
            && let Some((s, k)) = self.pablo_pick(font.pts())
        {
            let asc = s.font.ascent as i32;
            let h = s.font.height as i32;
            return LineMetrics {
                ascent: asc * k,
                descent: (h - asc) * k,
                leading: s.ext_leading * k,
            };
        }
        line_metrics(font)
    }

    /// Avanço do caractere, em pixels inteiros.
    pub fn advance(&self, font: &RunFont, c: char) -> i32 {
        let bold = i32::from(font.bold);
        if font.family == Family::Pablo
            && let Some((s, k)) = self.pablo_pick(font.pts())
        {
            let g = s
                .font
                .glyph(formats::cp1252::encode_char(c).unwrap_or(b'?'));
            return g.width as i32 * k + bold;
        }
        match self.face(font.family) {
            Some(face) => face.advance(c, font.pts()) + bold,
            // sem nenhuma TrueType: meio ponto por caractere
            None => (font.pts() as i32 + 1) / 2 + bold,
        }
    }

    /// Largura do texto (soma dos avanços; sem kerning, como no GDI).
    pub fn text_width(&self, font: &RunFont, text: &str) -> i32 {
        text.chars().map(|c| self.advance(font, c)).sum()
    }

    /// Largura e altura (da linha) do trecho.
    pub fn extent(&self, font: &RunFont, text: &str) -> (i32, i32) {
        (
            self.text_width(font, text),
            self.line_metrics(font).height(),
        )
    }
}

/// A medição da diagramação (`layout::Measure`): a entrelinha externa vai
/// junto com o descendente, embaixo da linha, como no original.
impl Measure for WriterFonts {
    fn advance(&self, font: &FontKey, c: char) -> i32 {
        WriterFonts::advance(self, &RunFont::from_key(font), c)
    }

    fn metrics(&self, font: &FontKey) -> (i32, i32) {
        let m = self.line_metrics(&RunFont::from_key(font));
        (m.ascent, m.descent + m.leading)
    }

    /// Altura das maiúsculas da TrueType original (as substitutas são
    /// ajustadas a ela); na Pablo, 3/4 do ascendente.
    fn cap_height(&self, font: &FontKey) -> i32 {
        let f = RunFont::from_key(font);
        match f.family.truetype() {
            Some(o) => (o.cap_height * f32::from(f.pts())).round() as i32,
            None => (self.line_metrics(&f).ascent * 3 + 2) / 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_measure_puts_the_leading_under_the_line() {
        let doc = Document::new();
        let p = CharProps {
            size: Size::Giant,
            bold: true,
            underline: true,
            ..CharProps::default()
        };
        let f = RunFont::of(&doc, &p);
        assert_eq!(f.family, Family::LucidaSans);
        assert_eq!((f.points, f.bold, f.underline), (48, true, true));
        let key = FontKey::of(&doc, &p);
        assert!(!RunFont::from_key(&key).underline);
        let w = WriterFonts::embedded(Vec::new());
        assert_eq!(Measure::metrics(&w, &key), (45, 12));
        assert_eq!(Measure::advance(&w, &key, 'M'), w.advance(&f, 'M'));
        assert_eq!(size_index(9), Some(6));
        assert_eq!(size_index(10), None);
    }

    #[test]
    fn families_by_name() {
        assert_eq!(Family::ALL.map(|f| f.name())[0], "Pablo Graphos");
        assert_eq!(Family::from_name("lucida sans"), Some(Family::LucidaSans));
        assert_eq!(Family::from_name("STENCIL"), Some(Family::Stencil));
        assert_eq!(Family::from_name("pablo"), Some(Family::Pablo));
        assert_eq!(Family::from_name("Impact\0\0"), Some(Family::Impact));
        assert_eq!(Family::from_name("Liberation Sans"), None);
        assert_eq!(Family::resolve("Times"), Family::LucidaSans);
        assert_eq!(RunFont::default().family, Family::LucidaSans);
        assert_eq!(RunFont::default().points, 12);
    }

    fn heights(f: Family) -> Vec<i32> {
        Size::ALL
            .iter()
            .map(|&s| line_metrics(&RunFont::new(f, s)).height())
            .collect()
    }

    #[test]
    fn cap_height_follows_the_original_metrics() {
        let f = WriterFonts::embedded(Vec::new());
        let key = |name, size_pt| FontKey {
            name,
            size_pt,
            bold: false,
            italic: false,
        };
        // Lucida Sans: 0,7227 do em
        assert_eq!(f.cap_height(&key("Lucida Sans", 12)), 9);
        assert_eq!(f.cap_height(&key("Lucida Sans", 54)), 39);
    }

    #[test]
    fn line_heights_are_the_ones_measured_in_the_original() {
        // altura do cursor em cada tamanho, Gigante..Micro (Wine, fontes do disquete)
        assert_eq!(heights(Family::LucidaSans), [57, 41, 28, 22, 17, 15, 13]);
        assert_eq!(heights(Family::Impact), [60, 44, 29, 22, 18, 16, 13]);
        assert_eq!(
            heights(Family::LucidaHandwriting),
            [68, 51, 33, 25, 20, 17, 15]
        );
        assert_eq!(
            heights(Family::LucidaBlackletter),
            [62, 47, 31, 23, 18, 17, 15]
        );
        assert_eq!(heights(Family::Stencil), [57, 43, 29, 21, 17, 14, 10]);
        // base da Mínima: 12 px abaixo do topo da linha
        let m = line_metrics(&RunFont::default());
        assert_eq!((m.ascent, m.descent, m.leading), (12, 3, 0));
    }

    #[test]
    fn other_sizes_use_the_gdi_formula() {
        // 10 pt de Lucida Sans: 1900·10/2048 = 9,28 → 9; 432·10/2048 = 2,11 → 2; 80·10/2048 → 0
        let f = RunFont {
            points: 10,
            ..RunFont::default()
        };
        assert_eq!(
            line_metrics(&f),
            LineMetrics {
                ascent: 9,
                descent: 2,
                leading: 0
            }
        );
        // a fórmula sem VDMX dá o mesmo que a tabela nos tamanhos da Stencil
        let o = Family::Stencil.truetype().unwrap();
        for s in Size::ALL {
            let p = s.pt();
            let (a, d, l) = o.lines[size_index(p).unwrap()];
            assert_eq!(
                (a, d, l),
                (
                    scale_round(o.win_ascent, p, o.upm),
                    scale_round(o.win_descent, p, o.upm),
                    scale_round(o.ext_leading, p, o.upm)
                ),
                "{p} pt"
            );
        }
    }

    #[test]
    fn underline_matches_the_original_lucida() {
        // topo do sublinhado abaixo da base e espessura, medidos (Gigante..Micro)
        let o = Family::LucidaSans.truetype().unwrap();
        let got: Vec<(i32, i32)> = Size::ALL.iter().map(|s| o.underline(s.pt())).collect();
        assert_eq!(
            got,
            [(4, 2), (3, 2), (2, 1), (2, 1), (1, 1), (1, 1), (1, 1)]
        );
    }

    #[test]
    fn pablo_scaling_follows_the_gdi_mapper() {
        let h = [9, 11, 16];
        let got: Vec<Option<(usize, i32)>> = Size::ALL
            .iter()
            .map(|s| pablo_strike(&h, s.pt() as i32))
            .collect();
        assert_eq!(
            got,
            [
                Some((2, 3)), // 48: 16 × 3
                Some((0, 4)), // 36: 9 × 4
                Some((1, 2)), // 24: 11 × 2
                Some((0, 2)), // 18: 9 × 2
                Some((1, 1)), // 14: 11 (menor é melhor que maior)
                Some((1, 1)), // 12: 11
                Some((0, 1)), // 9: 9
            ]
        );
        assert_eq!(pablo_strike(&[], 12), None);
        // sem os dados, as linhas seguem a mesma escolha
        let m = line_metrics(&RunFont::new(Family::Pablo, Size::Giant));
        assert_eq!((m.ascent, m.descent, m.leading), (51, 12, 6));
    }

    #[test]
    fn mask_shear_and_embolden() {
        let mut m = TextMask::new(0, 4, 3, 4, 2);
        m.cov[0] = 255; // linha 0, 2 acima da base
        m.cov[2 * 4 + 1] = 255; // linha da base
        m.shear(0.5);
        assert_eq!(m.at(1, 0), 255);
        assert_eq!(m.at(1, 2), 255);
        m.embolden(1);
        assert_eq!(m.at(2, 0), 255);
        assert_eq!(m.ink_bounds(), Some((1, 0, 3, 3)));
    }
}
