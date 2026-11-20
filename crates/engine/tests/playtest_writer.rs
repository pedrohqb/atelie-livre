//! Teste de jogo do Creative Writer pelo diretor de cenas: caminhos de
//! criança entre partes feitas por agentes diferentes (Escritório, salas,
//! Oficina de Idéias, Projetos, Biblioteca). Cada teste reproduz um defeito
//! achado no teste de jogo do Writer (`cw/report/playtest-writer.md`).

mod common;

use common::{Director, real_assets};
use engine::scene::{CwScene, Key, SceneId};
use engine::scenes::room::Floor;

/// Escritório com documento novo, já sem o balão de chegada do McZee.
fn office(d: &mut Director) {
    d.idle(10);
    // o primeiro clique fecha o balão de boas-vindas
    d.click(300, 300);
    d.idle(5);
}

/// Clica numa ferramenta da barra (x no meio da casa dela).
fn tool(d: &mut Director, x: i32) {
    d.click(x, 30);
    d.idle(3);
}

const WORD: i32 = 35;
const SOUNDS: i32 = 128;

/// Digita duas linhas e põe a capitular ("Era uma vez") no 1º parágrafo.
fn typed_with_drop_cap(d: &mut Director) {
    d.click(200, 200);
    d.text("Era uma vez um gato chamado Bolinha. Ele morava numa casa amarela");
    d.key(Key::Enter);
    d.text("e gostava de pular");
    tool(d, WORD);
    d.click(515, 77); // Mais escolhas: página 2
    d.click(180, 77); // Mix de estilos
    d.click(190, 168); // Era uma vez
    d.click(300, 186); // no 1º parágrafo
    d.click(470, 77); // Ponto de inserção: larga o pincel
}

fn library_sounds(d: &Director, from: usize) -> usize {
    d.trace_since(from)
        .iter()
        .filter(|l| l.starts_with("TRACE som Library"))
        .count()
}

/// Som da mão numa palavra da 1ª linha de um parágrafo com capitular: a
/// palavra ganha o som (e ele toca), como nas outras linhas.
#[test]
fn sound_goes_to_a_word_beside_a_drop_cap() {
    let Some(a) = real_assets() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let mut d = Director::new(
        &a,
        tmp.path(),
        SceneId::Cw(CwScene::NewDocument),
        Some("bia"),
    );
    office(&mut d);
    typed_with_drop_cap(&mut d);
    tool(&mut d, SOUNDS);
    d.click(50, 80); // teatro "Grave"
    d.click(75, 168); // Lá3: toca e vai para a mão
    let from = d.trace.len();
    d.click(234, 186); // "gato", ao lado da capitular
    assert_eq!(
        library_sounds(&d, from),
        1,
        "o som não foi para a palavra: {:?}",
        d.trace_since(from)
    );
}

/// Caminho de criança entre as telas do Writer, com os pontos de clique do
/// teste de jogo no app: elevador do saguão ao Escritório, clique duplo na
/// prancheta, porta Saída e "Jogar fora" de volta à sala; Oficina de Idéias,
/// Máquina, FEITO, "Escrever uma história", e a Saída volta à Oficina.
#[test]
fn kid_path_between_writer_screens() {
    let Some(a) = real_assets() else { return };
    let tmp = tempfile::tempdir().unwrap();
    let lobby = SceneId::Cw(CwScene::Building(Floor::Lobby));
    let mut d = Director::new(&a, tmp.path(), lobby, Some("bia"));
    d.idle(60);
    d.click(400, 420); // fecha o balão de chegada
    d.click(497, 233); // botão do elevador
    d.idle(120);
    d.click(580, 212); // andar 2
    // o elevador troca o andar dentro da mesma sala (sem troca de cena)
    d.idle(400);
    let studio = SceneId::Cw(CwScene::Building(Floor::Studio));
    d.click(400, 450);
    d.double_click(375, 160); // prancheta "Documento Novo"
    assert!(
        d.wait_for(SceneId::Cw(CwScene::NewDocument), 120),
        "{:?}",
        d.trace_since(0)
    );
    d.idle(10);
    d.click(300, 300);
    d.text("Era uma vez");
    d.click(610, 50); // porta Saída
    d.idle(30);
    d.click(228, 275); // Jogar fora
    assert!(d.wait_for(studio, 200), "{:?}", d.trace_since(0));
    // Oficina de Idéias: Máquina, FEITO, escrever, sair
    let ideas = SceneId::Cw(CwScene::Building(Floor::Tricks));
    let mut d = Director::new(&a, tmp.path(), ideas, Some("bia"));
    d.idle(60);
    d.click(300, 420);
    d.click(200, 330); // a máquina
    d.idle(30);
    d.click(95, 263); // "Começar uma coisa nova"
    assert!(d.wait_for(SceneId::Cw(CwScene::IdeaMachine), 200));
    d.idle(60);
    d.click(300, 100);
    d.click(465, 190); // a alavanca do peixe
    d.idle(300);
    d.click(355, 187); // FEITO
    d.idle(30);
    d.click(192, 203); // "Escrever uma história com essa sentença"
    let story = SceneId::Cw(CwScene::IdeaStory(
        engine::writer::ideas::story::IdeaSource::Machine,
    ));
    assert!(d.wait_for(story, 300), "{:?}", d.trace_since(0));
    d.idle(30);
    d.click(610, 50);
    d.idle(30);
    d.click(228, 275); // Jogar fora
    assert!(d.wait_for(ideas, 200), "{:?}", d.trace_since(0));
    let quits = d
        .trace
        .iter()
        .filter(|l| l.starts_with("perdido-no-leave"))
        .count();
    assert_eq!(quits, 0, "{:?}", d.trace);
}
