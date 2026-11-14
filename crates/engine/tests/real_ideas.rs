//! Máquina de Idéias e figuras-rebus com os dados originais (ATELIE_DATA).

use engine::assets::{Assets, ImageRef, Program};
use engine::rng::XorShift;
use engine::writer::ideas::{Grammar, Machine, Mode, Piece, Slot, rebus};
use std::path::PathBuf;

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
fn grammar_from_workshop() {
    let Some(d) = data_dir() else { return };
    let g = Grammar::load(&d).unwrap();
    let weights: Vec<(usize, usize)> = g
        .windows
        .iter()
        .map(|w| (w.text_weight(), w.figures.len()))
        .collect();
    assert_eq!(weights, [(102, 63), (126, 17), (82, 28)]);
    assert_eq!(g.windows[0].format, "    %s %s");
}

#[test]
fn machine_sentences_and_figure_rates() {
    let Some(d) = data_dir() else { return };
    let g = Grammar::load(&d).unwrap();
    let mut rng = XorShift::new(2026);
    let m = Machine::new(g, Default::default(), &mut rng);
    let sentence = m.sentence();
    assert!(!sentence.is_empty());
    if let Piece::Text(t) = &sentence[0] {
        assert!(!t.starts_with(' '), "{t:?}");
    }
    // proporções do original: 63/165, 17/143, 28/110
    let n = 30_000;
    for (w, p) in [(0, 63.0 / 165.0), (1, 17.0 / 143.0), (2, 28.0 / 110.0)] {
        let figs = (0..n)
            .filter(|_| matches!(m.pick(w, &mut rng, Mode::Any), Slot::Rebus(_)))
            .count();
        let rate = figs as f64 / n as f64;
        assert!(
            (rate - p).abs() < 0.015,
            "janela {w}: {rate} (esperado {p})"
        );
    }
}

#[test]
fn rebus_figures_from_writer_pic() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    // gato (c = 0x25) → 0x0049 colorida e 0x004a em 1 bit
    let cat = a.rebus(0x25).unwrap();
    let color = a.image(&ImageRef::Pic(Program::Writer, 0x49));
    let mono = a.rebus_mono(0x25).unwrap();
    assert_eq!((cat.width, cat.height), (30, 27));
    assert_eq!((mono.width, mono.height), (30, 27));
    // o canto é branco de fora (transparente); o resto conserva as cores
    assert_eq!(cat.rgba[3], 0);
    let opaque = cat.rgba.chunks_exact(4).filter(|p| p[3] != 0).count();
    assert!(opaque > 300 && opaque < 30 * 27, "{opaque}");
    for (c, o) in cat.rgba.chunks_exact(4).zip(color.rgba.chunks_exact(4)) {
        if c[3] != 0 {
            assert_eq!(&c[..3], &o[..3]);
        }
    }
    // todas as figuras das três janelas existem
    let g = Grammar::load(&d).unwrap();
    for w in &g.windows {
        for &c in &w.figures {
            let img = a.rebus(c).unwrap_or_else(|| panic!("rebus {c:#x}"));
            assert!(
                img.width >= 20 && img.width <= 40 && img.height <= 40,
                "{c:#x}"
            );
        }
    }
    assert!(a.rebus(0).is_none());
    let mut f = engine::Frame::new(64, 64);
    f.fill([0, 0, 255]);
    assert_eq!(rebus::draw(&mut f, &a, 0x25, 10, 10), Some((30, 27)));
    assert_eq!(f.pixel(10, 10), [0, 0, 255, 255]);
}
