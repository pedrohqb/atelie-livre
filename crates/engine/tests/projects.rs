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

/// Clica em Imprimir e confirma no balão "Você quer imprimir ...?".
fn print_now(p: &mut Paint, a: &Assets) {
    p.update(&[click(479, 30)], 0.0, a);
    let r = p.dialog().expect("balão de imprimir").option_rect(a, 0);
    p.update(&[click(r.0 + 3, r.1 + 3)], 0.0, a);
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
    // o PNG e o `.projeto.txt` (número de quadrinhos)
    assert_eq!(files(dir.path(), "ultimo-").len(), 2);
    assert_eq!(files(dir.path(), "quadrinhos-").len(), 2);
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
    print_now(&mut p, &a);
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
    print_now(&mut p, &a);
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
    assert!(matches!(s.project(), Some(Project::Button(b)) if b.kind() == Kind::Rect));
    print_now(&mut s, &a);
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
    let r = card_rect(1, 3);
    p.update(&[click(r.0 + 5, r.1 + 5)], 0.0, &a);
    assert!(matches!(p.project(), Some(Project::Button(b)) if b.kind() == Kind::Square));
    p.update(&stroke(cx + 120, cy - 120, cx + 120, cy - 120), 0.0, &a);
    let n = painted(p.canvas()).len();
    assert!(n > 0);
    // abre o painel pelo nicho e volta ao redondo: o canto some; o ovo desfaz
    p.update(&[click(540, 70)], 0.0, &a);
    let r = card_rect(0, 3);
    p.update(&[click(r.0 + 5, r.1 + 5)], 0.0, &a);
    assert!(matches!(p.project(), Some(Project::Button(b)) if b.kind() == Kind::Round));
    assert_eq!(painted(p.canvas()).len(), 0);
    p.update(&[click(270, 30)], 0.0, &a);
    assert_eq!(painted(p.canvas()).len(), n);
    let fx = p.leave();
    assert!(matches!(&fx[..], [Effect::Log(s)] if s.contains("guardado")));
    assert_eq!(files(dir.path(), "ultimo-botao").len(), 2);
    assert_eq!(files(dir.path(), "botao-").len(), 2);
    assert!(files(dir.path(), "ultimo.png").is_empty());
    let mut s = Paint::new_project(ProjectKind::Sticker, Some(dir.path().to_path_buf()), &a);
    s.leave();
    assert_eq!(files(dir.path(), "ultimo-adesivo").len(), 2);
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

#[test]
fn sticker_shape_and_panel_count_survive_reopening() {
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut s = Paint::new_project(ProjectKind::Sticker, Some(dir.path().to_path_buf()), &a);
    // 4.º cartão de 6: oval alto
    let r = card_rect(3, 6);
    s.update(&[click(r.0 + 5, r.1 + 5)], 0.0, &a);
    assert!(matches!(s.project(), Some(Project::Button(b)) if b.kind() == Kind::TallOval));
    s.leave();
    assert!(dir.path().join("ultimo-adesivo.projeto.txt").exists());
    let s = Paint::new_project(ProjectKind::Sticker, Some(dir.path().to_path_buf()), &a);
    assert!(matches!(s.project(), Some(Project::Button(b)) if b.kind() == Kind::TallOval));
    let mut k = Paint::new_project(ProjectKind::Comics, Some(dir.path().to_path_buf()), &a);
    let r = digit_rect(2);
    k.update(&[click(r.0 + 10, r.1 + 10)], 0.0, &a);
    k.leave();
    let k = Paint::new_project(ProjectKind::Comics, Some(dir.path().to_path_buf()), &a);
    assert!(matches!(k.project(), Some(Project::Comics(c)) if c.panels == 3));
}

fn text(s: &str) -> Input {
    Input::Text(s.into())
}

fn enter() -> Input {
    Input::Key {
        key: engine::scene::Key::Enter,
        ctrl: false,
        alt: false,
    }
}

/// Opção `k` de uma faixa que começa em `x0` (como `option_rect` da pintura).
fn opt(x0: i32, k: i32) -> (i32, i32) {
    (x0 + 37 * k + 16, 63 + 16)
}

#[test]
fn comics_easel_styles_moves_and_resizes_balloons() {
    use engine::paint::{Align, BalloonShape, Source};
    let (_d, a) = empty_assets();
    let mut p = Paint::new_project(ProjectKind::Comics, None, &a);
    // fecha o painel dos dígitos (1 quadrinho) e escolhe a Palavra
    let r = digit_rect(0);
    p.update(&[click(r.0 + 10, r.1 + 10), click(228, 30)], 0.0, &a);
    assert_eq!(p.tool(), engine::scenes::paint::Tool::Words);
    // cavalete: fonte pequena, oval com rabicho, centrado
    let (fx, fy) = opt(25, 1);
    let (sx, sy) = opt(25 + 2 * 37 + 8, 2);
    let (ax, ay) = opt(25 + 2 * 37 + 8 + 6 * 37 + 8, 1);
    p.update(&[click(fx, fy), click(sx, sy), click(ax, ay)], 0.0, &a);
    // digita um balão
    p.update(
        &[
            click(ORIGIN.0 + 100, ORIGIN.1 + 100),
            text("Oi tudo bem"),
            enter(),
        ],
        0.0,
        &a,
    );
    assert_eq!(p.objects().len(), 1);
    let o = p.objects().items[0].clone();
    let Source::Words { style, text: t, .. } = &o.source else {
        panic!("não é palavra");
    };
    assert_eq!(t, "Oi tudo bem");
    assert_eq!(style.shape, BalloonShape::OvalTail);
    assert_eq!(style.align, Align::Center);
    assert!(style.small);
    assert_eq!(style.width, 0);
    // o texto fica onde foi o clique: o balão começa antes
    assert!(o.x < 100 && o.y < 100);
    let (ox, oy, ow, oh) = (o.x, o.y, o.w, o.h);
    // arrastar o balão move, em vez de começar outro texto
    let (gx, gy) = (ORIGIN.0 + 100, ORIGIN.1 + 100);
    p.update(
        &[click(gx, gy), mv(gx + 30, gy + 20), up(gx + 30, gy + 20)],
        0.0,
        &a,
    );
    assert_eq!(p.objects().len(), 1);
    let o = p.objects().get(o.id).unwrap().clone();
    assert_eq!((o.x, o.y), (ox + 30, oy + 20));
    assert!(p.words_text().is_none());
    // Shift+arrasto muda a largura: o texto quebra e o balão fica mais estreito e alto
    let (gx, gy) = (gx + 30, gy + 20);
    p.update(
        &[
            Input::Shift(true),
            click(gx, gy),
            mv(gx - 40, gy),
            up(gx - 40, gy),
            Input::Shift(false),
        ],
        0.0,
        &a,
    );
    let o = p.objects().get(o.id).unwrap().clone();
    let Source::Words { style, .. } = &o.source else {
        panic!()
    };
    assert!(style.width > 0 && style.width < 80, "{}", style.width);
    assert!(o.w < ow && o.h > oh, "{}x{} vs {ow}x{oh}", o.w, o.h);
    // o ovo desfaz a largura
    p.update(&[click(270, 30)], 0.0, &a);
    let o2 = p.objects().get(o.id).unwrap().clone();
    assert_eq!((o2.w, o2.h), (ow, oh));
    p.update(&[click(270, 30), click(228, 30)], 0.0, &a);
    // clicar sem mover reabre o texto no mesmo lugar e estilo; Backspace e Enter gravam
    let o = p.objects().get(o.id).unwrap().clone();
    let (cx, cy) = (ORIGIN.0 + o.x + o.w / 2, ORIGIN.1 + o.y + 8);
    p.update(&[click(cx, cy), up(cx, cy)], 0.0, &a);
    assert_eq!(p.objects().len(), 0);
    assert_eq!(p.words_text(), Some("Oi tudo bem"));
    p.update(&[text("!"), enter()], 0.0, &a);
    assert_eq!(p.objects().len(), 1);
    let n = p.objects().items[0].clone();
    assert!(
        matches!(&n.source, Source::Words { text, style, .. } if text == "Oi tudo bem!" && style.shape == BalloonShape::OvalTail && style.width == o_width(&o))
    );
    assert_eq!((n.x, n.y), (o.x, o.y));
    // "espaço em branco": sem balão
    let (bx, by) = opt(25 + 2 * 37 + 8, 5);
    p.update(
        &[
            click(bx, by),
            click(ORIGIN.0 + 300, ORIGIN.1 + 200),
            text("x"),
            enter(),
        ],
        0.0,
        &a,
    );
    let last = p.objects().items.last().unwrap();
    assert!(
        matches!(&last.source, Source::Words { style, .. } if style.shape == BalloonShape::None)
    );
    assert_eq!((last.x, last.y), (300, 200));
}

fn o_width(o: &engine::paint::Object) -> u32 {
    match &o.source {
        engine::paint::Source::Words { style, .. } => style.width,
        _ => 0,
    }
}

#[test]
fn plain_paint_keeps_two_word_options() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new(None, &a);
    p.update(&[click(228, 30)], 0.0, &a);
    // opção 2 = balão "Oi!"; o cavalete não existe fora dos quadrinhos
    let (x, y) = opt(25, 1);
    p.update(
        &[
            click(x, y),
            click(ORIGIN.0 + 50, ORIGIN.1 + 50),
            text("a"),
            enter(),
        ],
        0.0,
        &a,
    );
    let o = &p.objects().items[0];
    assert!(
        matches!(&o.source, engine::paint::Source::Words { style, .. } if *style == engine::paint::WordsStyle::balloon())
    );
}

#[test]
fn comic_backgrounds_exist_in_real_data() {
    use engine::scenes::projects::comics::{BG_THUMB, load_backgrounds};
    let Some(d) = std::env::var_os("ATELIE_DATA").map(PathBuf::from) else {
        eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
        return;
    };
    let a = Assets::open(&d).unwrap();
    let bgs = load_backgrounds(&a);
    assert_eq!(bgs.len(), 11);
    assert_eq!(bgs[0].name, "Caverna");
    assert!(bgs.iter().any(|b| b.name == "Fundo do mar"));
    assert!(
        bgs.iter()
            .all(|b| (b.image.width, b.image.height) == (271, 219))
    );
    // pela tela: a miniatura arma o fundo e o clique no quadrinho pinta
    let mut p = Paint::new_project(ProjectKind::Comics, None, &a);
    assert!(matches!(p.project(), Some(Project::Comics(c)) if c.backgrounds.len() == 11));
    p.update(&[click(BG_THUMB.0 + 5, BG_THUMB.1 + 5)], 0.0, &a);
    let before = painted(p.canvas()).len();
    p.update(&[click(ORIGIN.0 + 200, ORIGIN.1 + 150)], 0.0, &a);
    assert!(painted(p.canvas()).len() > before + 1000);
    assert!(matches!(p.project(), Some(Project::Comics(c)) if !c.pending_background));
}

#[test]
fn comics_have_the_tails_roll_in_real_data() {
    let Some(d) = std::env::var_os("ATELIE_DATA").map(PathBuf::from) else {
        eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
        return;
    };
    let a = Assets::open(&d).unwrap();
    let plain = Paint::new(None, &a);
    let mut p = Paint::new_project(ProjectKind::Comics, None, &a);
    assert_eq!(p.stickers().roll_count(), plain.stickers().roll_count() + 1);
    let roll = p.stickers().roll_count() - 1;
    // ferramenta de adesivos, última página, último rolo, primeiro rabinho, cola no quadro
    p.update(&[click(129, 30)], 0.0, &a);
    let page = roll / engine::scenes::stickers::ROLLS_PER_PAGE;
    for _ in 0..page {
        p.update(&[click(510, 95)], 0.0, &a);
    }
    let slot = roll % engine::scenes::stickers::ROLLS_PER_PAGE;
    p.update(&[click(23 + 52 * slot as i32 + 20, 75)], 0.0, &a);
    assert!(p.stickers().film_open());
    // primeira célula da tira (logo abaixo da seta de subir)
    let fx = 33 + 52 * slot as i32 + 20;
    p.update(&[click(fx, 130)], 0.0, &a);
    assert!(p.stickers().holding());
    p.update(&[click(ORIGIN.0 + 200, ORIGIN.1 + 200)], 0.0, &a);
    assert_eq!(p.objects().len(), 1);
    let o = &p.objects().items[0];
    assert!(matches!(o.source, engine::paint::Source::Roll { roll: r, item: 0 } if r == roll));
    // rabinho ampliado 2×: entre 20 e 70 px, com contorno preto e interior branco
    assert!(
        o.w >= 20 && o.w <= 70 && o.h >= 20 && o.h <= 70,
        "{}x{}",
        o.w,
        o.h
    );
    assert!(o.image.px.contains(&Some(BLACK)));
    assert!(o.image.px.contains(&Some(WHITE)));
    assert!(o.image.px.iter().any(|p| p.is_none()));
}

#[test]
fn end_flow_saves_with_name_discards_or_starts_new() {
    use engine::scenes::projects::dialogs::Step;
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new_project(ProjectKind::Button, Some(dir.path().to_path_buf()), &a);
    let (cx, cy) = (CANVAS_W as i32 / 2, CANVAS_H as i32 / 2);
    p.update(&[click(540, 70)], 0.0, &a);
    p.update(&stroke(cx, cy, cx + 20, cy), 0.0, &a);
    // imprimir: pergunta, imprime, "D+. Você terminou!"
    print_now(&mut p, &a);
    assert_eq!(files(dir.path(), "impressao-").len(), 1);
    assert_eq!(p.dialog().map(|d| d.step), Some(Step::Done));
    // "Começar um novo": salvar? → nome → grava botao-<nome>.png e limpa o quadro
    let r = p.dialog().unwrap().option_rect(&a, 1);
    p.update(&[click(r.0 + 3, r.1 + 3)], 0.0, &a);
    assert_eq!(p.dialog().map(|d| d.step), Some(Step::SaveAsk));
    let r = p.dialog().unwrap().option_rect(&a, 0);
    p.update(&[click(r.0 + 3, r.1 + 3)], 0.0, &a);
    assert_eq!(p.dialog().map(|d| d.step), Some(Step::NameAsk));
    p.update(&[text("Meu Botão"), enter()], 0.0, &a);
    assert!(p.dialog().is_none());
    assert!(dir.path().join("botao-meu-botao.png").exists());
    assert!(dir.path().join("botao-meu-botao.projeto.txt").exists());
    assert_eq!(painted(p.canvas()).len(), 0);
    assert!(matches!(p.project(), Some(Project::Button(b)) if b.asking));
    // sair sem mudanças: direto
    let fx = p.update(&[click(600, 60)], 0.0, &a);
    assert!(fx.contains(&Effect::ChangeScene(engine::scene::SceneId::Projects)));
    // com mudanças, a porta pergunta; "Não, jogar fora" → "imprimir antes?" → "Não"
    p.update(&[click(540, 70)], 0.0, &a);
    p.update(&stroke(cx, cy, cx + 20, cy), 0.0, &a);
    let fx = p.update(&[click(600, 60)], 0.0, &a);
    assert!(fx.is_empty());
    assert_eq!(p.dialog().map(|d| d.step), Some(Step::SaveAsk));
    // clique fora cancela a saída
    p.update(&[click(100, 470)], 0.0, &a);
    assert!(p.dialog().is_none());
    p.update(&[click(600, 60)], 0.0, &a);
    let r = p.dialog().unwrap().option_rect(&a, 1);
    p.update(&[click(r.0 + 3, r.1 + 3)], 0.0, &a);
    assert_eq!(p.dialog().map(|d| d.step), Some(Step::PrintBeforeTrash));
    let r = p.dialog().unwrap().option_rect(&a, 1);
    let fx = p.update(&[click(r.0 + 3, r.1 + 3)], 0.0, &a);
    assert!(fx.contains(&Effect::ChangeScene(engine::scene::SceneId::Projects)));
    let fx = p.leave();
    assert!(matches!(&fx[..], [Effect::Log(s)] if s.contains("jogado fora")));
    assert!(!dir.path().join("ultimo-botao.png").exists());
    assert_eq!(files(dir.path(), "botao-").len(), 2);
    // reabrir vem em branco
    let q = Paint::new_project(ProjectKind::Button, Some(dir.path().to_path_buf()), &a);
    assert_eq!(painted(q.canvas()).len(), 0);
}
