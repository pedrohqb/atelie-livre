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
