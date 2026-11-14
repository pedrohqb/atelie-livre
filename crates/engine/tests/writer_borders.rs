//! Bordas e fundos de página desenhados a partir dos `.BOR` reais, com as
//! medidas tiradas das capturas do original (Visão normal, 1 pt = 1 pixel).

use engine::writer::borders::{
    PageArt, PageBackground, PageGeom, TextSide, border_layout, render_page,
};
use formats::bor::{self, BorFile};
use formats::winres::Image;
use std::path::PathBuf;

fn stuff(name: &str) -> Option<BorFile> {
    let dir = PathBuf::from(std::env::var_os("ATELIE_DATA")?);
    Some(bor::read(&std::fs::read(dir.join("SHARED/STUFF").join(name)).unwrap()).unwrap())
}

fn dark(img: &Image, x: u32, y: u32) -> bool {
    let i = ((y * img.width + x) * 4) as usize;
    (img.rgba[i] as u32 + img.rgba[i + 1] as u32 + img.rgba[i + 2] as u32) < 200
}

/// Trechos `[a, b]` de colunas com tinta na faixa de linhas dada.
fn ink_runs(img: &Image, rows: std::ops::Range<u32>) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut start = None;
    for x in 0..img.width {
        let ink = rows.clone().any(|y| dark(img, x, y));
        match (ink, start) {
            (true, None) => start = Some(x),
            (false, Some(s)) => {
                out.push((s, x - 1));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, img.width - 1));
    }
    out
}

#[test]
fn tribe_and_paw_match_the_screen() {
    let Some(m2) = stuff("2MAX100.BOR") else {
        return;
    };
    let m0 = stuff("0MAX100.BOR").unwrap();
    let page = PageGeom::new(595.0, 792.0, 72.0);
    // Tribe (w26_95, folha em x 22): cantos 54..88 e 550..584, 13 triângulos
    let tribe = PageArt {
        border: m2.border(4).cloned(),
        background: None,
    };
    let img = render_page(&page, &tribe, 1.0);
    let runs = ink_runs(&img, 40..60);
    assert_eq!(runs.first().unwrap().0 + 22, 54);
    assert_eq!(runs.last().unwrap().1 + 22, 584);
    // canto superior esquerdo preto de y 143 a 177 (folha em y 107)
    // canto superior esquerdo preto de y 143 a 177 (folha em y 107); o
    // último pixel pode faltar (arredondamento da caneta fina do GDI)
    let top = (0..120).find(|&y| dark(&img, 40, y)).unwrap();
    let bottom = (top..120)
        .take_while(|&y| dark(&img, 40, y))
        .last()
        .unwrap();
    assert_eq!(top, 143 - 107);
    assert!(bottom.abs_diff(177 - 107) <= 1, "{bottom}");
    // área de texto limpa
    assert!((72..523).all(|x| !dark(&img, x, 300)));
    // Paw (w26_101): 12 patas na faixa de cima, de x ≈ 43 a ≈ 595
    let paw = PageArt {
        border: m0.border(0).cloned(),
        background: None,
    };
    let img = render_page(&page, &paw, 1.0);
    let runs = ink_runs(&img, 30..62);
    assert_eq!(runs.len(), 12, "{runs:?}");
    assert!((runs[0].0 + 22).abs_diff(43) <= 2);
    assert!((runs[11].1 + 22).abs_diff(595) <= 2);
}

#[test]
fn every_border_and_background_renders() {
    let Some(m0) = stuff("0MAX100.BOR") else {
        return;
    };
    let files = [
        m0.clone(),
        stuff("1MAX100.BOR").unwrap(),
        stuff("2MAX100.BOR").unwrap(),
    ];
    for (w, h) in [(595.0, 792.0), (792.0, 595.0)] {
        let page = PageGeom::new(w, h, 72.0);
        for b in files.iter().flat_map(|f| f.borders.iter()) {
            let l = border_layout(&page, b);
            assert!(l.fits(&page), "{}", b.name);
            // a faixa fica ≈ 1,5 × a espessura (as peças esticadas encolhem ou crescem até meia peça)
            let e = b.band_pt();
            assert!(
                l.band > e * 0.7 && l.band < e * 1.4,
                "{}: {} vs {e}",
                b.name,
                l.band
            );
            let img = render_page(
                &page,
                &PageArt {
                    border: Some(b.clone()),
                    background: None,
                },
                0.5,
            );
            assert!(
                img.rgba.chunks_exact(4).any(|p| p[..3] != [255; 3]),
                "{} vazia",
                b.name
            );
        }
    }
    let tall = PageGeom::new(595.0, 792.0, 72.0);
    for bg in &m0.backgrounds {
        let art = PageArt {
            border: None,
            background: PageBackground::from_bor(&m0, bg.id),
        };
        let img = render_page(&tall, &art, 0.5);
        assert!(
            img.rgba.chunks_exact(4).any(|p| p[..3] != [255; 3]),
            "{} vazio",
            bg.name
        );
    }
    // Scroll: quatro obstáculos, o texto só à direita da tira esquerda
    let scroll = PageBackground::from_bor(&m0, 23).unwrap();
    let obs = scroll.obstacles();
    assert_eq!(obs.len(), 4);
    assert_eq!(obs[2].text_side, TextSide::Right);
    assert_eq!(obs[3].text_side, TextSide::Left);
    assert_eq!(obs[0].text_side, TextSide::None);
    // arte em tamanho natural a partir de (10, 17): o pergaminho começa em x ≈ 11
    let img = render_page(
        &tall,
        &PageArt {
            border: None,
            background: Some(scroll),
        },
        1.0,
    );
    assert!((0..10).all(|x| (0..792).all(|y| !dark(&img, x, y))));
    assert!((10..16).any(|x| (0..792).any(|y| dark(&img, x, y))));
}
