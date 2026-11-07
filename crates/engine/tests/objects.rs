//! Camada de objetos, Transformadores, Super-ferramentas e ajuda do McZee
//! pela tela de pintura (seção "Testes" da spec de objetos de 2026-10-31).
//! Roda sem dados do original; os testes que precisam dos balões `WHAT`
//! usam `ATELIE_DATA` se houver.

use engine::assets::Assets;
use engine::paint::store::load_png;
use engine::paint::{ORIGIN, WHITE};
use engine::scene::{Input, MouseButton, Scene};
use engine::scenes::Paint;
use engine::scenes::paint::{Tool, VacuumMode};
use engine::scenes::transformers::{Op, PAGE1, PAGE2, option_rect};
use std::path::PathBuf;

fn empty_assets() -> (tempfile::TempDir, Assets) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
    let a = Assets::open(dir.path()).unwrap();
    (dir, a)
}

#[allow(dead_code)]
fn assets() -> Option<Assets> {
    let dir = std::env::var_os("ATELIE_DATA").map(PathBuf::from)?;
    Some(Assets::open(&dir).expect("ATELIE_DATA válida"))
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

/// Pixels não brancos da composição.
fn painted(p: &Paint) -> usize {
    p.composed().px.iter().filter(|&&v| v != WHITE).count()
}

/// Um pixel opaco do objeto, em coordenadas da tela.
fn opaque_point(p: &Paint, id: u32) -> (i32, i32) {
    let o = p.objects().get(id).unwrap();
    let (x, y) = (0..o.h)
        .flat_map(|y| (0..o.w).map(move |x| (x, y)))
        .find(|&(x, y)| o.pixel(x, y).is_some())
        .expect("objeto com pixel opaco");
    (ORIGIN.0 + o.x + x, ORIGIN.1 + o.y + y)
}

/// Coloca o bicho 1 (peixe-espada; o 0 é a cobra, que vai embora) centrado em `(x, y)` da tela
/// e espera a animação acabar: o adesivo vira um objeto.
fn place_animal(p: &mut Paint, a: &Assets, x: i32, y: i32) -> u32 {
    p.update(&[click(129, 30), click(40, 80), click(54, 152)], 0.0, a);
    p.update(&[click(x, y)], 0.0, a);
    p.update(&[], 10.0, a);
    assert!(!p.stickers().is_animating());
    let n = p.objects().len();
    assert!(n > 0);
    p.objects().items[n - 1].id
}

#[test]
fn placed_sticker_is_an_object_saved_flat_and_reopened_editable() {
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
    let id = place_animal(&mut p, &a, 300, 300);
    let o = p.objects().get(id).unwrap().rect();
    // o fundo continua branco; a composição tem o adesivo
    assert_eq!(p.canvas().px.iter().filter(|&&v| v != WHITE).count(), 0);
    assert!(painted(&p) > 0);
    p.leave();
    assert!(dir.path().join("ultimo.objetos.json").exists());
    assert!(dir.path().join("ultimo.fundo.png").exists());
    // PNG achatado
    let flat = load_png(&dir.path().join("ultimo.png"), p.palette()).unwrap();
    assert!(flat.px.iter().any(|&v| v != WHITE));
    // reabre editável, com o objeto no mesmo lugar
    let q = Paint::new(Some(dir.path().to_path_buf()), &a);
    assert_eq!(q.objects().len(), 1);
    assert_eq!(q.objects().items[0].rect(), o);
    assert_eq!(q.canvas().px.iter().filter(|&&v| v != WHITE).count(), 0);
}

#[test]
fn thing_vacuum_removes_only_the_object_and_egg_restores() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new(None, &a);
    // pincelada no fundo
    p.update(
        &[
            click(ORIGIN.0 + 50, ORIGIN.1 + 50),
            mv(ORIGIN.0 + 80, ORIGIN.1 + 50),
            up(ORIGIN.0 + 80, ORIGIN.1 + 50),
        ],
        0.0,
        &a,
    );
    let background = p.canvas().px.iter().filter(|&&v| v != WHITE).count();
    assert!(background > 0);
    let id = place_animal(&mut p, &a, 300, 300);
    // aspirador, aspira-coisas (2.º botão), clique num pixel do objeto
    p.update(&[click(328, 30)], 0.0, &a);
    assert_eq!(p.tool(), Tool::Vacuum);
    p.update(&[click(25 + 37 + 10, 70)], 0.0, &a);
    assert_eq!(p.vacuum_mode(), VacuumMode::Stickers);
    let (cx, cy) = opaque_point(&p, id);
    p.update(&[click(cx, cy), up(cx, cy)], 0.0, &a);
    assert_eq!(p.objects().len(), 0);
    assert_eq!(
        p.canvas().px.iter().filter(|&&v| v != WHITE).count(),
        background
    );
    // o ovo devolve o objeto
    p.update(&[click(270, 30)], 0.0, &a);
    assert_eq!(p.objects().len(), 1);
}

#[test]
fn words_become_an_object_and_vacuum_removes_them() {
    use engine::scenes::paint::words::{OI_BTN, balloon_easel};
    let (_d, a) = empty_assets();
    let mut p = Paint::new(None, &a);
    p.update(&[click(228, 30)], 0.0, &a);
    assert_eq!(p.tool(), Tool::Words);
    // o quadro vazio não reage à Palavra; o balão vem do cavalete do Oi!
    p.update(&[click(200, 200), up(200, 200)], 0.0, &a);
    assert!(p.objects().is_empty());
    let (ox, oy) = (OI_BTN.0 + 10, OI_BTN.1 + 10);
    let (kx, ky) = (balloon_easel::OK.0 + 5, balloon_easel::OK.1 + 5);
    p.update(&[click(ox, oy), up(ox, oy)], 0.0, &a);
    p.update(
        &[Input::Text("Oi".into()), click(kx, ky), up(kx, ky)],
        0.0,
        &a,
    );
    assert_eq!(p.objects().len(), 1);
    let o = p.objects().items[0].clone();
    assert!(matches!(&o.source, engine::paint::Source::Words { text, .. } if text == "Oi"));
    p.update(&[click(328, 30), click(25 + 37 + 10, 70)], 0.0, &a);
    let (sx, sy) = opaque_point(&p, o.id);
    p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
    assert_eq!(p.objects().len(), 0);
}

#[test]
fn words_roll_places_and_swaps_dipitidus_with_real_data() {
    use engine::scenes::paint::words::ROLL;
    let Some(a) = assets() else {
        eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
    p.update(&[click(228, 30)], 0.0, &a);
    // a faixa usa os bitmaps reais: FORMA 13ff, Oi! 141f e o rolo 10af
    for id in [0x13ff, 0x141f, 0x10af] {
        assert!(a.has_image(&engine::assets::ImageRef::Pic(
            engine::assets::Program::Artist,
            id
        )));
    }
    let (rx, ry) = (ROLL.0 + 40, ROLL.1 + 20);
    p.update(&[click(rx, ry), up(rx, ry)], 0.0, &a);
    assert!(p.tails_column_open());
    let l = *p.tails_column().unwrap();
    assert_eq!(l.n, 4);
    let r = l.row_rect(2);
    p.update(
        &[click(r.0 + 10, r.1 + 10), up(r.0 + 10, r.1 + 10)],
        0.0,
        &a,
    );
    assert!(!p.tails_column_open());
    p.update(&[click(300, 300), up(300, 300)], 0.0, &a);
    assert_eq!(p.objects().len(), 1);
    let o = p.objects().items[0].clone();
    assert_eq!(o.source, engine::paint::Source::Tail(2));
    assert_eq!(o.center(), (300 - ORIGIN.0, 300 - ORIGIN.1));
    // o próximo clique no quadro não põe outro
    p.update(&[click(100, 300), up(100, 300)], 0.0, &a);
    assert_eq!(p.objects().len(), 1);
    // Shift+arrasto aumenta, mantendo a proporção
    let (sx, sy) = opaque_point(&p, o.id);
    p.update(
        &[
            Input::Shift(true),
            click(sx, sy),
            mv(sx + 20, sy),
            up(sx + 20, sy),
            Input::Shift(false),
        ],
        0.0,
        &a,
    );
    let big = p.objects().get(o.id).unwrap().clone();
    assert_eq!(big.w, o.w + 20);
    assert!(big.h > o.h);
    // clique sem mover abre o rolo para trocar o dipitidu
    let (sx, sy) = opaque_point(&p, o.id);
    p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
    assert!(p.tails_column_open());
    let r = p.tails_column().unwrap().row_rect(0);
    p.update(
        &[click(r.0 + 10, r.1 + 10), up(r.0 + 10, r.1 + 10)],
        0.0,
        &a,
    );
    assert_eq!(p.objects().items[0].source, engine::paint::Source::Tail(0));
    // grava e reabre
    p.leave();
    let q = Paint::new(Some(dir.path().to_path_buf()), &a);
    assert_eq!(q.objects().items[0].source, engine::paint::Source::Tail(0));
}

/// Escolhe a ferramenta Transformadores e a ação `op`.
fn pick_transformer(p: &mut Paint, a: &Assets, op: Op) {
    p.update(&[click(378, 30)], 0.0, a);
    assert_eq!(p.tool(), Tool::Transformers);
    let (page, k) = match PAGE1.iter().position(|b| b.op == op) {
        Some(k) => (1, k),
        None => (2, PAGE2.iter().position(|b| b.op == op).unwrap()),
    };
    while p.transformers().page() != page {
        p.update(&[click(510, 95)], 0.0, a);
    }
    let r = option_rect(k);
    p.update(&[click(r.0 + 10, r.1 + 10)], 0.0, a);
    assert_eq!(p.transformers().op(), op);
}

#[test]
fn moved_object_changes_png_and_json_reopens_at_new_place() {
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
    let id = place_animal(&mut p, &a, 300, 300);
    let before = p.composed();
    let r = p.objects().get(id).unwrap().rect();
    let (sx, sy) = opaque_point(&p, id);
    pick_transformer(&mut p, &a, Op::Move);
    p.update(&[click(sx, sy), mv(sx + 20, sy), up(sx + 20, sy)], 0.0, &a);
    assert_eq!(
        p.objects().get(id).unwrap().rect(),
        (r.0 + 20, r.1, r.2, r.3)
    );
    assert_ne!(p.composed().px, before.px);
    p.leave();
    let flat = load_png(&dir.path().join("ultimo.png"), p.palette()).unwrap();
    assert_eq!(flat.px, p.composed().px);
    let q = Paint::new(Some(dir.path().to_path_buf()), &a);
    assert_eq!(
        q.objects().get(id).unwrap().rect(),
        (r.0 + 20, r.1, r.2, r.3)
    );
}

#[test]
fn rotate_right_four_times_and_flip_twice_restore() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new(None, &a);
    let id = place_animal(&mut p, &a, 300, 300);
    let before = p.objects().get(id).unwrap().clone();
    let flat = p.composed();
    pick_transformer(&mut p, &a, Op::RotateRight);
    for _ in 0..4 {
        let (cx, cy) = {
            let o = p.objects().get(id).unwrap();
            let (cx, cy) = o.center();
            (ORIGIN.0 + cx, ORIGIN.1 + cy)
        };
        // o centro pode ser transparente: usa o primeiro pixel opaco se for
        let (sx, sy) = if p.objects().hit(cx - ORIGIN.0, cy - ORIGIN.1).is_some() {
            (cx, cy)
        } else {
            opaque_point(&p, id)
        };
        p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
    }
    assert_eq!(*p.objects().get(id).unwrap(), before);
    assert_eq!(p.composed().px, flat.px);
    for op in [Op::FlipH, Op::FlipV] {
        pick_transformer(&mut p, &a, op);
        let (sx, sy) = opaque_point(&p, id);
        p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
        assert_ne!(*p.objects().get(id).unwrap(), before);
        let (sx, sy) = opaque_point(&p, id);
        p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
        assert_eq!(*p.objects().get(id).unwrap(), before);
    }
    assert_eq!(p.composed().px, flat.px);
}

#[test]
fn linked_objects_move_together() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new(None, &a);
    let first = place_animal(&mut p, &a, 200, 300);
    let second = place_animal(&mut p, &a, 400, 300);
    assert_ne!(first, second);
    pick_transformer(&mut p, &a, Op::Link);
    let (x1, y1) = opaque_point(&p, first);
    let (x2, y2) = opaque_point(&p, second);
    p.update(
        &[click(x1, y1), up(x1, y1), click(x2, y2), up(x2, y2)],
        0.0,
        &a,
    );
    assert_eq!(p.objects().group(first).len(), 2);
    let r2 = p.objects().get(second).unwrap().rect();
    pick_transformer(&mut p, &a, Op::Move);
    p.update(
        &[click(x1, y1), mv(x1 + 15, y1 - 10), up(x1 + 15, y1 - 10)],
        0.0,
        &a,
    );
    assert_eq!(
        p.objects().get(second).unwrap().rect(),
        (r2.0 + 15, r2.1 - 10, r2.2, r2.3)
    );
}

#[test]
fn approach_vp_for_one_second_shrinks_and_moves_and_png_has_no_guides() {
    use engine::scenes::supertools::{BUTTONS, Op as SOp};
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
    let id = place_animal(&mut p, &a, 150, 350);
    let r0 = p.objects().get(id).unwrap().rect();
    p.update(&[click(430, 30)], 0.0, &a);
    assert_eq!(p.tool(), Tool::Supertools);
    let button = |op: SOp| {
        let b = BUTTONS.iter().find(|b| b.op == op).unwrap();
        let r = option_rect(b.k);
        click(r.0 + 10, r.1 + 10)
    };
    // quadriculado grande + ponto de fuga em (500, 150) da tela
    p.update(
        &[button(SOp::BigGrid), button(SOp::VanishingPoint)],
        0.0,
        &a,
    );
    p.update(&[click(500, 150), up(500, 150)], 0.0, &a);
    let vp = (500 - ORIGIN.0, 150 - ORIGIN.1);
    assert_eq!(p.guides().vp(), Some(vp));
    let dist = |r: (i32, i32, i32, i32)| {
        let (cx, cy) = (r.0 + r.2 / 2, r.1 + r.3 / 2);
        (((cx - vp.0).pow(2) + (cy - vp.1).pow(2)) as f32).sqrt()
    };
    p.update(&[button(SOp::Approach)], 0.0, &a);
    let (sx, sy) = opaque_point(&p, id);
    p.update(&[click(sx, sy)], 0.0, &a);
    for _ in 0..60 {
        p.update(&[], 1.0 / 60.0, &a);
    }
    p.update(&[up(sx, sy)], 0.0, &a);
    let r1 = p.objects().get(id).unwrap().rect();
    assert!(r1.2 < r0.2 && r1.3 < r0.3, "{r0:?} {r1:?}");
    let (d0, d1) = (dist(r0), dist(r1));
    assert!(d1 < d0 - 50.0 && d1 > d0 - 70.0, "{d0} {d1}");
    // o PNG achatado não tem o quadriculado nem o horizonte
    p.leave();
    let flat = load_png(&dir.path().join("ultimo.png"), p.palette()).unwrap();
    assert_eq!(flat.px, p.composed().px);
    let green = flat.px.iter().filter(|&&v| v != WHITE).count();
    assert_eq!(green, painted(&p));
    assert_eq!(flat.get(48, 0), Some(WHITE));
    assert_eq!(flat.get(10, vp.1), Some(WHITE));
}

#[test]
fn mczee_click_with_bucket_opens_paint_bucket_balloon() {
    let Some(a) = assets() else { return };
    let mut p = Paint::new(None, &a);
    // na primeira vez o balão do Pincel pode estar aberto: um clique fora fecha
    p.update(&[click(300, 300), up(300, 300)], 0.0, &a);
    p.update(&[click(78, 30)], 0.0, &a);
    assert_eq!(p.tool(), Tool::Bucket);
    // escolher a ferramenta já abre o balão (o McZee no nicho abre o menu)
    let h = p.help().expect("balão aberto");
    assert_eq!(h.title(), "Paint bucket");
    assert!(h.record().plain().contains("Lata de tinta"));
    assert!(h.x >= 445 && h.x + h.w <= 635);
    // opção "Troca-cores" (2.º botão da lata) abre o balão dela, com a tecla de atalho ligada
    p.update(&[click(113 + 37 + 10, 70)], 0.0, &a);
    let h = p.help().unwrap();
    assert_eq!(h.title(), "Color swapper");
    let (bx, by) = (h.x, h.y);
    let hh = h.height(&a);
    // procura a ligação da tecla de atalho varrendo o balão
    let mut target = None;
    for y in (by..by + hh).step_by(2) {
        for x in (bx..bx + h.w).step_by(4) {
            if let Some(t) = h.link_at(&a, x, y) {
                target = Some((x, y, t));
                break;
            }
        }
        if target.is_some() {
            break;
        }
    }
    // a primeira ligação é a palavra do glossário ("adesivo", registro 20)
    let (lx, ly, t) = target.expect("ligação no balão");
    assert_eq!(t, 20);
    p.update(&[click(lx, ly)], 0.0, &a);
    assert_eq!(p.help().unwrap().title(), "adesivo");
    let mut f = engine::Frame::new(640, 480);
    p.draw(&mut f, &a);
}

#[test]
fn help_without_data_never_opens_and_niche_click_is_harmless() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new(None, &a);
    p.update(&[click(556, 80), click(78, 30), click(556, 80)], 0.0, &a);
    assert!(p.help().is_none());
    assert_eq!(p.tool(), Tool::Bucket);
}

/// Primeira ligação (gema, tecla, glossário) do balão, varrendo-o.
fn first_link(h: &engine::help::HelpBalloon, a: &Assets) -> Option<(i32, i32, usize)> {
    let hh = h.height(a);
    (h.y..h.y + hh).step_by(2).find_map(|y| {
        (h.x..h.x + h.w)
            .step_by(4)
            .find_map(|x| h.link_at(a, x, y).map(|t| (x, y, t)))
    })
}

#[test]
fn brush_balloon_stays_open_while_paging_and_gems_open_beside_it() {
    let Some(a) = assets() else { return };
    let mut p = Paint::new(None, &a);
    p.update(&[click(300, 300), click(38, 40)], 0.0, &a);
    let main = p.help().expect("balão do pincel").key();
    let page = p.page();
    // setas de página: o balão continua aberto
    p.update(&[click(512, 96)], 0.0, &a);
    assert_ne!(p.page(), page);
    assert_eq!(p.help().map(|h| h.key()), Some(main));
    // uma gema abre o sub-balão à esquerda; o principal fica aberto
    let (lx, ly, t) = first_link(p.help().unwrap(), &a).expect("ligação");
    p.update(&[click(lx, ly)], 0.0, &a);
    let sub = p.help().expect("sub-balão");
    assert_eq!(sub.index, t);
    let parent = p.help_beside().expect("principal ao lado");
    assert_eq!(parent.key(), main);
    assert!(
        sub.x + sub.w <= parent.x,
        "{} {} {}",
        sub.x,
        sub.w,
        parent.x
    );
    let mut f = engine::Frame::new(640, 480);
    p.draw(&mut f, &a);
    // clique no principal fora das ligações fecha só o sub
    let (px, py) = (parent.x + 3, parent.y + 3);
    p.update(&[click(px, py)], 0.0, &a);
    assert_eq!(p.help().map(|h| h.key()), Some(main));
    assert!(p.help_beside().is_none());
    // de novo a gema, e um clique fora fecha os dois
    p.update(&[click(lx, ly)], 0.0, &a);
    assert!(p.help_beside().is_some());
    p.update(&[click(300, 420)], 0.0, &a);
    assert!(p.help().is_none() && p.help_beside().is_none());
}
