//! Galeria: os desenhos guardados (PNG na pasta de desenhos), em miniaturas;
//! clicar num deles o abre na pintura. Tela remontada (o original tem uma
//! galeria própria no saguão); fundo da parede do Ateliê.

use crate::assets::{Assets, ImageRef, Program};
use crate::paint::store::{LAST_NAME, load_png, save_png};
use crate::paint::{CANVAS_H, CANVAS_W, Canvas};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, SCREEN_H, SCREEN_W};
use formats::palette::Rgb;
use std::path::{Path, PathBuf};

const COLS: usize = 4;
const ROWS: usize = 3;
const PER_PAGE: usize = COLS * ROWS;
const THUMB_W: i32 = (CANVAS_W / 4) as i32;
const THUMB_H: i32 = (CANVAS_H / 4) as i32;
const GRID_X: i32 = 16;
const GRID_Y: i32 = 40;
const CELL_W: i32 = 152;
const CELL_H: i32 = 118;
const BACK: (i32, i32, i32, i32) = (16, 436, 120, 30);
const PREV: (i32, i32, i32, i32) = (420, 436, 90, 30);
const NEXT: (i32, i32, i32, i32) = (520, 436, 90, 30);

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const ORANGE: [u8; 3] = [255, 140, 50];

fn inside(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

struct Entry {
    path: PathBuf,
    thumb: Canvas,
}

pub struct Gallery {
    common: KidCommon,
    dir: Option<PathBuf>,
    pal: Vec<Rgb>,
    entries: Vec<Entry>,
    page: usize,
}

/// PNGs de desenho da pasta, mais recentes primeiro (`ultimo.png` fora, pois
/// repete o desenho mais recente).
pub fn list_drawings(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"))
                && p.file_name().is_some_and(|n| n != LAST_NAME)
        })
        .collect();
    files.sort();
    files.reverse();
    files
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

impl Gallery {
    pub fn new(dir: Option<PathBuf>, assets: &Assets) -> Gallery {
        let pal = assets.palettes(Program::Artist).pal256.clone();
        let entries = dir
            .as_deref()
            .map(list_drawings)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|path| {
                let c = load_png(&path, &pal)?;
                Some(Entry {
                    path,
                    thumb: thumbnail(&c),
                })
            })
            .collect();
        Gallery {
            common: KidCommon::new(None),
            dir,
            pal,
            entries,
            page: 0,
        }
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn page(&self) -> usize {
        self.page
    }

    fn pages(&self) -> usize {
        self.entries.len().div_ceil(PER_PAGE).max(1)
    }

    fn cell_rect(k: usize) -> (i32, i32, i32, i32) {
        (
            GRID_X + CELL_W * (k % COLS) as i32,
            GRID_Y + CELL_H * (k / COLS) as i32,
            THUMB_W + 4,
            THUMB_H + 4,
        )
    }

    /// Copia o desenho escolhido para `ultimo.png` e vai para a pintura.
    fn open(&self, k: usize) -> Vec<Effect> {
        let (Some(dir), Some(e)) = (&self.dir, self.entries.get(k)) else {
            return Vec::new();
        };
        match load_png(&e.path, &self.pal) {
            Some(c) => match save_png(&c, &self.pal, &dir.join(LAST_NAME)) {
                Ok(()) => vec![Effect::ChangeScene(SceneId::Paint)],
                Err(err) => vec![Effect::Log(format!("galeria: {err}"))],
            },
            None => vec![Effect::Log(format!(
                "galeria: não consegui abrir {}",
                e.path.display()
            ))],
        }
    }

    fn button(f: &mut Frame, r: (i32, i32, i32, i32), label: &str, a: &Assets) {
        f.rect(r.0, r.1, r.2, r.3, BLACK);
        f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, WHITE);
        match a.font(8) {
            Some(font) => {
                let w = font.width(label) as i32;
                f.text_font(&font, r.0 + r.2 / 2 - w / 2, r.1 + 8, label, BLACK);
            }
            None => f.text(r.0 + 6, r.1 + 11, label, BLACK),
        }
    }
}

impl Scene for Gallery {
    fn update(&mut self, input: &[Input], _dt: f32, _assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        for i in input {
            let Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } = *i
            else {
                continue;
            };
            if inside(BACK, x, y) {
                fx.push(Effect::ChangeScene(SceneId::Studio));
            } else if inside(PREV, x, y) {
                self.page = (self.page + self.pages() - 1) % self.pages();
            } else if inside(NEXT, x, y) {
                self.page = (self.page + 1) % self.pages();
            } else if let Some(k) = (0..PER_PAGE).find(|&k| inside(Self::cell_rect(k), x, y)) {
                fx.extend(self.open(self.page * PER_PAGE + k));
            }
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        frame.fill([0, 0, 0]);
        let bg = assets.image(&ImageRef::Pic(Program::Artist, 0x6c6b));
        frame.blit(
            &bg,
            (SCREEN_W as i32 - bg.width as i32) / 2,
            (SCREEN_H as i32 - bg.height as i32) / 2,
        );
        // faixa do título
        frame.rect(16, 8, 608, 26, BLACK);
        frame.rect(17, 9, 606, 24, ORANGE);
        let title = if self.entries.is_empty() {
            "Galeria: ainda não há desenhos guardados".to_string()
        } else {
            format!(
                "Galeria: {} desenho(s), página {} de {}",
                self.entries.len(),
                self.page + 1,
                self.pages()
            )
        };
        match assets.font(8) {
            Some(font) => {
                frame.text_font(&font, 26, 14, &title, BLACK);
            }
            None => frame.text(26, 17, &title, BLACK),
        }
        for k in 0..PER_PAGE {
            let Some(e) = self.entries.get(self.page * PER_PAGE + k) else {
                break;
            };
            let r = Self::cell_rect(k);
            frame.rect(r.0, r.1, r.2, r.3, BLACK);
            for y in 0..e.thumb.h {
                for x in 0..e.thumb.w {
                    let i = e.thumb.px[(y * e.thumb.w + x) as usize];
                    let p = self
                        .pal
                        .get(i as usize)
                        .copied()
                        .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                    frame.put(r.0 + 2 + x as i32, r.1 + 2 + y as i32, [p.r, p.g, p.b]);
                }
            }
        }
        Self::button(frame, BACK, "Voltar", assets);
        if self.pages() > 1 {
            Self::button(frame, PREV, "Anterior", assets);
            Self::button(frame, NEXT, "Próxima", assets);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::store::save_drawing;
    use crate::scenes::testutil::empty_assets;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    #[test]
    fn lists_opens_and_goes_back() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = &a.palettes(Program::Artist).pal256;
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(255);
        save_drawing(dir.path(), &c, pal, true, 1_793_404_800).unwrap();
        let c2 = Canvas::new(CANVAS_W, CANVAS_H);
        save_drawing(dir.path(), &c2, pal, true, 1_793_404_900).unwrap();
        // ultimo.png é o quadro branco; a galeria lista os dois com data, o mais novo primeiro
        let mut g = Gallery::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(g.count(), 2);
        assert_eq!(g.entries[1].thumb.px[0], 255);
        let fx = g.update(&[click(GRID_X + CELL_W + 10, GRID_Y + 10)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        // o escolhido (preto) virou o último
        let last = load_png(&dir.path().join(LAST_NAME), pal).unwrap();
        assert_eq!(last.px[0], 255);
        let fx = g.update(&[click(30, 450)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Studio)));
        let mut f = Frame::new(640, 480);
        g.draw(&mut f, &a);
    }

    #[test]
    fn empty_gallery_is_fine() {
        let (_d, a) = empty_assets();
        let mut g = Gallery::new(None, &a);
        assert_eq!(g.count(), 0);
        g.update(&[], 0.0, &a); // primeiro update manda o cursor
        assert!(
            g.update(&[click(GRID_X + 5, GRID_Y + 5)], 0.0, &a)
                .is_empty()
        );
        g.update(&[click(NEXT.0 + 5, NEXT.1 + 5)], 0.0, &a);
        assert_eq!(g.page(), 0);
    }
}
