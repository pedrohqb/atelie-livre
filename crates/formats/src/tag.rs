use std::fmt;

/// Etiqueta de quatro bytes, na ordem em que aparece no arquivo.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Tag(pub [u8; 4]);

impl Tag {
    /// Monta uma etiqueta a partir de texto, completando com espaços ou cortando em 4 bytes.
    pub fn from_str_lossy(s: &str) -> Tag {
        let mut b = [b' '; 4];
        for (dst, src) in b.iter_mut().zip(s.bytes()) {
            *dst = src;
        }
        Tag(b)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &b in &self.0 {
            if (0x20..0x7f).contains(&b) && b != b'\\' {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{b:02x}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_display_escapes_nonprintable() {
        assert_eq!(Tag(*b"STR#").to_string(), "STR#");
        assert_eq!(Tag([b'A', 0, b'B', 0xff]).to_string(), "A\\x00B\\xff");
    }

    #[test]
    fn from_str_lossy_pads_and_truncates() {
        assert_eq!(Tag::from_str_lossy("INK"), Tag(*b"INK "));
        assert_eq!(Tag::from_str_lossy("WHATEVER"), Tag(*b"WHAT"));
    }
}
