//! Cachorrinho do Seleciona-sons: o diálogo verde "achar sons". No original
//! ele procura sons na pasta do usuário e, "fora de Imaginópolis", no disco
//! todo; aqui lista os `.wav` da pasta de desenhos (e da subpasta `sons`, onde
//! o Gravador guarda) e, fora de Imaginópolis, os da pasta de música do
//! usuário (`XDG_MUSIC_DIR`, `~/Música` ou `~/Music`). Clicar num som toca;
//! JÁ põe o som na mão. Geometria do diálogo medida no original (varredura
//! `paint-sounds`, `tools_o_snd_dog.png`; a mesma do Cachorrinho dos adesivos).

use super::{Hand, pic, snd};
use crate::Frame;
use crate::assets::{Assets, SoundRef};
use crate::scene::Effect;
use crate::scenes::finder_dialog::{self as dialog, BLACK, Entry, FileList, Hit, Kind, ROWS};
use std::path::{Path, PathBuf};

/// Ícone do som escolhido na metade direita (o alto-falante do Gravador).
const ICON: (i32, i32) = (402, 160);
/// Limites da busca fora de Imaginópolis.
const MAX_DEPTH: usize = 4;
const MAX_FILES: usize = 500;

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

/// Entrada de um som: título pelo nome do arquivo e, em "Artista", a pasta.
fn entry(p: &Path) -> Entry {
    let folder = p
        .parent()
        .and_then(|d| d.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    Entry::file(p.to_path_buf(), dialog::stem(p), folder)
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
    v.push(Entry::outside());
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
    v.push(Entry::back());
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
    list: FileList,
    /// Listando os sons de fora (pasta de música).
    outside: bool,
    dir: Option<PathBuf>,
    music: Vec<PathBuf>,
}

impl Finder {
    /// Diálogo com os sons da pasta de desenhos; `music` são as pastas de fora.
    pub fn new(dir: Option<&Path>, music: Vec<PathBuf>) -> Finder {
        Finder {
            list: FileList::new(own_entries(dir)),
            outside: false,
            dir: dir.map(Path::to_path_buf),
            music,
        }
    }

    pub fn entries(&self) -> &[Entry] {
        self.list.entries()
    }

    pub fn selected(&self) -> Option<&Entry> {
        self.list.selected()
    }

    pub fn outside(&self) -> bool {
        self.outside
    }

    fn swap(&mut self, outside: bool) {
        self.outside = outside;
        self.list.set_entries(if outside {
            outside_entries(&self.music)
        } else {
            own_entries(self.dir.as_deref())
        });
    }

    pub fn click(&mut self, x: i32, y: i32) -> Reply {
        let mut r = Reply::default();
        match self.list.hit(x, y, false) {
            Some(Hit::Row(k)) => match self.list.entries()[k].kind.clone() {
                Kind::File(p) => {
                    self.list.select(k);
                    r.fx.push(Effect::PlaySound(SoundRef::File(p)));
                }
                Kind::Outside => self.swap(true),
                Kind::Back => self.swap(false),
            },
            Some(Hit::Folder) => {
                self.swap(!self.outside);
                r.fx.push(snd(super::SND_PAGE));
            }
            Some(Hit::ListUp) => {
                self.list.scroll_by(-(ROWS as i32 - 1));
                r.fx.push(snd(super::SND_PAGE));
            }
            Some(Hit::ListDown) => {
                self.list.scroll_by(ROWS as i32 - 1);
                r.fx.push(snd(super::SND_PAGE));
            }
            Some(Hit::Dog) => {
                // procurar de novo (sons novos gravados ou copiados)
                self.swap(self.outside);
                r.fx.push(snd(super::SND_DOG));
            }
            Some(Hit::Cancel) => {
                r.close = true;
                r.fx.push(snd(super::SND_CANCEL));
            }
            Some(Hit::Ok) => {
                if let Some(Entry {
                    kind: Kind::File(p),
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
            Some(Hit::TitlePrev | Hit::TitleNext) | None => {}
        }
        r
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        dim(f);
        dialog::draw_left(f, a, &self.list, self.outside, |f, y| {
            // nota musical
            let x = dialog::LIST.0;
            f.rect(x + 6, y + 8, 5, 4, BLACK);
            f.rect(x + 10, y + 1, 1, 9, BLACK);
            f.rect(x + 10, y + 1, 4, 2, BLACK);
        });
        // metade direita: o som escolhido
        let (title, folder) = match self.selected() {
            Some(e) => (e.title.clone(), e.artist.clone()),
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
        dialog::draw_right(f, a, &title, &folder, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::finder_dialog::{CANCEL_BTN, INSIDE, LINE_H, LIST, OK_BTN, OUTSIDE};
    use crate::ui::center;

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
        assert_eq!(f.selected().unwrap().artist, "sons");
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
}
