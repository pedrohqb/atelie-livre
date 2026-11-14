//! Figuras `PICT` do Macintosh (QuickDraw), versões 1 e 2 (ver
//! `docs/formats/pict.md`).
//!
//! O Creative Writer importa arquivos do Mac sem extensão pelo filtro
//! `PICTIMP.FLT` (`QD2GDI`). Aqui só os **bitmaps** são desenhados
//! (`BitsRect`, `PackBitsRect`, `DirectBitsRect` e as variantes com região),
//! que é o que as figuras de pintura e as fotos da época contêm; os demais
//! opcodes (retângulos, linhas, texto, polígonos…) são pulados e contados em
//! [`Picture::skipped`]. O resultado é uma imagem RGBA com fundo branco do
//! tamanho do quadro da figura.

use crate::winres::Image;
use crate::{ErrorKind, Format, Reader, Result};

const MAX_PIXELS: usize = 1 << 24;
const WHITE: [u8; 4] = [255, 255, 255, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];

/// Retângulo QuickDraw (`top`, `left`, `bottom`, `right`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub top: i16,
    pub left: i16,
    pub bottom: i16,
    pub right: i16,
}

impl Rect {
    pub fn width(&self) -> i32 {
        i32::from(self.right) - i32::from(self.left)
    }
    pub fn height(&self) -> i32 {
        i32::from(self.bottom) - i32::from(self.top)
    }
    fn is_valid(&self) -> bool {
        self.width() > 0 && self.height() > 0
    }
}

/// Figura lida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// 1 ou 2.
    pub version: u8,
    /// Quadro da figura (72 ppp).
    pub frame: Rect,
    /// Retângulo das coordenadas dos opcodes (o quadro, ou o retângulo de
    /// origem do cabeçalho estendido da versão 2).
    pub bounds: Rect,
    /// Quantos bitmaps foram desenhados.
    pub bitmaps: usize,
    /// Opcodes de desenho não suportados que foram pulados.
    pub skipped: Vec<u16>,
    pub image: Image,
}

/// Onde começa a figura: 512 nos arquivos (cabeçalho de aplicativo), 0 nas
/// figuras embutidas. `None` se não parece um `PICT`.
pub fn header_offset(data: &[u8]) -> Option<usize> {
    [512, 0].into_iter().find(|&o| looks_like_at(data, o))
}

/// O arquivo parece um `PICT`?
pub fn is_pict(data: &[u8]) -> bool {
    header_offset(data).is_some()
}

fn looks_like_at(data: &[u8], o: usize) -> bool {
    let Some(h) = data.get(o..o + 14) else {
        return false;
    };
    let be = |i: usize| i16::from_be_bytes([h[i], h[i + 1]]);
    let frame = Rect {
        top: be(2),
        left: be(4),
        bottom: be(6),
        right: be(8),
    };
    frame.is_valid() && (h[10..12] == [0x11, 0x01] || h[10..14] == [0x00, 0x11, 0x02, 0xff])
}

fn rect(r: &mut Reader<'_>) -> Result<Rect> {
    Ok(Rect {
        top: r.i16_be()?,
        left: r.i16_be()?,
        bottom: r.i16_be()?,
        right: r.i16_be()?,
    })
}

struct Canvas {
    bounds: Rect,
    w: usize,
    h: usize,
    rgba: Vec<u8>,
}

impl Canvas {
    /// Copia `src` (pixels RGBA, `sw`×`sh`, coordenadas de `sbounds`) do
    /// retângulo `from` para `to`, com escala pelo vizinho mais próximo.
    /// `keep` decide, por pixel, se ele é desenhado (modos de transferência).
    fn blit(
        &mut self,
        src: &[[u8; 4]],
        sbounds: Rect,
        from: Rect,
        to: Rect,
        keep: impl Fn([u8; 4]) -> bool,
    ) {
        if !from.is_valid() || !to.is_valid() {
            return;
        }
        let sw = sbounds.width();
        let sh = sbounds.height();
        let (fw, fh, tw, th) = (
            i64::from(from.width()),
            i64::from(from.height()),
            i64::from(to.width()),
            i64::from(to.height()),
        );
        for ty in to.top.max(self.bounds.top)..to.bottom.min(self.bounds.bottom) {
            let dy = i64::from(ty) - i64::from(to.top);
            let sy = i64::from(from.top) + dy * fh / th - i64::from(sbounds.top);
            if sy < 0 || sy >= i64::from(sh) {
                continue;
            }
            for tx in to.left.max(self.bounds.left)..to.right.min(self.bounds.right) {
                let dx = i64::from(tx) - i64::from(to.left);
                let sx = i64::from(from.left) + dx * fw / tw - i64::from(sbounds.left);
                if sx < 0 || sx >= i64::from(sw) {
                    continue;
                }
                let p = src[(sy * i64::from(sw) + sx) as usize];
                if !keep(p) {
                    continue;
                }
                let cx = (i32::from(tx) - i32::from(self.bounds.left)) as usize;
                let cy = (i32::from(ty) - i32::from(self.bounds.top)) as usize;
                let i = (cy * self.w + cx) * 4;
                self.rgba[i..i + 4].copy_from_slice(&p);
            }
        }
    }
}

struct State {
    v2: bool,
    fg: [u8; 4],
    bg: [u8; 4],
}

/// Lê uma figura `PICT` (arquivo com cabeçalho de 512 bytes ou figura
/// embutida).
pub fn read(data: &[u8]) -> Result<Picture> {
    let start = header_offset(data).ok_or_else(|| {
        Reader::new(data, Format::Pict).error(ErrorKind::BadMagic {
            expected: "PICT version 1 or 2",
        })
    })?;
    let pic = &data[start..];
    let mut r = Reader::new(pic, Format::Pict);
    let _size = r.u16_be()?;
    let frame = rect(&mut r)?;
    let v2 = pic[10] == 0x00;
    let mut st = State {
        v2,
        fg: BLACK,
        bg: WHITE,
    };
    let mut bounds = frame;
    if v2 {
        r.seek(14)?;
        // cabeçalho estendido: coordenadas na resolução nativa
        if r.u16_be()? == 0x0c00 {
            let version = r.i16_be()?;
            let _reserved = r.u16_be()?;
            if version == -2 {
                r.skip(8)?; // hRes, vRes
                let src = rect(&mut r)?;
                if src.is_valid() {
                    bounds = src;
                }
                r.skip(4)?;
            } else {
                r.skip(20)?;
            }
        } else {
            r.seek(14)?;
        }
    } else {
        r.seek(12)?;
    }
    let (w, h) = (bounds.width() as usize, bounds.height() as usize);
    if w.saturating_mul(h) > MAX_PIXELS {
        return Err(r.error(ErrorKind::Invalid(format!("picture too large: {w}x{h}"))));
    }
    let mut canvas = Canvas {
        bounds,
        w,
        h,
        rgba: WHITE.repeat(w * h),
    };
    let mut bitmaps = 0;
    let mut skipped = Vec::new();
    loop {
        if v2 && r.pos() % 2 == 1 {
            r.skip(1)?;
        }
        let op = if v2 { r.u16_be()? } else { u16::from(r.u8()?) };
        match op {
            0x00ff => break,
            0x0090 | 0x0091 | 0x0098 | 0x0099 => {
                bits(&mut r, op, &st, &mut canvas)?;
                bitmaps += 1;
            }
            0x009a | 0x009b => {
                direct_bits(&mut r, op, &st, &mut canvas)?;
                bitmaps += 1;
            }
            0x000e | 0x000f => {
                let c = classic_color(r.u32_be()?);
                if let Some(c) = c {
                    if op == 0x000e { st.fg = c } else { st.bg = c }
                }
            }
            0x001a | 0x001b => {
                let c = rgb48(&mut r)?;
                if op == 0x001a { st.fg = c } else { st.bg = c }
            }
            _ => {
                if is_drawing(op) {
                    skipped.push(op);
                }
                skip_op(&mut r, op, &st)?;
            }
        }
    }
    Ok(Picture {
        version: if v2 { 2 } else { 1 },
        frame,
        bounds,
        bitmaps,
        skipped,
        image: Image {
            width: canvas.w as u32,
            height: canvas.h as u32,
            rgba: canvas.rgba,
        },
    })
}

fn rgb48(r: &mut Reader<'_>) -> Result<[u8; 4]> {
    Ok([
        (r.u16_be()? >> 8) as u8,
        (r.u16_be()? >> 8) as u8,
        (r.u16_be()? >> 8) as u8,
        255,
    ])
}

/// Cores do QuickDraw clássico (`FgColor`/`BkColor`).
fn classic_color(c: u32) -> Option<[u8; 4]> {
    Some(match c {
        33 => BLACK,
        30 => WHITE,
        205 => [255, 0, 0, 255],
        341 => [0, 255, 0, 255],
        409 => [0, 0, 255, 255],
        273 => [0, 255, 255, 255],
        137 => [255, 0, 255, 255],
        69 => [255, 255, 0, 255],
        _ => return None,
    })
}

/// Opcodes que desenham algo (para avisar que a importação ficou parcial).
fn is_drawing(op: u16) -> bool {
    matches!(op, 0x0020..=0x0023 | 0x0028..=0x002b | 0x0030..=0x008f)
}

fn skip_sized16(r: &mut Reader<'_>) -> Result<()> {
    let n = r.u16_be()? as usize;
    r.skip(n)
}

/// Pula os dados de um opcode que não desenhamos.
fn skip_op(r: &mut Reader<'_>, op: u16, st: &State) -> Result<()> {
    let n: usize = match op {
        0x0000 | 0x0017..=0x0019 | 0x001c | 0x001e => 0,
        0x0001 | 0x0070..=0x0077 | 0x0080..=0x0087 => {
            // região ou polígono: o tamanho inclui a própria palavra
            let size = r.u16_be()? as usize;
            size.checked_sub(2)
                .ok_or_else(|| r.error(ErrorKind::Invalid("bad region size".into())))?
        }
        0x0002 | 0x0009 | 0x000a | 0x0010 => 8,
        0x0003 | 0x0005 | 0x0008 | 0x000d | 0x0015 | 0x0016 | 0x0023 | 0x00a0 => 2,
        0x0004 => 1,
        0x0006 | 0x0007 | 0x000b | 0x000c | 0x0021 => 4,
        0x0011 => {
            if st.v2 {
                2
            } else {
                1
            }
        }
        0x0012..=0x0014 => return skip_pixpat(r),
        0x001d | 0x001f | 0x0022 => 6,
        0x0020 => 8,
        0x0024..=0x0027 | 0x002c..=0x002f | 0x0092..=0x0097 | 0x009c..=0x009f | 0x00a2..=0x00af => {
            return skip_sized16(r);
        }
        0x0028 => return skip_text(r, 4),
        0x0029 | 0x002a => return skip_text(r, 1),
        0x002b => return skip_text(r, 2),
        0x0030..=0x0037 | 0x0040..=0x0047 | 0x0050..=0x0057 => 8,
        0x0038..=0x003f | 0x0048..=0x004f | 0x0058..=0x005f | 0x0078..=0x007f | 0x0088..=0x008f => {
            0
        }
        0x0060..=0x0067 => 12,
        0x0068..=0x006f => 4,
        0x00a1 => {
            r.skip(2)?;
            return skip_sized16(r);
        }
        0x00b0..=0x00cf | 0x8000..=0x80ff => 0,
        0x00d0..=0x00fe | 0x8100..=0xffff => r.u32_be()? as usize,
        0x0100..=0x7fff => (op as usize >> 8) * 2,
        _ => {
            return Err(r.error(ErrorKind::Unsupported(format!("opcode 0x{op:04x}"))));
        }
    };
    r.skip(n)
}

fn skip_text(r: &mut Reader<'_>, before: usize) -> Result<()> {
    r.skip(before)?;
    let n = r.u8()? as usize;
    r.skip(n)
}

/// `BkPixPat`/`PnPixPat`/`FillPixPat`: só pulamos.
fn skip_pixpat(r: &mut Reader<'_>) -> Result<()> {
    let kind = r.u16_be()?;
    r.skip(8)?; // padrão de 1 bit
    match kind {
        1 => {
            let pm = pixmap(r, true)?;
            let _ = color_table(r)?;
            let rows = pm.bounds.height().max(0) as usize;
            let _ = pixel_rows(r, &pm, rows, true, false)?;
            Ok(())
        }
        2 => r.skip(6),
        _ => Ok(()),
    }
}

struct PixMap {
    /// O bit 15 de `rowBytes` estava ligado: `PixMap` com tabela de cores.
    is_pixmap: bool,
    row_bytes: usize,
    bounds: Rect,
    pack_type: u16,
    pixel_size: u16,
    cmp_count: u16,
}

/// `rowBytes` + `bounds` e, se o bit 15 de `rowBytes` estiver ligado (ou
/// `force`), o resto do `PixMap` (36 bytes).
fn pixmap(r: &mut Reader<'_>, force: bool) -> Result<PixMap> {
    let rb = r.u16_be()?;
    let bounds = rect(r)?;
    let mut pm = PixMap {
        is_pixmap: rb & 0x8000 != 0,
        row_bytes: (rb & 0x3fff) as usize,
        bounds,
        pack_type: 0,
        pixel_size: 1,
        cmp_count: 1,
    };
    if rb & 0x8000 != 0 || force {
        let _version = r.u16_be()?;
        pm.pack_type = r.u16_be()?;
        let _pack_size = r.u32_be()?;
        r.skip(8)?; // hRes, vRes
        let _pixel_type = r.u16_be()?;
        pm.pixel_size = r.u16_be()?;
        pm.cmp_count = r.u16_be()?;
        let _cmp_size = r.u16_be()?;
        r.skip(12)?; // planeBytes, pmTable, pmReserved
    }
    if !bounds.is_valid() && bounds.width() != 0 {
        return Err(r.error(ErrorKind::Invalid("bad bitmap bounds".into())));
    }
    let w = bounds.width().max(0) as usize;
    let h = bounds.height().max(0) as usize;
    if w.saturating_mul(h) > MAX_PIXELS {
        return Err(r.error(ErrorKind::Invalid("bitmap too large".into())));
    }
    Ok(pm)
}

fn color_table(r: &mut Reader<'_>) -> Result<Vec<[u8; 4]>> {
    let _seed = r.u32_be()?;
    let flags = r.u16_be()?;
    let n = r.u16_be()? as usize + 1;
    if n > 256 {
        return Err(r.error(ErrorKind::Invalid(format!("{n} colors in table"))));
    }
    let mut pal = vec![BLACK; 256];
    for i in 0..n {
        let value = r.u16_be()? as usize;
        let c = rgb48(r)?;
        let idx = if flags & 0x8000 != 0 { i } else { value & 0xff };
        pal[idx] = c;
    }
    Ok(pal)
}

/// Lê as linhas de pixels de um bitmap. `packed`: o opcode admite
/// compressão (`PackBitsRect` e `PixPat`); `words`: PackBits de 16 bits
/// (packType 3).
fn pixel_rows(
    r: &mut Reader<'_>,
    pm: &PixMap,
    rows: usize,
    packed: bool,
    words: bool,
) -> Result<Vec<Vec<u8>>> {
    let rb = pm.row_bytes;
    let raw = |r: &mut Reader<'_>| -> Result<Vec<Vec<u8>>> {
        (0..rows).map(|_| Ok(r.bytes(rb)?.to_vec())).collect()
    };
    let compressed = |r: &mut Reader<'_>| -> Result<Vec<Vec<u8>>> {
        (0..rows)
            .map(|_| {
                let n = if rb > 250 {
                    r.u16_be()? as usize
                } else {
                    r.u8()? as usize
                };
                let src = r.bytes(n)?;
                Ok(unpack_row(src, rb, words))
            })
            .collect()
    };
    if !packed {
        return raw(r);
    }
    if rb >= 8 {
        return compressed(r);
    }
    // rowBytes < 8: pela especificação os dados vêm crus, mas há gravadores
    // (ImageMagick) que comprimem mesmo assim. Fica a leitura depois da qual
    // vem um opcode plausível.
    let start = r.pos();
    let raw_rows = raw(r);
    if raw_rows.is_ok() && plausible_next(r) {
        return raw_rows;
    }
    r.seek(start)?;
    let packed_rows = compressed(r);
    if packed_rows.is_ok() && plausible_next(r) {
        return packed_rows;
    }
    r.seek(start)?;
    raw(r)
}

fn plausible_next(r: &Reader<'_>) -> bool {
    let mut probe = r.clone();
    if probe.pos() % 2 == 1 && probe.skip(1).is_err() {
        return false;
    }
    match probe.u16_be() {
        Ok(op) => op <= 0x00af || op == 0x00ff || op == 0x0c00,
        Err(_) => false,
    }
}

/// Descomprime uma linha PackBits (de bytes ou de palavras), completando
/// com zeros ou cortando em `len` bytes.
fn unpack_row(src: &[u8], len: usize, words: bool) -> Vec<u8> {
    let unit = if words { 2 } else { 1 };
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    while i < src.len() && out.len() < len {
        let n = src[i];
        i += 1;
        if n < 128 {
            let k = (n as usize + 1) * unit;
            let end = (i + k).min(src.len());
            out.extend_from_slice(&src[i..end]);
            i = end;
        } else if n > 128 {
            let k = 257 - n as usize;
            let Some(v) = src.get(i..i + unit) else { break };
            for _ in 0..k {
                out.extend_from_slice(v);
            }
            i += unit;
        }
    }
    out.resize(len, 0);
    out
}

fn transfer_mode(mode: u16, st: &State) -> impl Fn([u8; 4]) -> bool {
    let bg = st.bg;
    let mode = mode & 0x7f;
    move |p| match mode {
        // srcOr, srcBic, transparent: o fundo não é desenhado
        1 | 3 | 36 => p != bg,
        _ => true,
    }
}

fn bits(r: &mut Reader<'_>, op: u16, st: &State, canvas: &mut Canvas) -> Result<()> {
    let pm = pixmap(r, false)?;
    let indexed = if pm.is_pixmap {
        Some(color_table(r)?)
    } else {
        None
    };
    let src_rect = rect(r)?;
    let dst_rect = rect(r)?;
    let mode = r.u16_be()?;
    if op & 1 == 1 {
        let size = r.u16_be()? as usize;
        r.skip(size.saturating_sub(2))?;
    }
    let w = pm.bounds.width().max(0) as usize;
    let h = pm.bounds.height().max(0) as usize;
    let rows = pixel_rows(r, &pm, h, op >= 0x0098, false)?;
    let depth = pm.pixel_size as usize;
    if !matches!(depth, 1 | 2 | 4 | 8) {
        return Err(r.error(ErrorKind::Unsupported(format!(
            "{depth}-bit indexed pixels"
        ))));
    }
    let pal: Vec<[u8; 4]> = match indexed {
        Some(p) => p,
        None => vec![st.bg, st.fg],
    };
    let mut px = Vec::with_capacity(w * h);
    for row in &rows {
        for x in 0..w {
            let bit = x * depth;
            let byte = row.get(bit / 8).copied().unwrap_or(0);
            let v = (byte >> (8 - depth - bit % 8)) & ((1u16 << depth) - 1) as u8;
            px.push(pal.get(v as usize).copied().unwrap_or(BLACK));
        }
    }
    canvas.blit(&px, pm.bounds, src_rect, dst_rect, transfer_mode(mode, st));
    Ok(())
}

fn direct_bits(r: &mut Reader<'_>, op: u16, st: &State, canvas: &mut Canvas) -> Result<()> {
    let _base = r.u32_be()?;
    let pm = pixmap(r, true)?;
    let src_rect = rect(r)?;
    let dst_rect = rect(r)?;
    let mode = r.u16_be()?;
    if op == 0x009b {
        let size = r.u16_be()? as usize;
        r.skip(size.saturating_sub(2))?;
    }
    let w = pm.bounds.width().max(0) as usize;
    let h = pm.bounds.height().max(0) as usize;
    let mut px = Vec::with_capacity(w * h);
    match pm.pixel_size {
        16 => {
            let packed = pm.pack_type != 1 && pm.row_bytes >= 8;
            let rows = if packed {
                pixel_rows(r, &pm, h, true, true)?
            } else {
                pixel_rows(r, &pm, h, false, false)?
            };
            for row in &rows {
                for x in 0..w {
                    let v = u16::from_be_bytes([
                        row.get(2 * x).copied().unwrap_or(0),
                        row.get(2 * x + 1).copied().unwrap_or(0),
                    ]);
                    let c5 = |s: u16| {
                        let c = ((v >> s) & 0x1f) as u8;
                        (c << 3) | (c >> 2)
                    };
                    px.push([c5(10), c5(5), c5(0), 255]);
                }
            }
        }
        32 => {
            let cmp = pm.cmp_count.clamp(3, 4) as usize;
            let rows: Vec<Vec<u8>> = match pm.pack_type {
                // RGB de 24 bits sem compressão e sem contagens
                2 => (0..h)
                    .map(|_| Ok(r.bytes(w * 3)?.to_vec()))
                    .collect::<Result<_>>()?,
                1 => pixel_rows(r, &pm, h, false, false)?,
                _ if pm.row_bytes < 8 => pixel_rows(r, &pm, h, false, false)?,
                // 0 ou 4: componentes em planos, cada linha em PackBits
                _ => {
                    let rb = pm.row_bytes;
                    (0..h)
                        .map(|_| {
                            let n = if rb > 250 {
                                r.u16_be()? as usize
                            } else {
                                r.u8()? as usize
                            };
                            Ok(unpack_row(r.bytes(n)?, cmp * w, false))
                        })
                        .collect::<Result<_>>()?
                }
            };
            for row in &rows {
                for x in 0..w {
                    let g = |i: usize| row.get(i).copied().unwrap_or(0);
                    let c = match pm.pack_type {
                        2 => [g(3 * x), g(3 * x + 1), g(3 * x + 2), 255],
                        1 => [g(4 * x + 1), g(4 * x + 2), g(4 * x + 3), 255],
                        _ if pm.row_bytes < 8 => [g(4 * x + 1), g(4 * x + 2), g(4 * x + 3), 255],
                        _ => {
                            let o = if cmp == 4 { w } else { 0 }; // pula o alfa
                            [g(o + x), g(o + w + x), g(o + 2 * w + x), 255]
                        }
                    };
                    px.push(c);
                }
            }
        }
        n => {
            return Err(r.error(ErrorKind::Unsupported(format!("{n}-bit direct pixels"))));
        }
    }
    canvas.blit(&px, pm.bounds, src_rect, dst_rect, transfer_mode(mode, st));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkbm::pack_bits;

    /// Montador de figuras de teste.
    struct Pic {
        v2: bool,
        b: Vec<u8>,
    }

    impl Pic {
        fn new(v2: bool, w: i16, h: i16) -> Pic {
            let mut b = vec![0, 0];
            for n in [0i16, 0, h, w] {
                b.extend(n.to_be_bytes());
            }
            if v2 {
                b.extend([0x00, 0x11, 0x02, 0xff, 0x0c, 0x00, 0xff, 0xff, 0xff, 0xff]);
                b.extend([0u8; 20]);
            } else {
                b.extend([0x11, 0x01]);
            }
            Pic { v2, b }
        }
        fn op(&mut self, op: u16) -> &mut Pic {
            if self.v2 {
                if self.b.len() % 2 == 1 {
                    self.b.push(0);
                }
                self.b.extend(op.to_be_bytes());
            } else {
                self.b.push(op as u8);
            }
            self
        }
        fn raw(&mut self, d: &[u8]) -> &mut Pic {
            self.b.extend_from_slice(d);
            self
        }
        fn w16(&mut self, v: u16) -> &mut Pic {
            self.raw(&v.to_be_bytes())
        }
        fn rect(&mut self, t: i16, l: i16, b: i16, r: i16) -> &mut Pic {
            for n in [t, l, b, r] {
                self.b.extend(n.to_be_bytes());
            }
            self
        }
        fn pixmap_fields(&mut self, pack: u16, size: u16, cmp: u16) -> &mut Pic {
            self.w16(0).w16(pack).raw(&[0; 4]);
            self.raw(&[0, 0x48, 0, 0, 0, 0x48, 0, 0]);
            self.w16(if size > 8 { 16 } else { 0 })
                .w16(size)
                .w16(cmp)
                .w16(8);
            self.raw(&[0; 12])
        }
        fn file(&mut self) -> Vec<u8> {
            self.op(0x00ff);
            let mut v = vec![0u8; 512];
            v.extend_from_slice(&self.b);
            v
        }
    }

    fn px(p: &Picture, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * p.image.width + x) * 4) as usize;
        [p.image.rgba[i], p.image.rgba[i + 1], p.image.rgba[i + 2]]
    }

    const W: [u8; 3] = [255, 255, 255];
    const K: [u8; 3] = [0, 0, 0];

    #[test]
    fn v2_bits_rect_with_skipped_ops() {
        let mut p = Pic::new(true, 12, 4);
        // região de recorte, comentário longo, estilo de texto, texto, retângulo
        p.op(0x0001).w16(10).rect(0, 0, 4, 12);
        p.op(0x00a1).w16(100).w16(3).raw(b"abc");
        p.op(0x0004).raw(&[1]);
        p.op(0x002b).raw(&[1, 1, 2]).raw(b"oi");
        p.op(0x0031).rect(0, 0, 2, 2);
        p.op(0x001a).raw(&[0xff, 0xff, 0, 0, 0, 0]); // frente vermelha
        // BitsRect 1 bit, 10×2, rowBytes 2, sem compressão
        p.op(0x0090).w16(2).rect(0, 0, 2, 10);
        p.rect(0, 0, 2, 10).rect(1, 1, 3, 11).w16(0);
        p.raw(&[0b1000_0000, 0b0100_0000, 0b0000_0001, 0b0000_0000]);
        let pic = read(&p.file()).unwrap();
        assert_eq!(pic.version, 2);
        assert_eq!((pic.image.width, pic.image.height), (12, 4));
        assert_eq!(pic.bitmaps, 1);
        assert_eq!(pic.skipped, [0x002b, 0x0031]);
        assert_eq!(px(&pic, 1, 1), [255, 0, 0]);
        assert_eq!(px(&pic, 10, 1), [255, 0, 0]);
        assert_eq!(px(&pic, 8, 2), [255, 0, 0]);
        assert_eq!(px(&pic, 2, 1), W);
        assert_eq!(px(&pic, 0, 0), W);
    }

    #[test]
    fn packbits_rect_one_bit_and_src_or() {
        // 64×2 de 1 bit, rowBytes 8 (comprimido); depois um segundo bitmap
        // em srcOr que só pinta os pixels de frente
        let mut p = Pic::new(true, 64, 2);
        let rows = [[0xffu8; 8], [0x00; 8]];
        p.op(0x0098).w16(8).rect(0, 0, 2, 64);
        p.rect(0, 0, 2, 64).rect(0, 0, 2, 64).w16(0);
        for row in rows {
            let c = pack_bits(&row);
            p.raw(&[c.len() as u8]).raw(&c);
        }
        p.op(0x0098).w16(8).rect(0, 0, 2, 64);
        p.rect(0, 0, 2, 64).rect(0, 0, 2, 64).w16(1);
        for row in [[0u8; 8], [0x80, 0, 0, 0, 0, 0, 0, 0]] {
            let c = pack_bits(&row);
            p.raw(&[c.len() as u8]).raw(&c);
        }
        let pic = read(&p.file()).unwrap();
        assert_eq!(pic.bitmaps, 2);
        assert_eq!(px(&pic, 63, 0), K); // o branco do segundo não apagou
        assert_eq!(px(&pic, 0, 1), K);
        assert_eq!(px(&pic, 1, 1), W);
    }

    #[test]
    fn indexed_pixmap_scaled() {
        // 4 bits, 4×2, tabela de 3 cores, desenhado em dobro (8×4)
        let mut p = Pic::new(true, 8, 4);
        p.op(0x0098)
            .w16(0x8000 | 2)
            .rect(0, 0, 2, 4)
            .pixmap_fields(0, 4, 1);
        p.raw(&[0, 0, 0, 0]).w16(0).w16(2);
        p.w16(0).raw(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        p.w16(1).raw(&[0, 0, 0, 0, 0xff, 0xff]);
        p.w16(2).raw(&[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);
        p.rect(0, 0, 2, 4).rect(0, 0, 4, 8).w16(0);
        p.raw(&[0x01, 0x20, 0x22, 0x10]); // rowBytes < 8: cru
        let pic = read(&p.file()).unwrap();
        assert_eq!(px(&pic, 0, 0), W);
        assert_eq!(px(&pic, 3, 1), [0, 0, 255]);
        assert_eq!(px(&pic, 4, 0), [0x12, 0x56, 0x9a]);
        assert_eq!(px(&pic, 7, 3), W);
        assert_eq!(px(&pic, 4, 2), [0, 0, 255]);
    }

    #[test]
    fn short_rows_packed_anyway() {
        // rowBytes 3 (< 8) mas comprimido, como grava o ImageMagick
        let mut p = Pic::new(true, 3, 2);
        p.op(0x0098)
            .w16(0x8000 | 3)
            .rect(0, 0, 2, 3)
            .pixmap_fields(0, 8, 1);
        p.raw(&[0, 0, 0, 0]).w16(0x8000).w16(1);
        p.w16(0).raw(&[0xff; 6]).w16(0).raw(&[0; 6]);
        p.rect(0, 0, 2, 3).rect(0, 0, 2, 3).w16(0);
        p.raw(&[2, 0xfe, 0x01]); // linha 0: 3 × índice 1 (preto)
        p.raw(&[3, 0x02, 0x00, 0x01, 0x00]); // linha 1: 0 1 0
        let pic = read(&p.file()).unwrap();
        assert_eq!([px(&pic, 0, 0), px(&pic, 2, 0)], [K, K]);
        assert_eq!([px(&pic, 0, 1), px(&pic, 1, 1), px(&pic, 2, 1)], [W, K, W]);
    }

    #[test]
    fn direct_bits_16_and_24() {
        let mut p = Pic::new(true, 8, 2);
        // 16 bits, packType 3: linha 0 toda vermelha (run de palavras), linha 1 literal
        p.op(0x009a)
            .raw(&[0, 0, 0, 0xff])
            .w16(0x8000 | 16)
            .rect(0, 0, 2, 8);
        p.pixmap_fields(3, 16, 3);
        p.rect(0, 0, 2, 8).rect(0, 0, 1, 8).w16(0); // só a primeira linha vai para o destino (encolhe)
        p.raw(&[3, 0xf9, 0x7c, 0x00]);
        let lit: Vec<u8> = [0u8, 7].into_iter().chain([0x03, 0xe0].repeat(8)).collect();
        p.raw(&lit[..1]).raw(&lit[1..]);
        // 32 bits, packType 2 (RGB cru): 2×1 na linha 1
        p.op(0x009a)
            .raw(&[0, 0, 0, 0xff])
            .w16(0x8000 | 8)
            .rect(0, 0, 1, 2);
        p.pixmap_fields(2, 32, 3);
        p.rect(0, 0, 1, 2).rect(1, 0, 2, 2).w16(0);
        p.raw(&[1, 2, 3, 200, 100, 50]);
        let pic = read(&p.file()).unwrap();
        assert_eq!(pic.bitmaps, 2);
        assert_eq!(px(&pic, 5, 0), [255, 0, 0]);
        assert_eq!(px(&pic, 0, 1), [1, 2, 3]);
        assert_eq!(px(&pic, 1, 1), [200, 100, 50]);
        assert_eq!(px(&pic, 2, 1), W);
    }

    #[test]
    fn direct_bits_planar_with_alpha() {
        // 32 bits, packType 4, 4 componentes (alfa primeiro), 2×1
        let mut p = Pic::new(true, 2, 1);
        p.op(0x009a)
            .raw(&[0, 0, 0, 0xff])
            .w16(0x8000 | 8)
            .rect(0, 0, 1, 2);
        p.pixmap_fields(4, 32, 4);
        p.rect(0, 0, 1, 2).rect(0, 0, 1, 2).w16(0x40);
        let planes = [0xff, 0xff, 10, 20, 30, 40, 50, 60];
        let c = pack_bits(&planes);
        p.raw(&[c.len() as u8]).raw(&c);
        let pic = read(&p.file()).unwrap();
        assert_eq!(px(&pic, 0, 0), [10, 30, 50]);
        assert_eq!(px(&pic, 1, 0), [20, 40, 60]);
    }

    #[test]
    fn version_1_and_embedded() {
        let mut p = Pic::new(false, 8, 1);
        p.op(0x00a0).w16(5); // comentário curto
        p.op(0x0090)
            .w16(2)
            .rect(0, 0, 1, 8)
            .rect(0, 0, 1, 8)
            .rect(0, 0, 1, 8)
            .w16(0);
        p.raw(&[0b1010_0000, 0]);
        let file = p.file();
        assert_eq!(header_offset(&file), Some(512));
        assert_eq!(header_offset(&file[512..]), Some(0));
        for data in [&file[..], &file[512..]] {
            let pic = read(data).unwrap();
            assert_eq!(pic.version, 1);
            assert_eq!([px(&pic, 0, 0), px(&pic, 1, 0), px(&pic, 2, 0)], [K, W, K]);
        }
    }

    #[test]
    fn rejects_garbage() {
        assert!(!is_pict(b"nada"));
        assert!(read(&[0u8; 600]).is_err());
        // opcode de bitmap truncado
        let mut p = Pic::new(true, 8, 8);
        p.op(0x0090).w16(2);
        let mut f = p.b.clone();
        f.splice(0..0, [0u8; 512]);
        assert!(read(&f).is_err());
        // figura grande demais
        let mut big = Pic::new(true, 0x7fff, 0x7fff).file();
        big.truncate(600);
        assert!(read(&big).is_err());
    }
}
