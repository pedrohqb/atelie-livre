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
    scan_styles(dirs, false)
}

/// Como [`scan`], mas aceitando famílias que só têm faces inclinadas ou de
/// outro peso (ex.: "Lucida Handwriting", que só existe em itálico); quando a
/// família tem a face normal, é ela que fica.
pub fn scan_any_style(dirs: &[PathBuf]) -> Vec<FontFace> {
    scan_styles(dirs, true)
}

fn scan_styles(dirs: &[PathBuf], any_style: bool) -> Vec<FontFace> {
    let mut files = Vec::new();
    for d in dirs {
        collect(d, &mut files, 0);
    }
    files.sort();
    let mut by_name: HashMap<String, (FontFace, bool)> = HashMap::new();
    for path in files {
        for (face, regular) in faces_in(&path, any_style) {
            let slot = by_name.entry(face.name.to_lowercase());
            let e = slot.or_insert((face.clone(), regular));
            if regular && !e.1 {
                *e = (face, true);
            }
        }
    }
    let mut out: Vec<FontFace> = by_name.into_values().map(|(f, _)| f).collect();
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

/// Faces de um arquivo (uma; várias numa coleção), com a indicação de face
/// normal; sem `any_style`, só as normais.
fn faces_in(path: &Path, any_style: bool) -> Vec<(FontFace, bool)> {
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
            let (name, regular) = face_name(&mut f, off as u64, any_style)?;
            let face = FontFace {
                name,
                path: path.to_path_buf(),
                index: k as u32,
            };
            Some((face, regular))
        })
        .collect()
}

/// Nome da família e se a face no deslocamento `off` é normal (peso 400,
/// largura normal, não inclinada); `None` sem contornos (`glyf` ou `CFF `)
/// ou, sem `any_style`, quando a face não é a normal.
fn face_name(f: &mut File, off: u64, any_style: bool) -> Option<(String, bool)> {
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
    let regular = os2.weight() == ttf_parser::Weight::Normal
        && os2.width() == ttf_parser::Width::Normal
        && os2.style() == ttf_parser::Style::Normal;
    if !regular && !any_style {
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
    pick(ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
        .or_else(|| pick(ttf_parser::name_id::FAMILY))
        .map(|n| (n, regular))
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

/// Família pelo nome em qualquer estilo (lista lida uma vez por execução,
/// só quando pedida).
pub fn find_any_style(name: &str) -> Option<&'static FontFace> {
    static LIST: OnceLock<Vec<FontFace>> = OnceLock::new();
    LIST.get_or_init(|| scan_any_style(&font_dirs()))
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

/// Estilo geral de uma fonte, para a última tentativa de substituição.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontClass {
    Sans,
    Serif,
    Mono,
    /// Manuscrita (Lucida Handwriting).
    Script,
    /// Gótica (Lucida Blackletter); cai na serifada.
    Blackletter,
    /// Letreiro grosso e estreito (Impact) ou vazado (Stencil); cai na sem serifa.
    Display,
}

/// Uma fonte do original e os substitutos, em ordem de preferência.
#[derive(Debug, Clone, Copy)]
pub struct Substitute {
    pub original: &'static str,
    pub class: FontClass,
    pub candidates: &'static [&'static str],
}

/// As 5 TrueType do produto (`STR#` 0x206; ver `docs/formats/fontes.md`),
/// que não vêm nos dados, e as fontes do Windows mais comuns em documentos
/// gravados em outras máquinas (e no Wine), com substitutos livres de
/// métricas ou estilo parecidos.
pub const SUBSTITUTES: &[Substitute] = &[
    Substitute {
        original: "Lucida Sans",
        class: FontClass::Sans,
        candidates: &[
            "Lucida Sans Unicode",
            "DejaVu Sans",
            "Bitstream Vera Sans",
            "Open Sans",
            "Verdana",
            "Noto Sans",
        ],
    },
    Substitute {
        original: "Lucida Handwriting",
        class: FontClass::Script,
        candidates: &[
            "Comic Neue",
            "Kalam",
            "Comic Sans MS",
            "Comic Relief",
            "Z003",
            "URW Chancery L",
            "TeX Gyre Chorus",
        ],
    },
    Substitute {
        original: "Lucida Blackletter",
        class: FontClass::Blackletter,
        candidates: &[
            "UnifrakturMaguntia",
            "UnifrakturCook",
            "Old English Text MT",
        ],
    },
    Substitute {
        original: "Impact",
        class: FontClass::Display,
        candidates: &[
            "Anton",
            "Oswald",
            "League Gothic",
            "Bebas Neue",
            "Haettenschweiler",
            "DejaVu Sans Condensed",
        ],
    },
    Substitute {
        original: "Stencil",
        class: FontClass::Display,
        candidates: &[
            "Stardos Stencil",
            "Allerta Stencil",
            "Saira Stencil One",
            "Black Ops One",
        ],
    },
    Substitute {
        original: "Arial",
        class: FontClass::Sans,
        candidates: &[
            "Liberation Sans",
            "Arimo",
            "Nimbus Sans",
            "Helvetica",
            "FreeSans",
        ],
    },
    Substitute {
        original: "MS Sans Serif",
        class: FontClass::Sans,
        candidates: &[
            "Microsoft Sans Serif",
            "Liberation Sans",
            "Arimo",
            "DejaVu Sans",
        ],
    },
    Substitute {
        original: "Times New Roman",
        class: FontClass::Serif,
        candidates: &["Liberation Serif", "Tinos", "Nimbus Roman", "FreeSerif"],
    },
    Substitute {
        original: "Courier New",
        class: FontClass::Mono,
        candidates: &["Liberation Mono", "Cousine", "Nimbus Mono PS", "FreeMono"],
    },
    Substitute {
        original: "Comic Sans MS",
        class: FontClass::Script,
        candidates: &["Comic Neue", "Comic Relief", "Kalam"],
    },
];

/// Fontes genéricas de cada estilo, na ordem de preferência.
fn generic(class: FontClass) -> &'static [&'static str] {
    const SANS: &[&str] = &[
        "Liberation Sans",
        "Arimo",
        "DejaVu Sans",
        "Noto Sans",
        "FreeSans",
        "Open Sans",
        "Cantarell",
        "Inter",
    ];
    const SERIF: &[&str] = &[
        "Liberation Serif",
        "Tinos",
        "DejaVu Serif",
        "Noto Serif",
        "FreeSerif",
    ];
    const MONO: &[&str] = &[
        "Liberation Mono",
        "Cousine",
        "DejaVu Sans Mono",
        "Noto Sans Mono",
        "FreeMono",
    ];
    const SCRIPT: &[&str] = &[
        "Comic Neue",
        "Kalam",
        "Z003",
        "URW Chancery L",
        "TeX Gyre Chorus",
    ];
    match class {
        FontClass::Sans | FontClass::Display => SANS,
        FontClass::Serif | FontClass::Blackletter => SERIF,
        FontClass::Mono => MONO,
        FontClass::Script => SCRIPT,
    }
}

/// Nome para comparação: minúsculas, sem espaços, hifens nem sublinhados.
fn norm(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Estilo provável de uma fonte fora da tabela, pelo nome.
fn classify(name: &str) -> FontClass {
    let n = norm(name);
    let has = |w: &[&str]| w.iter().any(|k| n.contains(k));
    if has(&["mono", "courier", "typewriter", "console", "code"]) {
        FontClass::Mono
    } else if has(&["sans", "gothic", "grotesk", "helvet"]) {
        FontClass::Sans
    } else if has(&["serif", "times", "roman", "garamond", "book", "georgia"]) {
        FontClass::Serif
    } else if has(&["script", "hand", "comic", "brush", "chancery"]) {
        FontClass::Script
    } else if has(&["fraktur", "blackletter", "oldenglish"]) {
        FontClass::Blackletter
    } else {
        FontClass::Sans
    }
}

/// Escolhe, entre `faces`, a mais parecida com a fonte `name`:
///
/// 1. a própria fonte, se estiver instalada (cópia do usuário);
/// 2. o primeiro substituto instalado da tabela [`SUBSTITUTES`];
/// 3. uma variante da mesma família ("Lucida Sans" → "Lucida Sans Unicode");
/// 4. a resposta de `fontconfig(name)` (um alias configurado no sistema), se
///    estiver entre as faces;
/// 5. uma fonte genérica do mesmo estilo (sem serifa, serifada, mono,
///    manuscrita), e para manuscrita e gótica, depois, as do estilo-base.
///
/// Nomes comparados sem diferença de maiúsculas, espaços ou hifens. `None`
/// se nada servir.
pub fn substitute_in<'a>(
    name: &str,
    faces: &'a [FontFace],
    fontconfig: impl Fn(&str) -> Option<String>,
) -> Option<&'a FontFace> {
    let find = |n: &str| {
        let n = norm(n);
        faces.iter().find(|f| norm(&f.name) == n)
    };
    if let Some(f) = find(name) {
        return Some(f);
    }
    let key = norm(name);
    let entry = SUBSTITUTES.iter().find(|s| norm(s.original) == key);
    if let Some(f) = entry.and_then(|e| e.candidates.iter().find_map(|c| find(c))) {
        return Some(f);
    }
    if !key.is_empty()
        && let Some(f) = faces
            .iter()
            .filter(|f| norm(&f.name).starts_with(&key))
            .min_by_key(|f| f.name.len())
    {
        return Some(f);
    }
    if let Some(f) = fontconfig(name).and_then(|n| find(&n)) {
        return Some(f);
    }
    let class = entry.map_or_else(|| classify(name), |e| e.class);
    let fallback: &[FontClass] = match class {
        FontClass::Script => &[FontClass::Script, FontClass::Sans],
        FontClass::Serif | FontClass::Blackletter => &[FontClass::Serif],
        FontClass::Mono => &[FontClass::Mono],
        FontClass::Sans | FontClass::Display => &[FontClass::Sans],
    };
    fallback
        .iter()
        .flat_map(|c| generic(*c).iter())
        .find_map(|n| find(n))
}

/// Família que o fontconfig (`fc-match`) usaria para `name`, só quando é um
/// alias de verdade (diferente da que ele dá para um nome inexistente).
pub fn fontconfig_alias(name: &str) -> Option<String> {
    fn family(pattern: &str) -> Option<String> {
        let out = std::process::Command::new("fc-match")
            .args(["-f", "%{family[0]}", pattern])
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        let s = String::from_utf8(out.stdout).ok()?;
        let s = s.trim();
        (out.status.success() && !s.is_empty()).then(|| s.to_string())
    }
    static DEFAULT: OnceLock<Option<String>> = OnceLock::new();
    let default = DEFAULT.get_or_init(|| family("atelie-livre-fonte-inexistente"));
    let got = family(name)?;
    (Some(&got) != default.as_ref()).then_some(got)
}

/// Fonte instalada para mostrar o texto escrito com `name` (ao ler o `FONT`
/// de um `.MAX` ou montar o Seleciona-letras): [`substitute_in`] sobre as
/// fontes instaladas, com o fontconfig, e por fim [`default_face`]. O
/// resultado de cada nome fica guardado.
pub fn substitute(name: &str) -> Option<&'static FontFace> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, Option<usize>>>> = OnceLock::new();
    let all = installed();
    let cache = CACHE.get_or_init(Default::default);
    let key = norm(name);
    if let Some(hit) = cache.lock().ok().and_then(|m| m.get(&key).copied()) {
        return hit.map(|i| &all[i]).or_else(default_face);
    }
    let found = substitute_in(name, all, fontconfig_alias)
        .and_then(|f| all.iter().position(|g| std::ptr::eq(f, g)));
    if let Ok(mut m) = cache.lock() {
        m.insert(key, found);
    }
    found.map(|i| &all[i]).or_else(default_face)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faces(names: &[&str]) -> Vec<FontFace> {
        names
            .iter()
            .map(|n| FontFace {
                name: n.to_string(),
                path: PathBuf::from(format!("/fontes/{n}.ttf")),
                index: 0,
            })
            .collect()
    }

    fn sub<'a>(name: &str, list: &'a [FontFace]) -> Option<&'a str> {
        substitute_in(name, list, |_| None).map(|f| f.name.as_str())
    }

    #[test]
    fn original_wins_when_installed() {
        let list = faces(&["Anton", "Impact", "Liberation Sans"]);
        assert_eq!(sub("Impact", &list), Some("Impact"));
        assert_eq!(sub("impact", &list), Some("Impact"));
    }

    #[test]
    fn table_of_product_fonts() {
        let list = faces(&[
            "Comic Neue",
            "DejaVu Sans",
            "Open Sans",
            "Oswald",
            "Allerta Stencil",
            "Unifraktur Maguntia",
            "Liberation Serif",
        ]);
        assert_eq!(sub("Lucida Sans", &list), Some("DejaVu Sans"));
        assert_eq!(sub("Lucida  Handwriting", &list), Some("Comic Neue"));
        // nomes comparados sem espaços
        assert_eq!(
            sub("Lucida Blackletter", &list),
            Some("Unifraktur Maguntia")
        );
        assert_eq!(sub("Impact", &list), Some("Oswald"));
        assert_eq!(sub("Stencil", &list), Some("Allerta Stencil"));
        assert_eq!(sub("Times New Roman", &list), Some("Liberation Serif"));
        // todas as 5 do produto estão na tabela
        for n in [
            "Lucida Sans",
            "Lucida Handwriting",
            "Lucida Blackletter",
            "Impact",
            "Stencil",
        ] {
            assert!(SUBSTITUTES.iter().any(|s| s.original == n), "{n}");
        }
    }

    #[test]
    fn same_family_variant() {
        let list = faces(&["Gill Sans MT", "Gill Sans MT Condensed", "DejaVu Sans"]);
        assert_eq!(sub("Gill Sans", &list), Some("Gill Sans MT"));
    }

    #[test]
    fn fontconfig_alias_is_used_before_generic_lists() {
        let list = faces(&["DejaVu Sans", "TeX Gyre Heros"]);
        let fc = |n: &str| (n == "Helvetica Neue").then(|| "TeX Gyre Heros".to_string());
        let got = substitute_in("Helvetica Neue", &list, fc).unwrap();
        assert_eq!(got.name, "TeX Gyre Heros");
        // alias que não está entre as faces é ignorado
        let fc = |_: &str| Some("Fonte Que Não Existe".to_string());
        assert_eq!(
            substitute_in("Algo", &list, fc).unwrap().name,
            "DejaVu Sans"
        );
    }

    #[test]
    fn generic_fallback_by_style() {
        let list = faces(&[
            "DejaVu Sans",
            "DejaVu Serif",
            "DejaVu Sans Mono",
            "Liberation Sans",
        ]);
        // sem nenhum substituto da tabela: estilo da fonte
        assert_eq!(sub("Lucida Blackletter", &list), Some("DejaVu Serif"));
        assert_eq!(sub("Stencil", &list), Some("Liberation Sans"));
        assert_eq!(sub("Lucida Handwriting", &list), Some("Liberation Sans"));
        assert_eq!(sub("Andale Mono", &list), Some("DejaVu Sans Mono"));
        assert_eq!(sub("Bookman Old Style", &list), Some("DejaVu Serif"));
        assert_eq!(sub("Fonte Desconhecida", &list), Some("Liberation Sans"));
        assert_eq!(sub("Lucida Sans", &[]), None);
        assert_eq!(sub("Lucida Sans", &faces(&["Wingdings"])), None);
    }

    #[test]
    fn substitute_on_this_system() {
        if installed().is_empty() {
            eprintln!("aviso: nenhuma fonte instalada; teste pulado");
            return;
        }
        // sempre há resposta, nem que seja a fonte padrão
        for n in [
            "Lucida Sans",
            "Lucida Handwriting",
            "Lucida Blackletter",
            "Impact",
            "Stencil",
        ] {
            let f = substitute(n).unwrap_or_else(|| panic!("{n}"));
            eprintln!("{n} -> {}", f.name);
        }
        if find("DejaVu Sans").is_some() && find("Lucida Sans").is_none() {
            assert_eq!(substitute("Lucida Sans").unwrap().name, "DejaVu Sans");
        }
        if find("Liberation Sans").is_some() && find("Arial").is_none() {
            assert_eq!(substitute("Arial").unwrap().name, "Liberation Sans");
        }
        // o cache devolve a mesma face
        assert!(std::ptr::eq(
            substitute("Impact").unwrap(),
            substitute("IMPACT").unwrap()
        ));
    }

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

    #[test]
    fn any_style_scan_keeps_italic_only_families() {
        let src = PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans-Oblique.ttf");
        if !src.exists() {
            eprintln!("aviso: DejaVu Sans Oblique não instalada; teste pulado");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(&src, dir.path().join("so-inclinada.ttf")).unwrap();
        let dirs = [dir.path().to_path_buf()];
        assert!(
            scan(&dirs).is_empty(),
            "a lista do cavalete só tem faces normais"
        );
        let any = scan_any_style(&dirs);
        assert_eq!(any.len(), 1);
        assert_eq!(any[0].name, "DejaVu Sans");
    }

    #[test]
    fn any_style_scan_prefers_the_regular_face() {
        let dir = PathBuf::from("/usr/share/fonts/truetype/dejavu");
        if !dir.join("DejaVuSans.ttf").exists() {
            eprintln!("aviso: DejaVu não instalada; teste pulado");
            return;
        }
        let any = scan_any_style(&[dir]);
        let sans = any.iter().find(|f| f.name == "DejaVu Sans").unwrap();
        assert!(sans.path.ends_with("DejaVuSans.ttf"), "{:?}", sans.path);
    }
}
