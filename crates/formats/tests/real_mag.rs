mod support;

use formats::chnk::{self, OwnedChunk};
use formats::{mag, pkbm};
use std::path::PathBuf;

fn samples() -> Option<Vec<PathBuf>> {
    let dir = support::data_dir()?.join("SHARED/USERS/MAGGIE");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("mag") || e.eq_ignore_ascii_case("ptm"))
        })
        .collect();
    files.sort();
    assert_eq!(files.len(), 18);
    Some(files)
}

#[test]
fn every_sample_document_reads() {
    let Some(files) = samples() else { return };
    for p in &files {
        let d = std::fs::read(p).unwrap();
        let pic = mag::read(&d).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        assert_eq!(pic.palette.len(), 256, "{}", p.display());
        assert_eq!(
            pic.pixels.len(),
            pic.width as usize * pic.height as usize,
            "{}",
            p.display()
        );
        assert!(!pic.title.is_empty(), "{}", p.display());
        if p.extension().unwrap().eq_ignore_ascii_case("mag")
            && !p.ends_with("SAMP12.MAG")
            && !p.ends_with("SAMP13.MAG")
            && !p.ends_with("SAMP14.MAG")
            && !p.ends_with("SAMP15.MAG")
            && !p.ends_with("SAMP3.MAG")
        {
            assert_eq!((pic.width, pic.height), (594, 347), "{}", p.display());
        }
    }
}

#[test]
fn samp6_has_known_title_and_author() {
    let Some(d) = support::read("SHARED/USERS/MAGGIE/SAMP6.MAG") else {
        return;
    };
    let pic = mag::read(&d).unwrap();
    assert_eq!(pic.title, "Fundo com Praia");
    assert_eq!(pic.author, "Lisa");
    assert_eq!((pic.width, pic.height), (594, 347));
    // a paleta do CTBL é a mesma embutida na miniatura
    let f = chnk::parse(&d).unwrap();
    let thumb = f
        .chunks
        .iter()
        .find(|c| c.tag.0 == *b"THMB" && c.id == 0)
        .unwrap();
    let t = pkbm::decode(thumb.parts[0].data).unwrap();
    assert_eq!(t.palette.as_deref(), Some(pic.palette.as_slice()));
    assert_eq!((t.width, t.height), (144, 84));
}

#[test]
fn encoding_real_pictures_roundtrips() {
    let Some(files) = samples() else { return };
    for p in &files {
        let d = std::fs::read(p).unwrap();
        let f = chnk::parse(&d).unwrap();
        for c in &f.chunks {
            for part in c.parts.iter().filter(|p| pkbm::is_pkbm(p.data)) {
                let b = pkbm::decode(part.data).unwrap();
                let e = pkbm::encode(&b);
                assert_eq!(
                    pkbm::decode(&e).unwrap(),
                    b,
                    "{} {} {}",
                    p.display(),
                    c.tag,
                    c.id
                );
                // o cabeçalho segue o das imagens dos documentos
                assert_eq!(&e[..16], &part.data[..16]);
                assert_eq!(&e[22..32], &part.data[22..32]);
                assert_eq!(&e[38..42], &part.data[38..42]);
            }
        }
    }
}

#[test]
fn reserializing_samples_parses_equal() {
    let Some(files) = samples() else { return };
    for p in &files {
        let d = std::fs::read(p).unwrap();
        let f = chnk::parse(&d).unwrap();
        let owned: Vec<OwnedChunk> = f.chunks.iter().map(OwnedChunk::from).collect();
        let w = chnk::write(f.creator, &owned);
        let g = chnk::parse(&w).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        assert_eq!(g.creator, f.creator);
        assert_eq!(g.version, f.version);
        let back: Vec<OwnedChunk> = g.chunks.iter().map(OwnedChunk::from).collect();
        assert_eq!(back, owned, "{}", p.display());
        assert_eq!(w.len() % 512, 0);
    }
}

#[test]
fn writing_a_sample_picture_reads_back_equal() {
    let Some(d) = support::read("SHARED/USERS/MAGGIE/SAMP6.MAG") else {
        return;
    };
    let pic = mag::read(&d).unwrap();
    let w = mag::write(&pic);
    assert_eq!(mag::read(&w).unwrap(), pic);
    // sem os objetos soltos (SOBJ/COBJ), mas o resto da estrutura igual à do original
    let f = chnk::parse(&d).unwrap();
    let g = chnk::parse(&w).unwrap();
    let kept: Vec<(formats::Tag, u16)> = f
        .chunks
        .iter()
        .filter(|c| !matches!(&c.tag.0, b"SOBJ" | b"COBJ"))
        .map(|c| (c.tag, c.id))
        .collect();
    let got: Vec<(formats::Tag, u16)> = g.chunks.iter().map(|c| (c.tag, c.id)).collect();
    assert_eq!(got, kept);
    for (a, b) in f
        .chunks
        .iter()
        .filter(|c| {
            matches!(
                &c.tag.0,
                b"CTBL" | b"FLOR" | b"WIZA" | b"FWLS" | b"FLS " | b"TITL" | b"AUTH" | b"SDOC"
            )
        })
        .zip(g.chunks.iter().filter(|c| {
            matches!(
                &c.tag.0,
                b"CTBL" | b"FLOR" | b"WIZA" | b"FWLS" | b"FLS " | b"TITL" | b"AUTH" | b"SDOC"
            )
        }))
    {
        assert_eq!(a.parts[0].data, b.parts[0].data, "{}", a.tag);
    }
}
