//! Plaqueta do quadro da parede do Ateliê: o clique a torna um campo de
//! nome (com o balão "Se você quiser, pode digitar outro nome!", `WHAT
//! 100d[18]`); Return (ou um clique fora) troca o nome do último quadro,
//! Esc desiste. A cópia com o nome (`<nome>.png`, `.mag` com o título,
//! objetos e fundo) passa a ter o nome novo, e `ultimo.nome.txt` também.
//!
//! No original, renomear um quadro que não é do usuário (um exemplo) abre o
//! aviso do Rex "Você não pode mudar esse nome"; no port o último quadro é
//! sempre uma cópia na pasta do usuário, então o aviso não aparece.

use crate::Frame;
use crate::assets::Assets;
use crate::paint::store::{
    LAST_NAME, NAME_FILE, background_name, load_name, load_png, name_stem, objects_name,
    save_mag_titled, save_png,
};
use crate::scene::{Input, Key};
use formats::fnt::Font;
use formats::palette::Rgb;
use std::path::Path;

type Rect = (i32, i32, i32, i32);

/// Plaqueta (área clicável) e a parte branca onde o nome é escrito.
pub const LABEL: Rect = (283, 186, 158, 21);
const FIELD: Rect = (297, 189, 133, 15);
/// Maior nome aceito (caracteres).
const MAX_CHARS: usize = 40;
/// Balão [18] e onde ele fica (abaixo da plaqueta).
pub const BALLOON_POS: (i32, i32) = (264, 212);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum End {
    /// Return: troca o nome.
    Commit,
    /// Esc: desiste.
    Cancel,
}

/// Campo de nome em edição; o texto começa todo selecionado (digitar troca).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameField {
    pub text: String,
    pub selected: bool,
}

impl NameField {
    pub fn new(current: &str) -> NameField {
        NameField {
            text: current.to_string(),
            selected: true,
        }
    }

    pub fn input(&mut self, i: &Input) -> Option<End> {
        match i {
            Input::Text(t) => {
                if self.selected {
                    self.text.clear();
                    self.selected = false;
                }
                for c in t.chars().filter(|c| !c.is_control()) {
                    if self.text.chars().count() < MAX_CHARS {
                        self.text.push(c);
                    }
                }
                None
            }
            Input::Key { key, .. } => match key {
                Key::Enter => Some(End::Commit),
                Key::Escape => Some(End::Cancel),
                Key::Backspace | Key::Delete => {
                    if self.selected {
                        self.text.clear();
                        self.selected = false;
                    } else {
                        self.text.pop();
                    }
                    None
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Nome final: o digitado, sem espaços nas pontas.
    pub fn name(&self) -> String {
        self.text.trim().to_string()
    }

    pub fn draw(&self, f: &mut Frame, font: Option<&Font>) {
        let (x, y, w, h) = FIELD;
        f.rect(x, y, w, h, [255, 255, 255]);
        let tw = match font {
            Some(fo) => fo.width(&self.text) as i32,
            None => 8 * self.text.chars().count() as i32,
        };
        let tx = x + 3;
        if self.selected && !self.text.is_empty() {
            f.rect(tx - 1, y + 1, tw + 2, h - 2, [0, 0, 128]);
        }
        let color = if self.selected {
            [255, 255, 255]
        } else {
            [0, 0, 0]
        };
        match font {
            Some(fo) => {
                f.text_font(fo, tx, y + 1, &self.text, color);
            }
            None => f.text(tx, y + 4, &self.text, color),
        }
        if !self.selected {
            f.rect(tx + tw + 1, y + 2, 1, h - 4, [0, 0, 0]);
        }
    }
}

/// Troca o nome do último quadro para `new`: renomeia a cópia com o nome
/// antigo (ou cria uma a partir de `ultimo.png`), grava o `.mag` com o
/// título novo e o `ultimo.nome.txt`. Um nome vazio não muda nada.
pub fn rename_last(dir: &Path, pal: &[Rgb], new: &str) -> Result<(), String> {
    let new = new.trim();
    let Some(new_stem) = name_stem(new) else {
        return Ok(());
    };
    let fail = |e: std::io::Error, p: &Path| format!("não consegui renomear {}: {e}", p.display());
    let last = load_png(&dir.join(LAST_NAME), pal)
        .ok_or_else(|| format!("não há último quadro em {}", dir.display()))?;
    let old_stem = load_name(dir).and_then(|n| name_stem(&n));
    let files = |stem: &str| {
        [
            format!("{stem}.png"),
            format!("{stem}.mag"),
            objects_name(stem),
            background_name(stem),
        ]
    };
    match old_stem.filter(|s| *s != new_stem && dir.join(format!("{s}.png")).exists()) {
        Some(old) => {
            for (from, to) in files(&old).iter().zip(files(&new_stem).iter()) {
                let (from, to) = (dir.join(from), dir.join(to));
                if from.exists() && !to.exists() {
                    std::fs::rename(&from, &to).map_err(|e| fail(e, &from))?;
                }
            }
        }
        None => {
            let png = dir.join(format!("{new_stem}.png"));
            if !png.exists() {
                save_png(&last, pal, &png).map_err(|e| fail(e, &png))?;
            }
        }
    }
    let mag = dir.join(format!("{new_stem}.mag"));
    save_mag_titled(&last, pal, &mag, new).map_err(|e| fail(e, &mag))?;
    let name_file = dir.join(NAME_FILE);
    std::fs::write(&name_file, new).map_err(|e| fail(e, &name_file))
}

/// O balão [18] com a posição da plaqueta.
pub fn balloon(assets: &Assets, user: &str) -> crate::balloon::Balloon {
    super::room_talk::load(
        assets,
        super::room_talk::rec::NAME,
        BALLOON_POS,
        user,
        crate::balloon::Balloon {
            x: 0,
            y: 0,
            w: 194,
            text: "Se você quiser, pode digitar outro nome!\nPara apagar alguma coisa\n• Aperte Backspace.".into(),
            options: Vec::new(),
            links: Vec::new(),
            figures: Vec::new(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::objects::Objects;
    use crate::paint::store::save_drawing_named;
    use crate::paint::{CANVAS_H, CANVAS_W, Canvas};

    fn key(k: Key) -> Input {
        Input::Key {
            key: k,
            ctrl: false,
            alt: false,
        }
    }

    #[test]
    fn typing_replaces_the_selected_name() {
        let mut f = NameField::new("Fundo Lunar");
        assert_eq!(f.input(&Input::Text("Lua".into())), None);
        assert_eq!(f.text, "Lua");
        f.input(&key(Key::Backspace));
        assert_eq!(f.text, "Lu");
        assert_eq!(f.input(&key(Key::Enter)), Some(End::Commit));
        assert_eq!(f.input(&key(Key::Escape)), Some(End::Cancel));
    }

    #[test]
    fn rename_moves_the_named_copy() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let pal: Vec<Rgb> = (0..256)
            .map(|i| Rgb {
                r: i as u8,
                g: i as u8,
                b: i as u8,
            })
            .collect();
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(9);
        save_drawing_named(d, &c, &Objects::default(), &pal, true, 0, Some("Casa")).unwrap();
        assert!(d.join("Casa.png").exists());
        rename_last(d, &pal, "Casa na Árvore").unwrap();
        assert!(!d.join("Casa.png").exists());
        assert!(d.join("Casa na Árvore.png").exists());
        let mag =
            formats::mag::read(&std::fs::read(d.join("Casa na Árvore.mag")).unwrap()).unwrap();
        assert_eq!(mag.title, "Casa na Árvore");
        assert_eq!(load_name(d).as_deref(), Some("Casa na Árvore"));
        // sem cópia com nome (gravado sem nome): cria uma
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        save_drawing_named(d, &c, &Objects::default(), &pal, true, 0, None).unwrap();
        rename_last(d, &pal, "Lua").unwrap();
        assert!(d.join("Lua.png").exists() && d.join("Lua.mag").exists());
        assert_eq!(load_name(d).as_deref(), Some("Lua"));
        // nome vazio: nada muda
        rename_last(d, &pal, "  ").unwrap();
        assert_eq!(load_name(d).as_deref(), Some("Lua"));
    }
}
