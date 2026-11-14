//! Diálogo verde de achar arquivos, o "Cachorrinho": o mesmo painel no
//! Seleciona-sons (achar sons) e no Seleciona-adesivos (achar figuras).
//! Metade esquerda: "Esta Pasta" (Imaginópolis ou Fora de Imaginópolis),
//! "Conteúdo" com a lista e os botões cachorro (procurar de novo), X e JÁ;
//! metade direita: o que o chamador mostra (ícone ou prévia), "Título" e
//! "Artista". Geometria medida no original (varreduras `paint-sounds`,
//! `tools_o_snd_dog.png`, e `paint-stickers`, `dog_*.png`).
//!
//! Este módulo cuida do painel, da lista (rolagem, seleção, teste de clique)
//! e do desenho comum; cada ferramenta decide o que cada botão faz, que som
//! toca e o que aparece na metade direita. O fichário verde da maleta
//! ([`super::portfolio`]) é outro desenho (mais largo, com árvore), e só
//! compartilha os componentes de [`crate::ui`].

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::ui::{Pen, Rect, inside};
use std::path::{Path, PathBuf};

pub const BLACK: [u8; 3] = [0, 0, 0];
pub const WHITE: [u8; 3] = [255, 255, 255];
pub const GREEN: [u8; 3] = [71, 140, 80];
const SELECTED: [u8; 3] = [200, 220, 255];
const CORNER: [u8; 3] = [96, 96, 96];
/// Painel verde com borda preta de 5 px.
pub const PANEL: Rect = (120, 111, 406, 257);
/// Tira de filme vertical que separa as duas metades.
const FILM_X: i32 = 312;
const FILM_W: i32 = 18;
pub const FOLDER_FIELD: Rect = (143, 139, 153, 19);
pub const LIST: Rect = (143, 178, 153, 136);
pub const LINE_H: i32 = 17;
pub const ROWS: usize = 8;
pub const FOLDER_PREV: Rect = (142, 120, 14, 16);
pub const FOLDER_NEXT: Rect = (283, 120, 14, 16);
pub const LIST_UP: Rect = (141, 161, 16, 14);
pub const LIST_DOWN: Rect = (282, 161, 16, 14);
pub const TITLE_PREV: Rect = (346, 279, 14, 16);
pub const TITLE_NEXT: Rect = (485, 279, 14, 16);
pub const TITLE_FIELD: Rect = (346, 297, 153, 19);
pub const ARTIST_FIELD: Rect = (346, 335, 153, 19);
/// Botões: cachorro (procurar de novo), X (cancelar), JÁ (usar).
pub const DOG_BTN: Rect = (157, 325, 37, 32);
pub const CANCEL_BTN: Rect = (207, 327, 28, 28);
pub const OK_BTN: Rect = (251, 328, 26, 27);
/// Figuras (e máscaras) das setas e dos botões, em `ARTIST.PIC`.
const ARROW_LEFT: (u16, u16) = (0x0325, 0x03ad);
const ARROW_RIGHT: (u16, u16) = (0x03af, 0x03b1);
const ARROW_UP: (u16, u16) = (0x03b3, 0x03b5);
const ARROW_DOWN: (u16, u16) = (0x03b7, 0x03b9);
const DOG_PIC: (u16, u16) = (0x025d, 0x0263);
const CANCEL_PIC: (u16, u16) = (0x03bb, 0x03bd);
const OK_PIC: (u16, u16) = (0x03bf, 0x03c1);
pub const OUTSIDE: &str = "Fora de Imaginópolis";
pub const INSIDE: &str = "Imaginópolis";

/// Uma linha da lista.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    File(PathBuf),
    /// Entrada que leva para "Fora de Imaginópolis".
    Outside,
    /// Entrada que volta para Imaginópolis.
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: Kind,
    pub title: String,
    /// Mostrado em "Artista" (o autor, ou a pasta do som).
    pub artist: String,
}

impl Entry {
    pub fn file(path: PathBuf, title: String, artist: String) -> Entry {
        Entry {
            kind: Kind::File(path),
            title,
            artist,
        }
    }

    /// "Fora de Imaginópolis", no fim da lista de dentro.
    pub fn outside() -> Entry {
        Entry {
            kind: Kind::Outside,
            title: OUTSIDE.into(),
            artist: String::new(),
        }
    }

    /// "Imaginópolis", no fim da lista de fora.
    pub fn back() -> Entry {
        Entry {
            kind: Kind::Back,
            title: INSIDE.into(),
            artist: String::new(),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.kind {
            Kind::File(p) => Some(p),
            _ => None,
        }
    }
}

/// Nome do arquivo sem extensão.
pub fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// O que há no ponto clicado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// Linha da lista (índice da entrada).
    Row(usize),
    /// Uma das setas de "Esta Pasta".
    Folder,
    ListUp,
    ListDown,
    TitlePrev,
    TitleNext,
    Dog,
    Cancel,
    Ok,
}

/// A lista do diálogo: entradas, rolagem e escolha.
#[derive(Debug, Clone)]
pub struct FileList {
    entries: Vec<Entry>,
    scroll: usize,
    selected: Option<usize>,
}

impl FileList {
    pub fn new(entries: Vec<Entry>) -> FileList {
        FileList {
            entries,
            scroll: 0,
            selected: None,
        }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected(&self) -> Option<&Entry> {
        self.selected.and_then(|k| self.entries.get(k))
    }

    /// Troca as entradas (outra pasta ou procurar de novo), do começo.
    pub fn set_entries(&mut self, entries: Vec<Entry>) {
        self.entries = entries;
        self.scroll = 0;
        self.selected = None;
    }

    /// Linha da lista em `(x, y)`, se há entrada nela (a faixa de cada
    /// linha é a do realce desenhado).
    pub fn row_at(&self, x: i32, y: i32) -> Option<usize> {
        if !inside(LIST, x, y) {
            return None;
        }
        let row = ((y - LIST.1 - 2).max(0) / LINE_H) as usize;
        let k = self.scroll + row;
        (row < ROWS && k < self.entries.len()).then_some(k)
    }

    /// Escolhe a entrada `k` e rola até ela.
    pub fn select(&mut self, k: usize) {
        self.selected = Some(k);
        if k < self.scroll {
            self.scroll = k;
        } else if k >= self.scroll + ROWS {
            self.scroll = k + 1 - ROWS;
        }
    }

    /// Esquece a escolha.
    pub fn deselect(&mut self) {
        self.selected = None;
    }

    /// Rola `d` linhas, sem passar das pontas.
    pub fn scroll_by(&mut self, d: i32) {
        let max = self.entries.len().saturating_sub(ROWS);
        self.scroll = (self.scroll as i32 + d).clamp(0, max as i32) as usize;
    }

    /// Entrada anterior/seguinte (dá a volta); `None` com a lista vazia.
    pub fn step(&self, forward: bool) -> Option<usize> {
        let n = self.entries.len();
        if n == 0 {
            return None;
        }
        let cur = self.selected.unwrap_or(0);
        Some(if forward {
            (cur + 1) % n
        } else {
            (cur + n - 1) % n
        })
    }

    /// O que há em `(x, y)`; `title_arrows` liga as setas de "Título".
    pub fn hit(&self, x: i32, y: i32, title_arrows: bool) -> Option<Hit> {
        if let Some(k) = self.row_at(x, y) {
            return Some(Hit::Row(k));
        }
        let buttons = [
            (FOLDER_PREV, Hit::Folder),
            (FOLDER_NEXT, Hit::Folder),
            (LIST_UP, Hit::ListUp),
            (LIST_DOWN, Hit::ListDown),
            (DOG_BTN, Hit::Dog),
            (CANCEL_BTN, Hit::Cancel),
            (OK_BTN, Hit::Ok),
        ];
        let titles = [(TITLE_PREV, Hit::TitlePrev), (TITLE_NEXT, Hit::TitleNext)];
        buttons
            .iter()
            .chain(titles.iter().filter(|_| title_arrows))
            .find(|(r, _)| inside(*r, x, y))
            .map(|(_, h)| *h)
    }
}

fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Artist, id)
}

fn masked(f: &mut Frame, a: &Assets, r: Rect, (img, mask): (u16, u16)) {
    f.blit_masked(&a.image(&pic(img)), &a.image(&pic(mask)), r.0, r.1);
}

/// Campo branco com o texto centrado (cortado com "…").
fn field(f: &mut Frame, pen: &Pen, r: Rect, s: &str) {
    f.rect(r.0, r.1, r.2, r.3, WHITE);
    let s = pen.clip(s, r.2 - 8);
    pen.centered(f, r.0 + r.2 / 2, r.1 + 4, &s, BLACK);
}

/// Painel, tira de filme e a metade esquerda: "Esta Pasta" (`outside`
/// escolhe o nome), a lista e os botões. `icon` desenha o ícone de uma
/// linha de arquivo com o canto em `(LIST.0, y)`. O fundo apagado da tela
/// fica com o chamador.
pub fn draw_left(
    f: &mut Frame,
    a: &Assets,
    list: &FileList,
    outside: bool,
    icon: impl Fn(&mut Frame, i32),
) {
    let pen = Pen::new(a);
    f.rect(PANEL.0, PANEL.1, PANEL.2, PANEL.3, BLACK);
    f.rect(PANEL.0 + 5, PANEL.1 + 5, PANEL.2 - 10, PANEL.3 - 10, GREEN);
    for corner in [
        (PANEL.0, PANEL.1),
        (PANEL.0 + PANEL.2 - 6, PANEL.1),
        (PANEL.0, PANEL.1 + PANEL.3 - 6),
        (PANEL.0 + PANEL.2 - 6, PANEL.1 + PANEL.3 - 6),
    ] {
        f.rect(corner.0, corner.1, 6, 6, CORNER);
    }
    f.rect(FILM_X, PANEL.1, FILM_W, PANEL.3, BLACK);
    for y in (PANEL.1 + 12..PANEL.1 + PANEL.3 - 8).step_by(8) {
        f.rect(FILM_X + 5, y, 7, 2, WHITE);
    }
    pen.centered(f, 219, 122, "Esta Pasta", BLACK);
    field(
        f,
        &pen,
        FOLDER_FIELD,
        if outside { OUTSIDE } else { INSIDE },
    );
    pen.centered(f, 219, 162, "Conteúdo", BLACK);
    f.rect(LIST.0, LIST.1, LIST.2, LIST.3, WHITE);
    for row in 0..ROWS {
        let k = list.scroll + row;
        let Some(e) = list.entries.get(k) else { break };
        let y = LIST.1 + 3 + LINE_H * row as i32;
        if list.selected == Some(k) {
            f.rect(LIST.0 + 1, y - 1, LIST.2 - 2, LINE_H, SELECTED);
        }
        let x = match e.kind {
            Kind::File(_) => {
                icon(f, y);
                LIST.0 + 20
            }
            _ => LIST.0 + 4,
        };
        let title = pen.clip(&e.title, LIST.0 + LIST.2 - 4 - x);
        pen.text(f, x, y + 2, &title, BLACK);
    }
    masked(f, a, FOLDER_PREV, ARROW_LEFT);
    masked(f, a, FOLDER_NEXT, ARROW_RIGHT);
    masked(f, a, LIST_UP, ARROW_UP);
    masked(f, a, LIST_DOWN, ARROW_DOWN);
    masked(f, a, DOG_BTN, DOG_PIC);
    masked(f, a, CANCEL_BTN, CANCEL_PIC);
    masked(f, a, OK_BTN, OK_PIC);
}

/// Metade direita, abaixo da figura: "Título" (com setas, se
/// `title_arrows`) e "Artista".
pub fn draw_right(f: &mut Frame, a: &Assets, title: &str, artist: &str, title_arrows: bool) {
    let pen = Pen::new(a);
    pen.centered(f, 422, 282, "Título", BLACK);
    if title_arrows {
        masked(f, a, TITLE_PREV, ARROW_LEFT);
        masked(f, a, TITLE_NEXT, ARROW_RIGHT);
    }
    field(f, &pen, TITLE_FIELD, title);
    pen.centered(f, 422, 320, "Artista", BLACK);
    field(f, &pen, ARTIST_FIELD, artist);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::center;

    fn list(n: usize) -> FileList {
        FileList::new(
            (0..n)
                .map(|k| {
                    Entry::file(
                        PathBuf::from(format!("/x/{k}")),
                        format!("d{k}"),
                        "ana".into(),
                    )
                })
                .collect(),
        )
    }

    #[test]
    fn rows_follow_the_drawn_highlight() {
        let l = list(12);
        assert_eq!(l.row_at(LIST.0 + 10, LIST.1), Some(0));
        assert_eq!(l.row_at(LIST.0 + 10, LIST.1 + 2 + LINE_H - 1), Some(0));
        assert_eq!(l.row_at(LIST.0 + 10, LIST.1 + 2 + LINE_H), Some(1));
        assert_eq!(l.row_at(LIST.0 + 10, LIST.1 + 4 + LINE_H * 7), Some(7));
        assert_eq!(l.row_at(LIST.0 + 10, LIST.1 + LIST.3 + 5), None);
        assert_eq!(list(2).row_at(LIST.0 + 10, LIST.1 + 4 + LINE_H * 3), None);
    }

    #[test]
    fn scroll_select_and_step() {
        let mut l = list(12);
        l.scroll_by(10);
        assert_eq!(l.scroll(), 4);
        l.scroll_by(-10);
        assert_eq!(l.scroll(), 0);
        l.select(11);
        assert_eq!(l.scroll(), 4);
        assert_eq!(l.selected().unwrap().title, "d11");
        assert_eq!(l.step(true), Some(0));
        assert_eq!(l.step(false), Some(10));
        l.select(2);
        assert_eq!(l.scroll(), 2);
        l.set_entries(vec![Entry::outside()]);
        assert_eq!((l.scroll(), l.selected()), (0, None));
        assert_eq!(FileList::new(Vec::new()).step(true), None);
    }

    #[test]
    fn hit_finds_rows_and_buttons() {
        let l = list(3);
        let (x, y) = (LIST.0 + 30, LIST.1 + 3 + LINE_H + 4);
        assert_eq!(l.hit(x, y, false), Some(Hit::Row(1)));
        for (r, h) in [
            (FOLDER_PREV, Hit::Folder),
            (FOLDER_NEXT, Hit::Folder),
            (LIST_UP, Hit::ListUp),
            (LIST_DOWN, Hit::ListDown),
            (DOG_BTN, Hit::Dog),
            (CANCEL_BTN, Hit::Cancel),
            (OK_BTN, Hit::Ok),
        ] {
            let (x, y) = center(r);
            assert_eq!(l.hit(x, y, false), Some(h));
        }
        let (x, y) = center(TITLE_NEXT);
        assert_eq!(l.hit(x, y, false), None);
        assert_eq!(l.hit(x, y, true), Some(Hit::TitleNext));
        assert_eq!(l.hit(5, 5, true), None);
    }

    #[test]
    fn entries_know_their_path() {
        let e = Entry::file(
            PathBuf::from("/a/b.wav"),
            stem(Path::new("/a/b.wav")),
            String::new(),
        );
        assert_eq!(e.title, "b");
        assert_eq!(e.path(), Some(Path::new("/a/b.wav")));
        assert_eq!(Entry::outside().path(), None);
        assert_eq!(Entry::back().title, INSIDE);
    }
}
