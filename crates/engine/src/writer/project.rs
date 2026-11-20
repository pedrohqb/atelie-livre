//! Documentos da Oficina de Projetos do Creative Writer (jornal, cartão e
//! faixa; tarefas P12 e P18 de `cw/report/projects.md`): a folha de um
//! projeto novo, as marcas gravadas no `.MAX` (`WIZA`, `FLOR 3`, `ADD `),
//! a borda copiada para dentro do documento e a palavra com forma esticada
//! através das N folhas da faixa.
//!
//! O que se sabe dos arquivos do original (`projects.md` §6,
//! `projects-faixa.md`, `projects-cartao.md`):
//!
//! - **Jornal**: começa com 2 colunas (`projects_06_jornal_b`) e a marca
//!   `DOP 0x4c` (`newspaper`).
//! - **Cartão**: folha 595×792, margens de 108 pt, 42 pt entre colunas,
//!   4 folhas fixas (`PGD`); a quinta não é impressa (`3c00[106]`).
//! - **Faixa**: folha 577×595 com margens de 72 pt e a marca `DOP 0x5b`
//!   (`FAIXA_PA.MAX`); de 1 a 8 folhas lado a lado; a palavra com forma vai
//!   de margem a margem através de todas (`SPLE` de 1587 pt em 3 folhas:
//!   3 × 577 − 2 × 72).
//! - `ADD[11]` foi 1 no jornal, 2 na faixa e 3 no cartão nos arquivos
//!   gravados ([`view_code`]); `ADD[10]` e `FLOR` = 3.

use super::assistant::{Event, Kind, LABEL_NONE, Menu, TALL, WIDE};
use super::convert::{self, MaxExtra};
use super::doc::{Document, DopProps, Orientation, PageNumber};
use super::object::{ObjRef, ObjectKind};
use formats::chnk::{self, OwnedChunk};
use formats::max::{self, FLOOR_PROJECTS};

/// Folhas fixas do cartão (Capa, Página Interna 2 e 3, Contracapa).
pub const CARD_PAGES: u16 = 4;
/// Folha da faixa (`FAIXA_PA.MAX`: `DOP 0x3e = 577`, `0x3f = 595`).
pub const BANNER_PAGE: (u16, u16) = (577, 595);
/// Bibliotecas de bordas e fundos do Writer (`SHARED/STUFF`).
pub const BORDER_LIBRARIES: [&str; 4] =
    ["0MAX100.BOR", "1MAX100.BOR", "2MAX100.BOR", "0MAG100.BOR"];

/// Andar na barra de baixo durante um projeto (e nos projetos reabertos).
pub const FLOOR_LABEL: &str = "Oficina de Projetos";
/// Avisos do Rex do bloco `0805` (`projects-faixa.md`): "Você não pode
/// mudar para Alto. As faixas são sempre impressas como Largo." e "Você não
/// pode pôr um fundo…" (cartão e faixa).
pub const ALERT_BLOCK: u16 = 0x0805;
pub const ALERT_BANNER_TALL: usize = 17;
pub const ALERT_NO_BACKGROUND: usize = 32;
/// O cartão tem 4 folhas: a quinta não é impressa (`3c00[106]`).
pub const CARD_FIFTH_PAGE: usize = 5;

/// Código do projeto no `.MAX`.
pub fn project_of(kind: Kind) -> max::Project {
    max::Project::from_bytes(kind.wiza())
}

/// Projeto de um `.MAX` (`None` para os documentos do Escritório).
pub fn kind_of(p: max::Project) -> Option<Kind> {
    Kind::from_wiza(p.to_bytes())
}

/// `ADD[11]` dos arquivos gravados no original (1 jornal, 2 faixa,
/// 3 cartão; talvez a visão em uso, talvez o projeto).
pub fn view_code(kind: Kind) -> u8 {
    match kind {
        Kind::News => 1,
        Kind::Banner => 2,
        Kind::Card => 3,
    }
}

/// Folha de um projeto novo.
pub fn page_setup(kind: Kind, dop: &mut DopProps) {
    match kind {
        Kind::News => {
            dop.columns = 2;
            dop.newspaper = true;
        }
        Kind::Card => {
            dop.margin_top = 108;
            dop.margin_left = 108;
            dop.margin_bottom = 108;
            dop.margin_right = 108;
            dop.column_gap = 42;
        }
        Kind::Banner => {
            (dop.page_width, dop.page_height) = BANNER_PAGE;
            dop.margin_top = 72;
            dop.margin_left = 72;
            dop.margin_bottom = 72;
            dop.margin_right = 72;
            dop.banner = true;
        }
    }
}

/// Marca o documento como do projeto `kind`, feito na Oficina de Projetos.
pub fn mark(extra: &mut MaxExtra, kind: Kind) {
    extra.add.project = project_of(kind);
    extra.add.floor = FLOOR_PROJECTS;
    extra.add.view = view_code(kind);
}

/// Documento em branco do projeto.
pub fn blank(kind: Kind) -> (Document, MaxExtra) {
    let (mut doc, mut extra) = convert::blank();
    page_setup(kind, &mut doc.dop);
    if kind == Kind::Card {
        doc.min_pages = CARD_PAGES;
    }
    mark(&mut extra, kind);
    (doc, extra)
}

/// Sem palavras nem objetos (o Max do fim mostra o aviso do Rex).
pub fn is_blank(doc: &Document) -> bool {
    doc.objects.is_empty() && doc.text().trim().is_empty()
}

/// Faixa com `pages` folhas (1…8): as folhas ficam fixas e cada palavra com
/// forma é esticada de margem a margem através de todas, na altura da área
/// de texto (centrada). Verdadeiro se algo mudou.
pub fn stretch_banner(doc: &mut Document, pages: u8) -> bool {
    let n = u16::from(pages.max(1));
    let dop = doc.dop.clone();
    let mut changed = doc.min_pages != n;
    doc.min_pages = n;
    let (pw, ph) = (i32::from(dop.page_width), i32::from(dop.page_height));
    let (ml, mr) = (i32::from(dop.margin_left), i32::from(dop.margin_right));
    let (mt, mb) = (i32::from(dop.margin_top), i32::from(dop.margin_bottom));
    let w = (i32::from(n) * pw - ml - mr).max(1);
    let avail = (ph - mt - mb).max(1);
    // `FAIXA_PA.MAX`: 421 pt de altura em 451 pt de área de texto
    let h = (avail * 421 / 451).max(1);
    for o in doc
        .objects
        .iter_mut()
        .filter(|o| o.kind == ObjectKind::Shaped)
    {
        let new = (
            0u16,
            ml as i16,
            (mt + (avail - h) / 2) as i16,
            w as u16,
            h as u16,
        );
        if (o.page, o.x, o.y, o.width, o.height) != new {
            (o.page, o.x, o.y, o.width, o.height) = new;
            changed = true;
        }
    }
    changed
}

/// Copia para dentro do documento a borda que a folha usa (`DOP 0x58 → BDR
/// n`), se ela ainda não está nos objetos: o original grava a borda inteira
/// (`WBDR` + `PKBM`) no `.MAX`. `libraries` são os `.BOR` (nome, bytes).
/// Verdadeiro se copiou.
pub fn copy_border(doc: &Document, extra: &mut MaxExtra, libraries: &[(&str, &[u8])]) -> bool {
    let Some(r) = doc.dop.border else {
        return false;
    };
    if extra.object(r).is_some() {
        return false;
    }
    for (_, data) in libraries {
        let Ok(f) = chnk::parse(data) else { continue };
        if let Some(c) = f.chunks.iter().find(|c| c.tag.0 == r.tag && c.id == r.id) {
            extra.objects.push(OwnedChunk::from(c));
            return true;
        }
    }
    false
}

/// O que o assistente acompanha no documento entre dois quadros: a cena
/// compara o antes e o depois de cada leva de entradas e manda ao
/// assistente os eventos que as ferramentas causaram ([`events`]), qualquer
/// que seja a interface delas (menus da Preparação da página, cavalete de
/// palavras com forma, rolos).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub columns: u16,
    pub orientation: Orientation,
    /// Largura e altura da folha (a faixa é "largo" com 577×595).
    pub size: (u16, u16),
    pub page_number: PageNumber,
    pub border: Option<ObjRef>,
    pub background: Option<ObjRef>,
    /// Palavras com forma e adesivos na página.
    pub shaped: usize,
    pub stickers: usize,
    /// Folhas na diagramação.
    pub pages: usize,
}

impl Snapshot {
    pub fn of(doc: &Document, pages: usize) -> Snapshot {
        let count = |k: ObjectKind| doc.objects.iter().filter(|o| o.kind == k).count();
        let d = &doc.dop;
        Snapshot {
            columns: d.columns,
            orientation: d.orientation(),
            size: (d.page_width, d.page_height),
            page_number: d.page_number,
            border: d.border,
            background: d.background,
            shaped: count(ObjectKind::Shaped),
            stickers: count(ObjectKind::Sticker),
            pages,
        }
    }
}

/// Eventos do assistente que a mudança `before` → `after` do projeto
/// `kind` representa: item do menu de colunas, de Alto/Largo ou de
/// etiquetas; borda escolhida; palavra com forma posta (OK do cavalete);
/// adesivo escolhido e posto; a quinta folha do cartão.
pub fn events(kind: Kind, before: &Snapshot, after: &Snapshot) -> Vec<Event> {
    let mut v = Vec::new();
    if after.columns != before.columns {
        v.push(Event::MenuItem(
            Menu::Columns,
            after.columns.clamp(1, 4) as u8,
        ));
    }
    if after.orientation != before.orientation {
        let o = match after.orientation {
            Orientation::Tall => TALL,
            Orientation::Wide => WIDE,
        };
        v.push(Event::MenuItem(Menu::Orientation, o));
    }
    if after.page_number != before.page_number {
        let code = after.page_number.code().unwrap_or(LABEL_NONE);
        v.push(Event::MenuItem(Menu::PageNumber, code));
    }
    if after.border != before.border && after.border.is_some() {
        v.push(Event::BorderPicked);
    }
    if after.shaped > before.shaped {
        v.push(Event::ShapedOk);
    }
    if after.stickers > before.stickers {
        v.extend([Event::StickerPicked, Event::StickerPlaced]);
    }
    if kind == Kind::Card && before.pages < CARD_FIFTH_PAGE && after.pages >= CARD_FIFTH_PAGE {
        v.push(Event::FifthPage);
    }
    v
}

/// O que o projeto não aceita (`projects-faixa.md`, `projects-bordas-fundos.md`):
/// a faixa não muda de Alto/Largo e cartões e faixas não têm fundo. Desfaz
/// a mudança no `doc` e devolve o aviso do Rex (`0805[N]`).
pub fn restrict(kind: Kind, before: &Snapshot, doc: &mut Document) -> Option<(u16, usize)> {
    let d = &mut doc.dop;
    if kind == Kind::Banner && (d.page_width, d.page_height) != before.size {
        (d.page_width, d.page_height) = before.size;
        d.page_number_rect = None;
        return Some((ALERT_BLOCK, ALERT_BANNER_TALL));
    }
    if kind != Kind::News && d.background.is_some() && d.background != before.background {
        d.background = before.background;
        return Some((ALERT_BLOCK, ALERT_NO_BACKGROUND));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::object::FloatObject;
    use formats::Tag;

    fn shaped(doc: &mut Document) {
        let r = ObjRef {
            tag: *b"SPLE",
            id: 0,
        };
        doc.objects.push(FloatObject::new(
            ObjectKind::Shaped,
            r,
            0,
            100,
            120,
            200,
            40,
        ));
    }

    #[test]
    fn blank_projects_have_their_page() {
        let (n, e) = blank(Kind::News);
        assert_eq!(n.dop.columns, 2);
        assert!(n.dop.newspaper);
        assert_eq!(e.add.project, max::Project::Newspaper);
        assert_eq!(e.add.floor, FLOOR_PROJECTS);
        assert_eq!(e.add.view, 1);
        let (c, e) = blank(Kind::Card);
        assert_eq!((c.dop.margin_left, c.dop.column_gap), (108, 42));
        assert_eq!(c.min_pages, CARD_PAGES);
        assert_eq!(e.add.project, max::Project::Card);
        let (b, e) = blank(Kind::Banner);
        assert_eq!((b.dop.page_width, b.dop.page_height), BANNER_PAGE);
        assert!(b.dop.banner);
        assert_eq!(e.add.project.to_bytes(), *b"banr");
        assert_eq!(e.add.view, 2);
    }

    #[test]
    fn kinds_round_trip_through_wiza() {
        for k in Kind::ALL {
            assert_eq!(kind_of(project_of(k)), Some(k));
        }
        assert_eq!(kind_of(max::Project::None), None);
    }

    #[test]
    fn blank_means_no_words_and_no_objects() {
        let (mut d, _) = blank(Kind::Banner);
        assert!(is_blank(&d));
        shaped(&mut d);
        assert!(!is_blank(&d));
        let (mut d, _) = blank(Kind::News);
        d.paras[0]
            .items
            .push(crate::writer::doc::Item::Char(' ', Default::default()));
        assert!(is_blank(&d), "só espaço");
    }

    #[test]
    fn banner_word_spans_all_pages_like_faixa_pa() {
        let (mut d, _) = blank(Kind::Banner);
        shaped(&mut d);
        assert!(stretch_banner(&mut d, 3));
        let o = &d.objects[0];
        // FAIXA_PA.MAX: SPLE de 1587 pt através de 3 folhas de 577
        assert_eq!((o.x, o.width), (72, 1587));
        assert_eq!(o.height, 421);
        assert_eq!(d.min_pages, 3);
        assert_eq!(d.object_span(0), 0..3);
        assert_eq!(d.object_pages(), 3);
        assert!(d.object_on_page(0, 2));
        assert!(!stretch_banner(&mut d, 3), "nada a mudar");
        stretch_banner(&mut d, 1);
        assert_eq!(d.object_span(0), 0..1);
        assert_eq!(d.objects[0].width, 577 - 144);
    }

    #[test]
    fn only_banners_span_pages() {
        let (mut d, _) = blank(Kind::News);
        shaped(&mut d);
        d.objects[0].width = 2000;
        assert_eq!(d.object_span(0), 0..1);
    }

    #[test]
    fn document_changes_become_assistant_events() {
        let (mut d, _) = blank(Kind::Card);
        let s0 = Snapshot::of(&d, 4);
        assert!(events(Kind::Card, &s0, &s0).is_empty());
        d.dop.set_columns(3);
        d.dop.set_orientation(Orientation::Wide);
        d.dop.page_number = PageNumber::Label;
        d.dop.border = Some(ObjRef {
            tag: *b"BDR ",
            id: 2,
        });
        shaped(&mut d);
        let s1 = Snapshot::of(&d, 5);
        assert_eq!(
            events(Kind::Card, &s0, &s1),
            [
                Event::MenuItem(Menu::Columns, 3),
                Event::MenuItem(Menu::Orientation, WIDE),
                Event::MenuItem(Menu::PageNumber, super::super::assistant::LABEL_PAGE),
                Event::BorderPicked,
                Event::ShapedOk,
                Event::FifthPage,
            ]
        );
        // a quinta folha só conta no cartão, e só ao chegar nela
        assert!(!events(Kind::News, &s0, &s1).contains(&Event::FifthPage));
        let s2 = Snapshot::of(&d, 6);
        assert!(events(Kind::Card, &s1, &s2).is_empty());
        // apagar a borda não é escolher uma
        d.dop.border = None;
        assert!(events(Kind::Card, &s2, &Snapshot::of(&d, 6)).is_empty());
    }

    #[test]
    fn stickers_are_picked_and_placed() {
        let (mut d, _) = blank(Kind::Card);
        let s0 = Snapshot::of(&d, 4);
        let r = ObjRef {
            tag: *b"PICT",
            id: 1,
        };
        d.objects
            .push(FloatObject::new(ObjectKind::Sticker, r, 0, 10, 10, 40, 40));
        assert_eq!(
            events(Kind::Card, &s0, &Snapshot::of(&d, 4)),
            [Event::StickerPicked, Event::StickerPlaced]
        );
    }

    #[test]
    fn banners_stay_wide_and_have_no_background() {
        let (mut d, _) = blank(Kind::Banner);
        let s0 = Snapshot::of(&d, 1);
        std::mem::swap(&mut d.dop.page_width, &mut d.dop.page_height);
        assert_eq!(
            restrict(Kind::Banner, &s0, &mut d),
            Some((ALERT_BLOCK, ALERT_BANNER_TALL))
        );
        assert_eq!((d.dop.page_width, d.dop.page_height), BANNER_PAGE);
        let fundo = Some(ObjRef {
            tag: *b"BDR ",
            id: 9,
        });
        d.dop.background = fundo;
        assert_eq!(
            restrict(Kind::Banner, &s0, &mut d),
            Some((ALERT_BLOCK, ALERT_NO_BACKGROUND))
        );
        assert_eq!(d.dop.background, None);
        // o jornal aceita fundo e muda de orientação
        let (mut n, _) = blank(Kind::News);
        let s0 = Snapshot::of(&n, 1);
        n.dop.background = fundo;
        n.dop.set_orientation(Orientation::Wide);
        assert_eq!(restrict(Kind::News, &s0, &mut n), None);
        assert_eq!(n.dop.background, fundo);
    }

    #[test]
    fn border_is_copied_from_the_libraries() {
        let (mut d, mut e) = blank(Kind::Card);
        assert!(!copy_border(&d, &mut e, &[]));
        let bdr = OwnedChunk::single(b"BDR ", 7, b"WBDR", vec![1, 2, 3]);
        let lib = chnk::write(Tag(*b"WBDR"), &[bdr]);
        d.dop.border = Some(ObjRef {
            tag: *b"BDR ",
            id: 7,
        });
        let before = e.objects.len();
        assert!(copy_border(&d, &mut e, &[("X.BOR", &lib)]));
        assert_eq!(e.objects.len(), before + 1);
        assert!(!copy_border(&d, &mut e, &[("X.BOR", &lib)]), "já copiada");
    }
}
