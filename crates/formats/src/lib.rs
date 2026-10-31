//! Leitores dos formatos de dados dos programas originais.
//!
//! Todos os leitores trabalham sobre fatias de bytes já carregadas e
//! devolvem [`FormatError`] em vez de entrar em pânico com dados ruins.

#![forbid(unsafe_code)]

pub mod cp1252;
pub mod error;
pub mod macres;
pub mod ne;
pub mod palette;
pub mod reader;
pub mod tag;

pub use error::{ErrorKind, Format, FormatError, Result};
pub use palette::Rgb;
pub use reader::Reader;
pub use tag::Tag;
