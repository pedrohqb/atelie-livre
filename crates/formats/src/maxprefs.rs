//! Preferências do usuário no Creative Writer (`USERS/<nome>/~MAXUSER.INF`;
//! ver `docs/formats/maxprefs.md`).
//!
//! É um arquivo `CHNK` com criador `INK ` e estes chunks:
//!
//! - `FILR` 0–4: os documentos pendurados na parede da Biblioteca;
//! - `FNTL` 0: cache com os nomes das fontes do sistema;
//! - `STTB` n e `SUDG` 0: entradas que o usuário pôs na Máquina de Idéias.
//!
//! O `~MAGUSER.INF` do Fine Artist tem o mesmo leiaute (registros `kwMAG`).
//! Arquivos gravados por uma sessão interrompida podem vir **sem o índice**
//! (os primeiros 512 bytes zerados, registros e fontes em sequência a partir
//! de 0x200); [`read`] aceita os dois.

use crate::chnk::{self, OwnedChunk, OwnedPart};
use crate::{ErrorKind, Format, Reader, Result, Tag, cp1252};

/// Criador do `~MAXUSER.INF` (o mesmo dos documentos `.MAX`).
pub const CREATOR: Tag = Tag(*b"INK ");
/// Tipo dos registros da parede no Writer.
pub const KIND_MAX: &str = "kwMAX";
/// Tipo dos registros da parede no Fine Artist (`~MAGUSER.INF`).
pub const KIND_MAG: &str = "kwMAG";
/// Lugares na parede da Biblioteca.
pub const WALL_SLOTS: u16 = 5;

const FILR: Tag = Tag(*b"FILR");
const FNTL: Tag = Tag(*b"FNTL");
const STTB: Tag = Tag(*b"STTB");
const BLOCK: usize = 512;

/// Documento pendurado na parede da Biblioteca.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallDoc {
    /// Lugar na parede (id do `FILR`, 0–4).
    pub slot: u16,
    /// "kwMAX" (Writer) ou "kwMAG" (Fine Artist).
    pub kind: String,
    /// Título do documento (o do chunk `TITL`).
    pub title: String,
    /// Pastas a partir de `MSKIDS`, ex. `["SHARED", "USERS", "MAX"]`.
    pub dirs: Vec<String>,
    /// Nome do arquivo (8.3), ex. "SAMP18.MAX".
    pub file: String,
}

impl WallDoc {
    /// Caminho relativo à pasta `MSKIDS`, com `/`.
    pub fn rel_path(&self) -> String {
        let mut s = self.dirs.join("/");
        if !s.is_empty() {
            s.push('/');
        }
        s.push_str(&self.file);
        s
    }

    fn encode(&self) -> Vec<u8> {
        let mut v = Vec::new();
        for s in [&self.kind, &self.title].into_iter().chain(&self.dirs) {
            push_cstr(&mut v, s);
        }
        v.push(0);
        v.extend(encode(&self.file));
        v
    }
}

/// Tabela de strings `STTB` (a mesma estrutura das listas da Máquina de
/// Idéias): `u16` tamanho do resto, `u16` n, `i16` (−1), `u16` bytes extras
/// por entrada, `u16` (0), n × `u16` offsets (a partir do byte 10) e as
/// entradas (Pascal + extras).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringTable {
    /// Campo de 16 bits depois da contagem (−1 em todos os vistos).
    pub unknown: i16,
    /// Campo de 16 bits antes dos offsets (0 em todos os vistos).
    pub reserved: u16,
    /// Bytes extras depois de cada texto (0 no `~MAXUSER.INF`, 2 na Máquina de Idéias).
    pub extra_len: u16,
    pub entries: Vec<(String, Vec<u8>)>,
}

impl StringTable {
    /// Tabela sem extras, como as do `~MAXUSER.INF`.
    pub fn new(texts: &[&str]) -> StringTable {
        StringTable {
            unknown: -1,
            reserved: 0,
            extra_len: 0,
            entries: texts.iter().map(|t| (t.to_string(), Vec::new())).collect(),
        }
    }

    pub fn texts(&self) -> Vec<&str> {
        self.entries.iter().map(|(t, _)| t.as_str()).collect()
    }
}

/// Lê um `STTB`.
pub fn parse_sttb(data: &[u8]) -> Result<StringTable> {
    let mut r = Reader::new(data, Format::Chnk);
    let _total = r.u16_le()?;
    let n = r.u16_le()? as usize;
    let unknown = r.i16_le()?;
    let extra_len = r.u16_le()?;
    let reserved = r.u16_le()?;
    let mut offsets = Vec::with_capacity(n);
    for _ in 0..n {
        offsets.push(r.u16_le()? as usize);
    }
    let mut entries = Vec::with_capacity(n);
    for o in offsets {
        r.seek(10 + o)?;
        let len = r.u8()? as usize;
        let text = cp1252::decode(r.bytes(len)?);
        let extra = r.bytes(extra_len as usize)?.to_vec();
        entries.push((text, extra));
    }
    Ok(StringTable {
        unknown,
        reserved,
        extra_len,
        entries,
    })
}

/// Grava um `STTB`. Os extras de cada entrada são cortados ou completados
/// com zeros até `extra_len`.
pub fn write_sttb(t: &StringTable) -> Vec<u8> {
    let n = t.entries.len();
    let mut body = Vec::new();
    let mut offsets = Vec::with_capacity(n);
    for (text, extra) in &t.entries {
        offsets.push(2 * n + body.len());
        push_pascal(&mut body, text);
        let mut e = extra.clone();
        e.resize(t.extra_len as usize, 0);
        body.extend_from_slice(&e);
    }
    let mut v = Vec::with_capacity(10 + 2 * n + body.len());
    v.extend_from_slice(&((2 * n + body.len()) as u16).to_le_bytes());
    v.extend_from_slice(&(n as u16).to_le_bytes());
    v.extend_from_slice(&t.unknown.to_le_bytes());
    v.extend_from_slice(&t.extra_len.to_le_bytes());
    v.extend_from_slice(&t.reserved.to_le_bytes());
    for o in offsets {
        v.extend_from_slice(&(o as u16).to_le_bytes());
    }
    v.extend_from_slice(&body);
    v
}

/// Conteúdo do `~MAXUSER.INF`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPrefs {
    /// Documentos da parede, pela ordem dos lugares.
    pub wall: Vec<WallDoc>,
    /// Cache das fontes do sistema, como gravado (uma entrada por face e
    /// conjunto de caracteres enumerados, por isso com repetições).
    pub fonts: Vec<String>,
    /// `STTB` por id: entradas do usuário na Máquina de Idéias (uma tabela
    /// por coluna).
    pub string_tables: Vec<(u16, StringTable)>,
    /// Outros chunks (`SUDG` e desconhecidos), guardados para regravar.
    pub other: Vec<OwnedChunk>,
}

impl UserPrefs {
    /// Preferências vazias (nenhum documento na parede, nenhuma fonte).
    pub fn new() -> UserPrefs {
        UserPrefs {
            wall: Vec::new(),
            fonts: Vec::new(),
            string_tables: Vec::new(),
            other: Vec::new(),
        }
    }

    /// Nomes de fontes sem repetição, na ordem do cache.
    pub fn unique_fonts(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for f in &self.fonts {
            if !out.contains(&f.as_str()) {
                out.push(f);
            }
        }
        out
    }

    /// Documento no lugar `slot` da parede.
    pub fn wall_slot(&self, slot: u16) -> Option<&WallDoc> {
        self.wall.iter().find(|d| d.slot == slot)
    }

    /// Tabela `STTB` de id `id`.
    pub fn string_table(&self, id: u16) -> Option<&StringTable> {
        self.string_tables
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, t)| t)
    }
}

impl Default for UserPrefs {
    fn default() -> Self {
        UserPrefs::new()
    }
}

fn encode(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| cp1252::encode_char(c).unwrap_or(b'?'))
        .collect()
}

fn push_cstr(v: &mut Vec<u8>, s: &str) {
    v.extend(encode(s).into_iter().filter(|&b| b != 0));
    v.push(0);
}

fn push_pascal(v: &mut Vec<u8>, s: &str) {
    let mut b = encode(s);
    b.truncate(255);
    v.push(b.len() as u8);
    v.extend_from_slice(&b);
}

fn cstr(r: &mut Reader<'_>, limit: usize) -> Result<String> {
    let start = r.pos();
    let rest = r.bytes(limit.saturating_sub(start))?;
    let Some(n) = rest.iter().position(|&b| b == 0) else {
        r.seek(start)?;
        return Err(r.error(ErrorKind::Invalid("string without terminator".into())));
    };
    r.seek(start + n + 1)?;
    Ok(cp1252::decode(&rest[..n]))
}

/// Lê o começo de um registro `FILR` (tipo, título, pastas e o zero que
/// fecha a lista); devolve o registro com o nome do arquivo vazio.
fn wall_head(r: &mut Reader<'_>, end: usize, slot: u16) -> Result<WallDoc> {
    let kind = cstr(r, end)?;
    let title = cstr(r, end)?;
    let mut dirs = Vec::new();
    loop {
        let d = cstr(r, end)?;
        if d.is_empty() {
            break;
        }
        if dirs.len() >= 16 {
            return Err(r.error(ErrorKind::Invalid("too many folders in FILR".into())));
        }
        dirs.push(d);
    }
    Ok(WallDoc {
        slot,
        kind,
        title,
        dirs,
        file: String::new(),
    })
}

/// Um `FILR`: o nome do arquivo vai até o fim do chunk, sem terminador.
pub fn parse_wall_doc(slot: u16, data: &[u8]) -> Result<WallDoc> {
    let mut r = Reader::new(data, Format::Chnk);
    let mut d = wall_head(&mut r, data.len(), slot)?;
    d.file = cp1252::decode(r.bytes(r.remaining())?);
    Ok(d)
}

/// Lê o cache `FNTL`: `u16` n, n+1 offsets `u16` a partir do início do chunk
/// (o último é o fim) e n nomes Pascal seguidos de um zero.
pub fn parse_font_list(data: &[u8]) -> Result<Vec<String>> {
    let mut r = Reader::new(data, Format::Chnk);
    let n = r.u16_le()? as usize;
    let mut offsets = Vec::with_capacity(n + 1);
    for _ in 0..=n {
        offsets.push(r.u16_le()? as usize);
    }
    let mut out = Vec::with_capacity(n);
    for &o in &offsets[..n] {
        r.seek(o)?;
        let len = r.u8()? as usize;
        out.push(cp1252::decode(r.bytes(len)?));
    }
    Ok(out)
}

/// Grava o cache `FNTL`.
pub fn write_font_list(fonts: &[String]) -> Vec<u8> {
    let n = fonts.len();
    let mut body = Vec::new();
    let mut offsets = Vec::with_capacity(n + 1);
    let head = 2 + 2 * (n + 1);
    for f in fonts {
        offsets.push(head + body.len());
        push_pascal(&mut body, f);
        body.push(0);
    }
    offsets.push(head + body.len());
    let mut v = Vec::with_capacity(head + body.len());
    v.extend_from_slice(&(n as u16).to_le_bytes());
    for o in offsets {
        v.extend_from_slice(&(o as u16).to_le_bytes());
    }
    v.extend_from_slice(&body);
    v
}

/// Lê um `~MAXUSER.INF` (ou `~MAGUSER.INF`), com ou sem índice.
pub fn read(data: &[u8]) -> Result<UserPrefs> {
    if chnk::is_chnk(data) {
        read_indexed(data)
    } else {
        read_headerless(data)
    }
}

fn read_indexed(data: &[u8]) -> Result<UserPrefs> {
    let f = chnk::parse(data)?;
    let mut p = UserPrefs::new();
    for c in &f.chunks {
        let Some(part) = c.parts.first() else {
            continue;
        };
        let at = |e: crate::FormatError| crate::FormatError {
            offset: e.offset + part.offset,
            ..e
        };
        match c.tag {
            FILR => p.wall.push(parse_wall_doc(c.id, part.data).map_err(at)?),
            FNTL if c.id == 0 => p.fonts = parse_font_list(part.data).map_err(at)?,
            STTB => p
                .string_tables
                .push((c.id, parse_sttb(part.data).map_err(at)?)),
            _ => p.other.push(OwnedChunk::from(c)),
        }
    }
    p.wall.sort_by_key(|d| d.slot);
    Ok(p)
}

/// Variante sem índice: 512 bytes de zeros, registros `kw…` em sequência
/// (o nome do arquivo, sem terminador, é lido como 8.3) e o `FNTL`.
fn read_headerless(data: &[u8]) -> Result<UserPrefs> {
    let mut r = Reader::new(data, Format::Chnk);
    if data.len() < BLOCK + 2 || data[..BLOCK].iter().any(|&b| b != 0) {
        return Err(r.error(ErrorKind::BadMagic {
            expected: "CHNK or 512 zero bytes",
        }));
    }
    r.seek(BLOCK)?;
    let mut p = UserPrefs::new();
    while data[r.pos()..].starts_with(b"kw") {
        let slot = p.wall.len() as u16;
        let mut d = wall_head(&mut r, data.len(), slot)?;
        let start = r.pos();
        let rest = &data[start..];
        let dot = rest
            .iter()
            .take(9)
            .position(|&b| b == b'.')
            .ok_or_else(|| r.error(ErrorKind::Invalid("file name is not 8.3".into())))?;
        let len = (dot + 4).min(rest.len());
        d.file = cp1252::decode(&rest[..len]);
        r.seek(start + len)?;
        p.wall.push(d);
    }
    let start = r.pos();
    p.fonts = parse_font_list(&data[start..]).map_err(|e| crate::FormatError {
        offset: e.offset + start,
        ..e
    })?;
    Ok(p)
}

/// Grava o `~MAXUSER.INF` como `CHNK` (`INK `): `FILR` por lugar, `FNTL`,
/// as `STTB` e os outros chunks guardados.
pub fn write(p: &UserPrefs) -> Vec<u8> {
    let part = |tag: Tag, data: Vec<u8>| OwnedPart { tag, data };
    let chunk = |tag: Tag, id: u16, data: Vec<u8>| OwnedChunk {
        tag,
        id,
        flags: 0,
        name: None,
        parts: vec![part(tag, data)],
    };
    let mut chunks = Vec::new();
    for d in &p.wall {
        chunks.push(chunk(FILR, d.slot, d.encode()));
    }
    chunks.push(chunk(FNTL, 0, write_font_list(&p.fonts)));
    for (id, t) in &p.string_tables {
        chunks.push(chunk(STTB, *id, write_sttb(t)));
    }
    chunks.extend(p.other.iter().cloned());
    chnk::write(CREATOR, &chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(slot: u16, title: &str, file: &str) -> WallDoc {
        WallDoc {
            slot,
            kind: KIND_MAX.into(),
            title: title.into(),
            dirs: vec!["SHARED".into(), "USERS".into(), "MAX".into()],
            file: file.into(),
        }
    }

    #[test]
    fn wall_doc_bytes_match_original() {
        let d = doc(0, "O Ponto de Fuga", "SAMP18.MAX");
        let b = d.encode();
        assert_eq!(
            b,
            b"kwMAX\0O Ponto de Fuga\0SHARED\0USERS\0MAX\0\0SAMP18.MAX"
        );
        assert_eq!(b.len(), 50); // tamanho do FILR 0 observado
        assert_eq!(parse_wall_doc(0, &b).unwrap(), d);
        assert_eq!(d.rel_path(), "SHARED/USERS/MAX/SAMP18.MAX");
    }

    #[test]
    fn font_list_layout() {
        let fonts = vec!["Liberation Sans".to_string(), "Caladea".to_string()];
        let b = write_font_list(&fonts);
        // n, 3 offsets, entradas Pascal + zero
        assert_eq!(&b[..8], &[2, 0, 8, 0, 25, 0, 34, 0]);
        assert_eq!(b[8], 15);
        assert_eq!(&b[24..26], &[0, 7]);
        assert_eq!(b.len(), 34);
        assert_eq!(parse_font_list(&b).unwrap(), fonts);
    }

    #[test]
    fn sttb_matches_original() {
        // STTB 0 do ~MAXUSER.INF gravado no original (27 bytes)
        let orig = b"\x11\x00\x01\x00\xff\xff\x00\x00\x00\x00\x02\x00\x0eO pirata Zeca,";
        let t = parse_sttb(orig).unwrap();
        assert_eq!(t.texts(), ["O pirata Zeca,"]);
        assert_eq!(t, StringTable::new(&["O pirata Zeca,"]));
        assert_eq!(write_sttb(&t), orig);
    }

    #[test]
    fn sttb_with_extras() {
        let mut t = StringTable::new(&["abaixo de", "em"]);
        t.extra_len = 2;
        for e in &mut t.entries {
            e.1 = vec![0xff, 0xff];
        }
        let b = write_sttb(&t);
        assert_eq!(&b[..10], &[21, 0, 2, 0, 0xff, 0xff, 2, 0, 0, 0]);
        assert_eq!(parse_sttb(&b).unwrap(), t);
    }

    #[test]
    fn round_trip() {
        let mut p = UserPrefs::new();
        p.wall = vec![
            doc(0, "O Ponto de Fuga", "SAMP18.MAX"),
            doc(1, "Jornal Mágico", "SAMP16.MAX"),
        ];
        p.fonts = ["Liberation Sans", "Liberation Sans", "DejaVu Sans"]
            .map(String::from)
            .to_vec();
        p.string_tables = vec![
            (0, StringTable::new(&["O pirata Zeca,"])),
            (2, StringTable::new(&["na lua."])),
        ];
        p.other = vec![OwnedChunk::single(
            b"SUDG",
            0,
            b"SUDG",
            vec![0, 0, 1, 0, 2, 0],
        )];
        let bytes = write(&p);
        assert!(bytes.starts_with(b"CHNKINK "));
        assert_eq!(bytes.len() % 512, 0);
        let q = read(&bytes).unwrap();
        assert_eq!(q, p);
        assert_eq!(q.unique_fonts(), ["Liberation Sans", "DejaVu Sans"]);
        assert_eq!(q.wall_slot(1).unwrap().title, "Jornal Mágico");
        assert_eq!(q.string_table(2).unwrap().texts(), ["na lua."]);
    }

    #[test]
    fn headerless_variant() {
        let mut v = vec![0u8; 512];
        v.extend(doc(0, "Fique Bom Logo!", "SAMP13.MAX").encode());
        v.extend(doc(1, "Feliz Aniversário!", "SAMP7.MAX").encode());
        v.extend(write_font_list(&["Liberation Mono".to_string()]));
        v.resize(4608, 0);
        let p = read(&v).unwrap();
        assert_eq!(p.wall.len(), 2);
        assert_eq!(p.wall[0].file, "SAMP13.MAX");
        assert_eq!(p.wall[1].title, "Feliz Aniversário!");
        assert_eq!(p.wall[1].file, "SAMP7.MAX");
        assert_eq!(p.fonts, ["Liberation Mono"]);
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(read(b"nada disso").is_err());
        let mut v = vec![0u8; 600];
        v[3] = 1;
        assert!(read(&v).is_err());
    }
}
