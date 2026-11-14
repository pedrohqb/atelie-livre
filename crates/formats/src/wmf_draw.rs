//! Rasterizador de metarquivos do Windows (WMF) vetoriais: as peças das
//! bordas (`WBDR` nos `.BOR`) e as figuras `META` (com `StretchDIB`).
//!
//! O metarquivo é tocado como o GDI do Windows 3.1 tocaria num contexto
//! cuja janela lógica (`SetWindowOrg`/`SetWindowExt`) é levada para um
//! retângulo de destino em pixels, sem suavização (o original desenha em
//! 16 cores ou em 1 bit). Ver `docs/formats/wmf.md`.
//!
//! Registros tratados:
//!
//! | função | registro |
//! |---|---|
//! | janela | `SetWindowOrg` 0x020B, `SetWindowExt` 0x020C, `OffsetWindowOrg` 0x020F, `ScaleWindowExt` 0x0410 |
//! | estado | `SaveDC` 0x001E, `RestoreDC` 0x0127, `SetROP2` 0x0104, `SetBkMode` 0x0102, `SetBkColor` 0x0201, `SetPolyFillMode` 0x0106, `IntersectClipRect` 0x0416 |
//! | objetos | `CreatePenIndirect` 0x02FA, `CreateBrushIndirect` 0x02FC, `DibCreatePatternBrush` 0x0142, `SelectObject` 0x012D, `DeleteObject` 0x01F0 (e os outros `Create*` só ocupam a tabela) |
//! | desenho | `MoveTo` 0x0214, `LineTo` 0x0213, `Polyline` 0x0325, `Polygon` 0x0324, `PolyPolygon` 0x0538, `Rectangle` 0x041B, `RoundRect` 0x061C, `Ellipse` 0x0418, `Arc` 0x0817, `Pie` 0x081A, `Chord` 0x0830 |
//! | bitmaps | `StretchDIB` 0x0F43, `DIBStretchBlt` 0x0B41, `DIBBitBlt` 0x0940 |
//!
//! Os demais (texto, paletas, modos de mapeamento, viewport) são ignorados.

use crate::winres::{Image, decode_bitmap};
use crate::{ErrorKind, Format, Reader, Result};

/// Funções WMF usadas pelo rasterizador.
pub mod func {
    pub const SAVE_DC: u16 = 0x001E;
    pub const SET_BK_MODE: u16 = 0x0102;
    pub const SET_MAP_MODE: u16 = 0x0103;
    pub const SET_ROP2: u16 = 0x0104;
    pub const SET_POLY_FILL_MODE: u16 = 0x0106;
    pub const RESTORE_DC: u16 = 0x0127;
    pub const SELECT_OBJECT: u16 = 0x012D;
    pub const DIB_CREATE_PATTERN_BRUSH: u16 = 0x0142;
    pub const DELETE_OBJECT: u16 = 0x01F0;
    pub const CREATE_PATTERN_BRUSH: u16 = 0x01F9;
    pub const SET_BK_COLOR: u16 = 0x0201;
    pub const SET_WINDOW_ORG: u16 = 0x020B;
    pub const SET_WINDOW_EXT: u16 = 0x020C;
    pub const OFFSET_WINDOW_ORG: u16 = 0x020F;
    pub const LINE_TO: u16 = 0x0213;
    pub const MOVE_TO: u16 = 0x0214;
    pub const CREATE_PEN_INDIRECT: u16 = 0x02FA;
    pub const CREATE_FONT_INDIRECT: u16 = 0x02FB;
    pub const CREATE_BRUSH_INDIRECT: u16 = 0x02FC;
    pub const POLYGON: u16 = 0x0324;
    pub const POLYLINE: u16 = 0x0325;
    pub const SCALE_WINDOW_EXT: u16 = 0x0410;
    pub const INTERSECT_CLIP_RECT: u16 = 0x0416;
    pub const ELLIPSE: u16 = 0x0418;
    pub const RECTANGLE: u16 = 0x041B;
    pub const POLY_POLYGON: u16 = 0x0538;
    pub const ROUND_RECT: u16 = 0x061C;
    pub const CREATE_REGION: u16 = 0x06FF;
    pub const CREATE_PALETTE: u16 = 0x00F7;
    pub const ARC: u16 = 0x0817;
    pub const PIE: u16 = 0x081A;
    pub const CHORD: u16 = 0x0830;
    pub const DIB_BIT_BLT: u16 = 0x0940;
    pub const DIB_STRETCH_BLT: u16 = 0x0B41;
    pub const STRETCH_DIB: u16 = 0x0F43;
}

/// Um registro: função e parâmetros crus (sem o tamanho e a função).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub func: u16,
    pub params: Vec<u8>,
}

impl Record {
    fn i16(&self, k: usize) -> Option<i16> {
        let b = self.params.get(2 * k..2 * k + 2)?;
        Some(i16::from_le_bytes([b[0], b[1]]))
    }

    fn u16(&self, k: usize) -> Option<u16> {
        self.i16(k).map(|v| v as u16)
    }

    fn color(&self, k: usize) -> Option<[u8; 3]> {
        let b = self.params.get(2 * k..2 * k + 3)?;
        Some([b[0], b[1], b[2]])
    }

    fn points(&self, at: usize, n: usize) -> Option<Vec<(f64, f64)>> {
        (0..n)
            .map(|i| {
                Some((
                    self.i16(at + 2 * i)? as f64,
                    self.i16(at + 2 * i + 1)? as f64,
                ))
            })
            .collect()
    }
}

/// Um metarquivo lido: retângulo lógico (do cabeçalho `META`, ou da janela
/// quando não há cabeçalho) e registros até o `EOF`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wmf {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub records: Vec<Record>,
}

/// Retângulo em pixels (ou pontos), com origem no canto superior esquerdo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    /// Pixels inteiros cobertos (centro dentro), `[x0, x1) × [y0, y1)`.
    fn pixels(&self) -> IRect {
        let (l, r) = order(self.x, self.x + self.w);
        let (t, b) = order(self.y, self.y + self.h);
        IRect {
            x0: (l - 0.5).ceil() as i32,
            y0: (t - 0.5).ceil() as i32,
            x1: (r - 0.5).ceil() as i32,
            y1: (b - 0.5).ceil() as i32,
        }
    }
}

fn order(a: f64, b: f64) -> (f64, f64) {
    if a <= b { (a, b) } else { (b, a) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IRect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl IRect {
    fn intersect(self, o: IRect) -> IRect {
        IRect {
            x0: self.x0.max(o.x0),
            y0: self.y0.max(o.y0),
            x1: self.x1.min(o.x1),
            y1: self.y1.min(o.y1),
        }
    }

    fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }
}

impl Wmf {
    /// Parte `META` dos `CHNK`: `"META"`, `i16 x, y, u16 largura, altura`
    /// e o WMF (cabeçalho padrão de 18 bytes).
    pub fn from_meta(data: &[u8]) -> Result<Wmf> {
        let mut r = Reader::new(data, Format::WinRes);
        if r.bytes(4)? != b"META" {
            return Err(r.error(ErrorKind::BadMagic { expected: "META" }));
        }
        let x = r.u16_le()? as i16;
        let y = r.u16_le()? as i16;
        let width = r.u16_le()?;
        let height = r.u16_le()?;
        let records = read_records(&data[12..], 12)?;
        Ok(Wmf {
            x,
            y,
            width,
            height,
            records,
        })
    }

    /// WMF cru (cabeçalho padrão); o retângulo vem da primeira janela.
    pub fn from_wmf(data: &[u8]) -> Result<Wmf> {
        let records = read_records(data, 0)?;
        let mut w = Wmf {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            records,
        };
        let (org, ext) = w.window();
        w.x = org.0 as i16;
        w.y = org.1 as i16;
        w.width = ext.0.abs() as u16;
        w.height = ext.1.abs() as u16;
        Ok(w)
    }

    /// Janela lógica inicial: o primeiro `SetWindowOrg`/`SetWindowExt`, ou o
    /// retângulo do cabeçalho.
    pub fn window(&self) -> ((f64, f64), (f64, f64)) {
        let mut org = None;
        let mut ext = None;
        for r in &self.records {
            match r.func {
                func::SET_WINDOW_ORG if org.is_none() => {
                    org = Some((r.i16(1).unwrap_or(0) as f64, r.i16(0).unwrap_or(0) as f64));
                }
                func::SET_WINDOW_EXT if ext.is_none() => {
                    ext = Some((r.i16(1).unwrap_or(1) as f64, r.i16(0).unwrap_or(1) as f64));
                }
                _ => {}
            }
        }
        (
            org.unwrap_or((self.x as f64, self.y as f64)),
            ext.unwrap_or((self.width.max(1) as f64, self.height.max(1) as f64)),
        )
    }

    /// Desenha com a janela lógica levada a `dest` (pixels de `img`),
    /// recortando em `dest`.
    pub fn draw(&self, img: &mut Image, dest: Rect) {
        self.draw_clipped(img, dest, dest);
    }

    /// Como [`Wmf::draw`], recortando em `clip` em vez de `dest`.
    pub fn draw_clipped(&self, img: &mut Image, dest: Rect, clip: Rect) {
        let full = IRect {
            x0: 0,
            y0: 0,
            x1: img.width as i32,
            y1: img.height as i32,
        };
        let clip = clip.pixels().intersect(full);
        if clip.is_empty() || dest.w == 0.0 || dest.h == 0.0 {
            return;
        }
        Player::new(self, img, dest, clip).play();
    }

    /// Imagem `w × h` transparente com o metarquivo inteiro.
    pub fn render(&self, w: u32, h: u32) -> Image {
        let mut img = Image {
            width: w,
            height: h,
            rgba: vec![0; (w * h * 4) as usize],
        };
        self.draw(&mut img, Rect::new(0.0, 0.0, w as f64, h as f64));
        img
    }
}

/// Lê cabeçalho e registros; `base` só serve para a posição dos erros.
fn read_records(data: &[u8], base: usize) -> Result<Vec<Record>> {
    let mut r = Reader::new(data, Format::WinRes);
    let kind = r.u16_le()?;
    let header_words = r.u16_le()? as usize;
    if !matches!(kind, 1 | 2) || header_words < 9 {
        return Err(r.error(ErrorKind::Invalid(format!(
            "cabeçalho WMF (tipo {kind}, {header_words} palavras) em 0x{base:x}"
        ))));
    }
    r.seek(header_words * 2)?;
    let mut out = Vec::new();
    while r.remaining() >= 6 {
        let start = r.pos();
        let words = r.u32_le()? as usize;
        let f = r.u16_le()?;
        if f == 0 {
            break;
        }
        if words < 3 {
            return Err(r.error(ErrorKind::Invalid("registro WMF curto".into())));
        }
        let len = words
            .checked_mul(2)
            .filter(|&l| start + l <= data.len())
            .ok_or_else(|| r.error(ErrorKind::Invalid("registro WMF além do fim".into())))?;
        out.push(Record {
            func: f,
            params: data[start + 6..start + len].to_vec(),
        });
        r.seek(start + len)?;
    }
    Ok(out)
}

// ---------------------------------------------------------------- estado

const PS_NULL: u16 = 5;
const PS_INSIDEFRAME: u16 = 6;

#[derive(Debug, Clone)]
struct Pen {
    style: u16,
    /// Largura lógica (0 = 1 pixel).
    width: f64,
    color: [u8; 3],
}

#[derive(Debug, Clone)]
struct Pattern {
    w: usize,
    h: usize,
    rgb: Vec<[u8; 3]>,
}

#[derive(Debug, Clone)]
enum Brush {
    Null,
    Solid([u8; 3]),
    /// Hachura 0..5 (`HS_*`) na cor dada; o fundo segue o `SetBkMode`.
    Hatch(u16, [u8; 3]),
    Pattern(std::rc::Rc<Pattern>),
}

#[derive(Debug, Clone)]
enum Obj {
    Pen(Pen),
    Brush(Brush),
    Other,
}

#[derive(Debug, Clone)]
struct Dc {
    pen: Pen,
    brush: Brush,
    clip: IRect,
    org: (f64, f64),
    ext: (f64, f64),
    rop2: u16,
    opaque: bool,
    bk: [u8; 3],
    winding: bool,
    pos: (f64, f64),
}

/// Cor que um pixel recebe.
#[derive(Clone, Copy)]
enum Paint<'a> {
    Color([u8; 3]),
    Brush(&'a Brush, [u8; 3], bool),
}

struct Player<'a> {
    wmf: &'a Wmf,
    img: &'a mut Image,
    dest: Rect,
    dc: Dc,
    saved: Vec<Dc>,
    objects: Vec<Option<Obj>>,
}

impl<'a> Player<'a> {
    fn new(wmf: &'a Wmf, img: &'a mut Image, dest: Rect, clip: IRect) -> Player<'a> {
        let (org, ext) = (
            (wmf.x as f64, wmf.y as f64),
            (wmf.width.max(1) as f64, wmf.height.max(1) as f64),
        );
        Player {
            wmf,
            img,
            dest,
            dc: Dc {
                pen: Pen {
                    style: 0,
                    width: 0.0,
                    color: [0; 3],
                },
                brush: Brush::Solid([255; 3]),
                clip,
                org,
                ext,
                rop2: 13,
                opaque: true,
                bk: [255; 3],
                winding: false,
                pos: (0.0, 0.0),
            },
            saved: Vec::new(),
            objects: Vec::new(),
        }
    }

    // ------------------------------------------------------- transformação

    fn sx(&self) -> f64 {
        self.dest.w
            / if self.dc.ext.0 == 0.0 {
                1.0
            } else {
                self.dc.ext.0
            }
    }

    fn sy(&self) -> f64 {
        self.dest.h
            / if self.dc.ext.1 == 0.0 {
                1.0
            } else {
                self.dc.ext.1
            }
    }

    fn dev(&self, p: (f64, f64)) -> (f64, f64) {
        (
            self.dest.x + (p.0 - self.dc.org.0) * self.sx(),
            self.dest.y + (p.1 - self.dc.org.1) * self.sy(),
        )
    }

    /// Retângulo lógico `(l, t, r, b)` em pixels, normalizado.
    fn dev_box(&self, l: f64, t: f64, r: f64, b: f64) -> Box4 {
        let (x0, y0) = self.dev((l, t));
        let (x1, y1) = self.dev((r, b));
        let (x0, x1) = order(x0, x1);
        let (y0, y1) = order(y0, y1);
        (x0, y0, x1, y1)
    }

    /// Largura da caneta em pixels (no mínimo 1), ou 0 sem caneta.
    fn pen_px(&self) -> f64 {
        if self.dc.pen.style == PS_NULL {
            return 0.0;
        }
        (self.dc.pen.width * self.sx().abs()).max(1.0)
    }

    // -------------------------------------------------------------- pixels

    fn put(&mut self, x: i32, y: i32, paint: Paint<'_>) {
        let c = self.dc.clip;
        if x < c.x0 || y < c.y0 || x >= c.x1 || y >= c.y1 {
            return;
        }
        let src = match paint {
            Paint::Color(c) => c,
            Paint::Brush(b, bk, opaque) => match b {
                Brush::Null => return,
                Brush::Solid(c) => *c,
                Brush::Hatch(style, c) => {
                    if hatch_on(*style, x, y) {
                        *c
                    } else if opaque {
                        bk
                    } else {
                        return;
                    }
                }
                Brush::Pattern(p) => {
                    p.rgb[(y.rem_euclid(p.h as i32) as usize) * p.w
                        + x.rem_euclid(p.w as i32) as usize]
                }
            },
        };
        let i = ((y as u32 * self.img.width + x as u32) * 4) as usize;
        let d = &mut self.img.rgba[i..i + 4];
        let out = match self.dc.rop2 {
            1 => [0; 3],
            16 => [255; 3],
            11 => return,
            6 => [!d[0], !d[1], !d[2]],
            7 => [d[0] ^ src[0], d[1] ^ src[1], d[2] ^ src[2]],
            4 => [!src[0], !src[1], !src[2]],
            9 => [d[0] & src[0], d[1] & src[1], d[2] & src[2]],
            15 => [d[0] | src[0], d[1] | src[1], d[2] | src[2]],
            _ => src,
        };
        d.copy_from_slice(&[out[0], out[1], out[2], 255]);
    }

    /// Pixels de `bbox` (pixels, `[x0,x1) × [y0,y1)`) cujo centro satisfaz `inside`.
    fn fill_where(&mut self, bbox: Box4, paint: Paint<'_>, inside: impl Fn(f64, f64) -> bool) {
        let r = IRect {
            x0: (bbox.0 - 1.0).floor() as i32,
            y0: (bbox.1 - 1.0).floor() as i32,
            x1: (bbox.2 + 1.0).ceil() as i32,
            y1: (bbox.3 + 1.0).ceil() as i32,
        }
        .intersect(self.dc.clip);
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                if inside(x as f64 + 0.5, y as f64 + 0.5) {
                    self.put(x, y, paint);
                }
            }
        }
    }

    /// Polígonos (pixels) preenchidos por varredura, regra par-ímpar ou
    /// de voltas.
    fn fill_polys(&mut self, polys: &[Vec<(f64, f64)>], winding: bool, paint: Paint<'_>) {
        let pts = polys.iter().flatten();
        let (mut y0, mut y1) = (f64::MAX, f64::MIN);
        for p in pts {
            y0 = y0.min(p.1);
            y1 = y1.max(p.1);
        }
        if y0 > y1 {
            return;
        }
        let ys = ((y0 - 0.5).ceil() as i32).max(self.dc.clip.y0);
        let ye = ((y1 - 0.5).ceil() as i32).min(self.dc.clip.y1);
        let mut xs: Vec<(f64, i32)> = Vec::new();
        for y in ys..ye {
            let yc = y as f64 + 0.5;
            xs.clear();
            for poly in polys {
                let n = poly.len();
                for i in 0..n {
                    let (a, b) = (poly[i], poly[(i + 1) % n]);
                    if (a.1 <= yc) != (b.1 <= yc) {
                        let t = (yc - a.1) / (b.1 - a.1);
                        xs.push((a.0 + t * (b.0 - a.0), if b.1 > a.1 { 1 } else { -1 }));
                    }
                }
            }
            xs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut wind = 0;
            for k in 0..xs.len() {
                let inside = if winding {
                    wind += xs[k].1;
                    wind != 0
                } else {
                    k % 2 == 0
                };
                if inside && k + 1 < xs.len() {
                    let xa = ((xs[k].0 - 0.5).ceil() as i32).max(self.dc.clip.x0);
                    let xb = ((xs[k + 1].0 - 0.5).ceil() as i32).min(self.dc.clip.x1);
                    for x in xa..xb {
                        self.put(x, y, paint);
                    }
                }
            }
        }
    }

    /// Segmento com a caneta atual (pixels).
    fn stroke_segment(&mut self, a: (f64, f64), b: (f64, f64), w: f64) {
        let paint = Paint::Color(self.dc.pen.color);
        if w <= 1.0 {
            self.thin_line(a, b, paint);
            return;
        }
        let h = w / 2.0;
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.0 {
            let (nx, ny) = (-dy / len * h, dx / len * h);
            let quad = vec![
                (a.0 + nx, a.1 + ny),
                (b.0 + nx, b.1 + ny),
                (b.0 - nx, b.1 - ny),
                (a.0 - nx, a.1 - ny),
            ];
            self.fill_polys(&[quad], false, paint);
        }
        for c in [a, b] {
            self.fill_where((c.0 - h, c.1 - h, c.0 + h, c.1 + h), paint, |x, y| {
                (x - c.0).powi(2) + (y - c.1).powi(2) <= h * h
            });
        }
    }

    /// Linha de 1 pixel (Bresenham entre os pixels das pontas).
    fn thin_line(&mut self, a: (f64, f64), b: (f64, f64), paint: Paint<'_>) {
        let (mut x0, mut y0) = (a.0.floor() as i32, a.1.floor() as i32);
        let (x1, y1) = (b.0.floor() as i32, b.1.floor() as i32);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let mut err = dx + dy;
        let mut guard = 0;
        loop {
            self.put(x0, y0, paint);
            if (x0 == x1 && y0 == y1) || guard > 1 << 16 {
                break;
            }
            guard += 1;
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn stroke_path(&mut self, pts: &[(f64, f64)], closed: bool) {
        let w = self.pen_px();
        if w == 0.0 || pts.is_empty() {
            return;
        }
        if pts.len() == 1 {
            self.stroke_segment(pts[0], pts[0], w);
        }
        for s in pts.windows(2) {
            self.stroke_segment(s[0], s[1], w);
        }
        if closed && pts.len() > 2 {
            self.stroke_segment(pts[pts.len() - 1], pts[0], w);
        }
    }

    /// Molduras de figuras fechadas: `(fora, dentro)` em pixels. Caneta
    /// `PS_INSIDEFRAME` (e a de 1 pixel) fica dentro da caixa; as outras,
    /// centradas na linha.
    fn frame_boxes(&self, b: Box4) -> (Box4, Box4, f64) {
        let w = self.pen_px();
        let (grow, shrink) = if w <= 1.0 || self.dc.pen.style == PS_INSIDEFRAME {
            (0.0, w)
        } else {
            (w / 2.0 - 0.5, w / 2.0 + 0.5)
        };
        (
            (b.0 - grow, b.1 - grow, b.2 + grow, b.3 + grow),
            (b.0 + shrink, b.1 + shrink, b.2 - shrink, b.3 - shrink),
            w,
        )
    }

    fn fill_brush_where(&mut self, bbox: Box4, inside: impl Fn(f64, f64) -> bool) {
        let brush = self.dc.brush.clone();
        if matches!(brush, Brush::Null) {
            return;
        }
        let (bk, opaque) = (self.dc.bk, self.dc.opaque);
        self.fill_where(bbox, Paint::Brush(&brush, bk, opaque), inside);
    }

    fn fill_brush_polys(&mut self, polys: &[Vec<(f64, f64)>]) {
        let brush = self.dc.brush.clone();
        if matches!(brush, Brush::Null) {
            return;
        }
        let (bk, opaque, winding) = (self.dc.bk, self.dc.opaque, self.dc.winding);
        self.fill_polys(polys, winding, Paint::Brush(&brush, bk, opaque));
    }

    // ------------------------------------------------------------- figuras

    fn rectangle(&mut self, b: Box4, round: Option<(f64, f64)>) {
        let (outer, inner, w) = self.frame_boxes(b);
        let shape = move |r: Box4, x: f64, y: f64| -> bool {
            if x < r.0 || x >= r.2 || y < r.1 || y >= r.3 {
                return false;
            }
            let Some((rx, ry)) = round else { return true };
            let rx = rx.min((r.2 - r.0) / 2.0).max(0.0);
            let ry = ry.min((r.3 - r.1) / 2.0).max(0.0);
            if rx <= 0.0 || ry <= 0.0 {
                return true;
            }
            let cx = x.max(r.0 + rx).min(r.2 - rx);
            let cy = y.max(r.1 + ry).min(r.3 - ry);
            ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
        };
        let rnd_in = round.map(|(rx, ry)| ((rx - w).max(0.0), (ry - w).max(0.0)));
        let shape_in = move |r: Box4, x: f64, y: f64| -> bool {
            if x < r.0 || x >= r.2 || y < r.1 || y >= r.3 {
                return false;
            }
            let Some((rx, ry)) = rnd_in else { return true };
            let rx = rx.min((r.2 - r.0) / 2.0);
            let ry = ry.min((r.3 - r.1) / 2.0);
            if rx <= 0.0 || ry <= 0.0 {
                return true;
            }
            let cx = x.max(r.0 + rx).min(r.2 - rx);
            let cy = y.max(r.1 + ry).min(r.3 - ry);
            ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
        };
        self.fill_brush_where(outer, |x, y| shape_in(inner, x, y));
        if w > 0.0 {
            let pen = Paint::Color(self.dc.pen.color);
            self.fill_where(outer, pen, |x, y| {
                shape(outer, x, y) && !shape_in(inner, x, y)
            });
        }
    }

    /// Elipse, arco, fatia (`Pie`) ou corda (`Chord`) na caixa `b` (pixels).
    fn ellipse(&mut self, b: Box4, arc: Option<ArcSpec>) {
        let in_ell = |r: Box4, x: f64, y: f64| -> bool {
            let (rx, ry) = ((r.2 - r.0) / 2.0, (r.3 - r.1) / 2.0);
            if rx <= 0.0 || ry <= 0.0 {
                return false;
            }
            let (cx, cy) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0);
            ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
        };
        let Some((kind, s, e)) = arc else {
            let (outer, inner, w) = self.frame_boxes(b);
            self.fill_brush_where(outer, |x, y| in_ell(inner, x, y));
            if w > 0.0 {
                let pen = Paint::Color(self.dc.pen.color);
                self.fill_where(outer, pen, |x, y| {
                    in_ell(outer, x, y) && !in_ell(inner, x, y)
                });
            }
            return;
        };
        let (cx, cy) = ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0);
        // ângulos no sentido anti-horário da tela (y para cima)
        let flip = self.sx() * self.sy() < 0.0;
        let ang = |x: f64, y: f64| -> f64 {
            let a = (-(y - cy)).atan2(x - cx);
            if flip { -a } else { a }
        };
        let (a0, a1) = (ang(s.0, s.1), ang(e.0, e.1));
        let tau = std::f64::consts::TAU;
        let mut span = (a1 - a0).rem_euclid(tau);
        if span == 0.0 {
            span = tau;
        }
        let in_span = move |x: f64, y: f64| (ang(x, y) - a0).rem_euclid(tau) <= span;
        // pontos do contorno nas direções de início e fim
        let (rx, ry) = ((b.2 - b.0) / 2.0, (b.3 - b.1) / 2.0);
        let on_ellipse = |p: (f64, f64)| -> (f64, f64) {
            let (dx, dy) = (p.0 - cx, p.1 - cy);
            let k = 1.0 / ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt().max(1e-9);
            (cx + dx * k, cy + dy * k)
        };
        let (ps, pe) = (on_ellipse(s), on_ellipse(e));
        let w = self.pen_px();
        match kind {
            ArcKind::Arc => {
                if w > 0.0 {
                    let (h0, h1) = (w / 2.0 - 0.5, w / 2.0 + 0.5);
                    let outer = (b.0 - h0, b.1 - h0, b.2 + h0, b.3 + h0);
                    let inner = (b.0 + h1, b.1 + h1, b.2 - h1, b.3 - h1);
                    let pen = Paint::Color(self.dc.pen.color);
                    self.fill_where(outer, pen, |x, y| {
                        in_ell(outer, x, y) && !in_ell(inner, x, y) && in_span(x, y)
                    });
                }
            }
            ArcKind::Pie | ArcKind::Chord => {
                let (outer, inner, w) = self.frame_boxes(b);
                // lado da corda onde fica o arco
                let mid = a0 + span / 2.0;
                let m = if flip { -mid } else { mid };
                let pm = (cx + rx * m.cos(), cy - ry * m.sin());
                let side = |x: f64, y: f64| (pe.0 - ps.0) * (y - ps.1) - (pe.1 - ps.1) * (x - ps.0);
                let sm = side(pm.0, pm.1);
                let region = |x: f64, y: f64| -> bool {
                    match kind {
                        ArcKind::Pie => in_span(x, y),
                        _ => side(x, y) * sm >= 0.0,
                    }
                };
                self.fill_brush_where(outer, |x, y| in_ell(inner, x, y) && region(x, y));
                if w > 0.0 {
                    let pen = Paint::Color(self.dc.pen.color);
                    self.fill_where(outer, pen, |x, y| {
                        in_ell(outer, x, y) && !in_ell(inner, x, y) && region(x, y)
                    });
                    if kind == ArcKind::Pie {
                        self.stroke_segment((cx, cy), ps, w);
                        self.stroke_segment((cx, cy), pe, w);
                    } else {
                        self.stroke_segment(ps, pe, w);
                    }
                }
            }
        }
    }

    fn stretch_dib(&mut self, rop: u32, src: (i16, i16, i16, i16), dst: Box4, dib: &[u8]) {
        let Ok(img) = decode_bitmap(dib) else { return };
        let (sx, sy, sw, sh) = (src.0 as f64, src.1 as f64, src.2 as f64, src.3 as f64);
        if sw == 0.0 || sh == 0.0 {
            return;
        }
        // DIB de baixo para cima: y da origem conta a partir da base
        let top = img.height as f64 - sy - sh.abs();
        let (x0, y0) = self.dev((dst.0, dst.1));
        let (x1, y1) = self.dev((dst.0 + dst.2, dst.1 + dst.3));
        let (l, r) = order(x0, x1);
        let (t, b) = order(y0, y1);
        let flip_x = (x1 < x0) != (sw < 0.0);
        let flip_y = (y1 < y0) != (sh < 0.0);
        let area = Rect::new(l, t, r - l, b - t)
            .pixels()
            .intersect(self.dc.clip);
        for y in area.y0..area.y1 {
            let mut v = (y as f64 + 0.5 - t) / (b - t);
            if flip_y {
                v = 1.0 - v;
            }
            let py = (top + v * sh.abs()).floor() as i64;
            if py < 0 || py >= img.height as i64 {
                continue;
            }
            for x in area.x0..area.x1 {
                let mut u = (x as f64 + 0.5 - l) / (r - l);
                if flip_x {
                    u = 1.0 - u;
                }
                let px = (sx + u * sw.abs()).floor() as i64;
                if px < 0 || px >= img.width as i64 {
                    continue;
                }
                let si = ((py as u32 * img.width + px as u32) * 4) as usize;
                let s = [img.rgba[si], img.rgba[si + 1], img.rgba[si + 2]];
                let di = ((y as u32 * self.img.width + x as u32) * 4) as usize;
                let d = &mut self.img.rgba[di..di + 4];
                let out = match rop {
                    0x0088_00C6 => [d[0] & s[0], d[1] & s[1], d[2] & s[2]],
                    0x00EE_0086 => [d[0] | s[0], d[1] | s[1], d[2] | s[2]],
                    0x0066_0046 => [d[0] ^ s[0], d[1] ^ s[1], d[2] ^ s[2]],
                    0x0033_0008 => [!s[0], !s[1], !s[2]],
                    0x00BB_0226 => [d[0] | !s[0], d[1] | !s[1], d[2] | !s[2]],
                    0x0022_0326 => [d[0] & !s[0], d[1] & !s[1], d[2] & !s[2]],
                    _ => s,
                };
                d.copy_from_slice(&[out[0], out[1], out[2], 255]);
            }
        }
    }

    // ------------------------------------------------------------ objetos

    fn add_object(&mut self, o: Obj) {
        match self.objects.iter_mut().find(|s| s.is_none()) {
            Some(slot) => *slot = Some(o),
            None => self.objects.push(Some(o)),
        }
    }

    // ------------------------------------------------------------ registro

    fn play(&mut self) {
        let wmf = self.wmf;
        for rec in &wmf.records {
            self.record(rec);
        }
    }

    fn record(&mut self, r: &Record) {
        use func::*;
        let i = |k| r.i16(k).map(|v| v as f64);
        match r.func {
            SET_WINDOW_ORG => {
                if let (Some(y), Some(x)) = (i(0), i(1)) {
                    self.dc.org = (x, y);
                }
            }
            SET_WINDOW_EXT => {
                if let (Some(y), Some(x)) = (i(0), i(1)) {
                    self.dc.ext = (x, y);
                }
            }
            OFFSET_WINDOW_ORG => {
                if let (Some(y), Some(x)) = (i(0), i(1)) {
                    self.dc.org = (self.dc.org.0 + x, self.dc.org.1 + y);
                }
            }
            SCALE_WINDOW_EXT => {
                if let (Some(yd), Some(yn), Some(xd), Some(xn)) = (i(0), i(1), i(2), i(3))
                    && yd != 0.0
                    && xd != 0.0
                {
                    self.dc.ext = (self.dc.ext.0 * xn / xd, self.dc.ext.1 * yn / yd);
                }
            }
            SAVE_DC => self.saved.push(self.dc.clone()),
            RESTORE_DC => {
                let n = r.i16(0).unwrap_or(-1);
                let keep = if n < 0 {
                    self.saved.len().checked_sub((-n) as usize)
                } else {
                    (n as usize)
                        .checked_sub(1)
                        .filter(|&k| k < self.saved.len())
                };
                if let Some(k) = keep {
                    self.dc = self.saved[k].clone();
                    self.saved.truncate(k);
                }
            }
            SET_ROP2 => self.dc.rop2 = r.u16(0).unwrap_or(13),
            SET_BK_MODE => self.dc.opaque = r.u16(0) == Some(2),
            SET_BK_COLOR => {
                if let Some(c) = r.color(0) {
                    self.dc.bk = c;
                }
            }
            SET_POLY_FILL_MODE => self.dc.winding = r.u16(0) == Some(2),
            SET_MAP_MODE => {}
            INTERSECT_CLIP_RECT => {
                if let (Some(b), Some(rt), Some(t), Some(l)) = (i(0), i(1), i(2), i(3)) {
                    let (x0, y0, x1, y1) = self.dev_box(l, t, rt, b);
                    let c = IRect {
                        x0: x0.round() as i32,
                        y0: y0.round() as i32,
                        x1: x1.round() as i32,
                        y1: y1.round() as i32,
                    };
                    self.dc.clip = self.dc.clip.intersect(c);
                }
            }
            CREATE_PEN_INDIRECT => {
                if let (Some(style), Some(w), Some(color)) = (r.u16(0), i(1), r.color(3)) {
                    self.add_object(Obj::Pen(Pen {
                        style: style & 0x0f,
                        width: w.abs(),
                        color,
                    }));
                }
            }
            CREATE_BRUSH_INDIRECT => {
                if let (Some(style), Some(color)) = (r.u16(0), r.color(1)) {
                    let b = match style {
                        0 => Brush::Solid(color),
                        2 => Brush::Hatch(r.u16(3).unwrap_or(0), color),
                        _ => Brush::Null,
                    };
                    self.add_object(Obj::Brush(b));
                }
            }
            DIB_CREATE_PATTERN_BRUSH => {
                let b = r
                    .params
                    .get(4..)
                    .filter(|_| r.u16(0) == Some(5))
                    .and_then(|dib| decode_bitmap(dib).ok())
                    .filter(|img| img.width > 0 && img.height > 0)
                    .map(|img| {
                        Brush::Pattern(std::rc::Rc::new(Pattern {
                            w: img.width as usize,
                            h: img.height as usize,
                            rgb: img
                                .rgba
                                .chunks_exact(4)
                                .map(|p| [p[0], p[1], p[2]])
                                .collect(),
                        }))
                    })
                    .unwrap_or(Brush::Solid([128; 3]));
                self.add_object(Obj::Brush(b));
            }
            CREATE_PATTERN_BRUSH => self.add_object(Obj::Brush(Brush::Solid([128; 3]))),
            CREATE_FONT_INDIRECT | CREATE_PALETTE | CREATE_REGION => self.add_object(Obj::Other),
            SELECT_OBJECT => {
                if let Some(Some(o)) = r.u16(0).and_then(|k| self.objects.get(k as usize)) {
                    match o.clone() {
                        Obj::Pen(p) => self.dc.pen = p,
                        Obj::Brush(b) => self.dc.brush = b,
                        Obj::Other => {}
                    }
                }
            }
            DELETE_OBJECT => {
                if let Some(slot) = r.u16(0).and_then(|k| self.objects.get_mut(k as usize)) {
                    *slot = None;
                }
            }
            MOVE_TO => {
                if let (Some(y), Some(x)) = (i(0), i(1)) {
                    self.dc.pos = (x, y);
                }
            }
            LINE_TO => {
                if let (Some(y), Some(x)) = (i(0), i(1)) {
                    let (a, b) = (self.dev(self.dc.pos), self.dev((x, y)));
                    self.stroke_path(&[a, b], false);
                    self.dc.pos = (x, y);
                }
            }
            POLYGON | POLYLINE => {
                let Some(n) = r.u16(0) else { return };
                let Some(pts) = r.points(1, n as usize) else {
                    return;
                };
                let pts: Vec<_> = pts.into_iter().map(|p| self.dev(p)).collect();
                if r.func == POLYGON {
                    self.fill_brush_polys(std::slice::from_ref(&pts));
                    self.stroke_path(&pts, true);
                } else {
                    self.stroke_path(&pts, false);
                }
            }
            POLY_POLYGON => {
                let Some(n) = r.u16(0) else { return };
                let counts: Option<Vec<u16>> = (0..n as usize).map(|k| r.u16(1 + k)).collect();
                let Some(counts) = counts else { return };
                let mut at = 1 + n as usize;
                let mut polys = Vec::new();
                for c in counts {
                    let Some(pts) = r.points(at, c as usize) else {
                        return;
                    };
                    at += 2 * c as usize;
                    polys.push(pts.into_iter().map(|p| self.dev(p)).collect::<Vec<_>>());
                }
                self.fill_brush_polys(&polys);
                for p in &polys {
                    self.stroke_path(p, true);
                }
            }
            RECTANGLE => {
                if let (Some(b), Some(rt), Some(t), Some(l)) = (i(0), i(1), i(2), i(3)) {
                    let bx = self.dev_box(l, t, rt, b);
                    self.rectangle(bx, None);
                }
            }
            ROUND_RECT => {
                if let (Some(h), Some(w), Some(b), Some(rt), Some(t), Some(l)) =
                    (i(0), i(1), i(2), i(3), i(4), i(5))
                {
                    let bx = self.dev_box(l, t, rt, b);
                    let (rx, ry) = ((w * self.sx()).abs() / 2.0, (h * self.sy()).abs() / 2.0);
                    self.rectangle(bx, Some((rx, ry)));
                }
            }
            ELLIPSE => {
                if let (Some(b), Some(rt), Some(t), Some(l)) = (i(0), i(1), i(2), i(3)) {
                    let bx = self.dev_box(l, t, rt, b);
                    self.ellipse(bx, None);
                }
            }
            ARC | PIE | CHORD => {
                let v: Option<Vec<f64>> = (0..8).map(i).collect();
                let Some(v) = v else { return };
                // yEnd, xEnd, yStart, xStart, bottom, right, top, left
                let bx = self.dev_box(v[7], v[6], v[5], v[4]);
                let s = self.dev((v[3], v[2]));
                let e = self.dev((v[1], v[0]));
                let kind = match r.func {
                    ARC => ArcKind::Arc,
                    PIE => ArcKind::Pie,
                    _ => ArcKind::Chord,
                };
                self.ellipse(bx, Some((kind, s, e)));
            }
            STRETCH_DIB => {
                // rop, uso, srcH, srcW, ySrc, xSrc, destH, destW, yDst, xDst, DIB
                let rop = r
                    .params
                    .get(0..4)
                    .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
                let v: Option<Vec<i16>> = (3..11).map(|k| r.i16(k)).collect();
                if let (Some(rop), Some(v), Some(dib)) = (rop, v, r.params.get(22..)) {
                    let src = (v[3], v[2], v[1], v[0]);
                    let dst = (v[7] as f64, v[6] as f64, v[5] as f64, v[4] as f64);
                    self.stretch_dib(rop, src, dst, dib);
                }
            }
            DIB_STRETCH_BLT => {
                // rop, srcH, srcW, ySrc, xSrc, destH, destW, yDst, xDst, DIB
                let rop = r
                    .params
                    .get(0..4)
                    .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
                let v: Option<Vec<i16>> = (2..10).map(|k| r.i16(k)).collect();
                if let (Some(rop), Some(v), Some(dib)) = (rop, v, r.params.get(20..)) {
                    let src = (v[3], v[2], v[1], v[0]);
                    let dst = (v[7] as f64, v[6] as f64, v[5] as f64, v[4] as f64);
                    self.stretch_dib(rop, src, dst, dib);
                }
            }
            DIB_BIT_BLT => {
                // rop, ySrc, xSrc, h, w, yDst, xDst, DIB
                let rop = r
                    .params
                    .get(0..4)
                    .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
                let v: Option<Vec<i16>> = (2..8).map(|k| r.i16(k)).collect();
                if let (Some(rop), Some(v), Some(dib)) = (rop, v, r.params.get(16..)) {
                    let src = (v[1], v[0], v[3], v[2]);
                    let dst = (v[5] as f64, v[4] as f64, v[3] as f64, v[2] as f64);
                    self.stretch_dib(rop, src, dst, dib);
                }
            }
            _ => {}
        }
    }
}

/// Caixa em pixels: esquerda, topo, direita, base.
type Box4 = (f64, f64, f64, f64);
/// Tipo, ponto de início e ponto de fim de um arco (pixels).
type ArcSpec = (ArcKind, (f64, f64), (f64, f64));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArcKind {
    Arc,
    Pie,
    Chord,
}

/// Hachuras `HS_*` em células de 8 pixels.
fn hatch_on(style: u16, x: i32, y: i32) -> bool {
    let (x, y) = (x.rem_euclid(8), y.rem_euclid(8));
    match style {
        0 => y == 4,
        1 => x == 4,
        2 => x == y,
        3 => x == 7 - y,
        4 => x == 4 || y == 4,
        _ => x == y || x == 7 - y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monta um WMF com cabeçalho e os registros dados `(função, palavras)`.
    pub(crate) fn wmf(records: &[(u16, &[i16])]) -> Vec<u8> {
        let mut v = vec![1, 0, 9, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        for (f, words) in records {
            v.extend_from_slice(&(3 + words.len() as u32).to_le_bytes());
            v.extend_from_slice(&f.to_le_bytes());
            for w in *words {
                v.extend_from_slice(&w.to_le_bytes());
            }
        }
        v.extend_from_slice(&3u32.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes());
        v
    }

    fn meta(x: i16, y: i16, w: u16, h: u16, records: &[(u16, &[i16])]) -> Vec<u8> {
        let mut m = b"META".to_vec();
        for v in [x as u16, y as u16, w, h] {
            m.extend_from_slice(&v.to_le_bytes());
        }
        m.extend_from_slice(&wmf(records));
        m
    }

    fn white(w: u32, h: u32) -> Image {
        Image {
            width: w,
            height: h,
            rgba: [255, 255, 255, 255].repeat((w * h) as usize),
        }
    }

    fn px(img: &Image, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * img.width + x) * 4) as usize;
        [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]]
    }

    fn count(img: &Image, c: [u8; 3]) -> usize {
        img.rgba.chunks_exact(4).filter(|p| p[..3] == c).count()
    }

    const BLACK: [u8; 3] = [0, 0, 0];
    const RED: [u8; 3] = [255, 0, 0];
    const WINDOW: [(u16, &[i16]); 2] = [
        (func::SET_WINDOW_ORG, &[0, 0]),
        (func::SET_WINDOW_EXT, &[100, 100]),
    ];
    const BLACK_BRUSH: (u16, &[i16]) = (func::CREATE_BRUSH_INDIRECT, &[0, 0, 0, 0]);
    const NULL_PEN: (u16, &[i16]) = (func::CREATE_PEN_INDIRECT, &[5, 0, 0, 0, 0]);

    #[test]
    fn reads_meta_header_and_records() {
        let m = Wmf::from_meta(&meta(-3, 4, 50, 60, &WINDOW)).unwrap();
        assert_eq!((m.x, m.y, m.width, m.height), (-3, 4, 50, 60));
        assert_eq!(m.records.len(), 2);
        assert_eq!(m.window(), ((0.0, 0.0), (100.0, 100.0)));
        assert!(Wmf::from_meta(b"NOPE").is_err());
        // registro que passa do fim
        let mut bad = meta(0, 0, 1, 1, &WINDOW);
        bad.truncate(bad.len() - 8);
        assert!(Wmf::from_meta(&bad).is_err());
    }

    #[test]
    fn rectangle_fills_and_scales_to_dest() {
        // retângulo preto na metade esquerda, desenhado em 20×10
        let recs = [
            WINDOW[0],
            WINDOW[1],
            BLACK_BRUSH,
            (func::SELECT_OBJECT, &[0]),
            NULL_PEN,
            (func::SELECT_OBJECT, &[1]),
            (func::RECTANGLE, &[100, 50, 0, 0]),
        ];
        let m = Wmf::from_wmf(&wmf(&recs)).unwrap();
        let mut img = white(20, 10);
        m.draw(&mut img, Rect::new(0.0, 0.0, 20.0, 10.0));
        assert_eq!(count(&img, BLACK), 100);
        assert_eq!(px(&img, 9, 9), BLACK);
        assert_eq!(px(&img, 10, 0), [255; 3]);
    }

    #[test]
    fn dest_offset_and_clip() {
        let recs = [
            WINDOW[0],
            WINDOW[1],
            BLACK_BRUSH,
            (func::SELECT_OBJECT, &[0]),
            (func::RECTANGLE, &[100, 100, 0, 0]),
        ];
        let m = Wmf::from_wmf(&wmf(&recs)).unwrap();
        let mut img = white(30, 30);
        m.draw_clipped(
            &mut img,
            Rect::new(5.0, 5.0, 10.0, 10.0),
            Rect::new(0.0, 0.0, 10.0, 10.0),
        );
        assert_eq!(count(&img, BLACK), 25);
        assert_eq!(px(&img, 5, 5), BLACK);
        assert_eq!(px(&img, 4, 5), [255; 3]);
    }

    #[test]
    fn inside_frame_pen_stays_inside_the_box() {
        // caneta vermelha de 10 unidades (2 px), pincel nulo
        let head = [
            WINDOW[0],
            WINDOW[1],
            (func::CREATE_BRUSH_INDIRECT, &[1, 0, 0, 0][..]),
            (func::SELECT_OBJECT, &[0]),
            (func::CREATE_PEN_INDIRECT, &[6, 10, 0, 0xff, 0]),
            (func::SELECT_OBJECT, &[1]),
        ];
        let mut recs = head.to_vec();
        recs.push((func::RECTANGLE, &[100, 100, 0, 0]));
        let mut img = white(20, 20);
        Wmf::from_wmf(&wmf(&recs))
            .unwrap()
            .draw(&mut img, Rect::new(0.0, 0.0, 20.0, 20.0));
        // moldura de 2 px por dentro: 20² − 16² pixels
        assert_eq!(count(&img, RED), 400 - 256);
        assert_eq!(px(&img, 0, 0), RED);
        assert_eq!(px(&img, 2, 10), [255; 3]);
        let mut recs = head.to_vec();
        recs.push((func::ELLIPSE, &[100, 100, 0, 0]));
        let mut img = white(20, 20);
        Wmf::from_wmf(&wmf(&recs))
            .unwrap()
            .draw(&mut img, Rect::new(0.0, 0.0, 20.0, 20.0));
        assert_eq!(px(&img, 0, 10), RED);
        assert_eq!(px(&img, 1, 10), RED);
        assert_eq!(px(&img, 2, 10), [255; 3]);
        assert_eq!(px(&img, 0, 0), [255; 3]);
    }

    #[test]
    fn ellipse_fill_is_round() {
        let recs = [
            WINDOW[0],
            WINDOW[1],
            BLACK_BRUSH,
            (func::SELECT_OBJECT, &[0]),
            NULL_PEN,
            (func::SELECT_OBJECT, &[1]),
            (func::ELLIPSE, &[100, 100, 0, 0]),
        ];
        let m = Wmf::from_wmf(&wmf(&recs)).unwrap();
        let img = m.render(40, 40);
        let n = img.rgba.chunks_exact(4).filter(|p| p[3] == 255).count() as f64;
        let area = std::f64::consts::PI * 400.0;
        assert!((n - area).abs() < 30.0, "{n} vs {area}");
        assert_eq!(img.rgba[3], 0, "canto fica transparente");
    }

    #[test]
    fn polygon_even_odd_and_winding() {
        // dois quadrados sobrepostos no mesmo sentido: o miolo some na regra par-ímpar
        let mut pp: Vec<i16> = vec![2, 4, 4];
        pp.extend_from_slice(&[0, 0, 60, 0, 60, 60, 0, 60]);
        pp.extend_from_slice(&[20, 20, 80, 20, 80, 80, 20, 80]);
        let base = [
            WINDOW[0],
            WINDOW[1],
            BLACK_BRUSH,
            (func::SELECT_OBJECT, &[0]),
            NULL_PEN,
            (func::SELECT_OBJECT, &[1]),
        ];
        let mut recs = base.to_vec();
        recs.push((func::POLY_POLYGON, &pp));
        let img = Wmf::from_wmf(&wmf(&recs)).unwrap().render(100, 100);
        let alpha = |x: u32, y: u32| img.rgba[((y * 100 + x) * 4 + 3) as usize];
        assert_eq!(alpha(10, 10), 255);
        assert_eq!(alpha(40, 40), 0);
        assert_eq!(alpha(70, 70), 255);
        let mut recs = base.to_vec();
        recs.push((func::SET_POLY_FILL_MODE, &[2]));
        recs.push((func::POLY_POLYGON, &pp));
        let img = Wmf::from_wmf(&wmf(&recs)).unwrap().render(100, 100);
        assert_eq!(img.rgba[((40 * 100 + 40) * 4 + 3) as usize], 255);
    }

    #[test]
    fn objects_reuse_freed_slots_and_save_restore() {
        // pincel vermelho no slot 0, apagado; o preto criado depois cai no 0
        let recs = [
            WINDOW[0],
            WINDOW[1],
            NULL_PEN,
            (func::SELECT_OBJECT, &[0]),
            (func::CREATE_BRUSH_INDIRECT, &[0, 0xff, 0, 0]),
            (func::DELETE_OBJECT, &[1]),
            BLACK_BRUSH,
            (func::SELECT_OBJECT, &[1]),
            (func::SAVE_DC, &[]),
            (func::INTERSECT_CLIP_RECT, &[100, 50, 0, 0]),
            (func::RECTANGLE, &[100, 100, 0, 0]),
            (func::RESTORE_DC, &[-1]),
            (func::RECTANGLE, &[100, 100, 50, 0]),
        ];
        let img = Wmf::from_wmf(&wmf(&recs)).unwrap().render(10, 10);
        let c = |x: u32, y: u32| px(&img, x, y);
        assert_eq!(c(2, 2), BLACK);
        assert_eq!(c(8, 2), [0; 3]);
        assert_eq!(
            img.rgba[((2 * 10 + 8) * 4 + 3) as usize],
            0,
            "fora do recorte"
        );
        assert_eq!(c(8, 8), BLACK, "recorte restaurado");
    }

    #[test]
    fn thick_line_and_polyline() {
        let recs = [
            WINDOW[0],
            WINDOW[1],
            (func::CREATE_PEN_INDIRECT, &[0, 20, 0, 0, 0]),
            (func::SELECT_OBJECT, &[0]),
            (func::MOVE_TO, &[50, 10]),
            (func::LINE_TO, &[50, 90]),
        ];
        let img = Wmf::from_wmf(&wmf(&recs)).unwrap().render(100, 100);
        let a = |x: u32, y: u32| img.rgba[((y * 100 + x) * 4 + 3) as usize];
        assert_eq!(a(50, 41), 255);
        assert_eq!(a(50, 59), 255);
        assert_eq!(a(50, 62), 0);
        assert_eq!(a(5, 50), 255, "ponta redonda");
        // 1 px
        let recs = [
            WINDOW[0],
            WINDOW[1],
            (func::POLYLINE, &[3, 0, 0, 99, 0, 99, 99]),
        ];
        let img = Wmf::from_wmf(&wmf(&recs)).unwrap().render(100, 100);
        assert_eq!(
            img.rgba.chunks_exact(4).filter(|p| p[3] == 255).count(),
            199
        );
    }

    #[test]
    fn arc_and_pie_follow_counterclockwise_order() {
        // arco do leste (start) ao norte (end), anti-horário = quarto de cima à direita
        let recs = [
            WINDOW[0],
            WINDOW[1],
            BLACK_BRUSH,
            (func::SELECT_OBJECT, &[0]),
            NULL_PEN,
            (func::SELECT_OBJECT, &[1]),
            (func::PIE, &[0, 50, 50, 100, 100, 100, 0, 0]),
        ];
        let img = Wmf::from_wmf(&wmf(&recs)).unwrap().render(100, 100);
        let a = |x: u32, y: u32| img.rgba[((y * 100 + x) * 4 + 3) as usize];
        assert_eq!(a(70, 30), 255);
        assert_eq!(a(30, 30), 0);
        assert_eq!(a(70, 70), 0);
        let recs = [
            WINDOW[0],
            WINDOW[1],
            (func::ARC, &[0, 50, 50, 100, 100, 100, 0, 0]),
        ];
        let img = Wmf::from_wmf(&wmf(&recs)).unwrap().render(100, 100);
        let a = |x: u32, y: u32| img.rgba[((y * 100 + x) * 4 + 3) as usize];
        assert_eq!(a(85, 15), 255);
        assert_eq!(a(15, 15), 0);
        assert_eq!(a(70, 30), 0);
    }

    #[test]
    fn round_rect_corners_are_cut() {
        let recs = [
            WINDOW[0],
            WINDOW[1],
            BLACK_BRUSH,
            (func::SELECT_OBJECT, &[0]),
            NULL_PEN,
            (func::SELECT_OBJECT, &[1]),
            (func::ROUND_RECT, &[40, 40, 100, 100, 0, 0]),
        ];
        let img = Wmf::from_wmf(&wmf(&recs)).unwrap().render(100, 100);
        let a = |x: u32, y: u32| img.rgba[((y * 100 + x) * 4 + 3) as usize];
        assert_eq!(a(1, 1), 0);
        assert_eq!(a(50, 1), 255);
        assert_eq!(a(10, 10), 255);
    }

    #[test]
    fn stretch_dib_and_pattern_brush() {
        // DIB 2×2 de 1 bit: linha de baixo (primeira gravada) preto, branco; de cima branco, preto
        let mut dib: Vec<u8> = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&[0; 24]);
        dib.extend_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0]);
        dib.extend_from_slice(&[0b0100_0000, 0, 0, 0, 0b1000_0000, 0, 0, 0]);
        let words = |b: &[u8]| -> Vec<i16> {
            b.chunks(2)
                .map(|c| i16::from_le_bytes([c[0], *c.get(1).unwrap_or(&0)]))
                .collect()
        };
        let mut sd: Vec<u8> = Vec::new();
        sd.extend_from_slice(&0x00CC_0020u32.to_le_bytes());
        for v in [0i16, 2, 2, 0, 0, 100, 100, 0, 0] {
            sd.extend_from_slice(&v.to_le_bytes());
        }
        sd.extend_from_slice(&dib);
        let sdw = words(&sd);
        let recs = [WINDOW[0], WINDOW[1], (func::STRETCH_DIB, sdw.as_slice())];
        let img = Wmf::from_wmf(&wmf(&recs)).unwrap().render(10, 10);
        assert_eq!(px(&img, 0, 0), [255; 3]);
        assert_eq!(px(&img, 9, 0), BLACK);
        assert_eq!(px(&img, 0, 9), BLACK);
        assert_eq!(px(&img, 9, 9), [255; 3]);
        // o mesmo DIB como pincel: xadrez de 1 px alinhado à origem do quadro
        let mut pb = vec![5u8, 0, 0, 0];
        pb.extend_from_slice(&dib);
        let pbw = words(&pb);
        let recs = [
            WINDOW[0],
            WINDOW[1],
            (func::DIB_CREATE_PATTERN_BRUSH, pbw.as_slice()),
            (func::SELECT_OBJECT, &[0]),
            NULL_PEN,
            (func::SELECT_OBJECT, &[1]),
            (func::RECTANGLE, &[100, 100, 0, 0]),
        ];
        let img = Wmf::from_wmf(&wmf(&recs)).unwrap().render(4, 4);
        assert_eq!(px(&img, 0, 0), [255; 3]);
        assert_eq!(px(&img, 1, 0), BLACK);
        assert_eq!(px(&img, 2, 0), [255; 3]);
        assert_eq!(px(&img, 0, 1), BLACK);
    }

    #[test]
    fn garbage_never_panics() {
        let mut seed = 7u32;
        for _ in 0..300 {
            let mut words = Vec::new();
            for _ in 0..12 {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                words.push((seed >> 8) as i16 % 300);
            }
            for f in [
                func::POLYGON,
                func::POLY_POLYGON,
                func::ARC,
                func::PIE,
                func::CHORD,
                func::ROUND_RECT,
                func::STRETCH_DIB,
                func::DIB_CREATE_PATTERN_BRUSH,
                func::RESTORE_DC,
                func::SCALE_WINDOW_EXT,
            ] {
                let recs = [WINDOW[0], WINDOW[1], (f, words.as_slice())];
                let _ = Wmf::from_wmf(&wmf(&recs)).unwrap().render(16, 16);
            }
        }
    }
}
