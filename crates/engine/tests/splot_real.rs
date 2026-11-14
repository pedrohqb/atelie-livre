//! Splot (palavras com forma) com os recursos do executável original.

use engine::assets::Assets;
use engine::paint::WordShape;
use engine::paint::shaped::{Face, Inks, ShapedWords, render};
use engine::paint::splot_res::SplotRes;
use std::path::PathBuf;

fn res() -> Option<SplotRes> {
    let Some(d) = std::env::var_os("ATELIE_DATA").map(PathBuf::from) else {
        eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
        return None;
    };
    Some(SplotRes::load(&Assets::open(&d).unwrap()))
}

const INKS: Inks = Inks {
    white: 0,
    black: 1,
    gray: 2,
    light: 3,
    dark: 4,
    pattern: [0; 8],
};

#[test]
fn menus_colors_and_patterns_come_from_the_exe() {
    let Some(r) = res() else { return };
    assert_eq!(r.shapes.len(), 14);
    assert_eq!(r.styles.len(), 9);
    assert_eq!(r.styles[6].label, "Preencher");
    assert_eq!(r.shadows[2], "Sombra do amanhecer");
    assert_eq!(r.color_sets.len(), 24);
    assert_eq!(r.color_sets[23].len(), 3);
    // cheio + 13 do PATS, com bit 1 = branco
    assert_eq!(r.patterns.len(), 14);
    assert_eq!(r.patterns[0], [0; 8]);
    assert_eq!(
        r.patterns[8],
        [0x0f, 0x87, 0xc3, 0xe1, 0xf0, 0x78, 0x3c, 0x1e]
    );
    assert_eq!(r.shape(WordShape::Head).map(|s| s.points.len()), Some(628));
    assert!(r.shape(WordShape::Plain).is_none());
}

#[test]
fn every_shape_draws_the_word() {
    let Some(r) = res() else { return };
    let w = ShapedWords {
        size: 32,
        ..ShapedWords::new("Palavras", 1)
    };
    let flat = render(&w, Face::Bitmap(None), &INKS, None);
    for shape in WordShape::ALL {
        let s = render(
            &ShapedWords { shape, ..w.clone() },
            Face::Bitmap(None),
            &INKS,
            r.shape(shape),
        );
        let ink = s.px.iter().filter(|p| **p == Some(1)).count();
        assert!(ink > 300, "{shape:?}: {ink}");
        assert!(
            s.w > 20 && s.h > 10 && s.w < 2000 && s.h < 2000,
            "{shape:?}"
        );
        if shape != WordShape::Plain {
            assert_ne!((s.w, s.h), (flat.w, flat.h), "{shape:?}");
        }
    }
    // arco embaixo: a largura do texto e o triplo da altura nas pontas
    let arch = render(
        &ShapedWords {
            shape: WordShape::Arch,
            ..w.clone()
        },
        Face::Bitmap(None),
        &INKS,
        r.shape(WordShape::Arch),
    );
    assert!((arch.w as i32 - flat.w as i32).abs() <= 6);
    assert!(arch.h > flat.h * 2);
}

/// Com `SPLOT_DUMP=pasta`, grava as imagens de cada forma (PGM) para a
/// folha comparativa com o original (ver `docs/formats/scrd.md`).
#[test]
fn dump_renders_for_the_comparison_sheet() {
    let Some(dir) = std::env::var_os("SPLOT_DUMP").map(PathBuf::from) else {
        return;
    };
    let Some(r) = res() else { return };
    let face = engine::paint::shaped::load_face("Liberation Sans").expect("Liberation Sans");
    let base = ShapedWords {
        font: "Liberation Sans".into(),
        ..ShapedWords::new("Palavras", 1)
    };
    let mut cases: Vec<(String, ShapedWords)> = WordShape::ALL
        .iter()
        .map(|&shape| {
            (
                shape.name().to_string(),
                ShapedWords {
                    shape,
                    ..base.clone()
                },
            )
        })
        .collect();
    let mut st = |name: &str, shape, styles: &str| {
        let mut w = ShapedWords {
            shape,
            ..base.clone()
        };
        w.set_styles(styles);
        cases.push((name.into(), w));
    };
    st("arco-embaixo_v", WordShape::Arch, "em-cima-embaixo");
    st("cresce_h", WordShape::GrowRight, "esquerda-direita");
    st(
        "cresce_h_fill",
        WordShape::GrowRight,
        "esquerda-direita,preencher",
    );
    std::fs::create_dir_all(&dir).unwrap();
    for (name, w) in cases {
        let s = render(&w, Face::Outline(&face), &INKS, r.shape(w.shape));
        let mut out = format!("P5\n{} {}\n255\n", s.w, s.h).into_bytes();
        out.extend(s.px.iter().map(|p| if p.is_some() { 0u8 } else { 255 }));
        std::fs::write(dir.join(format!("{name}.pgm")), out).unwrap();
    }
}
