//! Salas do prédio do Creative Writer (T5–T7, T11–T14): fluxo com assets
//! vazios, dados conferidos com os reais (`ATELIE_DATA`) e o despejo das
//! salas em estados fixos para as folhas de comparação com o original.

use super::*;
use crate::scene::CwScene;
use crate::scenes::room_spec::Action;
use crate::scenes::testutil::{assets, empty_assets};
use crate::scenes::writer_rooms::{self, WRITER_KIT, rec, snd};
use crate::scenes::{writer_floors, writer_gags as g, writer_lobby, writer_office_room};

fn click(x: i32, y: i32) -> Input {
    Input::Click {
        button: MouseButton::Left,
        x,
        y,
    }
}

fn sounds(fx: &[Effect]) -> Vec<u16> {
    fx.iter()
        .filter_map(|e| match e {
            Effect::PlaySound(SoundRef::Snd(Program::Writer, id)) => Some(*id),
            _ => None,
        })
        .collect()
}

fn changes(fx: &[Effect]) -> Vec<SceneId> {
    fx.iter()
        .filter_map(|e| match e {
            Effect::ChangeScene(id) => Some(*id),
            _ => None,
        })
        .collect()
}

fn spot(r: &Room, action: Action) -> Input {
    let h = r
        .spec
        .hotspots
        .iter()
        .find(|h| h.action == action)
        .unwrap_or_else(|| panic!("{action:?}"));
    click(h.rect.0 + h.rect.2 / 2, h.rect.1 + h.rect.3 / 2)
}

fn option(r: &Room, a: &Assets, k: usize) -> Input {
    let (bx, by, _, _) = r.balloon.as_ref().unwrap().option_rect(a, k);
    click(bx + 20, by + 3)
}

fn writer(floor: Floor, a: &Assets) -> Room {
    Room::for_program(Program::Writer, floor, None, None, a).with_user("ana", a)
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "atelie-cw-rooms-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn writer_floors_use_writer_rooms() {
    let (_d, a) = empty_assets();
    for (f, bg) in [
        (Floor::Lobby, writer_lobby::BACKGROUND),
        (Floor::Studio, writer_office_room::BACKGROUND),
        (Floor::Projects, writer_floors::PROJECTS_BG),
        (Floor::Tricks, writer_floors::IDEAS_BG),
    ] {
        let r = writer(f, &a);
        assert_eq!(r.spec.background, bg, "{f:?}");
        assert_eq!(r.kit().program, Program::Writer);
        assert!(r.balloon_open(), "{f:?}: balão de chegada");
        let c = r.character().unwrap();
        assert_eq!(c.program, Program::Writer);
        match f {
            Floor::Projects => assert_eq!((c.image, (c.x, c.y)), g::MAX_STANDING),
            Floor::Tricks => {
                assert!(
                    g::MAX_POSES
                        .iter()
                        .any(|p| p.0 == c.image && p.1 == (c.x, c.y))
                )
            }
            _ => assert!(g::MCZEE_COSTUMES.contains(&c.image)),
        }
    }
}

/// Saguão: as cinco zonas do mapa, o mostrador que só toca, a seta
/// "Biblioteca", o skate "LÁ FORA", as portas e o botão do elevador.
#[test]
fn lobby_map_dial_and_exits() {
    let (_d, a) = empty_assets();
    for (pt, floor) in [
        ((100, 40), Floor::Tricks),
        ((100, 66), Floor::Projects),
        ((100, 86), Floor::Studio),
        ((60, 133), Floor::Attic),
    ] {
        let mut r = writer(Floor::Lobby, &a);
        r.close();
        let fx = r.update(&[click(pt.0, pt.1)], 0.0, &a);
        assert_eq!(r.floor(), floor, "{pt:?}");
        assert_eq!(sounds(&fx)[0], snd::BOARD);
    }
    let mut r = writer(Floor::Lobby, &a);
    r.close();
    let fx = r.update(&[click(20, 105)], 0.0, &a);
    assert_eq!(changes(&fx), [SceneId::Cw(CwScene::Library)]);
    assert_eq!(sounds(&fx), [snd::BOARD]);
    // o mostrador só toca o som, e fecha o balão aberto
    let mut r = writer(Floor::Lobby, &a);
    assert!(r.balloon_open());
    let fx = r.update(&[click(320, 40)], 0.0, &a);
    assert!(!r.balloon_open());
    assert_eq!(sounds(&fx), [snd::DIAL]);
    assert!(changes(&fx).is_empty());
    let fx = r.update(&[spot(&r, Action::Gallery(snd::LIBRARY_ARROW))], 0.0, &a);
    assert_eq!(changes(&fx), [SceneId::Cw(CwScene::Library)]);
    assert_eq!(sounds(&fx), [snd::LIBRARY_ARROW]);
    let fx = r.update(&[spot(&r, Action::Outside)], 0.0, &a);
    assert_eq!(changes(&fx), [SceneId::Cw(CwScene::Street)]);
    assert_eq!(sounds(&fx), [snd::SKATE]);
    // botão de chamar e portas: elevador, com 2d4 + 311
    for pt in [(498, 240), (340, 300)] {
        let mut r = writer(Floor::Lobby, &a);
        r.close();
        let fx = r.update(&[click(pt.0, pt.1)], 0.0, &a);
        assert!(r.in_elevator(), "{pt:?}");
        assert_eq!(sounds(&fx)[..2], [snd::ELEVATOR_DOORS, 0x311]);
    }
}

/// Opções do balão de chegada do saguão: as dicas `[2]`–`[5]`.
#[test]
fn lobby_arrival_options_open_tips() {
    let (_d, a) = empty_assets();
    for (k, tip) in [
        (0, rec::LOBBY_OFFICE),
        (1, rec::LOBBY_PROJECTS),
        (2, rec::LOBBY_IDEAS),
        (3, rec::LOBBY_LIBRARY),
    ] {
        let mut r = writer(Floor::Lobby, &a);
        let o = option(&r, &a, k);
        let fx = r.update(&[o], 0.0, &a);
        assert_eq!(sounds(&fx), [snd::CHOICE]);
        let t = r.spec.tip(tip).unwrap();
        let b = r.balloon.as_ref().unwrap();
        assert_eq!((b.x, b.y), t.pos, "dica [{tip}]");
    }
}

/// A luminária troca pela da fumaça enquanto o "EPA!" está aberto.
#[test]
fn lamp_smokes_while_quit_is_open() {
    let (_d, a) = empty_assets();
    let mut r = writer(Floor::Lobby, &a);
    r.close();
    let fx = r.update(&[spot(&r, Action::Quit)], 0.0, &a);
    assert!(r.quit_open());
    assert_eq!(sounds(&fx), WRITER_KIT.snd.quit);
    let l = r.spec.lamp.unwrap();
    assert_eq!(l.smoke.0, g::LAMP_SMOKE);
    assert_eq!(l.smoke.1.1, l.normal.1.1 - 1);
    let mut f = Frame::new(640, 480);
    r.draw(&mut f, &a);
}

/// Escritório: clique duplo na prancheta = documento novo; o quadro e a
/// plaqueta, sem documento, não quebram.
#[test]
fn office_clipboard_and_wall_without_data() {
    let (_d, a) = empty_assets();
    let mut r = writer(Floor::Studio, &a);
    r.close();
    let easel = spot(&r, Action::Easel);
    let fx = r.update(std::slice::from_ref(&easel), 0.0, &a);
    // o balão sai da boca do McZee
    let b = r.balloon.as_ref().unwrap();
    assert_eq!((b.x, b.y), writer_rooms::MCZEE_MOUTH);
    assert_eq!(sounds(&fx), [snd::OBJECT]);
    let fx = r.update(&[easel], 0.1, &a);
    assert_eq!(changes(&fx), [SceneId::Cw(CwScene::NewDocument)]);
    // a plaqueta da prancheta só mostra a dica
    let mut r = writer(Floor::Studio, &a);
    r.close();
    r.update(&[click(380, 240)], 0.0, &a);
    assert!(r.naming.is_none());
    let mut f = Frame::new(640, 480);
    r.draw(&mut f, &a);
}

/// Projetos: as opções da chegada abrem os balões dos bichos; Ctrl+clique
/// começa direto; Ideias: Ctrl+clique na máquina e no quadro abre as telas.
#[test]
fn projects_and_ideas_objects() {
    let (_d, a) = empty_assets();
    let mut r = writer(Floor::Projects, &a);
    let o = option(&r, &a, 0);
    let fx = r.update(&[o], 0.0, &a);
    assert_eq!(r.talk, Talk::Pick(Pick::Newspaper));
    assert_eq!(sounds(&fx), [snd::CHOICE, snd::OBJECT, snd::PICK]);
    let b = r.balloon.as_ref().unwrap();
    assert_eq!((b.x, b.y), (28, 121));
    assert_eq!(b.options.len(), 3);
    for (pt, pick) in [
        ((130, 70), Pick::Newspaper),
        ((300, 50), Pick::Card),
        ((300, 170), Pick::Banner),
    ] {
        let mut r = writer(Floor::Projects, &a);
        r.close();
        r.update(&[Input::Ctrl(true)], 0.0, &a);
        let fx = r.update(&[click(pt.0, pt.1)], 0.0, &a);
        assert_eq!(
            changes(&fx),
            [SceneId::Cw(CwScene::NewDocument)],
            "{pick:?}"
        );
    }
    for (pt, to) in [
        ((220, 330), CwScene::IdeaMachine),
        ((430, 150), CwScene::PictureBoard),
    ] {
        let mut r = writer(Floor::Tricks, &a);
        r.close();
        r.update(&[Input::Ctrl(true)], 0.0, &a);
        let fx = r.update(&[click(pt.0, pt.1)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Cw(to)]);
    }
    // "Outra coisa?" e "Quero fazer outra coisa": as dicas do mastro
    let mut r = writer(Floor::Projects, &a);
    let o = option(&r, &a, 3);
    r.update(&[o], 0.0, &a);
    assert_eq!(
        (r.balloon.as_ref().unwrap().x, r.balloon.as_ref().unwrap().y),
        (339, 121)
    );
    let mut r = writer(Floor::Tricks, &a);
    let o = option(&r, &a, 2);
    r.update(&[o], 0.0, &a);
    assert_eq!(r.balloon.as_ref().unwrap().links.len(), 1);
}

/// O mastro do Writer dá a mesma volta do Fine Artist.
#[test]
fn writer_pole_cycle() {
    let (_d, a) = empty_assets();
    let mut r = writer(Floor::Lobby, &a);
    let mut seen = vec![];
    for _ in 0..5 {
        r.close();
        let up = spot(&r, Action::PoleUp);
        r.update(&[up], 0.0, &a);
        seen.push(r.floor());
    }
    assert_eq!(
        seen,
        [
            Floor::Studio,
            Floor::Projects,
            Floor::Tricks,
            Floor::Attic,
            Floor::Lobby
        ]
    );
}

/// A bolsa aberta e a ajuda desligada valem entre salas e sessões.
#[test]
fn bag_and_help_persist() {
    let (_d, a) = empty_assets();
    let users = tmp("persist");
    let mut r = writer(Floor::Lobby, &a).with_users_dir(Some(users.clone()), &a);
    r.close();
    let l = r.spec.bag;
    let fx = r.update(&[click(l.closed.0 + 20, l.closed.1 + 20)], 0.0, &a);
    assert!(r.bag().open);
    assert_eq!(sounds(&fx), [WRITER_KIT.snd.bag_open]);
    // outra sala: continua aberta
    let up = spot(&r, Action::PoleUp);
    r.close();
    r.update(&[up], 0.0, &a);
    assert_eq!(r.floor(), Floor::Studio);
    assert!(r.bag().open);
    // outra sessão
    let r = writer(Floor::Projects, &a).with_users_dir(Some(users.clone()), &a);
    assert!(r.bag().open);
    // ajuda desligada
    let mut r = writer(Floor::Lobby, &a).with_users_dir(Some(users.clone()), &a);
    r.close();
    r.help_off = false;
    // sem os dados a figura do McZee não tem tamanho: o menu direto
    r.open_helper(&mut Vec::new(), &a);
    assert!(matches!(r.talk, Talk::Helper { off: false }));
    let o = option(&r, &a, 0);
    r.update(&[o], 0.0, &a);
    assert!(r.help_off());
    let r = writer(Floor::Studio, &a).with_users_dir(Some(users), &a);
    assert!(r.help_off());
    assert!(!r.balloon_open());
    // o Fine Artist não guarda nada
    let r = Room::new(Floor::Lobby, None, None, &a).with_users_dir(Some(tmp("fa")), &a);
    assert!(!r.bag().open);
}

/// Ctrl+O abre a pasta (só no Writer).
#[test]
fn ctrl_o_opens_the_portfolio() {
    let (_d, a) = empty_assets();
    let key = Input::Key {
        key: Key::O,
        ctrl: true,
        alt: false,
    };
    let mut r = writer(Floor::Projects, &a);
    let fx = r.update(std::slice::from_ref(&key), 0.0, &a);
    assert!(fx.contains(&Effect::Push(SceneId::Cw(CwScene::Portfolio))));
    assert_eq!(sounds(&fx), [WRITER_KIT.snd.suitcase]);
    let mut r = Room::new(Floor::Lobby, None, None, &a);
    let fx = r.update(&[key], 0.0, &a);
    assert!(!fx.iter().any(|e| matches!(e, Effect::Push(_))));
}

/// Com os dados reais: figuras, máscaras, balões, roteiros.
#[test]
fn writer_rooms_match_real_data() {
    let Some(a) = assets() else { return };
    let p = Program::Writer;
    for f in [Floor::Lobby, Floor::Studio, Floor::Projects, Floor::Tricks] {
        let spec = writer_rooms::room(f);
        assert!(a.has_image(&ImageRef::Pic(p, spec.background)), "{f:?}");
        for e in spec.elements {
            assert_eq!(a.pic_next_id(p, e.id, 2), Some(e.mask), "{f:?} {:#x}", e.id);
        }
        let l = spec.lamp.unwrap();
        for id in [l.normal.0, l.smoke.0] {
            assert!(a.pic_next_id(p, id, 2).is_some(), "{id:#x}");
        }
        let recs = a.dialogs(p, WRITER_KIT.what);
        for t in spec.tips.iter().chain(WRITER_KIT.tips) {
            let r = &recs[t.record];
            assert!(r.options().is_empty(), "[{}]", t.record);
            let b = talk::tip_in(&WRITER_KIT, Some(t), &a, t.record, "ana");
            assert!(!b.text.is_empty(), "[{}]", t.record);
            assert_eq!(b.links.len(), t.links.len(), "[{}] {:?}", t.record, b.links);
        }
        for &id in spec
            .gags
            .iter()
            .chain(spec.surprises.pole)
            .chain(spec.surprises.hole)
            .chain(spec.wand_props.iter().flat_map(|w| w.scripts))
        {
            assert!(crate::anim::load_exe(&a, p, id).is_some(), "ATAG {id:#x}");
        }
        // o balão de chegada vem do original (opções esperadas)
        let r = writer(f, &a);
        let b = r.balloon.as_ref().unwrap();
        assert_eq!(b.w, recs[spec.arrival.unwrap().record].width as i32);
    }
    for c in g::MCZEE_COSTUMES {
        let i = a.image(&ImageRef::Pic(p, *c));
        assert_eq!((i.width, i.height), (149, 354), "{c:#x}");
    }
    for (id, _) in g::MAX_POSES {
        assert!(a.pic_next_id(p, *id, 2).is_some());
    }
    for pk in [
        Pick::Newspaper,
        Pick::Card,
        Pick::Banner,
        Pick::IdeaMachine,
        Pick::PictureBoard,
    ] {
        let b = pk.balloon_in(&WRITER_KIT, &a, "ana");
        assert_eq!(b.options.len(), 3, "{pk:?}");
        assert_eq!(b.options[talk::IGNORE], "Ignorar", "{pk:?}");
    }
    // a "Tecla de atalho" vira ligação
    let r = writer(Floor::Studio, &a);
    let b = r.tip(rec::OFFICE_POLE, &a);
    let targets: Vec<usize> = b.links.iter().map(|l| l.0).collect();
    assert_eq!(targets, [rec::POLE_DOWN, rec::HOTKEY_UP]);
    // a varinha do Writer tem o cursor e o balão dela
    assert!(a.cursor(&WRITER_KIT.wand_cursor).is_some());
    let w = wand_balloon(&WRITER_KIT, &a, "ana").unwrap();
    assert!(w.text.contains("Varinha"), "{}", w.text);
}

/// Com os dados reais: o quadro do Escritório mostra "O Ponto de Fuga" do
/// Max (dica `[0]`, plaqueta com o "EPA!"); um documento do usuário (cópia
/// de um exemplo) troca de título pela plaqueta.
#[test]
fn office_wall_document_with_real_data() {
    let Some(a) = assets() else { return };
    let users = tmp("office");
    let mut r = writer(Floor::Studio, &a).with_users_dir(Some(users.clone()), &a);
    let w = r.last_work.clone().expect("último documento");
    assert_eq!(w.title, "O Ponto de Fuga");
    assert!(!w.own);
    let img = w.image.as_ref().expect("miniatura");
    assert_eq!((img.width, img.height), (73, 98));
    r.close();
    let page = spot(&r, Action::Wall);
    r.update(std::slice::from_ref(&page), 0.0, &a);
    assert_eq!(r.balloon.as_ref().unwrap().w, 188, "dica [0] do Max");
    r.close();
    r.clicks.reset();
    let fx = r.update(&[spot(&r, Action::WallLabel)], 1.0, &a);
    assert!(r.alert.is_some());
    assert_eq!(sounds(&fx), [snd::OBJECT, WRITER_KIT.snd.quit[1]]);
    r.update(&[click(10, 10)], 0.0, &a);
    assert!(r.alert.is_none());
    // documento da Ana
    let src = crate::paths::find_ci(a.data_dir(), "SHARED/USERS/MAX/SAMP1.MAX").unwrap();
    std::fs::create_dir_all(users.join("ana")).unwrap();
    let mine = users.join("ana").join("CARTAO.MAX");
    std::fs::copy(&src, &mine).unwrap();
    crate::writer::prefs::remember_last(&users, "ana", a.data_dir(), &mine, "Meu cartão").unwrap();
    let mut r = writer(Floor::Studio, &a).with_users_dir(Some(users.clone()), &a);
    let w = r.last_work.clone().unwrap();
    assert!(w.own);
    assert_eq!(w.title, "Meu cartão");
    r.close();
    r.update(&[spot(&r, Action::WallLabel)], 0.0, &a);
    assert!(r.naming.is_some());
    r.update(&[Input::Text("Oi".into())], 0.0, &a);
    let fx = r.update(
        &[Input::Key {
            key: Key::Enter,
            ctrl: false,
            alt: false,
        }],
        0.0,
        &a,
    );
    assert!(
        fx.iter()
            .any(|e| matches!(e, Effect::Log(s) if s.contains("Oi"))),
        "{fx:?}"
    );
    let doc = formats::max::read(&std::fs::read(&mine).unwrap()).unwrap();
    assert_eq!(doc.add.title, "Oi");
    let p = crate::writer::prefs::load(Some(&users), "ana", a.data_dir());
    assert_eq!(p.wall_slot(0).unwrap().title, "Oi");
}

/// Com os dados reais: a varinha num bicho da parede põe o acessório dele
/// por 3 s e depois a bolsa se abre.
#[test]
fn wand_on_wall_animals_with_real_data() {
    let Some(a) = assets() else { return };
    let mut r = writer(Floor::Projects, &a);
    r.close();
    r.bag.open = true;
    r.bag.holding_wand = true;
    r.update(&[click(300, 50)], 0.0, &a);
    let s = r.surprise.as_ref().expect("acessório do burro");
    let (x, y) = s.position();
    assert!(
        (240..400).contains(&x) && (-60..120).contains(&y),
        "{x},{y}"
    );
    r.update(&[], 3.5, &a);
    assert!(r.surprise.is_none());
    assert!(r.bag().open);
}

/// Fixa o que a sala sorteia (fantasia, pose).
fn pin(r: &mut Room) {
    if let Some(c) = r.character.as_mut() {
        if g::MCZEE_COSTUMES.contains(&c.image) {
            c.image = g::MCZEE_COSTUMES[0];
        } else {
            (c.image, (c.x, c.y)) = g::MAX_STANDING;
        }
    }
}

/// Estados de uma sala no despejo: nome e cliques até ele.
type Steps<'a> = &'a [(&'a str, &'a [(i32, i32)])];

/// Com `ATELIE_DATA` e `ATELIE_DUMP`: grava quadros das salas do Writer nos
/// estados das capturas do original (`cw/shots/building_*`), para as folhas
/// de comparação (`cw-rooms-tools/sheets.py`).
#[test]
fn dump_writer_rooms_for_comparison() {
    let Some(a) = assets() else { return };
    if std::env::var_os("ATELIE_DUMP").is_none() {
        return;
    }
    // uma pasta de usuários nova por quadro: a bolsa aberta não passa adiante
    let users = || tmp("dump");
    let mut f = Frame::new(640, 480);
    let shot = |r: &mut Room, f: &mut Frame, name: &str| {
        pin(r);
        r.draw(f, &a);
        crate::scenes::testutil::dump(f, name);
    };
    let steps: &[(Floor, Steps)] = &[
        (
            Floor::Lobby,
            &[
                ("closed", &[(560, 200)]),
                ("mczee", &[(560, 200), (40, 300)]),
                ("lamp", &[(560, 200), (175, 200)]),
                ("bag_open", &[(560, 200), (150, 420)]),
            ],
        ),
        (
            Floor::Studio,
            &[
                ("base", &[(420, 300)]),
                ("frame", &[(420, 300), (200, 150)]),
                ("clip", &[(420, 300), (377, 150)]),
                ("frameplate", &[(420, 300), (200, 240)]),
            ],
        ),
        (
            Floor::Projects,
            &[
                ("base", &[(420, 300)]),
                ("max", &[(420, 300), (60, 330)]),
                ("moose", &[(420, 300), (130, 70)]),
                ("donkey", &[(420, 300), (300, 50)]),
                ("rhino", &[(420, 300), (300, 170)]),
            ],
        ),
        (
            Floor::Tricks,
            &[
                ("base", &[(420, 300)]),
                ("machine", &[(420, 300), (220, 330)]),
                ("quadro", &[(420, 300), (430, 150)]),
            ],
        ),
    ];
    for (floor, list) in steps {
        let mut r = writer(*floor, &a).with_users_dir(Some(users()), &a);
        shot(&mut r, &mut f, &format!("cw_{floor:?}_arrival"));
        for (name, clicks) in list.iter() {
            let mut r = writer(*floor, &a).with_users_dir(Some(users()), &a);
            for &(x, y) in clicks.iter() {
                r.update(&[click(x, y)], 0.0, &a);
                r.clicks.reset();
            }
            shot(&mut r, &mut f, &format!("cw_{floor:?}_{name}"));
        }
        // as opções da chegada, uma por uma
        let n = writer_rooms::room(*floor).arrival.unwrap().actions.len();
        for k in 0..n {
            let mut r = writer(*floor, &a).with_users_dir(Some(users()), &a);
            let o = option(&r, &a, k);
            r.update(&[o], 0.0, &a);
            shot(&mut r, &mut f, &format!("cw_{floor:?}_o{}", k + 1));
        }
    }
}
