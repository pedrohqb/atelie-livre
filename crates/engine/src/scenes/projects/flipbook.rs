//! Desenho animado: apresentação de figuras já feitas (da pasta de desenhos e
//! das amostras da instalação) em espaços numerados, cada uma com efeito de
//! transição, som e legenda, tocada em laço no quadro. Remontagem "usável
//! primeiro" dos roteiros de ajuda do original (`WHAT` `3a00`), descrita em
//! `docs/superpowers/specs/2026-10-31-projetos-design.md`.
//!
//! A tela de pintura (`paint.rs`) chama [`Flipbook::click`],
//! [`Flipbook::mouse_move`], [`Flipbook::mouse_up`], [`Flipbook::key`],
//! [`Flipbook::text`], [`Flipbook::update`], [`Flipbook::draw_strip`],
//! [`Flipbook::draw_canvas`] e [`Flipbook::save`]; toda a lógica fica aqui.
//!
//! Gestos: clique numa figura escolhe; arrastar leva a figura a outro espaço
//! (troca, ou move se vazio), da pasta aberta a um espaço, ou à pasta
//! fechada para tirá-la; clique duplo abre a figura na tela de pintura
//! (`Effect::Push(SceneId::FlipbookFigure)`) e, ao voltar, a figura
//! modificada entra no lugar da antiga.

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::paint::store::{load_picture, load_png, save_png};
use crate::paint::{CANVAS_H, CANVAS_W, Canvas, ORIGIN, WHITE};
use crate::scene::{Effect, Key, SceneId};
use crate::scenes::gallery::{list_drawings, list_samples};
use crate::scenes::sounds::{Category, load_categories};
use formats::fnt::Font;
use formats::gif;
use formats::palette::Rgb;
use std::path::{Path, PathBuf};
use std::rc::Rc;

type Rect = (i32, i32, i32, i32);

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Artist, id)
}

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE_RGB: [u8; 3] = [255, 255, 255];
const LIGHT: [u8; 3] = [140, 140, 140];
const DARK: [u8; 3] = [100, 100, 100];
const RED: [u8; 3] = [250, 0, 57];
const YELLOW: [u8; 3] = [255, 238, 76];

/// Espaços numerados ao começar; a seta "Mais espaços" acrescenta até
/// [`MAX_SLOTS`] (a tira do original vai de 2 a 10 casas).
pub const INITIAL_SLOTS: usize = 4;
pub const MAX_SLOTS: usize = 10;
/// Grade 4×3 dentro do quadro (12 células; no máximo 10 usadas).
const GRID_COLS: usize = 4;
const GRID_ROWS: usize = 3;
const CELL_W: i32 = CANVAS_W as i32 / GRID_COLS as i32;
const CELL_H: i32 = CANVAS_H as i32 / GRID_ROWS as i32;
/// Miniatura a 1/5.
const THUMB_SCALE: u32 = 5;
const THUMB_W: i32 = (CANVAS_W / THUMB_SCALE) as i32;
const THUMB_H: i32 = (CANVAS_H / THUMB_SCALE) as i32;
/// Distância a partir da qual um clique segurado vira arrasto.
const DRAG_START: i32 = 6;
/// Dois cliques na mesma figura dentro deste tempo são um clique duplo.
const DOUBLE_CLICK_S: f32 = 0.45;
/// Figura levada à tela de pintura pelo clique duplo (dentro de `ultimo-animacao/`).
pub const EDIT_NAME: &str = "figura-em-edicao.png";
/// Faixa branca da legenda na base da figura, ao tocar.
const CAPTION_H: i32 = 16;
/// Passos das transições (dissolver, cortinas).
pub const STEPS: u32 = 12;
/// Duração máxima de uma transição, em segundos (no máximo metade do intervalo).
const TRANSITION_S: f32 = 0.5;

/// Velocidades da régua TEMPO: milissegundos por figura (tartaruga → coelho).
pub const SPEEDS_MS: [u32; 5] = [4000, 2000, 1000, 500, 250];
const DEFAULT_SPEED: usize = 2;

/// Nome da pasta que reabre ao entrar de novo.
pub const LAST_DIR: &str = "ultimo-animacao";
pub const JSON_NAME: &str = "animacao.json";
pub const GIF_NAME: &str = "animacao.gif";

// --- faixa de opções (y 56..104) ---
pub const PLAY: Rect = (10, 58, 70, 12);
pub const PAUSE: Rect = (10, 73, 70, 12);
pub const STOP: Rect = (10, 88, 70, 12);
const RULER_X: i32 = 86;
/// Trilho amarelo da régua `02f7` (207×58, desenhada 10 px acima da faixa).
pub const TRACK: Rect = (RULER_X + 17, 65, 172, 19);
pub const TRACK_STOP0: i32 = TRACK.0 + 8;
pub const TRACK_STEP: i32 = 38;
pub const FOLDER_BTN: Rect = (300, 59, 60, 20);
pub const PREV_BTN: Rect = (364, 59, 22, 20);
pub const NEXT_BTN: Rect = (388, 59, 22, 20);
pub const EFFECT_BTN: Rect = (414, 59, 114, 20);
pub const SOUND_BTN: Rect = (300, 82, 60, 20);
pub const CAPTION_BTN: Rect = (364, 82, 70, 20);
/// Seta que acrescenta um espaço (só com todos os espaços preenchidos).
pub const ADD_BTN: Rect = (438, 82, 90, 20);
const STRIP_CLIP: Rect = (5, 56, 527, 48);

// --- pasta aberta dentro do quadro ---
const FOLDER_COLS: usize = 4;
const FOLDER_ROWS: usize = 4;
const FOLDER_PER_PAGE: usize = FOLDER_COLS * FOLDER_ROWS;
const FOLDER_CELL_H: i32 = 75;
pub const FOLDER_CLOSE: Rect = (ORIGIN.0 + 10, ORIGIN.1 + 324, 80, 18);
const FOLDER_PREV: Rect = (ORIGIN.0 + 440, ORIGIN.1 + 324, 60, 18);
const FOLDER_NEXT: Rect = (ORIGIN.0 + 510, ORIGIN.1 + 324, 60, 18);

// --- lista de sons dentro do quadro ---
const LIST_ROW_H: i32 = 15;
const LIST_ROWS: usize = 20;
const CAT_LIST: Rect = (
    ORIGIN.0 + 10,
    ORIGIN.1 + 10,
    170,
    LIST_ROWS as i32 * LIST_ROW_H + 4,
);
const SND_LIST: Rect = (
    ORIGIN.0 + 200,
    ORIGIN.1 + 10,
    220,
    LIST_ROWS as i32 * LIST_ROW_H + 4,
);
const SOUND_CLOSE: Rect = (ORIGIN.0 + 440, ORIGIN.1 + 10, 80, 18);

/// Efeito especial de transição para uma figura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Cut,
    Dissolve,
    WipeLeft,
    WipeRight,
    WipeUp,
    WipeDown,
    /// Sorteado a cada "tocar".
    Dice,
}

impl Transition {
    pub const ALL: [Transition; 7] = [
        Transition::Cut,
        Transition::Dissolve,
        Transition::WipeLeft,
        Transition::WipeRight,
        Transition::WipeUp,
        Transition::WipeDown,
        Transition::Dice,
    ];

    /// Nome gravado no `animacao.json`.
    pub fn name(self) -> &'static str {
        match self {
            Transition::Cut => "corte",
            Transition::Dissolve => "dissolver",
            Transition::WipeLeft => "cortina-esquerda",
            Transition::WipeRight => "cortina-direita",
            Transition::WipeUp => "cortina-cima",
            Transition::WipeDown => "cortina-baixo",
            Transition::Dice => "dado",
        }
    }

    pub fn from_name(s: &str) -> Option<Transition> {
        Transition::ALL.into_iter().find(|t| t.name() == s)
    }

    /// Rótulo do botão da faixa.
    fn label(self) -> &'static str {
        match self {
            Transition::Cut => "Corte",
            Transition::Dissolve => "Dissolver",
            Transition::WipeLeft => "Cortina p/ esq.",
            Transition::WipeRight => "Cortina p/ dir.",
            Transition::WipeUp => "Cortina p/ cima",
            Transition::WipeDown => "Cortina p/ baixo",
            Transition::Dice => "Dado (sorteia)",
        }
    }

    fn next(self) -> Transition {
        let k = Transition::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Transition::ALL[(k + 1) % Transition::ALL.len()]
    }
}

/// Som escolhido para uma figura: categoria e índice dentro dela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundPick {
    pub category: String,
    pub index: usize,
}

impl SoundPick {
    /// Forma gravada: `Categoria/índice`.
    fn to_key(&self) -> String {
        format!("{}/{}", self.category, self.index)
    }

    fn from_key(s: &str) -> Option<SoundPick> {
        let (cat, idx) = s.rsplit_once('/')?;
        Some(SoundPick {
            category: cat.to_string(),
            index: idx.parse().ok()?,
        })
    }
}

/// Uma figura num espaço numerado.
#[derive(Debug, Clone)]
pub struct Slot {
    /// Nome do arquivo de origem (só informativo).
    pub file: String,
    pub canvas: Canvas,
    thumb: Canvas,
    pub effect: Transition,
    pub sound: Option<SoundPick>,
    pub caption: String,
}

impl Slot {
    fn new(file: String, canvas: Canvas) -> Slot {
        let thumb = shrink(&canvas, THUMB_SCALE);
        Slot {
            file,
            canvas,
            thumb,
            effect: Transition::Cut,
            sound: None,
            caption: String::new(),
        }
    }

    /// Troca a figura, mantendo efeito, som e legenda.
    fn replace_canvas(&mut self, canvas: Canvas) {
        self.thumb = shrink(&canvas, THUMB_SCALE);
        self.canvas = canvas;
    }
}

/// Reduz o quadro `n` vezes (vizinho mais próximo).
fn shrink(c: &Canvas, n: u32) -> Canvas {
    let (w, h) = (c.w / n, c.h / n);
    let mut t = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            t.px[(y * w + x) as usize] = c.px[((y * n) * c.w + x * n) as usize];
        }
    }
    t
}

/// Entrada da pasta aberta. A miniatura só é lida quando a página aparece
/// (abrir a pasta com centenas de desenhos levava muitos segundos).
#[derive(Debug, Clone)]
struct FolderEntry {
    path: PathBuf,
    thumb: Option<Canvas>,
}

#[derive(Debug, Clone)]
struct Folder {
    entries: Vec<FolderEntry>,
    page: usize,
}

#[derive(Debug, Clone)]
struct SoundList {
    cat: usize,
}

/// Figura sendo arrastada (começa num clique numa figura ou numa figura da pasta).
#[derive(Debug, Clone)]
struct Drag {
    slot: usize,
    from: (i32, i32),
    at: (i32, i32),
    moved: bool,
}

/// Figura aberta na tela de pintura pelo clique duplo: espaço, arquivo e se
/// já passou o `update` do próprio clique (a cena de cima não atualiza a de
/// baixo; o `update` seguinte é o da volta).
#[derive(Debug, Clone)]
struct Editing {
    slot: usize,
    path: PathBuf,
    armed: bool,
}

/// Estado do "tocar".
#[derive(Debug, Clone)]
struct Playing {
    /// Posição na lista de espaços cheios.
    pos: usize,
    /// Segundos desde o começo da figura atual.
    t: f32,
    paused: bool,
    /// Se a figura atual é a primeira da apresentação (vem do branco).
    first: bool,
    /// Efeito resolvido (dado sorteado) de cada espaço cheio.
    effects: Vec<Transition>,
}

/// Gerador pequeno e determinístico (xorshift32) para o dado.
#[derive(Debug, Clone)]
struct XorShift(u32);

impl XorShift {
    fn next(&mut self) -> u32 {
        let mut x = self.0.max(1);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
}

/// Estado do desenho animado dentro da tela de pintura.
#[derive(Debug, Clone)]
pub struct Flipbook {
    pub slots: Vec<Option<Slot>>,
    selected: Option<usize>,
    speed: usize,
    play: Option<Playing>,
    folder: Option<Folder>,
    sound_list: Option<SoundList>,
    caption_edit: Option<String>,
    cats: Rc<Vec<Category>>,
    font: Option<Rc<Font>>,
    pal: Rc<Vec<Rgb>>,
    rng: XorShift,
    changed: bool,
    /// Mensagem curta mostrada na faixa (ex.: "Primeiro preencha os espaços!").
    notice: Option<(String, f32)>,
    drag: Option<Drag>,
    /// Último clique numa figura: espaço e tempo decorrido.
    last_click: Option<(usize, f32)>,
    editing: Option<Editing>,
}

impl Default for Flipbook {
    fn default() -> Flipbook {
        Flipbook {
            slots: vec![None; INITIAL_SLOTS],
            selected: None,
            speed: DEFAULT_SPEED,
            play: None,
            folder: None,
            sound_list: None,
            caption_edit: None,
            cats: Rc::new(Vec::new()),
            font: None,
            pal: Rc::new(Vec::new()),
            rng: XorShift(0x9e37_79b9),
            changed: false,
            notice: None,
            drag: None,
            last_click: None,
            editing: None,
        }
    }
}

impl Flipbook {
    /// Novo desenho animado, reabrindo `ultimo-animacao/` da pasta de desenhos se houver.
    pub fn open(drawings: Option<&Path>, assets: &Assets) -> Flipbook {
        let mut fb = Flipbook {
            cats: Rc::new(load_categories(assets)),
            font: assets.font(8),
            pal: Rc::new(assets.palettes(Program::Artist).pal256.clone()),
            ..Flipbook::default()
        };
        if let Some(d) = drawings {
            fb.load(&d.join(LAST_DIR));
        }
        fb
    }

    /// Espaços cheios, na ordem.
    pub fn filled(&self) -> Vec<usize> {
        (0..self.slots.len())
            .filter(|&k| self.slots[k].is_some())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Quantos espaços numerados há (de [`INITIAL_SLOTS`] a [`MAX_SLOTS`]).
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Seta "Mais espaços": acrescenta um espaço, só com todos preenchidos
    /// (senão o aviso do Rex, "Primeiro preencha os espaços!"). Falso se não
    /// acrescentou.
    pub fn add_slot(&mut self) -> bool {
        if self.slots.len() >= MAX_SLOTS {
            self.notice = Some((format!("Já são {MAX_SLOTS} espaços, o máximo"), 3.0));
            return false;
        }
        if self.slots.iter().any(|s| s.is_none()) {
            self.notice = Some(("Primeiro preencha os espaços!".into(), 3.0));
            return false;
        }
        self.slots.push(None);
        self.changed = true;
        true
    }

    /// Espaço cuja figura está na tela de pintura, se houver.
    pub fn editing(&self) -> Option<usize> {
        self.editing.as_ref().map(|e| e.slot)
    }

    /// Arquivo da figura levada à tela de pintura pelo clique duplo.
    pub fn edit_path(drawings: &Path) -> PathBuf {
        drawings.join(LAST_DIR).join(EDIT_NAME)
    }

    /// Clique duplo: grava a figura do espaço `k` em [`EDIT_NAME`] e pede a
    /// tela de pintura por cima; ao voltar, [`Flipbook::update`] reabre o
    /// arquivo no lugar da figura.
    pub fn edit(&mut self, k: usize, drawings: Option<&Path>) -> Vec<Effect> {
        let Some(Some(s)) = self.slots.get(k) else {
            return Vec::new();
        };
        let Some(d) = drawings else {
            self.notice = Some(("Sem pasta de desenhos: não dá para modificar".into(), 3.0));
            return Vec::new();
        };
        let path = Self::edit_path(d);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = save_png(&s.canvas, &self.pal, &path) {
            return vec![Effect::Log(format!(
                "desenho animado: não consegui gravar {}: {e}",
                path.display()
            ))];
        }
        self.drag = None;
        self.last_click = None;
        self.commit_caption();
        self.editing = Some(Editing {
            slot: k,
            path,
            armed: false,
        });
        vec![Effect::Push(SceneId::FlipbookFigure)]
    }

    /// Volta da tela de pintura: a figura modificada entra no lugar da antiga.
    fn finish_edit(&mut self) {
        let Some(e) = self.editing.take() else {
            return;
        };
        if let Some(c) = load_png(&e.path, &self.pal)
            && let Some(Some(s)) = self.slots.get_mut(e.slot)
            && s.canvas.px != c.px
        {
            s.replace_canvas(c);
            self.changed = true;
        }
        let _ = std::fs::remove_file(&e.path);
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn interval_ms(&self) -> u32 {
        SPEEDS_MS[self.speed]
    }

    pub fn speed(&self) -> usize {
        self.speed
    }

    pub fn set_speed(&mut self, k: usize) {
        self.speed = k.min(SPEEDS_MS.len() - 1);
        self.changed = true;
    }

    pub fn is_playing(&self) -> bool {
        self.play.as_ref().is_some_and(|p| !p.paused)
    }

    pub fn is_paused(&self) -> bool {
        self.play.as_ref().is_some_and(|p| p.paused)
    }

    pub fn folder_open(&self) -> bool {
        self.folder.is_some()
    }

    pub fn caption_editing(&self) -> Option<&str> {
        self.caption_edit.as_deref()
    }

    /// Índice do espaço cuja figura está sendo mostrada ao tocar.
    pub fn shown(&self) -> Option<usize> {
        let p = self.play.as_ref()?;
        self.filled().get(p.pos).copied()
    }

    /// Coloca a figura no primeiro espaço vazio. Falso se não há espaço.
    pub fn place(&mut self, file: String, canvas: Canvas) -> bool {
        let Some(k) = self.slots.iter().position(|s| s.is_none()) else {
            self.notice = Some(("Não há mais espaço: tire uma figura".into(), 3.0));
            return false;
        };
        self.slots[k] = Some(Slot::new(file, canvas));
        self.selected = Some(k);
        self.changed = true;
        true
    }

    /// Esvazia o espaço `k`.
    pub fn remove(&mut self, k: usize) {
        if k < self.slots.len() && self.slots[k].is_some() {
            self.slots[k] = None;
            self.changed = true;
        }
        if self.selected == Some(k) {
            self.selected = None;
        }
    }

    /// Troca as figuras dos espaços `a` e `b` (efeito, som e legenda vão junto).
    pub fn swap(&mut self, a: usize, b: usize) {
        if a < self.slots.len() && b < self.slots.len() && a != b {
            self.slots.swap(a, b);
            self.changed = true;
        }
    }

    fn selected_slot_mut(&mut self) -> Option<&mut Slot> {
        let k = self.selected?;
        self.slots.get_mut(k)?.as_mut()
    }

    pub fn set_effect(&mut self, k: usize, e: Transition) {
        if let Some(Some(s)) = self.slots.get_mut(k) {
            s.effect = e;
            self.changed = true;
        }
    }

    pub fn set_caption(&mut self, k: usize, text: &str) {
        if let Some(Some(s)) = self.slots.get_mut(k) {
            s.caption = text.to_string();
            self.changed = true;
        }
    }

    pub fn set_sound(&mut self, k: usize, pick: Option<SoundPick>) {
        if let Some(Some(s)) = self.slots.get_mut(k) {
            s.sound = pick;
            self.changed = true;
        }
    }

    fn sound_ref(&self, pick: &SoundPick) -> Option<SoundRef> {
        let cat = self.cats.iter().find(|c| c.name == pick.category)?;
        cat.sounds.get(pick.index).map(|(_, r)| r.clone())
    }

    fn roll(&mut self) -> Transition {
        let k = (self.rng.next() as usize) % (Transition::ALL.len() - 1);
        Transition::ALL[k]
    }

    /// Começa a tocar do início. Devolve o som da primeira figura, se houver.
    pub fn play(&mut self) -> Vec<Effect> {
        let order = self.filled();
        if order.is_empty() {
            self.notice = Some(("Primeiro preencha os espaços!".into(), 3.0));
            return Vec::new();
        }
        if let Some(p) = self.play.as_mut() {
            if p.paused {
                p.paused = false;
                return Vec::new();
            }
            return Vec::new();
        }
        let mut effects = Vec::with_capacity(order.len());
        for &k in &order {
            let e = self.slots[k].as_ref().map_or(Transition::Cut, |s| s.effect);
            effects.push(if e == Transition::Dice {
                self.roll()
            } else {
                e
            });
        }
        self.folder = None;
        self.sound_list = None;
        self.commit_caption();
        self.play = Some(Playing {
            pos: 0,
            t: 0.0,
            paused: false,
            first: true,
            effects,
        });
        self.sound_at(0)
    }

    pub fn pause(&mut self) {
        if let Some(p) = self.play.as_mut() {
            p.paused = true;
        }
    }

    pub fn stop(&mut self) {
        self.play = None;
    }

    fn sound_at(&self, pos: usize) -> Vec<Effect> {
        let order = self.filled();
        let Some(&k) = order.get(pos) else {
            return Vec::new();
        };
        self.slots[k]
            .as_ref()
            .and_then(|s| s.sound.as_ref())
            .and_then(|p| self.sound_ref(p))
            .map(|r| vec![Effect::PlaySound(r)])
            .unwrap_or_default()
    }

    /// Avança (ou volta) uma figura; parado, começa em pausa na figura escolhida.
    fn step(&mut self, delta: i32) -> Vec<Effect> {
        let order = self.filled();
        if order.is_empty() {
            self.notice = Some(("Primeiro preencha os espaços!".into(), 3.0));
            return Vec::new();
        }
        let n = order.len() as i32;
        match self.play.as_mut() {
            Some(p) => {
                p.pos = (p.pos as i32 + delta).rem_euclid(n) as usize;
                p.t = 0.0;
                p.first = false;
                p.paused = true;
            }
            None => {
                let start = self
                    .selected
                    .and_then(|k| order.iter().position(|&s| s == k))
                    .unwrap_or(0);
                let effects = order
                    .iter()
                    .map(|&k| self.slots[k].as_ref().map_or(Transition::Cut, |s| s.effect))
                    .map(|e| {
                        if e == Transition::Dice {
                            Transition::Cut
                        } else {
                            e
                        }
                    })
                    .collect();
                self.play = Some(Playing {
                    pos: start,
                    t: 0.0,
                    paused: true,
                    first: true,
                    effects,
                });
            }
        }
        let pos = self.play.as_ref().map_or(0, |p| p.pos);
        self.sound_at(pos)
    }

    /// Passa o tempo: troca de figura a cada intervalo enquanto toca.
    pub fn update(&mut self, dt: f32) -> Vec<Effect> {
        let mut fx = Vec::new();
        if let Some((_, t)) = self.notice.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.notice = None;
            }
        }
        if let Some((_, t)) = self.last_click.as_mut() {
            *t += dt.max(0.0);
        }
        if let Some(e) = self.editing.as_mut() {
            if e.armed {
                self.finish_edit();
            } else {
                e.armed = true;
            }
        }
        let n = self.count();
        let interval = self.interval_ms() as f32 / 1000.0;
        let Some(p) = self.play.as_mut() else {
            return fx;
        };
        if p.paused || n == 0 {
            return fx;
        }
        p.t += dt.max(0.0);
        let mut advanced = false;
        while p.t >= interval {
            p.t -= interval;
            p.pos = (p.pos + 1) % n;
            p.first = false;
            advanced = true;
        }
        if advanced {
            let pos = p.pos;
            fx.extend(self.sound_at(pos));
        }
        fx
    }

    fn transition_secs(&self) -> f32 {
        TRANSITION_S.min(self.interval_ms() as f32 / 2000.0)
    }

    /// Quadro mostrado agora ao tocar (figura com a transição e a legenda), se tocando.
    pub fn frame(&self) -> Option<Canvas> {
        let p = self.play.as_ref()?;
        let order = self.filled();
        let &k = order.get(p.pos)?;
        let cur = self.slots[k].as_ref()?;
        let prev = if p.first || order.len() < 2 {
            None
        } else {
            let pk = order[(p.pos + order.len() - 1) % order.len()];
            self.slots[pk].as_ref().map(|s| &s.canvas)
        };
        let effect = p.effects.get(p.pos).copied().unwrap_or(Transition::Cut);
        let step = if p.paused {
            STEPS
        } else {
            ((p.t / self.transition_secs()) * STEPS as f32).floor() as u32
        };
        Some(self.compose(prev, cur, effect, step.min(STEPS)))
    }

    /// Monta a figura `cur` sobre `prev` no passo `step` (0..=STEPS) da transição,
    /// com a legenda na base.
    pub fn compose(
        &self,
        prev: Option<&Canvas>,
        cur: &Slot,
        effect: Transition,
        step: u32,
    ) -> Canvas {
        let blank = Canvas::new(CANVAS_W, CANVAS_H);
        let prev = prev.unwrap_or(&blank);
        let mut out = transition(prev, &cur.canvas, effect, step);
        if step >= STEPS && !cur.caption.is_empty() {
            let y0 = CANVAS_H as i32 - CAPTION_H;
            for y in y0..CANVAS_H as i32 {
                for x in 0..CANVAS_W as i32 {
                    out.set(x, y, WHITE);
                }
            }
            let font = self.font.as_deref();
            let (tw, th) = crate::paint::text_size(font, &cur.caption);
            let x = (CANVAS_W as i32 - tw as i32) / 2;
            let y = y0 + (CAPTION_H - th as i32) / 2;
            out.draw_text(font, x.max(0), y, &cur.caption, crate::paint::BLACK);
        }
        out
    }

    // ----- cliques -----

    pub fn cell_rect(k: usize) -> Rect {
        (
            ORIGIN.0 + (k % GRID_COLS) as i32 * CELL_W,
            ORIGIN.1 + (k / GRID_COLS) as i32 * CELL_H,
            CELL_W,
            CELL_H,
        )
    }

    pub fn thumb_rect(k: usize) -> Rect {
        let c = Self::cell_rect(k);
        (c.0 + (CELL_W - THUMB_W) / 2, c.1 + 10, THUMB_W, THUMB_H)
    }

    /// Espaço numerado existente sob `(x, y)`.
    fn slot_at(&self, x: i32, y: i32) -> Option<usize> {
        (0..self.slots.len()).find(|&k| inside(Self::cell_rect(k), x, y))
    }

    pub fn folder_cell_rect(k: usize) -> Rect {
        (
            ORIGIN.0 + 15 + (k % FOLDER_COLS) as i32 * CELL_W,
            ORIGIN.1 + 20 + (k / FOLDER_COLS) as i32 * FOLDER_CELL_H,
            THUMB_W,
            THUMB_H + 4,
        )
    }

    fn open_folder(&mut self, drawings: Option<&Path>, assets: &Assets) {
        let mut paths = drawings.map(list_drawings).unwrap_or_default();
        paths.extend(list_samples(assets.data_dir()));
        let entries = paths
            .into_iter()
            .map(|path| FolderEntry { path, thumb: None })
            .collect();
        self.folder = Some(Folder { entries, page: 0 });
        self.sound_list = None;
        self.load_folder_page();
    }

    /// Lê as miniaturas da página aberta da pasta; o que não abre sai da lista.
    fn load_folder_page(&mut self) {
        let Some(f) = self.folder.as_mut() else {
            return;
        };
        loop {
            let from = (f.page * FOLDER_PER_PAGE).min(f.entries.len());
            let to = (from + FOLDER_PER_PAGE).min(f.entries.len());
            let Some(k) = (from..to).find(|&k| f.entries[k].thumb.is_none()) else {
                break;
            };
            match load_picture(&f.entries[k].path, &self.pal) {
                Some(c) => f.entries[k].thumb = Some(shrink(&c, THUMB_SCALE)),
                None => {
                    f.entries.remove(k);
                }
            }
        }
        let pages = Self::folder_pages(f);
        if f.page >= pages {
            f.page = pages - 1;
        }
    }

    fn folder_pages(f: &Folder) -> usize {
        f.entries.len().div_ceil(FOLDER_PER_PAGE).max(1)
    }

    fn commit_caption(&mut self) {
        if let Some(text) = self.caption_edit.take()
            && let Some(k) = self.selected
        {
            self.set_caption(k, &text);
        }
    }

    /// Clique na tela (fora da porta de saída). Devolve efeitos (sons, avisos).
    pub fn click(
        &mut self,
        x: i32,
        y: i32,
        drawings: Option<&Path>,
        assets: &Assets,
    ) -> Vec<Effect> {
        // faixa de opções
        if inside(PLAY, x, y) {
            return self.play();
        }
        if inside(PAUSE, x, y) {
            self.pause();
            return Vec::new();
        }
        if inside(STOP, x, y) {
            self.stop();
            return Vec::new();
        }
        if inside(TRACK, x, y) {
            let k = ((x - TRACK_STOP0 + TRACK_STEP / 2) / TRACK_STEP).clamp(0, 4);
            self.set_speed(k as usize);
            return Vec::new();
        }
        if inside(PREV_BTN, x, y) {
            return self.step(-1);
        }
        if inside(NEXT_BTN, x, y) {
            return self.step(1);
        }
        if self.play.is_some() {
            // qualquer outro clique tocando: para e volta aos espaços
            if inside(canvas_rect(), x, y) {
                self.stop();
            }
            return Vec::new();
        }
        if inside(FOLDER_BTN, x, y) {
            self.commit_caption();
            if self.folder.is_some() {
                self.folder = None;
            } else {
                self.open_folder(drawings, assets);
            }
            return Vec::new();
        }
        if inside(EFFECT_BTN, x, y) {
            self.commit_caption();
            if let Some(s) = self.selected_slot_mut() {
                s.effect = s.effect.next();
                self.changed = true;
            } else {
                self.notice = Some(("Escolha um espaço com figura".into(), 3.0));
            }
            return Vec::new();
        }
        if inside(SOUND_BTN, x, y) {
            self.commit_caption();
            if self.selected_slot_mut().is_some() {
                self.folder = None;
                self.sound_list = match self.sound_list {
                    Some(_) => None,
                    None => Some(SoundList { cat: 0 }),
                };
            } else {
                self.notice = Some(("Escolha um espaço com figura".into(), 3.0));
            }
            return Vec::new();
        }
        if inside(ADD_BTN, x, y) {
            self.commit_caption();
            self.add_slot();
            return Vec::new();
        }
        if inside(CAPTION_BTN, x, y) {
            if self.caption_edit.is_some() {
                self.commit_caption();
            } else if let Some(k) = self.selected
                && let Some(s) = self.slots[k].as_ref()
            {
                self.folder = None;
                self.sound_list = None;
                self.caption_edit = Some(s.caption.clone());
            } else {
                self.notice = Some(("Escolha um espaço com figura".into(), 3.0));
            }
            return Vec::new();
        }
        if !inside(canvas_rect(), x, y) {
            return Vec::new();
        }
        // lista de sons aberta
        if let Some(sl) = self.sound_list.as_mut() {
            if inside(SOUND_CLOSE, x, y) {
                self.sound_list = None;
                return Vec::new();
            }
            if inside(CAT_LIST, x, y) {
                let row = ((y - CAT_LIST.1 - 2) / LIST_ROW_H) as usize;
                if row < self.cats.len() {
                    sl.cat = row;
                }
                return Vec::new();
            }
            if inside(SND_LIST, x, y) {
                let row = ((y - SND_LIST.1 - 2) / LIST_ROW_H) as usize;
                let cat = sl.cat;
                let pick = if row == 0 {
                    None
                } else {
                    let Some(c) = self.cats.get(cat) else {
                        return Vec::new();
                    };
                    let Some((_, r)) = c.sounds.get(row - 1) else {
                        return Vec::new();
                    };
                    let r = r.clone();
                    let pick = SoundPick {
                        category: c.name.clone(),
                        index: row - 1,
                    };
                    if let Some(k) = self.selected {
                        self.set_sound(k, Some(pick));
                    }
                    self.sound_list = None;
                    return vec![Effect::PlaySound(r)];
                };
                if let Some(k) = self.selected {
                    self.set_sound(k, pick);
                }
                self.sound_list = None;
            }
            return Vec::new();
        }
        // pasta aberta
        if let Some(f) = self.folder.as_mut() {
            if inside(FOLDER_CLOSE, x, y) {
                self.folder = None;
                return Vec::new();
            }
            let pages = Self::folder_pages(f);
            if inside(FOLDER_PREV, x, y) {
                f.page = (f.page + pages - 1) % pages;
                self.load_folder_page();
                return Vec::new();
            }
            if inside(FOLDER_NEXT, x, y) {
                f.page = (f.page + 1) % pages;
                self.load_folder_page();
                return Vec::new();
            }
            if let Some(k) = (0..FOLDER_PER_PAGE).find(|&k| inside(Self::folder_cell_rect(k), x, y))
            {
                let idx = f.page * FOLDER_PER_PAGE + k;
                let Some(e) = f.entries.get(idx) else {
                    return Vec::new();
                };
                let path = e.path.clone();
                let Some(c) = load_picture(&path, &self.pal) else {
                    return vec![Effect::Log(format!(
                        "desenho animado: não consegui abrir {}",
                        path.display()
                    ))];
                };
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                // vai para o primeiro espaço vazio; se o botão for segurado e
                // arrastado, a figura segue o mouse até o espaço escolhido
                if self.place(name, c)
                    && let Some(k) = self.selected
                {
                    self.drag = Some(Drag {
                        slot: k,
                        from: (x, y),
                        at: (x, y),
                        moved: false,
                    });
                }
            }
            return Vec::new();
        }
        // espaços numerados
        self.commit_caption();
        if let Some(k) = self.slot_at(x, y) {
            // clique duplo numa figura: abre a tela de pintura com ela
            if self.slots[k].is_some()
                && self
                    .last_click
                    .is_some_and(|(s, t)| s == k && t < DOUBLE_CLICK_S)
            {
                self.selected = Some(k);
                return self.edit(k, drawings);
            }
            self.last_click = self.slots[k].as_ref().map(|_| (k, 0.0));
            // escolhido + outro espaço: troca (ou move, se o outro está vazio);
            // clicar de novo no escolhido desescolhe
            match (self.selected, self.slots[k].is_some()) {
                (Some(s), _) if s == k => self.selected = None,
                (Some(s), _) if self.slots[s].is_some() => {
                    self.swap(s, k);
                    self.selected = Some(k);
                }
                (_, true) => self.selected = Some(k),
                (_, false) => self.selected = None,
            }
            if self.slots[k].is_some() {
                self.drag = Some(Drag {
                    slot: k,
                    from: (x, y),
                    at: (x, y),
                    moved: false,
                });
            }
        } else {
            self.last_click = None;
        }
        Vec::new()
    }

    /// Mouse em movimento: a figura clicada passa a ser arrastada depois de
    /// [`DRAG_START`] px (da pasta, a pasta fecha para mostrar os espaços).
    pub fn mouse_move(&mut self, x: i32, y: i32) {
        let Some(d) = self.drag.as_mut() else {
            return;
        };
        d.at = (x, y);
        if !d.moved && ((x - d.from.0).abs() >= DRAG_START || (y - d.from.1).abs() >= DRAG_START) {
            d.moved = true;
            self.folder = None;
            self.last_click = None;
        }
    }

    /// Botão solto: a figura arrastada cai noutro espaço (troca, ou move se
    /// vazio) ou na pasta fechada (sai do desenho animado).
    pub fn mouse_up(&mut self, x: i32, y: i32) {
        let Some(d) = self.drag.take() else {
            return;
        };
        if !d.moved || self.slots.get(d.slot).is_none_or(|s| s.is_none()) {
            return;
        }
        if let Some(b) = self.slot_at(x, y) {
            if b != d.slot {
                self.swap(d.slot, b);
                self.selected = Some(b);
            }
        } else if inside(FOLDER_BTN, x, y) {
            self.remove(d.slot);
        }
    }

    /// Figura arrastada (miniatura centrada no mouse), se houver.
    pub fn dragging(&self) -> Option<(usize, (i32, i32))> {
        self.drag
            .as_ref()
            .filter(|d| d.moved)
            .map(|d| (d.slot, d.at))
    }

    /// Tecla: Delete esvazia o espaço escolhido; editando a legenda, Enter
    /// confirma e Backspace apaga.
    pub fn key(&mut self, key: Key) {
        if let Some(text) = self.caption_edit.as_mut() {
            match key {
                Key::Enter => self.commit_caption(),
                Key::Backspace => {
                    text.pop();
                }
                Key::Escape => self.caption_edit = None,
                _ => {}
            }
            return;
        }
        match key {
            Key::Delete | Key::Backspace => {
                if let Some(k) = self.selected {
                    self.remove(k);
                }
            }
            Key::Left => {
                self.step(-1);
            }
            Key::Right => {
                self.step(1);
            }
            _ => {}
        }
    }

    /// Texto digitado na legenda.
    pub fn text(&mut self, t: &str) {
        if let Some(text) = self.caption_edit.as_mut() {
            text.extend(
                t.chars()
                    .filter(|c| !c.is_control() && formats::cp1252::encode_char(*c).is_some()),
            );
        }
    }

    // ----- desenho -----

    fn rgb(&self, i: u8) -> [u8; 3] {
        self.pal.get(i as usize).map_or(BLACK, |p| [p.r, p.g, p.b])
    }

    fn blit_canvas(&self, f: &mut Frame, c: &Canvas, x: i32, y: i32) {
        for (k, &i) in c.px.iter().enumerate() {
            f.put(
                x + (k as u32 % c.w) as i32,
                y + (k as u32 / c.w) as i32,
                self.rgb(i),
            );
        }
    }

    fn label(&self, f: &mut Frame, x: i32, y: i32, s: &str, rgb: [u8; 3]) -> i32 {
        match self.font.as_deref() {
            Some(font) => f.text_font(font, x, y, s, rgb),
            None => {
                f.text(x, y, s, rgb);
                8 * s.chars().count() as i32
            }
        }
    }

    fn label_w(&self, s: &str) -> i32 {
        match self.font.as_deref() {
            Some(font) => font.width(s) as i32,
            None => 8 * s.chars().count() as i32,
        }
    }

    fn button(&self, f: &mut Frame, r: Rect, s: &str, on: bool) {
        f.rect(r.0, r.1, r.2, r.3, BLACK);
        f.rect(
            r.0 + 1,
            r.1 + 1,
            r.2 - 2,
            r.3 - 2,
            if on { YELLOW } else { WHITE_RGB },
        );
        let w = self.label_w(s);
        self.label(f, r.0 + (r.2 - w) / 2, r.1 + (r.3 - 10) / 2, s, BLACK);
    }

    /// Faixa de opções: TOCAR/PAUSA/PARAR, régua TEMPO, pasta, setas, efeito, som, legenda.
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        f.rect(
            STRIP_CLIP.0,
            STRIP_CLIP.1,
            STRIP_CLIP.2,
            STRIP_CLIP.3,
            LIGHT,
        );
        for (r, id, on) in [
            (PLAY, 0x3a61u16, self.is_playing()),
            (PAUSE, 0x3a59, self.is_paused()),
            (STOP, 0x3a5d, false),
        ] {
            let img = a.image(&pic(if on { id + 3 } else { id }));
            if (img.width, img.height) == (70, 12) {
                f.blit(&img, r.0, r.1);
                if on {
                    f.frame_rect(r.0, r.1, r.2, r.3, 1, RED);
                }
            } else {
                let s = match id {
                    0x3a61 => "TOCAR",
                    0x3a59 => "PAUSA",
                    _ => "PARAR",
                };
                self.button(f, r, s, on);
            }
        }
        // régua TEMPO: a imagem tem 58 px, a faixa 48; cortam-se os parafusos de cima
        let ruler = a.image(&pic(0x02f7));
        if (ruler.width, ruler.height) == (207, 58) {
            crate::scenes::stickers::blit_clipped(
                f,
                &ruler,
                &a.image(&pic(0x02f9)),
                RULER_X,
                STRIP_CLIP.1 - 10,
                STRIP_CLIP,
            );
        } else {
            f.rect(RULER_X, 58, 207, 44, DARK);
            f.rect(TRACK.0, TRACK.1, TRACK.2, TRACK.3, YELLOW);
            self.label(f, RULER_X + 80, 88, "TEMPO", WHITE_RGB);
        }
        f.blit_masked(
            &a.image(&pic(0x3a69)),
            &a.image(&pic(0x3a6d)),
            RULER_X + 30,
            91,
        );
        f.blit_masked(
            &a.image(&pic(0x3a6f)),
            &a.image(&pic(0x3a71)),
            RULER_X + 150,
            89,
        );
        let cx = TRACK_STOP0 + self.speed as i32 * TRACK_STEP;
        let cursor = a.image(&pic(0x3a65));
        if (cursor.width, cursor.height) == (16, 14) {
            f.blit_masked(&cursor, &a.image(&pic(0x3a68)), cx - 8, TRACK.1 + 2);
        } else {
            f.rect(cx - 4, TRACK.1 + 2, 8, TRACK.3 - 4, BLACK);
        }
        self.button(f, FOLDER_BTN, "Pasta", self.folder.is_some());
        self.button(f, PREV_BTN, "<", false);
        self.button(f, NEXT_BTN, ">", false);
        let effect = self
            .selected
            .and_then(|k| self.slots[k].as_ref())
            .map_or("Efeito", |s| s.effect.label());
        self.button(f, EFFECT_BTN, effect, false);
        let has_sound = self
            .selected
            .and_then(|k| self.slots[k].as_ref())
            .is_some_and(|s| s.sound.is_some());
        self.button(f, SOUND_BTN, "Som", has_sound || self.sound_list.is_some());
        self.button(f, CAPTION_BTN, "Legenda", self.caption_edit.is_some());
        self.button(f, ADD_BTN, "Mais espaços", false);
    }

    /// Conteúdo do quadro: espaços, pasta, lista de sons ou a figura tocando.
    pub fn draw_canvas(&self, f: &mut Frame, a: &Assets) {
        if let Some(c) = self.frame() {
            self.blit_canvas(f, &c, ORIGIN.0, ORIGIN.1);
            return;
        }
        f.rect(
            ORIGIN.0,
            ORIGIN.1,
            CANVAS_W as i32,
            CANVAS_H as i32,
            WHITE_RGB,
        );
        if self.sound_list.is_some() {
            self.draw_sound_list(f);
            return;
        }
        if let Some(folder) = &self.folder {
            self.draw_folder(f, folder);
            return;
        }
        if let Some((msg, _)) = &self.notice {
            let w = self.label_w(msg) + 16;
            let x = ORIGIN.0 + (CANVAS_W as i32 - w) / 2;
            f.rect(x, ORIGIN.1 + 2, w, 16, YELLOW);
            f.frame_rect(x, ORIGIN.1 + 2, w, 16, 1, BLACK);
            self.label(f, x + 8, ORIGIN.1 + 5, msg, BLACK);
        }
        let status = format!("{} figura(s), {} espaços", self.count(), self.slots.len());
        let w = self.label_w(&status);
        self.label(
            f,
            ORIGIN.0 + CANVAS_W as i32 - w - 6,
            ORIGIN.1 + CANVAS_H as i32 - 12,
            &status,
            DARK,
        );
        for k in 0..self.slots.len() {
            let t = Self::thumb_rect(k);
            match &self.slots[k] {
                Some(s) => {
                    self.blit_canvas(f, &s.thumb, t.0, t.1);
                    f.frame_rect(t.0 - 1, t.1 - 1, t.2 + 2, t.3 + 2, 1, BLACK);
                    let caption = match (&self.caption_edit, self.selected == Some(k)) {
                        (Some(e), true) => format!("{e}_"),
                        _ => s.caption.clone(),
                    };
                    let w = self.label_w(&caption);
                    self.label(f, t.0 + (t.2 - w) / 2, t.1 + t.3 + 4, &caption, BLACK);
                    if s.sound.is_some() {
                        // bolinha da nota musical no canto
                        f.rect(t.0 + t.2 - 10, t.1 + 2, 8, 8, BLACK);
                        f.rect(t.0 + t.2 - 9, t.1 + 3, 6, 6, YELLOW);
                    }
                }
                None => {
                    f.rect(t.0, t.1, t.2, t.3, [220, 220, 220]);
                    f.frame_rect(t.0, t.1, t.2, t.3, 1, LIGHT);
                }
            }
            if self.selected == Some(k) {
                f.frame_rect(t.0 - 3, t.1 - 3, t.2 + 6, t.3 + 6, 2, RED);
            }
            // número do espaço (dígitos coloridos do original, 24×33, com máscara)
            let digit = DIGITS.get(k).map(|&(id, mask)| (a.image(&pic(id)), mask));
            match digit {
                Some((img, mask)) if img.width <= 40 && img.height <= 40 => {
                    f.blit_masked(&img, &a.image(&pic(mask)), t.0 - 8, t.1 - 6);
                }
                _ => {
                    let n = (k + 1).to_string();
                    let w = 8 * n.len() as i32 + 6;
                    f.rect(t.0 - 6, t.1 - 6, w, 12, YELLOW);
                    f.frame_rect(t.0 - 6, t.1 - 6, w, 12, 1, BLACK);
                    f.text(t.0 - 3, t.1 - 4, &n, BLACK);
                }
            }
        }
        if let Some((k, (x, y))) = self.dragging()
            && let Some(s) = self.slots[k].as_ref()
        {
            let (x, y) = (x - THUMB_W / 2, y - THUMB_H / 2);
            self.blit_canvas(f, &s.thumb, x, y);
            f.frame_rect(x - 1, y - 1, THUMB_W + 2, THUMB_H + 2, 1, RED);
        }
    }

    /// Folha de impressão: as figuras na ordem, com o número e a legenda.
    pub fn print_sheet(&self) -> Canvas {
        let mut out = Canvas::new(CANVAS_W, CANVAS_H);
        let font = self.font.as_deref();
        for (k, s) in self.slots.iter().enumerate() {
            let Some(s) = s else { continue };
            let t = Self::thumb_rect(k);
            let (x0, y0) = (t.0 - ORIGIN.0, t.1 - ORIGIN.1);
            for (i, &v) in s.thumb.px.iter().enumerate() {
                out.set(
                    x0 + (i as u32 % s.thumb.w) as i32,
                    y0 + (i as u32 / s.thumb.w) as i32,
                    v,
                );
            }
            for x in x0 - 1..=x0 + THUMB_W {
                out.set(x, y0 - 1, crate::paint::BLACK);
                out.set(x, y0 + THUMB_H, crate::paint::BLACK);
            }
            for y in y0 - 1..=y0 + THUMB_H {
                out.set(x0 - 1, y, crate::paint::BLACK);
                out.set(x0 + THUMB_W, y, crate::paint::BLACK);
            }
            out.draw_text(font, x0, y0 - 11, &(k + 1).to_string(), crate::paint::BLACK);
            let (tw, _) = crate::paint::text_size(font, &s.caption);
            out.draw_text(
                font,
                x0 + (THUMB_W - tw as i32) / 2,
                y0 + THUMB_H + 3,
                &s.caption,
                crate::paint::BLACK,
            );
        }
        out
    }

    fn draw_folder(&self, f: &mut Frame, folder: &Folder) {
        f.rect(ORIGIN.0, ORIGIN.1, CANVAS_W as i32, CANVAS_H as i32, LIGHT);
        let pages = Self::folder_pages(folder);
        let title = if folder.entries.is_empty() {
            "Pasta: ainda não há figuras guardadas".to_string()
        } else {
            format!("Pasta: clique numa figura ({}/{})", folder.page + 1, pages)
        };
        self.label(f, ORIGIN.0 + 10, ORIGIN.1 + 5, &title, BLACK);
        let from = folder.page * FOLDER_PER_PAGE;
        for (k, e) in folder
            .entries
            .iter()
            .skip(from)
            .take(FOLDER_PER_PAGE)
            .enumerate()
        {
            let r = Self::folder_cell_rect(k);
            if let Some(t) = &e.thumb {
                self.blit_canvas(f, t, r.0, r.1);
            }
            f.frame_rect(r.0, r.1, THUMB_W, THUMB_H, 1, BLACK);
        }
        self.button(f, FOLDER_CLOSE, "Fechar", false);
        self.button(f, FOLDER_PREV, "<", false);
        self.button(f, FOLDER_NEXT, ">", false);
    }

    fn draw_list(&self, f: &mut Frame, r: Rect, names: &[String], hot: Option<usize>) {
        f.rect(r.0, r.1, r.2, r.3, BLACK);
        f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, WHITE_RGB);
        for (row, name) in names.iter().take(LIST_ROWS).enumerate() {
            let y = r.1 + 2 + row as i32 * LIST_ROW_H;
            if hot == Some(row) {
                f.rect(r.0 + 1, y, r.2 - 2, LIST_ROW_H, YELLOW);
            }
            if row > 0 {
                f.rect(r.0 + 1, y, r.2 - 2, 1, BLACK);
            }
            self.label(f, r.0 + 6, y + 2, name, BLACK);
        }
    }

    fn draw_sound_list(&self, f: &mut Frame) {
        let Some(sl) = &self.sound_list else {
            return;
        };
        let cats: Vec<String> = self.cats.iter().map(|c| c.name.clone()).collect();
        self.draw_list(f, CAT_LIST, &cats, Some(sl.cat));
        let mut names = vec!["Sem som".to_string()];
        if let Some(c) = self.cats.get(sl.cat) {
            names.extend(c.sounds.iter().map(|(n, _)| n.clone()));
        }
        let hot = self
            .selected
            .and_then(|k| self.slots[k].as_ref())
            .and_then(|s| s.sound.as_ref())
            .filter(|p| self.cats.get(sl.cat).is_some_and(|c| c.name == p.category))
            .map(|p| p.index + 1);
        self.draw_list(f, SND_LIST, &names, hot);
        self.button(f, SOUND_CLOSE, "Fechar", false);
        if self.cats.is_empty() {
            self.label(f, CAT_LIST.0 + 6, CAT_LIST.1 + 4, "sem MAG100.SND", BLACK);
        }
    }

    // ----- arquivos -----

    /// Há algo a guardar: alguma figura e (mudança ou nada guardado ainda).
    pub fn needs_save(&self) -> bool {
        self.count() > 0 && self.changed
    }

    /// Grava `animacao-AAAAMMDD-HHMMSS/` na pasta de desenhos (figuras, JSON e GIF)
    /// e a cópia `ultimo-animacao/`. Devolve a pasta com data.
    pub fn save(&mut self, drawings: &Path, now: u64) -> Result<PathBuf, String> {
        let stem = crate::paint::store::timestamp(now);
        let named = drawings.join(format!("animacao-{stem}"));
        self.write_dir(&named)?;
        let last = drawings.join(LAST_DIR);
        self.write_dir(&last)?;
        self.changed = false;
        Ok(named)
    }

    fn write_dir(&self, dir: &Path) -> Result<(), String> {
        let fail =
            |e: std::io::Error, p: &Path| format!("não consegui gravar {}: {e}", p.display());
        std::fs::create_dir_all(dir).map_err(|e| fail(e, dir))?;
        // apaga figuras antigas da cópia "ultimo"
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if n.starts_with("figura-") && n.ends_with(".png") {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        let order = self.filled();
        let mut json = String::from("{\n  \"figuras\": [\n");
        for (i, &k) in order.iter().enumerate() {
            let s = self.slots[k].as_ref().expect("espaço cheio");
            let p = dir.join(format!("figura-{}.png", i + 1));
            save_png(&s.canvas, &self.pal, &p).map_err(|e| fail(e, &p))?;
            json.push_str(&format!(
                "    {{\"arquivo\": \"{}\", \"efeito\": \"{}\", \"som\": \"{}\", \"legenda\": \"{}\"}}{}\n",
                json_escape(&s.file),
                s.effect.name(),
                s.sound.as_ref().map(SoundPick::to_key).unwrap_or_default(),
                json_escape(&s.caption),
                if i + 1 < order.len() { "," } else { "" }
            ));
        }
        json.push_str(&format!(
            "  ],\n  \"intervalo_ms\": {},\n  \"espacos\": {}\n}}\n",
            self.interval_ms(),
            self.slots.len()
        ));
        let p = dir.join(JSON_NAME);
        std::fs::write(&p, json).map_err(|e| fail(e, &p))?;
        let p = dir.join(GIF_NAME);
        std::fs::write(&p, self.gif()).map_err(|e| fail(e, &p))?;
        Ok(())
    }

    /// GIF animado com as transições renderizadas.
    pub fn gif(&self) -> Vec<u8> {
        let order = self.filled();
        let mut rng = self.rng.clone();
        let interval_cs = (self.interval_ms() / 10).max(1) as u16;
        let trans_cs = ((self.transition_secs() * 100.0) as u16).max(STEPS as u16);
        let mut frames: Vec<(Canvas, u16)> = Vec::new();
        for (i, &k) in order.iter().enumerate() {
            let s = self.slots[k].as_ref().expect("espaço cheio");
            let prev = (i > 0).then(|| &self.slots[order[i - 1]].as_ref().expect("cheio").canvas);
            let mut effect = s.effect;
            if effect == Transition::Dice {
                effect = Transition::ALL[(rng.next() as usize) % (Transition::ALL.len() - 1)];
            }
            if effect != Transition::Cut && prev.is_some() {
                for step in 0..STEPS {
                    frames.push((self.compose(prev, s, effect, step), trans_cs / STEPS as u16));
                }
                frames.push((
                    self.compose(prev, s, effect, STEPS),
                    interval_cs.saturating_sub(trans_cs).max(1),
                ));
            } else {
                frames.push((self.compose(prev, s, Transition::Cut, STEPS), interval_cs));
            }
        }
        let gf: Vec<gif::Frame<'_>> = frames
            .iter()
            .map(|(c, d)| gif::Frame {
                pixels: &c.px,
                delay_cs: *d,
            })
            .collect();
        gif::encode(CANVAS_W as u16, CANVAS_H as u16, &self.pal, &gf)
    }

    /// Reabre uma pasta gravada por [`Flipbook::save`]. Falso se não há nada lá.
    pub fn load(&mut self, dir: &Path) -> bool {
        let Ok(text) = std::fs::read_to_string(dir.join(JSON_NAME)) else {
            return false;
        };
        let mut slots: Vec<Option<Slot>> = Vec::new();
        for (i, obj) in json_objects(&text).into_iter().enumerate() {
            let Some(c) = load_png(&dir.join(format!("figura-{}.png", i + 1)), &self.pal) else {
                continue;
            };
            let file = json_field(obj, "arquivo").unwrap_or_default();
            let mut s = Slot::new(file, c);
            s.effect = json_field(obj, "efeito")
                .and_then(|e| Transition::from_name(&e))
                .unwrap_or(Transition::Cut);
            s.sound = json_field(obj, "som").and_then(|v| SoundPick::from_key(&v));
            s.caption = json_field(obj, "legenda").unwrap_or_default();
            slots.push(Some(s));
            if slots.len() == MAX_SLOTS {
                break;
            }
        }
        if slots.is_empty() {
            return false;
        }
        let spaces = text
            .split("\"espacos\":")
            .nth(1)
            .and_then(|r| r.trim().split(['}', ',', '\n']).next())
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);
        if let Some(ms) = text
            .split("\"intervalo_ms\":")
            .nth(1)
            .and_then(|r| r.trim().split(['}', ',', '\n']).next())
            .and_then(|v| v.trim().parse::<u32>().ok())
            && let Some(k) = SPEEDS_MS.iter().position(|&s| s == ms)
        {
            self.speed = k;
        }
        let n = spaces.max(slots.len()).clamp(INITIAL_SLOTS, MAX_SLOTS);
        slots.resize(n, None);
        self.slots = slots;
        self.selected = None;
        self.changed = false;
        true
    }
}

/// Dígitos coloridos 1–9 do `ARTIST.PIC` com a máscara (o 1 só tem cor e máscara;
/// os outros têm o mono entre as duas). O 10 é desenhado com texto.
const DIGITS: [(u16, u16); 9] = [
    (0x3a78, 0x3a79),
    (0x3a7b, 0x3a7d),
    (0x3a7f, 0x3a81),
    (0x3a83, 0x3a85),
    (0x3a87, 0x3a89),
    (0x3a8b, 0x3a8d),
    (0x3a8f, 0x3a91),
    (0x3a93, 0x3a95),
    (0x3a97, 0x3a99),
];

fn canvas_rect() -> Rect {
    (ORIGIN.0, ORIGIN.1, CANVAS_W as i32, CANVAS_H as i32)
}

/// Mistura `prev` → `next` no passo `step` de `STEPS` (0 = só `prev`, `STEPS` = só `next`).
pub fn transition(prev: &Canvas, next: &Canvas, effect: Transition, step: u32) -> Canvas {
    let step = step.min(STEPS);
    if step == 0 {
        return prev.clone();
    }
    if step >= STEPS || effect == Transition::Cut {
        return next.clone();
    }
    let (w, h) = (next.w as i64, next.h as i64);
    let mut out = prev.clone();
    for (k, v) in out.px.iter_mut().enumerate() {
        let (x, y) = (k as i64 % w, k as i64 / w);
        let show = match effect {
            Transition::Dissolve | Transition::Dice => {
                // dispersão determinística: hash do pixel comparado ao passo
                (pixel_hash(x as u32, y as u32) % STEPS) >= STEPS - step
            }
            Transition::WipeLeft => x >= w - w * step as i64 / STEPS as i64,
            Transition::WipeRight => x < w * step as i64 / STEPS as i64,
            Transition::WipeUp => y >= h - h * step as i64 / STEPS as i64,
            Transition::WipeDown => y < h * step as i64 / STEPS as i64,
            Transition::Cut => true,
        };
        if show {
            *v = next.px[k];
        }
    }
    out
}

/// Espalhamento determinístico de um pixel (misturador de 32 bits do MurmurHash3).
fn pixel_hash(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ 0x5bd1_e995;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn json_unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Objetos `{…}` da lista `figuras` (um por linha, como `write_dir` grava).
fn json_objects(text: &str) -> Vec<&str> {
    text.lines()
        .map(|l| l.trim().trim_end_matches(','))
        .filter(|l| l.starts_with('{') && l.ends_with('}') && l.contains("\"arquivo\""))
        .collect()
}

/// Valor de texto de `"chave": "…"` num objeto de uma linha.
fn json_field(obj: &str, key: &str) -> Option<String> {
    let k = format!("\"{key}\": \"");
    let i = obj.find(&k)? + k.len();
    let rest = &obj[i..];
    let mut end = None;
    let mut esc = false;
    for (j, c) in rest.char_indices() {
        if esc {
            esc = false;
        } else if c == '\\' {
            esc = true;
        } else if c == '"' {
            end = Some(j);
            break;
        }
    }
    Some(json_unescape(&rest[..end?]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    fn figure(v: u8) -> Canvas {
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(v);
        c
    }

    fn three(a: &Assets) -> Flipbook {
        let mut fb = Flipbook::open(None, a);
        for v in 1..=3 {
            assert!(fb.place(format!("f{v}.png"), figure(10 + v)));
        }
        fb
    }

    #[test]
    fn place_fills_in_order_and_stops_at_the_last_space() {
        let (_d, a) = empty_assets();
        let mut fb = Flipbook::open(None, &a);
        assert_eq!(fb.slot_count(), INITIAL_SLOTS);
        for v in 0..INITIAL_SLOTS as u8 {
            assert!(fb.place(String::new(), figure(v)));
        }
        assert!(!fb.place(String::new(), figure(9)));
        assert_eq!(fb.count(), INITIAL_SLOTS);
        fb.remove(1);
        assert_eq!(fb.filled().len(), INITIAL_SLOTS - 1);
        assert!(fb.place(String::new(), figure(9)));
        assert_eq!(fb.slots[1].as_ref().unwrap().canvas.px[0], 9);
    }

    #[test]
    fn add_slot_only_when_all_are_filled_up_to_ten() {
        let (_d, a) = empty_assets();
        let mut fb = three(&a);
        // há um espaço vazio: o Rex avisa e nada muda
        assert!(!fb.add_slot());
        assert_eq!(fb.slot_count(), INITIAL_SLOTS);
        assert!(
            fb.notice
                .as_ref()
                .is_some_and(|(m, _)| m.contains("Primeiro preencha"))
        );
        assert!(fb.place(String::new(), figure(4)));
        fb.click(ADD_BTN.0 + 2, ADD_BTN.1 + 2, None, &a);
        assert_eq!(fb.slot_count(), INITIAL_SLOTS + 1);
        assert!(fb.slots[INITIAL_SLOTS].is_none());
        // até 10
        for v in 0..10 {
            fb.place(String::new(), figure(v));
            fb.add_slot();
        }
        assert_eq!(fb.slot_count(), MAX_SLOTS);
        assert_eq!(fb.count(), MAX_SLOTS);
        assert!(!fb.add_slot());
        // o décimo espaço existe na tela e na folha de impressão
        let mut f = crate::Frame::new(640, 480);
        fb.draw_canvas(&mut f, &a);
        let sheet = fb.print_sheet();
        let t = Flipbook::thumb_rect(9);
        assert_eq!(
            sheet.px
                [((t.1 - ORIGIN.1 + 5) as u32 * CANVAS_W + (t.0 - ORIGIN.0 + 5) as u32) as usize],
            5
        );
    }

    #[test]
    fn drag_moves_swaps_and_removes() {
        let (_d, a) = empty_assets();
        let mut fb = three(&a);
        fb.click(
            Flipbook::cell_rect(2).0 + 10,
            Flipbook::cell_rect(2).1 + 10,
            None,
            &a,
        );
        assert_eq!(fb.selected(), None);
        // arrasta a figura 1 para o espaço vazio 4: move
        let (x, y) = (Flipbook::cell_rect(0).0 + 10, Flipbook::cell_rect(0).1 + 10);
        let r = Flipbook::cell_rect(3);
        fb.click(x, y, None, &a);
        fb.mouse_move(x + 2, y + 2);
        assert!(fb.dragging().is_none(), "ainda não andou o bastante");
        fb.mouse_move(r.0 + 10, r.1 + 10);
        assert_eq!(fb.dragging().map(|d| d.0), Some(0));
        fb.mouse_up(r.0 + 10, r.1 + 10);
        assert!(fb.slots[0].is_none());
        assert_eq!(fb.slots[3].as_ref().unwrap().canvas.px[0], 11);
        assert_eq!(fb.selected(), Some(3));
        // arrasta a 4 sobre a 2: troca
        let r2 = Flipbook::cell_rect(1);
        fb.click(r.0 + 10, r.1 + 10, None, &a);
        fb.mouse_move(r2.0 + 10, r2.1 + 10);
        fb.mouse_up(r2.0 + 10, r2.1 + 10);
        assert_eq!(fb.slots[1].as_ref().unwrap().canvas.px[0], 11);
        assert_eq!(fb.slots[3].as_ref().unwrap().canvas.px[0], 12);
        // soltar fora de tudo não muda nada; soltar na pasta fechada tira
        fb.click(r2.0 + 10, r2.1 + 10, None, &a);
        fb.mouse_move(5, 300);
        fb.mouse_up(5, 300);
        assert_eq!(fb.count(), 3);
        fb.click(r2.0 + 10, r2.1 + 10, None, &a);
        fb.mouse_move(FOLDER_BTN.0 + 5, FOLDER_BTN.1 + 5);
        fb.mouse_up(FOLDER_BTN.0 + 5, FOLDER_BTN.1 + 5);
        assert!(fb.slots[1].is_none());
        assert_eq!(fb.count(), 2);
        // um clique simples (sem andar) continua só escolhendo
        fb.click(r.0 + 10, r.1 + 10, None, &a);
        fb.mouse_up(r.0 + 10, r.1 + 10);
        assert_eq!(fb.selected(), Some(3));
        assert_eq!(fb.count(), 2);
    }

    #[test]
    fn drag_from_folder_lands_on_the_chosen_space() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = a.palettes(Program::Artist).pal256.clone();
        save_png(&figure(5), &pal, &dir.path().join("desenho-1.png")).unwrap();
        let mut fb = Flipbook::open(Some(dir.path()), &a);
        fb.click(FOLDER_BTN.0 + 2, FOLDER_BTN.1 + 2, Some(dir.path()), &a);
        let c = Flipbook::folder_cell_rect(0);
        let target = Flipbook::cell_rect(2);
        fb.click(c.0 + 5, c.1 + 5, Some(dir.path()), &a);
        fb.mouse_move(target.0 + 20, target.1 + 20);
        // a pasta fecha para mostrar os espaços enquanto arrasta
        assert!(!fb.folder_open());
        fb.mouse_up(target.0 + 20, target.1 + 20);
        assert!(fb.slots[0].is_none());
        assert_eq!(fb.slots[2].as_ref().unwrap().canvas.px[0], 5);
        assert_eq!(fb.count(), 1);
    }

    #[test]
    fn double_click_opens_the_figure_and_takes_it_back_changed() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = a.palettes(Program::Artist).pal256.clone();
        let mut fb = three(&a);
        fb.set_caption(1, "oi");
        // desescolhe a 3 (colocar deixa a última escolhida; escolhida + outra = troca)
        let r3 = Flipbook::cell_rect(2);
        fb.click(r3.0 + 10, r3.1 + 10, Some(dir.path()), &a);
        let r = Flipbook::cell_rect(1);
        fb.click(r.0 + 10, r.1 + 10, Some(dir.path()), &a);
        fb.update(0.1);
        let fx = fb.click(r.0 + 10, r.1 + 10, Some(dir.path()), &a);
        assert_eq!(fx, [Effect::Push(SceneId::FlipbookFigure)]);
        assert_eq!(fb.editing(), Some(1));
        let path = Flipbook::edit_path(dir.path());
        assert_eq!(load_png(&path, &pal).unwrap().px[0], 12);
        // o update do próprio clique não reabre; o seguinte (a volta) sim
        fb.update(0.0);
        assert_eq!(fb.editing(), Some(1));
        save_png(&figure(77), &pal, &path).unwrap();
        fb.update(0.0);
        assert_eq!(fb.editing(), None);
        assert_eq!(fb.slots[1].as_ref().unwrap().canvas.px[0], 77);
        assert_eq!(fb.slots[1].as_ref().unwrap().caption, "oi");
        assert!(!path.exists());
        assert!(fb.needs_save());
        // dois cliques separados por mais tempo não abrem
        fb.changed = false;
        fb.click(r.0 + 10, r.1 + 10, Some(dir.path()), &a);
        fb.update(1.0);
        assert!(
            fb.click(r.0 + 10, r.1 + 10, Some(dir.path()), &a)
                .is_empty()
        );
        // sem pasta de desenhos: aviso, sem cena
        fb.click(r.0 + 10, r.1 + 10, None, &a);
        assert!(fb.click(r.0 + 10, r.1 + 10, None, &a).is_empty());
        assert!(fb.notice.is_some());
    }

    #[test]
    fn play_at_1000ms_shows_second_figure_between_1_and_2_s() {
        let (_d, a) = empty_assets();
        let mut fb = three(&a);
        assert_eq!(fb.interval_ms(), 1000);
        fb.play();
        assert_eq!(fb.shown(), Some(0));
        let mut t = 0.0;
        while t < 0.95 {
            fb.update(0.05);
            t += 0.05;
            assert_eq!(fb.shown(), Some(0), "t={t}");
        }
        fb.update(0.1);
        assert_eq!(fb.shown(), Some(1));
        fb.update(0.9);
        assert_eq!(fb.shown(), Some(1));
        fb.update(0.1);
        assert_eq!(fb.shown(), Some(2));
        fb.update(1.0);
        assert_eq!(fb.shown(), Some(0), "laço");
        fb.stop();
        assert_eq!(fb.shown(), None);
    }

    #[test]
    fn dissolve_changes_only_part_of_the_pixels_midway() {
        let a = figure(1);
        let b = figure(2);
        let mid = transition(&a, &b, Transition::Dissolve, STEPS / 2);
        let changed = mid.px.iter().filter(|&&v| v == 2).count();
        let total = mid.px.len();
        assert!(
            changed > total / 3 && changed < total * 2 / 3,
            "{changed}/{total}"
        );
        assert_eq!(transition(&a, &b, Transition::Dissolve, 0), a);
        assert_eq!(transition(&a, &b, Transition::Dissolve, STEPS), b);
        let wipe = transition(&a, &b, Transition::WipeRight, STEPS / 2);
        assert_eq!(wipe.get(0, 0), Some(2));
        assert_eq!(wipe.get(CANVAS_W as i32 - 1, 0), Some(1));
        assert_eq!(transition(&a, &b, Transition::Cut, 1), b);
    }

    #[test]
    fn playing_frame_dissolves_then_shows_caption() {
        let (_d, a) = empty_assets();
        let mut fb = three(&a);
        fb.set_effect(1, Transition::Dissolve);
        fb.set_caption(1, "Oi");
        fb.play();
        fb.update(1.0);
        // começo da transição: mistura das figuras 1 e 2
        fb.update(0.2);
        let f = fb.frame().unwrap();
        assert!(f.px.contains(&11) && f.px.contains(&12));
        // fim da transição: figura 2 com a faixa branca da legenda
        fb.update(0.5);
        let f = fb.frame().unwrap();
        assert!(!f.px.contains(&11));
        assert_eq!(f.get(0, CANVAS_H as i32 - 1), Some(WHITE));
        assert!(f.px.contains(&crate::paint::BLACK), "texto da legenda");
    }

    #[test]
    fn save_writes_pngs_json_gif_and_reloads() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut fb = three(&a);
        fb.set_effect(2, Transition::Dissolve);
        fb.set_caption(0, "Era \"uma\" vez");
        fb.set_sound(
            1,
            Some(SoundPick {
                category: "Agudo".into(),
                index: 3,
            }),
        );
        fb.set_speed(4);
        fb.place("f4.png".into(), figure(14));
        assert!(fb.add_slot());
        fb.remove(3);
        assert_eq!(fb.slot_count(), 5);
        assert!(fb.needs_save());
        let named = fb.save(dir.path(), 0).unwrap();
        assert!(!fb.needs_save());
        assert_eq!(named.file_name().unwrap(), "animacao-19700101-000000");
        for d in [named.clone(), dir.path().join(LAST_DIR)] {
            for n in [
                "figura-1.png",
                "figura-2.png",
                "figura-3.png",
                JSON_NAME,
                GIF_NAME,
            ] {
                assert!(d.join(n).exists(), "{}", d.join(n).display());
            }
            assert!(!d.join("figura-4.png").exists());
        }
        let json = std::fs::read_to_string(named.join(JSON_NAME)).unwrap();
        assert!(json.contains("\"intervalo_ms\": 250"));
        assert!(json.contains("\"espacos\": 5"));
        assert!(json.contains("\"efeito\": \"dissolver\""));
        assert!(json.contains("\"som\": \"Agudo/3\""));
        let gif = std::fs::read(named.join(GIF_NAME)).unwrap();
        assert_eq!(&gif[..6], b"GIF89a");
        assert_eq!(gif.last(), Some(&0x3B));

        let mut back = Flipbook::open(Some(dir.path()), &a);
        assert_eq!(back.count(), 3);
        assert_eq!(back.slot_count(), 5);
        assert_eq!(back.speed(), 4);
        let s0 = back.slots[0].as_ref().unwrap();
        assert_eq!(s0.caption, "Era \"uma\" vez");
        assert_eq!(s0.file, "f1.png");
        assert_eq!(s0.canvas.px[0], 11);
        assert_eq!(
            back.slots[1]
                .as_ref()
                .unwrap()
                .sound
                .as_ref()
                .unwrap()
                .to_key(),
            "Agudo/3"
        );
        assert_eq!(back.slots[2].as_ref().unwrap().effect, Transition::Dissolve);
        assert!(!back.needs_save());
        back.remove(0);
        assert!(back.needs_save());
    }

    #[test]
    fn open_without_folder_is_empty() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let fb = Flipbook::open(Some(dir.path()), &a);
        assert_eq!(fb.count(), 0);
        assert!(!fb.needs_save());
        let mut fb = fb;
        assert!(fb.play().is_empty());
        assert!(fb.shown().is_none());
    }

    #[test]
    fn strip_clicks_play_pause_stop_and_speed() {
        let (_d, a) = empty_assets();
        let mut fb = three(&a);
        fb.click(PLAY.0 + 5, PLAY.1 + 5, None, &a);
        assert!(fb.is_playing());
        fb.click(PAUSE.0 + 5, PAUSE.1 + 5, None, &a);
        assert!(fb.is_paused());
        fb.update(5.0);
        assert_eq!(fb.shown(), Some(0));
        fb.click(NEXT_BTN.0 + 5, NEXT_BTN.1 + 5, None, &a);
        assert_eq!(fb.shown(), Some(1));
        fb.click(STOP.0 + 5, STOP.1 + 5, None, &a);
        assert!(fb.shown().is_none());
        // régua: parada 4 (coelho)
        fb.click(TRACK_STOP0 + 4 * TRACK_STEP, TRACK.1 + 5, None, &a);
        assert_eq!(fb.interval_ms(), 250);
        fb.click(TRACK_STOP0, TRACK.1 + 5, None, &a);
        assert_eq!(fb.interval_ms(), 4000);
    }

    #[test]
    fn slot_clicks_select_swap_and_delete() {
        let (_d, a) = empty_assets();
        let mut fb = three(&a);
        // colocar deixa o último espaço escolhido; clicar nele de novo desescolhe
        assert_eq!(fb.selected(), Some(2));
        let e = Flipbook::cell_rect(2);
        fb.click(e.0 + 10, e.1 + 10, None, &a);
        assert_eq!(fb.selected(), None);
        let (x, y) = (Flipbook::cell_rect(0).0 + 10, Flipbook::cell_rect(0).1 + 10);
        fb.click(x, y, None, &a);
        assert_eq!(fb.selected(), Some(0));
        let r = Flipbook::cell_rect(2);
        fb.click(r.0 + 10, r.1 + 10, None, &a);
        assert_eq!(fb.slots[0].as_ref().unwrap().canvas.px[0], 13);
        assert_eq!(fb.slots[2].as_ref().unwrap().canvas.px[0], 11);
        assert_eq!(fb.selected(), Some(2));
        fb.key(Key::Delete);
        assert!(fb.slots[2].is_none());
        assert_eq!(fb.count(), 2);
        // espaço vazio: nada escolhido
        fb.click(r.0 + 10, r.1 + 10, None, &a);
        assert_eq!(fb.selected(), None);
        // escolhido + espaço vazio: move a figura para lá
        fb.click(x, y, None, &a);
        fb.click(r.0 + 10, r.1 + 10, None, &a);
        assert!(fb.slots[0].is_none());
        assert_eq!(fb.slots[2].as_ref().unwrap().canvas.px[0], 13);
        assert_eq!(fb.selected(), Some(2));
    }

    #[test]
    fn caption_and_effect_buttons() {
        let (_d, a) = empty_assets();
        let mut fb = three(&a);
        let r = Flipbook::cell_rect(1);
        fb.click(r.0 + 10, r.1 + 10, None, &a);
        fb.click(EFFECT_BTN.0 + 2, EFFECT_BTN.1 + 2, None, &a);
        assert_eq!(fb.slots[1].as_ref().unwrap().effect, Transition::Dissolve);
        fb.click(CAPTION_BTN.0 + 2, CAPTION_BTN.1 + 2, None, &a);
        assert_eq!(fb.caption_editing(), Some(""));
        fb.text("Olá!");
        fb.key(Key::Backspace);
        fb.key(Key::Enter);
        assert_eq!(fb.caption_editing(), None);
        assert_eq!(fb.slots[1].as_ref().unwrap().caption, "Olá");
    }

    #[test]
    fn folder_lists_drawings_and_places_on_click() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = a.palettes(Program::Artist).pal256.clone();
        save_png(&figure(5), &pal, &dir.path().join("desenho-1.png")).unwrap();
        save_png(&figure(0), &pal, &dir.path().join("ultimo.png")).unwrap();
        let mut fb = Flipbook::open(Some(dir.path()), &a);
        fb.click(FOLDER_BTN.0 + 2, FOLDER_BTN.1 + 2, Some(dir.path()), &a);
        assert!(fb.folder_open());
        assert_eq!(fb.folder.as_ref().unwrap().entries.len(), 1);
        let r = Flipbook::folder_cell_rect(0);
        fb.click(r.0 + 5, r.1 + 5, Some(dir.path()), &a);
        assert_eq!(fb.count(), 1);
        assert_eq!(fb.slots[0].as_ref().unwrap().file, "desenho-1.png");
        fb.click(FOLDER_CLOSE.0 + 2, FOLDER_CLOSE.1 + 2, Some(dir.path()), &a);
        assert!(!fb.folder_open());
    }

    #[test]
    fn gif_has_transition_frames() {
        let (_d, a) = empty_assets();
        let mut fb = three(&a);
        fb.set_effect(1, Transition::WipeDown);
        let gif = fb.gif();
        // 1 (corte) + 13 (cortina) + 1 (corte) descritores de imagem
        let (w, h) = (
            (CANVAS_W as u16).to_le_bytes(),
            (CANVAS_H as u16).to_le_bytes(),
        );
        let images = gif
            .windows(11)
            .filter(|b| {
                b[0] == 0x2C
                    && b[1..5] == [0, 0, 0, 0]
                    && b[5..7] == w
                    && b[7..9] == h
                    && b[9] == 0
                    && b[10] == 8
            })
            .count();
        assert_eq!(images, 15);
    }

    #[test]
    fn json_fields_roundtrip_escapes() {
        let line =
            r#"{"arquivo": "a \"b\" c.png", "efeito": "dado", "som": "", "legenda": "x\\y"}"#;
        assert_eq!(json_field(line, "arquivo").unwrap(), "a \"b\" c.png");
        assert_eq!(json_field(line, "legenda").unwrap(), "x\\y");
        assert_eq!(json_field(line, "som").unwrap(), "");
        assert_eq!(json_escape("a\"b\\"), "a\\\"b\\\\");
    }
}
