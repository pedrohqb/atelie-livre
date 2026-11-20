mod support;

use formats::maca::{self, Maca};

/// A tabela da Maçã do `WRITER.PIC` PT-BR (`docs/formats/maca.md`): 1866
/// registros, todos no balde do seu hash, e as palavras medidas no original.
#[test]
fn writer_pic_rebus_table() {
    let Some(pic) = support::read("WRITER/WRITER.PIC") else {
        return;
    };
    let m = Maca::read(&pic).unwrap();
    assert_eq!(m.len(), 1866);
    let figures: std::collections::BTreeSet<u8> = m.entries().map(|e| e.2).collect();
    assert_eq!(figures.len(), 253);
    assert!(!figures.contains(&126) && !figures.contains(&193));
    for (word, n) in [
        ("gato", Some(0x25)),
        ("Gato", Some(0x25)),
        ("GATO", Some(0x25)),
        ("gatos", Some(0x25)),
        ("aeronave", Some(1)),
        ("sono", Some(0x0c)),
        ("pique-nique", Some(0x99)),
        ("bicicleta", Some(0x0f)),
        ("parque", Some(0xeb)),
        ("café", Some(0xa9)),
        ("bzzz", Some(0x40)),
        ("abelha", None),
        ("maç", None),
    ] {
        assert_eq!(m.lookup(word), n, "{word}");
    }
    assert_eq!(m.lookup_all("gato"), vec![0x25, 0xc6]);
    assert_eq!(m.lookup_all("sono"), vec![0x0c, 0x61, 0x7b]);
    assert_eq!(maca::color_id(0x25), Some(0x0049));
    // o ARTIST.PIC não tem tabela
    if let Some(art) = support::read("ARTIST/ARTIST.PIC") {
        assert!(Maca::read(&art).is_err());
    }
}
