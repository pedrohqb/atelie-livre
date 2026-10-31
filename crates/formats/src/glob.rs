//! Arquivos `.PIC`: índice `GLOB` de imagens.

use crate::{ErrorKind, Format, Reader, Result};

const MAGIC: &[u8; 12] = b"GLOBGLOBGLOB";

#[derive(Debug, Clone)]
pub struct GlobEntry<'a> {
    pub id: u16,
    pub offset: usize,
    pub data: &'a [u8],
}

pub fn is_glob(data: &[u8]) -> bool {
    data.starts_with(MAGIC)
}

/// Lê o índice; o tamanho de cada entrada vai até o próximo offset distinto (ou o fim).
pub fn parse(data: &[u8]) -> Result<Vec<GlobEntry<'_>>> {
    let mut r = Reader::new(data, Format::Glob);
    if r.bytes(12).ok() != Some(MAGIC.as_slice()) {
        return Err(Reader::new(data, Format::Glob).error(ErrorKind::BadMagic {
            expected: "GLOBGLOBGLOB",
        }));
    }
    r.skip(4)?;
    let count = r.u16_le()? as usize;
    let mut table = Vec::with_capacity(count);
    for _ in 0..count {
        let id = r.u16_le()?;
        let at = r.pos();
        let offset = r.u32_le()? as usize;
        table.push((id, offset, at));
    }
    let table_end = r.pos();
    for &(id, offset, at) in &table {
        if offset < table_end || offset >= data.len() {
            let mut e = Reader::new(data, Format::Glob);
            e.seek(at)?;
            return Err(e.error(ErrorKind::Invalid(format!(
                "entry {id}: offset 0x{offset:x} out of range"
            ))));
        }
    }
    let mut starts: Vec<usize> = table.iter().map(|t| t.1).collect();
    starts.sort_unstable();
    starts.dedup();
    let mut entries: Vec<GlobEntry<'_>> = table
        .into_iter()
        .map(|(id, offset, _)| {
            let i = starts.partition_point(|&s| s <= offset);
            let end = starts.get(i).copied().unwrap_or(data.len());
            GlobEntry {
                id,
                offset,
                data: &data[offset..end],
            }
        })
        .collect();
    // A última entrada, no maior offset e só com zeros, é um marcador de fim.
    if let Some(last) = entries.last()
        && Some(&last.offset) == starts.last()
        && last.data.iter().all(|&b| b == 0)
    {
        entries.pop();
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GLOB com entradas `(id, offset)` e tamanho total `len`.
    fn build(entries: &[(u16, u32)], len: usize) -> Vec<u8> {
        let mut v = b"GLOBGLOBGLOB".to_vec();
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        for (id, off) in entries {
            v.extend_from_slice(&id.to_le_bytes());
            v.extend_from_slice(&off.to_le_bytes());
        }
        v.resize(len, 0xee);
        v
    }

    #[test]
    fn entries_sized_by_next_offset_even_out_of_order() {
        let d = build(&[(7, 40), (9, 30)], 50);
        let e = parse(&d).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!((e[0].id, e[0].offset, e[0].data.len()), (7, 40, 10));
        assert_eq!((e[1].id, e[1].offset, e[1].data.len()), (9, 30, 10));
    }

    #[test]
    fn repeated_offsets_share_data() {
        let d = build(&[(1, 40), (2, 40), (3, 50)], 55);
        let e = parse(&d).unwrap();
        assert_eq!(e[0].data.len(), 10);
        assert_eq!(e[1].data.len(), 10);
        assert_eq!(e[2].data.len(), 5);
    }

    #[test]
    fn trailing_zero_entry_is_end_marker() {
        // a última entrada (maior offset) só com zeros marca o fim da anterior
        let mut d = build(&[(1, 30), (0x7d91, 40)], 48);
        d[30..40].fill(0x11);
        d[40..48].fill(0);
        let e = parse(&d).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].data, &[0x11; 10]);
    }

    #[test]
    fn trailing_nonzero_entry_is_kept() {
        let d = build(&[(1, 30), (2, 40)], 48);
        assert_eq!(parse(&d).unwrap().len(), 2);
    }

    #[test]
    fn offset_inside_table_is_invalid() {
        let d = build(&[(1, 20)], 40);
        assert!(parse(&d).is_err());
    }

    #[test]
    fn offset_at_or_past_end_is_invalid() {
        assert!(parse(&build(&[(1, 40)], 40)).is_err());
        assert!(parse(&build(&[(1, 0xffff_ffff)], 40)).is_err());
    }

    #[test]
    fn detects_signature() {
        assert!(is_glob(&build(&[], 18)));
        assert!(!is_glob(b"GLOBGLOB"));
        assert!(parse(b"XLOBGLOBGLOB\x01\x00\x01\x00\x00\x00").is_err());
    }

    #[test]
    fn every_truncation_is_handled() {
        let d = build(&[(7, 40), (9, 30)], 50);
        for n in 0..d.len() {
            let _ = parse(&d[..n]);
        }
    }
}
