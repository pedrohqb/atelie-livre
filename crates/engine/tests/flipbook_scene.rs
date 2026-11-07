//! Desenho animado pela tela de pintura (só a trait `Scene`): abrir a pasta,
//! colocar três figuras, tocar a 1000 ms e guardar ao sair.

use engine::assets::{Assets, Program};
use engine::paint::store::save_png;
use engine::paint::{CANVAS_H, CANVAS_W, Canvas};
use engine::scene::{Effect, Input, MouseButton, Scene, SceneId};
use engine::scenes::Paint;
use engine::scenes::projects::ProjectKind;
use engine::scenes::projects::flipbook::{self, Flipbook, GIF_NAME, JSON_NAME, LAST_DIR, PLAY};
use std::path::PathBuf;

/// Assets reais se `ATELIE_DATA` existir, senão um diretório só com um ARTIST.EXE vazio.
fn assets() -> (tempfile::TempDir, Assets) {
    let dir = tempfile::tempdir().unwrap();
    match std::env::var_os("ATELIE_DATA").map(PathBuf::from) {
        Some(d) => (dir, Assets::open(&d).unwrap()),
        None => {
            std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
            let a = Assets::open(dir.path()).unwrap();
            (dir, a)
        }
    }
}

fn click(x: i32, y: i32) -> Input {
    Input::Click {
        button: MouseButton::Left,
        x,
        y,
    }
}

fn figure(v: u8) -> Canvas {
    let mut c = Canvas::new(CANVAS_W, CANVAS_H);
    c.px.fill(v);
    c
}

#[test]
fn three_figures_play_and_save() {
    let (_d, a) = assets();
    let pal = a.palettes(Program::Artist).pal256.clone();
    let dir = tempfile::tempdir().unwrap();
    // três desenhos na pasta (a galeria lista do mais novo ao mais velho)
    for (k, v) in [(1u8, 20u8), (2, 30), (3, 40)] {
        save_png(
            &figure(v),
            &pal,
            &dir.path().join(format!("desenho-{k}.png")),
        )
        .unwrap();
    }
    let mut p = Paint::new_project(ProjectKind::Flipbook, Some(dir.path().to_path_buf()), &a);
    assert!(p.flipbook().is_some());
    // abre a pasta e clica nas três primeiras figuras
    p.update(
        &[click(
            flipbook::FOLDER_BTN.0 + 3,
            flipbook::FOLDER_BTN.1 + 3,
        )],
        0.0,
        &a,
    );
    assert!(p.flipbook().unwrap().folder_open());
    for k in 0..3 {
        let r = Flipbook::folder_cell_rect(k);
        p.update(&[click(r.0 + 4, r.1 + 4)], 0.0, &a);
    }
    let fb = p.flipbook().unwrap();
    assert_eq!(fb.count(), 3);
    assert_eq!(fb.interval_ms(), 1000);
    // TOCAR: figura 2 entre 1,0 e 2,0 s
    p.update(&[click(PLAY.0 + 3, PLAY.1 + 3)], 0.0, &a);
    assert_eq!(p.flipbook().unwrap().shown(), Some(0));
    p.update(&[], 0.9, &a);
    assert_eq!(p.flipbook().unwrap().shown(), Some(0));
    p.update(&[], 0.2, &a);
    assert_eq!(p.flipbook().unwrap().shown(), Some(1));
    p.update(&[], 0.8, &a);
    assert_eq!(p.flipbook().unwrap().shown(), Some(1));
    p.update(&[], 0.2, &a);
    assert_eq!(p.flipbook().unwrap().shown(), Some(2));
    // desenha sem explodir, tocando e parado
    let mut f = engine::Frame::new(640, 480);
    p.draw(&mut f, &a);
    p.update(&[click(300, 300)], 0.0, &a);
    assert!(p.flipbook().unwrap().shown().is_none());
    p.draw(&mut f, &a);
    // sair guarda a pasta com data e a cópia "ultimo-animacao"
    let fx = p.leave();
    assert!(
        fx.iter()
            .any(|e| matches!(e, Effect::Log(m) if m.contains("guardado"))),
        "{fx:?}"
    );
    let named: Vec<PathBuf> = std::fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("animacao-"))
        })
        .collect();
    assert_eq!(named.len(), 1);
    for d in [named[0].clone(), dir.path().join(LAST_DIR)] {
        for n in [
            "figura-1.png",
            "figura-2.png",
            "figura-3.png",
            JSON_NAME,
            GIF_NAME,
        ] {
            assert!(d.join(n).is_file(), "{}", d.join(n).display());
        }
    }
    // entrar de novo reabre as três figuras
    let p2 = Paint::new_project(ProjectKind::Flipbook, Some(dir.path().to_path_buf()), &a);
    assert_eq!(p2.flipbook().unwrap().count(), 3);
    // a porta de saída volta ao andar dos Projetos
    let mut p2 = p2;
    let fx = p2.update(&[click(600, 60)], 0.0, &a);
    assert!(fx.contains(&Effect::ChangeScene(SceneId::Projects)));
}

/// Clique duplo numa figura pede a tela de pintura por cima; ela abre com a
/// figura, a porta de saída grava e volta; o desenho animado reabre a figura.
#[test]
fn double_click_edits_a_figure_in_the_paint_scene() {
    let (_d, a) = assets();
    let pal = a.palettes(Program::Artist).pal256.clone();
    let dir = tempfile::tempdir().unwrap();
    save_png(&figure(20), &pal, &dir.path().join("desenho-1.png")).unwrap();
    let drawings = Some(dir.path().to_path_buf());
    let mut p = Paint::new_project(ProjectKind::Flipbook, drawings.clone(), &a);
    p.update(
        &[click(
            flipbook::FOLDER_BTN.0 + 3,
            flipbook::FOLDER_BTN.1 + 3,
        )],
        0.0,
        &a,
    );
    let r = Flipbook::folder_cell_rect(0);
    p.update(&[click(r.0 + 4, r.1 + 4)], 0.0, &a);
    p.update(
        &[click(
            flipbook::FOLDER_CLOSE.0 + 2,
            flipbook::FOLDER_CLOSE.1 + 2,
        )],
        0.0,
        &a,
    );
    assert_eq!(p.flipbook().unwrap().count(), 1);
    // o nicho do McZee abre a apresentação da Maggie (sem travar sem dados)
    p.update(&[click(550, 80)], 0.0, &a);
    let mut f = engine::Frame::new(640, 480);
    p.draw(&mut f, &a);
    // clique duplo na figura 1
    let c = Flipbook::cell_rect(0);
    p.update(&[click(c.0 + 10, c.1 + 10)], 0.0, &a);
    let fx = p.update(&[click(c.0 + 10, c.1 + 10)], 0.05, &a);
    assert!(
        fx.contains(&Effect::Push(SceneId::FlipbookFigure)),
        "{fx:?}"
    );
    assert_eq!(p.flipbook().unwrap().editing(), Some(0));
    // a tela de pintura por cima abre com a figura e grava ao sair pela porta
    let mut e = Paint::new_flipbook_figure(drawings.clone(), &a);
    assert!(e.edit_file().is_some());
    assert_eq!(e.canvas().px[0], 20);
    let mut f = engine::Frame::new(640, 480);
    e.draw(&mut f, &a);
    e.update(&[click(600, 60)], 0.0, &a);
    let fx = e.update(&[click(600, 60)], 0.0, &a);
    assert!(fx.contains(&Effect::Back), "{fx:?}");
    // simula uma modificação e a saída
    save_png(&figure(33), &pal, e.edit_file().unwrap()).unwrap();
    let e2 = Paint::new_flipbook_figure(drawings, &a);
    assert_eq!(e2.canvas().px[0], 33);
    let mut e2 = e2;
    let fx = e2.leave();
    assert!(
        fx.iter()
            .any(|e| matches!(e, Effect::Log(m) if m.contains("gravada"))),
        "{fx:?}"
    );
    // de volta: a figura modificada entra no lugar
    p.update(&[], 0.0, &a);
    let fb = p.flipbook().unwrap();
    assert_eq!(fb.editing(), None);
    assert_eq!(fb.slots[0].as_ref().unwrap().canvas.px[0], 33);
    assert!(fb.needs_save());
    assert!(!Flipbook::edit_path(dir.path()).exists());
}
