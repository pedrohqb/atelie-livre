//! Texto formatado dos balões (registros `WHAT`) com as medidas do original:
//! negrito (sobreimpressão deslocada 1 px, +1 px por caractere), cores da
//! paleta de 16 cores do programa, recuo (`06 1` recua o parágrafo inteiro,
//! `06 2` deixa o primeiro "token" — marcador, número, bolinha — na margem e
//! pula para o recuo), centralização (`04`), figuras no meio do texto
//! (ícones, gemas, chaves, bolinhas das opções), palavras do glossário com
//! caixa tracejada e caixas de texto.
//!
//! Medidas (capturas do Fine Artist no Wine): linha de texto de 14 px (fonte
//! de 13 + 1), figuras com a própria altura e centralizadas na linha, 6 px
//! entre parágrafos, recuo de 18 px, opção numa vaga de 16×15 com a bolinha
//! no meio. As coordenadas de [`Laid`] são relativas à origem do texto (canto
//! de cima, à esquerda, da primeira linha).

use crate::Frame;
use crate::assets::{Assets, Program};
use formats::fnt::Font;
use formats::what::{
    Item, PIC_GLOSSARY, PIC_HELP, PIC_OPTION, PIC_OPTION_PICTURE, PIC_TIP, PIC_TIP_MAX,
};

pub type Rect = (i32, i32, i32, i32);

/// Altura de uma linha de texto (célula de 13 px da fonte + 1 de entrelinha).
pub const LINE: i32 = 14;
/// Espaço a mais depois de um parágrafo.
pub const PARA: i32 = 6;
/// Recuo dos níveis 1 e 2.
pub const TAB: i32 = 18;
/// Vaga da bolinha de uma opção.
pub const OPTION_W: i32 = 16;
pub const OPTION_H: i32 = 15;
/// Bolinha verde das opções com figura.
pub const BALL_W: i32 = 15;
pub const BALL_H: i32 = 14;
/// Altura de uma caixa de texto (`05 0000xxxx`).
pub const FIELD_H: i32 = 16;
/// Tecla de atalho (chave) e dica da Maggie.
pub const PIC_KEY: u16 = 0x21c;
pub const PIC_MAGGIE: u16 = 0x21e;
/// A palavra do glossário ocupa a caixa tracejada: 2 px antes do texto e
/// 4 depois.
pub const GLOSSARY_EXTRA: i32 = 6;
/// Caixa de texto (o parâmetro é a largura).
pub const PIC_FIELD: u16 = 0;

const BLACK: [u8; 3] = [0, 0, 0];

/// Paleta de 16 cores do programa (`LOGPALETTE` 4 do `ARTIST.EXE`).
const PALETTE16: [[u8; 3]; 16] = [
    [255, 255, 255],
    [0, 0, 0],
    [255, 238, 76],
    [255, 140, 50],
    [185, 101, 36],
    [250, 74, 114],
    [250, 0, 57],
    [136, 136, 230],
    [97, 65, 219],
    [208, 22, 222],
    [147, 44, 163],
    [92, 203, 83],
    [71, 140, 80],
    [140, 140, 140],
    [190, 190, 190],
    [100, 100, 100],
];

/// Cor do código `03`: a numeração é a do Mac (0 branco, `F` preto) e cai
/// na paleta do Windows uma posição adiante — `9` roxo dos títulos, `7`
/// azul do McZee, `4` rosa da Maggie, `8` violeta do Max, `B` verde.
pub fn color(c: u8) -> [u8; 3] {
    match c {
        0 => PALETTE16[0],
        1..=14 => PALETTE16[c as usize + 1],
        _ => BLACK,
    }
}

/// O que um pedaço clicável faz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hot {
    /// Ligação de dica (gema `21b` ou lâmpada do Max `21d`) para o registro.
    Tip(usize),
    /// Outras ligações com ícone: tecla de atalho (`21c`), Maggie (`21e`).
    Link(usize),
    /// Palavra do glossário: registro com a definição.
    Glossary(usize),
}

impl Hot {
    pub fn target(self) -> usize {
        match self {
            Hot::Tip(t) | Hot::Link(t) | Hot::Glossary(t) => t,
        }
    }
}

/// Pedaço já posicionado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Run {
    Text {
        x: i32,
        y: i32,
        text: String,
        bold: bool,
        color: [u8; 3],
        /// Palavra do glossário: caixa tracejada em volta.
        glossary: bool,
    },
    /// Figura do `WHAT` (desenhada por [`crate::help_icons`]); `owner` é a
    /// opção com figura (`22e`) a que ela pertence.
    Picture {
        x: i32,
        y: i32,
        id: u16,
        owner: Option<usize>,
    },
    /// Bolinha de uma opção (`option` = ordem da opção no balão).
    Option {
        x: i32,
        y: i32,
        id: u16,
        option: usize,
    },
    /// Caixa de texto `field`, com `w` pixels.
    Field {
        x: i32,
        y: i32,
        w: i32,
        field: usize,
    },
}

/// Resultado da disposição.
#[derive(Debug, Clone, Default)]
pub struct Laid {
    pub runs: Vec<Run>,
    /// Altura do conteúdo: do topo da primeira linha ao pé da última.
    pub height: i32,
    /// Largura ocupada (a maior linha).
    pub width: i32,
    /// Pedaços clicáveis, na ordem em que aparecem.
    pub hots: Vec<(Rect, Hot)>,
    /// Linhas de cada opção com texto (a ordem de `Record::options`).
    pub options: Vec<Rect>,
    /// Retângulos das caixas de texto, na ordem.
    pub fields: Vec<Rect>,
    /// Programa do balão (de que `.PIC` vêm as figuras), o de [`Ctx`].
    pub program: Program,
}

impl Laid {
    /// Retângulos das ligações de dica, na ordem (uma por gema/lâmpada).
    pub fn tips(&self) -> Vec<(Rect, usize)> {
        self.hots
            .iter()
            .filter_map(|(r, h)| match h {
                Hot::Tip(t) => Some((*r, *t)),
                _ => None,
            })
            .collect()
    }

    /// Pedaço clicável em (x, y), relativo à origem.
    pub fn hot_at(&self, x: i32, y: i32) -> Option<Hot> {
        let at = |glossary: bool| {
            self.hots
                .iter()
                .find(|(r, h)| matches!(h, Hot::Glossary(_)) == glossary && inside(*r, x, y))
                .map(|(_, h)| *h)
        };
        at(true).or_else(|| at(false))
    }
}

pub fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

/// O que a disposição precisa saber além dos itens.
pub struct Ctx<'a> {
    pub assets: &'a Assets,
    /// Programa do balão: de que tabela e de que `.PIC` vêm as figuras
    /// (ver [`crate::help_icons::icon_for`]).
    pub program: Program,
    pub font: Option<&'a Font>,
    /// Troca o `@`.
    pub name: &'a str,
    /// Palavra do glossário do registro `k` (o título dele).
    pub glossary: &'a dyn Fn(usize) -> Option<String>,
    /// Registro mostrado: a gema que aponta para ele (ou para o 0) é só
    /// enfeite do título.
    pub own: Option<usize>,
}

/// Largura de um texto na fonte do balão (ou na 8×8), com o negrito.
pub fn text_width(font: Option<&Font>, s: &str, bold: bool) -> i32 {
    let n = s.chars().count() as i32;
    let w = match font {
        Some(f) => f.width(s) as i32,
        None => 8 * n,
    };
    if bold { w + n } else { w }
}

/// Unidade indivisível de uma linha.
#[derive(Debug, Clone)]
enum Atom {
    Word {
        text: String,
        bold: bool,
        color: [u8; 3],
        glossary: Option<usize>,
    },
    Space {
        w: i32,
    },
    Pic {
        id: u16,
        w: i32,
        h: i32,
        /// Opção com figura (`22e`) a que a figura pertence.
        owner: Option<usize>,
    },
    Option {
        id: u16,
        option: usize,
    },
    Field {
        w: i32,
        field: usize,
    },
}

/// Parágrafo: átomos, com o recuo e o alinhamento do primeiro visível e a
/// ligação de cada átomo.
#[derive(Default)]
struct Para {
    atoms: Vec<(Atom, Option<Hot>, Option<usize>)>,
    indent: Option<u8>,
    center: bool,
    /// Uma figura que o port não tem foi deixada de fora: vazio, o
    /// parágrafo some.
    dropped: bool,
}

fn atom_size(ctx: &Ctx, a: &Atom) -> (i32, i32) {
    match a {
        Atom::Word {
            text,
            bold,
            glossary,
            ..
        } => {
            let w = text_width(ctx.font, text, *bold);
            (
                if glossary.is_some() {
                    w + GLOSSARY_EXTRA
                } else {
                    w
                },
                LINE,
            )
        }
        Atom::Space { w } => (*w, 0),
        Atom::Pic { w, h, .. } => (*w, *h),
        Atom::Option { id, .. } if *id == PIC_OPTION_PICTURE => (BALL_W, BALL_H),
        Atom::Option { .. } => (OPTION_W, OPTION_H),
        Atom::Field { w, .. } => (*w, FIELD_H),
    }
}

/// Tamanho de uma figura do `WHAT`, se o port a tem.
fn picture_size(ctx: &Ctx, id: u16) -> Option<(i32, i32)> {
    crate::help_icons::size(ctx.assets, ctx.program, id)
}

fn push(p: &mut Para, a: Atom, hot: Option<Hot>, opt: Option<usize>, fmt: (u8, bool)) {
    if !matches!(a, Atom::Space { .. }) && p.indent.is_none() {
        p.indent = Some(fmt.0);
        p.center = fmt.1;
    }
    p.atoms.push((a, hot, opt));
}

/// Estado das opções enquanto se lê o registro.
#[derive(Default)]
struct Options {
    /// Opção aberta (índice que ela terá) e se já tem texto.
    open: Option<usize>,
    has_text: bool,
    /// Opções com texto já fechadas.
    count: usize,
    /// Bolinhas verdes (`22e`) já vistas.
    pictures: usize,
}

impl Options {
    fn close(&mut self) {
        if self.open.is_some() && self.has_text {
            self.count += 1;
        }
        self.open = None;
        self.has_text = false;
    }
}

/// Quebra os itens em parágrafos de átomos.
fn paragraphs(items: &[Item], ctx: &Ctx) -> Vec<Para> {
    let mut paras = vec![Para::default()];
    let mut fmt = (0u8, false);
    // ligação em vigor até o fim do parágrafo (gema, chave, Maggie)
    let mut link: Option<Hot> = None;
    let mut opts = Options::default();
    let mut fields = 0usize;
    // opção com figura em vigor (as figuras que seguem são dela)
    let mut owner: Option<usize> = None;
    let space = |bold: bool| text_width(ctx.font, " ", bold);
    for it in items {
        match it {
            Item::Format { indent, center } => fmt = (*indent, *center),
            Item::Newline => {
                link = None;
                owner = None;
                opts.close();
                paras.push(Para::default());
            }
            Item::Text {
                text,
                bold,
                color: c,
                indent,
                signature,
                ..
            } => {
                fmt = (*indent, *signature);
                let p = paras.last_mut().expect("parágrafo");
                let text = text.replace('@', ctx.name);
                let opt = opts.open;
                let word = |w: &mut String| Atom::Word {
                    text: std::mem::take(w),
                    bold: *bold,
                    color: color(*c),
                    glossary: None,
                };
                let mut w = String::new();
                for ch in text.chars() {
                    // o original quebra também no espaço não separável
                    // ("depressa —\u{a0}ninguém"); o Writer o leva para o
                    // começo da linha seguinte (" ninguém")
                    if ch == '\u{a0}' && ctx.program == Program::Writer {
                        if !w.is_empty() {
                            push(p, word(&mut w), link, opt, fmt);
                        }
                        w.push(' ');
                    } else if ch == ' ' || ch == '\u{a0}' {
                        if !w.is_empty() {
                            push(p, word(&mut w), link, opt, fmt);
                        }
                        push(p, Atom::Space { w: space(*bold) }, link, opt, fmt);
                    } else {
                        w.push(ch);
                        // o original também quebra depois do hífen
                        // ("Seleciona-" / "sons")
                        if ch == '-' && w.chars().count() > 1 {
                            push(p, word(&mut w), link, opt, fmt);
                        }
                    }
                }
                if !w.is_empty() {
                    push(p, word(&mut w), link, opt, fmt);
                }
                if opts.open.is_some() && !text.trim().is_empty() {
                    opts.has_text = true;
                }
            }
            Item::Picture { id, param } => {
                let p = paras.last_mut().expect("parágrafo");
                match *id {
                    PIC_GLOSSARY => {
                        let target = (*param & 0x7fff) as usize;
                        let Some(word) = (ctx.glossary)(target).filter(|w| !w.trim().is_empty())
                        else {
                            continue;
                        };
                        let opt = opts.open;
                        // a palavra fica com o estilo do texto em volta
                        let (bold, c) = p
                            .atoms
                            .iter()
                            .rev()
                            .find_map(|(a, _, _)| match a {
                                Atom::Word { bold, color, .. } => Some((*bold, *color)),
                                _ => None,
                            })
                            .unwrap_or((false, BLACK));
                        // a expressão inteira numa caixa só
                        let a = Atom::Word {
                            text: word.split_whitespace().collect::<Vec<_>>().join(" "),
                            bold,
                            color: c,
                            glossary: Some(target),
                        };
                        push(p, a, link, opt, fmt);
                    }
                    PIC_OPTION => {
                        opts.close();
                        link = None;
                        opts.open = Some(opts.count);
                        let a = Atom::Option {
                            id: PIC_OPTION,
                            option: opts.count,
                        };
                        push(p, a, None, opts.open, fmt);
                    }
                    PIC_OPTION_PICTURE => {
                        link = None;
                        let a = Atom::Option {
                            id: PIC_OPTION_PICTURE,
                            option: opts.pictures,
                        };
                        owner = Some(opts.pictures);
                        opts.pictures += 1;
                        push(p, a, None, None, fmt);
                    }
                    PIC_FIELD => {
                        let a = Atom::Field {
                            w: (*param as i32).max(40),
                            field: fields,
                        };
                        fields += 1;
                        push(p, a, None, None, fmt);
                    }
                    id => {
                        if matches!(id, PIC_TIP | PIC_TIP_MAX | PIC_KEY | PIC_MAGGIE) {
                            let target = (*param & 0x7fff) as usize;
                            let title = target == 0 || Some(target) == ctx.own;
                            link = (!title).then_some(if matches!(id, PIC_TIP | PIC_TIP_MAX) {
                                Hot::Tip(target)
                            } else {
                                Hot::Link(target)
                            });
                        } else if id == PIC_HELP {
                            link = None;
                        }
                        let Some((w, h)) = picture_size(ctx, id) else {
                            p.dropped = true;
                            continue;
                        };
                        push(p, Atom::Pic { id, w, h, owner }, link, opts.open, fmt);
                    }
                }
            }
        }
    }
    paras
}

fn union(r: Option<Rect>, s: Rect) -> Rect {
    match r {
        None => s,
        Some(r) => {
            let (x0, y0) = (r.0.min(s.0), r.1.min(s.1));
            let (x1, y1) = ((r.0 + r.2).max(s.0 + s.2), (r.1 + r.3).max(s.1 + s.3));
            (x0, y0, x1 - x0, y1 - y0)
        }
    }
}

fn is_space(a: &Atom) -> bool {
    matches!(a, Atom::Space { .. })
}

/// Quebra um parágrafo em linhas: (átomo, x) de cada uma.
fn break_lines(para: &Para, width: i32, ctx: &Ctx) -> Vec<Vec<(usize, i32)>> {
    let indent = para.indent.unwrap_or(0);
    let cont_x = if indent > 0 { TAB } else { 0 };
    let mut lines: Vec<Vec<(usize, i32)>> = vec![Vec::new()];
    let mut x = if indent == 1 { TAB } else { 0 };
    // recuo pendente (`06 2`): o primeiro espaço depois do primeiro token
    // pula para o recuo
    let mut hanging = indent == 2;
    let mut seen = false;
    for (k, (atom, ..)) in para.atoms.iter().enumerate() {
        let (w, _) = atom_size(ctx, atom);
        if is_space(atom) {
            if !seen && !hanging {
                // espaços no começo do parágrafo contam
                x += w;
                continue;
            }
            if hanging {
                // o espaço anda e, se não chegou ao recuo, pula até ele
                // ("• Clique" vai a 18; a bolinha de 16 + espaço passa)
                hanging = false;
                if x + w < TAB {
                    x = TAB;
                    continue;
                }
            }
            let line = lines.last_mut().expect("linha");
            if line.is_empty() {
                // espaço no começo de uma linha quebrada some
                continue;
            }
            line.push((k, x));
            x += w;
            continue;
        }
        seen = true;
        let line = lines.last_mut().expect("linha");
        let visible = line.iter().any(|(j, _)| !is_space(&para.atoms[*j].0));
        if x + w > width && visible {
            while line
                .last()
                .is_some_and(|(j, _)| is_space(&para.atoms[*j].0))
            {
                line.pop();
            }
            hanging = false;
            lines.push(Vec::new());
            x = cont_x;
        }
        lines.last_mut().expect("linha").push((k, x));
        x += w;
    }
    lines
}

/// Dispõe os itens de um registro numa largura de `width` pixels.
pub fn layout(items: &[Item], width: i32, ctx: &Ctx) -> Laid {
    let paras = paragraphs(items, ctx);
    // tira os parágrafos vazios do fim (a quebra de linha final do registro)
    let mut last = paras.len();
    while last > 0 && paras[last - 1].atoms.iter().all(|(a, ..)| is_space(a)) {
        last -= 1;
    }
    let mut out = Laid {
        program: ctx.program,
        ..Laid::default()
    };
    let mut y = 0;
    let mut opt_rects: Vec<Option<Rect>> = Vec::new();
    let blank = |p: &Para| p.atoms.iter().all(|(a, ..)| is_space(a));
    for (pi, para) in paras[..last]
        .iter()
        .filter(|p| !(p.dropped && blank(p)))
        .enumerate()
    {
        if pi > 0 {
            y += PARA;
        }
        for line in break_lines(para, width, ctx) {
            let size = |j: usize| atom_size(ctx, &para.atoms[j].0);
            let lh = line
                .iter()
                .map(|(j, _)| size(*j).1)
                .max()
                .unwrap_or(LINE)
                .max(LINE);
            let right_edge = line.iter().map(|(j, x)| x + size(*j).0).max().unwrap_or(0);
            let shift = if para.center {
                (width - right_edge).max(0) / 2
            } else {
                0
            };
            out.width = out.width.max(right_edge + shift);
            for &(j, ax) in &line {
                let (atom, hot, opt) = &para.atoms[j];
                let (w, h) = size(j);
                let ax = ax + shift;
                let ay = y + (lh - h) / 2;
                let cell = (ax, y, w, lh);
                match atom {
                    Atom::Word {
                        text,
                        bold,
                        color,
                        glossary,
                    } => {
                        let tx = if glossary.is_some() { ax + 2 } else { ax };
                        out.runs.push(Run::Text {
                            x: tx,
                            y: ay,
                            text: text.clone(),
                            bold: *bold,
                            color: *color,
                            glossary: glossary.is_some(),
                        });
                        if let Some(t) = glossary {
                            out.hots.push(((ax, ay, w, LINE - 1), Hot::Glossary(*t)));
                        }
                    }
                    Atom::Space { .. } => {}
                    Atom::Pic { id, owner, .. } => out.runs.push(Run::Picture {
                        x: ax,
                        y: ay,
                        id: *id,
                        owner: *owner,
                    }),
                    Atom::Option { id, option } => out.runs.push(Run::Option {
                        x: ax,
                        y: ay,
                        id: *id,
                        option: *option,
                    }),
                    Atom::Field { w, field } => {
                        out.runs.push(Run::Field {
                            x: ax,
                            y: ay,
                            w: *w,
                            field: *field,
                        });
                        out.fields.push((ax, ay, *w, FIELD_H));
                    }
                }
                if let Some(h) = hot {
                    // a ligação ocupa a linha, do ícone ao fim do texto
                    match out.hots.iter_mut().rev().find(|(_, k)| k == h) {
                        Some((r, _)) if r.1 + r.3 >= y => *r = union(Some(*r), cell),
                        _ => out.hots.push((cell, *h)),
                    }
                }
                if let Some(o) = opt {
                    if opt_rects.len() <= *o {
                        opt_rects.resize(*o + 1, None);
                    }
                    opt_rects[*o] = Some(union(opt_rects[*o], cell));
                }
            }
            y += lh;
        }
    }
    out.height = y;
    // a opção vale a largura toda do balão (como o clique do original)
    out.options = opt_rects
        .into_iter()
        .flatten()
        .map(|r| (0, r.1, width, r.3))
        .collect();
    out
}

/// Desenha um texto na fonte do balão (ou na 8×8), com negrito.
pub fn draw_text(
    f: &mut Frame,
    font: Option<&Font>,
    x: i32,
    y: i32,
    s: &str,
    bold: bool,
    c: [u8; 3],
) {
    match font {
        Some(fo) => {
            if bold {
                // sobreimpressão deslocada, cada caractere 1 px mais largo
                let mut gx = x;
                for ch in s.chars() {
                    let mut b = [0u8; 4];
                    let one = ch.encode_utf8(&mut b);
                    let w = f.text_font(fo, gx, y, one, c);
                    f.text_font(fo, gx + 1, y, one, c);
                    gx += w + 1;
                }
            } else {
                f.text_font(fo, x, y, s, c);
            }
        }
        None => {
            f.text(x, y, s, c);
            if bold {
                f.text(x + 1, y, s, c);
            }
        }
    }
}

/// Caixa tracejada das palavras do glossário: traços de 3 px com 3 de
/// intervalo, alinhados à tela (medidos no original).
pub fn draw_dashed(f: &mut Frame, x: i32, y: i32, w: i32, h: i32) {
    let on = |v: i32, phase: i32| (v + phase).rem_euclid(6) < 3;
    for dx in 0..w {
        if on(x + dx, 5) {
            f.put(x + dx, y, BLACK);
            f.put(x + dx, y + h - 1, BLACK);
        }
    }
    for dy in 0..h {
        if on(y + dy, 3) {
            f.put(x, y + dy, BLACK);
        }
        if on(y + dy, 2) {
            f.put(x + w - 1, y + dy, BLACK);
        }
    }
}

/// Desenha os pedaços de `lay` com a origem em `(ox, oy)`. `field` desenha
/// uma caixa de texto (o conteúdo é de quem a tem).
pub fn draw(
    f: &mut Frame,
    a: &Assets,
    font: Option<&Font>,
    lay: &Laid,
    (ox, oy): (i32, i32),
    checked: Option<usize>,
    field: &mut dyn FnMut(&mut Frame, usize, Rect),
) {
    for run in &lay.runs {
        match run {
            Run::Text {
                x,
                y,
                text,
                bold,
                color,
                glossary,
            } => {
                draw_text(f, font, ox + x, oy + y, text, *bold, *color);
                if *glossary {
                    let w = text_width(font, text, *bold);
                    draw_dashed(f, ox + x - 2, oy + y, w + GLOSSARY_EXTRA, LINE - 1);
                }
            }
            Run::Picture { x, y, id, owner } => {
                let p = lay.program;
                let dim =
                    crate::help_icons::dimmed(p, *id) && (owner.is_none() || *owner != checked);
                crate::help_icons::draw_with(f, a, p, *id, ox + x, oy + y, dim)
            }
            Run::Option { x, y, id, option } => {
                let id = if *id == PIC_OPTION_PICTURE && checked == Some(*option) {
                    // bolinha verde marcada
                    0x22f
                } else {
                    *id
                };
                draw_option(f, a, lay.program, id, ox + x, oy + y);
            }
            Run::Field { x, y, w, field: k } => field(f, *k, (ox + x, oy + y, *w, FIELD_H)),
        }
    }
}

/// Bolinha de opção na vaga de 16×15: a preta (`22c`) no meio; a verde
/// (`22e`, 15×14) e a verde marcada (`22f`) no canto.
fn draw_option(f: &mut Frame, a: &Assets, p: Program, id: u16, x: i32, y: i32) {
    use crate::help_icons::{draw, size};
    match id {
        0x22f => crate::text_balloon::blit_pic_in(f, a, p, 0x040d, x, y, false),
        PIC_OPTION_PICTURE => draw(f, a, p, id, x, y),
        _ => match size(a, p, id) {
            Some((w, h)) => draw(f, a, p, id, x + (OPTION_W - w) / 2, y + (OPTION_H - h) / 2),
            None => crate::balloon::draw_bullet(f, x + 3, y + 2),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    fn text(t: &str) -> Item {
        Item::Text {
            text: t.into(),
            bold: false,
            color: 0xf,
            indent: 0,
            key: false,
            signature: false,
        }
    }

    /// Primeiro texto de cada linha (sem fonte: 8 px por caractere).
    fn line_starts(program: Program) -> Vec<String> {
        let (_d, a) = empty_assets();
        let glossary = |_: usize| None;
        let ctx = Ctx {
            assets: &a,
            program,
            font: None,
            name: "",
            glossary: &glossary,
            own: None,
        };
        // "aaaa —" cabe em 64 px, "aaaa — bbbb" não
        let lay = layout(&[text("aaaa —\u{a0}bbbb")], 64, &ctx);
        let mut starts = Vec::new();
        let mut last_y = None;
        for r in &lay.runs {
            if let Run::Text { y, text, .. } = r
                && last_y != Some(*y)
            {
                starts.push(text.clone());
                last_y = Some(*y);
            }
        }
        starts
    }

    #[test]
    fn non_breaking_space_breaks_and_the_writer_keeps_it() {
        // no Fine Artist o espaço some na quebra; no Writer ele abre a linha
        assert_eq!(line_starts(Program::Artist), ["aaaa", "bbbb"]);
        assert_eq!(line_starts(Program::Writer), ["aaaa", " bbbb"]);
    }
}
