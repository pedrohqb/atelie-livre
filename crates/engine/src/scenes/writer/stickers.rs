//! Seleciona-adesivos (`office-ferramentas.md` §1): rolos de clip-art das
//! quatro bibliotecas (29 rolos em ordem de número, 7 por página), tira de
//! filme com os adesivos do rolo, setas de página, Cachorrinho e Cavalete
//! de lápis de cera.
//!
//! O adesivo escolhido na tira fica "na mão" (balão `0fa1[3]`, o desenho
//! segue o ponteiro na página) e o clique na folha o cola como **objeto
//! solto** centrado no clique (som `021c`, balão `0fa1[4]`), com o contorno
//! `TWPB` para o texto contornar; clicar fora da folha o larga. Com a
//! ferramenta, arrastar um objeto o move e clicar nele (sem arrastar) o
//! leva ao Cavalete para modificar. O Cachorrinho é o diálogo "achar
//! figuras" do Fine Artist (`paint::find_pictures`) com a pasta do usuário
//! do Writer; o Cavalete é o de `scenes::writer_easel`: o OK põe o desenho
//! (recortado ao que foi pintado) na folha à vista, no canto da área de
//! texto, como o original.
//!
//! Geometria medida no original (`office_04`, `office_47`, `office_51`):
//! tampa esquerda `0245` em x 16 (os primeiros 14 px), molduras `0249` de
//! 52 px a partir de x 30, tampa direita = os últimos 12 px da `0239`
//! logo depois do último rolo da página; tira de filme x 40 + 52·casa, de y
//! 98 a 441, com 8 casas de cantos arredondados (34 px, passo 41) e as
//! setinhas `02db`/`02df` embaixo. A 5ª página do original mostra dois
//! rolos com o mesmo ícone do rolo 28 (não há rolo 29 nas bibliotecas): o
//! port mostra um.

use super::editor::{Editor, PAGE};
use super::tools::{
    Cx, Pic, Rect, SND_CHOOSE, SND_SWITCH, StripButton, ToolId, ToolStrip, WHAT_STICKERS, inside,
    pic,
};
use crate::assets::{Assets, Program};
use crate::paint::drawings::USER_STICKER_PREFIX;
use crate::paint::stamp::Stamp;
use crate::paint::store::load_picture;
use crate::scene::{Input, MouseButton};
use crate::scenes::paint::find_pictures::{FindPictures, Outcome, own_entries, sample_entries};
use crate::scenes::paint::sticker_easel::{canvas_to_stamp, gray_region};
use crate::scenes::stickers::{blit_clipped, blit_part};
use crate::scenes::writer_easel::{CrayonEasel, EaselOutcome};
use crate::writer::object::ObjectKind;
use crate::{Frame, Image};
use formats::palette::Rgb;
use std::rc::Rc;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const YELLOW: [u8; 3] = [255, 238, 76];

pub const ROLLS_PER_PAGE: usize = 7;
const SLOT_X0: i32 = 30;
const SLOT_W: i32 = 52;
const STRIP_Y: i32 = 57;
/// Tampa esquerda: os primeiros 14 px da `0245` (o resto é um rolo).
const CAP_LEFT_X: i32 = 16;
const FILM_X0: i32 = 40;
const FILM_Y: i32 = 98;
const FILM_H: i32 = 344;
const FILM_W: i32 = 42;
const FILM_CELLS: usize = 8;
const CELL_Y0: i32 = 102;
const CELL_STEP: i32 = 41;
const CELL: i32 = 34;
/// Setas de página dos rolos, com o número da página no meio (fundo `08d9`).
pub const ARROW_UP: Rect = (511, 56, 16, 14);
pub const ARROW_DOWN: Rect = (511, 86, 16, 14);
const ARROWS_BACK: Pic = Pic::masked(0x08d9, 0x08dd, 510, 56);
/// Abrir a tira de filme de um rolo.
const SND_ROLL: u16 = 0x0233;
/// Colar o adesivo; arrastar um objeto.
const SND_PASTE: u16 = 0x021c;
const SND_DRAG: u16 = 0x022e;
/// Abrir o Cachorrinho (`0226` + `02f1`) e o Cavalete (`0224` + `0267`);
/// OK do Cavalete.
const SND_DOG: [u16; 2] = [0x0226, 0x02f1];
const SND_EASEL: [u16; 2] = [0x0224, 0x0267];
const SND_OK: u16 = 0x02ee;
/// Balões do bloco `0fa1`.
const HELP_PICKER: usize = 0;
const HELP_CLICK_STICKER: usize = 2;
const HELP_THEN_CLICK: usize = 3;
const HELP_FOR_THE_HAND: usize = 4;
const HELP_EASEL: usize = 6;
const HELP_DOG: usize = 11;
/// Arrasto mínimo (px) para mover em vez de abrir o Cavalete.
const DRAG_SLOP: i32 = 3;

/// Cachorrinho (procura figuras) e Cavalete de lápis de cera.
const BUTTONS: [StripButton; 2] = [
    StripButton {
        pic: Pic::masked(0x025d, 0x0263, 414, 62),
        w: 37,
        h: 32,
        help: Some((WHAT_STICKERS, HELP_DOG)),
        name: "Cachorrinho",
    },
    StripButton {
        pic: Pic::masked(0x0881, 0x0883, 459, 57),
        w: 43,
        h: 41,
        help: Some((WHAT_STICKERS, HELP_EASEL)),
        name: "Cavalete de lápis de cera",
    },
];

fn slot_x(k: usize) -> i32 {
    SLOT_X0 + SLOT_W * k as i32
}

fn slot_rect(k: usize) -> Rect {
    (slot_x(k), STRIP_Y, SLOT_W, 42)
}

/// Geometria da tira de filme sob o rolo da casa `slot`.
pub struct Film {
    x: i32,
}

impl Film {
    pub fn rect(&self) -> Rect {
        (self.x, FILM_Y, FILM_W, FILM_H)
    }

    pub fn cell(&self, j: usize) -> Rect {
        (self.x + 4, CELL_Y0 + CELL_STEP * j as i32, CELL, CELL)
    }

    fn up(&self) -> Rect {
        (self.x + 6, 428, 12, 12)
    }

    fn down(&self) -> Rect {
        (self.x + 24, 428, 12, 12)
    }
}

/// O que está na mão para colar.
#[derive(Clone)]
pub enum Held {
    /// Adesivo `item` do rolo `roll` (número do `CARO`), com a miniatura.
    Clip {
        roll: u16,
        item: usize,
        preview: Option<Rc<Image>>,
    },
    /// Figura achada pelo Cachorrinho.
    Picture { img: Rc<Image>, title: String },
}

/// Arrasto de um objeto colado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Grab {
    object: usize,
    from: (i32, i32),
    last: (i32, i32),
    moved: bool,
}

#[derive(Default)]
pub struct StickersStrip {
    roll_page: usize,
    roll: usize,
    film_open: bool,
    scroll: usize,
    held: Option<Held>,
    mouse: (i32, i32),
    grab: Option<Grab>,
    finder: Option<FindPictures>,
    /// Cavalete aberto e o objeto que ele modifica (`None` = desenho novo).
    easel: Option<(CrayonEasel, Option<usize>)>,
}

/// Roda uma ação sobre objetos sem mexer na vista (as ações do Editor
/// rolam até o ponto de inserção; colar ou arrastar um adesivo não).
pub fn keep_view<R>(ed: &mut Editor, f: impl FnOnce(&mut Editor) -> R) -> R {
    let scroll = ed.scroll;
    let r = f(ed);
    ed.scroll = scroll;
    r
}

/// Paleta das figuras do Cachorrinho (a do Fine Artist).
fn picture_palette(a: &Assets) -> Vec<Rgb> {
    a.palettes(Program::Artist).pal256.clone()
}

/// Carimbo (já cortado ao que foi pintado) como imagem: fora dele,
/// transparente.
fn stamp_image(st: &Stamp, pal: &[Rgb]) -> Image {
    let mut rgba = Vec::with_capacity(st.px.len() * 4);
    for v in &st.px {
        match v {
            Some(k) => {
                let c = crate::ui::pal_rgb(pal, *k);
                rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
            None => rgba.extend_from_slice(&[255, 255, 255, 0]),
        }
    }
    Image {
        width: st.w,
        height: st.h,
        rgba,
    }
}

/// O desenho do cavalete cortado ao que não é branco, com o canto do corte;
/// `None` em branco.
pub fn crop_drawing(img: &Image) -> Option<(Image, (u32, u32))> {
    let (w, h) = (img.width, img.height);
    let ink = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        let p = &img.rgba[i..i + 4];
        p[3] != 0 && !(p[0] > 250 && p[1] > 250 && p[2] > 250)
    };
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if ink(x, y) {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut rgba = Vec::with_capacity(((x1 - x0) * (y1 - y0) * 4) as usize);
    for y in y0..y1 {
        let a = ((y * w + x0) * 4) as usize;
        let b = ((y * w + x1) * 4) as usize;
        rgba.extend_from_slice(&img.rgba[a..b]);
    }
    Some((
        Image {
            width: x1 - x0,
            height: y1 - y0,
            rgba,
        },
        (x0, y0),
    ))
}

impl StickersStrip {
    pub fn film_open(&self) -> bool {
        self.film_open
    }

    pub fn held(&self) -> Option<&Held> {
        self.held.as_ref()
    }

    pub fn finder(&self) -> Option<&FindPictures> {
        self.finder.as_ref()
    }

    pub fn easel_open(&self) -> bool {
        self.easel.is_some()
    }

    pub fn film(&self) -> Film {
        Film {
            x: FILM_X0 + SLOT_W * (self.roll % ROLLS_PER_PAGE) as i32,
        }
    }

    fn roll_pages(ed: &Editor) -> usize {
        ed.rolls.len().div_ceil(ROLLS_PER_PAGE).max(1)
    }

    /// Rolos desta página.
    fn page_rolls(&self, ed: &Editor) -> usize {
        ed.rolls
            .len()
            .saturating_sub(self.roll_page * ROLLS_PER_PAGE)
            .min(ROLLS_PER_PAGE)
    }

    fn close_all(&mut self) {
        self.film_open = false;
        self.held = None;
        self.grab = None;
        self.finder = None;
        self.easel = None;
    }

    // --- colar ------------------------------------------------------------

    /// Cola o que está na mão centrado em `(x, y)` da tela; `false` fora
    /// das folhas.
    fn paste(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        let Some(p) = ed.to_page(x, y) else {
            return false;
        };
        let Some(held) = self.held.take() else {
            return false;
        };
        keep_view(ed, |ed| {
            let k = match &held {
                Held::Clip { roll, item, .. } => ed.add_sticker(*roll, *item, p.page, p.x, p.y),
                Held::Picture { img, title } => ed.add_picture(img, title, p.page, p.x, p.y),
            };
            // centrado no clique (o mesmo passo do Ovo)
            if let Some(o) = ed.doc().objects.get(k) {
                let (dx, dy) = (-i32::from(o.width) / 2, -i32::from(o.height) / 2);
                let (pw, ph) = (
                    i32::from(ed.doc().dop.page_width),
                    i32::from(ed.doc().dop.page_height),
                );
                ed.doc_mut().objects[k].move_by(dx, dy, pw, ph);
                ed.after_edit();
            }
        });
        cx.sound(SND_PASTE);
        cx.help((WHAT_STICKERS, HELP_FOR_THE_HAND));
        true
    }

    // --- Cachorrinho ------------------------------------------------------

    fn open_finder(&mut self, ed: &Editor, cx: &mut Cx) {
        for s in SND_DOG {
            cx.sound(s);
        }
        cx.help((WHAT_STICKERS, HELP_DOG));
        self.film_open = false;
        self.held = None;
        let entries = own_entries(ed.dir(), "");
        self.finder = Some(FindPictures::new(entries).with_label("Autor"));
    }

    fn finder_click(&mut self, ed: &Editor, cx: &mut Cx, x: i32, y: i32) {
        let pal = picture_palette(cx.assets);
        let Some(fd) = self.finder.as_mut() else {
            return;
        };
        match fd.click(x, y, &pal, cx.fx) {
            Outcome::Nothing => {}
            Outcome::Cancel => {
                self.finder = None;
                cx.help((WHAT_STICKERS, HELP_PICKER));
            }
            Outcome::Folder(outside) => {
                let entries = if outside {
                    sample_entries(cx.assets.data_dir())
                } else {
                    own_entries(ed.dir(), "")
                };
                fd.swap_entries(entries, outside);
            }
            Outcome::Use(path, title) => {
                self.finder = None;
                match load_picture(&path, &pal).and_then(|c| canvas_to_stamp(&c)) {
                    Some(st) => {
                        self.held = Some(Held::Picture {
                            img: Rc::new(stamp_image(&st, &pal)),
                            title,
                        });
                        cx.help((WHAT_STICKERS, HELP_THEN_CLICK));
                    }
                    None => cx.log(format!("cachorrinho: não deu para usar {}", path.display())),
                }
            }
        }
    }

    // --- Cavalete ---------------------------------------------------------

    fn open_easel(&mut self, cx: &mut Cx, editing: Option<(usize, &Image)>) {
        for s in SND_EASEL {
            cx.sound(s);
        }
        // o cavalete mostra o próprio balão (0fa1[6])
        cx.close_help();
        self.film_open = false;
        self.held = None;
        self.easel = Some(match editing {
            Some((k, img)) => (CrayonEasel::with_image(img, cx.assets), Some(k)),
            None => (CrayonEasel::blank(cx.assets), None),
        });
    }

    fn easel_input(&mut self, ed: &mut Editor, cx: &mut Cx, i: Input) {
        let Some((easel, _)) = self.easel.as_mut() else {
            return;
        };
        let Some(out) = easel.handle(&i, cx.fx, cx.assets) else {
            return;
        };
        let Some((_, editing)) = self.easel.take() else {
            return;
        };
        let EaselOutcome::Ok(img) = out else {
            return;
        };
        cx.sound(SND_OK);
        keep_view(ed, |ed| match editing {
            Some(k) => replace_picture(ed, k, &img),
            None => place_drawing(ed, &img),
        });
    }
}

/// Título do desenho novo do Cavalete (o nome dos adesivos do usuário).
fn drawing_title() -> String {
    USER_STICKER_PREFIX.trim_end_matches('-').to_string()
}

/// OK no Cavalete com desenho novo: o desenho (cortado) vai para a folha à
/// vista, no canto da área de texto mais o lugar onde foi desenhado.
fn place_drawing(ed: &mut Editor, img: &Image) {
    let Some((cut, (dx, dy))) = crop_drawing(img) else {
        return;
    };
    let page = ed.page();
    let (left, top, _, _) = ed.doc().dop.text_area();
    ed.add_picture(
        &cut,
        &drawing_title(),
        page,
        left + dx as i32,
        top + dy as i32,
    );
}

/// OK no Cavalete com um objeto: a figura nova toma o lugar dele (no mesmo
/// canto, no tamanho do desenho), numa ação só do Ovo.
fn replace_picture(ed: &mut Editor, k: usize, img: &Image) {
    let Some(old) = ed.doc().objects.get(k).cloned() else {
        return;
    };
    let Some((cut, _)) = crop_drawing(img) else {
        return;
    };
    let new = ed.add_picture(
        &cut,
        &drawing_title(),
        usize::from(old.page),
        i32::from(old.x),
        i32::from(old.y),
    );
    if new > k {
        // o Ovo guardou o estado antes do add_picture: tirar o antigo é
        // parte da mesma ação
        if k < ed.extra.frame_extra.len() {
            ed.extra.frame_extra.remove(k);
        }
        let doc = ed.doc_mut();
        let sound = old.sound;
        doc.remove_object(k);
        if let Some(o) = doc.objects.get_mut(new - 1) {
            o.sound = sound;
        }
        ed.after_edit();
    }
}

impl StickersStrip {
    fn draw_film(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        let film = self.film();
        let (fx, fy, fw, fh) = film.rect();
        // amarelo pontilhado (xadrez com branco) com borda preta dos lados e
        // embaixo (em cima, o rolo)
        for y in fy..fy + fh - 1 {
            for x in fx + 1..fx + fw - 1 {
                f.put(x, y, if (x + y) % 2 == 0 { YELLOW } else { WHITE });
            }
        }
        f.rect(fx, fy, 1, fh, BLACK);
        f.rect(fx + fw - 1, fy, 1, fh, BLACK);
        f.rect(fx, fy + fh - 1, fw, 1, BLACK);
        f.rect(fx, fy, fw, 1, BLACK);
        // separadores entre as casas e acima das setas
        for j in 0..FILM_CELLS {
            let y = CELL_Y0 + CELL_STEP * j as i32 + CELL + 3;
            f.rect(fx + 1, y, fw - 2, 1, BLACK);
        }
        let Some(roll) = ed.rolls.get(self.roll) else {
            return;
        };
        for j in 0..FILM_CELLS {
            let Some(item) = roll.items.get(self.scroll + j) else {
                break;
            };
            let r = film.cell(j);
            rounded_box(f, r);
            if let Some(t) = &item.preview {
                let (tw, th) = (t.width as i32, t.height as i32);
                let inner = (r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2);
                let (x, y) = (r.0 + r.2 / 2 - tw / 2, r.1 + r.3 / 2 - th / 2);
                blit_clipped(f, t, t, x, y, inner);
            }
        }
        let (u, d) = (film.up(), film.down());
        f.blit_masked(&a.image(&pic(0x02db)), &a.image(&pic(0x02dd)), u.0, u.1);
        f.blit_masked(&a.image(&pic(0x02df)), &a.image(&pic(0x02e1)), d.0, d.1);
    }

    /// O que está na mão, centrado no ponteiro, dentro da página.
    fn draw_held(&self, f: &mut Frame) {
        let (mx, my) = self.mouse;
        if !inside(PAGE, mx, my) || self.finder.is_some() || self.easel.is_some() {
            return;
        }
        let img = match &self.held {
            Some(Held::Clip {
                preview: Some(p), ..
            }) => p.clone(),
            Some(Held::Picture { img, .. }) => img.clone(),
            _ => return,
        };
        let (w, h) = (img.width as i32, img.height as i32);
        let none = Image {
            width: 0,
            height: 0,
            rgba: Vec::new(),
        };
        // a miniatura de traço serve de máscara de si mesma
        let mask: &Image = match &self.held {
            Some(Held::Clip { .. }) => &img,
            _ => &none,
        };
        blit_clipped(f, &img, mask, mx - w / 2, my - h / 2, PAGE);
    }
}

/// Casa da tira: retângulo branco com borda preta e cantos arredondados.
fn rounded_box(f: &mut Frame, (x, y, w, h): Rect) {
    f.rect(x + 2, y, w - 4, h, BLACK);
    f.rect(x, y + 2, w, h - 4, BLACK);
    f.rect(x + 1, y + 1, w - 2, h - 2, BLACK);
    f.rect(x + 2, y + 1, w - 4, h - 2, WHITE);
    f.rect(x + 1, y + 2, w - 2, h - 4, WHITE);
}

impl ToolStrip for StickersStrip {
    fn id(&self) -> ToolId {
        ToolId::Stickers
    }

    fn deactivate(&mut self, _ed: &mut Editor, _cx: &mut Cx) {
        self.close_all();
    }

    fn activate(&mut self, _ed: &mut Editor, _cx: &mut Cx, _again: bool) {
        self.close_all();
    }

    fn modal(&self) -> bool {
        self.finder.is_some() || self.easel.is_some()
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        self.mouse = (x, y);
        if self.finder.is_some() {
            self.finder_click(ed, cx, x, y);
            return true;
        }
        if self.easel.is_some() {
            let click = Input::Click {
                button: MouseButton::Left,
                x,
                y,
            };
            self.easel_input(ed, cx, click);
            return true;
        }
        if self.film_open {
            let film = self.film();
            if inside(film.rect(), x, y) {
                let n = ed.rolls.get(self.roll).map_or(0, |r| r.items.len());
                let max_scroll = n.saturating_sub(FILM_CELLS);
                if inside(film.up(), x, y) {
                    cx.sound(SND_SWITCH);
                    self.scroll = self.scroll.saturating_sub(1);
                } else if inside(film.down(), x, y) {
                    cx.sound(SND_SWITCH);
                    self.scroll = (self.scroll + 1).min(max_scroll);
                } else if let Some(j) = (0..FILM_CELLS).find(|&j| inside(film.cell(j), x, y)) {
                    let item = self.scroll + j;
                    if item < n {
                        cx.sound(SND_CHOOSE);
                        cx.help((WHAT_STICKERS, HELP_THEN_CLICK));
                        let r = &ed.rolls[self.roll];
                        self.held = Some(Held::Clip {
                            roll: r.id,
                            item,
                            preview: r.items[item].preview.clone(),
                        });
                        self.film_open = false;
                    }
                }
                return true;
            }
        }
        if inside(ARROW_UP, x, y) || inside(ARROW_DOWN, x, y) {
            cx.sound(SND_SWITCH);
            let pages = Self::roll_pages(ed);
            self.roll_page = if inside(ARROW_UP, x, y) {
                (self.roll_page + pages - 1) % pages
            } else {
                (self.roll_page + 1) % pages
            };
            self.film_open = false;
            return true;
        }
        if let Some(k) = (0..self.page_rolls(ed)).find(|&k| inside(slot_rect(k), x, y)) {
            let roll = self.roll_page * ROLLS_PER_PAGE + k;
            if self.film_open && roll == self.roll {
                self.film_open = false;
            } else {
                cx.sound(SND_ROLL);
                cx.help((WHAT_STICKERS, HELP_CLICK_STICKER));
                self.roll = roll;
                self.scroll = 0;
                self.film_open = true;
            }
            return true;
        }
        if let Some(k) = StripButton::find(&BUTTONS, x, y) {
            if k == 0 {
                self.open_finder(ed, cx);
            } else {
                self.open_easel(cx, None);
            }
            return true;
        }
        self.film_open = false;
        if self.held.is_some() {
            if !self.paste(ed, cx, x, y) {
                // fora da folha: larga o adesivo
                self.held = None;
                cx.close_help();
            }
            return true;
        }
        if let Some(k) = ed.object_at(x, y)
            && ed.doc().objects[k].kind.is_movable()
        {
            self.grab = Some(Grab {
                object: k,
                from: (x, y),
                last: (x, y),
                moved: false,
            });
            return true;
        }
        false
    }

    fn motion(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) {
        self.mouse = (x, y);
        if let Some((easel, _)) = self.easel.as_mut() {
            easel.handle(&Input::MouseMove { x, y }, cx.fx, cx.assets);
            return;
        }
        let Some(g) = self.grab.as_mut() else {
            return;
        };
        if !g.moved {
            if (x - g.from.0).abs() < DRAG_SLOP && (y - g.from.1).abs() < DRAG_SLOP {
                return;
            }
            g.moved = true;
            ed.record();
            cx.sound(SND_DRAG);
        }
        let (dx, dy) = (x - g.last.0, y - g.last.1);
        g.last = (x, y);
        let (pw, ph) = (
            i32::from(ed.doc().dop.page_width),
            i32::from(ed.doc().dop.page_height),
        );
        if let Some(o) = ed.doc_mut().objects.get_mut(g.object) {
            o.move_by(dx, dy, pw, ph);
        }
    }

    fn release(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) {
        self.mouse = (x, y);
        if self.easel.is_some() {
            let up = Input::MouseUp {
                button: MouseButton::Left,
                x,
                y,
            };
            self.easel_input(ed, cx, up);
            return;
        }
        let Some(g) = self.grab.take() else {
            return;
        };
        if g.moved {
            keep_view(ed, Editor::after_edit);
            return;
        }
        // clique sem arrastar: modificar no Cavalete
        let Some(o) = ed.doc().objects.get(g.object) else {
            return;
        };
        if matches!(o.kind, ObjectKind::Sticker | ObjectKind::Drawing)
            && let Some(img) = ed.picture(o)
        {
            let img = (*img).clone();
            self.open_easel(cx, Some((g.object, &img)));
        }
    }

    /// Faixa de opções dos adesivos: rolos da página, Cachorrinho, Cavalete e setas.
    fn draw_strip(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        let n = self.page_rolls(ed);
        if n > 0 {
            let left = a.image(&pic(0x0245));
            blit_part(
                f,
                &left,
                &a.image(&pic(0x0247)),
                CAP_LEFT_X,
                STRIP_Y,
                (0, 14),
            );
            let right = a.image(&pic(0x0239));
            blit_part(
                f,
                &right,
                &a.image(&pic(0x023b)),
                slot_x(n - 1),
                STRIP_Y,
                (52, 64),
            );
        }
        for k in 0..n {
            let r = &ed.rolls[self.roll_page * ROLLS_PER_PAGE + k];
            let rect = slot_rect(k);
            f.blit_masked(
                &a.image(&pic(0x0249)),
                &a.image(&pic(0x024b)),
                rect.0,
                rect.1,
            );
            if let Some(icon) = &r.icon {
                let (w, h) = (icon.width as i32, icon.height as i32);
                f.blit(icon, rect.0 + 26 - w / 2, rect.1 + 21 - h / 2);
            }
        }
        StripButton::draw_all(&BUTTONS, f, a);
        draw_page_arrows(f, a, self.roll_page + 1);
    }

    fn draw_front(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        if self.film_open {
            self.draw_film(ed, f, a);
        }
        self.draw_held(f);
        if let Some(fd) = &self.finder {
            fd.draw(f, a, &picture_palette(a));
        }
        if let Some((easel, _)) = &self.easel {
            // a moldura e as barras ficam apagadas atrás do cavalete (a
            // página, não)
            let (w, h) = (f.width() as i32, f.height() as i32);
            let (px, py, pw, ph) = PAGE;
            for r in [
                (0, 0, w, py),
                (0, py + ph, w, h - py - ph),
                (0, py, px, ph),
                (px + pw, py, w - px - pw, ph),
            ] {
                gray_region(f, r);
            }
            easel.draw(f, a);
        }
    }
}

/// Setas ▲▼ das faixas de rolos (Adesivos) com o número da página.
pub fn draw_page_arrows(f: &mut Frame, a: &Assets, page: usize) {
    ARROWS_BACK.draw(f, a);
    f.blit_masked(
        &a.image(&pic(0x03b3)),
        &a.image(&pic(0x03b5)),
        ARROW_UP.0,
        ARROW_UP.1,
    );
    f.blit_masked(
        &a.image(&pic(0x03b7)),
        &a.image(&pic(0x03b9)),
        ARROW_DOWN.0,
        ARROW_DOWN.1,
    );
    f.rect(511, 72, 16, 12, BLACK);
    f.rect(512, 73, 14, 10, WHITE);
    f.text(515, 74, &page.to_string(), BLACK);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Effect, Key, Scene};
    use crate::scenes::testutil::{assets, empty_assets};
    use crate::scenes::writer::Writer;
    use crate::writer::doc::Pos;

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
                Effect::PlaySound(crate::assets::SoundRef::Snd(Program::Writer, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    fn select(w: &mut Writer, a: &Assets) {
        let (x, y) = crate::ui::center(ToolId::Stickers.def().hit);
        w.update(&[click(x, y)], 0.0, a);
        assert_eq!(w.tool(), ToolId::Stickers);
    }

    fn square(w: u32, h: u32) -> Rc<Image> {
        Rc::new(Image {
            width: w,
            height: h,
            rgba: [200u8, 0, 0, 255].repeat((w * h) as usize),
        })
    }

    #[test]
    fn geometry_matches_the_original() {
        // 7 rolos: a tampa direita (últimos 12 px da 0239) fica em 342 + 52
        assert_eq!(slot_x(6), 342);
        let film = Film { x: FILM_X0 };
        assert_eq!(film.rect(), (40, 98, 42, 344));
        assert_eq!(film.cell(1), (44, 143, 34, 34));
        assert_eq!(film.cell(7).1 + CELL, 423);
        assert_eq!(film.up(), (46, 428, 12, 12));
    }

    #[test]
    fn crop_keeps_only_the_drawing() {
        let mut img = Image {
            width: 10,
            height: 8,
            rgba: [255u8, 255, 255, 255].repeat(80),
        };
        for (x, y) in [(3, 2), (6, 5)] {
            let i = (y * 10 + x) * 4;
            img.rgba[i..i + 3].copy_from_slice(&[0, 0, 0]);
        }
        let (cut, at) = crop_drawing(&img).unwrap();
        assert_eq!((cut.width, cut.height, at), (4, 4, (3, 2)));
        img.rgba = [255u8, 255, 255, 255].repeat(80);
        assert!(crop_drawing(&img).is_none());
    }

    #[test]
    fn held_picture_is_pasted_centered_and_outside_drops_it() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        select(&mut w, &a);
        w.tools.stickers.held = Some(Held::Picture {
            img: square(20, 10),
            title: "Bola".into(),
        });
        let (x, y) = (300, 300);
        let p = w.editor().to_page(x, y).unwrap();
        let fx = w.update(&[click(x, y)], 0.0, &a);
        assert!(sounds(&fx).contains(&SND_PASTE));
        assert!(w.tools.stickers.held().is_none());
        let o = &w.doc().objects[0];
        assert_eq!((i32::from(o.x), i32::from(o.y)), (p.x - 10, p.y - 5));
        assert_eq!((o.width, o.height), (20, 10));
        assert!(o.outline.is_some());
        // um passo do Ovo tira o adesivo
        assert!(w.editor_mut().undo_step());
        assert!(w.doc().objects.is_empty());
        // fora da folha (o cinza da faixa): larga
        w.tools.stickers.held = Some(Held::Picture {
            img: square(4, 4),
            title: "x".into(),
        });
        w.update(&[click(455, 101)], 0.0, &a);
        assert!(w.tools.stickers.held().is_none());
        assert!(w.doc().objects.is_empty());
    }

    #[test]
    fn drag_moves_and_click_opens_the_easel_to_modify() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[Input::Text("um texto".into())], 0.0, &a);
        select(&mut w, &a);
        w.tools.stickers.held = Some(Held::Picture {
            img: square(40, 40),
            title: "Bola".into(),
        });
        w.update(&[click(300, 300)], 0.0, &a);
        let before = w.doc().objects[0].clone();
        // arrastar move (som 022e) e é uma ação só
        let fx = w.update(
            &[
                click(300, 300),
                Input::MouseMove { x: 310, y: 305 },
                Input::MouseMove { x: 320, y: 310 },
                up(320, 310),
            ],
            0.0,
            &a,
        );
        assert!(sounds(&fx).contains(&SND_DRAG));
        let o = &w.doc().objects[0];
        assert_eq!((o.x - before.x, o.y - before.y), (20, 10));
        assert_eq!(w.editor().sel.caret, Pos::new(0, 8));
        assert!(w.editor_mut().undo_step());
        assert_eq!(w.doc().objects[0].x, before.x);
        // clique sem arrastar: o Cavalete abre com a figura
        let fx = w.update(&[click(300, 300), up(300, 300)], 0.0, &a);
        assert!(sounds(&fx).contains(&0x0267));
        assert!(w.tools.stickers.easel_open());
        assert!(w.tools.get(ToolId::Stickers).modal());
        // OK do cavalete: troca a figura no mesmo lugar, numa ação do Ovo
        let n = w.editor().undo.undo_len();
        let (easel, k) = w.tools.stickers.easel.take().unwrap();
        assert_eq!(k, Some(0));
        let img = easel.image();
        replace_picture(w.editor_mut(), 0, &img);
        assert_eq!(w.doc().objects.len(), 1);
        assert_eq!(w.doc().objects[0].x, before.x);
        assert_eq!(w.editor().undo.undo_len(), n + 1);
        assert_eq!(w.editor().extra.frame_extra.len(), 1);
    }

    #[test]
    fn easel_button_opens_and_no_closes() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        select(&mut w, &a);
        let fx = w.update(&[click(480, 75)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![0x0224, 0x0267]);
        assert!(w.tools.stickers.easel_open());
        // o cavalete fecha com NÃO (e um desenho em branco não cola nada)
        place_drawing(w.editor_mut(), &CrayonEasel::blank(&a).image());
        assert!(w.doc().objects.is_empty());
        w.update(
            &[Input::Key {
                key: Key::Escape,
                ctrl: false,
                alt: false,
            }],
            0.0,
            &a,
        );
    }

    #[test]
    fn dog_opens_the_finder_and_cancel_closes() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut w = Writer::new_blank(Some(dir.path().to_path_buf()), &a);
        select(&mut w, &a);
        let fx = w.update(&[click(430, 78)], 0.0, &a);
        assert_eq!(sounds(&fx), SND_DOG.to_vec());
        let fd = w.tools.stickers.finder().unwrap();
        // só "Fora de Imaginópolis" numa pasta vazia
        assert_eq!(fd.entries().len(), 1);
        let (x, y) = crate::ui::center(crate::scenes::finder_dialog::CANCEL_BTN);
        w.update(&[click(x, y)], 0.0, &a);
        assert!(w.tools.stickers.finder().is_none());
    }

    #[test]
    fn real_rolls_film_and_paste() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new_blank(None, &a);
        let ids: Vec<u16> = w.editor().rolls.iter().map(|r| r.id).collect();
        assert_eq!(ids, (0..29).collect::<Vec<u16>>());
        select(&mut w, &a);
        // 1º rolo (Hobby) → tira de filme → 2º adesivo → na mão
        let fx = w.update(&[click(56, 78)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_ROLL]);
        assert!(w.tools.stickers.film_open());
        let c = w.tools.stickers.film().cell(1);
        let fx = w.update(&[click(c.0 + 10, c.1 + 10)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_CHOOSE]);
        assert!(matches!(
            w.tools.stickers.held(),
            Some(Held::Clip {
                roll: 0,
                item: 1,
                ..
            })
        ));
        w.update(&[click(300, 300)], 0.0, &a);
        let o = &w.doc().objects[0];
        assert_eq!(o.kind, ObjectKind::Sticker);
        let chunk = w.editor().extra.object(o.object).unwrap();
        assert!(chunk.parts.iter().any(|p| p.tag.0 == *b"TWPB"));
        // 5 páginas de rolos: a última tem um
        for _ in 0..4 {
            w.update(&[click(519, 92)], 0.0, &a);
        }
        assert_eq!(w.tools.stickers.roll_page, 4);
        assert_eq!(w.tools.stickers.page_rolls(w.editor()), 1);
    }
}
