//! Galeria: os desenhos guardados (PNG e `.MAG` na pasta de desenhos), em
//! miniaturas, e, numa segunda seção, os desenhos de exemplo da instalação
//! (`SHARED/USERS/*/*.MAG`); clicar num deles o abre na pintura. Tela
//! remontada (o original tem uma galeria própria no saguão); fundo da parede
//! do Ateliê.

use crate::assets::{Assets, ImageRef, Program};
use crate::paint::store::{LAST_NAME, load_picture, save_png};
use crate::paint::{CANVAS_H, CANVAS_W, Canvas};
use crate::paths::find_ci;
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
const BLUE: [u8; 3] = [40, 80, 200];

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
    /// Desenhos da criança; vêm primeiro, nas suas próprias páginas.
    own: Vec<Entry>,
    /// Desenhos de exemplo da instalação; segunda seção.
    samples: Vec<Entry>,
    page: usize,
}

fn has_ext(p: &Path, ext: &str) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// Desenhos da pasta (`.png` e `.mag`), mais recentes primeiro (`ultimo.png`
/// fora, pois repete o desenho mais recente; `*.fundo.png` fora, pois é o
/// fundo sem os objetos). Um `.mag` fica de fora quando há um `.png` de mesmo
/// nome: são o mesmo desenho gravado duas vezes.
pub fn list_drawings(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let all: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    let is_background = |p: &Path| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".fundo.png"))
    };
    let mut files: Vec<PathBuf> = all
        .iter()
        .filter(|p| {
            (has_ext(p, "png")
                && p.file_name().is_some_and(|n| n != LAST_NAME)
                && !is_background(p))
                || (has_ext(p, "mag") && !all.contains(&p.with_extension("png")))
        })
        .cloned()
        .collect();
    files.sort();
    files.reverse();
    files
}

/// Desenhos de exemplo da instalação: `SHARED/USERS/*/*.MAG`, em ordem.
pub fn list_samples(data_dir: &Path) -> Vec<PathBuf> {
    let Some(users) = find_ci(data_dir, "SHARED/USERS") else {
        return Vec::new();
    };
    let Ok(rd) = std::fs::read_dir(&users) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let mut files = Vec::new();
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        let mut here: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| has_ext(p, "mag"))
            .collect();
        here.sort();
        files.extend(here);
    }
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
        let load = |paths: Vec<PathBuf>| -> Vec<Entry> {
            paths
                .into_iter()
                .filter_map(|path| {
                    let c = load_picture(&path, &pal)?;
                    Some(Entry {
                        path,
                        thumb: thumbnail(&c),
                    })
                })
                .collect()
        };
        let own = load(dir.as_deref().map(list_drawings).unwrap_or_default());
        let samples = load(list_samples(assets.data_dir()));
        Gallery {
            common: KidCommon::new(None),
            dir,
            pal,
            own,
            samples,
            page: 0,
        }
    }

    /// Desenhos da criança mais exemplos.
    pub fn count(&self) -> usize {
        self.own.len() + self.samples.len()
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    pub fn page(&self) -> usize {
        self.page
    }

    /// Páginas da primeira seção; os exemplos começam numa página nova.
    fn own_pages(&self) -> usize {
        self.own.len().div_ceil(PER_PAGE)
    }

    fn pages(&self) -> usize {
        (self.own_pages() + self.samples.len().div_ceil(PER_PAGE)).max(1)
    }

    /// Entradas da página atual e se ela é de exemplos.
    fn page_entries(&self) -> (&[Entry], bool) {
        let own_pages = self.own_pages();
        if self.page < own_pages {
            let from = self.page * PER_PAGE;
            (&self.own[from..self.own.len().min(from + PER_PAGE)], false)
        } else {
            let from = (self.page - own_pages) * PER_PAGE;
            let to = self.samples.len().min(from + PER_PAGE);
            (&self.samples[from.min(to)..to], true)
        }
    }

    fn cell_rect(k: usize) -> (i32, i32, i32, i32) {
        (
            GRID_X + CELL_W * (k % COLS) as i32,
            GRID_Y + CELL_H * (k / COLS) as i32,
            THUMB_W + 4,
            THUMB_H + 4,
        )
    }

    /// Copia o desenho escolhido (da página atual) para `ultimo.png` e vai para a pintura.
    fn open(&self, k: usize) -> Vec<Effect> {
        let (Some(dir), Some(e)) = (&self.dir, self.page_entries().0.get(k)) else {
            return Vec::new();
        };
        match load_picture(&e.path, &self.pal) {
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
                fx.extend(self.open(k));
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
        let (entries, is_sample) = self.page_entries();
        let title = if self.count() == 0 {
            "Galeria: ainda não há desenhos guardados".to_string()
        } else if is_sample {
            format!(
                "Exemplos do Fine Artist: {} desenho(s), página {} de {}",
                self.samples.len(),
                self.page + 1,
                self.pages()
            )
        } else {
            format!(
                "Galeria: {} desenho(s), página {} de {}",
                self.own.len(),
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
        for (k, e) in entries.iter().enumerate() {
            let r = Self::cell_rect(k);
            frame.rect(r.0, r.1, r.2, r.3, if is_sample { BLUE } else { BLACK });
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
    use crate::paint::store::{load_png, save_drawing};
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
        assert_eq!(g.sample_count(), 0);
        assert_eq!(list_drawings(dir.path()).len(), 2);
        assert_eq!(g.own[1].thumb.px[0], 255);
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
    fn lists_mag_files_without_png_twin_and_opens_them() {
        use crate::paint::store::save_mag;
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = &a.palettes(Program::Artist).pal256;
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(9);
        // gravação normal: .png + .mag do mesmo desenho → uma entrada só
        save_drawing(dir.path(), &c, pal, true, 1_793_404_800).unwrap();
        // um .mag solto (vindo de fora) → entra
        c.px.fill(7);
        save_mag(&c, pal, &dir.path().join("Casa.MAG")).unwrap();
        // um .mag de outro tamanho não abre e fica de fora
        save_mag(&Canvas::new(3, 3), pal, &dir.path().join("mini.mag")).unwrap();
        let files = list_drawings(dir.path());
        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|p| p.ends_with("Casa.MAG")));
        assert!(
            !files
                .iter()
                .any(|p| p.ends_with("desenho-20261031-000000.mag"))
        );
        let mut g = Gallery::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(g.count(), 2);
        let k = g
            .own
            .iter()
            .position(|e| e.path.ends_with("Casa.MAG"))
            .unwrap();
        assert_eq!(g.own[k].thumb.px[0], 7);
        let r = Gallery::cell_rect(k);
        let fx = g.update(&[click(r.0 + 5, r.1 + 5)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        let last = load_png(&dir.path().join(LAST_NAME), pal).unwrap();
        assert_eq!(last.px[0], 7);
    }

    #[test]
    fn samples_come_in_a_second_section() {
        let (d, a) = empty_assets();
        let pal = &a.palettes(Program::Artist).pal256;
        let users = d.path().join("shared/users/maggie");
        std::fs::create_dir_all(&users).unwrap();
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(5);
        crate::paint::store::save_mag(&c, pal, &users.join("SAMP1.MAG")).unwrap();
        crate::paint::store::save_mag(&c, pal, &users.join("samp2.mag")).unwrap();
        std::fs::write(users.join("SAMP1.PTM"), b"nada").unwrap();
        assert_eq!(list_samples(d.path()).len(), 2);
        assert!(list_samples(&d.path().join("nao-existe")).is_empty());

        let dir = tempfile::tempdir().unwrap();
        let own = Canvas::new(CANVAS_W, CANVAS_H);
        save_drawing(dir.path(), &own, pal, true, 1_793_404_800).unwrap();
        let mut g = Gallery::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!((g.count(), g.sample_count()), (3, 2));
        assert_eq!(g.pages(), 2);
        assert_eq!(g.page_entries().0.len(), 1);
        assert!(!g.page_entries().1);
        let mut f = Frame::new(640, 480);
        g.draw(&mut f, &a);
        g.update(&[click(NEXT.0 + 5, NEXT.1 + 5)], 0.0, &a);
        assert_eq!(g.page(), 1);
        let (entries, is_sample) = g.page_entries();
        assert_eq!(entries.len(), 2);
        assert!(is_sample);
        g.draw(&mut f, &a);
        // abrir um exemplo copia para ultimo.png
        let fx = g.update(&[click(GRID_X + 10, GRID_Y + 10)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        let last = crate::paint::store::load_png(&dir.path().join(LAST_NAME), pal).unwrap();
        assert_eq!(last.px[0], 5);
        // sem pasta de desenhos, os exemplos aparecem mas não abrem
        let mut g = Gallery::new(None, &a);
        assert_eq!((g.count(), g.sample_count()), (2, 2));
        assert!(g.page_entries().1);
        g.update(&[], 0.0, &a); // primeiro update manda o cursor
        assert!(
            g.update(&[click(GRID_X + 10, GRID_Y + 10)], 0.0, &a)
                .is_empty()
        );
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
