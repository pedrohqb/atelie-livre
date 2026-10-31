//! Tela de pintura do Fine Artist: o quadro do Ateliê de Pintura com pincel,
//! lata de tinta, ovo desfazer, aspirador (limpar), cores e padrões.
//! Posições medidas no original (spec 2b, seção 4).

use super::stickers::Stickers;
use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::paint::stamp::Stamp;
use crate::paint::store::{LAST_NAME, load_png, save_drawing};
use crate::paint::{
    CANVAS_H, CANVAS_W, Canvas, History, Ink, ORIGIN, Shape, line_points, page_shapes,
};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use formats::palette::Rgb;
use std::path::PathBuf;

/// Cores da coluna esquerda: índices da paleta de 256 cores, de cima para baixo.
pub const COLORS: [u8; 38] = [
    0, 2, 18, 19, 20, 21, 22, 5, 6, 25, 26, 27, 28, 9, 10, 31, 32, 33, 7, 8, 36, 37, 38, 11, 12,
    41, 42, 43, 3, 4, 46, 47, 48, 14, 13, 15, 52, 1,
];
const SWATCHES: usize = 38;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const DARK: [u8; 3] = [100, 100, 100];
const LIGHT: [u8; 3] = [140, 140, 140];
const ORANGE: [u8; 3] = [255, 140, 50];
const BROWN: [u8; 3] = [185, 101, 36];
const RED: [u8; 3] = [250, 0, 57];

const PAGE_COUNT: u8 = 5;
const STAMP_SPACING: i32 = 16;
const EXIT: (i32, i32, i32, i32) = (580, 18, 56, 88);
const ARROW_UP: (i32, i32, i32, i32) = (504, 58, 16, 14);
const ARROW_DOWN: (i32, i32, i32, i32) = (504, 90, 16, 14);
/// Carimbos da página 2 do pincel (`ARTIST.PIC`).
const STAMPS: [u16; 12] = [
    0x164b, 0x16a9, 0x16ab, 0x16ad, 0x16af, 0x16b1, 0x16b3, 0x16b5, 0x16b7, 0x16b9, 0x16bb, 0x16bd,
];
/// Botões das páginas 2, 3 e 5 do pincel.
const PAGE2_BUTTONS: [u16; 12] = [
    0x17e5, 0x17e7, 0x17e9, 0x1814, 0x1817, 0x1819, 0x181b, 0x181d, 0x181f, 0x1821, 0x1823, 0x1825,
];
const PAGE3_BUTTONS: [u16; 12] = [
    0x17eb, 0x17ed, 0x17ef, 0x17f1, 0x17f3, 0x17f5, 0x17f7, 0x17f9, 0x17fb, 0x1827, 0x1829, 0x182b,
];
const PAGE5_BUTTONS: [u16; 12] = [
    0x17b5, 0x17cf, 0x17d1, 0x17d3, 0x17d5, 0x17d7, 0x17d9, 0x17db, 0x17dd, 0x17df, 0x17e1, 0x17e3,
];
const BUCKET_BUTTONS: [u16; 10] = [
    0x1590, 0x1591, 0x1593, 0x1595, 0x1597, 0x1599, 0x159b, 0x159d, 0x159f, 0x15a1,
];
const VACUUM_BUTTONS: [u16; 8] = [
    0x1557, 0x1559, 0x1567, 0x1561, 0x155f, 0x155b, 0x155c, 0x1569,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Brush,
    Bucket,
    Stickers,
    Vacuum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Select(Tool),
    Undo,
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
        action: Action::Unavailable("sons"),
    },
    ToolDef {
        x: 211,
        y: 13,
        w: 35,
        normal: (0x1031, 0x103b),
        active: (0x1061, 0x1063),
        action: Action::Unavailable("palavras"),
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
        action: Action::Unavailable("imprimir"),
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

struct Stroke {
    last: (i32, i32),
    last_stamp: (i32, i32),
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
    canvas: Canvas,
    history: History,
    tool: Tool,
    color: usize,
    pattern: usize,
    page: u8,
    /// Forma escolhida: (página, botão).
    brush: (u8, usize),
    bucket_option: usize,
    /// Forma correspondente a `brush`, guardada para não recriar a cada movimento.
    shape: Option<Shape>,
    stroke: Option<Stroke>,
    changed: bool,
    stickers: Stickers,
    mouse: (i32, i32),
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
            canvas,
            history: History::default(),
            tool: Tool::Brush,
            color: SWATCHES - 1,
            pattern: 0,
            page: 1,
            brush: (1, 0),
            bucket_option: 0,
            shape: None,
            stroke: None,
            changed: false,
            stickers: Stickers::new(assets),
            mouse: (0, 0),
            vacuum: VacuumMode::Cut,
            vacuum_anchor: None,
            clipboard: None,
        };
        p.set_brush(1, 0);
        p
    }

    fn set_brush(&mut self, page: u8, k: usize) {
        self.brush = (page, k);
        self.shape = match page {
            2 => self.stamps.get(k).cloned(),
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
            Tool::Stickers => 0,
            Tool::Brush => 12,
            Tool::Bucket => BUCKET_BUTTONS.len(),
            Tool::Vacuum => VACUUM_BUTTONS.len(),
        }
    }

    fn options_x0(&self) -> i32 {
        if self.tool == Tool::Bucket { 113 } else { 25 }
    }

    fn start_stroke(&mut self, cx: i32, cy: i32) {
        let Some(shape) = self.shape.as_ref() else {
            return;
        };
        self.history.record(&self.canvas);
        let ink = self.ink();
        self.canvas.stamp(shape, cx, cy, &ink);
        self.changed = true;
        self.stroke = Some(Stroke {
            last: (cx, cy),
            last_stamp: (cx, cy),
        });
    }

    fn extend_stroke(&mut self, cx: i32, cy: i32) {
        let Some(shape) = self.shape.as_ref() else {
            return;
        };
        let Some(st) = self.stroke.as_mut() else {
            return;
        };
        let ink = Ink {
            color: COLORS[self.color],
            pattern: self.patterns[self.pattern],
        };
        let spaced = self.brush.0 == 2;
        for p in line_points(st.last, (cx, cy)).into_iter().skip(1) {
            if spaced {
                let (dx, dy) = (p.0 - st.last_stamp.0, p.1 - st.last_stamp.1);
                if dx * dx + dy * dy < STAMP_SPACING * STAMP_SPACING {
                    continue;
                }
                st.last_stamp = p;
            }
            self.canvas.stamp(shape, p.0, p.1, &ink);
        }
        st.last = (cx, cy);
    }

    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        self.stroke = None;
        self.finish_sticker();
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
        if inside(canvas_rect(), x, y) {
            let (cx, cy) = (x - ORIGIN.0, y - ORIGIN.1);
            match self.tool {
                Tool::Brush => self.start_stroke(cx, cy),
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
                Tool::Stickers => {}
            }
            return;
        }
        if inside(EXIT, x, y) {
            fx.push(Effect::ChangeScene(SceneId::Studio));
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
                }
                Action::Undo => {
                    if self.history.toggle(&mut self.canvas) {
                        self.changed = true;
                    }
                }
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
                Tool::Brush if matches!(self.page, 1 | 2 | 4) => self.set_brush(self.page, k),
                Tool::Brush => fx.push(Effect::Log(format!(
                    "página {} do pincel: ainda não disponível",
                    self.page
                ))),
                Tool::Bucket => self.bucket_option = k,
                Tool::Stickers => {}
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
        match a.font(8) {
            Some(font) => {
                f.text_font(&font, 69, 461, "Ateliê", BLACK);
                let name = "Quadro Novo";
                let w = font.width(name) as i32;
                f.text_font(&font, 320 - w / 2, 462, name, BLACK);
            }
            None => {
                f.text(64, 464, "Ateliê", BLACK);
                f.text(276, 464, "Quadro Novo", BLACK);
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
                                _ => &PAGE5_BUTTONS,
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
                    self.extend_stroke(x - ORIGIN.0, y - ORIGIN.1);
                    self.stroke = None;
                    if self.vacuum_anchor.is_some() {
                        self.vacuum_release(x - ORIGIN.0, y - ORIGIN.1);
                    }
                }
                _ => {}
            }
        }
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
        self.draw_options(frame, assets);
        self.draw_columns(frame);
        self.draw_canvas(frame);
        // retângulo elástico do aspirador
        if self.vacuum_anchor.is_some()
            && let Some((x0, y0, w, h)) =
                self.drag_rect(self.mouse.0 - ORIGIN.0, self.mouse.1 - ORIGIN.1)
        {
            frame.frame_rect(ORIGIN.0 + x0, ORIGIN.1 + y0, w, h, 1, BLACK);
        }
        self.stickers
            .draw_overlay(frame, assets, &self.pal, self.mouse);
    }

    fn leave(&mut self) -> Vec<Effect> {
        self.stroke = None;
        self.finish_sticker();
        let Some(dir) = &self.drawings else {
            return vec![Effect::Log(
                "sem pasta de desenhos (HOME e XDG_DATA_HOME vazios): desenho não guardado".into(),
            )];
        };
        let msg = match save_drawing(dir, &self.canvas, &self.pal, self.changed, now_secs()) {
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
        assert_eq!(p.page(), 5);
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
        let fx = p.update(&[click(228, 30)], 0.0, &a);
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::Log(s) if s.contains("palavras")))
        );
        assert_eq!(p.tool(), Tool::Brush);
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
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
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

    #[test]
    fn draw_does_not_panic_without_data() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        let mut f = Frame::new(640, 480);
        for t in [70, 320, 129, 20] {
            p.update(&[click(t, 30)], 0.0, &a);
            p.draw(&mut f, &a);
        }
        assert_eq!(f.pixel(300, 300), [255, 255, 255, 255]);
    }
}
