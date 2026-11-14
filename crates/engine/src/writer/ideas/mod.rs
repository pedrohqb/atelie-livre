//! Oficina de Idéias do Creative Writer, sem SDL: a Máquina de Idéias
//! (sentenças malucas), as figuras-rebus e a gravação das ideias do usuário
//! (entradas da Máquina, figuras próprias do Quadro e o Caderno de Idéias).
//! As cenas ficam em `scenes/`.

pub mod machine;

pub use machine::{Grammar, Machine, Mode, Piece, RemoveError, SPIN_MS, Slot, WINDOWS};
