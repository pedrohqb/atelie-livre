//! Re-movedores, cola e tesoura (W12, `office-ferramentas.md` §6): na
//! faixa, Recortar, Copiar, Colar, Remove-sons e Remove-figuras (o "borrão"
//! verde), Ponto de inserção e Jogar tudo fora (explosão `0845`).
//!
//! Cada botão escolhe um modo (o original não destaca o botão: só o cursor
//! muda sobre a página) e a ação vem no clique na página:
//!
//! - **Recortar** / **Copiar**: arrastar sobre as palavras (ao soltar, a
//!   seleção vai para a área de transferência) ou clicar num adesivo ou
//!   palavra com forma; o texto vai também para a área de transferência da
//!   sessão (a do Caderno de Idéias, `writer::ideas::session::clip`);
//! - **Colar**: cada clique cola onde foi clicado (o texto no ponto de
//!   inserção, o objeto centrado no clique); sem nada copiado, `0805[19]`;
//! - **Remove-sons**: tira o som do adesivo ou da palavra clicada;
//! - **Remove-figuras**: tira o adesivo ou a palavra com forma clicada;
//! - **Ponto de inserção**: volta ao clique normal (põe o cursor de texto);
//! - **Jogar tudo fora**: o clique na página apaga texto e objetos
//!   (desfazível, som `025f`).
//!
//! O McZee explica a ferramenta nas três primeiras vezes que ela é
//! escolhida, mesmo com a ajuda desligada ([`Cx::help_even_off`]).

use super::Writer;
use super::cursors;
use super::editor::{Editor, PAGE};
use super::tools::{
    Cx, HelpRef, SND_OPTION, StripButton, ToolId, ToolStrip, WHAT_REMOVERS, inside, strip_button,
};
use crate::Frame;
use crate::assets::{Assets, CursorRef};
use crate::scene::Effect;
use crate::writer::object::ObjectKind;
use crate::writer::selection::Clip;

const W: u16 = WHAT_REMOVERS;

/// Botões do borrão e o Ponto de inserção, na ordem de [`Mode::STRIP`].
const STRIP: [StripButton; 6] = [
    strip_button(0x0839, Some(0x083b), (58, 54, 56, 48), (W, 1), "Recortar"),
    strip_button(0x083d, Some(0x083f), (114, 54, 37, 48), (W, 2), "Copiar"),
    strip_button(0x0841, Some(0x0843), (151, 54, 44, 48), (W, 3), "Colar"),
    strip_button(
        0x0849,
        Some(0x084b),
        (195, 54, 45, 48),
        (W, 6),
        "Remove-sons",
    ),
    strip_button(
        0x0851,
        Some(0x0853),
        (240, 54, 66, 48),
        (W, 7),
        "Remove-figuras",
    ),
    strip_button(
        0x0831,
        Some(0x0833),
        (356, 61, 29, 34),
        (W, 13),
        "Ponto de inserção",
    ),
];

/// Jogar tudo fora: a explosão.
pub const EXPLOSION: StripButton = strip_button(
    0x0845,
    Some(0x0847),
    (435, 55, 45, 45),
    (W, 9),
    "Jogar tudo fora",
);

/// Sons das ações (`office-ferramentas.md` §6).
pub const SND_CUT: u16 = 0x022f;
pub const SND_COPY: u16 = 0x0288;
pub const SND_PASTE: u16 = 0x028d;
pub const SND_ALL_GONE: u16 = 0x025f;
/// Balões depois da ação: "Onde foi parar?", "Click!", depois de colar.
pub const HELP_CUT_DONE: HelpRef = (W, 4);
pub const HELP_COPY_DONE: HelpRef = (W, 5);
pub const HELP_PASTE_DONE: HelpRef = (W, 14);
/// "Não há nada para colar" (mensagens do editor, `0805[19]`).
pub const HELP_NOTHING_TO_PASTE: HelpRef = (0x0805, 19);
/// Vezes que o McZee explica a ferramenta mesmo desligado.
pub const FORCED_HELP_TIMES: u8 = 3;

/// O que o clique na página faz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Cut,
    Copy,
    Paste,
    RemoveSounds,
    RemovePictures,
    AllGone,
}

impl Mode {
    /// Modo de cada botão de [`STRIP`] (`None` = Ponto de inserção).
    const STRIP: [Option<Mode>; 6] = [
        Some(Mode::Cut),
        Some(Mode::Copy),
        Some(Mode::Paste),
        Some(Mode::RemoveSounds),
        Some(Mode::RemovePictures),
        None,
    ];

    fn cursor(self) -> u16 {
        match self {
            Mode::Cut => cursors::CUT,
            Mode::Copy => cursors::COPY,
            Mode::Paste => cursors::PASTE,
            Mode::RemoveSounds => cursors::SOUND,
            Mode::RemovePictures => cursors::PICTURE,
            Mode::AllGone => cursors::ALL_GONE,
        }
    }
}

/// Pedido à cena para a área de transferência (só o [`Writer`] conhece a da
/// sessão do usuário).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipRequest {
    /// O editor copiou texto: vai também para a da sessão.
    Shared,
    /// Colar no ponto da tela.
    PasteAt(i32, i32),
}

#[derive(Default)]
pub struct RemoversStrip {
    mode: Option<Mode>,
    /// Arrastando sobre as palavras para recortar ou copiar.
    dragging: bool,
    /// Vezes que a ferramenta foi escolhida.
    uses: u8,
}

impl RemoversStrip {
    pub fn mode(&self) -> Option<Mode> {
        self.mode
    }

    /// Recorta ou copia o que a seleção tem.
    fn take_selection(&self, ed: &mut Editor, cx: &mut Cx, cut: bool) {
        let ok = if cut { ed.cut() } else { ed.copy() };
        if ok {
            cx.sound(if cut { SND_CUT } else { SND_COPY });
            cx.help(if cut { HELP_CUT_DONE } else { HELP_COPY_DONE });
            cx.clip(ClipRequest::Shared);
        }
    }

    /// Recorta ou copia o objeto `k`.
    fn take_object(ed: &mut Editor, cx: &mut Cx, k: usize, cut: bool) {
        let doc = ed.doc().clone();
        if !ed.clipboard.copy_object(&doc, k) {
            return;
        }
        if cut {
            ed.remove_object(k);
        }
        cx.sound(if cut { SND_CUT } else { SND_COPY });
        cx.help(if cut { HELP_CUT_DONE } else { HELP_COPY_DONE });
    }

    /// Clique na página com o modo `m`.
    fn page_press(&mut self, m: Mode, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) {
        let object = ed
            .object_at(x, y)
            .filter(|&k| ed.doc().objects[k].kind != ObjectKind::PageFrame);
        match m {
            Mode::Cut | Mode::Copy => match object {
                Some(k) => Self::take_object(ed, cx, k, m == Mode::Cut),
                None => {
                    ed.press_page(x, y, false);
                    self.dragging = true;
                }
            },
            Mode::Paste => cx.clip(ClipRequest::PasteAt(x, y)),
            Mode::RemoveSounds => match object {
                Some(k) => {
                    if ed.doc().objects[k].sound.is_some() {
                        ed.edit(|d, _| d.remove_object_sound(k));
                    }
                }
                None => {
                    remove_word_sound(ed, x, y);
                }
            },
            Mode::RemovePictures => {
                if let Some(k) = object {
                    ed.remove_object(k);
                }
            }
            Mode::AllGone => {
                if ed.clear_all() {
                    cx.sound(SND_ALL_GONE);
                }
            }
        }
    }
}

/// Remove-sons numa palavra: tira os sons de palavra (`SNDT`) que cobrem o
/// caractere clicado. Verdadeiro se havia.
pub fn remove_word_sound(ed: &mut Editor, x: i32, y: i32) -> bool {
    let Some(p) = ed.to_page(x, y) else {
        return false;
    };
    let pos = ed.layout().pos_at(p.page, p.x, p.y);
    let cp = crate::writer::convert::cp_of(ed.doc(), pos);
    let before = ed.extra.word_sounds.len();
    ed.extra
        .word_sounds
        .retain(|w| !(w.start <= cp && cp < w.end.max(w.start + 1)));
    ed.extra.word_sounds.len() != before
}

impl ToolStrip for RemoversStrip {
    fn id(&self) -> ToolId {
        ToolId::Removers
    }

    fn activate(&mut self, _ed: &mut Editor, cx: &mut Cx, again: bool) {
        if !again && self.uses < FORCED_HELP_TIMES {
            self.uses += 1;
            cx.help_even_off(ToolId::Removers.def().help);
        }
    }

    fn deactivate(&mut self, _ed: &mut Editor, _cx: &mut Cx) {
        self.mode = None;
        self.dragging = false;
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        let chosen = if inside(EXPLOSION.rect(), x, y) {
            Some((&EXPLOSION, Some(Mode::AllGone)))
        } else {
            StripButton::find(&STRIP, x, y).map(|k| (&STRIP[k], Mode::STRIP[k]))
        };
        if let Some((b, mode)) = chosen {
            cx.sound(SND_OPTION);
            if let Some(h) = b.help {
                cx.help(h);
            }
            self.mode = mode;
            return true;
        }
        match self.mode {
            Some(m) if inside(PAGE, x, y) => {
                self.page_press(m, ed, cx, x, y);
                true
            }
            _ => false,
        }
    }

    fn release(&mut self, ed: &mut Editor, cx: &mut Cx, _x: i32, _y: i32) {
        if !std::mem::take(&mut self.dragging) {
            return;
        }
        if !ed.sel.is_empty() {
            self.take_selection(ed, cx, self.mode == Some(Mode::Cut));
        }
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
        EXPLOSION.pic.draw(f, a);
    }

    fn cursor(&self, _ed: &Editor, x: i32, y: i32) -> Option<CursorRef> {
        let m = self.mode?;
        inside(PAGE, x, y).then(|| cursors::writer(m.cursor()))
    }
}

impl Writer {
    /// Pedido dos Re-movedores para a área de transferência.
    pub(super) fn clip_request(&mut self, r: ClipRequest, fx: &mut Vec<Effect>, a: &Assets) {
        match r {
            ClipRequest::Shared => self.share_clip(),
            ClipRequest::PasteAt(x, y) => {
                let pasted = match &self.ed.clipboard.clip {
                    Some(Clip::Object(_)) => self.paste_object_at(x, y),
                    _ => {
                        self.ed.press_page(x, y, false);
                        self.ed.release_page();
                        self.paste()
                    }
                };
                if pasted {
                    fx.push(super::tools::snd(SND_PASTE));
                    self.mczee.show(HELP_PASTE_DONE, false, a);
                } else {
                    self.mczee.show(HELP_NOTHING_TO_PASTE, false, a);
                }
            }
        }
    }

    /// Cola o objeto da área de transferência centrado no ponto da tela.
    fn paste_object_at(&mut self, x: i32, y: i32) -> bool {
        let Some(p) = self.ed.to_page(x, y) else {
            return false;
        };
        let Some(Clip::Object(o)) = self.ed.clipboard.clip.clone() else {
            return false;
        };
        let mut o = o;
        o.page = p.page as u16;
        o.x = (p.x - i32::from(o.width) / 2).clamp(-(i32::from(o.width)) + 8, 32767) as i16;
        o.y = (p.y - i32::from(o.height) / 2).clamp(-(i32::from(o.height)) + 8, 32767) as i16;
        self.ed.add_object(o, None);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Input, Key, MouseButton, Scene};
    use crate::scenes::testutil::empty_assets;
    use crate::ui::center;
    use crate::writer::object::{FloatObject, ObjRef};

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn up(x: i32, y: i32) -> Input {
        Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(crate::assets::SoundRef::Snd(_, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    fn text(w: &Writer) -> String {
        w.doc().paras[0]
            .items
            .iter()
            .filter_map(|i| match i {
                crate::writer::doc::Item::Char(c, _) => Some(*c),
                _ => None,
            })
            .collect()
    }

    fn removers(a: &Assets, typed: &str) -> Writer {
        let mut w = Writer::new_blank(None, a).with_user("ana");
        w.update(&[Input::Text(typed.into()), click(330, 30)], 0.0, a);
        assert_eq!(w.tool(), ToolId::Removers);
        w
    }

    fn mode_button(k: usize) -> (i32, i32) {
        center(STRIP[k].rect())
    }

    /// Ponto da tela do caractere `k` do primeiro parágrafo.
    fn char_point(w: &Writer, k: usize) -> (i32, i32) {
        let pos = crate::writer::doc::Pos { para: 0, index: k };
        let c = w.editor().layout().caret(pos);
        w.editor().to_screen(c.page, c.x + 1, c.top + 3)
    }

    #[test]
    fn explosion_is_chosen_then_clicked_on_the_page_with_undo() {
        let (_d, a) = empty_assets();
        let mut w = removers(&a, "abc");
        let (x, y) = center(EXPLOSION.rect());
        assert_eq!((x, y), (457, 77));
        w.update(&[click(x, y)], 0.0, &a);
        assert_eq!(text(&w), "abc");
        assert_eq!(w.tools().removers.mode(), Some(Mode::AllGone));
        assert_eq!(
            w.tools().removers.cursor(w.editor(), 300, 300),
            Some(cursors::writer(cursors::ALL_GONE))
        );
        let fx = w.update(&[click(300, 300)], 0.0, &a);
        assert!(w.doc().is_empty());
        assert!(sounds(&fx).contains(&SND_ALL_GONE));
        w.update(
            &[Input::Key {
                key: Key::Z,
                ctrl: true,
                alt: false,
            }],
            0.0,
            &a,
        );
        assert_eq!(text(&w), "abc");
    }

    #[test]
    fn cut_by_dragging_over_words_then_paste_where_clicked() {
        let (_d, a) = empty_assets();
        let mut w = removers(&a, "abcdef");
        let (bx, by) = mode_button(0);
        w.update(&[click(bx, by)], 0.0, &a);
        let (x0, y0) = char_point(&w, 1);
        let (x1, y1) = char_point(&w, 3);
        let fx = w.update(
            &[click(x0, y0), Input::MouseMove { x: x1, y: y1 }, up(x1, y1)],
            0.0,
            &a,
        );
        assert_eq!(text(&w), "adef");
        assert!(sounds(&fx).contains(&SND_CUT));
        // vai também para a área de transferência da sessão
        assert!(crate::writer::ideas::session::clip("ana").is_some());
        // Colar no fim, duas vezes
        let (px, py) = mode_button(2);
        w.update(&[click(px, py)], 0.0, &a);
        let end = char_point(&w, 4);
        let fx = w.update(&[click(end.0 + 2, end.1)], 0.0, &a);
        assert!(sounds(&fx).contains(&SND_PASTE));
        assert_eq!(text(&w), "adefbc");
        let end = char_point(&w, 6);
        w.update(&[click(end.0 + 2, end.1)], 0.0, &a);
        assert_eq!(text(&w), "adefbcbc");
    }

    #[test]
    fn copy_keeps_the_text_and_insertion_point_goes_back_to_typing() {
        let (_d, a) = empty_assets();
        let mut w = removers(&a, "abc");
        let (bx, by) = mode_button(1);
        w.update(&[click(bx, by)], 0.0, &a);
        let (x0, y0) = char_point(&w, 0);
        let (x1, y1) = char_point(&w, 2);
        let fx = w.update(
            &[click(x0, y0), Input::MouseMove { x: x1, y: y1 }, up(x1, y1)],
            0.0,
            &a,
        );
        assert_eq!(text(&w), "abc");
        assert!(sounds(&fx).contains(&SND_COPY));
        let (ix, iy) = mode_button(5);
        w.update(&[click(ix, iy)], 0.0, &a);
        assert_eq!(w.tools().removers.mode(), None);
        assert_eq!(w.tools().removers.cursor(w.editor(), 300, 300), None);
    }

    #[test]
    fn paste_with_nothing_copied_says_so() {
        let (_d, a) = empty_assets();
        let mut w = removers(&a, "abc");
        let (bx, by) = mode_button(2);
        w.update(&[click(bx, by)], 0.0, &a);
        let fx = w.update(&[click(300, 300)], 0.0, &a);
        assert!(!sounds(&fx).contains(&SND_PASTE));
        assert_eq!(text(&w), "abc");
    }

    fn with_sticker(w: &mut Writer) -> (i32, i32) {
        let mut o = FloatObject::new(ObjectKind::Sticker, ObjRef::pict(0), 0, 100, 300, 60, 60);
        o.sound = Some(ObjRef::snd(3));
        w.editor_mut().add_object(o, None);
        w.editor().to_screen(0, 130, 330)
    }

    #[test]
    fn objects_are_cut_pasted_and_removed() {
        let (_d, a) = empty_assets();
        let mut w = removers(&a, "abc");
        let (sx, sy) = with_sticker(&mut w);
        assert_eq!(w.editor().object_at(sx, sy), Some(0));
        // Remove-sons
        let (bx, by) = mode_button(3);
        w.update(&[click(bx, by), click(sx, sy)], 0.0, &a);
        assert_eq!(w.doc().objects[0].sound, None);
        // Recortar o adesivo e colar duas vezes
        let (bx, by) = mode_button(0);
        let fx = w.update(&[click(bx, by), click(sx, sy)], 0.0, &a);
        assert!(w.doc().objects.is_empty());
        assert!(sounds(&fx).contains(&SND_CUT));
        let (bx, by) = mode_button(2);
        w.update(&[click(bx, by), click(sx, sy), click(sx + 50, sy)], 0.0, &a);
        assert_eq!(w.doc().objects.len(), 2);
        assert_eq!(w.doc().objects[0].width, 60);
        // Remove-figuras
        let (bx, by) = mode_button(4);
        w.update(&[click(bx, by), click(sx + 50, sy)], 0.0, &a);
        assert_eq!(w.doc().objects.len(), 1);
    }

    #[test]
    fn help_is_forced_the_first_three_times_only() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        let mut cx_fx = Vec::new();
        let mut cx = Cx::new(&mut cx_fx, &a);
        let mut r = RemoversStrip::default();
        let mut ed = Editor::blank(None, &a);
        for k in 0..5 {
            r.activate(&mut ed, &mut cx, false);
            let forced = cx.take_help().map(|(_, f)| f);
            assert_eq!(forced, (k < 3).then_some(true), "{k}");
        }
        r.activate(&mut ed, &mut cx, true);
        assert_eq!(cx.take_help(), None);
        let _ = w.update(&[], 0.0, &a);
    }
}
