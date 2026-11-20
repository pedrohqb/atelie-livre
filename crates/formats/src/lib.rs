//! Leitores dos formatos de dados dos programas originais.
//!
//! Todos os leitores trabalham sobre fatias de bytes já carregadas e
//! devolvem [`FormatError`] em vez de entrar em pânico com dados ruins.

#![forbid(unsafe_code)]

pub mod atag;
pub mod bor;
pub mod chnk;
pub mod cp1252;
pub mod dct;
pub mod error;
pub mod fnt;
pub mod gif;
pub mod glob;
pub mod macpaint;
pub mod macres;
pub mod mag;
pub mod max;
pub mod maxprefs;
pub mod ne;
pub mod palette;
pub mod pict;
pub mod pkbm;
pub mod reader;
pub mod render;
pub mod scrd;
pub mod sple;
pub mod splot_res;
pub mod tag;
pub mod twpb;
pub mod wave;
pub mod what;
pub mod winres;
pub mod winres_ui;
pub mod wmf;
pub mod wmf_draw;
pub mod workshop;

pub use error::{ErrorKind, Format, FormatError, Result};
pub use palette::Rgb;
pub use reader::Reader;
pub use tag::Tag;
