//! Oficina de Projetos pela tela de pintura: história em quadrinhos e
//! botão/adesivo (seção "Testes" da spec de 2026-10-31). Roda sem dados do
//! original; o último teste confere os dígitos no `ARTIST.PIC` se houver
//! `ATELIE_DATA`.

use engine::assets::{Assets, ImageRef, Program};
use engine::paint::store::load_png;
use engine::paint::{BLACK, CANVAS_H, CANVAS_W, Canvas, ORIGIN, WHITE};
use engine::scene::{Effect, Input, MouseButton, Scene};
use engine::scenes::Paint;
use engine::scenes::projects::button::{Kind, PAGE_H, PAGE_W, card_rect};
use engine::scenes::projects::comics::{LINE, digit_rect, layout};
use engine::scenes::projects::{Project, ProjectKind};
use std::path::{Path, PathBuf};

fn empty_assets() -> (tempfile::TempDir, Assets) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
    let a = Assets::open(dir.path()).unwrap();
    (dir, a)
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

/// Traço do pincel de `(x0, y0)` a `(x1, y1)`, coordenadas do quadro.
fn stroke(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Input> {
    vec![
        click(ORIGIN.0 + x0, ORIGIN.1 + y0),
        mv(ORIGIN.0 + x1, ORIGIN.1 + y1),
        up(ORIGIN.0 + x1, ORIGIN.1 + y1),
    ]
}

fn painted(c: &Canvas) -> Vec<(i32, i32)> {
    c.px.iter()
        .enumerate()
        .filter(|(_, v)| **v != WHITE)
        .map(|(k, _)| ((k as u32 % c.w) as i32, (k as u32 / c.w) as i32))
        .collect()
}

fn files(dir: &Path, prefix: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(prefix))
        .collect();
    v.sort();
    v
}

#[test]
fn comics_four_panels_clip_to_panel_and_png_has_no_frames() {
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new_project(ProjectKind::Comics, Some(dir.path().to_path_buf()), &a);
    assert!(matches!(p.project(), Some(Project::Comics(c)) if c.asking && c.panels == 1));
    // escolhe 4 quadrinhos no painel da faixa
    let r = digit_rect(3);
    p.update(&[click(r.0 + 10, r.1 + 10)], 0.0, &a);
    assert!(matches!(p.project(), Some(Project::Comics(c)) if !c.asking && c.panels == 4));
    // traço que começa no 1.º quadrinho e cruza a calha até o 2.º
    let (x, y, w, _) = layout(4)[0];
    let gutter = x + w; // 293
    let right = layout(4)[1].0; // 301
    p.update(&stroke(gutter - 20, y + 60, right + 30, y + 60), 0.0, &a);
    let pts = painted(p.canvas());
    assert!(!pts.is_empty());
    assert!(pts.iter().all(|&(px, _)| px < gutter - LINE), "{pts:?}");
    assert!(pts.iter().any(|&(px, _)| px >= gutter - LINE - 12));
    // o traço que começa na calha não pinta nada
    let before = pts.len();
    p.update(&stroke(gutter + 3, y + 100, gutter + 3, y + 140), 0.0, &a);
    assert_eq!(painted(p.canvas()).len(), before);
    // guardar: o PNG tem o desenho, sem as molduras; o último reabre
    let fx = p.leave();
    assert!(matches!(&fx[..], [Effect::Log(s)] if s.contains("guardado")));
    assert_eq!(files(dir.path(), "ultimo-").len(), 1);
    assert_eq!(files(dir.path(), "quadrinhos-").len(), 1);
    let pal = a.palettes(Program::Artist).pal256.clone();
    let saved = load_png(&dir.path().join("ultimo-quadrinhos.png"), &pal).unwrap();
    assert_eq!(painted(&saved), painted(p.canvas()));
    for (fx0, fy0, fw, fh) in layout(4) {
        for xx in fx0..fx0 + fw {
            assert_eq!(saved.get(xx, fy0), Some(WHITE));
            assert_eq!(saved.get(xx, fy0 + fh - 1), Some(WHITE));
        }
    }
    let q = Paint::new_project(ProjectKind::Comics, Some(dir.path().to_path_buf()), &a);
    assert_eq!(painted(q.canvas()), painted(&saved));
}

#[test]
fn comics_print_includes_frames_and_niche_reopens_panel() {
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new_project(ProjectKind::Comics, Some(dir.path().to_path_buf()), &a);
    let r = digit_rect(1);
    p.update(&[click(r.0 + 10, r.1 + 10)], 0.0, &a);
    p.update(&[click(479, 30)], 0.0, &a);
    let name = files(dir.path(), "impressao-").pop().unwrap();
    let pal = a.palettes(Program::Artist).pal256.clone();
    let page = load_png(&dir.path().join(name), &pal).unwrap();
    let (x, y, w, _) = layout(2)[0];
    assert_eq!(page.get(x, y), Some(BLACK));
    assert_eq!(page.get(x + w - 1, y + 50), Some(BLACK));
    assert_eq!(page.get(x + 50, y + 50), Some(WHITE));
    // o quadro em si continua sem moldura
    assert_eq!(p.canvas().get(x, y), Some(WHITE));
    // o nicho reabre o painel e trocar o número não apaga nada
    p.update(&stroke(x + 40, y + 40, x + 60, y + 40), 0.0, &a);
    let n = painted(p.canvas()).len();
    p.update(&[click(540, 70)], 0.0, &a);
    assert!(matches!(p.project(), Some(Project::Comics(c)) if c.asking));
    let r = digit_rect(0);
    p.update(&[click(r.0 + 10, r.1 + 10)], 0.0, &a);
    assert!(matches!(p.project(), Some(Project::Comics(c)) if c.panels == 1));
    assert_eq!(painted(p.canvas()).len(), n);
}

#[test]
fn button_stroke_across_border_paints_only_inside() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new_project(ProjectKind::Button, None, &a);
    assert!(matches!(p.project(), Some(Project::Button(b)) if b.kind() == Kind::Round));
    let (cx, cy) = (CANVAS_W as i32 / 2, CANVAS_H as i32 / 2);
    p.update(&stroke(cx + 60, cy, cx + 220, cy), 0.0, &a);
    let pts = painted(p.canvas());
    assert!(!pts.is_empty());
    let inner = 150.0 - 3.0 - 12.0;
    for &(x, y) in &pts {
        let (dx, dy) = (x as f32 + 0.5 - cx as f32, y as f32 + 0.5 - cy as f32);
        assert!(
            (dx * dx + dy * dy).sqrt() < inner,
            "({x}, {y}) fora do botão"
        );
    }
    assert!(pts.iter().any(|&(x, _)| x > cx + 120));
}

#[test]
fn button_print_sheet_has_six_copies_and_sticker_twelve() {
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let pal = a.palettes(Program::Artist).pal256.clone();
    let mut p = Paint::new_project(ProjectKind::Button, Some(dir.path().to_path_buf()), &a);
    let (cx, cy) = (CANVAS_W as i32 / 2, CANVAS_H as i32 / 2);
    p.update(&stroke(cx, cy, cx, cy), 0.0, &a);
    p.update(&[click(479, 30)], 0.0, &a);
    let name = files(dir.path(), "impressao-").pop().unwrap();
    let (w, h, px) = read_png(&dir.path().join(&name));
    assert_eq!((w, h), (PAGE_W, PAGE_H));
    let black = pal[BLACK as usize];
    let is_black = |x: u32, y: u32| {
        let i = ((y * w + x) * 3) as usize;
        px[i..i + 3] == [black.r, black.g, black.b]
    };
    // seis centros pintados, um por célula de 413×389
    let mut n = 0;
    for r in 0..3 {
        for c in 0..2 {
            let (x, y) = (c * 413 + 206, r * 389 + 194);
            if (0..3).any(|dx| (0..3).any(|dy| is_black(x + dx - 1, y + dy - 1))) {
                n += 1;
            }
        }
    }
    assert_eq!(n, 6);
    // adesivo: 3×4 cópias reduzidas (outra pasta: o nome leva só a hora)
    let dir = tempfile::tempdir().unwrap();
    let mut s = Paint::new_project(ProjectKind::Sticker, Some(dir.path().to_path_buf()), &a);
    assert!(matches!(s.project(), Some(Project::Button(b)) if b.kind() == Kind::Sticker));
    s.update(&[click(479, 30)], 0.0, &a);
    let name = files(dir.path(), "impressao-").pop().unwrap();
    let (w, h, px) = read_png(&dir.path().join(name));
    assert_eq!((w, h), (PAGE_W, PAGE_H));
    let mut rows = 0;
    for r in 0..4 {
        let y = r * (PAGE_H / 4) + PAGE_H / 8;
        let blacks = (0..w)
            .filter(|&x| px[((y * w + x) * 3) as usize..][..3] == [black.r, black.g, black.b])
            .count();
        // 3 cópias × 2 bordas de ~2 px
        if blacks >= 9 {
            rows += 1;
        }
    }
    assert_eq!(rows, 4);
}

#[test]
fn button_switch_shape_whitens_outside_and_saves_by_kind() {
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new_project(ProjectKind::Button, Some(dir.path().to_path_buf()), &a);
    let (cx, cy) = (CANVAS_W as i32 / 2, CANVAS_H as i32 / 2);
    // canto do botão quadrado: fora do redondo
    let r = card_rect(1);
    p.update(&[click(r.0 + 5, r.1 + 5)], 0.0, &a);
    assert!(matches!(p.project(), Some(Project::Button(b)) if b.kind() == Kind::Square));
    p.update(&stroke(cx + 120, cy - 120, cx + 120, cy - 120), 0.0, &a);
    let n = painted(p.canvas()).len();
    assert!(n > 0);
    // abre o painel pelo nicho e volta ao redondo: o canto some; o ovo desfaz
    p.update(&[click(540, 70)], 0.0, &a);
    let r = card_rect(0);
    p.update(&[click(r.0 + 5, r.1 + 5)], 0.0, &a);
    assert!(matches!(p.project(), Some(Project::Button(b)) if b.kind() == Kind::Round));
    assert_eq!(painted(p.canvas()).len(), 0);
    p.update(&[click(270, 30)], 0.0, &a);
    assert_eq!(painted(p.canvas()).len(), n);
    let fx = p.leave();
    assert!(matches!(&fx[..], [Effect::Log(s)] if s.contains("guardado")));
    assert_eq!(files(dir.path(), "ultimo-botao").len(), 1);
    assert_eq!(files(dir.path(), "botao-").len(), 1);
    assert!(files(dir.path(), "ultimo.png").is_empty());
    let mut s = Paint::new_project(ProjectKind::Sticker, Some(dir.path().to_path_buf()), &a);
    s.leave();
    assert_eq!(files(dir.path(), "ultimo-adesivo").len(), 1);
    assert!(files(dir.path(), "adesivo-").is_empty());
}

#[test]
fn exit_door_returns_to_projects_floor() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new_project(ProjectKind::Comics, None, &a);
    let fx = p.update(&[click(600, 60)], 0.0, &a);
    assert!(fx.contains(&Effect::ChangeScene(engine::scene::SceneId::Projects)));
}

fn read_png(path: &Path) -> (u32, u32, Vec<u8>) {
    let dec = png::Decoder::new(std::fs::File::open(path).unwrap());
    let mut reader = dec.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgb);
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

#[test]
fn digits_exist_in_real_data() {
    let Some(d) = std::env::var_os("ATELIE_DATA").map(PathBuf::from) else {
        eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
        return;
    };
    let a = Assets::open(&d).unwrap();
    for (ids, w, h) in [
        ([0x3a78, 0x3a79, 0x3a79], 24, 33),
        ([0x3a7b, 0x3a7c, 0x3a7d], 33, 36),
        ([0x3a7f, 0x3a80, 0x3a81], 32, 37),
        ([0x3a83, 0x3a84, 0x3a85], 36, 33),
    ] {
        for id in ids {
            let img = a.image(&ImageRef::Pic(Program::Artist, id));
            assert_eq!((img.width, img.height), (w, h), "{id:#x}");
        }
    }
}
