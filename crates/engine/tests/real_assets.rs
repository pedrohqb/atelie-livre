use engine::assets::{Assets, CursorRef, ImageRef, ItemKind, Program, SoundRef};
use std::path::{Path, PathBuf};

fn data_dir() -> Option<PathBuf> {
    match std::env::var_os("ATELIE_DATA") {
        Some(d) => Some(PathBuf::from(d)),
        None => {
            eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
            None
        }
    }
}

fn copy_lower(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let to = dst.join(e.file_name().to_string_lossy().to_lowercase());
        if e.path().is_dir() {
            copy_lower(&e.path(), &to);
        } else {
            std::fs::copy(e.path(), to).unwrap();
        }
    }
}

#[test]
fn opens_real_data() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let cat = a.catalog();
    assert_eq!(cat[0].name, "ARTIST.PIC");
    assert_eq!(cat[0].items.len(), 2468);
    for name in [
        "WRITER.PIC",
        "MAG100.ART",
        "Ícones",
        "Cursores",
        "ARTIST.SND",
        "WRITER.SND",
    ] {
        assert!(cat.iter().any(|s| s.name == name), "{name}");
    }
    let snd = cat.iter().find(|s| s.name == "MAG100.SND").unwrap();
    assert_eq!(snd.items.len(), 50);
    assert!(
        snd.items
            .iter()
            .all(|i| matches!(i.kind, ItemKind::Sound(_)))
    );
}

#[test]
fn splash_and_street_are_full_screen() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    for id in [0x6ca7, 0x6d01] {
        let img = a.image(&ImageRef::Pic(Program::Artist, id));
        assert_eq!((img.width, img.height), (640, 480), "{id:#x}");
    }
}

#[test]
fn default_cursor_has_hotspot_inside() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let (img, (x, y)) = a.cursor(&CursorRef(Program::Artist, 1)).unwrap();
    assert_eq!((img.width, img.height), (32, 32));
    assert!(x < 32 && y < 32);
}

#[test]
fn sound_0x209_decodes() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let pcm = a.sound(&SoundRef::Snd(Program::Artist, 0x209)).unwrap();
    assert!(!pcm.samples.is_empty());
    assert!(a.sound(&SoundRef::Snd(Program::Artist, 0x7777)).is_none());
}

#[test]
fn missing_image_gives_placeholder() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let img = a.image(&ImageRef::Pic(Program::Artist, 0x0001));
    assert_eq!((img.width, img.height), (32, 32));
    let lib = a.image(&ImageRef::Library {
        file: "NAOEXISTE.ART".into(),
        index: 0,
    });
    assert_eq!((lib.width, lib.height), (32, 32));
}

#[test]
fn library_images_decode_with_document_palette() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let img = a.image(&ImageRef::Library {
        file: "MAG100.ART".into(),
        index: 0,
    });
    assert!(img.width > 1 && img.height > 1);
}

#[test]
fn artist_only_install_lists_only_artist() {
    let Some(d) = data_dir() else { return };
    let tmp = tempfile::tempdir().unwrap();
    std::fs::copy(d.join("ARTIST.EXE"), tmp.path().join("artist.exe")).unwrap();
    copy_lower(&d.join("ARTIST"), &tmp.path().join("artist"));
    let a = Assets::open(tmp.path()).unwrap();
    let names: Vec<&str> = a.catalog().iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"ARTIST.PIC"));
    assert!(!names.contains(&"WRITER.PIC") && !names.contains(&"WRITER.SND"));
    let img = a.image(&ImageRef::Pic(Program::Artist, 0x6ca7));
    assert_eq!((img.width, img.height), (640, 480));
}

#[test]
fn sign_in_sky_matches_original_colors() {
    // céu da tela de entrada (0x4139), conferido numa captura do programa original no Wine
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let img = a.image(&ImageRef::Pic(Program::Artist, 0x4139));
    let mut counts = std::collections::HashMap::new();
    for p in img.rgba.chunks(4) {
        *counts.entry([p[0], p[1], p[2]]).or_insert(0usize) += 1;
    }
    let top = counts.into_iter().max_by_key(|(_, n)| *n).unwrap().0;
    assert_eq!(top, [136, 136, 230]);
}

#[test]
fn street_grays_match_original() {
    // na captura da rua (0x6d01) no original, o cinza médio (140) cobre 54885 pixels
    // e o claro (190) 18784; trocados, a calçada sai clara demais
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let img = a.image(&ImageRef::Pic(Program::Artist, 0x6d01));
    let count = |c: [u8; 3]| img.rgba.chunks(4).filter(|p| p[..3] == c).count();
    assert_eq!(count([140, 140, 140]), 54885);
    assert_eq!(count([190, 190, 190]), 18784);
}
