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

/// Um desenho com adesivo, guardado com nome ("A"), e depois um quadro novo
/// como último desenho. Devolve a pasta de desenhos.
fn drawing_with_sticker_then_another(a: &engine::assets::Assets, root: &std::path::Path) -> std::path::PathBuf {
    use engine::scene::Scene;
    let mut d = Director::new(a, root, SceneId::PaintNew, Some("bia"));
    let dir = d.drawings();
    d.idle(3);
    for (x, y) in [(300, 455), (128, 28), (300, 455), (95, 78), (95, 123), (200, 300)] {
        d.click(x, y);
    }
    d.click(300, 467); // nome do quadro
    d.text("A");
    d.key(engine::scene::Key::Enter);
    d.idle(3);
    d.close();
    assert!(dir.join("A.objetos.json").exists(), "o adesivo não foi guardado com o desenho");
    // um quadro novo, pintado e guardado como último desenho
    let mut p = engine::scenes::Paint::new_blank(Some(dir.clone()), a);
    for i in [
        click(300, 455),
        up(300, 455),
        click(100, 200),
        Input::MouseMove { x: 300, y: 300 },
        up(300, 300),
    ] {
        p.update(&[i], common::DT, a);
    }
    p.leave();
    assert!(!dir.join("ultimo.objetos.json").exists());
    std::fs::write(dir.join("galeria.txt"), format!("{}\n\n\n", dir.join("A.png").display())).unwrap();
    dir
}

/// Abrir pela galeria (clique duplo na moldura) um desenho com adesivos leva
/// junto os objetos: eles continuam editáveis na pintura.
#[test]
fn gallery_open_keeps_objects_editable() {
    let Some(a) = real_assets() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let dir = drawing_with_sticker_then_another(&a, tmp.path());
    let mut d = Director::new(&a, tmp.path(), SceneId::Gallery, Some("bia"));
    d.idle(30);
    d.click(450, 440); // fecha o balão de chegada
    d.double_click(195, 115);
    assert!(d.wait_for(SceneId::Paint, 10), "{:?}", d.trace_since(0));
    let p = engine::scenes::Paint::new(Some(dir.clone()), &a);
    assert_eq!(p.objects().len(), 1, "o adesivo não voltou como objeto");
}

/// Jogar um desenho no lixo leva junto os objetos, o fundo e o `.mag`.
#[test]
fn gallery_trash_removes_companion_files() {
    let Some(a) = real_assets() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let dir = drawing_with_sticker_then_another(&a, tmp.path());
    let mut d = Director::new(&a, tmp.path(), SceneId::Gallery, Some("bia"));
    d.idle(30);
    d.click(450, 440);
    d.drag((195, 115), (570, 405), 12);
    d.idle(10);
    d.click(191, 258); // "Jogue no lixo!"
    d.idle(10);
    let left: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.starts_with("A."))
        .collect();
    assert!(left.is_empty(), "sobraram arquivos do desenho jogado fora: {left:?}");
}
