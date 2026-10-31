//! Ferramenta Seleciona-sons da tela de pintura: categorias (carrosséis
//! `CSRO` de `MAG100.SND`) na faixa, lista de nomes à esquerda e "pontos de
//! som" colocados no quadro, que tocam ao serem clicados. Os pontos ficam num
//! arquivo `ultimo.sons.json` ao lado do desenho. Remontagem aproximada: no
//! original o som se associa a adesivos e palavras.

use crate::Frame;
use crate::assets::{Assets, ItemKind, SoundRef};
use crate::balloon::wrap;
use crate::paint::{CANVAS_H, CANVAS_W, ORIGIN};

type Rect = (i32, i32, i32, i32);

/// Retângulo do quadro na tela.
const ORIGIN_RECT: Rect = (ORIGIN.0, ORIGIN.1, CANVAS_W as i32, CANVAS_H as i32);
use formats::chnk;
use std::path::Path;

const LIBRARY: &str = "MAG100.SND";
pub const SPOTS_NAME: &str = "ultimo.sons.json";
const STRIP_Y: i32 = 60;
const BANNER_W: i32 = 118;
const BANNER_H: i32 = 36;
const BANNER_X0: i32 = 10;
const LIST_X: i32 = 60;
const LIST_Y: i32 = 112;
const LIST_W: i32 = 90;
const LINE_H: i32 = 15;
const LIST_ROWS: usize = 14;
const HIT_R: i32 = 10;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const YELLOW: [u8; 3] = [255, 238, 76];
const BLUE: [u8; 3] = [97, 65, 219];

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

/// Uma categoria: nome e sons (nome, referência).
#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub sounds: Vec<(String, SoundRef)>,
}

/// Ponto de som colocado no quadro (coordenadas do quadro).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    pub x: i32,
    pub y: i32,
    pub name: String,
    pub sound: SoundRef,
}

/// Lê as categorias de `MAG100.SND`: `CSRO` (lista de ids `SND`) + nomes.
pub fn load_categories(assets: &Assets) -> Vec<Category> {
    // sons por id de chunk, na ordem do catálogo (= ordem dos chunks no arquivo)
    let Some(src) = assets
        .catalog()
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(LIBRARY))
    else {
        return Vec::new();
    };
    let mut by_id = std::collections::HashMap::new();
    for item in &src.items {
        if let ItemKind::Sound(r) = &item.kind {
            // rótulo "SND  n nome" (a etiqueta tem um espaço no fim)
            let mut parts = item.label.split_whitespace();
            let (Some(tag), Some(id)) = (parts.next(), parts.next()) else {
                continue;
            };
            if tag != "SND" {
                continue;
            }
            let name = parts.collect::<Vec<_>>().join(" ");
            if let Ok(id) = id.parse::<u16>() {
                by_id.insert(id, (name, r.clone()));
            }
        }
    }
    let Some(data) = assets.library_bytes(LIBRARY) else {
        return Vec::new();
    };
    let Ok(f) = chnk::parse(data) else {
        return Vec::new();
    };
    let mut cats: Vec<(u16, Category)> = f
        .chunks
        .iter()
        .filter(|c| &c.tag.0 == b"CSRO")
        .filter_map(|c| {
            let list = c.parts.iter().find(|p| &p.tag.0 == b"CSRO")?;
            let ids = crate::clipart::parse_list(list.data);
            let sounds: Vec<(String, SoundRef)> =
                ids.iter().filter_map(|id| by_id.get(id).cloned()).collect();
            (!sounds.is_empty()).then(|| {
                (
                    c.id,
                    Category {
                        name: c.name.clone().unwrap_or_default(),
                        sounds,
                    },
                )
            })
        })
        .collect();
    cats.sort_by_key(|(id, _)| *id);
    cats.into_iter().map(|(_, c)| c).collect()
}

pub struct Sounds {
    cats: Vec<Category>,
    cat: usize,
    list_open: bool,
    scroll: usize,
    /// Som escolhido (categoria, índice), a colocar no quadro.
    chosen: Option<(usize, usize)>,
    pub spots: Vec<Spot>,
    /// Ponto tocado há pouco, para o realce (índice, tempo restante).
    flash: Option<(usize, f32)>,
}

impl Sounds {
    pub fn new(assets: &Assets) -> Sounds {
        Sounds {
            cats: load_categories(assets),
            cat: 0,
            list_open: false,
            scroll: 0,
            chosen: None,
            spots: Vec::new(),
            flash: None,
        }
    }

    pub fn categories(&self) -> usize {
        self.cats.len()
    }

    pub fn list_open(&self) -> bool {
        self.list_open
    }

    pub fn chosen(&self) -> Option<(usize, usize)> {
        self.chosen
    }

    pub fn reset(&mut self) {
        self.list_open = false;
        self.chosen = None;
    }

    pub fn update(&mut self, dt: f32) {
        if let Some((_, t)) = self.flash.as_mut() {
            *t -= dt.max(0.0);
            if *t <= 0.0 {
                self.flash = None;
            }
        }
    }

    fn banner_rect(k: usize) -> Rect {
        (
            BANNER_X0 + k as i32 * (BANNER_W + 6),
            STRIP_Y,
            BANNER_W,
            BANNER_H,
        )
    }

    fn list_rect(&self) -> Rect {
        let n = self
            .cats
            .get(self.cat)
            .map_or(0, |c| c.sounds.len())
            .min(LIST_ROWS) as i32
            + 1;
        (LIST_X, LIST_Y, LIST_W, n * LINE_H + 4)
    }

    /// Clique com a ferramenta ativa. Devolve `Some(som a tocar)` quando um
    /// ponto foi tocado, `Some(None)` quando o clique foi tratado sem som, e
    /// `None` quando não era desta ferramenta.
    pub fn click(&mut self, x: i32, y: i32) -> Option<Option<SoundRef>> {
        if self.list_open && inside(self.list_rect(), x, y) {
            let row = ((y - LIST_Y - 2) / LINE_H) as usize;
            let cat = &self.cats[self.cat];
            if row == 0 {
                // "Sem som": nada na mão
                self.chosen = None;
            } else if self.scroll + row - 1 < cat.sounds.len() {
                self.chosen = Some((self.cat, self.scroll + row - 1));
            }
            self.list_open = false;
            return Some(None);
        }
        if let Some(k) = (0..self.cats.len()).find(|&k| inside(Self::banner_rect(k), x, y)) {
            if self.list_open && k == self.cat {
                self.list_open = false;
            } else {
                self.cat = k;
                self.scroll = 0;
                self.list_open = true;
            }
            return Some(None);
        }
        if inside(ORIGIN_RECT, x, y) {
            let (cx, cy) = (x - ORIGIN_RECT.0, y - ORIGIN_RECT.1);
            // tocar um ponto existente
            if let Some(i) = self
                .spots
                .iter()
                .position(|s| (s.x - cx).abs() <= HIT_R && (s.y - cy).abs() <= HIT_R)
            {
                self.flash = Some((i, 0.6));
                return Some(Some(self.spots[i].sound.clone()));
            }
            if let Some((c, i)) = self.chosen {
                let (name, sound) = self.cats[c].sounds[i].clone();
                self.spots.push(Spot {
                    x: cx,
                    y: cy,
                    name,
                    sound: sound.clone(),
                });
                self.chosen = None;
                return Some(Some(sound));
            }
            return Some(None);
        }
        None
    }

    /// Faixa de opções: faixas com os nomes das categorias.
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        let font = a.font(8);
        for (k, c) in self.cats.iter().enumerate() {
            let r = Self::banner_rect(k);
            f.rect(r.0, r.1, r.2, r.3, BLACK);
            f.rect(
                r.0 + 1,
                r.1 + 1,
                r.2 - 2,
                r.3 - 2,
                if k == self.cat { YELLOW } else { WHITE },
            );
            f.rect(r.0 + 1, r.1 + 1, r.2 - 2, 8, BLUE);
            let label = c.name.as_str();
            match &font {
                Some(fo) => {
                    let w = fo.width(label) as i32;
                    f.text_font(fo, r.0 + r.2 / 2 - w / 2, r.1 + 14, label, BLACK);
                }
                None => f.text(r.0 + 4, r.1 + 16, label, BLACK),
            }
        }
        if self.cats.is_empty() {
            f.text(BANNER_X0, STRIP_Y + 14, "sem MAG100.SND", BLACK);
        }
    }

    /// Lista de sons aberta e pontos de som no quadro.
    pub fn draw_overlay(&self, f: &mut Frame, a: &Assets) {
        let font = a.font(8);
        for (i, s) in self.spots.iter().enumerate() {
            let (x, y) = (ORIGIN_RECT.0 + s.x, ORIGIN_RECT.1 + s.y);
            let hot = self.flash.is_some_and(|(k, _)| k == i);
            // nota musical: bolinha com haste
            let c = if hot { BLUE } else { BLACK };
            for dy in -3..=3 {
                for dx in -4..=4 {
                    if dx * dx + dy * dy * 2 <= 16 {
                        f.put(x + dx, y + 4 + dy, c);
                    }
                }
            }
            f.rect(x + 3, y - 8, 2, 13, c);
            f.rect(x + 4, y - 8, 5, 2, c);
        }
        if !self.list_open {
            return;
        }
        let Some(cat) = self.cats.get(self.cat) else {
            return;
        };
        let r = self.list_rect();
        f.rect(r.0, r.1, r.2, r.3, BLACK);
        f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, WHITE);
        let mut names: Vec<&str> = vec!["Sem som"];
        names.extend(
            cat.sounds
                .iter()
                .skip(self.scroll)
                .take(LIST_ROWS)
                .map(|(n, _)| n.as_str()),
        );
        for (row, name) in names.iter().enumerate() {
            let y = r.1 + 2 + row as i32 * LINE_H;
            if row > 0 {
                f.rect(r.0 + 1, y, r.2 - 2, 1, BLACK);
            }
            let shown = match &font {
                Some(fo) => wrap(fo, name, (LIST_W - 8) as u32)
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                None => name.to_string(),
            };
            match &font {
                Some(fo) => {
                    let w = fo.width(&shown) as i32;
                    f.text_font(fo, r.0 + r.2 / 2 - w / 2, y + 1, &shown, BLACK);
                }
                None => f.text(r.0 + 4, y + 3, &shown, BLACK),
            }
        }
    }

    /// Grava os pontos como JSON simples (sem dependência extra).
    pub fn save_spots(&self, path: &Path) -> std::io::Result<()> {
        let mut out = String::from("[\n");
        for (i, s) in self.spots.iter().enumerate() {
            let (file, index) = match &s.sound {
                SoundRef::Library { file, index } => (file.clone(), *index),
                SoundRef::Snd(_, id) => ("".to_string(), *id as usize),
            };
            out.push_str(&format!(
                "  {{\"x\": {}, \"y\": {}, \"name\": \"{}\", \"file\": \"{}\", \"index\": {}}}{}\n",
                s.x,
                s.y,
                s.name.replace('"', ""),
                file,
                index,
                if i + 1 < self.spots.len() { "," } else { "" }
            ));
        }
        out.push_str("]\n");
        if self.spots.is_empty() {
            let _ = std::fs::remove_file(path);
            return Ok(());
        }
        std::fs::write(path, out)
    }

    /// Lê os pontos gravados por `save_spots` (tolerante: linhas inválidas são puladas).
    pub fn load_spots(&mut self, path: &Path) {
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        self.spots.clear();
        for line in text.lines() {
            let line = line.trim().trim_end_matches(',');
            if !line.starts_with('{') {
                continue;
            }
            let field = |key: &str| -> Option<String> {
                let k = format!("\"{key}\": ");
                let i = line.find(&k)? + k.len();
                let rest = &line[i..];
                let v = if let Some(r) = rest.strip_prefix('"') {
                    r[..r.find('"')?].to_string()
                } else {
                    rest[..rest.find([',', '}'])?].trim().to_string()
                };
                Some(v)
            };
            let (Some(x), Some(y), Some(name), Some(file), Some(index)) = (
                field("x"),
                field("y"),
                field("name"),
                field("file"),
                field("index"),
            ) else {
                continue;
            };
            let (Ok(x), Ok(y), Ok(index)) = (x.parse(), y.parse(), index.parse::<usize>()) else {
                continue;
            };
            if file.is_empty() {
                continue;
            }
            self.spots.push(Spot {
                x,
                y,
                name,
                sound: SoundRef::Library { file, index },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake() -> Sounds {
        Sounds {
            cats: vec![Category {
                name: "Agudo".into(),
                sounds: vec![
                    (
                        "Sol3".into(),
                        SoundRef::Library {
                            file: LIBRARY.into(),
                            index: 10,
                        },
                    ),
                    (
                        "Lá4".into(),
                        SoundRef::Library {
                            file: LIBRARY.into(),
                            index: 12,
                        },
                    ),
                ],
            }],
            cat: 0,
            list_open: false,
            scroll: 0,
            chosen: None,
            spots: Vec::new(),
            flash: None,
        }
    }

    #[test]
    fn choose_place_play_and_save() {
        let mut s = fake();
        let r = Sounds::banner_rect(0);
        assert_eq!(s.click(r.0 + 5, r.1 + 5), Some(None));
        assert!(s.list_open());
        // linha 2 = segundo som
        assert_eq!(
            s.click(LIST_X + 10, LIST_Y + 2 + 2 * LINE_H + 3),
            Some(None)
        );
        assert_eq!(s.chosen(), Some((0, 1)));
        let played = s.click(ORIGIN_RECT.0 + 100, ORIGIN_RECT.1 + 100);
        assert!(matches!(
            played,
            Some(Some(SoundRef::Library { index: 12, .. }))
        ));
        assert_eq!(s.spots.len(), 1);
        // tocar o ponto de novo
        let played = s.click(ORIGIN_RECT.0 + 104, ORIGIN_RECT.1 + 97);
        assert!(matches!(played, Some(Some(_))));
        assert_eq!(s.spots.len(), 1);
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(SPOTS_NAME);
        s.save_spots(&p).unwrap();
        let mut t = fake();
        t.load_spots(&p);
        assert_eq!(t.spots, s.spots);
        // fora da ferramenta: clique na barra não é tratado
        assert_eq!(s.click(300, 30), None);
    }
}
