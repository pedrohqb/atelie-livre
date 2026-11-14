//! Gravação de documentos `.MAX`: ida e volta e estrutura igual à de um
//! documento salvo pelo original (`TESTE1.MAX`: "abc" digitado num documento
//! novo do Escritório, autora "Ana Teste", salvo como "TESTE1" no Wine).

use formats::chnk::{self, OwnedChunk};
use formats::max::{
    self, Align, CharProps, Document, DopProps, Effect, Font, Frame, FrameKind, Indent, ObjRef,
    Page, ParaProps, Project, Run, WordSound,
};
use formats::{Rgb, Tag};

fn hex(s: &str) -> Vec<u8> {
    let s: String = s.split_whitespace().collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// Partes de `TESTE1.MAX` (menos a miniatura), na ordem do arquivo:
/// etiqueta do chunk, offset e bytes (das páginas FKP, só o começo).
const TESTE1: [(&str, usize, &str); 16] = [
    ("TEXT", 512, "6162630d0d"),
    (
        "FDPP",
        1024,
        "020001000000000004020000050200000000000000000000",
    ),
    (
        "FDPC",
        1536,
        "010001000000000005020000000000000000000000000000",
    ),
    (
        "DOP ",
        2048,
        "183e53023f1803404800414800424800434800450c00492400",
    ),
    ("BTEP", 2073, "010004000000000000000502000000040000"),
    ("BTEC", 2091, "010004000000000000000502000000060000"),
    ("PGD ", 2109, "01000200ff0000000000050000000000"),
    (
        "FONT",
        2125,
        "12000100ffff0400000002000b4c75636964612053616e7300000100",
    ),
    ("PRNT", 2153, "70727231480048005302180300000000000000000000"),
    (
        "SELN",
        2175,
        "0200ff00030000000300000000000000ffff000000000000000000000000000000000000000000000000000000000000",
    ),
    (
        "ADD ",
        0,
        "000000000000000000000201feff010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000654455354453100746f204e6f766f0000000000000000000000000000000000000009416e61205465737465000000000000000000000000000000000000000000000000",
    ),
    ("AUTH", 0, "416e61205465737465"),
    ("TITL", 0, "544553544531"),
    ("FLOR", 0, "0200"),
    ("WIZA", 0, "00000000"),
    ("INK ", 0, "04000400"),
];

#[test]
fn new_document_has_the_structure_of_teste1() {
    let mut d = Document::from_text("abc");
    d.add.title = "TESTE1".into();
    d.add.author = "Ana Teste".into();
    // no Wine a Lucida Sans (ausente) veio com a família 1
    d.fonts[0].family = 1;
    let w = max::write(&d);
    assert_eq!(w.len(), 3072);
    let f = chnk::parse(&w).unwrap();
    assert_eq!(f.creator, Tag(*b"INK "));
    let tags: Vec<String> = f
        .chunks
        .iter()
        .filter(|c| c.tag.0 != *b"THMB")
        .map(|c| c.tag.to_string())
        .collect();
    let want: Vec<&str> = TESTE1.iter().map(|t| t.0).collect();
    assert_eq!(tags, want);
    for (tag, off, bytes) in TESTE1 {
        let c = f.chunks.iter().find(|c| c.tag.to_string() == tag).unwrap();
        let p = &c.parts[0];
        if off != 0 {
            assert_eq!(p.offset, off, "{tag}");
        }
        let want = hex(bytes);
        if tag == "ADD " {
            // o original deixa o resto de "Documento Novo" depois do título
            let got = max::Add::from_bytes(p.data);
            let orig = max::Add::from_bytes(&want);
            assert_eq!(
                (got.title, got.author, got.floor, got.view, got.field_12),
                (
                    orig.title,
                    orig.author,
                    orig.floor,
                    orig.view,
                    orig.field_12
                )
            );
            assert_eq!(&p.data[..0x3c], &want[..0x3c]);
            continue;
        }
        assert_eq!(&p.data[..want.len()], want.as_slice(), "{tag}");
        if matches!(tag, "FDPP" | "FDPC") {
            assert_eq!(p.data.len(), 512);
            assert!(p.data[want.len()..].iter().all(|&b| b == 0), "{tag}");
        } else {
            assert_eq!(p.data.len(), want.len(), "{tag}");
        }
    }
    // miniatura 144 × 108 de 4 bits logo depois do SELN
    let thmb = f.chunks.iter().find(|c| c.tag.0 == *b"THMB").unwrap();
    assert_eq!(thmb.parts[0].offset, 2223);
    let t = formats::pkbm::decode(thmb.parts[0].data).unwrap();
    assert_eq!((t.width, t.height, t.depth), (144, 108, 4));
    // e o arquivo relido é o documento
    let back = max::read(&w).unwrap();
    assert_eq!(back, d);
    assert_eq!(back.plain_text(), "abc");
}

fn red() -> Rgb {
    Rgb {
        r: 0xfa,
        g: 0,
        b: 0x39,
    }
}

/// Documento com todas as propriedades, objetos e várias páginas FKP.
fn rich() -> Document {
    let body: String = (0..120)
        .map(|k| format!("Parágrafo {k} com ç, é e “aspas”."))
        .collect::<Vec<_>>()
        .join("\n");
    let mut d = Document::from_text(&body);
    let len = d.len();
    // um trecho de caractere por palavra, alternando estilos: mais de uma página FDPC
    let mut runs = Vec::new();
    let mut start = 0;
    for (i, c) in d.text.chars().enumerate() {
        if c == ' ' || i as u32 + 1 == len {
            let k = runs.len();
            let mut p = CharProps {
                bold: k % 2 == 0,
                italic: k % 3 == 0,
                underline: k % 5 == 0,
                effect: (k % 7 == 0).then_some(Effect::from_code((k % 8) as u8 + 1)),
                font: (k % 4 != 0).then_some((k % 4) as u16),
                size: Some(max::SIZES_PT[k % 7]),
                ..CharProps::default()
            };
            if k % 6 == 0 {
                p.set_text_color(Some(max::TEXT_COLORS[k % 16]));
            }
            if k % 11 == 0 {
                p.set_highlight_color(Some(red()));
            }
            p.anchor = k == 3;
            runs.push(Run {
                start,
                end: i as u32 + 1,
                props: p,
            });
            start = i as u32 + 1;
        }
    }
    d.char_runs = runs;
    for (k, r) in d.para_runs.iter_mut().enumerate() {
        r.props = ParaProps {
            align: [Align::Left, Align::Right, Align::Center, Align::Justify][k % 4],
            indent: [
                Indent::None,
                Indent::FirstLine,
                Indent::Hanging,
                Indent::All,
            ][k % 4],
            line_spacing: k % 2 == 1,
            space_after: k % 3 == 1,
            drop_cap: (k % 9 == 0).then_some(3),
            raised_cap: k % 10 == 5,
            new_page: k == 60,
            bullets: k % 7 == 2,
            extra: Vec::new(),
        };
    }
    let mid = d.para_runs[60].start;
    d.pages = vec![
        Page {
            start: 0,
            end: mid,
            flags: 1,
        },
        Page {
            start: mid,
            end: len,
            flags: 4,
        },
    ];
    d.fonts = vec![
        Font::new("Lucida Sans", 0, 2),
        Font::new("Stencil", 1, 5),
        Font::new("Lucida Blackletter", 2, 4),
        Font::new("Impact", 3, 2),
    ];
    d.dop = DopProps {
        page_width: 792,
        page_height: 595,
        margin_top: 108,
        margin_left: 108,
        margin_bottom: 108,
        margin_right: 108,
        columns: 3,
        newspaper: true,
        page_number_style: Some(1),
        page_number_rect: Some([580, 364, 595, 428]),
        border_outer: Some(49),
        border_inner: Some(33),
        border: Some(ObjRef::new(b"BDR ", 0)),
        border_alt: Some(ObjRef::new(b"BDR ", 0)),
        ..DopProps::default()
    };
    let mut pict = Frame::new(
        FrameKind::Picture,
        ObjRef::new(b"PICT", 2),
        254,
        570,
        149,
        150,
    );
    pict.sound = Some(ObjRef::new(b"SND ", 60));
    pict.wrap = Some(3);
    pict.page = 1;
    let mut card = Frame::new(FrameKind::PageFrame, ObjRef::new(b"BDR ", 0), 0, 0, 2, 2);
    card.object = None;
    card.flag_2a = false;
    d.frames = vec![card, pict];
    d.word_sounds = vec![WordSound {
        start: 0,
        end: 12,
        sound: ObjRef::new(b"SND ", 803),
        reserved: [0, 0, 0xb4, 0x4f],
    }];
    let mut bdr = OwnedChunk::single(b"BDR ", 0, b"WBDR", vec![1, 2, 3]);
    bdr.parts.push(chnk::OwnedPart {
        tag: Tag(*b"PKBM"),
        data: vec![4; 9],
    });
    bdr.name = Some("Paw".into());
    d.objects = vec![
        bdr,
        OwnedChunk::single(b"PICT", 2, b"META", vec![7; 100]),
        OwnedChunk::single(b"SND ", 60, b"PCM ", vec![8; 30]),
        OwnedChunk::single(b"SND ", 803, b"PCM ", vec![9; 31]),
    ];
    d.selection = Some((10, 20));
    d.add.project = Project::Card;
    d.add.floor = max::FLOOR_PROJECTS;
    d.add.encrypted = true;
    d.add.password = "Segredo Ç".into();
    d.add.title = "Um título bem comprido, maior que trinta e três letras".into();
    d.add.author = "Márcia Muniz".into();
    d
}

#[test]
fn rich_document_roundtrips() {
    let d = rich();
    let w = max::write(&d);
    let f = chnk::parse(&w).unwrap();
    let n = |t: &[u8; 4]| f.chunks.iter().filter(|c| c.tag.0 == *t).count();
    assert!(n(b"FDPC") > 1, "o teste deve cobrir várias páginas FDPC");
    assert!(n(b"FDPP") > 1, "o teste deve cobrir várias páginas FDPP");
    for c in &f.chunks {
        if matches!(&c.tag.0, b"TEXT" | b"FDPP" | b"FDPC") {
            assert_eq!(c.parts[0].offset % 512, 0, "{}", c.tag);
        }
    }
    let back = max::read(&w).unwrap();
    assert_eq!(back, d);
    // e a segunda gravação é idêntica à primeira
    assert_eq!(max::write(&back), w);
    assert_eq!(back.add.password, "Segredo Ç");
    // o trecho 3 ("ç, ") é âncora: não aparece no texto puro
    assert!(back.plain_text().starts_with("Parágrafo 0 com é e"));
    // PICT 2, SND 60, BDR 0 (citada duas vezes no DOP), SND 803
    assert_eq!(back.object_refs().len(), 4);
}

#[test]
fn writer_repairs_inconsistent_runs_and_pages() {
    let mut d = Document::from_text("abc def");
    d.char_runs = vec![Run {
        start: 2,
        end: 99,
        props: CharProps {
            bold: true,
            ..CharProps::default()
        },
    }];
    d.para_runs.clear();
    d.pages.clear();
    let back = max::read(&max::write(&d)).unwrap();
    let len = d.len();
    assert_eq!(back.char_runs.len(), 2);
    assert_eq!((back.char_runs[0].start, back.char_runs[0].end), (0, 2));
    assert!(back.char_runs[0].props.is_default());
    assert_eq!(
        (back.char_runs[1].end, back.char_runs[1].props.bold),
        (len, true)
    );
    assert_eq!(back.para_runs.len(), 1);
    assert_eq!(
        back.pages,
        [Page {
            start: 0,
            end: len,
            flags: 0
        }]
    );
}

#[test]
fn optional_chunks_are_omitted_and_defaults_apply() {
    let mut d = Document::new();
    d.selection = None;
    d.printer = None;
    d.thumbnail = None;
    let w = max::write(&d);
    let f = chnk::parse(&w).unwrap();
    for t in [b"SELN", b"PRNT", b"THMB", b"FRAM", b"SNDT"] {
        assert!(!f.chunks.iter().any(|c| c.tag.0 == *t));
    }
    assert_eq!(max::read(&w).unwrap(), d);
    // sem DOP, ADD e trechos: padrões
    let keep: Vec<OwnedChunk> = f
        .chunks
        .iter()
        .filter(|c| matches!(&c.tag.0, b"TEXT"))
        .map(OwnedChunk::from)
        .collect();
    let bare = max::read(&chnk::write(max::CREATOR, &keep)).unwrap();
    assert_eq!(bare.dop, DopProps::default());
    assert_eq!(bare.add, max::Add::default());
    assert!(bare.char_runs.is_empty() && bare.pages.is_empty());
    assert_eq!(bare.plain_text(), "");
}

#[test]
fn read_rejects_other_files_and_survives_damage() {
    assert!(max::read(b"nada").is_err());
    let other = chnk::write(Tag(*b"SPLT"), &[]);
    assert!(max::read(&other).is_err());
    let no_text = chnk::write(max::CREATOR, &[]);
    assert!(max::read(&no_text).is_err());
    let mut d = rich();
    d.text = d.text.chars().take(300).collect();
    let w = max::write(&d);
    for n in (0..w.len()).step_by(61) {
        let _ = max::read(&w[..n]);
    }
    for i in (0..w.len()).step_by(5) {
        for x in [0x00, 0xff, 0x80] {
            let mut c = w.clone();
            c[i] = x;
            let _ = max::read(&c);
        }
    }
}
