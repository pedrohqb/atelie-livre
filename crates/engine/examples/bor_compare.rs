//! Prancha de comparação das bordas e fundos de página com o original.
//!
//! ```text
//! bor_compare DADOS CAPTURAS AMOSTRAS SAIDA.png
//! ```
//!
//! `DADOS` é a pasta `MSKIDS`, `CAPTURAS` a pasta das capturas do Creative
//! Writer (Visão normal, folha em (22, 107) da tela, 1 pt = 1 pixel) e
//! `AMOSTRAS` a pasta com `BORDA_PA.MAX` e `FUNDO_SC.MAX` gravados no
//! original. Cada linha mostra a captura, o desenho do port e a diferença
//! (vermelho: só no original; azul: só no port; preto: nos dois), e o
//! programa imprime a concordância fora da área de texto.

use engine::writer::borders::{PageArt, PageBackground, PageGeom, render_page};
use engine::writer::ideas::store::{read_png, write_png};
use engine::{Frame, Image};
use formats::bor;
use std::path::Path;

const PAGE_X: u32 = 22;
const PAGE_Y: u32 = 107;
const VIEW_W: u32 = 595;
const VIEW_H: u32 = 351;

fn dark(p: &[u8]) -> bool {
    (p[0] as u32 + p[1] as u32 + p[2] as u32) < 200
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() != 4 {
        eprintln!("uso: bor_compare DADOS CAPTURAS AMOSTRAS SAIDA.png");
        std::process::exit(2);
    }
    let (data, shots, samples) = (Path::new(&a[0]), Path::new(&a[1]), Path::new(&a[2]));
    let stuff =
        |f: &str| bor::read(&std::fs::read(data.join("SHARED/STUFF").join(f)).unwrap()).unwrap();
    let doc = |f: &str| formats::max::read(&std::fs::read(samples.join(f)).unwrap()).unwrap();
    let m0 = stuff("0MAX100.BOR");
    let m2 = stuff("2MAX100.BOR");
    let page = PageGeom::new(595.0, 792.0, 72.0);
    let cases: Vec<(&str, &str, PageArt)> = vec![
        (
            "Tribe (2MAX100.BOR, BDR 4)",
            "w26_95_tribe_normal.png",
            PageArt {
                border: m2.border(4).cloned(),
                background: None,
            },
        ),
        (
            "Paw (documento BORDA_PA.MAX)",
            "w26_101_paw.png",
            PageArt::from_document(&doc("BORDA_PA.MAX")),
        ),
        (
            "Scroll (documento FUNDO_SC.MAX)",
            "w26_111_scroll.png",
            PageArt::from_document(&doc("FUNDO_SC.MAX")),
        ),
        (
            "Marble (0MAX100.BOR, IBGF 26)",
            "projects_131b.png",
            PageArt {
                border: None,
                background: PageBackground::from_bor(&m0, 26),
            },
        ),
    ];
    let row_h = VIEW_H + 16;
    let mut sheet = Frame::new(3 * VIEW_W + 20, cases.len() as u32 * row_h);
    sheet.fill([255, 255, 255]);
    let text = page.text_rect();
    for (k, (label, shot, art)) in cases.iter().enumerate() {
        assert!(
            art.border.is_some() || art.background.is_some(),
            "{label}: nada lido"
        );
        let screen = read_png(&shots.join(shot)).expect("captura");
        let ours = render_page(&page, art, 1.0);
        let mut orig = Image {
            width: VIEW_W,
            height: VIEW_H,
            rgba: vec![255; (VIEW_W * VIEW_H * 4) as usize],
        };
        let mut mine = orig.clone();
        let mut diff = orig.clone();
        let (mut same, mut total) = (0usize, 0usize);
        for y in 0..VIEW_H {
            for x in 0..VIEW_W {
                let s = (((PAGE_Y + y) * screen.width + PAGE_X + x) * 4) as usize;
                let o = (y * ours.width + x) as usize * 4;
                let d = (y * VIEW_W + x) as usize * 4;
                let sp = &screen.rgba[s..s + 4];
                orig.rgba[d..d + 4].copy_from_slice(sp);
                mine.rgba[d..d + 4].copy_from_slice(&ours.rgba[o..o + 4]);
                let (a, b) = (dark(sp), dark(&ours.rgba[o..o + 4]));
                let c = match (a, b) {
                    (true, true) => [0, 0, 0, 255],
                    (true, false) => [230, 30, 30, 255],
                    (false, true) => [30, 60, 230, 255],
                    _ => [255, 255, 255, 255],
                };
                diff.rgba[d..d + 4].copy_from_slice(&c);
                let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
                let in_text =
                    fx >= text.x && fx < text.x + text.w && fy >= text.y && fy < text.y + text.h;
                // fora do texto e da moldura de 1 pixel da folha
                if !in_text && x > 0 && x + 1 < VIEW_W {
                    total += 1;
                    same += (a == b) as usize;
                }
            }
        }
        let score = same as f64 / total as f64;
        println!("{label:34} concordância fora do texto: {score:.3}");
        let y0 = k as u32 * row_h;
        sheet.text(
            4,
            y0 as i32 + 4,
            &format!("{label}  -  {score:.3}"),
            [0, 0, 0],
        );
        for (i, img) in [&orig, &mine, &diff].into_iter().enumerate() {
            sheet.blit(img, (i as u32 * (VIEW_W + 10)) as i32, (y0 + 16) as i32);
        }
    }
    let out = Image {
        width: sheet.width(),
        height: sheet.height(),
        rgba: sheet.pixels().to_vec(),
    };
    write_png(Path::new(&a[3]), &out).expect("gravar");
}
