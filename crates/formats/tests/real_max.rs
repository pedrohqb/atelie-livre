mod support;

use formats::chnk;
use formats::max::{self, Document, FrameKind, ObjRef, Project};
use std::path::PathBuf;

fn samples() -> Option<Vec<PathBuf>> {
    let dir = support::data_dir()?.join("SHARED/USERS/MAX");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("max")))
        .collect();
    files.sort();
    assert_eq!(files.len(), 21);
    Some(files)
}

fn read(rel: &str) -> Option<Document> {
    let d = support::read(rel)?;
    Some(max::read(&d).unwrap())
}

/// Trechos em ordem, sem buracos nem sobreposição, cobrindo `[0, len)`.
fn covers<P>(runs: &[max::Run<P>], len: u32) -> bool {
    let mut at = 0;
    for r in runs {
        if r.start != at || r.end <= r.start {
            return false;
        }
        at = r.end;
    }
    at == len
}

#[test]
fn every_sample_reads_consistently() {
    let Some(files) = samples() else { return };
    for p in &files {
        let name = p.display();
        let doc = max::read(&std::fs::read(p).unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let len = doc.len();
        assert!(len > 0, "{name}");
        assert!(covers(&doc.char_runs, len), "{name}");
        assert!(covers(&doc.para_runs, len), "{name}");
        // um trecho de parágrafo por parágrafo
        let ends: Vec<u32> = doc.para_runs.iter().map(|r| r.end).collect();
        let paras: Vec<u32> = doc.paragraphs().iter().map(|p| p.1).collect();
        assert_eq!(ends, paras, "{name}");
        // folhas contíguas até o fim do texto
        assert_eq!(doc.pages.first().map(|p| p.start), Some(0), "{name}");
        assert!(
            doc.pages.windows(2).all(|w| w[0].end == w[1].start),
            "{name}"
        );
        assert_eq!(doc.pages.last().unwrap().end, len, "{name}");
        // nenhum sprm desconhecido
        assert!(
            doc.char_runs.iter().all(|r| r.props.extra.is_empty()),
            "{name}"
        );
        assert!(
            doc.para_runs.iter().all(|r| r.props.extra.is_empty()),
            "{name}"
        );
        assert!(doc.frames.iter().all(|f| f.extra.is_empty()), "{name}");
        assert!(doc.dop.extra.is_empty(), "{name}");
        // tamanhos entre os 7 do Writer
        for r in &doc.char_runs {
            assert!(max::SIZES_PT.contains(&r.props.size_pt()), "{name}");
        }
        // fonte padrão e referências resolvidas
        assert_eq!(doc.fonts[0].name, max::DEFAULT_FONT, "{name}");
        assert_eq!(doc.fonts[0].ftc, 0, "{name}");
        for r in doc.object_refs() {
            assert!(doc.object(r).is_some(), "{name}: {r:?}");
        }
        assert!(!doc.add.title.is_empty(), "{name}");
        assert_eq!(doc.add.author, "Márcia Muniz", "{name}");
        assert!(!doc.add.encrypted, "{name}");
        assert!(doc.thumbnail_bitmap().is_some(), "{name}");
        assert!(doc.selection.is_some(), "{name}");
    }
}

/// Regravar um documento lido reproduz o arquivo: cabeçalho, índice e todas
/// as partes iguais byte a byte (só o enchimento entre as partes difere, que
/// no original é lixo de memória).
#[test]
fn every_sample_is_rewritten_identically() {
    let Some(files) = samples() else { return };
    for p in &files {
        let name = p.display();
        let orig = std::fs::read(p).unwrap();
        let doc = max::read(&orig).unwrap();
        let w = max::write(&doc);
        assert_eq!(w.len(), orig.len(), "{name}");
        assert_eq!(&w[..32], &orig[..32], "{name}");
        let a = chnk::parse(&orig).unwrap();
        let b = chnk::parse(&w).unwrap();
        assert_eq!(a.chunks.len(), b.chunks.len(), "{name}");
        for (x, y) in a.chunks.iter().zip(&b.chunks) {
            assert_eq!(
                (x.tag, x.id, x.flags, &x.name),
                (y.tag, y.id, y.flags, &y.name),
                "{name}"
            );
            for (px, py) in x.parts.iter().zip(&y.parts) {
                assert_eq!((px.tag, px.offset), (py.tag, py.offset), "{name} {}", x.tag);
                assert_eq!(px.data, py.data, "{name} {} {}", x.tag, x.id);
            }
        }
        assert_eq!(max::read(&w).unwrap(), doc, "{name}");
    }
}

#[test]
fn samp1_card() {
    let Some(doc) = read("SHARED/USERS/MAX/SAMP1.MAX") else {
        return;
    };
    assert_eq!(doc.add.title, "A vida é um osso duro de roer");
    assert_eq!(doc.add.project, Project::Card);
    assert_eq!(doc.add.floor, max::FLOOR_PROJECTS);
    // Times citada com ftc 4, mas o texto usa ftc 1: cai na fonte padrão
    assert_eq!(doc.font_name(4), "Times");
    assert_eq!(doc.font_name(1), max::DEFAULT_FONT);
    // cartão: moldura de página na página 3 e figura desenhada
    assert_eq!(doc.frames[0].kind, FrameKind::PageFrame);
    assert_eq!(doc.frames[0].page, 3);
    assert_eq!(doc.frames[1].object, Some(ObjRef::new(b"PICT", 0)));
    assert_eq!(doc.pages.len(), 4);
    assert_eq!(doc.pages[0].flags, 1);
    assert_eq!(doc.char_runs.len(), 5);
    assert_eq!(doc.char_runs[0].props.size_pt(), 36);
    // "Feliz Dia dos " em preto (0x0c sem 0x10), "Namorados" em vermelho
    let r = &doc.char_runs[1];
    assert_eq!(doc.text_range(r.start, r.end), "Feliz Dia dos ");
    assert!(r.props.bold);
    assert_eq!(
        r.props.text_color(),
        Some(formats::Rgb { r: 0, g: 0, b: 0 })
    );
    let r = &doc.char_runs[2];
    assert_eq!(doc.text_range(r.start, r.end), "Namorados");
    assert_eq!(r.props.size_pt(), 48);
    assert_eq!(
        r.props.text_color(),
        Some(formats::Rgb {
            r: 0xfa,
            g: 0,
            b: 0x39
        })
    );
    assert_eq!(doc.dop.page_number_rect, Some([742, 172, 792, 440]));
}

#[test]
fn samp18_fonts_border_and_two_char_pages() {
    let Some(data) = support::read("SHARED/USERS/MAX/SAMP18.MAX") else {
        return;
    };
    let doc = max::read(&data).unwrap();
    let names: Vec<&str> = doc.fonts.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        ["Lucida Sans", "Stencil", "Lucida Blackletter", "Impact"]
    );
    let f = chnk::parse(&data).unwrap();
    assert_eq!(f.chunks.iter().filter(|c| c.tag.0 == *b"FDPC").count(), 2);
    assert_eq!(doc.dop.border, Some(ObjRef::new(b"BDR ", 72)));
    assert_eq!(doc.dop.border_alt, doc.dop.border);
    assert_eq!(
        (doc.dop.border_outer, doc.dop.border_inner),
        (Some(25), Some(17))
    );
    let bdr = doc.object(ObjRef::new(b"BDR ", 72)).unwrap();
    assert_eq!(bdr.name.as_deref(), Some("Flowers...Modern"));
    // âncoras de objetos na linha
    assert_eq!(doc.char_runs.iter().filter(|r| r.props.anchor).count(), 8);
    assert!(!doc.plain_text().contains('\r'));
    assert_eq!(doc.add.project, Project::None);
    assert_eq!(doc.add.floor, max::FLOOR_OFFICE);
}

#[test]
fn samp6_word_sounds() {
    let Some(doc) = read("SHARED/USERS/MAX/SAMP6.MAX") else {
        return;
    };
    assert_eq!(doc.word_sounds.len(), 2);
    assert_eq!(doc.word_sounds[0].sound, ObjRef::new(b"SND ", 62));
    let last = doc.word_sounds[1];
    assert_eq!(last.sound, ObjRef::new(b"SND ", 803));
    assert_eq!(doc.text_range(last.start, last.end), "Mães.\r");
    assert_eq!(doc.frames.len(), 15);
    assert!(doc.frames.iter().any(|f| f.sound.is_some()));
}

#[test]
fn samp15_newspaper_and_samp17_banner() {
    let Some(doc) = read("SHARED/USERS/MAX/SAMP15.MAX") else {
        return;
    };
    assert!(doc.dop.newspaper);
    assert_eq!(doc.dop.columns, 3);
    assert!(doc.frames.iter().any(|f| f.kind == FrameKind::Splot));
    assert!(doc.object(ObjRef::new(b"SPLE", 0)).is_some());
    let Some(doc) = read("SHARED/USERS/MAX/SAMP17.MAX") else {
        return;
    };
    assert!(doc.dop.banner);
    assert_eq!(doc.add.project, Project::Banner);
    assert_eq!((doc.dop.page_width, doc.dop.page_height), (542, 767));
}
