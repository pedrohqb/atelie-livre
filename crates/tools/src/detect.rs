//! Identificação de arquivos pela assinatura, nunca pela extensão.

use formats::{chnk, glob, wave};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Ne,
    Glob,
    Chnk,
    RiffWave,
    Unknown,
}

pub fn detect(data: &[u8]) -> FileKind {
    if is_ne(data) {
        FileKind::Ne
    } else if glob::is_glob(data) {
        FileKind::Glob
    } else if chnk::is_chnk(data) {
        FileKind::Chnk
    } else if wave::is_riff_wave(data) {
        FileKind::RiffWave
    } else {
        FileKind::Unknown
    }
}

fn is_ne(data: &[u8]) -> bool {
    if !data.starts_with(b"MZ") || data.len() < 0x40 {
        return false;
    }
    let off = u32::from_le_bytes([data[0x3c], data[0x3d], data[0x3e], data[0x3f]]) as usize;
    data.get(off..off.saturating_add(2)) == Some(b"NE".as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_by_signature() {
        let mut ne = vec![0u8; 0x48];
        ne[0..2].copy_from_slice(b"MZ");
        ne[0x3c] = 0x40;
        ne[0x40..0x42].copy_from_slice(b"NE");
        assert_eq!(detect(&ne), FileKind::Ne);
        assert_eq!(detect(b"GLOBGLOBGLOB\x01\x00"), FileKind::Glob);
        assert_eq!(detect(b"CHNKSPLT\x04\x00\x04\x00"), FileKind::Chnk);
        assert_eq!(detect(b"RIFF\x00\x00\x00\x00WAVEfmt "), FileKind::RiffWave);
        assert_eq!(detect(b"MZ just a dos program"), FileKind::Unknown);
        assert_eq!(detect(b"hello"), FileKind::Unknown);
        assert_eq!(detect(b""), FileKind::Unknown);
    }
}
