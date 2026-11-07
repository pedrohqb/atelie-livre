use super::balloon_easel::{self, BalloonEasel};
use super::shaped_easel::{self, Menu};
use super::ui::Rect;
use super::{Easel, FORMA_BTN, OI_BTN, ROLL};
use crate::paint::text::{Align, BalloonShape};
use crate::paint::{Shadow, Source, WordShape};
use crate::scene::{Input, Key, MouseButton, Scene};
use crate::scenes::paint::{ORIGIN, Paint, Tool};
use crate::scenes::testutil::empty_assets;

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

fn mv(x: i32, y: i32) -> Input {
    Input::MouseMove { x, y }
}

fn mid(r: Rect) -> (i32, i32) {
    (r.0 + r.2 / 2, r.1 + r.3 / 2)
}

fn press(r: Rect) -> [Input; 2] {
    let (x, y) = mid(r);
    [click(x, y), up(x, y)]
}

fn text(s: &str) -> Input {
    Input::Text(s.into())
}

fn key(k: Key) -> Input {
    Input::Key {
        key: k,
        ctrl: false,
        alt: false,
    }
}

fn words_tool(a: &crate::assets::Assets) -> Paint {
    let mut p = Paint::new(None, a);
    p.update(&[click(228, 30)], 0.0, a);
    assert_eq!(p.tool(), Tool::Words);
    p
}

/// Casa `k` da lista aberta no cavalete de palavras com forma.
fn shaped_item(p: &Paint, k: usize) -> (i32, i32) {
    let Some(Easel::Shaped(e)) = &p.words.easel else {
        panic!("cavalete fechado");
    };
    let mut l = *e.menu_list().expect("lista fechada");
    l.show(k);
    // a lista do cavalete rola sozinha só ao abrir; aqui só olhamos as visíveis
    assert!(
        k >= l.scroll && k < l.scroll + l.visible,
        "casa {k} escondida"
    );
    mid(l.row_rect(k - l.scroll))
}

fn balloon(p: &Paint) -> &BalloonEasel {
    match &p.words.easel {
        Some(Easel::Balloon(e)) => e,
        _ => panic!("cavalete de balões fechado"),
    }
}

#[test]
fn canvas_click_does_nothing_without_easel() {
    let (_d, a) = empty_assets();
    let mut p = words_tool(&a);
    let fx = p.update(&[click(300, 300), up(300, 300), text("abc")], 0.0, &a);
    assert!(fx.is_empty(), "{fx:?}");
    assert!(p.objects().is_empty());
    assert_eq!(p.words_easel(), None);
    assert_eq!(p.words_text(), None);
}

#[test]
fn forma_easel_types_styles_and_places_a_centered_object() {
    let (_d, a) = empty_assets();
    super::set_user_name("Ana");
    let mut p = words_tool(&a);
    let fx = p.update(&press(FORMA_BTN), 0.0, &a);
    assert!(fx.contains(&crate::scenes::paint_sfx::play(
        crate::scenes::paint_sfx::WORDS_OPTION
    )));
    assert_eq!(p.words_easel(), Some(true));
    assert_eq!(p.words_text(), Some("Ana"));
    // o texto começa selecionado: digitar troca tudo
    p.update(&[text("Oi"), key(Key::Backspace), text("la")], 0.0, &a);
    assert_eq!(p.words_text(), Some("Ola"));
    // cliques no quadro não passam do cavalete
    p.update(&[click(150, 150), up(150, 150)], 0.0, &a);
    assert!(p.objects().is_empty());
    // tamanho, giro e listas
    let size0 = p.shaped_words().unwrap().size;
    p.update(&press(shaped_easel::PLUS), 0.0, &a);
    assert!(p.shaped_words().unwrap().size > size0);
    p.update(&press(shaped_easel::ROT_LEFT), 0.0, &a);
    assert_eq!(p.shaped_words().unwrap().angle, 5);
    p.update(&press(shaped_easel::button_rect(1)), 0.0, &a);
    let (x, y) = shaped_item(&p, 2);
    p.update(&[click(x, y), up(x, y)], 0.0, &a);
    assert_eq!(p.shaped_words().unwrap().shape, WordShape::Wave);
    p.update(&press(shaped_easel::button_rect(2)), 0.0, &a);
    let (x, y) = shaped_item(&p, 1);
    p.update(&[click(x, y), up(x, y)], 0.0, &a);
    assert!(p.shaped_words().unwrap().bold);
    p.update(&press(shaped_easel::button_rect(5)), 0.0, &a);
    let (x, y) = shaped_item(&p, 1);
    p.update(&[click(x, y), up(x, y)], 0.0, &a);
    assert_eq!(p.shaped_words().unwrap().shadow, Shadow::Normal);
    // clique fora fecha a lista sem mudar nada
    p.update(&press(shaped_easel::button_rect(3)), 0.0, &a);
    p.update(&[click(600, 470), up(600, 470)], 0.0, &a);
    let Some(Easel::Shaped(e)) = &p.words.easel else {
        panic!()
    };
    assert_eq!(e.menu(), None);
    // OK: um objeto no meio do quadro
    p.update(&press(shaped_easel::OK), 0.0, &a);
    assert_eq!(p.words_easel(), None);
    assert_eq!(p.objects().len(), 1);
    let o = p.objects().items[0].clone();
    let Source::Shaped(w) = &o.source else {
        panic!("não é palavra com forma");
    };
    assert_eq!(w.text, "Ola");
    assert!(w.bold && w.shape == WordShape::Wave && w.angle == 5);
    let (cx, cy) = o.center();
    assert!((cx - crate::paint::CANVAS_W as i32 / 2).abs() <= 1);
    assert!((cy - crate::paint::CANVAS_H as i32 / 2).abs() <= 1);
    // o ovo desfaz
    p.update(&[click(270, 30)], 0.0, &a);
    assert!(p.objects().is_empty());
}

#[test]
fn shaped_easel_cancel_empty_and_held_rotation() {
    let (_d, a) = empty_assets();
    let mut p = words_tool(&a);
    p.update(&press(FORMA_BTN), 0.0, &a);
    p.update(&press(shaped_easel::NO), 0.0, &a);
    assert_eq!(p.words_easel(), None);
    assert!(p.objects().is_empty());
    // texto apagado: OK não põe nada
    p.update(&press(FORMA_BTN), 0.0, &a);
    p.update(&[key(Key::Backspace)], 0.0, &a);
    assert_eq!(p.words_text(), Some(""));
    p.update(&press(shaped_easel::OK), 0.0, &a);
    assert!(p.objects().is_empty());
    // girar segurando o botão: continua girando até soltar
    p.update(&press(FORMA_BTN), 0.0, &a);
    let (x, y) = mid(shaped_easel::ROT_RIGHT);
    p.update(&[click(x, y)], 0.0, &a);
    assert_eq!(p.shaped_words().unwrap().angle, 355);
    p.update(&[], 0.5, &a);
    p.update(&[], 0.2, &a);
    let a1 = p.shaped_words().unwrap().angle;
    assert!(a1 < 355 && a1 > 300, "{a1}");
    p.update(&[up(x, y)], 0.0, &a);
    p.update(&[], 1.0, &a);
    assert_eq!(p.shaped_words().unwrap().angle, a1);
    // Esc fecha sem colocar
    p.update(&[key(Key::Escape)], 0.0, &a);
    assert_eq!(p.words_easel(), None);
}

#[test]
fn clicking_a_shaped_word_reopens_and_ok_changes_it_in_place() {
    let (_d, a) = empty_assets();
    let mut p = words_tool(&a);
    p.update(&press(FORMA_BTN), 0.0, &a);
    p.update(&[text("Casa")], 0.0, &a);
    p.update(&press(shaped_easel::OK), 0.0, &a);
    let o = p.objects().items[0].clone();
    let (sx, sy) = opaque(&p, o.id);
    // arrastar move
    p.update(
        &[click(sx, sy), mv(sx + 20, sy + 10), up(sx + 20, sy + 10)],
        0.0,
        &a,
    );
    let moved = p.objects().get(o.id).unwrap().clone();
    assert_eq!((moved.x, moved.y), (o.x + 20, o.y + 10));
    assert_eq!(p.words_easel(), None);
    // clicar sem mover reabre no cavalete, com o texto
    let (sx, sy) = opaque(&p, o.id);
    p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
    assert_eq!(p.words_easel(), Some(true));
    assert_eq!(p.words_text(), Some("Casa"));
    p.update(&press(shaped_easel::button_rect(2)), 0.0, &a);
    let (x, y) = shaped_item(&p, 5);
    p.update(&[click(x, y), up(x, y)], 0.0, &a);
    p.update(&press(shaped_easel::OK), 0.0, &a);
    assert_eq!(p.objects().len(), 1);
    let n = p.objects().items[0].clone();
    assert_eq!(n.id, o.id);
    assert_eq!(n.center(), moved.center());
    assert!(matches!(&n.source, Source::Shaped(w) if w.vertical && w.text == "Casa"));
    assert!(n.h > n.w);
}

/// Um pixel de tela opaco do objeto.
fn opaque(p: &Paint, id: u32) -> (i32, i32) {
    let o = p.objects().get(id).unwrap();
    for oy in 0..o.h {
        for ox in 0..o.w {
            if o.pixel(ox, oy).is_some() {
                return (ORIGIN.0 + o.x + ox, ORIGIN.1 + o.y + oy);
            }
        }
    }
    panic!("objeto vazio");
}

#[test]
fn oi_easel_makes_balloons_with_comics_styles() {
    let (_d, a) = empty_assets();
    let mut p = words_tool(&a);
    p.update(&press(OI_BTN), 0.0, &a);
    assert_eq!(p.words_easel(), Some(false));
    assert_eq!(p.words_text(), Some("Oi!"));
    // Enter não termina o balão (o texto quebra sozinho)
    p.update(&[text("Oi tudo bem"), key(Key::Enter)], 0.0, &a);
    assert_eq!(p.words_easel(), Some(false));
    let pick = |p: &mut Paint, button: usize, k: usize| {
        p.update(&press(balloon_easel::button_rect(button)), 0.0, &a);
        let l = *balloon(p).menu_list().unwrap();
        let (x, y) = mid(l.row_rect(k));
        p.update(&[click(x, y), up(x, y)], 0.0, &a);
    };
    pick(&mut p, 0, 1);
    pick(&mut p, 1, 2);
    pick(&mut p, 2, 1);
    let st = balloon(&p).style;
    assert!(st.small);
    assert_eq!(st.shape, BalloonShape::OvalTail);
    assert_eq!(st.align, Align::Center);
    p.update(&press(balloon_easel::OK), 0.0, &a);
    assert_eq!(p.objects().len(), 1);
    let o = p.objects().items[0].clone();
    let Source::Words { text: t, style, .. } = &o.source else {
        panic!("não é balão");
    };
    assert_eq!(t, "Oi tudo bem");
    assert_eq!(*style, st);
    // Shift+arrasto estreita o balão: o texto quebra e ele fica mais alto
    let (sx, sy) = (ORIGIN.0 + o.x + o.w / 2, ORIGIN.1 + o.y + 6);
    p.update(
        &[
            Input::Shift(true),
            click(sx, sy),
            mv(sx - 40, sy),
            up(sx - 40, sy),
            Input::Shift(false),
        ],
        0.0,
        &a,
    );
    let n = p.objects().get(o.id).unwrap().clone();
    assert!(n.w < o.w && n.h > o.h, "{}x{} vs {}x{}", n.w, n.h, o.w, o.h);
    // clique reabre com o texto e o estilo; NÃO deixa como estava
    let (sx, sy) = (ORIGIN.0 + n.x + n.w / 2, ORIGIN.1 + n.y + 6);
    p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
    assert_eq!(p.words_text(), Some("Oi tudo bem"));
    assert_eq!(balloon(&p).style.shape, BalloonShape::OvalTail);
    p.update(&press(balloon_easel::NO), 0.0, &a);
    assert_eq!(p.objects().items[0].source, n.source);
    // OK com o texto mudado troca o mesmo objeto
    p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
    p.update(&[text("Tchau")], 0.0, &a);
    p.update(&press(balloon_easel::OK), 0.0, &a);
    assert_eq!(p.objects().len(), 1);
    assert!(matches!(&p.objects().items[0].source, Source::Words { text, .. } if text == "Tchau"));
}

#[test]
fn comics_use_the_same_easels() {
    let (_d, a) = empty_assets();
    let mut p = Paint::new_project(crate::scenes::projects::ProjectKind::Comics, None, &a);
    let r = crate::scenes::projects::comics::digit_rect(0);
    p.update(&[click(r.0 + 10, r.1 + 10), click(228, 30)], 0.0, &a);
    p.update(&press(OI_BTN), 0.0, &a);
    assert_eq!(p.words_easel(), Some(false));
    p.update(&press(balloon_easel::OK), 0.0, &a);
    assert_eq!(p.objects().len(), 1);
}

#[test]
fn shaped_words_survive_save_and_reopen() {
    let (_d, a) = empty_assets();
    let dir = tempfile::tempdir().unwrap();
    let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
    p.update(&[click(228, 30)], 0.0, &a);
    p.update(&press(FORMA_BTN), 0.0, &a);
    p.update(&[text("Sol")], 0.0, &a);
    p.update(&press(shaped_easel::button_rect(1)), 0.0, &a);
    let (x, y) = shaped_item(&p, 7);
    p.update(&[click(x, y), up(x, y)], 0.0, &a);
    p.update(&press(shaped_easel::OK), 0.0, &a);
    let before = p.objects().items[0].clone();
    p.leave();
    let q = Paint::new(Some(dir.path().to_path_buf()), &a);
    assert_eq!(q.objects().len(), 1);
    let o = &q.objects().items[0];
    assert_eq!(o.source, before.source);
    assert_eq!(
        (o.x, o.y, o.w, o.h),
        (before.x, before.y, before.w, before.h)
    );
    assert_eq!(o.image.px, before.image.px);
}

#[test]
fn roll_without_library_stays_closed() {
    let (_d, a) = empty_assets();
    let mut p = words_tool(&a);
    p.update(&press(ROLL), 0.0, &a);
    assert!(!p.tails_column_open());
    // o menu da lista de estilos usa os rótulos do original
    p.update(&press(FORMA_BTN), 0.0, &a);
    p.update(&press(shaped_easel::button_rect(2)), 0.0, &a);
    let Some(Easel::Shaped(e)) = &p.words.easel else {
        panic!()
    };
    assert_eq!(e.menu(), Some(Menu::Styles));
    // desenhar com tudo aberto não trava
    let mut f = crate::Frame::new(640, 480);
    p.draw(&mut f, &a);
}
