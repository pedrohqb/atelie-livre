//! Desenhos salvos do Fine Artist (`.MAG`; ver `docs/formats/mag.md`).
//!
//! Um `.MAG` é um arquivo `CHNK` little-endian de criador `SPLT`. O leitor
//! aproveita a paleta (`CTBL`), a pintura (`PKBM 0`), o título (`TITL`) e o
//! autor (`AUTH`) e ignora os adesivos e palavras soltos (`SOBJ`, `COBJ`), que
//! o original já desenhou na pintura. O gravador produz os mesmos chunks, na
//! mesma ordem, de um desenho de exemplo da instalação (`SAMP6.MAG`), sem
//! objetos soltos.

use crate::chnk::{self, OwnedChunk};
use crate::{ErrorKind, Format, FormatError, Result, Rgb, Tag, cp1252, palette, pkbm};

/// Tamanho da pintura do Fine Artist.
pub const PICTURE_W: u16 = 594;
pub const PICTURE_H: u16 = 347;
/// Largura das miniaturas (`THMB 0`); a altura segue a proporção, até 144.
pub const THUMB_W: u16 = 144;
const THUMB_MAX_H: u16 = 144;
/// Canto superior esquerdo da pintura na janela do original, gravado no `SDOC`.
const ORIGIN: (u16, u16) = (23, 109);

/// Criador dos documentos do Fine Artist.
pub const CREATOR: Tag = Tag(*b"SPLT");

/// Bytes 20/21 do cabeçalho `PKBM` da pintura principal nos desenhos salvos.
const PICTURE_UNKNOWN: pkbm::UnknownBytes = (40, 4);

// Partes fixas copiadas de `SHARED/USERS/MAGGIE/SAMP6.MAG` (instalação em
// português, 1995). Os significados são desconhecidos, exceto os tamanhos e
// retângulos da pintura, que `write` ajusta ao tamanho gravado.
/// `FLOR 0`, parte `FLAG`.
const FLOR: [u8; 2] = [0x02, 0x00];
/// `WIZA 0`, parte `FLAG`.
const WIZA: [u8; 4] = [0; 4];
/// `FWLS 0`: cabeçalho `01 00 07 00 "FWLS"`, 1, zeros, largura e altura (offsets 14 e 16).
const FWLS: [u8; 26] = [
    0x01, 0x00, 0x07, 0x00, b'F', b'W', b'L', b'S', 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x52, 0x02,
    0x5b, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
/// `FLS  0`: cabeçalho `01 00 07 00 "FLS "`, um `FOS ` com dois retângulos
/// `(0, 0, largura, altura)` (offsets 22 e 30) e constantes.
const FLS: [u8; 166] = [
    0x01, 0x00, 0x07, 0x00, b'F', b'L', b'S', b' ', 0x01, 0x00, 0x01, 0x00, 0x07, 0x00, b'F', b'O',
    b'S', b' ', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x52, 0x02, 0x5b, 0x01, 0x00, 0x00,
    0x00, 0x00, 0x52, 0x02, 0x5b, 0x01, 0x03, 0x00, 0x00, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x10, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff,
    0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
    0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
/// `SDOC 0`: cabeçalho `01 00 07 00 "SDOC"`, `TLPS`, 1, 1, o retângulo da
/// pintura na janela `(23, 109, 617, 456)` (offsets 16 a 22) e constantes.
const SDOC: [u8; 88] = [
    0x01, 0x00, 0x07, 0x00, b'S', b'D', b'O', b'C', b'T', b'L', b'P', b'S', 0x01, 0x00, 0x01, 0x00,
    0x17, 0x00, 0x6d, 0x00, 0x69, 0x02, 0xc8, 0x01, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0xcc, 0x10, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Um desenho: título, autor, paleta de 256 cores e índices da pintura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub title: String,
    pub author: String,
    pub palette: Vec<Rgb>,
    pub width: u16,
    pub height: u16,
    /// Um índice da paleta por pixel, linha a linha, `width × height`.
    pub pixels: Vec<u8>,
}

impl Picture {
    /// Desenho de 594×347 com a paleta dada.
    pub fn new(title: &str, author: &str, palette: Vec<Rgb>, pixels: Vec<u8>) -> Picture {
        Picture {
            title: title.into(),
            author: author.into(),
            palette,
            width: PICTURE_W,
            height: PICTURE_H,
            pixels,
        }
    }
}

fn err(offset: usize, kind: ErrorKind) -> FormatError {
    FormatError {
        format: Format::Mag,
        offset,
        kind,
    }
}

/// Lê um desenho salvo (qualquer tamanho; o Fine Artist grava 594×347).
///
/// Paleta: `CTBL` do arquivo; na falta, a embutida na miniatura ou na pintura;
/// sem nenhuma, erro. Título e autor ausentes ficam vazios.
pub fn read(data: &[u8]) -> Result<Picture> {
    let f = chnk::parse(data)?;
    let part = |tag: &[u8; 4], id: u16| {
        f.chunks
            .iter()
            .find(|c| c.tag.0 == *tag && c.id == id)
            .and_then(|c| c.parts.first())
    };
    let Some(pic) = part(b"PKBM", 0) else {
        return Err(err(0, ErrorKind::Invalid("no PKBM 0 chunk".into())));
    };
    let bitmap = pkbm::decode(pic.data).map_err(|e| err(pic.offset + e.offset, e.kind))?;
    let ctbl = part(b"CTBL", 0)
        .map(|p| palette::parse_ctbl(p.data).map_err(|e| err(p.offset + e.offset, e.kind)));
    let palette = match ctbl {
        Some(p) => p?,
        None => {
            let thumb = part(b"THMB", 0)
                .and_then(|p| pkbm::decode(p.data).ok())
                .and_then(|b| b.palette);
            match thumb.or(bitmap.palette) {
                Some(p) => p,
                None => return Err(err(0, ErrorKind::Invalid("no palette".into()))),
            }
        }
    };
    let text = |tag: &[u8; 4]| {
        part(tag, 0)
            .map(|p| cp1252::decode(p.data))
            .unwrap_or_default()
    };
    Ok(Picture {
        title: text(b"TITL"),
        author: text(b"AUTH"),
        palette,
        width: bitmap.width,
        height: bitmap.height,
        pixels: bitmap.pixels,
    })
}

/// Reduz por vizinho mais próximo para `THUMB_W` de largura, altura na
/// proporção (até 144, como nas miniaturas do original).
pub fn thumbnail(pixels: &[u8], w: u16, h: u16) -> (Vec<u8>, u16, u16) {
    if w == 0 || h == 0 {
        return (Vec::new(), 0, 0);
    }
    let tw = THUMB_W.min(w);
    let th = ((h as u32 * tw as u32 + w as u32 / 2) / w as u32).clamp(1, THUMB_MAX_H as u32) as u16;
    let mut out = Vec::with_capacity(tw as usize * th as usize);
    for y in 0..th as usize {
        let sy = y * h as usize / th as usize;
        for x in 0..tw as usize {
            let sx = x * w as usize / tw as usize;
            out.push(pixels.get(sy * w as usize + sx).copied().unwrap_or(0));
        }
    }
    (out, tw, th)
}

fn text_bytes(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| cp1252::encode_char(c).unwrap_or(b'?'))
        .collect()
}

fn put16(v: &mut [u8], at: usize, n: u16) {
    v[at..at + 2].copy_from_slice(&n.to_le_bytes());
}

/// Grava o desenho como `.MAG`, com os chunks na ordem de `SAMP6.MAG`:
/// `CTBL`, `FLOR`, `WIZA`, `FWLS`, `PKBM 0`, `FLS `, `THMB 0` (miniatura),
/// `THMB 1` (cópia da pintura com paleta), `TITL`, `AUTH`, `SDOC`.
pub fn write(p: &Picture) -> Vec<u8> {
    let (w, h) = (p.width, p.height);
    let mut fwls = FWLS;
    put16(&mut fwls, 14, w);
    put16(&mut fwls, 16, h);
    let mut fls = FLS;
    for at in [26, 34] {
        put16(&mut fls, at, w);
        put16(&mut fls, at + 2, h);
    }
    let mut sdoc = SDOC;
    put16(&mut sdoc, 16, ORIGIN.0);
    put16(&mut sdoc, 18, ORIGIN.1);
    put16(&mut sdoc, 20, ORIGIN.0 + w);
    put16(&mut sdoc, 22, ORIGIN.1 + h);

    let picture = pkbm::Bitmap {
        width: w,
        height: h,
        depth: 8,
        pixels: p.pixels.clone(),
        palette: None,
    };
    let full = pkbm::Bitmap {
        palette: Some(p.palette.clone()),
        ..picture.clone()
    };
    let (tp, tw, th) = thumbnail(&p.pixels, w, h);
    let thumb = pkbm::Bitmap {
        width: tw,
        height: th,
        depth: 8,
        pixels: tp,
        palette: Some(p.palette.clone()),
    };
    let chunks = [
        OwnedChunk::single(b"CTBL", 0, b"CTBL", palette::write_ctbl(&p.palette)),
        OwnedChunk::single(b"FLOR", 0, b"FLAG", FLOR.to_vec()),
        OwnedChunk::single(b"WIZA", 0, b"FLAG", WIZA.to_vec()),
        OwnedChunk::single(b"FWLS", 0, b"FWLS", fwls.to_vec()),
        OwnedChunk::single(
            b"PKBM",
            0,
            b"PKBM",
            pkbm::encode_with(&picture, PICTURE_UNKNOWN),
        ),
        OwnedChunk::single(b"FLS ", 0, b"FLS ", fls.to_vec()),
        OwnedChunk::single(b"THMB", 0, b"PKBM", pkbm::encode(&thumb)),
        OwnedChunk::single(b"THMB", 1, b"PKBM", pkbm::encode(&full)),
        OwnedChunk::single(b"TITL", 0, b"TEXT", text_bytes(&p.title)),
        OwnedChunk::single(b"AUTH", 0, b"TEXT", text_bytes(&p.author)),
        OwnedChunk::single(b"SDOC", 0, b"SDOC", sdoc.to_vec()),
    ];
    chnk::write(CREATOR, &chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pal() -> Vec<Rgb> {
        (0..=255u8)
            .map(|i| Rgb {
                r: i,
                g: i.wrapping_mul(3),
                b: 255 - i,
            })
            .collect()
    }

    fn picture() -> Picture {
        let n = PICTURE_W as usize * PICTURE_H as usize;
        let pixels = (0..n)
            .map(|k| if k % 1000 < 600 { 7 } else { (k % 251) as u8 })
            .collect();
        Picture::new("Quadro Novo", "Lisa Çã", pal(), pixels)
    }

    #[test]
    fn write_then_read_roundtrips() {
        let p = picture();
        let d = write(&p);
        assert!(chnk::is_chnk(&d));
        assert_eq!(d.len() % 0x200, 0);
        assert_eq!(read(&d).unwrap(), p);
    }

    #[test]
    fn written_chunks_follow_samp6_order() {
        let d = write(&picture());
        let f = chnk::parse(&d).unwrap();
        assert_eq!(f.creator, CREATOR);
        let ids: Vec<(String, u16, String)> = f
            .chunks
            .iter()
            .map(|c| (c.tag.to_string(), c.id, c.parts[0].tag.to_string()))
            .collect();
        let want = [
            ("CTBL", 0, "CTBL"),
            ("FLOR", 0, "FLAG"),
            ("WIZA", 0, "FLAG"),
            ("FWLS", 0, "FWLS"),
            ("PKBM", 0, "PKBM"),
            ("FLS ", 0, "FLS "),
            ("THMB", 0, "PKBM"),
            ("THMB", 1, "PKBM"),
            ("TITL", 0, "TEXT"),
            ("AUTH", 0, "TEXT"),
            ("SDOC", 0, "SDOC"),
        ];
        assert_eq!(ids.len(), want.len());
        for (got, (t, i, pt)) in ids.iter().zip(want) {
            assert_eq!((got.0.as_str(), got.1, got.2.as_str()), (t, i, pt));
        }
        // partes fixas iguais às de SAMP6.MAG para 594×347
        assert_eq!(f.chunks[3].parts[0].data, &FWLS);
        assert_eq!(f.chunks[5].parts[0].data, &FLS);
        assert_eq!(f.chunks[10].parts[0].data, &SDOC);
        assert_eq!(f.chunks[8].parts[0].data, b"Quadro Novo");
        assert_eq!(f.chunks[9].parts[0].data, b"Lisa \xc7\xe3");
        // miniatura de 144×84 com paleta; cópia de 594×347 com paleta
        let t = pkbm::decode(f.chunks[6].parts[0].data).unwrap();
        assert_eq!((t.width, t.height, t.depth), (144, 84, 8));
        assert_eq!(t.palette.as_deref(), Some(pal().as_slice()));
        let c = pkbm::decode(f.chunks[7].parts[0].data).unwrap();
        assert_eq!((c.width, c.height), (PICTURE_W, PICTURE_H));
        assert_eq!(c.pixels, picture().pixels);
        // a pintura principal não traz paleta e usa os bytes 20/21 do original
        let m = f.chunks[4].parts[0].data;
        assert_eq!((m[20], m[21]), (40, 4));
        assert_eq!(pkbm::decode(m).unwrap().palette, None);
    }

    #[test]
    fn thumbnail_sizes_match_originals() {
        let px = vec![0u8; 594 * 347];
        assert_eq!(thumbnail(&px, 594, 347).1, 144);
        assert_eq!(thumbnail(&px, 594, 347).2, 84);
        let px = vec![0u8; 270 * 216];
        assert_eq!(thumbnail(&px, 270, 216).2, 115);
        let px = vec![0u8; 162 * 198];
        assert_eq!(thumbnail(&px, 162, 198).2, 144);
        let px = vec![0u8; 162 * 162];
        assert_eq!(thumbnail(&px, 162, 162).2, 144);
        let px = vec![0u8; 216 * 162];
        assert_eq!(thumbnail(&px, 216, 162).2, 108);
        assert_eq!(thumbnail(&[], 0, 5), (Vec::new(), 0, 0));
        // imagem menor que a miniatura não é ampliada
        let (t, w, h) = thumbnail(&[1, 2, 3, 4], 2, 2);
        assert_eq!((w, h), (2, 2));
        assert_eq!(t, [1, 2, 3, 4]);
    }

    #[test]
    fn thumbnail_is_nearest_neighbour() {
        let mut px = vec![0u8; 594 * 347];
        px[0] = 9;
        px[(346 * 594) + 593] = 8;
        let (t, w, h) = thumbnail(&px, 594, 347);
        assert_eq!(t[0], 9);
        assert_eq!(t.len(), w as usize * h as usize);
        assert!(t.iter().filter(|&&v| v == 9).count() == 1);
    }

    #[test]
    fn other_sizes_adjust_fixed_parts() {
        let mut p = picture();
        p.width = 162;
        p.height = 198;
        p.pixels.truncate(162 * 198);
        let d = write(&p);
        let f = chnk::parse(&d).unwrap();
        let fwls = f.chunks[3].parts[0].data;
        assert_eq!(&fwls[14..18], &[162, 0, 198, 0]);
        let sdoc = f.chunks[10].parts[0].data;
        assert_eq!(&sdoc[16..24], &[23, 0, 109, 0, 185, 0, 51, 1]);
        assert_eq!(read(&d).unwrap(), p);
    }

    #[test]
    fn read_without_ctbl_uses_thumbnail_palette() {
        let p = picture();
        let d = write(&p);
        let f = chnk::parse(&d).unwrap();
        let chunks: Vec<OwnedChunk> = f
            .chunks
            .iter()
            .filter(|c| c.tag.0 != *b"CTBL")
            .map(OwnedChunk::from)
            .collect();
        let d2 = chnk::write(CREATOR, &chunks);
        assert_eq!(read(&d2).unwrap(), p);
        // sem paleta alguma
        let chunks: Vec<OwnedChunk> = chunks.into_iter().filter(|c| c.tag.0 != *b"THMB").collect();
        let d3 = chnk::write(CREATOR, &chunks);
        let e = read(&d3).unwrap_err();
        assert_eq!(e.format, Format::Mag);
        assert!(matches!(e.kind, ErrorKind::Invalid(_)));
    }

    #[test]
    fn read_without_picture_or_texts() {
        let d = chnk::write(CREATOR, &[]);
        assert!(read(&d).is_err());
        let p = picture();
        let d = write(&p);
        let f = chnk::parse(&d).unwrap();
        let chunks: Vec<OwnedChunk> = f
            .chunks
            .iter()
            .filter(|c| !matches!(&c.tag.0, b"TITL" | b"AUTH"))
            .map(OwnedChunk::from)
            .collect();
        let got = read(&chnk::write(CREATOR, &chunks)).unwrap();
        assert_eq!(got.title, "");
        assert_eq!(got.author, "");
        assert_eq!(got.pixels, p.pixels);
    }

    #[test]
    fn read_rejects_garbage_and_truncations() {
        assert!(read(b"nada").is_err());
        let d = write(&picture());
        for n in (0..d.len()).step_by(97) {
            let _ = read(&d[..n]);
        }
    }
}
