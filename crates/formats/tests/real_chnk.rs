mod support;

use formats::{chnk, pkbm, wave};
use std::path::{Path, PathBuf};

fn chnk_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            chnk_files(&p, out);
        } else if chnk::is_chnk(&std::fs::read(&p).unwrap()) {
            out.push(p);
        }
    }
}

#[derive(Default, Debug, PartialEq)]
struct Counts {
    images: usize,
    sounds: usize,
    nested: usize,
}

/// Percorre um CHNK (e os aninhados), decodificando toda imagem e todo som.
fn walk(data: &[u8], name: &str) -> Counts {
    let f = chnk::parse(data).unwrap_or_else(|e| panic!("{name}: {e}"));
    let mut c = Counts::default();
    for chunk in &f.chunks {
        for part in &chunk.parts {
            assert!(part.offset + part.data.len() <= data.len());
            if chnk::is_chnk(part.data) {
                let inner = walk(part.data, name);
                c.images += inner.images;
                c.sounds += inner.sounds;
                c.nested += 1 + inner.nested;
            } else if pkbm::is_pkbm(part.data) {
                pkbm::decode(part.data)
                    .unwrap_or_else(|e| panic!("{name} {} {}: {e}", chunk.tag, chunk.id));
                c.images += 1;
            } else if wave::is_riff_wave(part.data) {
                wave::decode(part.data)
                    .unwrap_or_else(|e| panic!("{name} {} {}: {e}", chunk.tag, chunk.id));
                c.sounds += 1;
            }
        }
    }
    c
}

#[test]
fn every_chnk_file_parses_and_decodes() {
    let Some(dir) = support::data_dir() else {
        return;
    };
    let mut files = Vec::new();
    chnk_files(&dir.join("SHARED"), &mut files);
    assert_eq!(files.len(), 54);
    for p in &files {
        walk(&std::fs::read(p).unwrap(), &p.display().to_string());
    }
}

#[test]
fn library_contents() {
    let cases = [
        ("SHARED/STUFF/MAG100.ART", 128, 0, 0),
        ("SHARED/STUFF/A-MAG100.ANI", 169, 0, 16),
        ("SHARED/STUFF/W-MAG100.CHK", 64, 0, 0),
        ("SHARED/STUFF/0MAG100.BOR", 11, 0, 0),
        ("SHARED/STUFF/MAG100.SND", 0, 50, 0),
        ("SHARED/STUFF/MAX100.SND", 0, 63, 0),
    ];
    for (file, images, sounds, nested) in cases {
        let Some(d) = support::read(file) else { return };
        assert_eq!(
            walk(&d, file),
            Counts {
                images,
                sounds,
                nested
            },
            "{file}"
        );
    }
    let Some(d) = support::read("SHARED/STUFF/W-MAG100.CHK") else {
        return;
    };
    assert_eq!(chnk::parse(&d).unwrap().byte_order, chnk::ByteOrder::Big);
}

#[test]
fn names_are_decoded() {
    let Some(d) = support::read("SHARED/STUFF/MAG100.SND") else {
        return;
    };
    let f = chnk::parse(&d).unwrap();
    assert_eq!(f.chunks[0].name.as_deref(), Some("Braquissauro"));
}
