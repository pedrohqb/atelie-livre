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

#[test]
fn every_sound_decodes() {
    use formats::wave;
    for (file, count) in [("ARTIST/ARTIST.SND", 131), ("WRITER/WRITER.SND", 122)] {
        let Some(d) = support::read(file) else { return };
        let ne = ne::parse(&d).unwrap();
        let mut n = 0;
        for r in ne.of_kind("SND") {
            let pcm =
                wave::decode(r.data).unwrap_or_else(|e| panic!("{file} SND {:#x}: {e}", r.id));
            assert_eq!(pcm.channels, 1, "{file} {:#x}", r.id);
            assert!(
                matches!(pcm.sample_rate, 11025 | 11127 | 11128 | 44100),
                "{file} {:#x}: {} Hz",
                r.id,
                pcm.sample_rate
            );
            assert!(!pcm.samples.is_empty());
            n += 1;
        }
        assert_eq!(n, count);
    }
}

#[test]
fn artist_sounds_loop_points() {
    use formats::wave;
    let Some(d) = support::read("ARTIST/ARTIST.SND") else {
        return;
    };
    let ne = ne::parse(&d).unwrap();
    let points = |id| {
        wave::decode(ne.find("SND", id).unwrap().data)
            .unwrap()
            .loop_points
    };
    assert_eq!(points(0x21e), Some((0, 6933)));
    assert_eq!(points(0x229), Some((6348, 18557)));
    assert_eq!(points(0x213), None);
    let looping = ne
        .of_kind("SND")
        .filter(|r| wave::decode(r.data).unwrap().loop_points.is_some())
        .count();
    assert_eq!(looping, 21);
}

#[test]
fn standard_windows_resources_decode() {
    use formats::winres;
    for file in ["ARTIST.EXE", "WRITER.EXE"] {
        let Some(d) = support::read(file) else { return };
        let ne = ne::parse(&d).unwrap();
        for r in ne.of_kind("ICON") {
            let img = winres::decode_icon(r.data)
                .unwrap_or_else(|e| panic!("{file} ICON {:#x}: {e}", r.id));
            assert_eq!((img.width, img.height), (32, 32));
        }
        for r in ne.of_kind("CURSOR") {
            let (img, (x, y)) = winres::decode_cursor(r.data)
                .unwrap_or_else(|e| panic!("{file} CURSOR {:#x}: {e}", r.id));
            assert_eq!((img.width, img.height), (32, 32));
            assert!(x < 32 && y < 32);
        }
        for r in ne.of_kind("BITMAP") {
            winres::decode_bitmap(r.data)
                .unwrap_or_else(|e| panic!("{file} BITMAP {:#x}: {e}", r.id));
        }
        for r in ne.of_kind("STRING") {
            winres::parse_string_table(r.id, r.data)
                .unwrap_or_else(|e| panic!("{file} STRING {:#x}: {e}", r.id));
        }
    }
    let Some(d) = support::read("ARTIST.EXE") else {
        return;
    };
    let ne = ne::parse(&d).unwrap();
    assert_eq!(ne.of_kind("ICON").count(), 48);
    let strings = winres::parse_string_table(1, ne.find("STRING", 1).unwrap().data).unwrap();
    assert_eq!(strings[0], (1, "Erro".to_string()));
    assert!(strings[1].1.starts_with("Você precisa"));
}

#[test]
fn writer_menu_dialogs_and_accelerators() {
    use formats::winres_ui::{self, MenuItem};
    let Some(d) = support::read("WRITER.EXE") else {
        return;
    };
    let ne = ne::parse(&d).unwrap();

    let menu = winres_ui::parse_menu(ne.find("MENU", 0x3e8).unwrap().data).unwrap();
    let bar: Vec<String> = menu.items.iter().map(MenuItem::label).collect();
    assert_eq!(bar, ["Arquivo", "Editar", "Ferramentas"]);
    let salvar = menu.find(0x1f43).unwrap();
    assert_eq!(salvar.label(), "Salvar Como...");
    assert_eq!(salvar.shortcut(), Some("Ctrl+S"));
    assert_eq!(
        menu.find(0x1f5c).unwrap().label(),
        "Sobre o Creative Writer..."
    );
    assert_eq!(
        menu.find(0x1f50).unwrap().label(),
        "Redefinir Máquina de Idéias..."
    );
    // Voltar ao Creative Writer e Sair têm o mesmo comando
    assert_eq!(
        menu.commands()
            .iter()
            .filter(|(_, c)| matches!(c, MenuItem::Command { id: 0x1f46, .. }))
            .count(),
        2
    );

    let acc = winres_ui::parse_accelerators(ne.find("ACCELERATOR", 0x3e9).unwrap().data).unwrap();
    let keys: Vec<(char, u16)> = acc
        .iter()
        .map(|a| (a.ctrl_letter().unwrap(), a.command))
        .collect();
    assert_eq!(
        keys,
        [
            ('N', 0x1f41),
            ('O', 0x1f42),
            ('S', 0x1f43),
            ('Q', 0x1f46),
            ('Z', 0x1f47),
            ('X', 0x1f49),
            ('C', 0x1f4a),
            ('V', 0x1f4b)
        ]
    );

    let dialogs = |id| winres_ui::parse_dialog(ne.find("DIALOG", id).unwrap().data).unwrap();
    let sobre = dialogs(0x7d0);
    assert_eq!(sobre.caption, "Creative Writer");
    assert_eq!(sobre.font, Some((8, "MS Sans Serif".into())));
    assert_eq!(
        sobre.static_texts(),
        [
            "Versão 1.0",
            "20 de outubro de 1995",
            "® 1993-1995  Microsoft Corporation. Direitos reservados.",
            "Olhe a barra de menu para ver comandos úteis."
        ]
    );
    assert_eq!(sobre.buttons(), [(1, "OK".to_string())]);

    let som = dialogs(0xbb8);
    assert_eq!(som.caption, "Controle de Som");
    assert_eq!(
        som.buttons(),
        [
            (0xbba, "Ligar sons".to_string()),
            (0xbb9, "Desligar sons".to_string())
        ]
    );
    assert!(som.control(0xbb9).unwrap().is_default_button());

    let decod = dialogs(0x1388);
    assert_eq!(decod.caption, "Decodificar o Documento");
    assert!(
        decod
            .static_texts()
            .contains(&"Detetive McZee para salvar o dia")
    );
    assert_eq!(decod.buttons(), [(1, "Valeu, McZee".to_string())]);

    let ideias = dialogs(0x1770);
    assert_eq!(ideias.caption, "Redefinir a Máquina de Idéias");
    assert_eq!(
        ideias.static_texts()[1],
        "Remove todas as entradas\nque você colocou."
    );
    assert_eq!(
        ideias.buttons(),
        [
            (2, "Deixar como está".to_string()),
            (3, "Apagar".to_string())
        ]
    );

    let impr = dialogs(0x1b58);
    assert_eq!(impr.caption, "Opções de Impressão");
    assert_eq!((impr.cx, impr.cy), (230, 112));
    assert_eq!(
        impr.buttons(),
        [
            (0x1b5a, "Opções locais".to_string()),
            (0x1b59, "Opções globais".to_string()),
            (2, "Deixar como está".to_string())
        ]
    );
}

#[test]
fn artist_menu_and_dialogs() {
    use formats::winres_ui;
    let Some(d) = support::read("ARTIST.EXE") else {
        return;
    };
    let ne = ne::parse(&d).unwrap();
    let named = |kind: &str, name: &str| {
        ne.of_kind(kind)
            .find(|r| r.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{kind} {name}"))
            .data
    };
    let menu = winres_ui::parse_menu(named("MENU", "ADULTMENU")).unwrap();
    assert_eq!(menu.items.len(), 3);
    assert_eq!(menu.find(0x1f).unwrap().label(), "Salvar Como BMP...");
    assert_eq!(menu.find(0x28).unwrap().label(), "Sobre o Fine Artist...");

    let sobre = winres_ui::parse_dialog(named("DIALOG", "ABOUTSPLAT")).unwrap();
    assert_eq!(sobre.caption, "Fine Artist");
    assert_eq!(sobre.static_texts()[0], "Versão 1.0");
    let tt = winres_ui::parse_dialog(named("DIALOG", "TTDIALOG")).unwrap();
    assert!(tt.static_texts()[0].starts_with("Não consigo achar as fontes TrueType."));
    assert_eq!(
        tt.buttons(),
        [(1, "Continuar".to_string()), (2, "Sair".to_string())]
    );
    for name in ["PRINTOPTIONS", "SAVEAS", "SOUNDDIALOG"] {
        let d = winres_ui::parse_dialog(named("DIALOG", name)).unwrap();
        assert!(!d.controls.is_empty(), "{name}");
    }
}
