//! Engine do Ateliê Livre: recursos originais, quadro 640×480 e cenas.
//! Não depende de janela nem de dispositivo de som.

#![forbid(unsafe_code)]

pub mod frame;
pub mod paths;

pub use formats::winres::Image;
pub use frame::{Frame, SCREEN_H, SCREEN_W};
