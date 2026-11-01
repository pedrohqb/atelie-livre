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

#[test]
fn patterns_from_real_exe_has_43() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let p = a.patterns(Program::Artist);
    assert_eq!(p.len(), 43);
    assert_eq!(p[0], [0; 8]);
    assert_eq!(p[3], [0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa]);
}

#[test]
fn paint_frame_matches_original_colors() {
    // valores medidos numa captura da tela de pintura do original no Wine
    use engine::scene::Scene;
    use engine::scenes::Paint;
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let p = Paint::new(None, &a);
    let mut f = engine::Frame::new(640, 480);
    p.draw(&mut f, &a);
    assert_eq!(f.pixel(13, 131), [255, 213, 0, 255]); // 3ª cor
    assert_eq!(f.pixel(13, 446), [0, 0, 0, 255]); // preto, a última
    assert_eq!(f.pixel(300, 300), [255, 255, 255, 255]); // quadro branco
    assert_eq!(f.pixel(530, 30), [100, 100, 100, 255]); // barra de ferramentas
    assert_eq!(f.pixel(530, 80), [140, 140, 140, 255]); // faixa de opções
    assert_eq!(f.pixel(637, 300), [255, 140, 50, 255]); // moldura laranja
}

#[test]
fn sticker_frames_have_masks_and_thumbnails() {
    use engine::scenes::stickers::{ANIMALS, THUMBS};
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    for frames in ANIMALS {
        for &id in frames {
            let img = a.image(&ImageRef::Pic(Program::Artist, id));
            let mask = a.image(&ImageRef::Pic(Program::Artist, id + 6));
            assert_eq!(
                (img.width, img.height),
                (mask.width, mask.height),
                "{id:#x}"
            );
            // a placeholder é um quadriculado magenta: um quadro real não tem magenta puro
            assert!(
                !img.rgba.chunks(4).any(|p| p[..3] == [255, 0, 255]),
                "{id:#x}"
            );
        }
    }
    for id in THUMBS {
        let t = a.image(&ImageRef::Pic(Program::Artist, id));
        assert!(t.width <= 30 && t.height <= 30, "{id:#x}");
    }
}

#[test]
fn clip_art_rolls_are_the_28_carousels() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let rolls = engine::clipart::load_rolls(&a);
    let ids: Vec<u16> = rolls.iter().map(|r| r.id).collect();
    assert_eq!(ids, (0..28).collect::<Vec<u16>>());
    // rolos conferidos no original: 0 começa pelo skate (128×46), 3 é dinossauro
    let skate = rolls[0].items[0].art().unwrap();
    assert_eq!((skate.width, skate.height), (128, 46));
    assert_eq!(rolls[0].name, "Hobby");
    for r in &rolls {
        assert!(r.icon.is_some(), "rolo {}", r.id);
        for it in &r.items {
            assert!(it.art().is_some(), "rolo {}", r.id);
            assert!(it.preview.is_some(), "rolo {}", r.id);
        }
    }
}

#[test]
fn clip_art_sticker_is_placed_at_full_size() {
    use engine::scene::{Input, MouseButton, Scene};
    use engine::scenes::Paint;
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let mut p = Paint::new(None, &a);
    let c = |x, y| Input::Click {
        button: MouseButton::Left,
        x,
        y,
    };
    // ferramenta de adesivos, rolo 2 (skate), primeira casa da tira amarela
    p.update(&[c(129, 30), c(100, 80)], 0.0, &a);
    assert_eq!(p.stickers().roll(), 1);
    assert!(p.stickers().film_open());
    p.update(&[c(106, 120)], 0.0, &a);
    assert!(p.stickers().holding());
    p.update(&[c(330, 280)], 0.0, &a);
    // o adesivo é um objeto sobre o quadro: medir na composição
    let flat = p.composed();
    let px = &flat.px;
    let w = flat.w as usize;
    let (mut x0, mut x1) = (usize::MAX, 0);
    for (k, v) in px.iter().enumerate() {
        if *v != 0 {
            x0 = x0.min(k % w);
            x1 = x1.max(k % w);
        }
    }
    // o skate tem 128 pixels de largura e fica centrado no clique
    assert_eq!(x1 - x0 + 1, 128);
    assert_eq!(x0 as i32, 330 - 23 - 64);
    // página seguinte de rolos
    p.update(&[c(510, 95)], 0.0, &a);
    assert_eq!(p.stickers().page(), 1);
}

#[test]
fn pablo_font_loads_three_sizes() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    for (pt, h) in [(6, 12), (8, 13), (16, 21)] {
        let f = a.font(pt).expect("fonte");
        assert_eq!(f.height, h);
        assert_eq!(f.name, "Pablo Graphos");
    }
    // "Quadro Novo" mede 65 pixels na barra do original
    assert_eq!(a.font(8).unwrap().width("Quadro Novo"), 65);
}

#[test]
fn words_font_16pt_renders_oi_wider_than_10px() {
    use engine::paint::{Canvas, text_size};
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let font = a.font(16).expect("fonte de 16 pontos");
    let (w, h) = text_size(Some(&font), "Oi");
    assert!(w > 10, "largura {w}");
    assert_eq!(h, 21);
    let mut c = Canvas::new(100, 40);
    c.draw_text(Some(&font), 5, 5, "Oi", 1);
    let cols: std::collections::BTreeSet<usize> =
        c.px.iter()
            .enumerate()
            .filter(|(_, v)| **v == 1)
            .map(|(k, _)| k % 100)
            .collect();
    let span = cols.iter().max().unwrap() - cols.iter().min().unwrap() + 1;
    assert!(span > 10, "glifos ocupam {span} colunas");
}

#[test]
fn ani_animations_have_scripts_and_images() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let anims = a.animations("A-MAG100.ANI");
    assert_eq!(anims.len(), 16);
    for (id, script, images) in &anims {
        let s = formats::atag::parse(script).unwrap_or_else(|e| panic!("ATAG {id}: {e}"));
        assert!(!s.steps.is_empty(), "ATAG {id}");
        assert!(images.len() >= 2, "ATAG {id}");
        // cada célula aponta para uma imagem existente, com máscara do mesmo tamanho
        for c in &s.cells {
            let img = a.image(&images[c.image as usize]);
            let mask = a.image(&images[c.mask as usize]);
            assert_eq!(
                (img.width, img.height),
                (mask.width, mask.height),
                "ATAG {id}"
            );
        }
    }
}

#[test]
fn ani_script_reproduces_recorded_swordfish() {
    // gravação do original: peixe-espada (bicho 1) clicado em (330,280); aos
    // 10 quadros de 1/15 s aparece 0x755b em (-62,-70) do clique; aos 13, 0x7563 em (103,-27)
    use engine::scene::{Input, MouseButton, Scene};
    use engine::scenes::Paint;
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let mut p = Paint::new(None, &a);
    let c = |x, y| Input::Click {
        button: MouseButton::Left,
        x,
        y,
    };
    p.update(&[c(129, 30), c(40, 80), c(54, 152)], 0.0, &a);
    p.update(&[c(330, 280)], 0.0, &a);
    let mut f = engine::Frame::new(640, 480);
    // passo 1 do roteiro (200 a 400 ms): 0x755b com canto em (330-62, 280-70)
    p.update(&[], 0.3, &a);
    p.draw(&mut f, &a);
    let img = a.image(&ImageRef::Pic(Program::Artist, 0x755b));
    let mask = a.image(&ImageRef::Pic(Program::Artist, 0x755b + 6));
    let (x0, y0) = (330 - 62, 280 - 70);
    let mut ok = 0;
    let mut n = 0;
    for y in 0..img.height {
        for x in 0..img.width {
            let i = ((y * img.width + x) * 4) as usize;
            if mask.rgba[i] < 128 {
                n += 1;
                let px = f.pixel(x0 as u32 + x, y0 as u32 + y);
                if px[..3] == img.rgba[i..i + 3] {
                    ok += 1;
                }
            }
        }
    }
    assert!(ok as f32 / n as f32 > 0.97, "{ok}/{n}");
}

#[test]
fn sounds_tool_lists_mag100_categories() {
    use engine::scene::{Effect, Input, MouseButton, Scene};
    use engine::scenes::Paint;
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let mut p = Paint::new(None, &a);
    let c = |x, y| Input::Click {
        button: MouseButton::Left,
        x,
        y,
    };
    p.update(&[c(170, 30)], 0.0, &a);
    assert_eq!(p.sounds().categories(), 4); // Agudo, Dinossauro, AnimalDeFazenda, Maluco
    p.update(&[c(20, 70)], 0.0, &a); // primeira categoria abre a lista
    assert!(p.sounds().list_open());
    p.update(&[c(100, 112 + 2 + 15 + 3)], 0.0, &a); // primeiro som
    assert!(p.sounds().chosen().is_some());
    let fx = p.update(&[c(300, 300)], 0.0, &a);
    assert!(fx.iter().any(|e| matches!(e, Effect::PlaySound(_))));
    assert_eq!(p.sounds().spots.len(), 1);
}

#[test]
fn gallery_lists_sample_pictures() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    assert_eq!(a.data_dir(), d.as_path());
    // 16 .MAG em SHARED/USERS/MAGGIE, dos quais 11 são desenhos de 594×347
    assert_eq!(engine::scenes::gallery::list_samples(&d).len(), 16);
    let g = engine::scenes::Gallery::new(None, &a);
    assert_eq!(g.sample_count(), 11);
    assert_eq!(g.count(), 11);
    // SAMP6 abre com as cores aproximadas à paleta do programa
    let pal = &a.palettes(Program::Artist).pal256;
    let c =
        engine::paint::store::load_picture(&d.join("SHARED/USERS/MAGGIE/SAMP6.MAG"), pal).unwrap();
    assert_eq!((c.w, c.h), (594, 347));
    assert!(c.px.iter().any(|&p| p != 0));
}

#[test]
fn what_blocks_decode_with_options_and_links() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    // 3c00: assistente do botão/adesivo; registro 2 pergunta botão ou adesivo
    let w = a.dialogs(Program::Artist, 0x3c00);
    assert_eq!(w.len(), 34);
    assert_eq!(w[2].title, "Button or sticker");
    assert_eq!((w[2].width, w[2].height), (0xd0, 0x46));
    assert_eq!(w[2].options().len(), 2);
    assert!(w[2].options()[1].1.contains("adesivo"));
    // 3a00: desenho animado; "pasta" é a palavra do glossário 78
    let w = a.dialogs(Program::Artist, 0x3a00);
    assert_eq!(w[78].title, "pasta");
    assert_eq!(w[4].links(), vec![(78, String::new())]);
    // todos os blocos decodificam
    for id in a.exe_resource_ids(Program::Artist, "WHAT") {
        let w = a.dialogs(Program::Artist, id);
        assert!(!w.is_empty(), "WHAT {id:04x}");
        for r in &w {
            assert!(r.width > 0, "WHAT {id:04x} {}", r.title);
        }
    }
}

#[test]
fn glossary_words_fill_the_balloon_text() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let w = a.dialogs(Program::Artist, 0x100d);
    // 100d[9]: "Dê um [clique duplo] na imagem do quadro."
    let b = engine::balloon::Balloon::from_block(&w, 9, 0, 0, "x").unwrap();
    assert!(b.text.contains("clique duplo"), "{}", b.text);
}
