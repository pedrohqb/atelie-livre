//! Camada de objetos, Transformadores, Super-ferramentas e ajuda do McZee
//! pela tela de pintura (seção "Testes" da spec de objetos de 2026-10-31).
//! Roda sem dados do original; os testes que precisam dos balões `WHAT`
//! usam `ATELIE_DATA` se houver.

use engine::assets::Assets;
use engine::paint::store::load_png;
use engine::paint::{ORIGIN, WHITE};
use engine::scene::{Input, MouseButton, Scene};
use engine::scenes::Paint;
use engine::scenes::paint::{Tool, VacuumMode};
use engine::scenes::transformers::{Op, PAGE1, PAGE2, option_rect};
use std::path::PathBuf;

fn empty_assets() -> (tempfile::TempDir, Assets) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
    let a = Assets::open(dir.path()).unwrap();
    (dir, a)
}

#[allow(dead_code)]
fn assets() -> Option<Assets> {
    let dir = std::env::var_os("ATELIE_DATA").map(PathBuf::from)?;
    Some(Assets::open(&dir).expect("ATELIE_DATA válida"))
}

fn click(x: i32, y: i32) -> Input {
    Input::Click {
        button: MouseButton::Left,
        x,
        y,
    }
}

fn mv(x: i32, y: i32) -> Input {
    Input::MouseMove { x, y }
}

fn up(x: i32, y: i32) -> Input {
    Input::MouseUp {
        button: MouseButton::Left,
        x,
        y,
    }
}

/// Pixels não brancos da composição.
fn painted(p: &Paint) -> usize {
    p.composed().px.iter().filter(|&&v| v != WHITE).count()
}

/// Um pixel opaco do objeto, em coordenadas da tela.
fn opaque_point(p: &Paint, id: u32) -> (i32, i32) {
    let o = p.objects().get(id).unwrap();
    let (x, y) = (0..o.h)
        .flat_map(|y| (0..o.w).map(move |x| (x, y)))
        .find(|&(x, y)| o.pixel(x, y).is_some())
        .expect("objeto com pixel opaco");
    (ORIGIN.0 + o.x + x, ORIGIN.1 + o.y + y)
}

/// Coloca o bicho 1 (peixe-espada; o 0 é a cobra, que vai embora) centrado em `(x, y)` da tela
/// e espera a animação acabar: o adesivo vira um objeto.
fn place_animal(p: &mut Paint, a: &Assets, x: i32, y: i32) -> u32 {
    p.update(&[click(129, 30), click(40, 80), click(54, 152)], 0.0, a);
    p.update(&[click(x, y)], 0.0, a);
    p.update(&[], 10.0, a);
    assert!(!p.stickers().is_animating());
    let n = p.objects().len();
    assert!(n > 0);
    p.objects().items[n - 1].id
}

#[test]
fn placed_sticker_is_an_object_saved_flat_and_reopened_editable() {
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
    let id = place_animal(&mut p, &a, 300, 300);
    let o = p.objects().get(id).unwrap().rect();
    // o fundo continua branco; a composição tem o adesivo
    assert_eq!(p.canvas().px.iter().filter(|&&v| v != WHITE).count(), 0);
    assert!(painted(&p) > 0);
    p.leave();
    assert!(dir.path().join("ultimo.objetos.json").exists());
    assert!(dir.path().join("ultimo.fundo.png").exists());
    // PNG achatado
    let flat = load_png(&dir.path().join("ultimo.png"), p.palette()).unwrap();
    assert!(flat.px.iter().any(|&v| v != WHITE));
    // reabre editável, com o objeto no mesmo lugar
    let q = Paint::new(Some(dir.path().to_path_buf()), &a);
    assert_eq!(q.objects().len(), 1);
    assert_eq!(q.objects().items[0].rect(), o);
    assert_eq!(q.canvas().px.iter().filter(|&&v| v != WHITE).count(), 0);
}

#[test]
fn thing_vacuum_removes_only_the_object_and_egg_restores() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new(None, &a);
    // pincelada no fundo
    p.update(
        &[
            click(ORIGIN.0 + 50, ORIGIN.1 + 50),
            mv(ORIGIN.0 + 80, ORIGIN.1 + 50),
            up(ORIGIN.0 + 80, ORIGIN.1 + 50),
        ],
        0.0,
        &a,
    );
    let background = p.canvas().px.iter().filter(|&&v| v != WHITE).count();
    assert!(background > 0);
    let id = place_animal(&mut p, &a, 300, 300);
    // aspirador, aspira-coisas (2.º botão), clique num pixel do objeto
    p.update(&[click(328, 30)], 0.0, &a);
    assert_eq!(p.tool(), Tool::Vacuum);
    p.update(&[click(25 + 37 + 10, 70)], 0.0, &a);
    assert_eq!(p.vacuum_mode(), VacuumMode::Stickers);
    let (cx, cy) = opaque_point(&p, id);
    p.update(&[click(cx, cy), up(cx, cy)], 0.0, &a);
    assert_eq!(p.objects().len(), 0);
    assert_eq!(
        p.canvas().px.iter().filter(|&&v| v != WHITE).count(),
        background
    );
    // o ovo devolve o objeto
    p.update(&[click(270, 30)], 0.0, &a);
    assert_eq!(p.objects().len(), 1);
}

#[test]
fn words_become_an_object_and_vacuum_removes_them() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new(None, &a);
    p.update(&[click(228, 30)], 0.0, &a);
    assert_eq!(p.tool(), Tool::Words);
    p.update(
        &[
            click(200, 200),
            Input::Text("Oi".into()),
            Input::Key {
                key: engine::scene::Key::Enter,
                ctrl: false,
                alt: false,
            },
        ],
        0.0,
        &a,
    );
    assert_eq!(p.objects().len(), 1);
    let o = p.objects().items[0].clone();
    assert_eq!((o.x, o.y), (200 - ORIGIN.0, 200 - ORIGIN.1));
    assert!(matches!(&o.source, engine::paint::Source::Words { text, .. } if text == "Oi"));
    p.update(&[click(328, 30), click(25 + 37 + 10, 70)], 0.0, &a);
    let (sx, sy) = opaque_point(&p, o.id);
    p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
    assert_eq!(p.objects().len(), 0);
}

/// Escolhe a ferramenta Transformadores e a ação `op`.
fn pick_transformer(p: &mut Paint, a: &Assets, op: Op) {
    p.update(&[click(378, 30)], 0.0, a);
    assert_eq!(p.tool(), Tool::Transformers);
    let (page, k) = match PAGE1.iter().position(|b| b.op == op) {
        Some(k) => (1, k),
        None => (2, PAGE2.iter().position(|b| b.op == op).unwrap()),
    };
    while p.transformers().page() != page {
        p.update(&[click(510, 95)], 0.0, a);
    }
    let r = option_rect(k);
    p.update(&[click(r.0 + 10, r.1 + 10)], 0.0, a);
    assert_eq!(p.transformers().op(), op);
}

#[test]
fn moved_object_changes_png_and_json_reopens_at_new_place() {
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
    let id = place_animal(&mut p, &a, 300, 300);
    let before = p.composed();
    let r = p.objects().get(id).unwrap().rect();
    let (sx, sy) = opaque_point(&p, id);
    pick_transformer(&mut p, &a, Op::Move);
    p.update(&[click(sx, sy), mv(sx + 20, sy), up(sx + 20, sy)], 0.0, &a);
    assert_eq!(
        p.objects().get(id).unwrap().rect(),
        (r.0 + 20, r.1, r.2, r.3)
    );
    assert_ne!(p.composed().px, before.px);
    p.leave();
    let flat = load_png(&dir.path().join("ultimo.png"), p.palette()).unwrap();
    assert_eq!(flat.px, p.composed().px);
    let q = Paint::new(Some(dir.path().to_path_buf()), &a);
    assert_eq!(
        q.objects().get(id).unwrap().rect(),
        (r.0 + 20, r.1, r.2, r.3)
    );
}

#[test]
fn rotate_right_four_times_and_flip_twice_restore() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new(None, &a);
    let id = place_animal(&mut p, &a, 300, 300);
    let before = p.objects().get(id).unwrap().clone();
    let flat = p.composed();
    pick_transformer(&mut p, &a, Op::RotateRight);
    for _ in 0..4 {
        let (cx, cy) = {
            let o = p.objects().get(id).unwrap();
            let (cx, cy) = o.center();
            (ORIGIN.0 + cx, ORIGIN.1 + cy)
        };
        // o centro pode ser transparente: usa o primeiro pixel opaco se for
        let (sx, sy) = if p.objects().hit(cx - ORIGIN.0, cy - ORIGIN.1).is_some() {
            (cx, cy)
        } else {
            opaque_point(&p, id)
        };
        p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
    }
    assert_eq!(*p.objects().get(id).unwrap(), before);
    assert_eq!(p.composed().px, flat.px);
    for op in [Op::FlipH, Op::FlipV] {
        pick_transformer(&mut p, &a, op);
        let (sx, sy) = opaque_point(&p, id);
        p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
        assert_ne!(*p.objects().get(id).unwrap(), before);
        let (sx, sy) = opaque_point(&p, id);
        p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
        assert_eq!(*p.objects().get(id).unwrap(), before);
    }
    assert_eq!(p.composed().px, flat.px);
}

#[test]
fn linked_objects_move_together() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new(None, &a);
    let first = place_animal(&mut p, &a, 200, 300);
    let second = place_animal(&mut p, &a, 400, 300);
    assert_ne!(first, second);
    pick_transformer(&mut p, &a, Op::Link);
    let (x1, y1) = opaque_point(&p, first);
    let (x2, y2) = opaque_point(&p, second);
    p.update(
        &[click(x1, y1), up(x1, y1), click(x2, y2), up(x2, y2)],
        0.0,
        &a,
    );
    assert_eq!(p.objects().group(first).len(), 2);
    let r2 = p.objects().get(second).unwrap().rect();
    pick_transformer(&mut p, &a, Op::Move);
    p.update(
        &[click(x1, y1), mv(x1 + 15, y1 - 10), up(x1 + 15, y1 - 10)],
        0.0,
        &a,
    );
    assert_eq!(
        p.objects().get(second).unwrap().rect(),
        (r2.0 + 15, r2.1 - 10, r2.2, r2.3)
    );
}

#[test]
fn approach_vp_for_one_second_shrinks_and_moves_and_png_has_no_guides() {
    use engine::scenes::supertools::{BUTTONS, Op as SOp};
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
    let id = place_animal(&mut p, &a, 150, 350);
    let r0 = p.objects().get(id).unwrap().rect();
    p.update(&[click(430, 30)], 0.0, &a);
    assert_eq!(p.tool(), Tool::Supertools);
    let button = |op: SOp| {
        let b = BUTTONS.iter().find(|b| b.op == op).unwrap();
        let r = option_rect(b.k);
        click(r.0 + 10, r.1 + 10)
    };
    // quadriculado grande + ponto de fuga em (500, 150) da tela
    p.update(
        &[button(SOp::BigGrid), button(SOp::VanishingPoint)],
        0.0,
        &a,
    );
    p.update(&[click(500, 150), up(500, 150)], 0.0, &a);
    let vp = (500 - ORIGIN.0, 150 - ORIGIN.1);
    assert_eq!(p.guides().vp(), Some(vp));
    let dist = |r: (i32, i32, i32, i32)| {
        let (cx, cy) = (r.0 + r.2 / 2, r.1 + r.3 / 2);
        (((cx - vp.0).pow(2) + (cy - vp.1).pow(2)) as f32).sqrt()
    };
    p.update(&[button(SOp::Approach)], 0.0, &a);
    let (sx, sy) = opaque_point(&p, id);
    p.update(&[click(sx, sy)], 0.0, &a);
    for _ in 0..60 {
        p.update(&[], 1.0 / 60.0, &a);
    }
    p.update(&[up(sx, sy)], 0.0, &a);
    let r1 = p.objects().get(id).unwrap().rect();
    assert!(r1.2 < r0.2 && r1.3 < r0.3, "{r0:?} {r1:?}");
    let (d0, d1) = (dist(r0), dist(r1));
    assert!(d1 < d0 - 50.0 && d1 > d0 - 70.0, "{d0} {d1}");
    // o PNG achatado não tem o quadriculado nem o horizonte
    p.leave();
    let flat = load_png(&dir.path().join("ultimo.png"), p.palette()).unwrap();
    assert_eq!(flat.px, p.composed().px);
    let green = flat.px.iter().filter(|&&v| v != WHITE).count();
    assert_eq!(green, painted(&p));
    assert_eq!(flat.get(48, 0), Some(WHITE));
    assert_eq!(flat.get(10, vp.1), Some(WHITE));
}
