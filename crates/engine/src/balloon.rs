//! Balões de fala do McZee e da Maggie, como no original: caixa com o fundo
//! pontilhado amarelo (padrão 40 do `PAT#`), borda preta de 2 pixels, cantos
//! arredondados nos balões de fala e retos nas dicas e avisos, rabicho curvo
//! até quem fala e o texto formatado do registro `WHAT` (ver [`crate::rich`]).

use crate::Frame;
use crate::assets::Assets;
use crate::rich::{self, Ctx, Laid, Rect};
use formats::fnt::Font;
use formats::what::{
    Item, PIC_GLOSSARY, PIC_OPTION, PIC_OPTION_PICTURE, PIC_TIP, PIC_TIP_MAX, Record,
};
use std::rc::Rc;

const YELLOW: [u8; 3] = [255, 238, 76];
const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];
const GRAY: [u8; 3] = [120, 120, 120];
/// Da borda de fora da caixa até o texto (2 de borda + 3).
pub const PAD: i32 = 5;
/// A caixa tem a largura do registro mais 2.
pub const BOX_EXTRA: i32 = 2;
/// Do pé da última linha até a borda de fora (4 + 2 de borda).
pub const PAD_BOTTOM: i32 = 6;
/// A largura de quebra é a do registro menos 8.
pub const WRAP_LESS: i32 = 8;
const LINE_GAP: i32 = 2;

/// Registro `WHAT` de onde o balão saiu, para desenhá-lo com a formatação;
/// vale enquanto o texto, as opções e as ligações do balão forem os dele.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RichSource {
    pub items: Vec<Item>,
    /// Palavras do glossário: (registro, palavra).
    pub glossary: Vec<(usize, String)>,
    /// Índice do registro no bloco.
    pub own: Option<usize>,
    /// Nome do usuário (troca o `@`).
    pub name: String,
    text: String,
    options: Vec<String>,
    links: Vec<(usize, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Balloon {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub text: String,
    pub options: Vec<String>,
    /// Ligações para dicas (lâmpada `21b`): (registro de destino, texto),
    /// desenhadas entre o texto e as opções.
    pub links: Vec<(usize, String)>,
    /// Figuras do `.PIC` no meio do texto ("clique até ver [ícone]"): cada
    /// uma ocupa, na ordem, uma linha [`FIGURE`] de `text`.
    pub figures: Vec<u16>,
    /// Registro original, com a formatação (negrito, cores, ícones).
    pub rich: Option<Rc<RichSource>>,
    /// Ponta do rabicho (boca de quem fala); `None` = sem rabicho.
    pub tail: Option<(i32, i32)>,
}

/// Linha de `Balloon::text` que é o lugar de uma figura.
pub const FIGURE: char = '\u{1}';

impl Balloon {
    /// Balão a partir de um registro `WHAT` do original: o texto antes da
    /// primeira opção vira `text` (o `@` vira `name`), as opções ficam na ordem
    /// em que aparecem e a largura é a do original.
    /// Como [`Balloon::from_record`] para o registro `k` de um bloco, com as
    /// palavras do glossário (figura `ffff`) trocadas pelo título do registro
    /// que elas apontam ("Dê um *clique duplo* na imagem").
    pub fn from_block(records: &[Record], k: usize, x: i32, y: i32, name: &str) -> Option<Balloon> {
        let r = records.get(k)?;
        let mut glossary = Vec::new();
        let mut plain = r.clone();
        plain.items = r
            .items
            .iter()
            .cloned()
            .map(|i| match i {
                Item::Picture { id, param } if id == PIC_GLOSSARY => {
                    let t = (param & 0x7fff) as usize;
                    let word = records.get(t).map(|r| r.title.trim().to_string());
                    if let Some(w) = &word {
                        glossary.push((t, w.clone()));
                    }
                    Item::Text {
                        text: word.unwrap_or_default(),
                        bold: true,
                        color: 0xf,
                        indent: 0,
                        key: false,
                        signature: false,
                    }
                }
                other => other,
            })
            .collect();
        let mut b = Balloon::from_record_in(&plain, x, y, name, Some(k));
        b.links.retain(|(t, _)| *t < records.len());
        if let Some(src) = b.rich.as_mut() {
            let src = Rc::make_mut(src);
            src.items = r.items.clone();
            src.glossary = glossary;
            src.links = b.links.clone();
        }
        Some(b)
    }

    pub fn from_record(r: &Record, x: i32, y: i32, name: &str) -> Balloon {
        Balloon::from_record_in(r, x, y, name, None)
    }

    /// `own` é o índice do registro no bloco, se conhecido: a lâmpada que
    /// aponta para ele — ou para o registro 0, como a do título dos
    /// registros de dica (`21B0000`) — é só enfeite do título, que fica no
    /// texto; as outras lâmpadas viram ligações (`links`).
    fn from_record_in(r: &Record, x: i32, y: i32, name: &str, own: Option<usize>) -> Balloon {
        let is_title = |param: u16| {
            let t = (param & 0x7fff) as usize;
            t == 0 || Some(t) == own
        };
        let mut text = String::new();
        let mut figures = Vec::new();
        let mut in_options = false;
        // dentro de uma linha de dica (lâmpada): o texto vai para `links`
        let mut in_link = false;
        for i in &r.items {
            match i {
                Item::Picture { id, .. } if *id == PIC_OPTION || *id == PIC_OPTION_PICTURE => {
                    in_options = true;
                }
                Item::Picture { id, param } if *id == PIC_TIP || *id == PIC_TIP_MAX => {
                    in_link = !is_title(*param);
                }
                // figura do `.PIC` no texto (ícone de ferramenta, rolo…)
                Item::Picture { id, .. }
                    if !in_options && !in_link && *id >= 0x1000 && *id != PIC_GLOSSARY =>
                {
                    text.push('\n');
                    text.push(FIGURE);
                    text.push('\n');
                    figures.push(*id);
                }
                Item::Text { text: t, .. } if !in_options && !in_link => text.push_str(t),
                Item::Newline if !in_options => {
                    if in_link {
                        in_link = false;
                    } else {
                        text.push('\n');
                    }
                }
                _ => {}
            }
        }
        let text = text.replace('@', name).trim().to_string();
        let options: Vec<String> = r.options().into_iter().map(|o| o.1).collect();
        let links: Vec<(usize, String)> = r
            .links()
            .into_iter()
            .filter(|(t, s)| !s.is_empty() && !is_title(*t))
            .map(|(t, s)| (t as usize, s))
            .collect();
        let rich = RichSource {
            items: r.items.clone(),
            glossary: Vec::new(),
            own,
            name: name.to_string(),
            text: text.clone(),
            options: options.clone(),
            links: links.clone(),
        };
        Balloon {
            x,
            y,
            w: r.width.max(120) as i32,
            text,
            options,
            links,
            figures,
            rich: Some(Rc::new(rich)),
            tail: None,
        }
    }
}

/// Quebra o texto em linhas que cabem em `width` pixels.
pub fn wrap(font: &Font, text: &str, width: u32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let cand = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if font.width(&cand) <= width || line.is_empty() {
                line = cand;
            } else {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            }
        }
        lines.push(line);
    }
    lines
}

/// Fonte dos balões: "Pablo Graphos" de 8 pontos, se instalada.
pub fn balloon_font(a: &Assets) -> Option<Rc<Font>> {
    a.font(8)
}

/// Altura de uma linha de texto do balão; sem fonte, a 8×8.
pub fn line_height(font: Option<&Font>) -> i32 {
    font.map_or(10, |f| f.height as i32 + LINE_GAP)
}

/// Quebra o texto na fonte, ou só nas quebras de linha sem ela.
pub fn wrap_opt(font: Option<&Font>, text: &str, width: u32) -> Vec<String> {
    match font {
        Some(f) => wrap(f, text, width),
        None => text.split('\n').map(str::to_string).collect(),
    }
}

/// Fundo dos balões: o padrão 40 do `PAT#` do `ARTIST.EXE`, invertido
/// (amarelo onde o bit está ligado), alinhado à tela como um pincel do
/// Windows.
const DOTS: [u8; 8] = [0x09, 0xa0, 0x09, 0x40, 0x12, 0x84, 0x21, 0x88];

/// Cor do fundo pontilhado no pixel (x, y) da tela.
pub fn dot(x: i32, y: i32) -> [u8; 3] {
    if (DOTS[(y & 7) as usize] >> (7 - (x & 7))) & 1 == 1 {
        YELLOW
    } else {
        WHITE
    }
}

/// Borda inteira (linha de cima ou de baixo).
const ALL: i32 = i32::MAX;
/// Canto arredondado, medido no original: por linha a partir do topo (e do
/// pé), até onde é transparente e até onde é borda preta.
const CORNER_TOP: [(i32, i32); 7] = [(5, ALL), (4, ALL), (2, 7), (2, 6), (1, 4), (0, 4), (0, 3)];
const CORNER_BOTTOM: [(i32, i32); 6] = [(4, ALL), (3, ALL), (2, 6), (1, 5), (0, 4), (0, 3)];

/// Caixa do balão em (x, y) com `w`×`h` pixels (borda incluída).
pub fn draw_frame(f: &mut Frame, x: i32, y: i32, w: i32, h: i32, rounded: bool) {
    for dy in 0..h {
        let from_bottom = h - 1 - dy;
        let (out, border) = if rounded && dy < CORNER_TOP.len() as i32 {
            CORNER_TOP[dy as usize]
        } else if rounded && from_bottom < CORNER_BOTTOM.len() as i32 {
            CORNER_BOTTOM[from_bottom as usize]
        } else if dy < 2 || from_bottom < 2 {
            (0, ALL)
        } else {
            (0, 2)
        };
        for dx in 0..w {
            let d = dx.min(w - 1 - dx);
            if d < out {
                continue;
            }
            let c = if border == ALL || d < border {
                BLACK
            } else {
                dot(x + dx, y + dy)
            };
            f.put(x + dx, y + dy, c);
        }
    }
}

/// Caixa do balão de fala (cantos arredondados).
pub fn draw_box(f: &mut Frame, x: i32, y: i32, w: i32, h: i32) {
    draw_frame(f, x, y, w, h, true);
}

/// Caixa das dicas e dos avisos "EPA!" (cantos retos, mesmo fundo).
pub fn draw_box_square(f: &mut Frame, x: i32, y: i32, w: i32, h: i32) {
    draw_frame(f, x, y, w, h, false);
}

/// Rabicho curvo de uma caixa (x, y, w, h) até a ponta `tip`, que fica fora
/// dela: sai do lado mais perto da ponta, com 12 px na base e 8 na ponta,
/// curvado para o lado de quem fala (como o do original), contorno de 2 px
/// e o fundo pontilhado por dentro, abrindo a borda da caixa.
pub fn draw_tail_box(f: &mut Frame, b: (i32, i32, i32, i32), tip: (i32, i32)) {
    let (x, y, w, h) = b;
    // lado e sistema (u para fora, v ao longo do lado)
    let side = if tip.1 < y {
        0
    } else if tip.0 >= x + w {
        1
    } else if tip.0 < x {
        3
    } else if tip.1 >= y + h {
        2
    } else {
        return;
    };
    // (u, v) -> tela
    let to_xy = |u: i32, v: i32| match side {
        0 => (v, y + 1 - u),
        1 => (x + w - 2 + u, v),
        2 => (v, y + h - 2 + u),
        _ => (x + 1 - u, v),
    };
    let (ut, vt, v_lo, v_hi) = match side {
        0 => (y + 1 - tip.1, tip.0, x + 10, x + w - 11),
        1 => (tip.0 - (x + w - 2), tip.1, y + 10, y + h - 11),
        2 => (tip.1 - (y + h - 2), tip.0, x + 10, x + w - 11),
        _ => (x + 1 - tip.0, tip.1, y + 10, y + h - 11),
    };
    if ut < 4 || v_lo > v_hi {
        return;
    }
    // a base fica um pouco atrás da ponta, do lado do centro da caixa
    let center = (v_lo + v_hi) / 2;
    let lean = if vt >= center { 1 } else { -1 };
    let base = (vt - lean * 7).clamp(v_lo, v_hi);
    let (b0, b1) = (base - 6, base + 6);
    let (t0, t1) = (vt - 4, vt + 4);
    let edges = |u: i32| {
        let t = (u.max(0) as f32 / ut as f32).clamp(0.0, 1.0);
        // a borda do lado para onde ele se curva fica reta perto da caixa
        let (e0, e1) = if lean > 0 { (t, t * t) } else { (t * t, t) };
        let l = (b0 as f32 + (t0 - b0) as f32 * e0).round() as i32;
        let r = (b1 as f32 + (t1 - b1) as f32 * e1).round() as i32;
        (l, r)
    };
    // dentro do rabicho (a caixa, u ≤ 0, conta como dentro)
    let inside = |u: i32, v: i32| {
        if u > ut {
            return false;
        }
        let (l, r) = edges(u);
        v >= l && v <= r
    };
    for u in -1..=ut {
        let (l, r) = edges(u);
        for v in l..=r {
            let (px, py) = to_xy(u, v);
            // contorno de 2 px: perto do lado de fora em qualquer direção
            let edge = (-2..=2i32).any(|du| {
                (-2..=2i32)
                    .any(|dv| du.abs() + dv.abs() <= 2 && u + du > -2 && !inside(u + du, v + dv))
            });
            // na borda da caixa (u ≤ 0) o meio se abre para o rabicho
            let c = if edge { BLACK } else { dot(px, py) };
            f.put(px, py, c);
        }
    }
}

/// Rabicho saindo do topo de uma caixa (perto de `x_base`, com a borda de
/// cima em `top`) até a ponta `tip`.
pub fn draw_tail(f: &mut Frame, x_base: i32, top: i32, tip: (i32, i32)) {
    // caixa larga o bastante em volta da base (só o topo importa)
    draw_tail_box(f, (x_base - 40, top, 80, 40), tip);
}

/// Bolinha preta com brilho branco (marca das opções), 10×10 a partir de (x, y).
pub fn draw_bullet(f: &mut Frame, bx: i32, by: i32) {
    for dy in 0..10 {
        for dx in 0..10 {
            let (ex, ey) = (dx as f32 - 4.5, dy as f32 - 4.5);
            if ex * ex + ey * ey <= 22.0 {
                f.put(bx + dx, by + dy, BLACK);
            }
        }
    }
    f.put(bx + 6, by + 3, WHITE);
    f.put(bx + 5, by + 3, WHITE);
    f.put(bx + 6, by + 2, WHITE);
    f.put(bx + 6, by + 4, WHITE);
    f.put(bx + 7, by + 3, WHITE);
}

/// Lâmpada das dicas (14×14 a partir de (x, y)) para quando o `.PIC` falta.
pub fn draw_lamp(f: &mut Frame, x: i32, y: i32) {
    for dy in 0..9 {
        for dx in 0..9 {
            let (ex, ey) = (dx - 4, dy - 4);
            let d = ex * ex + ey * ey;
            if d <= 16 {
                f.put(x + 2 + dx, y + dy, if d >= 11 { BLACK } else { YELLOW });
            }
        }
    }
    f.rect(x + 4, y + 9, 5, 2, BLACK);
    f.rect(x + 5, y + 11, 3, 2, GRAY);
}

/// Texto numa linha: na fonte do balão ou na 8×8.
pub fn draw_line(f: &mut Frame, font: Option<&Font>, x: i32, y: i32, s: &str) {
    match font {
        Some(fo) => {
            f.text_font(fo, x, y, s, BLACK);
        }
        None => f.text(x, y, s, BLACK),
    }
}

/// Itens equivalentes a um texto simples: parágrafos em preto, as figuras
/// nos lugares de [`FIGURE`], as ligações com a gema e as opções em
/// negrito com a bolinha, como os balões do original.
pub fn plain_items(
    text: &str,
    figures: &[u16],
    links: &[(usize, String)],
    options: &[String],
) -> Vec<Item> {
    let t = |s: &str, bold: bool, indent: u8| Item::Text {
        text: s.to_string(),
        bold,
        color: 0xf,
        indent,
        key: false,
        signature: false,
    };
    let mut items = Vec::new();
    let mut figs = figures.iter();
    for line in text.split('\n') {
        if line.trim() == FIGURE.to_string() {
            if let Some(&id) = figs.next() {
                items.push(Item::Picture { id, param: 0 });
                items.push(Item::Newline);
            }
            continue;
        }
        items.push(t(line, false, 0));
        items.push(Item::Newline);
    }
    for (target, s) in links {
        items.push(Item::Format {
            indent: 1,
            center: false,
        });
        items.push(Item::Picture {
            id: PIC_TIP,
            param: *target as u16,
        });
        items.push(t(&format!("  {s}"), false, 1));
        items.push(Item::Newline);
    }
    for (k, o) in options.iter().enumerate() {
        items.push(Item::Format {
            indent: 2,
            center: false,
        });
        items.push(Item::Picture {
            id: PIC_OPTION,
            param: k as u16,
        });
        items.push(t(&format!(" {o}"), true, 2));
        items.push(Item::Newline);
    }
    items
}

/// `small` aparece em `big`, na mesma ordem.
fn is_subsequence(small: &[String], big: &[String]) -> bool {
    let mut it = big.iter();
    small.iter().all(|s| it.any(|b| b == s))
}

/// Os itens sem os parágrafos das opções que não estão em `keep`.
fn drop_options(items: &[Item], keep: &[String]) -> Vec<Item> {
    let mut out = Vec::new();
    let mut para: Vec<Item> = Vec::new();
    let flush = |para: &mut Vec<Item>, out: &mut Vec<Item>| {
        let is_option = para
            .iter()
            .any(|i| matches!(i, Item::Picture { id, .. } if *id == PIC_OPTION));
        let text: String = para
            .iter()
            .filter_map(|i| match i {
                Item::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        if !is_option || keep.iter().any(|k| k == text.trim()) {
            out.append(para);
        } else {
            // guarda a formatação (negrito, recuo) que o parágrafo mudava
            out.extend(para.drain(..).filter(|i| matches!(i, Item::Format { .. })));
        }
        para.clear();
    };
    for i in items {
        let end = matches!(i, Item::Newline);
        para.push(i.clone());
        if end {
            flush(&mut para, &mut out);
        }
    }
    flush(&mut para, &mut out);
    out
}

impl Balloon {
    fn font(a: &Assets) -> Option<Rc<Font>> {
        balloon_font(a)
    }

    /// Itens a desenhar: os do registro se o balão ainda é o dele; senão,
    /// os do texto simples.
    fn items(&self) -> (std::borrow::Cow<'_, [Item]>, Option<&RichSource>) {
        match &self.rich {
            Some(src)
                if src.text == self.text
                    && src.options == self.options
                    && src.links == self.links =>
            {
                (std::borrow::Cow::Borrowed(&src.items[..]), Some(src))
            }
            // a cena tirou algumas opções: tira os parágrafos delas
            Some(src)
                if src.text == self.text
                    && src.links == self.links
                    && is_subsequence(&self.options, &src.options) =>
            {
                (
                    std::borrow::Cow::Owned(drop_options(&src.items, &self.options)),
                    Some(src),
                )
            }
            _ => (
                std::borrow::Cow::Owned(plain_items(
                    &self.text,
                    &self.figures,
                    &self.links,
                    &self.options,
                )),
                None,
            ),
        }
    }

    /// Disposição do texto (coordenadas relativas à origem do texto).
    fn layout(&self, a: &Assets) -> Laid {
        let font = Self::font(a);
        let (items, src) = self.items();
        let glossary = |k: usize| {
            src.and_then(|s| s.glossary.iter().find(|(t, _)| *t == k))
                .map(|(_, w)| w.clone())
        };
        let ctx = Ctx {
            assets: a,
            font: font.as_deref(),
            name: src.map_or("", |s| s.name.as_str()),
            glossary: &glossary,
            own: src.and_then(|s| s.own),
        };
        rich::layout(&items, self.w - WRAP_LESS, &ctx)
    }

    fn origin(&self) -> (i32, i32) {
        (self.x + PAD, self.y + PAD)
    }

    /// Largura da caixa desenhada.
    pub fn box_width(&self) -> i32 {
        self.w + BOX_EXTRA
    }

    pub fn height(&self, a: &Assets) -> i32 {
        PAD + self.layout(a).height + PAD_BOTTOM
    }

    fn abs(&self, r: Rect) -> Rect {
        let (ox, oy) = self.origin();
        (ox + r.0, oy + r.1, r.2, r.3)
    }

    /// Retângulo (x, y, w, h) da ligação `k`.
    pub fn link_rect(&self, a: &Assets, k: usize) -> (i32, i32, i32, i32) {
        let lay = self.layout(a);
        let tips = lay.tips();
        let Some((target, _)) = self.links.get(k) else {
            return (self.x, self.y, 0, 0);
        };
        // a mesma ligação pode aparecer antes: conta as ocorrências
        let nth = self.links[..k].iter().filter(|(t, _)| t == target).count();
        tips.iter()
            .filter(|(_, t)| t == target)
            .nth(nth)
            .or_else(|| tips.get(k))
            .map_or((self.x, self.y, 0, 0), |(r, _)| self.abs(*r))
    }

    /// Retângulo (x, y, w, h) da opção `k` (com metade do espaço entre as
    /// opções em cima e embaixo).
    pub fn option_rect(&self, a: &Assets, k: usize) -> (i32, i32, i32, i32) {
        let lay = self.layout(a);
        lay.options.get(k).map_or((self.x, self.y, 0, 0), |r| {
            let (x, y, w, h) = self.abs(*r);
            (x, y - rich::PARA / 2, w, h + rich::PARA)
        })
    }

    /// Retângulo da palavra `word` dentro da opção `k`, como foi quebrada
    /// em linhas (a palavra do glossário numa opção: "parecer
    /// *tridimensional*").
    pub fn option_word_rect(
        &self,
        a: &Assets,
        k: usize,
        word: &str,
    ) -> Option<(i32, i32, i32, i32)> {
        let font = Self::font(a);
        let lay = self.layout(a);
        let opt = *lay.options.get(k)?;
        lay.runs.iter().find_map(|r| match r {
            rich::Run::Text {
                x, y, text, bold, ..
            } if *y >= opt.1 && *y < opt.1 + opt.3 && text.contains(word) => {
                let at = text.find(word)?;
                let x = x + rich::text_width(font.as_deref(), &text[..at], *bold);
                let w = rich::text_width(font.as_deref(), word, *bold);
                Some(self.abs((x, *y, w, rich::LINE)))
            }
            _ => None,
        })
    }

    /// Índice da opção clicada em (x, y), se houver.
    pub fn hit(&self, a: &Assets, x: i32, y: i32) -> Option<usize> {
        (0..self.options.len()).find(|&k| rich::inside(self.option_rect(a, k), x, y))
    }

    /// Registro da ligação (dica) clicada em (x, y), se houver.
    pub fn link_hit(&self, a: &Assets, x: i32, y: i32) -> Option<usize> {
        (0..self.links.len())
            .find(|&k| rich::inside(self.link_rect(a, k), x, y))
            .map(|k| self.links[k].0)
    }

    pub fn contains(&self, a: &Assets, x: i32, y: i32) -> bool {
        let h = self.height(a);
        x >= self.x && y >= self.y && x < self.x + self.box_width() && y < self.y + h
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let font = Self::font(a);
        let lay = self.layout(a);
        let h = PAD + lay.height + PAD_BOTTOM;
        draw_box(f, self.x, self.y, self.box_width(), h);
        if let Some(tip) = self.tail {
            draw_tail_box(f, (self.x, self.y, self.box_width(), h), tip);
        }
        let (ox, oy) = self.origin();
        rich::draw(
            f,
            a,
            font.as_deref(),
            &lay,
            (ox, oy),
            None,
            &mut |_, _, _| {},
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_breaks_on_width() {
        // fonte sintética: cada glifo tem 3 px (ver formats::fnt tests)
        let mut d = vec![0u8; 0x76 + 3 * 4];
        d[0..2].copy_from_slice(&0x200u16.to_le_bytes());
        d[0x4a..0x4c].copy_from_slice(&2u16.to_le_bytes());
        d[0x58..0x5a].copy_from_slice(&2u16.to_le_bytes());
        d[0x5f] = 0x20;
        d[0x60] = 0x21;
        let bits = d.len() as u16;
        for i in 0..3 {
            d[0x76 + 4 * i..0x78 + 4 * i].copy_from_slice(&3u16.to_le_bytes());
            d[0x78 + 4 * i..0x7a + 4 * i].copy_from_slice(&bits.to_le_bytes());
        }
        d.extend_from_slice(&[0, 0]);
        let f = formats::fnt::parse(&d).unwrap();
        // "aa aa aa" = 8 glifos de 3 px = 24; largura 15 → quebra em duas linhas
        let lines = wrap(&f, "aa aa aa", 15);
        assert_eq!(lines, vec!["aa aa", "aa"]);
        assert_eq!(wrap(&f, "x\ny", 100), vec!["x", "y"]);
    }

    #[test]
    fn options_are_hit_in_order() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        let b = Balloon {
            x: 100,
            y: 50,
            w: 200,
            text: "Oi".into(),
            options: vec!["um".into(), "dois".into()],
            links: Vec::new(),
            figures: Vec::new(),
            ..Default::default()
        };
        let r0 = b.option_rect(&a, 0);
        let r1 = b.option_rect(&a, 1);
        assert!(r1.1 > r0.1);
        assert_eq!(b.hit(&a, r0.0 + 5, r0.1 + 2), Some(0));
        assert_eq!(b.hit(&a, r1.0 + 5, r1.1 + 2), Some(1));
        assert_eq!(b.hit(&a, 0, 0), None);
        assert!(b.contains(&a, 150, 60));
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        assert_eq!(f.pixel(100, 60)[..3], [0, 0, 0]);
    }

    fn tip_record(param: u16, title: &str, link: u16) -> Record {
        let t = |s: &str| Item::Text {
            text: s.into(),
            bold: false,
            color: 0xf,
            indent: 0,
            key: false,
            signature: false,
        };
        Record {
            title: "tip".into(),
            width: 200,
            height: 0,
            style: 0,
            items: vec![
                Item::Picture { id: PIC_TIP, param },
                t(title),
                Item::Newline,
                t("Corpo da dica."),
                Item::Newline,
                Item::Picture {
                    id: PIC_TIP,
                    param: link,
                },
                t("Outra dica"),
                Item::Newline,
            ],
        }
    }

    #[test]
    fn title_lamp_stays_in_the_text_and_other_lamps_become_links() {
        // `21B0000` no título (registro de dica) e uma ligação para o 2
        let recs = vec![
            tip_record(0, "intro", 0x7fff),
            tip_record(0, "Quer mudar a cor?", 2),
            tip_record(2, "Mesmo registro", 1),
        ];
        let b = Balloon::from_block(&recs, 1, 0, 0, "").unwrap();
        assert!(b.text.starts_with("Quer mudar a cor?"), "{}", b.text);
        assert_eq!(b.links, vec![(2, "Outra dica".to_string())]);
        // a lâmpada que aponta para o próprio registro também é título
        let b = Balloon::from_block(&recs, 2, 0, 0, "").unwrap();
        assert!(b.text.starts_with("Mesmo registro"), "{}", b.text);
        assert_eq!(b.links, vec![(1, "Outra dica".to_string())]);
        // destino fora do bloco é descartado
        let b = Balloon::from_block(&recs, 0, 0, 0, "").unwrap();
        assert!(b.links.is_empty());
    }

    #[test]
    fn inline_picture_takes_its_place_in_the_text() {
        // "Então, clique até ver [ícone]" (`4000[65]`)
        let t = |s: &str| Item::Text {
            text: s.into(),
            bold: false,
            color: 0xf,
            indent: 0,
            key: false,
            signature: false,
        };
        let r = Record {
            title: "until".into(),
            width: 200,
            height: 0,
            style: 0,
            items: vec![
                t("Então, clique até ver"),
                Item::Picture {
                    id: 0x105f,
                    param: 0,
                },
                t("depois"),
            ],
        };
        let b = Balloon::from_record(&r, 0, 0, "");
        assert_eq!(b.figures, vec![0x105f]);
        assert_eq!(b.text, format!("Então, clique até ver\n{FIGURE}\ndepois"));
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
    }

    #[test]
    fn missing_picture_takes_no_room() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        let with = Balloon {
            x: 0,
            y: 0,
            w: 200,
            text: format!("ver\n{FIGURE}\ndepois"),
            options: Vec::new(),
            links: Vec::new(),
            figures: vec![0x1021],
            ..Default::default()
        };
        let without = Balloon {
            text: "ver\ndepois".into(),
            figures: Vec::new(),
            ..with.clone()
        };
        assert_eq!(with.height(&a), without.height(&a));
    }
}
