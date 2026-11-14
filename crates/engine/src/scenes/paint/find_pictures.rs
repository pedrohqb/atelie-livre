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
use crate::scenes::finder_dialog::{self as dialog, Entry, FileList, Hit, Kind, stem};
use crate::paint::drawings::{list_drawings, list_samples, mag_meta, shrink};
use crate::scenes::stickers::USER_PREFIX;
use crate::ui::{Pen, Rect, blit_canvas, inside};
use std::path::{Path, PathBuf};

/// Prévia (1/4 do quadro) na metade direita.
const PREVIEW: Rect = (349, 130, 149, 87);
/// Pergunta "Isso pode levar algum tempo…" e suas duas opções.
const PROMPT: Rect = (240, 182, 162, 112);
const PROMPT_YES: Rect = (244, 238, 154, 28);
const PROMPT_NO: Rect = (244, 270, 154, 20);
/// Balão "Cachorrinho" (`WHAT 1005`, 12) e dica "…então, clique no lugar" (3).
const HELP_DOG: usize = 12;
const TIP_PICK: usize = 3;

pub struct FindPictures {
    list: FileList,
    /// Miniatura do desenho escolhido.
    preview: Option<Canvas>,
    /// Listando os desenhos de exemplo ("fora de Imaginópolis").
    outside: bool,
    /// Pergunta "Isso pode levar algum tempo…" aberta.
    prompt: bool,
}

/// Entradas da pasta de desenhos: cada desenho e, no fim, "Fora de Imaginópolis".
pub fn own_entries(dir: Option<&Path>, artist: &str) -> Vec<Entry> {
    let mut v: Vec<Entry> = dir
        .map(list_drawings)
        .unwrap_or_default()
        .into_iter()
        .map(|p| Entry::file(p.clone(), stem(&p), artist.to_string()))
        .collect();
    v.push(Entry::outside());
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
            let (title, author) = mag_meta(&p).unwrap_or_default();
            let title = if title.trim().is_empty() {
                stem(&p)
            } else {
                title
            };
            let artist = if author.trim().is_empty() {
                folder
            } else {
                author
            };
            Entry::file(p, title, artist)
        })
        .collect();
    v.push(Entry::back());
    v
}

impl FindPictures {
    pub fn new(entries: Vec<Entry>) -> FindPictures {
        FindPictures {
            list: FileList::new(entries),
            preview: None,
            outside: false,
            prompt: false,
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

    pub fn prompting(&self) -> bool {
        self.prompt
    }

    fn select(&mut self, k: usize, pal: &[Rgb]) {
        self.list.select(k);
        self.preview = self.list.entries()[k]
            .path()
            .and_then(|p| load_picture(p, pal))
            .map(|c| shrink(&c, 4));
    }

    fn swap_entries(&mut self, entries: Vec<Entry>, outside: bool) {
        self.list.set_entries(entries);
        self.outside = outside;
        self.preview = None;
        self.prompt = false;
    }
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
        match self.list.hit(x, y, true) {
            Some(Hit::Cancel) => {
                fx.push(sfx::play(sfx::OPTION));
                Outcome::Cancel
            }
            Some(Hit::Ok) => {
                fx.push(sfx::play(sfx::OPTION));
                match self.selected() {
                    Some(Entry {
                        kind: Kind::File(p),
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
                }
            }
            Some(Hit::Dog) => {
                fx.push(sfx::play(sfx::TOOLS[2]));
                Outcome::Folder(self.outside)
            }
            Some(Hit::Folder) => {
                fx.push(sfx::play(sfx::PAGE_ARROW));
                Outcome::Folder(!self.outside)
            }
            Some(h @ (Hit::ListUp | Hit::ListDown)) => {
                fx.push(sfx::play(sfx::PAGE_ARROW));
                self.list.scroll_by(if h == Hit::ListUp { -1 } else { 1 });
                Outcome::Nothing
            }
            Some(h @ (Hit::TitlePrev | Hit::TitleNext)) => {
                fx.push(sfx::play(sfx::PAGE_ARROW));
                if let Some(k) = self.list.step(h == Hit::TitleNext) {
                    self.select(k, pal);
                }
                Outcome::Nothing
            }
            Some(Hit::Row(k)) => {
                fx.push(sfx::play(sfx::OPTION));
                match self.list.entries()[k].kind {
                    Kind::Outside => {
                        self.list.select(k);
                        self.preview = None;
                        self.prompt = true;
                    }
                    Kind::Back => return Outcome::Folder(false),
                    Kind::File(_) => self.select(k, pal),
                }
                Outcome::Nothing
            }
            None => Outcome::Nothing,
        }
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
        gray_region(f, (0, 0, f.width() as i32, f.height() as i32));
        dialog::draw_left(f, a, &fd.list, fd.outside, |f, y| {
            // quadrinho de figura
            let x = dialog::LIST.0;
            f.frame_rect(x + 4, y + 2, 12, 9, 1, BLACK);
            f.rect(x + 6, y + 6, 8, 3, dialog::GREEN);
        });
        // metade direita
        if let Some(t) = &fd.preview {
            f.rect(
                PREVIEW.0 - 1,
                PREVIEW.1 - 1,
                t.w as i32 + 2,
                t.h as i32 + 2,
                BLACK,
            );
            blit_canvas(f, t, &self.pal, PREVIEW.0, PREVIEW.1);
        }
        let (title, artist) = match fd.selected() {
            Some(e) => (e.title.clone(), e.artist.clone()),
            None => (
                String::new(),
                fd.entries()
                    .first()
                    .map(|e| e.artist.clone())
                    .unwrap_or_default(),
            ),
        };
        dialog::draw_right(f, a, &title, &artist, true);
        if fd.prompt {
            let pen = Pen::new(a);
            let font = pen.font();
            let text = |f: &mut Frame, x: i32, y: i32, s: &str, c: [u8; 3]| pen.text(f, x, y, s, c);
            crate::balloon::draw_box(f, PROMPT.0, PROMPT.1, PROMPT.2, PROMPT.3);
            let mut y = PROMPT.1 + 6;
            for line in crate::balloon::wrap_opt(
                font,
                "Isso pode levar algum tempo. Você quer mesmo procurar fora de Imaginópolis?",
                (PROMPT.2 - 12) as u32,
            ) {
                text(f, PROMPT.0 + 6, y, &line, BLACK);
                y += crate::balloon::line_height(font);
            }
            let option = |f: &mut Frame, r: Rect, s: &str| {
                crate::balloon::draw_bullet(f, r.0 + 6, r.1 + 6);
                let mut y = r.1;
                for line in crate::balloon::wrap_opt(font, s, (r.2 - 16) as u32) {
                    text(f, r.0 + 16, y, &line, BLACK);
                    y += crate::balloon::line_height(font);
                }
            };
            option(f, PROMPT_YES, "Sim, continue procurando");
            option(f, PROMPT_NO, "Não, pare de procurar");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str) -> Entry {
        Entry::file(
            PathBuf::from(format!("/x/{title}.png")),
            title.into(),
            "ana".into(),
        )
    }

    #[test]
    fn title_arrows_step_through_the_list() {
        let mut fd = FindPictures::new((0..12).map(|k| entry(&format!("d{k}"))).collect());
        let (x, y) = crate::ui::center(dialog::TITLE_PREV);
        let mut fx = Vec::new();
        fd.click(x, y, &[], &mut fx);
        assert_eq!(fd.selected().unwrap().title, "d11");
        assert_eq!(fd.list.scroll(), 4);
        let (x, y) = crate::ui::center(dialog::TITLE_NEXT);
        fd.click(x, y, &[], &mut fx);
        assert_eq!(fd.selected().unwrap().title, "d0");
        assert_eq!(fd.list.scroll(), 0);
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
}
