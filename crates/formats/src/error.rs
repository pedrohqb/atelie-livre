use std::fmt;

/// Formato que estava sendo lido quando o erro ocorreu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Ne,
    Palette,
    MacRes,
    WinRes,
    Glob,
    Pkbm,
    Wave,
    Chnk,
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Format::Ne => "ne",
            Format::Palette => "palette",
            Format::MacRes => "macres",
            Format::WinRes => "winres",
            Format::Glob => "glob",
            Format::Pkbm => "pkbm",
            Format::Wave => "wave",
            Format::Chnk => "chnk",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorKind {
    #[error("unexpected end of data: wanted {wanted} bytes, {available} available")]
    UnexpectedEof { wanted: usize, available: usize },
    #[error("bad signature, expected {expected}")]
    BadMagic { expected: &'static str },
    #[error("invalid data: {0}")]
    Invalid(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{format} @0x{offset:x}: {kind}")]
pub struct FormatError {
    pub format: Format,
    pub offset: usize,
    pub kind: ErrorKind,
}

pub type Result<T> = std::result::Result<T, FormatError>;
