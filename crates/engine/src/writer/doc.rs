//! Documento do Escritório: lista de parágrafos; cada parágrafo tem um
//! alinhamento e uma sequência de itens — caracteres com estilo (tamanho,
//! cor, negrito, inclinado, sublinhado) e adesivos de clip-art. Os "trechos"
//! (runs) são os caracteres consecutivos de mesmo estilo (`Paragraph::runs`).
//! O cursor é (parágrafo, posição entre itens). O histórico do ovo guarda
//! uma cópia do documento antes de cada ação.

use crate::paint::BLACK;

/// Tamanhos da lista "Dimensiona-palavras", do maior ao menor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    Giant,
    Huge,
    Large,
    #[default]
    Medium,
    Small,
    Tiny,
}

impl Size {
    /// Ordem da lista suspensa do original: Gigante, Enorme, Grande, Médio, Pequeno, Minúsculo.
    pub const ALL: [Size; 6] = [
        Size::Giant,
        Size::Huge,
        Size::Large,
        Size::Medium,
        Size::Small,
        Size::Tiny,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Size::Giant => "Gigante",
            Size::Huge => "Enorme",
            Size::Large => "Grande",
            Size::Medium => "Médio",
            Size::Small => "Pequeno",
            Size::Tiny => "Minúsculo",
        }
    }

    /// Fonte em pontos da "Pablo Graphos" usada como base (os grandes ampliam a de 16).
    pub fn points(self) -> u16 {
        match self {
            Size::Tiny => 6,
            Size::Small => 8,
            _ => 16,
        }
    }

    /// Fator de ampliação (vizinho mais próximo) sobre a fonte base.
    pub fn scale(self) -> u32 {
        match self {
            Size::Giant => 4,
            Size::Huge => 3,
            Size::Large => 2,
            _ => 1,
        }
    }

    /// Fator sobre a fonte 8×8 reserva, quando a "Pablo Graphos" falta.
    pub fn fallback_scale(self) -> u32 {
        match self {
            Size::Giant => 8,
            Size::Huge => 6,
            Size::Large => 4,
            Size::Medium => 2,
            Size::Small | Size::Tiny => 1,
        }
    }
}

/// Estilo de um caractere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub size: Size,
    /// Índice da paleta de 256 cores.
    pub color: u8,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            size: Size::Medium,
            color: BLACK,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

/// Alinhamento do parágrafo ("Posicionador").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Right,
    Center,
    Justify,
}

impl Align {
    pub const ALL: [Align; 4] = [Align::Left, Align::Right, Align::Center, Align::Justify];

    pub fn name(self) -> &'static str {
        match self {
            Align::Left => "Esquerda",
            Align::Right => "Direita",
            Align::Center => "Centro",
            Align::Justify => "Esquerda e direita",
        }
    }
}

/// Adesivo de clip-art dentro do texto: rolo (número do `CARO`), posição no
/// rolo e tamanho em pixels (o desenho em si fica com a cena).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sticker {
    pub roll: u16,
    pub index: usize,
    pub w: u32,
    pub h: u32,
}

/// Tamanho de um adesivo cujo desenho não foi encontrado.
pub const PLACEHOLDER_SIZE: u32 = 32;

impl Sticker {
    pub fn placeholder(roll: u16, index: usize) -> Sticker {
        Sticker {
            roll,
            index,
            w: PLACEHOLDER_SIZE,
            h: PLACEHOLDER_SIZE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Char(char, Style),
    Sticker(Sticker),
}

impl Item {
    pub fn style(&self) -> Option<Style> {
        match self {
            Item::Char(_, s) => Some(*s),
            Item::Sticker(_) => None,
        }
    }

    pub fn is_space(&self) -> bool {
        matches!(self, Item::Char(c, _) if c.is_whitespace())
    }
}

/// Trecho de um parágrafo: texto de um só estilo, ou um adesivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Run {
    Text { text: String, style: Style },
    Sticker(Sticker),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paragraph {
    pub align: Align,
    pub items: Vec<Item>,
}

impl Paragraph {
    pub fn from_text(text: &str, style: Style) -> Paragraph {
        Paragraph {
            align: Align::Left,
            items: text.chars().map(|c| Item::Char(c, style)).collect(),
        }
    }

    /// Caracteres consecutivos de mesmo estilo viram um trecho; adesivos, um cada.
    pub fn runs(&self) -> Vec<Run> {
        let mut runs: Vec<Run> = Vec::new();
        for item in &self.items {
            match *item {
                Item::Sticker(s) => runs.push(Run::Sticker(s)),
                Item::Char(c, style) => match runs.last_mut() {
                    Some(Run::Text { text, style: st }) if *st == style => text.push(c),
                    _ => runs.push(Run::Text {
                        text: c.to_string(),
                        style,
                    }),
                },
            }
        }
        runs
    }

    /// Só os caracteres (os adesivos são pulados).
    pub fn text(&self) -> String {
        self.items
            .iter()
            .filter_map(|i| match i {
                Item::Char(c, _) => Some(*c),
                Item::Sticker(_) => None,
            })
            .collect()
    }
}

/// Posição do cursor: entre os itens `index - 1` e `index` do parágrafo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    pub para: usize,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub paras: Vec<Paragraph>,
    pub cursor: Cursor,
    /// Estilo do que for digitado em seguida.
    pub style: Style,
}

impl Default for Document {
    fn default() -> Document {
        Document::new()
    }
}

impl Document {
    /// Um parágrafo vazio.
    pub fn new() -> Document {
        Document {
            paras: vec![Paragraph::default()],
            cursor: Cursor::default(),
            style: Style::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.paras.iter().all(|p| p.items.is_empty())
    }

    pub fn para(&self) -> &Paragraph {
        &self.paras[self.cursor.para]
    }

    fn para_mut(&mut self) -> &mut Paragraph {
        let k = self.cursor.para;
        &mut self.paras[k]
    }

    /// Garante que o cursor aponta para um lugar existente.
    pub fn clamp_cursor(&mut self) {
        if self.paras.is_empty() {
            self.paras.push(Paragraph::default());
        }
        self.cursor.para = self.cursor.para.min(self.paras.len() - 1);
        self.cursor.index = self.cursor.index.min(self.para().items.len());
    }

    fn insert_item(&mut self, item: Item) {
        let at = self.cursor.index;
        self.para_mut().items.insert(at, item);
        self.cursor.index += 1;
    }

    /// Caracteres de controle são ignorados.
    pub fn insert_char(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        let style = self.style;
        self.insert_item(Item::Char(c, style));
    }

    pub fn insert_text(&mut self, text: &str) {
        for c in text.chars() {
            self.insert_char(c);
        }
    }

    pub fn insert_sticker(&mut self, s: Sticker) {
        self.insert_item(Item::Sticker(s));
    }

    /// Enter: divide o parágrafo no cursor (o novo herda o alinhamento).
    pub fn newline(&mut self) {
        let at = self.cursor.index;
        let align = self.para().align;
        let rest = self.para_mut().items.split_off(at);
        let next = Paragraph { align, items: rest };
        self.paras.insert(self.cursor.para + 1, next);
        self.cursor = Cursor {
            para: self.cursor.para + 1,
            index: 0,
        };
    }

    /// Backspace: apaga o item antes do cursor, ou junta com o parágrafo anterior.
    /// Verdadeiro se algo mudou.
    pub fn backspace(&mut self) -> bool {
        if self.cursor.index > 0 {
            let at = self.cursor.index - 1;
            self.para_mut().items.remove(at);
            self.cursor.index = at;
            true
        } else if self.cursor.para > 0 {
            let cur = self.paras.remove(self.cursor.para);
            self.cursor.para -= 1;
            let prev = &mut self.paras[self.cursor.para];
            self.cursor.index = prev.items.len();
            prev.items.extend(cur.items);
            true
        } else {
            false
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor.index > 0 {
            self.cursor.index -= 1;
        } else if self.cursor.para > 0 {
            self.cursor.para -= 1;
            self.cursor.index = self.para().items.len();
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor.index < self.para().items.len() {
            self.cursor.index += 1;
        } else if self.cursor.para + 1 < self.paras.len() {
            self.cursor.para += 1;
            self.cursor.index = 0;
        }
    }

    /// Home: começo do parágrafo (a linha visual fica com a diagramação).
    pub fn home(&mut self) {
        self.cursor.index = 0;
    }

    pub fn end(&mut self) {
        self.cursor.index = self.para().items.len();
    }

    pub fn set_align(&mut self, align: Align) {
        self.para_mut().align = align;
    }

    /// Estilo do item antes do cursor (para o tamanho do cursor), senão o de digitação.
    pub fn style_at_cursor(&self) -> Style {
        self.cursor
            .index
            .checked_sub(1)
            .and_then(|i| self.para().items.get(i))
            .and_then(Item::style)
            .unwrap_or(self.style)
    }

    /// A "explosão" da tesoura: apaga tudo.
    pub fn clear(&mut self) {
        self.paras = vec![Paragraph::default()];
        self.cursor = Cursor::default();
    }

    /// Troca o tamanho (e as medidas) de todos os adesivos por `size(rolo, n)`.
    pub fn resize_stickers(&mut self, mut size: impl FnMut(u16, usize) -> Option<(u32, u32)>) {
        for p in &mut self.paras {
            for item in &mut p.items {
                if let Item::Sticker(s) = item
                    && let Some((w, h)) = size(s.roll, s.index)
                {
                    s.w = w;
                    s.h = h;
                }
            }
        }
    }
}

/// Pausa de digitação que fecha uma ação do ovo, em segundos.
pub const TYPING_PAUSE: f32 = 1.0;

/// Desfazer/refazer de um nível: uma cópia do documento antes da última ação.
/// Digitação contínua (sem pausa de `TYPING_PAUSE`) é uma ação só; Enter,
/// adesivos, estilo e a explosão são ações próprias.
#[derive(Debug, Clone, Default)]
pub struct History {
    undo: Option<Document>,
    redo: Option<Document>,
    /// Há uma ação de digitação aberta (as próximas teclas entram nela).
    typing: bool,
    idle: f32,
}

impl History {
    /// Antes de uma ação própria (Enter, adesivo, estilo, explosão).
    pub fn record(&mut self, doc: &Document) {
        self.undo = Some(doc.clone());
        self.redo = None;
        self.typing = false;
    }

    /// Antes de digitar ou apagar um caractere: abre a ação, se não houver uma aberta.
    pub fn record_typing(&mut self, doc: &Document) {
        if !self.typing {
            self.record(doc);
            self.typing = true;
        }
        self.idle = 0.0;
    }

    /// Fecha a ação de digitação aberta (mudança de parágrafo, cursor etc.).
    pub fn close(&mut self) {
        self.typing = false;
    }

    pub fn tick(&mut self, dt: f32) {
        if self.typing {
            self.idle += dt.max(0.0);
            if self.idle >= TYPING_PAUSE {
                self.typing = false;
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        self.undo.is_some()
    }

    /// O ovo: desfaz, ou refaz o que acabou de ser desfeito. Verdadeiro se mudou.
    pub fn toggle(&mut self, doc: &mut Document) -> bool {
        self.typing = false;
        if let Some(mut prev) = self.undo.take() {
            std::mem::swap(doc, &mut prev);
            self.redo = Some(prev);
            true
        } else if let Some(mut next) = self.redo.take() {
            std::mem::swap(doc, &mut next);
            self.undo = Some(next);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big() -> Style {
        Style {
            size: Size::Giant,
            ..Style::default()
        }
    }

    #[test]
    fn new_document_is_one_empty_paragraph() {
        let d = Document::new();
        assert_eq!(d.paras.len(), 1);
        assert!(d.is_empty());
        assert_eq!(d.cursor, Cursor::default());
        assert_eq!(d.style, Style::default());
        assert_eq!(Style::default().color, BLACK);
    }

    #[test]
    fn insert_and_runs() {
        let mut d = Document::new();
        d.insert_text("ab");
        d.style = big();
        d.insert_text("c\u{7}");
        assert_eq!(d.para().text(), "abc");
        assert_eq!(d.cursor.index, 3);
        let runs = d.para().runs();
        assert_eq!(runs.len(), 2);
        assert_eq!(
            runs[0],
            Run::Text {
                text: "ab".into(),
                style: Style::default()
            }
        );
        assert_eq!(
            runs[1],
            Run::Text {
                text: "c".into(),
                style: big()
            }
        );
    }

    #[test]
    fn insert_in_the_middle_and_sticker() {
        let mut d = Document::new();
        d.insert_text("ac");
        d.move_left();
        d.insert_char('b');
        assert_eq!(d.para().text(), "abc");
        assert_eq!(d.cursor.index, 2);
        d.insert_sticker(Sticker::placeholder(3, 4));
        assert_eq!(d.para().items.len(), 4);
        assert!(matches!(d.para().items[2], Item::Sticker(s) if s.roll == 3 && s.index == 4));
        assert_eq!(d.para().runs().len(), 3);
        assert_eq!(d.para().text(), "abc");
    }

    #[test]
    fn newline_splits_and_backspace_joins() {
        let mut d = Document::new();
        d.insert_text("abcd");
        d.set_align(Align::Center);
        d.move_left();
        d.move_left();
        d.newline();
        assert_eq!(d.paras.len(), 2);
        assert_eq!(d.paras[0].text(), "ab");
        assert_eq!(d.paras[1].text(), "cd");
        assert_eq!(d.paras[1].align, Align::Center);
        assert_eq!(d.cursor, Cursor { para: 1, index: 0 });
        assert!(d.backspace());
        assert_eq!(d.paras.len(), 1);
        assert_eq!(d.paras[0].text(), "abcd");
        assert_eq!(d.cursor, Cursor { para: 0, index: 2 });
        assert!(d.backspace());
        assert_eq!(d.paras[0].text(), "acd");
        d.home();
        assert!(!d.backspace());
    }

    #[test]
    fn cursor_moves_across_paragraphs() {
        let mut d = Document::new();
        d.insert_text("ab");
        d.newline();
        d.insert_text("c");
        d.home();
        d.move_left();
        assert_eq!(d.cursor, Cursor { para: 0, index: 2 });
        d.move_right();
        assert_eq!(d.cursor, Cursor { para: 1, index: 0 });
        d.end();
        d.move_right();
        assert_eq!(d.cursor, Cursor { para: 1, index: 1 });
        d.style = big();
        assert_eq!(d.style_at_cursor(), Style::default());
        d.home();
        assert_eq!(d.style_at_cursor(), big());
    }

    #[test]
    fn clear_and_resize() {
        let mut d = Document::new();
        d.insert_sticker(Sticker::placeholder(1, 0));
        d.insert_sticker(Sticker::placeholder(2, 0));
        d.resize_stickers(|roll, _| (roll == 2).then_some((10, 20)));
        let sizes: Vec<_> = d
            .para()
            .items
            .iter()
            .map(|i| match i {
                Item::Sticker(s) => (s.w, s.h),
                _ => (0, 0),
            })
            .collect();
        assert_eq!(sizes, [(32, 32), (10, 20)]);
        d.clear();
        assert!(d.is_empty());
        assert_eq!(d.cursor, Cursor::default());
    }

    #[test]
    fn history_groups_typing_and_toggles() {
        let mut d = Document::new();
        let mut h = History::default();
        assert!(!h.toggle(&mut d));
        for c in "abc".chars() {
            h.record_typing(&d);
            d.insert_char(c);
            h.tick(0.2);
        }
        assert!(h.toggle(&mut d));
        assert!(d.is_empty());
        assert!(h.toggle(&mut d));
        assert_eq!(d.para().text(), "abc");
        // pausa de 1 s: nova ação
        h.tick(1.5);
        h.record_typing(&d);
        d.insert_char('d');
        assert!(h.toggle(&mut d));
        assert_eq!(d.para().text(), "abc");
        // ação própria fecha a digitação
        h.record_typing(&d);
        d.insert_char('x');
        h.record(&d);
        d.newline();
        h.record_typing(&d);
        d.insert_char('y');
        assert!(h.toggle(&mut d));
        assert_eq!(d.paras.len(), 2);
        assert_eq!(d.paras[1].text(), "");
    }
}
