mod support;

use formats::{wmf, workshop::Workshop};

#[test]
fn workshop_tables_grammar_and_pictures() {
    let Some(d) = support::read("SHARED/STUFF/WORKSHOP.CHK") else {
        return;
    };
    let w = Workshop::parse(&d).unwrap();

    let sizes: Vec<(u16, usize)> = w.tables.iter().map(|(i, t)| (*i, t.len())).collect();
    assert_eq!(sizes, [(0, 102), (1, 100), (2, 126), (3, 24), (4, 82)]);
    for (_, t) in &w.tables {
        assert_eq!(t.extra_len, 2);
        assert!(t.entries.iter().all(|e| e.sound().is_none()));
    }
    let t = |i| w.table(i).unwrap();
    assert_eq!(t(0).entries[0].text, "A aranha caranguejeira,");
    assert_eq!(t(0).entries[9].text, "A mãe do mafioso,");
    assert_eq!(t(1).entries[10].text, "cuja capacidade ninguém discutia,");
    assert_eq!(t(4).entries[56].text, "uma dobra do tempo.");
    assert_eq!(t(4).entries[81].text, "uma tundra gelada.");

    let g = &w.grammar;
    assert_eq!(g.header, 0);
    let rules: Vec<(&[u16], &str, usize)> = g
        .windows
        .iter()
        .map(|r| (&r.tables[..], r.format.as_str(), r.figures.len()))
        .collect();
    assert_eq!(
        rules,
        [
            (&[0u16, 1][..], "    %s %s", 63),
            (&[2][..], "%s", 17),
            (&[3, 4][..], "%s %s", 28)
        ]
    );
    let codes = |i: usize| -> Vec<u8> { g.windows[i].figures.iter().map(|f| f.code).collect() };
    assert_eq!(codes(0)[..6], [0x03, 0x0b, 0x0d, 0x15, 0x18, 0x25]);
    assert_eq!(*codes(0).last().unwrap(), 0xff);
    assert_eq!(codes(1)[..3], [0x0a, 0x3a, 0x3c]);
    assert!(codes(2).contains(&0x1a));
    assert!(
        g.windows
            .iter()
            .flat_map(|r| &r.figures)
            .all(|f| f.sound.is_none())
    );

    let l = &w.picture_list;
    assert_eq!(l.user_file, "~MYPICS");
    assert_eq!(l.unknown, 0);
    assert_eq!(l.pictures.len(), 28);
    assert!(
        l.pictures
            .iter()
            .enumerate()
            .all(|(i, p)| *p == (i as u16, None))
    );

    assert_eq!(w.pictures.len(), 28);
    let ordered = w.pictures_in_order();
    assert_eq!(ordered.len(), 28);
    assert_eq!(ordered[0].title, "Cara de cão");
    assert_eq!(ordered[24].title, "Terra à vista");
    assert_eq!(ordered[27].title, "O resgate");
    let sizes = [(0usize, 316u32, 226u32), (24, 504, 210), (25, 111, 225)];
    for (i, wd, ht) in sizes {
        let img = wmf::meta_dib(ordered[i].meta).unwrap();
        assert_eq!((img.width, img.height), (wd, ht), "PICT {i}");
    }
    for p in &w.pictures {
        wmf::meta_dib(p.meta).unwrap_or_else(|e| panic!("PICT {}: {e}", p.id));
    }
}
