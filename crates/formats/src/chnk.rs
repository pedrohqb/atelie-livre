//! Arquivos `CHNK`: documentos e bibliotecas (ver `docs/formats/chnk.md`).

use crate::{ErrorKind, Format, Reader, Result, Tag, cp1252};

/// Flag de registro: a parte é um arquivo `CHNK` aninhado.
pub const FLAG_NESTED: u16 = 4;
const NO_SEGMENT: u32 = u32::MAX;
const SUPPORTED_VERSION: u16 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    Little,
    Big,
}

#[derive(Debug, Clone)]
pub struct Part<'a> {
    pub tag: Tag,
    pub offset: usize,
    pub data: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct Chunk<'a> {
    pub tag: Tag,
    pub id: u16,
    pub flags: u16,
    pub name: Option<String>,
    pub parts: Vec<Part<'a>>,
}

#[derive(Debug, Clone)]
pub struct ChunkFile<'a> {
    pub creator: Tag,
    pub version: (u16, u16),
    pub byte_order: ByteOrder,
    pub chunks: Vec<Chunk<'a>>,
}

pub fn is_chnk(data: &[u8]) -> bool {
    data.starts_with(b"CHNK")
}

struct Ordered<'a> {
    r: Reader<'a>,
    big: bool,
}

impl Ordered<'_> {
    fn u16(&mut self) -> Result<u16> {
        if self.big {
            self.r.u16_be()
        } else {
            self.r.u16_le()
        }
    }
    fn u32(&mut self) -> Result<u32> {
        if self.big {
            self.r.u32_be()
        } else {
            self.r.u32_le()
        }
    }
}

pub fn parse(data: &[u8]) -> Result<ChunkFile<'_>> {
    if !is_chnk(data) {
        return Err(Reader::new(data, Format::Chnk).error(ErrorKind::BadMagic { expected: "CHNK" }));
    }
    let mut r = Reader::new(data, Format::Chnk);
    r.seek(4)?;
    let creator = r.tag()?;
    let order = r.bytes(2)?;
    let big = order[0] == 0 && order[1] != 0;
    let mut o = Ordered { r, big };
    o.r.seek(8)?;
    let version = (o.u16()?, o.u16()?);
    if version.1 > SUPPORTED_VERSION {
        return Err(o
            .r
            .error(ErrorKind::Unsupported(format!("CHNK version {version:?}"))));
    }
    let count = o.u16()? as usize;

    let mut chunks = Vec::with_capacity(count.min(data.len() / 12));
    let mut visited = Vec::new();
    let mut segment = 24u32;
    while segment != NO_SEGMENT {
        if visited.contains(&segment) {
            return Err(o.r.error(ErrorKind::Invalid(format!(
                "index segment loop at 0x{segment:x}"
            ))));
        }
        visited.push(segment);
        o.r.seek(segment as usize)?;
        let _size = o.u16()?;
        let records = o.u16()? as usize;
        let next = o.u32()?;
        for _ in 0..records {
            if chunks.len() == count {
                return Err(o.r.error(ErrorKind::Invalid(
                    "more index records than declared".into(),
                )));
            }
            chunks.push(parse_record(&mut o, data)?);
        }
        segment = next;
    }
    if chunks.len() != count {
        return Err(o.r.error(ErrorKind::Invalid(format!(
            "index has {} records, header declares {count}",
            chunks.len()
        ))));
    }
    let byte_order = if big {
        ByteOrder::Big
    } else {
        ByteOrder::Little
    };
    Ok(ChunkFile {
        creator,
        version,
        byte_order,
        chunks,
    })
}

fn parse_record<'a>(o: &mut Ordered<'_>, data: &'a [u8]) -> Result<Chunk<'a>> {
    let start = o.r.pos();
    let len = o.u16()? as usize;
    let tag = o.r.tag()?;
    let id = o.u16()?;
    let part_count = o.u16()? as usize;
    let flags = o.u16()?;
    if len < 12 + 12 * part_count {
        return Err(o.r.error(ErrorKind::Invalid(format!(
            "record of {len} bytes with {part_count} parts"
        ))));
    }
    let mut parts = Vec::with_capacity(part_count);
    for _ in 0..part_count {
        let at = o.r.pos();
        let ptag = o.r.tag()?;
        let offset = o.u32()? as usize;
        let size = o.u32()? as usize;
        let Some(pdata) = data.get(offset..offset.saturating_add(size)) else {
            let mut e = o.r.clone();
            e.seek(at)?;
            return Err(e.error(ErrorKind::Invalid(format!(
                "part {ptag} of {tag} {id} at 0x{offset:x}+0x{size:x} beyond end of file"
            ))));
        };
        parts.push(Part {
            tag: ptag,
            offset,
            data: pdata,
        });
    }
    let end = start + len;
    let name = if o.r.pos() < end {
        let n = o.r.u8()? as usize;
        if o.r.pos() + n > end {
            return Err(o
                .r
                .error(ErrorKind::Invalid("chunk name beyond record".into())));
        }
        Some(cp1252::decode(o.r.bytes(n)?))
    } else {
        None
    };
    o.r.seek(end)?;
    Ok(Chunk {
        tag,
        id,
        flags,
        name,
        parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorKind, Tag};

    struct Rec<'a> {
        tag: &'a [u8; 4],
        id: u16,
        flags: u16,
        parts: Vec<(&'a [u8; 4], Vec<u8>)>,
        name: Option<&'a [u8]>,
    }

    fn rec<'a>(tag: &'a [u8; 4], id: u16, parts: Vec<(&'a [u8; 4], Vec<u8>)>) -> Rec<'a> {
        Rec {
            tag,
            id,
            flags: 0,
            parts,
            name: None,
        }
    }

    /// Monta um CHNK; os primeiros `first` registros ficam no trecho inicial, o resto num segundo trecho.
    fn build(big: bool, recs: &[Rec], first: usize) -> Vec<u8> {
        let p16 = |v: u16| {
            if big {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let p32 = |v: u32| {
            if big {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let mut file = vec![0u8; 0x200];
        // dados
        let mut offsets = Vec::new();
        for r in recs {
            let mut o = Vec::new();
            for (_, data) in &r.parts {
                o.push(file.len() as u32);
                file.extend_from_slice(data);
            }
            offsets.push(o);
        }
        let encode = |r: &Rec, o: &[u32]| {
            let mut v = Vec::new();
            v.extend_from_slice(&[0, 0]);
            v.extend_from_slice(r.tag);
            v.extend_from_slice(&p16(r.id));
            v.extend_from_slice(&p16(r.parts.len() as u16));
            v.extend_from_slice(&p16(r.flags));
            for ((tag, data), off) in r.parts.iter().zip(o) {
                v.extend_from_slice(*tag);
                v.extend_from_slice(&p32(*off));
                v.extend_from_slice(&p32(data.len() as u32));
            }
            if let Some(n) = r.name {
                v.push(n.len() as u8);
                v.extend_from_slice(n);
                if v.len() % 2 == 1 {
                    v.push(0);
                }
            }
            let len = p16(v.len() as u16);
            v[0..2].copy_from_slice(&len);
            v
        };
        let encoded: Vec<Vec<u8>> = recs
            .iter()
            .zip(&offsets)
            .map(|(r, o)| encode(r, o))
            .collect();
        let total: usize = encoded.iter().map(Vec::len).sum();
        let seg2 = if first < recs.len() {
            file.len() as u32
        } else {
            u32::MAX
        };
        if first < recs.len() {
            let body: Vec<u8> = encoded[first..].concat();
            file.extend_from_slice(&p16(8 + body.len() as u16));
            file.extend_from_slice(&p16((recs.len() - first) as u16));
            file.extend_from_slice(&p32(u32::MAX));
            file.extend_from_slice(&body);
        }
        let mut h = Vec::new();
        h.extend_from_slice(b"CHNK");
        h.extend_from_slice(b"SPLT");
        h.extend_from_slice(&p16(4));
        h.extend_from_slice(&p16(4));
        h.extend_from_slice(&p16(recs.len() as u16));
        h.extend_from_slice(&p16(0x0300));
        h.extend_from_slice(&p32(total as u32));
        h.extend_from_slice(&p32(file.len() as u32));
        h.extend_from_slice(&p16(0x200));
        h.extend_from_slice(&p16(first.min(recs.len()) as u16));
        h.extend_from_slice(&p32(seg2));
        let first_body: Vec<u8> = encoded[..first.min(recs.len())].concat();
        assert!(32 + first_body.len() <= 0x200);
        file[..32].copy_from_slice(&h);
        file[32..32 + first_body.len()].copy_from_slice(&first_body);
        file
    }

    fn sample<'a>() -> Vec<Rec<'a>> {
        vec![
            Rec {
                tag: b"BDR ",
                id: 6,
                flags: 0,
                parts: vec![(b"WBDR", vec![1, 2, 3]), (b"PKBM", vec![9; 5])],
                name: Some(b"Bola\xe7o"),
            },
            rec(b"CTBL", 0, vec![(b"CTBL", vec![7; 4])]),
            Rec {
                tag: b"ATAG",
                id: 1,
                flags: FLAG_NESTED,
                parts: vec![(b"ATAG", vec![0; 2])],
                name: None,
            },
        ]
    }

    fn check_sample(f: &ChunkFile<'_>) {
        assert_eq!(f.creator, Tag(*b"SPLT"));
        assert_eq!(f.version, (4, 4));
        assert_eq!(f.chunks.len(), 3);
        let c = &f.chunks[0];
        assert_eq!((c.tag, c.id, c.flags), (Tag(*b"BDR "), 6, 0));
        assert_eq!(c.name.as_deref(), Some("Bolaço"));
        assert_eq!(c.parts.len(), 2);
        assert_eq!(c.parts[0].tag, Tag(*b"WBDR"));
        assert_eq!(c.parts[0].data, &[1, 2, 3]);
        assert_eq!(c.parts[1].offset, 0x203);
        assert_eq!(c.parts[1].data, &[9; 5]);
        assert_eq!(f.chunks[1].name, None);
        assert_eq!(f.chunks[2].flags, FLAG_NESTED);
    }

    #[test]
    fn parses_synthetic_little_endian() {
        let d = build(false, &sample(), 3);
        let f = parse(&d).unwrap();
        assert_eq!(f.byte_order, ByteOrder::Little);
        check_sample(&f);
    }

    #[test]
    fn parses_synthetic_big_endian() {
        let d = build(true, &sample(), 3);
        let f = parse(&d).unwrap();
        assert_eq!(f.byte_order, ByteOrder::Big);
        check_sample(&f);
    }

    #[test]
    fn follows_second_index_segment() {
        let d = build(false, &sample(), 1);
        check_sample(&parse(&d).unwrap());
    }

    #[test]
    fn part_outside_file_is_invalid() {
        let mut d = build(false, &sample(), 3);
        let len = d.len();
        d.truncate(len - 1);
        assert!(matches!(parse(&d).unwrap_err().kind, ErrorKind::Invalid(_)));
    }

    #[test]
    fn record_count_mismatch_is_invalid() {
        let mut d = build(false, &sample(), 3);
        d[12] = 4;
        assert!(parse(&d).is_err());
    }

    #[test]
    fn segment_loop_is_detected() {
        let mut d = build(false, &sample(), 1);
        let seg2 = u32::from_le_bytes(d[28..32].try_into().unwrap()) as usize;
        d[seg2 + 4..seg2 + 8].copy_from_slice(&(seg2 as u32).to_le_bytes());
        d[12] = 200; // pede mais registros do que existem, forçando seguir o laço
        assert!(parse(&d).is_err());
    }

    #[test]
    fn rejects_bad_signature_and_version() {
        let mut d = build(false, &sample(), 3);
        d[0] = b'X';
        assert_eq!(
            parse(&d).unwrap_err().kind,
            ErrorKind::BadMagic { expected: "CHNK" }
        );
        let mut d = build(false, &sample(), 3);
        d[8..12].copy_from_slice(&[9, 0, 9, 0]);
        assert!(parse(&d).is_err());
        assert!(is_chnk(b"CHNKxxxx"));
        assert!(!is_chnk(b"CHN2"));
    }

    #[test]
    fn every_truncation_and_corruption_is_handled() {
        let d = build(false, &sample(), 1);
        for n in 0..d.len() {
            let _ = parse(&d[..n]);
        }
        for i in 0..d.len() {
            for x in [0x00, 0xff, 0x7f, 0x80] {
                let mut c = d.clone();
                c[i] = x;
                let _ = parse(&c);
            }
        }
    }
}
