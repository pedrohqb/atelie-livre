//! Cachorrinho da ferramenta Seleciona-adesivos: o diálogo "achar figuras".
//! No original, procura figuras na pasta do usuário (e, "fora de
//! Imaginópolis", no disco todo); aqui lista os desenhos guardados na pasta
//! de desenhos e, fora de Imaginópolis, os desenhos de exemplo da instalação
//! (`SHARED/USERS/*/*.MAG`). A figura escolhida vira um adesivo do usuário
//! (`adesivo-<nome>.png`, cortado ao que foi pintado) e vai para a mão.
//! Geometria medida no original (varredura `paint-stickers`, `dog_*.png`).

use super::sticker_easel::{canvas_to_stamp, gray_region};
use super::*;
use crate::paint::store::{load_picture, save_stamp_png};
use crate::paths::safe_component;
use crate::scenes::gallery::{list_drawings, list_samples};
use crate::scenes::stickers::USER_PREFIX;
use std::path::{Path, PathBuf};

type Rect = (i32, i32, i32, i32);

const GREEN: [u8; 3] = [71, 140, 80];
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
const TITLE_PREV: Rect = (346, 279, 14, 16);
const TITLE_NEXT: Rect = (485, 279, 14, 16);
const TITLE_FIELD: Rect = (346, 297, 153, 19);
const ARTIST_FIELD: Rect = (346, 335, 153, 19);
/// Prévia (1/4 do quadro) na metade direita.
const PREVIEW: Rect = (349, 130, 149, 87);
/// Botões: cachorro (procurar de novo), X (cancelar), JÁ (usar).
const DOG_BTN: Rect = (157, 325, 37, 32);
const CANCEL_BTN: Rect = (207, 327, 28, 28);
const OK_BTN: Rect = (251, 328, 26, 27);
/// Pergunta "Isso pode levar algum tempo…" e suas duas opções.
const PROMPT: Rect = (240, 182, 162, 112);
const PROMPT_YES: Rect = (244, 238, 154, 28);
const PROMPT_NO: Rect = (244, 270, 154, 20);
/// Balão "Cachorrinho" (`WHAT 1005`, 12) e dica "…então, clique no lugar" (3).
const HELP_DOG: usize = 12;
const TIP_PICK: usize = 3;
const OUTSIDE: &str = "Fora de Imaginópolis";
const INSIDE: &str = "Imaginópolis";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Picture(PathBuf),
    /// Entrada que leva aos desenhos de exemplo.
    Outside,
    /// Entrada que volta aos desenhos do usuário.
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    kind: Kind,
    pub title: String,
    pub artist: String,
}

pub struct FindPictures {
    entries: Vec<Entry>,
    scroll: usize,
    selected: Option<usize>,
    /// Miniatura do desenho escolhido.
    preview: Option<Canvas>,
    /// Listando os desenhos de exemplo ("fora de Imaginópolis").
    outside: bool,
    /// Pergunta "Isso pode levar algum tempo…" aberta.
    prompt: bool,
}

/// Nome do arquivo sem extensão.
fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Entradas da pasta de desenhos: cada desenho e, no fim, "Fora de Imaginópolis".
pub fn own_entries(dir: Option<&Path>, artist: &str) -> Vec<Entry> {
    let mut v: Vec<Entry> = dir
        .map(list_drawings)
        .unwrap_or_default()
        .into_iter()
        .map(|p| Entry {
            title: stem(&p),
            artist: artist.to_string(),
            kind: Kind::Picture(p),
        })
        .collect();
    v.push(Entry {
        kind: Kind::Outside,
        title: OUTSIDE.into(),
        artist: String::new(),
    });
    v
}

/// Desenhos de exemplo (`SHARED/USERS/<autor>/*.MAG`), com título e autor do
/// arquivo, e, no fim, a volta a Imaginópolis.
pub fn sample_entries(data_dir: &Path) -> Vec<Entry> {
    let mut v: Vec<Entry> = list_samples(data_dir)
        .into_iter()
        .map(|p| {
            let folder = p
                .parent()
                .and_then(|d| d.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let (title, author) = std::fs::read(&p)
                .ok()
                .and_then(|d| formats::mag::read(&d).ok())
                .map(|m| (m.title, m.author))
                .unwrap_or_default();
            Entry {
                title: if title.trim().is_empty() {
                    stem(&p)
                } else {
                    title
                },
                artist: if author.trim().is_empty() {
                    folder
                } else {
                    author
                },
                kind: Kind::Picture(p),
            }
        })
        .collect();
    v.push(Entry {
        kind: Kind::Back,
        title: INSIDE.into(),
        artist: String::new(),
    });
    v
}

impl FindPictures {
    pub fn new(entries: Vec<Entry>) -> FindPictures {
        FindPictures {
            entries,
            scroll: 0,
            selected: None,
            preview: None,
            outside: false,
            prompt: false,
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

    pub fn prompting(&self) -> bool {
        self.prompt
    }

    /// Linha da lista em `(x, y)`, se há entrada nela.
    fn row_at(&self, x: i32, y: i32) -> Option<usize> {
        if !inside(LIST, x, y) {
            return None;
        }
        let row = ((y - LIST.1 - 3) / LINE_H) as usize;
        let k = self.scroll + row;
        (row < ROWS && k < self.entries.len()).then_some(k)
    }

    fn select(&mut self, k: usize, pal: &[Rgb]) {
        self.selected = Some(k);
        self.preview = match &self.entries[k].kind {
            Kind::Picture(p) => load_picture(p, pal).map(|c| thumbnail(&c)),
            _ => None,
        };
        if k < self.scroll {
            self.scroll = k;
        } else if k >= self.scroll + ROWS {
            self.scroll = k + 1 - ROWS;
        }
    }

    fn scroll_by(&mut self, d: i32) {
        let max = self.entries.len().saturating_sub(ROWS);
        self.scroll = (self.scroll as i32 + d).clamp(0, max as i32) as usize;
    }

    fn swap_entries(&mut self, entries: Vec<Entry>, outside: bool) {
        self.entries = entries;
        self.outside = outside;
        self.scroll = 0;
        self.selected = None;
        self.preview = None;
        self.prompt = false;
    }
}

/// Reduz o quadro a 1/4 (vizinho mais próximo).
fn thumbnail(c: &Canvas) -> Canvas {
    let (w, h) = (c.w / 4, c.h / 4);
    let mut t = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            t.px[(y * w + x) as usize] = c.px[((y * 4) * c.w + x * 4) as usize];
        }
    }
    t
}

/// O que o clique no diálogo decidiu.
enum Outcome {
    Nothing,
    Cancel,
    /// Usar a figura deste caminho, com este título.
    Use(PathBuf, String),
    /// Trocar a listagem: `true` = desenhos de exemplo.
    Folder(bool),
}

impl FindPictures {
    fn click(&mut self, x: i32, y: i32, pal: &[Rgb], fx: &mut Vec<Effect>) -> Outcome {
        if self.prompt {
            if inside(PROMPT_YES, x, y) {
                fx.push(sfx::play(sfx::OPTION));
                return Outcome::Folder(true);
            }
            if inside(PROMPT_NO, x, y) {
                fx.push(sfx::play(sfx::OPTION));
                self.prompt = false;
            }
            return Outcome::Nothing;
        }
        if inside(CANCEL_BTN, x, y) {
            fx.push(sfx::play(sfx::OPTION));
            return Outcome::Cancel;
        }
        if inside(OK_BTN, x, y) {
            fx.push(sfx::play(sfx::OPTION));
            return match self.selected() {
                Some(Entry {
                    kind: Kind::Picture(p),
                    title,
                    ..
                }) => Outcome::Use(p.clone(), title.clone()),
                Some(Entry {
                    kind: Kind::Outside,
                    ..
                }) => {
                    self.prompt = true;
                    Outcome::Nothing
                }
                Some(Entry {
                    kind: Kind::Back, ..
                }) => Outcome::Folder(false),
                None => Outcome::Nothing,
            };
        }
        if inside(DOG_BTN, x, y) {
            fx.push(sfx::play(sfx::TOOLS[2]));
            return Outcome::Folder(self.outside);
        }
        if inside(FOLDER_PREV, x, y) || inside(FOLDER_NEXT, x, y) {
            fx.push(sfx::play(sfx::PAGE_ARROW));
            return Outcome::Folder(!self.outside);
        }
        if inside(LIST_UP, x, y) {
            fx.push(sfx::play(sfx::PAGE_ARROW));
            self.scroll_by(-1);
            return Outcome::Nothing;
        }
        if inside(LIST_DOWN, x, y) {
            fx.push(sfx::play(sfx::PAGE_ARROW));
            self.scroll_by(1);
            return Outcome::Nothing;
        }
        if inside(TITLE_PREV, x, y) || inside(TITLE_NEXT, x, y) {
            fx.push(sfx::play(sfx::PAGE_ARROW));
            let n = self.entries.len();
            if n > 0 {
                let cur = self.selected.unwrap_or(0);
                let k = if inside(TITLE_NEXT, x, y) {
                    (cur + 1) % n
                } else {
                    (cur + n - 1) % n
                };
                self.select(k, pal);
            }
            return Outcome::Nothing;
        }
        if let Some(k) = self.row_at(x, y) {
            fx.push(sfx::play(sfx::OPTION));
            match self.entries[k].kind {
                Kind::Outside => {
                    self.selected = Some(k);
                    self.preview = None;
                    self.prompt = true;
                }
                Kind::Back => return Outcome::Folder(false),
                Kind::Picture(_) => self.select(k, pal),
            }
        }
        Outcome::Nothing
    }
}

impl Paint {
    /// Apelido do usuário, para o campo "Artista" do Cachorrinho.
    pub fn with_user(mut self, user: &str) -> Paint {
        self.artist = Some(user.to_string());
        self
    }

    pub fn finder(&self) -> Option<&FindPictures> {
        self.finder.as_ref()
    }

    pub(super) fn open_finder(&mut self, assets: &Assets) {
        let entries = own_entries(
            self.drawings.as_deref(),
            self.artist.as_deref().unwrap_or(""),
        );
        self.finder = Some(FindPictures::new(entries));
        self.auto_help(WHAT_STICKERS, HELP_DOG, assets);
    }

    pub(super) fn finder_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(fd) = self.finder.as_mut() else {
            return;
        };
        match fd.click(x, y, &self.pal, fx) {
            Outcome::Nothing => {}
            Outcome::Cancel => {
                self.finder = None;
                self.auto_help(WHAT_STICKERS, 0, assets);
            }
            Outcome::Folder(outside) => {
                let entries = if outside {
                    sample_entries(assets.data_dir())
                } else {
                    own_entries(
                        self.drawings.as_deref(),
                        self.artist.as_deref().unwrap_or(""),
                    )
                };
                fd.swap_entries(entries, outside);
            }
            Outcome::Use(path, title) => {
                self.finder = None;
                match self.picture_sticker(&path, &title) {
                    Ok(()) => self.auto_help(WHAT_STICKERS, TIP_PICK, assets),
                    Err(msg) => {
                        fx.push(Effect::Log(msg));
                        self.auto_help(WHAT_STICKERS, 0, assets);
                    }
                }
            }
        }
    }

    /// Transforma o desenho em adesivo do usuário (gravado na pasta) e o põe na mão.
    fn picture_sticker(&mut self, path: &Path, title: &str) -> Result<(), String> {
        let c = load_picture(path, &self.pal)
            .ok_or_else(|| format!("cachorrinho: não deu para abrir {}", path.display()))?;
        let stamp =
            canvas_to_stamp(&c).ok_or_else(|| format!("cachorrinho: {title} está em branco"))?;
        let name = format!("{USER_PREFIX}{}.png", safe_component(title));
        let dir = self
            .drawings
            .as_ref()
            .ok_or_else(|| "cachorrinho: sem pasta de desenhos".to_string())?;
        std::fs::create_dir_all(dir)
            .and_then(|_| save_stamp_png(&stamp, &self.pal, &dir.join(&name)))
            .map_err(|e| format!("cachorrinho: não deu para gravar {name}: {e}"))?;
        self.tool = Tool::Stickers;
        self.stickers.hold_user(name, stamp, &self.pal);
        Ok(())
    }

    pub(super) fn finder_draw(&self, f: &mut Frame, a: &Assets) {
        let Some(fd) = &self.finder else {
            return;
        };
        let font = a.font(8);
        let text = |f: &mut Frame, x: i32, y: i32, s: &str, c: [u8; 3]| -> i32 {
            match &font {
                Some(fo) => f.text_font(fo, x, y, s, c),
                None => {
                    f.text(x, y, s, c);
                    8 * s.chars().count() as i32
                }
            }
        };
        let width = |s: &str| -> i32 {
            match &font {
                Some(fo) => fo.width(s) as i32,
                None => 8 * s.chars().count() as i32,
            }
        };
        gray_region(f, (0, 0, f.width() as i32, f.height() as i32));
        // painel
        f.rect(PANEL.0, PANEL.1, PANEL.2, PANEL.3, BLACK);
        f.rect(PANEL.0 + 5, PANEL.1 + 5, PANEL.2 - 10, PANEL.3 - 10, GREEN);
        for corner in [
            (PANEL.0, PANEL.1),
            (PANEL.0 + PANEL.2 - 6, PANEL.1),
            (PANEL.0, PANEL.1 + PANEL.3 - 6),
            (PANEL.0 + PANEL.2 - 6, PANEL.1 + PANEL.3 - 6),
        ] {
            f.rect(corner.0, corner.1, 6, 6, [96, 96, 96]);
        }
        f.rect(FILM_X, PANEL.1, FILM_W, PANEL.3, BLACK);
        for y in (PANEL.1 + 12..PANEL.1 + PANEL.3 - 8).step_by(8) {
            f.rect(FILM_X + 5, y, 7, 2, WHITE);
        }
        // metade esquerda
        let label = |f: &mut Frame, s: &str, cx: i32, y: i32| {
            let w = width(s);
            text(f, cx - w / 2, y, s, BLACK);
        };
        let field = |f: &mut Frame, r: Rect, s: &str| {
            f.rect(r.0, r.1, r.2, r.3, WHITE);
            let w = width(s);
            text(f, r.0 + r.2 / 2 - w / 2, r.1 + 4, s, BLACK);
        };
        label(f, "Esta Pasta", 219, 122);
        field(f, FOLDER_FIELD, if fd.outside { OUTSIDE } else { INSIDE });
        label(f, "Conteúdo", 219, 162);
        f.rect(LIST.0, LIST.1, LIST.2, LIST.3, WHITE);
        for row in 0..ROWS {
            let k = fd.scroll + row;
            let Some(e) = fd.entries.get(k) else { break };
            let y = LIST.1 + 3 + LINE_H * row as i32;
            if fd.selected == Some(k) {
                f.rect(LIST.0 + 1, y - 1, LIST.2 - 2, LINE_H, [200, 220, 255]);
            }
            let x = match e.kind {
                Kind::Picture(_) => {
                    // quadrinho de figura
                    f.frame_rect(LIST.0 + 4, y + 2, 12, 9, 1, BLACK);
                    f.rect(LIST.0 + 6, y + 6, 8, 3, GREEN);
                    LIST.0 + 20
                }
                _ => LIST.0 + 4,
            };
            let title = clip_text(&e.title, |s| width(s), LIST.0 + LIST.2 - 4 - x);
            text(f, x, y + 2, &title, BLACK);
        }
        let arrow = |f: &mut Frame, r: Rect, img: u16, mask: u16| {
            f.blit_masked(&a.image(&pic(img)), &a.image(&pic(mask)), r.0, r.1);
        };
        arrow(f, FOLDER_PREV, 0x0325, 0x03ad);
        arrow(f, FOLDER_NEXT, 0x03af, 0x03b1);
        arrow(f, LIST_UP, 0x03b3, 0x03b5);
        arrow(f, LIST_DOWN, 0x03b7, 0x03b9);
        arrow(f, DOG_BTN, 0x025d, 0x0263);
        arrow(f, CANCEL_BTN, 0x03bb, 0x03bd);
        arrow(f, OK_BTN, 0x03bf, 0x03c1);
        // metade direita
        if let Some(t) = &fd.preview {
            f.rect(
                PREVIEW.0 - 1,
                PREVIEW.1 - 1,
                t.w as i32 + 2,
                t.h as i32 + 2,
                BLACK,
            );
            for (k, &i) in t.px.iter().enumerate() {
                let x = PREVIEW.0 + (k as u32 % t.w) as i32;
                let y = PREVIEW.1 + (k as u32 / t.w) as i32;
                f.put(x, y, self.rgb(i));
            }
        }
        label(f, "Título", 422, 282);
        arrow(f, TITLE_PREV, 0x0325, 0x03ad);
        arrow(f, TITLE_NEXT, 0x03af, 0x03b1);
        let (title, artist) = match fd.selected() {
            Some(e) => (e.title.clone(), e.artist.clone()),
            None => (
                String::new(),
                fd.entries
                    .first()
                    .map(|e| e.artist.clone())
                    .unwrap_or_default(),
            ),
        };
        field(
            f,
            TITLE_FIELD,
            &clip_text(&title, |s| width(s), TITLE_FIELD.2 - 8),
        );
        label(f, "Artista", 422, 320);
        field(
            f,
            ARTIST_FIELD,
            &clip_text(&artist, |s| width(s), ARTIST_FIELD.2 - 8),
        );
        if fd.prompt {
            crate::balloon::draw_box(f, PROMPT.0, PROMPT.1, PROMPT.2, PROMPT.3);
            let mut y = PROMPT.1 + 6;
            for line in crate::balloon::wrap_opt(
                font.as_deref(),
                "Isso pode levar algum tempo. Você quer mesmo procurar fora de Imaginópolis?",
                (PROMPT.2 - 12) as u32,
            ) {
                text(f, PROMPT.0 + 6, y, &line, BLACK);
                y += crate::balloon::line_height(font.as_deref());
            }
            let option = |f: &mut Frame, r: Rect, s: &str| {
                crate::balloon::draw_bullet(f, r.0 + 6, r.1 + 6);
                let mut y = r.1;
                for line in crate::balloon::wrap_opt(font.as_deref(), s, (r.2 - 16) as u32) {
                    text(f, r.0 + 16, y, &line, BLACK);
                    y += crate::balloon::line_height(font.as_deref());
                }
            };
            option(f, PROMPT_YES, "Sim, continue procurando");
            option(f, PROMPT_NO, "Não, pare de procurar");
        }
    }
}

/// Corta o texto com "…" para caber em `max` pixels.
fn clip_text(s: &str, width: impl Fn(&str) -> i32, max: i32) -> String {
    if width(s) <= max {
        return s.to_string();
    }
    let mut out: String = s.to_string();
    while !out.is_empty() && width(&format!("{out}…")) > max {
        out.pop();
    }
    format!("{out}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str) -> Entry {
        Entry {
            kind: Kind::Picture(PathBuf::from(format!("/x/{title}.png"))),
            title: title.into(),
            artist: "ana".into(),
        }
    }

    #[test]
    fn rows_scroll_and_select_follow_the_list() {
        let mut fd = FindPictures::new((0..12).map(|k| entry(&format!("d{k}"))).collect());
        assert_eq!(fd.row_at(LIST.0 + 10, LIST.1 + 4), Some(0));
        assert_eq!(fd.row_at(LIST.0 + 10, LIST.1 + 4 + LINE_H * 7), Some(7));
        assert_eq!(fd.row_at(LIST.0 + 10, LIST.1 + LIST.3 + 5), None);
        fd.scroll_by(10);
        assert_eq!(fd.scroll, 4);
        fd.scroll_by(-10);
        assert_eq!(fd.scroll, 0);
        fd.select(11, &[]);
        assert_eq!(fd.scroll, 4);
        assert_eq!(fd.selected().unwrap().title, "d11");
    }

    #[test]
    fn own_entries_end_with_outside_and_use_the_artist() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("desenho-1.png"), b"x").unwrap();
        let v = own_entries(Some(dir.path()), "bia");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].title, "desenho-1");
        assert_eq!(v[0].artist, "bia");
        assert_eq!(v[1].kind, Kind::Outside);
        assert_eq!(own_entries(None, "").len(), 1);
    }

    #[test]
    fn clip_text_adds_ellipsis() {
        let w = |s: &str| 8 * s.chars().count() as i32;
        assert_eq!(clip_text("abc", w, 100), "abc");
        assert_eq!(clip_text("abcdefgh", w, 40), "abcd…");
    }
}
