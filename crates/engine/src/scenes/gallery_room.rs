//! Galeria do saguão como no original: três molduras na parede com os
//! desenhos, a pasta no chão com todos os trabalhos, a impressora e a
//! lixeira. Arrastar um quadro troca de moldura, vai para a impressora ou
//! para a lixeira; clique duplo abre o quadro na pintura. Medido na captura
//! `scratch/orig_gallery1.png`/`orig_gallery2.png` (fora do repositório); o
//! papel de parede de corações não está no `ARTIST.PIC` e é desenhado aqui.
//!
//! A Bolsa Mágica fica aberta no chão, como no original: a maleta vermelha
//! abre a pasta, o livro verde conta a história do McZee e a Varinha Mágica,
//! no buraco do rato, faz sair uma surpresa (`room_objects`). A pasta também
//! abre sozinha por cima das salas ([`GalleryRoom::portfolio`]).

use super::gags::{self, Character};
use super::room_objects::{self as objects, BagPart, MagicBag, Target, snd};
use crate::Frame;
use crate::anim::Animator;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::Balloon;
use crate::paint::store::{LAST_NAME, load_picture, save_png, timestamp_name};
use crate::paint::{CANVAS_H, CANVAS_W, Canvas};
use crate::rng::XorShift;
use crate::scene::{DEFAULT_CURSOR, Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use formats::palette::Rgb;
use std::path::{Path, PathBuf};

type Rect = (i32, i32, i32, i32);

/// Molduras da parede (`6c79`, máscara `6c7d`, 200×140).
pub const FRAMES: [(i32, i32); 3] = [(95, 50), (323, 50), (211, 195)];
const FRAME_IMG: (u16, u16) = (0x6c79, 0x6c7d);
/// Área do desenho dentro da moldura (1/4 do quadro) e a plaqueta do nome.
const PICTURE_OFF: (i32, i32) = (24, 25);
const PLATE: Rect = (32, 117, 140, 12);
const THUMB_W: u32 = CANVAS_W / 4;
const THUMB_H: u32 = CANVAS_H / 4;

/// Elementos fixos: (cor, máscara, x, y). A Bolsa Mágica e o que sai dela
/// (livro verde, maleta e varinha) vêm de `room_objects::GALLERY_BAG`.
const ELEMENTS: [(u16, u16, i32, i32); 5] = [
    (0x681b, 0x6821, 16, 3),    // Tchau
    (0x6c87, 0x6c89, 430, 193), // impressora no quadro da parede
    (0x6c7f, 0x6c81, 527, 276), // seta "Saguão"
    (0x683b, 0x6841, 445, 348), // buraco do rato
    (0x6c83, 0x6c85, 523, 359), // lixeira
];
const TCHAU: Rect = (16, 90, 56, 45);
const PRINTER: Rect = (430, 193, 60, 74);
const LOBBY_ARROW: Rect = (527, 276, 91, 46);
const TRASH: Rect = (523, 359, 92, 108);
const MOUSE_HOLE: Rect = (445, 348, 54, 47);
/// A maleta vermelha (a pasta de trabalhos).
#[cfg(test)]
const FOLDER: Rect = (304, 391, 99, 74);

/// Pasta aberta: painel com 4×3 miniaturas e setas.
const PANEL: Rect = (40, 40, 560, 360);
const COLS: usize = 4;
const ROWS: usize = 3;
const PER_PAGE: usize = COLS * ROWS;
const CELL: (i32, i32) = (134, 112);
const PANEL_PREV: Rect = (60, 368, 80, 24);
const PANEL_NEXT: Rect = (500, 368, 80, 24);
const PANEL_CLOSE: Rect = (280, 368, 80, 24);

/// Bloco `WHAT` das salas: boas-vindas da galeria e reciclagem.
const WHAT: u16 = 0x100d;
const WELCOME: usize = 6;
const WELCOME2: usize = 7;
const RECYCLE: usize = 73;
/// Dicas mostradas pelas opções do balão de boas-vindas.
const TIP_REORGANIZE: usize = 8;
const TIP_OPEN: usize = 9;
const TIP_PRINT: usize = 10;
const TIP_TRASH: usize = 11;
/// "Wand - library": a varinha no buraco do rato da Galeria.
const WAND_LIBRARY: usize = 57;

const DOUBLE_CLICK: f32 = 0.4;
/// Arquivo com os nomes dos quadros das três molduras.
pub const WALL_FILE: &str = "galeria.txt";

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const WALL_A: [u8; 3] = [136, 136, 230];
const WALL_B: [u8; 3] = [71, 140, 80];
const FLOOR_A: [u8; 3] = [100, 100, 100];
const BASEBOARD: (i32, i32) = (375, 395);

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn frame_rect(k: usize) -> Rect {
    (FRAMES[k].0, FRAMES[k].1, 200, 140)
}

/// Um quadro: arquivo, miniatura 1/4 e nome para a plaqueta.
pub struct Item {
    pub path: PathBuf,
    pub name: String,
    pub sample: bool,
    thumb: Canvas,
}

/// Nome para mostrar: o título do `.MAG`, ou o nome do arquivo sem o carimbo.
fn display_name(path: &Path) -> String {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mag"))
        && let Ok(d) = std::fs::read(path)
        && let Ok(p) = formats::mag::read(&d)
        && !p.title.trim().is_empty()
    {
        return p.title.trim().to_string();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // desenho-AAAAMMDD-HHMMSS → "Desenho 31/10 14:05"
    let parts: Vec<&str> = stem.splitn(3, '-').collect();
    if parts.len() == 3 && parts[1].len() == 8 && parts[2].len() >= 4 {
        let (d, t) = (parts[1], parts[2]);
        let mut kind = parts[0].to_string();
        if let Some(c) = kind.get_mut(0..1) {
            c.make_ascii_uppercase();
        }
        return format!("{kind} {}/{} {}:{}", &d[6..8], &d[4..6], &t[0..2], &t[2..4]);
    }
    stem
}

fn thumbnail(c: &Canvas) -> Canvas {
    let (w, h) = (THUMB_W, THUMB_H);
    let mut t = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            t.px[(y * w + x) as usize] = c.px[((y * 4) * c.w + x * 4) as usize];
        }
    }
    t
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Wall(usize),
    Folder(usize),
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    from: Source,
    start: (i32, i32),
    pos: (i32, i32),
    moved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ask {
    /// Boas-vindas (`100d[6]`).
    Welcome,
    /// "Outra coisa..." (`100d[7]`).
    Welcome2,
    /// Dica sem opções.
    Tip,
    /// Jogar fora o quadro (índice em `items`).
    Recycle(usize),
}

pub struct GalleryRoom {
    common: KidCommon,
    dir: Option<PathBuf>,
    pal: Vec<Rgb>,
    items: Vec<Item>,
    /// Índice em `items` do quadro de cada moldura.
    wall: [Option<usize>; 3],
    folder_open: bool,
    folder_page: usize,
    drag: Option<Drag>,
    last_click: Option<(Source, f32)>,
    clock: f32,
    balloon: Option<(Ask, Balloon)>,
    character: Option<Character>,
    mouse: (i32, i32),
    /// Só a pasta, aberta pela maleta de uma sala (por cima dela).
    overlay: bool,
    bag: MagicBag,
    surprise: Option<Animator>,
    rng: XorShift,
    wand_cursor: bool,
}

impl GalleryRoom {
    pub fn new(click: Option<SoundRef>, dir: Option<PathBuf>, assets: &Assets) -> GalleryRoom {
        let pal = assets.palettes(Program::Artist).pal256.clone();
        let mut items = Vec::new();
        let mut add = |paths: Vec<PathBuf>, sample: bool| {
            for path in paths {
                if let Some(c) = load_picture(&path, &pal) {
                    items.push(Item {
                        name: display_name(&path),
                        thumb: thumbnail(&c),
                        path,
                        sample,
                    });
                }
            }
        };
        add(
            dir.as_deref()
                .map(super::gallery::list_drawings)
                .unwrap_or_default(),
            false,
        );
        add(super::gallery::list_samples(assets.data_dir()), true);
        let wall = load_wall(dir.as_deref(), &items);
        let mut rng = XorShift::from_clock();
        let character = Character::pick(gags::MCZEE_COSTUMES, gags::MCZEE_LOBBY, &mut rng);
        let mut g = GalleryRoom {
            common: KidCommon::new(click),
            dir,
            pal,
            items,
            wall,
            folder_open: false,
            folder_page: 0,
            drag: None,
            last_click: None,
            clock: 0.0,
            balloon: None,
            character,
            mouse: (0, 0),
            overlay: false,
            bag: MagicBag::new(objects::GALLERY_BAG, true),
            surprise: None,
            rng,
            wand_cursor: false,
        };
        g.ask(Ask::Welcome, WELCOME, assets);
        g
    }

    /// Só a pasta de trabalhos, aberta pela maleta vermelha de uma sala:
    /// desenhada por cima da sala; fechar (ou clicar fora) volta a ela, e um
    /// clique duplo abre o quadro na pintura.
    pub fn portfolio(
        click: Option<SoundRef>,
        dir: Option<PathBuf>,
        assets: &Assets,
    ) -> GalleryRoom {
        let mut g = GalleryRoom::new(click, dir, assets);
        g.balloon = None;
        g.overlay = true;
        g.folder_open = true;
        g
    }

    pub fn bag(&self) -> &MagicBag {
        &self.bag
    }

    pub fn surprise_playing(&self) -> bool {
        self.surprise.is_some()
    }

    fn use_bag(&mut self, part: BagPart, fx: &mut Vec<Effect>, assets: &Assets) {
        match part {
            BagPart::Bag => {
                let open = self.bag.holding_wand || !self.bag.open;
                self.bag.holding_wand = false;
                self.bag.open = open;
                fx.push(Effect::PlaySound(snd(if open {
                    objects::SND_BAG_OPEN
                } else {
                    objects::SND_BAG_CLOSE
                })));
            }
            BagPart::Suitcase => {
                fx.push(Effect::PlaySound(snd(objects::SND_SUITCASE)));
                self.folder_open = true;
            }
            BagPart::Book => {
                fx.extend(objects::SND_BOOK.map(|id| Effect::PlaySound(snd(id))));
                fx.push(Effect::Push(SceneId::BookStory));
            }
            BagPart::Wand => {
                self.bag.holding_wand = true;
                self.bag.open = false;
                fx.push(Effect::PlaySound(snd(objects::SND_WAND_TAKE)));
                self.ask(Ask::Tip, WAND_LIBRARY, assets);
                if let Some((_, b)) = &mut self.balloon {
                    b.options.clear();
                }
            }
        }
    }

    fn sync_cursor(&mut self, fx: &mut Vec<Effect>) {
        if self.bag.holding_wand != self.wand_cursor {
            self.wand_cursor = self.bag.holding_wand;
            fx.push(Effect::SetCursor(if self.wand_cursor {
                objects::WAND_CURSOR
            } else {
                DEFAULT_CURSOR
            }));
        }
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn wall(&self) -> [Option<usize>; 3] {
        self.wall
    }

    pub fn folder_open(&self) -> bool {
        self.folder_open
    }

    fn ask(&mut self, kind: Ask, k: usize, assets: &Assets) {
        let fallback = match kind {
            Ask::Welcome => Balloon {
                x: 0,
                y: 0,
                w: 239,
                text: "Seja bem-vindo à Galeria! O que você quer fazer?".into(),
                options: vec![
                    "Reorganizar a parede".into(),
                    "Abrir um quadro".into(),
                    "Jogar fora alguns quadros".into(),
                    "Ver outros quadros".into(),
                    "Outra coisa...".into(),
                ],
            },
            Ask::Welcome2 => Balloon {
                x: 0,
                y: 0,
                w: 206,
                text: "O que você quer fazer?".into(),
                options: vec![
                    "Imprimir um desenho".into(),
                    "Voltar para outras escolhas...".into(),
                ],
            },
            Ask::Recycle(_) => Balloon {
                x: 0,
                y: 0,
                w: 256,
                text: "Você quer mesmo jogar isto fora?".into(),
                options: vec!["É, jogar fora".into(), "Não, eu quero manter!".into()],
            },
            Ask::Tip => Balloon {
                x: 0,
                y: 0,
                w: 200,
                text: String::new(),
                options: Vec::new(),
            },
        };
        let records = assets.dialogs(Program::Artist, WHAT);
        let mut b =
            Balloon::from_block(&records, k, 0, 0, super::room::DEFAULT_USER).unwrap_or(fallback);
        // as opções de disquete do original não existem aqui
        if kind == Ask::Welcome2 {
            b.options
                .retain(|o| !o.contains("disquete") && !o.contains("unidades"));
        }
        b.x = 150;
        b.y = 170;
        self.balloon = Some((kind, b));
    }

    /// Arquivo do quadro da moldura `k`.
    pub fn wall_path(&self, k: usize) -> Option<&Path> {
        self.wall[k].map(|i| self.items[i].path.as_path())
    }

    fn save_wall(&self) -> Option<Effect> {
        let dir = self.dir.as_ref()?;
        let text: String = self
            .wall
            .iter()
            .map(|w| {
                w.map(|i| self.items[i].path.to_string_lossy().to_string())
                    .unwrap_or_default()
                    + "\n"
            })
            .collect();
        std::fs::write(dir.join(WALL_FILE), text)
            .err()
            .map(|e| Effect::Log(format!("galeria: {e}")))
    }

    fn open(&self, i: usize) -> Vec<Effect> {
        let (Some(dir), Some(it)) = (&self.dir, self.items.get(i)) else {
            return Vec::new();
        };
        match load_picture(&it.path, &self.pal) {
            Some(c) => match save_png(&c, &self.pal, &dir.join(LAST_NAME)) {
                Ok(()) => vec![Effect::ChangeScene(SceneId::Paint)],
                Err(e) => vec![Effect::Log(format!("galeria: {e}"))],
            },
            None => vec![Effect::Log(format!(
                "galeria: não consegui abrir {}",
                it.path.display()
            ))],
        }
    }

    fn print(&self, i: usize) -> Effect {
        let (Some(dir), Some(it)) = (&self.dir, self.items.get(i)) else {
            return Effect::Log("imprimir: sem pasta de desenhos".into());
        };
        let Some(c) = load_picture(&it.path, &self.pal) else {
            return Effect::Log(format!(
                "imprimir: não consegui abrir {}",
                it.path.display()
            ));
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(timestamp_name(now).replace("desenho-", "impressao-"));
        if let Err(e) = save_png(&c, &self.pal, &path) {
            return Effect::Log(format!("imprimir: {e}"));
        }
        match std::process::Command::new("lp")
            .arg(&path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => Effect::Log(format!("imprimindo {}", path.display())),
            Err(_) => Effect::Log(format!(
                "sem impressora (lp): o desenho ficou em {}",
                path.display()
            )),
        }
    }

    /// Joga fora o quadro: só desenhos da criança (os exemplos ficam).
    fn recycle(&mut self, i: usize) -> Vec<Effect> {
        let Some(it) = self.items.get(i) else {
            return Vec::new();
        };
        if it.sample {
            return vec![Effect::Log(
                "os exemplos do Fine Artist não podem ir para a lixeira".into(),
            )];
        }
        let mut fx = Vec::new();
        if let Err(e) = std::fs::remove_file(&it.path) {
            fx.push(Effect::Log(format!("lixeira: {e}")));
            return fx;
        }
        // o .mag gêmeo do .png vai junto
        let twin = it.path.with_extension("mag");
        if twin != it.path {
            let _ = std::fs::remove_file(twin);
        }
        self.items.remove(i);
        for w in self.wall.iter_mut() {
            *w = match *w {
                Some(j) if j == i => None,
                Some(j) if j > i => Some(j - 1),
                other => other,
            };
        }
        fx.extend(self.save_wall());
        fx
    }

    fn folder_cell(k: usize) -> Rect {
        (
            PANEL.0 + 14 + CELL.0 * (k % COLS) as i32,
            PANEL.1 + 30 + CELL.1 * (k / COLS) as i32,
            THUMB_W as i32 + 4,
            THUMB_H as i32 + 4,
        )
    }

    fn folder_pages(&self) -> usize {
        self.items.len().div_ceil(PER_PAGE).max(1)
    }

    fn hit(&self, x: i32, y: i32) -> Option<Source> {
        if self.folder_open {
            return (0..PER_PAGE)
                .find(|&k| inside(Self::folder_cell(k), x, y))
                .map(|k| self.folder_page * PER_PAGE + k)
                .filter(|&i| i < self.items.len())
                .map(Source::Folder);
        }
        (0..3)
            .rev()
            .find(|&k| inside(frame_rect(k), x, y))
            .map(Source::Wall)
    }

    fn item_of(&self, s: Source) -> Option<usize> {
        match s {
            Source::Wall(k) => self.wall[k],
            Source::Folder(i) => Some(i),
        }
    }

    fn press(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        if let Some((kind, b)) = &self.balloon {
            let kind = *kind;
            if let Some(k) = b.hit(assets, x, y) {
                self.balloon = None;
                self.answer(kind, k, fx, assets);
                return;
            }
            let inside_balloon = b.contains(assets, x, y);
            self.balloon = None;
            // o balão da varinha deixa o clique seguir
            if inside_balloon || !self.bag.holding_wand {
                return;
            }
        }
        if self.folder_open {
            if inside(PANEL_CLOSE, x, y) {
                self.folder_open = false;
                if self.overlay {
                    fx.push(Effect::Back);
                }
                return;
            }
            if inside(PANEL_PREV, x, y) {
                self.folder_page =
                    (self.folder_page + self.folder_pages() - 1) % self.folder_pages();
                return;
            }
            if inside(PANEL_NEXT, x, y) {
                self.folder_page = (self.folder_page + 1) % self.folder_pages();
                return;
            }
        }
        if let Some(src) = self.hit(x, y) {
            if self.item_of(src).is_some() {
                self.drag = Some(Drag {
                    from: src,
                    start: (x, y),
                    pos: (x, y),
                    moved: false,
                });
            }
            return;
        }
        if self.folder_open {
            if !inside(PANEL, x, y) {
                self.folder_open = false;
                if self.overlay {
                    fx.push(Effect::Back);
                }
            }
            return;
        }
        if let Some(part) = self.bag.hit(x, y) {
            self.use_bag(part, fx, assets);
        } else if self.bag.holding_wand && inside(MOUSE_HOLE, x, y) {
            // a varinha no buraco do rato: uma surpresa, e ela volta à bolsa
            self.bag.holding_wand = false;
            fx.push(Effect::PlaySound(snd(objects::SND_WAND_HOLE)));
            match objects::GALLERY_SURPRISES.start(Target::Hole, &mut self.rng, assets) {
                Some(mut a) => {
                    fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
                    self.surprise = Some(a);
                }
                None => self.bag.open = true,
            }
        } else if inside(LOBBY_ARROW, x, y) {
            fx.push(Effect::ChangeScene(SceneId::Building));
        } else if inside(TCHAU, x, y) {
            fx.push(Effect::Quit);
        } else if let Some(c) = &self.character
            && x < 120
            && y > c.y
        {
            self.ask(Ask::Welcome, WELCOME, assets);
        }
    }

    fn release(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(d) = self.drag.take() else { return };
        let Some(i) = self.item_of(d.from) else {
            return;
        };
        if !d.moved {
            // clique: duplo abre o quadro
            if let Some((s, t)) = self.last_click
                && s == d.from
                && self.clock - t < DOUBLE_CLICK
            {
                self.last_click = None;
                fx.extend(self.open(i));
                return;
            }
            self.last_click = Some((d.from, self.clock));
            return;
        }
        if inside(PRINTER, x, y) {
            fx.push(self.print(i));
            return;
        }
        if inside(TRASH, x, y) {
            self.ask(Ask::Recycle(i), RECYCLE, assets);
            return;
        }
        if let Some(k) = (0..3).rev().find(|&k| inside(frame_rect(k), x, y)) {
            match d.from {
                Source::Wall(j) => self.wall.swap(j, k),
                Source::Folder(_) => self.wall[k] = Some(i),
            }
            fx.extend(self.save_wall());
        }
    }

    fn answer(&mut self, kind: Ask, k: usize, fx: &mut Vec<Effect>, assets: &Assets) {
        match (kind, k) {
            (Ask::Welcome, 0) => self.ask(Ask::Tip, TIP_REORGANIZE, assets),
            (Ask::Welcome, 1) => self.ask(Ask::Tip, TIP_OPEN, assets),
            (Ask::Welcome, 2) => self.ask(Ask::Tip, TIP_TRASH, assets),
            (Ask::Welcome, 3) => self.folder_open = true,
            (Ask::Welcome, _) => self.ask(Ask::Welcome2, WELCOME2, assets),
            (Ask::Welcome2, 0) => self.ask(Ask::Tip, TIP_PRINT, assets),
            (Ask::Welcome2, _) => self.ask(Ask::Welcome, WELCOME, assets),
            (Ask::Recycle(i), 0) => fx.extend(self.recycle(i)),
            _ => {}
        }
    }

    fn draw_background(f: &mut Frame) {
        for y in 0..480 {
            for x in 0..640 {
                let even = (x + y) % 2 == 0;
                let c = if y < BASEBOARD.0 {
                    if even { WALL_B } else { WALL_A }
                } else if y <= BASEBOARD.1 {
                    BLACK
                } else if even {
                    WALL_B
                } else {
                    FLOOR_A
                };
                f.put(x, y, c);
            }
        }
        // corações do papel de parede (fileiras alternadas, medidos na captura)
        const HEART: [(i32, i32); 12] = [
            (1, 0),
            (5, 0),
            (0, 1),
            (2, 1),
            (4, 1),
            (6, 1),
            (1, 2),
            (3, 2),
            (5, 2),
            (2, 3),
            (4, 3),
            (3, 4),
        ];
        let mut row = 0;
        let mut y = 7;
        while y < BASEBOARD.0 - 8 {
            let x0 = if row % 2 == 0 { 10 } else { 46 };
            let mut x = x0;
            while x < 640 {
                for (dx, dy) in HEART {
                    f.put(x + dx, y + dy, BLACK);
                }
                x += 71;
            }
            y += if row % 2 == 0 { 22 } else { 20 };
            row += 1;
        }
    }

    fn draw_thumb(&self, f: &mut Frame, t: &Canvas, x: i32, y: i32) {
        for ty in 0..t.h {
            for tx in 0..t.w {
                let p = self
                    .pal
                    .get(t.px[(ty * t.w + tx) as usize] as usize)
                    .copied()
                    .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                f.put(x + tx as i32, y + ty as i32, [p.r, p.g, p.b]);
            }
        }
    }

    fn draw_text_centered(f: &mut Frame, a: &Assets, r: Rect, s: &str) {
        match a.font(8) {
            Some(font) => {
                let mut s = s.to_string();
                if font.width(&s) as i32 > r.2 {
                    let mut chars: Vec<char> = s.chars().collect();
                    while chars.len() > 1 {
                        chars.pop();
                        let t: String = chars.iter().collect::<String>() + "…";
                        if font.width(&t) as i32 <= r.2 {
                            break;
                        }
                    }
                    s = chars.iter().collect::<String>() + "…";
                }
                let w = font.width(&s) as i32;
                f.text_font(&font, r.0 + (r.2 - w) / 2, r.1, &s, BLACK);
            }
            None => f.text(r.0 + 2, r.1 + 2, s, BLACK),
        }
    }

    fn button(f: &mut Frame, a: &Assets, r: Rect, label: &str) {
        f.rect(r.0, r.1, r.2, r.3, BLACK);
        f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, WHITE);
        Self::draw_text_centered(f, a, (r.0, r.1 + 6, r.2, r.3), label);
    }

    fn draw_folder(&self, f: &mut Frame, a: &Assets) {
        let (x, y, w, h) = PANEL;
        f.rect(x - 2, y - 2, w + 4, h + 4, BLACK);
        f.rect(x, y, w, h, [255, 238, 76]);
        let title = format!(
            "Pasta: {} quadro(s), página {} de {}",
            self.items.len(),
            self.folder_page + 1,
            self.folder_pages()
        );
        Self::draw_text_centered(f, a, (x, y + 8, w, 14), &title);
        for k in 0..PER_PAGE {
            let i = self.folder_page * PER_PAGE + k;
            let Some(it) = self.items.get(i) else { break };
            let r = Self::folder_cell(k);
            f.rect(
                r.0,
                r.1,
                r.2,
                r.3,
                if it.sample { [40, 80, 200] } else { BLACK },
            );
            self.draw_thumb(f, &it.thumb, r.0 + 2, r.1 + 2);
            Self::draw_text_centered(f, a, (r.0, r.1 + r.3 + 2, r.2, 12), &it.name);
        }
        Self::button(f, a, PANEL_CLOSE, "Fechar");
        if self.folder_pages() > 1 {
            Self::button(f, a, PANEL_PREV, "Anterior");
            Self::button(f, a, PANEL_NEXT, "Próxima");
        }
    }
}

/// Molduras guardadas em `galeria.txt`; o que faltar é preenchido com os
/// desenhos mais recentes (e depois os exemplos).
fn load_wall(dir: Option<&Path>, items: &[Item]) -> [Option<usize>; 3] {
    let mut wall = [None; 3];
    if let Some(text) = dir.and_then(|d| std::fs::read_to_string(d.join(WALL_FILE)).ok()) {
        for (k, line) in text.lines().take(3).enumerate() {
            wall[k] = items
                .iter()
                .position(|it| it.path.to_string_lossy() == line.trim());
        }
    }
    let mut next = 0;
    for w in wall.iter_mut() {
        if w.is_none() {
            while next < items.len() && wall_contains(&[*w], next) {
                next += 1;
            }
            if next < items.len() {
                *w = Some(next);
                next += 1;
            }
        }
    }
    // sem repetição quando o arquivo e o preenchimento coincidem
    if wall[0].is_some() && wall[0] == wall[1] {
        wall[1] = None;
    }
    wall
}

fn wall_contains(w: &[Option<usize>], i: usize) -> bool {
    w.contains(&Some(i))
}

impl Scene for GalleryRoom {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        self.clock += dt;
        let mut fx = self.common.update(input);
        if let Some(a) = &mut self.surprise {
            fx.extend(a.update(dt).into_iter().map(Effect::PlaySound));
            if a.is_finished() {
                self.surprise = None;
                self.bag.open = true;
            }
        }
        for i in input {
            match *i {
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.mouse = (x, y);
                    if self.surprise.is_none() {
                        self.press(x, y, &mut fx, assets);
                    }
                }
                Input::MouseMove { x, y } => {
                    self.mouse = (x, y);
                    if let Some(d) = self.drag.as_mut() {
                        d.pos = (x, y);
                        // só a pasta (por cima de uma sala): não há parede para arrastar
                        if !self.overlay && (x - d.start.0).abs() + (y - d.start.1).abs() > 4 {
                            d.moved = true;
                            // arrastando da pasta: ela se fecha para mostrar a parede
                            if matches!(d.from, Source::Folder(_)) {
                                self.folder_open = false;
                            }
                        }
                    }
                }
                Input::MouseUp {
                    button: MouseButton::Left,
                    x,
                    y,
                } => self.release(x, y, &mut fx, assets),
                _ => {}
            }
        }
        self.sync_cursor(&mut fx);
        fx
    }

    fn overlay(&self) -> bool {
        self.overlay
    }

    fn draw(&self, f: &mut Frame, a: &Assets) {
        if self.overlay {
            self.draw_folder(f, a);
            return;
        }
        Self::draw_background(f);
        for (img, mask, x, y) in ELEMENTS {
            f.blit_masked(
                &a.image(&ImageRef::Pic(Program::Artist, img)),
                &a.image(&ImageRef::Pic(Program::Artist, mask)),
                x,
                y,
            );
        }
        let frame_img = a.image(&ImageRef::Pic(Program::Artist, FRAME_IMG.0));
        let frame_mask = a.image(&ImageRef::Pic(Program::Artist, FRAME_IMG.1));
        for (k, &(x, y)) in FRAMES.iter().enumerate() {
            f.blit_masked(&frame_img, &frame_mask, x, y);
            let dragging = matches!(self.drag, Some(d) if d.moved && d.from == Source::Wall(k));
            if let Some(i) = self.wall[k]
                && !dragging
            {
                let it = &self.items[i];
                self.draw_thumb(f, &it.thumb, x + PICTURE_OFF.0, y + PICTURE_OFF.1);
                Self::draw_text_centered(
                    f,
                    a,
                    (x + PLATE.0, y + PLATE.1, PLATE.2, PLATE.3),
                    &it.name,
                );
            }
        }
        self.bag.draw(f, a);
        if let Some(c) = &self.character {
            c.draw(f, a);
        }
        if let Some(s) = &self.surprise {
            s.draw(f, a);
        }
        if self.folder_open {
            self.draw_folder(f, a);
        }
        if let Some(d) = self.drag
            && d.moved
            && let Some(i) = self.item_of(d.from)
        {
            let t = &self.items[i].thumb;
            let (x, y) = (d.pos.0 - t.w as i32 / 2, d.pos.1 - t.h as i32 / 2);
            f.rect(x - 2, y - 2, t.w as i32 + 4, t.h as i32 + 4, BLACK);
            self.draw_thumb(f, t, x, y);
        }
        if let Some((_, b)) = &self.balloon {
            b.draw(f, a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::store::save_drawing;
    use crate::scenes::testutil::empty_assets;

    fn down(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }
    fn up(x: i32, y: i32) -> Input {
        Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        }
    }
    fn mv(x: i32, y: i32) -> Input {
        Input::MouseMove { x, y }
    }

    fn setup(n: u64) -> (tempfile::TempDir, tempfile::TempDir, Assets) {
        let (d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = a.palettes(Program::Artist).pal256.clone();
        for k in 0..n {
            let mut c = Canvas::new(CANVAS_W, CANVAS_H);
            c.px.fill(k as u8 + 2);
            save_drawing(dir.path(), &c, &pal, true, 1_793_404_800 + 60 * k).unwrap();
        }
        (d, dir, a)
    }

    fn room(dir: &Path, a: &Assets) -> GalleryRoom {
        let mut g = GalleryRoom::new(None, Some(dir.to_path_buf()), a);
        g.balloon = None;
        g
    }

    #[test]
    fn wall_shows_most_recent_and_names() {
        let (_d, dir, a) = setup(4);
        let g = room(dir.path(), &a);
        assert_eq!(g.items().len(), 4);
        assert_eq!(g.wall(), [Some(0), Some(1), Some(2)]);
        assert!(
            g.items()[0].name.starts_with("Desenho "),
            "{}",
            g.items()[0].name
        );
    }

    #[test]
    fn double_click_opens_in_paint() {
        let (_d, dir, a) = setup(2);
        let mut g = room(dir.path(), &a);
        let (x, y) = (FRAMES[0].0 + 100, FRAMES[0].1 + 60);
        let first = g.update(&[down(x, y), up(x, y)], 0.05, &a);
        assert!(!first.contains(&Effect::ChangeScene(SceneId::Paint)));
        let fx = g.update(&[down(x, y), up(x, y)], 0.1, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        assert!(dir.path().join(LAST_NAME).exists());
    }

    #[test]
    fn drag_swaps_frames_and_persists() {
        let (_d, dir, a) = setup(3);
        let mut g = room(dir.path(), &a);
        let (x0, y0) = (FRAMES[0].0 + 100, FRAMES[0].1 + 60);
        let (x1, y1) = (FRAMES[1].0 + 100, FRAMES[1].1 + 60);
        g.update(
            &[down(x0, y0), mv(x0 + 50, y0), mv(x1, y1), up(x1, y1)],
            0.1,
            &a,
        );
        assert_eq!(g.wall(), [Some(1), Some(0), Some(2)]);
        let g2 = room(dir.path(), &a);
        assert_eq!(g2.wall(), [Some(1), Some(0), Some(2)]);
    }

    #[test]
    fn trash_asks_and_deletes() {
        let (_d, dir, a) = setup(2);
        let mut g = room(dir.path(), &a);
        let path = g.wall_path(0).unwrap().to_path_buf();
        let (x0, y0) = (FRAMES[0].0 + 100, FRAMES[0].1 + 60);
        let (tx, ty) = (TRASH.0 + 40, TRASH.1 + 50);
        g.update(&[down(x0, y0), mv(tx, ty), up(tx, ty)], 0.1, &a);
        let (kind, b) = g.balloon.as_ref().unwrap();
        assert!(matches!(kind, Ask::Recycle(0)));
        let r = b.option_rect(&a, 0);
        g.update(&[down(r.0 + 4, r.1 + 4), up(r.0 + 4, r.1 + 4)], 0.1, &a);
        assert!(!path.exists());
        assert_eq!(g.items().len(), 1);
        assert_eq!(g.wall(), [None, Some(0), None]);
    }

    #[test]
    fn folder_places_on_wall() {
        let (_d, dir, a) = setup(5);
        let mut g = room(dir.path(), &a);
        let (fx_, fy) = (FOLDER.0 + 20, FOLDER.1 + 20);
        g.update(&[down(fx_, fy), up(fx_, fy)], 0.1, &a);
        assert!(g.folder_open());
        // a quinta figura da pasta vai para a moldura 2; a pasta fecha ao arrastar
        let c = GalleryRoom::folder_cell(4);
        let (cx, cy) = (c.0 + 20, c.1 + 20);
        let (wx, wy) = (FRAMES[2].0 + 100, FRAMES[2].1 + 60);
        g.update(
            &[down(cx, cy), mv(cx + 30, cy), mv(wx, wy), up(wx, wy)],
            0.1,
            &a,
        );
        assert!(!g.folder_open());
        assert_eq!(g.wall(), [Some(0), Some(1), Some(4)]);
    }

    #[test]
    fn printer_exports_png() {
        let (_d, dir, a) = setup(1);
        let mut g = room(dir.path(), &a);
        let (x0, y0) = (FRAMES[0].0 + 100, FRAMES[0].1 + 60);
        let (px, py) = (PRINTER.0 + 30, PRINTER.1 + 30);
        let fx = g.update(&[down(x0, y0), mv(px, py), up(px, py)], 0.1, &a);
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::Log(s) if s.contains("impressao-")))
        );
    }

    #[test]
    fn arrow_goes_to_lobby() {
        let (_d, dir, a) = setup(0);
        let mut g = room(dir.path(), &a);
        let fx = g.update(&[down(560, 295), up(560, 295)], 0.1, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Building)));
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        g.draw(&mut f, &a);
    }

    /// Só a pasta, aberta pela maleta de uma sala: por cima dela, clique
    /// duplo abre o quadro e fechar volta à sala.
    #[test]
    fn portfolio_over_a_room() {
        let (_d, dir, a) = setup(2);
        let mut g = GalleryRoom::portfolio(None, Some(dir.path().to_path_buf()), &a);
        assert!(g.overlay());
        assert!(g.folder_open());
        let c = GalleryRoom::folder_cell(0);
        let (cx, cy) = (c.0 + 20, c.1 + 20);
        // arrastar não tira a figura da pasta (não há parede aqui)
        g.update(&[down(cx, cy), mv(cx + 40, cy), up(cx + 40, cy)], 0.1, &a);
        assert!(g.folder_open());
        let fx = g.update(
            &[down(cx, cy), up(cx, cy), down(cx, cy), up(cx, cy)],
            0.1,
            &a,
        );
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        let mut g = GalleryRoom::portfolio(None, Some(dir.path().to_path_buf()), &a);
        let fx = g.update(
            &[
                down(PANEL_CLOSE.0 + 5, PANEL_CLOSE.1 + 5),
                up(PANEL_CLOSE.0 + 5, PANEL_CLOSE.1 + 5),
            ],
            0.1,
            &a,
        );
        assert!(fx.contains(&Effect::Back));
        let mut g = GalleryRoom::portfolio(None, Some(dir.path().to_path_buf()), &a);
        let fx = g.update(&[down(5, 5), up(5, 5)], 0.1, &a);
        assert!(fx.contains(&Effect::Back));
        // por cima da sala: só o painel é desenhado
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        f.fill([1, 2, 3]);
        g.draw(&mut f, &a);
        assert_eq!(f.pixel(5, 470)[..3], [1, 2, 3]);
    }

    /// Bolsa aberta no chão da Galeria: livro conta a história, a varinha
    /// vira o cursor e, no buraco do rato (sem dados), volta para a bolsa.
    #[test]
    fn gallery_bag_book_and_wand() {
        let (_d, dir, a) = setup(0);
        let mut g = room(dir.path(), &a);
        assert!(g.bag().open);
        let fx = g.update(&[down(270, 380), up(270, 380)], 0.1, &a);
        assert!(fx.contains(&Effect::Push(SceneId::BookStory)));
        let fx = g.update(&[down(270, 450), up(270, 450)], 0.1, &a);
        assert!(g.bag().holding_wand);
        assert!(fx.contains(&Effect::SetCursor(objects::WAND_CURSOR)));
        let fx = g.update(&[down(470, 370), up(470, 370)], 0.1, &a);
        assert!(!g.bag().holding_wand);
        assert!(g.bag().open);
        assert!(fx.contains(&Effect::PlaySound(snd(objects::SND_WAND_HOLE))));
    }

    /// Com os dados reais: a surpresa do buraco do rato toca e, no fim, a
    /// bolsa volta a ficar aberta com a varinha.
    #[test]
    fn gallery_mouse_hole_surprise_with_real_data() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let mut g = room(dir.path(), &a);
        g.update(&[down(270, 450), up(270, 450)], 0.1, &a);
        g.update(&[down(470, 370), up(470, 370)], 0.1, &a);
        assert!(g.surprise_playing());
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        g.update(&[], 0.5, &a);
        g.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "gallery_hole");
        for _ in 0..80 {
            g.update(&[], 0.1, &a);
        }
        assert!(!g.surprise_playing());
        assert!(g.bag().open && !g.bag().holding_wand);
    }
}
