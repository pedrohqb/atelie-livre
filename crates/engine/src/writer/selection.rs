//! Seleção e área de transferência interna do Escritório.
//!
//! A seleção é uma âncora e um cursor ([`Selection`]); vazia, é o ponto de
//! inserção. Shift+setas e o arrasto do mouse movem só o cursor. A área de
//! transferência ([`Clipboard`]) guarda um trecho de texto (com estilos e
//! figuras do Maçã) ou um objeto solto (adesivo, palavra com forma) — os
//! Re-movedores e Ctrl+X/C/V. Não é a área de transferência do sistema.

use super::doc::{CharProps, Document, Fragment, Pos, Range};
use super::layout::Layout;
use super::object::FloatObject;

/// Âncora e cursor. O cursor é onde o ponto de inserção pisca.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Selection {
    pub anchor: Pos,
    pub caret: Pos,
    /// Coluna (x da folha) que ↑/↓ tentam manter.
    pub goal_x: Option<i32>,
}

/// Movimentos do teclado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    /// Ctrl+←.
    WordLeft,
    /// Ctrl+→.
    WordRight,
    Up,
    Down,
    /// Home.
    LineStart,
    /// End.
    LineEnd,
    /// Ctrl+Home.
    DocStart,
    /// Ctrl+End.
    DocEnd,
}

impl Selection {
    /// Ponto de inserção em `p`.
    pub fn at(p: Pos) -> Selection {
        Selection {
            anchor: p,
            caret: p,
            goal_x: None,
        }
    }

    pub fn new(anchor: Pos, caret: Pos) -> Selection {
        Selection {
            anchor,
            caret,
            goal_x: None,
        }
    }

    pub fn range(&self) -> Range {
        Range::new(self.anchor, self.caret)
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.caret
    }

    /// Leva o cursor a `p`; com `extend` (Shift, arrasto) a âncora fica.
    pub fn move_to(&mut self, p: Pos, extend: bool) {
        self.caret = p;
        if !extend {
            self.anchor = p;
        }
        self.goal_x = None;
    }

    /// Ajusta as duas pontas a posições que existem no documento.
    pub fn clamp(&mut self, doc: &Document) {
        self.anchor = doc.clamp(self.anchor);
        self.caret = doc.clamp(self.caret);
    }

    /// Seleciona tudo.
    pub fn select_all(&mut self, doc: &Document) {
        *self = Selection::new(doc.start(), doc.end());
    }

    /// Seleciona a palavra em `p` (clique duplo).
    pub fn select_word(&mut self, doc: &Document, p: Pos) {
        let w = doc.word_at(p);
        *self = Selection::new(w.start, w.end);
    }

    /// Começo do arrasto do mouse (ou clique simples).
    pub fn begin_drag(&mut self, p: Pos) {
        *self = Selection::at(p);
    }

    /// O mouse arrastado até `p`.
    pub fn drag_to(&mut self, p: Pos) {
        self.move_to(p, true);
    }

    /// Movimento do teclado; com `extend` (Shift) estende a seleção. Sem
    /// Shift, ← e → numa seleção a desfazem na ponta correspondente.
    pub fn apply(&mut self, doc: &Document, layout: &Layout, m: Motion, extend: bool) {
        self.clamp(doc);
        let r = self.range();
        let c = self.caret;
        let goal = self.goal_x;
        let target = match m {
            Motion::Left if !extend && !r.is_empty() => r.start,
            Motion::Right if !extend && !r.is_empty() => r.end,
            Motion::Left => doc.prev(c).unwrap_or(c),
            Motion::Right => doc.next(c).unwrap_or(c),
            Motion::WordLeft => doc.prev_word(c),
            Motion::WordRight => doc.next_word(c),
            Motion::Up | Motion::Down => {
                let x = goal.unwrap_or_else(|| layout.caret(c).x);
                let to = match layout.vertical(c, m == Motion::Up, x) {
                    Some(p) => p,
                    None if m == Motion::Up => doc.start(),
                    None => doc.end(),
                };
                self.move_to(to, extend);
                self.goal_x = Some(x);
                return;
            }
            Motion::LineStart => layout.line_home(c),
            Motion::LineEnd => layout.line_end(c),
            Motion::DocStart => doc.start(),
            Motion::DocEnd => doc.end(),
        };
        self.move_to(target, extend);
    }

    /// Propriedades do que for digitado: as do ponto em que a seleção começa.
    pub fn typing_props(&self, doc: &Document) -> CharProps {
        doc.props_at(self.range().start)
    }

    /// Digita `text` no lugar da seleção, com `props`.
    pub fn type_text(&mut self, doc: &mut Document, text: &str, props: CharProps) {
        let at = doc.delete(self.range());
        let end = doc.insert_text(at, text, props);
        *self = Selection::at(end);
    }

    /// Backspace: apaga a seleção ou o item antes do cursor. Verdadeiro se
    /// algo mudou.
    pub fn backspace(&mut self, doc: &mut Document) -> bool {
        self.clamp(doc);
        if !self.is_empty() {
            *self = Selection::at(doc.delete(self.range()));
            return true;
        }
        match doc.backspace(self.caret) {
            Some(p) => {
                *self = Selection::at(p);
                true
            }
            None => false,
        }
    }

    /// Delete: apaga a seleção ou o item depois do cursor.
    pub fn delete_forward(&mut self, doc: &mut Document) -> bool {
        self.clamp(doc);
        if !self.is_empty() {
            *self = Selection::at(doc.delete(self.range()));
            return true;
        }
        doc.delete_forward(self.caret)
    }

    /// Enter: quebra o parágrafo (apagando a seleção antes).
    pub fn newline(&mut self, doc: &mut Document) {
        let at = doc.delete(self.range());
        *self = Selection::at(doc.split(at));
    }
}

/// O que está na área de transferência.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clip {
    Text(Fragment),
    Object(FloatObject),
}

/// Área de transferência interna (recortar, copiar, colar).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Clipboard {
    pub clip: Option<Clip>,
}

impl Clipboard {
    /// Nada copiado (o Colar avisa com `0805[19]`).
    pub fn is_empty(&self) -> bool {
        self.clip.is_none()
    }

    /// Copia a seleção. Falso se ela está vazia.
    pub fn copy(&mut self, doc: &Document, sel: &Selection) -> bool {
        let r = sel.range();
        if r.is_empty() {
            return false;
        }
        self.clip = Some(Clip::Text(doc.slice(r)));
        true
    }

    /// Recorta a seleção (o cursor fica onde ela começava).
    pub fn cut(&mut self, doc: &mut Document, sel: &mut Selection) -> bool {
        if !self.copy(doc, sel) {
            return false;
        }
        *sel = Selection::at(doc.delete(sel.range()));
        true
    }

    /// Copia o objeto `k`.
    pub fn copy_object(&mut self, doc: &Document, k: usize) -> bool {
        match doc.objects.get(k) {
            Some(o) => {
                self.clip = Some(Clip::Object(o.clone()));
                true
            }
            None => false,
        }
    }

    /// Recorta o objeto `k`.
    pub fn cut_object(&mut self, doc: &mut Document, k: usize) -> bool {
        match doc.remove_object(k) {
            Some(o) => {
                self.clip = Some(Clip::Object(o));
                true
            }
            None => false,
        }
    }

    /// Cola o texto no lugar da seleção (o cursor vai para depois dele).
    /// Falso se a área de transferência não tem texto.
    pub fn paste_text(&self, doc: &mut Document, sel: &mut Selection) -> bool {
        let Some(Clip::Text(f)) = &self.clip else {
            return false;
        };
        let at = doc.delete(sel.range());
        *sel = Selection::at(doc.insert_fragment(at, f));
        true
    }

    /// Cola o objeto centrado no ponto `(cx, cy)` da folha `page` (pode
    /// colar quantas vezes quiser); devolve o índice do novo objeto.
    pub fn paste_object(&self, doc: &mut Document, page: u16, cx: i32, cy: i32) -> Option<usize> {
        let Some(Clip::Object(o)) = &self.clip else {
            return None;
        };
        let mut o = o.clone();
        let (w, h) = (i32::from(o.width), i32::from(o.height));
        o.page = page;
        o.move_by(
            cx - w / 2 - i32::from(o.x),
            cy - h / 2 - i32::from(o.y),
            i32::MAX / 4,
            i32::MAX / 4,
        );
        Some(doc.add_object(o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::{CharChange, Item, Size};
    use crate::writer::layout::SimpleMeasure;
    use crate::writer::object::{ObjRef, ObjectKind};

    fn doc(text: &str) -> Document {
        Document::from_text(text, CharProps::default())
    }

    fn lay(d: &Document) -> Layout {
        Layout::build(d, &SimpleMeasure)
    }

    #[test]
    fn shift_arrows_extend_and_plain_arrows_collapse() {
        let d = doc("abc\ndef");
        let l = lay(&d);
        let mut s = Selection::at(Pos::new(0, 1));
        s.apply(&d, &l, Motion::Right, true);
        s.apply(&d, &l, Motion::Right, true);
        assert_eq!(s.range(), Range::new(Pos::new(0, 1), Pos::new(0, 3)));
        assert_eq!(d.text_range(s.range()), "bc");
        s.apply(&d, &l, Motion::Right, true);
        assert_eq!(d.text_range(s.range()), "bc\n");
        s.apply(&d, &l, Motion::Left, false);
        assert_eq!(s, Selection::at(Pos::new(0, 1)));
        // seleção para trás
        s.apply(&d, &l, Motion::Left, true);
        assert_eq!(s.anchor, Pos::new(0, 1));
        assert_eq!(s.caret, Pos::new(0, 0));
        s.apply(&d, &l, Motion::Right, false);
        assert_eq!(s, Selection::at(Pos::new(0, 1)));
        s.apply(&d, &l, Motion::DocEnd, true);
        assert_eq!(d.text_range(s.range()), "bc\ndef");
        s.apply(&d, &l, Motion::DocStart, false);
        assert!(s.is_empty() && s.caret == d.start());
        s.apply(&d, &l, Motion::Left, false);
        assert_eq!(s.caret, d.start());
    }

    #[test]
    fn words_lines_and_vertical_goal() {
        let d = doc("abcdef\nx\nabcdef");
        let l = lay(&d);
        let mut s = Selection::at(Pos::new(0, 5));
        s.apply(&d, &l, Motion::Down, false);
        assert_eq!(s.caret, Pos::new(1, 1));
        s.apply(&d, &l, Motion::Down, false);
        // a coluna desejada volta depois da linha curta
        assert_eq!(s.caret, Pos::new(2, 5));
        s.apply(&d, &l, Motion::Down, true);
        assert_eq!(s.caret, d.end());
        assert_eq!(s.anchor, Pos::new(2, 5));
        s.apply(&d, &l, Motion::LineStart, false);
        assert_eq!(s.caret, Pos::new(2, 0));
        s.apply(&d, &l, Motion::LineEnd, false);
        assert_eq!(s.caret, Pos::new(2, 6));
        s.apply(&d, &l, Motion::Up, false);
        assert_eq!(s.caret, Pos::new(1, 1));
        s.apply(&d, &l, Motion::Up, false);
        assert_eq!(s.caret, Pos::new(0, 6));
        s.apply(&d, &l, Motion::Up, false);
        assert_eq!(s.caret, d.start());
        let d = doc("Era uma vez");
        let l = lay(&d);
        let mut s = Selection::at(Pos::new(0, 0));
        s.apply(&d, &l, Motion::WordRight, true);
        assert_eq!(d.text_range(s.range()), "Era ");
        s.apply(&d, &l, Motion::WordLeft, false);
        assert_eq!(s.caret, Pos::new(0, 0));
    }

    #[test]
    fn drag_word_and_all() {
        let d = doc("Era uma vez");
        let mut s = Selection::default();
        s.begin_drag(Pos::new(0, 9));
        s.drag_to(Pos::new(0, 5));
        assert_eq!(d.text_range(s.range()), "ma v");
        s.select_word(&d, Pos::new(0, 5));
        assert_eq!(d.text_range(s.range()), "uma");
        s.select_all(&d);
        assert_eq!(d.text_range(s.range()), "Era uma vez");
        let mut s = Selection::new(Pos::new(9, 99), Pos::new(0, 0));
        s.clamp(&d);
        assert_eq!(s.range(), d.all());
    }

    #[test]
    fn typing_replaces_the_selection() {
        let mut d = doc("Era uma vez");
        d.apply_char(
            Range::new(Pos::new(0, 4), Pos::new(0, 7)),
            CharChange::Size(Size::Large),
        );
        let mut s = Selection::new(Pos::new(0, 4), Pos::new(0, 7));
        let p = s.typing_props(&d);
        assert_eq!(
            p.size,
            Size::Minimal,
            "estilo do ponto em que a seleção começa"
        );
        let big = d.props_at(Pos::new(0, 5));
        s.type_text(&mut d, "duas", big);
        assert_eq!(d.text(), "Era duas vez");
        assert_eq!(s, Selection::at(Pos::new(0, 8)));
        s = Selection::new(Pos::new(0, 3), Pos::new(0, 8));
        s.newline(&mut d);
        assert_eq!(d.text(), "Era\n vez");
        assert_eq!(s.caret, Pos::new(1, 0));
        assert!(s.backspace(&mut d));
        assert_eq!(d.text(), "Era vez");
        s = Selection::new(Pos::new(0, 0), Pos::new(0, 4));
        assert!(s.backspace(&mut d));
        assert_eq!(d.text(), "vez");
        assert!(!s.backspace(&mut d));
        assert!(s.delete_forward(&mut d));
        assert_eq!(d.text(), "ez");
        s = Selection::new(Pos::new(0, 0), Pos::new(0, 2));
        assert!(s.delete_forward(&mut d));
        assert!(!s.delete_forward(&mut d));
        assert!(d.is_empty());
    }

    #[test]
    fn cut_copy_paste_text() {
        let mut d = doc("O gato\ne a bicicleta");
        d.insert_item(Pos::new(0, 6), Item::Rebus(0x49));
        let mut cb = Clipboard::default();
        let mut s = Selection::at(Pos::new(0, 2));
        assert!(cb.is_empty());
        assert!(!cb.copy(&d, &s));
        assert!(!cb.paste_text(&mut d, &mut s));
        // copia "gato[figura]\ne a"
        s = Selection::new(Pos::new(0, 2), Pos::new(1, 3));
        assert!(cb.copy(&d, &s));
        let before = d.clone();
        assert!(cb.cut(&mut d, &mut s));
        assert_eq!(d.text(), "O  bicicleta");
        assert_eq!(s, Selection::at(Pos::new(0, 2)));
        assert!(cb.paste_text(&mut d, &mut s));
        assert_eq!(d, before);
        assert_eq!(s.caret, Pos::new(1, 3));
        // colar de novo, sobre uma seleção
        s = Selection::new(d.end(), Pos::new(1, 4));
        assert!(cb.paste_text(&mut d, &mut s));
        assert_eq!(d.text(), "O gato\ne a gato\ne a");
        assert!(cb.paste_object(&mut d, 0, 0, 0).is_none());
    }

    #[test]
    fn cut_copy_paste_objects() {
        let mut d = doc("x");
        d.add_object(FloatObject::new(
            ObjectKind::Sticker,
            ObjRef::pict(2),
            0,
            10,
            20,
            40,
            30,
        ));
        let mut cb = Clipboard::default();
        assert!(!cb.copy_object(&d, 3));
        assert!(cb.copy_object(&d, 0));
        let k = cb.paste_object(&mut d, 1, 200, 300).unwrap();
        assert_eq!(k, 1);
        let o = &d.objects[1];
        assert_eq!((o.page, o.x, o.y, o.width, o.height), (1, 180, 285, 40, 30));
        assert!(cb.cut_object(&mut d, 0));
        assert_eq!(d.objects.len(), 1);
        assert!(!cb.cut_object(&mut d, 5));
        let mut s = Selection::at(d.start());
        assert!(!cb.paste_text(&mut d, &mut s), "objeto não cola como texto");
        assert_eq!(cb.paste_object(&mut d, 0, 30, 35), Some(1));
        assert_eq!((d.objects[1].x, d.objects[1].y), (10, 20));
    }
}
