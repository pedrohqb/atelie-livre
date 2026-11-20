use super::*;
use crate::scenes::testutil::{assets, empty_assets};
use crate::writer::doc::Orientation;
use crate::writer::object::{FloatObject, ObjRef, ObjectKind};

fn click(x: i32, y: i32) -> Input {
    Input::Click {
        button: MouseButton::Left,
        x,
        y,
    }
}

fn key(key: Key) -> Input {
    Input::Key {
        key,
        ctrl: false,
        alt: false,
    }
}

fn text(s: &str) -> Input {
    Input::Text(s.to_string())
}

fn sounds(fx: &[Effect]) -> Vec<u16> {
    fx.iter()
        .filter_map(|e| match e {
            Effect::PlaySound(SoundRef::Snd(Program::Writer, id)) => Some(*id),
            _ => None,
        })
        .collect()
}

fn logs(fx: &[Effect]) -> String {
    fx.iter()
        .filter_map(|e| match e {
            Effect::Log(s) => Some(s.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Projeto `kind` do usuário "ana" numa pasta de usuários temporária, já
/// começado (primeiro quadro).
fn project(kind: Kind, a: &Assets) -> (tempfile::TempDir, ProjectScene) {
    let users = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(users.path().join("ana")).unwrap();
    let mut p = ProjectScene::new(
        kind,
        Some(users.path().to_path_buf()),
        "ana",
        None,
        "Ana Lima",
        a,
    );
    p.update(&[], 0.0, a);
    (users, p)
}

fn shaped(p: &mut ProjectScene) {
    let r = ObjRef {
        tag: *b"SPLE",
        id: 0,
    };
    let o = FloatObject::new(ObjectKind::Shaped, r, 0, 100, 120, 200, 40);
    let ed = p.writer_mut().editor_mut();
    ed.record();
    ed.add_object(o, None);
}

/// Pontos de clique (os das âncoras do original).
const WORD_TOOL: (i32, i32) = (36, 26);
const PAGE_TOOL: (i32, i32) = (180, 28);
const PRINT_TOOL: (i32, i32) = (492, 26);
const SHAPE: (i32, i32) = (466, 76);
const COLUMNS: (i32, i32) = (200, 70);
const PRINT_MCZEE: (i32, i32) = (370, 76);
/// Fileira de baixo da porta, fora do retângulo do Max.
const DOOR_BOTTOM: (i32, i32) = (600, 103);

fn at(p: &mut ProjectScene, a: &Assets, (x, y): (i32, i32)) -> Vec<Effect> {
    p.update(&[click(x, y)], 0.0, a)
}

#[test]
fn starts_in_project_mode_with_the_max() {
    let (_d, a) = empty_assets();
    let users = tempfile::tempdir().unwrap();
    let mut p = ProjectScene::new(Kind::News, Some(users.path().into()), "ana", None, "", &a);
    let fx = p.update(&[], 0.0, &a);
    assert_eq!(p.assistant().at(), "início");
    assert!(logs(&fx).contains("assistente: passo início"));
    // o McZee não dá as boas-vindas: quem fala é o Max
    assert!(!sounds(&fx).contains(&super::super::writer::mczee::SND_ENTER));
    let w = p.writer();
    assert_eq!(w.floor(), "Oficina de Projetos");
    assert_eq!(w.title(), "Estamos fazendo um Jornal");
    assert_eq!(w.doc().dop.columns, 2);
    assert!(w.doc().dop.newspaper);
    assert_eq!(w.editor().extra.add.project.to_bytes(), *b"news");
}

#[test]
fn the_door_is_the_max() {
    let (_d, a) = empty_assets();
    let (_u, mut p) = project(Kind::Card, &a);
    at(&mut p, &a, DOOR_BOTTOM);
    // Max fora de hora: "O que você quer que eu faça agora?" (3803[0]),
    // nunca o "Epa!" da porta Saída
    assert!(p.assistant().quitting());
    assert!(p.writer().dialog().is_none());
    assert!(!p.writer().door_open());
}

#[test]
fn tool_button_menu_and_shaped_word_drive_the_news_script() {
    let (_d, a) = empty_assets();
    let (_u, mut p) = project(Kind::News, &a);
    let mut fx = Vec::new();
    p.send(Event::Option(1), &mut fx, &a);
    p.send(Event::Option(0), &mut fx, &a);
    assert_eq!(p.assistant().at(), "colunas");
    at(&mut p, &a, PAGE_TOOL);
    assert_eq!(p.writer().tool(), ToolId::Page);
    assert_eq!(p.assistant().at(), "cria-colunas");
    at(&mut p, &a, COLUMNS);
    assert_eq!(p.assistant().at(), "n-colunas");
    // o menu de colunas é da Preparação da página: qualquer interface que
    // mude o documento serve
    p.writer_mut().editor_mut().apply_dop(|d| d.set_columns(3));
    p.update(&[], 0.0, &a);
    assert_eq!(p.assistant().at(), "manchete?");
    p.send(Event::Option(0), &mut fx, &a);
    // a Palavra não está ativa (a Página está): "Clique aqui."
    assert_eq!(p.assistant().at(), "palavra");
    at(&mut p, &a, WORD_TOOL);
    assert_eq!(p.assistant().at(), "forma");
    at(&mut p, &a, SHAPE);
    assert_eq!(p.assistant().at(), "manchete");
    shaped(&mut p);
    p.update(&[], 0.0, &a);
    assert_eq!(p.assistant().at(), "livre");
}

#[test]
fn page_click_and_levers_are_events() {
    let (_d, a) = empty_assets();
    let (_u, mut p) = project(Kind::Card, &a);
    at(&mut p, &a, PAGE_TOOL);
    // a alavanca leva os rolos BORDA para a faixa
    at(&mut p, &a, (515, 70));
    assert_eq!(
        p.classify(150, 70, &a),
        [Event::ButtonClicked(Button::BorderRoll)]
    );
    assert_eq!(p.classify(300, 300, &a), [Event::PageClicked]);
    // FORMA só na primeira página da faixa da Palavra
    at(&mut p, &a, WORD_TOOL);
    assert_eq!(
        p.classify(SHAPE.0, SHAPE.1, &a),
        [Event::ButtonClicked(Button::Shape)]
    );
    at(&mut p, &a, (515, 70));
    assert!(p.writer().tools().word.more());
    assert!(p.classify(SHAPE.0, SHAPE.1, &a).is_empty());
    at(&mut p, &a, (80, 28));
    assert_eq!(p.writer().tool(), ToolId::Stickers);
    assert_eq!(
        p.classify(40, 70, &a),
        [Event::ButtonClicked(Button::StickerRoll)]
    );
    assert_eq!(
        p.classify(PRINT_TOOL.0, PRINT_TOOL.1, &a),
        [Event::ToolClicked(Tool::Print)]
    );
}

#[test]
fn save_names_marks_the_file_then_print_and_leave() {
    let (_d, a) = empty_assets();
    let (users, mut p) = project(Kind::News, &a);
    let mut fx = Vec::new();
    for o in [1, 1, 1] {
        p.send(Event::Option(o), &mut fx, &a);
    }
    assert_eq!(p.assistant().at(), "livre");
    p.update(&[text("Notícias da escola")], 0.0, &a);
    at(&mut p, &a, (600, 50));
    assert_eq!(p.assistant().at(), "salvar");
    p.send(Event::Option(0), &mut fx, &a);
    assert_eq!(p.assistant().at(), "nome");
    assert!(p.writer().renaming().is_some());
    p.update(&[text("Jornal Teste")], 0.0, &a);
    p.update(&[key(Key::Enter)], 0.0, &a);
    assert_eq!(p.assistant().at(), "imprimir?");
    // nome DOS a partir do título, com as marcas do projeto
    let file = users.path().join("ana").join("JORNAL_T.MAX");
    let m = formats::max::read(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(m.add.project, formats::max::Project::Newspaper);
    assert_eq!(m.add.floor, formats::max::FLOOR_PROJECTS);
    assert_eq!(m.add.view, 1);
    assert_eq!(m.add.title, "Jornal Teste");
    assert_eq!(p.writer().title(), "Jornal Teste");
    // imprimir: ferramenta, botão do McZee (a máquina de impressão do
    // Escritório abre) e, quando ela fecha, "Tudo pronto!"
    p.send(Event::Option(0), &mut fx, &a);
    assert_eq!(p.assistant().at(), "imprimir");
    at(&mut p, &a, PRINT_TOOL);
    assert_eq!(p.assistant().at(), "imprimir-mczee");
    let fx = at(&mut p, &a, PRINT_MCZEE);
    assert!(logs(&fx).contains("imprimir:"), "{}", logs(&fx));
    assert!(p.writer().printing());
    assert_eq!(p.assistant().at(), "imprimindo");
    // o Max fica apagado por baixo da máquina
    at(&mut p, &a, (600, 50));
    assert!(!p.assistant().quitting());
    p.update(&[key(Key::Escape)], 0.0, &a);
    assert!(!p.writer().printing());
    assert_eq!(p.assistant().at(), "pronto");
    let mut fx = Vec::new();
    p.send(Event::Option(2), &mut fx, &a);
    assert!(
        fx.contains(&Effect::ChangeScene(SceneId::Cw(CwScene::Building(
            Floor::Projects
        ))))
    );
    let s = sounds(&fx);
    assert!(s.ends_with(&SND_LEAVE));
}

#[test]
fn trash_discards_and_new_starts_a_blank_project() {
    let (_d, a) = empty_assets();
    let (_u, mut p) = project(Kind::News, &a);
    let mut fx = Vec::new();
    for o in [1, 1, 1] {
        p.send(Event::Option(o), &mut fx, &a);
    }
    // documento vazio: o Rex avisa e o Max fica no passo
    at(&mut p, &a, (600, 50));
    assert_eq!(p.assistant().at(), "livre");
    assert!(p.assistant().alert().is_some());
    p.send(Event::Dismiss, &mut fx, &a);
    p.update(&[text("Oi")], 0.0, &a);
    at(&mut p, &a, (600, 50));
    p.send(Event::Option(1), &mut fx, &a);
    assert_eq!(p.assistant().at(), "lixo");
    p.send(Event::Option(1), &mut fx, &a);
    assert_eq!(p.assistant().at(), "pronto");
    assert!(!p.writer().editor().changed(), "jogado fora");
    assert!(p.leave().is_empty(), "nada a gravar ao sair");
    p.send(Event::Option(1), &mut fx, &a);
    assert_eq!(p.assistant().at(), "início");
    assert!(project::is_blank(p.writer().doc()));
    assert_eq!(p.writer().doc().dop.columns, 2);
}

#[test]
fn banner_pages_field_stretches_the_word_across_the_sheets() {
    let (_d, a) = empty_assets();
    let (_u, mut p) = project(Kind::Banner, &a);
    let mut fx = Vec::new();
    p.send(Event::Option(1), &mut fx, &a);
    // a Palavra já está ativa: vai direto ao botão Forma
    assert_eq!(p.assistant().at(), "forma");
    at(&mut p, &a, SHAPE);
    assert_eq!(p.assistant().at(), "mensagem");
    shaped(&mut p);
    p.update(&[], 0.0, &a);
    assert_eq!(p.assistant().at(), "páginas");
    let o = &p.writer().doc().objects[0];
    assert_eq!((o.x, o.width), (72, 577 - 144), "de margem a margem");
    p.update(&[text("3")], 0.0, &a);
    assert_eq!(p.assistant().field(), Some("3"));
    let fx = p.update(&[key(Key::Enter)], 0.0, &a);
    assert!(logs(&fx).contains("faixa com 3 páginas"));
    assert_eq!(p.assistant().at(), "livre");
    let d = p.writer().doc();
    assert_eq!(d.min_pages, 3);
    assert_eq!(d.objects[0].width, 1587);
    assert_eq!(p.writer().editor().layout().pages.len(), 3);
    assert_eq!(p.writer().editor().layout().pages[2].objects, [0]);
}

#[test]
fn banner_refuses_tall_and_backgrounds_with_the_rex() {
    let (_d, a) = empty_assets();
    let (_u, mut p) = project(Kind::Banner, &a);
    p.writer_mut()
        .editor_mut()
        .apply_dop(|d| std::mem::swap(&mut d.page_width, &mut d.page_height));
    let fx = p.update(&[], 0.0, &a);
    assert!(sounds(&fx).contains(&SND_REX));
    assert_eq!(
        (
            p.writer().doc().dop.page_width,
            p.writer().doc().dop.page_height
        ),
        project::BANNER_PAGE
    );
    // o jornal muda de orientação à vontade
    let (_u, mut n) = project(Kind::News, &a);
    n.writer_mut()
        .editor_mut()
        .apply_dop(|d| d.set_orientation(Orientation::Wide));
    n.update(&[], 0.0, &a);
    assert_eq!(n.writer().doc().dop.orientation(), Orientation::Wide);
}

#[test]
fn card_fifth_page_brings_the_rex() {
    let (_d, a) = empty_assets();
    let (_u, mut p) = project(Kind::Card, &a);
    assert_eq!(p.writer().editor().layout().pages.len(), 4);
    p.writer_mut().editor_mut().doc_mut().min_pages = 5;
    p.update(&[], 0.0, &a);
    let al = p.assistant().alert().expect("aviso da 5ª página");
    assert_eq!((al.block, al.record), (0x3c00, 106));
}

/// Com os dados reais: a pasta do assistente só tem as amostras de jornal
/// do Max, os balões apontam a seta do Título e o Já, e o Já abre a
/// amostra (como cópia) e segue o roteiro.
#[test]
fn real_assets_samples_folder_filters_by_project() {
    let Some(a) = assets() else { return };
    let (_u, mut p) = project(Kind::News, &a);
    let mut fx = Vec::new();
    p.send(Event::Option(0), &mut fx, &a);
    assert_eq!(p.assistant().at(), "amostra");
    assert_eq!(
        p.samples().unwrap(),
        ["Jornal Musical", "Jornal Mágico", "Jornal Tradicional"]
    );
    let placed = p.assistant().placed(&a);
    let more = placed
        .iter()
        .find(|b| b.shown.anchor == max::Anchor::Folder(max::Spot::More))
        .unwrap();
    // à direita da seta (o original em x = 522; o balão do port é mais
    // largo e encosta na borda da tela)
    assert_eq!(more.balloon.y, 277);
    assert!(more.balloon.x > 496 && more.balloon.x + more.balloon.box_width() <= 636);
    let ok = placed
        .iter()
        .find(|b| b.shown.anchor == max::Anchor::Folder(max::Spot::Ok))
        .unwrap();
    assert_eq!((ok.balloon.x, ok.balloon.y), (240, 380));
    let (gx, gy, _, _) = super::super::portfolio::BUTTON_GO;
    at(&mut p, &a, (gx + 5, gy + 5));
    assert!(p.samples().is_none());
    assert_eq!(p.assistant().at(), "amostra-ok");
    assert_eq!(p.writer().title(), "Jornal Musical");
    assert!(p.writer().editor().file().is_none(), "cópia da amostra");
    // as faixas: só as três do Max
    let (_u, mut b) = project(Kind::Banner, &a);
    b.send(Event::Option(0), &mut fx, &a);
    assert_eq!(b.samples().unwrap().len(), 3);
}

/// Com os dados reais: a borda da folha vai para dentro do `.MAX`.
#[test]
fn real_assets_border_is_copied_when_saving() {
    let Some(a) = assets() else { return };
    let lib = formats::chnk::parse(a.library_bytes("0MAX100.BOR").unwrap()).unwrap();
    let c = lib.chunks.iter().find(|c| &c.tag.0 == b"BDR ").unwrap();
    let r = ObjRef {
        tag: *b"BDR ",
        id: c.id,
    };
    let (users, mut p) = project(Kind::News, &a);
    p.writer_mut()
        .editor_mut()
        .apply_dop(|d| d.set_border(Some(r)));
    p.update(&[text("Parabéns")], 0.0, &a);
    let mut fx = Vec::new();
    for o in [1, 1, 1, 0] {
        if o == 0 {
            // o Max no trabalho livre: salvar
            at(&mut p, &a, (600, 50));
        }
        p.send(Event::Option(o), &mut fx, &a);
    }
    assert_eq!(p.assistant().at(), "nome");
    assert!(p.writer().editor().extra.object(r).is_some());
    p.update(&[text("Jornal Borda"), key(Key::Enter)], 0.0, &a);
    let m =
        formats::max::read(&std::fs::read(users.path().join("ana/JORNAL_B.MAX")).unwrap()).unwrap();
    assert!(
        m.object(formats::max::ObjRef::new(b"BDR ", c.id)).is_some(),
        "BDR copiada"
    );
    assert_eq!(m.add.project, formats::max::Project::Newspaper);
}

/// Um projeto gravado e reaberto pela pasta abre no Escritório sem o
/// assistente: andar "Oficina de Projetos" e Saída para o andar.
#[test]
fn reopened_project_exits_to_the_projects_floor() {
    let (_d, a) = empty_assets();
    let (users, mut p) = project(Kind::Card, &a);
    p.update(&[text("Oi")], 0.0, &a);
    let file = p
        .writer_mut()
        .editor_mut()
        .save_as("Cartao Teste", "Ana", false)
        .unwrap();
    let dir = users.path().join("ana");
    let w = Writer::open(Some(dir.clone()), &file, &a).unwrap();
    assert_eq!(w.floor(), project::FLOOR_LABEL);
    assert_eq!(w.exit_to(), SceneId::Cw(CwScene::Building(Floor::Projects)));
    // um documento do Escritório continua saindo para a sala
    let mut e = Writer::new_blank(Some(dir), &a);
    e.editor_mut().type_text("x");
    let f = e.editor_mut().save_as("Texto", "Ana", false).unwrap();
    let w = Writer::open(Some(users.path().join("ana")), &f, &a).unwrap();
    assert_ne!(w.floor(), project::FLOOR_LABEL);
}

/// Com os dados reais: FORMA abre o cavalete de palavras com forma do
/// Escritório, os balões apontam o campo e o OK (`projects_33_manchete_3`)
/// e o OK põe a palavra, o que leva o assistente adiante.
#[test]
fn real_assets_shaped_easel_ok_moves_the_news_on() {
    let Some(a) = assets() else { return };
    let (_u, mut p) = project(Kind::News, &a);
    let mut fx = Vec::new();
    for o in [1, 1, 0] {
        p.send(Event::Option(o), &mut fx, &a);
    }
    assert_eq!(p.assistant().at(), "forma");
    at(&mut p, &a, SHAPE);
    assert_eq!(p.assistant().at(), "manchete");
    assert!(p.writer().shaped().is_open());
    let placed = p.assistant().placed(&a);
    let ok = placed
        .iter()
        .find(|b| b.shown.anchor == max::Anchor::Easel(max::Spot::Ok))
        .unwrap();
    assert_eq!(ok.balloon.y, 418);
    assert!(ok.balloon.x + ok.balloon.box_width() > 512);
    // o Max fica apagado com o cavalete aberto
    at(&mut p, &a, (600, 50));
    assert!(!p.assistant().quitting());
    let (x, y, w, h) = crate::scenes::paint::words::shaped_easel::OK;
    at(&mut p, &a, (x + w / 2, y + h / 2));
    assert!(!p.writer().shaped().is_open());
    assert_eq!(p.writer().doc().objects.len(), 1);
    assert_eq!(p.assistant().at(), "livre");
}
