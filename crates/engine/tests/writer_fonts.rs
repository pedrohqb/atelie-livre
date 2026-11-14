//! Fontes do Writer com as substitutas embutidas (o mesmo em qualquer
//! máquina) e, com `ATELIE_DATA`, a Pablo Graphos dos dados originais.

use engine::Frame;
use engine::writer::doc::Size;
use engine::writer::fonts::{Family, RunFont, Source, WriterFonts};
use std::path::PathBuf;

const TRUETYPE: [Family; 5] = [
    Family::LucidaSans,
    Family::LucidaHandwriting,
    Family::LucidaBlackletter,
    Family::Impact,
    Family::Stencil,
];

fn fonts() -> WriterFonts {
    WriterFonts::embedded(Vec::new())
}

#[test]
fn every_truetype_family_has_an_embedded_substitute() {
    let f = fonts();
    for fam in TRUETYPE {
        assert!(
            matches!(f.source(fam), Source::Embedded(_)),
            "{fam:?}: {:?}",
            f.source(fam)
        );
    }
    assert_eq!(f.source(Family::Pablo), Source::Missing);
}

#[test]
fn substitutes_have_about_the_width_of_the_originals() {
    // avanço de "HHxg MMMM iiii" nas originais (GDI: arredondado por glifo), 12 e 48 pt
    let original = [(92, 379), (109, 445), (100, 397), (77, 304), (94, 383)];
    let f = fonts();
    for (fam, (w12, w48)) in TRUETYPE.into_iter().zip(original) {
        for (size, want) in [(Size::Minimal, w12), (Size::Giant, w48)] {
            let got = f.text_width(&RunFont::new(fam, size), "HHxg MMMM iiii");
            let err = (got - want).abs() as f32 / want as f32;
            assert!(err < 0.12, "{fam:?} {size:?}: {got} × {want} no original");
        }
    }
}

#[test]
fn capitals_have_the_height_of_the_originals_and_sit_on_the_baseline() {
    // altura do 'H' na original em 48 pt (0,7227 em na Lucida Sans: 35 px medidos)
    let caps = [34.7, 38.9, 35.6, 37.9, 32.0];
    let f = fonts();
    for (fam, cap) in TRUETYPE.into_iter().zip(caps) {
        let font = RunFont::new(fam, Size::Giant);
        let m = f.render(&font, "H");
        let lm = f.line_metrics(&font);
        let (_, y0, _, _) = m.ink_bounds().expect("tinta");
        assert!(
            ((lm.ascent - y0) as f32 - cap).abs() <= 2.0,
            "{fam:?}: H com {} px acima da base, original {cap}",
            lm.ascent - y0
        );
        // 'n' (sem descendente em nenhuma) assenta na base
        let (_, _, _, y1) = f.render(&font, "n").ink_bounds().unwrap();
        assert!(
            (y1 - lm.ascent).abs() <= 1,
            "{fam:?}: fundo do n em {y1}, base {}",
            lm.ascent
        );
    }
}

#[test]
fn bold_adds_one_pixel_per_character() {
    let f = fonts();
    for size in Size::ALL {
        let font = RunFont::new(Family::LucidaSans, size);
        let plain = f.text_width(&font, "HHxg MMMM iiii");
        assert_eq!(
            f.text_width(&font.bold(true), "HHxg MMMM iiii"),
            plain + 14,
            "{size:?}"
        );
        let m = f.render(&font.bold(true), "l");
        let thin = f.render(&font, "l");
        let w = |m: &engine::writer::fonts::TextMask| {
            let b = m.ink_bounds().unwrap();
            b.2 - b.0
        };
        assert!(w(&m) > w(&thin), "{size:?}: negrito mais grosso");
    }
}

#[test]
fn italic_leans_right_and_keeps_the_advance() {
    let f = fonts();
    let font = RunFont::new(Family::LucidaSans, Size::Giant);
    assert_eq!(
        f.text_width(&font.italic(true), "HI"),
        f.text_width(&font, "HI")
    );
    let up = f.render(&font, "I");
    let it = f.render(&font.italic(true), "I");
    let left =
        |m: &engine::writer::fonts::TextMask, y: i32| (m.x..m.x + m.w).find(|&x| m.at(x, y) >= 128);
    let (top, bottom) = (15, up.baseline - 1);
    assert_eq!(left(&up, top), left(&up, bottom));
    let lean = left(&it, top).unwrap() - left(&it, bottom).unwrap();
    // 0,24 px por linha
    assert!((6..=9).contains(&lean), "inclinação de {lean} px");
}

#[test]
fn underline_sits_below_the_baseline() {
    let f = fonts();
    let font = RunFont::new(Family::LucidaSans, Size::Giant).underline(true);
    let m = f.render(&font, "ab");
    let base = m.baseline;
    // Gigante: 2 linhas a partir de 4 abaixo da base (medido no original)
    for x in 0..m.advance {
        assert_eq!(m.at(x, base + 4), 255);
        assert_eq!(m.at(x, base + 5), 255);
    }
    let plain = f.render(&RunFont::new(Family::LucidaSans, Size::Giant), "ab");
    assert_eq!(plain.at(m.advance / 2, base + 4), 0);
}

#[test]
fn stencil_draws_lowercase_as_capitals() {
    let f = fonts();
    let font = RunFont::new(Family::Stencil, Size::Large);
    assert_eq!(f.advance(&font, 'a'), f.advance(&font, 'A'));
    assert_eq!(f.advance(&font, 'ç'), f.advance(&font, 'Ç'));
    assert_eq!(f.render(&font, "era").cov, f.render(&font, "ERA").cov);
}

#[test]
fn draws_in_color_without_smoothing() {
    let f = fonts();
    let mut frame = Frame::new(200, 80);
    frame.fill([255, 255, 255]);
    let font = RunFont::new(Family::Impact, Size::Huge);
    let adv = f.draw(&mut frame, 10, 5, &font, "Olá", [250, 0, 57], false);
    assert_eq!(adv, f.text_width(&font, "Olá"));
    let mut red = 0;
    for y in 0..80 {
        for x in 0..200 {
            match frame.pixel(x, y) {
                [255, 255, 255, _] => {}
                [250, 0, 57, _] => red += 1,
                p => panic!("cor misturada {p:?} em ({x}, {y}) sem suavizar"),
            }
        }
    }
    assert!(red > 100);
    // suavizado: aparecem tons intermediários
    let mut frame = Frame::new(200, 80);
    frame.fill([255, 255, 255]);
    f.draw(&mut frame, 10, 5, &font, "Olá", [0, 0, 0], true);
    let gray = (0..80)
        .flat_map(|y| (0..200).map(move |x| (x, y)))
        .filter(|&(x, y)| matches!(frame.pixel(x, y), [v, _, _, _] if v > 0 && v < 255))
        .count();
    assert!(gray > 0);
}

#[test]
fn empty_text_has_no_ink() {
    let f = fonts();
    let m = f.render(&RunFont::default(), "");
    assert_eq!(m.advance, 0);
    assert_eq!(m.ink_bounds(), None);
    assert_eq!(f.extent(&RunFont::default(), ""), (0, 15));
}

#[test]
fn system_resolution_finds_something_for_every_family() {
    // com o sistema, cada família tem fonte (original, embutida ou do sistema)
    let f = WriterFonts::new(Vec::new());
    for fam in TRUETYPE {
        assert_ne!(f.source(fam), Source::Missing, "{fam:?}");
    }
}

fn data_dir() -> Option<PathBuf> {
    match std::env::var_os("ATELIE_DATA") {
        Some(d) => Some(PathBuf::from(d)),
        None => {
            eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
            None
        }
    }
}

#[test]
fn pablo_graphos_from_the_real_data() {
    let Some(d) = data_dir() else { return };
    let a = engine::assets::Assets::open(&d).unwrap();
    let strikes: Vec<_> = [6, 8, 16].iter().filter_map(|&p| a.font(p)).collect();
    assert_eq!(strikes.len(), 3);
    let f = WriterFonts::embedded(strikes);
    assert_eq!(f.source(Family::Pablo), Source::Bitmap);
    // Gigante = fonte de 16 pontos ampliada 3×: célula 63, ascendente 51, entrelinha 6
    let big = RunFont::new(Family::Pablo, Size::Giant);
    let lm = f.line_metrics(&big);
    assert_eq!((lm.ascent, lm.descent, lm.leading), (51, 12, 6));
    assert_eq!(lm, engine::writer::fonts::line_metrics(&big));
    let small = RunFont::new(Family::Pablo, Size::Minimal);
    assert_eq!(
        f.text_width(&big, "M"),
        3 * a.font(16).unwrap().width("M") as i32
    );
    assert_eq!(
        f.text_width(&small, "M"),
        a.font(8).unwrap().width("M") as i32
    );
    let m = f.render(&big, "H");
    let (_, _, _, y1) = m.ink_bounds().unwrap();
    assert_eq!(y1, lm.ascent);
}
