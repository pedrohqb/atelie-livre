mod support;

use formats::glob;

#[test]
fn pic_entry_counts_and_signatures() {
    for (file, count) in [("ARTIST/ARTIST.PIC", 2468), ("WRITER/WRITER.PIC", 2311)] {
        let Some(d) = support::read(file) else { return };
        let entries = glob::parse(&d).unwrap();
        assert_eq!(entries.len(), count, "{file}");
        for e in &entries {
            assert_eq!(&e.data[..8], b"\x00\x01\x00\x00PKBM", "{file} id {}", e.id);
        }
    }
}

#[test]
fn every_pic_image_decodes() {
    use formats::pkbm;
    for file in ["ARTIST/ARTIST.PIC", "WRITER/WRITER.PIC"] {
        let Some(d) = support::read(file) else { return };
        for e in glob::parse(&d).unwrap() {
            let b = pkbm::decode(e.data).unwrap_or_else(|err| panic!("{file} id {}: {err}", e.id));
            assert_eq!(b.pixels.len(), b.width as usize * b.height as usize);
            assert!(matches!(b.depth, 1 | 4), "{file} id {}", e.id);
            assert!(b.pixels.iter().all(|&p| (p as u32) < (1 << b.depth)));
            assert!(b.palette.is_none());
        }
    }
}

#[test]
fn known_image_dimensions() {
    use formats::pkbm;
    let Some(d) = support::read("ARTIST/ARTIST.PIC") else {
        return;
    };
    let entries = glob::parse(&d).unwrap();
    let get = |id: u16| pkbm::decode(entries.iter().find(|e| e.id == id).unwrap().data).unwrap();
    let b = get(590);
    assert_eq!((b.width, b.height, b.depth), (112, 37, 1));
    let b = get(32141);
    assert_eq!((b.width, b.height, b.depth), (103, 144, 4));
}
