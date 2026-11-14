//! Propriedades dos documentos `.MAX`: o espaço único de códigos "sprm"
//! (código de 1 byte + argumento de tamanho fixo por código) e as estruturas
//! tipadas de caractere, parágrafo, moldura e documento.
//!
//! Cada estrutura espelha os sprms que o original grava: um campo por código,
//! na ordem em que o Creative Writer os escreve (`docs/formats/max.md`).
//! Códigos sem campo próprio (desconhecidos, de outra categoria ou com valor
//! fora do conhecido) ficam em `extra` e são regravados como vieram.

use super::ObjRef;
use crate::{Rgb, Tag};

/// Tamanho do argumento de cada código conhecido; `None` = código
/// desconhecido (o resto do grpprl não pode ser interpretado).
pub fn arg_len(code: u8) -> Option<usize> {
    Some(match code {
        // caractere
        0x03..=0x06 | 0x0b..=0x0d => 1,
        0x0e | 0x0f => 2,
        0x10 | 0x11 => 6,
        // parágrafo
        0x19..=0x20 => 1,
        // moldura
        0x27 => 2,
        0x28..=0x2a => 1,
        0x2c..=0x31 => 2,
        0x35 | 0x37 => 6,
        // documento
        0x3e..=0x43 | 0x45 | 0x47..=0x4a | 0x51..=0x54 => 2,
        0x4c | 0x4d | 0x5b => 1,
        0x58 | 0x5a => 6,
        _ => return None,
    })
}

/// Um sprm cru: código e argumento.
///
/// Num código desconhecido o argumento é todo o resto do grpprl, porque o
/// tamanho não é conhecido.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sprm {
    pub code: u8,
    pub arg: Vec<u8>,
}

impl Sprm {
    fn new(code: u8, arg: &[u8]) -> Sprm {
        Sprm {
            code,
            arg: arg.to_vec(),
        }
    }
}

/// Divide um grpprl em sprms. Um código desconhecido, ou um argumento
/// cortado, leva o resto dos bytes num último sprm.
pub fn parse_grpprl(mut b: &[u8]) -> Vec<Sprm> {
    let mut out = Vec::new();
    while let Some((&code, rest)) = b.split_first() {
        match arg_len(code) {
            Some(n) if n <= rest.len() => {
                out.push(Sprm::new(code, &rest[..n]));
                b = &rest[n..];
            }
            _ => {
                out.push(Sprm::new(code, rest));
                break;
            }
        }
    }
    out
}

fn put_flag(v: &mut Vec<u8>, code: u8, on: bool) {
    if on {
        v.extend_from_slice(&[code, 0xff]);
    }
}

fn put_u8(v: &mut Vec<u8>, code: u8, x: Option<u8>) {
    if let Some(x) = x {
        v.extend_from_slice(&[code, x]);
    }
}

fn put_u16(v: &mut Vec<u8>, code: u8, x: Option<u16>) {
    if let Some(x) = x {
        v.push(code);
        v.extend_from_slice(&x.to_le_bytes());
    }
}

fn put_color(v: &mut Vec<u8>, code: u8, c: Option<MacColor>) {
    if let Some(c) = c {
        v.push(code);
        v.extend_from_slice(&c.to_bytes());
    }
}

fn put_ref(v: &mut Vec<u8>, code: u8, r: Option<ObjRef>) {
    if let Some(r) = r {
        v.push(code);
        v.extend_from_slice(&r.to_bytes());
    }
}

fn put_extra(v: &mut Vec<u8>, extra: &[Sprm]) {
    for s in extra {
        v.push(s.code);
        v.extend_from_slice(&s.arg);
    }
}

fn u16_of(a: &[u8]) -> u16 {
    u16::from_le_bytes([a[0], a[1]])
}

fn nonzero(x: u16) -> Option<u16> {
    (x != 0).then_some(x)
}

/// Cor no estilo do Mac: três componentes de 16 bits, com a cor no byte
/// alto. O byte baixo nem sempre repete o alto nos exemplos, por isso é
/// guardado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacColor {
    pub r: u16,
    pub g: u16,
    pub b: u16,
}

impl MacColor {
    /// A cor de 8 bits por componente (bytes altos).
    pub fn rgb(self) -> Rgb {
        Rgb {
            r: (self.r >> 8) as u8,
            g: (self.g >> 8) as u8,
            b: (self.b >> 8) as u8,
        }
    }

    /// Cor de 8 bits com o byte repetido (`#fa0039` → `fafa 0000 3939`),
    /// como o original grava as cores da tabela `CSCH`.
    pub fn from_rgb(c: Rgb) -> MacColor {
        MacColor {
            r: c.r as u16 * 0x101,
            g: c.g as u16 * 0x101,
            b: c.b as u16 * 0x101,
        }
    }

    fn from_bytes(a: &[u8]) -> MacColor {
        MacColor {
            r: u16_of(&a[0..]),
            g: u16_of(&a[2..]),
            b: u16_of(&a[4..]),
        }
    }

    fn to_bytes(self) -> [u8; 6] {
        let mut a = [0; 6];
        a[0..2].copy_from_slice(&self.r.to_le_bytes());
        a[2..4].copy_from_slice(&self.g.to_le_bytes());
        a[4..6].copy_from_slice(&self.b.to_le_bytes());
        a
    }
}

/// Efeito especial do texto (sprm `0x0b`; nomes da `STR#` 0x807).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    /// 1 "Pisca-pisca".
    Blink,
    /// 2 "Formiguinhas".
    Ants,
    /// 3 "Vapt".
    Vapt,
    /// 4 "Vupt".
    Vupt,
    /// 5 "Brilho".
    Glow,
    /// 6 "Treme-treme".
    Shake,
    /// 7 "Maluco".
    Crazy,
    /// Outro valor, guardado como veio.
    Other(u8),
}

impl Effect {
    pub fn from_code(v: u8) -> Effect {
        match v {
            1 => Effect::Blink,
            2 => Effect::Ants,
            3 => Effect::Vapt,
            4 => Effect::Vupt,
            5 => Effect::Glow,
            6 => Effect::Shake,
            7 => Effect::Crazy,
            v => Effect::Other(v),
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Effect::Blink => 1,
            Effect::Ants => 2,
            Effect::Vapt => 3,
            Effect::Vupt => 4,
            Effect::Glow => 5,
            Effect::Shake => 6,
            Effect::Crazy => 7,
            Effect::Other(v) => v,
        }
    }
}

/// Propriedades de caractere (páginas `FDPC`). O padrão (`Default`) é o
/// texto sem sprm: Lucida Sans (ftc 0), 12 pt ("Mínima"), preto, sem estilo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CharProps {
    /// `0x03`: caractere especial — âncora de um objeto dentro da linha; o
    /// byte no texto não é uma letra e não deve ser exibido nem exportado.
    pub anchor: bool,
    /// `0x04`: negrito.
    pub bold: bool,
    /// `0x05`: inclinado (itálico).
    pub italic: bool,
    /// `0x06`: sublinhado.
    pub underline: bool,
    /// `0x0b`: efeito especial.
    pub effect: Option<Effect>,
    /// `0x0c`: o texto tem cor própria (a de [`CharProps::color`]; sem ela,
    /// preto). Use [`CharProps::text_color`] / [`CharProps::set_text_color`].
    pub has_color: bool,
    /// `0x10`: cor do texto.
    pub color: Option<MacColor>,
    /// `0x0d`: marca-texto ligado (a cor está em [`CharProps::highlight`]).
    pub has_highlight: bool,
    /// `0x11`: cor do marca-texto.
    pub highlight: Option<MacColor>,
    /// `0x0e`: fonte, `ftc` da tabela `FONT` (ausente = 0, Lucida Sans).
    pub font: Option<u16>,
    /// `0x0f`: tamanho em pontos (ausente = 12). Ver [`super::SIZES_PT`].
    pub size: Option<u16>,
    /// Sprms sem campo próprio, regravados no fim, como vieram.
    pub extra: Vec<Sprm>,
}

impl CharProps {
    pub fn from_sprms(sprms: &[Sprm]) -> CharProps {
        let mut p = CharProps::default();
        for s in sprms {
            let a = &s.arg;
            match (s.code, a.len()) {
                (0x03, 1) => p.anchor = a[0] != 0,
                (0x04, 1) => p.bold = a[0] != 0,
                (0x05, 1) => p.italic = a[0] != 0,
                (0x06, 1) => p.underline = a[0] != 0,
                (0x0b, 1) if a[0] != 0 => p.effect = Some(Effect::from_code(a[0])),
                (0x0c, 1) => p.has_color = a[0] != 0,
                (0x0d, 1) => p.has_highlight = a[0] != 0,
                (0x0e, 2) => p.font = Some(u16_of(a)),
                (0x0f, 2) => p.size = Some(u16_of(a)),
                (0x10, 6) => p.color = Some(MacColor::from_bytes(a)),
                (0x11, 6) => p.highlight = Some(MacColor::from_bytes(a)),
                _ => p.extra.push(s.clone()),
            }
        }
        p
    }

    /// Lê um grpprl de caractere.
    pub fn from_grpprl(b: &[u8]) -> CharProps {
        CharProps::from_sprms(&parse_grpprl(b))
    }

    /// Grpprl em ordem crescente de código, como o original.
    pub fn to_grpprl(&self) -> Vec<u8> {
        let mut v = Vec::new();
        if self.anchor {
            v.extend_from_slice(&[0x03, 1]);
        }
        put_flag(&mut v, 0x04, self.bold);
        put_flag(&mut v, 0x05, self.italic);
        put_flag(&mut v, 0x06, self.underline);
        put_u8(&mut v, 0x0b, self.effect.map(Effect::code));
        put_flag(&mut v, 0x0c, self.has_color);
        put_flag(&mut v, 0x0d, self.has_highlight);
        put_u16(&mut v, 0x0e, self.font);
        put_u16(&mut v, 0x0f, self.size);
        put_color(&mut v, 0x10, self.color);
        put_color(&mut v, 0x11, self.highlight);
        put_extra(&mut v, &self.extra);
        v
    }

    /// Sem nenhum sprm.
    pub fn is_default(&self) -> bool {
        *self == CharProps::default()
    }

    /// Cor do texto; `None` = padrão (preto).
    pub fn text_color(&self) -> Option<Rgb> {
        self.has_color.then(|| {
            self.color
                .map(MacColor::rgb)
                .unwrap_or(Rgb { r: 0, g: 0, b: 0 })
        })
    }

    /// Define a cor do texto (`0x0c` + `0x10`); `None` volta ao preto padrão.
    pub fn set_text_color(&mut self, c: Option<Rgb>) {
        self.has_color = c.is_some();
        self.color = c.map(MacColor::from_rgb);
    }

    /// Cor do marca-texto, se ligado.
    pub fn highlight_color(&self) -> Option<Rgb> {
        self.highlight
            .filter(|_| self.has_highlight)
            .map(MacColor::rgb)
    }

    /// Liga (`Some`) ou desliga o marca-texto (`0x0d` + `0x11`).
    pub fn set_highlight_color(&mut self, c: Option<Rgb>) {
        self.has_highlight = c.is_some();
        self.highlight = c.map(MacColor::from_rgb);
    }

    /// Tamanho em pontos, com o padrão de 12.
    pub fn size_pt(&self) -> u16 {
        self.size.unwrap_or(super::DEFAULT_SIZE_PT)
    }

    /// `ftc` da fonte, com o padrão 0.
    pub fn font_ftc(&self) -> u16 {
        self.font.unwrap_or(0)
    }
}

/// Alinhamento do parágrafo (sprm `0x19`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Align {
    /// 0 ou ausente.
    #[default]
    Left,
    /// 1.
    Right,
    /// 2.
    Center,
    /// 3: "esquerda e direita".
    Justify,
}

/// Recuo do parágrafo (sprm `0x1a`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Indent {
    /// Ausente.
    #[default]
    None,
    /// 1: só a primeira linha.
    FirstLine,
    /// 2: todas menos a primeira (deduzido dos exemplos).
    Hanging,
    /// 3: todas as linhas.
    All,
}

/// Propriedades de parágrafo (páginas `FDPP`). O padrão é o parágrafo sem
/// sprm: alinhado à esquerda, sem recuo nem espaço extra.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ParaProps {
    /// `0x19`.
    pub align: Align,
    /// `0x1a`.
    pub indent: Indent,
    /// `0x1c`: espaço extra entre as linhas (entrelinha dupla).
    pub line_spacing: bool,
    /// `0x1b`: espaço extra depois do parágrafo (meia linha).
    pub space_after: bool,
    /// `0x1d`: letra capitular (grande e caída); o valor (3 nos exemplos)
    /// deve ser o número de linhas.
    pub drop_cap: Option<u8>,
    /// `0x1e`: letra destacada (grande e saltada).
    pub raised_cap: bool,
    /// `0x1f`: o parágrafo começa uma nova página.
    pub new_page: bool,
    /// `0x20`: marcadores (lista com bolinha).
    pub bullets: bool,
    /// Sprms sem campo próprio.
    pub extra: Vec<Sprm>,
}

impl ParaProps {
    pub fn from_sprms(sprms: &[Sprm]) -> ParaProps {
        let mut p = ParaProps::default();
        for s in sprms {
            let a = &s.arg;
            match (s.code, a.len()) {
                (0x19, 1) if a[0] <= 3 => {
                    p.align =
                        [Align::Left, Align::Right, Align::Center, Align::Justify][a[0] as usize]
                }
                (0x1a, 1) if a[0] <= 3 => {
                    p.indent = [
                        Indent::None,
                        Indent::FirstLine,
                        Indent::Hanging,
                        Indent::All,
                    ][a[0] as usize]
                }
                (0x1b, 1) => p.space_after = a[0] != 0,
                (0x1c, 1) => p.line_spacing = a[0] != 0,
                (0x1d, 1) if a[0] != 0 => p.drop_cap = Some(a[0]),
                (0x1e, 1) => p.raised_cap = a[0] != 0,
                (0x1f, 1) => p.new_page = a[0] != 0,
                (0x20, 1) => p.bullets = a[0] != 0,
                _ => p.extra.push(s.clone()),
            }
        }
        p
    }

    /// Lê um grpprl de parágrafo.
    pub fn from_grpprl(b: &[u8]) -> ParaProps {
        ParaProps::from_sprms(&parse_grpprl(b))
    }

    /// Grpprl em ordem crescente de código.
    pub fn to_grpprl(&self) -> Vec<u8> {
        let mut v = Vec::new();
        let align = match self.align {
            Align::Left => None,
            Align::Right => Some(1),
            Align::Center => Some(2),
            Align::Justify => Some(3),
        };
        put_u8(&mut v, 0x19, align);
        let indent = match self.indent {
            Indent::None => None,
            Indent::FirstLine => Some(1),
            Indent::Hanging => Some(2),
            Indent::All => Some(3),
        };
        put_u8(&mut v, 0x1a, indent);
        put_flag(&mut v, 0x1b, self.space_after);
        put_flag(&mut v, 0x1c, self.line_spacing);
        put_u8(&mut v, 0x1d, self.drop_cap);
        put_flag(&mut v, 0x1e, self.raised_cap);
        put_flag(&mut v, 0x1f, self.new_page);
        put_flag(&mut v, 0x20, self.bullets);
        put_extra(&mut v, &self.extra);
        v
    }

    pub fn is_default(&self) -> bool {
        *self == ParaProps::default()
    }
}

/// Tipo de objeto solto (sprm `0x28`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameKind {
    /// 2: Splot (palavras com forma, chunk `SPLE`).
    Splot,
    /// 3: figura ou adesivo (`PICT`).
    Picture,
    /// 4: moldura de página (cartões).
    PageFrame,
    /// Outro valor (0 quando o sprm falta).
    Other(u8),
}

impl FrameKind {
    pub fn from_code(v: u8) -> FrameKind {
        match v {
            2 => FrameKind::Splot,
            3 => FrameKind::Picture,
            4 => FrameKind::PageFrame,
            v => FrameKind::Other(v),
        }
    }

    pub fn code(self) -> u8 {
        match self {
            FrameKind::Splot => 2,
            FrameKind::Picture => 3,
            FrameKind::PageFrame => 4,
            FrameKind::Other(v) => v,
        }
    }
}

/// Uma moldura da lista `FRAM`: posição e tamanho de um objeto solto
/// (figura, Splot, moldura de cartão), em pontos da página. Os campos
/// numéricos valem 0 quando o sprm falta, e o gravador omite os que valem 0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame {
    /// `0x27`: página (0 = primeira).
    pub page: u16,
    /// `0x28`.
    pub kind: FrameKind,
    /// `0x29`: modo de contorno do texto ("Texto flutuante"; 2 ou 3, a mapear).
    pub wrap: Option<u8>,
    /// `0x2a`: sempre `ff` nas figuras e Splots do original (significado
    /// desconhecido).
    pub flag_2a: bool,
    /// `0x2c` / `0x2d`: canto superior esquerdo.
    pub x: i16,
    pub y: i16,
    /// `0x2e` / `0x2f`: tamanho atual.
    pub width: u16,
    pub height: u16,
    /// `0x30` / `0x31`: tamanho original (difere do atual depois de esticar).
    pub orig_width: u16,
    pub orig_height: u16,
    /// `0x35`: o objeto (`PICT n`, `SPLE n`, `BDR n`).
    pub object: Option<ObjRef>,
    /// `0x37`: som preso ao objeto (`SND n`).
    pub sound: Option<ObjRef>,
    /// Sprms sem campo próprio.
    pub extra: Vec<Sprm>,
}

impl Frame {
    /// Figura ou Splot no lugar e tamanho dados (tamanho original igual ao
    /// atual), com `0x2a` ligado como no original.
    pub fn new(kind: FrameKind, object: ObjRef, x: i16, y: i16, width: u16, height: u16) -> Frame {
        Frame {
            page: 0,
            kind,
            wrap: None,
            flag_2a: true,
            x,
            y,
            width,
            height,
            orig_width: width,
            orig_height: height,
            object: Some(object),
            sound: None,
            extra: Vec::new(),
        }
    }

    pub fn from_sprms(sprms: &[Sprm]) -> Frame {
        let mut f = Frame {
            page: 0,
            kind: FrameKind::Other(0),
            wrap: None,
            flag_2a: false,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            orig_width: 0,
            orig_height: 0,
            object: None,
            sound: None,
            extra: Vec::new(),
        };
        for s in sprms {
            let a = &s.arg;
            match (s.code, a.len()) {
                (0x27, 2) => f.page = u16_of(a),
                (0x28, 1) => f.kind = FrameKind::from_code(a[0]),
                (0x29, 1) => f.wrap = Some(a[0]),
                (0x2a, 1) => f.flag_2a = a[0] != 0,
                (0x2c, 2) => f.x = u16_of(a) as i16,
                (0x2d, 2) => f.y = u16_of(a) as i16,
                (0x2e, 2) => f.width = u16_of(a),
                (0x2f, 2) => f.height = u16_of(a),
                (0x30, 2) => f.orig_width = u16_of(a),
                (0x31, 2) => f.orig_height = u16_of(a),
                (0x35, 6) => f.object = Some(ObjRef::from_bytes(a)),
                (0x37, 6) => f.sound = Some(ObjRef::from_bytes(a)),
                _ => f.extra.push(s.clone()),
            }
        }
        f
    }

    /// Grpprl em ordem crescente de código, omitindo os campos zerados.
    pub fn to_grpprl(&self) -> Vec<u8> {
        let mut v = Vec::new();
        put_u16(&mut v, 0x27, nonzero(self.page));
        put_u8(&mut v, 0x28, Some(self.kind.code()));
        put_u8(&mut v, 0x29, self.wrap);
        put_flag(&mut v, 0x2a, self.flag_2a);
        put_u16(&mut v, 0x2c, nonzero(self.x as u16));
        put_u16(&mut v, 0x2d, nonzero(self.y as u16));
        put_u16(&mut v, 0x2e, nonzero(self.width));
        put_u16(&mut v, 0x2f, nonzero(self.height));
        put_u16(&mut v, 0x30, nonzero(self.orig_width));
        put_u16(&mut v, 0x31, nonzero(self.orig_height));
        put_ref(&mut v, 0x35, self.object);
        put_ref(&mut v, 0x37, self.sound);
        put_extra(&mut v, &self.extra);
        v
    }
}

/// Propriedades do documento (chunk `DOP `), em pontos (1/72 pol.).
///
/// Os oito primeiros campos são sempre gravados; os demais só quando
/// presentes (diferentes do padrão). O padrão é o documento novo do
/// original: folha "Alto" de 595 × 792, margens de 72 (2,5 cm), uma coluna.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DopProps {
    /// `0x3e` / `0x3f`: 595 × 792 "Alto", 792 × 595 "Largo", 542 × 767 nas faixas.
    pub page_width: u16,
    pub page_height: u16,
    /// `0x40`–`0x43`: margens (ordem topo, esquerda, base, direita deduzida;
    /// o original grava as quatro iguais): 36, 54, 72, 90 ou 108.
    pub margin_top: u16,
    pub margin_left: u16,
    pub margin_bottom: u16,
    pub margin_right: u16,
    /// `0x45`: espaço entre colunas (12; 42 nos cartões).
    pub column_gap: u16,
    /// `0x49`: 36 em todos os documentos (significado desconhecido).
    pub field_49: u16,
    /// `0x4a` (= colunas − 1): número de colunas, 1 a 3.
    pub columns: u16,
    /// `0x4c`: `ff` nos jornais (a mapear).
    pub newspaper: bool,
    /// `0x4d`: estilo do número de página: 1 "Pág. N", 2 "N" (deduzido).
    pub page_number_style: Option<u8>,
    /// `0x51`–`0x54`: retângulo do número de página (topo, esquerda, base,
    /// direita).
    pub page_number_rect: Option<[u16; 4]>,
    /// `0x48` / `0x47`: larguras da borda de página (externa/interna, deduzido).
    pub border_outer: Option<u16>,
    pub border_inner: Option<u16>,
    /// `0x58` / `0x5a`: a borda (`BDR n`); o original grava as duas iguais.
    pub border: Option<ObjRef>,
    pub border_alt: Option<ObjRef>,
    /// `0x5b`: `ff` nas faixas.
    pub banner: bool,
    /// Sprms sem campo próprio.
    pub extra: Vec<Sprm>,
}

impl Default for DopProps {
    fn default() -> DopProps {
        DopProps {
            page_width: 595,
            page_height: 792,
            margin_top: 72,
            margin_left: 72,
            margin_bottom: 72,
            margin_right: 72,
            column_gap: 12,
            field_49: 36,
            columns: 1,
            newspaper: false,
            page_number_style: None,
            page_number_rect: None,
            border_outer: None,
            border_inner: None,
            border: None,
            border_alt: None,
            banner: false,
            extra: Vec::new(),
        }
    }
}

impl DopProps {
    /// Lê os sprms; os oito campos sempre gravados ficam com 0 se faltarem.
    pub fn from_sprms(sprms: &[Sprm]) -> DopProps {
        let mut d = DopProps {
            page_width: 0,
            page_height: 0,
            margin_top: 0,
            margin_left: 0,
            margin_bottom: 0,
            margin_right: 0,
            column_gap: 0,
            field_49: 0,
            ..DopProps::default()
        };
        let mut rect = [None::<u16>; 4];
        for s in sprms {
            let a = &s.arg;
            match (s.code, a.len()) {
                (0x3e, 2) => d.page_width = u16_of(a),
                (0x3f, 2) => d.page_height = u16_of(a),
                (0x40, 2) => d.margin_top = u16_of(a),
                (0x41, 2) => d.margin_left = u16_of(a),
                (0x42, 2) => d.margin_bottom = u16_of(a),
                (0x43, 2) => d.margin_right = u16_of(a),
                (0x45, 2) => d.column_gap = u16_of(a),
                (0x47, 2) => d.border_inner = Some(u16_of(a)),
                (0x48, 2) => d.border_outer = Some(u16_of(a)),
                (0x49, 2) => d.field_49 = u16_of(a),
                (0x4a, 2) => d.columns = u16_of(a).saturating_add(1),
                (0x4c, 1) => d.newspaper = a[0] != 0,
                (0x4d, 1) => d.page_number_style = Some(a[0]),
                (c @ 0x51..=0x54, 2) => rect[(c - 0x51) as usize] = Some(u16_of(a)),
                (0x58, 6) => d.border = Some(ObjRef::from_bytes(a)),
                (0x5a, 6) => d.border_alt = Some(ObjRef::from_bytes(a)),
                (0x5b, 1) => d.banner = a[0] != 0,
                _ => d.extra.push(s.clone()),
            }
        }
        if rect.iter().any(Option::is_some) {
            d.page_number_rect = Some(rect.map(|x| x.unwrap_or(0)));
        }
        d
    }

    /// Grpprl na ordem do original: `3e`–`43`, `45`, `49`, `4a`, `4c`,
    /// `4d`, `51`–`54`, `48`, `58`, `47`, `5a`, `5b`.
    pub fn to_grpprl(&self) -> Vec<u8> {
        let mut v = Vec::new();
        for (code, x) in [
            (0x3e, self.page_width),
            (0x3f, self.page_height),
            (0x40, self.margin_top),
            (0x41, self.margin_left),
            (0x42, self.margin_bottom),
            (0x43, self.margin_right),
            (0x45, self.column_gap),
            (0x49, self.field_49),
        ] {
            put_u16(&mut v, code, Some(x));
        }
        put_u16(&mut v, 0x4a, (self.columns > 1).then(|| self.columns - 1));
        put_flag(&mut v, 0x4c, self.newspaper);
        put_u8(&mut v, 0x4d, self.page_number_style);
        if let Some(r) = self.page_number_rect {
            for (k, x) in r.into_iter().enumerate() {
                put_u16(&mut v, 0x51 + k as u8, Some(x));
            }
        }
        put_u16(&mut v, 0x48, self.border_outer);
        put_ref(&mut v, 0x58, self.border);
        put_u16(&mut v, 0x47, self.border_inner);
        put_ref(&mut v, 0x5a, self.border_alt);
        put_flag(&mut v, 0x5b, self.banner);
        put_extra(&mut v, &self.extra);
        v
    }
}

impl ObjRef {
    pub(super) fn from_bytes(a: &[u8]) -> ObjRef {
        ObjRef {
            tag: Tag([a[0], a[1], a[2], a[3]]),
            id: u16_of(&a[4..]),
        }
    }

    pub(super) fn to_bytes(self) -> [u8; 6] {
        let mut a = [0; 6];
        a[..4].copy_from_slice(&self.tag.0);
        a[4..].copy_from_slice(&self.id.to_le_bytes());
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        let s: String = s.split_whitespace().collect();
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn spacing_sprms_1b_after_and_1c_between_lines() {
        // MIX_TE53.MAX (office-complemento.md §3.1): "Era uma vez" grava
        // `1d 03 1b ff` = capitular + espaço depois do parágrafo
        let p = ParaProps::from_grpprl(&hex("1d03 1bff"));
        assert!(p.space_after && !p.line_spacing);
        assert_eq!(p.drop_cap, Some(3));
        let p = ParaProps::from_grpprl(&hex("1cff"));
        assert!(p.line_spacing && !p.space_after);
        assert_eq!(p.to_grpprl(), hex("1cff"));
        let p = ParaProps {
            space_after: true,
            ..ParaProps::default()
        };
        assert_eq!(p.to_grpprl(), hex("1bff"));
    }

    #[test]
    fn parse_grpprl_stops_at_unknown_code() {
        let s = parse_grpprl(&hex("04ff 0e0100 07 aabb"));
        assert_eq!(s.len(), 3);
        assert_eq!(s[1], Sprm::new(0x0e, &[1, 0]));
        assert_eq!(s[2], Sprm::new(0x07, &[0xaa, 0xbb]));
        // argumento cortado
        assert_eq!(parse_grpprl(&[0x10, 1, 2]), [Sprm::new(0x10, &[1, 2])]);
        assert!(parse_grpprl(&[]).is_empty());
    }

    #[test]
    fn char_props_from_samp1_grpprl() {
        // SAMP1.MAX: negrito, cor, ftc 1, 48 pt, #fa0039
        let g = hex("04ff 0cff 0e0100 0f3000 10fafa00003939");
        let p = CharProps::from_grpprl(&g);
        assert!(p.bold && p.has_color && !p.italic);
        assert_eq!(p.font, Some(1));
        assert_eq!(p.size_pt(), 48);
        assert_eq!(
            p.text_color(),
            Some(Rgb {
                r: 0xfa,
                g: 0,
                b: 0x39
            })
        );
        assert!(p.extra.is_empty());
        assert_eq!(p.to_grpprl(), g);
    }

    #[test]
    fn char_props_keep_low_color_bytes_and_unknown_sprms() {
        let g = hex("0cff 109393f32ca3a3 0b05 07 0102");
        let p = CharProps::from_grpprl(&g);
        assert_eq!(p.effect, Some(Effect::Glow));
        assert_eq!(p.color.unwrap().g, 0x2cf3);
        assert_eq!(p.text_color().unwrap().g, 0x2c);
        assert_eq!(p.extra, [Sprm::new(0x07, &[1, 2])]);
        // regravado em ordem crescente, extra no fim
        assert_eq!(p.to_grpprl(), hex("0b05 0cff 109393f32ca3a3 07 0102"));
    }

    #[test]
    fn color_setters_and_defaults() {
        let mut p = CharProps::default();
        assert!(p.is_default());
        assert_eq!(p.text_color(), None);
        assert_eq!((p.size_pt(), p.font_ftc()), (12, 0));
        let red = Rgb {
            r: 0xfa,
            g: 0,
            b: 0x39,
        };
        p.set_text_color(Some(red));
        assert_eq!(p.to_grpprl(), hex("0cff 10fafa00003939"));
        p.set_highlight_color(Some(red));
        assert_eq!(p.highlight_color(), Some(red));
        p.set_text_color(None);
        p.set_highlight_color(None);
        assert!(p.is_default());
        // 0x0c sem 0x10 = preto
        let p = CharProps::from_grpprl(&hex("0cff"));
        assert_eq!(p.text_color(), Some(Rgb { r: 0, g: 0, b: 0 }));
        assert_eq!(MacColor::from_rgb(red).rgb(), red);
        assert_eq!(Effect::from_code(9), Effect::Other(9));
        for c in 1..=9 {
            assert_eq!(Effect::from_code(c).code(), c);
        }
    }

    #[test]
    fn para_props_roundtrip() {
        for g in [
            "1902",
            "1903",
            "1901 1a01 1bff 1cff 1d03 1eff 1fff 20ff",
            "1a02",
        ] {
            let g = hex(g);
            let p = ParaProps::from_grpprl(&g);
            assert!(p.extra.is_empty());
            assert_eq!(p.to_grpprl(), g);
        }
        let p = ParaProps::from_grpprl(&hex("1900"));
        assert!(p.is_default());
        let p = ParaProps::from_grpprl(&hex("1909"));
        assert_eq!(p.extra.len(), 1);
        assert_eq!(p.to_grpprl(), hex("1909"));
    }

    #[test]
    fn frame_from_s3() {
        let g = hex("2803 2aff 2cfe00 2d3a02 2e9500 2f9600 309500 319600 355049435402 00");
        let f = Frame::from_sprms(&parse_grpprl(&g));
        assert_eq!(f.kind, FrameKind::Picture);
        assert_eq!((f.x, f.y, f.width, f.height), (254, 570, 149, 150));
        assert_eq!(f.object, Some(ObjRef::new(b"PICT", 2)));
        assert!(f.flag_2a && f.page == 0 && f.sound.is_none());
        assert_eq!(f.to_grpprl(), g);
        assert_eq!(
            Frame::new(
                FrameKind::Picture,
                ObjRef::new(b"PICT", 2),
                254,
                570,
                149,
                150
            ),
            f
        );
        // moldura de cartão: página 3, contorno 2, 2 × 2
        let g = hex("270300 2804 2902 2e0200 2f0200");
        let f = Frame::from_sprms(&parse_grpprl(&g));
        assert_eq!((f.page, f.kind, f.wrap), (3, FrameKind::PageFrame, Some(2)));
        assert_eq!(f.to_grpprl(), g);
    }

    #[test]
    fn dop_default_is_the_new_document() {
        let g = hex("3e5302 3f1803 404800 414800 424800 434800 450c00 492400");
        assert_eq!(DopProps::default().to_grpprl(), g);
        assert_eq!(DopProps::from_sprms(&parse_grpprl(&g)), DopProps::default());
    }

    #[test]
    fn dop_s4_keeps_original_order() {
        // S4.MAX: Largo, margens 108, 3 colunas, "Pág. N", borda de patinhas
        let g = hex(
            "3e1803 3f5302 406c00 416c00 426c00 436c00 450c00 492400 4a0200 4d01
             514402 526c01 535302 54ac01 483100 584244522000 00 472100 5a424452200000",
        );
        let d = DopProps::from_sprms(&parse_grpprl(&g));
        assert_eq!((d.page_width, d.page_height, d.columns), (792, 595, 3));
        assert_eq!(d.page_number_style, Some(1));
        assert_eq!(d.page_number_rect, Some([580, 364, 595, 428]));
        assert_eq!((d.border_outer, d.border_inner), (Some(49), Some(33)));
        assert_eq!(d.border, Some(ObjRef::new(b"BDR ", 0)));
        assert!(d.extra.is_empty());
        assert_eq!(d.to_grpprl(), g);
        // faixa (SAMP17): borda e 0x5b no fim
        let g = hex(
            "3e1e02 3fff02 404800 414800 424800 434800 450c00 492400 482500
             584244522 03c00 471900 5a424452203c00 5bff",
        );
        let d = DopProps::from_sprms(&parse_grpprl(&g));
        assert!(d.banner);
        assert_eq!(d.to_grpprl(), g);
        // jornal (SAMP15)
        let g = hex("3e5302 3f1803 403600 413600 423600 433600 450c00 492400 4a0200 4cff");
        let d = DopProps::from_sprms(&parse_grpprl(&g));
        assert!(d.newspaper);
        assert_eq!(d.to_grpprl(), g);
    }

    #[test]
    fn every_known_code_has_a_size() {
        let known = [
            0x03, 0x04, 0x05, 0x06, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x27, 0x28, 0x29, 0x2a, 0x2c, 0x2d, 0x2e, 0x2f, 0x30,
            0x31, 0x35, 0x37, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x45, 0x47, 0x48, 0x49, 0x4a,
            0x4c, 0x4d, 0x51, 0x52, 0x53, 0x54, 0x58, 0x5a, 0x5b,
        ];
        for c in 0..=255u8 {
            assert_eq!(arg_len(c).is_some(), known.contains(&c), "0x{c:02x}");
        }
    }
}
