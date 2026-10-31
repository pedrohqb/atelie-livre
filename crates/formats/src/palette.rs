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

/// Chunk `CTBL` dos arquivos CHNK: `u16` marca de ordem (1), `u16`, 4 bytes,
/// `u16` contagem, contagem × { u16 r, u16 g, u16 b } (cor = byte alto).
pub fn parse_ctbl(data: &[u8]) -> Result<Vec<Rgb>> {
    let mut r = Reader::new(data, Format::Palette);
    let big = match r.bytes(2)? {
        [0, 1] => true,
        [1, 0] => false,
        _ => {
            return Err(
                Reader::new(data, Format::Palette).error(ErrorKind::BadMagic {
                    expected: "CTBL byte order mark",
                }),
            );
        }
    };
    let rd16 = |r: &mut Reader| if big { r.u16_be() } else { r.u16_le() };
    r.seek(8)?;
    let count = rd16(&mut r)? as usize;
    if count.saturating_mul(6) > r.remaining() {
        return Err(r.error(ErrorKind::UnexpectedEof {
            wanted: count * 6,
            available: r.remaining(),
        }));
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let (cr, cg, cb) = (rd16(&mut r)?, rd16(&mut r)?, rd16(&mut r)?);
        out.push(Rgb {
            r: (cr >> 8) as u8,
            g: (cg >> 8) as u8,
            b: (cb >> 8) as u8,
        });
    }
    Ok(out)
}

/// Índice da cor mais próxima (distância euclidiana em RGB) entre as 256 primeiras.
pub fn nearest(pal: &[Rgb], r: u8, g: u8, b: u8) -> u8 {
    let d = |p: &Rgb| {
        let dr = p.r as i32 - r as i32;
        let dg = p.g as i32 - g as i32;
        let db = p.b as i32 - b as i32;
        dr * dr + dg * dg + db * db
    };
    pal.iter()
        .take(256)
        .enumerate()
        .min_by_key(|(_, p)| d(p))
        .map(|(i, _)| i as u8)
        .unwrap_or(0)
}

/// Grava um chunk `CTBL` little-endian com a paleta: marca de ordem (1), 7,
/// a etiqueta `COBJ` (é a que os desenhos salvos trazem), contagem e
/// contagem × { u16 r, u16 g, u16 b } com a cor no byte alto.
pub fn write_ctbl(pal: &[Rgb]) -> Vec<u8> {
    let mut v = Vec::with_capacity(10 + 6 * pal.len());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&7u16.to_le_bytes());
    v.extend_from_slice(b"COBJ");
    v.extend_from_slice(&(pal.len() as u16).to_le_bytes());
    for c in pal {
        v.extend_from_slice(&((c.r as u16) << 8).to_le_bytes());
        v.extend_from_slice(&((c.g as u16) << 8).to_le_bytes());
        v.extend_from_slice(&((c.b as u16) << 8).to_le_bytes());
    }
    v
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

    fn ctbl(big: bool, colors: &[(u16, u16, u16)]) -> Vec<u8> {
        let p16 = |v: u16| {
            if big {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let mut v = Vec::new();
        v.extend_from_slice(&p16(1));
        v.extend_from_slice(&p16(6));
        v.extend_from_slice(b"CTBL");
        v.extend_from_slice(&p16(colors.len() as u16));
        for &(r, g, b) in colors {
            v.extend_from_slice(&p16(r));
            v.extend_from_slice(&p16(g));
            v.extend_from_slice(&p16(b));
        }
        v
    }

    #[test]
    fn ctbl_both_byte_orders() {
        let colors = [(0xff00, 0x8000, 0x0010), (0, 0xffff, 0x4000)];
        let want = [
            Rgb {
                r: 255,
                g: 128,
                b: 0,
            },
            Rgb {
                r: 0,
                g: 255,
                b: 64,
            },
        ];
        assert_eq!(parse_ctbl(&ctbl(false, &colors)).unwrap(), want);
        assert_eq!(parse_ctbl(&ctbl(true, &colors)).unwrap(), want);
    }

    #[test]
    fn ctbl_rejects_bad_mark_and_short_data() {
        let mut d = ctbl(false, &[(1, 2, 3)]);
        d[0] = 7;
        assert!(parse_ctbl(&d).is_err());
        let d = ctbl(false, &[(1, 2, 3)]);
        assert!(parse_ctbl(&d[..d.len() - 1]).is_err());
    }

    #[test]
    fn nearest_picks_closest_among_first_256() {
        let pal = [
            Rgb { r: 0, g: 0, b: 0 },
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            Rgb { r: 200, g: 0, b: 0 },
        ];
        assert_eq!(nearest(&pal, 10, 10, 10), 0);
        assert_eq!(nearest(&pal, 250, 250, 250), 1);
        assert_eq!(nearest(&pal, 180, 20, 0), 2);
        assert_eq!(nearest(&[], 1, 2, 3), 0);
        let mut big = vec![Rgb { r: 0, g: 0, b: 0 }; 256];
        big.push(Rgb {
            r: 255,
            g: 255,
            b: 255,
        });
        assert_eq!(nearest(&big, 255, 255, 255), 0);
    }

    #[test]
    fn write_ctbl_roundtrips() {
        let pal: Vec<Rgb> = (0..=255u8)
            .map(|i| Rgb {
                r: i,
                g: 255 - i,
                b: i ^ 0x55,
            })
            .collect();
        let d = write_ctbl(&pal);
        assert_eq!(d.len(), 1546);
        assert_eq!(&d[..10], b"\x01\x00\x07\x00COBJ\x00\x01");
        assert_eq!(parse_ctbl(&d).unwrap(), pal);
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
