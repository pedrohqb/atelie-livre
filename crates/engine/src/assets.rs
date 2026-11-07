//! Recursos originais (imagens, sons, cursores) carregados da pasta `MSKIDS`.

use crate::Image;
use crate::paths::{find_ci, walk_files};
use formats::fnt::Font;
use formats::render::{Palettes, bitmap_to_rgba};
use formats::wave::Pcm;
use formats::{chnk, fnt, glob, ne, pkbm, wave, winres};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Program {
    Artist,
    Writer,
}

impl Program {
    fn name(self) -> &'static str {
        match self {
            Program::Artist => "ARTIST",
            Program::Writer => "WRITER",
        }
    }
    fn idx(self) -> usize {
        self as usize
    }
}

const PROGRAMS: [Program; 2] = [Program::Artist, Program::Writer];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageRef {
    Pic(Program, u16),
    Library { file: String, index: usize },
    Icon(Program, u16),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SoundRef {
    Snd(Program, u16),
    Library {
        file: String,
        index: usize,
    },
    /// Arquivo `.wav` do usuário (Cachorrinho, Gravador).
    File(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CursorRef(pub Program, pub u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Image(ImageRef),
    Sound(SoundRef),
    Cursor(CursorRef),
}

#[derive(Debug, Clone)]
pub struct Item {
    pub label: String,
    pub kind: ItemKind,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Debug)]
pub enum AssetsError {
    NotFound(PathBuf),
    NotInstallation(PathBuf),
}

impl fmt::Display for AssetsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetsError::NotFound(p) => write!(f, "pasta não encontrada: {}", p.display()),
            AssetsError::NotInstallation(p) => write!(
                f,
                "{} não parece uma instalação do Fine Artist ou do Creative Writer (falta ARTIST.EXE ou WRITER.EXE)",
                p.display()
            ),
        }
    }
}

impl std::error::Error for AssetsError {}

/// Cursor decodificado com o hotspot.
pub type Cursor = (Rc<Image>, (u16, u16));

/// Imagem ou som dentro de uma biblioteca CHNK: trecho absoluto no arquivo.
struct LibItem {
    range: Range<usize>,
    palettes: Palettes,
}

struct Library {
    data: Vec<u8>,
    images: Vec<LibItem>,
    sounds: Vec<LibItem>,
}

pub struct Assets {
    dir: PathBuf,
    exe: [Option<Vec<u8>>; 2],
    pic: [Option<Vec<u8>>; 2],
    snd: [Option<Vec<u8>>; 2],
    palettes: [Palettes; 2],
    /// Ids das imagens de cada `.PIC`, em ordem crescente.
    pic_ids: [Vec<u16>; 2],
    libraries: HashMap<String, Library>,
    catalog: Vec<Source>,
    images: RefCell<HashMap<ImageRef, Rc<Image>>>,
    sounds: RefCell<HashMap<SoundRef, Option<Rc<Pcm>>>>,
    cursors: RefCell<HashMap<CursorRef, Option<Cursor>>>,
    warned: RefCell<HashSet<String>>,
    fonts: Vec<Rc<Font>>,
    /// Volume atual dos sons (0–100), mostrado no controle de volume do
    /// Seleciona-sons; a plataforma o ajusta pela configuração.
    volume: Cell<u8>,
}

/// Volume inicial (o mesmo padrão da configuração do app).
pub const DEFAULT_VOLUME: u8 = 80;

/// Quadriculado magenta e preto de 4 px, usado no lugar de imagens ausentes.
pub fn placeholder(w: u32, h: u32) -> Image {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let on = ((x / 4) + (y / 4)) % 2 == 0;
            rgba.extend_from_slice(if on {
                &[255, 0, 255, 255]
            } else {
                &[0, 0, 0, 255]
            });
        }
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
}

fn read_ci(dir: &Path, rel: &str) -> Option<Vec<u8>> {
    std::fs::read(find_ci(dir, rel)?).ok()
}

impl Assets {
    pub fn open(dir: &Path) -> Result<Assets, AssetsError> {
        if !dir.is_dir() {
            return Err(AssetsError::NotFound(dir.to_path_buf()));
        }
        let exe = PROGRAMS.map(|p| read_ci(dir, &format!("{}.EXE", p.name())));
        if exe.iter().all(Option::is_none) {
            return Err(AssetsError::NotInstallation(dir.to_path_buf()));
        }
        let pic = PROGRAMS.map(|p| read_ci(dir, &format!("{0}/{0}.PIC", p.name())));
        let snd = PROGRAMS.map(|p| read_ci(dir, &format!("{0}/{0}.SND", p.name())));
        let palettes = PROGRAMS.map(|p| Palettes::from_exe(exe[p.idx()].as_deref()));
        // bibliotecas usam a paleta do ARTIST.EXE, ou a do WRITER.EXE na falta dele
        let lib_pal = if exe[0].is_some() {
            palettes[0].clone()
        } else {
            palettes[1].clone()
        };

        let mut libraries = HashMap::new();
        let mut lib_sources = Vec::new();
        if let Some(stuff) = find_ci(dir, "SHARED/STUFF") {
            for path in walk_files(&stuff) {
                let Ok(data) = std::fs::read(&path) else {
                    continue;
                };
                if !chnk::is_chnk(&data) {
                    continue;
                }
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut images = Vec::new();
                let mut sounds = Vec::new();
                let mut items = Vec::new();
                collect_library(
                    &data,
                    0,
                    &lib_pal,
                    &name,
                    &mut images,
                    &mut sounds,
                    &mut items,
                    0,
                );
                if !items.is_empty() {
                    lib_sources.push(Source {
                        name: name.clone(),
                        items,
                    });
                    libraries.insert(
                        name,
                        Library {
                            data,
                            images,
                            sounds,
                        },
                    );
                }
            }
        }
        lib_sources.sort_by(|a, b| a.name.cmp(&b.name));

        let mut catalog = Vec::new();
        let mut pic_ids: [Vec<u16>; 2] = [Vec::new(), Vec::new()];
        for p in PROGRAMS {
            if let Some(d) = &pic[p.idx()] {
                let entries = glob::parse(d).unwrap_or_default();
                let items: Vec<Item> = entries
                    .iter()
                    .map(|e| Item {
                        label: format!("{:04x}", e.id),
                        kind: ItemKind::Image(ImageRef::Pic(p, e.id)),
                    })
                    .collect();
                let mut ids: Vec<u16> = entries.iter().map(|e| e.id).collect();
                ids.sort_unstable();
                ids.dedup();
                pic_ids[p.idx()] = ids;
                push_source(&mut catalog, format!("{}.PIC", p.name()), items);
            }
        }
        catalog.extend(lib_sources);
        let mut icons = Vec::new();
        let mut cursors = Vec::new();
        for p in PROGRAMS {
            if let Some(f) = exe[p.idx()].as_deref().and_then(|d| ne::parse(d).ok()) {
                for r in f.of_kind("ICON").filter(|r| r.name.is_none()) {
                    icons.push(Item {
                        label: format!("{} {:04x}", p.name(), r.id),
                        kind: ItemKind::Image(ImageRef::Icon(p, r.id)),
                    });
                }
                for r in f.of_kind("CURSOR").filter(|r| r.name.is_none()) {
                    cursors.push(Item {
                        label: format!("{} {:04x}", p.name(), r.id),
                        kind: ItemKind::Cursor(CursorRef(p, r.id)),
                    });
                }
            }
        }
        push_source(&mut catalog, "Ícones".into(), icons);
        push_source(&mut catalog, "Cursores".into(), cursors);
        for p in PROGRAMS {
            if let Some(f) = snd[p.idx()].as_deref().and_then(|d| ne::parse(d).ok()) {
                let items = f
                    .of_kind("SND")
                    .filter(|r| r.name.is_none())
                    .map(|r| Item {
                        label: format!("{:04x}", r.id),
                        kind: ItemKind::Sound(SoundRef::Snd(p, r.id)),
                    })
                    .collect();
                push_source(&mut catalog, format!("{}.SND", p.name()), items);
            }
        }

        // fonte dos balões e barras: "Pablo Graphos" (SHARED/STUFF/PABLO.FON)
        let fonts = read_ci(dir, "SHARED/STUFF/PABLO.FON")
            .and_then(|d| {
                let f = ne::parse(&d).ok()?;
                Some(
                    f.of_kind("FONT")
                        .filter_map(|r| fnt::parse(r.data).ok())
                        .map(Rc::new)
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap_or_default();
        Ok(Assets {
            dir: dir.to_path_buf(),
            exe,
            pic,
            snd,
            palettes,
            pic_ids,
            libraries,
            catalog,
            images: RefCell::new(HashMap::new()),
            sounds: RefCell::new(HashMap::new()),
            cursors: RefCell::new(HashMap::new()),
            warned: RefCell::new(HashSet::new()),
            fonts,
            volume: Cell::new(DEFAULT_VOLUME),
        })
    }

    /// Volume dos sons (0–100).
    pub fn volume(&self) -> u8 {
        self.volume.get()
    }

    /// Guarda o volume (limitado a 100); quem toca os sons é a plataforma
    /// (`Effect::SetVolume`).
    pub fn set_volume(&self, v: u8) {
        self.volume.set(v.min(100));
    }

    /// Esquece o som guardado em cache (um arquivo regravado com o mesmo nome).
    pub fn forget_sound(&self, r: &SoundRef) {
        self.sounds.borrow_mut().remove(r);
    }

    /// Pasta `MSKIDS` de onde os dados foram abertos.
    pub fn data_dir(&self) -> &Path {
        &self.dir
    }

    /// Fonte "Pablo Graphos" no tamanho em pontos (6, 8 ou 16), se instalada.
    pub fn font(&self, points: u16) -> Option<Rc<Font>> {
        self.fonts.iter().find(|f| f.points == points).cloned()
    }

    /// Paletas de 16 e 256 cores do programa.
    pub fn palettes(&self, p: Program) -> &Palettes {
        &self.palettes[p.idx()]
    }

    /// Bytes de um recurso numerado do executável (`ATAG`, `PAT#`, ...), se existir.
    pub fn exe_resource(&self, p: Program, kind: &str, id: u16) -> Option<Vec<u8>> {
        self.exe[p.idx()]
            .as_deref()
            .and_then(|d| ne::parse(d).ok())
            .and_then(|f| f.find(kind, id).map(|r| r.data.to_vec()))
    }

    /// Balões de um bloco `WHAT` do executável (ajuda e assistentes), vazio se faltar.
    pub fn dialogs(&self, p: Program, block: u16) -> Vec<formats::what::Record> {
        self.exe_resource(p, "WHAT", block)
            .and_then(|d| formats::what::parse(&d).ok())
            .unwrap_or_default()
    }

    /// Ids dos recursos numerados de um tipo do executável, em ordem crescente.
    pub fn exe_resource_ids(&self, p: Program, kind: &str) -> Vec<u16> {
        let mut ids: Vec<u16> = self.exe[p.idx()]
            .as_deref()
            .and_then(|d| ne::parse(d).ok())
            .map(|f| {
                f.of_kind(kind)
                    .filter(|r| r.name.is_none())
                    .map(|r| r.id)
                    .collect()
            })
            .unwrap_or_default();
        ids.sort_unstable();
        ids
    }

    /// Ids das imagens do `.PIC` do programa, em ordem crescente (vazio se faltar).
    pub fn pic_ids(&self, p: Program) -> &[u16] {
        &self.pic_ids[p.idx()]
    }

    /// Id da imagem que antecede `id` no `.PIC` (a versão colorida de uma
    /// imagem de 1 bit dos roteiros `ATAG`, ver `docs/formats/atag.md`).
    pub fn pic_prev_id(&self, p: Program, id: u16) -> Option<u16> {
        let ids = &self.pic_ids[p.idx()];
        let k = ids.binary_search(&id).ok()?;
        k.checked_sub(1).map(|j| ids[j])
    }

    /// Id da `n`-ésima imagem depois de `id` no `.PIC` (as versões de 1 bit e
    /// a máscara de uma figura colorida são as duas imagens seguintes).
    pub fn pic_next_id(&self, p: Program, id: u16, n: usize) -> Option<u16> {
        let ids = &self.pic_ids[p.idx()];
        let k = ids.binary_search(&id).ok()?;
        ids.get(k + n).copied()
    }

    /// Padrões 8×8 do `PAT#` 0x80 do executável; vazio se faltar.
    pub fn patterns(&self, p: Program) -> Vec<[u8; 8]> {
        self.exe[p.idx()]
            .as_deref()
            .and_then(|d| ne::parse(d).ok())
            .and_then(|f| f.find("PAT#", 0x80).map(|r| r.data.to_vec()))
            .and_then(|d| formats::macres::parse_pattern_list(&d).ok())
            .unwrap_or_default()
    }

    /// Animações de uma biblioteca `.ANI`: para cada chunk `ATAG` (em ordem de
    /// número), os bytes do roteiro e os `ImageRef` dos `PKBM` aninhados, em ordem.
    pub fn animations(&self, name: &str) -> Vec<(u16, Vec<u8>, Vec<ImageRef>)> {
        let wanted = name.to_lowercase();
        let Some((file, lib)) = self
            .libraries
            .iter()
            .find(|(k, _)| k.to_lowercase() == wanted)
        else {
            return Vec::new();
        };
        let Ok(f) = chnk::parse(&lib.data) else {
            return Vec::new();
        };
        // mesma ordem de `collect_library`: chunks e partes em ordem, aninhados por dentro
        let mut next_image = 0usize;
        let mut out = Vec::new();
        for c in &f.chunks {
            for p in &c.parts {
                if chnk::is_chnk(p.data) {
                    let Ok(inner) = chnk::parse(p.data) else {
                        continue;
                    };
                    let mut script = None;
                    let mut images = Vec::new();
                    for ic in &inner.chunks {
                        for ip in &ic.parts {
                            if pkbm::is_pkbm(ip.data) {
                                images.push(ImageRef::Library {
                                    file: file.clone(),
                                    index: next_image,
                                });
                                next_image += 1;
                            } else if &ic.tag.0 == b"ATAG" && script.is_none() {
                                script = Some(ip.data.to_vec());
                            }
                        }
                    }
                    if let Some(sc) = script {
                        out.push((c.id, sc, images));
                    }
                } else if pkbm::is_pkbm(p.data) {
                    next_image += 1;
                }
            }
        }
        out.sort_by_key(|(id, _, _)| *id);
        out
    }

    /// Conteúdo bruto de uma biblioteca de `SHARED/STUFF` (nome sem distinguir maiúsculas).
    pub fn library_bytes(&self, name: &str) -> Option<&[u8]> {
        let wanted = name.to_lowercase();
        self.libraries
            .iter()
            .find(|(k, _)| k.to_lowercase() == wanted)
            .map(|(_, l)| l.data.as_slice())
    }

    pub fn catalog(&self) -> &[Source] {
        &self.catalog
    }

    fn warn(&self, what: String) {
        if self.warned.borrow_mut().insert(what.clone()) {
            eprintln!("aviso: {what}");
        }
    }

    /// Há uma imagem válida para `r`? (Sem aviso; a imagem fica em cache.)
    pub fn has_image(&self, r: &ImageRef) -> bool {
        if self.images.borrow().contains_key(r) {
            return true;
        }
        match self.load_image(r) {
            Ok(img) => {
                self.images.borrow_mut().insert(r.clone(), Rc::new(img));
                true
            }
            Err(_) => false,
        }
    }

    /// Imagem decodificada (em cache); recurso ausente ou inválido vira `placeholder(32, 32)`.
    pub fn image(&self, r: &ImageRef) -> Rc<Image> {
        if let Some(img) = self.images.borrow().get(r) {
            return img.clone();
        }
        let img = match self.load_image(r) {
            Ok(img) => img,
            Err(e) => {
                self.warn(format!("imagem {r:?}: {e}"));
                placeholder(32, 32)
            }
        };
        let img = Rc::new(img);
        self.images.borrow_mut().insert(r.clone(), img.clone());
        img
    }

    fn load_image(&self, r: &ImageRef) -> Result<Image, String> {
        match r {
            ImageRef::Pic(p, id) => {
                let data = self.pic[p.idx()].as_deref().ok_or("arquivo .PIC ausente")?;
                let entries = glob::parse(data).map_err(|e| e.to_string())?;
                let e = entries
                    .iter()
                    .find(|e| e.id == *id)
                    .ok_or("id inexistente")?;
                let b = pkbm::decode(e.data).map_err(|e| e.to_string())?;
                Ok(bitmap_to_rgba(&b, &self.palettes[p.idx()]))
            }
            ImageRef::Library { file, index } => {
                let lib = self.libraries.get(file).ok_or("biblioteca ausente")?;
                let item = lib.images.get(*index).ok_or("índice inexistente")?;
                let b = pkbm::decode(&lib.data[item.range.clone()]).map_err(|e| e.to_string())?;
                Ok(bitmap_to_rgba(&b, &item.palettes))
            }
            ImageRef::Icon(p, id) => {
                let data = self.exe[p.idx()].as_deref().ok_or("executável ausente")?;
                let f = ne::parse(data).map_err(|e| e.to_string())?;
                let res = f.find("ICON", *id).ok_or("id inexistente")?;
                winres::decode_icon(res.data).map_err(|e| e.to_string())
            }
        }
    }

    pub fn sound(&self, r: &SoundRef) -> Option<Rc<Pcm>> {
        if let Some(s) = self.sounds.borrow().get(r) {
            return s.clone();
        }
        let s = match self.load_sound(r) {
            Ok(p) => Some(Rc::new(p)),
            Err(e) => {
                self.warn(format!("som {r:?}: {e}"));
                None
            }
        };
        // um arquivo do usuário pode aparecer depois (gravação em andamento)
        if s.is_some() || !matches!(r, SoundRef::File(_)) {
            self.sounds.borrow_mut().insert(r.clone(), s.clone());
        }
        s
    }

    fn load_sound(&self, r: &SoundRef) -> Result<Pcm, String> {
        let data: &[u8] = match r {
            SoundRef::Snd(p, id) => {
                let dll = self.snd[p.idx()].as_deref().ok_or("arquivo .SND ausente")?;
                let f = ne::parse(dll).map_err(|e| e.to_string())?;
                let res = f.find("SND", *id).ok_or("id inexistente")?;
                return wave::decode(res.data).map_err(|e| e.to_string());
            }
            SoundRef::Library { file, index } => {
                let lib = self.libraries.get(file).ok_or("biblioteca ausente")?;
                let item = lib.sounds.get(*index).ok_or("índice inexistente")?;
                &lib.data[item.range.clone()]
            }
            SoundRef::File(path) => {
                let data = std::fs::read(path).map_err(|e| e.to_string())?;
                return wave::decode(&data).map_err(|e| e.to_string());
            }
        };
        wave::decode(data).map_err(|e| e.to_string())
    }

    pub fn cursor(&self, c: &CursorRef) -> Option<Cursor> {
        if let Some(v) = self.cursors.borrow().get(c) {
            return v.clone();
        }
        let loaded = (|| -> Result<(Image, (u16, u16)), String> {
            let data = self.exe[c.0.idx()].as_deref().ok_or("executável ausente")?;
            let f = ne::parse(data).map_err(|e| e.to_string())?;
            let res = f.find("CURSOR", c.1).ok_or("id inexistente")?;
            winres::decode_cursor(res.data).map_err(|e| e.to_string())
        })();
        let v = match loaded {
            Ok((img, hs)) => Some((Rc::new(img), hs)),
            Err(e) => {
                self.warn(format!("cursor {c:?}: {e}"));
                None
            }
        };
        self.cursors.borrow_mut().insert(*c, v.clone());
        v
    }
}

fn push_source(catalog: &mut Vec<Source>, name: String, items: Vec<Item>) {
    if !items.is_empty() {
        catalog.push(Source { name, items });
    }
}

const MAX_NESTING: usize = 8;

/// Percorre um CHNK (e os aninhados), anotando imagens e sons com o trecho absoluto.
#[allow(clippy::too_many_arguments)]
fn collect_library(
    data: &[u8],
    base: usize,
    pal: &Palettes,
    file: &str,
    images: &mut Vec<LibItem>,
    sounds: &mut Vec<LibItem>,
    items: &mut Vec<Item>,
    depth: usize,
) {
    let Ok(f) = chnk::parse(data) else { return };
    let pal = pal.for_chnk(&f);
    for c in &f.chunks {
        let label = match &c.name {
            Some(n) => format!("{} {} {n}", c.tag, c.id),
            None => format!("{} {}", c.tag, c.id),
        };
        for p in &c.parts {
            let range = base + p.offset..base + p.offset + p.data.len();
            if chnk::is_chnk(p.data) {
                if depth + 1 < MAX_NESTING && p.data.len() < data.len() {
                    collect_library(
                        p.data,
                        range.start,
                        &pal,
                        file,
                        images,
                        sounds,
                        items,
                        depth + 1,
                    );
                }
            } else if pkbm::is_pkbm(p.data) {
                let index = images.len();
                images.push(LibItem {
                    range,
                    palettes: pal.clone(),
                });
                let kind = ItemKind::Image(ImageRef::Library {
                    file: file.to_string(),
                    index,
                });
                items.push(Item {
                    label: label.clone(),
                    kind,
                });
            } else if wave::is_riff_wave(p.data) {
                let index = sounds.len();
                sounds.push(LibItem {
                    range,
                    palettes: pal.clone(),
                });
                let kind = ItemKind::Sound(SoundRef::Library {
                    file: file.to_string(),
                    index,
                });
                items.push(Item {
                    label: label.clone(),
                    kind,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_missing_dir_is_not_found() {
        let e = Assets::open(Path::new("/nao/existe/mskids")).err().unwrap();
        assert!(matches!(e, AssetsError::NotFound(_)));
        assert!(!e.to_string().is_empty());
    }

    #[test]
    fn open_dir_without_exe_is_not_installation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("leia-me.txt"), b"x").unwrap();
        assert!(matches!(
            Assets::open(dir.path()).err().unwrap(),
            AssetsError::NotInstallation(_)
        ));
    }

    #[test]
    fn patterns_without_exe_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("WRITER.EXE"), b"MZ lixo").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        assert!(a.patterns(Program::Artist).is_empty());
        assert!(a.patterns(Program::Writer).is_empty());
        assert_eq!(a.palettes(Program::Artist).pal256.len(), 256);
    }

    #[test]
    fn has_image_is_false_without_pic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        assert!(!a.has_image(&ImageRef::Pic(Program::Artist, 0x1000)));
        // `image` continua devolvendo o quadriculado
        assert_eq!(a.image(&ImageRef::Pic(Program::Artist, 0x1000)).width, 32);
    }

    #[test]
    fn placeholder_is_checkered() {
        let p = placeholder(32, 32);
        assert_eq!((p.width, p.height), (32, 32));
        assert_eq!(&p.rgba[0..4], &[255, 0, 255, 255]);
        let i = 4 * 4;
        assert_eq!(&p.rgba[i..i + 4], &[0, 0, 0, 255]);
    }
}
