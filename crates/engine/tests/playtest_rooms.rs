//! Teste de jogo das salas do prédio pelo diretor de cenas: caminhos entre
//! cenas feitas por agentes diferentes e cliques de criança em máquina lenta.

mod common;

use common::{Director, real_assets};
use engine::scene::{Input, MouseButton, SceneId};

fn click(x: i32, y: i32) -> Input {
    Input::Click {
        button: MouseButton::Left,
        x,
        y,
    }
}

fn up(x: i32, y: i32) -> Input {
    Input::MouseUp {
        button: MouseButton::Left,
        x,
        y,
    }
}

/// Ateliê sem balão de chegada (o primeiro clique fora o fecha).
fn studio(d: &mut Director) {
    d.idle(10);
    d.click(620, 300);
    d.idle(30);
}

/// Duplo clique no cavalete quando os dois cliques chegam no mesmo quadro
/// (mouse rápido, ou a máquina demorou a desenhar): abre o quadro novo.
#[test]
fn easel_double_click_in_one_frame_opens_paint() {
    let Some(a) = real_assets() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let mut d = Director::new(&a, tmp.path(), SceneId::Studio, Some("bia"));
    studio(&mut d);
    d.step(&[click(190, 220), up(190, 220), click(190, 220), up(190, 220)]);
    d.idle(2);
    assert_eq!(d.top(), SceneId::PaintNew, "{:?}", d.trace_since(0));
}

/// Duplo clique no cavalete numa máquina lenta: os dois cliques chegam em
/// quadros seguidos, mas cada quadro levou meio segundo.
#[test]
fn easel_double_click_on_slow_frames_opens_paint() {
    let Some(a) = real_assets() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let mut d = Director::new(&a, tmp.path(), SceneId::Studio, Some("bia"));
    studio(&mut d);
    d.dt = 0.5;
    d.step(&[click(190, 220), up(190, 220)]);
    d.step(&[click(190, 220), up(190, 220)]);
    d.dt = common::DT;
    d.idle(2);
    assert_eq!(d.top(), SceneId::PaintNew, "{:?}", d.trace_since(0));
}
