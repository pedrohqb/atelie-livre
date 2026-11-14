//! `SHARED/STUFF/WORKSHOP.CHK`: dados da Oficina de Idéias do Creative
//! Writer (ver `docs/formats/workshop.md`).
//!
//! - `STTB n`: tabelas de frases dos rolos da Máquina de Idéias;
//! - `SMDG 0`: a gramática da Máquina (que tabelas e que figuras-rebus cada
//!   uma das três janelas usa, e o formato do texto);
//! - `PWDG 0`: a lista de figuras do Quadro de Figuras;
//! - `PICT n`: as figuras do Max (parte `META`, lida por [`crate::wmf::meta_dib`]).
//!
//! O `SUDG` (só no `~MAXUSER.INF` do usuário) diz qual `STTB` guarda as
//! entradas próprias de cada janela. Todos os números são little-endian.

use crate::{ErrorKind, Format, Reader, Result, chnk, cp1252};

/// Valor "nenhum" das tabelas, figuras e sons.
pub const NONE: u16 = 0xFFFF;
/// Tamanho do campo do formato (Pascal) num registro do `SMDG`.
const FORMAT_FIELD: usize = 62;
/// Tabelas por janela num registro do `SMDG`.
const TABLES_PER_WINDOW: usize = 3;

fn some(v: u16) -> Option<u16> {
    (v != NONE).then_some(v)
}

/// Uma frase de um `STTB` com os bytes extras que vêm depois dela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub text: String,
    pub extra: Vec<u8>,
}

impl Entry {
    /// Som (`SND` do `WRITER.SND`) tocado quando a frase aparece: os dois
    /// bytes extras, se houver e não forem `FF FF` (no `WORKSHOP.CHK` nenhuma
    /// frase tem som).
    pub fn sound(&self) -> Option<u16> {
        match self.extra[..] {
            [lo, hi] => some(u16::from_le_bytes([lo, hi])),
            _ => None,
        }
    }
}

/// Tabela de strings `STTB`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringTable {
    /// Bytes extras depois de cada string (2 no `WORKSHOP.CHK`, 0 no
    /// `~MAXUSER.INF`).
    pub extra_len: u16,
    pub entries: Vec<Entry>,
}

impl StringTable {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn texts(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.text.as_str())
    }
}

/// Lê um `STTB`: `u16 tamanho − 10`, `u16 n`, `i16 −1`, `u16 extras`,
/// `u16 0`, `n × u16 offset` (a partir do byte 10) e as strings Pascal
/// (Windows-1252), cada uma seguida dos bytes extras.
pub fn parse_sttb(data: &[u8]) -> Result<StringTable> {
    let mut r = Reader::new(data, Format::Chnk);
    let _size = r.u16_le()?;
    let n = r.u16_le()? as usize;
    let _minus_one = r.i16_le()?;
    let extra_len = r.u16_le()?;
    let _zero = r.u16_le()?;
    let mut offsets = Vec::with_capacity(n.min(data.len() / 2));
    for _ in 0..n {
        offsets.push(r.u16_le()? as usize);
    }
    let mut entries = Vec::with_capacity(offsets.len());
    for o in offsets {
        r.seek(10 + o)?;
        let len = r.u8()? as usize;
        let text = cp1252::decode(r.bytes(len)?);
        let extra = r.bytes(extra_len as usize)?.to_vec();
        entries.push(Entry { text, extra });
    }
    Ok(StringTable { extra_len, entries })
}

/// Figura-rebus de uma janela da Máquina: o caractere `code` (imagem
/// `2·code − 1` e máscara `2·code` do `WRITER.PIC`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Figure {
    pub code: u8,
    /// Som tocado quando a figura aparece (nenhuma tem no original).
    pub sound: Option<u16>,
}

/// Regra de uma janela da Máquina de Idéias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRule {
    /// Números dos `STTB` cujas frases preenchem os `%s` do formato, em ordem.
    pub tables: Vec<u16>,
    /// Formato do texto, com um `%s` por tabela (`"    %s %s"`, `"%s"`...).
    pub format: String,
    pub figures: Vec<Figure>,
}

/// Gramática `SMDG` da Máquina de Idéias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grammar {
    /// Primeira palavra do chunk (0 no `WORKSHOP.CHK`).
    pub header: u16,
    /// Uma regra por janela (3 no original).
    pub windows: Vec<WindowRule>,
}

/// Lê um `SMDG`: `u16` e, até o fim, registros de 70 bytes (`3 × u16`
/// tabela, `u16` número de figuras `F`, formato Pascal num campo de 62
/// bytes) seguidos de `F × { u16 0x100 | caractere, u16 som }`.
pub fn parse_smdg(data: &[u8]) -> Result<Grammar> {
    let mut r = Reader::new(data, Format::Chnk);
    let header = r.u16_le()?;
    let mut windows = Vec::new();
    while r.remaining() > 0 {
        let mut tables = Vec::with_capacity(TABLES_PER_WINDOW);
        for _ in 0..TABLES_PER_WINDOW {
            if let Some(t) = some(r.u16_le()?) {
                tables.push(t);
            }
        }
        let count = r.u16_le()? as usize;
        let field = r.bytes(FORMAT_FIELD)?;
        let len = field[0] as usize;
        let Some(text) = field.get(1..1 + len) else {
            return Err(r.error(ErrorKind::Invalid(format!("SMDG format of {len} bytes"))));
        };
        let format = cp1252::decode(text);
        let mut figures = Vec::with_capacity(count.min(r.remaining() / 4));
        for _ in 0..count {
            let v = r.u16_le()?;
            let sound = some(r.u16_le()?);
            if v >> 8 != 1 {
                return Err(r.error(ErrorKind::Invalid(format!(
                    "SMDG figure 0x{v:04x} without the 0x100 flag"
                ))));
            }
            figures.push(Figure {
                code: v as u8,
                sound,
            });
        }
        windows.push(WindowRule {
            tables,
            format,
            figures,
        });
    }
    Ok(Grammar { header, windows })
}

/// Lista `PWDG` do Quadro de Figuras.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureList {
    /// Arquivo do usuário com as figuras próprias (`"~MYPICS"`, sem a
    /// extensão `.CHK`).
    pub user_file: String,
    /// Palavra depois do nome (0 no original).
    pub unknown: u16,
    /// Figuras do Max, na ordem da lista: número do `PICT` e som.
    pub pictures: Vec<(u16, Option<u16>)>,
}

/// Lê um `PWDG`: `u16 n`, nome Pascal, `u16 0`, `n × { u16 PICT, u16 som }`.
pub fn parse_pwdg(data: &[u8]) -> Result<PictureList> {
    let mut r = Reader::new(data, Format::Chnk);
    let n = r.u16_le()? as usize;
    let len = r.u8()? as usize;
    let user_file = cp1252::decode(r.bytes(len)?);
    let unknown = r.u16_le()?;
    let mut pictures = Vec::with_capacity(n.min(r.remaining() / 4));
    for _ in 0..n {
        let pict = r.u16_le()?;
        pictures.push((pict, some(r.u16_le()?)));
    }
    Ok(PictureList {
        user_file,
        unknown,
        pictures,
    })
}

/// `SUDG` do `~MAXUSER.INF`: número do `STTB` com as entradas próprias de
/// cada janela (`None` = nenhuma).
pub fn parse_sudg(data: &[u8]) -> Result<Vec<Option<u16>>> {
    let mut r = Reader::new(data, Format::Chnk);
    let mut v = Vec::with_capacity(data.len() / 2);
    while r.remaining() >= 2 {
        v.push(some(r.u16_le()?));
    }
    Ok(v)
}

/// Figura do Max no Quadro de Figuras.
#[derive(Debug, Clone)]
pub struct Picture<'a> {
    pub id: u16,
    /// Título (nome do chunk no índice).
    pub title: String,
    /// Parte `META` (passe a [`crate::wmf::meta_dib`]).
    pub meta: &'a [u8],
}

/// Conteúdo do `WORKSHOP.CHK`.
#[derive(Debug, Clone)]
pub struct Workshop<'a> {
    /// `STTB` por número, na ordem do arquivo.
    pub tables: Vec<(u16, StringTable)>,
    pub grammar: Grammar,
    pub picture_list: PictureList,
    /// `PICT` por número, na ordem do arquivo (não a da lista).
    pub pictures: Vec<Picture<'a>>,
}

impl<'a> Workshop<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Workshop<'a>> {
        let f = chnk::parse(data)?;
        let invalid =
            |what: &str| Reader::new(data, Format::Chnk).error(ErrorKind::Invalid(what.into()));
        if f.byte_order != chnk::ByteOrder::Little {
            return Err(invalid("WORKSHOP big-endian"));
        }
        let mut tables = Vec::new();
        let mut grammar = None;
        let mut picture_list = None;
        let mut pictures = Vec::new();
        for c in &f.chunks {
            let Some(part) = c.parts.first() else {
                continue;
            };
            match &c.tag.0 {
                b"STTB" => tables.push((c.id, parse_sttb(part.data)?)),
                b"SMDG" if grammar.is_none() => grammar = Some(parse_smdg(part.data)?),
                b"PWDG" if picture_list.is_none() => picture_list = Some(parse_pwdg(part.data)?),
                b"PICT" => {
                    if let Some(meta) = c.parts.iter().find(|p| &p.tag.0 == b"META") {
                        pictures.push(Picture {
                            id: c.id,
                            title: c.name.clone().unwrap_or_default(),
                            meta: meta.data,
                        });
                    }
                }
                _ => {}
            }
        }
        let Some(grammar) = grammar else {
            return Err(invalid("WORKSHOP without SMDG"));
        };
        let Some(picture_list) = picture_list else {
            return Err(invalid("WORKSHOP without PWDG"));
        };
        Ok(Workshop {
            tables,
            grammar,
            picture_list,
            pictures,
        })
    }

    pub fn table(&self, id: u16) -> Option<&StringTable> {
        self.tables.iter().find(|(i, _)| *i == id).map(|(_, t)| t)
    }

    pub fn picture(&self, id: u16) -> Option<&Picture<'a>> {
        self.pictures.iter().find(|p| p.id == id)
    }

    /// Figuras do Max na ordem da lista do Quadro (as ausentes são puladas).
    pub fn pictures_in_order(&self) -> Vec<&Picture<'a>> {
        self.picture_list
            .pictures
            .iter()
            .filter_map(|(id, _)| self.picture(*id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `STTB` com as strings dadas e `extra` bytes `FF` depois de cada uma.
    fn sttb(texts: &[&[u8]], extra: usize) -> Vec<u8> {
        let n = texts.len();
        let mut body = Vec::new();
        let mut offs = Vec::new();
        for t in texts {
            offs.push(2 * n + body.len());
            body.push(t.len() as u8);
            body.extend_from_slice(t);
            body.extend(std::iter::repeat_n(0xFF, extra));
        }
        let mut v = Vec::new();
        v.extend_from_slice(&((2 * n + body.len()) as u16).to_le_bytes());
        v.extend_from_slice(&(n as u16).to_le_bytes());
        v.extend_from_slice(&(-1i16).to_le_bytes());
        v.extend_from_slice(&(extra as u16).to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes());
        for o in offs {
            v.extend_from_slice(&(o as u16).to_le_bytes());
        }
        v.extend_from_slice(&body);
        v
    }

    #[test]
    fn sttb_with_extras_and_latin1() {
        let t = parse_sttb(&sttb(&[b"A bai", b"m\xe3e"], 2)).unwrap();
        assert_eq!(t.extra_len, 2);
        assert_eq!(t.texts().collect::<Vec<_>>(), ["A bai", "mãe"]);
        assert_eq!(t.entries[0].extra, [0xFF, 0xFF]);
        assert_eq!(t.entries[0].sound(), None);
    }

    #[test]
    fn sttb_from_user_file() {
        // STTB 0 do ~MAXUSER.INF gravado pelo original (sem extras)
        let mut d = vec![0x10, 0, 1, 0, 0xFF, 0xFF, 0, 0, 0, 0, 2, 0, 14];
        d.extend_from_slice(b"O pirata Zeca,");
        let t = parse_sttb(&d).unwrap();
        assert_eq!(t.texts().collect::<Vec<_>>(), ["O pirata Zeca,"]);
        assert!(t.entries[0].extra.is_empty());
    }

    #[test]
    fn sttb_sound_in_extra_bytes() {
        let mut d = sttb(&[b"x"], 2);
        let n = d.len();
        d[n - 2..].copy_from_slice(&0x021du16.to_le_bytes());
        assert_eq!(parse_sttb(&d).unwrap().entries[0].sound(), Some(0x021d));
    }

    #[test]
    fn sttb_truncated_is_an_error() {
        let d = sttb(&[b"abc"], 2);
        assert!(parse_sttb(&d[..d.len() - 1]).is_err());
        assert!(parse_sttb(&d[..5]).is_err());
    }

    fn smdg_record(tables: [u16; 3], format: &[u8], figs: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        for t in tables {
            v.extend_from_slice(&t.to_le_bytes());
        }
        v.extend_from_slice(&(figs.len() as u16).to_le_bytes());
        let mut field = vec![0u8; FORMAT_FIELD];
        field[0] = format.len() as u8;
        field[1..1 + format.len()].copy_from_slice(format);
        v.extend_from_slice(&field);
        for &c in figs {
            v.extend_from_slice(&(0x100 | c as u16).to_le_bytes());
            v.extend_from_slice(&NONE.to_le_bytes());
        }
        v
    }

    #[test]
    fn smdg_three_windows() {
        let mut d = vec![0, 0];
        d.extend(smdg_record([0, 1, NONE], b"    %s %s", &[3, 0x0b]));
        d.extend(smdg_record([2, NONE, NONE], b"%s", &[0x0a]));
        d.extend(smdg_record([3, 4, NONE], b"%s %s", &[]));
        let g = parse_smdg(&d).unwrap();
        assert_eq!(g.windows.len(), 3);
        assert_eq!(g.windows[0].tables, [0, 1]);
        assert_eq!(g.windows[0].format, "    %s %s");
        assert_eq!(
            g.windows[0].figures,
            [
                Figure {
                    code: 3,
                    sound: None
                },
                Figure {
                    code: 0x0b,
                    sound: None
                }
            ]
        );
        assert_eq!(g.windows[1].tables, [2]);
        assert!(g.windows[2].figures.is_empty());
    }

    #[test]
    fn smdg_rejects_figure_without_flag_and_truncation() {
        let mut d = vec![0, 0];
        d.extend(smdg_record([0, NONE, NONE], b"%s", &[5]));
        let n = d.len();
        d[n - 3] = 0; // 0x0005 em vez de 0x0105
        assert!(parse_smdg(&d).is_err());
        let mut d = vec![0, 0];
        d.extend(smdg_record([0, NONE, NONE], b"%s", &[5]));
        assert!(parse_smdg(&d[..d.len() - 2]).is_err());
    }

    #[test]
    fn pwdg_list() {
        let mut d = vec![2, 0, 7];
        d.extend_from_slice(b"~MYPICS");
        d.extend_from_slice(&[0, 0, 0, 0, 0xFF, 0xFF, 5, 0, 0x2d, 0x02]);
        let l = parse_pwdg(&d).unwrap();
        assert_eq!(l.user_file, "~MYPICS");
        assert_eq!(l.unknown, 0);
        assert_eq!(l.pictures, [(0, None), (5, Some(0x022d))]);
        assert!(parse_pwdg(&d[..d.len() - 1]).is_err());
    }

    #[test]
    fn sudg_windows() {
        assert_eq!(
            parse_sudg(&[0, 0, 0xFF, 0xFF, 2, 0]).unwrap(),
            [Some(0), None, Some(2)]
        );
    }
}
