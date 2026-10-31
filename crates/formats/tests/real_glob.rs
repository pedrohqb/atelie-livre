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
