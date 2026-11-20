//! Ferramenta Palavra (`office-palavra.md`): faixa de duas páginas
//! (interruptor "Mais escolhas", som `021d`), listas em [`super::word_lists`]
//! e o pincel ([`crate::writer::brush`]).
//!
//! Fluxo do original: clicar num botão abre a lista (som `0233`) e o balão;
//! escolher um item fecha a lista (som `022d`) e carrega o pincel (cursor
//! `0004`); arrastar o pincel sobre o texto muda as palavras inteiras
//! tocadas (som `021c`, `0285` no Dimensiona-palavras, ao apertar e `021e`
//! ao soltar) — um clique não basta; para as opções de parágrafo basta
//! clicar. O Aplicador (`0220`) passa o pincel no documento inteiro, e com
//! Ctrl grava o padrão dos documentos novos; o Ponto de inserção (`0224`)
//! larga o pincel. Enquanto há pincel o ponto de inserção some.

use std::rc::Rc;

use super::cursors::BRUSH;
use super::editor::Editor;
use super::tools::{
    Cx, SND_CHOOSE, SND_OPTION, SND_SWITCH, StripButton, ToolId, ToolStrip, WHAT_WORD, cursor,
    inside, strip_button, unavailable,
};
pub use super::word_lists::List;
use crate::Frame;
use crate::assets::{Assets, CursorRef};
use crate::scene::Key;
use crate::writer::brush::Brush;
use crate::writer::doc::{CharChange, Document, Pos, Range};
use crate::writer::layout::Layout;
use crate::writer::new_style::NewStyle;

/// Pincel apertado sobre o texto.
pub const SND_BRUSH: u16 = 0x021c;
/// Pincel do Dimensiona-palavras apertado.
pub const SND_BRUSH_SIZE: u16 = 0x0285;
/// Pincel solto.
pub const SND_BRUSH_UP: u16 = 0x021e;
/// Aplicador.
pub const SND_APPLICATOR: u16 = 0x0220;
/// Ponto de inserção.
pub const SND_INSERTION: u16 = 0x0224;

const fn button(
    id: u16,
    mask: Option<u16>,
    r: (i32, i32, i32, i32),
    help: usize,
    name: &'static str,
) -> StripButton {
    strip_button(id, mask, r, (WHAT_WORD, help), name)
}

const APPLICATOR: &str = "Aplicador";
const INSERTION: &str = "Ponto de inserção";
const SWITCH: &str = "Mais escolhas";
const SHAPED: &str = "FORMA";

/// Página 1 da faixa, na ordem da tela (`office-palavra.md` §2).
pub const STRIP: [StripButton; 9] = [
    button(0x0275, None, (24, 61, 49, 34), 3, "Seleciona-letras"),
    button(0x0811, None, (89, 61, 49, 34), 4, "Dimensiona-palavras"),
    button(0x027b, None, (154, 61, 49, 34), 5, "Seleciona-cores"),
    button(0x0279, None, (219, 61, 49, 34), 7, "Seleciona-estilos"),
    button(0x0bc3, None, (284, 61, 49, 34), 8, "Posicionador"),
    button(0x080b, Some(0x080d), (349, 58, 33, 39), 9, APPLICATOR),
    button(0x0831, Some(0x0833), (398, 61, 29, 34), 10, INSERTION),
    button(0x0269, Some(0x026b), (443, 57, 46, 41), 11, SHAPED),
    button(0x0229, Some(0x022b), (508, 59, 20, 30), 25, SWITCH),
];

/// Página 2 (`office-palavra.md` §3; o interruptor fica igual nas capturas).
pub const STRIP2: [StripButton; 9] = [
    button(0x0863, None, (24, 61, 49, 34), 12, "Iluminador"),
    button(0x080f, None, (89, 61, 49, 34), 13, "Efeitos especiais"),
    button(0x087d, None, (154, 61, 49, 34), 14, "Mix de estilos"),
    button(0x0815, None, (219, 61, 49, 34), 15, "Estilos de parágrafo"),
    button(0x08d5, None, (284, 61, 49, 34), 16, "Espaçamento"),
    button(0x0817, None, (349, 61, 49, 34), 17, "Recuos"),
    button(0x080b, Some(0x080d), (414, 58, 33, 39), 9, APPLICATOR),
    button(0x0831, Some(0x0833), (463, 61, 29, 34), 10, INSERTION),
    button(0x0229, Some(0x022b), (508, 59, 20, 30), 25, SWITCH),
];

/// Lista aberta por cada botão da página 1 (os outros são ações).
const LISTS: [Option<List>; 9] = [
    Some(List::Fonts),
    Some(List::Sizes),
    Some(List::Colors),
    Some(List::Styles),
    Some(List::Align),
    None,
    None,
    None,
    None,
];
const LISTS2: [Option<List>; 9] = [
    Some(List::Highlight),
    Some(List::Effects),
    Some(List::Mix),
    Some(List::ParaStyles),
    Some(List::Spacing),
    Some(List::Indents),
    None,
    None,
    None,
];

/// Uma passada do pincel em andamento.
struct Stroke {
    /// O documento antes da passada (cada movimento reaplica sobre ele).
    base: Document,
    /// A diagramação de antes: as posições não mudam enquanto as palavras
    /// crescem embaixo do pincel.
    layout: Rc<Layout>,
    anchor: Pos,
    start: (i32, i32),
    dragged: bool,
    recorded: bool,
}

#[derive(Default)]
pub struct WordStrip {
    /// Página 2 da faixa ("Mais escolhas").
    more: bool,
    list: Option<List>,
    /// Primeira fonte visível na lista do Seleciona-letras.
    font_scroll: usize,
    brush: Option<Brush>,
    stroke: Option<Stroke>,
}

impl WordStrip {
    pub fn list(&self) -> Option<List> {
        self.list
    }

    pub fn brush(&self) -> Option<Brush> {
        self.brush
    }

    /// Está na página 2 da faixa.
    pub fn more(&self) -> bool {
        self.more
    }

    fn buttons(&self) -> (&'static [StripButton; 9], &'static [Option<List>; 9]) {
        if self.more {
            (&STRIP2, &LISTS2)
        } else {
            (&STRIP, &LISTS)
        }
    }

    /// Retângulo do item `k` de uma lista (as fontes: contando da primeira
    /// visível).
    pub fn list_item_rect(list: List, k: usize) -> super::tools::Rect {
        list.row_rect(k)
    }

    /// Clique com uma lista aberta: escolhe o item e fecha; as setas das
    /// fontes rolam a lista sem fechar.
    fn list_click(&mut self, cx: &mut Cx, list: List, x: i32, y: i32) {
        if list == List::Fonts {
            let [up, down] = List::font_arrows();
            let max = list.items().saturating_sub(super::word_lists::FONT_ROWS);
            if inside(up, x, y) || inside(down, x, y) {
                let s = if inside(up, x, y) {
                    self.font_scroll.saturating_sub(1)
                } else {
                    (self.font_scroll + 1).min(max)
                };
                if s != self.font_scroll {
                    cx.sound(SND_SWITCH);
                    self.font_scroll = s;
                }
                return;
            }
        }
        self.list = None;
        let first = if list == List::Fonts {
            self.font_scroll
        } else {
            0
        };
        let Some(k) = (0..list.rows())
            .find(|&r| inside(list.row_rect(r), x, y))
            .map(|r| first + r)
            .filter(|&k| k < list.items())
        else {
            return;
        };
        cx.sound(SND_CHOOSE);
        self.brush = Some(list.brush(k));
    }

    fn button(&mut self, ed: &mut Editor, cx: &mut Cx, k: usize) {
        let (buttons, lists) = self.buttons();
        let b = &buttons[k];
        if let Some(list) = lists[k] {
            cx.sound(SND_OPTION);
            if let Some(h) = b.help {
                cx.help(h);
            }
            self.list = Some(list);
            return;
        }
        match b.name {
            SWITCH => {
                cx.sound(SND_SWITCH);
                self.more = !self.more;
            }
            APPLICATOR => {
                cx.sound(SND_APPLICATOR);
                match self.brush {
                    Some(brush) if cx.ctrl => {
                        // o padrão dos documentos novos
                        let mut s = NewStyle::load(ed.dir());
                        brush.apply_style(&mut s.font, &mut s.chars, &mut s.para);
                        match ed.dir().map(|d| s.save(d)) {
                            Some(Ok(())) => {
                                cx.log("Aplicador: estilo dos documentos novos gravado")
                            }
                            Some(Err(e)) => cx.log(format!("Aplicador: {e}")),
                            None => cx.log("Aplicador: sem pasta do usuário"),
                        }
                        cx.help((WHAT_WORD, 2));
                    }
                    Some(brush) => {
                        let mut d = ed.doc().clone();
                        brush.apply_all(&mut d);
                        if d != *ed.doc() {
                            ed.record();
                            *ed.doc_mut() = d;
                        }
                    }
                    None => cx.help((WHAT_WORD, 9)),
                }
            }
            INSERTION => {
                cx.sound(SND_INSERTION);
                cx.help((WHAT_WORD, 10));
                self.brush = None;
            }
            // abre o cavalete de palavras com forma (`super::shaped`)
            SHAPED => cx.open_shaped(None),
            _ => unavailable(b, cx),
        }
    }

    /// Reaplica o pincel do começo da passada até `cur`.
    fn repaint(&mut self, ed: &mut Editor, cur: Pos) {
        let (Some(s), Some(brush)) = (self.stroke.as_mut(), self.brush) else {
            return;
        };
        let mut d = s.base.clone();
        brush.stroke(&mut d, Range::new(s.anchor, cur), s.dragged);
        if d != *ed.doc() {
            if !s.recorded {
                ed.record();
                s.recorded = true;
            }
            *ed.doc_mut() = d;
        }
    }

    fn stroke_pos(&self, ed: &Editor, x: i32, y: i32) -> Option<Pos> {
        let s = self.stroke.as_ref()?;
        let pt = ed.to_page(x, y)?;
        Some(s.layout.pos_at(pt.page, pt.x, pt.y))
    }
}

impl ToolStrip for WordStrip {
    fn id(&self) -> ToolId {
        ToolId::Word
    }

    fn deactivate(&mut self, _ed: &mut Editor, _cx: &mut Cx) {
        self.list = None;
        self.brush = None;
        self.stroke = None;
    }

    fn modal(&self) -> bool {
        self.list.is_some()
    }

    fn hides_caret(&self) -> bool {
        self.list.is_some() || self.brush.is_some()
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if let Some(list) = self.list {
            self.list_click(cx, list, x, y);
            return true;
        }
        let (buttons, _) = self.buttons();
        if let Some(k) = StripButton::find(buttons, x, y) {
            self.button(ed, cx, k);
            return true;
        }
        let Some(brush) = self.brush else {
            return false;
        };
        let Some(pt) = ed.to_page(x, y) else {
            return false;
        };
        let size = matches!(brush, Brush::Char(CharChange::Size(_)));
        cx.sound(if size { SND_BRUSH_SIZE } else { SND_BRUSH });
        let layout = ed.layout();
        let anchor = layout.pos_at(pt.page, pt.x, pt.y);
        self.stroke = Some(Stroke {
            base: ed.doc().clone(),
            layout,
            anchor,
            start: (x, y),
            dragged: false,
            recorded: false,
        });
        self.repaint(ed, anchor);
        true
    }

    fn motion(&mut self, ed: &mut Editor, _cx: &mut Cx, x: i32, y: i32) {
        let Some(s) = self.stroke.as_mut() else {
            return;
        };
        if (x, y) != s.start {
            s.dragged = true;
        }
        if let Some(cur) = self.stroke_pos(ed, x, y) {
            self.repaint(ed, cur);
        }
    }

    fn release(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) {
        if self.stroke.is_none() {
            return;
        }
        self.motion(ed, cx, x, y);
        self.stroke = None;
        cx.sound(SND_BRUSH_UP);
    }

    fn key(&mut self, _ed: &mut Editor, _cx: &mut Cx, key: Key, _ctrl: bool) -> bool {
        // Esc fecha a lista sem escolher (sem som)
        if key == Key::Escape && self.list.is_some() {
            self.list = None;
            return true;
        }
        false
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(self.buttons().0, f, a);
    }

    fn draw_front(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        if let Some(list) = self.list {
            list.draw(ed, self.brush, self.font_scroll, f, a);
        }
    }

    fn cursor(&self, ed: &Editor, x: i32, y: i32) -> Option<CursorRef> {
        let on_page = ed.to_page(x, y).is_some();
        (self.list.is_none() && self.brush.is_some() && (on_page || self.stroke.is_some()))
            .then(|| cursor(BRUSH))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Effect;
    use crate::scenes::testutil::empty_assets;
    use crate::writer::doc::{Align, CharProps, Size};
    use crate::writer::mix::Mix;

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(crate::assets::SoundRef::Snd(_, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    struct T {
        ed: Editor,
        w: WordStrip,
        a: Assets,
        fx: Vec<Effect>,
        ctrl: bool,
        _d: tempfile::TempDir,
    }

    impl T {
        fn new(text: &str) -> T {
            let (d, a) = empty_assets();
            let mut ed = Editor::blank(None, &a);
            for (k, line) in text.split('\n').enumerate() {
                if k > 0 {
                    ed.key(Key::Enter, false, false);
                }
                ed.type_text(line);
            }
            ed.undo.clear();
            T {
                ed,
                w: WordStrip::default(),
                a,
                fx: Vec::new(),
                ctrl: false,
                _d: d,
            }
        }

        fn run(&mut self, f: impl FnOnce(&mut WordStrip, &mut Editor, &mut Cx)) {
            let mut cx = Cx::new(&mut self.fx, &self.a);
            cx.ctrl = self.ctrl;
            f(&mut self.w, &mut self.ed, &mut cx);
        }

        fn press(&mut self, x: i32, y: i32) -> bool {
            let mut r = false;
            self.run(|w, ed, cx| r = w.press(ed, cx, x, y));
            r
        }

        fn drag(&mut self, from: (i32, i32), to: (i32, i32)) {
            self.press(from.0, from.1);
            self.run(|w, ed, cx| {
                w.motion(ed, cx, (from.0 + to.0) / 2, (from.1 + to.1) / 2);
                w.motion(ed, cx, to.0, to.1);
                w.release(ed, cx, to.0, to.1);
            });
        }

        /// Abre a lista do botão `k` e escolhe o item `item`.
        fn choose(&mut self, k: usize, item: usize) {
            let (b, _) = self.w.buttons();
            let (x, y, _, _) = b[k].rect();
            self.press(x + 5, y + 5);
            let list = self.w.list().unwrap();
            let (x, y, _, _) = list.row_rect(item);
            self.press(x + 5, y + 5);
            assert_eq!(self.w.list(), None);
        }

        /// Ponto da tela sobre o item `index` do parágrafo `para`.
        fn at(&self, para: usize, index: usize) -> (i32, i32) {
            let l = self.ed.layout();
            let c = l.caret(Pos::new(para, index));
            let (x, y) = self.ed.to_screen(c.page, c.x + 2, c.top + c.height / 2);
            (x, y)
        }

        fn props(&self, para: usize, k: usize) -> CharProps {
            *self.ed.doc().paras[para].items[k].props().unwrap()
        }
    }

    #[test]
    fn choosing_an_item_loads_the_brush_and_dragging_paints_whole_words() {
        let mut t = T::new("Era uma vez um gato");
        t.choose(1, 2); // Grande
        assert_eq!(sounds(&t.fx), [SND_OPTION, SND_CHOOSE]);
        assert_eq!(
            t.w.brush(),
            Some(Brush::Char(CharChange::Size(Size::Large)))
        );
        // nada mudou só por escolher
        assert!(t.props(0, 0).is_default());
        assert!(t.w.hides_caret());
        let page = t.at(0, 5);
        assert_eq!(t.w.cursor(&t.ed, page.0, page.1), Some(cursor(BRUSH)));
        // um clique não basta
        t.fx.clear();
        t.press(page.0, page.1);
        t.run(|w, ed, cx| w.release(ed, cx, page.0, page.1));
        assert_eq!(sounds(&t.fx), [SND_BRUSH_SIZE, SND_BRUSH_UP]);
        assert!(t.props(0, 5).is_default());
        assert_eq!(t.ed.undo.undo_len(), 0);
        // arrastar do meio de "uma" ao meio de "vez"
        let (from, to) = (t.at(0, 5), t.at(0, 9));
        t.drag(from, to);
        let sizes: Vec<_> = (0..t.ed.doc().paras[0].len())
            .map(|k| t.props(0, k).size)
            .collect();
        assert_eq!(sizes[4..11], [Size::Large; 7]);
        assert_eq!(sizes[3], Size::Minimal);
        assert_eq!(sizes[11], Size::Minimal);
        // uma ação no Ovo
        assert_eq!(t.ed.undo.undo_len(), 1);
        // o pincel continua carregado até o Ponto de inserção
        t.fx.clear();
        let (x, y, _, _) = STRIP[6].rect();
        t.press(x + 3, y + 3);
        assert_eq!(sounds(&t.fx), [SND_INSERTION]);
        assert_eq!(t.w.brush(), None);
        assert!(!t.w.hides_caret());
        assert_eq!(t.w.cursor(&t.ed, page.0, page.1), None);
        // sem pincel, o clique na página é do editor
        assert!(!t.press(page.0, page.1));
    }

    #[test]
    fn paragraph_brush_needs_only_a_click() {
        let mut t = T::new("abc\ndef");
        t.choose(4, 2); // Centro
        let p = t.at(1, 1);
        t.press(p.0, p.1);
        t.run(|w, ed, cx| w.release(ed, cx, p.0, p.1));
        assert_eq!(t.ed.doc().paras[1].props.align, Align::Center);
        assert_eq!(t.ed.doc().paras[0].props.align, Align::Left);
    }

    #[test]
    fn switch_shows_the_second_page() {
        let mut t = T::new("abc def");
        let (x, y, _, _) = STRIP[8].rect();
        t.press(x + 3, y + 3);
        assert!(t.w.more());
        assert_eq!(sounds(&t.fx), [SND_SWITCH]);
        // Mix Maluco arrastado sobre "abc"
        t.choose(2, 0);
        assert_eq!(t.w.brush(), Some(Brush::Mix(Mix::Crazy)));
        let (from, to) = (t.at(0, 0), t.at(0, 2));
        t.drag(from, to);
        let p = t.props(0, 0);
        assert_eq!(p.size, Size::Huge);
        assert!(t.props(0, 4).is_default());
        // de volta à página 1
        t.press(x + 3, y + 3);
        assert!(!t.w.more());
    }

    #[test]
    fn applicator_paints_the_whole_document() {
        let mut t = T::new("abc\ndef");
        // sem pincel: só o balão
        let (x, y, _, _) = STRIP[5].rect();
        t.press(x + 3, y + 3);
        assert_eq!(t.ed.undo.undo_len(), 0);
        t.choose(3, 1); // Negrito
        t.fx.clear();
        t.press(x + 3, y + 3);
        assert_eq!(sounds(&t.fx), [SND_APPLICATOR]);
        assert!(
            t.ed.doc()
                .paras
                .iter()
                .all(|p| p.mark.bold && p.items.iter().all(|i| i.props().unwrap().bold))
        );
        assert_eq!(t.ed.undo.undo_len(), 1);
    }

    #[test]
    fn ctrl_applicator_saves_the_style_for_new_documents() {
        let dir = std::env::temp_dir().join(format!("atelie-word-ctrl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut t = T::new("abc");
        t.ed = Editor::blank(Some(dir.clone()), &t.a);
        t.choose(1, 0); // Gigante
        t.ctrl = true;
        let (x, y, _, _) = STRIP[5].rect();
        t.press(x + 3, y + 3);
        // o documento atual não muda
        assert_eq!(t.ed.undo.undo_len(), 0);
        let blank = Editor::blank(Some(dir.clone()), &t.a);
        assert_eq!(blank.typing_props().size, Size::Giant);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn font_list_scrolls_with_the_arrows() {
        let mut t = T::new("abc");
        t.press(30, 70);
        assert_eq!(t.w.list(), Some(List::Fonts));
        let [_, down] = List::font_arrows();
        t.press(down.0 + 2, down.1 + 2);
        assert_eq!(t.w.list(), Some(List::Fonts));
        t.press(down.0 + 2, down.1 + 2);
        // 6 fontes, 5 visíveis: a última linha agora é a 6ª (Stencil)
        let (x, y, _, _) = List::Fonts.row_rect(4);
        t.press(x + 3, y + 3);
        assert_eq!(t.w.brush(), Some(Brush::Font("Stencil")));
    }

    #[test]
    fn lists_draw_without_data() {
        let mut t = T::new("abc");
        let mut f = Frame::new(640, 480);
        for more in [false, true] {
            t.w.more = more;
            for k in 0..6 {
                let (b, lists) = t.w.buttons();
                if lists[k].is_some() {
                    let (x, y, _, _) = b[k].rect();
                    t.press(x + 3, y + 3);
                    t.w.draw_front(&t.ed, &mut f, &t.a);
                    t.w.draw_strip(&t.ed, &mut f, &t.a);
                    t.run(|w, ed, cx| {
                        w.key(ed, cx, Key::Escape, false);
                    });
                }
            }
        }
    }
}
