//! Seleciona-sons (`office-ferramentas.md` §2): a mesma ferramenta da
//! pintura (`scenes::sounds`: três teatros de sons por página, lista de
//! nomes, volume, Cachorrinho, Gravador e setas), com a faixa do Escritório
//! ([`LAYOUT`]) e ligada ao documento.
//!
//! O som escolhido fica "na mão" (balão `0fa2[3]`) e vai para a palavra, o
//! adesivo ou a palavra com forma clicada: o documento ganha uma cópia do
//! som (chunk `SND n` com o número da biblioteca, ou um número livre para os
//! sons do Cachorrinho e do Gravador), a palavra o guarda no
//! [`CharProps::sound`](crate::writer::doc::CharProps) (tabela `SNDT` do
//! `.MAX`) e o objeto no campo `sound` (sprm `0x37`). "Sem Som" tira o
//! som; o número de página não aceita som (`0805[18]`). Sem nada na mão, o
//! clique numa palavra ou objeto com som toca o som; a Varinha Mágica do
//! McZee faz o mesmo ([`wand_sound`]). As palavras com som aparecem
//! emolduradas de preto na página, como no original.

use super::editor::{Editor, PAGE};
use super::tools::{Cx, ToolId, ToolStrip, WHAT_SOUNDS};
use crate::Frame;
use crate::assets::{Assets, ItemKind, Program, SoundRef};
use crate::scene::{Effect, Key};
use crate::scenes::sounds::{
    self as fa, Finder, HELP_DOG, HELP_PICK, HELP_RECORDER, HELP_THEN_CLICK, HELP_VOLUME, Hand,
    Layout, Recorder, Sounds,
};
use crate::writer::doc::{Document, Pos};
use crate::writer::layout::Layout as TextLayout;
use crate::writer::object::ObjRef;
use formats::Tag;
use formats::chnk::{self, OwnedChunk, OwnedPart};

/// A faixa do Seleciona-sons no Escritório (captura `office_05`; lista
/// medida no original com as categorias Grave e Ação).
pub const LAYOUT: Layout = Layout {
    program: Program::Writer,
    banner: (0x024d, 0x024f, false),
    banner_x0: 20,
    banner_step: 113,
    banner_y: 59,
    volume: (362, 56, 34, 44),
    track: (0x0255, 0x0257, 368, 60),
    knob: (0x0259, 0x025b, 364),
    knob_top: 61,
    knob_bottom: 86,
    dog: (405, 62, 37, 32),
    dog_pic: (0x025d, 0x0263, 405, 62),
    recorder: (454, 57, 43, 41),
    recorder_pic: (0x0265, 0x0267),
    arrow_up: (511, 56, 16, 14),
    arrow_down: (511, 86, 16, 14),
    arrows_back: Some((0x08d9, 0x08dd, 510, 56)),
    page_box: (511, 72, 16, 12),
    list_x0: 30,
    list_step: 112,
    list_y: 95,
    row: (94, 22, 21),
    list_rows: 18,
    plain_arrows: true,
    canvas: PAGE,
};

/// Balões do bloco `0fa2`: depois de associar (Varinha Mágica) e o
/// Cachorrinho; `0805[18]`: número de página.
const HELP_AFTER: usize = 8;
const HELP_WRITER_DOG: usize = 9;
const NO_PAGE_NUMBER: (u16, usize) = (0x0805, 18);
const BLACK: [u8; 3] = [0, 0, 0];

/// Balão do `0fa2` que corresponde ao balão da ferramenta da pintura.
fn writer_help(k: usize) -> usize {
    if k == HELP_DOG { HELP_WRITER_DOG } else { k }
}

/// O que está debaixo de um clique na página, para os sons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Objeto solto (adesivo, desenho, palavra com forma).
    Object(usize),
    /// Palavra (ou linhas vazias seguidas) na posição.
    Word(Pos),
    PageNumber,
}

/// A casa da "palavra" dos sons em `p` serve de alvo: letras ou `\r`
/// (linha vazia), não os espaços entre palavras.
fn targetable(doc: &Document, p: Pos) -> bool {
    match doc.item(p) {
        Some(i) => i.is_word(),
        None => p.index == doc.paras.get(p.para).map_or(0, |par| par.len()),
    }
}

/// O alvo sob o ponto `(x, y)` da tela.
pub fn target_at(ed: &Editor, x: i32, y: i32) -> Option<Target> {
    if let Some(k) = ed.object_at(x, y) {
        return Some(Target::Object(k));
    }
    let p = ed.to_page(x, y)?;
    let l = ed.layout();
    let hit =
        |(x0, y0, x1, y1): (i32, i32, i32, i32)| (x0..x1).contains(&p.x) && (y0..y1).contains(&p.y);
    if l.pages
        .get(p.page)
        .and_then(|s| s.number.as_ref())
        .is_some_and(|n| hit(n.rect))
    {
        return Some(Target::PageNumber);
    }
    let doc = ed.doc();
    let at = l.pos_at(p.page, p.x, p.y);
    [Some(at), doc.prev(at)]
        .into_iter()
        .flatten()
        .filter(|&c| targetable(doc, c))
        .find(|&c| {
            l.selection_rects(doc.sound_run(c))
                .iter()
                .any(|r| r.page == p.page && hit(r.rect))
        })
        .map(Target::Word)
}

/// O som guardado no alvo.
pub fn sound_of(ed: &Editor, t: Target) -> Option<ObjRef> {
    match t {
        Target::Object(k) => ed.doc().objects.get(k)?.sound,
        Target::Word(p) => ed.doc().word_sound(p).map(|(_, s)| s),
        Target::PageNumber => None,
    }
}

/// Tocar o som `r` do documento (o WAVE do chunk `SND `).
pub fn play(ed: &Editor, r: ObjRef, a: &Assets) -> Option<Effect> {
    let c = ed.extra.object(r)?;
    let pcm = c.parts.iter().find(|p| p.tag.0 == *b"PCM ")?;
    a.register_wave(&pcm.data).map(Effect::PlaySound)
}

/// Varinha Mágica: o som da palavra ou do objeto sob o ponto da tela.
pub fn wand_sound(ed: &Editor, x: i32, y: i32, a: &Assets) -> Option<Effect> {
    let s = sound_of(ed, target_at(ed, x, y)?)?;
    play(ed, s, a)
}

/// Número do chunk `SND ` de um som de biblioteca (o rótulo do catálogo é
/// "SND  n nome").
fn library_id(a: &Assets, file: &str, sound: &SoundRef) -> Option<u16> {
    let src = a
        .catalog()
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(file))?;
    let item = src
        .items
        .iter()
        .find(|i| matches!(&i.kind, ItemKind::Sound(r) if r == sound))?;
    let mut parts = item.label.split_whitespace();
    (parts.next()? == "SND")
        .then(|| parts.next()?.parse().ok())
        .flatten()
}

/// Cópia do som para o documento: o `SND n` da biblioteca (com nome) ou um
/// `SND` novo com o WAVE do arquivo.
pub fn sound_chunk(a: &Assets, name: &str, sound: &SoundRef) -> Option<OwnedChunk> {
    match sound {
        SoundRef::Library { file, .. } => {
            let id = library_id(a, file, sound)?;
            let f = chnk::parse(a.library_bytes(file)?).ok()?;
            let c = f
                .chunks
                .iter()
                .find(|c| c.tag.0 == *b"SND " && c.id == id)?;
            Some(OwnedChunk {
                tag: c.tag,
                id: c.id,
                flags: c.flags,
                name: c.name.clone(),
                parts: c
                    .parts
                    .iter()
                    .map(|p| OwnedPart {
                        tag: p.tag,
                        data: p.data.to_vec(),
                    })
                    .collect(),
            })
        }
        SoundRef::File(p) => {
            let data = std::fs::read(p).ok()?;
            Some(OwnedChunk {
                tag: Tag(*b"SND "),
                id: 0,
                flags: 0,
                name: Some(name.to_string()),
                parts: vec![OwnedPart {
                    tag: Tag(*b"PCM "),
                    data,
                }],
            })
        }
        SoundRef::Snd(..) | SoundRef::Wave(_) => None,
    }
}

/// Guarda a cópia do som no documento e devolve a referência: o mesmo som
/// já guardado é reaproveitado; o número da biblioteca é mantido quando
/// está livre (como o original grava), senão vai para um número livre.
pub fn store_sound(ed: &mut Editor, c: OwnedChunk) -> ObjRef {
    if let Some(old) = ed
        .extra
        .objects
        .iter()
        .find(|o| o.tag == c.tag && o.parts == c.parts)
    {
        return ObjRef::snd(old.id);
    }
    if ed.extra.object(ObjRef::snd(c.id)).is_none() {
        let r = ObjRef::snd(c.id);
        ed.extra.objects.push(c);
        return r;
    }
    ed.extra.add_object(c)
}

/// Põe (ou tira, com `None`) o som do alvo; uma ação do Ovo.
pub fn attach(ed: &mut Editor, t: Target, s: Option<ObjRef>) -> bool {
    match t {
        Target::Object(k) if k < ed.doc().objects.len() => {
            ed.record();
            ed.doc_mut().objects[k].sound = s;
        }
        Target::Word(p) => {
            ed.record();
            ed.doc_mut().set_word_sound(p, s);
        }
        _ => return false,
    }
    ed.after_edit();
    true
}

/// Molduras pretas em volta das palavras com som da folha `page` (a vista
/// do Escritório; não saem na impressão).
pub fn draw_word_boxes(
    doc: &Document,
    l: &TextLayout,
    v: &mut Frame,
    page: usize,
    ox: i32,
    oy: i32,
) {
    for (run, _, _) in doc.word_sounds() {
        for r in l.selection_rects(run) {
            let (x0, y0, x1, y1) = r.rect;
            if r.page == page && x1 > x0 {
                v.frame_rect(ox + x0 - 1, oy + y0, x1 - x0 + 2, y1 - y0, 1, BLACK);
            }
        }
    }
}

#[derive(Default)]
pub struct SoundsStrip {
    /// A ferramenta (criada na primeira vez que é escolhida).
    tool: Option<Sounds>,
}

impl SoundsStrip {
    fn tool(&mut self, a: &Assets) -> &mut Sounds {
        self.tool
            .get_or_insert_with(|| Sounds::with_layout(LAYOUT, a))
    }

    /// O som na mão (para testes e o cursor).
    pub fn hand(&self) -> Option<&Hand> {
        self.tool.as_ref()?.hand()
    }

    /// O som escolhido (lista, Cachorrinho ou Gravador) vai para a mão.
    fn hold(&mut self, h: Hand, cx: &mut Cx) {
        self.tool(cx.assets).set_hand(h);
        cx.help((WHAT_SOUNDS, HELP_THEN_CLICK));
    }

    fn reply(&mut self, fx: Vec<Effect>, close: bool, chosen: Option<Hand>, cx: &mut Cx) {
        cx.fx.extend(fx);
        if close {
            let t = self.tool(cx.assets);
            t.finder = None;
            t.recorder = None;
            cx.close_help();
        }
        if let Some(h) = chosen {
            self.hold(h, cx);
        }
    }

    /// Clique na página com a ferramenta: associar o som da mão ou tocar.
    fn on_page(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        let a = cx.assets;
        let target = target_at(ed, x, y);
        let Some(hand) = self.tool(a).take_hand() else {
            // sem som na mão: o clique numa coisa com som a toca; no resto,
            // o clique é o do texto
            return match target.and_then(|t| sound_of(ed, t)) {
                Some(s) => {
                    cx.fx.extend(play(ed, s, a));
                    true
                }
                None => false,
            };
        };
        match target {
            None => {
                // fora de uma palavra: o som continua na mão
                self.tool(a).set_hand(hand);
                cx.help((WHAT_SOUNDS, HELP_THEN_CLICK));
            }
            Some(Target::PageNumber) => {
                self.tool(a).set_hand(hand);
                cx.help(NO_PAGE_NUMBER);
            }
            Some(t) => {
                let s = match &hand {
                    Hand::Silence => None,
                    Hand::Sound { name, sound } => {
                        sound_chunk(a, name, sound).map(|c| store_sound(ed, c))
                    }
                };
                if hand.sound().is_some() && s.is_none() {
                    cx.log("Seleciona-sons: som sem dados");
                    return true;
                }
                if super::stickers::keep_view(ed, |ed| attach(ed, t, s)) {
                    cx.fx.extend(hand.sound().cloned().map(Effect::PlaySound));
                    cx.help((WHAT_SOUNDS, HELP_AFTER));
                }
            }
        }
        true
    }
}

impl ToolStrip for SoundsStrip {
    fn id(&self) -> ToolId {
        ToolId::Sounds
    }

    fn activate(&mut self, _ed: &mut Editor, cx: &mut Cx, _again: bool) {
        let t = self.tool(cx.assets);
        cx.fx.extend(t.stop_effects());
        t.reset();
    }

    fn deactivate(&mut self, _ed: &mut Editor, cx: &mut Cx) {
        if let Some(t) = self.tool.as_mut() {
            cx.fx.extend(t.stop_effects());
            t.reset();
        }
    }

    fn modal(&self) -> bool {
        self.tool.as_ref().is_some_and(|t| t.modal())
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        let a = cx.assets;
        let t = self.tool(a);
        if let Some(d) = t.finder.as_mut() {
            let r = d.click(x, y);
            self.reply(r.fx, r.close, r.chosen, cx);
            return true;
        }
        if let Some(rec) = t.recorder.as_mut() {
            let r = rec.click(x, y, a);
            self.reply(r.fx, r.close, r.chosen, cx);
            return true;
        }
        let r = t.click(x, y, a);
        if !r.handled {
            return false;
        }
        cx.fx.extend(r.fx);
        if let Some(k) = r.help {
            match k {
                HELP_DOG => {
                    t.finder = Some(Finder::new(ed.dir(), fa::finder::music_dirs()));
                }
                HELP_RECORDER => t.recorder = Some(Recorder::new(ed.dir())),
                HELP_PICK | HELP_VOLUME | HELP_THEN_CLICK => {}
                _ => {}
            }
            cx.help((WHAT_SOUNDS, writer_help(k)));
        }
        if r.canvas {
            return self.on_page(ed, cx, x, y);
        }
        true
    }

    fn motion(&mut self, _ed: &mut Editor, cx: &mut Cx, _x: i32, y: i32) {
        if let Some(t) = self.tool.as_mut() {
            cx.fx.extend(t.drag(y, cx.assets));
        }
    }

    fn release(&mut self, _ed: &mut Editor, cx: &mut Cx, _x: i32, _y: i32) {
        if let Some(t) = self.tool.as_mut() {
            cx.fx.extend(t.release());
        }
    }

    fn key(&mut self, _ed: &mut Editor, cx: &mut Cx, key: Key, _ctrl: bool) -> bool {
        let a = cx.assets;
        let Some(rec) = self.tool.as_mut().and_then(|t| t.recorder.as_mut()) else {
            return false;
        };
        let r = rec.key(key, a);
        self.reply(r.fx, r.close, r.chosen, cx);
        true
    }

    fn text(&mut self, _ed: &mut Editor, _cx: &mut Cx, text: &str) -> bool {
        match self.tool.as_mut().and_then(|t| t.recorder.as_mut()) {
            Some(rec) => {
                rec.type_text(text);
                true
            }
            None => false,
        }
    }

    fn update(&mut self, _ed: &mut Editor, cx: &mut Cx, dt: f32) {
        if let Some(rec) = self.tool.as_mut().and_then(|t| t.recorder.as_mut()) {
            cx.fx.extend(rec.tick(dt));
        }
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        if let Some(t) = &self.tool {
            t.draw_strip(f, a);
        }
    }

    fn draw_front(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        if let Some(t) = &self.tool {
            t.draw_overlay(f, a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::inside;

    #[test]
    fn layout_matches_the_measured_strip() {
        // a lista cai debaixo da faixa clicada (x 30, 142, 254)
        assert_eq!(LAYOUT.list_x0 + 2 * LAYOUT.list_step, 254);
        assert_eq!(LAYOUT.list_y + 17 * LAYOUT.row.2, 452);
        assert!(inside(LAYOUT.dog, 420, 75));
        assert!(inside(LAYOUT.recorder, 470, 75));
        assert!(inside(LAYOUT.canvas, 300, 300));
        assert_eq!(writer_help(HELP_DOG), HELP_WRITER_DOG);
        assert_eq!(writer_help(HELP_PICK), 2);
    }

    use crate::scene::{Input, MouseButton, Scene};
    use crate::scenes::testutil::empty_assets;
    use crate::scenes::writer::Writer;
    use crate::writer::convert::to_max;
    use formats::wave::{Pcm, encode_pcm16};

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    /// Ponto da tela no meio da letra `k` do parágrafo 0.
    fn letter(w: &Writer, k: usize) -> (i32, i32) {
        let ed = w.editor();
        let c = ed.layout().caret(Pos::new(0, k));
        ed.to_screen(c.page, c.x + 2, c.top + c.height / 2)
    }

    fn waves(fx: &[Effect]) -> usize {
        fx.iter()
            .filter(|e| matches!(e, Effect::PlaySound(SoundRef::Wave(_))))
            .count()
    }

    #[test]
    fn a_sound_goes_to_the_clicked_word_and_plays_back() {
        let (dir, a) = empty_assets();
        let wav = dir.path().join("galo.wav");
        let pcm = Pcm {
            sample_rate: 11025,
            channels: 1,
            samples: vec![0, 100, -100, 0],
            loop_points: None,
        };
        std::fs::write(&wav, encode_pcm16(&pcm)).unwrap();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[Input::Text("o gato azul".into())], 0.0, &a);
        let (tx, ty) = crate::ui::center(ToolId::Sounds.def().hit);
        w.update(&[click(tx, ty)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Sounds);
        let sound = SoundRef::File(wav.clone());
        w.tools.sounds.tool.as_mut().unwrap().set_hand(Hand::Sound {
            name: "galo".into(),
            sound: sound.clone(),
        });
        // no espaço entre as palavras: o som continua na mão
        let (x, y) = letter(&w, 1);
        w.update(&[click(x + 1, y)], 0.0, &a);
        assert!(w.tools.sounds.hand().is_some());
        assert!(!w.doc().has_word_sounds());
        // na palavra: toca, vai para "gato" e sai da mão
        let (x, y) = letter(&w, 3);
        let fx = w.update(&[click(x, y)], 0.0, &a);
        assert!(fx.contains(&Effect::PlaySound(sound)));
        assert!(w.tools.sounds.hand().is_none());
        let (run, s) = w.doc().word_sound(Pos::new(0, 4)).unwrap();
        assert_eq!(w.doc().text_range(run), "gato");
        let c = w.editor().extra.object(s).unwrap();
        assert_eq!(c.name.as_deref(), Some("galo"));
        let m = to_max(w.doc(), &w.editor().extra, &[], None);
        assert_eq!(m.word_sounds.len(), 1);
        assert_eq!(m.word_sounds[0].start, 2);
        assert_eq!(m.word_sounds[0].end, m.len() - 1);
        // sem nada na mão, clicar na palavra toca o som do documento
        let fx = w.update(&[click(x, y)], 0.0, &a);
        assert_eq!(waves(&fx), 1);
        // a palavra aparece emoldurada
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        w.draw(&mut f, &a);
        let ed = w.editor();
        let c = ed.layout().caret(Pos::new(0, 2));
        let (bx, by) = ed.to_screen(c.page, c.x - 1, c.top + 1);
        assert_eq!(&f.pixel(bx as u32, by as u32)[..3], &[0, 0, 0]);
        // a Varinha Mágica toca; o Ovo tira o som
        w.mczee.set_wand(true);
        let fx = w.update(&[click(x, y)], 0.0, &a);
        assert_eq!(waves(&fx), 1);
        w.mczee.set_wand(false);
        assert!(w.editor_mut().undo_step());
        assert!(!w.doc().has_word_sounds());
    }

    #[test]
    fn silence_removes_and_the_page_number_refuses() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[Input::Text("um".into())], 0.0, &a);
        w.editor_mut()
            .doc_mut()
            .set_word_sound(Pos::new(0, 0), Some(ObjRef::snd(5)));
        let (tx, ty) = crate::ui::center(ToolId::Sounds.def().hit);
        w.update(&[click(tx, ty)], 0.0, &a);
        w.tools
            .sounds
            .tool
            .as_mut()
            .unwrap()
            .set_hand(Hand::Silence);
        let (x, y) = letter(&w, 1);
        w.update(&[click(x, y)], 0.0, &a);
        assert!(!w.doc().has_word_sounds());
        assert_eq!(sound_of(w.editor(), Target::Word(Pos::new(0, 0))), None);
    }
}
