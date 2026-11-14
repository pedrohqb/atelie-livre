//! Lições dos Truques de Desenho: a apresentação (quadro) e a pintura.

use engine::assets::Assets;
use engine::scene::{Effect, Input, MouseButton, Scene, SceneId};
use engine::scenes::Paint;
use engine::scenes::tricks_basic;
use engine::scenes::tricks_board::TricksBoard;
use engine::scenes::wizard::{BOARD_PANEL, STUDIO_BALLOON, TrickKind};
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

fn studio(kind: TrickKind, dir: Option<PathBuf>, a: &Assets) -> Paint {
    Paint::new_tricks(kind, kind.studio_step(), dir, a)
}

#[test]
fn basic_studio_starts_at_the_training_intro() {
    let Some(a) = data() else { return };
    let dir = tempfile::tempdir().unwrap();
    let mut p = studio(TrickKind::Basic, Some(dir.path().to_path_buf()), &a);
    let w = p.wizard().unwrap();
    assert_eq!(w.current(), tricks_basic::STUDIO);
    // "Seja bem-vindo ao meu ateliê de treinamento!", encostado à direita
    assert_eq!(w.step().record, (0x3800, 3));
    let b = w.balloon(&a, "").unwrap();
    assert_eq!(b.x + b.w, STUDIO_BALLOON.0);
    assert_eq!(b.y, STUDIO_BALLOON.1);
    assert!(!w.has_back());
    // a figura pequena do Chiquinho no painel da Maggie
    assert_eq!(w.maggie_panel(), Some(0x3865));
    let mut f = engine::Frame::new(engine::SCREEN_W, engine::SCREEN_H);
    p.draw(&mut f, &a);
    // molduras laranja: "Sua Área de Desenho" (x 28) e a da Maggie (x 428)
    assert_eq!(f.pixel(28, 300)[..3], [255, 140, 50]);
    assert_eq!(f.pixel(428, 380)[..3], [255, 140, 50]);
    next(&mut p, &a);
    assert_eq!(p.wizard().unwrap().step().record, (0x3800, 1));
    // um traço no quadro
    let _ = p.update(
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
    // a Maggie da porta fora de hora pergunta o que fazer (`3803[0]`)
    let fx = click(&mut p, &a, 600, 60);
    assert!(!fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))));
    assert!(p.wizard().unwrap().quitting());
    // "Parar de trabalhar aqui" volta ao andar e a lição grava "truque-…png"
    let fx = option(&mut p, &a, 1);
    assert!(
        fx.contains(&Effect::ChangeScene(SceneId::TricksFloor)),
        "{fx:?}"
    );
    p.leave();
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().any(|n| n.starts_with("truque-")), "{names:?}");
    assert!(!names.iter().any(|n| n.starts_with("ultimo")), "{names:?}");
}

#[test]
fn three_d_studio_starts_at_its_intro() {
    let Some(a) = data() else { return };
    let p = studio(TrickKind::ThreeD, None, &a);
    assert_eq!(p.wizard().unwrap().step().record, (0x4000, 50));
}

#[test]
fn lesson_without_data_draws() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("ARTIST.EXE"), b"MZ").unwrap();
    let a = Assets::open(d.path()).unwrap();
    let p = studio(TrickKind::Basic, None, &a);
    let mut f = engine::Frame::new(engine::SCREEN_W, engine::SCREEN_H);
    p.draw(&mut f, &a);
    let b = TricksBoard::new(TrickKind::ThreeD, None);
    b.draw(&mut f, &a);
}

fn board_click(b: &mut TricksBoard, a: &Assets, x: i32, y: i32) -> Vec<Effect> {
    let mut fx = b.update(
        &[Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }],
        0.05,
        a,
    );
    fx.extend(b.update(
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

fn board_next(b: &mut TricksBoard, a: &Assets) {
    let bal = b.wizard().balloon(a, "").unwrap();
    let y = bal.y + bal.height(a) + 4;
    let n = if b.wizard().has_back() { 2 } else { 1 };
    board_click(b, a, bal.x + 12 + 41 * n + 10, y + 6);
}

/// Leva a apresentação básica ao passo 8 ("Oba! … Desafio", com a lâmpada).
fn basic_to_step_8(b: &mut TricksBoard, a: &Assets) {
    let r = b.wizard().balloon(a, "").unwrap().option_rect(a, 0);
    board_click(b, a, r.0 + 6, r.1 + 4);
    board_next(b, a);
    board_next(b, a);
    let (px, py, _, _) = BOARD_PANEL;
    board_click(b, a, px + 150, py + 140);
    board_next(b, a);
    board_click(b, a, px + 80, py + 136);
    for (x, y, w, h) in tricks_basic::PIG_CIRCLES {
        board_click(b, a, px + x + w / 2, py + y + h / 2);
    }
    board_next(b, a);
    assert_eq!(b.wizard().current(), 8);
}

#[test]
fn lamp_opens_the_tip_and_a_click_outside_only_closes_it() {
    let Some(a) = data() else { return };
    let mut b = TricksBoard::new(TrickKind::Basic, None);
    basic_to_step_8(&mut b, &a);
    let bal = b.wizard().balloon(&a, "").unwrap();
    assert!(!bal.links.is_empty(), "o passo 8 tem a dica Desafio");
    let r = bal.link_rect(&a, 0);
    board_click(&mut b, &a, r.0 + 20, r.1 + r.3 / 2);
    assert!(b.help().is_some(), "a lâmpada abre a dica");
    assert_eq!(b.wizard().current(), 8);
    // o clique fora só fecha a dica
    board_click(&mut b, &a, 30, 440);
    assert!(b.help().is_none());
    assert_eq!(b.wizard().current(), 8);
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
