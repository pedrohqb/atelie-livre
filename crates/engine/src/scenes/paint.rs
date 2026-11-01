//! Tela de pintura do Fine Artist: o quadro do Ateliê de Pintura com pincel,
//! lata de tinta, palavras, ovo desfazer, aspirador (limpar), cores e padrões.
//! Posições medidas no original (spec 2b, seção 4).

use super::projects::flipbook::Flipbook;
use super::projects::{Project, ProjectKind};
use super::sounds::{SPOTS_NAME, Sounds};
use super::stickers::Stickers;
use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::paint::geom::{
    ellipse_points, fill_ellipse_points, fill_rect_points, fill_triangle_points, polyline_points,
    rect_points, thicken, triangle_points,
};
use crate::paint::stamp::Stamp;
use crate::paint::store::{LAST_NAME, load_png, save_drawing};
use crate::paint::{
    CANVAS_H, CANVAS_W, Canvas, History, Ink, ORIGIN, Shape, balloon_pixels, effect_shape,
    line_points, page_shapes, text_pixels, text_size,
};
use crate::rng::XorShift;
use crate::scene::{Effect, Input, Key, KidCommon, MouseButton, Scene, SceneId};
use formats::fnt::Font;
use formats::palette::Rgb;
use std::path::PathBuf;
use std::rc::Rc;

pub use crate::paint::COLORS;
const SWATCHES: usize = COLORS.len();

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const DARK: [u8; 3] = [100, 100, 100];
const LIGHT: [u8; 3] = [140, 140, 140];
const ORANGE: [u8; 3] = [255, 140, 50];
const BROWN: [u8; 3] = [185, 101, 36];
const RED: [u8; 3] = [250, 0, 57];

const PAGE_COUNT: u8 = 6;
const STAMP_SPACING: i32 = 16;
const EXIT: (i32, i32, i32, i32) = (580, 18, 56, 88);
const ARROW_UP: (i32, i32, i32, i32) = (504, 58, 16, 14);
const ARROW_DOWN: (i32, i32, i32, i32) = (504, 90, 16, 14);
/// Carimbos da página 2 do pincel (`ARTIST.PIC`).
const STAMPS: [u16; 12] = [
    0x164b, 0x16a9, 0x16ab, 0x16ad, 0x16af, 0x16b1, 0x16b3, 0x16b5, 0x16b7, 0x16b9, 0x16bb, 0x16bd,
];
/// Carimbos de 21×21 dos efeitos 6 a 11 da página 3 (trilho, estrada, onda, filme, água, grama).
const EFFECT_STAMPS: [u16; 6] = [0x11a3, 0x11b9, 0x16bf, 0x16d6, 0x16e1, 0x16f1];
/// Botões das páginas 2, 3, 5 e 6 do pincel.
const PAGE2_BUTTONS: [u16; 12] = [
    0x17e5, 0x17e7, 0x17e9, 0x1814, 0x1817, 0x1819, 0x181b, 0x181d, 0x181f, 0x1821, 0x1823, 0x1825,
];
const PAGE3_BUTTONS: [u16; 12] = [
    0x17eb, 0x17ed, 0x17ef, 0x17f1, 0x17f3, 0x17f5, 0x17f7, 0x17f9, 0x17fb, 0x1827, 0x1829, 0x182b,
];
const PAGE5_BUTTONS: [u16; 12] = [
    0x17b5, 0x17cf, 0x17d1, 0x17d3, 0x17d5, 0x17d7, 0x17d9, 0x17db, 0x17dd, 0x17df, 0x17e1, 0x17e3,
];
const PAGE6_BUTTONS: [u16; 12] = [
    0x17fd, 0x17fe, 0x1800, 0x1802, 0x1804, 0x1806, 0x1808, 0x180a, 0x180c, 0x180e, 0x1810, 0x1812,
];
/// Sprays da página 6: (raio em pixels, pontos por atualização), do ralo ao denso.
const SPRAYS: [(i32, u32); 12] = [
    (6, 2),
    (6, 5),
    (6, 12),
    (10, 3),
    (10, 8),
    (10, 20),
    (14, 4),
    (14, 12),
    (14, 30),
    (18, 6),
    (18, 16),
    (18, 40),
];
/// Distância mínima entre vértices da poligonal (página 5, botão 4).
const POLY_STEP: i32 = 8;
/// Espessura da serpentina (página 5, botão 5).
const SERPENTINE_T: i32 = 3;
const BUCKET_BUTTONS: [u16; 10] = [
    0x1590, 0x1591, 0x1593, 0x1595, 0x1597, 0x1599, 0x159b, 0x159d, 0x159f, 0x15a1,
];
const VACUUM_BUTTONS: [u16; 8] = [
    0x1557, 0x1559, 0x1567, 0x1561, 0x155f, 0x155b, 0x155c, 0x1569,
];
/// Opções da ferramenta Palavra: texto simples ("FORMA") e balão de fala ("Oi!").
const WORDS_BUTTONS: [u16; 2] = [0x1797, 0x1799];
/// Período do piscar do cursor de texto, em segundos (meio período aceso).
const CARET_PERIOD: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Brush,
    Bucket,
    Stickers,
    Words,
    Sounds,
    Vacuum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Select(Tool),
    Undo,
    /// Imprimir: exporta o quadro em PNG e manda para a impressora (`lp`), se houver.
    Print,
    Unavailable(&'static str),
}

/// Ferramenta da barra: ícone normal e ativo, com as máscaras, e a largura do clique.
struct ToolDef {
    x: i32,
    y: i32,
    w: i32,
    normal: (u16, u16),
    active: (u16, u16),
    action: Action,
}

const TOOLS: [ToolDef; 10] = [
    ToolDef {
        x: 10,
        y: 10,
        w: 37,
        normal: (0x0495, 0x1001),
        active: (0x1013, 0x1015),
        action: Action::Select(Tool::Brush),
    },
    ToolDef {
        x: 57,
        y: 10,
        w: 42,
        normal: (0x1003, 0x1005),
        active: (0x1017, 0x1019),
        action: Action::Select(Tool::Bucket),
    },
    ToolDef {
        x: 113,
        y: 13,
        w: 32,
        normal: (0x101b, 0x1023),
        active: (0x1025, 0x1027),
        action: Action::Select(Tool::Stickers),
    },
    ToolDef {
        x: 157,
        y: 10,
        w: 34,
        normal: (0x1029, 0x102b),
        active: (0x102d, 0x102f),
        action: Action::Select(Tool::Sounds),
    },
    ToolDef {
        x: 211,
        y: 13,
        w: 35,
        normal: (0x1031, 0x103b),
        active: (0x1061, 0x1063),
        action: Action::Select(Tool::Words),
    },
    ToolDef {
        x: 257,
        y: 8,
        w: 44,
        normal: (0x103d, 0x103f),
        active: (0x103d, 0x103f),
        action: Action::Undo,
    },
    ToolDef {
        x: 306,
        y: 13,
        w: 45,
        normal: (0x100b, 0x100d),
        active: (0x100b, 0x100d),
        action: Action::Select(Tool::Vacuum),
    },
    ToolDef {
        x: 360,
        y: 9,
        w: 36,
        normal: (0x1045, 0x104b),
        active: (0x1065, 0x1067),
        action: Action::Unavailable("transformadores"),
    },
    ToolDef {
        x: 412,
        y: 12,
        w: 35,
        normal: (0x1007, 0x1009),
        active: (0x1069, 0x106b),
        action: Action::Unavailable("super-ferramentas"),
    },
    ToolDef {
        x: 458,
        y: 13,
        w: 42,
        normal: (0x104d, 0x104f),
        active: (0x106d, 0x106f),
        action: Action::Print,
    },
];

fn inside(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Artist, id)
}

/// Retângulo do `k`-ésimo botão de uma faixa que começa em `x0`.
fn option_rect(x0: i32, k: usize) -> (i32, i32, i32, i32) {
    (x0 + 37 * k as i32, 63, 32, 32)
}

fn color_rect(k: usize) -> (i32, i32, i32, i32) {
    (5, 110 + 9 * k as i32, 17, 9)
}

fn pattern_rect(k: usize) -> (i32, i32, i32, i32) {
    (618, 110 + 9 * k as i32, 17, 9)
}

fn canvas_rect() -> (i32, i32, i32, i32) {
    (ORIGIN.0, ORIGIN.1, CANVAS_W as i32, CANVAS_H as i32)
}

/// O que está sendo arrastado no quadro, em coordenadas do quadro.
enum Stroke {
    /// Páginas 1, 2 e 4: carimba a forma ao longo do caminho.
    Brush {
        last: (i32, i32),
        last_stamp: (i32, i32),
    },
    /// Página 5: forma elástica entre a âncora e o ponteiro, pintada só ao soltar.
    Geom {
        kind: usize,
        anchor: (i32, i32),
        pointer: (i32, i32),
        /// Vértices das curvas livres e da poligonal.
        verts: Vec<(i32, i32)>,
    },
    /// Página 6: borrifa pontos ao redor do ponteiro a cada atualização.
    Spray {
        rng: XorShift,
        pos: (i32, i32),
        radius: i32,
        dots: u32,
    },
}

/// Pixels da forma geométrica `kind` da página 5 entre `anchor` e `pointer`.
fn geom_pixels(
    kind: usize,
    anchor: (i32, i32),
    pointer: (i32, i32),
    verts: &[(i32, i32)],
) -> Vec<(i32, i32)> {
    match kind {
        0 => line_points(anchor, pointer),
        1 => {
            let mut v = line_points(anchor, pointer);
            v.extend(line_points(anchor, (pointer.0, anchor.1)));
            v
        }
        2 => polyline_points(verts, false),
        3 => polyline_points(verts, true),
        4 => {
            let mut v = polyline_points(verts, false);
            if let Some(&last) = verts.last() {
                v.extend(line_points(last, pointer));
            }
            v
        }
        5 => thicken(&polyline_points(verts, false), SERPENTINE_T),
        6 => fill_rect_points(anchor, pointer),
        7 => fill_ellipse_points(anchor, pointer),
        8 => fill_triangle_points(anchor, pointer),
        9 => rect_points(anchor, pointer),
        10 => ellipse_points(anchor, pointer),
        _ => triangle_points(anchor, pointer),
    }
}

/// Opções do aspirador, na ordem dos botões do original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VacuumMode {
    /// Aspira tinta: arrasta um retângulo, que vai para a "área de transferência".
    Cut,
    /// Aspira adesivos (ainda não disponível).
    Stickers,
    /// Cola o que foi aspirado ou copiado, centrado no clique.
    Paste,
    /// Aspira-área: apaga a região contínua da cor clicada.
    Area,
    /// Aspira-cópias: copia um retângulo arrastado.
    Copy,
    /// Aspira-cores: apaga só a cor ativa dentro do retângulo arrastado.
    Color,
    /// Remove-sons (ainda não disponível).
    Sounds,
    /// Jogar tudo fora: limpa o quadro.
    All,
}

const VACUUM_MODES: [VacuumMode; 8] = [
    VacuumMode::Cut,
    VacuumMode::Stickers,
    VacuumMode::Paste,
    VacuumMode::Area,
    VacuumMode::Copy,
    VacuumMode::Color,
    VacuumMode::Sounds,
    VacuumMode::All,
];

/// Texto em edição: canto superior esquerdo da linha (coordenadas do quadro).
struct WordsEdit {
    x: i32,
    y: i32,
    text: String,
    /// Tempo acumulado para o piscar do cursor.
    blink: f32,
}

pub struct Paint {
    common: KidCommon,
    vacuum: VacuumMode,
    /// Canto inicial de um arrasto do aspirador (coordenadas do quadro).
    vacuum_anchor: Option<(i32, i32)>,
    /// Trecho aspirado ou copiado.
    clipboard: Option<Stamp>,
    drawings: Option<PathBuf>,
    pal: Vec<Rgb>,
    patterns: Vec<[u8; 8]>,
    stamps: Vec<Shape>,
    /// Carimbos dos efeitos 6 a 11 da página 3.
    effect_stamps: Vec<Shape>,
    canvas: Canvas,
    history: History,
    tool: Tool,
    color: usize,
    pattern: usize,
    page: u8,
    /// Forma escolhida: (página, botão).
    brush: (u8, usize),
    bucket_option: usize,
    /// Opção da Palavra: 0 = texto simples, 1 = balão de fala.
    words_option: usize,
    words: Option<WordsEdit>,
    /// Fonte das palavras: "Pablo Graphos" 16 pt, senão 8 pt; `None` usa a 8×8 reserva.
    font: Option<Rc<Font>>,
    /// Forma correspondente a `brush`, guardada para não recriar a cada movimento.
    shape: Option<Shape>,
    stroke: Option<Stroke>,
    changed: bool,
    stickers: Stickers,
    sounds: Sounds,
    mouse: (i32, i32),
    /// Projeto da Oficina em andamento, se a tela foi aberta por ele.
    project: Option<Project>,
}

impl Paint {
    pub fn new(drawings: Option<PathBuf>, assets: &Assets) -> Paint {
        let pal = assets.palettes(Program::Artist).pal256.clone();
        let mut patterns = assets.patterns(Program::Artist);
        patterns.resize(SWATCHES.max(patterns.len()), [0; 8]);
        let stamps = STAMPS
            .iter()
            .map(|&id| Shape::from_stamp(&assets.image(&pic(id))))
            .collect();
        let canvas = drawings
            .as_ref()
            .and_then(|d| load_png(&d.join(LAST_NAME), &pal))
            .unwrap_or_else(|| Canvas::new(CANVAS_W, CANVAS_H));
        let mut p = Paint {
            common: KidCommon::new(None),
            drawings,
            pal,
            patterns,
            stamps,
            effect_stamps: EFFECT_STAMPS
                .iter()
                .map(|&id| Shape::from_stamp(&assets.image(&pic(id))))
                .collect(),
            canvas,
            history: History::default(),
            tool: Tool::Brush,
            color: SWATCHES - 1,
            pattern: 0,
            page: 1,
            brush: (1, 0),
            bucket_option: 0,
            words_option: 0,
            words: None,
            font: assets.font(16).or_else(|| assets.font(8)),
            shape: None,
            stroke: None,
            changed: false,
            stickers: Stickers::new(assets),
            sounds: Sounds::new(assets),
            mouse: (0, 0),
            vacuum: VacuumMode::Cut,
            vacuum_anchor: None,
            clipboard: None,
            project: None,
        };
        p.set_brush(1, 0);
        if let Some(d) = &p.drawings {
            p.sounds.load_spots(&d.join(SPOTS_NAME));
        }
        p
    }

    /// Abre a tela como um projeto da Oficina: reabre o último trabalho desse
    /// tipo (`ultimo-<tipo>.png`), se houver, senão um quadro novo.
    pub fn new_project(kind: ProjectKind, drawings: Option<PathBuf>, assets: &Assets) -> Paint {
        let mut p = Paint::new(None, assets);
        p.drawings = drawings;
        if kind == ProjectKind::Flipbook {
            p.project = Some(Project::Flipbook(Box::new(Flipbook::open(
                p.drawings.as_deref(),
                assets,
            ))));
            return p;
        }
        if let Some(c) = p
            .drawings
            .as_ref()
            .and_then(|d| load_png(&d.join(kind.last_name()), &p.pal))
        {
            p.canvas = c;
        }
        let project = Project::new(kind);
        project.prepare(&mut p.canvas);
        p.project = Some(project);
        p
    }

    /// Desenho animado em andamento, se a tela é esse projeto.
    pub fn flipbook(&self) -> Option<&Flipbook> {
        match &self.project {
            Some(Project::Flipbook(fb)) => Some(fb),
            _ => None,
        }
    }

    pub fn project(&self) -> Option<&Project> {
        self.project.as_ref()
    }

    /// Grava `impressao-AAAAMMDD-HHMMSS.png` na pasta de desenhos e tenta `lp`.
    fn print(&self) -> Effect {
        let Some(dir) = &self.drawings else {
            return Effect::Log("imprimir: sem pasta de desenhos".into());
        };
        let name =
            crate::paint::store::timestamp_name(now_secs()).replace("desenho-", "impressao-");
        let path = dir.join(name);
        let page = match &self.project {
            Some(p) => p.print_page(&self.canvas, &self.pal),
            None => self.canvas.clone(),
        };
        if let Err(e) = crate::paint::store::save_png(&page, &self.pal, &path) {
            return Effect::Log(format!(
                "imprimir: não consegui gravar {}: {e}",
                path.display()
            ));
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

    pub fn sounds(&self) -> &Sounds {
        &self.sounds
    }

    fn set_brush(&mut self, page: u8, k: usize) {
        self.brush = (page, k);
        self.shape = match page {
            2 => self.stamps.get(k).cloned(),
            3 if k < 6 => Some(effect_shape(k)),
            3 => self.effect_stamps.get(k - 6).cloned(),
            5 | 6 => None,
            p => page_shapes(p).into_iter().nth(k),
        };
    }

    pub fn tool(&self) -> Tool {
        self.tool
    }

    /// Índice da paleta de 256 cores da cor ativa.
    pub fn color(&self) -> u8 {
        COLORS[self.color]
    }

    pub fn pattern(&self) -> usize {
        self.pattern
    }

    pub fn page(&self) -> u8 {
        self.page
    }

    /// Forma escolhida: (página, botão).
    pub fn brush(&self) -> (u8, usize) {
        self.brush
    }

    pub fn canvas(&self) -> &Canvas {
        &self.canvas
    }

    /// Ferramenta de adesivos (rolos, tira, adesivo na mão, animação).
    pub fn stickers(&self) -> &Stickers {
        &self.stickers
    }

    /// Texto da Palavra em edição, se houver.
    pub fn words_text(&self) -> Option<&str> {
        self.words.as_ref().map(|w| w.text.as_str())
    }

    fn start_words(&mut self, cx: i32, cy: i32) {
        self.words = Some(WordsEdit {
            x: cx,
            y: cy,
            text: String::new(),
            blink: 0.0,
        });
    }

    /// Grava no quadro o texto em edição (uma ação de desfazer); texto vazio não pinta nada.
    fn commit_words(&mut self) {
        let Some(w) = self.words.take() else {
            return;
        };
        if w.text.is_empty() {
            return;
        }
        let font = self.font.clone();
        let font = font.as_deref();
        self.history.record(&self.canvas);
        if self.words_option == 1 {
            self.canvas.draw_balloon(font, w.x, w.y, &w.text);
        }
        self.canvas
            .draw_text(font, w.x, w.y, &w.text, COLORS[self.color]);
        self.changed = true;
    }

    fn type_text(&mut self, text: &str) {
        let Some(w) = self.words.as_mut() else {
            return;
        };
        w.text.extend(
            text.chars()
                .filter(|c| !c.is_control() && formats::cp1252::encode_char(*c).is_some()),
        );
        w.blink = 0.0;
    }

    fn key(&mut self, key: Key) {
        if self.words.is_none() {
            return;
        }
        match key {
            Key::Enter => self.commit_words(),
            Key::Backspace => {
                if let Some(w) = self.words.as_mut() {
                    w.text.pop();
                    w.blink = 0.0;
                }
            }
            _ => {}
        }
    }

    fn ink(&self) -> Ink {
        Ink {
            color: COLORS[self.color],
            pattern: self.patterns[self.pattern],
        }
    }

    fn finish_sticker(&mut self) {
        if self.stickers.finish(&mut self.canvas, &mut self.history) {
            self.changed = true;
        }
    }

    fn option_count(&self) -> usize {
        match self.tool {
            Tool::Stickers | Tool::Sounds => 0,
            Tool::Brush => 12,
            Tool::Bucket => BUCKET_BUTTONS.len(),
            Tool::Words => WORDS_BUTTONS.len(),
            Tool::Vacuum => VACUUM_BUTTONS.len(),
        }
    }

    fn options_x0(&self) -> i32 {
        if self.tool == Tool::Bucket { 113 } else { 25 }
    }

    fn start_stroke(&mut self, cx: i32, cy: i32) {
        match self.brush.0 {
            5 => {
                self.stroke = Some(Stroke::Geom {
                    kind: self.brush.1,
                    anchor: (cx, cy),
                    pointer: (cx, cy),
                    verts: vec![(cx, cy)],
                });
            }
            6 => {
                let (radius, dots) = SPRAYS[self.brush.1.min(SPRAYS.len() - 1)];
                self.history.record(&self.canvas);
                let seed = (cx as u32).wrapping_mul(2654435761)
                    ^ (cy as u32).wrapping_mul(40503)
                    ^ (now_secs() as u32);
                self.stroke = Some(Stroke::Spray {
                    rng: XorShift::new(seed),
                    pos: (cx, cy),
                    radius,
                    dots,
                });
                self.spray_tick();
            }
            _ => {
                let Some(shape) = self.shape.as_ref() else {
                    return;
                };
                self.history.record(&self.canvas);
                let ink = self.ink();
                self.canvas.stamp(shape, cx, cy, &ink);
                self.changed = true;
                self.stroke = Some(Stroke::Brush {
                    last: (cx, cy),
                    last_stamp: (cx, cy),
                });
            }
        }
    }

    fn extend_stroke(&mut self, cx: i32, cy: i32) {
        let ink = self.ink();
        let spaced = matches!(self.brush.0, 2 | 3);
        match self.stroke.as_mut() {
            Some(Stroke::Brush { last, last_stamp }) => {
                let Some(shape) = self.shape.as_ref() else {
                    return;
                };
                for p in line_points(*last, (cx, cy)).into_iter().skip(1) {
                    if spaced {
                        let (dx, dy) = (p.0 - last_stamp.0, p.1 - last_stamp.1);
                        if dx * dx + dy * dy < STAMP_SPACING * STAMP_SPACING {
                            continue;
                        }
                        *last_stamp = p;
                    }
                    self.canvas.stamp(shape, p.0, p.1, &ink);
                }
                *last = (cx, cy);
            }
            Some(Stroke::Geom {
                kind,
                pointer,
                verts,
                ..
            }) => {
                *pointer = (cx, cy);
                match *kind {
                    2 | 3 | 5 => verts.push((cx, cy)),
                    4 => {
                        let l = verts.last().copied().unwrap_or((cx, cy));
                        let (dx, dy) = (cx - l.0, cy - l.1);
                        if dx * dx + dy * dy >= POLY_STEP * POLY_STEP {
                            verts.push((cx, cy));
                        }
                    }
                    _ => {}
                }
            }
            Some(Stroke::Spray { pos, .. }) => *pos = (cx, cy),
            None => {}
        }
    }

    /// Solta o botão: a forma elástica da página 5 vira pintura (uma ação do ovo).
    fn end_stroke(&mut self, cx: i32, cy: i32) {
        self.extend_stroke(cx, cy);
        if let Some(Stroke::Geom {
            kind,
            anchor,
            pointer,
            mut verts,
        }) = self.stroke.take()
        {
            if kind == 4 && verts.last() != Some(&pointer) {
                verts.push(pointer);
            }
            let pixels = geom_pixels(kind, anchor, pointer, &verts);
            self.history.record(&self.canvas);
            let ink = self.ink();
            for (x, y) in pixels {
                self.canvas.plot(x, y, &ink);
            }
            self.changed = true;
        }
        self.stroke = None;
    }

    /// Uma rajada do spray ao redor do ponteiro (chamada a cada atualização).
    fn spray_tick(&mut self) {
        let ink = self.ink();
        let Some(Stroke::Spray {
            rng,
            pos,
            radius,
            dots,
        }) = self.stroke.as_mut()
        else {
            return;
        };
        let mut n = 0;
        while n < *dots {
            let (dx, dy) = (rng.offset(*radius), rng.offset(*radius));
            if dx * dx + dy * dy > *radius * *radius {
                continue;
            }
            self.canvas.plot(pos.0 + dx, pos.1 + dy, &ink);
            n += 1;
        }
        self.changed = true;
    }

    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        // desenho animado: fora da porta de saída, tudo é do projeto
        if let Some(Project::Flipbook(fb)) = self.project.as_mut()
            && !inside(EXIT, x, y)
        {
            fx.extend(fb.click(x, y, self.drawings.as_deref(), assets));
            return;
        }
        self.stroke = None;
        self.finish_sticker();
        // qualquer clique encerra o texto que estava sendo digitado
        self.commit_words();
        if let Some(p) = self.project.as_mut() {
            if let Some(changed) = p.click(x, y, &mut self.canvas, &mut self.history) {
                self.changed |= changed;
                return;
            }
            if inside(canvas_rect(), x, y) {
                p.clip_at(&mut self.canvas, x - ORIGIN.0, y - ORIGIN.1);
            }
        }
        if self.tool == Tool::Stickers {
            match self
                .stickers
                .click(x, y, &mut self.canvas, &mut self.history, assets, &self.pal)
            {
                Some(placed) => {
                    self.changed |= placed;
                    return;
                }
                // qualquer outro clique solta o adesivo que estava na mão
                None => self.stickers.reset(),
            }
        }
        if self.tool == Tool::Sounds {
            match self.sounds.click(x, y) {
                Some(Some(snd)) => {
                    fx.push(Effect::PlaySound(snd));
                    self.changed = true;
                    return;
                }
                Some(None) => return,
                None => self.sounds.reset(),
            }
        }
        if inside(canvas_rect(), x, y) {
            let (cx, cy) = (x - ORIGIN.0, y - ORIGIN.1);
            match self.tool {
                Tool::Brush => self.start_stroke(cx, cy),
                Tool::Words => self.start_words(cx, cy),
                Tool::Bucket => {
                    let before = self.canvas.px.clone();
                    let ink = self.ink();
                    self.canvas.fill(cx, cy, &ink);
                    if self.canvas.px != before {
                        self.history.record_previous(before);
                        self.changed = true;
                    }
                }
                Tool::Vacuum => self.vacuum_press(cx, cy),
                Tool::Stickers | Tool::Sounds => {}
            }
            return;
        }
        if inside(EXIT, x, y) {
            let back = if self.project.is_some() {
                SceneId::Projects
            } else {
                SceneId::Studio
            };
            fx.push(Effect::ChangeScene(back));
            return;
        }
        if let Some(t) = TOOLS
            .iter()
            .find(|t| inside((t.x - 5, 5, t.w + 11, 50), x, y))
        {
            match t.action {
                Action::Select(tool) => {
                    self.tool = tool;
                    self.stickers.reset();
                    self.sounds.reset();
                }
                Action::Undo => {
                    if self.history.toggle(&mut self.canvas) {
                        self.changed = true;
                    }
                }
                Action::Print => fx.push(self.print()),
                Action::Unavailable(name) => fx.push(Effect::Log(format!(
                    "{name}: ferramenta ainda não disponível"
                ))),
            }
            return;
        }
        if let Some(k) = (0..SWATCHES).find(|&k| inside(color_rect(k), x, y)) {
            self.color = k;
            return;
        }
        if let Some(k) = (0..SWATCHES).find(|&k| inside(pattern_rect(k), x, y)) {
            self.pattern = k;
            return;
        }
        if self.tool == Tool::Brush {
            if inside(ARROW_UP, x, y) {
                self.page = if self.page == 1 {
                    PAGE_COUNT
                } else {
                    self.page - 1
                };
                return;
            }
            if inside(ARROW_DOWN, x, y) {
                self.page = self.page % PAGE_COUNT + 1;
                return;
            }
        }
        let x0 = self.options_x0();
        if let Some(k) = (0..self.option_count()).find(|&k| inside(option_rect(x0, k), x, y)) {
            match self.tool {
                Tool::Brush => self.set_brush(self.page, k),
                Tool::Bucket => self.bucket_option = k,
                Tool::Words => self.words_option = k,
                Tool::Stickers | Tool::Sounds => {}
                Tool::Vacuum => {
                    self.vacuum = VACUUM_MODES[k];
                    if matches!(self.vacuum, VacuumMode::Stickers | VacuumMode::Sounds) {
                        fx.push(Effect::Log("aspirador: opção ainda não disponível".into()));
                    }
                }
            }
        }
    }

    pub fn vacuum_mode(&self) -> VacuumMode {
        self.vacuum
    }

    pub fn has_clipboard(&self) -> bool {
        self.clipboard.is_some()
    }

    /// Retângulo (x0, y0, w, h) entre a âncora e o ponto, dentro do quadro.
    fn drag_rect(&self, cx: i32, cy: i32) -> Option<(i32, i32, i32, i32)> {
        let (ax, ay) = self.vacuum_anchor?;
        let x0 = ax.min(cx).max(0);
        let y0 = ay.min(cy).max(0);
        let x1 = (ax.max(cx) + 1).min(self.canvas.w as i32);
        let y1 = (ay.max(cy) + 1).min(self.canvas.h as i32);
        (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
    }

    fn copy_rect(&self, r: (i32, i32, i32, i32)) -> Stamp {
        let (x0, y0, w, h) = r;
        let px = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| self.canvas.get(x0 + x, y0 + y))
            .collect();
        Stamp {
            w: w as u32,
            h: h as u32,
            px,
        }
    }

    fn vacuum_press(&mut self, cx: i32, cy: i32) {
        match self.vacuum {
            VacuumMode::Cut | VacuumMode::Copy | VacuumMode::Color => {
                self.vacuum_anchor = Some((cx, cy));
            }
            VacuumMode::Paste => {
                if let Some(s) = &self.clipboard {
                    self.history.record(&self.canvas);
                    let (w, h) = (s.w as i32, s.h as i32);
                    self.canvas.paste(s, cx - w / 2, cy - h / 2);
                    self.changed = true;
                }
            }
            VacuumMode::Area => {
                let before = self.canvas.px.clone();
                self.canvas.fill(cx, cy, &Ink::solid(crate::paint::WHITE));
                if self.canvas.px != before {
                    self.history.record_previous(before);
                    self.changed = true;
                }
            }
            VacuumMode::All => {
                self.history.record(&self.canvas);
                self.canvas.clear();
                self.changed = true;
            }
            VacuumMode::Stickers | VacuumMode::Sounds => {}
        }
    }

    fn vacuum_release(&mut self, cx: i32, cy: i32) {
        let Some(r) = self.drag_rect(cx, cy) else {
            self.vacuum_anchor = None;
            return;
        };
        self.vacuum_anchor = None;
        let (x0, y0, w, h) = r;
        match self.vacuum {
            VacuumMode::Copy => self.clipboard = Some(self.copy_rect(r)),
            VacuumMode::Cut => {
                self.clipboard = Some(self.copy_rect(r));
                self.history.record(&self.canvas);
                for y in y0..y0 + h {
                    for x in x0..x0 + w {
                        self.canvas.plot(x, y, &Ink::solid(crate::paint::WHITE));
                    }
                }
                self.changed = true;
            }
            VacuumMode::Color => {
                let color = COLORS[self.color];
                let before = self.canvas.px.clone();
                for y in y0..y0 + h {
                    for x in x0..x0 + w {
                        if self.canvas.get(x, y) == Some(color) {
                            self.canvas.plot(x, y, &Ink::solid(crate::paint::WHITE));
                        }
                    }
                }
                if self.canvas.px != before {
                    self.history.record_previous(before);
                    self.changed = true;
                }
            }
            _ => {}
        }
    }

    fn rgb(&self, index: u8) -> [u8; 3] {
        let p = self
            .pal
            .get(index as usize)
            .copied()
            .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
        [p.r, p.g, p.b]
    }

    fn draw_chrome(&self, f: &mut Frame, a: &Assets) {
        f.fill(DARK);
        // bordas laranja: preto, claro, escuro, preto (relevo)
        for (y, c) in [(0, BLACK), (1, ORANGE), (2, ORANGE), (3, BROWN), (4, BLACK)] {
            f.rect(0, y, 640, 1, c);
        }
        for (y, c) in [
            (104, BLACK),
            (105, ORANGE),
            (106, ORANGE),
            (107, BROWN),
            (108, BLACK),
        ] {
            f.rect(0, y, 640, 1, c);
        }
        for (y, c) in [
            (456, BLACK),
            (457, ORANGE),
            (458, ORANGE),
            (459, BROWN),
            (460, BLACK),
        ] {
            f.rect(0, y, 640, 1, c);
        }
        for (y, c) in [
            (475, BLACK),
            (476, ORANGE),
            (477, ORANGE),
            (478, BROWN),
            (479, BLACK),
        ] {
            f.rect(0, y, 640, 1, c);
        }
        for (x, c) in [(0, BLACK), (1, BROWN), (2, ORANGE), (3, ORANGE), (4, BLACK)] {
            f.rect(x, 0, 1, 480, c);
        }
        for (x, c) in [
            (635, BLACK),
            (636, BROWN),
            (637, ORANGE),
            (638, ORANGE),
            (639, BLACK),
        ] {
            f.rect(x, 0, 1, 480, c);
        }
        // faixa de opções e linha sob a barra de ferramentas
        f.rect(5, 56, 630, 48, LIGHT);
        f.rect(5, 55, 630, 1, BLACK);
        // nicho escuro do McZee
        f.rect(532, 55, 1, 49, BLACK);
        f.rect(533, 56, 47, 48, DARK);
        // colunas de cores e padrões, bordas do quadro
        f.rect(5, 109, 17, 347, WHITE);
        f.rect(618, 109, 17, 347, WHITE);
        f.rect(22, 108, 1, 349, BLACK);
        f.rect(617, 108, 1, 349, BLACK);
        // barra inferior
        for (x0, x1) in [(22, 137), (155, 486), (518, 546)] {
            f.rect(x0, 461, 1, 14, BLACK);
            f.rect(x1, 461, 1, 14, BLACK);
            f.rect(x0 + 1, 461, x1 - x0 - 1, 14, WHITE);
        }
        // textos na "Pablo Graphos" de 8 pontos, como no original
        let (label, name) = match &self.project {
            Some(p) => (p.kind().label(), p.kind().new_name()),
            None => ("Ateliê", "Quadro Novo"),
        };
        match a.font(8) {
            Some(font) => {
                f.text_font(&font, 69, 461, label, BLACK);
                let w = font.width(name) as i32;
                f.text_font(&font, 320 - w / 2, 462, name, BLACK);
            }
            None => {
                f.text(64, 464, label, BLACK);
                f.text(276, 464, name, BLACK);
            }
        }
    }

    fn draw_tools(&self, f: &mut Frame, a: &Assets) {
        for t in &TOOLS {
            let active = t.action == Action::Select(self.tool);
            let (img, mask) = if active { t.active } else { t.normal };
            if active {
                let w = a.image(&pic(img)).width as i32;
                f.rect(t.x - 5, 5, w + 11, 51, LIGHT);
                f.rect(t.x - 6, 5, 1, 50, BLACK);
                f.rect(t.x + w + 6, 5, 1, 50, BLACK);
            }
            f.blit_masked(&a.image(&pic(img)), &a.image(&pic(mask)), t.x, t.y);
        }
        f.blit_masked(&a.image(&pic(0x1055)), &a.image(&pic(0x1057)), 537, 49);
        // ícones da barra inferior
        f.blit(&a.image(&pic(0x03c3)), 27, 463);
        f.blit(&a.image(&pic(0x1403)), 526, 462);
        f.blit_masked(&a.image(&pic(0x1059)), &a.image(&pic(0x105b)), 580, 18);
    }

    fn draw_shape_button(&self, f: &mut Frame, r: (i32, i32, i32, i32), s: &Shape) {
        f.rect(r.0, r.1, r.2, r.3, BLACK);
        f.rect(r.0 + 3, r.1 + 3, r.2 - 6, r.3 - 6, WHITE);
        let x0 = r.0 + 16 - s.w as i32 / 2;
        let y0 = r.1 + 16 - s.h as i32 / 2;
        for (k, c) in s.cells.iter().enumerate() {
            if *c == crate::paint::Cell::Ink {
                f.put(
                    x0 + (k as u32 % s.w) as i32,
                    y0 + (k as u32 / s.w) as i32,
                    BLACK,
                );
            }
        }
    }

    fn draw_options(&self, f: &mut Frame, a: &Assets) {
        let selected = match self.tool {
            Tool::Brush => {
                f.blit_masked(&a.image(&pic(0x1129)), &a.image(&pic(0x112b)), 4, 56);
                let shapes = page_shapes(self.page);
                for k in 0..12 {
                    let r = option_rect(25, k);
                    match self.page {
                        1 | 4 => self.draw_shape_button(f, r, &shapes[k]),
                        p => {
                            let ids = match p {
                                2 => &PAGE2_BUTTONS,
                                3 => &PAGE3_BUTTONS,
                                5 => &PAGE5_BUTTONS,
                                _ => &PAGE6_BUTTONS,
                            };
                            f.blit(&a.image(&pic(ids[k])), r.0, r.1);
                        }
                    }
                }
                f.blit_masked(
                    &a.image(&pic(0x03b3)),
                    &a.image(&pic(0x03b5)),
                    ARROW_UP.0,
                    ARROW_UP.1,
                );
                f.blit_masked(
                    &a.image(&pic(0x03b7)),
                    &a.image(&pic(0x03b9)),
                    ARROW_DOWN.0,
                    ARROW_DOWN.1,
                );
                f.rect(504, 75, 16, 12, BLACK);
                f.rect(505, 76, 14, 10, WHITE);
                f.text(508, 77, &self.page.to_string(), BLACK);
                (self.brush.0 == self.page).then_some(self.brush.1)
            }
            Tool::Bucket => {
                f.blit_masked(&a.image(&pic(0x1121)), &a.image(&pic(0x1123)), 92, 56);
                f.rect(12, 60, 66, 40, BLACK);
                let ink = self.ink();
                for y in 62..98 {
                    for x in 14..76 {
                        f.put(x, y, self.rgb(ink.index_at(x, y)));
                    }
                }
                for (k, id) in BUCKET_BUTTONS.iter().enumerate() {
                    let r = option_rect(113, k);
                    f.blit(&a.image(&pic(*id)), r.0, r.1);
                }
                Some(self.bucket_option)
            }
            Tool::Stickers => {
                self.stickers.draw_strip(f, a);
                None
            }
            Tool::Sounds => {
                self.sounds.draw_strip(f, a);
                None
            }
            Tool::Words => {
                for (k, id) in WORDS_BUTTONS.iter().enumerate() {
                    let r = option_rect(25, k);
                    f.blit(&a.image(&pic(*id)), r.0, r.1);
                }
                Some(self.words_option)
            }
            Tool::Vacuum => {
                f.blit_masked(&a.image(&pic(0x1119)), &a.image(&pic(0x111b)), 4, 56);
                for (k, id) in VACUUM_BUTTONS.iter().enumerate() {
                    let r = option_rect(25, k);
                    f.blit(&a.image(&pic(*id)), r.0, r.1);
                }
                VACUUM_MODES.iter().position(|m| *m == self.vacuum)
            }
        };
        if let Some(k) = selected {
            let r = option_rect(self.options_x0(), k);
            f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, RED);
        }
    }

    fn draw_columns(&self, f: &mut Frame) {
        let ink = self.ink();
        for (k, &color) in COLORS.iter().enumerate() {
            let (_, y, _, _) = color_rect(k);
            let t = if k == self.color { 2 } else { 1 };
            f.rect(8 - t, y + 1 - t, 11 + 2 * t, 6 + 2 * t, BLACK);
            f.rect(8, y + 1, 11, 6, self.rgb(color));
            let t = if k == self.pattern { 2 } else { 1 };
            f.rect(621 - t, y + 1 - t, 11 + 2 * t, 6 + 2 * t, BLACK);
            let swatch = Ink {
                color: ink.color,
                pattern: self.patterns[k],
            };
            for yy in y + 1..y + 7 {
                for xx in 621..632 {
                    f.put(xx, yy, self.rgb(swatch.index_at(xx, yy)));
                }
            }
        }
    }

    fn draw_canvas(&self, f: &mut Frame) {
        for (k, &i) in self.canvas.px.iter().enumerate() {
            let x = ORIGIN.0 + (k as u32 % self.canvas.w) as i32;
            let y = ORIGIN.1 + (k as u32 / self.canvas.w) as i32;
            f.put(x, y, self.rgb(i));
        }
    }

    /// Texto em edição sobre o quadro, com o balão (se for a opção) e o cursor piscando.
    fn draw_words(&self, f: &mut Frame) {
        let Some(w) = self.words.as_ref() else {
            return;
        };
        let font = self.font.as_deref();
        let (sx, sy) = (ORIGIN.0 + w.x, ORIGIN.1 + w.y);
        let (tw, th) = text_size(font, &w.text);
        let r = canvas_rect();
        let mut put = |x: i32, y: i32, rgb: [u8; 3]| {
            if inside(r, x, y) {
                f.put(x, y, rgb);
            }
        };
        if self.words_option == 1 {
            balloon_pixels(sx, sy, tw, th, |x, y, edge| {
                put(x, y, if edge { BLACK } else { WHITE })
            });
        }
        let ink = self.rgb(COLORS[self.color]);
        text_pixels(font, &w.text, |dx, dy| put(sx + dx, sy + dy, ink));
        if w.blink % CARET_PERIOD < CARET_PERIOD / 2.0 {
            for dy in 0..th as i32 {
                put(sx + tw as i32, sy + dy, ink);
            }
        }
    }

    /// Prévia elástica da forma da página 5, só na tela (o quadro não muda).
    fn draw_preview(&self, f: &mut Frame) {
        let Some(Stroke::Geom {
            kind,
            anchor,
            pointer,
            verts,
        }) = &self.stroke
        else {
            return;
        };
        let ink = self.ink();
        for (x, y) in geom_pixels(*kind, *anchor, *pointer, verts) {
            if x >= 0 && y >= 0 && (x as u32) < self.canvas.w && (y as u32) < self.canvas.h {
                let (sx, sy) = (x + ORIGIN.0, y + ORIGIN.1);
                f.put(sx, sy, self.rgb(ink.index_at(sx, sy)));
            }
        }
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Scene for Paint {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        for i in input {
            match *i {
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.mouse = (x, y);
                    self.click(x, y, &mut fx, assets)
                }
                Input::MouseMove { x, y } => {
                    self.mouse = (x, y);
                    self.extend_stroke(x - ORIGIN.0, y - ORIGIN.1)
                }
                Input::MouseUp {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.end_stroke(x - ORIGIN.0, y - ORIGIN.1);
                    if self.vacuum_anchor.is_some() {
                        self.vacuum_release(x - ORIGIN.0, y - ORIGIN.1);
                    }
                }
                Input::Key {
                    key,
                    ctrl: false,
                    alt: false,
                } => match self.project.as_mut() {
                    Some(Project::Flipbook(fb)) => fb.key(key),
                    _ => self.key(key),
                },
                Input::Text(ref t) => match self.project.as_mut() {
                    Some(Project::Flipbook(fb)) => fb.text(t),
                    _ => self.type_text(t),
                },
                _ => {}
            }
        }
        if let Some(Project::Flipbook(fb)) = self.project.as_mut() {
            fx.extend(fb.update(dt));
            return fx;
        }
        if let Some(w) = self.words.as_mut() {
            w.blink = (w.blink + dt) % CARET_PERIOD;
        }
        self.spray_tick();
        self.sounds.update(dt);
        if self
            .stickers
            .update(dt, &mut self.canvas, &mut self.history)
        {
            self.changed = true;
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        self.draw_chrome(frame, assets);
        self.draw_tools(frame, assets);
        if let Some(Project::Flipbook(fb)) = &self.project {
            fb.draw_strip(frame, assets);
            self.draw_columns(frame);
            fb.draw_canvas(frame, assets);
            return;
        }
        match &self.project {
            Some(p) if p.asking() => p.draw_strip(frame, assets),
            _ => self.draw_options(frame, assets),
        }
        self.draw_columns(frame);
        self.draw_canvas(frame);
        if let Some(p) = &self.project {
            p.draw_overlay(frame, assets);
        }
        // retângulo elástico do aspirador
        if self.vacuum_anchor.is_some()
            && let Some((x0, y0, w, h)) =
                self.drag_rect(self.mouse.0 - ORIGIN.0, self.mouse.1 - ORIGIN.1)
        {
            frame.frame_rect(ORIGIN.0 + x0, ORIGIN.1 + y0, w, h, 1, BLACK);
        }
        self.draw_words(frame);
        self.draw_preview(frame);
        self.stickers
            .draw_overlay(frame, assets, &self.pal, self.mouse);
        if self.tool == Tool::Sounds {
            self.sounds.draw_overlay(frame, assets);
        }
    }

    fn leave(&mut self) -> Vec<Effect> {
        self.stroke = None;
        self.finish_sticker();
        self.commit_words();
        let Some(dir) = &self.drawings else {
            return vec![Effect::Log(
                "sem pasta de desenhos (HOME e XDG_DATA_HOME vazios): desenho não guardado".into(),
            )];
        };
        if let Some(Project::Flipbook(fb)) = self.project.as_mut() {
            if !fb.needs_save() {
                return vec![Effect::Log("desenho animado sem mudanças".into())];
            }
            let msg = match fb.save(dir, now_secs()) {
                Ok(p) => format!("desenho animado guardado em {}", p.display()),
                Err(e) => e,
            };
            return vec![Effect::Log(msg)];
        }
        if let Err(e) = self.sounds.save_spots(&dir.join(SPOTS_NAME)) {
            return vec![Effect::Log(format!("sons: {e}"))];
        }
        let saved = match &self.project {
            Some(p) => super::projects::save_picture(
                dir,
                p.kind(),
                &self.canvas,
                &self.pal,
                self.changed,
                now_secs(),
            ),
            None => save_drawing(dir, &self.canvas, &self.pal, self.changed, now_secs()),
        };
        let msg = match saved {
            Ok(()) => format!("desenho guardado em {}", dir.display()),
            Err(e) => e,
        };
        self.changed = false;
        vec![Effect::Log(msg)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::WHITE as W;
    use crate::scenes::stickers::ANIMALS;
    use crate::scenes::testutil::empty_assets;

    fn click(x: i32, y: i32) -> Input {
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

    fn painted(p: &Paint) -> usize {
        p.canvas().px.iter().filter(|&&v| v != W).count()
    }

    #[test]
    fn exit_door_goes_to_studio() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        let fx = p.update(&[click(600, 60)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Studio)));
    }

    #[test]
    fn color_click_selects() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        assert_eq!(p.color(), 1);
        p.update(&[click(13, 113 + 9 * 8)], 0.0, &a);
        assert_eq!(p.color(), COLORS[8]);
    }

    #[test]
    fn pattern_click_selects() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(625, 113 + 9 * 3)], 0.0, &a);
        assert_eq!(p.pattern(), 3);
    }

    #[test]
    fn page_arrows_wrap() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(510, 62)], 0.0, &a);
        assert_eq!(p.page(), PAGE_COUNT);
        assert_eq!(PAGE_COUNT, 6);
        p.update(&[click(510, 95)], 0.0, &a);
        p.update(&[click(510, 95)], 0.0, &a);
        assert_eq!(p.page(), 2);
        // escolher a forma 3 da página 2
        p.update(&[click(25 + 37 * 3 + 10, 70)], 0.0, &a);
        assert_eq!(p.brush(), (2, 3));
    }

    #[test]
    fn brush_drag_paints_and_mouseup_ends() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(100, 200)], 0.0, &a);
        let one = painted(&p);
        assert_eq!(one, 208);
        p.update(&[Input::MouseMove { x: 200, y: 200 }], 0.0, &a);
        let line = painted(&p);
        assert!(line > one + 100 * 15);
        p.update(
            &[up(200, 200), Input::MouseMove { x: 300, y: 300 }],
            0.0,
            &a,
        );
        assert_eq!(painted(&p), line);
    }

    #[test]
    fn drag_outside_is_clipped() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(30, 120)], 0.0, &a);
        p.update(
            &[
                Input::MouseMove { x: -500, y: -500 },
                Input::MouseMove { x: 900, y: 900 },
                up(2000, -3),
            ],
            0.0,
            &a,
        );
        assert!(painted(&p) > 0);
    }

    #[test]
    fn click_after_lost_mouseup_starts_new_stroke() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(100, 150)], 0.0, &a);
        // sem MouseUp; novo clique longe: não liga as duas posições
        p.update(&[click(500, 400)], 0.0, &a);
        assert_eq!(painted(&p), 2 * 208);
    }

    #[test]
    fn bucket_fills() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(70, 30)], 0.0, &a);
        assert_eq!(p.tool(), Tool::Bucket);
        p.update(&[click(300, 300)], 0.0, &a);
        assert_eq!(painted(&p), (CANVAS_W * CANVAS_H) as usize);
    }

    #[test]
    fn bucket_on_same_color_changes_nothing() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
        p.update(&[click(13, 113)], 0.0, &a); // branco
        p.update(&[click(70, 30), click(300, 300)], 0.0, &a);
        p.leave();
        // só ultimo.png: nada mudou, nenhuma cópia com data
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn leave_without_dir_logs() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        assert!(matches!(&p.leave()[..], [Effect::Log(s)] if s.contains("não guardado")));
    }

    #[test]
    fn egg_undoes_and_redoes() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(100, 200), up(100, 200)], 0.0, &a);
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 0);
        assert_eq!(p.tool(), Tool::Brush);
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 208);
    }

    #[test]
    fn vacuum_all_clears_and_egg_restores() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(100, 200), up(100, 200)], 0.0, &a);
        p.update(&[click(320, 30)], 0.0, &a);
        assert_eq!(p.tool(), Tool::Vacuum);
        p.update(&[click(300, 79)], 0.0, &a);
        assert_eq!(p.vacuum_mode(), VacuumMode::All);
        p.update(&[click(300, 300)], 0.0, &a);
        assert_eq!(painted(&p), 0);
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 208);
    }

    #[test]
    fn vacuum_cut_and_paste() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(100, 200), up(100, 200)], 0.0, &a);
        p.update(&[click(320, 30), click(40, 79)], 0.0, &a);
        assert_eq!(p.vacuum_mode(), VacuumMode::Cut);
        // arrasta um retângulo em volta do carimbo (centro 100,200; raio 8)
        p.update(
            &[
                click(80, 180),
                Input::MouseMove { x: 120, y: 220 },
                up(120, 220),
            ],
            0.0,
            &a,
        );
        assert_eq!(painted(&p), 0);
        assert!(p.has_clipboard());
        p.update(&[click(40 + 37 * 2 + 16, 79), click(400, 300)], 0.0, &a);
        assert_eq!(p.vacuum_mode(), VacuumMode::Paste);
        assert_eq!(painted(&p), 208);
        assert_eq!(p.canvas().get(400 - 23, 300 - 109), Some(1));
    }

    #[test]
    fn vacuum_area_and_color() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(100, 200), up(100, 200)], 0.0, &a);
        p.update(
            &[click(13, 113 + 9 * 8), click(300, 200), up(300, 200)],
            0.0,
            &a,
        );
        p.update(
            &[click(320, 30), click(40 + 37 * 3 + 16, 79), click(100, 200)],
            0.0,
            &a,
        );
        assert_eq!(p.vacuum_mode(), VacuumMode::Area);
        assert_eq!(painted(&p), 208);
        // aspira-cores com a cor ativa (índice 8 da coluna) num retângulo que cobre tudo
        p.update(
            &[click(40 + 37 * 5 + 16, 79), click(30, 120), up(600, 450)],
            0.0,
            &a,
        );
        assert_eq!(painted(&p), 0);
    }

    #[test]
    fn inactive_tool_logs() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        let fx = p.update(&[click(378, 30)], 0.0, &a);
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::Log(s) if s.contains("transformadores")))
        );
        assert_eq!(p.tool(), Tool::Brush);
    }

    #[test]
    fn print_exports_png() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
        let fx = p.update(&[click(479, 30)], 0.0, &a);
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::Log(s) if s.contains("impressao-")))
        );
        assert!(std::fs::read_dir(dir.path()).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("impressao-")
        }));
        assert_eq!(p.tool(), Tool::Brush);
    }

    #[test]
    fn sounds_tool_without_library_is_harmless() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
        p.update(&[click(170, 30)], 0.0, &a);
        assert_eq!(p.tool(), Tool::Sounds);
        assert_eq!(p.sounds().categories(), 0);
        let fx = p.update(&[click(300, 300)], 0.0, &a);
        assert!(!fx.iter().any(|e| matches!(e, Effect::PlaySound(_))));
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        p.leave();
        assert!(!dir.path().join(SPOTS_NAME).exists());
    }

    fn text(s: &str) -> Input {
        Input::Text(s.to_string())
    }

    fn key(k: Key) -> Input {
        Input::Key {
            key: k,
            ctrl: false,
            alt: false,
        }
    }

    /// Escolhe a Palavra, clica no quadro em (100, 200) e digita `s`.
    fn words(a: &Assets, s: &str) -> Paint {
        let mut p = Paint::new(None, a);
        p.update(&[click(228, 30)], 0.0, a);
        assert_eq!(p.tool(), Tool::Words);
        p.update(&[click(100, 200), text(s)], 0.0, a);
        p
    }

    #[test]
    fn words_typed_then_enter_paint_text() {
        let (_d, a) = empty_assets();
        let mut p = words(&a, "Oi");
        assert_eq!(p.words_text(), Some("Oi"));
        assert_eq!(painted(&p), 0);
        p.update(&[key(Key::Enter)], 0.0, &a);
        assert_eq!(p.words_text(), None);
        let n = painted(&p);
        assert!(n > 0);
        // fonte 8×8 reserva: tudo dentro de 16×8 a partir de (100, 200) na tela
        let c = p.canvas();
        for (k, &v) in c.px.iter().enumerate() {
            if v != W {
                let (x, y) = ((k as u32 % c.w) as i32, (k as u32 / c.w) as i32);
                assert!((77..93).contains(&x) && (91..99).contains(&y), "({x}, {y})");
                assert_eq!(v, 1);
            }
        }
        // o ovo desfaz o texto e refaz
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 0);
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), n);
    }

    #[test]
    fn words_backspace_removes_and_filters_chars() {
        let (_d, a) = empty_assets();
        let mut p = words(&a, "Olá");
        p.update(&[key(Key::Backspace), text("中"), text("!\u{7}")], 0.0, &a);
        assert_eq!(p.words_text(), Some("Ol!"));
        p.update(&[key(Key::Backspace), key(Key::Backspace)], 0.0, &a);
        p.update(&[key(Key::Backspace), key(Key::Backspace)], 0.0, &a);
        assert_eq!(p.words_text(), Some(""));
    }

    #[test]
    fn words_empty_commit_paints_nothing_and_no_undo() {
        let (_d, a) = empty_assets();
        let mut p = words(&a, "");
        p.update(&[key(Key::Enter)], 0.0, &a);
        assert_eq!(painted(&p), 0);
        assert_eq!(p.words_text(), None);
        // nada para o ovo desfazer
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 0);
    }

    #[test]
    fn words_click_elsewhere_commits_and_restarts() {
        let (_d, a) = empty_assets();
        let mut p = words(&a, "A");
        p.update(&[click(300, 300)], 0.0, &a);
        let n = painted(&p);
        assert!(n > 0);
        assert_eq!(p.words_text(), Some(""));
        p.update(&[text("B"), click(13, 150)], 0.0, &a);
        assert!(painted(&p) > n);
        assert_eq!(p.words_text(), None);
    }

    #[test]
    fn words_switching_tool_commits() {
        let (_d, a) = empty_assets();
        let mut p = words(&a, "A");
        p.update(&[click(20, 30)], 0.0, &a);
        assert_eq!(p.tool(), Tool::Brush);
        assert!(painted(&p) > 0);
        assert_eq!(p.words_text(), None);
    }

    #[test]
    fn words_leave_commits() {
        let (_d, a) = empty_assets();
        let mut p = words(&a, "A");
        p.leave();
        assert!(painted(&p) > 0);
    }

    #[test]
    fn words_balloon_option_paints_white_and_outline() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        // quadro cinza para distinguir o branco do balão
        p.update(
            &[click(13, 113 + 9 * 8), click(70, 30), click(300, 300)],
            0.0,
            &a,
        );
        let gray = COLORS[8];
        assert_eq!(p.canvas().get(0, 0), Some(gray));
        p.update(&[click(228, 30), click(25 + 37 + 10, 70)], 0.0, &a);
        p.update(&[click(13, 113 + 9 * 37)], 0.0, &a); // tinta preta
        p.update(&[click(100, 200), text("Oi"), key(Key::Enter)], 0.0, &a);
        let c = p.canvas();
        // texto 16×8 em (77, 91): balão de (71, 85) a (99, 105) exclusivos
        assert_eq!(c.get(71, 85), Some(1));
        assert_eq!(c.get(98, 104), Some(1));
        assert_eq!(c.get(72, 86), Some(0));
        assert_eq!(c.get(76, 90), Some(0));
        assert_eq!(c.get(94, 99), Some(0));
        assert_eq!(c.get(70, 84), Some(gray));
        assert!(c.px.contains(&1));
        // cursor de texto e balão não travam o desenho
        let mut f = Frame::new(640, 480);
        p.update(&[click(300, 300), text("x")], 0.3, &a);
        p.draw(&mut f, &a);
        p.update(&[], 0.4, &a);
        p.draw(&mut f, &a);
    }

    #[test]
    fn words_text_ignored_without_session() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(
            &[text("abc"), key(Key::Enter), key(Key::Backspace)],
            0.0,
            &a,
        );
        assert_eq!(p.words_text(), None);
        assert_eq!(painted(&p), 0);
    }

    #[test]
    fn leave_saves_drawing_and_reopens() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
        p.update(&[click(100, 200), up(100, 200)], 0.0, &a);
        let fx = p.leave();
        assert!(matches!(&fx[..], [Effect::Log(s)] if s.contains("guardado")));
        assert!(dir.path().join(LAST_NAME).exists());
        // ultimo.png mais a cópia com data em .png e em .mag
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3);
        let q = Paint::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(painted(&q), 208);
    }

    #[test]
    fn leave_unwritable_logs() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arquivo");
        std::fs::write(&file, b"x").unwrap();
        let mut p = Paint::new(Some(file.join("sub")), &a);
        let fx = p.leave();
        assert!(matches!(&fx[..], [Effect::Log(s)] if s.contains("não consegui")));
    }

    #[test]
    fn sticker_roll_film_hold_place_and_paste() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(129, 30)], 0.0, &a);
        assert_eq!(p.tool(), Tool::Stickers);
        p.update(&[click(40, 80)], 0.0, &a);
        assert!(p.stickers().film_open());
        // casa 1 da tira: o peixe-espada
        p.update(&[click(54, 103 + 39 + 10)], 0.0, &a);
        assert_eq!(p.stickers().held_animal(), Some(1));
        assert!(!p.stickers().film_open());
        p.update(&[click(300, 300)], 0.0, &a);
        assert!(p.stickers().is_animating());
        assert_eq!(painted(&p), 0);
        p.update(&[], 10.0, &a);
        assert!(!p.stickers().is_animating());
        assert!(painted(&p) > 0);
        assert_eq!(p.stickers().held_animal(), None);
        // o ovo desfaz a colagem
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 0);
    }

    #[test]
    fn click_elsewhere_drops_held_sticker() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(129, 30), click(40, 80), click(54, 113)], 0.0, &a);
        assert_eq!(p.stickers().held_animal(), Some(0));
        p.update(&[click(13, 150)], 0.0, &a);
        assert_eq!(p.stickers().held_animal(), None);
        p.update(&[click(300, 300)], 0.0, &a);
        assert!(!p.stickers().is_animating());
    }

    #[test]
    fn film_scrolls_within_bounds() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(129, 30), click(40, 80)], 0.0, &a);
        for _ in 0..20 {
            p.update(&[click(60, 456)], 0.0, &a);
        }
        // última casa visível = último bicho
        p.update(&[click(54, 103 + 39 * 8 + 10)], 0.0, &a);
        assert_eq!(p.stickers().held_animal(), Some(ANIMALS.len() - 1));
        p.update(&[click(40, 80)], 0.0, &a);
        for _ in 0..20 {
            p.update(&[click(42, 456)], 0.0, &a);
        }
        p.update(&[click(54, 113)], 0.0, &a);
        assert_eq!(p.stickers().held_animal(), Some(0));
    }

    #[test]
    fn snake_leaves_without_pasting() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(
            &[
                click(129, 30),
                click(40, 80),
                click(54, 113),
                click(100, 300),
            ],
            0.0,
            &a,
        );
        assert!(p.stickers().is_animating());
        for _ in 0..200 {
            p.update(&[], 0.1, &a);
        }
        assert!(!p.stickers().is_animating());
        assert_eq!(painted(&p), 0);
    }

    #[test]
    fn snake_does_not_mark_drawing_changed() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
        p.update(
            &[
                click(129, 30),
                click(40, 80),
                click(54, 113),
                click(100, 300),
            ],
            0.0,
            &a,
        );
        for _ in 0..200 {
            p.update(&[], 0.1, &a);
        }
        p.leave();
        // só ultimo.png: a cobra foi embora sem pintar nada
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn roll_click_toggles_film() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(129, 30), click(40, 80)], 0.0, &a);
        assert!(p.stickers().film_open());
        p.update(&[click(40, 80)], 0.0, &a);
        assert!(!p.stickers().film_open());
    }

    #[test]
    fn click_during_animation_pastes_first() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(
            &[
                click(129, 30),
                click(40, 80),
                click(54, 152),
                click(300, 300),
            ],
            0.0,
            &a,
        );
        assert!(p.stickers().is_animating());
        p.update(&[click(13, 150)], 0.0, &a);
        assert!(!p.stickers().is_animating());
        assert!(painted(&p) > 0);
    }

    #[test]
    fn leave_during_animation_saves_sticker() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
        p.update(
            &[
                click(129, 30),
                click(40, 80),
                click(54, 152),
                click(300, 300),
            ],
            0.0,
            &a,
        );
        p.leave();
        let q = Paint::new(Some(dir.path().to_path_buf()), &a);
        assert!(painted(&q) > 0);
    }

    /// Vai para a página `page` do pincel e escolhe o botão `k`.
    fn pick(p: &mut Paint, a: &Assets, page: u8, k: usize) {
        while p.page() != page {
            p.update(&[click(510, 95)], 0.0, a);
        }
        p.update(&[click(25 + 37 * k as i32 + 10, 70)], 0.0, a);
    }

    #[test]
    fn page5_and_page6_options_select() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 7);
        assert_eq!(p.brush(), (5, 7));
        pick(&mut p, &a, 6, 11);
        assert_eq!(p.brush(), (6, 11));
        // página 3: efeitos (bolha = anel espaçado)
        p.update(&[click(510, 62), click(510, 62), click(510, 62)], 0.0, &a);
        assert_eq!(p.page(), 3);
        p.update(&[click(35, 70)], 0.0, &a);
        assert_eq!(p.brush(), (3, 0));
        p.update(&[click(200, 300), up(200, 300)], 0.0, &a);
        assert!(painted(&p) > 20);
        assert_eq!(p.canvas().get(200 - 23, 300 - 109), Some(0));
    }

    #[test]
    fn rectangle_commits_on_mouseup_and_egg_undoes() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 6);
        p.update(&[click(100, 200)], 0.0, &a);
        p.update(&[Input::MouseMove { x: 150, y: 250 }], 0.0, &a);
        assert_eq!(painted(&p), 0);
        // a prévia aparece na tela sem mudar o quadro
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        assert_ne!(f.pixel(120, 220), [255, 255, 255, 255]);
        assert_eq!(f.pixel(300, 300), [255, 255, 255, 255]);
        assert_eq!(painted(&p), 0);
        p.update(&[up(150, 250)], 0.0, &a);
        assert_eq!(painted(&p), 51 * 51);
        p.update(&[Input::MouseMove { x: 300, y: 300 }], 0.0, &a);
        assert_eq!(painted(&p), 51 * 51);
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 0);
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 51 * 51);
    }

    #[test]
    fn page5_outlines_and_curves_paint() {
        let (_d, a) = empty_assets();
        // contorno de retângulo 11×11 = 40 pixels
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 9);
        p.update(&[click(100, 200), up(110, 210)], 0.0, &a);
        assert_eq!(painted(&p), 40);
        // reta com base: diagonal de 11 + base de 10 novos
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 1);
        p.update(&[click(100, 200), up(110, 210)], 0.0, &a);
        assert_eq!(painted(&p), 21);
        // curva fechada: três vértices viram um triângulo fechado
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 3);
        p.update(
            &[
                click(100, 200),
                Input::MouseMove { x: 110, y: 200 },
                up(110, 210),
            ],
            0.0,
            &a,
        );
        assert_eq!(painted(&p), 30);
        // serpentina de 3 pixels: um ponto só pinta 3×3
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 5);
        p.update(&[click(100, 200), up(100, 200)], 0.0, &a);
        assert_eq!(painted(&p), 9);
        // poligonal: movimentos curtos não viram vértice
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 4);
        p.update(
            &[
                click(100, 200),
                Input::MouseMove { x: 103, y: 200 },
                Input::MouseMove { x: 100, y: 203 },
                up(100, 200),
            ],
            0.0,
            &a,
        );
        assert_eq!(painted(&p), 1);
    }

    #[test]
    fn spray_adds_pixels_over_time() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 6, 0);
        p.update(&[click(300, 300)], 0.0, &a);
        let first = painted(&p);
        assert!(first > 0);
        for _ in 0..30 {
            p.update(&[], 0.016, &a);
        }
        let later = painted(&p);
        assert!(later > first);
        // tudo dentro do raio 6
        for (k, &v) in p.canvas().px.iter().enumerate() {
            if v != W {
                let (x, y) = ((k as u32 % CANVAS_W) as i32, (k as u32 / CANVAS_W) as i32);
                let (dx, dy) = (x - (300 - ORIGIN.0), y - (300 - ORIGIN.1));
                assert!(dx * dx + dy * dy <= 36, "pixel fora do raio: {x},{y}");
            }
        }
        p.update(&[up(300, 300)], 0.0, &a);
        let ended = painted(&p);
        for _ in 0..10 {
            p.update(&[], 0.016, &a);
        }
        assert_eq!(painted(&p), ended);
        // um traço inteiro é uma só ação do ovo
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 0);
    }

    #[test]
    fn draw_does_not_panic_without_data() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        let mut f = Frame::new(640, 480);
        for t in [70, 320, 129, 228, 20] {
            p.update(&[click(t, 30)], 0.0, &a);
            p.draw(&mut f, &a);
        }
        assert_eq!(f.pixel(300, 300), [255, 255, 255, 255]);
    }
}
