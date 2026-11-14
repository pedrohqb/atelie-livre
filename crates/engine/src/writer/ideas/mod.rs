//! Oficina de Idéias do Creative Writer, sem SDL: a Máquina de Idéias
//! (sentenças malucas), as figuras-rebus, a gravação das ideias do usuário
//! (entradas da Máquina, figuras próprias do Quadro e o Caderno de Idéias),
//! a lista de figuras do Quadro, o texto do Caderno, o estado da sessão e
//! a ponte para o editor. As cenas ficam em `scenes/writer_ideas`.

pub mod board;
pub mod machine;
pub mod notes;
pub mod rebus;
pub mod session;
pub mod store;
pub mod story;

pub use machine::{Grammar, Machine, Mode, Piece, RemoveError, SPIN_MS, Slot, WINDOWS};
