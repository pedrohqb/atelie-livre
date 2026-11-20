//! Preparação da página (`office-ferramentas.md` §3,
//! `projects-bordas-fundos.md`): página 1 com o rolo FUNDO, Cria-colunas,
//! Alto ou largo, Margens e Nº de página; página 2 com os três rolos BORDA.
//!
//! - As quatro listas descem do botão (y 94, itens de 49 px separados por
//!   linhas pretas, figura `0c..` no meio do item e o ✓ `029d` no atual)
//!   e mudam o documento na hora ([`Editor::apply_dop`], uma ação do Ovo).
//! - Os rolos abrem uma tira de filme de 86 px (y 98..421): "Nada" e as
//!   miniaturas `PKBM` dos `.BOR` (`0MAX100` para os fundos e a borda 1,
//!   `1MAX100` e `2MAX100` para as bordas 2 e 3), 4 por vista (77 px), com
//!   a borda atual em amarelo (o FUNDO é todo pontilhado de amarelo) e as
//!   setas ↑↓ embaixo (som `02bc`). Escolher toca
//!   `022d` + `021c` e arma o rolo de pintar: o clique seguinte numa margem
//!   da folha põe o fundo ou a borda em todas as folhas (som `0220`); "Nada"
//!   tira na hora. O documento guarda cópias dos chunks (`BDR`, `IBGF` com o
//!   `PICT` e o `FRAM`), e as molduras do fundo passam a desviar o texto.
//! - Restrições: cartão e faixa não aceitam fundo (`0805[32]`); a faixa
//!   não aceita Alto (`0805[17]`); borda mais larga que a margem lembra
//!   `0fa5[11]`.

use super::cursors;
use super::editor::{Editor, to_owned};
use super::tools::{
    Cx, Pic, SND_CHOOSE, SND_OPTION, SND_SWITCH, SWITCH, SWITCH_RECT, StripButton, ToolId,
    ToolStrip, WHAT_PAGE, inside, pic, strip_button,
};
use crate::Frame;
use crate::Image;
use crate::assets::{Assets, CursorRef, Program};
use crate::scene::Key;
use crate::scenes::writer_office_room::bitmap_image;
use crate::ui::Rect;
use crate::writer::borders::{PageGeom, border_layout};
use crate::writer::doc::{Margin, Orientation, PageNumber};
use crate::writer::object::ObjRef;
use formats::Tag;
use formats::bor::{self, Border, Ibgf};
use formats::chnk::{self, OwnedChunk, OwnedPart};
use formats::max::Project;
use std::cell::OnceCell;

const PAGE1: [StripButton; 5] = [
    strip_button(
        0x0821,
        Some(0x0823),
        (38, 57, 120, 42),
        (WHAT_PAGE, 2),
        "Fundo",
    ),
    strip_button(
        0x086b,
        None,
        (188, 61, 49, 34),
        (WHAT_PAGE, 4),
        "Cria-colunas",
    ),
    strip_button(
        0x08c3,
        None,
        (267, 61, 49, 34),
        (WHAT_PAGE, 6),
        "Alto ou largo",
    ),
    strip_button(0x0869, None, (346, 61, 49, 34), (WHAT_PAGE, 8), "Margens"),
    strip_button(
        0x0875,
        None,
        (425, 61, 49, 34),
        (WHAT_PAGE, 9),
        "Nº de página",
    ),
];

const PAGE2: [StripButton; 3] = [
    strip_button(
        0x0953,
        Some(0x0955),
        (104, 57, 108, 42),
        (WHAT_PAGE, 3),
        "Borda 1",
    ),
    strip_button(
        0x0957,
        Some(0x0959),
        (212, 57, 94, 42),
        (WHAT_PAGE, 3),
        "Borda 2",
    ),
    strip_button(
        0x0825,
        Some(0x0827),
        (306, 57, 106, 42),
        (WHAT_PAGE, 3),
        "Borda 3",
    ),
];

/// Sons: aplicar o fundo ou a borda (`0220`), escolher a miniatura (`022d`
/// + `021c`), rolar a tira (`02bc`).
pub const SND_APPLY: u16 = 0x0220;
pub const SND_PICK: u16 = 0x021c;
pub const SND_SCROLL: u16 = 0x02bc;
/// Avisos (`WHAT 0805`): cartão ou faixa sem fundo; faixa sempre Larga.
pub const WARN_NO_BACKGROUND: (u16, usize) = (0x0805, 32);
pub const WARN_BANNER_WIDE: (u16, usize) = (0x0805, 17);
/// "Bordas e margens": a margem precisa caber a borda.
pub const HELP_BORDER_MARGIN: (u16, usize) = (WHAT_PAGE, 11);

/// Listas da página 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Columns,
    Orientation,
    Margins,
    Numbers,
}

/// Lista: canto, largura, figuras dos itens e o centro (x) delas
/// (medidos em `office_63`..`office_66`).
struct ListDef {
    x: i32,
    w: i32,
    items: &'static [u16],
    image_cx: i32,
}

const LIST_TOP: i32 = 94;
const LIST_ITEM_H: i32 = 49;
/// O ✓ do item atual, em (x + 4, topo do item + 19).
const CHECK: u16 = 0x029d;
const CHECK_DX: i32 = 4;
const CHECK_DY: i32 = 19;

const COLUMNS: ListDef = ListDef {
    x: 188,
    w: 59,
    items: &[0x0c0f, 0x0c11, 0x0c13, 0x0c15],
    image_cx: 226,
};
const ORIENTATION: ListDef = ListDef {
    x: 267,
    w: 67,
    items: &[0x0c33, 0x0c35],
    image_cx: 309,
};
const MARGINS: ListDef = ListDef {
    x: 346,
    w: 59,
    items: &[0x0c05, 0x0c07, 0x0c09, 0x0c0b, 0x0c0d],
    image_cx: 384,
};
const NUMBERS: ListDef = ListDef {
    x: 404,
    w: 59,
    items: &[0x0c17, 0x0c19, 0x0c1b],
    image_cx: 442,
};

impl List {
    fn def(self) -> &'static ListDef {
        match self {
            List::Columns => &COLUMNS,
            List::Orientation => &ORIENTATION,
            List::Margins => &MARGINS,
            List::Numbers => &NUMBERS,
        }
    }

    pub fn rect(self) -> Rect {
        let d = self.def();
        (d.x, LIST_TOP, d.w, 1 + d.items.len() as i32 * LIST_ITEM_H)
    }

    pub fn item_rect(self, k: usize) -> Rect {
        let d = self.def();
        (d.x, LIST_TOP + k as i32 * LIST_ITEM_H, d.w, LIST_ITEM_H + 1)
    }

    /// Item marcado com ✓ no documento.
    fn current(self, ed: &Editor) -> usize {
        let d = &ed.doc().dop;
        match self {
            List::Columns => usize::from(d.columns.clamp(1, 4) - 1),
            List::Orientation => usize::from(d.orientation() == Orientation::Wide),
            List::Margins => Margin::ALL
                .iter()
                .position(|m| *m == d.margin())
                .unwrap_or(2),
            List::Numbers => PageNumber::ALL
                .iter()
                .position(|n| *n == d.page_number)
                .unwrap_or(0),
        }
    }
}

/// Rolos: FUNDO (página 1) e os três BORDA (página 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Roll {
    Background,
    Border(usize),
}

/// Arquivos dos rolos (`SHARED/STUFF`).
pub const BACKGROUND_FILE: &str = "0MAX100.BOR";
pub const BORDER_FILES: [&str; 3] = ["0MAX100.BOR", "1MAX100.BOR", "2MAX100.BOR"];

const ROLL_W: i32 = 86;
const ROLL_TOP: i32 = 98;
const ROLL_ITEM_H: i32 = 77;
/// Itens à vista (o "Nada" conta).
const ROLL_ROWS: usize = 4;
/// Faixa das setas, embaixo da tira (406..421).
const ROLL_ARROWS_H: i32 = 15;
const ROLL_UP: (u16, u16) = (0x02db, 0x02dd);
const ROLL_DOWN: (u16, u16) = (0x02df, 0x02e1);
const ROLL_ARROW_DX: [i32; 2] = [19, 53];
const ROLL_ARROW_DY: i32 = 2;
const YELLOW: [u8; 3] = [255, 238, 76];
const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];

impl Roll {
    /// x da tira: 10 px depois do começo do corpo do rolo (`w26_100`,
    /// `w26_92`, `office_57`).
    fn x(self) -> i32 {
        match self {
            Roll::Background => 62,
            Roll::Border(k) => [128, 222, 316][k.min(2)],
        }
    }

    fn index(self) -> usize {
        match self {
            Roll::Background => 0,
            Roll::Border(k) => 1 + k.min(2),
        }
    }

    pub fn rect(self) -> Rect {
        (
            self.x(),
            ROLL_TOP,
            ROLL_W,
            ROLL_ROWS as i32 * ROLL_ITEM_H + ROLL_ARROWS_H + 1,
        )
    }

    /// Linha `row` (0 = a de cima) da tira.
    pub fn row_rect(self, row: usize) -> Rect {
        (
            self.x(),
            ROLL_TOP + row as i32 * ROLL_ITEM_H,
            ROLL_W,
            ROLL_ITEM_H + 1,
        )
    }

    /// Seta ↑ (0) ou ↓ (1).
    pub fn arrow_rect(self, k: usize) -> Rect {
        (
            self.x() + ROLL_ARROW_DX[k],
            ROLL_TOP + ROLL_ROWS as i32 * ROLL_ITEM_H + ROLL_ARROW_DY,
            12,
            12,
        )
    }
}

/// Uma miniatura de rolo.
pub struct RollItem {
    pub id: u16,
    pub name: String,
    pub thumb: Option<Image>,
    /// Fundo de folha Larga.
    pub landscape: bool,
}

/// As miniaturas dos quatro rolos, lidas uma vez dos `.BOR`.
#[derive(Default)]
pub struct Catalog {
    pub backgrounds: Vec<RollItem>,
    pub borders: [Vec<RollItem>; 3],
}

impl Catalog {
    pub fn load(a: &Assets) -> Catalog {
        let pal = a.palettes(Program::Writer);
        let thumb = |b: &Option<formats::pkbm::Bitmap>| {
            b.as_ref().map(|b| bitmap_image(b, &pal.pal16, &pal.pal256))
        };
        let read = |name: &str| a.library_bytes(name).and_then(|d| bor::read(d).ok());
        let mut c = Catalog::default();
        if let Some(f) = read(BACKGROUND_FILE) {
            c.backgrounds = f
                .backgrounds
                .iter()
                .map(|b| RollItem {
                    id: b.id,
                    name: b.name.clone(),
                    thumb: thumb(&b.thumb),
                    landscape: b.landscape,
                })
                .collect();
        }
        for (k, file) in BORDER_FILES.iter().enumerate() {
            if let Some(f) = read(file) {
                c.borders[k] = f
                    .borders
                    .iter()
                    .map(|b| RollItem {
                        id: b.id,
                        name: b.name.clone(),
                        thumb: thumb(&b.thumb),
                        landscape: false,
                    })
                    .collect();
            }
        }
        c
    }

    /// Itens do rolo (sem o "Nada"); o FUNDO só mostra os da orientação
    /// da folha.
    pub fn items(&self, roll: Roll, landscape: bool) -> Vec<&RollItem> {
        match roll {
            Roll::Background => self
                .backgrounds
                .iter()
                .filter(|b| b.landscape == landscape)
                .collect(),
            Roll::Border(k) => self.borders[k.min(2)].iter().collect(),
        }
    }
}

/// Escolha feita no rolo, esperando o clique na margem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Armed {
    pub roll: Roll,
    pub id: u16,
}

#[derive(Default)]
pub struct PageStrip {
    /// Página 2 da faixa (bordas).
    more: bool,
    list: Option<List>,
    roll: Option<Roll>,
    /// Primeiro item à vista em cada rolo (0 = "Nada").
    top: [usize; 4],
    armed: Option<Armed>,
    catalog: OnceCell<Catalog>,
}

/// Cópia de `c` no documento: com o mesmo número se estiver livre (os
/// fundos apontam os seus `PICT` pelo número), senão com um livre.
fn keep_chunk(ed: &mut Editor, c: OwnedChunk, keep_id: bool) -> u16 {
    let r = ObjRef {
        tag: c.tag.0,
        id: c.id,
    };
    match ed.extra.object(r) {
        Some(old) if *old == c => c.id,
        None if keep_id => {
            let id = c.id;
            ed.extra.objects.push(c);
            id
        }
        _ => ed.extra.add_object(c).id,
    }
}

/// Folha do documento (para medir a borda).
fn page_geom(ed: &Editor) -> PageGeom {
    let d = &ed.doc().dop;
    PageGeom {
        width: f64::from(d.page_width),
        height: f64::from(d.page_height),
        margin_top: f64::from(d.margin_top),
        margin_left: f64::from(d.margin_left),
        margin_bottom: f64::from(d.margin_bottom),
        margin_right: f64::from(d.margin_right),
    }
}

/// Põe a borda `id` do arquivo `file` em todas as folhas. `false` = não
/// achou; `Some(false)` no segundo valor = a margem é estreita para ela.
pub fn apply_border(ed: &mut Editor, a: &Assets, file: &str, id: u16) -> Option<bool> {
    let data = a.library_bytes(file)?;
    let f = chnk::parse(data).ok()?;
    let c = f
        .chunks
        .iter()
        .find(|c| c.tag.0 == *b"BDR " && c.id == id)?;
    let border = Border::from_chunk(c).ok()?;
    let new_id = keep_chunk(ed, to_owned(c), false);
    let (inner, outer) = border.dop_widths();
    ed.apply_dop(|d| {
        d.set_border(Some(ObjRef::bdr(new_id)));
        d.border_inner = Some(inner);
        d.border_outer = Some(outer);
    });
    Some(border_layout(&page_geom(ed), &border).fits(&page_geom(ed)))
}

/// Tira a borda das folhas.
pub fn remove_border(ed: &mut Editor) {
    ed.apply_dop(|d| {
        d.set_border(None);
        d.border_inner = None;
        d.border_outer = None;
    });
}

/// Põe o fundo `id` de [`BACKGROUND_FILE`] em todas as folhas: copia o
/// `IBGF` (renumerado), o `PICT` da arte e os das molduras, e o `FRAM`, e
/// as molduras de tipo 6 passam a desviar o texto.
pub fn apply_background(ed: &mut Editor, a: &Assets, id: u16) -> Option<()> {
    let data = a.library_bytes(BACKGROUND_FILE)?;
    let f = chnk::parse(data).ok()?;
    let file = bor::read(data).ok()?;
    let bg = file.background(id)?;
    let chunk = |tag: &[u8; 4], id: u16| f.chunks.iter().find(|c| c.tag.0 == *tag && c.id == id);
    let ic = chunk(b"IBGF", id)?;
    let mut ibgf = Ibgf::parse(ic.parts.iter().find(|p| p.tag.0 == *b"IBGF")?.data).ok()?;
    for (_, pid) in bg.pictures() {
        if let Some(p) = chunk(b"PICT", pid) {
            keep_chunk(ed, to_owned(p), true);
        }
    }
    if let Some(pid) = ibgf.pict {
        ibgf.pict = chunk(b"PICT", pid).map(|p| keep_chunk(ed, to_owned(p), true));
    }
    if let Some(fid) = ibgf.fram {
        // `FRAM 0` é o das molduras do próprio documento: a cópia do fundo
        // vai num número a partir de 1 (o original grava `FRAM 1`)
        ibgf.fram = chunk(b"FRAM", fid).map(|p| {
            let mut c = to_owned(p);
            c.id = ed.extra.free_id(*b"FRAM").max(1);
            let id = c.id;
            ed.extra.objects.push(c);
            id
        });
    }
    let mut copy = to_owned(ic);
    for p in copy.parts.iter_mut().filter(|p| p.tag.0 == *b"IBGF") {
        *p = OwnedPart {
            tag: Tag(*b"IBGF"),
            data: ibgf.to_bytes().to_vec(),
        };
    }
    let new_id = ed.extra.add_object(copy).id;
    let obstacles = bg
        .obstacles()
        .map(|o| {
            let mut o = o.clone();
            o.page = u16::MAX;
            o
        })
        .collect();
    ed.apply_dop(|d| {
        d.background = Some(ObjRef {
            tag: *b"IBGF",
            id: new_id,
        });
        d.obstacles = obstacles;
    });
    Some(())
}

/// Tira o fundo das folhas.
pub fn remove_background(ed: &mut Editor) {
    ed.apply_dop(|d| {
        d.background = None;
        d.obstacles.clear();
    });
}

/// Cartão e faixa não aceitam fundo.
fn refuses_background(ed: &Editor) -> bool {
    ed.doc().dop.banner || matches!(ed.extra.add.project, Project::Card | Project::Banner)
}

fn is_banner(ed: &Editor) -> bool {
    ed.doc().dop.banner || ed.extra.add.project == Project::Banner
}

/// Nome do chunk da borda ou do fundo em uso (para o amarelo do rolo).
fn current_name(ed: &Editor, roll: Roll) -> Option<String> {
    let r = match roll {
        Roll::Background => ed.doc().dop.background,
        Roll::Border(_) => ed.doc().dop.border,
    }?;
    ed.extra
        .object(r)
        .map(|c| c.name.clone().unwrap_or_default())
}

impl PageStrip {
    fn buttons(&self) -> &'static [StripButton] {
        if self.more { &PAGE2 } else { &PAGE1 }
    }

    pub fn list(&self) -> Option<List> {
        self.list
    }

    pub fn roll(&self) -> Option<Roll> {
        self.roll
    }

    pub fn armed(&self) -> Option<Armed> {
        self.armed
    }

    /// Uma borda escolhida no rolo espera o clique na folha.
    pub fn border_armed(&self) -> bool {
        self.armed
            .is_some_and(|a| matches!(a.roll, Roll::Border(_)))
    }

    fn catalog(&self, a: &Assets) -> &Catalog {
        self.catalog.get_or_init(|| Catalog::load(a))
    }

    fn list_click(&mut self, ed: &mut Editor, cx: &mut Cx, list: List, x: i32, y: i32) {
        self.list = None;
        let n = list.def().items.len();
        let Some(k) = (0..n).find(|&k| {
            let (ix, iy, iw, ih) = list.item_rect(k);
            inside((ix, iy + 1, iw, ih - 1), x, y)
        }) else {
            return;
        };
        cx.sound(SND_CHOOSE);
        match list {
            List::Columns => ed.apply_dop(|d| d.set_columns(k as u16 + 1)),
            List::Orientation => {
                let o = [Orientation::Tall, Orientation::Wide][k];
                if o == Orientation::Tall && is_banner(ed) {
                    cx.help_even_off(WARN_BANNER_WIDE);
                    return;
                }
                if ed.doc().dop.orientation() != o {
                    ed.apply_dop(|d| d.set_orientation(o));
                }
            }
            List::Margins => ed.apply_dop(|d| d.set_margin(Margin::ALL[k])),
            List::Numbers => ed.apply_dop(|d| d.set_page_number(PageNumber::ALL[k])),
        }
    }

    fn roll_click(&mut self, ed: &mut Editor, cx: &mut Cx, roll: Roll, x: i32, y: i32) {
        let landscape = ed.doc().dop.orientation() == Orientation::Wide;
        let n = 1 + self.catalog(cx.assets).items(roll, landscape).len();
        let top = &mut self.top[roll.index()];
        for (k, dir) in [(0, -1i32), (1, 1)] {
            if inside(roll.arrow_rect(k), x, y) {
                let max = n.saturating_sub(ROLL_ROWS);
                let next = (*top as i32 + dir).clamp(0, max as i32) as usize;
                if next != *top {
                    *top = next;
                    cx.sound(SND_SCROLL);
                }
                return;
            }
        }
        self.roll = None;
        let Some(row) = (0..ROLL_ROWS).find(|&r| {
            let (rx, ry, rw, rh) = roll.row_rect(r);
            inside((rx, ry + 1, rw, rh - 1), x, y)
        }) else {
            return;
        };
        let item = *top + row;
        if item >= n {
            return;
        }
        cx.sound(SND_CHOOSE);
        if item == 0 {
            // "Nada": tira na hora
            self.armed = None;
            match roll {
                Roll::Background if ed.doc().dop.background.is_some() => remove_background(ed),
                Roll::Border(_) if ed.doc().dop.border.is_some() => remove_border(ed),
                _ => {}
            }
            return;
        }
        let id = self.catalog(cx.assets).items(roll, landscape)[item - 1].id;
        cx.sound(SND_PICK);
        self.armed = Some(Armed { roll, id });
    }

    /// Clique com um fundo ou uma borda escolhidos: numa margem da folha
    /// (fora do texto e dos objetos) aplica a todas as folhas.
    fn apply_click(&mut self, ed: &mut Editor, cx: &mut Cx, armed: Armed, x: i32, y: i32) -> bool {
        let Some(p) = ed.to_page(x, y) else {
            return false;
        };
        let (x0, y0, x1, y1) = ed.doc().dop.text_area();
        let in_margin = !((x0..x1).contains(&p.x) && (y0..y1).contains(&p.y));
        if !in_margin || ed.object_at(x, y).is_some() {
            // borda em adesivo e clique no meio do texto: ainda não
            cx.help(match armed.roll {
                Roll::Background => (WHAT_PAGE, 2),
                Roll::Border(_) => (WHAT_PAGE, 3),
            });
            return true;
        }
        self.armed = None;
        match armed.roll {
            Roll::Background => {
                if apply_background(ed, cx.assets, armed.id).is_some() {
                    cx.sound(SND_APPLY);
                }
            }
            Roll::Border(k) => match apply_border(ed, cx.assets, BORDER_FILES[k], armed.id) {
                Some(fits) => {
                    cx.sound(SND_APPLY);
                    if !fits {
                        cx.help(HELP_BORDER_MARGIN);
                    }
                }
                None => cx.log("borda não encontrada"),
            },
        }
        true
    }

    fn draw_list(&self, ed: &Editor, f: &mut Frame, a: &Assets, list: List) {
        let (x, y, w, h) = list.rect();
        f.rect(x, y, w, h, BLACK);
        f.rect(x + 1, y + 1, w - 2, h - 2, WHITE);
        let d = list.def();
        let cur = list.current(ed);
        for (k, &id) in d.items.iter().enumerate() {
            let (_, iy, _, _) = list.item_rect(k);
            if k > 0 {
                f.rect(x, iy, w, 1, BLACK);
            }
            let img = a.image(&pic(id));
            let (iw, ih) = (img.width as i32, img.height as i32);
            f.blit(&img, d.image_cx - iw / 2, iy + (LIST_ITEM_H - ih + 1) / 2);
            if k == cur {
                f.blit(&a.image(&pic(CHECK)), x + CHECK_DX, iy + CHECK_DY);
            }
        }
    }

    fn draw_roll(&self, ed: &Editor, f: &mut Frame, a: &Assets, roll: Roll) {
        let landscape = ed.doc().dop.orientation() == Orientation::Wide;
        let items = self.catalog(a).items(roll, landscape);
        let current = current_name(ed, roll);
        let (x, y, w, h) = roll.rect();
        f.rect(x, y, w, h, BLACK);
        f.rect(x + 1, y + 1, w - 2, h - 2, WHITE);
        let top = self.top[roll.index()];
        for row in 0..ROLL_ROWS {
            let (rx, ry, rw, rh) = roll.row_rect(row);
            f.rect(rx, ry, rw, 1, BLACK);
            let k = top + row;
            let on = match k {
                0 => current.is_none(),
                _ => items
                    .get(k - 1)
                    .is_some_and(|it| current.as_deref() == Some(it.name.as_str())),
            };
            if roll == Roll::Background {
                // o FUNDO é todo pontilhado de amarelo e branco (`office_57`,
                // `w26_110`), sem destaque do atual
                for yy in ry + 1..ry + rh - 1 {
                    for xx in (rx + 1 + (yy + rx + 1) % 2..rx + rw - 1).step_by(2) {
                        f.put(xx, yy, YELLOW);
                    }
                }
            } else if on && k <= items.len() {
                f.rect(rx + 1, ry + 1, rw - 2, rh - 2, YELLOW);
            }
            if k == 0 {
                let font = a.font(8);
                let s = "Nada";
                match &font {
                    Some(font) => {
                        let tw = font.width(s) as i32;
                        f.text_font(font, rx + (rw - tw) / 2, ry + rh / 2 - 6, s, BLACK);
                    }
                    None => f.text(rx + (rw - 32) / 2, ry + rh / 2 - 4, s, BLACK),
                }
            } else if let Some(img) = items.get(k - 1).and_then(|it| it.thumb.as_ref()) {
                let (iw, ih) = (img.width as i32, img.height as i32);
                f.blit(img, rx + (rw - iw) / 2, ry + (rh - ih + 1) / 2);
            }
        }
        let (_, ay, _, _) = roll.arrow_rect(0);
        f.rect(x, ay - ROLL_ARROW_DY, w, 1, BLACK);
        for (k, (id, mask)) in [ROLL_UP, ROLL_DOWN].into_iter().enumerate() {
            let (ax, ay, _, _) = roll.arrow_rect(k);
            Pic::masked(id, mask, ax, ay).draw(f, a);
        }
    }
}

impl ToolStrip for PageStrip {
    fn id(&self) -> ToolId {
        ToolId::Page
    }

    fn deactivate(&mut self, _ed: &mut Editor, _cx: &mut Cx) {
        self.list = None;
        self.roll = None;
        self.armed = None;
    }

    fn modal(&self) -> bool {
        self.list.is_some() || self.roll.is_some()
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if let Some(list) = self.list {
            self.list_click(ed, cx, list, x, y);
            return true;
        }
        if let Some(roll) = self.roll {
            if inside(roll.rect(), x, y) {
                self.roll_click(ed, cx, roll, x, y);
            } else {
                self.roll = None;
            }
            return true;
        }
        if inside(SWITCH_RECT, x, y) {
            cx.sound(SND_SWITCH);
            cx.help((WHAT_PAGE, 1));
            self.more = !self.more;
            self.armed = None;
            return true;
        }
        if let Some(k) = StripButton::find(self.buttons(), x, y) {
            let b = &self.buttons()[k];
            self.armed = None;
            cx.sound(SND_OPTION);
            if let Some(h) = b.help {
                cx.help(h);
            }
            if self.more {
                self.roll = Some(Roll::Border(k));
            } else if k == 0 {
                if refuses_background(ed) {
                    cx.help_even_off(WARN_NO_BACKGROUND);
                } else {
                    self.roll = Some(Roll::Background);
                }
            } else {
                self.list = Some(
                    [
                        List::Columns,
                        List::Orientation,
                        List::Margins,
                        List::Numbers,
                    ][k - 1],
                );
            }
            return true;
        }
        if let Some(armed) = self.armed {
            return self.apply_click(ed, cx, armed, x, y);
        }
        false
    }

    fn key(&mut self, _ed: &mut Editor, _cx: &mut Cx, key: Key, _ctrl: bool) -> bool {
        if key == Key::Escape && (self.modal() || self.armed.is_some()) {
            self.list = None;
            self.roll = None;
            self.armed = None;
            return true;
        }
        false
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(self.buttons(), f, a);
        SWITCH.draw(f, a);
    }

    fn draw_front(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        if let Some(list) = self.list {
            self.draw_list(ed, f, a, list);
        }
        if let Some(roll) = self.roll {
            self.draw_roll(ed, f, a, roll);
        }
    }

    fn cursor(&self, ed: &Editor, x: i32, y: i32) -> Option<CursorRef> {
        (self.armed.is_some() && !self.modal() && ed.to_page(x, y).is_some())
            .then(|| cursors::writer(cursors::ROLLER))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Effect;
    use crate::scenes::testutil::empty_assets;
    use crate::scenes::writer::tools::snd;

    fn press(s: &mut PageStrip, ed: &mut Editor, a: &Assets, x: i32, y: i32) -> Vec<Effect> {
        let mut fx = Vec::new();
        let mut cx = Cx::new(&mut fx, a);
        s.press(ed, &mut cx, x, y);
        let _ = cx.take_help();
        fx
    }

    fn center((x, y, w, h): Rect) -> (i32, i32) {
        (x + w / 2, y + h / 2)
    }

    #[test]
    fn lists_change_the_page_setup_with_undo() {
        let (_d, a) = empty_assets();
        let mut ed = Editor::blank(None, &a);
        let mut s = PageStrip::default();
        // Cria-colunas → 3 colunas
        press(&mut s, &mut ed, &a, 200, 70);
        assert_eq!(s.list(), Some(List::Columns));
        assert_eq!(List::Columns.rect(), (188, 94, 59, 197));
        let (x, y) = center(List::Columns.item_rect(2));
        let fx = press(&mut s, &mut ed, &a, x, y);
        assert!(fx.contains(&snd(SND_CHOOSE)));
        assert_eq!(ed.doc().dop.columns, 3);
        assert_eq!(s.list(), None);
        // Margens → 3,8 cm; Nº de página → "1"; Largo
        press(&mut s, &mut ed, &a, 360, 70);
        let (x, y) = center(List::Margins.item_rect(4));
        press(&mut s, &mut ed, &a, x, y);
        assert_eq!(ed.doc().dop.margin_left, 108);
        press(&mut s, &mut ed, &a, 440, 70);
        let (x, y) = center(List::Numbers.item_rect(2));
        press(&mut s, &mut ed, &a, x, y);
        assert_eq!(ed.doc().dop.page_number, PageNumber::Plain);
        press(&mut s, &mut ed, &a, 280, 70);
        let (x, y) = center(List::Orientation.item_rect(1));
        press(&mut s, &mut ed, &a, x, y);
        assert_eq!(
            (ed.doc().dop.page_width, ed.doc().dop.page_height),
            (792, 595)
        );
        // cada escolha é uma ação do Ovo
        assert!(ed.undo_step());
        assert_eq!(ed.doc().dop.page_width, 595);
        // clicar fora fecha a lista sem mudar nada
        press(&mut s, &mut ed, &a, 200, 70);
        press(&mut s, &mut ed, &a, 500, 300);
        assert_eq!((s.list(), ed.doc().dop.columns), (None, 3));
    }

    #[test]
    fn banner_refuses_tall_and_backgrounds() {
        let (_d, a) = empty_assets();
        let mut ed = Editor::blank(None, &a);
        ed.extra.add.project = Project::Banner;
        ed.apply_dop(|d| d.set_orientation(Orientation::Wide));
        let mut s = PageStrip::default();
        press(&mut s, &mut ed, &a, 280, 70);
        let (x, y) = center(List::Orientation.item_rect(0));
        press(&mut s, &mut ed, &a, x, y);
        assert_eq!(ed.doc().dop.orientation(), Orientation::Wide);
        let mut fx = Vec::new();
        let mut cx = Cx::new(&mut fx, &a);
        s.press(&mut ed, &mut cx, 60, 70);
        assert_eq!(cx.take_help(), Some((WARN_NO_BACKGROUND, true)));
        assert_eq!(s.roll(), None);
    }

    #[test]
    fn roll_without_data_has_only_nothing() {
        let (_d, a) = empty_assets();
        let mut ed = Editor::blank(None, &a);
        let mut s = PageStrip::default();
        press(&mut s, &mut ed, &a, 515, 70);
        press(&mut s, &mut ed, &a, 150, 70);
        assert_eq!(s.roll(), Some(Roll::Border(0)));
        assert_eq!(Roll::Border(0).rect(), (128, 98, 86, 324));
        // a seta não rola sem itens; "Nada" fecha sem mudar
        let fx = press(&mut s, &mut ed, &a, 128 + 53 + 5, 98 + 308 + 6);
        assert!(!fx.contains(&snd(SND_SCROLL)));
        let (x, y) = center(Roll::Border(0).row_rect(0));
        press(&mut s, &mut ed, &a, x, y);
        assert_eq!(
            (s.roll(), s.armed(), ed.doc().dop.border),
            (None, None, None)
        );
    }

    #[test]
    fn real_assets_background_and_border_survive_save_and_open() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let mut ed = Editor::blank(Some(dir.path().to_path_buf()), &a);
        ed.type_text("Era uma vez");
        let mut s = PageStrip::default();
        // rolo FUNDO: Nada + 12 fundos de folha Alta; o 1º é o Scroll
        press(&mut s, &mut ed, &a, 100, 75);
        let cat = s.catalog(&a);
        assert_eq!(cat.items(Roll::Background, false).len(), 12);
        assert!(
            cat.items(Roll::Background, false)[0]
                .name
                .ends_with("Scroll")
        );
        assert!(cat.items(Roll::Border(0), false)[0].name.ends_with("Paw"));
        let (x, y) = center(Roll::Background.row_rect(1));
        let fx = press(&mut s, &mut ed, &a, x, y);
        assert!(fx.contains(&snd(SND_PICK)));
        assert!(s.armed().is_some());
        assert_eq!(
            s.cursor(&ed, 40, 300),
            Some(cursors::writer(cursors::ROLLER))
        );
        // clique no meio do texto não aplica; na margem, sim
        press(&mut s, &mut ed, &a, 300, 200);
        assert!(ed.doc().dop.background.is_none());
        let fx = press(&mut s, &mut ed, &a, 40, 300);
        assert!(fx.contains(&snd(SND_APPLY)));
        assert!(ed.doc().dop.background.is_some());
        assert_eq!(ed.doc().dop.obstacles.len(), 4);
        // o texto passa para baixo do rolo de cima do pergaminho
        assert!(ed.layout().lines[0].top > 120);
        // borda Paw (rolo 1)
        press(&mut s, &mut ed, &a, 515, 70);
        press(&mut s, &mut ed, &a, 150, 75);
        let (x, y) = center(Roll::Border(0).row_rect(1));
        press(&mut s, &mut ed, &a, x, y);
        press(&mut s, &mut ed, &a, 40, 300);
        assert!(ed.doc().dop.border.is_some());
        assert_eq!(ed.doc().dop.border_inner, Some(33));
        // a folha ganha tinta na margem (borda e fundo)
        let f = ed.render_page_with_art(&a, 0);
        let inked = (0..60).filter(|&y| f.pixel(30, y * 10)[0] < 128).count();
        assert!(inked > 0);
        // gravar e abrir de novo
        let path = ed.save_as("fundo", "teste", false).unwrap();
        let mut back = Editor::blank(Some(dir.path().to_path_buf()), &a);
        back.open(&path).unwrap();
        let d = &back.doc().dop;
        assert_eq!(d.obstacles.len(), 4);
        assert!(back.extra.object(d.background.unwrap()).is_some());
        assert!(back.extra.object(d.border.unwrap()).is_some());
        assert_eq!(back.render_page_with_art(&a, 0).pixels(), f.pixels());
        // a impressão recebe a folha sem a arte (ela a redesenha)
        assert_ne!(ed.render_page(&a, 0).pixels(), f.pixels());
        // Nada tira o fundo (desfazível)
        press(&mut s, &mut back, &a, 515, 70);
        press(&mut s, &mut back, &a, 100, 75);
        let (x, y) = center(Roll::Background.row_rect(0));
        press(&mut s, &mut back, &a, x, y);
        assert!(back.doc().dop.background.is_none());
        assert!(back.doc().dop.obstacles.is_empty());
        assert!(back.undo_step());
        assert!(back.doc().dop.background.is_some());
    }
}
