//! Estado da Oficina de Idéias que vale para a sessão inteira do programa
//! (as cenas são recriadas a cada troca de tela):
//!
//! - a Máquina de Idéias: a sentença é sorteada uma vez e continua a mesma
//!   ao voltar à Máquina (`ideas.md` §3, "Entrada");
//! - a última figura vista no Quadro de Figuras;
//! - a última aba usada do Caderno de Idéias;
//! - a área de transferência do Caderno (tesoura/câmera), que o editor cola;
//! - a história pendente para o editor (I10: "Escrever uma história com
//!   essa sentença/figura"), que o Escritório integrado pega com
//!   [`take_story`].
//!
//! Fica num `thread_local` (o laço do programa é uma thread só; cada teste
//! roda na sua thread e tem a sua sessão). Trocar de usuário zera tudo.

use super::machine::{Machine, Piece};
use super::story::IdeaStory;
use crate::Image;
use std::cell::RefCell;

/// Abas do Caderno de Idéias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// "Máquina de Idéias": as sentenças guardadas.
    #[default]
    Machine,
    /// "Figuras": uma página por figura.
    Pictures,
    /// "Lembretes": texto livre.
    Notes,
}

/// O que a tesoura ou a câmera do Caderno puseram na área de transferência.
#[derive(Debug, Clone)]
pub enum Clip {
    /// Texto (com figuras-rebus) — uma sentença ou um trecho dos Lembretes.
    Text(Vec<Piece>),
    /// Uma figura do Caderno, com o título.
    Picture(Image, String),
}

/// Qual figura do Quadro estava à mostra: do Max (número do `PICT`) ou do
/// usuário (título, já que a lista muda quando ele põe figuras novas).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardKey {
    Max(u16),
    User(String),
}

#[derive(Default)]
struct Session {
    user: String,
    machine: Option<Machine>,
    board: Option<BoardKey>,
    tab: Tab,
    clip: Option<Clip>,
    story: Option<IdeaStory>,
}

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::default());
}

fn with<R>(user: &str, f: impl FnOnce(&mut Session) -> R) -> R {
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        if s.user != user {
            *s = Session {
                user: user.to_string(),
                ..Session::default()
            };
        }
        f(&mut s)
    })
}

/// A Máquina da sessão do usuário, se já foi aberta.
pub fn machine(user: &str) -> Option<Machine> {
    with(user, |s| s.machine.clone())
}

/// Guarda a Máquina (sentença e entradas) para a próxima vez.
pub fn set_machine(user: &str, m: &Machine) {
    with(user, |s| s.machine = Some(m.clone()));
}

pub fn board(user: &str) -> Option<BoardKey> {
    with(user, |s| s.board.clone())
}

pub fn set_board(user: &str, k: BoardKey) {
    with(user, |s| s.board = Some(k));
}

/// Aba em que o Caderno abre (a última usada).
pub fn tab(user: &str) -> Tab {
    with(user, |s| s.tab)
}

pub fn set_tab(user: &str, t: Tab) {
    with(user, |s| s.tab = t);
}

/// Área de transferência do Caderno (o editor cola daqui).
pub fn clip(user: &str) -> Option<Clip> {
    with(user, |s| s.clip.clone())
}

pub fn set_clip(user: &str, c: Clip) {
    with(user, |s| s.clip = Some(c));
}

/// Deixa a história para o editor abrir (antes de `ChangeScene` para
/// [`IdeaStory::scene`]).
pub fn set_story(user: &str, story: IdeaStory) {
    with(user, |s| s.story = Some(story));
}

/// A história pendente, uma vez só (o editor a pega ao ser montado).
pub fn take_story(user: &str) -> Option<IdeaStory> {
    with(user, |s| s.story.take())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_live_for_the_user_until_another_one_comes() {
        set_tab("ana", Tab::Notes);
        set_board("ana", BoardKey::Max(3));
        assert_eq!(tab("ana"), Tab::Notes);
        assert_eq!(board("ana"), Some(BoardKey::Max(3)));
        // outro usuário: sessão nova
        assert_eq!(tab("beto"), Tab::Machine);
        assert_eq!(board("beto"), None);
        assert_eq!(board("ana"), None);
    }

    #[test]
    fn story_is_taken_once() {
        let st = IdeaStory::from_sentence(&[Piece::Text("Era uma vez".into())]);
        set_story("u", st);
        assert!(take_story("u").is_some());
        assert!(take_story("u").is_none());
    }
}
