//! Bordas e fundos de página (`SHARED/STUFF/*.BOR`; ver `docs/formats/bor.md`).
//!
//! Os `.BOR` são `CHNK` de criador `ICCC` com quatro tipos de chunk:
//!
//! - `BDR n` (nome = o da borda): parte `WBDR` (espessura, mapa das 8
//!   posições e as peças em WMF vetorial) e a miniatura `PKBM` 80×72;
//! - `IBGF n` (nome = o do fundo): parte `IBGF` de 22 bytes (figura `PICT`,
//!   orientação, deslocamento e molduras `FRAM`) e a miniatura `PKBM`;
//! - `PICT n`: parte `META` (WMF com um `StretchDIB`);
//! - `FRAM n`: lista de molduras, no mesmo formato do chunk `FRAM` dos
//!   documentos `.MAX` ([`max::Frame`]).
//!
//! Os documentos que usam uma borda ou um fundo guardam cópias desses
//! chunks (`BDR n`, `IBGF n`, `PICT n`, `FRAM n`): [`Border::from_parts`],
//! [`Ibgf::parse`] e [`read_frames`] leem as partes soltas.

use crate::chnk::{self, Chunk};
use crate::max::{self, Frame};
use crate::winres::Image;
use crate::wmf_draw::{Rect, Wmf};
use crate::{ErrorKind, Format, FormatError, Reader, Result, Tag, pkbm};

/// Criador dos `.BOR`.
pub const CREATOR: Tag = Tag(*b"ICCC");

/// Posições do mapa das peças, na ordem do `WBDR`.
pub const SLOTS: [Slot; 8] = [
    Slot::TopLeft,
    Slot::Top,
    Slot::TopRight,
    Slot::Right,
    Slot::BottomRight,
    Slot::Bottom,
    Slot::BottomLeft,
    Slot::Left,
];

/// Posição de uma peça na borda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

impl Slot {
    /// Índice no mapa (`0..8`).
    pub fn index(self) -> usize {
        SLOTS.iter().position(|&s| s == self).unwrap_or(0)
    }

    pub fn is_corner(self) -> bool {
        self.index() % 2 == 0
    }
}

/// Uma borda (`BDR n`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Border {
    pub id: u16,
    pub name: String,
    /// Espessura nominal em pontos (16..36); a faixa desenhada mede ≈ 1,5×.
    pub thickness_pt: u16,
    /// Lado lógico das peças de canto (unidades do WMF).
    pub corner_size: u16,
    /// Largura e altura lógicas das peças de lado.
    pub piece_size: (u16, u16),
    /// Peça de cada posição (ordem de [`SLOTS`]).
    pub map: [u8; 8],
    pub pieces: Vec<Wmf>,
    /// Miniatura 80×72 (a borda em 3×3 células).
    pub thumb: Option<pkbm::Bitmap>,
}

impl Border {
    /// Lê as partes `WBDR` e (opcional) `PKBM` de um chunk `BDR`.
    pub fn from_parts(id: u16, name: &str, wbdr: &[u8], thumb: Option<&[u8]>) -> Result<Border> {
        let mut r = Reader::new(wbdr, Format::Chnk);
        let version = r.u16_le()?;
        if version != 1 {
            return Err(r.error(ErrorKind::Unsupported(format!("WBDR versão {version}"))));
        }
        let thickness_pt = r.u16_le()?;
        let corner_size = r.u16_le()?;
        let piece_size = (r.u16_le()?, r.u16_le()?);
        let map: [u8; 8] = r.bytes(8)?.try_into().expect("8 bytes");
        let n = r.u16_le()? as usize;
        let sizes = (0..n).map(|_| r.u32_le()).collect::<Result<Vec<u32>>>()?;
        let mut pieces = Vec::with_capacity(n);
        for size in sizes {
            let at = r.pos();
            let data = r.bytes(size as usize)?;
            pieces.push(Wmf::from_meta(data).map_err(|e| FormatError {
                format: Format::Chnk,
                offset: at + e.offset,
                kind: e.kind,
            })?);
        }
        if let Some(&k) = map.iter().find(|&&k| k as usize >= n) {
            return Err(r.error(ErrorKind::Invalid(format!("WBDR: peça {k} de {n}"))));
        }
        let thumb = thumb.map(pkbm::decode).transpose()?;
        Ok(Border {
            id,
            name: name.to_string(),
            thickness_pt,
            corner_size,
            piece_size,
            map,
            pieces,
            thumb,
        })
    }

    /// Lê um chunk `BDR` (de um `.BOR` ou de um documento).
    pub fn from_chunk(c: &Chunk<'_>) -> Result<Border> {
        let wbdr = part(c, b"WBDR").ok_or_else(|| missing("WBDR"))?;
        Border::from_parts(
            c.id,
            c.name.as_deref().unwrap_or(""),
            wbdr,
            part(c, b"PKBM"),
        )
    }

    /// A peça da posição dada.
    pub fn piece(&self, slot: Slot) -> &Wmf {
        &self.pieces[self.map[slot.index()] as usize]
    }

    /// Espessura da faixa desenhada, em pontos: 1,5 × a nominal.
    pub fn band_pt(&self) -> f64 {
        self.thickness_pt as f64 * 1.5
    }

    /// Valores que o original grava no `DOP` com esta borda: `0x47` =
    /// espessura + 1 e `0x48` = faixa (1,5 × espessura) + 1.
    pub fn dop_widths(&self) -> (u16, u16) {
        (
            self.thickness_pt + 1,
            (self.thickness_pt * 3 / 2).saturating_add(1),
        )
    }
}

impl Border {
    /// Desenha as peças nos retângulos dados (pixels), cada uma recortada no
    /// seu retângulo.
    pub fn draw_tiles(&self, img: &mut Image, tiles: &[(Slot, Rect)]) {
        for (slot, r) in tiles {
            self.piece(*slot).draw(img, *r);
        }
    }
}

/// Ladrilha uma borda dentro de `outer`: cantos quadrados `t × t` nos
/// quatro cantos, `n_h` peças em cima e embaixo e `n_v` à esquerda e à
/// direita, esticadas para preencher exatamente o espaço entre os cantos
/// (sem rotação: cada lado usa a sua peça). É o arranjo da página e das
/// miniaturas dos `.BOR`.
pub fn tile(outer: Rect, t: f64, n_h: usize, n_v: usize) -> Vec<(Slot, Rect)> {
    let (x0, y0, x1, y1) = (outer.x, outer.y, outer.x + outer.w, outer.y + outer.h);
    let mut v = vec![
        (Slot::TopLeft, Rect::new(x0, y0, t, t)),
        (Slot::TopRight, Rect::new(x1 - t, y0, t, t)),
        (Slot::BottomRight, Rect::new(x1 - t, y1 - t, t, t)),
        (Slot::BottomLeft, Rect::new(x0, y1 - t, t, t)),
    ];
    if n_h > 0 {
        let w = (outer.w - 2.0 * t) / n_h as f64;
        for i in 0..n_h {
            let x = x0 + t + i as f64 * w;
            v.push((Slot::Top, Rect::new(x, y0, w, t)));
            v.push((Slot::Bottom, Rect::new(x, y1 - t, w, t)));
        }
    }
    if n_v > 0 {
        let h = (outer.h - 2.0 * t) / n_v as f64;
        for j in 0..n_v {
            let y = y0 + t + j as f64 * h;
            v.push((Slot::Right, Rect::new(x1 - t, y, t, h)));
            v.push((Slot::Left, Rect::new(x0, y, t, h)));
        }
    }
    v
}

/// A parte `IBGF` (22 bytes) de um fundo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ibgf {
    /// `PICT n` com a arte em tamanho natural (alguns fundos não têm: a arte
    /// vem só das molduras).
    pub pict: Option<u16>,
    /// Fundo de página "Largo" (byte 6 = `ff`).
    pub landscape: bool,
    /// Deslocamento da arte na página, em pontos.
    pub dx: i16,
    pub dy: i16,
    /// `FRAM n` com os obstáculos ao texto e figuras soltas.
    pub fram: Option<u16>,
}

impl Ibgf {
    pub const LEN: usize = 22;

    pub fn parse(data: &[u8]) -> Result<Ibgf> {
        let mut r = Reader::new(data, Format::Chnk);
        let pict_tag = r.bytes(4)?;
        let pict_id = r.u16_le()?;
        let flags = r.u16_le()?;
        r.skip(4)?;
        let dx = r.u16_le()? as i16;
        let dy = r.u16_le()? as i16;
        let fram_tag = r.bytes(4)?;
        let fram_id = r.u16_le()?;
        Ok(Ibgf {
            pict: (pict_tag == b"PICT").then_some(pict_id),
            landscape: flags & 0xff != 0,
            dx,
            dy,
            fram: (fram_tag == b"FRAM").then_some(fram_id),
        })
    }

    /// Os 22 bytes, como o original grava.
    pub fn to_bytes(&self) -> [u8; Ibgf::LEN] {
        let mut b = [0u8; Ibgf::LEN];
        if let Some(id) = self.pict {
            b[0..4].copy_from_slice(b"PICT");
            b[4..6].copy_from_slice(&id.to_le_bytes());
        }
        if self.landscape {
            b[6] = 0xff;
        }
        b[12..14].copy_from_slice(&self.dx.to_le_bytes());
        b[14..16].copy_from_slice(&self.dy.to_le_bytes());
        if let Some(id) = self.fram {
            b[16..20].copy_from_slice(b"FRAM");
            b[20..22].copy_from_slice(&id.to_le_bytes());
        }
        b
    }
}

/// Um fundo de página (`IBGF n`) com as molduras já resolvidas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Background {
    pub id: u16,
    pub name: String,
    pub pict: Option<u16>,
    pub landscape: bool,
    pub dx: i16,
    pub dy: i16,
    pub fram: Option<u16>,
    /// Molduras do `FRAM` (vazio sem `FRAM`): obstáculos ao texto (tipo 6),
    /// figuras (tipo 3, `0x35 → PICT n`) e moldura de página (tipo 4).
    pub frames: Vec<Frame>,
    /// Miniatura: 54×72 (retrato) ou 80×60 (paisagem).
    pub thumb: Option<pkbm::Bitmap>,
}

impl Background {
    pub fn new(
        id: u16,
        name: &str,
        ibgf: Ibgf,
        frames: Vec<Frame>,
        thumb: Option<pkbm::Bitmap>,
    ) -> Background {
        Background {
            id,
            name: name.to_string(),
            pict: ibgf.pict,
            landscape: ibgf.landscape,
            dx: ibgf.dx,
            dy: ibgf.dy,
            fram: ibgf.fram,
            frames,
            thumb,
        }
    }

    pub fn ibgf(&self) -> Ibgf {
        Ibgf {
            pict: self.pict,
            landscape: self.landscape,
            dx: self.dx,
            dy: self.dy,
            fram: self.fram,
        }
    }

    /// Molduras que desviam o texto (tipo 6).
    pub fn obstacles(&self) -> impl Iterator<Item = &Frame> {
        self.frames
            .iter()
            .filter(|f| f.kind.code() == FRAME_OBSTACLE)
    }

    /// Molduras com figura (`0x35 → PICT n`).
    pub fn pictures(&self) -> impl Iterator<Item = (&Frame, u16)> {
        self.frames.iter().filter_map(|f| match f.object {
            Some(o) if o.tag.0 == *b"PICT" => Some((f, o.id)),
            _ => None,
        })
    }
}

impl Background {
    /// Desenha o fundo numa página que ocupa `page` (pixels) e mede
    /// `page_pt` pontos: a arte do `PICT` em tamanho natural com a origem da
    /// janela WMF em `(dx, dy)` pt, depois as figuras das molduras nos seus
    /// retângulos; tudo recortado na página. `picture` acha um `PICT n`.
    pub fn draw<'p>(
        &self,
        img: &mut Image,
        page: Rect,
        page_pt: (f64, f64),
        picture: &dyn Fn(u16) -> Option<&'p Wmf>,
    ) {
        let (sx, sy) = (page.w / page_pt.0, page.h / page_pt.1);
        let px = |x: f64, y: f64, w: f64, h: f64| {
            Rect::new(page.x + x * sx, page.y + y * sy, w * sx, h * sy)
        };
        if let Some(art) = self.pict.and_then(picture) {
            let (_, ext) = art.window();
            let dest = px(self.dx as f64, self.dy as f64, ext.0.abs(), ext.1.abs());
            art.draw_clipped(img, dest, page);
        }
        for (f, id) in self.pictures() {
            if let Some(p) = picture(id) {
                let dest = px(f.x as f64, f.y as f64, f.width as f64, f.height as f64);
                p.draw_clipped(img, dest, page);
            }
        }
    }
}

/// Tipo (`0x28`) das molduras de obstáculo dos fundos.
pub const FRAME_OBSTACLE: u8 = 6;

/// Um `.BOR` inteiro, na ordem do arquivo (= ordem dos rolos).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorFile {
    pub borders: Vec<Border>,
    pub backgrounds: Vec<Background>,
    /// Figuras `PICT n`.
    pub pictures: Vec<(u16, Wmf)>,
}

impl BorFile {
    pub fn picture(&self, id: u16) -> Option<&Wmf> {
        self.pictures.iter().find(|p| p.0 == id).map(|p| &p.1)
    }

    pub fn border(&self, id: u16) -> Option<&Border> {
        self.borders.iter().find(|b| b.id == id)
    }

    pub fn background(&self, id: u16) -> Option<&Background> {
        self.backgrounds.iter().find(|b| b.id == id)
    }
}

/// Lê um `.BOR`.
pub fn read(data: &[u8]) -> Result<BorFile> {
    let f = chnk::parse(data)?;
    if f.creator != CREATOR {
        return Err(FormatError {
            format: Format::Chnk,
            offset: 4,
            kind: ErrorKind::BadMagic { expected: "ICCC" },
        });
    }
    let chunk = |tag: &[u8; 4], id: u16| f.chunks.iter().find(|c| c.tag.0 == *tag && c.id == id);
    let mut out = BorFile {
        borders: Vec::new(),
        backgrounds: Vec::new(),
        pictures: Vec::new(),
    };
    for c in &f.chunks {
        match &c.tag.0 {
            b"BDR " => out.borders.push(Border::from_chunk(c)?),
            b"IBGF" => {
                let ibgf = Ibgf::parse(part(c, b"IBGF").ok_or_else(|| missing("IBGF"))?)?;
                let frames = match ibgf.fram.and_then(|id| chunk(b"FRAM", id)) {
                    Some(fc) => read_frames(part(fc, b"FRAM").ok_or_else(|| missing("FRAM"))?)?,
                    None => Vec::new(),
                };
                let thumb = part(c, b"PKBM").map(pkbm::decode).transpose()?;
                out.backgrounds.push(Background::new(
                    c.id,
                    c.name.as_deref().unwrap_or(""),
                    ibgf,
                    frames,
                    thumb,
                ));
            }
            b"PICT" => {
                if let Some(m) = part(c, b"META") {
                    out.pictures.push((c.id, Wmf::from_meta(m)?));
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Lê uma parte `FRAM` (`u16 n` + `n × (u8 tamanho, grpprl)`).
pub fn read_frames(data: &[u8]) -> Result<Vec<Frame>> {
    max::read_frames(data, 0)
}

fn part<'a>(c: &Chunk<'a>, tag: &[u8; 4]) -> Option<&'a [u8]> {
    c.parts.iter().find(|p| p.tag.0 == *tag).map(|p| p.data)
}

fn missing(what: &str) -> FormatError {
    FormatError {
        format: Format::Chnk,
        offset: 0,
        kind: ErrorKind::Invalid(format!("chunk sem a parte {what}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chnk::OwnedChunk;
    use crate::max::FrameKind;

    fn meta(w: u16, h: u16) -> Vec<u8> {
        let mut m = b"META".to_vec();
        for v in [0, 0, w, h] {
            m.extend_from_slice(&v.to_le_bytes());
        }
        m.extend_from_slice(&[1, 0, 9, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        m.extend_from_slice(&5u32.to_le_bytes());
        m.extend_from_slice(&0x020Cu16.to_le_bytes());
        m.extend_from_slice(&h.to_le_bytes());
        m.extend_from_slice(&w.to_le_bytes());
        m.extend_from_slice(&3u32.to_le_bytes());
        m.extend_from_slice(&0u16.to_le_bytes());
        m.extend_from_slice(&[0x02, 0x00, 0x99]); // lixo depois do EOF, como no original
        m
    }

    fn wbdr(th: u16, map: [u8; 8], n: usize) -> Vec<u8> {
        let mut v = Vec::new();
        for x in [1, th, 480, 480, 480] {
            v.extend_from_slice(&x.to_le_bytes());
        }
        v.extend_from_slice(&map);
        v.extend_from_slice(&(n as u16).to_le_bytes());
        let m = meta(480, 480);
        for _ in 0..n {
            v.extend_from_slice(&(m.len() as u32).to_le_bytes());
        }
        for _ in 0..n {
            v.extend_from_slice(&m);
        }
        v
    }

    #[test]
    fn border_parts() {
        let b =
            Border::from_parts(4, "Tribe", &wbdr(24, [0, 1, 0, 2, 0, 3, 0, 4], 5), None).unwrap();
        assert_eq!(b.thickness_pt, 24);
        assert_eq!(b.pieces.len(), 5);
        assert_eq!(b.piece(Slot::Left).width, 480);
        assert_eq!(b.band_pt(), 36.0);
        assert_eq!(b.dop_widths(), (25, 37));
        assert!(Slot::TopRight.is_corner() && !Slot::Bottom.is_corner());
        // mapa apontando para peça que não existe
        assert!(Border::from_parts(0, "", &wbdr(24, [0, 1, 0, 1, 0, 1, 0, 1], 1), None).is_err());
        assert!(Border::from_parts(0, "", &wbdr(24, [0; 8], 1)[..30], None).is_err());
    }

    #[test]
    fn tile_puts_square_corners_and_stretched_sides() {
        let t = tile(Rect::new(10.0, 20.0, 80.0, 72.0), 24.0, 1, 1);
        assert_eq!(t.len(), 8);
        let get = |s: Slot| t.iter().find(|x| x.0 == s).unwrap().1;
        assert_eq!(get(Slot::TopLeft), Rect::new(10.0, 20.0, 24.0, 24.0));
        assert_eq!(get(Slot::Top), Rect::new(34.0, 20.0, 32.0, 24.0));
        assert_eq!(get(Slot::BottomRight), Rect::new(66.0, 68.0, 24.0, 24.0));
        assert_eq!(get(Slot::Left), Rect::new(10.0, 44.0, 24.0, 24.0));
        let t = tile(Rect::new(0.0, 0.0, 100.0, 100.0), 10.0, 4, 2);
        assert_eq!(t.iter().filter(|x| x.0 == Slot::Bottom).count(), 4);
        assert_eq!(
            t.iter()
                .filter(|x| x.0 == Slot::Right)
                .map(|x| x.1.h)
                .sum::<f64>(),
            80.0
        );
    }

    #[test]
    fn ibgf_roundtrip() {
        let raw = [
            b'P', b'I', b'C', b'T', 0x22, 0, 0, 0, 0, 0, 0, 0, 10, 0, 17, 0, b'F', b'R', b'A',
            b'M', 0x10, 0,
        ];
        let i = Ibgf::parse(&raw).unwrap();
        assert_eq!(
            i,
            Ibgf {
                pict: Some(34),
                landscape: false,
                dx: 10,
                dy: 17,
                fram: Some(16)
            }
        );
        assert_eq!(i.to_bytes(), raw);
        let mut none = [0u8; 22];
        none[6] = 0xff;
        none[14..16].copy_from_slice(&(-8i16).to_le_bytes());
        let i = Ibgf::parse(&none).unwrap();
        assert!(i.pict.is_none() && i.fram.is_none() && i.landscape);
        assert_eq!(i.dy, -8);
        assert_eq!(i.to_bytes(), none);
        assert!(Ibgf::parse(&raw[..10]).is_err());
    }

    #[test]
    fn reads_a_small_bor() {
        // FRAM com um obstáculo (6) e uma figura (3 → PICT 7)
        let mut fram = vec![2, 0];
        let g1 = [
            0x28, 6, 0x29, 3, 0x2c, 73, 0, 0x2d, 10, 0, 0x2e, 0xbb, 1, 0x2f, 121, 0,
        ];
        fram.push(g1.len() as u8);
        fram.extend_from_slice(&g1);
        let g2 = [
            0x28, 3, 0x2a, 0xff, 0x2c, 5, 0, 0x35, b'P', b'I', b'C', b'T', 7, 0,
        ];
        fram.push(g2.len() as u8);
        fram.extend_from_slice(&g2);
        let mut ibgf = Ibgf {
            pict: None,
            landscape: false,
            dx: 0,
            dy: 0,
            fram: Some(3),
        }
        .to_bytes()
        .to_vec();
        ibgf[0] = 0;
        let chunks = vec![
            OwnedChunk {
                tag: Tag(*b"BDR "),
                id: 6,
                flags: 0,
                name: Some("Checkered".into()),
                parts: vec![chnk::OwnedPart {
                    tag: Tag(*b"WBDR"),
                    data: wbdr(24, [0; 8], 1),
                }],
            },
            OwnedChunk::single(b"IBGF", 2, b"IBGF", ibgf),
            OwnedChunk::single(b"FRAM", 3, b"FRAM", fram),
            OwnedChunk::single(b"PICT", 7, b"META", meta(94, 152)),
        ];
        let data = chnk::write(CREATOR, &chunks);
        let f = read(&data).unwrap();
        assert_eq!(f.borders.len(), 1);
        assert_eq!(f.borders[0].name, "Checkered");
        let bg = f.background(2).unwrap();
        assert_eq!(bg.frames.len(), 2);
        assert_eq!(bg.obstacles().count(), 1);
        let o = bg.obstacles().next().unwrap();
        assert_eq!(
            (o.x, o.y, o.width, o.height, o.wrap),
            (73, 10, 443, 121, Some(3))
        );
        assert_eq!(bg.frames[1].kind, FrameKind::Picture);
        assert_eq!(bg.pictures().map(|p| p.1).collect::<Vec<_>>(), vec![7]);
        assert_eq!(f.picture(7).unwrap().height, 152);
        // criador errado
        let other = chnk::write(Tag(*b"INK "), &chunks);
        assert!(read(&other).is_err());
    }

    #[test]
    fn frames_with_2b_and_all_pages() {
        // cópia num documento: `27 ffff` (todas as páginas) e `2b ff`
        let g = [
            0x27, 0xff, 0xff, 0x28, 6, 0x29, 2, 0x2b, 0xff, 0x2c, 0x47, 0, 0x2d, 0x7f, 0, 0x2e, 12,
            0, 0x2f, 0x2b, 2,
        ];
        let mut fram = vec![1, 0, g.len() as u8];
        fram.extend_from_slice(&g);
        let fs = read_frames(&fram).unwrap();
        assert_eq!(fs[0].page, 0xffff);
        assert_eq!(
            (fs[0].x, fs[0].y, fs[0].width, fs[0].height),
            (71, 127, 12, 555)
        );
        assert_eq!(fs[0].wrap, Some(2));
        assert!(read_frames(&[3, 0, 9]).is_err());
    }
}
