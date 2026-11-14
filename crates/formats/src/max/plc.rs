//! Tabelas PLC (partes `PLC ` de `BTEP`, `BTEC`, `PGD ` e `SNDT`):
//!
//! ```text
//! u16 n, u16 cb (tamanho do elemento), u16 extra
//! (n+1) × u32 posições, n × cb elementos
//! ```

use super::err;
use crate::{ErrorKind, Result};

/// Valor do campo `extra` nas tabelas do original.
pub const EXTRA_BTE: u16 = 0;
pub const EXTRA_PGD: u16 = 0x00ff;
pub const EXTRA_SNDT: u16 = 0xff00;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plc<'a> {
    pub extra: u16,
    /// `n + 1` posições.
    pub pos: Vec<u32>,
    /// `n` elementos de `cb` bytes.
    pub elems: Vec<&'a [u8]>,
}

/// Lê uma tabela cujos elementos devem ter `cb` bytes; `base` é o offset da
/// parte no arquivo (para os erros).
pub fn read(b: &[u8], cb: usize, base: usize) -> Result<Plc<'_>> {
    let bad = |at: usize, m: String| err(base + at, ErrorKind::Invalid(m));
    if b.len() < 6 {
        return Err(bad(0, "PLC too short".into()));
    }
    let n = u16::from_le_bytes([b[0], b[1]]) as usize;
    let got_cb = u16::from_le_bytes([b[2], b[3]]) as usize;
    let extra = u16::from_le_bytes([b[4], b[5]]);
    if got_cb != cb {
        return Err(bad(
            2,
            format!("PLC element of {got_cb} bytes, expected {cb}"),
        ));
    }
    let need = 6 + 4 * (n + 1) + cb * n;
    if b.len() < need {
        return Err(bad(0, format!("PLC of {n} elements needs {need} bytes")));
    }
    let pos = (0..=n)
        .map(|k| {
            let at = 6 + 4 * k;
            u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
        })
        .collect();
    let o = 6 + 4 * (n + 1);
    let elems = (0..n).map(|k| &b[o + cb * k..o + cb * (k + 1)]).collect();
    Ok(Plc { extra, pos, elems })
}

/// Grava uma tabela; `pos` deve ter um item a mais que `elems`.
pub fn write(extra: u16, cb: u16, pos: &[u32], elems: &[Vec<u8>]) -> Vec<u8> {
    debug_assert_eq!(pos.len(), elems.len() + 1);
    let mut v = Vec::with_capacity(6 + 4 * pos.len() + cb as usize * elems.len());
    v.extend_from_slice(&(elems.len() as u16).to_le_bytes());
    v.extend_from_slice(&cb.to_le_bytes());
    v.extend_from_slice(&extra.to_le_bytes());
    for p in pos {
        v.extend_from_slice(&p.to_le_bytes());
    }
    for e in elems {
        let mut e = e.clone();
        e.resize(cb as usize, 0);
        v.extend_from_slice(&e);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teste1_btep() {
        let b = [1, 0, 4, 0, 0, 0, 0, 0, 0, 0, 5, 2, 0, 0, 0, 4, 0, 0];
        let p = read(&b, 4, 0).unwrap();
        assert_eq!(p.pos, [0, 0x205]);
        assert_eq!(p.elems, [&[0, 4, 0, 0][..]]);
        assert_eq!(write(EXTRA_BTE, 4, &p.pos, &[vec![0, 4, 0, 0]]), b);
    }

    #[test]
    fn s3_pgd() {
        let b = [
            2, 0, 2, 0, 0xff, 0, 0, 0, 0, 0, 0x2a, 0, 0, 0, 0x56, 0, 0, 0, 0, 0, 0, 0,
        ];
        let p = read(&b, 2, 0).unwrap();
        assert_eq!(p.extra, EXTRA_PGD);
        assert_eq!(p.pos, [0, 42, 86]);
        assert_eq!(write(EXTRA_PGD, 2, &p.pos, &[vec![0, 0], vec![0, 0]]), b);
    }

    #[test]
    fn bad_tables_are_errors() {
        assert!(read(&[1, 0, 4], 4, 0).is_err());
        assert!(read(&[1, 0, 2, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0], 4, 0).is_err());
        assert!(read(&[9, 0, 4, 0, 0, 0, 0, 0, 0, 0], 4, 0).is_err());
        let p = read(&[0, 0, 4, 0, 0, 0, 7, 0, 0, 0], 4, 0).unwrap();
        assert_eq!((p.pos.as_slice(), p.elems.len()), (&[7][..], 0));
    }
}
