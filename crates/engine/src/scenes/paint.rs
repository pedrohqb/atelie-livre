//! Tela de pintura do Fine Artist: o quadro do Ateliê de Pintura com pincel,
//! lata de tinta, palavras, ovo desfazer, aspirador (limpar), cores e padrões.
//! Posições medidas no original (spec 2b, seção 4).

use super::paint_sfx as sfx;
use super::projects::flipbook::Flipbook;
use super::projects::{Project, ProjectKind};
use super::sounds::{SPOTS_NAME, Sounds};
use super::stickers::{Placed, Stickers};
use super::supertools::{GUIDES_NAME, Guides, Notice, Supertools};
use super::transformers::Transformers;
use super::wizard::{
    BTN_BACK, BTN_NEXT, BTN_STOP, MAGGIE_PANEL, Target, TrickKind, Wizard, WizardEvent,
    WizardOutcome,
};
use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::help::HelpBalloon;
use crate::paint::brush::{Pen, Stroke, Tiles};
use crate::paint::objects::words_stamp;
use crate::paint::stamp::Stamp;
use crate::paint::store::{load_drawing, save_drawing_objects};
use crate::paint::{
    Align, BalloonShape, CANVAS_H, CANVAS_W, Canvas, History, Ink, ORIGIN, Objects, Shape, Source,
    WordsStyle, page_shapes, text_pixels, words_layout,
};
use crate::scene::{Effect, Input, Key, KidCommon, MouseButton, Scene, SceneId};
use formats::fnt::Font;
use formats::palette::Rgb;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

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
const EXIT: (i32, i32, i32, i32) = (580, 18, 56, 88);
/// Nicho do McZee à direita da faixa: clique abre o balão de ajuda.
const NICHE: (i32, i32, i32, i32) = (533, 56, 47, 48);
/// Canto do balão de ajuda, à direita do quadro.
const HELP_POS: (i32, i32) = (445, 115);
/// Blocos `WHAT` da ajuda de cada ferramenta.
const WHAT_BRUSH: u16 = 0x1002;
const WHAT_BUCKET: u16 = 0x1003;
const WHAT_SOUNDS: u16 = 0x1004;
const WHAT_STICKERS: u16 = 0x1005;
const WHAT_TRANSFORMERS: u16 = 0x1006;
const WHAT_EGG: u16 = 0x1007;
const WHAT_VACUUM: u16 = 0x1008;
const WHAT_WORDS: u16 = 0x1009;
const WHAT_SUPERTOOLS: u16 = 0x100a;
const WHAT_PRINT: u16 = 0x100b;
/// Avisos do Rex ("Você ainda não definiu um ponto de fuga" é o 5).
const WHAT_ERRORS: u16 = 0x100f;
/// Bloco `WHAT` do mapa do prédio (registro 3: "Mapa do prédio").
const WHAT_MAP: u16 = 0x100e;
/// Ícone do andar, no canto inferior esquerdo da barra inferior.
const MAP_ICON: (i32, i32, i32, i32) = (24, 460, 36, 20);
/// Cartaz do prédio (o mesmo do saguão) aberto sobre o quadro, com borda.
const MAP_PIC: u16 = 0x6c01;
const MAP_FRAME: (i32, i32, i32, i32) = (0, 300, 190, 142);
const MAP_POS: (i32, i32) = (2, 302);
/// Balão "Mapa do prédio", ao lado do cartaz.
const MAP_HELP_POS: (i32, i32) = (196, 330);
/// O balão do Pincel aparece na primeira vez que a tela abre na sessão.
static FIRST_HELP: AtomicBool = AtomicBool::new(true);
const ARROW_UP: (i32, i32, i32, i32) = (504, 58, 16, 14);
const ARROW_DOWN: (i32, i32, i32, i32) = (504, 90, 16, 14);
/// Carimbos da página 2 do pincel (`ARTIST.PIC`).
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
const BUCKET_BUTTONS: [u16; 10] = [
    0x1590, 0x1591, 0x1593, 0x1595, 0x1597, 0x1599, 0x159b, 0x159d, 0x159f, 0x15a1,
];
const VACUUM_BUTTONS: [u16; 8] = [
    0x1557, 0x1559, 0x1567, 0x1561, 0x155f, 0x155b, 0x155c, 0x1569,
];
/// Opções da ferramenta Palavra: texto simples ("FORMA") e balão de fala ("Oi!").
const WORDS_BUTTONS: [u16; 2] = [0x1797, 0x1799];
/// Bloco `WHAT` da história em quadrinhos: o Cavalete de balões (fonte 17,
/// forma 18, posição 19) e o arrasto (20).
const WHAT_COMICS: u16 = 0x3e00;
/// Cavalete de história em quadrinhos na faixa: fontes, formas e posições.
const EASEL_FONTS_X0: i32 = 25;
const EASEL_SHAPES_X0: i32 = EASEL_FONTS_X0 + 2 * 37 + 8;
const EASEL_ALIGN_X0: i32 = EASEL_SHAPES_X0 + 6 * 37 + 8;
/// Menor largura de um balão esticado com Shift+arrasto.
const MIN_BALLOON_W: u32 = crate::paint::text::MIN_WRAP;
/// Período do piscar do cursor de texto, em segundos (meio período aceso).
const CARET_PERIOD: f32 = 1.0;
/// Meio período do piscar do contorno do objeto sob o ponteiro, em segundos.
const BLINK_HALF: f32 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Brush,
    Bucket,
    Stickers,
    Words,
    Sounds,
    Vacuum,
    Transformers,
    Supertools,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Select(Tool),
    Undo,
    /// Imprimir: exporta o quadro em PNG e manda para a impressora (`lp`), se houver.
    Print,
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
        active: (0x100f, 0x1011),
        action: Action::Select(Tool::Vacuum),
    },
    ToolDef {
        x: 360,
        y: 9,
        w: 36,
        normal: (0x1045, 0x104b),
        active: (0x1065, 0x1067),
        action: Action::Select(Tool::Transformers),
    },
    ToolDef {
        x: 412,
        y: 12,
        w: 35,
        normal: (0x1007, 0x1009),
        active: (0x1069, 0x106b),
        action: Action::Select(Tool::Supertools),
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

/// Botão do balão da lição: evento, posição e imagens (cor, mono).
type WizardButton = (WizardEvent, (i32, i32), (u16, u16));

/// Som do botão da faixa sob `(x, y)`: seta de página ou opção `k`.
fn strip_sound(x: i32, y: i32, option: fn(usize) -> u16) -> Option<Effect> {
    if inside(ARROW_UP, x, y) || inside(ARROW_DOWN, x, y) {
        return Some(sfx::play(sfx::PAGE_ARROW));
    }
    (0..12)
        .find(|&k| inside(option_rect(25, k), x, y))
        .map(|k| sfx::play(option(k)))
}

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

/// Andar do mapa do prédio sob `(x, y)`: faixas dos rótulos, como no
/// cartaz do saguão (relativas ao cartaz).
fn map_floor_at(x: i32, y: i32) -> Option<SceneId> {
    [
        (38, SceneId::TricksFloor),
        (58, SceneId::Projects),
        (78, SceneId::Studio),
        (98, SceneId::Building),
    ]
    .into_iter()
    .find(|&(fy, _)| inside((MAP_POS.0 + 98, MAP_POS.1 + fy, 90, 18), x, y))
    .map(|(_, dest)| dest)
}

fn canvas_rect() -> (i32, i32, i32, i32) {
    (ORIGIN.0, ORIGIN.1, CANVAS_W as i32, CANVAS_H as i32)
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

/// Balão de fala sendo arrastado com a Palavra (mover; com Shift, largura).
struct WordsDrag {
    id: u32,
    last: (i32, i32),
    resize: bool,
    /// Largura de quebra no começo do arrasto (0 = a largura do texto).
    width0: u32,
    start_x: i32,
    moved: bool,
    /// O ovo já guardou o estado anterior.
    recorded: bool,
}

/// Botão do Cavalete de balões.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EaselPick {
    Font(bool),
    Shape(BalloonShape),
    Align(Align),
}

/// Botão do Cavalete sob `(x, y)`.
fn easel_hit(x: i32, y: i32) -> Option<EaselPick> {
    if let Some(k) = (0..2).find(|&k| inside(option_rect(EASEL_FONTS_X0, k), x, y)) {
        return Some(EaselPick::Font(k == 1));
    }
    if let Some(k) =
        (0..BalloonShape::ALL.len()).find(|&k| inside(option_rect(EASEL_SHAPES_X0, k), x, y))
    {
        return Some(EaselPick::Shape(BalloonShape::ALL[k]));
    }
    (0..Align::ALL.len())
        .find(|&k| inside(option_rect(EASEL_ALIGN_X0, k), x, y))
        .map(|k| EaselPick::Align(Align::ALL[k]))
}

pub struct Paint {
    common: KidCommon,
    vacuum: VacuumMode,
    /// Canto inicial de um arrasto do aspirador (coordenadas do quadro).
    vacuum_anchor: Option<(i32, i32)>,
    /// Trecho aspirado ou copiado.
    clipboard: Option<Stamp>,
    /// Objeto sob o ponteiro quando o aspira-cópias foi apertado.
    vacuum_object: Option<u32>,
    drawings: Option<PathBuf>,
    pal: Vec<Rgb>,
    patterns: Vec<[u8; 8]>,
    canvas: Canvas,
    /// Adesivos e palavras por cima da pintura.
    objects: Objects,
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
    /// Estilo do Cavalete de balões (história em quadrinhos).
    words_style: WordsStyle,
    words: Option<WordsEdit>,
    words_drag: Option<WordsDrag>,
    /// Shift apertado.
    shift: bool,
    /// Fonte das palavras: "Pablo Graphos" 16 pt, senão 8 pt; `None` usa a 8×8 reserva.
    font: Option<Rc<Font>>,
    /// Fonte pequena do Cavalete ("Pablo Graphos" 8 pt).
    font_small: Option<Rc<Font>>,
    /// O pincel escolhido em `brush`, com suas imagens.
    pen: Pen,
    stroke: Option<Stroke>,
    changed: bool,
    stickers: Stickers,
    sounds: Sounds,
    transformers: Transformers,
    supertools: Supertools,
    /// Balão de ajuda do McZee aberto.
    help: Option<HelpBalloon>,
    /// Blocos `WHAT` já lidos.
    dialogs: HashMap<u16, Rc<Vec<formats::what::Record>>>,
    mouse: (i32, i32),
    /// Tempo acumulado, para o piscar do contorno dos objetos.
    time: f32,
    /// Projeto da Oficina em andamento, se a tela foi aberta por ele.
    project: Option<Project>,
    /// Lição dos Truques de Desenho em andamento.
    wizard: Option<Wizard>,
    trick: Option<TrickKind>,
    /// Relógio para o pisca-pisca do alvo da lição.
    wizard_t: f32,
    /// Mapa do prédio aberto sobre o quadro.
    map_open: bool,
    /// Som em laço e animação dos ícones da barra.
    sfx: sfx::State,
}

impl Paint {
    pub fn new(drawings: Option<PathBuf>, assets: &Assets) -> Paint {
        let pal = assets.palettes(Program::Artist).pal256.clone();
        let mut patterns = assets.patterns(Program::Artist);
        patterns.resize(SWATCHES.max(patterns.len()), [0; 8]);
        let pen = Pen::new(
            Tiles::load(|id| Shape::from_stamp(&assets.image(&pic(id)))),
            pal.clone(),
        );
        let (canvas, records) = drawings
            .as_ref()
            .and_then(|d| load_drawing(d, "ultimo", &pal))
            .unwrap_or_else(|| (Canvas::new(CANVAS_W, CANVAS_H), Vec::new()));
        let mut p = Paint {
            common: KidCommon::new(None),
            drawings,
            pal,
            patterns,
            canvas,
            objects: Objects::default(),
            history: History::default(),
            tool: Tool::Brush,
            color: SWATCHES - 1,
            pattern: 0,
            page: 1,
            brush: (1, 0),
            bucket_option: 0,
            words_option: 0,
            words_style: WordsStyle::balloon(),
            words: None,
            words_drag: None,
            shift: false,
            font: assets.font(16).or_else(|| assets.font(8)),
            font_small: assets.font(8),
            pen,
            stroke: None,
            changed: false,
            stickers: Stickers::new(assets),
            sounds: Sounds::new(assets),
            transformers: Transformers::new(),
            supertools: Supertools::new(),
            help: None,
            dialogs: HashMap::new(),
            mouse: (0, 0),
            time: 0.0,
            vacuum: VacuumMode::Cut,
            vacuum_anchor: None,
            vacuum_object: None,
            clipboard: None,
            project: None,
            wizard: None,
            trick: None,
            wizard_t: 0.0,
            map_open: false,
            sfx: sfx::State::default(),
        };
        p.set_brush(1, 0);
        p.restore_objects(&records, assets);
        if let Some(d) = &p.drawings {
            p.sounds.load_spots(&d.join(SPOTS_NAME));
            if let Some(g) = Guides::load(&d.join(GUIDES_NAME)) {
                p.supertools.guides = g;
            }
        }
        if FIRST_HELP.swap(false, Ordering::Relaxed) {
            p.open_help(WHAT_BRUSH, 1, assets);
        }
        p
    }

    /// Abre o balão de ajuda do registro `index` do bloco `WHAT` (se existir).
    fn open_help(&mut self, block: u16, index: usize, assets: &Assets) {
        // na lição quem fala é a Maggie
        if self.wizard.is_some() {
            return;
        }
        self.open_help_at(block, index, HELP_POS, assets);
    }

    /// Como `open_help`, com o balão em `pos`.
    fn open_help_at(&mut self, block: u16, index: usize, pos: (i32, i32), assets: &Assets) {
        let records = self
            .dialogs
            .entry(block)
            .or_insert_with(|| Rc::new(assets.dialogs(Program::Artist, block)))
            .clone();
        self.help = HelpBalloon::new(records, block, index, pos.0, pos.1);
    }

    /// Mapa do prédio aberto?
    pub fn map_open(&self) -> bool {
        self.map_open
    }

    /// Clique com o mapa do prédio aberto: um andar troca de cena (a troca
    /// grava o desenho); qualquer clique fecha o mapa.
    fn map_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>) {
        self.map_open = false;
        self.help = None;
        if let Some(dest) = map_floor_at(x, y) {
            fx.push(Effect::ChangeScene(dest));
        }
    }

    fn draw_map(&self, f: &mut Frame, a: &Assets) {
        let (x, y, w, h) = MAP_FRAME;
        f.rect(x, y, w, h, WHITE);
        f.frame_rect(x, y, w, h, 2, BLACK);
        f.blit(&a.image(&pic(MAP_PIC)), MAP_POS.0, MAP_POS.1);
    }

    /// Balão de ajuda aberto, se houver.
    pub fn help(&self) -> Option<&HelpBalloon> {
        self.help.as_ref()
    }

    /// Bloco e registro da ajuda da ferramenta atual (`option` = da opção escolhida).
    fn help_target(&self, option: bool) -> (u16, usize) {
        match self.tool {
            Tool::Brush => (
                WHAT_BRUSH,
                match (option, self.brush) {
                    (true, (2, k)) if k < 3 => 5 + k,
                    (true, (3, k)) if k < 9 => 8 + k,
                    (true, (5, k)) => 17 + k,
                    (true, (6, k)) => 40 + k,
                    _ => 1,
                },
            ),
            Tool::Bucket => (WHAT_BUCKET, if option { 1 + self.bucket_option } else { 0 }),
            Tool::Stickers => (WHAT_STICKERS, 0),
            Tool::Sounds => (WHAT_SOUNDS, 0),
            Tool::Words => (WHAT_WORDS, 0),
            Tool::Vacuum => (
                WHAT_VACUUM,
                if option {
                    match self.vacuum {
                        VacuumMode::Cut => 13,
                        VacuumMode::Stickers => 1,
                        VacuumMode::Paste => 9,
                        VacuumMode::Area => 6,
                        VacuumMode::Copy => 7,
                        VacuumMode::Color => 2,
                        VacuumMode::Sounds => 3,
                        VacuumMode::All => 4,
                    }
                } else {
                    0
                },
            ),
            Tool::Transformers => (
                WHAT_TRANSFORMERS,
                if option {
                    self.transformers.help_record()
                } else {
                    0
                },
            ),
            Tool::Supertools => (
                WHAT_SUPERTOOLS,
                if option {
                    self.supertools.help_record()
                } else {
                    0
                },
            ),
        }
    }

    fn show_help(&mut self, option: bool, assets: &Assets) {
        let (block, index) = self.help_target(option);
        self.open_help(block, index, assets);
    }

    /// Remonta os objetos lidos do `*.objetos.json` (adesivos pelos rolos,
    /// PNG do usuário na pasta de desenhos, palavras pela fonte).
    fn restore_objects(&mut self, records: &[crate::paint::objects::Record], assets: &Assets) {
        let font = self.font.clone();
        let small = self.font_small.clone();
        self.objects = Objects::from_records(records, |src| match src {
            Source::Roll { roll, item } => self.stickers.stamp_for(*roll, *item, assets, &self.pal),
            Source::Png(name) => {
                let dir = self.drawings.as_ref()?;
                // só nomes simples, dentro da pasta de desenhos
                if name.contains(['/', '\\']) {
                    return None;
                }
                crate::paint::store::load_stamp_png(&dir.join(name), &self.pal)
            }
            Source::Words { text, style, color } => {
                let f = if style.small {
                    small.as_deref().or(font.as_deref())
                } else {
                    font.as_deref()
                };
                Some(words_stamp(f, text, style, *color))
            }
        });
    }

    /// Quadro com os objetos por cima (o que se vê e o que se grava).
    pub fn composed(&self) -> Canvas {
        crate::paint::store::compose_unclipped(&self.canvas, &self.objects)
    }

    pub fn objects(&self) -> &Objects {
        &self.objects
    }

    /// Paleta de 256 cores do quadro.
    pub fn palette(&self) -> &[Rgb] {
        &self.pal
    }

    /// Coloca um adesivo como objeto (uma ação do ovo).
    fn place_sticker(&mut self, placed: Placed) {
        self.history.record_all(&self.canvas, &self.objects);
        self.objects.place(
            Rc::new(placed.stamp),
            placed.x,
            placed.y,
            Source::Roll {
                roll: placed.roll,
                item: placed.item,
            },
        );
        self.changed = true;
    }

    pub fn transformers(&self) -> &Transformers {
        &self.transformers
    }

    pub fn supertools(&self) -> &Supertools {
        &self.supertools
    }

    /// Linhas guia das Super-ferramentas (só na tela).
    pub fn guides(&self) -> &Guides {
        &self.supertools.guides
    }

    /// Objeto sob o ponteiro, se a ferramenta mexe em objetos (o contorno pisca).
    fn hovered_object(&self) -> Option<u32> {
        let handles = matches!(
            (self.tool, self.vacuum),
            (Tool::Vacuum, VacuumMode::Stickers | VacuumMode::Copy)
                | (Tool::Transformers | Tool::Supertools, _)
        );
        if !handles || !inside(canvas_rect(), self.mouse.0, self.mouse.1) {
            return None;
        }
        self.objects
            .hit(self.mouse.0 - ORIGIN.0, self.mouse.1 - ORIGIN.1)
    }

    /// Abre a tela como um projeto da Oficina: reabre o último trabalho desse
    /// tipo (`ultimo-<tipo>.png`), se houver, senão um quadro novo.
    /// Abre a tela como uma lição dos Truques de Desenho: quadro novo e o
    /// assistente da Maggie (spec `2026-10-31-truques-design.md`).
    pub fn new_tricks(kind: TrickKind, drawings: Option<PathBuf>, assets: &Assets) -> Paint {
        let mut p = Paint::new(None, assets);
        p.drawings = drawings;
        p.help = None;
        p.wizard = Some(match kind {
            TrickKind::Basic => super::tricks_basic::wizard(),
            TrickKind::ThreeD => super::tricks_3d::wizard(),
        });
        p.trick = Some(kind);
        p
    }

    pub fn wizard(&self) -> Option<&Wizard> {
        self.wizard.as_ref()
    }

    /// Botões Volta / Pára / Segue do balão da lição, com o evento de cada um.
    fn wizard_buttons(&self, assets: &Assets) -> Vec<WizardButton> {
        let Some(w) = &self.wizard else {
            return Vec::new();
        };
        let b = w.balloon(assets, "");
        let y = b.y + b.height(assets) + 4;
        let mut out = Vec::new();
        let mut x = b.x + 12;
        if w.has_back() && !w.stopping() {
            out.push((WizardEvent::Back, (x, y), BTN_BACK));
            x += 41;
        }
        if !w.stopping() {
            out.push((WizardEvent::Stop, (x, y), BTN_STOP));
            x += 41;
        }
        if w.has_next() && !w.stopping() {
            out.push((WizardEvent::Next, (x, y), BTN_NEXT));
        }
        out
    }

    /// Clique durante a lição: balão, botões, painel da Maggie e McZee são
    /// do assistente (devolve `true`); o resto segue para a pintura.
    fn wizard_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) -> bool {
        let Some(w) = &self.wizard else { return false };
        let b = w.balloon(assets, "");
        let mut ev = b.hit(assets, x, y).map(WizardEvent::Option);
        if ev.is_none() {
            ev = self
                .wizard_buttons(assets)
                .into_iter()
                .find(|(_, (bx, by), _)| inside((*bx, *by, 31, 14), x, y))
                .map(|(e, _, _)| e);
        }
        let w = self.wizard.as_ref().expect("lição");
        if ev.is_none() {
            ev = w.panel_hit(x, y);
        }
        if ev.is_none() && inside(NICHE, x, y) {
            ev = Some(WizardEvent::McZee);
        }
        match ev {
            Some(e) => {
                self.wizard_event(e, fx, assets);
                true
            }
            // o balão é opaco: clique dentro dele sem opção não vai para o quadro
            None => b.contains(assets, x, y),
        }
    }

    fn wizard_event(&mut self, e: WizardEvent, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(w) = self.wizard.as_mut() else {
            return;
        };
        match w.on_event(e) {
            WizardOutcome::Stay => {}
            WizardOutcome::Entered(_) => {
                if let Some(id) = w.step().your_area {
                    self.paint_guide_picture(id, assets);
                }
            }
            WizardOutcome::FreePaint => self.wizard = None,
            WizardOutcome::Leave(id) => fx.push(Effect::ChangeScene(id)),
        }
    }

    /// Pinta no quadro a figura de partida do passo ("já comecei para você"):
    /// os pixels não brancos, centrados, nas cores mais próximas da paleta.
    fn paint_guide_picture(&mut self, id: u16, assets: &Assets) {
        let img = assets.image(&pic(id));
        let ox = (CANVAS_W as i32 - img.width as i32) / 2;
        let oy = (CANVAS_H as i32 - img.height as i32) / 2;
        self.history.record_all(&self.canvas, &self.objects);
        for y in 0..img.height {
            for x in 0..img.width {
                let i = ((y * img.width + x) * 4) as usize;
                let (r, g, b) = (img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]);
                if (r, g, b) == (255, 255, 255) {
                    continue;
                }
                let (cx, cy) = (ox + x as i32, oy + y as i32);
                if cx < 0 || cy < 0 || cx >= CANVAS_W as i32 || cy >= CANVAS_H as i32 {
                    continue;
                }
                let k = crate::paint::store::nearest(&self.pal, r, g, b);
                self.canvas.px[(cy as u32 * CANVAS_W + cx as u32) as usize] = k;
            }
        }
        self.changed = true;
    }

    /// Depois de um clique comum: avisa o assistente da ferramenta ou do botão
    /// da faixa escolhidos (é assim que os passos "clique em…" avançam).
    fn wizard_after_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        if self.wizard.is_none() {
            return;
        }
        if let Some(i) = TOOLS
            .iter()
            .position(|t| inside((t.x - 5, 5, t.w + 11, 50), x, y))
        {
            self.wizard_event(WizardEvent::Tool(i), fx, assets);
        } else if (63..95).contains(&y) && (25..500).contains(&x) && (x - 25) % 37 < 32 {
            self.wizard_event(
                WizardEvent::OptionButton(((x - 25) / 37) as usize),
                fx,
                assets,
            );
        }
    }

    fn draw_wizard(&self, f: &mut Frame, a: &Assets) {
        let Some(w) = &self.wizard else { return };
        w.draw_panel(f, a);
        let blink = (self.wizard_t * 3.0) as i32 % 2 == 0;
        let color = if blink { [255, 238, 76] } else { BLACK };
        match w.target() {
            Some(Target::Tool(i)) => {
                if let Some(t) = TOOLS.get(i) {
                    f.frame_rect(t.x - 6, 4, t.w + 13, 52, 3, color);
                }
            }
            Some(Target::Option(k)) => {
                let r = option_rect(25, k);
                f.frame_rect(r.0 - 3, r.1 - 3, r.2 + 6, r.3 + 6, 3, color);
            }
            Some(Target::Panel) | Some(Target::PanelArea(_)) => {
                let (x, y, pw, ph) = MAGGIE_PANEL;
                f.frame_rect(x - 5, y - 5, pw + 10, ph + 10, 3, color);
            }
            None => {}
        }
        let fallback = self.trick.map_or("", |t| t.label());
        w.balloon(a, fallback).draw(f, a);
        for (_, (x, y), (img, _mono)) in self.wizard_buttons(a) {
            f.blit(&a.image(&pic(img)), x, y);
        }
    }

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
        if kind == ProjectKind::Comics
            && let Some(tails) = crate::clipart::load_tails(assets)
        {
            p.stickers.add_roll(tails);
        }
        if let Some((c, records)) = p
            .drawings
            .as_ref()
            .and_then(|d| load_drawing(d, &format!("ultimo-{}", kind.file_stem()), &p.pal))
        {
            p.canvas = c;
            p.restore_objects(&records, assets);
        }
        let mut project = Project::new(kind);
        project.load_assets(assets);
        if let Some(d) = &p.drawings {
            let stem = format!("ultimo-{}", kind.file_stem());
            project.apply_meta(&super::projects::load_meta(d, &stem));
        }
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
        if let Err(e) = std::fs::create_dir_all(dir) {
            return Effect::Log(format!(
                "imprimir: não consegui criar {}: {e}",
                dir.display()
            ));
        }
        let flat = self.composed();
        let page = match &self.project {
            Some(p) => p.print_page(&flat, &self.pal, self.font.as_deref()),
            None => flat,
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
        self.pen.select(page, k);
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

    /// A faixa da Palavra é o Cavalete de balões (só na história em quadrinhos).
    fn words_easel(&self) -> bool {
        matches!(self.project, Some(Project::Comics(_)))
    }

    /// Estilo da palavra em edição: o do Cavalete, ou a opção da faixa.
    fn current_style(&self) -> WordsStyle {
        if self.words_easel() {
            self.words_style
        } else if self.words_option == 1 {
            WordsStyle::balloon()
        } else {
            WordsStyle::plain()
        }
    }

    /// Fonte de um estilo (a pequena, se pedida e instalada).
    fn words_font(&self, style: &WordsStyle) -> Option<&Font> {
        if style.small {
            self.font_small.as_deref().or(self.font.as_deref())
        } else {
            self.font.as_deref()
        }
    }

    /// Grava no quadro o texto em edição (uma ação de desfazer); texto vazio não pinta nada.
    fn commit_words(&mut self) {
        let Some(w) = self.words.take() else {
            return;
        };
        if w.text.is_empty() {
            return;
        }
        let style = self.current_style();
        let color = COLORS[self.color];
        let stamp = words_stamp(self.words_font(&style), &w.text, &style, color);
        // o canto do texto fica onde o clique foi; o balão sobra em volta
        let l = words_layout(self.words_font(&style), &w.text, &style);
        self.history.record_all(&self.canvas, &self.objects);
        self.objects.place(
            Rc::new(stamp),
            w.x - l.grid.pad_x,
            w.y - l.grid.pad_y,
            Source::Words {
                text: w.text,
                style,
                color,
            },
        );
        self.changed = true;
    }

    /// Reabre uma palavra colocada para mudar o texto: o objeto sai (o ovo
    /// devolve) e o texto volta a ser digitado no mesmo lugar e estilo.
    fn edit_words(&mut self, id: u32) {
        let Some(o) = self.objects.get(id) else {
            return;
        };
        let Source::Words { text, style, color } = o.source.clone() else {
            return;
        };
        let (ox, oy) = (o.x, o.y);
        let l = words_layout(self.words_font(&style), &text, &style);
        self.objects.remove(id);
        if self.words_easel() {
            self.words_style = style;
        } else {
            self.words_option = usize::from(style.has_balloon());
        }
        if let Some(k) = COLORS.iter().position(|&c| c == color) {
            self.color = k;
        }
        self.words = Some(WordsEdit {
            x: ox + l.grid.pad_x,
            y: oy + l.grid.pad_y,
            text,
            blink: 0.0,
        });
        self.changed = true;
    }

    /// Começa a arrastar a palavra `id` com a Palavra (mover; com Shift, a largura).
    fn press_words_object(&mut self, id: u32, cx: i32, cy: i32) {
        let width0 = match self.objects.get(id).map(|o| &o.source) {
            Some(Source::Words { style, .. }) => style.width,
            _ => 0,
        };
        self.words_drag = Some(WordsDrag {
            id,
            last: (cx, cy),
            resize: self.shift,
            width0,
            start_x: cx,
            moved: false,
            recorded: false,
        });
    }

    fn drag_words_object(&mut self, cx: i32, cy: i32) {
        let Some(d) = self.words_drag.as_mut() else {
            return;
        };
        if (cx, cy) == d.last {
            return;
        }
        if !d.recorded {
            self.history.record_all(&self.canvas, &self.objects);
            d.recorded = true;
        }
        d.moved = true;
        let (id, resize, width0, start_x, last) = (d.id, d.resize, d.width0, d.start_x, d.last);
        d.last = (cx, cy);
        if resize {
            let Some(o) = self.objects.get(id) else {
                return;
            };
            let Source::Words { text, style, color } = o.source.clone() else {
                return;
            };
            let base = if width0 == 0 {
                words_layout(self.words_font(&style), &text, &style).tw as u32
            } else {
                width0
            };
            let width = (base as i32 + cx - start_x).max(MIN_BALLOON_W as i32) as u32;
            let style = WordsStyle { width, ..style };
            let stamp = words_stamp(self.words_font(&style), &text, &style, color);
            if let Some(o) = self.objects.get_mut(id) {
                o.w = stamp.w as i32;
                o.h = stamp.h as i32;
                o.image = Rc::new(stamp);
                o.source = Source::Words { text, style, color };
            }
        } else {
            self.objects.move_by(id, cx - last.0, cy - last.1);
        }
        self.changed = true;
    }

    /// Solta a palavra arrastada; um clique sem mover reabre o texto.
    fn release_words_object(&mut self) {
        let Some(d) = self.words_drag.take() else {
            return;
        };
        if !d.moved {
            self.history.record_all(&self.canvas, &self.objects);
            self.edit_words(d.id);
        }
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

    /// Lata de tinta em `(cx, cy)` do quadro: dentro de um adesivo ou palavra
    /// pinta o próprio objeto (como no original); no fundo, as bordas dos
    /// objetos seguram a tinta.
    fn pour(&mut self, cx: i32, cy: i32) {
        let ink = self.ink();
        if let Some(id) = self.objects.hit(cx, cy) {
            self.history.record_all(&self.canvas, &self.objects);
            let Some(o) = self.objects.get_mut(id) else {
                return;
            };
            let (ox, oy) = (cx - o.x, cy - o.y);
            if !o.fill_at(ox, oy, &ink) {
                return;
            }
            // a imagem mudou: passa a ser um adesivo do usuário, gravado ao lado
            let name = format!(
                "adesivo-{}-{id}.png",
                crate::paint::store::timestamp(now_secs())
            );
            if let Some(dir) = &self.drawings
                && std::fs::create_dir_all(dir).is_ok()
                && crate::paint::store::save_stamp_png(&o.image, &self.pal, &dir.join(&name))
                    .is_ok()
            {
                o.source = Source::Png(name);
            }
            self.changed = true;
            return;
        }
        let seen = self.composed();
        let before = self.canvas.px.clone();
        self.canvas.fill_within(cx, cy, &ink, &seen.px);
        if self.canvas.px != before {
            self.history.record_previous(before);
            self.changed = true;
        }
    }

    fn ink(&self) -> Ink {
        Ink {
            color: COLORS[self.color],
            pattern: self.patterns[self.pattern],
        }
    }

    fn finish_sticker(&mut self) {
        if let Some(placed) = self.stickers.finish() {
            self.place_sticker(placed);
        }
    }

    fn option_count(&self) -> usize {
        match self.tool {
            Tool::Stickers | Tool::Sounds | Tool::Transformers | Tool::Supertools => 0,
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
        // as formas elásticas guardam o desfazer só ao soltar
        let elastic = self.pen.elastic();
        if !elastic {
            self.history.record(&self.canvas);
        }
        let ink = self.ink();
        let seed = (cx as u32).wrapping_mul(2654435761)
            ^ (cy as u32).wrapping_mul(40503)
            ^ (now_secs() as u32);
        self.stroke = self.pen.press(&mut self.canvas, (cx, cy), &ink, seed);
        self.changed |= !elastic && self.stroke.is_some();
    }

    fn extend_stroke(&mut self, cx: i32, cy: i32) {
        let ink = self.ink();
        if let Some(s) = self.stroke.as_mut() {
            self.pen.drag(s, &mut self.canvas, (cx, cy), &ink);
        }
    }

    /// Solta o botão: a forma elástica vira pintura (uma ação do ovo).
    fn end_stroke(&mut self, cx: i32, cy: i32) {
        self.extend_stroke(cx, cy);
        let Some(s) = self.stroke.take() else {
            return;
        };
        let ink = self.ink();
        let before = s.is_elastic().then(|| self.canvas.px.clone());
        self.pen.release(s, &mut self.canvas, (cx, cy), &ink);
        if let Some(before) = before
            && before != self.canvas.px
        {
            self.history.record_previous(before);
            self.changed = true;
        }
    }

    /// Uma atualização com o botão apertado (o spray borrifa).
    fn spray_tick(&mut self) {
        let ink = self.ink();
        if let Some(s) = self.stroke.as_mut() {
            self.changed |= self.pen.tick(s, &mut self.canvas, &ink);
        }
    }

    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        if self.map_open {
            self.map_click(x, y, fx);
            return;
        }
        if inside(MAP_ICON, x, y) {
            self.map_open = true;
            fx.push(sfx::play(sfx::MAP));
            self.open_help_at(WHAT_MAP, 3, MAP_HELP_POS, assets);
            return;
        }
        // desenho animado: fora da porta de saída, tudo é do projeto
        if let Some(Project::Flipbook(fb)) = self.project.as_mut()
            && !inside(EXIT, x, y)
        {
            fx.extend(fb.click(x, y, self.drawings.as_deref(), assets));
            return;
        }
        if let Some(h) = self.help.take() {
            if h.contains(assets, x, y) {
                // ligação (dica, tecla, glossário) abre o registro ligado no lugar
                self.help = match h.link_at(assets, x, y) {
                    Some(target) => h.follow(target).or(Some(h)),
                    None => Some(h),
                };
                return;
            }
            // clique fora fecha o balão e segue
        }
        self.stroke = None;
        // cria-lados e triângulo 2 terminam com um clique fora do quadro
        // (as tiras de cores e padrões não contam)
        if (ORIGIN.0..ORIGIN.0 + CANVAS_W as i32).contains(&x) && !inside(canvas_rect(), x, y) {
            self.pen.end_figure();
        }
        self.finish_sticker();
        // qualquer clique encerra o texto que estava sendo digitado
        if self.words.as_ref().is_some_and(|w| !w.text.is_empty()) {
            fx.push(sfx::play(sfx::WORDS_DONE));
        }
        self.commit_words();
        if let Some(p) = self.project.as_mut() {
            if let Some(changed) = p.click(x, y, &mut self.canvas, &mut self.history, assets) {
                self.changed |= changed;
                return;
            }
            if inside(canvas_rect(), x, y) {
                p.clip_at(&mut self.canvas, x - ORIGIN.0, y - ORIGIN.1);
            }
        }
        if self.tool == Tool::Stickers {
            let before = (self.stickers.holding(), self.stickers.film_open());
            match self.stickers.click(x, y, assets, &self.pal) {
                Some(placed) => {
                    let arrow = inside(ARROW_UP, x, y) || inside(ARROW_DOWN, x, y);
                    let after = (self.stickers.holding(), self.stickers.film_open());
                    fx.extend(sfx::sticker(placed.is_some(), before, after, arrow).map(sfx::play));
                    if let Some(placed) = placed {
                        self.place_sticker(placed);
                    }
                    return;
                }
                // qualquer outro clique solta o adesivo que estava na mão
                None => self.stickers.reset(),
            }
        }
        if self.tool == Tool::Sounds {
            // som preso a um objeto: colocar no adesivo clicado, ou tocar o dele
            if inside(canvas_rect(), x, y)
                && let Some(id) = self.objects.hit(x - ORIGIN.0, y - ORIGIN.1)
            {
                if let Some((_, snd)) = self.sounds.take_chosen() {
                    self.history.record_all(&self.canvas, &self.objects);
                    if let Some(o) = self.objects.get_mut(id) {
                        o.sound = Some(snd.clone());
                    }
                    fx.push(Effect::PlaySound(snd));
                    self.changed = true;
                    return;
                }
                if let Some(snd) = self.objects.get(id).and_then(|o| o.sound.clone()) {
                    fx.push(Effect::PlaySound(snd));
                    return;
                }
            }
            match self.sounds.click(x, y) {
                Some(Some(snd)) => {
                    fx.push(Effect::PlaySound(snd));
                    self.changed = true;
                    return;
                }
                Some(None) => {
                    if inside(ARROW_UP, x, y) || inside(ARROW_DOWN, x, y) {
                        fx.push(sfx::play(sfx::PAGE_ARROW));
                    } else if !inside(canvas_rect(), x, y) {
                        fx.push(sfx::play(sfx::SOUND_CATEGORY));
                    }
                    return;
                }
                None => self.sounds.reset(),
            }
        }
        if inside(canvas_rect(), x, y) {
            let (cx, cy) = (x - ORIGIN.0, y - ORIGIN.1);
            match self.tool {
                Tool::Brush => {
                    self.start_stroke(cx, cy);
                    if self.stroke.is_some() {
                        let id = sfx::stroke(self.brush.0, self.brush.1);
                        fx.push(self.sfx.start_loop(id));
                    }
                }
                Tool::Words => {
                    // em cima de uma palavra colocada: arrastar (ou clicar para mudar)
                    let hit = self.objects.hit(cx, cy).filter(|&id| {
                        matches!(
                            self.objects.get(id).map(|o| &o.source),
                            Some(Source::Words { .. })
                        )
                    });
                    match hit {
                        Some(id) => self.press_words_object(id, cx, cy),
                        None => {
                            self.start_words(cx, cy);
                            fx.push(sfx::play(sfx::WORDS_START));
                        }
                    }
                }
                Tool::Bucket => {
                    fx.push(sfx::play(sfx::bucket(self.bucket_option)));
                    self.pour(cx, cy);
                }
                Tool::Vacuum => {
                    let k = VACUUM_MODES.iter().position(|&m| m == self.vacuum);
                    match k.and_then(sfx::vacuum) {
                        Some((id, true)) => fx.push(self.sfx.start_loop(id)),
                        Some((id, false)) => fx.push(sfx::play(id)),
                        None => {}
                    }
                    self.vacuum_press(cx, cy);
                }
                Tool::Transformers => {
                    if self.transformers.press(
                        cx,
                        cy,
                        &mut self.objects,
                        &self.canvas,
                        &mut self.history,
                    ) {
                        self.changed = true;
                    }
                }
                Tool::Supertools => {
                    if sfx::supertool_traces(self.supertools.op()) {
                        fx.push(self.sfx.start_loop(sfx::GUIDE_STROKE));
                    }
                    let mut changed = false;
                    let notice = self.supertools.press(
                        cx,
                        cy,
                        &mut self.objects,
                        &self.canvas,
                        &mut self.history,
                        &mut changed,
                    );
                    self.changed |= changed;
                    if let Some(n) = notice {
                        self.notice(n, fx, assets);
                    }
                }
                Tool::Stickers | Tool::Sounds => {}
            }
            return;
        }
        if inside(sfx::MAGNIFIER_ICON, x, y) {
            fx.push(sfx::play(sfx::MAGNIFIER));
            return;
        }
        if self.project.is_none() && inside(NICHE, x, y) {
            fx.push(sfx::play(sfx::MCZEE));
            self.show_help(false, assets);
            return;
        }
        if inside(EXIT, x, y) {
            let back = if self.trick.is_some() {
                SceneId::TricksFloor
            } else if self.project.is_some() {
                SceneId::Projects
            } else {
                SceneId::Studio
            };
            fx.push(Effect::ChangeScene(back));
            return;
        }
        if let Some((i, t)) = TOOLS
            .iter()
            .enumerate()
            .find(|(_, t)| inside((t.x - 5, 5, t.w + 11, 50), x, y))
        {
            fx.push(sfx::play(sfx::TOOLS[i]));
            self.sfx.tool_clicked(i);
            match t.action {
                Action::Select(tool) => {
                    self.tool = tool;
                    self.stickers.reset();
                    self.sounds.reset();
                    self.transformers.reset();
                    self.supertools.reset();
                    self.show_help(false, assets);
                }
                Action::Undo => {
                    if self.history.toggle(&mut self.canvas, &mut self.objects) {
                        self.changed = true;
                    }
                    self.open_help(WHAT_EGG, 0, assets);
                }
                Action::Print => {
                    fx.push(self.print());
                    self.open_help(WHAT_PRINT, 0, assets);
                }
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
        if self.tool == Tool::Transformers {
            if let Some(log) = self.transformers.click_strip(x, y) {
                fx.extend(strip_sound(x, y, sfx::transformer_option));
                if let Some(msg) = log {
                    fx.push(Effect::Log(msg.into()));
                }
                if !inside(ARROW_UP, x, y) && !inside(ARROW_DOWN, x, y) {
                    self.show_help(true, assets);
                }
            }
            return;
        }
        if self.tool == Tool::Supertools {
            if self.supertools.click_strip(x, y) {
                fx.extend(strip_sound(x, y, sfx::supertool_option));
                if inside(crate::scenes::supertools::SWITCH, x, y) {
                    self.open_help(WHAT_SUPERTOOLS, 17, assets);
                } else {
                    self.show_help(true, assets);
                }
            }
            return;
        }
        if self.tool == Tool::Brush {
            if inside(ARROW_UP, x, y) {
                fx.push(sfx::play(sfx::PAGE_ARROW));
                self.page = if self.page == 1 {
                    PAGE_COUNT
                } else {
                    self.page - 1
                };
                return;
            }
            if inside(ARROW_DOWN, x, y) {
                fx.push(sfx::play(sfx::PAGE_ARROW));
                self.page = self.page % PAGE_COUNT + 1;
                return;
            }
        }
        if self.tool == Tool::Words && self.words_easel() {
            if let Some(pick) = easel_hit(x, y) {
                fx.push(sfx::play(sfx::WORDS_OPTION));
                let record = match pick {
                    EaselPick::Font(small) => {
                        self.words_style.small = small;
                        17
                    }
                    EaselPick::Shape(shape) => {
                        self.words_style.shape = shape;
                        18
                    }
                    EaselPick::Align(align) => {
                        self.words_style.align = align;
                        19
                    }
                };
                self.open_help(WHAT_COMICS, record, assets);
            }
            return;
        }
        let x0 = self.options_x0();
        if let Some(k) = (0..self.option_count()).find(|&k| inside(option_rect(x0, k), x, y)) {
            fx.push(sfx::play(match self.tool {
                Tool::Words => sfx::WORDS_OPTION,
                Tool::Vacuum => sfx::VACUUM_OPTION,
                _ => sfx::OPTION,
            }));
            match self.tool {
                Tool::Brush => self.set_brush(self.page, k),
                Tool::Bucket => self.bucket_option = k,
                Tool::Words => self.words_option = k,
                Tool::Stickers | Tool::Sounds | Tool::Transformers | Tool::Supertools => {}
                Tool::Vacuum => {
                    self.vacuum = VACUUM_MODES[k];
                    if self.vacuum == VacuumMode::Sounds {
                        fx.push(Effect::Log("aspirador: opção ainda não disponível".into()));
                    }
                }
            }
            self.show_help(true, assets);
        }
    }

    /// Aviso de uma ferramenta: o balão do Rex do original, mais o registro.
    fn notice(&mut self, n: Notice, fx: &mut Vec<Effect>, assets: &Assets) {
        match n {
            Notice::NoVanishingPoint => {
                fx.push(Effect::Log(
                    "super-ferramentas: você ainda não definiu um ponto de fuga".into(),
                ));
                self.open_help(WHAT_ERRORS, 5, assets);
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
                self.vacuum_object = self.objects.hit(cx, cy);
            }
            // aspira-coisas: tira o objeto clicado
            VacuumMode::Stickers => {
                if let Some(id) = self.objects.hit(cx, cy) {
                    self.history.record_all(&self.canvas, &self.objects);
                    self.objects.remove(id);
                    self.changed = true;
                }
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
                self.history.record_all(&self.canvas, &self.objects);
                self.canvas.clear();
                self.objects = Objects::default();
                self.changed = true;
            }
            VacuumMode::Sounds => {}
        }
    }

    /// Imagem do objeto como está na tela (escala, inversões e giros aplicados).
    fn object_stamp(&self, id: u32) -> Option<Stamp> {
        let o = self.objects.get(id)?;
        let px = (0..o.h)
            .flat_map(|y| (0..o.w).map(move |x| (x, y)))
            .map(|(x, y)| o.pixel(x, y))
            .collect();
        Some(Stamp {
            w: o.w as u32,
            h: o.h as u32,
            px,
        })
    }

    fn vacuum_release(&mut self, cx: i32, cy: i32) {
        let object = self.vacuum_object.take();
        let Some(r) = self.drag_rect(cx, cy) else {
            self.vacuum_anchor = None;
            return;
        };
        self.vacuum_anchor = None;
        let (x0, y0, w, h) = r;
        match self.vacuum {
            // um clique (sem arrastar) num objeto copia o objeto
            VacuumMode::Copy if w <= 2 && h <= 2 && object.is_some() => {
                self.clipboard = object.and_then(|id| self.object_stamp(id));
            }
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
        let (label, name) = match (&self.project, self.trick) {
            (_, Some(t)) => ("Truques", t.label().to_string()),
            (Some(p), None) => (p.kind().label(), p.display_name()),
            (None, None) => ("Ateliê", "Quadro Novo".to_string()),
        };
        let name = name.as_str();
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
        for (i, t) in TOOLS.iter().enumerate() {
            let active = t.action == Action::Select(self.tool);
            let (mut img, mut mask) = if active { t.active } else { t.normal };
            let (mut x, mut y) = (t.x, t.y);
            let anim = self.sfx.icon(i, t.normal.1);
            if let Some(im) = anim {
                // centrada no ícone normal
                let (n, m) = (a.image(&pic(t.normal.0)), a.image(&pic(im.0)));
                x += (n.width as i32 - m.width as i32) / 2;
                y += (n.height as i32 - m.height as i32) / 2;
                (img, mask) = im;
            }
            if active || anim.is_some() {
                let w = a
                    .image(&pic(if anim.is_some() { t.normal.0 } else { img }))
                    .width as i32;
                f.rect(t.x - 5, 5, w + 11, 51, LIGHT);
                f.rect(t.x - 6, 5, 1, 50, BLACK);
                f.rect(t.x + w + 6, 5, 1, 50, BLACK);
            }
            f.blit_masked(&a.image(&pic(img)), &a.image(&pic(mask)), x, y);
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
            Tool::Words if self.words_easel() => {
                self.draw_easel(f, a);
                None
            }
            Tool::Words => {
                for (k, id) in WORDS_BUTTONS.iter().enumerate() {
                    let r = option_rect(25, k);
                    f.blit(&a.image(&pic(*id)), r.0, r.1);
                }
                Some(self.words_option)
            }
            Tool::Transformers => {
                self.transformers.draw_strip(f, a);
                None
            }
            Tool::Supertools => {
                self.supertools.draw_strip(f, a);
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
        let flat = self.composed();
        for (k, &i) in flat.px.iter().enumerate() {
            let x = ORIGIN.0 + (k as u32 % flat.w) as i32;
            let y = ORIGIN.1 + (k as u32 / flat.w) as i32;
            f.put(x, y, self.rgb(i));
        }
    }

    /// Contorno piscando (preto e branco a cada 0,25 s) do objeto sob o ponteiro.
    fn draw_blink(&self, f: &mut Frame) {
        let Some(id) = self.hovered_object() else {
            return;
        };
        let Some(o) = self.objects.get(id) else {
            return;
        };
        let on = (self.time / BLINK_HALF) as i64 % 2 == 0;
        let c = if on { BLACK } else { WHITE };
        let r = canvas_rect();
        for (x, y) in o.outline() {
            let (sx, sy) = (ORIGIN.0 + x, ORIGIN.1 + y);
            if inside(r, sx, sy) {
                f.put(sx, sy, c);
            }
        }
    }

    /// Texto em edição sobre o quadro, com o balão (se for o estilo) e o cursor piscando.
    fn draw_words(&self, f: &mut Frame) {
        let Some(w) = self.words.as_ref() else {
            return;
        };
        let style = self.current_style();
        let font = self.words_font(&style);
        let l = words_layout(font, &w.text, &style);
        // o canto do texto fica no clique; a imagem começa `pad` antes
        let (sx, sy) = (ORIGIN.0 + w.x - l.grid.pad_x, ORIGIN.1 + w.y - l.grid.pad_y);
        let r = canvas_rect();
        let mut put = |x: i32, y: i32, rgb: [u8; 3]| {
            if inside(r, x, y) {
                f.put(x, y, rgb);
            }
        };
        for y in 0..l.grid.h {
            for x in 0..l.grid.w {
                if let Some(edge) = l.grid.at(x, y) {
                    put(sx + x, sy + y, if edge { BLACK } else { WHITE });
                }
            }
        }
        let ink = self.rgb(COLORS[self.color]);
        for (k, (line, dx)) in l.lines.iter().enumerate() {
            let (x0, y0) = (
                sx + l.grid.pad_x + dx,
                sy + l.grid.pad_y + k as i32 * l.line_h,
            );
            text_pixels(font, line, |dx, dy| put(x0 + dx, y0 + dy, ink));
        }
        if w.blink % CARET_PERIOD < CARET_PERIOD / 2.0 {
            for dy in 0..l.line_h {
                put(sx + l.caret.0, sy + l.caret.1 + dy, ink);
            }
        }
    }

    /// Cavalete de balões na faixa: duas fontes, seis formas (a última é o
    /// "espaço em branco", sem balão) e três posições; as escolhas em vermelho.
    fn draw_easel(&self, f: &mut Frame, a: &Assets) {
        let st = &self.words_style;
        let button = |f: &mut Frame, r: (i32, i32, i32, i32), chosen: bool| {
            f.rect(r.0, r.1, r.2, r.3, WHITE);
            f.frame_rect(r.0, r.1, r.2, r.3, 1, BLACK);
            if chosen {
                f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, RED);
            }
        };
        for k in 0..2 {
            let r = option_rect(EASEL_FONTS_X0, k);
            button(f, r, st.small == (k == 1));
            let font = if k == 0 {
                self.font.as_deref()
            } else {
                self.font_small.as_deref().or(self.font.as_deref())
            };
            let (tw, th) = crate::paint::text_size(font, "Aa");
            let (x0, y0) = (r.0 + (r.2 - tw as i32) / 2, r.1 + (r.3 - th as i32) / 2);
            text_pixels(font, "Aa", |dx, dy| {
                if inside(r, x0 + dx, y0 + dy) {
                    f.put(x0 + dx, y0 + dy, BLACK);
                }
            });
        }
        for (k, shape) in BalloonShape::ALL.iter().enumerate() {
            let r = option_rect(EASEL_SHAPES_X0, k);
            button(f, r, st.shape == *shape);
            let g = crate::paint::text::balloon_grid(*shape, 10, 5);
            let (x0, y0) = (r.0 + (r.2 - g.w) / 2, r.1 + (r.3 - g.h) / 2);
            for y in 0..g.h {
                for x in 0..g.w {
                    if let Some(edge) = g.at(x, y)
                        && inside(r, x0 + x, y0 + y)
                    {
                        f.put(x0 + x, y0 + y, if edge { BLACK } else { WHITE });
                    }
                }
            }
            // as "palavras": um tracinho no bloco de texto
            f.rect(x0 + g.pad_x, y0 + g.pad_y + 2, 10, 1, DARK);
        }
        for (k, align) in Align::ALL.iter().enumerate() {
            let r = option_rect(EASEL_ALIGN_X0, k);
            button(f, r, st.align == *align);
            for (row, len) in [(0, 20), (1, 12), (2, 16), (3, 8)] {
                let x = match align {
                    Align::Left => r.0 + 6,
                    Align::Center => r.0 + (r.2 - len) / 2,
                    Align::Right => r.0 + r.2 - 6 - len,
                };
                f.rect(x, r.1 + 7 + row * 5, len, 2, BLACK);
            }
        }
        if let Some(font) = a.font(8) {
            f.text_font(&font, EASEL_ALIGN_X0 + 3 * 37 + 4, 66, "Cavalete", BLACK);
        }
    }

    /// Prévia elástica (rachadura, teia e página 5), só na tela (o quadro não muda).
    fn draw_preview(&self, f: &mut Frame) {
        let Some(s) = &self.stroke else {
            return;
        };
        let ink = self.ink();
        for (x, y) in self.pen.preview(s) {
            if self.canvas.can_paint(x, y) {
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
        self.wizard_t += dt;
        self.sfx.update(dt);
        for i in input {
            if let Some(p) = self.project.as_mut()
                && p.dialog_input(i, assets)
            {
                continue;
            }
            match *i {
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.mouse = (x, y);
                    if self.wizard.is_some() && self.wizard_click(x, y, &mut fx, assets) {
                        continue;
                    }
                    self.click(x, y, &mut fx, assets);
                    self.wizard_after_click(x, y, &mut fx, assets);
                }
                Input::Shift(down) => self.shift = down,
                Input::MouseMove { x, y } => {
                    self.mouse = (x, y);
                    self.drag_words_object(x - ORIGIN.0, y - ORIGIN.1);
                    self.extend_stroke(x - ORIGIN.0, y - ORIGIN.1);
                    if self
                        .transformers
                        .drag(x - ORIGIN.0, y - ORIGIN.1, &mut self.objects)
                    {
                        self.changed = true;
                    }
                    self.supertools.drag(x - ORIGIN.0, y - ORIGIN.1);
                }
                Input::MouseUp {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    if std::mem::take(&mut self.sfx.looping) {
                        fx.push(Effect::StopLoop);
                    }
                    let painting = self.stroke.is_some();
                    self.end_stroke(x - ORIGIN.0, y - ORIGIN.1);
                    if painting && self.wizard.is_some() {
                        self.wizard_event(WizardEvent::Painted, &mut fx, assets);
                    }
                    if self.vacuum_anchor.is_some() {
                        self.vacuum_release(x - ORIGIN.0, y - ORIGIN.1);
                    }
                    self.release_words_object();
                    self.transformers.release();
                    self.supertools.release();
                }
                Input::Key {
                    key,
                    ctrl: false,
                    alt: false,
                } => match self.project.as_mut() {
                    Some(Project::Flipbook(fb)) => fb.key(key),
                    _ => {
                        if key == Key::Enter
                            && self.words.as_ref().is_some_and(|w| !w.text.is_empty())
                        {
                            fx.push(sfx::play(sfx::WORDS_DONE));
                        }
                        self.key(key)
                    }
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
        if dt.is_finite() && dt > 0.0 {
            self.time = (self.time + dt) % 3600.0;
        }
        self.spray_tick();
        self.sounds.update(dt);
        if self.transformers.update(dt, &mut self.objects) {
            self.changed = true;
        }
        if self.supertools.update(dt, &mut self.objects) {
            self.changed = true;
        }
        if let Some(placed) = self.stickers.update(dt) {
            self.place_sticker(placed);
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
            if self.map_open {
                self.draw_map(frame, assets);
                if let Some(h) = &self.help {
                    h.draw(frame, assets);
                }
            }
            return;
        }
        match &self.project {
            Some(p) if p.asking() => p.draw_strip(frame, assets),
            _ => self.draw_options(frame, assets),
        }
        self.draw_columns(frame);
        self.draw_canvas(frame);
        self.supertools.guides.draw(frame);
        self.draw_blink(frame);
        if self.tool == Tool::Transformers {
            self.transformers.draw_overlay(frame, &self.objects);
        }
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
        if self.map_open {
            self.draw_map(frame, assets);
        }
        if let Some(h) = &self.help {
            h.draw(frame, assets);
        }
        self.draw_wizard(frame, assets);
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
        if self.trick.is_some() {
            // a lição grava só o que foi desenhado, sem mexer no último desenho
            if !self.changed {
                return Vec::new();
            }
            let flat = self.composed();
            let path = dir.join(format!(
                "truque-{}.png",
                crate::paint::store::timestamp(now_secs())
            ));
            self.changed = false;
            return vec![Effect::Log(
                match crate::paint::store::save_png(&flat, &self.pal, &path) {
                    Ok(()) => format!("desenho da lição guardado em {}", path.display()),
                    Err(e) => format!("não consegui gravar {}: {e}", path.display()),
                },
            )];
        }
        if let Err(e) = self.sounds.save_spots(&dir.join(SPOTS_NAME)) {
            return vec![Effect::Log(format!("sons: {e}"))];
        }
        if let Err(e) = self.supertools.guides.save(&dir.join(GUIDES_NAME)) {
            return vec![Effect::Log(format!("guias: {e}"))];
        }
        let saved = match &self.project {
            Some(p) => super::projects::save_picture_meta(
                dir,
                p.kind(),
                &self.canvas,
                &self.objects,
                &self.pal,
                &p.meta(),
                self.changed,
                None,
                now_secs(),
            ),
            None => save_drawing_objects(
                dir,
                &self.canvas,
                &self.objects,
                &self.pal,
                self.changed,
                now_secs(),
            ),
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
    use crate::paint::store::{LAST_NAME, load_png};
    use crate::scenes::stickers::ANIMALS;
    use crate::scenes::testutil::empty_assets;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn mv(x: i32, y: i32) -> Input {
        Input::MouseMove { x, y }
    }

    fn up(x: i32, y: i32) -> Input {
        Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    /// Pixels não brancos da composição (fundo + objetos).
    fn painted(p: &Paint) -> usize {
        p.composed().px.iter().filter(|&&v| v != W).count()
    }

    #[test]
    fn exit_door_goes_to_studio() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        let fx = p.update(&[click(600, 60)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Studio)));
    }

    #[test]
    fn building_map_opens_goes_to_floor_and_closes() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(35, 470)], 0.0, &a);
        assert!(p.map_open());
        // "Oficina de Projetos"
        let fx = p.update(&[click(MAP_POS.0 + 120, MAP_POS.1 + 66)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Projects)));
        assert!(!p.map_open());
        // clique fora fecha sem trocar de cena nem pintar
        p.update(&[click(35, 470)], 0.0, &a);
        assert!(p.map_open());
        let fx = p.update(&[click(320, 200)], 0.0, &a);
        assert!(!p.map_open());
        assert!(!fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))));
        assert_eq!(painted(&p), 0);
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

    fn artist(id: u16) -> crate::assets::SoundRef {
        crate::assets::SoundRef::Snd(Program::Artist, id)
    }

    #[test]
    fn choosing_tools_plays_measured_sounds() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        for (x, id) in [(70, 0x0214), (230, 0x0209), (330, 0x0215), (30, 0x0213)] {
            let fx = p.update(&[click(x, 30)], 0.0, &a);
            assert!(fx.contains(&Effect::PlaySound(artist(id))), "{x}: {fx:?}");
        }
        // opção da faixa e seta de página do pincel
        let fx = p.update(&[click(25 + 37 * 3 + 16, 79)], 0.0, &a);
        assert!(fx.contains(&Effect::PlaySound(artist(0x021c))));
        let fx = p.update(&[click(ARROW_DOWN.0 + 4, ARROW_DOWN.1 + 4)], 0.0, &a);
        assert!(fx.contains(&Effect::PlaySound(artist(0x021d))));
        // cor: sem som
        let r = color_rect(3);
        let fx = p.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert!(!fx.iter().any(|e| matches!(e, Effect::PlaySound(_))));
    }

    #[test]
    fn brush_stroke_loops_until_release() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        let fx = p.update(&[click(100, 150)], 0.0, &a);
        assert!(fx.contains(&Effect::PlayLoop(artist(0x021e))), "{fx:?}");
        let fx = p.update(&[mv(200, 200)], 0.0, &a);
        assert!(!fx.contains(&Effect::StopLoop));
        let fx = p.update(&[up(200, 200)], 0.0, &a);
        assert!(fx.contains(&Effect::StopLoop));
        // soltar de novo não para outra vez
        let fx = p.update(&[up(200, 200)], 0.0, &a);
        assert!(!fx.contains(&Effect::StopLoop));
        // forma da página 3 (efeitos): som da opção escolhida
        p.set_brush(3, 4);
        let fx = p.update(&[click(100, 150)], 0.0, &a);
        assert!(fx.contains(&Effect::PlayLoop(artist(0x0301))), "{fx:?}");
    }

    #[test]
    fn bucket_pour_and_vacuum_sounds() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.update(&[click(70, 30)], 0.0, &a);
        let fx = p.update(&[click(300, 300)], 0.0, &a);
        assert!(fx.contains(&Effect::PlaySound(artist(0x0225))));
        p.update(&[click(330, 30)], 0.0, &a);
        let fx = p.update(&[click(200, 200)], 0.0, &a);
        assert!(fx.contains(&Effect::PlayLoop(artist(0x0229))));
        let fx = p.update(&[up(250, 250)], 0.0, &a);
        assert!(fx.contains(&Effect::StopLoop));
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

    /// Moldura quadrada de 40×40 com 4 px de borda preta, miolo branco opaco
    /// e um buraco transparente de 10×10 no meio.
    fn ring() -> Rc<Stamp> {
        let mut px = vec![Some(crate::paint::WHITE); 40 * 40];
        for y in 0..40 {
            for x in 0..40 {
                if !(4..36).contains(&x) || !(4..36).contains(&y) {
                    px[y * 40 + x] = Some(crate::paint::BLACK);
                } else if (15..25).contains(&x) && (15..25).contains(&y) {
                    px[y * 40 + x] = None;
                }
            }
        }
        Rc::new(Stamp { w: 40, h: 40, px })
    }

    #[test]
    fn bucket_inside_sticker_paints_the_sticker() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
        let id = p.objects.place(ring(), 100, 100, Source::Png("x".into()));
        p.update(&[click(70, 30)], 0.0, &a);
        // miolo branco do adesivo (fora do buraco)
        p.update(&[click(ORIGIN.0 + 108, ORIGIN.1 + 108)], 0.0, &a);
        let o = p.objects.get(id).unwrap();
        assert_eq!(o.pixel(8, 8), Some(COLORS[SWATCHES - 1]));
        assert_eq!(o.pixel(1, 1), Some(crate::paint::BLACK));
        assert_eq!(o.pixel(20, 20), None);
        // o fundo não foi tocado e o adesivo virou um PNG do usuário
        assert!(p.canvas.px.iter().all(|&i| i == crate::paint::WHITE));
        assert!(matches!(&o.source, Source::Png(n) if n.starts_with("adesivo-")));
        let Source::Png(n) = &o.source else {
            unreachable!()
        };
        let back = crate::paint::store::load_stamp_png(&dir.path().join(n), &p.pal).unwrap();
        assert_eq!(back.px[8 * 40 + 8], Some(COLORS[SWATCHES - 1]));
        assert_eq!(back.px[20 * 40 + 20], None);
        // o ovo desfaz
        p.update(&[click(279, 30)], 0.0, &a);
        assert_eq!(
            p.objects.get(id).unwrap().pixel(8, 8),
            Some(crate::paint::WHITE)
        );
    }

    #[test]
    fn bucket_outside_stops_at_sticker_edges() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.objects.place(ring(), 100, 100, Source::Png("x".into()));
        p.update(&[click(70, 30), click(ORIGIN.0 + 5, ORIGIN.1 + 5)], 0.0, &a);
        // o fundo à vista no buraco do adesivo continua branco
        assert_eq!(
            p.canvas.px[(120 * CANVAS_W + 120) as usize],
            crate::paint::WHITE
        );
        // o resto do fundo (fora do adesivo) foi pintado
        assert_ne!(
            p.canvas.px[(5 * CANVAS_W + 5) as usize],
            crate::paint::WHITE
        );
        assert_ne!(
            p.canvas.px[(300 * CANVAS_W + 500) as usize],
            crate::paint::WHITE
        );
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
    fn supertools_guides_are_overlay_only_and_saved() {
        use crate::scenes::supertools::{Grid, Op};
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
        p.update(&[click(430, 30)], 0.0, &a);
        assert_eq!(p.tool(), Tool::Supertools);
        // quadriculado grande (3.º botão) e ponto de fuga no quadro
        p.update(&[click(25 + 37 * 2 + 10, 70)], 0.0, &a);
        assert_eq!(p.guides().grid, Grid::Big);
        p.update(&[click(25 + 37 * 5 + 10, 70)], 0.0, &a);
        assert_eq!(p.supertools().op(), Op::VanishingPoint);
        p.update(&[click(300, 250), up(300, 250)], 0.0, &a);
        assert_eq!(p.guides().horizon, Some((250 - ORIGIN.1, 300 - ORIGIN.0)));
        // linha do PF sem PF avisa (depois de apagar o horizonte com o ímã)
        p.update(
            &[click(25 + 37 * 10 + 10, 70), click(100, 250), up(100, 250)],
            0.0,
            &a,
        );
        assert_eq!(p.guides().horizon, None);
        let fx = p.update(
            &[click(25 + 37 * 6 + 10, 70), click(100, 300), up(100, 300)],
            0.0,
            &a,
        );
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::Log(s) if s.contains("ponto de fuga")))
        );
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        assert_eq!(painted(&p), 0);
        p.leave();
        assert!(dir.path().join(GUIDES_NAME).exists());
        let q = Paint::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(q.guides().grid, Grid::Big);
        // o PNG não tem guias
        let flat = load_png(&dir.path().join(LAST_NAME), q.palette()).unwrap();
        assert!(flat.px.iter().all(|&v| v == W));
    }

    #[test]
    fn transformers_move_object_by_drag_and_strip_pages() {
        use crate::scenes::transformers::Op;
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        let id = place_fish(&mut p, &a, 300, 300);
        let r = p.objects().get(id).unwrap().rect();
        let o = p.objects().get(id).unwrap();
        let (x, y) = (0..o.h)
            .flat_map(|y| (0..o.w).map(move |x| (x, y)))
            .find(|&(x, y)| o.pixel(x, y).is_some())
            .unwrap();
        let (sx, sy) = (ORIGIN.0 + o.x + x, ORIGIN.1 + o.y + y);
        p.update(&[click(378, 30)], 0.0, &a);
        assert_eq!(p.tool(), Tool::Transformers);
        assert_eq!(p.transformers().op(), Op::Move);
        p.update(
            &[click(sx, sy), mv(sx + 20, sy + 5), up(sx + 20, sy + 5)],
            0.0,
            &a,
        );
        assert_eq!(
            p.objects().get(id).unwrap().rect(),
            (r.0 + 20, r.1 + 5, r.2, r.3)
        );
        // página 2, inverter lateral
        p.update(&[click(510, 95)], 0.0, &a);
        assert_eq!(p.transformers().page(), 2);
        p.update(&[click(25 + 37 * 4 + 10, 70)], 0.0, &a);
        assert_eq!(p.transformers().op(), Op::FlipH);
        p.update(&[click(sx + 20, sy + 5), up(sx + 20, sy + 5)], 0.0, &a);
        assert!(p.objects().get(id).unwrap().flip_h);
        // o ovo desfaz a inversão
        p.update(&[click(270, 30)], 0.0, &a);
        assert!(!p.objects().get(id).unwrap().flip_h);
        let mut f = Frame::new(640, 480);
        p.update(&[mv(sx + 20, sy + 5)], 0.1, &a);
        p.draw(&mut f, &a);
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

    /// Coloca o peixe-espada (bicho 1) centrado em `(x, y)` e espera a animação.
    fn place_fish(p: &mut Paint, a: &Assets, x: i32, y: i32) -> u32 {
        p.update(&[click(129, 30), click(40, 80), click(54, 152)], 0.0, a);
        p.update(&[click(x, y)], 0.0, a);
        p.update(&[], 10.0, a);
        p.objects().items.last().unwrap().id
    }

    #[test]
    fn sticker_object_sound_follows_object_and_is_saved() {
        use crate::assets::SoundRef;
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = Paint::new(Some(dir.path().to_path_buf()), &a);
        let id = place_fish(&mut p, &a, 300, 300);
        let o = p.objects().get(id).unwrap();
        let (x, y) = (0..o.h)
            .flat_map(|y| (0..o.w).map(move |x| (x, y)))
            .find(|&(x, y)| o.pixel(x, y).is_some())
            .unwrap();
        let (sx, sy) = (ORIGIN.0 + o.x + x, ORIGIN.1 + o.y + y);
        // sem biblioteca de sons não há som para escolher: prende um à mão
        let snd = SoundRef::Library {
            file: "MAG100.SND".into(),
            index: 3,
        };
        p.objects.get_mut(id).unwrap().sound = Some(snd.clone());
        p.update(&[click(170, 30)], 0.0, &a);
        let fx = p.update(&[click(sx, sy)], 0.0, &a);
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::PlaySound(s) if *s == snd))
        );
        p.leave();
        let q = Paint::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(q.objects().get(id).unwrap().sound, Some(snd));
    }

    #[test]
    fn copy_vacuum_click_copies_object_and_paste_paints_pixels() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        let id = place_fish(&mut p, &a, 300, 300);
        let o = p.objects().get(id).unwrap();
        let (x, y) = (0..o.h)
            .flat_map(|y| (0..o.w).map(move |x| (x, y)))
            .find(|&(x, y)| o.pixel(x, y).is_some())
            .unwrap();
        let (sx, sy) = (ORIGIN.0 + o.x + x, ORIGIN.1 + o.y + y);
        p.update(&[click(328, 30), click(25 + 37 * 4 + 10, 70)], 0.0, &a);
        assert_eq!(p.vacuum_mode(), VacuumMode::Copy);
        p.update(&[click(sx, sy), up(sx, sy)], 0.0, &a);
        assert!(p.has_clipboard());
        assert_eq!(p.objects().len(), 1);
        p.update(&[click(25 + 37 * 2 + 10, 70), click(100, 400)], 0.0, &a);
        // a cópia é tinta no fundo, não um objeto
        assert_eq!(p.objects().len(), 1);
        assert!(p.canvas().px.iter().any(|&v| v != W));
    }

    #[test]
    fn jogar_tudo_fora_removes_objects_and_egg_restores_them() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        place_fish(&mut p, &a, 300, 300);
        p.update(&[click(328, 30), click(25 + 37 * 7 + 10, 70)], 0.0, &a);
        p.update(&[click(300, 300)], 0.0, &a);
        assert_eq!(p.objects().len(), 0);
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(p.objects().len(), 1);
    }

    #[test]
    fn blink_outline_only_with_thing_vacuum_over_object() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        let id = place_fish(&mut p, &a, 300, 300);
        let o = p.objects().get(id).unwrap();
        let (x, y) = (0..o.h)
            .flat_map(|y| (0..o.w).map(move |x| (x, y)))
            .find(|&(x, y)| o.pixel(x, y).is_some())
            .unwrap();
        let (sx, sy) = (ORIGIN.0 + o.x + x, ORIGIN.1 + o.y + y);
        p.update(&[Input::MouseMove { x: sx, y: sy }], 0.0, &a);
        assert_eq!(p.hovered_object(), None);
        p.update(&[click(328, 30), click(25 + 37 + 10, 70)], 0.0, &a);
        p.update(&[Input::MouseMove { x: sx, y: sy }], 0.0, &a);
        assert_eq!(p.hovered_object(), Some(id));
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        p.update(&[], 0.3, &a);
        p.draw(&mut f, &a);
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
        let c = p.composed();
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
        let c = p.composed();
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
        // página 3: bolha do tamanho do movimento, oca
        p.update(&[click(510, 62), click(510, 62), click(510, 62)], 0.0, &a);
        assert_eq!(p.page(), 3);
        p.update(&[click(35, 70)], 0.0, &a);
        assert_eq!(p.brush(), (3, 0));
        p.update(
            &[
                click(200, 300),
                Input::MouseMove { x: 230, y: 300 },
                up(230, 300),
            ],
            0.0,
            &a,
        );
        assert!(painted(&p) > 100);
        assert_eq!(p.canvas().get(230 - 23, 300 - 109), Some(0));
        assert_ne!(p.canvas().get(260 - 23, 300 - 109), Some(0));
    }

    #[test]
    fn rectangle_commits_on_mouseup_and_egg_undoes() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 9);
        p.update(&[click(100, 200)], 0.0, &a);
        p.update(&[Input::MouseMove { x: 150, y: 250 }], 0.0, &a);
        assert_eq!(painted(&p), 0);
        // a prévia aparece na tela sem mudar o quadro
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        assert_ne!(f.pixel(100, 220), [255, 255, 255, 255]);
        assert_eq!(f.pixel(120, 220), [255, 255, 255, 255]);
        assert_eq!(painted(&p), 0);
        p.update(&[up(150, 250)], 0.0, &a);
        assert_eq!(painted(&p), 4 * 50);
        p.update(&[Input::MouseMove { x: 300, y: 300 }], 0.0, &a);
        assert_eq!(painted(&p), 4 * 50);
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 0);
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(painted(&p), 4 * 50);
    }

    #[test]
    fn page5_shapes_paint_through_the_scene() {
        let (_d, a) = empty_assets();
        // contorno de retângulo 11×11 = 40 pixels
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 9);
        p.update(&[click(100, 200), up(110, 210)], 0.0, &a);
        assert_eq!(painted(&p), 40);
        // linha radial a 45°: 11 pixels
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 1);
        p.update(&[click(100, 200), up(110, 210)], 0.0, &a);
        assert_eq!(painted(&p), 11);
        // cria-lados: o segundo arrasto liga o fim do primeiro ao ponto onde soltou
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 5, 4);
        p.update(&[click(100, 200), up(150, 200)], 0.0, &a);
        p.update(&[click(300, 300), up(150, 250)], 0.0, &a);
        assert_eq!(painted(&p), 51 + 50);
        // clique fora do quadro termina a figura: o próximo lado é solto
        p.update(&[click(485, 100)], 0.0, &a);
        p.update(&[click(300, 300), up(310, 300)], 0.0, &a);
        assert_eq!(painted(&p), 51 + 50 + 11);
    }

    #[test]
    fn spray_adds_pixels_over_time() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 3, 2);
        p.update(&[click(300, 300)], 0.0, &a);
        let first = painted(&p);
        assert!(first > 0);
        for _ in 0..30 {
            p.update(&[], 0.016, &a);
        }
        let later = painted(&p);
        assert!(later > first);
        // tudo dentro do raio 10
        for (k, &v) in p.canvas().px.iter().enumerate() {
            if v != W {
                let (x, y) = ((k as u32 % CANVAS_W) as i32, (k as u32 / CANVAS_W) as i32);
                let (dx, dy) = (x - (300 - ORIGIN.0), y - (300 - ORIGIN.1));
                assert!(dx * dx + dy * dy <= 100, "pixel fora do raio: {x},{y}");
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
    fn page6_chalk_paints_textured_and_egg_undoes() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        pick(&mut p, &a, 6, 1);
        p.update(
            &[
                click(100, 200),
                Input::MouseMove { x: 300, y: 200 },
                up(300, 200),
            ],
            0.0,
            &a,
        );
        let n = painted(&p);
        // faixa de 16 de altura por ~216 de largura, com uns 3/4 pintados
        assert!((1800..3300).contains(&n), "{n}");
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
