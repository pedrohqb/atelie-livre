//! Um traço sintético por modalidade, conferindo os pixels.

use super::*;
use crate::paint::canvas::WHITE;

const RED: u8 = 9;

fn ink() -> Ink {
    Ink::solid(RED)
}

/// Paleta de teste: 0 branco, 1 preto, o resto um degradê de vermelho (`i`, 0, 0).
fn pal() -> Vec<Rgb> {
    (0..256)
        .map(|i| match i {
            0 => Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            1 => Rgb { r: 0, g: 0, b: 0 },
            _ => Rgb {
                r: i as u8,
                g: 0,
                b: 0,
            },
        })
        .collect()
}

fn full(w: u32) -> Shape {
    Shape {
        w,
        h: w,
        cells: vec![Cell::Ink; (w * w) as usize],
    }
}

/// Imagens sintéticas: ladrilhos 21×21 cheios, carimbos 16×16 cheios e
/// pegadas de um pixel só (para contar quantas foram postas).
fn tiles() -> Tiles {
    Tiles::load(|id| {
        if FOOTPRINTS.contains(&id) {
            full(1)
        } else if STAMPS.contains(&id) {
            full(16)
        } else {
            full(CELL as u32)
        }
    })
}

fn pen(page: u8, k: usize) -> Pen {
    let mut p = Pen::new(tiles(), pal());
    p.select(page, k);
    p
}

fn painted(c: &Canvas) -> usize {
    c.px.iter().filter(|&&v| v != WHITE).count()
}

fn ink_at(c: &Canvas, x: i32, y: i32) -> bool {
    c.get(x, y).is_some_and(|v| v != WHITE)
}

/// Traço completo: aperta no primeiro ponto, arrasta pelos outros e solta no último.
fn stroke(p: &mut Pen, c: &mut Canvas, pts: &[Px]) {
    let mut s = p.press(c, pts[0], &ink(), 7).expect("traço");
    for &q in &pts[1..] {
        p.drag(&mut s, c, q, &ink());
    }
    p.release(s, c, *pts.last().unwrap(), &ink());
}

#[test]
fn page2_stamps_only_when_moving_16_pixels_per_event() {
    let mut c = Canvas::new(300, 100);
    let mut p = pen(2, 0);
    // passos de 10: carimba a cada dois movimentos (20 pixels)
    let pts: Vec<Px> = (0..=10).map(|i| (20 + 10 * i, 50)).collect();
    stroke(&mut p, &mut c, &pts);
    // 1 no clique + 5 → 6 carimbos de 16×16, sem sobrepor
    assert_eq!(painted(&c), 6 * 256);
    // arrasto rápido (passos de 40): um carimbo por movimento
    let mut c = Canvas::new(300, 100);
    stroke(&mut p, &mut c, &[(20, 50), (60, 50), (100, 50)]);
    assert_eq!(painted(&c), 3 * 256);
}

#[test]
fn bubbles_radius_is_the_distance_moved() {
    let mut c = Canvas::new(200, 200);
    let mut p = pen(3, 0);
    stroke(&mut p, &mut c, &[(100, 100), (101, 100)]);
    assert_eq!(painted(&c), 0, "movimento curto não faz bolha");
    stroke(&mut p, &mut c, &[(50, 100), (100, 100)]);
    // círculo de raio 50 em volta de (100, 100), oco
    assert!(ink_at(&c, 150, 100));
    assert!(ink_at(&c, 100, 50));
    assert!(!ink_at(&c, 100, 100));
}

#[test]
fn scribble_and_spray_paint_near_the_path() {
    let mut c = Canvas::new(200, 100);
    let mut p = pen(3, 1);
    stroke(&mut p, &mut c, &[(10, 50), (60, 50), (110, 50)]);
    let n = painted(&c);
    assert!(n >= 100, "{n}");
    assert!((0..200).all(|x| (0..40).all(|y| !ink_at(&c, x, y))));
    let mut c = Canvas::new(200, 100);
    let mut p = pen(3, 2);
    let mut s = p.press(&mut c, (100, 50), &ink(), 3).unwrap();
    for _ in 0..20 {
        assert!(p.tick(&mut s, &mut c, &ink()));
    }
    let n = painted(&c);
    assert!(n > 40, "{n}");
    for y in 0..100 {
        for x in 0..200 {
            if ink_at(&c, x, y) {
                assert!((x - 100).pow(2) + (y - 50).pow(2) <= 100);
            }
        }
    }
}

#[test]
fn crack_and_web_are_elastic_and_sized_by_the_drag() {
    for k in [3, 4] {
        let mut c = Canvas::new(300, 300);
        let mut p = pen(3, k);
        assert!(p.elastic());
        let mut s = p.press(&mut c, (150, 150), &ink(), 5).unwrap();
        p.drag(&mut s, &mut c, (210, 150), &ink());
        assert_eq!(painted(&c), 0, "efeito {k} só pinta ao soltar");
        assert!(!p.preview(&s).is_empty());
        p.release(s, &mut c, (210, 150), &ink());
        let n = painted(&c);
        assert!(n > 200, "efeito {k}: {n}");
        // nada além de ~1,2 × a distância do centro
        for y in 0..300 {
            for x in 0..300 {
                if ink_at(&c, x, y) {
                    let d2 = (x - 150).pow(2) + (y - 150).pow(2);
                    assert!(d2 <= 75 * 75, "efeito {k} em ({x},{y})");
                }
            }
        }
    }
}

#[test]
fn spiral_grows_with_each_movement_up_to_ten_pixels() {
    let mut c = Canvas::new(300, 300);
    let mut p = pen(3, 5);
    let mut s = p.press(&mut c, (150, 150), &ink(), 1).unwrap();
    assert!(painted(&c) > 0, "o clique já desenha o miolo");
    // 4 movimentos de 50 pixels crescem só 10 cada: raio 47
    for i in 1..=4 {
        p.drag(&mut s, &mut c, (150 + 50 * i, 150), &ink());
    }
    let Stroke::Spiral { r, .. } = s else {
        panic!("não é espiral")
    };
    assert_eq!(r, SPIRAL_START + 40.0);
    let far = (0..300)
        .flat_map(|y| (0..300).map(move |x| (x, y)))
        .filter(|&(x, y)| ink_at(&c, x, y))
        .map(|(x, y)| (((x - 150).pow(2) + (y - 150).pow(2)) as f64).sqrt())
        .fold(0.0, f64::max);
    assert!((44.0..=50.0).contains(&far), "{far}");
}

#[test]
fn tracks_follow_the_21_pixel_grid_and_skip_the_last_cell() {
    let mut c = Canvas::new(300, 300);
    let mut p = pen(3, 7);
    // da casa (1,1) até a casa (4,1): desenha 1, 2 e 3; a 4 (onde parou) não
    stroke(&mut p, &mut c, &[(30, 30), (60, 30), (90, 30)]);
    assert_eq!(painted(&c), 3 * 21 * 21);
    assert!(ink_at(&c, 21, 21));
    assert!(ink_at(&c, 83, 41));
    assert!(!ink_at(&c, 84, 30));
    assert!(!ink_at(&c, 20, 30));
}

#[test]
fn track_tiles_are_chosen_by_entry_and_exit_sides() {
    let t = tiles();
    let set = &t.tracks[0];
    let same = |a: Side, b: Side, v: &Vec<Shape>| std::ptr::eq(set.tile(a, b, 0).unwrap(), &v[0]);
    assert!(same(Side::W, Side::E, &set.h));
    assert!(same(Side::N, Side::N, &set.v));
    assert!(same(Side::E, Side::S, &set.se));
    assert!(same(Side::S, Side::W, &set.sw));
    assert!(same(Side::W, Side::N, &set.nw));
    assert!(same(Side::N, Side::E, &set.ne));
    // variantes se alternam
    let river = &t.tracks[3];
    assert!(std::ptr::eq(
        river.tile(Side::E, Side::W, 1).unwrap(),
        &river.h[1]
    ));
    // diagonal: a trilha desce primeiro e depois vira (escada)
    let mut c = Canvas::new(300, 300);
    let mut p = pen(3, 6);
    stroke(&mut p, &mut c, &[(30, 30), (52, 52), (74, 74)]);
    // casas (1,1), (1,2), (2,2), (2,3) desenhadas; (3,3) não
    assert_eq!(painted(&c), 4 * 21 * 21);
    assert!(ink_at(&c, 30, 50));
    assert!(!ink_at(&c, 50, 30));
}

#[test]
fn footprints_walk_in_eight_directions() {
    let mut c = Canvas::new(300, 300);
    let mut p = pen(3, 8);
    stroke(&mut p, &mut c, &[(30, 30), (115, 115)]);
    // diagonal pura: casas (1,1), (2,2), (3,3), (4,4) — uma pegada cada
    assert_eq!(painted(&c), 4);
    assert!(ink_at(&c, 31, 31));
    assert!(ink_at(&c, 94, 94));
    assert_eq!(foot_index((-1, -1)), 5);
    assert_eq!(foot_index((1, -1)), 7);
}

#[test]
fn page5_lines_and_mirrors() {
    let mut c = Canvas::new(300, 300);
    stroke(&mut pen(5, 0), &mut c, &[(10, 10), (50, 50), (100, 10)]);
    assert_eq!(painted(&c), 91, "reta da âncora ao ponto onde soltou");
    let mut c = Canvas::new(300, 300);
    stroke(&mut pen(5, 1), &mut c, &[(100, 100), (200, 140)]);
    // 21,8° → 15°: termina em (100 + 108·cos15°, 100 + 108·sen15°)
    assert!(ink_at(&c, 204, 128));
    assert!(!ink_at(&c, 200, 140));
    let mut c = Canvas::new(300, 300);
    stroke(&mut pen(5, 2), &mut c, &[(150, 100), (200, 150)]);
    assert!(ink_at(&c, 100, 150), "espelho à esquerda");
    let mut c = Canvas::new(300, 300);
    stroke(&mut pen(5, 3), &mut c, &[(150, 100), (200, 150)]);
    assert!(ink_at(&c, 200, 50), "espelho em cima");
}

#[test]
fn side_maker_continues_from_last_end_until_reselected() {
    let mut c = Canvas::new(300, 300);
    let mut p = pen(5, 4);
    stroke(&mut p, &mut c, &[(10, 10), (100, 10)]);
    // o segundo arrasto começa longe, mas a linha sai do fim do primeiro
    stroke(&mut p, &mut c, &[(200, 200), (100, 100)]);
    assert!(ink_at(&c, 100, 50));
    assert!(!ink_at(&c, 150, 150));
    p.select(5, 4);
    let before = painted(&c);
    stroke(&mut p, &mut c, &[(200, 200), (250, 200)]);
    assert_eq!(painted(&c) - before, 51);
}

#[test]
fn centered_shapes_and_corner_shapes() {
    let mut c = Canvas::new(400, 400);
    stroke(&mut pen(5, 6), &mut c, &[(200, 200), (300, 200)]);
    // meio lado = 100/√2 ≈ 71
    assert!(ink_at(&c, 271, 200));
    assert!(ink_at(&c, 129, 129));
    assert!(!ink_at(&c, 300, 200));
    let mut c = Canvas::new(400, 400);
    stroke(&mut pen(5, 7), &mut c, &[(200, 200), (260, 280)]);
    assert!(ink_at(&c, 300, 200), "raio 100");
    assert!(ink_at(&c, 200, 100));
    let mut c = Canvas::new(400, 400);
    stroke(&mut pen(5, 8), &mut c, &[(200, 200), (200, 100)]);
    assert!(ink_at(&c, 200, 100), "ápice em cima");
    assert!(ink_at(&c, 200, 250), "base a meio raio embaixo");
    let mut c = Canvas::new(400, 400);
    stroke(&mut pen(5, 9), &mut c, &[(10, 10), (50, 30)]);
    assert_eq!(painted(&c), 2 * 41 + 2 * 19);
    let mut c = Canvas::new(400, 400);
    stroke(&mut pen(5, 10), &mut c, &[(10, 10), (50, 30)]);
    assert!(ink_at(&c, 30, 10));
    assert!(!ink_at(&c, 10, 10));
}

#[test]
fn curve_maker_rounds_corners_but_keeps_ends() {
    let mut c = Canvas::new(300, 300);
    stroke(
        &mut pen(5, 5),
        &mut c,
        &[(10, 200), (60, 100), (110, 200), (160, 100)],
    );
    assert!(ink_at(&c, 10, 200));
    assert!(ink_at(&c, 160, 100));
    // os cantos ficam arredondados: não chega ao vértice (60, 100)
    assert!(!ink_at(&c, 60, 100));
    assert!((105..=165).any(|y| ink_at(&c, 60, y)));
}

#[test]
fn triangle2_takes_two_drags() {
    let mut c = Canvas::new(300, 300);
    let mut p = pen(5, 11);
    stroke(&mut p, &mut c, &[(50, 200), (150, 200)]);
    assert_eq!(painted(&c), 101);
    stroke(&mut p, &mut c, &[(10, 10), (100, 100)]);
    assert!(ink_at(&c, 125, 150), "lado 2 até o terceiro canto");
    assert!(ink_at(&c, 75, 150), "lado 3 de volta ao início");
    // a próxima vez começa outro triângulo
    let before = painted(&c);
    stroke(&mut p, &mut c, &[(200, 20), (250, 20)]);
    assert_eq!(painted(&c) - before, 51);
}

#[test]
fn chalk_pastel_and_crayon_leave_texture() {
    for (k, d) in [(0, 8), (1, 16), (2, 32), (6, 4), (7, 8), (8, 24)] {
        let mut c = Canvas::new(300, 100);
        stroke(&mut pen(6, k), &mut c, &[(50, 50), (250, 50)]);
        let rows = (0..100)
            .filter(|&y| (0..300).any(|x| ink_at(&c, x, y)))
            .count() as i32;
        assert!((rows - d).abs() <= 1, "botão {k}: {rows} linhas");
        // uns 3/4 dos pixels no meio do traço, nunca todos
        let red = (100..200).filter(|&x| c.get(x, 50) == Some(RED)).count();
        assert!((55..=95).contains(&red), "botão {k}: {red}%");
    }
    // lápis de cera: miolo da cor, borda só no tom claro
    let mut c = Canvas::new(300, 100);
    stroke(&mut pen(6, 4), &mut c, &[(50, 50), (250, 50)]);
    let edge: Vec<u8> = (100..200).map(|x| c.get(x, 43).unwrap()).collect();
    assert!(edge.iter().all(|&v| v != RED));
    assert!(edge.iter().filter(|&&v| v != WHITE).count() > 50);
    let core = (100..200).filter(|&x| c.get(x, 50) == Some(RED)).count();
    assert!(core > 60, "{core}");
}

#[test]
fn watercolor_lighten_darken_mix_once_per_stroke() {
    let mut c = Canvas::new(100, 60);
    c.px.fill(100);
    let mut p = pen(6, 9);
    // vai e volta no mesmo lugar: mistura uma vez só
    stroke(&mut p, &mut c, &[(20, 30), (80, 30), (20, 30)]);
    assert_eq!(c.get(50, 30), Some(55), "metade entre 100 e o vermelho 9");
    assert_eq!(c.get(50, 5), Some(100));
    let mut c = Canvas::new(100, 60);
    c.px.fill(100);
    stroke(&mut pen(6, 11), &mut c, &[(20, 30), (80, 30)]);
    assert_eq!(c.get(50, 30), Some(65), "escurecer 35% para o preto");
    let mut c = Canvas::new(100, 60);
    c.px.fill(100);
    stroke(&mut pen(6, 10), &mut c, &[(20, 30), (80, 30)]);
    assert_ne!(c.get(50, 30), Some(100), "clarear");
    // o clarear não muda o branco
    let mut c = Canvas::new(100, 60);
    stroke(&mut pen(6, 10), &mut c, &[(20, 30), (80, 30)]);
    assert_eq!(painted(&c), 0);
}

#[test]
fn every_option_paints_something() {
    for page in 1..=6u8 {
        for k in 0..12 {
            let mut c = Canvas::new(300, 300);
            c.px.fill(100);
            let mut p = pen(page, k);
            stroke(&mut p, &mut c, &[(40, 40), (100, 120), (200, 130)]);
            if page == 5 && k == 11 {
                stroke(&mut p, &mut c, &[(250, 250), (260, 260)]);
            }
            assert!(
                c.px.iter().any(|&v| v != 100),
                "página {page}, botão {k} não pintou nada"
            );
        }
    }
}

#[test]
fn clip_is_respected() {
    let mut c = Canvas::new(300, 300);
    c.clip = Some((0, 0, 100, 100));
    for (page, k) in [(3, 7), (6, 2), (6, 9), (5, 7), (3, 5)] {
        let mut p = pen(page, k);
        stroke(&mut p, &mut c, &[(50, 50), (150, 150), (250, 250)]);
    }
    for y in 0..300 {
        for x in 0..300 {
            if x >= 100 || y >= 100 {
                assert!(!ink_at(&c, x, y), "({x},{y}) fora do recorte");
            }
        }
    }
}

#[test]
fn real_tiles_load_and_have_ink() {
    let Some(assets) = crate::scenes::testutil::assets() else {
        return;
    };
    let t = Tiles::load(|id| {
        Shape::from_stamp(&assets.image(&crate::assets::ImageRef::Pic(
            crate::assets::Program::Artist,
            id,
        )))
    });
    assert!(t.stamps.iter().all(has_ink));
    assert!(t.feet.iter().all(|s| (s.w, s.h) == (21, 21) && has_ink(s)));
    for set in &t.tracks {
        for v in [&set.h, &set.v, &set.se, &set.sw, &set.nw, &set.ne] {
            assert!(!v.is_empty());
            assert!(v.iter().all(|s| (s.w, s.h) == (21, 21) && has_ink(s)));
        }
    }
}
