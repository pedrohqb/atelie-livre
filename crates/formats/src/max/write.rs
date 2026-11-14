//! Gravação de documentos `.MAX`.

use super::{
    CREATOR, Document, Page, Run, TEXT_FC, encode_text,
    fkp::{self, FkpRun, PAGE},
    font, plc,
};
use crate::chnk::{self, OwnedChunk};

/// Conteúdo do chunk `INK ` (versão do formato).
const INK: [u8; 4] = [4, 0, 4, 0];
const SELN_LEN: usize = 48;

/// Grava o documento como `.MAX`, na ordem de chunks do original:
///
/// `TEXT` (em 0x200), `FDPP 0..`, `FDPC 0..` (páginas FKP alinhadas a 512),
/// os [`Document::objects`], `DOP `, `FRAM` (se houver molduras), `BTEP`,
/// `BTEC`, `PGD `, `FONT`, `SNDT` (se houver sons em palavras), `PRNT`,
/// `SELN`, `THMB` (os três se presentes), `ADD `, `AUTH`, `TITL`, `FLOR`,
/// `WIZA`, `INK `.
///
/// Os trechos são gravados como estão; só são cortados ao texto e, onde não
/// cobrem o texto, completados com trechos sem propriedades. Sem folhas,
/// grava uma folha com o texto inteiro.
pub fn write(doc: &Document) -> Vec<u8> {
    let text = encode_text(&doc.text);
    let len = text.len() as u32;
    let fc = |cp: u32| TEXT_FC + cp;
    let fkp_runs = |runs: Vec<(u32, u32, Vec<u8>)>| -> Vec<FkpRun> {
        runs.into_iter()
            .map(|(s, e, grpprl)| FkpRun {
                start: fc(s),
                end: fc(e),
                grpprl,
            })
            .collect()
    };
    let para_pages = fkp::write_pages(&fkp_runs(cover(&doc.para_runs, len, |p| p.to_grpprl())));
    let char_pages = fkp::write_pages(&fkp_runs(cover(&doc.char_runs, len, |p| p.to_grpprl())));

    // as páginas FKP vêm logo depois do texto, alinhadas a 512
    let fkp_base = (TEXT_FC + len).next_multiple_of(PAGE as u32);
    let bte = |pages: &[[u8; PAGE]], first: u32| {
        let mut pos = Vec::with_capacity(pages.len() + 1);
        let mut elems = Vec::with_capacity(pages.len());
        for (k, p) in pages.iter().enumerate() {
            pos.push(u32::from_le_bytes([p[4], p[5], p[6], p[7]]));
            elems.push((first + (k * PAGE) as u32).to_le_bytes().to_vec());
        }
        pos.push(fc(len));
        plc::write(plc::EXTRA_BTE, 4, &pos, &elems)
    };
    let btep = bte(&para_pages, fkp_base);
    let btec = bte(&char_pages, fkp_base + (para_pages.len() * PAGE) as u32);

    let mut chunks: Vec<OwnedChunk> = Vec::new();
    chunks.push(OwnedChunk::single(b"TEXT", 0, b"TEXT", text));
    for (tag, pages) in [(b"FDPP", &para_pages), (b"FDPC", &char_pages)] {
        for (k, p) in pages.iter().enumerate() {
            chunks.push(OwnedChunk::single(tag, k as u16, tag, p.to_vec()));
        }
    }
    chunks.extend(doc.objects.iter().cloned());

    let mut dop = doc.dop.to_grpprl();
    dop.truncate(255);
    dop.insert(0, dop.len() as u8);
    chunks.push(OwnedChunk::single(b"DOP ", 0, b"DOP ", dop));
    if !doc.frames.is_empty() {
        let mut v = (doc.frames.len() as u16).to_le_bytes().to_vec();
        for f in &doc.frames {
            let mut g = f.to_grpprl();
            g.truncate(255);
            v.push(g.len() as u8);
            v.extend_from_slice(&g);
        }
        chunks.push(OwnedChunk::single(b"FRAM", 0, b"FRAM", v));
    }
    chunks.push(OwnedChunk::single(b"BTEP", 0, b"PLC ", btep));
    chunks.push(OwnedChunk::single(b"BTEC", 0, b"PLC ", btec));

    let whole = [Page {
        start: 0,
        end: len,
        flags: 0,
    }];
    let pages: &[Page] = if doc.pages.is_empty() {
        &whole
    } else {
        &doc.pages
    };
    let mut pos: Vec<u32> = pages.iter().map(|p| p.start).collect();
    pos.push(pages.last().map_or(len, |p| p.end));
    let elems: Vec<Vec<u8>> = pages
        .iter()
        .map(|p| p.flags.to_le_bytes().to_vec())
        .collect();
    chunks.push(OwnedChunk::single(
        b"PGD ",
        0,
        b"PLC ",
        plc::write(plc::EXTRA_PGD, 2, &pos, &elems),
    ));
    chunks.push(OwnedChunk::single(
        b"FONT",
        0,
        b"FONT",
        font::write(&doc.fonts),
    ));

    if !doc.word_sounds.is_empty() {
        let mut pos: Vec<u32> = doc.word_sounds.iter().map(|w| w.start).collect();
        pos.push(doc.word_sounds.last().map_or(0, |w| w.end));
        let elems: Vec<Vec<u8>> = doc
            .word_sounds
            .iter()
            .map(|w| [&w.reserved[..], &w.sound.to_bytes()[..]].concat())
            .collect();
        chunks.push(OwnedChunk::single(
            b"SNDT",
            0,
            b"PLC ",
            plc::write(plc::EXTRA_SNDT, 10, &pos, &elems),
        ));
    }
    if let Some(p) = &doc.printer {
        chunks.push(OwnedChunk::single(b"PRNT", 0, b"WNPR", p.clone()));
    }
    if let Some((s, e)) = doc.selection {
        let mut v = vec![0u8; SELN_LEN];
        v[0..4].copy_from_slice(&[2, 0, 0xff, 0]);
        v[4..8].copy_from_slice(&s.to_le_bytes());
        v[8..12].copy_from_slice(&e.to_le_bytes());
        v[16..18].copy_from_slice(&(-1i16).to_le_bytes());
        chunks.push(OwnedChunk::single(b"SELN", 0, b"SELN", v));
    }
    if let Some(t) = &doc.thumbnail {
        chunks.push(OwnedChunk::single(b"THMB", 0, b"PKBM", t.clone()));
    }
    let add = &doc.add;
    chunks.push(OwnedChunk::single(
        b"ADD ",
        0,
        b"ADD ",
        add.to_bytes().to_vec(),
    ));
    chunks.push(OwnedChunk::single(
        b"AUTH",
        0,
        b"TEXT",
        encode_text(&add.author),
    ));
    chunks.push(OwnedChunk::single(
        b"TITL",
        0,
        b"TEXT",
        encode_text(&add.title),
    ));
    chunks.push(OwnedChunk::single(
        b"FLOR",
        0,
        b"FLAG",
        add.floor.to_le_bytes().to_vec(),
    ));
    chunks.push(OwnedChunk::single(
        b"WIZA",
        0,
        b"LONG",
        add.project.to_bytes().to_vec(),
    ));
    chunks.push(OwnedChunk::single(b"INK ", 0, b"INK ", INK.to_vec()));

    chnk::write_aligned(CREATOR, &chunks, |c| {
        matches!(&c.tag.0, b"TEXT" | b"FDPP" | b"FDPC")
    })
}

/// Trechos `(início, fim, grpprl)` cobrindo `[0, len)`: cortados ao texto,
/// sem sobreposição, com os buracos preenchidos sem propriedades.
fn cover<P>(runs: &[Run<P>], len: u32, grpprl: impl Fn(&P) -> Vec<u8>) -> Vec<(u32, u32, Vec<u8>)> {
    let mut out = Vec::with_capacity(runs.len() + 1);
    let mut at = 0;
    for r in runs {
        let (s, e) = (r.start.max(at), r.end.min(len));
        if e <= s {
            continue;
        }
        if s > at {
            out.push((at, s, Vec::new()));
        }
        out.push((s, e, grpprl(&r.props)));
        at = e;
    }
    if at < len {
        out.push((at, len, Vec::new()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::cover;
    use crate::chnk;

    #[test]
    fn cover_fills_gaps_and_clips() {
        let r = |start, end| Run {
            start,
            end,
            props: 1u8,
        };
        let g = |_: &u8| vec![9];
        assert_eq!(cover::<u8>(&[], 3, g), [(0, 3, vec![])]);
        assert_eq!(
            cover(&[r(1, 2), r(1, 5), r(7, 9)], 6, g),
            [
                (0, 1, vec![]),
                (1, 2, vec![9]),
                (2, 5, vec![9]),
                (5, 6, vec![])
            ]
        );
        assert!(cover(&[r(0, 4)], 0, g).is_empty());
    }

    #[test]
    fn new_document_roundtrips() {
        for s in ["", "abc", "um\ndois\n\ntrês ç €"] {
            let d = Document::from_text(s);
            let w = write(&d);
            assert_eq!(w.len() % 512, 0);
            assert_eq!(read(&w).unwrap(), d, "{s:?}");
        }
    }

    #[test]
    fn layout_follows_teste1() {
        let mut d = Document::from_text("abc");
        d.add.title = "TESTE1".into();
        d.add.author = "Ana Teste".into();
        let w = write(&d);
        let f = chnk::parse(&w).unwrap();
        let names: Vec<String> = f.chunks.iter().map(|c| c.tag.to_string()).collect();
        assert_eq!(
            names,
            [
                "TEXT", "FDPP", "FDPC", "DOP ", "BTEP", "BTEC", "PGD ", "FONT", "PRNT", "SELN",
                "THMB", "ADD ", "AUTH", "TITL", "FLOR", "WIZA", "INK "
            ]
        );
        let offs: Vec<usize> = f.chunks[..3].iter().map(|c| c.parts[0].offset).collect();
        assert_eq!(offs, [0x200, 0x400, 0x600]);
        assert_eq!(f.chunks[0].parts[0].data, b"abc\r\r");
    }
}
