//! Engine do Ateliê Livre: recursos originais, quadro 640×480 e cenas.
//! Não depende de janela nem de dispositivo de som.

#![forbid(unsafe_code)]

pub mod assets;
pub mod balloon;
pub mod clipart;
pub mod frame;
pub mod paint;
pub mod paths;
pub mod scene;
pub mod scenes;
pub mod writer;

pub use formats::wave::Pcm;
pub use formats::winres::Image;
pub use frame::{Frame, SCREEN_H, SCREEN_W};
