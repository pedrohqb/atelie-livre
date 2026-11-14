//! Teste de jogo da tela de pintura pelo diretor de cenas: ferramentas
//! feitas por agentes diferentes usadas em sequência, como uma criança.

mod common;

use common::{Director, real_assets};
use engine::scene::SceneId;

const SND_ROTATE: &str = "Snd(Artist, 540)";

fn sounds(d: &Director, from: usize) -> Vec<String> {
    d.trace_since(from)
        .iter()
        .filter(|l| l.starts_with("TRACE som"))
        .cloned()
        .collect()
}

/// Cada clique num botão de girar do cavalete FORMA gira a palavra (e toca
/// o som), mesmo depois do balão "Para girar" aparecer.
#[test]
fn shaped_easel_rotates_on_every_click() {
    let Some(a) = real_assets() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let mut d = Director::new(&a, tmp.path(), SceneId::PaintNew, Some("bia"));
    d.idle(5);
    d.click(300, 455);
    d.click(228, 25); // Palavra
    d.click(300, 455);
    d.click(105, 77); // FORMA
    d.idle(5);
    for (k, (x, y)) in [(476, 325), (518, 325), (476, 325), (476, 325), (518, 325)]
        .into_iter()
        .enumerate()
    {
        let before = d.snapshot();
        let from = d.trace.len();
        if k % 2 == 0 {
            d.click(x, y);
        } else {
            d.quick_click(x, y);
        }
        let n = sounds(&d, from)
            .iter()
            .filter(|s| s.contains(SND_ROTATE))
            .count();
        assert!(
            n == 1 && d.changed_pixels(&before) > 0,
            "clique {k} em ({x},{y}) não girou: {:?}",
            d.trace_since(from)
        );
    }
}

/// Um bicho animado colocado no quadro termina a animação e vira objeto
/// (senão fica por cima de tudo o que vier depois).
#[test]
fn placed_animal_settles_into_an_object() {
    use engine::scene::Scene;
    use engine::scenes::Paint;
    let Some(a) = real_assets() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let mut p = Paint::new_blank(Some(tmp.path().to_path_buf()), &a);
    let mut step = |p: &mut Paint, i: Vec<engine::scene::Input>| {
        p.update(&i, common::DT, &a);
    };
    let click = |x, y| {
        vec![engine::scene::Input::Click {
            button: engine::scene::MouseButton::Left,
            x,
            y,
        }]
    };
    let up = |x, y| {
        vec![engine::scene::Input::MouseUp {
            button: engine::scene::MouseButton::Left,
            x,
            y,
        }]
    };
    for (x, y) in [
        (300, 455),
        (128, 28),
        (300, 455),
        (50, 78),
        (55, 123),
        (300, 200),
    ] {
        step(&mut p, click(x, y));
        step(&mut p, up(x, y));
    }
    assert!(p.stickers().is_animating(), "a cobra começa a animar");
    let mut frames = 0;
    while p.stickers().is_animating() && frames < 60 * 60 {
        step(&mut p, vec![]);
        frames += 1;
    }
    assert!(
        !p.stickers().is_animating(),
        "a cobra ainda anima depois de {} s",
        frames / 60
    );
    eprintln!("cobra parou em {:.1} s", frames as f32 / 60.0);
}
