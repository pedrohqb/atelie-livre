//! Executáveis e DLLs no formato NE (Windows 3.x): módulos importados e recursos.

use crate::{ErrorKind, Format, Reader, Result};

const STANDARD_TYPES: [&str; 16] = [
    "CURSOR",
    "BITMAP",
    "ICON",
    "MENU",
    "DIALOG",
    "STRING",
    "FONTDIR",
    "FONT",
    "ACCELERATOR",
    "RCDATA",
    "MESSAGETABLE",
    "GROUP_CURSOR",
    "",
    "GROUP_ICON",
    "",
    "VERSION",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceKind {
    Numeric(u16),
    Named(String),
}

impl ResourceKind {
    /// Nome usual do tipo: nomes padrão do Windows, `#n` para outros numéricos.
    pub fn label(&self) -> String {
        match self {
            ResourceKind::Numeric(n) => match STANDARD_TYPES.get((*n as usize).wrapping_sub(1)) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => format!("#{n}"),
            },
            ResourceKind::Named(s) => s.clone(),
        }
    }
}

/// Um recurso. `data` inclui o preenchimento de alinhamento do NE.
#[derive(Debug, Clone)]
pub struct Resource<'a> {
    pub kind: ResourceKind,
    /// Id numérico (bit 0x8000 já limpo); 0 quando o id é um nome.
    pub id: u16,
    pub name: Option<String>,
    pub data: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct NeFile<'a> {
    pub modules: Vec<String>,
    pub resources: Vec<Resource<'a>>,
}

impl<'a> NeFile<'a> {
    pub fn of_kind<'s>(&'s self, label: &'s str) -> impl Iterator<Item = &'s Resource<'a>> {
        self.resources
            .iter()
            .filter(move |r| r.kind.label() == label)
    }

    pub fn find(&self, label: &str, id: u16) -> Option<&Resource<'a>> {
        self.resources
            .iter()
            .find(|r| r.name.is_none() && r.id == id && r.kind.label() == label)
    }
}

fn pascal_at(data: &[u8], pos: usize) -> Result<String> {
    let mut r = Reader::new(data, Format::Ne);
    r.seek(pos)?;
    let n = r.u8()? as usize;
    Ok(String::from_utf8_lossy(r.bytes(n)?).into_owned())
}

pub fn parse(data: &[u8]) -> Result<NeFile<'_>> {
    let mut r = Reader::new(data, Format::Ne);
    if r.bytes(2)
        .map_err(|_| r.error(ErrorKind::BadMagic { expected: "MZ" }))?
        != b"MZ"
    {
        return Err(Reader::new(data, Format::Ne).error(ErrorKind::BadMagic { expected: "MZ" }));
    }
    r.seek(0x3c)?;
    let ne = r.u32_le()? as usize;
    r.seek(ne)?;
    if r.bytes(2).ok() != Some(b"NE".as_slice()) {
        return Err(r.error(ErrorKind::BadMagic { expected: "NE" }));
    }
    r.seek(ne + 0x1e)?;
    let mod_count = r.u16_le()? as usize;
    r.seek(ne + 0x24)?;
    let res_tab = ne + r.u16_le()? as usize;
    r.seek(ne + 0x28)?;
    let modref_tab = ne + r.u16_le()? as usize;
    let imp_tab = ne + r.u16_le()? as usize;

    let mut modules = Vec::with_capacity(mod_count.min(256));
    r.seek(modref_tab)?;
    for _ in 0..mod_count {
        let off = r.u16_le()? as usize;
        modules.push(pascal_at(data, imp_tab + off)?);
    }

    let mut resources = Vec::new();
    r.seek(res_tab)?;
    let shift = r.u16_le()?;
    if shift > 15 {
        return Err(r.error(ErrorKind::Invalid(format!("alignment shift {shift}"))));
    }
    loop {
        let type_id = r.u16_le()?;
        if type_id == 0 {
            break;
        }
        let count = r.u16_le()?;
        r.skip(4)?;
        let kind = if type_id & 0x8000 != 0 {
            ResourceKind::Numeric(type_id & 0x7fff)
        } else {
            ResourceKind::Named(pascal_at(data, res_tab + type_id as usize)?)
        };
        for _ in 0..count {
            let entry = r.pos();
            let off = (r.u16_le()? as usize) << shift;
            let len = (r.u16_le()? as usize) << shift;
            r.skip(2)?;
            let raw_id = r.u16_le()?;
            r.skip(4)?;
            let (id, name) = if raw_id & 0x8000 != 0 {
                (raw_id & 0x7fff, None)
            } else {
                (0, Some(pascal_at(data, res_tab + raw_id as usize)?))
            };
            let Some(bytes) = data.get(off..off.saturating_add(len)) else {
                let mut e = r.clone();
                e.seek(entry)?;
                return Err(e.error(ErrorKind::Invalid(format!(
                    "resource {} {id} at 0x{off:x}+0x{len:x} beyond end of file",
                    kind.label()
                ))));
            };
            resources.push(Resource {
                kind: kind.clone(),
                id,
                name,
                data: bytes,
            });
        }
    }
    Ok(NeFile { modules, resources })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ErrorKind;

    fn put16(v: &mut [u8], at: usize, x: u16) {
        v[at..at + 2].copy_from_slice(&x.to_le_bytes());
    }

    /// NE mínimo: módulo KERNEL, ICON id 1 e WHAT (nomeado) id 0x201, dados "ABCD", shift 4.
    pub(crate) fn tiny_ne() -> Vec<u8> {
        tiny_ne_with(&[
            (0x8003, None, 0x8001, b"ABCD"),
            (0, Some("WHAT"), 0x8201, b"EFGH"),
        ])
    }

    /// Monta um NE com recursos `(type_id numérico, nome do tipo, id, dados)`.
    pub(crate) fn tiny_ne_with(res: &[(u16, Option<&str>, u16, &[u8])]) -> Vec<u8> {
        let ne = 0x40;
        let mut v = vec![0u8; ne + 0x40];
        v[0..2].copy_from_slice(b"MZ");
        v[0x3c..0x40].copy_from_slice(&(ne as u32).to_le_bytes());
        v[ne..ne + 2].copy_from_slice(b"NE");
        put16(&mut v, ne + 0x1e, 1);
        // tabela de recursos
        let rt = v.len();
        put16(&mut v, ne + 0x24, (rt - ne) as u16);
        v.extend_from_slice(&4u16.to_le_bytes());
        let mut name_fixups = Vec::new();
        let mut data_fixups = Vec::new();
        for (tid, tname, id, _) in res {
            let tpos = v.len();
            v.extend_from_slice(&tid.to_le_bytes());
            v.extend_from_slice(&1u16.to_le_bytes());
            v.extend_from_slice(&[0; 4]);
            if let Some(n) = tname {
                name_fixups.push((tpos, *n));
            }
            data_fixups.push(v.len());
            v.extend_from_slice(&[0; 4]); // offset, length
            v.extend_from_slice(&0u16.to_le_bytes());
            v.extend_from_slice(&id.to_le_bytes());
            v.extend_from_slice(&[0; 4]);
        }
        v.extend_from_slice(&0u16.to_le_bytes());
        for (tpos, n) in name_fixups {
            let off = (v.len() - rt) as u16;
            put16(&mut v, tpos, off);
            v.push(n.len() as u8);
            v.extend_from_slice(n.as_bytes());
        }
        // tabela de módulos e nomes importados
        let modref = v.len();
        put16(&mut v, ne + 0x28, (modref - ne) as u16);
        v.extend_from_slice(&1u16.to_le_bytes());
        let imp = v.len();
        put16(&mut v, ne + 0x2a, (imp - ne) as u16);
        v.push(0);
        v.push(6);
        v.extend_from_slice(b"KERNEL");
        // dados alinhados em 16
        for (fix, (_, _, _, data)) in data_fixups.into_iter().zip(res) {
            while v.len() % 16 != 0 {
                v.push(0);
            }
            let pos = (v.len() >> 4) as u16;
            put16(&mut v, fix, pos);
            put16(&mut v, fix + 2, data.len().div_ceil(16) as u16);
            v.extend_from_slice(data);
            while v.len() % 16 != 0 {
                v.push(0xaa);
            }
        }
        v
    }

    #[test]
    fn parses_numeric_and_named_types() {
        let d = tiny_ne();
        let ne = parse(&d).unwrap();
        assert_eq!(ne.modules, ["KERNEL"]);
        let icon = ne.find("ICON", 1).unwrap();
        assert_eq!(icon.kind, ResourceKind::Numeric(3));
        assert_eq!(icon.data[..4], *b"ABCD");
        assert_eq!(icon.data.len(), 16);
        let what = ne.find("WHAT", 0x201).unwrap();
        assert_eq!(what.kind, ResourceKind::Named("WHAT".into()));
        assert_eq!(what.name, None);
        assert_eq!(ne.of_kind("WHAT").count(), 1);
    }

    #[test]
    fn labels_for_standard_types() {
        assert_eq!(ResourceKind::Numeric(1).label(), "CURSOR");
        assert_eq!(ResourceKind::Numeric(14).label(), "GROUP_ICON");
        assert_eq!(ResourceKind::Numeric(13).label(), "#13");
        assert_eq!(ResourceKind::Numeric(99).label(), "#99");
        assert_eq!(ResourceKind::Named("STR#".into()).label(), "STR#");
    }

    #[test]
    fn rejects_non_ne() {
        let mut d = vec![0u8; 0x80];
        d[0..2].copy_from_slice(b"MZ");
        d[0x3c] = 0x40;
        let e = parse(&d).unwrap_err();
        assert_eq!(e.kind, ErrorKind::BadMagic { expected: "NE" });
        assert!(parse(b"hello").is_err());
    }

    #[test]
    fn truncated_resource_is_error_not_panic() {
        let d = tiny_ne();
        let cut = d.len() - 20;
        assert!(parse(&d[..cut]).is_err());
    }

    #[test]
    fn every_truncation_is_handled() {
        let d = tiny_ne();
        for n in 0..d.len() {
            let _ = parse(&d[..n]);
        }
    }

    #[test]
    fn every_byte_corruption_is_handled() {
        let d = tiny_ne();
        for i in 0..d.len() {
            for x in [0x00, 0xff, 0x80] {
                let mut c = d.clone();
                c[i] = x;
                let _ = parse(&c);
            }
        }
    }
}
