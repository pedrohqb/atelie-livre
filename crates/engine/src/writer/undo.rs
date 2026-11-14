//! Pilha de desfazer do Ovo (Escritório).
//!
//! O original guarda até **20** ações (balão `0fa9[0]`); a faixa do Ovo mostra
//! um ovo chocado por ação que pode ser desfeita e um ovo inteiro por ação
//! que pode ser refeita, e Ctrl+Z **alterna** desfazer e refazer (o segundo
//! Ctrl+Z refaz). A pilha é genérica no estado guardado: a cena guarda um
//! [`Snapshot`] (documento e seleção) antes de cada ação.
//!
//! Digitação: no original, um texto digitado de uma vez volta em dois
//! passos — o 1º desfazer apaga tudo menos a 1ª letra e o 2º apaga a 1ª
//! letra. [`UndoStack::record_typing`] reproduz isso: a primeira tecla é uma
//! ação e as seguintes, até uma pausa de [`TYPING_PAUSE`] ou outra ação,
//! formam a segunda.

use super::doc::Document;
use super::selection::Selection;
use std::collections::VecDeque;

/// Ações guardadas.
pub const UNDO_LIMIT: usize = 20;
/// Pausa de digitação que fecha uma ação, em segundos.
pub const TYPING_PAUSE: f32 = 1.0;

/// Faixa do Ovo: fila de ovos a partir de (144, 68), um a cada 13 px.
pub const EGG_X0: i32 = 144;
pub const EGG_Y: i32 = 68;
pub const EGG_STEP: i32 = 13;
/// Ovo chocado (ação que pode ser desfeita), 13×19, no `WRITER.PIC`.
pub const EGG_HATCHED: u16 = 0x08e3;
/// Ovo inteiro (ação que pode ser refeita), 13×17.
pub const EGG_WHOLE: u16 = 0x08df;

/// Um ovo da faixa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Egg {
    Hatched,
    Whole,
}

impl Egg {
    pub fn pic(self) -> u16 {
        match self {
            Egg::Hatched => EGG_HATCHED,
            Egg::Whole => EGG_WHOLE,
        }
    }
}

/// Estado guardado pela cena antes de cada ação.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub doc: Document,
    pub sel: Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Typing {
    #[default]
    Idle,
    /// A primeira tecla virou uma ação; a próxima abre a segunda.
    First,
    /// As teclas entram na ação aberta.
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Last {
    #[default]
    Other,
    Undo,
    Redo,
}

/// Desfazer/refazer de até [`UNDO_LIMIT`] ações.
#[derive(Debug, Clone)]
pub struct UndoStack<T> {
    /// Estados antes de cada ação, do mais antigo ao mais novo.
    undo: VecDeque<T>,
    /// Estados desfeitos, o próximo a refazer no fim.
    redo: Vec<T>,
    typing: Typing,
    idle: f32,
    last: Last,
}

impl<T> Default for UndoStack<T> {
    fn default() -> UndoStack<T> {
        UndoStack {
            undo: VecDeque::new(),
            redo: Vec::new(),
            typing: Typing::Idle,
            idle: 0.0,
            last: Last::Other,
        }
    }
}

impl<T> UndoStack<T> {
    pub fn new() -> UndoStack<T> {
        UndoStack::default()
    }

    fn push(&mut self, before: T) {
        self.undo.push_back(before);
        while self.undo.len() > UNDO_LIMIT {
            self.undo.pop_front();
        }
        self.redo.clear();
        self.last = Last::Other;
    }

    /// Antes de uma ação (Enter, estilo, adesivo, colar, Jogar tudo fora…):
    /// guarda o estado e fecha a digitação.
    pub fn record(&mut self, before: T) {
        self.push(before);
        self.typing = Typing::Idle;
    }

    /// Antes de digitar ou apagar uma letra: `before` só é chamado quando a
    /// tecla abre uma ação (ver o comentário do módulo).
    pub fn record_typing(&mut self, before: impl FnOnce() -> T) {
        match self.typing {
            Typing::Idle => {
                self.push(before());
                self.typing = Typing::First;
            }
            Typing::First => {
                self.push(before());
                self.typing = Typing::Open;
            }
            Typing::Open => {}
        }
        self.idle = 0.0;
    }

    /// Fecha a digitação aberta (o cursor mudou de lugar, outra ferramenta…).
    pub fn close(&mut self) {
        self.typing = Typing::Idle;
    }

    /// Passa o tempo; uma pausa de [`TYPING_PAUSE`] fecha a digitação.
    pub fn tick(&mut self, dt: f32) {
        if self.typing != Typing::Idle {
            self.idle += dt.max(0.0);
            if self.idle >= TYPING_PAUSE {
                self.typing = Typing::Idle;
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Ações que podem ser desfeitas (ovos chocados).
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Ações que podem ser refeitas (ovos inteiros).
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// DESFAZER: troca `cur` pelo estado antes da última ação.
    pub fn undo(&mut self, cur: &mut T) -> bool {
        let Some(prev) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push(std::mem::replace(cur, prev));
        self.typing = Typing::Idle;
        self.last = Last::Undo;
        true
    }

    /// REFAZER: volta a última ação desfeita.
    pub fn redo(&mut self, cur: &mut T) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push_back(std::mem::replace(cur, next));
        self.typing = Typing::Idle;
        self.last = Last::Redo;
        true
    }

    /// Ctrl+Z: depois de um desfazer, refaz; senão, desfaz (e, sem nada a
    /// desfazer, refaz).
    pub fn toggle(&mut self, cur: &mut T) -> bool {
        if self.last == Last::Undo && self.can_redo() {
            self.redo(cur)
        } else {
            self.undo(cur) || self.redo(cur)
        }
    }

    /// Os ovos da faixa, da esquerda para a direita: os chocados e depois
    /// os inteiros (no máximo [`UNDO_LIMIT`] ao todo).
    pub fn eggs(&self) -> Vec<Egg> {
        std::iter::repeat_n(Egg::Hatched, self.undo.len())
            .chain(std::iter::repeat_n(Egg::Whole, self.redo.len()))
            .collect()
    }

    /// Posição do `k`-ésimo ovo na faixa.
    pub fn egg_pos(k: usize) -> (i32, i32) {
        (EGG_X0 + EGG_STEP * k as i32, EGG_Y)
    }

    /// Esquece tudo (documento novo ou aberto).
    pub fn clear(&mut self) {
        *self = UndoStack::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::{CharProps, Pos};

    #[test]
    fn keeps_twenty_actions() {
        let mut u = UndoStack::new();
        let mut cur = 0;
        for k in 1..=25 {
            u.record(cur);
            cur = k;
        }
        assert_eq!(u.undo_len(), 20);
        let mut n = 0;
        while u.undo(&mut cur) {
            n += 1;
        }
        assert_eq!(n, 20);
        assert_eq!(cur, 5, "as 5 mais antigas se perderam");
        assert_eq!(u.redo_len(), 20);
        assert_eq!(u.eggs().len(), 20);
        assert!(u.eggs().iter().all(|&e| e == Egg::Whole));
        while u.redo(&mut cur) {}
        assert_eq!(cur, 25);
    }

    #[test]
    fn new_action_drops_redo() {
        let mut u = UndoStack::new();
        let mut cur = "a".to_string();
        u.record(cur.clone());
        cur = "ab".into();
        u.record(cur.clone());
        cur = "abc".into();
        assert!(u.undo(&mut cur));
        assert_eq!(cur, "ab");
        assert_eq!(u.eggs(), vec![Egg::Hatched, Egg::Whole]);
        u.record(cur.clone());
        cur = "abX".into();
        assert!(!u.can_redo());
        assert!(u.undo(&mut cur));
        assert_eq!(cur, "ab");
        assert_eq!(u.redo_len(), 1);
    }

    #[test]
    fn ctrl_z_alternates() {
        let mut u = UndoStack::new();
        let mut cur = 0;
        assert!(!u.toggle(&mut cur));
        u.record(cur);
        cur = 1;
        u.record(cur);
        cur = 2;
        assert!(u.toggle(&mut cur));
        assert_eq!(cur, 1);
        assert!(u.toggle(&mut cur));
        assert_eq!(cur, 2, "o 2º Ctrl+Z refaz");
        assert!(u.toggle(&mut cur));
        assert_eq!(cur, 1);
        // DESFAZER duas vezes na faixa, depois Ctrl+Z refaz uma
        assert!(u.undo(&mut cur));
        assert_eq!(cur, 0);
        assert!(u.toggle(&mut cur));
        assert_eq!(cur, 1);
        // sem nada a desfazer, Ctrl+Z refaz
        let mut u = UndoStack::new();
        let mut cur = 0;
        u.record(cur);
        cur = 1;
        u.undo(&mut cur);
        u.close();
        u.last = Last::Other;
        assert!(u.toggle(&mut cur));
        assert_eq!(cur, 1);
    }

    #[test]
    fn typing_undoes_in_two_steps_like_the_original() {
        let mut u: UndoStack<Snapshot> = UndoStack::new();
        let mut cur = Snapshot::default();
        let props = CharProps::default();
        for c in "abc".chars() {
            u.record_typing(|| cur.clone());
            let s = &mut cur;
            s.sel.type_text(&mut s.doc, &c.to_string(), props);
            u.tick(0.2);
        }
        assert_eq!(u.undo_len(), 2);
        assert!(u.undo(&mut cur));
        assert_eq!(cur.doc.text(), "a", "1º desfazer: tudo menos a 1ª letra");
        assert_eq!(cur.sel.caret, Pos::new(0, 1));
        assert!(u.undo(&mut cur));
        assert_eq!(cur.doc.text(), "");
        assert!(u.redo(&mut cur) && u.redo(&mut cur));
        assert_eq!(cur.doc.text(), "abc");
    }

    #[test]
    fn pause_and_other_actions_close_typing() {
        let mut u = UndoStack::new();
        let mut calls = 0;
        let mut rec = |u: &mut UndoStack<i32>| {
            u.record_typing(|| {
                calls += 1;
                calls
            })
        };
        rec(&mut u);
        rec(&mut u);
        rec(&mut u);
        assert_eq!(u.undo_len(), 2);
        u.tick(TYPING_PAUSE);
        rec(&mut u);
        assert_eq!(u.undo_len(), 3);
        u.record(0);
        rec(&mut u);
        assert_eq!(u.undo_len(), 5);
        u.close();
        rec(&mut u);
        assert_eq!(u.undo_len(), 6);
        let mut cur = 0;
        u.undo(&mut cur);
        rec(&mut u);
        assert_eq!((u.undo_len(), u.redo_len()), (6, 0));
        u.clear();
        assert_eq!(u.undo_len(), 0);
    }

    #[test]
    fn egg_strip() {
        assert_eq!(UndoStack::<u8>::egg_pos(0), (144, 68));
        assert_eq!(UndoStack::<u8>::egg_pos(19), (144 + 13 * 19, 68));
        assert_eq!(Egg::Hatched.pic(), 0x08e3);
        assert_eq!(Egg::Whole.pic(), 0x08df);
    }
}
