//! Paletas `LOGPALETTE` do Windows.

use crate::{ErrorKind, Format, Reader, Result};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// `u16_le` versão (0x0300), `u16_le` contagem, contagem × `{r, g, b, flags}`.
pub fn parse_logpalette(data: &[u8]) -> Result<Vec<Rgb>> {
    let mut r = Reader::new(data, Format::Palette);
    let version = r.u16_le()?;
    if version != 0x0300 {
        return Err(
            Reader::new(data, Format::Palette).error(ErrorKind::Invalid(format!(
                "LOGPALETTE version 0x{version:04x}"
            ))),
        );
    }
    let count = r.u16_le()? as usize;
    let entries = r.bytes(count.saturating_mul(4))?;
    Ok(entries
        .chunks_exact(4)
        .map(|c| Rgb {
            r: c[0],
            g: c[1],
            b: c[2],
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;

    #[test]
    fn logpalette_two_entries() {
        let d = [0x00, 0x03, 2, 0, 255, 255, 255, 0, 1, 2, 3, 0, 0xaa];
        assert_eq!(
            parse_logpalette(&d).unwrap(),
            [
                Rgb {
                    r: 255,
                    g: 255,
                    b: 255
                },
                Rgb { r: 1, g: 2, b: 3 }
            ]
        );
    }

    #[test]
    fn logpalette_rejects_wrong_version() {
        let d = [0x00, 0x02, 0, 0];
        assert!(matches!(
            parse_logpalette(&d).unwrap_err().kind,
            ErrorKind::Invalid(_)
        ));
    }

    #[test]
    fn logpalette_count_beyond_data_is_error() {
        let d = [0x00, 0x03, 9, 0, 1, 2, 3, 0];
        assert!(parse_logpalette(&d).is_err());
    }
}
