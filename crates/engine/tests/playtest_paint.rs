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
    let step = |p: &mut Paint, i: Vec<engine::scene::Input>| {
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

/// Clica (aperta e solta em quadros seguidos) na pintura.
fn tap(p: &mut engine::scenes::Paint, a: &engine::assets::Assets, x: i32, y: i32) {
    use engine::scene::{Input, MouseButton, Scene};
    p.update(&[Input::MouseMove { x, y }], common::DT, a);
    p.update(
        &[Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }],
        common::DT,
        a,
    );
    p.update(
        &[Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        }],
        common::DT,
        a,
    );
    for _ in 0..3 {
        p.update(&[], common::DT, a);
    }
}

/// Coloca um adesivo de clip-art (rolo 2, o primeiro do filme) em `(x, y)` e
/// devolve o centro dele na tela.
fn place_clipart(p: &mut engine::scenes::Paint, a: &engine::assets::Assets, x: i32, y: i32) -> (i32, i32) {
    let n = p.objects().len();
    tap(p, a, 128, 28); // Seleciona-adesivos
    tap(p, a, 95, 78); // rolo 2
    tap(p, a, 95, 123); // primeiro adesivo do filme (embaixo do rolo)
    tap(p, a, x, y);
    assert_eq!(p.objects().len(), n + 1, "o adesivo não foi colocado");
    let r = p.objects().records().last().cloned().unwrap();
    let o = engine::paint::ORIGIN;
    (o.0 + r.x + r.w / 2, o.1 + r.y + r.h / 2)
}

/// Seleciona-sons associa um som ao adesivo; o Remove-sons do aspirador o
/// tira (e o ovo devolve).
#[test]
fn vacuum_removes_the_sound_of_a_sticker() {
    let Some(a) = real_assets() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let mut p = engine::scenes::Paint::new_blank(Some(tmp.path().to_path_buf()), &a);
    tap(&mut p, &a, 300, 455);
    let (x, y) = place_clipart(&mut p, &a, 200, 300);
    tap(&mut p, &a, 170, 25); // Seleciona-sons
    tap(&mut p, &a, 290, 77); // categoria "Dinossauro"
    tap(&mut p, &a, 60, 313); // um som da lista
    tap(&mut p, &a, x, y); // associa ao adesivo
    let sound = |p: &engine::scenes::Paint| p.objects().records().last().unwrap().sound.clone();
    assert!(sound(&p).is_some(), "o som não foi associado ao adesivo");
    tap(&mut p, &a, 325, 25); // aspirador
    tap(&mut p, &a, 25 + 37 * 6 + 16, 79); // Remove-sons
    tap(&mut p, &a, x, y);
    assert!(sound(&p).is_none(), "o Remove-sons não tirou o som do adesivo");
    tap(&mut p, &a, 275, 25); // ovo
    assert!(sound(&p).is_some(), "o ovo não devolveu o som");
}
