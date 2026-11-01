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
