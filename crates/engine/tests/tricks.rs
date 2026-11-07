//! Lições dos Truques de Desenho na tela de pintura.

use engine::assets::Assets;
use engine::scene::{Effect, Input, MouseButton, Scene, SceneId};
use engine::scenes::Paint;
use engine::scenes::wizard::{MAGGIE_PANEL, TrickKind};
use std::path::PathBuf;

fn data() -> Option<Assets> {
    let d = PathBuf::from(std::env::var_os("ATELIE_DATA")?);
    Assets::open(&d).ok()
}

fn click(p: &mut Paint, a: &Assets, x: i32, y: i32) -> Vec<Effect> {
    let mut fx = p.update(
        &[Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }],
        0.05,
        a,
    );
    fx.extend(p.update(
        &[Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        }],
        0.05,
        a,
    ));
    fx
}

fn option(p: &mut Paint, a: &Assets, k: usize) -> Vec<Effect> {
    let r = p
        .wizard()
        .unwrap()
        .balloon(a, "")
        .unwrap()
        .option_rect(a, k);
    click(p, a, r.0 + 6, r.1 + 4)
}

fn next(p: &mut Paint, a: &Assets) -> Vec<Effect> {
    let b = p.wizard().unwrap().balloon(a, "").unwrap();
    let y = b.y + b.height(a) + 4;
    // Segue é o último botão: Volta (se houver), Pára, Segue a cada 41 px
    let n = if p.wizard().unwrap().has_back() { 2 } else { 1 };
    click(p, a, b.x + 12 + 41 * n + 10, y + 6)
}

#[test]
fn basic_lesson_runs_and_paints_the_guide() {
    let Some(a) = data() else { return };
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new_tricks(TrickKind::Basic, Some(dir.path().to_path_buf()), &a);
    assert_eq!(p.wizard().unwrap().current(), 0);
    option(&mut p, &a, 0);
    assert_eq!(p.wizard().unwrap().current(), 1);
    next(&mut p, &a);
    next(&mut p, &a);
    assert_eq!(p.wizard().unwrap().current(), 3);
    assert!(p.wizard().unwrap().maggie_panel().is_some());
    // clique no porco, no painel da Maggie
    click(&mut p, &a, MAGGIE_PANEL.0 + 150, MAGGIE_PANEL.1 + 140);
    assert_eq!(p.wizard().unwrap().current(), 4);
    // o passo 4 pinta as formas claras no quadro da criança
    assert!(p.composed().px.iter().any(|&i| i != 0));
    // um traço no quadro avisa a lição; o Segue continua
    let fx = p.update(
        &[
            Input::Click {
                button: MouseButton::Left,
                x: 60,
                y: 420,
            },
            Input::MouseMove { x: 120, y: 430 },
            Input::MouseUp {
                button: MouseButton::Left,
                x: 120,
                y: 430,
            },
        ],
        0.05,
        &a,
    );
    assert!(fx.iter().all(|e| !matches!(e, Effect::ChangeScene(_))));
    next(&mut p, &a);
    assert_eq!(p.wizard().unwrap().current(), 5);
    // a porta leva de volta ao andar dos Truques e a lição grava "truque-…png"
    let fx = click(&mut p, &a, 600, 60);
    assert!(fx.contains(&Effect::ChangeScene(SceneId::TricksFloor)));
    p.leave();
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().any(|n| n.starts_with("truque-")), "{names:?}");
    assert!(!names.iter().any(|n| n.starts_with("ultimo")), "{names:?}");
}

#[test]
fn stop_and_free_paint_end_the_lesson() {
    let Some(a) = data() else { return };
    let mut p = Paint::new_tricks(TrickKind::ThreeD, None, &a);
    // "Levar você para o lugar onde você pode treinar?" = pintar por conta própria
    option(&mut p, &a, 1);
    assert!(p.wizard().is_none());
}

#[test]
fn lesson_without_data_draws() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("ARTIST.EXE"), b"MZ").unwrap();
    let a = Assets::open(d.path()).unwrap();
    let p = Paint::new_tricks(TrickKind::Basic, None, &a);
    let mut f = engine::Frame::new(engine::SCREEN_W, engine::SCREEN_H);
    p.draw(&mut f, &a);
}

/// Leva a lição básica ao passo 8 ("Oba! … Desafio", com a lâmpada).
fn basic_to_step_8(p: &mut Paint, a: &Assets) {
    option(p, a, 0);
    next(p, a);
    next(p, a);
    click(p, a, MAGGIE_PANEL.0 + 150, MAGGIE_PANEL.1 + 140);
    next(p, a);
    click(p, a, MAGGIE_PANEL.0 + 80, MAGGIE_PANEL.1 + 136);
    for (x, y, w, h) in engine::scenes::tricks_basic::PIG_CIRCLES {
        click(p, a, MAGGIE_PANEL.0 + x + w / 2, MAGGIE_PANEL.1 + y + h / 2);
    }
    next(p, a);
    assert_eq!(p.wizard().unwrap().current(), 8);
}

#[test]
fn lamp_opens_the_tip_and_a_click_outside_only_closes_it() {
    let Some(a) = data() else { return };
    let mut p = Paint::new_tricks(TrickKind::Basic, None, &a);
    basic_to_step_8(&mut p, &a);
    let b = p.wizard().unwrap().balloon(&a, "").unwrap();
    assert!(!b.links.is_empty(), "o passo 8 tem a dica Desafio");
    let r = b.link_rect(&a, 0);
    click(&mut p, &a, r.0 + 20, r.1 + r.3 / 2);
    assert!(p.help().is_some(), "a lâmpada abre a dica");
    assert_eq!(p.wizard().unwrap().current(), 8);
    // o clique fora fecha a dica e não pinta
    let before = p.composed();
    click(&mut p, &a, 560, 420);
    assert!(p.help().is_none());
    assert!(
        p.composed().px == before.px,
        "o clique que fecha a dica pintou"
    );
}

#[test]
fn balloon_figure_takes_the_height_of_the_icon() {
    let Some(a) = data() else { return };
    use engine::balloon::{Balloon, FIGURE};
    let with = Balloon {
        x: 0,
        y: 0,
        w: 200,
        text: format!("Então, clique até ver\n{FIGURE}"),
        options: Vec::new(),
        links: Vec::new(),
        figures: vec![engine::scenes::tricks_3d::ICON_LINE_BRUSH],
    };
    let without = Balloon {
        text: "Então, clique até ver".into(),
        figures: Vec::new(),
        ..with.clone()
    };
    // o ícone do pincel Linha tem 32 px de altura
    assert!(
        with.height(&a) >= without.height(&a) + 32,
        "{}",
        with.height(&a)
    );
}
