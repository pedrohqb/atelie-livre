//! Escritório do Creative Writer: a página com o editor de texto, a barra de
//! ferramentas, a faixa de opções de cada ferramenta, a barra de baixo e o
//! McZee (`WRITER.PIC`, `WRITER.SND`, balões `WHAT` do `WRITER.EXE`).
//!
//! Organização (contrato em `docs/telas/creative-writer.md`):
//! - [`editor`]: documento, página, digitação, desfazer e gravação;
//! - [`tools`]: as 10 ferramentas da barra e o trait [`ToolStrip`];
//! - um arquivo por ferramenta (`word_strip`, `stickers`, `sounds`,
//!   `page_strip`, `speller`, `egg`, `removers`, `transformers`, `viewer`,
//!   `print`), mais a porta (`exit`);
//! - [`chrome`]: moldura, barra de ferramentas e barra de baixo;
//! - [`mczee`]: balões automáticos e menu do McZee; [`cursors`]: cursores.

mod chrome;
pub mod cursors;
pub mod editor;
mod egg;
mod exit;
pub mod mczee;
mod page_strip;
mod print;
mod removers;
mod sounds;
mod speller;
mod stickers;
pub mod tools;
mod transformers;
mod viewer;
mod word_strip;

use crate::Frame;
use crate::assets::{Assets, CursorRef};
use crate::scene::{DEFAULT_CURSOR, Effect, Input, KidCommon, MouseButton, Scene};
use crate::writer::Document;
use cursors::Pointer;
pub use editor::{Editor, LIBRARIES, MARGIN, PAGE, TEXT_H, TEXT_W};
use mczee::{McZee, Request};
use std::path::PathBuf;
pub use tools::{Cx, ToolId, ToolStrip};
use tools::{SND_TOOL_AGAIN, WHAT_GENERAL, inside, snd};
pub use word_strip::List;

/// Uma instância de cada ferramenta (na ordem da barra).
#[derive(Default)]
pub struct Tools {
    pub word: word_strip::WordStrip,
    pub stickers: stickers::StickersStrip,
    pub sounds: sounds::SoundsStrip,
    pub page: page_strip::PageStrip,
    pub speller: speller::SpellerStrip,
    pub egg: egg::EggStrip,
    pub removers: removers::RemoversStrip,
    pub transformers: transformers::TransformersStrip,
    pub viewer: viewer::ViewerStrip,
    pub print: print::PrintStrip,
}

impl Tools {
    pub fn get(&self, id: ToolId) -> &dyn ToolStrip {
        match id {
            ToolId::Word => &self.word,
            ToolId::Stickers => &self.stickers,
            ToolId::Sounds => &self.sounds,
            ToolId::Page => &self.page,
            ToolId::Speller => &self.speller,
            ToolId::Egg => &self.egg,
            ToolId::Removers => &self.removers,
            ToolId::Transformers => &self.transformers,
            ToolId::Viewer => &self.viewer,
            ToolId::Print => &self.print,
        }
    }

    pub fn get_mut(&mut self, id: ToolId) -> &mut dyn ToolStrip {
        match id {
            ToolId::Word => &mut self.word,
            ToolId::Stickers => &mut self.stickers,
            ToolId::Sounds => &mut self.sounds,
            ToolId::Page => &mut self.page,
            ToolId::Speller => &mut self.speller,
            ToolId::Egg => &mut self.egg,
            ToolId::Removers => &mut self.removers,
            ToolId::Transformers => &mut self.transformers,
            ToolId::Viewer => &mut self.viewer,
            ToolId::Print => &mut self.print,
        }
    }
}

/// Setas ▲▼ da barra de baixo (rolar a vista; hoje trocam de página).
const SND_ARROW_UP: u16 = 0x02bc;
const SND_ARROW_DOWN: u16 = 0x02bd;
/// "Andando por aí e olhando as páginas", na primeira seta usada.
const WHAT_PAGES: usize = 11;

pub struct Writer {
    common: KidCommon,
    ed: Editor,
    tools: Tools,
    tool: ToolId,
    mczee: McZee,
    pointer: Pointer,
    mouse: (i32, i32),
    ctrl: bool,
    shift: bool,
    /// Nome do documento na barra de baixo.
    title: String,
    /// Já entrou (som e balão de boas-vindas).
    started: bool,
    /// Alguma seta da barra de baixo já foi usada (balão só na primeira).
    arrows_used: bool,
}

impl Writer {
    pub fn new(texts: Option<PathBuf>, assets: &Assets) -> Writer {
        Writer {
            common: KidCommon::new(None),
            ed: Editor::new(texts, assets),
            tools: Tools::default(),
            tool: ToolId::Word,
            mczee: McZee::default(),
            pointer: Pointer::default(),
            mouse: (0, 0),
            ctrl: false,
            shift: false,
            title: chrome::DEFAULT_TITLE.to_string(),
            started: false,
            arrows_used: false,
        }
    }

    pub fn doc(&self) -> &Document {
        &self.ed.doc
    }

    pub fn editor(&self) -> &Editor {
        &self.ed
    }

    pub fn tool(&self) -> ToolId {
        self.tool
    }

    pub fn tools(&self) -> &Tools {
        &self.tools
    }

    pub fn page(&self) -> usize {
        self.ed.page
    }

    pub fn mczee(&self) -> &McZee {
        &self.mczee
    }

    /// Cursor do mouse atual.
    pub fn cursor(&self) -> CursorRef {
        self.pointer.current()
    }

    /// Roda um gancho da ferramenta `id` e abre o balão que ela pedir.
    fn with_tool<R>(
        &mut self,
        id: ToolId,
        fx: &mut Vec<Effect>,
        a: &Assets,
        f: impl FnOnce(&mut dyn ToolStrip, &mut Editor, &mut Cx) -> R,
    ) -> R {
        let mut cx = Cx::new(fx, a);
        cx.ctrl = self.ctrl;
        cx.shift = self.shift;
        let r = f(self.tools.get_mut(id), &mut self.ed, &mut cx);
        if let Some((h, forced)) = cx.take_help() {
            self.mczee.show(h, forced, a);
        }
        r
    }

    /// Escolhe a ferramenta `id` na barra: som, aba, balão do McZee.
    fn select(&mut self, id: ToolId, fx: &mut Vec<Effect>, a: &Assets) {
        let again = id == self.tool;
        let def = id.def();
        fx.push(snd(if again { SND_TOOL_AGAIN } else { def.sound }));
        if !again {
            let old = self.tool;
            self.with_tool(old, fx, a, |t, ed, cx| t.deactivate(ed, cx));
            self.tool = id;
        }
        self.mczee.set_wand(false);
        self.mczee.show(def.help, false, a);
        self.with_tool(id, fx, a, |t, ed, cx| t.activate(ed, cx, again));
    }

    /// Pedido do menu do McZee que ainda não tem tela.
    fn request(&mut self, r: Request, fx: &mut Vec<Effect>) {
        let what = match r {
            Request::New => "Novo",
            Request::Open => "Pasta",
            Request::Save => "Salvar",
            Request::Notebook => "Caderno de Idéias",
        };
        fx.push(Effect::Log(format!("{what}: ainda não disponível")));
    }

    fn bottom_arrow(&mut self, delta: i32, fx: &mut Vec<Effect>, a: &Assets) {
        fx.push(snd(if delta < 0 {
            SND_ARROW_UP
        } else {
            SND_ARROW_DOWN
        }));
        if !std::mem::replace(&mut self.arrows_used, true) {
            self.mczee.show((WHAT_GENERAL, WHAT_PAGES), false, a);
        }
        self.ed.set_page(delta);
    }

    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        if let Some(req) = self.mczee.front_click(x, y, fx, a) {
            if let Some(r) = req {
                self.request(r, fx);
            }
            return;
        }
        if self.mczee.wand() && inside(PAGE, x, y) {
            // W11: tocar o som da palavra ou do adesivo
            fx.push(Effect::Log("Varinha Mágica: nenhum som aqui".into()));
            return;
        }
        let tool = self.tool;
        if self.tools.get(tool).modal() {
            self.with_tool(tool, fx, a, |t, ed, cx| t.press(ed, cx, x, y));
            return;
        }
        if let Some(id) = ToolId::at(x, y) {
            self.select(id, fx, a);
            return;
        }
        if mczee::niche_hit(x, y) {
            self.mczee.niche_click(fx, a);
            return;
        }
        if inside(exit::DOOR_RECT, x, y) {
            self.with_tool(tool, fx, a, |_, ed, cx| exit::click(ed, cx));
            return;
        }
        if inside(chrome::arrow_rect(chrome::ARROW_UP), x, y) {
            self.bottom_arrow(-1, fx, a);
            return;
        }
        if inside(chrome::arrow_rect(chrome::ARROW_DOWN), x, y) {
            self.bottom_arrow(1, fx, a);
            return;
        }
        if self.with_tool(tool, fx, a, |t, ed, cx| t.press(ed, cx, x, y)) {
            return;
        }
        self.ed.click_page(x, y);
    }

    /// Cursor que o ponto `(x, y)` pede.
    fn wanted_cursor(&self, a: &Assets) -> CursorRef {
        let (x, y) = self.mouse;
        let tool = self.tools.get(self.tool);
        if self.mczee.wand() {
            cursors::writer(cursors::WAND)
        } else if self.mczee.covers(a, x, y) {
            DEFAULT_CURSOR
        } else if let Some(c) = tool.cursor(&self.ed, x, y) {
            c
        } else if inside(PAGE, x, y) && !tool.modal() {
            cursors::writer(cursors::TEXT)
        } else {
            DEFAULT_CURSOR
        }
    }
}

impl Scene for Writer {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        if !std::mem::replace(&mut self.started, true) {
            fx.push(snd(mczee::SND_ENTER));
            self.mczee.show(mczee::WELCOME, false, assets);
        }
        for i in input {
            match *i {
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.mouse = (x, y);
                    self.click(x, y, &mut fx, assets);
                }
                Input::MouseMove { x, y } => {
                    self.mouse = (x, y);
                    let t = self.tool;
                    self.with_tool(t, &mut fx, assets, |t, ed, cx| t.motion(ed, cx, x, y));
                }
                Input::MouseUp {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.mouse = (x, y);
                    let t = self.tool;
                    self.with_tool(t, &mut fx, assets, |t, ed, cx| t.release(ed, cx, x, y));
                }
                Input::Key {
                    key,
                    ctrl,
                    alt: false,
                } => {
                    let t = self.tool;
                    let done =
                        self.with_tool(t, &mut fx, assets, |t, ed, cx| t.key(ed, cx, key, ctrl));
                    if !done && !ctrl {
                        self.ed.key(key);
                    }
                }
                Input::Text(ref t) => self.ed.type_text(t),
                Input::Shift(on) => self.shift = on,
                Input::Ctrl(on) => self.ctrl = on,
                _ => {}
            }
        }
        self.ed.tick(dt);
        let t = self.tool;
        if dt.is_finite() {
            self.tools.get_mut(t).tick(&mut self.ed, dt.max(0.0));
        }
        self.ed.caret_visible = !self.tools.get(t).modal();
        let want = self.wanted_cursor(assets);
        if let Some(e) = self.pointer.set(want) {
            fx.push(e);
        }
        fx
    }

    fn draw(&self, f: &mut Frame, a: &Assets) {
        let tool = self.tools.get(self.tool);
        chrome::draw_frame(f);
        chrome::draw_toolbar(f, a, self.tool);
        self.mczee.draw_niche(f, a);
        exit::draw(f, a);
        tool.draw_strip(&self.ed, f, a);
        self.ed.draw_page(f);
        chrome::draw_bottom(f, a, &self.ed, &self.title);
        tool.draw_front(&self.ed, f, a);
        self.mczee.draw(f, a);
    }

    fn leave(&mut self) -> Vec<Effect> {
        vec![Effect::Log(self.ed.save())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Program;
    use crate::scene::Key;
    use crate::scenes::testutil::{assets, empty_assets};
    use crate::ui::center;
    use crate::writer::legacy::doc::Run;
    use crate::writer::{Align, Item, Size};
    use crate::{SCREEN_H, SCREEN_W};
    use word_strip::WordStrip;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            ctrl: false,
            alt: false,
        }
    }

    fn text(s: &str) -> Input {
        Input::Text(s.into())
    }

    /// Clique no meio da ferramenta `id` da barra.
    fn tool(id: ToolId) -> Input {
        let (x, y) = center(id.def().hit);
        click(x, y)
    }

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(crate::assets::SoundRef::Snd(Program::Writer, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// Pixels não brancos dentro da página.
    fn inked(w: &Writer, a: &Assets) -> usize {
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, a);
        let mut n = 0;
        for y in PAGE.1..PAGE.1 + PAGE.3 {
            for x in PAGE.0..PAGE.0 + PAGE.2 {
                if f.pixel(x as u32, y as u32) != [255, 255, 255, 255] {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn typing_shows_text_on_the_page() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        // cursor apagado (meio período) para contar só o texto
        w.update(&[], 0.6, &a);
        assert_eq!(inked(&w, &a), 0);
        w.update(&[text("Oi")], 0.0, &a);
        w.update(&[], 0.6, &a);
        assert_eq!(w.doc().para().text(), "Oi");
        assert!(inked(&w, &a) > 20);
        // o cursor aceso acrescenta uma linha de 2 px
        w.update(&[], 0.5, &a);
        assert!(inked(&w, &a) > 20 + 32);
    }

    #[test]
    fn entering_plays_the_welcome_sound_once() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        assert_eq!(sounds(&w.update(&[], 0.0, &a)), vec![mczee::SND_ENTER]);
        assert!(sounds(&w.update(&[], 0.0, &a)).is_empty());
    }

    #[test]
    fn enter_makes_a_new_paragraph_and_egg_undoes() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[text("a"), key(Key::Enter), text("b")], 0.0, &a);
        assert_eq!(w.doc().paras.len(), 2);
        assert_eq!(w.doc().paras[1].text(), "b");
        w.update(&[click(280, 30)], 0.0, &a); // ovo
        assert_eq!(w.tool(), ToolId::Egg);
        assert_eq!(w.doc().paras.len(), 2);
        assert_eq!(w.doc().paras[1].text(), "");
        w.update(&[click(280, 30)], 0.0, &a);
        assert_eq!(w.doc().paras[1].text(), "b");
        // DESFAZER da faixa também alterna
        w.update(&[click(60, 75)], 0.0, &a);
        assert_eq!(w.doc().paras[1].text(), "");
    }

    #[test]
    fn size_list_changes_the_run_style() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[click(100, 70)], 0.0, &a);
        assert_eq!(w.tools().word.list(), Some(List::Sizes));
        // primeiro item: Gigante
        let r = WordStrip::list_item_rect(List::Sizes, 0);
        w.update(&[click(r.0 + 5, r.1 + 5), text("a")], 0.0, &a);
        assert_eq!(w.tools().word.list(), None);
        assert_eq!(w.doc().style.size, Size::Giant);
        let runs = w.doc().para().runs();
        assert!(matches!(&runs[0], Run::Text { style, .. } if style.size == Size::Giant));
        // clique fora fecha a lista sem escolher
        w.update(&[click(100, 70), click(300, 300)], 0.0, &a);
        assert_eq!(w.tools().word.list(), None);
        assert_eq!(w.doc().style.size, Size::Giant);
        // Esc também fecha
        w.update(&[click(100, 70), key(Key::Escape)], 0.0, &a);
        assert_eq!(w.tools().word.list(), None);
    }

    #[test]
    fn open_list_takes_clicks_before_the_toolbar() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[click(100, 70)], 0.0, &a);
        // clicar numa ferramenta com a lista aberta só fecha a lista
        w.update(&[tool(ToolId::Stickers)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Word);
        assert_eq!(w.tools().word.list(), None);
    }

    #[test]
    fn color_and_style_lists_apply() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[click(170, 70)], 0.0, &a);
        let r = WordStrip::list_item_rect(List::Colors, 3);
        w.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert_eq!(w.doc().style.color, crate::paint::COLORS[3]);
        w.update(&[click(230, 70)], 0.0, &a);
        let r = WordStrip::list_item_rect(List::Styles, 1);
        w.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert!(w.doc().style.bold);
        w.update(&[click(300, 70)], 0.0, &a);
        let r = WordStrip::list_item_rect(List::Align, 2);
        w.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert_eq!(w.doc().para().align, Align::Center);
    }

    #[test]
    fn sticker_without_data_uses_placeholder() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[click(80, 30)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Stickers);
        assert_eq!(w.editor().rolls.len(), 0);
        // sem rolos: cliques na faixa e nas setas não fazem nada
        w.update(&[click(40, 70), click(515, 60), click(515, 90)], 0.0, &a);
        assert!(!w.tools().stickers.film_open());
        w.ed.insert_sticker(5, 2);
        assert!(matches!(
            w.doc().para().items[0],
            Item::Sticker(s) if s.w == 32 && s.h == 32
        ));
        assert!(inked(&w, &a) > 60);
    }

    #[test]
    fn removers_explosion_clears_with_undo() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[text("abc"), click(330, 30)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Removers);
        let (x, y) = center(removers::EXPLOSION.rect());
        assert_eq!((x, y), (457, 77));
        let fx = w.update(&[click(x, y)], 0.0, &a);
        assert!(w.doc().is_empty());
        assert!(sounds(&fx).contains(&0x025f));
        w.update(&[click(280, 30)], 0.0, &a);
        assert_eq!(w.doc().para().text(), "abc");
    }

    #[test]
    fn cursor_keys_and_click_move_the_cursor() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[text("ab"), key(Key::Left), text("x")], 0.0, &a);
        assert_eq!(w.doc().para().text(), "axb");
        w.update(
            &[key(Key::Home), text("1"), key(Key::End), text("2")],
            0.0,
            &a,
        );
        assert_eq!(w.doc().para().text(), "1axb2");
        w.update(&[key(Key::Enter), text("c"), key(Key::Up)], 0.0, &a);
        assert_eq!(w.doc().cursor.para, 0);
        w.update(&[key(Key::Down)], 0.0, &a);
        assert_eq!(w.doc().cursor.para, 1);
        // clique no começo da primeira linha
        w.update(&[click(PAGE.0 + MARGIN, PAGE.1 + MARGIN + 2)], 0.0, &a);
        assert_eq!(w.doc().cursor, crate::writer::Cursor { para: 0, index: 0 });
        w.update(&[key(Key::Backspace)], 0.0, &a);
        assert_eq!(w.doc().para().text(), "1axb2");
    }

    #[test]
    fn pages_follow_the_cursor_and_arrows() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        // 16 px por linha (8×8 reserva ×2): 301 px cabem 18 linhas
        let mut inputs = Vec::new();
        for _ in 0..25 {
            inputs.push(text("a"));
            inputs.push(key(Key::Enter));
        }
        w.update(&inputs, 0.0, &a);
        assert_eq!(w.page(), 1);
        w.update(&[key(Key::PageUp)], 0.0, &a);
        assert_eq!(w.page(), 0);
        let (dx, dy) = center(chrome::arrow_rect(chrome::ARROW_DOWN));
        let (ux, uy) = center(chrome::arrow_rect(chrome::ARROW_UP));
        let fx = w.update(&[click(dx, dy)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_ARROW_DOWN]);
        assert_eq!(w.page(), 1);
        w.update(&[click(dx, dy)], 0.0, &a);
        assert_eq!(w.page(), 1);
        w.update(&[click(ux, uy)], 0.0, &a);
        assert_eq!(w.page(), 0);
        w.update(&[text("b")], 0.0, &a);
        assert_eq!(w.page(), 1);
    }

    #[test]
    fn leave_saves_and_reload_restores() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut w = Writer::new(Some(dir.path().to_path_buf()), &a);
        w.update(&[text("Olá"), key(Key::Enter), text("mundo")], 0.0, &a);
        w.ed.insert_sticker(1, 0);
        assert!(matches!(&w.leave()[..], [Effect::Log(s)] if s.contains("guardado em")));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
        let again = Writer::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(again.doc().paras.len(), 2);
        assert_eq!(again.doc().paras[0].text(), "Olá");
        assert_eq!(again.doc().paras[1].text(), "mundo");
        assert!(matches!(
            again.doc().paras[1].items[5],
            Item::Sticker(s) if s.roll == 1 && s.w == 32
        ));
        let mut untouched = Writer::new(Some(dir.path().to_path_buf()), &a);
        untouched.leave();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn leave_without_dir_logs() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        assert!(matches!(&w.leave()[..], [Effect::Log(s)] if s.contains("não guardado")));
    }

    #[test]
    fn every_tool_is_selectable_with_its_sound_and_strip_buttons_log() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[], 0.0, &a);
        assert!(w.update(&[click(600, 60)], 0.0, &a).contains(&Effect::Quit));
        // a Palavra já está ativa: começa pela segunda e termina nela
        for id in ToolId::ALL.into_iter().cycle().skip(1).take(10) {
            let fx = w.update(&[tool(id)], 0.0, &a);
            assert_eq!(w.tool(), id);
            assert!(sounds(&fx).contains(&id.def().sound), "{id:?}");
        }
        // de novo na mesma: som de reabrir o balão
        let fx = w.update(&[tool(ToolId::Word)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_TOOL_AGAIN]);
        // botão de faixa ainda sem função: som de opção e registro
        w.update(&[tool(ToolId::Sounds)], 0.0, &a);
        let fx = w.update(&[click(470, 70)], 0.0, &a);
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::Log(s) if s.contains("Gravador")))
        );
        assert_eq!(sounds(&fx), vec![tools::SND_OPTION]);
        // o interruptor troca a página da faixa da Preparação da página
        w.update(&[tool(ToolId::Page)], 0.0, &a);
        let fx = w.update(&[click(515, 70), click(150, 75)], 0.0, &a);
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::Log(s) if s.contains("Borda")))
        );
    }

    #[test]
    fn mczee_niche_opens_the_menu_and_dismiss_turns_help_off() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        w.update(&[], 0.0, &a);
        let fx = w.update(&[click(556, 80)], 0.0, &a);
        assert!(w.mczee().menu_open());
        assert_eq!(sounds(&fx), vec![0x02ce]);
        // cara do McZee: "vou embora?"; a primeira bolinha desliga a ajuda
        w.update(&[click(556, 120)], 0.0, &a);
        assert!(!w.mczee().menu_open());
        assert_eq!(w.mczee().talk(), Some(mczee::Talk::Dismiss));
        assert!(w.mczee().help_on());
        // pedidos do menu sem tela ainda: registro
        w.update(&[click(300, 300), click(556, 80)], 0.0, &a);
        let fx = w.update(&[click(556, 300)], 0.0, &a);
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::Log(s) if s.contains("Caderno")))
        );
        w.update(&[click(556, 80)], 0.0, &a);
        let fx = w.update(&[click(556, 430)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(crate::scene::SceneId::BookStory)));
        // Varinha: cursor de varinha até escolher uma ferramenta
        w.update(&[click(556, 80), click(556, 345)], 0.0, &a);
        assert!(w.mczee().wand());
        assert_eq!(w.cursor(), cursors::writer(cursors::WAND));
        w.update(&[tool(ToolId::Word)], 0.0, &a);
        assert!(!w.mczee().wand());
    }

    #[test]
    fn cursor_is_text_over_the_page_and_default_elsewhere() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new(None, &a);
        let fx = w.update(&[Input::MouseMove { x: 300, y: 300 }], 0.0, &a);
        let text_cursor = cursors::writer(cursors::TEXT);
        assert!(fx.contains(&Effect::SetCursor(text_cursor)));
        // sem repetir o mesmo cursor
        let fx = w.update(&[Input::MouseMove { x: 310, y: 300 }], 0.0, &a);
        assert!(!fx.iter().any(|e| matches!(e, Effect::SetCursor(_))));
        let fx = w.update(&[Input::MouseMove { x: 300, y: 30 }], 0.0, &a);
        assert!(fx.contains(&Effect::SetCursor(DEFAULT_CURSOR)));
        // lista aberta: a página não mostra o cursor de texto
        w.update(
            &[click(100, 70), Input::MouseMove { x: 300, y: 300 }],
            0.0,
            &a,
        );
        assert_eq!(w.cursor(), DEFAULT_CURSOR);
    }

    #[test]
    fn real_assets_balloons_follow_tools_and_options() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new(None, &a);
        w.update(&[], 0.0, &a);
        // boas-vindas: "Oi! Aqui é o McZee!"
        assert_eq!(w.mczee().help(), Some(mczee::WELCOME));
        // os cursores usados existem no WRITER.EXE
        for c in [cursors::TEXT, cursors::WAND, cursors::BRUSH, cursors::CUT] {
            assert!(a.cursor(&cursors::writer(c)).is_some(), "{c:x}");
        }
        for id in ToolId::ALL {
            w.update(&[tool(id)], 0.0, &a);
            assert_eq!(w.mczee().help(), Some(id.def().help), "{id:?}");
        }
        // opção da faixa: Dimensiona-palavras = 0fa0[4]
        w.update(&[tool(ToolId::Word), click(100, 70)], 0.0, &a);
        assert_eq!(w.mczee().help(), Some((tools::WHAT_WORD, 4)));
        w.update(&[key(Key::Escape)], 0.0, &a);
        // "dê o fora": a primeira bolinha desliga a ajuda
        w.update(&[click(300, 300), click(556, 80), click(556, 120)], 0.0, &a);
        assert_eq!(w.mczee().talk(), Some(mczee::Talk::Dismiss));
        let (x, y) = center(w.mczee().talk_option(&a, 0).unwrap());
        w.update(&[click(x, y)], 0.0, &a);
        assert!(!w.mczee().help_on());
        // sem o McZee, escolher ferramenta não abre balão
        w.update(&[tool(ToolId::Viewer)], 0.0, &a);
        assert_eq!(w.mczee().help(), None);
        // o nicho vazio pergunta se ele deve voltar; "Sim, por favor!"
        w.update(&[click(556, 80)], 0.0, &a);
        assert_eq!(w.mczee().talk(), Some(mczee::Talk::Return));
        let (x, y) = center(w.mczee().talk_option(&a, 0).unwrap());
        w.update(&[click(x, y), tool(ToolId::Print)], 0.0, &a);
        assert!(w.mczee().help_on());
        assert_eq!(w.mczee().help(), Some(ToolId::Print.def().help));
    }

    #[test]
    fn real_assets_chrome_draws_icons_and_text() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new(None, &a);
        assert!(!w.editor().rolls.is_empty());
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        // ícone da Palavra, em cima da aba clara
        assert_ne!(f.pixel(30, 30), [190, 190, 190, 255]);
        // adesivo real: tamanho do desenho e tinta preta na página
        w.update(&[click(80, 30), click(40, 70)], 0.0, &a);
        assert!(w.tools().stickers.film_open());
        let c = w.tools().stickers.film().cell(0);
        w.update(&[click(c.0 + 5, c.1 + 5)], 0.0, &a);
        assert!(!w.tools().stickers.film_open());
        assert!(matches!(w.doc().para().items[0], Item::Sticker(s) if s.w != 32 || s.h != 32));
        w.update(&[], 0.6, &a);
        assert!(inked(&w, &a) > 0);
        // texto na "Pablo Graphos" de 16 pontos, num parágrafo só de texto
        w.update(&[key(Key::Enter), text("Olá")], 0.0, &a);
        let l = w.editor().layout();
        assert_eq!(l.lines[1].height, a.font(16).unwrap().height as i32);
    }

    /// Com `ATELIE_DUMP`, grava a tela com a faixa de cada ferramenta e o
    /// menu do McZee (para comparar com `cw/shots/office_*.png`).
    #[test]
    fn real_assets_dump_every_strip() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new(None, &a);
        w.update(&[], 0.0, &a);
        for id in ToolId::ALL {
            w.update(&[tool(id)], 0.0, &a);
            let mut f = Frame::new(SCREEN_W, SCREEN_H);
            w.draw(&mut f, &a);
            crate::scenes::testutil::dump(&f, &format!("writer_{:?}", id).to_lowercase());
        }
        w.update(&[click(300, 300), click(556, 80)], 0.0, &a);
        assert!(w.mczee().menu_open());
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "writer_menu");
    }
}
