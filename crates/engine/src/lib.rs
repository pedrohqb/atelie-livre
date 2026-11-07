//! Engine do Ateliê Livre: recursos originais, quadro 640×480 e cenas.
//! Não depende de janela nem de dispositivo de som.

#![forbid(unsafe_code)]

pub mod anim;
pub mod assets;
pub mod balloon;
pub mod clipart;
pub mod frame;
pub mod help;
pub mod help_icons;
pub mod paint;
pub mod paths;
pub mod rng;
pub mod scene;
pub mod scenes;
pub mod text_balloon;
pub mod users;
pub mod writer;

pub use formats::wave::{Pcm, encode_pcm16};
pub use formats::winres::Image;
pub use frame::{Frame, SCREEN_H, SCREEN_W};
