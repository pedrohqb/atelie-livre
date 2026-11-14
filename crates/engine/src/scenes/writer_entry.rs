//! Entrada do Creative Writer (T1): a mesma tela do Fine Artist com os dados
//! do `WRITER.EXE`/`WRITER.PIC`/`WRITER.SND`, medidos no original
//! (`cw/report/building.md` §3–4).
//!
//! - Fundo `3c41` (idêntico ao `4139` do FA) e McZee com livro `4c43`
//!   (difere do FA no livro); botões, setas e bolinhas têm os mesmos ids e
//!   desenhos nos dois `.PIC`.
//! - Balões `WHAT 4e00` (Entrada) e `4e02` (avisos); "EPA!" de saída
//!   `0805[12]` ("Sair do Creative Writer").
//! - Sons próprios: nome na lista `22d`, McZee e Procurar `281`, bolinha e
//!   "Continuar trabalhando" `2ff`; os outros são os do FA.
//! - Opção 4 "Ir direto para o Escritório": área de escrita com um
//!   documento novo, com a vinheta de entrada no Escritório (`304`).

use super::entry_spec::{EntrySounds, EntrySpec, QuitSpec};
use crate::assets::Program;
use crate::scene::{CwScene, SceneId};

/// "EPA!" do Creative Writer (`WHAT 0805[12]` "idsQuit", 223×75).
pub const WRITER_QUIT: QuitSpec = QuitSpec {
    program: Program::Writer,
    block: 0x0805,
    record: 12,
    w: 223,
    options: ["Sair do Creative Writer", "Continuar trabalhando"],
};

pub const WRITER_ENTRY: EntrySpec = EntrySpec {
    program: Program::Writer,
    background: 0x3c41,
    mczee: (0x4c43, 435, 112),
    what: 0x4e00,
    errors: 0x4e02,
    quit: &WRITER_QUIT,
    alert_art: 0x2cf,
    snd: EntrySounds {
        opening: [0x300, 0x2f6, 0x2fd],
        select: 0x22d,
        // não medido no Writer: o mesmo tique do nome
        select_home: 0x22d,
        arrow: 0x2bc,
        mczee: 0x281,
        me: 0x239,
        first: 0x2fb,
        // 13868 B: `281`, `295` e `2d3` têm o mesmo tamanho; a tabela de
        // pontos quentes dá `281` para o McZee
        browse: 0x281,
        stop: 0x2ec,
        alert: 0x2fe,
        refuse: 0x217,
        choice: 0x2ff,
        next: 0x2eb,
        back: 0x2f5,
    },
    direct: SceneId::Cw(CwScene::NewDocument),
    direct_sounds: &[0x304],
    direct_label: "Ir direto para o Escritório",
    default_user: "escritor",
};
