//! Documento rico do Escritório do Creative Writer.
//!
//! O modelo segue o `.MAX` (`docs/formats/max.md`, `formats::max`), com os
//! mesmos nomes e semânticas, para que a conversão seja campo a campo:
//!
//! - [`CharProps`] = página `FDPC` (sprms `0x04`–`0x11`): fonte (`ftc` da
//!   [`FontTable`]), um dos 7 [`Size`]s, cor e iluminador (RGB; o
//!   Seleciona-cores oferece as 16 de [`CSCH`]), negrito, inclinado,
//!   sublinhado e [`Effect`] especial;
//! - [`ParaProps`] = página `FDPP` (sprms `0x19`–`0x20`): alinhamento,
//!   recuo, espaçamento e estilo de parágrafo;
//! - [`DopProps`] = chunk `DOP ` (sprms `0x3e`–`0x5b`): folha, margens,
//!   colunas, alto/largo, borda, fundo e número de página;
//! - [`FloatObject`] = moldura `FRAM` (ver [`super::object`]).
//!
//! Por dentro, o texto é uma lista de [`Paragraph`]s (nunca vazia), cada
//! um com seus itens: caracteres com as propriedades de cada um, ou figuras
//! do "Maçã" dentro da linha ([`Item::Rebus`], o caractere-âncora do
//! sprm `0x03`). O `\r` que fecha cada parágrafo não é item: as
//! propriedades dele ficam em [`Paragraph::mark`] (dão a altura da linha
//! vazia). Posições são [`Pos`] (parágrafo, índice entre itens).

use super::object::{FloatObject, ObjRef};
use formats::Rgb;

/// Trecho do modelo provisório, ainda usado pelos testes da cena atual.
#[doc(hidden)]
pub use super::legacy::doc::Run;

const fn rgb(c: u32) -> Rgb {
    Rgb {
        r: (c >> 16) as u8,
        g: (c >> 8) as u8,
        b: c as u8,
    }
}

/// As 16 cores do Seleciona-cores e do Iluminador (recurso `CSCH` do
/// `WRITER.EXE`), na ordem da lista: branco, amarelo, laranja, marrom, rosa,
/// vermelho, lavanda, roxo, magenta, violeta, verde, verde-escuro,
/// cinza-claro, cinza, cinza-escuro e preto. (A captura da lista mediu o
/// magenta como `#d016de`, que também aparece em documentos de exemplo.)
pub const CSCH: [Rgb; 16] = [
    rgb(0xffffff),
    rgb(0xffee4c),
    rgb(0xff8c32),
    rgb(0xb96524),
    rgb(0xfa4a72),
    rgb(0xfa0039),
    rgb(0x8888e6),
    rgb(0x6141db),
    rgb(0xde38c7),
    rgb(0x932ca3),
    rgb(0x5ccb53),
    rgb(0x478c50),
    rgb(0xbebebe),
    rgb(0x8c8c8c),
    rgb(0x646464),
    rgb(0x000000),
];

/// Cor do texto sem cor própria.
pub const BLACK: Rgb = rgb(0);

/// Fonte de `ftc` 0, a padrão dos documentos (`STR#` 0x800 [1]).
pub const DEFAULT_FONT: &str = "Lucida Sans";

/// As fontes da lista do Seleciona-letras (`STR#` 0x206), com a família do
/// `LOGFONT` gravada na tabela `FONT` (1 romana, 2 sem serifa, 4 script,
/// 5 decorativa; as de Pablo e Handwriting são deduzidas).
pub const FONTS: [(&str, u16); 6] = [
    ("Pablo Graphos", 5),
    ("Lucida Sans", 2),
    ("Lucida Handwriting", 4),
    ("Lucida Blackletter", 4),
    ("Impact", 2),
    ("Stencil", 5),
];

/// Os 7 tamanhos do Dimensiona-palavras, na ordem da lista.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Size {
    Giant,
    Huge,
    Large,
    Medium,
    Small,
    /// "Mínima": o padrão (texto sem o sprm `0x0f`).
    #[default]
    Minimal,
    Micro,
}

impl Size {
    pub const ALL: [Size; 7] = [
        Size::Giant,
        Size::Huge,
        Size::Large,
        Size::Medium,
        Size::Small,
        Size::Minimal,
        Size::Micro,
    ];

    /// Tamanho em pontos (sprm `0x0f`), medido em documentos do original.
    pub fn pt(self) -> u16 {
        match self {
            Size::Giant => 48,
            Size::Huge => 36,
            Size::Large => 24,
            Size::Medium => 18,
            Size::Small => 14,
            Size::Minimal => 12,
            Size::Micro => 9,
        }
    }

    /// O tamanho da lista mais perto de `pt` (documentos do Mac podem ter
    /// outros; nos 27 exemplos só aparecem os 7).
    pub fn from_pt(pt: u16) -> Size {
        Size::ALL
            .into_iter()
            .min_by_key(|s| s.pt().abs_diff(pt))
            .unwrap_or_default()
    }

    pub fn name(self) -> &'static str {
        match self {
            Size::Giant => "Gigante",
            Size::Huge => "Enorme",
            Size::Large => "Grande",
            Size::Medium => "Média",
            Size::Small => "Pequena",
            Size::Minimal => "Mínima",
            Size::Micro => "Micro",
        }
    }
}

/// Efeito especial animado (sprm `0x0b`, nomes da `STR#` 0x807).
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
    Sparkle,
    /// 6 "Treme-treme".
    Shake,
    /// 7 "Maluco".
    Crazy,
    /// Outro valor, guardado como veio.
    Other(u8),
}

impl Effect {
    /// Os efeitos da lista "Efeitos especiais", na ordem da lista (depois de
    /// "Nada de especial").
    pub const LIST: [Effect; 6] = [
        Effect::Blink,
        Effect::Ants,
        Effect::Sparkle,
        Effect::Shake,
        Effect::Vapt,
        Effect::Vupt,
    ];

    pub fn code(self) -> u8 {
        match self {
            Effect::Blink => 1,
            Effect::Ants => 2,
            Effect::Vapt => 3,
            Effect::Vupt => 4,
            Effect::Sparkle => 5,
            Effect::Shake => 6,
            Effect::Crazy => 7,
            Effect::Other(v) => v,
        }
    }

    pub fn from_code(v: u8) -> Effect {
        match v {
            1 => Effect::Blink,
            2 => Effect::Ants,
            3 => Effect::Vapt,
            4 => Effect::Vupt,
            5 => Effect::Sparkle,
            6 => Effect::Shake,
            7 => Effect::Crazy,
            v => Effect::Other(v),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Effect::Blink => "Pisca-pisca",
            Effect::Ants => "Formiguinhas",
            Effect::Vapt => "Vapt",
            Effect::Vupt => "Vupt",
            Effect::Sparkle => "Brilho",
            Effect::Shake => "Treme-treme",
            Effect::Crazy => "Maluco",
            Effect::Other(_) => "?",
        }
    }
}

/// Propriedades de um caractere. O padrão é o texto sem sprm: Lucida Sans
/// (ftc 0), "Mínima" (12 pt), preto, sem estilo nem efeito.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharProps {
    /// `0x0e`: fonte, `ftc` da [`FontTable`] do documento.
    pub font: u16,
    /// `0x0f`.
    pub size: Size,
    /// `0x0c` + `0x10`: cor do texto; `None` = preto.
    pub color: Option<Rgb>,
    /// `0x0d` + `0x11`: cor do Iluminador (fundo atrás das letras).
    pub highlight: Option<Rgb>,
    /// `0x04`.
    pub bold: bool,
    /// `0x05`.
    pub italic: bool,
    /// `0x06`.
    pub underline: bool,
    /// `0x0b`.
    pub effect: Option<Effect>,
}

impl CharProps {
    /// Cor efetiva do texto.
    pub fn text_color(&self) -> Rgb {
        self.color.unwrap_or(BLACK)
    }

    /// Sem nenhum sprm.
    pub fn is_default(&self) -> bool {
        *self == CharProps::default()
    }
}

/// Estilos do Seleciona-estilos: "Simples" tira os três; os outros ligam o
/// seu sem desligar os demais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextStyle {
    Plain,
    Bold,
    Italic,
    Underline,
}

impl TextStyle {
    pub const ALL: [TextStyle; 4] = [
        TextStyle::Plain,
        TextStyle::Bold,
        TextStyle::Italic,
        TextStyle::Underline,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TextStyle::Plain => "Simples",
            TextStyle::Bold => "Negrito",
            TextStyle::Italic => "Inclinado",
            TextStyle::Underline => "Sublinhado",
        }
    }
}

/// O que um pincel da Palavra muda nos caracteres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharChange {
    /// Seleciona-letras (`ftc`).
    Font(u16),
    /// Dimensiona-palavras.
    Size(Size),
    /// Seleciona-cores (`None` = preto padrão).
    Color(Option<Rgb>),
    /// Iluminador (`None` = sem).
    Highlight(Option<Rgb>),
    /// Seleciona-estilos.
    Style(TextStyle),
    /// Efeitos especiais (`None` = "Nada de especial").
    Effect(Option<Effect>),
    /// Tudo de uma vez (Super-simples, Aplicador com um estilo pronto).
    All(CharProps),
}

impl CharChange {
    pub fn apply(self, p: &mut CharProps) {
        match self {
            CharChange::Font(f) => p.font = f,
            CharChange::Size(s) => p.size = s,
            CharChange::Color(c) => p.color = c.filter(|&c| c != BLACK),
            CharChange::Highlight(c) => p.highlight = c,
            CharChange::Style(TextStyle::Plain) => {
                p.bold = false;
                p.italic = false;
                p.underline = false;
            }
            CharChange::Style(TextStyle::Bold) => p.bold = true,
            CharChange::Style(TextStyle::Italic) => p.italic = true,
            CharChange::Style(TextStyle::Underline) => p.underline = true,
            CharChange::Effect(e) => p.effect = e,
            CharChange::All(all) => *p = all,
        }
    }
}

/// Alinhamento (Posicionador; sprm `0x19`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Align {
    #[default]
    Left,
    Right,
    Center,
    /// "Esquerda e direita".
    Justify,
}

impl Align {
    /// Ordem da lista: Esquerda, Direita, Centro, Esquerda e direita.
    pub const ALL: [Align; 4] = [Align::Left, Align::Right, Align::Center, Align::Justify];

    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(v: u8) -> Align {
        match v {
            1 => Align::Right,
            2 => Align::Center,
            3 => Align::Justify,
            _ => Align::Left,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Align::Left => "Esquerda",
            Align::Right => "Direita",
            Align::Center => "Centro",
            Align::Justify => "Esquerda e direita",
        }
    }
}

/// Recuos (sprm `0x1a`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Indent {
    /// "Não recuar".
    #[default]
    None,
    /// 1: só a primeira linha.
    FirstLine,
    /// 2: todas menos a primeira.
    Hanging,
    /// 3: todas as linhas.
    All,
}

impl Indent {
    pub const ALL: [Indent; 4] = [
        Indent::None,
        Indent::FirstLine,
        Indent::Hanging,
        Indent::All,
    ];

    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(v: u8) -> Indent {
        match v {
            1 => Indent::FirstLine,
            2 => Indent::Hanging,
            3 => Indent::All,
            _ => Indent::None,
        }
    }
}

/// Itens da lista "Estilos de parágrafo" (exclusivos entre si).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ParaStyle {
    /// "Nada de especial".
    #[default]
    Normal,
    /// "Capitular": letra grande e caída (sprm `0x1d` = 3 linhas).
    DropCap,
    /// "Destacada": letra grande e saltada (`0x1e`).
    RaisedCap,
    /// "Marcadores": lista com bolinha (`0x20`).
    Bullets,
}

/// Itens da lista "Espaçamento" (exclusivos entre si).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Spacing {
    /// "Nada de especial".
    #[default]
    Normal,
    /// "Começa uma nova página" (`0x1f`).
    NewPage,
    /// Espaço extra entre as linhas (`0x1b`).
    ExtraLines,
    /// Espaço extra depois do parágrafo (`0x1c`).
    ExtraAfter,
}

/// Linhas da capitular gravadas pelo original (sprm `0x1d` = 3).
pub const DROP_CAP_LINES: u8 = 3;

/// Propriedades de parágrafo (página `FDPP`). O padrão é o parágrafo sem
/// sprm: à esquerda, sem recuo nem espaço extra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ParaProps {
    /// `0x19`.
    pub align: Align,
    /// `0x1a`.
    pub indent: Indent,
    /// `0x1b`: espaço extra entre as linhas.
    pub line_spacing: bool,
    /// `0x1c`: espaço extra depois do parágrafo.
    pub space_after: bool,
    /// `0x1d`: capitular, com o número de linhas.
    pub drop_cap: Option<u8>,
    /// `0x1e`: letra destacada.
    pub raised_cap: bool,
    /// `0x1f`: começa uma nova página.
    pub new_page: bool,
    /// `0x20`: marcadores.
    pub bullets: bool,
}

impl ParaProps {
    pub fn para_style(&self) -> ParaStyle {
        if self.drop_cap.is_some() {
            ParaStyle::DropCap
        } else if self.raised_cap {
            ParaStyle::RaisedCap
        } else if self.bullets {
            ParaStyle::Bullets
        } else {
            ParaStyle::Normal
        }
    }

    pub fn set_para_style(&mut self, s: ParaStyle) {
        self.drop_cap = (s == ParaStyle::DropCap).then_some(DROP_CAP_LINES);
        self.raised_cap = s == ParaStyle::RaisedCap;
        self.bullets = s == ParaStyle::Bullets;
    }

    pub fn spacing(&self) -> Spacing {
        if self.new_page {
            Spacing::NewPage
        } else if self.line_spacing {
            Spacing::ExtraLines
        } else if self.space_after {
            Spacing::ExtraAfter
        } else {
            Spacing::Normal
        }
    }

    pub fn set_spacing(&mut self, s: Spacing) {
        self.new_page = s == Spacing::NewPage;
        self.line_spacing = s == Spacing::ExtraLines;
        self.space_after = s == Spacing::ExtraAfter;
    }

    pub fn is_default(&self) -> bool {
        *self == ParaProps::default()
    }
}

/// O que um pincel de parágrafo muda (basta clicar no parágrafo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParaChange {
    Align(Align),
    Indent(Indent),
    Style(ParaStyle),
    Spacing(Spacing),
    All(ParaProps),
}

impl ParaChange {
    pub fn apply(self, p: &mut ParaProps) {
        match self {
            ParaChange::Align(a) => p.align = a,
            ParaChange::Indent(i) => p.indent = i,
            ParaChange::Style(s) => p.set_para_style(s),
            ParaChange::Spacing(s) => p.set_spacing(s),
            ParaChange::All(all) => *p = all,
        }
    }
}

/// As 5 margens da lista (Preparação da página), em pontos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Margin {
    /// 1,3 cm.
    Cm13,
    /// 2 cm.
    Cm2,
    /// 2,5 cm (padrão).
    #[default]
    Cm25,
    /// 3,2 cm.
    Cm32,
    /// 3,8 cm.
    Cm38,
}

impl Margin {
    pub const ALL: [Margin; 5] = [
        Margin::Cm13,
        Margin::Cm2,
        Margin::Cm25,
        Margin::Cm32,
        Margin::Cm38,
    ];

    pub fn pt(self) -> u16 {
        match self {
            Margin::Cm13 => 36,
            Margin::Cm2 => 54,
            Margin::Cm25 => 72,
            Margin::Cm32 => 90,
            Margin::Cm38 => 108,
        }
    }

    /// A margem da lista mais perto de `pt`.
    pub fn from_pt(pt: u16) -> Margin {
        Margin::ALL
            .into_iter()
            .min_by_key(|m| m.pt().abs_diff(pt))
            .unwrap_or_default()
    }

    pub fn name(self) -> &'static str {
        match self {
            Margin::Cm13 => "1,3 cm",
            Margin::Cm2 => "2 cm",
            Margin::Cm25 => "2,5 cm",
            Margin::Cm32 => "3,2 cm",
            Margin::Cm38 => "3,8 cm",
        }
    }
}

/// "Alto ou largo".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Orientation {
    #[default]
    Tall,
    Wide,
}

/// Número de página (sprm `0x4d`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PageNumber {
    /// Nenhum.
    #[default]
    None,
    /// 1: "Pág. 1" (`STR#` 0x800 [2] "Pág. %d").
    Label,
    /// 2: só "1".
    Plain,
}

impl PageNumber {
    pub const ALL: [PageNumber; 3] = [PageNumber::None, PageNumber::Label, PageNumber::Plain];

    pub fn code(self) -> Option<u8> {
        match self {
            PageNumber::None => None,
            PageNumber::Label => Some(1),
            PageNumber::Plain => Some(2),
        }
    }

    pub fn from_code(v: Option<u8>) -> PageNumber {
        match v {
            Some(1) => PageNumber::Label,
            Some(2) => PageNumber::Plain,
            _ => PageNumber::None,
        }
    }

    /// Texto do número da página `n` (0 = primeira).
    pub fn text(self, n: usize) -> Option<String> {
        match self {
            PageNumber::None => None,
            PageNumber::Label => Some(format!("Pág. {}", n + 1)),
            PageNumber::Plain => Some((n + 1).to_string()),
        }
    }
}

/// Largura e altura padrão da caixa do número de página, em pontos
/// (medidas na folha "Largo" de `S4.MAX`: (580, 364, 595, 428)).
pub const PAGE_NUMBER_W: u16 = 64;
pub const PAGE_NUMBER_H: u16 = 15;

/// Colunas possíveis no Cria-colunas.
pub const MAX_COLUMNS: u16 = 4;

/// Propriedades do documento (chunk `DOP `), em pontos (1/72 pol.). O padrão
/// é o documento novo do original (`18 3e 53 02 3f 18 03 40 48 00 …`):
/// folha "Alto" de 595 × 792, margens de 72 (2,5 cm), uma coluna com
/// 12 pt entre colunas. Na Visão normal 1 pt ≈ 1 pixel (a página ocupa
/// x 23..615 da tela, o texto começa em (93, 180)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DopProps {
    /// `0x3e` / `0x3f`: 595 × 792 "Alto", 792 × 595 "Largo", 542 × 767 nas
    /// faixas.
    pub page_width: u16,
    pub page_height: u16,
    /// `0x40`–`0x43` (topo, esquerda, base, direita): o original grava as
    /// quatro iguais, uma das 5 de [`Margin`].
    pub margin_top: u16,
    pub margin_left: u16,
    pub margin_bottom: u16,
    pub margin_right: u16,
    /// `0x45`: espaço entre colunas (12; 42 nos cartões).
    pub column_gap: u16,
    /// `0x49`: 36 em todos os documentos (significado desconhecido).
    pub field_49: u16,
    /// `0x4a` + 1: colunas, 1 a [`MAX_COLUMNS`].
    pub columns: u16,
    /// `0x4c`: ligado nos jornais (a mapear).
    pub newspaper: bool,
    /// `0x5b`: ligado nas faixas.
    pub banner: bool,
    /// `0x4d`.
    pub page_number: PageNumber,
    /// `0x51`–`0x54`: caixa do número (topo, esquerda, base, direita); pode
    /// ser arrastada. `None` = a posição padrão ([`DopProps::page_number_box`]).
    pub page_number_rect: Option<[u16; 4]>,
    /// `0x58`: a borda de página (`BDR n`).
    pub border: Option<ObjRef>,
    /// `0x5a`: segunda referência de borda; o original a grava igual à
    /// `border`.
    pub border_alt: Option<ObjRef>,
    /// `0x47` / `0x48`: larguras da borda (interna/externa, deduzido).
    pub border_inner: Option<u16>,
    pub border_outer: Option<u16>,
    /// Fundo (rolo FUNDO, figuras dos `.BOR`) em todas as páginas. O sprm
    /// que o grava não aparece nos exemplos (W26); a conversão o ignora até
    /// ser identificado.
    pub background: Option<ObjRef>,
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
            banner: false,
            page_number: PageNumber::None,
            page_number_rect: None,
            border: None,
            border_alt: None,
            border_inner: None,
            border_outer: None,
            background: None,
        }
    }
}

impl DopProps {
    pub fn margin(&self) -> Margin {
        Margin::from_pt(self.margin_top)
    }

    /// Põe as quatro margens iguais, como o original.
    pub fn set_margin(&mut self, m: Margin) {
        let v = m.pt();
        self.margin_top = v;
        self.margin_left = v;
        self.margin_bottom = v;
        self.margin_right = v;
    }

    pub fn orientation(&self) -> Orientation {
        if self.page_width > self.page_height {
            Orientation::Wide
        } else {
            Orientation::Tall
        }
    }

    /// Alto ou largo: troca largura e altura (a caixa do número volta à
    /// posição padrão).
    pub fn set_orientation(&mut self, o: Orientation) {
        if self.orientation() != o {
            std::mem::swap(&mut self.page_width, &mut self.page_height);
            self.page_number_rect = None;
        }
    }

    /// Cria-colunas: 1 a 4.
    pub fn set_columns(&mut self, n: u16) {
        self.columns = n.clamp(1, MAX_COLUMNS);
    }

    /// Escolhe a borda (as duas referências iguais, como o original grava).
    pub fn set_border(&mut self, b: Option<ObjRef>) {
        self.border = b;
        self.border_alt = b;
    }

    pub fn set_page_number(&mut self, n: PageNumber) {
        self.page_number = n;
    }

    /// Caixa do número de página `(topo, esquerda, base, direita)`: a
    /// gravada, ou centrada no pé da folha.
    pub fn page_number_box(&self) -> [u16; 4] {
        self.page_number_rect.unwrap_or_else(|| {
            let cx = self.page_width / 2;
            let left = cx.saturating_sub(PAGE_NUMBER_W / 2);
            [
                self.page_height.saturating_sub(PAGE_NUMBER_H),
                left,
                self.page_height,
                left + PAGE_NUMBER_W,
            ]
        })
    }

    /// Área do texto `(x0, y0, x1, y1)` na folha, com o fim exclusivo.
    pub fn text_area(&self) -> (i32, i32, i32, i32) {
        let x0 = i32::from(self.margin_left);
        let y0 = i32::from(self.margin_top);
        let x1 = (i32::from(self.page_width) - i32::from(self.margin_right)).max(x0 + 1);
        let y1 = (i32::from(self.page_height) - i32::from(self.margin_bottom)).max(y0 + 1);
        (x0, y0, x1, y1)
    }

    /// Faixas `(x0, x1)` de cada coluna dentro da área do texto.
    pub fn column_spans(&self) -> Vec<(i32, i32)> {
        let (x0, _, x1, _) = self.text_area();
        let n = i32::from(self.columns.clamp(1, MAX_COLUMNS));
        let gap = i32::from(self.column_gap);
        let w = ((x1 - x0 - gap * (n - 1)) / n).max(1);
        (0..n)
            .map(|k| {
                let a = x0 + k * (w + gap);
                (a, a + w)
            })
            .collect()
    }
}

/// Uma fonte da tabela `FONT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontEntry {
    pub ftc: u16,
    pub name: String,
    /// Família do `LOGFONT` (`lfPitchAndFamily >> 4`).
    pub family: u16,
}

/// Tabela de fontes do documento (chunk `FONT`). O `ftc` 0 é sempre a
/// Lucida Sans; um `ftc` ausente (documentos do Mac) vale a padrão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontTable {
    pub entries: Vec<FontEntry>,
}

impl Default for FontTable {
    fn default() -> FontTable {
        FontTable {
            entries: vec![FontEntry {
                ftc: 0,
                name: DEFAULT_FONT.into(),
                family: 2,
            }],
        }
    }
}

impl FontTable {
    /// Nome da fonte `ftc`, ou a padrão.
    pub fn name(&self, ftc: u16) -> &str {
        self.entries
            .iter()
            .find(|e| e.ftc == ftc)
            .map_or(DEFAULT_FONT, |e| e.name.as_str())
    }

    pub fn ftc_of(&self, name: &str) -> Option<u16> {
        self.entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
            .map(|e| e.ftc)
    }

    /// O `ftc` da fonte, acrescentando-a se faltar.
    pub fn ensure(&mut self, name: &str) -> u16 {
        if let Some(f) = self.ftc_of(name) {
            return f;
        }
        let ftc = self.entries.iter().map(|e| e.ftc + 1).max().unwrap_or(0);
        let family = FONTS
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map_or(0, |&(_, f)| f);
        self.entries.push(FontEntry {
            ftc,
            name: name.into(),
            family,
        });
        ftc
    }
}

/// Um elemento do parágrafo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Char(char, CharProps),
    /// Figura do "Maçã" (Coloca-figuras) dentro da linha: o byte do
    /// caractere-âncora no `TEXT` (sprm `0x03`). Não é texto: a exportação e
    /// o corretor o pulam.
    Rebus(u8),
}

impl Item {
    pub fn char(&self) -> Option<char> {
        match self {
            Item::Char(c, _) => Some(*c),
            Item::Rebus(_) => None,
        }
    }

    pub fn props(&self) -> Option<&CharProps> {
        match self {
            Item::Char(_, p) => Some(p),
            Item::Rebus(_) => None,
        }
    }

    pub fn is_space(&self) -> bool {
        matches!(self, Item::Char(c, _) if c.is_whitespace())
    }

    /// Faz parte de uma palavra (para o pincel e a quebra de linha): letras,
    /// números, pontuação colada e figuras.
    pub fn is_word(&self) -> bool {
        !self.is_space()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paragraph {
    pub props: ParaProps,
    pub items: Vec<Item>,
    /// Propriedades do `\r` que fecha o parágrafo.
    pub mark: CharProps,
}

impl Paragraph {
    pub fn new(props: ParaProps, mark: CharProps) -> Paragraph {
        Paragraph {
            props,
            items: Vec::new(),
            mark,
        }
    }

    pub fn from_text(text: &str, p: CharProps) -> Paragraph {
        Paragraph {
            props: ParaProps::default(),
            items: text.chars().map(|c| Item::Char(c, p)).collect(),
            mark: p,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Só o texto (sem as figuras).
    pub fn text(&self) -> String {
        self.items.iter().filter_map(Item::char).collect()
    }

    /// Trechos de itens consecutivos com as mesmas propriedades (cada
    /// figura é um trecho), como `(início, fim)`.
    pub fn spans(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = Vec::new();
        for (k, it) in self.items.iter().enumerate() {
            match out.last_mut() {
                Some((s, e)) if it.props().is_some() && self.items[*s].props() == it.props() => {
                    *e = k + 1
                }
                _ => out.push((k, k + 1)),
            }
        }
        out
    }
}

/// Posição entre itens: antes do item `index` do parágrafo `para`
/// (`index == len` = no fim, antes do `\r`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Pos {
    pub para: usize,
    pub index: usize,
}

impl Pos {
    pub fn new(para: usize, index: usize) -> Pos {
        Pos { para, index }
    }
}

/// Intervalo `start..end` de posições, com `start <= end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Range {
    pub start: Pos,
    pub end: Pos,
}

impl Range {
    /// Intervalo entre duas posições em qualquer ordem.
    pub fn new(a: Pos, b: Pos) -> Range {
        Range {
            start: a.min(b),
            end: a.max(b),
        }
    }

    pub fn at(p: Pos) -> Range {
        Range { start: p, end: p }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, p: Pos) -> bool {
        self.start <= p && p < self.end
    }
}

/// Trecho de documento copiado: parágrafos com as propriedades de origem; o
/// primeiro e o último podem ser parciais (entre dois deles há uma quebra
/// de parágrafo). As fontes vão pelo nome, para colar em outro documento.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fragment {
    pub paras: Vec<Paragraph>,
    /// `(ftc, nome)` das fontes usadas.
    pub fonts: Vec<(u16, String)>,
}

impl Fragment {
    pub fn is_empty(&self) -> bool {
        self.paras.len() <= 1 && self.paras.iter().all(Paragraph::is_empty)
    }

    /// Texto, com `\n` entre os parágrafos.
    pub fn text(&self) -> String {
        self.paras
            .iter()
            .map(Paragraph::text)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// O documento: texto, objetos soltos, folha e fontes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Nunca vazio.
    pub paras: Vec<Paragraph>,
    /// Na ordem de desenho (o último fica por cima).
    pub objects: Vec<FloatObject>,
    pub dop: DopProps,
    pub fonts: FontTable,
}

impl Default for Document {
    fn default() -> Document {
        Document::new()
    }
}

impl Document {
    /// Documento Novo: um parágrafo vazio, folha padrão.
    pub fn new() -> Document {
        Document {
            paras: vec![Paragraph::default()],
            objects: Vec::new(),
            dop: DopProps::default(),
            fonts: FontTable::default(),
        }
    }

    /// Documento com o texto (`\n` separa parágrafos) num só estilo.
    pub fn from_text(text: &str, p: CharProps) -> Document {
        let mut d = Document::new();
        d.paras = text
            .split('\n')
            .map(|l| Paragraph::from_text(l.trim_end_matches('\r'), p))
            .collect();
        d
    }

    /// Sem texto nem objetos.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty() && self.paras.iter().all(Paragraph::is_empty)
    }

    /// Texto puro ("Salvar como texto"), parágrafos separados por `\n`, sem
    /// as figuras.
    pub fn text(&self) -> String {
        self.paras
            .iter()
            .map(Paragraph::text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Texto do intervalo, com `\n` nas quebras de parágrafo.
    pub fn text_range(&self, r: Range) -> String {
        self.slice(r).text()
    }

    pub fn start(&self) -> Pos {
        Pos::default()
    }

    pub fn end(&self) -> Pos {
        let p = self.paras.len() - 1;
        Pos::new(p, self.paras[p].len())
    }

    /// O documento inteiro.
    pub fn all(&self) -> Range {
        Range::new(self.start(), self.end())
    }

    /// A posição válida mais perto.
    pub fn clamp(&self, p: Pos) -> Pos {
        let para = p.para.min(self.paras.len() - 1);
        Pos::new(para, p.index.min(self.paras[para].len()))
    }

    pub fn item(&self, p: Pos) -> Option<&Item> {
        self.paras.get(p.para)?.items.get(p.index)
    }

    /// Uma posição para trás (o fim do parágrafo anterior conta como uma).
    pub fn prev(&self, p: Pos) -> Option<Pos> {
        let p = self.clamp(p);
        if p.index > 0 {
            Some(Pos::new(p.para, p.index - 1))
        } else if p.para > 0 {
            Some(Pos::new(p.para - 1, self.paras[p.para - 1].len()))
        } else {
            None
        }
    }

    pub fn next(&self, p: Pos) -> Option<Pos> {
        let p = self.clamp(p);
        if p.index < self.paras[p.para].len() {
            Some(Pos::new(p.para, p.index + 1))
        } else if p.para + 1 < self.paras.len() {
            Some(Pos::new(p.para + 1, 0))
        } else {
            None
        }
    }

    /// Começo da palavra anterior (Ctrl+←).
    pub fn prev_word(&self, p: Pos) -> Pos {
        let mut p = self.clamp(p);
        while p.index > 0 && self.paras[p.para].items[p.index - 1].is_space() {
            p.index -= 1;
        }
        if p.index == 0 {
            return self.prev(p).unwrap_or(p);
        }
        while p.index > 0 && self.paras[p.para].items[p.index - 1].is_word() {
            p.index -= 1;
        }
        p
    }

    /// Começo da palavra seguinte (Ctrl+→).
    pub fn next_word(&self, p: Pos) -> Pos {
        let mut p = self.clamp(p);
        let items = &self.paras[p.para].items;
        if p.index == items.len() {
            return self.next(p).unwrap_or(p);
        }
        while p.index < items.len() && items[p.index].is_word() {
            p.index += 1;
        }
        while p.index < items.len() && items[p.index].is_space() {
            p.index += 1;
        }
        p
    }

    /// A palavra em `p` (o pincel vale para palavras inteiras): os itens
    /// não-espaço em volta de `p`; vazio se `p` está entre espaços.
    pub fn word_at(&self, p: Pos) -> Range {
        let p = self.clamp(p);
        let items = &self.paras[p.para].items;
        let mut s = p.index;
        while s > 0 && items[s - 1].is_word() {
            s -= 1;
        }
        let mut e = p.index;
        while e < items.len() && items[e].is_word() {
            e += 1;
        }
        Range::new(Pos::new(p.para, s), Pos::new(p.para, e))
    }

    /// Estende o intervalo até cobrir as palavras que ele toca (o arrasto do
    /// pincel). Um intervalo vazio vira a palavra em que está.
    pub fn expand_to_words(&self, r: Range) -> Range {
        let s = self.word_at(r.start).start;
        let end = if r.is_empty() {
            r.end
        } else {
            // a palavra do último item tocado
            self.prev(r.end).filter(|&q| q >= r.start).unwrap_or(r.end)
        };
        let w = self.word_at(end);
        let e = if r.is_empty() || w.end > end {
            w.end
        } else {
            r.end
        };
        Range::new(s.min(r.start), e.max(r.end))
    }

    /// Propriedades para o que for digitado em `p`: as do caractere antes,
    /// senão as do seguinte, senão as do `\r` do parágrafo.
    pub fn props_at(&self, p: Pos) -> CharProps {
        let p = self.clamp(p);
        let par = &self.paras[p.para];
        let before = p
            .index
            .checked_sub(1)
            .and_then(|i| par.items[..=i].iter().rev().find_map(Item::props));
        before
            .or_else(|| par.items[p.index..].iter().find_map(Item::props))
            .copied()
            .unwrap_or(par.mark)
    }

    /// Insere o texto em `p`; `\n` (ou `\r`) quebra o parágrafo e outros
    /// caracteres de controle são ignorados. Devolve a posição depois do
    /// texto.
    pub fn insert_text(&mut self, p: Pos, text: &str, props: CharProps) -> Pos {
        let mut p = self.clamp(p);
        let mut pending_cr = false;
        for c in text.chars() {
            if c == '\r' || (c == '\n' && !pending_cr) {
                p = self.split(p);
            } else if !c.is_control() {
                p = self.insert_item(p, Item::Char(c, props));
            }
            pending_cr = c == '\r';
        }
        p
    }

    /// Insere um item em `p`; devolve a posição depois dele.
    pub fn insert_item(&mut self, p: Pos, item: Item) -> Pos {
        let p = self.clamp(p);
        self.paras[p.para].items.insert(p.index, item);
        Pos::new(p.para, p.index + 1)
    }

    /// Enter: divide o parágrafo em `p`. O novo herda as propriedades do
    /// parágrafo (menos "começa uma nova página") e do `\r`; devolve o começo
    /// dele.
    pub fn split(&mut self, p: Pos) -> Pos {
        let p = self.clamp(p);
        let par = &mut self.paras[p.para];
        let rest = par.items.split_off(p.index);
        let mut props = par.props;
        props.new_page = false;
        // o `\r` do primeiro pedaço fica com o estilo do último caractere
        let mark = par.mark;
        if let Some(last) = par.items.iter().rev().find_map(Item::props) {
            par.mark = *last;
        }
        let next = Paragraph {
            props,
            items: rest,
            mark,
        };
        self.paras.insert(p.para + 1, next);
        Pos::new(p.para + 1, 0)
    }

    /// Apaga o intervalo, juntando os parágrafos das pontas (o resultado
    /// fica com as propriedades do primeiro). Devolve `r.start`.
    pub fn delete(&mut self, r: Range) -> Pos {
        let (s, e) = (self.clamp(r.start), self.clamp(r.end));
        if s >= e {
            return s;
        }
        if s.para == e.para {
            self.paras[s.para].items.drain(s.index..e.index);
        } else {
            let tail = self.paras[e.para].items.split_off(e.index);
            let mark = self.paras[e.para].mark;
            self.paras.drain(s.para + 1..=e.para);
            let first = &mut self.paras[s.para];
            first.items.truncate(s.index);
            first.items.extend(tail);
            first.mark = mark;
        }
        s
    }

    /// Backspace: apaga o item antes de `p` (ou junta com o parágrafo
    /// anterior). Devolve a nova posição, ou `None` no começo do documento.
    pub fn backspace(&mut self, p: Pos) -> Option<Pos> {
        let prev = self.prev(p)?;
        Some(self.delete(Range::new(prev, p)))
    }

    /// Delete: apaga o item depois de `p`. Falso no fim do documento.
    pub fn delete_forward(&mut self, p: Pos) -> bool {
        match self.next(p) {
            Some(n) => {
                self.delete(Range::new(p, n));
                true
            }
            None => false,
        }
    }

    /// Cópia do intervalo.
    pub fn slice(&self, r: Range) -> Fragment {
        let (s, e) = (
            self.clamp(r.start),
            self.clamp(r.end).max(self.clamp(r.start)),
        );
        let mut paras = Vec::new();
        for k in s.para..=e.para {
            let p = &self.paras[k];
            let a = if k == s.para { s.index } else { 0 };
            let b = if k == e.para { e.index } else { p.len() };
            paras.push(Paragraph {
                props: p.props,
                items: p.items[a..b].to_vec(),
                mark: p.mark,
            });
        }
        let mut used: Vec<u16> = paras
            .iter()
            .flat_map(|p| {
                p.items
                    .iter()
                    .filter_map(Item::props)
                    .map(|c| c.font)
                    .chain([p.mark.font])
            })
            .collect();
        used.sort_unstable();
        used.dedup();
        let fonts = used
            .into_iter()
            .map(|f| (f, self.fonts.name(f).to_string()))
            .collect();
        Fragment { paras, fonts }
    }

    /// Cola o trecho em `p` e devolve a posição depois dele. As fontes são
    /// casadas pelo nome com a tabela deste documento. Se o trecho tem mais
    /// de um parágrafo, cada um volta com as propriedades de origem e o
    /// último se junta ao resto do parágrafo de `p` (recortar e colar no
    /// mesmo lugar devolve o documento original).
    pub fn insert_fragment(&mut self, p: Pos, f: &Fragment) -> Pos {
        let p = self.clamp(p);
        if f.paras.is_empty() {
            return p;
        }
        let map: Vec<(u16, u16)> = f
            .fonts
            .iter()
            .map(|(old, name)| (*old, self.fonts.ensure(name)))
            .collect();
        let remap = |mut c: CharProps| {
            if let Some(&(_, new)) = map.iter().find(|(old, _)| *old == c.font) {
                c.font = new;
            }
            c
        };
        let remap_item = |it: &Item| match *it {
            Item::Char(ch, c) => Item::Char(ch, remap(c)),
            r => r,
        };
        let n = f.paras.len();
        let mut at = p;
        for (k, par) in f.paras.iter().enumerate() {
            if k > 0 {
                at = self.split(at);
            }
            for it in &par.items {
                at = self.insert_item(at, remap_item(it));
            }
        }
        if n > 1 {
            // cada parágrafo fechado por um `\r` colado fica com as
            // propriedades de origem; o último, com as do seu parágrafo de
            // origem, mas com o `\r` de `p`
            for (k, par) in f.paras.iter().enumerate() {
                let dst = &mut self.paras[p.para + k];
                dst.props = par.props;
                if k + 1 < n {
                    dst.mark = remap(par.mark);
                }
            }
        }
        at
    }

    /// Aplica a mudança aos caracteres do intervalo e aos `\r` que ele cobre;
    /// num parágrafo vazio, um intervalo vazio muda o `\r` (para a linha
    /// vazia ter o tamanho escolhido).
    pub fn apply_char(&mut self, r: Range, ch: CharChange) {
        let (s, e) = (self.clamp(r.start), self.clamp(r.end));
        for k in s.para..=e.para {
            let par = &mut self.paras[k];
            let a = if k == s.para { s.index } else { 0 };
            let b = if k == e.para { e.index } else { par.len() };
            for it in &mut par.items[a..b] {
                if let Item::Char(_, p) = it {
                    ch.apply(p);
                }
            }
            // o `\r` acompanha quando o intervalo passa por ele ou cobre o
            // parágrafo inteiro (inclusive o vazio)
            if k < e.para || (a == 0 && b == par.len()) {
                ch.apply(&mut par.mark);
            }
        }
    }

    /// Aplica a mudança a todos os parágrafos tocados pelo intervalo.
    pub fn apply_para(&mut self, r: Range, ch: ParaChange) {
        let (s, e) = (self.clamp(r.start), self.clamp(r.end));
        for par in &mut self.paras[s.para..=e.para] {
            ch.apply(&mut par.props);
        }
    }

    /// "Jogar tudo fora": apaga o texto e os objetos (a folha fica).
    pub fn clear(&mut self) {
        self.paras = vec![Paragraph::default()];
        self.objects.clear();
    }

    /// O objeto de cima no ponto `(x, y)` da página `page`.
    pub fn object_at(&self, page: usize, x: i32, y: i32) -> Option<usize> {
        self.objects
            .iter()
            .rposition(|o| o.on_page(page) && o.contains(x, y))
    }

    /// Põe o objeto por cima dos outros; devolve o índice.
    pub fn add_object(&mut self, o: FloatObject) -> usize {
        self.objects.push(o);
        self.objects.len() - 1
    }

    pub fn remove_object(&mut self, k: usize) -> Option<FloatObject> {
        (k < self.objects.len()).then(|| self.objects.remove(k))
    }

    /// Remove-sons: tira o som do objeto. Verdadeiro se havia.
    pub fn remove_object_sound(&mut self, k: usize) -> bool {
        self.objects
            .get_mut(k)
            .and_then(|o| o.sound.take())
            .is_some()
    }

    /// Páginas que os objetos exigem (o maior número de página + 1).
    pub fn object_pages(&self) -> usize {
        self.objects
            .iter()
            .map(|o| usize::from(o.page) + 1)
            .max()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::object::{ObjectKind, Wrap};

    fn big() -> CharProps {
        CharProps {
            size: Size::Giant,
            ..CharProps::default()
        }
    }

    fn doc(text: &str) -> Document {
        Document::from_text(text, CharProps::default())
    }

    #[test]
    fn defaults_match_the_original() {
        let d = Document::new();
        assert_eq!(d.paras.len(), 1);
        assert!(d.is_empty());
        let p = CharProps::default();
        assert_eq!((p.font, p.size, p.size.pt()), (0, Size::Minimal, 12));
        assert_eq!(p.text_color(), BLACK);
        assert!(p.is_default() && ParaProps::default().is_default());
        assert_eq!(d.fonts.name(0), "Lucida Sans");
        assert_eq!(d.fonts.name(9), "Lucida Sans");
        let dop = &d.dop;
        assert_eq!((dop.page_width, dop.page_height), (595, 792));
        assert_eq!(dop.margin(), Margin::Cm25);
        assert_eq!(dop.text_area(), (72, 72, 523, 720));
        assert_eq!(dop.column_spans(), vec![(72, 523)]);
        assert_eq!(CSCH.len(), 16);
        assert_eq!(CSCH[5], rgb(0xfa0039));
        assert_eq!(CSCH[15], BLACK);
    }

    #[test]
    fn sizes_effects_and_codes() {
        let pts: Vec<u16> = Size::ALL.iter().map(|s| s.pt()).collect();
        assert_eq!(pts, [48, 36, 24, 18, 14, 12, 9]);
        for s in Size::ALL {
            assert_eq!(Size::from_pt(s.pt()), s);
        }
        assert_eq!(Size::from_pt(10), Size::Micro);
        assert_eq!(Size::from_pt(100), Size::Giant);
        for v in 1..=9 {
            assert_eq!(Effect::from_code(v).code(), v);
        }
        assert_eq!(Effect::from_code(5), Effect::Sparkle);
        assert_eq!(Effect::LIST.len(), 6);
        for a in Align::ALL {
            assert_eq!(Align::from_code(a.code()), a);
        }
        for i in Indent::ALL {
            assert_eq!(Indent::from_code(i.code()), i);
        }
        assert_eq!(Indent::Hanging.code(), 2);
        for n in PageNumber::ALL {
            assert_eq!(PageNumber::from_code(n.code()), n);
        }
        assert_eq!(PageNumber::Label.text(0).as_deref(), Some("Pág. 1"));
        assert_eq!(PageNumber::Plain.text(4).as_deref(), Some("5"));
        let ms: Vec<u16> = Margin::ALL.iter().map(|m| m.pt()).collect();
        assert_eq!(ms, [36, 54, 72, 90, 108]);
    }

    #[test]
    fn char_changes() {
        let mut p = CharProps::default();
        CharChange::Style(TextStyle::Bold).apply(&mut p);
        CharChange::Style(TextStyle::Underline).apply(&mut p);
        assert!(p.bold && p.underline && !p.italic);
        CharChange::Style(TextStyle::Plain).apply(&mut p);
        assert!(!p.bold && !p.underline);
        CharChange::Color(Some(CSCH[5])).apply(&mut p);
        assert_eq!(p.text_color(), CSCH[5]);
        CharChange::Color(Some(BLACK)).apply(&mut p);
        assert_eq!(p.color, None, "preto é o padrão, sem sprm");
        CharChange::Highlight(Some(CSCH[1])).apply(&mut p);
        CharChange::Effect(Some(Effect::Blink)).apply(&mut p);
        CharChange::Size(Size::Large).apply(&mut p);
        CharChange::Font(3).apply(&mut p);
        assert_eq!(
            p,
            CharProps {
                font: 3,
                size: Size::Large,
                highlight: Some(CSCH[1]),
                effect: Some(Effect::Blink),
                ..CharProps::default()
            }
        );
    }

    #[test]
    fn para_groups_are_exclusive() {
        let mut p = ParaProps::default();
        p.set_para_style(ParaStyle::DropCap);
        assert_eq!(p.drop_cap, Some(3));
        p.set_para_style(ParaStyle::Bullets);
        assert_eq!((p.drop_cap, p.bullets), (None, true));
        assert_eq!(p.para_style(), ParaStyle::Bullets);
        p.set_spacing(Spacing::ExtraAfter);
        p.set_spacing(Spacing::NewPage);
        assert!(p.new_page && !p.space_after);
        assert_eq!(p.spacing(), Spacing::NewPage);
        ParaChange::Indent(Indent::FirstLine).apply(&mut p);
        ParaChange::Align(Align::Center).apply(&mut p);
        // Relatório: espaço depois + recuo da 1ª linha convivem
        ParaChange::Spacing(Spacing::ExtraAfter).apply(&mut p);
        assert_eq!(
            (p.indent, p.align, p.space_after),
            (Indent::FirstLine, Align::Center, true)
        );
        assert!(p.bullets);
    }

    #[test]
    fn page_setup() {
        let mut d = DopProps::default();
        d.set_margin(Margin::Cm38);
        assert_eq!(
            (d.margin_top, d.margin_left, d.margin_bottom, d.margin_right),
            (108, 108, 108, 108)
        );
        d.set_orientation(Orientation::Wide);
        assert_eq!((d.page_width, d.page_height), (792, 595));
        assert_eq!(d.orientation(), Orientation::Wide);
        // posição medida em S4.MAX ("Largo")
        assert_eq!(d.page_number_box(), [580, 364, 595, 428]);
        d.set_columns(3);
        let cols = d.column_spans();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].0, 108);
        assert_eq!(cols[1].0 - cols[0].1, 12);
        assert!(cols[2].1 <= 792 - 108);
        d.set_columns(9);
        assert_eq!(d.columns, 4);
        d.set_columns(0);
        assert_eq!(d.columns, 1);
        d.set_border(Some(ObjRef::bdr(0)));
        assert_eq!(d.border, d.border_alt);
        d.page_number_rect = Some([1, 2, 3, 4]);
        d.set_orientation(Orientation::Wide);
        assert_eq!(d.page_number_rect, Some([1, 2, 3, 4]));
        d.set_orientation(Orientation::Tall);
        assert_eq!(d.page_number_rect, None);
    }

    #[test]
    fn font_table() {
        let mut t = FontTable::default();
        assert_eq!(t.ensure("lucida sans"), 0);
        assert_eq!(t.ensure("Stencil"), 1);
        assert_eq!(t.ensure("Impact"), 2);
        assert_eq!(t.ensure("Stencil"), 1);
        assert_eq!(t.entries[1].family, 5);
        assert_eq!(t.name(2), "Impact");
        assert_eq!(t.ftc_of("Times"), None);
    }

    #[test]
    fn insert_split_and_delete() {
        let mut d = doc("abc");
        let p = d.insert_text(Pos::new(0, 1), "X\nY", big());
        assert_eq!(d.text(), "aX\nYbc");
        assert_eq!(p, Pos::new(1, 1));
        // \r\n conta uma quebra só
        d.insert_text(d.end(), "1\r\n2", CharProps::default());
        assert_eq!(d.text(), "aX\nYbc1\n2");
        let p = d.delete(Range::new(Pos::new(0, 1), Pos::new(2, 0)));
        assert_eq!(p, Pos::new(0, 1));
        assert_eq!(d.text(), "a2");
        assert_eq!(d.backspace(Pos::new(0, 1)), Some(Pos::new(0, 0)));
        assert_eq!(d.text(), "2");
        assert_eq!(d.backspace(Pos::new(0, 0)), None);
        assert!(d.delete_forward(Pos::new(0, 0)));
        assert!(!d.delete_forward(Pos::new(0, 0)));
        assert_eq!(d.text(), "");
    }

    #[test]
    fn split_keeps_props_but_not_new_page() {
        let mut d = doc("abcd");
        d.apply_para(d.all(), ParaChange::Align(Align::Right));
        d.apply_para(d.all(), ParaChange::Spacing(Spacing::NewPage));
        d.apply_char(
            Range::new(Pos::new(0, 0), Pos::new(0, 4)),
            CharChange::Size(Size::Huge),
        );
        let p = d.split(Pos::new(0, 2));
        assert_eq!(p, Pos::new(1, 0));
        assert_eq!(d.paras[1].props.align, Align::Right);
        assert!(d.paras[0].props.new_page && !d.paras[1].props.new_page);
        // Enter no fim: o parágrafo novo, vazio, digita no mesmo tamanho
        let e = d.end();
        let p = d.split(e);
        assert_eq!(d.props_at(p).size, Size::Huge);
        assert_eq!(d.paras.len(), 3);
    }

    #[test]
    fn delete_across_paragraphs_keeps_first_props() {
        let mut d = doc("ab\ncd");
        d.paras[0].props.align = Align::Center;
        d.backspace(Pos::new(1, 0));
        assert_eq!(d.paras.len(), 1);
        assert_eq!(d.paras[0].props.align, Align::Center);
        assert_eq!(d.text(), "abcd");
    }

    #[test]
    fn navigation_and_words() {
        let d = doc("Era uma vez\num gato");
        assert_eq!(d.prev(Pos::new(1, 0)), Some(Pos::new(0, 11)));
        assert_eq!(d.next(Pos::new(0, 11)), Some(Pos::new(1, 0)));
        assert_eq!(d.next(d.end()), None);
        assert_eq!(d.prev(d.start()), None);
        assert_eq!(d.next_word(Pos::new(0, 0)), Pos::new(0, 4));
        assert_eq!(d.next_word(Pos::new(0, 8)), Pos::new(0, 11));
        assert_eq!(d.next_word(Pos::new(0, 11)), Pos::new(1, 0));
        assert_eq!(d.prev_word(Pos::new(0, 6)), Pos::new(0, 4));
        assert_eq!(d.prev_word(Pos::new(0, 4)), Pos::new(0, 0));
        assert_eq!(d.prev_word(Pos::new(1, 0)), Pos::new(0, 11));
        let w = d.word_at(Pos::new(0, 5));
        assert_eq!(d.text_range(w), "uma");
        assert!(d.word_at(Pos::new(0, 3)).end == Pos::new(0, 3));
        // arrasto do pincel de "a" (em "Era") até "m" (em "uma"): palavras inteiras
        let r = d.expand_to_words(Range::new(Pos::new(0, 2), Pos::new(0, 5)));
        assert_eq!(d.text_range(r), "Era uma");
        // clique (intervalo vazio) dentro de "vez"
        let r = d.expand_to_words(Range::at(Pos::new(0, 9)));
        assert_eq!(d.text_range(r), "vez");
        // atravessando parágrafos
        let r = d.expand_to_words(Range::new(Pos::new(0, 9), Pos::new(1, 1)));
        assert_eq!(d.text_range(r), "vez\num");
    }

    #[test]
    fn props_at_prefers_previous_char() {
        let mut d = doc("ab");
        d.apply_char(
            Range::new(Pos::new(0, 1), Pos::new(0, 2)),
            CharChange::Size(Size::Giant),
        );
        assert_eq!(d.props_at(Pos::new(0, 0)).size, Size::Minimal);
        assert_eq!(d.props_at(Pos::new(0, 2)).size, Size::Giant);
        let mut d = Document::new();
        d.insert_item(Pos::new(0, 0), Item::Rebus(0x25));
        d.paras[0].mark = big();
        assert_eq!(d.props_at(Pos::new(0, 1)), big());
    }

    #[test]
    fn apply_char_ranges_and_marks() {
        let mut d = doc("ab\n\ncd");
        d.apply_char(d.all(), CharChange::Color(Some(CSCH[2])));
        assert!(d.paras.iter().all(|p| p.mark.color == Some(CSCH[2])));
        let mut d = doc("ab\ncd");
        d.apply_char(
            Range::new(Pos::new(0, 1), Pos::new(1, 1)),
            CharChange::Style(TextStyle::Bold),
        );
        let bold: Vec<bool> = d
            .paras
            .iter()
            .flat_map(|p| p.items.iter().map(|i| i.props().unwrap().bold))
            .collect();
        assert_eq!(bold, [false, true, true, false]);
        assert!(d.paras[0].mark.bold && !d.paras[1].mark.bold);
        // parágrafo vazio: o clique muda o \r
        let mut d = Document::new();
        d.apply_char(Range::at(d.start()), CharChange::Size(Size::Large));
        assert_eq!(d.props_at(d.start()).size, Size::Large);
    }

    #[test]
    fn rebus_is_not_text() {
        let mut d = doc("gato");
        let p = d.insert_item(Pos::new(0, 4), Item::Rebus(0x49));
        assert_eq!(p, Pos::new(0, 5));
        assert_eq!(d.text(), "gato");
        assert_eq!(d.paras[0].len(), 5);
        assert_eq!(d.paras[0].spans(), vec![(0, 4), (4, 5)]);
        assert!(d.item(Pos::new(0, 4)).unwrap().is_word());
        assert_eq!(d.text_range(d.word_at(Pos::new(0, 2))), "gato");
    }

    #[test]
    fn slice_and_paste_round_trip() {
        let mut d = doc("Era uma vez\num gato\nazul");
        d.paras[1].props.align = Align::Center;
        d.paras[1].items[0] = Item::Char('u', big());
        let original = d.clone();
        for (a, b) in [
            (Pos::new(0, 4), Pos::new(0, 7)),
            (Pos::new(0, 4), Pos::new(1, 2)),
            (Pos::new(0, 4), Pos::new(2, 2)),
            (Pos::new(0, 0), Pos::new(2, 4)),
            (Pos::new(1, 7), Pos::new(2, 0)),
        ] {
            let r = Range::new(a, b);
            let f = d.slice(r);
            let mut e = d.clone();
            let p = e.delete(r);
            let end = e.insert_fragment(p, &f);
            assert_eq!(end, b, "{a:?}..{b:?}");
            assert_eq!(e, original, "{a:?}..{b:?}");
        }
    }

    #[test]
    fn paste_into_another_document_maps_fonts() {
        let mut a = doc("xy");
        let stencil = a.fonts.ensure("Stencil");
        a.apply_char(a.all(), CharChange::Font(stencil));
        let f = a.slice(a.all());
        assert_eq!(f.fonts, vec![(1, "Stencil".to_string())]);
        let mut b = Document::new();
        b.fonts.ensure("Impact");
        b.insert_fragment(b.start(), &f);
        let ftc = b.fonts.ftc_of("Stencil").unwrap();
        assert_eq!(ftc, 2);
        assert_eq!(b.paras[0].items[0].props().unwrap().font, ftc);
        assert_eq!(f.text(), "xy");
        assert!(!f.is_empty() && Fragment::default().is_empty());
    }

    #[test]
    fn objects_on_pages() {
        let mut d = doc("x");
        let a = d.add_object(FloatObject::new(
            ObjectKind::Sticker,
            ObjRef::pict(2),
            0,
            10,
            10,
            50,
            50,
        ));
        let b = d.add_object(FloatObject::new(
            ObjectKind::Shaped,
            ObjRef::sple(0),
            2,
            30,
            30,
            50,
            50,
        ));
        assert_eq!((a, b), (0, 1));
        assert_eq!(d.object_pages(), 3);
        assert_eq!(d.object_at(0, 40, 40), Some(0));
        assert_eq!(d.object_at(2, 40, 40), Some(1));
        d.objects[0].wrap = Wrap::AllPages;
        assert_eq!(d.object_at(2, 20, 20), Some(0));
        assert_eq!(d.object_at(2, 40, 40), Some(1), "o de cima ganha");
        d.objects[1].sound = Some(ObjRef::snd(7));
        assert!(d.remove_object_sound(1));
        assert!(!d.remove_object_sound(1));
        assert!(d.remove_object(5).is_none());
        assert_eq!(d.remove_object(0).map(|o| o.object), Some(ObjRef::pict(2)));
        d.clear();
        assert!(d.is_empty());
    }
}
