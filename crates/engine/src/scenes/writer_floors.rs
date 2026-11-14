//! Oficinas de Projetos (andar 3) e de Ideias (andar 4) do Creative Writer
//! (tarefa T7; `cw/report/building-salas.md` §4–5): o Max (`7e03` em pé,
//! `7dff` sentado; som `283`), os bichos da parede (alce JORNAL, burro
//! CARTÃO, rinoceronte FAIXA), a Máquina de Ideias e o Quadro de Figuras,
//! balões `0bb8[31]`/`[42]`, `[69]`/`[70]`/`[71]`.
//!
//! Escreva aqui os `RoomSpec` com `kit: &WRITER_KIT` (`writer_rooms`) e
//! troque [`PROJECTS`] e [`IDEAS`] por eles. Os objetos com balão de
//! escolha já existem (`Pick::Newspaper`…`Pick::PictureBoard`); os dados
//! deles estão em [`pick`] (registro, posição medida, texto reserva, dica
//! do "Ignorar" e o que a primeira opção começa).

use super::room_spec::RoomSpec;
use super::room_talk::{Pick, PickSpec, Start};
use super::rooms_data;
use crate::scene::{CwScene, SceneId};

/// Oficina de Projetos do Writer (até T7, a do Fine Artist).
pub const PROJECTS: &RoomSpec = &rooms_data::PROJECTS_ROOM;
/// Oficina de Ideias do Writer (até T7, os Truques do Fine Artist).
pub const IDEAS: &RoomSpec = &rooms_data::TRICKS_ROOM;

/// Enquanto os projetos do Writer não existem, a primeira opção abre a
/// área de escrita com um documento novo.
const START: Start = Start::Scene(SceneId::Cw(CwScene::NewDocument));

/// Balão de escolha dos objetos do Writer (`WHAT 0bb8`; posições medidas,
/// `building-salas.md` §4–5).
pub fn pick(p: Pick) -> PickSpec {
    let (record, pos, w, text, options, skip) = match p {
        Pick::Newspaper => (
            36,
            (28, 109),
            210,
            "Você quer:",
            [
                "Começar um novo jornal?",
                "Abrir um jornal que você já criou?",
            ],
            38,
        ),
        Pick::Card => (
            35,
            (205, 81),
            204,
            "Oi! Você quer:",
            [
                "Criar um novo cartão?",
                "Trabalhar com um cartão que você já criou?",
            ],
            38,
        ),
        Pick::Banner => (
            37,
            (225, 217),
            202,
            "Oi! Você quer:",
            [
                "Fazer uma nova faixa?",
                "Trabalhar com uma faixa que você já criou?",
            ],
            38,
        ),
        Pick::IdeaMachine => (
            44,
            (82, 231),
            193,
            "Tudo bem! Você quer:",
            [
                "Começar uma coisa nova?",
                "Trabalhar com uma história que você já começou?",
            ],
            45,
        ),
        _ => (
            49,
            (326, 77),
            180,
            "O Quadro de Figuras! Você quer:",
            [
                "Começar do começo",
                "Trabalhar com uma história que você já começou",
            ],
            50,
        ),
    };
    // a Máquina de Idéias e o Quadro de Figuras abrem os close-ups
    let start = match p {
        Pick::IdeaMachine => Start::Scene(SceneId::Cw(CwScene::IdeaMachine)),
        Pick::PictureBoard => Start::Scene(SceneId::Cw(CwScene::PictureBoard)),
        _ => START,
    };
    PickSpec {
        record,
        pos,
        w,
        text,
        options,
        skip,
        start,
    }
}
