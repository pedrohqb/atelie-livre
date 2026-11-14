//! Desempenho por cena: tempo de CPU da thread por quadro (atualizar +
//! desenhar), com o ponteiro passeando e cliques de vez em quando. Ignorado
//! por padrão; rode em release:
//!
//! `ATELIE_DATA=… cargo test --release -p engine --test playtest_perf -- --ignored --nocapture`
//!
//! Mede CPU (não relógio de parede) para não confundir carga da máquina com
//! custo do quadro. O orçamento de um quadro a 60 Hz é 16,7 ms.

mod common;

use common::{Director, Rng, real_assets};
use engine::scene::{Input, SceneId};
use engine::scenes::projects::ProjectKind;
use engine::scenes::wizard::TrickKind;

/// Tempo de CPU desta thread, em ms (`/proc/thread-self/stat`, campos 14 e 15).
fn cpu_ms() -> f64 {
    let s = std::fs::read_to_string("/proc/thread-self/stat").unwrap_or_default();
    let rest = s.rsplit_once(')').map(|(_, r)| r).unwrap_or("");
    let f: Vec<&str> = rest.split_whitespace().collect();
    let tick = |i: usize| f.get(i).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    // depois do ")": estado é o 0; utime é o 11, stime o 12 (relógio de 100 Hz)
    (tick(11) + tick(12)) * 10.0
}

#[test]
#[ignore]
fn frame_cost_per_scene() {
    let Some(a) = real_assets() else { return };
    let frames = 600;
    let scenes = [
        SceneId::Imaginopolis,
        SceneId::Street,
        SceneId::Building,
        SceneId::Studio,
        SceneId::Projects,
        SceneId::TricksFloor,
        SceneId::Gallery,
        SceneId::PaintNew,
        SceneId::Project(ProjectKind::Comics),
        SceneId::Project(ProjectKind::Flipbook),
        SceneId::Tricks(TrickKind::Basic),
    ];
    let mut report = Vec::new();
    for id in scenes {
        let tmp = tempfile::tempdir().unwrap();
        let mut d = Director::new(&a, tmp.path(), id, Some("bia"));
        d.idle(30);
        let mut rng = Rng(42);
        let t0 = cpu_ms();
        let w0 = std::time::Instant::now();
        for k in 0..frames {
            let (x, y) = (rng.range(30, 600), rng.range(120, 440));
            d.step(&[Input::MouseMove { x, y }]);
            if k % 120 == 0 && d.top() != id {
                break;
            }
        }
        let cpu = (cpu_ms() - t0) / frames as f64;
        let wall = w0.elapsed().as_secs_f64() * 1000.0 / frames as f64;
        report.push(format!(
            "{id:?}: {cpu:.1} ms de CPU por quadro ({wall:.1} ms de parede)"
        ));
    }
    eprintln!("{}", report.join("\n"));
}

/// Girar uma palavra FORMA com "Efeito tridimensional": tempo de CPU de cada
/// clique no botão de girar (cada um redesenha a palavra com a sombra 3D).
#[test]
#[ignore]
fn shaped_words_3d_rotate_cost() {
    let Some(a) = real_assets() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let mut d = Director::new(&a, tmp.path(), SceneId::PaintNew, Some("bia"));
    d.idle(5);
    for (x, y) in [(300, 455), (228, 25), (300, 455), (105, 77)] {
        d.click(x, y);
    }
    d.text("Oi gente");
    d.click(416, 378); // efeitos
    d.click(400, 347); // Efeito tridimensional
    let mut costs = Vec::new();
    for _ in 0..5 {
        let t0 = cpu_ms();
        d.click(518, 325);
        costs.push(cpu_ms() - t0);
    }
    eprintln!("girar com 3D: {costs:?} ms de CPU por clique (5 quadros cada)");
}
