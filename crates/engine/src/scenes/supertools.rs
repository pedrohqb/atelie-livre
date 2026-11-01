//! Super-ferramentas de desenho da tela de pintura (`1007`; ajuda `WHAT
//! 100a`): linhas guia que ficam só na tela — quadriculados, ponto de fuga com
//! linha do horizonte, linhas do ponto de fuga, eixos de construção, ímã que
//! apaga uma linha e interruptor mágico que esconde todas — mais Aproximar e
//! Afastar do PF, que deslizam um objeto em perspectiva. As guias vão para
//! `ultimo.guias.json`, nunca para o PNG. Ícones e posições medidos no
//! original (`scratch/orig_sf1.png`, `tools/wine/locate.py`).

use super::transformers::{CAP_LEFT, CAP_RIGHT, option_rect};
use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::paint::objects::{MIN_SIZE, segment_distance};
use crate::paint::{CANVAS_H, CANVAS_W, Canvas, History, ORIGIN, Objects};
use std::path::Path;

type Rect = (i32, i32, i32, i32);
type Point = (i32, i32);

pub const GUIDES_NAME: &str = "ultimo.guias.json";
const RED: [u8; 3] = [250, 0, 57];
const GRID_GREEN: [u8; 3] = [0, 200, 0];
const LINE_GREEN: [u8; 3] = [0, 150, 0];
const BLUE: [u8; 3] = [60, 60, 230];
/// Passo dos quadriculados, em pixels.
pub const BIG_GRID: i32 = 48;
pub const SMALL_GRID: i32 = 24;
/// Raio do ponto de fuga e distância para pegar pontas e linhas.
const VP_R: i32 = 5;
const GRAB: f32 = 6.0;
/// Velocidade de Aproximar/Afastar do PF, em pixels por segundo.
const SLIDE_SPEED: f32 = 60.0;
/// Interruptor mágico (22×41 em x 464, y 58): ligado `1491`, desligado `1497`.
pub const SWITCH: Rect = (464, 58, 22, 41);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Grid {
    #[default]
    None,
    Big,
    Small,
}

/// Linhas guia (coordenadas do quadro).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Guides {
    pub grid: Grid,
    /// Linha do horizonte (y) e ponto de fuga (x) sobre ela.
    pub horizon: Option<(i32, i32)>,
    /// Ponta solta de cada linha do PF (a outra ponta é o PF).
    pub vp_lines: Vec<Point>,
    pub axes: Vec<(Point, Point)>,
    pub visible: bool,
}

impl Guides {
    pub fn new() -> Guides {
        Guides {
            visible: true,
            ..Default::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.grid == Grid::None
            && self.horizon.is_none()
            && self.vp_lines.is_empty()
            && self.axes.is_empty()
    }

    pub fn vp(&self) -> Option<Point> {
        self.horizon.map(|(y, x)| (x, y))
    }

    /// Apaga a linha (do PF, eixo ou horizonte) a até `GRAB` px do ponto;
    /// apagar o horizonte leva as linhas do PF junto. Verdadeiro se apagou.
    pub fn erase_at(&mut self, x: i32, y: i32) -> bool {
        let p = (x, y);
        if let Some(vp) = self.vp()
            && let Some(i) = self
                .vp_lines
                .iter()
                .position(|&e| segment_distance(e, vp, p) <= GRAB)
        {
            self.vp_lines.remove(i);
            return true;
        }
        if let Some(i) = self
            .axes
            .iter()
            .position(|&(a, b)| segment_distance(a, b, p) <= GRAB)
        {
            self.axes.remove(i);
            return true;
        }
        if let Some((hy, _)) = self.horizon
            && (y - hy).abs() as f32 <= GRAB
        {
            self.horizon = None;
            self.vp_lines.clear();
            return true;
        }
        false
    }

    pub fn to_json(&self) -> String {
        let grid = match self.grid {
            Grid::None => "nenhum",
            Grid::Big => "grande",
            Grid::Small => "pequeno",
        };
        let horizon = match self.horizon {
            Some((y, x)) => format!("[{y}, {x}]"),
            None => "null".into(),
        };
        let lines: Vec<String> = self
            .vp_lines
            .iter()
            .map(|(x, y)| format!("[{x}, {y}]"))
            .collect();
        let axes: Vec<String> = self
            .axes
            .iter()
            .map(|((x0, y0), (x1, y1))| format!("[{x0}, {y0}, {x1}, {y1}]"))
            .collect();
        format!(
            "{{\n\"quadriculado\": \"{grid}\",\n\"visivel\": {},\n\"horizonte\": {horizon},\n\"linhas_pf\": [{}],\n\"eixos\": [{}]\n}}\n",
            self.visible,
            lines.join(", "),
            axes.join(", ")
        )
    }

    /// Lê o JSON de `to_json` (tolerante: campos faltando ficam vazios).
    pub fn parse_json(text: &str) -> Option<Guides> {
        let line = |key: &str| -> Option<&str> {
            let k = format!("\"{key}\":");
            text.lines()
                .find(|l| l.trim_start().starts_with(&k))
                .map(|l| {
                    let i = l.find(&k).unwrap_or(0) + k.len();
                    l[i..].trim().trim_end_matches(',')
                })
        };
        let nums = |s: &str| -> Vec<i32> {
            s.split(|c: char| !(c.is_ascii_digit() || c == '-'))
                .filter(|t| !t.is_empty())
                .filter_map(|t| t.parse().ok())
                .collect()
        };
        let grid = match line("quadriculado")? {
            "\"grande\"" => Grid::Big,
            "\"pequeno\"" => Grid::Small,
            _ => Grid::None,
        };
        let visible = line("visivel").is_none_or(|v| v == "true");
        let horizon = line("horizonte").and_then(|h| match nums(h)[..] {
            [y, x] => Some((y, x)),
            _ => None,
        });
        let vp_lines = line("linhas_pf")
            .map(|l| nums(l).chunks_exact(2).map(|c| (c[0], c[1])).collect())
            .unwrap_or_default();
        let axes = line("eixos")
            .map(|l| {
                nums(l)
                    .chunks_exact(4)
                    .map(|c| ((c[0], c[1]), (c[2], c[3])))
                    .collect()
            })
            .unwrap_or_default();
        Some(Guides {
            grid,
            horizon,
            vp_lines,
            axes,
            visible,
        })
    }

    /// Grava as guias; sem nenhuma, apaga o arquivo.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if self.is_empty() {
            let _ = std::fs::remove_file(path);
            return Ok(());
        }
        std::fs::write(path, self.to_json())
    }

    pub fn load(path: &Path) -> Option<Guides> {
        Guides::parse_json(&std::fs::read_to_string(path).ok()?)
    }

    /// Desenha as guias sobre o quadro (só na tela).
    pub fn draw(&self, f: &mut Frame) {
        if !self.visible {
            return;
        }
        let (w, h) = (CANVAS_W as i32, CANVAS_H as i32);
        let step = match self.grid {
            Grid::None => 0,
            Grid::Big => BIG_GRID,
            Grid::Small => SMALL_GRID,
        };
        if step > 0 {
            for x in (step..w).step_by(step as usize) {
                for y in 0..h {
                    if y % 4 < 2 {
                        f.put(ORIGIN.0 + x, ORIGIN.1 + y, GRID_GREEN);
                    }
                }
            }
            for y in (step..h).step_by(step as usize) {
                for x in 0..w {
                    if x % 4 < 2 {
                        f.put(ORIGIN.0 + x, ORIGIN.1 + y, GRID_GREEN);
                    }
                }
            }
        }
        let clip = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h;
        let mut line = |a: Point, b: Point, c: [u8; 3]| {
            for (x, y) in crate::paint::line_points(a, b) {
                if clip(x, y) {
                    f.put(ORIGIN.0 + x, ORIGIN.1 + y, c);
                }
            }
        };
        if let Some(vp) = self.vp() {
            for &e in &self.vp_lines {
                line(e, vp, LINE_GREEN);
            }
        }
        for &(a, b) in &self.axes {
            line(a, b, LINE_GREEN);
        }
        if let Some((hy, vx)) = self.horizon {
            line((0, hy), (w - 1, hy), BLUE);
            for dy in -VP_R..=VP_R {
                for dx in -VP_R..=VP_R {
                    let d = dx * dx + dy * dy;
                    if d <= VP_R * VP_R && d > (VP_R - 2) * (VP_R - 2) {
                        let (x, y) = (vx + dx, hy + dy);
                        if clip(x, y) {
                            f.put(ORIGIN.0 + x, ORIGIN.1 + y, BLUE);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    NoGrid,
    BigGrid,
    SmallGrid,
    VanishingPoint,
    VpLine,
    Axis,
    /// Aproximar do PF: o objeto desliza para o PF encolhendo.
    Approach,
    /// Afastar do PF: o objeto se afasta do PF crescendo.
    Recede,
    /// Apagar linha guia (ímã).
    Eraser,
}

/// Botão da faixa: ação, casa `k` (x = 25 + 37k), ícone colorido e mono.
pub struct Button {
    pub op: Op,
    pub k: usize,
    pub color: u16,
    pub mono: u16,
}

const fn b(op: Op, k: usize, color: u16, mono: u16) -> Button {
    Button { op, k, color, mono }
}

pub const BUTTONS: [Button; 9] = [
    b(Op::NoGrid, 0, 0x1485, 0x1488),
    b(Op::SmallGrid, 1, 0x148d, 0x1490),
    b(Op::BigGrid, 2, 0x1489, 0x148c),
    b(Op::VanishingPoint, 5, 0x1477, 0x1478),
    b(Op::VpLine, 6, 0x1479, 0x147c),
    b(Op::Approach, 7, 0x1481, 0x1484),
    b(Op::Recede, 8, 0x14d5, 0x14d6),
    b(Op::Axis, 9, 0x147d, 0x1480),
    b(Op::Eraser, 10, 0x14d7, 0x14d8),
];

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Artist, id)
}

fn dist(a: Point, b: Point) -> f32 {
    (((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)) as f32).sqrt()
}

/// O que está sendo arrastado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drag {
    Vp,
    VpLine(usize),
    /// Eixo novo a partir do início.
    NewAxis(Point),
    /// Ponta `end` (0 ou 1) do eixo `idx`.
    AxisEnd(usize, usize),
    /// Eixo inteiro, pela última posição do ponteiro.
    AxisMove(usize, Point),
}

/// Objeto deslizando para (ou do) PF com o botão apertado.
#[derive(Debug, Clone, Copy)]
struct Slide {
    id: u32,
    /// Tamanho e distância ao PF quando começou.
    size0: (i32, i32),
    dist0: f32,
    /// Centro, com fração, para o movimento ser suave.
    center: (f32, f32),
}

/// Aviso que a ferramenta pede para mostrar (balão do original).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice {
    /// `WHAT 100f[5]`: "Você ainda não definiu um ponto de fuga."
    NoVanishingPoint,
}

pub struct Supertools {
    op: Op,
    pub guides: Guides,
    drag: Option<Drag>,
    slide: Option<Slide>,
}

impl Default for Supertools {
    fn default() -> Self {
        Self::new()
    }
}

impl Supertools {
    pub fn new() -> Supertools {
        Supertools {
            op: Op::VanishingPoint,
            guides: Guides::new(),
            drag: None,
            slide: None,
        }
    }

    pub fn op(&self) -> Op {
        self.op
    }

    /// Botão escolhido (casa `k`), para o realce vermelho.
    pub fn selected(&self) -> Option<usize> {
        BUTTONS.iter().find(|b| b.op == self.op).map(|b| b.k)
    }

    /// Índice do registro de ajuda (`WHAT 100a`) da ação escolhida.
    pub fn help_record(&self) -> usize {
        match self.op {
            Op::NoGrid => 1,
            Op::BigGrid => 2,
            Op::SmallGrid => 3,
            Op::VanishingPoint => 4,
            Op::VpLine => 9,
            Op::Axis => 10,
            Op::Approach => 11,
            Op::Recede => 14,
            Op::Eraser => 16,
        }
    }

    pub fn reset(&mut self) {
        self.drag = None;
        self.slide = None;
    }

    /// Clique na faixa de opções: verdadeiro se foi tratado aqui.
    pub fn click_strip(&mut self, x: i32, y: i32) -> bool {
        if inside(SWITCH, x, y) {
            self.guides.visible = !self.guides.visible;
            return true;
        }
        let Some(btn) = BUTTONS.iter().find(|b| inside(option_rect(b.k), x, y)) else {
            return false;
        };
        self.op = btn.op;
        match btn.op {
            Op::NoGrid => self.guides.grid = Grid::None,
            Op::BigGrid => self.guides.grid = Grid::Big,
            Op::SmallGrid => self.guides.grid = Grid::Small,
            _ => {}
        }
        true
    }

    /// Botão apertado em `(cx, cy)` do quadro. Devolve o aviso a mostrar, se
    /// houver; `changed` diz se o desenho (objetos) mudou.
    pub fn press(
        &mut self,
        cx: i32,
        cy: i32,
        objs: &mut Objects,
        canvas: &Canvas,
        history: &mut History,
        changed: &mut bool,
    ) -> Option<Notice> {
        let p = (cx, cy);
        match self.op {
            Op::NoGrid | Op::BigGrid | Op::SmallGrid => {}
            Op::VanishingPoint => {
                if let Some(vp) = self.guides.vp()
                    && dist(vp, p) <= GRAB
                {
                    self.drag = Some(Drag::Vp);
                } else if let Some((hy, _)) = self.guides.horizon
                    && (cy - hy).abs() as f32 <= GRAB
                {
                    self.drag = Some(Drag::Vp);
                } else {
                    self.guides.horizon = Some((cy, cx));
                    self.drag = Some(Drag::Vp);
                }
            }
            Op::VpLine => {
                if self.guides.vp().is_none() {
                    return Some(Notice::NoVanishingPoint);
                }
                if let Some(i) = self
                    .guides
                    .vp_lines
                    .iter()
                    .position(|&e| dist(e, p) <= GRAB)
                {
                    self.drag = Some(Drag::VpLine(i));
                } else {
                    self.guides.vp_lines.push(p);
                    self.drag = Some(Drag::VpLine(self.guides.vp_lines.len() - 1));
                }
            }
            Op::Axis => {
                let axes = &self.guides.axes;
                if let Some((i, end)) = axes.iter().enumerate().find_map(|(i, &(a, b))| {
                    if dist(a, p) <= GRAB {
                        Some((i, 0))
                    } else if dist(b, p) <= GRAB {
                        Some((i, 1))
                    } else {
                        None
                    }
                }) {
                    self.drag = Some(Drag::AxisEnd(i, end));
                } else if let Some(i) = axes
                    .iter()
                    .position(|&(a, b)| segment_distance(a, b, p) <= GRAB)
                {
                    self.drag = Some(Drag::AxisMove(i, p));
                } else {
                    self.drag = Some(Drag::NewAxis(p));
                }
            }
            Op::Approach | Op::Recede => {
                let Some(vp) = self.guides.vp() else {
                    return Some(Notice::NoVanishingPoint);
                };
                if let Some(id) = objs.hit(cx, cy)
                    && let Some(o) = objs.get(id)
                {
                    let c = o.center();
                    history.record_all(canvas, objs);
                    self.slide = Some(Slide {
                        id,
                        size0: (o.w, o.h),
                        dist0: dist(c, vp).max(1.0),
                        center: (c.0 as f32, c.1 as f32),
                    });
                    *changed = true;
                }
            }
            Op::Eraser => {
                self.guides.erase_at(cx, cy);
            }
        }
        None
    }

    /// Ponteiro movido com o botão apertado.
    pub fn drag(&mut self, cx: i32, cy: i32) {
        let p = (cx, cy);
        match self.drag {
            None => {}
            Some(Drag::Vp) => self.guides.horizon = Some((cy, cx)),
            Some(Drag::VpLine(i)) => {
                if let Some(e) = self.guides.vp_lines.get_mut(i) {
                    *e = p;
                }
            }
            Some(Drag::NewAxis(start)) => {
                if dist(start, p) >= 2.0 {
                    self.guides.axes.push((start, p));
                    self.drag = Some(Drag::AxisEnd(self.guides.axes.len() - 1, 1));
                }
            }
            Some(Drag::AxisEnd(i, end)) => {
                if let Some(ax) = self.guides.axes.get_mut(i) {
                    if end == 0 {
                        ax.0 = p;
                    } else {
                        ax.1 = p;
                    }
                }
            }
            Some(Drag::AxisMove(i, last)) => {
                if let Some(ax) = self.guides.axes.get_mut(i) {
                    let (dx, dy) = (cx - last.0, cy - last.1);
                    ax.0 = (ax.0.0 + dx, ax.0.1 + dy);
                    ax.1 = (ax.1.0 + dx, ax.1.1 + dy);
                }
                self.drag = Some(Drag::AxisMove(i, p));
            }
        }
    }

    pub fn release(&mut self) {
        self.drag = None;
        self.slide = None;
    }

    /// Desliza o objeto de Aproximar/Afastar. Verdadeiro se mudou.
    pub fn update(&mut self, dt: f32, objs: &mut Objects) -> bool {
        let (Some(s), Some(vp)) = (self.slide.as_mut(), self.guides.vp()) else {
            return false;
        };
        let Some(o) = objs.get_mut(s.id) else {
            self.slide = None;
            return false;
        };
        if !dt.is_finite() || dt <= 0.0 {
            return false;
        }
        let (vx, vy) = (vp.0 as f32, vp.1 as f32);
        let (dx, dy) = (vx - s.center.0, vy - s.center.1);
        let d = (dx * dx + dy * dy).sqrt();
        let step = SLIDE_SPEED * dt;
        if self.op == Op::Approach {
            if d <= 1.0 {
                return false;
            }
            let t = (step / d).min(1.0);
            s.center = (s.center.0 + dx * t, s.center.1 + dy * t);
        } else {
            // afastar: para longe do PF na mesma direção (ou para baixo, se em cima dele)
            let (ux, uy) = if d < 0.5 {
                (0.0, 1.0)
            } else {
                (-dx / d, -dy / d)
            };
            let nx = s.center.0 + ux * step;
            let ny = s.center.1 + uy * step;
            if nx < 0.0 || ny < 0.0 || nx >= CANVAS_W as f32 || ny >= CANVAS_H as f32 {
                return false;
            }
            s.center = (nx, ny);
        }
        let nd = ((vx - s.center.0).powi(2) + (vy - s.center.1).powi(2)).sqrt();
        let scale = nd / s.dist0;
        let w = ((s.size0.0 as f32 * scale).round() as i32).max(MIN_SIZE);
        let h = ((s.size0.1 as f32 * scale).round() as i32).max(MIN_SIZE);
        let (cx, cy) = (s.center.0.round() as i32, s.center.1.round() as i32);
        let before = o.rect();
        o.w = w;
        o.h = h;
        o.x = cx - w / 2;
        o.y = cy - h / 2;
        o.rect() != before
    }

    /// Faixa de opções: dois grupos com tampas verdes, botões e o interruptor.
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        f.blit(&a.image(&pic(CAP_LEFT)), 4, 57);
        let sep = (a.image(&pic(CAP_RIGHT.0)), a.image(&pic(CAP_RIGHT.1)));
        f.blit_masked(&sep.0, &sep.1, 25 + 37 * 3 - 2, 56);
        f.blit(&a.image(&pic(CAP_LEFT)), 25 + 37 * 5 - 21, 57);
        f.blit_masked(&sep.0, &sep.1, 25 + 37 * 11 - 2, 56);
        for btn in &BUTTONS {
            let r = option_rect(btn.k);
            let id = if btn.op == self.op {
                btn.color
            } else {
                btn.mono
            };
            f.blit(&a.image(&pic(id)), r.0, r.1);
        }
        if let Some(k) = self.selected() {
            let r = option_rect(k);
            f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, RED);
        }
        let switch = if self.guides.visible { 0x1491 } else { 0x1497 };
        f.blit(&a.image(&pic(switch)), SWITCH.0, SWITCH.1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Source;
    use crate::paint::stamp::Stamp;
    use std::rc::Rc;

    fn stamp() -> Rc<Stamp> {
        Rc::new(Stamp {
            w: 20,
            h: 10,
            px: vec![Some(3); 200],
        })
    }

    fn pick(t: &mut Supertools, op: Op) {
        let b = BUTTONS.iter().find(|b| b.op == op).unwrap();
        let r = option_rect(b.k);
        assert!(t.click_strip(r.0 + 3, r.1 + 3));
        assert_eq!(t.op(), op);
    }

    fn env() -> (Objects, Canvas, History, bool) {
        (
            Objects::default(),
            Canvas::new(CANVAS_W, CANVAS_H),
            History::default(),
            false,
        )
    }

    #[test]
    fn grids_and_switch() {
        let mut t = Supertools::new();
        pick(&mut t, Op::BigGrid);
        assert_eq!(t.guides.grid, Grid::Big);
        pick(&mut t, Op::SmallGrid);
        assert_eq!(t.guides.grid, Grid::Small);
        pick(&mut t, Op::NoGrid);
        assert_eq!(t.guides.grid, Grid::None);
        assert!(t.click_strip(470, 70));
        assert!(!t.guides.visible);
        assert!(!t.click_strip(300, 300));
        assert_eq!(t.help_record(), 1);
    }

    #[test]
    fn vanishing_point_lines_axes_eraser() {
        let mut t = Supertools::new();
        let (mut o, c, mut h, mut ch) = env();
        pick(&mut t, Op::VpLine);
        assert_eq!(
            t.press(10, 10, &mut o, &c, &mut h, &mut ch),
            Some(Notice::NoVanishingPoint)
        );
        pick(&mut t, Op::VanishingPoint);
        assert_eq!(t.press(300, 150, &mut o, &c, &mut h, &mut ch), None);
        assert_eq!(t.guides.horizon, Some((150, 300)));
        // arrastar o PF move horizonte e PF
        t.drag(310, 160);
        t.release();
        assert_eq!(t.guides.horizon, Some((160, 310)));
        pick(&mut t, Op::VpLine);
        t.press(50, 300, &mut o, &c, &mut h, &mut ch);
        t.release();
        t.press(52, 298, &mut o, &c, &mut h, &mut ch);
        t.drag(60, 320);
        t.release();
        assert_eq!(t.guides.vp_lines, vec![(60, 320)]);
        pick(&mut t, Op::Axis);
        t.press(100, 100, &mut o, &c, &mut h, &mut ch);
        t.drag(100, 100);
        assert!(t.guides.axes.is_empty());
        t.drag(200, 120);
        t.release();
        assert_eq!(t.guides.axes, vec![((100, 100), (200, 120))]);
        // meio move; ponta muda
        t.press(150, 110, &mut o, &c, &mut h, &mut ch);
        t.drag(160, 115);
        t.release();
        assert_eq!(t.guides.axes, vec![((110, 105), (210, 125))]);
        t.press(110, 105, &mut o, &c, &mut h, &mut ch);
        t.drag(90, 90);
        t.release();
        assert_eq!(t.guides.axes, vec![((90, 90), (210, 125))]);
        pick(&mut t, Op::Eraser);
        t.press(150, 108, &mut o, &c, &mut h, &mut ch);
        assert!(t.guides.axes.is_empty());
        t.press(400, 162, &mut o, &c, &mut h, &mut ch);
        assert_eq!(t.guides.horizon, None);
        assert!(t.guides.vp_lines.is_empty());
        assert!(t.guides.is_empty());
    }

    #[test]
    fn approach_shrinks_and_moves_toward_vp_recede_grows() {
        let mut t = Supertools::new();
        let (mut o, c, mut h, mut ch) = env();
        let id = o.place(stamp(), 100, 200, Source::Roll { roll: 1, item: 0 });
        pick(&mut t, Op::Approach);
        assert_eq!(
            t.press(105, 205, &mut o, &c, &mut h, &mut ch),
            Some(Notice::NoVanishingPoint)
        );
        t.guides.horizon = Some((100, 400));
        assert_eq!(t.press(105, 205, &mut o, &c, &mut h, &mut ch), None);
        assert!(ch);
        let d0 = dist(o.get(id).unwrap().center(), (400, 100));
        for _ in 0..60 {
            t.update(1.0 / 60.0, &mut o);
        }
        t.release();
        let obj = o.get(id).unwrap();
        let d1 = dist(obj.center(), (400, 100));
        assert!(d1 < d0 - 50.0 && d1 > d0 - 70.0, "{d0} {d1}");
        assert!(obj.w < 20 && obj.h < 10);
        // afastar devolve o tamanho conforme a distância
        pick(&mut t, Op::Recede);
        let (cx, cy) = obj.center();
        t.press(cx, cy, &mut o, &c, &mut h, &mut ch);
        for _ in 0..60 {
            t.update(1.0 / 60.0, &mut o);
        }
        t.release();
        let obj = o.get(id).unwrap();
        assert!(dist(obj.center(), (400, 100)) > d1 + 50.0);
        assert!(obj.w > 10);
        assert!(!t.update(0.1, &mut o));
    }

    #[test]
    fn guides_json_roundtrip_and_file() {
        let g = Guides {
            grid: Grid::Small,
            horizon: Some((120, -5)),
            vp_lines: vec![(1, 2), (30, 40)],
            axes: vec![((1, 2), (3, 4))],
            visible: false,
        };
        assert_eq!(Guides::parse_json(&g.to_json()), Some(g.clone()));
        assert_eq!(Guides::parse_json("lixo"), None);
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(GUIDES_NAME);
        g.save(&p).unwrap();
        assert_eq!(Guides::load(&p), Some(g));
        Guides::new().save(&p).unwrap();
        assert!(!p.exists());
        assert_eq!(Guides::load(&p), None);
    }

    #[test]
    fn draw_strip_and_guides_with_empty_assets() {
        let (_d, a) = crate::scenes::testutil::empty_assets();
        let mut t = Supertools::new();
        t.guides.grid = Grid::Big;
        t.guides.horizon = Some((100, 200));
        t.guides.vp_lines = vec![(0, 0)];
        t.guides.axes = vec![((10, 10), (50, 50))];
        let mut f = Frame::new(640, 480);
        t.draw_strip(&mut f, &a);
        t.guides.draw(&mut f);
        assert_eq!(
            f.pixel((ORIGIN.0 + 48) as u32, ORIGIN.1 as u32)[..3],
            GRID_GREEN
        );
        assert_eq!(
            f.pixel((ORIGIN.0 + 10) as u32, (ORIGIN.1 + 100) as u32)[..3],
            BLUE
        );
        t.guides.visible = false;
        let mut g = Frame::new(640, 480);
        t.guides.draw(&mut g);
        assert_eq!(
            g.pixel((ORIGIN.0 + 48) as u32, ORIGIN.1 as u32)[..3],
            [0, 0, 0]
        );
    }
}
