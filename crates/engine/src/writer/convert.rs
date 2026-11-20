//! Conversão entre o documento do Escritório ([`doc::Document`]) e o
//! arquivo `.MAX` ([`formats::max::Document`]), campo a campo (tarefas E-1,
//! E-2 e W22; ver `docs/telas/creative-writer-modelo.md`).
//!
//! O modelo guarda o que o editor mostra e muda. O resto do arquivo
//! (cópias dos objetos `PICT`/`SPLE`/`SND `/`BDR `, sons em palavras,
//! impressora, miniatura, `ADD `, flags das folhas e molduras que o modelo não
//! representa, como os obstáculos dos fundos) fica num [`MaxExtra`], que a
//! gravação devolve ao arquivo como veio.
//!
//! Texto: cada `\r` fecha um parágrafo. O original grava um `\r` a mais no
//! fim ("abc" → `"abc\r\r"`); a leitura tira esse `\r` quando o texto
//! termina com dois, e a gravação sempre o põe.

use super::doc::{
    self, Align, CharProps, Document, DopProps, Effect, FontEntry, FontTable, Indent, Item,
    PageNumber, ParaProps, Paragraph, Pos, Range, Size,
};
use super::object::{FloatObject, ObjRef, ObjectKind, Outline, Wrap};
use formats::Tag;
use formats::chnk::OwnedChunk;
use formats::max::{self, FrameKind, MacColor};
use formats::twpb;

/// O que o `.MAX` tem e o modelo do documento não guarda.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MaxExtra {
    /// Cópias dos objetos (`PICT`, `SPLE`, `SND `, `BDR ` e outros), na
    /// ordem do arquivo.
    pub objects: Vec<OwnedChunk>,
    /// Sons em palavras (`SNDT`; W11). Gravados como vieram, menos os que
    /// passarem do fim do texto.
    pub word_sounds: Vec<max::WordSound>,
    /// Impressora (`PRNT`); `None` = a padrão.
    pub printer: Option<Vec<u8>>,
    /// Miniatura (`THMB`) como `PKBM` cru.
    pub thumbnail: Option<Vec<u8>>,
    /// Título, autor, projeto, andar, senha.
    pub add: max::Add,
    /// Flags de cada folha (`PGD `; os cartões marcam capa e miolo).
    pub page_flags: Vec<u16>,
    /// Sprms do `DOP ` sem campo no modelo (o fundo `0x56`…).
    pub dop_extra: Vec<max::Sprm>,
    /// Molduras que o modelo não representa (obstáculos dos fundos, tipo
    /// 6, em todas as folhas), regravadas como vieram, depois das outras.
    pub frames: Vec<max::Frame>,
    /// Sprms sem campo das molduras representadas, na ordem de
    /// [`Document::objects`] (o editor mantém a lista em ordem ao apagar).
    pub frame_extra: Vec<(ObjRef, Vec<max::Sprm>)>,
}

impl MaxExtra {
    /// Para um documento novo: impressora padrão e `ADD ` de documento novo.
    pub fn new() -> MaxExtra {
        MaxExtra {
            printer: Some(max::DEFAULT_PRINTER.to_vec()),
            ..MaxExtra::default()
        }
    }

    /// O chunk do objeto citado.
    pub fn object(&self, r: ObjRef) -> Option<&OwnedChunk> {
        self.objects
            .iter()
            .find(|c| c.tag.0 == r.tag && c.id == r.id)
    }

    /// Primeiro número livre para um chunk `tag`.
    pub fn free_id(&self, tag: [u8; 4]) -> u16 {
        self.objects
            .iter()
            .filter(|c| c.tag.0 == tag)
            .map(|c| c.id + 1)
            .max()
            .unwrap_or(0)
    }

    /// Guarda uma cópia do chunk com um número livre e devolve a referência.
    pub fn add_object(&mut self, mut c: OwnedChunk) -> ObjRef {
        let id = self.free_id(c.tag.0);
        c.id = id;
        let r = ObjRef { tag: c.tag.0, id };
        self.objects.push(c);
        r
    }
}

fn to_ref(r: max::ObjRef) -> ObjRef {
    ObjRef {
        tag: r.tag.0,
        id: r.id,
    }
}

fn from_ref(r: ObjRef) -> max::ObjRef {
    max::ObjRef {
        tag: Tag(r.tag),
        id: r.id,
    }
}

/// Byte do caractere-âncora (o texto do `.MAX` é Windows-1252 sem perdas).
fn char_byte(c: char) -> u8 {
    match c as u32 {
        v @ (0..=0x7f | 0x81 | 0x8d | 0x8f | 0x90 | 0x9d) => v as u8,
        _ => formats::cp1252::encode_char(c).unwrap_or(b'?'),
    }
}

/// Caractere do byte, como o leitor do `.MAX` decodifica.
fn byte_char(b: u8) -> char {
    match b {
        0..=0x7f | 0x81 | 0x8d | 0x8f | 0x90 | 0x9d => char::from(b),
        _ => formats::cp1252::decode(&[b]).chars().next().unwrap_or('?'),
    }
}

fn char_props(p: &max::CharProps) -> CharProps {
    CharProps {
        font: p.font.unwrap_or(0),
        size: p.size.map_or(Size::default(), Size::from_pt),
        color: p.text_color(),
        highlight: p.highlight_color(),
        bold: p.bold,
        italic: p.italic,
        underline: p.underline,
        effect: p.effect.map(|e| Effect::from_code(e.code())),
        sound: None,
    }
}

fn max_char_props(p: &CharProps) -> max::CharProps {
    let mut m = max::CharProps {
        bold: p.bold,
        italic: p.italic,
        underline: p.underline,
        effect: p.effect.map(|e| max::Effect::from_code(e.code())),
        font: (p.font != 0).then_some(p.font),
        size: (p.size != Size::default()).then(|| p.size.pt()),
        ..max::CharProps::default()
    };
    m.set_text_color(p.color);
    m.set_highlight_color(p.highlight);
    m
}

fn anchor_props() -> max::CharProps {
    max::CharProps {
        anchor: true,
        ..max::CharProps::default()
    }
}

fn para_props(p: &max::ParaProps) -> ParaProps {
    ParaProps {
        align: match p.align {
            max::Align::Left => Align::Left,
            max::Align::Right => Align::Right,
            max::Align::Center => Align::Center,
            max::Align::Justify => Align::Justify,
        },
        indent: match p.indent {
            max::Indent::None => Indent::None,
            max::Indent::FirstLine => Indent::FirstLine,
            max::Indent::Hanging => Indent::Hanging,
            max::Indent::All => Indent::All,
        },
        line_spacing: p.line_spacing,
        space_after: p.space_after,
        drop_cap: p.drop_cap,
        raised_cap: p.raised_cap,
        new_page: p.new_page,
        bullets: p.bullets,
    }
}

fn max_para_props(p: &ParaProps) -> max::ParaProps {
    max::ParaProps {
        align: match p.align {
            Align::Left => max::Align::Left,
            Align::Right => max::Align::Right,
            Align::Center => max::Align::Center,
            Align::Justify => max::Align::Justify,
        },
        indent: match p.indent {
            Indent::None => max::Indent::None,
            Indent::FirstLine => max::Indent::FirstLine,
            Indent::Hanging => max::Indent::Hanging,
            Indent::All => max::Indent::All,
        },
        line_spacing: p.line_spacing,
        space_after: p.space_after,
        drop_cap: p.drop_cap,
        raised_cap: p.raised_cap,
        new_page: p.new_page,
        bullets: p.bullets,
        extra: Vec::new(),
    }
}

fn dop(d: &max::DopProps) -> (DopProps, Vec<max::Sprm>) {
    let out = DopProps {
        page_width: d.page_width,
        page_height: d.page_height,
        margin_top: d.margin_top,
        margin_left: d.margin_left,
        margin_bottom: d.margin_bottom,
        margin_right: d.margin_right,
        column_gap: d.column_gap,
        field_49: d.field_49,
        columns: d.columns.max(1),
        newspaper: d.newspaper,
        banner: d.banner,
        page_number: PageNumber::from_code(d.page_number_style),
        page_number_rect: d.page_number_rect,
        border: d.border.map(to_ref),
        border_alt: d.border_alt.map(to_ref),
        border_inner: d.border_inner,
        border_outer: d.border_outer,
        background: super::borders::background_ref(d).map(to_ref),
        obstacles: Vec::new(),
    };
    (out, d.extra.clone())
}

/// Molduras de fundo (tipo 6, em todas as folhas) que desviam o texto.
fn is_obstacle(f: &max::Frame) -> bool {
    f.kind == FrameKind::Other(formats::bor::FRAME_OBSTACLE) && f.page == u16::MAX
}

/// Sprms sem campo do `DOP`, com o `0x56` (fundo) no lugar dele, posto ou
/// tirado conforme o modelo.
fn dop_extra_with_background(d: &DopProps, extra: &[max::Sprm]) -> Vec<max::Sprm> {
    let code = super::borders::SPRM_BACKGROUND;
    let arg = d.background.map(|r| {
        let mut a = r.tag.to_vec();
        a.extend_from_slice(&r.id.to_le_bytes());
        a
    });
    let mut out: Vec<max::Sprm> = Vec::with_capacity(extra.len() + 1);
    let mut placed = false;
    for s in extra {
        if s.code == code {
            if let (Some(a), false) = (&arg, placed) {
                out.push(max::Sprm {
                    code,
                    arg: a.clone(),
                });
                placed = true;
            }
        } else {
            out.push(s.clone());
        }
    }
    if let (Some(a), false) = (arg, placed) {
        out.push(max::Sprm { code, arg: a });
    }
    out
}

fn max_dop(d: &DopProps, extra: &[max::Sprm]) -> max::DopProps {
    max::DopProps {
        page_width: d.page_width,
        page_height: d.page_height,
        margin_top: d.margin_top,
        margin_left: d.margin_left,
        margin_bottom: d.margin_bottom,
        margin_right: d.margin_right,
        column_gap: d.column_gap,
        field_49: d.field_49,
        columns: d.columns.max(1),
        newspaper: d.newspaper,
        page_number_style: d.page_number.code(),
        page_number_rect: d.page_number_rect,
        border_outer: d.border_outer,
        border_inner: d.border_inner,
        border: d.border.map(from_ref),
        border_alt: d.border_alt.map(from_ref),
        banner: d.banner,
        extra: dop_extra_with_background(d, extra),
    }
}

/// Contorno do modelo a partir do `TWPB` (cuja borda direita é a margem
/// medida da direita).
pub fn outline_of(t: &twpb::Twpb) -> Outline {
    let w = t.width as i16;
    Outline {
        width: t.width,
        height: t.height,
        left: t.left.clone(),
        right: t.right.iter().map(|&(m, y)| (w - m, y)).collect(),
    }
}

fn has_part(c: &OwnedChunk, tag: &[u8; 4]) -> bool {
    c.parts.iter().any(|p| p.tag.0 == *tag)
}

/// O objeto do modelo para uma moldura; `None` se o modelo não a representa.
fn frame_object(f: &max::Frame, objects: &[OwnedChunk]) -> Option<FloatObject> {
    let object = to_ref(f.object?);
    let chunk = objects
        .iter()
        .find(|c| c.tag.0 == object.tag && c.id == object.id);
    let kind = match f.kind {
        FrameKind::Splot => ObjectKind::Shaped,
        FrameKind::PageFrame => ObjectKind::PageFrame,
        // o desenho do cavalete não tem a prévia PKBM
        FrameKind::Picture if chunk.is_some_and(|c| !has_part(c, b"PKBM")) => ObjectKind::Drawing,
        FrameKind::Picture => ObjectKind::Sticker,
        FrameKind::Other(_) => return None,
    };
    if f.page == u16::MAX {
        return None;
    }
    let outline = chunk
        .and_then(|c| c.parts.iter().find(|p| p.tag.0 == *b"TWPB"))
        .and_then(|p| twpb::read(&p.data).ok())
        .map(|t| outline_of(&t));
    Some(FloatObject {
        kind,
        object,
        page: f.page,
        x: f.x,
        y: f.y,
        width: f.width,
        height: f.height,
        orig_width: if f.orig_width == 0 {
            f.width
        } else {
            f.orig_width
        },
        orig_height: if f.orig_height == 0 {
            f.height
        } else {
            f.orig_height
        },
        wrap: Wrap::from_code(f.wrap),
        sound: f.sound.map(to_ref),
        border: None,
        outline,
    })
}

fn max_frame(o: &FloatObject, extra: &[max::Sprm]) -> max::Frame {
    let kind = match o.kind {
        ObjectKind::Shaped => FrameKind::Splot,
        ObjectKind::Sticker | ObjectKind::Drawing => FrameKind::Picture,
        ObjectKind::PageFrame => FrameKind::PageFrame,
    };
    let mut f = max::Frame::new(kind, from_ref(o.object), o.x, o.y, o.width, o.height);
    f.page = o.page;
    f.wrap = o.wrap.code();
    f.orig_width = o.orig_width;
    f.orig_height = o.orig_height;
    f.sound = o.sound.map(from_ref);
    f.extra = extra.to_vec();
    f
}

/// Documento do editor a partir do arquivo lido.
pub fn from_max(m: &max::Document) -> (Document, MaxExtra) {
    let mut text: Vec<char> = m.text.chars().collect();
    if text.ends_with(&['\r', '\r']) {
        text.pop();
    }
    let mut paras = Vec::new();
    let mut cur = Paragraph::default();
    let mut start = 0u32;
    for (i, &c) in text.iter().enumerate() {
        let cp = i as u32;
        let props = m.char_props_at(cp);
        if c == '\r' {
            cur.mark = char_props(props);
            cur.props = para_props(m.para_props_at(cp));
            paras.push(std::mem::take(&mut cur));
            start = cp + 1;
        } else if props.anchor {
            cur.items.push(Item::Rebus(char_byte(c)));
        } else {
            cur.items.push(Item::Char(c, char_props(props)));
        }
    }
    if !cur.items.is_empty() || paras.is_empty() {
        // resto sem `\r`: as propriedades do último trecho
        cur.props = para_props(m.para_props_at(start));
        cur.mark = char_props(m.char_props_at(start));
        paras.push(cur);
    }
    let fonts = FontTable {
        entries: if m.fonts.is_empty() {
            FontTable::default().entries
        } else {
            m.fonts
                .iter()
                .map(|f| FontEntry {
                    ftc: f.ftc,
                    name: f.name.clone(),
                    family: f.family,
                })
                .collect()
        },
    };
    let (mut dop, dop_extra) = dop(&m.dop);
    let mut objects = Vec::new();
    let mut frame_extra = Vec::new();
    let mut frames = Vec::new();
    for f in &m.frames {
        match frame_object(f, &m.objects) {
            Some(o) => {
                frame_extra.push((o.object, f.extra.clone()));
                objects.push(o);
            }
            None if is_obstacle(f) => dop.obstacles.push(f.clone()),
            None => frames.push(f.clone()),
        }
    }
    let mut doc = Document {
        paras,
        objects,
        dop,
        fonts,
    };
    // sons em palavras: cada elemento do `SNDT` marca a palavra que começa
    // no seu cp (o fim gravado é só o começo do seguinte)
    for w in &m.word_sounds {
        let at = pos_of(&doc, w.start);
        doc.set_word_sound(at, Some(to_ref(w.sound)));
    }
    let extra = MaxExtra {
        objects: m.objects.clone(),
        word_sounds: m.word_sounds.clone(),
        printer: m.printer.clone(),
        thumbnail: m.thumbnail.clone(),
        add: m.add.clone(),
        page_flags: m.pages.iter().map(|p| p.flags).collect(),
        dop_extra,
        frames,
        frame_extra,
    };
    (doc, extra)
}

/// Posição do modelo → cp do texto do arquivo (cada parágrafo ocupa os
/// seus itens e o `\r`).
pub fn cp_of(doc: &Document, p: Pos) -> u32 {
    let before: usize = doc.paras[..p.para.min(doc.paras.len())]
        .iter()
        .map(|q| q.items.len() + 1)
        .sum();
    (before + p.index) as u32
}

/// cp do arquivo → posição do modelo (o `\r` final a mais vira o fim).
pub fn pos_of(doc: &Document, cp: u32) -> Pos {
    let mut left = cp as usize;
    for (k, p) in doc.paras.iter().enumerate() {
        if left <= p.items.len() {
            return Pos::new(k, left);
        }
        left -= p.items.len() + 1;
    }
    doc.end()
}

/// Arquivo para gravar o documento: `pages` são os começos das folhas da
/// diagramação (`Layout::page_starts`), `sel` a seleção a guardar.
pub fn to_max(
    doc: &Document,
    extra: &MaxExtra,
    pages: &[Pos],
    sel: Option<Range>,
) -> max::Document {
    let mut text = String::new();
    let mut char_runs: Vec<max::Run<max::CharProps>> = Vec::new();
    let mut para_runs: Vec<max::Run<max::ParaProps>> = Vec::new();
    let mut cp = 0u32;
    let mut push_char = |text: &mut String, c: char, props: max::CharProps, cp: &mut u32| {
        text.push(c);
        match char_runs.last_mut() {
            Some(r) if r.props == props && r.end == *cp => r.end += 1,
            _ => char_runs.push(max::Run {
                start: *cp,
                end: *cp + 1,
                props,
            }),
        }
        *cp += 1;
    };
    for p in &doc.paras {
        let start = cp;
        for it in &p.items {
            match *it {
                Item::Char(c, ref props) => push_char(&mut text, c, max_char_props(props), &mut cp),
                Item::Rebus(b) => push_char(&mut text, byte_char(b), anchor_props(), &mut cp),
            }
        }
        push_char(&mut text, '\r', max_char_props(&p.mark), &mut cp);
        para_runs.push(max::Run {
            start,
            end: cp,
            props: max_para_props(&p.props),
        });
    }
    // o `\r` a mais do fim, com as propriedades do último parágrafo
    let last = doc.paras.last().expect("ao menos um parágrafo");
    let start = cp;
    push_char(&mut text, '\r', max_char_props(&last.mark), &mut cp);
    para_runs.push(max::Run {
        start,
        end: cp,
        props: max_para_props(&last.props),
    });
    let len = cp;
    let mut starts: Vec<u32> = pages.iter().map(|&p| cp_of(doc, p).min(len)).collect();
    if starts.is_empty() {
        starts.push(0);
    }
    starts[0] = 0;
    let pages = starts
        .iter()
        .enumerate()
        .map(|(k, &s)| max::Page {
            start: s,
            end: starts.get(k + 1).copied().unwrap_or(len).max(s),
            flags: extra.page_flags.get(k).copied().unwrap_or(0),
        })
        .collect();
    let mut frames: Vec<max::Frame> = doc
        .objects
        .iter()
        .enumerate()
        .map(|(k, o)| {
            let ex = extra
                .frame_extra
                .get(k)
                .filter(|(r, _)| *r == o.object)
                .map_or(&[][..], |(_, e)| &e[..]);
            max_frame(o, ex)
        })
        .collect();
    frames.extend(extra.frames.iter().cloned());
    frames.extend(doc.dop.obstacles.iter().cloned());
    let selection = sel.map(|r| (cp_of(doc, r.start).min(len), cp_of(doc, r.end).min(len)));
    max::Document {
        text,
        char_runs,
        para_runs,
        fonts: doc
            .fonts
            .entries
            .iter()
            .map(|f| max::Font::new(&f.name, f.ftc, f.family))
            .collect(),
        pages,
        dop: max_dop(&doc.dop, &extra.dop_extra),
        frames,
        word_sounds: word_sounds(doc, extra, len),
        objects: extra.objects.clone(),
        selection,
        printer: extra.printer.clone(),
        thumbnail: extra.thumbnail.clone(),
        add: extra.add.clone(),
    }
}

/// Tabela `SNDT`: um elemento por palavra com som, no cp da primeira casa
/// com som, até o começo do seguinte; o último vai até antes do `\r` final
/// do documento (como o original grava: `SAMP6`, 141 cps, termina em 140). Os 4 bytes sem uso voltam como vieram; um som sem
/// cópia no documento (colado de outro) fica de fora.
fn word_sounds(doc: &Document, extra: &MaxExtra, len: u32) -> Vec<max::WordSound> {
    let anchors: Vec<(u32, ObjRef)> = doc
        .word_sounds()
        .into_iter()
        .filter(|(_, _, s)| extra.object(*s).is_some())
        .map(|(_, at, s)| (cp_of(doc, at).min(len), s))
        .collect();
    anchors
        .iter()
        .enumerate()
        .map(|(k, &(start, s))| {
            let sound = from_ref(s);
            let reserved = extra
                .word_sounds
                .iter()
                .find(|w| w.start == start && w.sound == sound)
                .map_or([0; 4], |w| w.reserved);
            max::WordSound {
                start,
                end: anchors
                    .get(k + 1)
                    .map_or(len.saturating_sub(1), |a| a.0)
                    .max(start),
                sound,
                reserved,
            }
        })
        .collect()
}

/// Cor `#rrggbb` de uma `MacColor` (para registros e testes).
pub fn hex(c: MacColor) -> String {
    let c = c.rgb();
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// Documento em branco do Escritório (o que "NOVO" e a prancheta abrem).
pub fn blank() -> (Document, MaxExtra) {
    (doc::Document::new(), MaxExtra::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::{CharChange, ParaChange, Spacing};

    fn sample() -> Document {
        let mut d = Document::from_text("Era uma vez\num gato", CharProps::default());
        let red = doc::CSCH[5];
        d.apply_char(
            Range::new(Pos::new(0, 4), Pos::new(0, 7)),
            CharChange::Color(Some(red)),
        );
        d.apply_char(
            Range::new(Pos::new(1, 0), Pos::new(1, 2)),
            CharChange::Size(Size::Giant),
        );
        d.apply_para(
            Range::at(Pos::new(1, 0)),
            ParaChange::Spacing(Spacing::ExtraAfter),
        );
        d.paras[1].items.insert(3, Item::Rebus(0x25));
        d.add_object(FloatObject::new(
            ObjectKind::Sticker,
            ObjRef::pict(0),
            0,
            100,
            200,
            50,
            40,
        ));
        d
    }

    #[test]
    fn roundtrip_through_max_bytes() {
        let d = sample();
        let extra = MaxExtra::new();
        let m = to_max(&d, &extra, &[Pos::default()], Some(Range::at(d.end())));
        // "Era uma vez\r" + "um %gato\r" + o `\r` a mais
        assert_eq!(m.text, "Era uma vez\rum %gato\r\r");
        assert_eq!(m.para_runs.len(), 3);
        assert!(m.para_runs[1].props.space_after);
        assert!(m.char_props_at(15).anchor);
        assert_eq!(m.selection, Some((20, 20)));
        assert_eq!(m.frames.len(), 1);
        let bytes = max::write(&m);
        let back = max::read(&bytes).unwrap();
        let (d2, e2) = from_max(&back);
        assert_eq!(d2.paras, d.paras);
        assert_eq!(d2.dop, d.dop);
        assert_eq!(d2.fonts, d.fonts);
        assert_eq!(d2.objects, d.objects);
        assert_eq!(e2.page_flags, vec![0]);
    }

    #[test]
    fn new_document_is_like_the_original() {
        let (d, extra) = blank();
        let m = to_max(&d, &extra, &[Pos::default()], Some(Range::at(d.start())));
        let orig = max::Document::new();
        assert_eq!(m.text, orig.text);
        assert_eq!(m.char_runs, orig.char_runs);
        assert_eq!(m.para_runs, orig.para_runs);
        assert_eq!(m.dop, orig.dop);
        assert_eq!(m.fonts, orig.fonts);
        assert_eq!(m.printer, orig.printer);
        // e lido de volta é o documento em branco
        let (back, _) = from_max(&orig);
        assert_eq!(back, d);
    }

    #[test]
    fn single_final_return_is_a_paragraph_and_cps_map() {
        let m = max::Document::from_text("ab\ncd");
        let (d, _) = from_max(&m);
        assert_eq!(d.paras.len(), 2);
        assert_eq!(cp_of(&d, Pos::new(1, 1)), 4);
        assert_eq!(pos_of(&d, 4), Pos::new(1, 1));
        assert_eq!(pos_of(&d, 99), d.end());
        // um `\r` só no fim (exemplos do Max): nada a tirar
        let mut m = max::Document::from_text("x");
        m.text = "x\r".into();
        m.char_runs[0].end = 2;
        m.para_runs.truncate(1);
        m.para_runs[0].end = 2;
        let (d, _) = from_max(&m);
        assert_eq!(d.paras.len(), 1);
        assert_eq!(d.text(), "x");
    }

    #[test]
    fn unknown_frames_pass_through() {
        let mut m = max::Document::new();
        let mut f = max::Frame::new(
            FrameKind::Other(6),
            max::ObjRef::new(b"PICT", 3),
            73,
            10,
            443,
            121,
        );
        f.page = u16::MAX;
        m.frames.push(f.clone());
        let (d, extra) = from_max(&m);
        assert!(d.objects.is_empty());
        let back = to_max(&d, &extra, &[Pos::default()], None);
        assert_eq!(back.frames, vec![f]);
    }

    #[test]
    fn extra_numbers_objects() {
        let mut e = MaxExtra::new();
        let c = OwnedChunk::single(b"PICT", 9, b"META", vec![1, 2]);
        assert_eq!(e.add_object(c.clone()), ObjRef::pict(0));
        assert_eq!(e.add_object(c), ObjRef::pict(1));
        assert!(e.object(ObjRef::pict(1)).is_some());
        assert_eq!(e.free_id(*b"SND "), 0);
    }
}
