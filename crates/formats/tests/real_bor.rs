//! `.BOR` reais: inventário e desenho comparado às miniaturas `PKBM`.

mod support;

use formats::bor::{self, BorFile, Slot, tile};
use formats::max::FrameKind;
use formats::pkbm;
use formats::winres::Image;
use formats::wmf_draw::Rect;

fn load(name: &str) -> Option<BorFile> {
    let data = support::read(&format!("SHARED/STUFF/{name}"))?;
    Some(bor::read(&data).unwrap_or_else(|e| panic!("{name}: {e}")))
}

fn white(w: u32, h: u32) -> Image {
    Image {
        width: w,
        height: h,
        rgba: [255, 255, 255, 255].repeat((w * h) as usize),
    }
}

/// Pixels não brancos (o índice 0 é o branco em 1 e em 4 bits).
fn inked_thumb(b: &pkbm::Bitmap) -> Vec<bool> {
    b.pixels.iter().map(|&i| i != 0).collect()
}

fn inked(img: &Image) -> Vec<bool> {
    img.rgba
        .chunks_exact(4)
        .map(|p| p[3] > 0 && (p[0] as u32 + p[1] as u32 + p[2] as u32) < 750)
        .collect()
}

fn agree(a: &[bool], b: &[bool]) -> f64 {
    a.iter().zip(b).filter(|(x, y)| x == y).count() as f64 / a.len() as f64
}

#[test]
fn inventory_matches_the_rolls() {
    let Some(m0) = load("0MAX100.BOR") else {
        return;
    };
    let m1 = load("1MAX100.BOR").unwrap();
    let m2 = load("2MAX100.BOR").unwrap();
    let g0 = load("0MAG100.BOR").unwrap();
    let names = |f: &BorFile| f.borders.iter().map(|b| b.name.clone()).collect::<Vec<_>>();
    assert_eq!(
        names(&m0),
        [
            "Paw",
            "Light Bulb",
            "Safari",
            "Vine",
            "Palms...Black",
            "Snowflakes",
            "Grade",
            "Clocks",
            "Party (Katie)"
        ]
    );
    assert_eq!(
        names(&m1),
        [
            "Seattle",
            "People Waving",
            "Cabins",
            "Hearts",
            "Ladybugs",
            "Trees",
            "Pencils",
            "Balloon"
        ]
    );
    assert_eq!(
        names(&m2),
        [
            "Checkered",
            "Tribe",
            "Triangle Party",
            "Notes",
            "Circles",
            "Stars",
            "Flowers...Modern"
        ]
    );
    assert!(g0.borders.is_empty());
    let all: Vec<_> = [&m0, &m1, &m2]
        .iter()
        .flat_map(|f| f.borders.iter())
        .collect();
    assert_eq!(all.len(), 24);
    for b in &all {
        assert!([16, 24, 30, 32, 36].contains(&b.thickness_pt), "{}", b.name);
        let t = b.thumb.as_ref().unwrap();
        assert_eq!((t.width, t.height), (80, 72), "{}", b.name);
    }
    let tribe = m2.border(4).unwrap();
    assert_eq!(tribe.map, [0, 1, 0, 2, 0, 3, 0, 4]);
    assert_eq!(tribe.dop_widths(), (25, 37));
    let paw = m0.border(0).unwrap();
    assert_eq!((paw.thickness_pt, paw.pieces.len()), (32, 1));
    assert_eq!(paw.dop_widths(), (33, 49));
    let vine = m0.border(18).unwrap();
    assert_eq!(vine.pieces.len(), 8);
    assert_eq!(
        (vine.piece(Slot::Top).width, vine.piece(Slot::Top).height),
        (715, 480)
    );
    assert_eq!(
        (vine.piece(Slot::Left).width, vine.piece(Slot::Left).height),
        (480, 715)
    );

    // fundos: 12 retrato + 3 paisagem no Writer, 11 paisagem no Fine Artist
    assert_eq!(m0.backgrounds.len(), 15);
    assert_eq!(m0.backgrounds.iter().filter(|b| b.landscape).count(), 3);
    assert_eq!(g0.backgrounds.len(), 11);
    assert!(g0.backgrounds.iter().all(|b| b.landscape));
    let portrait: Vec<_> = m0
        .backgrounds
        .iter()
        .filter(|b| !b.landscape)
        .map(|b| b.name.as_str())
        .collect();
    assert_eq!(
        &portrait[..3],
        ["PD - Scroll", "PD - Marble", "PD - Island Fever-Tall"]
    );
    for (f, n) in [(&m0, 15), (&g0, 11)] {
        let mut k = 0;
        for bg in &f.backgrounds {
            let t = bg.thumb.as_ref().unwrap();
            let size = if bg.landscape { (80, 60) } else { (54, 72) };
            assert_eq!((t.width, t.height), size, "{}", bg.name);
            if let Some(id) = bg.pict {
                assert!(f.picture(id).is_some(), "{}", bg.name);
            }
            for (_, id) in bg.pictures() {
                assert!(f.picture(id).is_some(), "{} PICT {id}", bg.name);
            }
            k += 1;
        }
        assert_eq!(k, n);
    }
    // Scroll: PICT 34 de 574×757 em (10, 17), quatro obstáculos (W26)
    let scroll = m0.background(23).unwrap();
    assert_eq!(
        (scroll.pict, scroll.dx, scroll.dy, scroll.fram),
        (Some(34), 10, 17, Some(16))
    );
    let art = m0.picture(34).unwrap();
    assert_eq!((art.width, art.height), (574, 757));
    let obs: Vec<_> = scroll
        .obstacles()
        .map(|f| (f.x, f.y, f.width, f.height, f.wrap.unwrap()))
        .collect();
    assert_eq!(
        obs,
        [
            (73, 10, 443, 121, 3),
            (73, 677, 443, 107, 3),
            (71, 127, 12, 555, 2),
            (501, 125, 12, 555, 1)
        ]
    );
    // fundos sem PICT: a arte vem das molduras de figura
    for id in [19, 21, 2, 4] {
        let bg = m0.background(id).unwrap();
        assert!(bg.pict.is_none());
        assert!(bg.pictures().count() > 0);
        assert!(bg.frames.iter().all(|f| f.kind == FrameKind::Picture));
    }
    // T-shirt: moldura de página (tipo 4)
    assert_eq!(
        m0.background(25).unwrap().frames[0].kind,
        FrameKind::PageFrame
    );
}

/// Cada borda desenhada no arranjo da sua miniatura (cantos quadrados, lados
/// esticados; o número de peças varia de miniatura para miniatura e é
/// procurado) bate com a miniatura `PKBM`.
#[test]
fn borders_match_their_thumbnails() {
    let Some(m0) = load("0MAX100.BOR") else {
        return;
    };
    let files = [
        m0,
        load("1MAX100.BOR").unwrap(),
        load("2MAX100.BOR").unwrap(),
    ];
    let mut scores = Vec::new();
    for b in files.iter().flat_map(|f| f.borders.iter()) {
        let t = b.thumb.as_ref().unwrap();
        let (w, h) = (t.width as u32, t.height as u32);
        let want = inked_thumb(t);
        let mut best = (0.0, 0, 0);
        for n in 1..=6 {
            for m in 1..=6 {
                for t in [h as f64 / (m + 2) as f64, w as f64 / (n + 2) as f64] {
                    let mut img = white(w, h);
                    b.draw_tiles(
                        &mut img,
                        &tile(Rect::new(0.0, 0.0, w as f64, h as f64), t, n, m),
                    );
                    let a = agree(&inked(&img), &want);
                    if a > best.0 {
                        best = (a, n, m);
                    }
                }
            }
        }
        eprintln!("{:20} {}x{} {:.3}", b.name, best.1, best.2, best.0);
        assert!(best.0 >= 0.72, "{}: {:.3}", b.name, best.0);
        scores.push((b.name.clone(), best.0));
    }
    let mean = scores.iter().map(|s| s.1).sum::<f64>() / scores.len() as f64;
    assert!(mean >= 0.89, "média {mean:.3}");
    let get = |n: &str| scores.iter().find(|s| s.0 == n).unwrap().1;
    // as peças de formas retas saem pixel a pixel
    assert!(get("Checkered") >= 0.99);
    assert!(get("Tribe") >= 0.98);
    assert!(
        get("Hearts") >= 0.93 && get("Ladybugs") >= 0.93,
        "bordas coloridas"
    );
}

/// Cada fundo desenhado numa página do tamanho da miniatura (4× e reduzido
/// preservando o preto, com o contorno de 1 pixel) bate com ela; a origem da
/// janela WMF vai para `(dx, dy)`.
#[test]
fn backgrounds_match_their_thumbnails() {
    let Some(m0) = load("0MAX100.BOR") else {
        return;
    };
    let g0 = load("0MAG100.BOR").unwrap();
    let render = |f: &BorFile, bg: &bor::Background, shifted: bool| -> f64 {
        let t = bg.thumb.as_ref().unwrap();
        let (w, h) = (t.width as u32, t.height as u32);
        let page_pt = if bg.landscape {
            (792.0, 612.0)
        } else {
            (612.0, 792.0)
        };
        let k = 4;
        let mut big = white(w * k, h * k);
        let mut b = bg.clone();
        if shifted {
            // hipótese descartada: coordenadas lógicas somadas a (dx, dy)
            if let Some(p) = bg.pict.and_then(|id| f.picture(id)) {
                b.dx += p.x;
                b.dy += p.y;
            }
        }
        b.draw(
            &mut big,
            Rect::new(0.0, 0.0, (w * k) as f64, (h * k) as f64),
            page_pt,
            &|id| f.picture(id),
        );
        let ink = inked(&big);
        let mut small = vec![false; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let any = (0..k)
                    .any(|dy| (0..k).any(|dx| ink[((y * k + dy) * w * k + x * k + dx) as usize]));
                small[(y * w + x) as usize] = any || x == 0 || y == 0 || x == w - 1 || y == h - 1;
            }
        }
        agree(&small, &inked_thumb(t))
    };
    let mut total = 0.0;
    let mut n = 0;
    for f in [&m0, &g0] {
        for bg in &f.backgrounds {
            let a = render(f, bg, false);
            let b = render(f, bg, true);
            eprintln!("{:32} {a:.3} (deslocado {b:.3})", bg.name);
            assert!(a >= 0.78, "{}: {a:.3}", bg.name);
            assert!(a >= b, "{}", bg.name);
            total += a;
            n += 1;
        }
    }
    assert!(total / n as f64 >= 0.88);
    // onde a origem da janela não é (0, 0) a diferença aparece
    let dinos = m0.background(0).unwrap();
    assert!(render(&m0, dinos, false) > render(&m0, dinos, true) + 0.05);
}
