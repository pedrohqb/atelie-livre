//! Tipos de letra instalados (TrueType/OpenType) para as palavras com forma.
//!
//! O Cavalete de palavras com forma do original lista as fontes TrueType do
//! Windows (no Wine, só "Liberation Sans"). Aqui a lista vem das pastas de
//! fontes do sistema: uma entrada por família, só a face normal (nem negrito
//! nem inclinada — esses efeitos o cavalete faz por conta própria). Para não
//! ler centenas de megabytes, só o diretório de tabelas e as tabelas `name` e
//! `OS/2` de cada arquivo são lidos; o arquivo inteiro só quando a fonte é usada.

use ab_glyph::FontVec;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;

/// Uma família instalada: nome e onde está a face normal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFace {
    pub name: String,
    pub path: PathBuf,
    /// Índice da face numa coleção `.ttc` (0 nos `.ttf`/`.otf`).
    pub index: u32,
}

/// Pastas onde procurar fontes, na ordem: as do sistema, as do Flatpak
/// (fontes do hospedeiro) e as do usuário.
pub fn font_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = [
        "/usr/share/fonts",
        "/usr/local/share/fonts",
        "/run/host/fonts",
        "/run/host/local-fonts",
        "/run/host/user-fonts",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    if let Some(d) = std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()) {
        dirs.push(PathBuf::from(d).join("fonts"));
    }
    if let Some(h) = std::env::var_os("HOME").filter(|h| !h.is_empty()) {
        let h = PathBuf::from(h);
        dirs.push(h.join(".local/share/fonts"));
        dirs.push(h.join(".fonts"));
    }
    dirs
}

/// Famílias instaladas (lidas uma vez por execução).
pub fn installed() -> &'static [FontFace] {
    static LIST: OnceLock<Vec<FontFace>> = OnceLock::new();
    LIST.get_or_init(|| scan(&font_dirs()))
}

/// Procura as famílias nas pastas (recursivamente), sem repetir nomes, em
/// ordem alfabética.
pub fn scan(dirs: &[PathBuf]) -> Vec<FontFace> {
    let mut files = Vec::new();
    for d in dirs {
        collect(d, &mut files, 0);
    }
    files.sort();
    let mut by_name: HashMap<String, FontFace> = HashMap::new();
    for path in files {
        for face in faces_in(&path) {
            by_name.entry(face.name.to_lowercase()).or_insert(face);
        }
    }
    let mut out: Vec<FontFace> = by_name.into_values().collect();
    out.sort_by_key(|f| f.name.to_lowercase());
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>, depth: u32) {
    if depth > 8 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out, depth + 1);
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
        {
            out.push(p);
        }
    }
}

fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn read_at(f: &mut File, at: u64, len: usize) -> Option<Vec<u8>> {
    if len > 1 << 20 {
        return None;
    }
    f.seek(SeekFrom::Start(at)).ok()?;
    let mut buf = vec![0; len];
    f.read_exact(&mut buf).ok()?;
    Some(buf)
}

/// Faces normais de um arquivo (uma; várias numa coleção).
fn faces_in(path: &Path) -> Vec<FontFace> {
    let Ok(mut f) = File::open(path) else {
        return Vec::new();
    };
    let Some(head) = read_at(&mut f, 0, 12) else {
        return Vec::new();
    };
    let offsets: Vec<u32> = if &head[0..4] == b"ttcf" {
        let n = be32(&head, 8).unwrap_or(0).min(64) as usize;
        match read_at(&mut f, 12, 4 * n) {
            Some(b) => (0..n).filter_map(|k| be32(&b, 4 * k)).collect(),
            None => Vec::new(),
        }
    } else {
        vec![0]
    };
    offsets
        .iter()
        .enumerate()
        .filter_map(|(k, &off)| {
            let name = face_name(&mut f, off as u64)?;
            Some(FontFace {
                name,
                path: path.to_path_buf(),
                index: k as u32,
            })
        })
        .collect()
}

/// Nome da família, se a face no deslocamento `off` é normal (peso 400,
/// largura normal, não inclinada) e tem contornos (`glyf` ou `CFF `).
fn face_name(f: &mut File, off: u64) -> Option<String> {
    let dir = read_at(f, off, 12)?;
    let n = be16(&dir, 4)? as usize;
    let recs = read_at(f, off + 12, 16 * n)?;
    let mut tables = HashMap::new();
    for k in 0..n {
        let r = &recs[16 * k..16 * k + 16];
        let tag: [u8; 4] = r[0..4].try_into().ok()?;
        tables.insert(tag, (be32(r, 8)? as u64, be32(r, 12)? as usize));
    }
    if !tables.contains_key(b"glyf") && !tables.contains_key(b"CFF ") {
        return None;
    }
    let (o, l) = *tables.get(b"OS/2")?;
    let os2 = read_at(f, o, l)?;
    let os2 = ttf_parser::os2::Table::parse(&os2)?;
    if os2.weight() != ttf_parser::Weight::Normal
        || os2.width() != ttf_parser::Width::Normal
        || os2.style() != ttf_parser::Style::Normal
    {
        return None;
    }
    let (o, l) = *tables.get(b"name")?;
    let data = read_at(f, o, l)?;
    let table = ttf_parser::name::Table::parse(&data)?;
    let pick = |id: u16| {
        let mut found: Option<String> = None;
        for n in table.names {
            if n.name_id != id || !n.is_unicode() {
                continue;
            }
            let Some(s) = n.to_string().filter(|s| !s.trim().is_empty()) else {
                continue;
            };
            // inglês dos Estados Unidos primeiro
            if n.language_id == 0x409 || found.is_none() {
                let english = n.language_id == 0x409;
                found = Some(s);
                if english {
                    break;
                }
            }
        }
        found
    };
    pick(ttf_parser::name_id::TYPOGRAPHIC_FAMILY).or_else(|| pick(ttf_parser::name_id::FAMILY))
}

/// Fontes já lidas, por arquivo e face (`None` = arquivo inválido).
type Loaded = HashMap<(PathBuf, u32), Option<Rc<FontVec>>>;

thread_local! {
    static LOADED: RefCell<Loaded> = RefCell::new(HashMap::new());
}

/// Fonte carregada (lida do disco na primeira vez).
pub fn load(face: &FontFace) -> Option<Rc<FontVec>> {
    let key = (face.path.clone(), face.index);
    LOADED.with(|m| {
        m.borrow_mut()
            .entry(key)
            .or_insert_with(|| {
                let data = std::fs::read(&face.path).ok()?;
                FontVec::try_from_vec_and_index(data, face.index)
                    .ok()
                    .map(Rc::new)
            })
            .clone()
    })
}

/// Família pelo nome (sem diferença de maiúsculas).
pub fn find(name: &str) -> Option<&'static FontFace> {
    installed()
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case(name))
}

/// Família inicial do cavalete: uma sem serifa comum, se houver; senão a primeira.
pub fn default_face() -> Option<&'static FontFace> {
    const PREFERRED: [&str; 6] = [
        "Liberation Sans",
        "Arial",
        "DejaVu Sans",
        "FreeSans",
        "Noto Sans",
        "Cantarell",
    ];
    PREFERRED
        .iter()
        .find_map(|n| find(n))
        .or_else(|| installed().first())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_of_missing_dir_is_empty() {
        assert!(scan(&[PathBuf::from("/nao/existe")]).is_empty());
    }

    #[test]
    fn scan_reads_family_of_regular_faces_only() {
        // DejaVu costuma estar instalada; sem ela, o teste não tem o que ver
        let dir = PathBuf::from("/usr/share/fonts/truetype/dejavu");
        if !dir.join("DejaVuSans.ttf").exists() {
            eprintln!("aviso: DejaVu não instalada; teste pulado");
            return;
        }
        let list = scan(&[dir]);
        let sans = list.iter().find(|f| f.name == "DejaVu Sans").unwrap();
        assert!(sans.path.ends_with("DejaVuSans.ttf"), "{:?}", sans.path);
        // negrito e inclinado não entram como famílias próprias
        assert!(!list.iter().any(|f| f.name.contains("Bold")));
        assert!(load(sans).is_some());
    }
}
