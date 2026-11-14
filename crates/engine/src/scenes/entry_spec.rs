//! Dados da Entrada por programa ([`EntrySpec`]): a tela, os botões e o
//! cadastro são os mesmos no Fine Artist e no Creative Writer (o mesmo
//! `~USER.INF` em `USERS\`); mudam o fundo, o McZee, os blocos `WHAT`, o
//! "EPA!" de saída, alguns sons e o destino da opção 4 ("Ir direto
//! para…"). O do Writer está em `writer_entry`.

use super::entry::{self, snd};
use crate::assets::Program;
use crate::scene::SceneId;
use crate::text_balloon::PIC_ALERT;

/// "EPA! O que você quer fazer?" (Sair do programa / Continuar
/// trabalhando), da Entrada e das salas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuitSpec {
    pub program: Program,
    /// Bloco e registro do `WHAT`.
    pub block: u16,
    pub record: usize,
    /// Largura e opções reserva (sem os dados originais).
    pub w: i32,
    pub options: [&'static str; 2],
}

/// Sons da Entrada (ids do `.SND` do programa).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntrySounds {
    /// Ao entrar na tela, em sequência.
    pub opening: [u16; 3],
    /// Tique ao escolher um nome da lista.
    pub select: u16,
    /// Tique ao escolher a pasta no passo "Onde você quer que eu guarde".
    pub select_home: u16,
    /// Setas da lista.
    pub arrow: u16,
    /// Clique no McZee.
    pub mczee: u16,
    /// "Sou eu" e "Coloque aqui".
    pub me: u16,
    /// "Primeira vez".
    pub first: u16,
    /// "Procurar".
    pub browse: u16,
    /// Botão Para.
    pub stop: u16,
    /// Vinheta dos avisos "EPA!".
    pub alert: u16,
    /// Clique fora de um balão modal.
    pub refuse: u16,
    /// Opção escolhida (bolinha), inclusive "Continuar trabalhando".
    pub choice: u16,
    /// Botão Segue.
    pub next: u16,
    /// Botão Volta.
    pub back: u16,
}

/// A Entrada de um programa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntrySpec {
    pub program: Program,
    /// Fundo (céu, nuvens, plataforma).
    pub background: u16,
    /// McZee de livro na mão: figura colorida (1 bit e máscara são as duas
    /// seguintes no `.PIC`) e posição.
    pub mczee: (u16, i32, i32),
    /// Blocos `WHAT` da Entrada e dos avisos.
    pub what: u16,
    pub errors: u16,
    pub quit: &'static QuitSpec,
    /// Arte "EPA!" (casinha do Rex e cachorro) do `.PIC` do programa. Os
    /// balões (`TextBalloon`) ainda a desenham do `ARTIST.PIC` (a figura é
    /// a mesma); fica aqui para quando eles souberem o programa.
    pub alert_art: u16,
    pub snd: EntrySounds,
    /// Opção 4 de "Que bom ver você de novo" ("Ir direto para…"): o destino
    /// e sons tocados junto com a bolinha.
    pub direct: SceneId,
    pub direct_sounds: &'static [u16],
    /// Texto reserva da opção 4.
    pub direct_label: &'static str,
    /// Nome no lugar do `@` quando ninguém foi escolhido.
    pub default_user: &'static str,
}

/// "EPA!" do Fine Artist (`WHAT 1001[2]`).
pub const ARTIST_QUIT: QuitSpec = QuitSpec {
    program: Program::Artist,
    block: 0x1001,
    record: 2,
    w: 210,
    options: ["Sair do Fine Artist", "Continuar trabalhando"],
};

/// Entrada do Fine Artist (medida no original; `docs/telas/fine-artist.md`).
pub const ARTIST_ENTRY: EntrySpec = EntrySpec {
    program: Program::Artist,
    background: entry::BACKGROUND,
    mczee: entry::MCZEE,
    what: 0x4400,
    errors: 0x4402,
    quit: &ARTIST_QUIT,
    alert_art: PIC_ALERT,
    snd: EntrySounds {
        opening: snd::OPENING,
        select: snd::SELECT,
        select_home: snd::SELECT_HOME,
        arrow: snd::ARROW,
        mczee: snd::MCZEE,
        me: snd::ME,
        first: snd::FIRST,
        browse: snd::BROWSE,
        stop: snd::STOP,
        alert: snd::ALERT,
        refuse: snd::REFUSE,
        choice: snd::CHOICE,
        next: snd::NEXT,
        back: snd::BACK,
    },
    // direto à tela de pintura, sem passar pelo cômodo
    direct: SceneId::Paint,
    direct_sounds: &[],
    direct_label: "Ir direto para o Ateliê de Pintura",
    default_user: "artista",
};

/// A Entrada do programa `p`.
pub fn for_program(p: Program) -> &'static EntrySpec {
    match p {
        Program::Artist => &ARTIST_ENTRY,
        Program::Writer => &super::writer_entry::WRITER_ENTRY,
    }
}
