//! Palavras com forma: o que o Cavalete de palavras com forma do original
//! ("Spletter easel", `WHAT` 1009 [1]) produz — texto num tipo de letra
//! instalado, entortado por uma forma (arco, onda, triângulo, círculo…),
//! com estilos (negrito, inclinado, contorno, maiúsculas, vertical), cores
//! (uma ou alternadas por letra), padrão, uma de oito sombras, tamanho e giro.
//!
//! A imagem é montada em três passos:
//! 1. **máscara** de cobertura (0–255) do texto numa linha (ou empilhado, no
//!    vertical), com o número da letra de cada pixel;
//! 2. **forma**: a máscara é esticada entre uma curva de cima e uma de baixo
//!    (formas de envelope) ou enrolada em volta de um centro (arco, círculo);
//! 3. **cópias**: as sombras são cópias da máscara transformadas (deslocadas,
//!    deitadas, giradas, encolhidas) desenhadas por baixo da cópia principal;
//!    o giro do cavalete entra na transformação de todas. Cada pixel do
//!    resultado amostra a máscara de forma bilinear e vira tinta quando a
//!    cobertura passa da metade (letras sem meio-tom, como no original).

use super::stamp::Stamp;
use super::text::text_pixels;
use super::ttf::{self, FontFace};
use ab_glyph::{Font as _, FontVec, PxScale, ScaleFont as _, point};
use formats::fnt::Font;
use std::f32::consts::PI;

/// Forma das palavras, na ordem da lista do botão FORMA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WordShape {
    /// Retângulo: sem deformação.
    #[default]
    Plain,
    /// Em cima reto, embaixo um arco que sobe no meio.
    Arch,
    Wave,
    /// Mais alto no meio (hexágono).
    Inflate,
    /// Cresce para a direita.
    GrowRight,
    /// Encolhe para a direita.
    ShrinkRight,
    Zigzag,
    /// Ponta para cima: as letras do meio são as mais altas.
    TriangleUp,
    /// Ponta para baixo.
    TriangleDown,
    /// Letras sobre um arco (meia-lua para cima).
    ArcUp,
    /// Letras em volta de um círculo.
    Circle,
}

impl WordShape {
    pub const ALL: [WordShape; 11] = [
        WordShape::Plain,
        WordShape::Arch,
        WordShape::Wave,
        WordShape::Inflate,
        WordShape::GrowRight,
        WordShape::ShrinkRight,
        WordShape::Zigzag,
        WordShape::TriangleUp,
        WordShape::TriangleDown,
        WordShape::ArcUp,
        WordShape::Circle,
    ];

    /// Nome gravado no `*.objetos.json`.
    pub fn name(self) -> &'static str {
        match self {
            WordShape::Plain => "retangulo",
            WordShape::Arch => "arco-embaixo",
            WordShape::Wave => "onda",
            WordShape::Inflate => "inflado",
            WordShape::GrowRight => "cresce",
            WordShape::ShrinkRight => "encolhe",
            WordShape::Zigzag => "zigue-zague",
            WordShape::TriangleUp => "triangulo",
            WordShape::TriangleDown => "triangulo-baixo",
            WordShape::ArcUp => "arco",
            WordShape::Circle => "circulo",
        }
    }

    pub fn from_name(s: &str) -> Option<WordShape> {
        WordShape::ALL.into_iter().find(|w| w.name() == s)
    }

    /// Altura da forma em relação à da linha de texto (formas de envelope).
    fn stretch(self) -> f32 {
        match self {
            WordShape::Plain => 1.0,
            WordShape::Wave => 1.5,
            WordShape::TriangleUp | WordShape::TriangleDown => 1.8,
            _ => 1.6,
        }
    }

    /// Curvas de cima e de baixo em `u` ∈ [0, 1] (da esquerda para a
    /// direita), como frações da altura da forma.
    pub fn envelope(self, u: f32) -> (f32, f32) {
        let mid = (2.0 * u - 1.0).abs();
        match self {
            WordShape::Plain | WordShape::ArcUp | WordShape::Circle => (0.0, 1.0),
            WordShape::Arch => (0.0, 1.0 - 0.4 * (PI * u).sin()),
            WordShape::Wave => {
                let t = 0.165 * (1.0 - (2.0 * PI * u).sin());
                (t, t + 0.67)
            }
            WordShape::Inflate => (0.3 * mid, 1.0 - 0.3 * mid),
            WordShape::GrowRight => (0.3 * (1.0 - u), 1.0 - 0.3 * (1.0 - u)),
            WordShape::ShrinkRight => (0.3 * u, 1.0 - 0.3 * u),
            WordShape::Zigzag => {
                let z = ((u * 2.5).fract() * 2.0 - 1.0).abs();
                (0.375 * z, 0.375 * z + 0.625)
            }
            WordShape::TriangleUp => (0.55 * mid, 1.0),
            WordShape::TriangleDown => (0.0, 1.0 - 0.55 * mid),
        }
    }
}

/// Sombra, na ordem do menu do botão de sombras.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shadow {
    #[default]
    None,
    Normal,
    /// Sombra deitada no chão, para a esquerda (sol nascendo).
    Dawn,
    /// Sombra deitada no chão, para a direita.
    Dusk,
    /// Cópias giradas cada vez mais claras.
    SpinFade,
    /// Cópias em contorno giradas em volta de um centro, como uma roda.
    SpinCircle,
    /// Cópias menores indo para um ponto de fuga em cima.
    Converge,
    /// Cópias empilhadas na diagonal: as letras ganham espessura.
    Extrude,
}

impl Shadow {
    pub const ALL: [Shadow; 8] = [
        Shadow::None,
        Shadow::Normal,
        Shadow::Dawn,
        Shadow::Dusk,
        Shadow::SpinFade,
        Shadow::SpinCircle,
        Shadow::Converge,
        Shadow::Extrude,
    ];

    /// Texto do menu (o do original).
    pub fn label(self) -> &'static str {
        match self {
            Shadow::None => "Sem sombra",
            Shadow::Normal => "Sombra normal",
            Shadow::Dawn => "Sombra do amanhecer",
            Shadow::Dusk => "Sombra do anoitecer",
            Shadow::SpinFade => "Girar e sumir",
            Shadow::SpinCircle => "Girar em círculo",
            Shadow::Converge => "Sombra convergente",
            Shadow::Extrude => "Efeito tridimensional",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Shadow::None => "nenhuma",
            Shadow::Normal => "normal",
            Shadow::Dawn => "amanhecer",
            Shadow::Dusk => "anoitecer",
            Shadow::SpinFade => "girar-sumir",
            Shadow::SpinCircle => "girar-circulo",
            Shadow::Converge => "convergente",
            Shadow::Extrude => "tridimensional",
        }
    }

    pub fn from_name(s: &str) -> Option<Shadow> {
        Shadow::ALL.into_iter().find(|w| w.name() == s)
    }
}

/// Tamanhos dos botões + e −, em pixels de altura da linha.
pub const SIZES: [u32; 10] = [16, 24, 32, 48, 64, 80, 96, 128, 160, 200];
/// Tamanho inicial (o "Ana" do original ocupa uns 300×130 no quadro).
pub const DEFAULT_SIZE: u32 = 128;
/// Texto mais longo aceito no campo.
pub const MAX_CHARS: usize = 60;

/// Uma palavra com forma: tudo o que o cavalete escolhe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapedWords {
    pub text: String,
    /// Família instalada; vazio = "Pablo Graphos", a fonte do programa.
    pub font: String,
    pub shape: WordShape,
    pub bold: bool,
    pub italic: bool,
    pub outline: bool,
    pub caps: bool,
    pub vertical: bool,
    /// Índices da paleta; mais de um = as letras alternam as cores.
    pub colors: Vec<u8>,
    /// Padrão da pintura (índice na coluna de padrões; 0 = cheio).
    pub pattern: u8,
    pub shadow: Shadow,
    /// Altura da linha, em pixels.
    pub size: u32,
    /// Giro em graus, no sentido anti-horário (0..360).
    pub angle: i32,
}

impl ShapedWords {
    /// Palavra com a fonte do programa e os estilos do começo; a família
    /// inicial do cavalete vem de [`default_font`].
    pub fn new(text: &str, color: u8) -> ShapedWords {
        ShapedWords {
            text: text.to_string(),
            font: String::new(),
            shape: WordShape::Plain,
            bold: false,
            italic: false,
            outline: false,
            caps: false,
            vertical: false,
            colors: vec![color],
            pattern: 0,
            shadow: Shadow::None,
            size: DEFAULT_SIZE,
            angle: 0,
        }
    }

    /// Texto que vai para a imagem (em maiúsculas, se pedido).
    pub fn shown_text(&self) -> String {
        if self.caps {
            self.text.to_uppercase()
        } else {
            self.text.clone()
        }
    }

    /// Estilos ligados, como gravados (`negrito,inclinado,...`).
    pub fn styles(&self) -> String {
        let mut v = Vec::new();
        for (on, name) in [
            (self.bold, "negrito"),
            (self.italic, "inclinado"),
            (self.outline, "contorno"),
            (self.caps, "maiusculas"),
            (self.vertical, "vertical"),
        ] {
            if on {
                v.push(name);
            }
        }
        v.join(",")
    }

    pub fn set_styles(&mut self, s: &str) {
        let has = |n: &str| s.split(',').any(|p| p.trim() == n);
        self.bold = has("negrito");
        self.italic = has("inclinado");
        self.outline = has("contorno");
        self.caps = has("maiusculas");
        self.vertical = has("vertical");
    }

    /// Um passo de tamanho para cima (`+`) ou para baixo (`−`); falso se já está no fim.
    pub fn step_size(&mut self, up: bool) -> bool {
        let k = SIZES
            .iter()
            .position(|&s| s >= self.size)
            .unwrap_or(SIZES.len() - 1);
        let next = if up {
            if SIZES[k] > self.size { k } else { k + 1 }
        } else {
            k.checked_sub(1).unwrap_or(usize::MAX)
        };
        match SIZES.get(next) {
            Some(&s) if s != self.size => {
                self.size = s;
                true
            }
            _ => false,
        }
    }

    /// Gira `deg` graus (positivo = anti-horário).
    pub fn rotate(&mut self, deg: i32) {
        self.angle = (self.angle + deg).rem_euclid(360);
    }
}

/// Tintas que a imagem usa além das cores escolhidas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inks {
    pub white: u8,
    pub black: u8,
    /// Sombras: cinza médio, claro e escuro.
    pub gray: u8,
    pub light: u8,
    pub dark: u8,
    /// Padrão 8×8 (bit 1 = branco), relativo ao canto da imagem.
    pub pattern: [u8; 8],
}

/// Fonte para desenhar: uma família instalada ou a de bitmap do programa.
#[derive(Clone, Copy)]
pub enum Face<'a> {
    Outline(&'a FontVec),
    Bitmap(Option<&'a Font>),
}

/// Família inicial do cavalete (uma sem serifa instalada; vazio = a do programa).
pub fn default_font() -> String {
    ttf::default_face()
        .map(|f| f.name.clone())
        .unwrap_or_default()
}

/// Família `name` carregada, se instalada.
pub fn load_face(name: &str) -> Option<std::rc::Rc<FontVec>> {
    if name.is_empty() {
        return None;
    }
    ttf::find(name).and_then(ttf::load)
}

/// Famílias da lista do botão ABC: a do programa primeiro, depois as instaladas.
pub fn font_names() -> Vec<String> {
    let mut v = vec![String::new()];
    v.extend(ttf::installed().iter().map(|f: &FontFace| f.name.clone()));
    v
}

/// Nome mostrado de uma família (vazio = a do programa).
pub fn font_label(name: &str) -> &str {
    if name.is_empty() {
        "Pablo Graphos"
    } else {
        name
    }
}

/// Máscara de cobertura com o número da letra de cada pixel.
#[derive(Debug, Clone)]
struct Mask {
    w: i32,
    h: i32,
    a: Vec<u8>,
    letter: Vec<u16>,
}

impl Mask {
    fn new(w: i32, h: i32) -> Mask {
        let (w, h) = (w.max(1), h.max(1));
        Mask {
            w,
            h,
            a: vec![0; (w * h) as usize],
            letter: vec![0; (w * h) as usize],
        }
    }

    fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            0
        } else {
            self.a[(y * self.w + x) as usize]
        }
    }

    fn put(&mut self, x: i32, y: i32, v: u8, letter: u16) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = (y * self.w + x) as usize;
        if v > self.a[i] {
            self.a[i] = v;
            self.letter[i] = letter;
        }
    }

    /// Cobertura bilinear em `(fx, fy)` (centros dos pixels em `+0,5`) e a
    /// letra do pixel mais próximo.
    fn sample(&self, fx: f32, fy: f32) -> (f32, u16) {
        let (x, y) = (fx - 0.5, fy - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (tx, ty) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i32, y0 as i32);
        let v = |dx: i32, dy: i32| self.get(x0 + dx, y0 + dy) as f32;
        let a = v(0, 0) * (1.0 - tx) * (1.0 - ty)
            + v(1, 0) * tx * (1.0 - ty)
            + v(0, 1) * (1.0 - tx) * ty
            + v(1, 1) * tx * ty;
        let (nx, ny) = (fx.floor() as i32, fy.floor() as i32);
        let letter = if nx >= 0 && ny >= 0 && nx < self.w && ny < self.h {
            self.letter[(ny * self.w + nx) as usize]
        } else {
            0
        };
        (a, letter)
    }
}

/// Uma linha de texto (ou um caractere) numa máscara, com a letra `first` em diante.
fn line_mask(face: Face, text: &str, size: u32, first: u16) -> Mask {
    let size = size.max(4) as f32;
    match face {
        Face::Outline(font) => {
            let scaled = font.as_scaled(PxScale::from(size));
            let ascent = scaled.ascent();
            let h = (ascent - scaled.descent()).ceil() as i32 + 2;
            let mut caret = 1.0f32;
            let mut glyphs = Vec::new();
            let mut prev = None;
            for c in text.chars() {
                let id = font.glyph_id(c);
                if let Some(p) = prev {
                    caret += scaled.kern(p, id);
                }
                glyphs.push(
                    id.with_scale_and_position(PxScale::from(size), point(caret, ascent + 1.0)),
                );
                caret += scaled.h_advance(id);
                prev = Some(id);
            }
            let mut m = Mask::new(caret.ceil() as i32 + 2, h);
            for (k, g) in glyphs.into_iter().enumerate() {
                if let Some(o) = font.outline_glyph(g) {
                    let b = o.px_bounds();
                    let letter = first + k as u16;
                    o.draw(|x, y, c| {
                        let v = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
                        m.put(
                            b.min.x as i32 + x as i32,
                            b.min.y as i32 + y as i32,
                            v,
                            letter,
                        );
                    });
                }
            }
            m
        }
        Face::Bitmap(font) => {
            // glifos de bitmap na resolução própria, ampliados de forma bilinear
            let (_, nh) = super::text::text_size(font, "");
            let nh = nh.max(1) as i32;
            let mut x = 0i32;
            let mut native = Vec::new();
            for (k, c) in text.chars().enumerate() {
                let s = c.to_string();
                text_pixels(font, &s, |dx, dy| native.push((x + dx, dy, k as u16)));
                x += super::text::text_size(font, &s).0 as i32;
            }
            let mut src = Mask::new(x.max(1) + 2, nh + 2);
            for (px, py, l) in native {
                src.put(px + 1, py + 1, 255, first + l);
            }
            let scale = size / nh as f32;
            let mut m = Mask::new(
                (src.w as f32 * scale).ceil() as i32,
                (src.h as f32 * scale).ceil() as i32,
            );
            for y in 0..m.h {
                for x in 0..m.w {
                    let (a, l) = src.sample((x as f32 + 0.5) / scale, (y as f32 + 0.5) / scale);
                    m.put(x, y, a.round() as u8, l);
                }
            }
            m
        }
    }
}

/// Texto todo: numa linha, ou uma letra embaixo da outra (centradas).
fn text_mask(face: Face, w: &ShapedWords) -> Mask {
    let text = w.shown_text();
    let text = if text.is_empty() {
        " ".to_string()
    } else {
        text
    };
    if !w.vertical {
        return line_mask(face, &text, w.size, 0);
    }
    let parts: Vec<Mask> = text
        .chars()
        .enumerate()
        .map(|(k, c)| line_mask(face, &c.to_string(), w.size, k as u16))
        .collect();
    // letras mais juntas que a altura da linha: 80%
    let step = parts.iter().map(|m| m.h).max().unwrap_or(1) * 4 / 5;
    let width = parts.iter().map(|m| m.w).max().unwrap_or(1);
    let mut out = Mask::new(
        width,
        step * (parts.len() as i32 - 1) + parts.last().map_or(1, |m| m.h),
    );
    for (k, p) in parts.iter().enumerate() {
        let (ox, oy) = ((width - p.w) / 2, k as i32 * step);
        for y in 0..p.h {
            for x in 0..p.w {
                let i = (y * p.w + x) as usize;
                out.put(ox + x, oy + y, p.a[i], p.letter[i]);
            }
        }
    }
    out
}

/// Inclina as letras para a direita (cisalhamento de 1/5).
fn shear(m: &Mask) -> Mask {
    let extra = (m.h as f32 * 0.2).ceil() as i32;
    let mut out = Mask::new(m.w + extra, m.h);
    for y in 0..m.h {
        let dx = ((m.h - y) as f32 * 0.2).round() as i32;
        for x in 0..m.w {
            let i = (y * m.w + x) as usize;
            out.put(x + dx, y, m.a[i], m.letter[i]);
        }
    }
    out
}

/// Engrossa as letras `r` pixels (máximo num quadrado).
fn dilate(m: &Mask, r: i32) -> Mask {
    let mut out = Mask::new(m.w + 2 * r, m.h + 2 * r);
    for y in 0..m.h {
        for x in 0..m.w {
            let i = (y * m.w + x) as usize;
            if m.a[i] == 0 {
                continue;
            }
            for dy in -r..=r {
                for dx in -r..=r {
                    out.put(x + r + dx, y + r + dy, m.a[i], m.letter[i]);
                }
            }
        }
    }
    out
}

/// Entorta a máscara pela forma.
fn warp(m: &Mask, shape: WordShape) -> Mask {
    match shape {
        WordShape::Plain => m.clone(),
        WordShape::ArcUp | WordShape::Circle => polar(m, shape == WordShape::Circle),
        _ => {
            let (w, h) = (m.w as f32, m.h as f32);
            let oh = (h * shape.stretch()).ceil() as i32;
            let mut out = Mask::new(m.w, oh);
            for x in 0..m.w {
                let u = (x as f32 + 0.5) / w;
                let (t, b) = shape.envelope(u);
                let (t, b) = (t * oh as f32, b * oh as f32);
                if b - t < 1.0 {
                    continue;
                }
                for y in t.floor() as i32..b.ceil() as i32 {
                    let v = (y as f32 + 0.5 - t) / (b - t);
                    if !(0.0..=1.0).contains(&v) {
                        continue;
                    }
                    let (a, l) = m.sample(x as f32 + 0.5, v * h);
                    out.put(x, y, a.round() as u8, l);
                }
            }
            out
        }
    }
}

/// Enrola a linha num arco (metade de cima) ou num círculo inteiro, com o
/// pé das letras para o centro.
fn polar(m: &Mask, full: bool) -> Mask {
    let (w, h) = (m.w as f32, m.h as f32);
    let turn = if full { 2.0 * PI } else { PI };
    let r_mid = (w / turn).max(if full { h * 0.8 } else { h });
    let span = (w / r_mid).min(turn);
    let r_out = r_mid + h / 2.0;
    let side = (2.0 * r_out).ceil() as i32 + 2;
    let oh = if full { side } else { r_out.ceil() as i32 + 2 };
    let (cx, cy) = (
        side as f32 / 2.0,
        if full { side as f32 / 2.0 } else { r_out + 1.0 },
    );
    let mut out = Mask::new(side, oh);
    for y in 0..oh {
        for x in 0..side {
            let dx = x as f32 + 0.5 - cx;
            let dy = cy - (y as f32 + 0.5);
            let r = (dx * dx + dy * dy).sqrt();
            let v = (r_out - r) / h;
            if !(0.0..=1.0).contains(&v) {
                continue;
            }
            // ângulo no sentido horário a partir do alto; o texto fica centrado em cima
            let phi = dx.atan2(dy);
            let u = (phi + span / 2.0) / span;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let (a, l) = m.sample(u * w, v * h);
            out.put(x, y, a.round() as u8, l);
        }
    }
    out
}

/// Como uma cópia da máscara é pintada.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Paint {
    /// As letras, nas cores escolhidas (com padrão ou contorno).
    Main,
    Solid(u8),
    /// Só a borda, preta, com o miolo branco.
    Outline(u8),
    /// Cor com pontos faltando (nível 0 = cheio, 4 = um em cinco).
    Dither(u8, u8),
}

/// Cópia da máscara: destino = `a`·origem + `t`.
#[derive(Debug, Clone, Copy)]
struct Layer {
    a: [f32; 4],
    t: (f32, f32),
    paint: Paint,
}

impl Layer {
    fn new(a: [f32; 4], t: (f32, f32), paint: Paint) -> Layer {
        Layer { a, t, paint }
    }

    fn apply(&self, p: (f32, f32)) -> (f32, f32) {
        (
            self.a[0] * p.0 + self.a[1] * p.1 + self.t.0,
            self.a[2] * p.0 + self.a[3] * p.1 + self.t.1,
        )
    }

    /// `other` depois desta.
    fn then(&self, other: &Layer) -> Layer {
        let [a, b, c, d] = self.a;
        let [e, f, g, h] = other.a;
        Layer {
            a: [e * a + f * c, e * b + f * d, g * a + h * c, g * b + h * d],
            t: other.apply(self.t),
            paint: self.paint,
        }
    }

    fn inverse(&self) -> Option<([f32; 4], (f32, f32))> {
        let [a, b, c, d] = self.a;
        let det = a * d - b * c;
        if det.abs() < 1e-6 {
            return None;
        }
        let inv = [d / det, -b / det, -c / det, a / det];
        let t = (
            -(inv[0] * self.t.0 + inv[1] * self.t.1),
            -(inv[2] * self.t.0 + inv[3] * self.t.1),
        );
        Some((inv, t))
    }
}

fn rot(deg: f32) -> [f32; 4] {
    let r = deg.to_radians();
    // anti-horário na tela (y para baixo)
    [r.cos(), r.sin(), -r.sin(), r.cos()]
}

/// Giro de `deg` em volta de `p`.
fn rot_about(deg: f32, p: (f32, f32), paint: Paint) -> Layer {
    let a = rot(deg);
    Layer::new(
        a,
        (
            p.0 - (a[0] * p.0 + a[1] * p.1),
            p.1 - (a[2] * p.0 + a[3] * p.1),
        ),
        paint,
    )
}

/// Cópias da sombra (de trás para a frente) e a principal, por último.
fn copies(w: &ShapedWords, mw: f32, mh: f32, inks: &Inks) -> Vec<Layer> {
    const I: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
    let d = (mh / 12.0).round().max(2.0);
    let mut v = Vec::new();
    match w.shadow {
        Shadow::None => {}
        Shadow::Normal => v.push(Layer::new(I, (d, d), Paint::Solid(inks.gray))),
        Shadow::Dawn => v.push(Layer::new(
            [1.0, 0.9, 0.0, 0.4],
            (-0.9 * mh, 0.6 * mh),
            Paint::Solid(inks.light),
        )),
        Shadow::Dusk => v.push(Layer::new(
            [1.0, -0.9, 0.0, 0.4],
            (0.9 * mh, 0.6 * mh),
            Paint::Solid(inks.dark),
        )),
        Shadow::SpinFade => {
            for k in (1..=4).rev() {
                v.push(rot_about(
                    -12.0 * k as f32,
                    (mw / 2.0, mh / 2.0),
                    Paint::Dither(inks.gray, k as u8),
                ));
            }
        }
        Shadow::SpinCircle => {
            let p = (-0.4 * mh, mh / 2.0);
            for k in 1..12 {
                v.push(rot_about(30.0 * k as f32, p, Paint::Outline(inks.black)));
            }
        }
        Shadow::Converge => {
            let vp = (mw / 2.0, -1.5 * mh);
            for k in (1..=4).rev() {
                let s = 1.0 - 0.18 * k as f32;
                v.push(Layer::new(
                    [s, 0.0, 0.0, s],
                    ((1.0 - s) * vp.0, (1.0 - s) * vp.1),
                    Paint::Dither(inks.gray, (k - 1) as u8),
                ));
            }
        }
        Shadow::Extrude => {
            let depth = (mh / 10.0).round().max(3.0) as i32;
            for i in (1..=depth).rev() {
                v.push(Layer::new(I, (i as f32, i as f32), Paint::Solid(inks.gray)));
            }
        }
    }
    v.push(Layer::new(I, (0.0, 0.0), Paint::Main));
    // o giro do cavalete, em volta do centro da forma, vale para todas
    if w.angle != 0 {
        let r = rot_about(w.angle as f32, (mw / 2.0, mh / 2.0), Paint::Main);
        v = v.iter().map(|c| c.then(&r)).collect();
    }
    v
}

/// Pixels cobertos por uma cópia, numa grade `w`×`h` com origem em `org`.
fn coverage(m: &Mask, c: &Layer, org: (i32, i32), w: i32, h: i32) -> Vec<Option<u16>> {
    let mut out = vec![None; (w * h) as usize];
    let Some((inv, t)) = c.inverse() else {
        return out;
    };
    let (x0, y0, x1, y1) = bbox(m, c);
    for y in (y0 - org.1).max(0)..(y1 - org.1).min(h) {
        for x in (x0 - org.0).max(0)..(x1 - org.0).min(w) {
            let (dx, dy) = ((x + org.0) as f32 + 0.5, (y + org.1) as f32 + 0.5);
            let sx = inv[0] * dx + inv[1] * dy + t.0;
            let sy = inv[2] * dx + inv[3] * dy + t.1;
            let (a, l) = m.sample(sx, sy);
            if a >= 127.5 {
                out[(y * w + x) as usize] = Some(l);
            }
        }
    }
    out
}

/// Caixa (x0, y0, x1, y1 exclusivos) da máscara transformada.
fn bbox(m: &Mask, c: &Layer) -> (i32, i32, i32, i32) {
    let pts = [
        (0.0, 0.0),
        (m.w as f32, 0.0),
        (0.0, m.h as f32),
        (m.w as f32, m.h as f32),
    ]
    .map(|p| c.apply(p));
    let x0 = pts.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor() as i32 - 1;
    let y0 = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor() as i32 - 1;
    let x1 = pts.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil() as i32 + 1;
    let y1 = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as i32 + 1;
    (x0, y0, x1, y1)
}

/// Pixel `(x, y)` está a até `r` de fora da cobertura?
fn near_edge(cov: &[Option<u16>], w: i32, h: i32, x: i32, y: i32, r: i32) -> bool {
    for dy in -r..=r {
        for dx in -r..=r {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h || cov[(ny * w + nx) as usize].is_none() {
                return true;
            }
        }
    }
    false
}

/// A imagem da palavra com forma, pronta para virar objeto.
pub fn render(w: &ShapedWords, face: Face, inks: &Inks) -> Stamp {
    let mut m = text_mask(face, w);
    if w.italic {
        m = shear(&m);
    }
    if w.bold {
        m = dilate(&m, (w.size as i32 / 24).max(1));
    }
    let m = warp(&m, w.shape);
    let cs = copies(w, m.w as f32, m.h as f32, inks);
    let boxes: Vec<_> = cs.iter().map(|c| bbox(&m, c)).collect();
    let x0 = boxes.iter().map(|b| b.0).min().unwrap_or(0);
    let y0 = boxes.iter().map(|b| b.1).min().unwrap_or(0);
    let x1 = boxes.iter().map(|b| b.2).max().unwrap_or(1);
    let y1 = boxes.iter().map(|b| b.3).max().unwrap_or(1);
    let (gw, gh) = ((x1 - x0).max(1), (y1 - y0).max(1));
    let mut px: Vec<Option<u8>> = vec![None; (gw * gh) as usize];
    let colors = if w.colors.is_empty() {
        vec![inks.black]
    } else {
        w.colors.clone()
    };
    let edge_r = (w.size as i32 / 48).max(1);
    for c in &cs {
        let cov = coverage(&m, c, (x0, y0), gw, gh);
        for y in 0..gh {
            for x in 0..gw {
                let i = (y * gw + x) as usize;
                let Some(letter) = cov[i] else { continue };
                let v = match c.paint {
                    Paint::Solid(v) => Some(v),
                    Paint::Dither(v, level) => {
                        ((x * 3 + y * 5).rem_euclid(5) >= level as i32).then_some(v)
                    }
                    Paint::Outline(v) => Some(if near_edge(&cov, gw, gh, x, y, 1) {
                        v
                    } else {
                        inks.white
                    }),
                    Paint::Main => {
                        let color = colors[letter as usize % colors.len()];
                        Some(if w.outline {
                            if near_edge(&cov, gw, gh, x, y, edge_r) {
                                color
                            } else {
                                inks.white
                            }
                        } else if (inks.pattern[y.rem_euclid(8) as usize] >> (7 - x.rem_euclid(8)))
                            & 1
                            == 1
                        {
                            inks.white
                        } else {
                            color
                        })
                    }
                };
                if v.is_some() {
                    px[i] = v;
                }
            }
        }
    }
    crop(gw, gh, px)
}

/// Recorta a grade ao menor retângulo com pixels.
fn crop(w: i32, h: i32, px: Vec<Option<u8>>) -> Stamp {
    let on = |x: i32, y: i32| px[(y * w + x) as usize].is_some();
    let rows: Vec<i32> = (0..h).filter(|&y| (0..w).any(|x| on(x, y))).collect();
    let cols: Vec<i32> = (0..w).filter(|&x| (0..h).any(|y| on(x, y))).collect();
    let (Some(&ya), Some(&yb), Some(&xa), Some(&xb)) =
        (rows.first(), rows.last(), cols.first(), cols.last())
    else {
        return Stamp {
            w: 1,
            h: 1,
            px: vec![None],
        };
    };
    let (cw, ch) = (xb - xa + 1, yb - ya + 1);
    let mut out = Vec::with_capacity((cw * ch) as usize);
    for y in ya..=yb {
        for x in xa..=xb {
            out.push(px[(y * w + x) as usize]);
        }
    }
    Stamp {
        w: cw as u32,
        h: ch as u32,
        px: out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INKS: Inks = Inks {
        white: 0,
        black: 1,
        gray: 2,
        light: 3,
        dark: 4,
        pattern: [0; 8],
    };

    fn words(text: &str) -> ShapedWords {
        ShapedWords {
            font: String::new(),
            size: 32,
            ..ShapedWords::new(text, 1)
        }
    }

    fn count(s: &Stamp, v: u8) -> usize {
        s.px.iter().filter(|p| **p == Some(v)).count()
    }

    fn plain(w: &ShapedWords) -> Stamp {
        render(w, Face::Bitmap(None), &INKS)
    }

    #[test]
    fn plain_text_scales_bitmap_font_to_size() {
        let s = plain(&words("Ana"));
        // fonte 8×8 reserva ampliada 4× (32 px): três letras ≈ 96 px
        assert!((80..=110).contains(&s.w), "{}", s.w);
        assert!((16..=36).contains(&s.h), "{}", s.h);
        assert!(count(&s, 1) > 300);
        assert_eq!(count(&s, 0), 0);
        // tamanho maior, imagem maior
        let big = plain(&ShapedWords {
            size: 64,
            ..words("Ana")
        });
        assert!(big.w > s.w * 3 / 2 && big.h > s.h * 3 / 2);
    }

    #[test]
    fn shapes_change_the_image_and_stay_filled() {
        let base = plain(&words("Palavras"));
        for shape in WordShape::ALL {
            let s = plain(&ShapedWords {
                shape,
                ..words("Palavras")
            });
            assert!(count(&s, 1) > 200, "{shape:?}");
            if shape != WordShape::Plain {
                assert_ne!((s.w, s.h, &s.px), (base.w, base.h, &base.px), "{shape:?}");
            }
            assert_eq!(WordShape::from_name(shape.name()), Some(shape));
        }
        // envelopes: o triângulo é mais baixo nas pontas que no meio
        let (t0, _) = WordShape::TriangleUp.envelope(0.0);
        let (tm, _) = WordShape::TriangleUp.envelope(0.5);
        assert!(t0 > tm);
        // o círculo é (quase) quadrado; o arco, mais largo que alto
        let c = plain(&ShapedWords {
            shape: WordShape::Circle,
            ..words("Palavras longas aqui")
        });
        assert!(
            (c.w as i32 - c.h as i32).abs() < c.w as i32 / 4,
            "{}x{}",
            c.w,
            c.h
        );
        let a = plain(&ShapedWords {
            shape: WordShape::ArcUp,
            ..words("Palavras longas")
        });
        assert!(a.w > a.h, "{}x{}", a.w, a.h);
    }

    #[test]
    fn styles_bold_italic_vertical_caps_outline() {
        let n = count(&plain(&words("Ana")), 1);
        let bold = plain(&ShapedWords {
            bold: true,
            ..words("Ana")
        });
        assert!(count(&bold, 1) > n);
        // H tem as hastes inteiras: inclinado, fica mais largo
        let it = plain(&ShapedWords {
            italic: true,
            ..words("HH")
        });
        assert!(it.w > plain(&words("HH")).w);
        let v = plain(&ShapedWords {
            vertical: true,
            ..words("Ana")
        });
        assert!(v.h > v.w, "{}x{}", v.w, v.h);
        let caps = ShapedWords {
            caps: true,
            ..words("Ana")
        };
        assert_eq!(caps.shown_text(), "ANA");
        // contorno: borda na cor, miolo branco
        let o = plain(&ShapedWords {
            outline: true,
            bold: true,
            size: 64,
            ..words("I")
        });
        assert!(count(&o, 0) > 0 && count(&o, 1) > 0);
        let mut st = words("x");
        st.set_styles("negrito,vertical");
        assert!(st.bold && st.vertical && !st.italic);
        assert_eq!(st.styles(), "negrito,vertical");
    }

    #[test]
    fn colors_alternate_by_letter_and_pattern_whitens() {
        let s = plain(&ShapedWords {
            colors: vec![5, 6],
            ..words("ABAB")
        });
        assert!(count(&s, 5) > 0 && count(&s, 6) > 0);
        assert_eq!(count(&s, 1), 0);
        let inks = Inks {
            pattern: [0xaa; 8],
            ..INKS
        };
        let p = render(&words("MM"), Face::Bitmap(None), &inks);
        assert!(count(&p, 0) > 0 && count(&p, 1) > 0);
    }

    #[test]
    fn shadows_add_shadow_inks_under_the_letters() {
        let base = plain(&words("Ana"));
        for shadow in Shadow::ALL {
            let s = plain(&ShapedWords {
                shadow,
                ..words("Ana")
            });
            assert!(count(&s, 1) > 0, "{shadow:?}");
            let shade = count(&s, 2) + count(&s, 3) + count(&s, 4);
            if shadow == Shadow::None {
                assert_eq!(shade, 0);
                assert_eq!((s.w, s.h), (base.w, base.h));
            } else if shadow != Shadow::SpinCircle {
                assert!(shade > 0, "{shadow:?}");
            } else {
                // roda de cópias em contorno: maior e com branco por dentro
                assert!(s.w > base.w * 2 && count(&s, 0) > 0);
            }
            assert_eq!(Shadow::from_name(shadow.name()), Some(shadow));
        }
    }

    #[test]
    fn rotation_swaps_extent_and_steps() {
        let flat = plain(&words("Palavra"));
        let mut w = words("Palavra");
        w.rotate(90);
        let up = plain(&w);
        assert!(
            (up.w as i32 - flat.h as i32).abs() <= 3,
            "{} vs {}",
            up.w,
            flat.h
        );
        assert!(
            (up.h as i32 - flat.w as i32).abs() <= 3,
            "{} vs {}",
            up.h,
            flat.w
        );
        w.rotate(-100);
        assert_eq!(w.angle, 350);
        // tamanhos: degraus da lista, sem passar das pontas
        let mut s = words("a");
        s.size = DEFAULT_SIZE;
        assert!(s.step_size(true));
        assert_eq!(s.size, 160);
        s.size = 200;
        assert!(!s.step_size(true));
        s.size = 16;
        assert!(!s.step_size(false));
        assert!(s.step_size(true));
        assert_eq!(s.size, 24);
        s.size = 50;
        assert!(s.step_size(false));
        assert_eq!(s.size, 48);
    }

    #[test]
    fn outline_font_renders_when_installed() {
        let Some(face) = ttf::default_face() else {
            eprintln!("aviso: nenhuma fonte instalada; teste pulado");
            return;
        };
        let font = ttf::load(face).unwrap();
        let s = render(
            &ShapedWords {
                font: face.name.clone(),
                size: 48,
                ..words("Ana")
            },
            Face::Outline(&font),
            &INKS,
        );
        assert!(s.h >= 30 && s.h <= 60, "{}", s.h);
        assert!(s.w > s.h, "{}x{}", s.w, s.h);
        assert!(count(&s, 1) > 500);
    }
}
