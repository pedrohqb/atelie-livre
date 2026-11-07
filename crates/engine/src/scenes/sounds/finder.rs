//! Cachorrinho do Seleciona-sons: o diálogo verde "achar sons". No original
//! ele procura sons na pasta do usuário e, "fora de Imaginópolis", no disco
//! todo; aqui lista os `.wav` da pasta de desenhos (e da subpasta `sons`, onde
//! o Gravador guarda) e, fora de Imaginópolis, os da pasta de música do
//! usuário (`XDG_MUSIC_DIR`, `~/Música` ou `~/Music`). Clicar num som toca;
//! JÁ põe o som na mão. Geometria do diálogo medida no original (varredura
//! `paint-sounds`, `tools_o_snd_dog.png`; a mesma do Cachorrinho dos adesivos).

use super::{Hand, Rect, inside, pic, snd};
use crate::Frame;
use crate::assets::{Assets, SoundRef};
use crate::scene::Effect;
use std::path::{Path, PathBuf};

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const GREEN: [u8; 3] = [71, 140, 80];
const SELECTED: [u8; 3] = [200, 220, 255];
const DIM: [u8; 3] = [96, 96, 96];
/// Painel verde com borda preta de 5 px.
const PANEL: Rect = (120, 111, 406, 257);
/// Tira de filme vertical que separa as duas metades.
const FILM_X: i32 = 312;
const FILM_W: i32 = 18;
const FOLDER_FIELD: Rect = (143, 139, 153, 19);
const LIST: Rect = (143, 178, 153, 136);
const LINE_H: i32 = 17;
const ROWS: usize = 8;
const FOLDER_PREV: Rect = (142, 120, 14, 16);
const FOLDER_NEXT: Rect = (283, 120, 14, 16);
const LIST_UP: Rect = (141, 161, 16, 14);
const LIST_DOWN: Rect = (282, 161, 16, 14);
const TITLE_FIELD: Rect = (346, 297, 153, 19);
const ARTIST_FIELD: Rect = (346, 335, 153, 19);
/// Ícone do som escolhido na metade direita (o alto-falante do Gravador).
const ICON: (i32, i32) = (402, 160);
/// Botões: cachorro (procurar de novo), X (cancelar), JÁ (usar).
const DOG_BTN: Rect = (157, 325, 37, 32);
const CANCEL_BTN: Rect = (207, 327, 28, 28);
const OK_BTN: Rect = (251, 328, 26, 27);
const OUTSIDE: &str = "Fora de Imaginópolis";
const INSIDE: &str = "Imaginópolis";
/// Limites da busca fora de Imaginópolis.
const MAX_DEPTH: usize = 4;
const MAX_FILES: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Sound(PathBuf),
    /// Entrada que leva aos sons de fora (pasta de música).
    Outside,
    /// Entrada que volta aos sons da pasta de desenhos.
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    kind: Kind,
    pub title: String,
    /// Pasta onde o som está (mostrada em "Artista").
    pub folder: String,
}

/// Resposta a um clique no diálogo.
#[derive(Debug, Default, PartialEq)]
pub struct Reply {
    pub fx: Vec<Effect>,
    /// Fechar o diálogo.
    pub close: bool,
    /// Som escolhido com JÁ.
    pub chosen: Option<Hand>,
}

fn is_wav(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
}

fn entry(p: &Path) -> Entry {
    Entry {
        kind: Kind::Sound(p.to_path_buf()),
        title: p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        folder: p
            .parent()
            .and_then(|d| d.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

/// `.wav` de uma pasta (sem descer), em ordem, sem os ocultos (gravações em
/// andamento começam com ponto).
fn wavs_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_wav(p))
        .filter(|p| {
            !p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .collect();
    v.sort();
    v
}

/// Sons da pasta de desenhos e da subpasta `sons`, e no fim "Fora de Imaginópolis".
pub fn own_entries(dir: Option<&Path>) -> Vec<Entry> {
    let mut v: Vec<Entry> = Vec::new();
    if let Some(d) = dir {
        for p in wavs_in(d)
            .iter()
            .chain(wavs_in(&d.join(super::recorder::SOUNDS_DIR)).iter())
        {
            v.push(entry(p));
        }
    }
    v.push(Entry {
        kind: Kind::Outside,
        title: OUTSIDE.into(),
        folder: String::new(),
    });
    v
}

/// `.wav` sob as pastas, até `MAX_DEPTH` níveis e `MAX_FILES` arquivos; no
/// fim, "Imaginópolis" para voltar.
pub fn outside_entries(dirs: &[PathBuf]) -> Vec<Entry> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if out.len() >= MAX_FILES {
            return;
        }
        out.extend(wavs_in(dir).into_iter().take(MAX_FILES - out.len()));
        if depth == 0 {
            return;
        }
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut subs: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                // links para pastas não são seguidos (evita laços)
                std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir())
                    && !p
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            })
            .collect();
        subs.sort();
        for s in subs {
            walk(&s, depth - 1, out);
        }
    }
    let mut files = Vec::new();
    for d in dirs {
        walk(d, MAX_DEPTH, &mut files);
    }
    files.sort();
    files.dedup();
    let mut v: Vec<Entry> = files.iter().map(|p| entry(p)).collect();
    v.push(Entry {
        kind: Kind::Back,
        title: INSIDE.into(),
        folder: String::new(),
    });
    v
}

/// Pastas de música do usuário: `XDG_MUSIC_DIR` (do ambiente ou de
/// `user-dirs.dirs`), `~/Música` e `~/Music`, as que existirem.
pub fn music_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut v = Vec::new();
    if let Some(d) = std::env::var_os("XDG_MUSIC_DIR") {
        v.push(PathBuf::from(d));
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|h| h.join(".config")));
    if let Some(text) = config.and_then(|c| std::fs::read_to_string(c.join("user-dirs.dirs")).ok())
        && let Some(d) = parse_user_dirs(&text, home.as_deref())
    {
        v.push(d);
    }
    if let Some(h) = &home {
        v.push(h.join("Música"));
        v.push(h.join("Music"));
    }
    let mut out: Vec<PathBuf> = Vec::new();
    for d in v {
        if d.is_dir() && !out.contains(&d) {
            out.push(d);
        }
    }
    out
}

/// `XDG_MUSIC_DIR="$HOME/Música"` de um `user-dirs.dirs`.
fn parse_user_dirs(text: &str, home: Option<&Path>) -> Option<PathBuf> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("XDG_MUSIC_DIR="))?;
    let v = line["XDG_MUSIC_DIR=".len()..].trim_matches('"');
    match v.strip_prefix("$HOME") {
        Some(rest) => Some(home?.join(rest.trim_start_matches('/'))),
        None if v.starts_with('/') => Some(PathBuf::from(v)),
        None => None,
    }
}

/// Corta o texto com "…" para caber em `max` pixels.
pub fn clip_text(s: &str, width: impl Fn(&str) -> i32, max: i32) -> String {
    if width(s) <= max {
        return s.to_string();
    }
    let mut out: String = s.to_string();
    while !out.is_empty() && width(&format!("{out}…")) > max {
        out.pop();
    }
    format!("{out}…")
}

/// Pinta o fundo de cinza pontilhado (a tela "apagada" atrás dos diálogos).
pub fn dim(f: &mut Frame) {
    let (w, h) = (f.width() as i32, f.height() as i32);
    for y in 0..h {
        for x in 0..w {
            if (x + y) % 2 == 0 {
                f.put(x, y, [140, 140, 140]);
            }
        }
    }
}

pub struct Finder {
    entries: Vec<Entry>,
    scroll: usize,
    selected: Option<usize>,
    /// Listando os sons de fora (pasta de música).
    outside: bool,
    dir: Option<PathBuf>,
    music: Vec<PathBuf>,
}

impl Finder {
    /// Diálogo com os sons da pasta de desenhos; `music` são as pastas de fora.
    pub fn new(dir: Option<&Path>, music: Vec<PathBuf>) -> Finder {
        Finder {
            entries: own_entries(dir),
            scroll: 0,
            selected: None,
            outside: false,
            dir: dir.map(Path::to_path_buf),
            music,
        }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn selected(&self) -> Option<&Entry> {
        self.selected.and_then(|k| self.entries.get(k))
    }

    pub fn outside(&self) -> bool {
        self.outside
    }

    fn row_at(&self, x: i32, y: i32) -> Option<usize> {
        if !inside(LIST, x, y) {
            return None;
        }
        let k = self.scroll + ((y - LIST.1 - 1) / LINE_H).max(0) as usize;
        (k < self.entries.len()).then_some(k)
    }

    fn swap(&mut self, outside: bool) {
        self.outside = outside;
        self.entries = if outside {
            outside_entries(&self.music)
        } else {
            own_entries(self.dir.as_deref())
        };
        self.scroll = 0;
        self.selected = None;
    }

    pub fn click(&mut self, x: i32, y: i32) -> Reply {
        let mut r = Reply::default();
        if let Some(k) = self.row_at(x, y) {
            match self.entries[k].kind.clone() {
                Kind::Sound(p) => {
                    self.selected = Some(k);
                    r.fx.push(Effect::PlaySound(SoundRef::File(p)));
                }
                Kind::Outside => self.swap(true),
                Kind::Back => self.swap(false),
            }
        } else if inside(FOLDER_PREV, x, y) || inside(FOLDER_NEXT, x, y) {
            self.swap(!self.outside);
            r.fx.push(snd(super::SND_PAGE));
        } else if inside(LIST_UP, x, y) {
            self.scroll = self.scroll.saturating_sub(ROWS - 1);
            r.fx.push(snd(super::SND_PAGE));
        } else if inside(LIST_DOWN, x, y) {
            let max = self.entries.len().saturating_sub(ROWS);
            self.scroll = (self.scroll + ROWS - 1).min(max);
            r.fx.push(snd(super::SND_PAGE));
        } else if inside(DOG_BTN, x, y) {
            // procurar de novo (sons novos gravados ou copiados)
            self.swap(self.outside);
            r.fx.push(snd(super::SND_DOG));
        } else if inside(CANCEL_BTN, x, y) {
            r.close = true;
            r.fx.push(snd(super::SND_CANCEL));
        } else if inside(OK_BTN, x, y) {
            if let Some(Entry {
                kind: Kind::Sound(p),
                title,
                ..
            }) = self.selected().cloned()
            {
                r.close = true;
                r.chosen = Some(Hand::Sound {
                    name: title,
                    sound: SoundRef::File(p),
                });
            }
        }
        r
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let font = a.font(8);
        let text = |f: &mut Frame, x: i32, y: i32, s: &str| match &font {
            Some(fo) => {
                f.text_font(fo, x, y, s, BLACK);
            }
            None => f.text(x, y, s, BLACK),
        };
        let width = |s: &str| -> i32 {
            match &font {
                Some(fo) => fo.width(s) as i32,
                None => 8 * s.chars().count() as i32,
            }
        };
        dim(f);
        f.rect(PANEL.0, PANEL.1, PANEL.2, PANEL.3, BLACK);
        f.rect(PANEL.0 + 5, PANEL.1 + 5, PANEL.2 - 10, PANEL.3 - 10, GREEN);
        for corner in [
            (PANEL.0, PANEL.1),
            (PANEL.0 + PANEL.2 - 6, PANEL.1),
            (PANEL.0, PANEL.1 + PANEL.3 - 6),
            (PANEL.0 + PANEL.2 - 6, PANEL.1 + PANEL.3 - 6),
        ] {
            f.rect(corner.0, corner.1, 6, 6, DIM);
        }
        f.rect(FILM_X, PANEL.1, FILM_W, PANEL.3, BLACK);
        for y in (PANEL.1 + 12..PANEL.1 + PANEL.3 - 8).step_by(8) {
            f.rect(FILM_X + 5, y, 7, 2, WHITE);
        }
        let label = |f: &mut Frame, s: &str, cx: i32, y: i32| text(f, cx - width(s) / 2, y, s);
        let field = |f: &mut Frame, r: Rect, s: &str| {
            f.rect(r.0, r.1, r.2, r.3, WHITE);
            let s = clip_text(s, width, r.2 - 8);
            text(f, r.0 + r.2 / 2 - width(&s) / 2, r.1 + 4, &s);
        };
        label(f, "Esta Pasta", 219, 122);
        field(f, FOLDER_FIELD, if self.outside { OUTSIDE } else { INSIDE });
        label(f, "Conteúdo", 219, 162);
        f.rect(LIST.0, LIST.1, LIST.2, LIST.3, WHITE);
        for row in 0..ROWS {
            let k = self.scroll + row;
            let Some(e) = self.entries.get(k) else { break };
            let y = LIST.1 + 3 + LINE_H * row as i32;
            if self.selected == Some(k) {
                f.rect(LIST.0 + 1, y - 1, LIST.2 - 2, LINE_H, SELECTED);
            }
            let x = match e.kind {
                Kind::Sound(_) => {
                    // nota musical
                    f.rect(LIST.0 + 6, y + 8, 5, 4, BLACK);
                    f.rect(LIST.0 + 10, y + 1, 1, 9, BLACK);
                    f.rect(LIST.0 + 10, y + 1, 4, 2, BLACK);
                    LIST.0 + 20
                }
                _ => LIST.0 + 4,
            };
            let title = clip_text(&e.title, width, LIST.0 + LIST.2 - 4 - x);
            text(f, x, y + 2, &title);
        }
        let masked = |f: &mut Frame, r: Rect, img: u16, mask: u16| {
            f.blit_masked(&a.image(&pic(img)), &a.image(&pic(mask)), r.0, r.1);
        };
        masked(f, FOLDER_PREV, 0x0325, 0x03ad);
        masked(f, FOLDER_NEXT, 0x03af, 0x03b1);
        masked(f, LIST_UP, 0x03b3, 0x03b5);
        masked(f, LIST_DOWN, 0x03b7, 0x03b9);
        masked(f, DOG_BTN, 0x025d, 0x0263);
        masked(f, CANCEL_BTN, 0x03bb, 0x03bd);
        masked(f, OK_BTN, 0x03bf, 0x03c1);
        // metade direita: o som escolhido
        let (title, folder) = match self.selected() {
            Some(e) => (e.title.clone(), e.folder.clone()),
            None => (String::new(), String::new()),
        };
        if self.selected().is_some() {
            f.blit_masked(
                &a.image(&pic(0x0265)),
                &a.image(&pic(0x0267)),
                ICON.0,
                ICON.1,
            );
        }
        label(f, "Título", 422, 282);
        field(f, TITLE_FIELD, &title);
        label(f, "Artista", 422, 320);
        field(f, ARTIST_FIELD, &folder);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(r: Rect) -> (i32, i32) {
        (r.0 + r.2 / 2, r.1 + r.3 / 2)
    }

    fn row(k: usize) -> (i32, i32) {
        (LIST.0 + 30, LIST.1 + 3 + LINE_H * k as i32 + 4)
    }

    #[test]
    fn lists_own_sounds_then_music_and_back() {
        let d = tempfile::tempdir().unwrap();
        let draw = d.path().join("desenhos");
        std::fs::create_dir_all(draw.join("sons")).unwrap();
        std::fs::write(draw.join("b.WAV"), b"x").unwrap();
        std::fs::write(draw.join("nota.txt"), b"x").unwrap();
        std::fs::write(draw.join("sons").join("Meu som.wav"), b"x").unwrap();
        std::fs::write(draw.join("sons").join(".gravacao-1.wav"), b"x").unwrap();
        let music = d.path().join("Música");
        std::fs::create_dir_all(music.join("a").join("b")).unwrap();
        std::fs::write(music.join("a").join("b").join("galo.wav"), b"x").unwrap();
        std::fs::write(music.join("x.mp3"), b"x").unwrap();
        let mut f = Finder::new(Some(&draw), vec![music.clone()]);
        let titles: Vec<&str> = f.entries().iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["b", "Meu som", OUTSIDE]);
        // clicar num som seleciona e toca
        let (x, y) = row(1);
        let r = f.click(x, y);
        assert_eq!(
            r.fx,
            vec![Effect::PlaySound(SoundRef::File(
                draw.join("sons").join("Meu som.wav")
            ))]
        );
        assert_eq!(f.selected().unwrap().folder, "sons");
        // fora de Imaginópolis
        let (x, y) = row(2);
        f.click(x, y);
        assert!(f.outside());
        let titles: Vec<&str> = f.entries().iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["galo", INSIDE]);
        // JÁ sem nada escolhido não fecha
        let (x, y) = center(OK_BTN);
        assert!(!f.click(x, y).close);
        let (x, y) = row(0);
        f.click(x, y);
        let (x, y) = center(OK_BTN);
        let r = f.click(x, y);
        assert!(r.close);
        assert_eq!(
            r.chosen,
            Some(Hand::Sound {
                name: "galo".into(),
                sound: SoundRef::File(music.join("a").join("b").join("galo.wav"))
            })
        );
        // X fecha sem escolher
        let (x, y) = center(CANCEL_BTN);
        let r = f.click(x, y);
        assert!(r.close && r.chosen.is_none());
    }

    #[test]
    fn user_dirs_music_line() {
        let home = Path::new("/home/ana");
        assert_eq!(
            parse_user_dirs("# x\nXDG_MUSIC_DIR=\"$HOME/Música\"\n", Some(home)),
            Some(PathBuf::from("/home/ana/Música"))
        );
        assert_eq!(
            parse_user_dirs("XDG_MUSIC_DIR=\"/mnt/som\"", None),
            Some(PathBuf::from("/mnt/som"))
        );
        assert_eq!(parse_user_dirs("XDG_MUSIC_DIR=\"rel\"", None), None);
        assert_eq!(parse_user_dirs("", None), None);
    }

    #[test]
    fn clip_text_adds_ellipsis() {
        let w = |s: &str| s.chars().count() as i32;
        assert_eq!(clip_text("abc", w, 5), "abc");
        assert_eq!(clip_text("abcdef", w, 4), "abc…");
    }
}
