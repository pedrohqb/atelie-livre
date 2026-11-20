//! Teste de macaco: entradas aleatórias de criança (cliques, arrastos,
//! teclas, texto, esperas) em todas as cenas do Fine Artist e do Creative
//! Writer, seguindo as trocas de cena como o laço do app. Procura pânicos
//! no `update`, no `draw` e no `leave`. Reproduzível pela semente.
//!
//! `PLAYTEST_BURSTS` (padrão 25, rápido para o `cargo test`; a rodada do teste
//! de jogo usou 120) dá o número de rajadas por cena e semente;
//! `PLAYTEST_SEEDS` (padrão 1) o número de sementes; `PLAYTEST_SCENE` limita a
//! uma cena inicial (nome do `Debug` do `SceneId`, ex. `Paint`).

mod common;

use common::{Director, Rng, kid_burst, real_assets};
use engine::scene::{CwScene, Input, SceneId};
use engine::scenes::projects::ProjectKind;
use engine::scenes::room::Floor;
use engine::scenes::wizard::TrickKind;
use engine::writer::assistant::Kind as WriterProject;
use engine::writer::ideas::story::IdeaSource;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn env_num(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn starts() -> Vec<SceneId> {
    vec![
        SceneId::Splash,
        SceneId::Entry,
        SceneId::Story,
        SceneId::Imaginopolis,
        SceneId::Street,
        SceneId::Building,
        SceneId::Studio,
        SceneId::Projects,
        SceneId::TricksFloor,
        SceneId::Gallery,
        SceneId::Paint,
        SceneId::PaintNew,
        SceneId::Project(ProjectKind::Comics),
        SceneId::Project(ProjectKind::Button),
        SceneId::Project(ProjectKind::Sticker),
        SceneId::Project(ProjectKind::Flipbook),
        SceneId::Tricks(TrickKind::Basic),
        SceneId::Tricks(TrickKind::ThreeD),
        SceneId::TricksStudio(TrickKind::Basic, TrickKind::Basic.studio_step()),
        SceneId::TricksStudio(TrickKind::ThreeD, TrickKind::ThreeD.studio_step()),
        // Creative Writer (teste de jogo do Writer)
        SceneId::WriterSplash,
        SceneId::Cw(CwScene::Entry),
        SceneId::Cw(CwScene::Story),
        SceneId::Cw(CwScene::Imaginopolis),
        SceneId::Cw(CwScene::Street),
        SceneId::Cw(CwScene::Building(Floor::Lobby)),
        SceneId::Cw(CwScene::Building(Floor::Studio)),
        SceneId::Cw(CwScene::Building(Floor::Projects)),
        SceneId::Cw(CwScene::Building(Floor::Tricks)),
        SceneId::Cw(CwScene::Building(Floor::Attic)),
        SceneId::Cw(CwScene::Library),
        SceneId::Cw(CwScene::Portfolio),
        SceneId::Cw(CwScene::NewDocument),
        SceneId::Writer,
        SceneId::Cw(CwScene::IdeaMachine),
        SceneId::Cw(CwScene::PictureBoard),
        SceneId::Cw(CwScene::IdeaStory(IdeaSource::Machine)),
        SceneId::Cw(CwScene::Project(WriterProject::News)),
        SceneId::Cw(CwScene::Project(WriterProject::Card)),
        SceneId::Cw(CwScene::Project(WriterProject::Banner)),
        SceneId::Cw(CwScene::ProjectFolder(WriterProject::Card)),
    ]
}

#[test]
fn monkey_never_panics() {
    let Some(assets) = real_assets() else {
        return;
    };
    // nenhuma impressão de verdade
    // SAFETY: os testes deste arquivo não leem o ambiente em outras threads.
    unsafe { std::env::set_var("ATELIE_LP", "true") };
    let bursts = env_num("PLAYTEST_BURSTS", 25);
    let seeds = env_num("PLAYTEST_SEEDS", 1);
    let only = std::env::var("PLAYTEST_SCENE").ok();
    let mut failures = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    for start in &starts() {
        if only.as_deref().is_some_and(|o| o != format!("{start:?}")) {
            continue;
        }
        for seed in 1..=seeds {
            let tmp = tempfile::tempdir().unwrap();
            let mut d = Director::new(&assets, tmp.path(), *start, Some("macaco"));
            let mut rng =
                Rng(0x9E37_79B9_7F4A_7C15 ^ (seed * 7919) ^ (format!("{start:?}").len() as u64));
            let mut recent: Vec<(SceneId, Vec<Input>)> = Vec::new();
            for _ in 0..bursts {
                for frame in kid_burst(&mut rng) {
                    let here = d.top();
                    visited.insert(format!("{here:?}"));
                    recent.push((here, frame.clone()));
                    if recent.len() > 40 {
                        recent.remove(0);
                    }
                    let r = catch_unwind(AssertUnwindSafe(|| {
                        d.step(&frame);
                    }));
                    if r.is_err() {
                        failures.push(format!(
                            "pânico partindo de {start:?} (semente {seed}) em {here:?}; últimos quadros: {recent:?}"
                        ));
                        break;
                    }
                    if d.quit {
                        // sair pelo EPA: recomeça da cena inicial
                        d = Director::new(&assets, tmp.path(), *start, Some("macaco"));
                    }
                }
                if failures.len() > 20 {
                    break;
                }
            }
            if catch_unwind(AssertUnwindSafe(|| d.close())).is_err() {
                failures.push(format!(
                    "pânico no leave partindo de {start:?} (semente {seed})"
                ));
            }
        }
    }
    eprintln!("cenas visitadas pelo macaco: {visited:?}");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
