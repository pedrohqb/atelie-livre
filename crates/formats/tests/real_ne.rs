mod support;

use formats::ne;

fn assert_counts(file: &str, expected: &[(&str, usize)]) {
    let Some(d) = support::read(file) else { return };
    let ne = ne::parse(&d).unwrap();
    for (label, n) in expected {
        assert_eq!(ne.of_kind(label).count(), *n, "{file}: {label}");
    }
}

#[test]
fn artist_exe_resources() {
    assert_counts(
        "ARTIST.EXE",
        &[
            ("ICON", 48),
            ("GROUP_ICON", 16),
            ("CURSOR", 90),
            ("GROUP_CURSOR", 90),
            ("WHAT", 34),
            ("ATAG", 162),
            ("SCRD", 14),
            ("STR#", 17),
            ("STR", 3),
            ("SWND", 36),
            ("DSPC", 27),
            ("BTNW", 37),
            ("LOGPALETTE", 2),
            ("TMEN", 3),
        ],
    );
}

#[test]
fn writer_exe_resources() {
    assert_counts(
        "WRITER.EXE",
        &[
            ("CURSOR", 35),
            ("BITMAP", 6),
            ("ATAG", 155),
            ("WHAT", 32),
            ("SCRD", 14),
            ("STR#", 14),
            ("LOGPALETTE", 2),
            ("TMEN", 2),
        ],
    );
}

#[test]
fn sound_dlls() {
    assert_counts("ARTIST/ARTIST.SND", &[("SND", 131)]);
    assert_counts("WRITER/WRITER.SND", &[("SND", 122)]);
}

#[test]
fn artist_modules() {
    let Some(d) = support::read("ARTIST.EXE") else {
        return;
    };
    let ne = ne::parse(&d).unwrap();
    assert_eq!(
        ne.modules,
        [
            "KERNEL", "GDI", "WIN87EM", "USER", "KEYBOARD", "COMMDLG", "MMSYSTEM", "TOOLHELP"
        ]
    );
}

#[test]
fn artist_palettes() {
    use formats::{Rgb, palette::parse_logpalette};
    let Some(d) = support::read("ARTIST.EXE") else {
        return;
    };
    let ne = ne::parse(&d).unwrap();
    let pal16 = parse_logpalette(ne.find("LOGPALETTE", 4).unwrap().data).unwrap();
    let pal256 = parse_logpalette(ne.find("LOGPALETTE", 8).unwrap().data).unwrap();
    assert_eq!(pal16.len(), 16);
    assert_eq!(pal256.len(), 256);
    assert_eq!(
        pal16[0],
        Rgb {
            r: 255,
            g: 255,
            b: 255
        }
    );
    assert_eq!(pal16[1], Rgb { r: 0, g: 0, b: 0 });
}

#[test]
fn artist_texts() {
    use formats::macres::{parse_cstring, parse_str_list};
    let Some(d) = support::read("ARTIST.EXE") else {
        return;
    };
    let ne = ne::parse(&d).unwrap();
    assert_eq!(
        parse_str_list(ne.find("STR#", 0x206).unwrap().data).unwrap()[0],
        "Pablo Graphos"
    );
    assert!(
        parse_cstring(ne.find("STR", 0x1005).unwrap().data)
            .unwrap()
            .starts_with("Eu não posso salvar")
    );
}

#[test]
fn all_str_lists_decode() {
    use formats::macres::parse_str_list;
    for file in ["ARTIST.EXE", "WRITER.EXE"] {
        let Some(d) = support::read(file) else { return };
        let ne = ne::parse(&d).unwrap();
        for r in ne.of_kind("STR#") {
            parse_str_list(r.data).unwrap_or_else(|e| panic!("{file} STR# {:#x}: {e}", r.id));
        }
    }
}

#[test]
fn pattern_lists_fit() {
    use formats::macres::parse_pattern_list;
    for file in ["ARTIST.EXE", "WRITER.EXE"] {
        let Some(d) = support::read(file) else { return };
        let ne = ne::parse(&d).unwrap();
        for r in ne.of_kind("PAT#") {
            let pats = parse_pattern_list(r.data).unwrap();
            assert!(2 + 16 * pats.len() <= r.data.len(), "{file}");
        }
    }
    let Some(d) = support::read("ARTIST.EXE") else {
        return;
    };
    let ne = ne::parse(&d).unwrap();
    let pats = parse_pattern_list(ne.find("PAT#", 0x80).unwrap().data).unwrap();
    assert_eq!(pats.len(), 43);
    assert_eq!(pats[1], [0x00, 0x44, 0x00, 0x11, 0x00, 0x44, 0x00, 0x11]);
    assert_eq!(pats[3], [0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa]);
}
