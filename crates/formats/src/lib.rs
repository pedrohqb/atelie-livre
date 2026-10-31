//! Leitores dos formatos de dados dos programas originais.
//!
//! Todos os leitores trabalham sobre fatias de bytes já carregadas e
//! devolvem [`FormatError`] em vez de entrar em pânico com dados ruins.

#![forbid(unsafe_code)]

pub mod atag;
pub mod chnk;
pub mod cp1252;
pub mod error;
pub mod fnt;
pub mod glob;
pub mod macres;
pub mod mag;
pub mod ne;
pub mod palette;
pub mod pkbm;
pub mod reader;
pub mod render;
pub mod tag;
pub mod wave;
pub mod what;
pub mod winres;
pub mod wmf;

pub use error::{ErrorKind, Format, FormatError, Result};
pub use palette::Rgb;
pub use reader::Reader;
pub use tag::Tag;
