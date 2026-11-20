//! FORMA (W7): o Cavalete de palavras com forma do Fine Artist
//! (`scenes::paint::words::shaped_easel`) aberto no Escritório
//! (`office-palavra.md` §8). Abre pelo botão FORMA da Palavra com o nome do
//! usuário (selecionado) ou clicando numa palavra com forma da página; OK
//! põe a palavra como objeto flutuante (`SPLE` no `.MAX`,
//! `writer::splot`) no ponto de inserção, ou troca a que foi reaberta.
//!
//! Diferenças do Fine Artist: a paleta são as 16 cores do `CSCH`, e os
//! botões ＋ e − mostram os "preferidos" (`TDRL`) em vez de mudar o tamanho.

use super::editor::Editor;
use super::tools::{HelpRef, WHAT_WORD, snd};
use crate::Frame;
use crate::assets::{Assets, Program};
use crate::paint::shaped::{self, ShapedWords};
use crate::paint::splot_res::SplotRes;
use crate::scene::{Effect, Input, MouseButton};
use crate::scenes::paint::words::shaped_easel::{self, Ctx, ShapedEasel};
use crate::writer::convert;
use crate::writer::object::{FloatObject, ObjRef, ObjectKind};
use crate::writer::splot;
use formats::chnk::{OwnedChunk, OwnedPart};
use formats::fnt::Font;
use formats::palette::Rgb;
use formats::sple::Choices;
use formats::{Tag, sple, twpb};
use std::rc::Rc;

/// Abrir o cavalete (`0224`), OK (`02ee`) e NÃO (`02ed`).
pub const SND_OPEN: u16 = 0x0224;
pub const SND_OK: u16 = 0x02ee;
pub const SND_NO: u16 = 0x02ed;
/// Balões do bloco da Palavra: "Digite as palavras aqui…", girar.
pub const HELP_TYPE: HelpRef = (WHAT_WORD, 11);
pub const HELP_ROTATE: HelpRef = (WHAT_WORD, 19);

/// O que o cavalete usa do executável e da paleta (carregado na 1ª vez).
struct Data {
    res: SplotRes,
    pal: Vec<Rgb>,
    color_sets: Vec<Vec<u8>>,
    favorites: Vec<Choices>,
    pablo: Option<Rc<Font>>,
    ui_font: Option<Rc<Font>>,
}

impl Data {
    fn load(a: &Assets) -> Data {
        let res = SplotRes::load(a);
        let pal = splot::palette();
        let color_sets = res
            .color_sets
            .iter()
            .map(|set| set.iter().map(|c| nearest(&pal, *c)).collect())
            .collect();
        let favorites = a
            .exe_resource(Program::Writer, "TDRL", 1)
            .and_then(|d| formats::splot_res::read_tdrl(&d).ok())
            .map(|v| v.iter().map(formats::splot_res::favorite).collect())
            .unwrap_or_default();
        Data {
            res,
            pal,
            color_sets,
            favorites,
            pablo: a.font(16),
            ui_font: a.font(8),
        }
    }

    fn ctx(&self) -> Ctx<'_> {
        Ctx {
            pal: &self.pal,
            patterns: &self.res.patterns,
            pablo: self.pablo.as_deref(),
            ui_font: self.ui_font.as_deref(),
            inks: splot::INKS,
            color_sets: &self.color_sets,
            res: &self.res,
        }
    }
}

fn nearest(pal: &[Rgb], c: Rgb) -> u8 {
    let d = |p: &Rgb| {
        let f = |a: u8, b: u8| (i32::from(a) - i32::from(b)).pow(2);
        f(p.r, c.r) + f(p.g, c.g) + f(p.b, c.b)
    };
    (0..pal.len()).min_by_key(|&k| d(&pal[k])).unwrap_or(0) as u8
}

/// O cavalete no Escritório.
#[derive(Default)]
pub struct ShapedHost {
    data: Option<Data>,
    easel: Option<ShapedEasel>,
    /// Objeto da página que o cavalete modifica (`None` = palavra nova).
    editing: Option<usize>,
    rotate_help_shown: bool,
}

impl ShapedHost {
    pub fn is_open(&self) -> bool {
        self.easel.is_some()
    }

    /// Texto no campo do cavalete aberto.
    pub fn text(&self) -> Option<&str> {
        self.easel.as_ref().map(|e| e.field.text.as_str())
    }

    /// A palavra do cavalete aberto.
    pub fn words(&self) -> Option<&ShapedWords> {
        self.easel.as_ref().map(|e| &e.words)
    }

    /// Abre o cavalete: palavra nova com `name` (o nome do usuário) ou o
    /// objeto `editing` da página. Devolve o balão do McZee.
    pub fn open(
        &mut self,
        ed: &Editor,
        editing: Option<usize>,
        name: &str,
        fx: &mut Vec<Effect>,
        a: &Assets,
    ) -> Option<HelpRef> {
        let data = self.data.get_or_insert_with(|| Data::load(a));
        let words = match editing.and_then(|k| ed.doc().objects.get(k)) {
            Some(o) if o.kind == ObjectKind::Shaped => ed
                .extra
                .object(o.object)
                .and_then(|c| c.parts.iter().find(|p| p.tag.0 == *b"SPLE"))
                .and_then(|p| sple::read(&p.data).ok())
                .map(|s| splot::words_of(&s, &data.color_sets))?,
            Some(_) => return None,
            None => {
                let mut w = ShapedWords::new(name, splot::INKS.black);
                w.font = shaped::default_font();
                w
            }
        };
        let mut e = ShapedEasel::new(words, None);
        e.favorites = data.favorites.clone();
        e.refresh(&data.ctx());
        self.easel = Some(e);
        self.editing = editing;
        fx.push(snd(SND_OPEN));
        Some(HELP_TYPE)
    }

    /// Uma entrada com o cavalete aberto (todas ficam com ele). Devolve o
    /// balão do McZee a abrir, se houver.
    pub fn input(
        &mut self,
        i: &Input,
        ed: &mut Editor,
        fx: &mut Vec<Effect>,
        a: &Assets,
    ) -> Option<HelpRef> {
        let (Some(e), Some(data)) = (self.easel.as_mut(), self.data.as_ref()) else {
            return None;
        };
        // o clique direito fica com o McZee
        if let Input::Click {
            button: MouseButton::Right,
            ..
        } = i
        {
            return None;
        }
        let ctx = data.ctx();
        let r = e.input(i, a, &ctx);
        e.refresh(&ctx);
        if let Some(s) = r.sound {
            fx.push(snd(s));
        }
        let mut help = None;
        if r.sound == Some(shaped_easel::SND_ROTATE) && !self.rotate_help_shown {
            self.rotate_help_shown = true;
            help = Some(HELP_ROTATE);
        }
        match r.done {
            Some(true) => {
                fx.push(snd(SND_OK));
                self.finish(ed, a);
            }
            Some(false) => {
                fx.push(snd(SND_NO));
                self.easel = None;
            }
            None => {}
        }
        help
    }

    /// OK: monta o `SPLE` e põe (ou troca) o objeto. Texto vazio só fecha.
    fn finish(&mut self, ed: &mut Editor, a: &Assets) {
        let (Some(mut e), Some(data)) = (self.easel.take(), self.data.as_ref()) else {
            return;
        };
        e.words.text = e.field.text.clone();
        if e.words.text.trim().is_empty() {
            return;
        }
        let ctx = data.ctx();
        let stamp = shaped_easel::stamp_of(&e.words, &ctx);
        let scrd = e
            .words
            .shape
            .scrd_id()
            .and_then(|id| a.exe_resource(Program::Writer, "SCRD", id))
            .or_else(|| {
                e.words
                    .shape
                    .scrd_id()
                    .and_then(|id| a.exe_resource(Program::Artist, "SCRD", id))
            });
        let s = splot::build(
            &e.words,
            &stamp,
            &data.pal,
            &data.color_sets,
            scrd.as_deref(),
        );
        let (w, h) = splot::page_size(stamp.w, stamp.h);
        let (chunk, outline) = chunk_of(&s, w, h);
        let old = self.editing.and_then(|k| ed.doc().objects.get(k).cloned());
        let (page, x, y) = match &old {
            Some(o) => (o.page, o.x, o.y),
            None => {
                let c = ed.layout().caret(ed.sel.caret);
                let (x, y) = place_at(&ed.doc().dop, c.x, c.top, w, h);
                (c.page as u16, x, y)
            }
        };
        let mut o = FloatObject::new(ObjectKind::Shaped, ObjRef::sple(0), page, x, y, w, h);
        o.outline = Some(outline);
        match (self.editing, old) {
            (Some(k), Some(old)) => {
                o.wrap = old.wrap;
                o.sound = old.sound;
                o.border = old.border;
                ed.replace_object(k, o, Some(chunk));
            }
            _ => {
                ed.add_object(o, Some(chunk));
            }
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if let Some(e) = self.easel.as_mut() {
            e.tick(dt);
            if let Some(d) = self.data.as_ref() {
                e.refresh(&d.ctx());
            }
        }
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        if let (Some(e), Some(d)) = (&self.easel, &self.data) {
            e.draw(f, a, &d.ctx());
        }
    }
}

/// Canto de um objeto novo `w × h` no ponto de inserção `(x, y)` da
/// folha: dentro das margens, como o original (o "Ana" posto depois de
/// "resolveu ler" começa perto da margem esquerda, `office_34_forma_ok`).
pub fn place_at(dop: &crate::writer::doc::DopProps, x: i32, y: i32, w: u16, h: u16) -> (i16, i16) {
    let left = i32::from(dop.margin_left);
    let right = i32::from(dop.page_width) - i32::from(dop.margin_right) - i32::from(w);
    let bottom = i32::from(dop.page_height) - i32::from(dop.margin_bottom) - i32::from(h);
    let top = i32::from(dop.margin_top);
    (x.min(right).max(left) as i16, y.min(bottom).max(top) as i16)
}

/// Chunk `SPLE` (partes `SPLE` e `TWPB`) do Splot `s` com `w × h` pontos na
/// página, e o contorno para o texto contornar.
pub fn chunk_of(s: &sple::Sple, w: u16, h: u16) -> (OwnedChunk, crate::writer::object::Outline) {
    let img = splot::draw(s, u32::from(w), u32::from(h));
    let mask: Vec<bool> = img.rgba.chunks_exact(4).map(|p| p[3] != 0).collect();
    let t = twpb::from_mask(&mask, img.width, img.height);
    let chunk = OwnedChunk {
        tag: Tag(*b"SPLE"),
        id: 0,
        flags: 0,
        name: None,
        parts: vec![
            OwnedPart {
                tag: Tag(*b"SPLE"),
                data: sple::write(s),
            },
            OwnedPart {
                tag: Tag(*b"TWPB"),
                data: twpb::write(&t),
            },
        ],
    };
    (chunk, convert::outline_of(&t))
}

#[cfg(test)]
mod tests {
    use super::super::Writer;
    use super::*;
    use crate::scene::Scene;
    use crate::scenes::paint::words::shaped_easel::{NO, OK, PLUS};
    use crate::scenes::testutil::{assets, empty_assets};

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    /// Botão FORMA da faixa da Palavra.
    const FORMA_BUTTON: (i32, i32) = (460, 75);

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(crate::assets::SoundRef::Snd(Program::Writer, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn forma_opens_with_the_user_name_and_ok_places_an_object() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a).with_user("Ana");
        let fx = w.update(&[click(FORMA_BUTTON.0, FORMA_BUTTON.1)], 0.0, &a);
        assert!(w.shaped().is_open());
        assert_eq!(w.shaped().text(), Some("Ana"));
        assert!(sounds(&fx).contains(&SND_OPEN));
        // a página não recebe o que se digita: vai para o campo
        w.update(&[Input::Text("Bia".into())], 0.0, &a);
        assert_eq!(w.shaped().text(), Some("Bia"));
        assert!(w.doc().is_empty());
        let fx = w.update(&[click(OK.0 + 5, OK.1 + 5)], 0.0, &a);
        assert!(!w.shaped().is_open());
        assert!(sounds(&fx).contains(&SND_OK));
        assert_eq!(w.doc().objects.len(), 1);
        let o = &w.doc().objects[0];
        assert_eq!(o.kind, ObjectKind::Shaped);
        assert_eq!(o.object.tag, ObjRef::SPLE);
        let c = w.editor().extra.object(o.object).unwrap();
        let s = sple::read(&c.parts[0].data).unwrap();
        assert_eq!(s.text(), b"Bia");
        // desfazível
        w.editor_mut().undo_step();
        assert!(w.doc().objects.is_empty());
    }

    #[test]
    fn new_objects_stay_inside_the_margins() {
        let dop = crate::writer::doc::DopProps::default();
        assert_eq!(
            (dop.page_width, dop.margin_left, dop.margin_right),
            (595, 72, 72)
        );
        assert_eq!(place_at(&dop, 400, 200, 300, 119), (223, 200));
        assert_eq!(place_at(&dop, 80, 200, 300, 119), (80, 200));
        assert_eq!(place_at(&dop, 10, 700, 300, 119), (72, 601));
    }

    #[test]
    fn no_closes_without_placing() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a).with_user("Ana");
        w.update(&[click(FORMA_BUTTON.0, FORMA_BUTTON.1)], 0.0, &a);
        let fx = w.update(&[click(NO.0 + 5, NO.1 + 5)], 0.0, &a);
        assert!(!w.shaped().is_open());
        assert!(sounds(&fx).contains(&SND_NO));
        assert!(w.doc().objects.is_empty());
    }

    #[test]
    fn real_assets_splot_is_drawn_reopened_and_favorites_step() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new_blank(None, &a).with_user("Ana");
        w.update(&[click(FORMA_BUTTON.0, FORMA_BUTTON.1)], 0.0, &a);
        // ＋ mostra o primeiro "preferido" (cabeça, efeito tridimensional)
        w.update(&[click(PLUS.0 + 5, PLUS.1 + 5)], 0.0, &a);
        let words = w.shaped().words().unwrap();
        assert_eq!(words.shape, crate::paint::shaped::WordShape::Head);
        assert_eq!(words.shadow, crate::paint::shaped::Shadow::Extrude);
        assert_eq!(words.text, "Ana");
        w.update(&[click(OK.0 + 5, OK.1 + 5)], 0.0, &a);
        let o = w.doc().objects[0].clone();
        assert!(o.width <= splot::MAX_W as u16 && o.height <= splot::MAX_H as u16);
        assert!(o.width == splot::MAX_W as u16 || o.height == splot::MAX_H as u16);
        // desenhado com tinta (não o retângulo de reserva)
        let img = w.editor().picture(&o).unwrap();
        let inked = img.rgba.chunks_exact(4).filter(|p| p[3] != 0).count();
        assert!(inked > 1000, "{inked}");
        // clique no objeto reabre o cavalete com as escolhas
        let (sx, sy) = w
            .editor()
            .to_screen(o.page as usize, o.x.into(), o.y.into());
        let hit = (0..i32::from(o.height))
            .flat_map(|dy| (0..i32::from(o.width)).map(move |dx| (dx, dy)))
            .find(|&(dx, dy)| w.editor().object_at(sx + dx, sy + dy) == Some(0));
        let (dx, dy) = hit.expect("objeto visível na tela");
        w.update(&[click(sx + dx, sy + dy)], 0.0, &a);
        assert!(w.shaped().is_open());
        let words = w.shaped().words().unwrap();
        assert_eq!(words.shape, crate::paint::shaped::WordShape::Head);
        assert_eq!(words.text, "Ana");
        // OK de novo troca o mesmo objeto
        w.update(
            &[Input::Text("Lia".into()), click(OK.0 + 5, OK.1 + 5)],
            0.0,
            &a,
        );
        assert_eq!(w.doc().objects.len(), 1);
        assert_eq!((w.doc().objects[0].x, w.doc().objects[0].y), (o.x, o.y));
    }

    #[test]
    fn real_assets_example_splots_are_drawn() {
        let Some(a) = assets() else { return };
        let dir = a.data_dir().join("SHARED/USERS/MAX");
        let mut w = Writer::open(None, &dir.join("SAMP15.MAX"), &a).unwrap();
        let k = w
            .doc()
            .objects
            .iter()
            .position(|o| o.kind == ObjectKind::Shaped)
            .unwrap();
        let o = w.doc().objects[k].clone();
        let img = w.editor().picture(&o).unwrap();
        assert_eq!(
            (img.width, img.height),
            (u32::from(o.width), u32::from(o.height))
        );
        // letras pretas e sombra cinza
        let px: Vec<&[u8]> = img.rgba.chunks_exact(4).filter(|p| p[3] != 0).collect();
        assert!(px.iter().any(|p| p[..3] == [0, 0, 0]));
        assert!(px.iter().any(|p| p[..3] == [0x8c, 0x8c, 0x8c]));
        let _ = w.update(&[], 0.0, &a);
    }
}
