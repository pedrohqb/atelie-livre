//! O documento aberto no Escritório e a vista da página: texto rico,
//! objetos soltos, seleção, digitação, Ovo (20 ações), área de transferência,
//! diagramação com as fontes do Writer, rolagem e os arquivos `.MAX`.
//!
//! É o que as ferramentas leem e mudam (recebem `&mut Editor`); a API está
//! descrita em `docs/telas/creative-writer.md` ("API do `Editor`"). Regras:
//!
//! - o documento só muda por [`Editor::doc_mut`] ou pelos métodos de edição
//!   (que já guardam o estado para o Ovo e marcam o documento como mudado);
//!   isso descarta a diagramação guardada;
//! - toda ação desfazível chama [`Editor::record`] antes de mudar;
//! - coordenadas: a folha é medida em pontos e, na Visão normal, 1 ponto =
//!   1 pixel; [`Editor::to_page`] e [`Editor::to_screen`] convertem.
//!
//! Na Visão normal a folha começa em (21, 107) na tela — o texto, com as
//! margens de 72 pontos do documento novo, em (93, 179), como no original
//! (`office.md` §2, `writer-fontes.md`) — e as folhas ficam empilhadas,
//! com [`PAGE_GAP`] entre elas. O Visor de páginas muda o zoom e a
//! arrumação ([`Editor::view`], `view.rs`): as conversões e o desenho passam
//! todos por [`Editor::sheets`].

use super::tools::inside;
use super::view::{Sheets, View};
use crate::Frame;
use crate::Image;
use crate::assets::Assets;
use crate::clipart::{ClipRoll, load_rolls, parse_list};
use crate::scene::Key;
use crate::ui::Rect;
use crate::writer::borders;
use crate::writer::cipher;
use crate::writer::convert::{self, MaxExtra};
use crate::writer::doc::{CharChange, CharProps, Document, DopProps, Item, ParaChange};
use crate::writer::fonts::{RunFont, WriterFonts};
use crate::writer::ideas::story::{self, IdeaStory};
use crate::writer::layout::{FontKey, Layout, Measure};
use crate::writer::new_style::NewStyle;
use crate::writer::object::{FloatObject, ObjRef, ObjectKind};
use crate::writer::selection::{Clip, Clipboard, Motion, Selection};
use crate::writer::store;
use crate::writer::undo::{Snapshot, UndoStack};
use formats::chnk::{self, OwnedChunk, OwnedPart};
use formats::{max, pkbm, twpb, wmf};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Bibliotecas de clip-art dos rolos de adesivos: as mesmas quatro do Fine
/// Artist (o original mostra os rolos de todas, em ordem de número).
pub const LIBRARIES: [&str; 4] = crate::clipart::LIBRARIES;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
/// Fundo da área da página fora das folhas (as laterais cinza).
const BACKDROP: [u8; 3] = [140, 140, 140];

/// Área da página na tela (a vista): x 23..615, y 108..456.
pub const PAGE: Rect = (23, 108, 593, 349);
/// Canto da primeira folha na tela, sem rolagem (Visão normal).
pub const ORIGIN: (i32, i32) = (21, 107);
/// Espaço entre duas folhas empilhadas na vista: nenhum. Cada folha tem a
/// sua moldura preta de 1 px e a de baixo encosta na de cima (medido no
/// Visor do original, `view.rs`).
pub const PAGE_GAP: i32 = 0;
/// Fundo da vista "todas as páginas" da barra de baixo (`office.md` §4).
const OVERVIEW: [u8; 3] = [208, 22, 222];
/// Período do piscar do cursor (meio período aceso).
pub const CARET_PERIOD: f32 = 1.0;
/// Folga (pontos) mantida em volta do cursor ao rolar até ele.
const FOLLOW_MARGIN: i32 = 16;

/// Ponto da tela sobre uma folha: folha e coordenadas nela (pontos).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagePoint {
    pub page: usize,
    pub x: i32,
    pub y: i32,
}

thread_local! {
    /// As fontes custam para carregar (procura no sistema): uma por
    /// conjunto de fontes da Pablo Graphos.
    static FONTS: RefCell<Vec<(usize, Rc<WriterFonts>)>> = const { RefCell::new(Vec::new()) };
}

/// As fontes do Writer, com as da Pablo Graphos dos dados originais.
pub fn writer_fonts(assets: &Assets) -> Rc<WriterFonts> {
    let pablo: Vec<_> = [6, 8, 16].iter().filter_map(|&p| assets.font(p)).collect();
    let key = pablo.iter().map(|f| Rc::as_ptr(f) as usize).sum::<usize>();
    FONTS.with(|c| {
        let mut c = c.borrow_mut();
        if let Some((_, f)) = c.iter().find(|(k, _)| *k == key) {
            return f.clone();
        }
        // nos testes, só as embutidas: o mesmo resultado em qualquer máquina
        let f = Rc::new(if cfg!(test) {
            WriterFonts::embedded(pablo)
        } else {
            WriterFonts::new(pablo)
        });
        c.push((key, f.clone()));
        f
    })
}

/// Cópia de um chunk lido (para guardar no documento).
pub(super) fn to_owned(c: &chnk::Chunk) -> OwnedChunk {
    OwnedChunk {
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
    }
}

/// Imagem `w`×`h`. Ampliar repete pixels; encolher junta cada bloco pelo
/// mínimo de cada canal, como o `StretchDIBits` do GDI no modo
/// `BLACKONWHITE` (o traço preto não some ao encolher um adesivo).
fn scaled(img: &Image, w: u32, h: u32) -> Image {
    if (img.width, img.height) == (w, h) || w == 0 || h == 0 || img.width == 0 {
        return img.clone();
    }
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let (sy0, sy1) = (
            y * img.height / h,
            ((y + 1) * img.height / h).max(y * img.height / h + 1),
        );
        for x in 0..w {
            let (sx0, sx1) = (
                x * img.width / w,
                ((x + 1) * img.width / w).max(x * img.width / w + 1),
            );
            let mut px = [255u8, 255, 255, 0];
            for sy in sy0..sy1.min(img.height) {
                for sx in sx0..sx1.min(img.width) {
                    let i = ((sy * img.width + sx) * 4) as usize;
                    let p = &img.rgba[i..i + 4];
                    if p[3] == 0 {
                        continue;
                    }
                    for c in 0..3 {
                        px[c] = px[c].min(p[c]);
                    }
                    px[3] = 255;
                }
            }
            rgba.extend_from_slice(&px);
        }
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
}

/// Desenho de um `PICT` do documento (o metarquivo, com o bitmap dentro).
fn decode_pict(c: &OwnedChunk) -> Option<Image> {
    let meta = c.parts.iter().find(|p| p.tag.0 == *b"META")?;
    wmf::meta_dib(&meta.data).ok()
}

/// Folga de cada lado de uma figura do Maçã na linha (avanço = 4 + largura
/// + 4, `office-complemento.md` §1).
pub const REBUS_PAD: i32 = 4;

/// A medição da diagramação: as fontes do Writer e o tamanho de verdade das
/// figuras do Maçã (de pé na base da linha).
struct DocMeasure<'a> {
    fonts: &'a WriterFonts,
    rebus: &'a HashMap<u8, (i32, i32)>,
    /// Agente Secreto: mede as letras trocadas que a tela mostra.
    scrambled: bool,
}

impl Measure for DocMeasure<'_> {
    fn advance(&self, font: &FontKey, c: char) -> i32 {
        Measure::advance(self.fonts, font, shown(c, self.scrambled))
    }

    fn metrics(&self, font: &FontKey) -> (i32, i32) {
        Measure::metrics(self.fonts, font)
    }

    fn cap_height(&self, font: &FontKey) -> i32 {
        Measure::cap_height(self.fonts, font)
    }

    fn rebus_size(&self, id: u8, font: &FontKey) -> (i32, i32) {
        match self.rebus.get(&id) {
            Some(&(w, h)) => (w + 2 * REBUS_PAD, h),
            None => Measure::rebus_size(self.fonts, id, font),
        }
    }
}

/// Tamanho de cada figura do Maçã (as que existem no `WRITER.PIC`).
fn rebus_sizes(a: &Assets) -> HashMap<u8, (i32, i32)> {
    (1..=255u8)
        .filter_map(|c| {
            let (id, _) = crate::writer::ideas::rebus::image_ids(c)?;
            let r = crate::assets::ImageRef::Pic(crate::assets::Program::Writer, id);
            a.has_image(&r).then(|| {
                let img = a.image(&r);
                (c, (img.width as i32, img.height as i32))
            })
        })
        .collect()
}

/// O caractere como a tela o mostra (embaralhado pelo Agente Secreto).
fn shown(c: char, scrambled: bool) -> char {
    if scrambled {
        cipher::scramble_char(c)
    } else {
        c
    }
}

fn is_white(p: &[u8]) -> bool {
    p[3] == 0 || (p[0] > 250 && p[1] > 250 && p[2] > 250)
}

/// O que define o fundo e a borda desenhados: as referências e a folha.
type ArtKey = (Option<ObjRef>, Option<ObjRef>, [u16; 6]);

/// Pixels não brancos do fundo e da borda de uma folha de `width` pontos.
struct ArtInk {
    width: u32,
    ink: Vec<(u32, [u8; 3])>,
}

/// Desenhos dos objetos já decodificados, por conteúdo e tamanho.
type PictureCache = HashMap<(ObjRef, u16, u16), Option<Rc<Image>>>;

pub struct Editor {
    /// Pasta do usuário (os `.MAX`).
    dir: Option<PathBuf>,
    /// Arquivo do documento aberto (`None` = ainda não gravado).
    file: Option<PathBuf>,
    doc: Document,
    /// O resto do `.MAX` (objetos, sons em palavras, título…).
    pub extra: MaxExtra,
    pub sel: Selection,
    /// O Ovo: estados antes de cada ação (até 20).
    pub undo: UndoStack<Snapshot>,
    /// Área de transferência interna (Ctrl+X/C/V, Re-movedores).
    pub clipboard: Clipboard,
    /// Estilo escolhido nas listas sem nada selecionado: vale para o que for
    /// digitado em seguida no ponto de inserção.
    pending: Option<CharProps>,
    fonts: Rc<WriterFonts>,
    /// Tamanho das figuras do Maçã.
    rebus: HashMap<u8, (i32, i32)>,
    layout: RefCell<Option<Rc<Layout>>>,
    pictures: RefCell<PictureCache>,
    /// Rolagem da vista, em pixels da tela (pontos na Visão normal).
    pub scroll: (i32, i32),
    /// Zoom e arrumação das folhas (Visor de páginas); mude com
    /// [`Editor::set_view`].
    pub view: View,
    /// Folhas reduzidas ou ampliadas para o zoom atual, por (folha, tamanho).
    scaled: RefCell<HashMap<(usize, i32, i32), Rc<Frame>>>,
    /// Fundo e borda da folha desenhados (pixels não brancos), pela chave
    /// do que os define.
    art: RefCell<Option<(ArtKey, Rc<ArtInk>)>>,
    /// Rolos de adesivos das bibliotecas do Writer.
    pub rolls: Vec<ClipRoll>,
    libs: Vec<(&'static str, Rc<Vec<u8>>)>,
    /// Relógio do piscar do ponto de inserção.
    pub blink: f32,
    /// Ponto de inserção visível (as listas abertas o escondem).
    pub caret_visible: bool,
    /// Relógio dos efeitos especiais animados, em ms.
    pub effects_ms: u64,
    changed: bool,
    /// Arrastando o mouse para selecionar.
    dragging: bool,
}

impl Editor {
    /// Abre o documento gravado por último na pasta do usuário (ou um em
    /// branco).
    pub fn new(dir: Option<PathBuf>, assets: &Assets) -> Editor {
        let mut ed = Editor::blank(dir, assets);
        if let Some(last) = ed.dir.as_deref().and_then(store::latest) {
            // um arquivo que não abre fica de fora: começa em branco
            let _ = ed.open(&last);
        }
        ed
    }

    /// Documento em branco ("Documento Novo").
    pub fn blank(dir: Option<PathBuf>, assets: &Assets) -> Editor {
        let rolls = load_rolls(assets)
            .into_iter()
            .filter(|r| LIBRARIES.contains(&r.library))
            .collect();
        let libs = LIBRARIES
            .iter()
            .filter_map(|&n| Some((n, Rc::new(assets.library_bytes(n)?.to_vec()))))
            .collect();
        let (mut doc, extra) = convert::blank();
        NewStyle::load(dir.as_deref()).apply_to(&mut doc);
        Editor {
            dir,
            file: None,
            doc,
            extra,
            sel: Selection::default(),
            undo: UndoStack::new(),
            clipboard: Clipboard::default(),
            pending: None,
            fonts: writer_fonts(assets),
            rebus: rebus_sizes(assets),
            layout: RefCell::new(None),
            pictures: RefCell::new(HashMap::new()),
            scroll: (0, 0),
            view: View::default(),
            scaled: RefCell::new(HashMap::new()),
            art: RefCell::new(None),
            rolls,
            libs,
            blink: 0.0,
            caret_visible: true,
            effects_ms: 0,
            changed: false,
            dragging: false,
        }
    }

    // ---------------------------------------------------------------
    // Leitura

    pub fn doc(&self) -> &Document {
        &self.doc
    }

    /// O documento para mudar (descarta a diagramação guardada e marca o
    /// documento como mudado). Chame [`Editor::record`] antes, para o Ovo.
    pub fn doc_mut(&mut self) -> &mut Document {
        self.invalidate();
        self.changed = true;
        &mut self.doc
    }

    pub fn fonts(&self) -> &WriterFonts {
        &self.fonts
    }

    /// A diagramação do documento (guardada até a próxima mudança).
    pub fn layout(&self) -> Rc<Layout> {
        if let Some(l) = self.layout.borrow().as_ref() {
            return l.clone();
        }
        let m = DocMeasure {
            fonts: &self.fonts,
            rebus: &self.rebus,
            scrambled: self.encrypted(),
        };
        let l = Rc::new(Layout::build(&self.doc, &m));
        *self.layout.borrow_mut() = Some(l.clone());
        l
    }

    fn invalidate(&mut self) {
        *self.layout.get_mut() = None;
        self.scaled.get_mut().clear();
    }

    /// Documento embaralhado pelo Agente Secreto (`ADD[5]`): a tela troca
    /// as letras e cobre os objetos.
    pub fn encrypted(&self) -> bool {
        self.extra.add.encrypted
    }

    /// OK no painel SENHA do Agente Secreto (ver [`cipher::ok`]): embaralha
    /// ou desembaralha a exibição; o texto não muda. Não entra no Ovo.
    pub fn secret_ok(&mut self, typed: &str, master_keys: bool) -> cipher::Outcome {
        let r = cipher::ok(&mut self.extra.add, typed, master_keys);
        if matches!(r, cipher::Outcome::Scrambled | cipher::Outcome::Unscrambled) {
            self.invalidate();
            self.changed = true;
        }
        r
    }

    /// O documento mudou desde que foi aberto ou gravado.
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Título (nome dado ao salvar); `None` = "Documento Novo".
    pub fn title(&self) -> Option<&str> {
        let t = self.extra.add.title.as_str();
        (!t.is_empty()).then_some(t)
    }

    /// O arquivo `.MAX` do documento, se já foi gravado ou aberto.
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// Pasta do usuário.
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// Propriedades do que for digitado agora.
    pub fn typing_props(&self) -> CharProps {
        self.pending
            .unwrap_or_else(|| self.sel.typing_props(&self.doc))
    }

    /// Cor `rgb` como `[u8; 3]`.
    pub fn rgb(c: formats::Rgb) -> [u8; 3] {
        [c.r, c.g, c.b]
    }

    // ---------------------------------------------------------------
    // Vista

    /// Onde as folhas caem na tela na vista atual (sem a rolagem).
    pub fn sheets(&self) -> Sheets {
        self.view.sheets(
            i32::from(self.doc.dop.page_width),
            i32::from(self.doc.dop.page_height),
            self.layout().page_count(),
        )
    }

    /// Pixels por ponto na vista atual (1 na Visão normal).
    pub fn scale(&self) -> f64 {
        self.sheets().scale
    }

    fn origin_in(s: &Sheets, page: usize) -> (i32, i32) {
        match s.origins.get(page) {
            Some(&o) => o,
            None => {
                let n = s.origins.len().max(1);
                let (x, y) = s.origins.last().copied().unwrap_or(ORIGIN);
                (x, y + (page + 1 - n) as i32 * (s.h + PAGE_GAP))
            }
        }
    }

    /// Canto da folha `page` na tela.
    pub fn page_origin(&self, page: usize) -> (i32, i32) {
        let (x, y) = Self::origin_in(&self.sheets(), page);
        (x - self.scroll.0, y - self.scroll.1)
    }

    /// Ponto da folha → tela.
    pub fn to_screen(&self, page: usize, x: i32, y: i32) -> (i32, i32) {
        let s = self.sheets();
        let (ox, oy) = Self::origin_in(&s, page);
        let k = |v: i32| (f64::from(v) * s.scale).floor() as i32;
        (ox - self.scroll.0 + k(x), oy - self.scroll.1 + k(y))
    }

    /// Tela → ponto da folha (`None` fora da vista ou entre folhas).
    pub fn to_page(&self, x: i32, y: i32) -> Option<PagePoint> {
        if !inside(PAGE, x, y) {
            return None;
        }
        let s = self.sheets();
        let (w, h) = (
            i32::from(self.doc.dop.page_width),
            i32::from(self.doc.dop.page_height),
        );
        let (sx, sy) = (x + self.scroll.0, y + self.scroll.1);
        s.origins.iter().enumerate().find_map(|(page, &(ox, oy))| {
            let inside_sheet = (ox..ox + s.w).contains(&sx) && (oy..oy + s.h).contains(&sy);
            let p = |v: i32| (f64::from(v) / s.scale).floor() as i32;
            inside_sheet.then(|| PagePoint {
                page,
                x: p(sx - ox).clamp(0, w - 1),
                y: p(sy - oy).clamp(0, h - 1),
            })
        })
    }

    /// Folha no meio da vista ("Pág. N" da barra de baixo): a que contém o
    /// meio da vista ou, entre folhas, a mais perto dele.
    pub fn page(&self) -> usize {
        let s = self.sheets();
        let (cx, cy) = (
            PAGE.0 + PAGE.2 / 2 + self.scroll.0,
            PAGE.1 + PAGE.3 / 2 + self.scroll.1,
        );
        let dist = |&(x, y): &(i32, i32)| {
            let dx = (x - cx).max(cx - (x + s.w - 1)).max(0);
            let dy = (y - cy).max(cy - (y + s.h - 1)).max(0);
            (dy, dx)
        };
        (0..s.origins.len())
            .min_by_key(|&k| dist(&s.origins[k]))
            .unwrap_or(0)
    }

    /// Rola até mostrar o alto da folha `page`.
    pub fn show_page(&mut self, page: usize) {
        let s = self.sheets();
        let n = s.origins.len().max(1);
        let (_, y) = Self::origin_in(&s, page.min(n - 1));
        self.scroll.1 = y - ORIGIN.1;
        self.scroll_by(0, 0);
    }

    /// Rola a vista (setas da barra de baixo, Visor); limitada às folhas.
    pub fn scroll_by(&mut self, dx: i32, dy: i32) {
        let (max_x, max_y) = self.sheets().max_scroll();
        self.scroll.0 = (self.scroll.0 + dx).clamp(0, max_x);
        self.scroll.1 = (self.scroll.1 + dy).clamp(0, max_y);
    }

    /// Troca a vista (Visor de páginas), mantendo a folha do meio no alto.
    pub fn set_view(&mut self, v: View) {
        if v == self.view {
            return;
        }
        let page = self.page();
        self.view = v;
        self.scroll = (0, 0);
        self.scaled.get_mut().clear();
        self.show_page(page);
    }

    /// Rola o mínimo para o cursor aparecer.
    pub fn follow_caret(&mut self) {
        let l = self.layout();
        let c = l.caret(self.sel.caret);
        let (_, sy) = self.to_screen(c.page, c.x, c.top);
        let height = (f64::from(c.height) * self.scale()).ceil() as i32;
        let top = PAGE.1 + FOLLOW_MARGIN.min(PAGE.3 / 4);
        let bottom = PAGE.1 + PAGE.3 - FOLLOW_MARGIN.min(PAGE.3 / 4);
        if sy < top {
            self.scroll.1 -= top - sy;
        } else if sy + height > bottom {
            self.scroll.1 += sy + height - bottom;
        }
        let (sx, _) = self.to_screen(c.page, c.x, c.top);
        if sx < PAGE.0 {
            self.scroll.0 -= PAGE.0 - sx + FOLLOW_MARGIN;
        } else if sx >= PAGE.0 + PAGE.2 {
            self.scroll.0 += sx - (PAGE.0 + PAGE.2) + FOLLOW_MARGIN;
        }
        self.scroll_by(0, 0);
    }

    // ---------------------------------------------------------------
    // Edição

    /// Guarda o estado para o Ovo antes de uma ação.
    pub fn record(&mut self) {
        self.undo.record(Snapshot {
            doc: self.doc.clone(),
            sel: self.sel,
        });
    }

    /// Depois de uma ação: acende o cursor e rola até ele.
    pub fn after_edit(&mut self) {
        self.invalidate();
        self.changed = true;
        self.blink = 0.0;
        self.sel.clamp(&self.doc);
        self.follow_caret();
    }

    /// Uma ação desfazível qualquer sobre o documento e a seleção.
    pub fn edit<R>(&mut self, f: impl FnOnce(&mut Document, &mut Selection) -> R) -> R {
        self.record();
        self.pending = None;
        let r = f(&mut self.doc, &mut self.sel);
        self.after_edit();
        r
    }

    /// Texto digitado no lugar da seleção.
    pub fn type_text(&mut self, text: &str) {
        let clean: String = text
            .chars()
            .filter(|c| !c.is_control() && formats::cp1252::encode_char(*c).is_some())
            .collect();
        if clean.is_empty() {
            return;
        }
        let props = self.typing_props();
        let (doc, sel) = (&self.doc, self.sel);
        self.undo.record_typing(|| Snapshot {
            doc: doc.clone(),
            sel,
        });
        self.sel.type_text(&mut self.doc, &clean, props);
        self.pending = None;
        self.after_edit();
    }

    fn record_typing(&mut self) {
        let (doc, sel) = (&self.doc, self.sel);
        self.undo.record_typing(|| Snapshot {
            doc: doc.clone(),
            sel,
        });
    }

    /// Teclas de edição e movimento (sem Ctrl: setas, Home/End; com Ctrl:
    /// palavras e começo/fim do documento). `shift` estende a seleção.
    pub fn key(&mut self, key: Key, ctrl: bool, shift: bool) -> bool {
        self.blink = 0.0;
        let motion = match (key, ctrl) {
            (Key::Left, false) => Some(Motion::Left),
            (Key::Right, false) => Some(Motion::Right),
            (Key::Left, true) => Some(Motion::WordLeft),
            (Key::Right, true) => Some(Motion::WordRight),
            (Key::Up, _) => Some(Motion::Up),
            (Key::Down, _) => Some(Motion::Down),
            (Key::Home, false) => Some(Motion::LineStart),
            (Key::End, false) => Some(Motion::LineEnd),
            (Key::Home, true) => Some(Motion::DocStart),
            (Key::End, true) => Some(Motion::DocEnd),
            _ => None,
        };
        if let Some(m) = motion {
            self.undo.close();
            self.pending = None;
            let l = self.layout();
            self.sel.apply(&self.doc, &l, m, shift);
            self.follow_caret();
            return true;
        }
        match key {
            Key::Enter if !ctrl => {
                let props = self.typing_props();
                self.edit(|d, s| {
                    s.newline(d);
                    // o parágrafo novo começa com o estilo de digitação
                    d.paras[s.caret.para].mark = props;
                });
                self.pending = Some(props);
            }
            Key::Backspace if !ctrl => {
                self.record_typing();
                if self.sel.backspace(&mut self.doc) {
                    self.after_edit();
                } else {
                    self.undo.close();
                }
            }
            Key::Delete if !ctrl => {
                self.record_typing();
                if self.sel.delete_forward(&mut self.doc) {
                    self.after_edit();
                }
            }
            Key::PageUp => self.scroll_by(0, -(PAGE.3 - FOLLOW_MARGIN)),
            Key::PageDown => self.scroll_by(0, PAGE.3 - FOLLOW_MARGIN),
            _ => return false,
        }
        true
    }

    /// Muda os caracteres da seleção; sem seleção, o estilo de digitação
    /// (as listas da Palavra até o pincel da W5).
    pub fn apply_char(&mut self, ch: CharChange) {
        if self.sel.is_empty() {
            let mut p = self.typing_props();
            ch.apply(&mut p);
            self.undo.close();
            self.pending = Some(p);
            return;
        }
        let r = self.sel.range();
        self.edit(|d, _| d.apply_char(r, ch));
    }

    /// Muda os parágrafos tocados pela seleção (ou o do cursor).
    pub fn apply_para(&mut self, ch: ParaChange) {
        let r = self.sel.range();
        self.edit(|d, _| d.apply_para(r, ch));
    }

    /// Muda a folha (Preparação da página).
    pub fn apply_dop(&mut self, f: impl FnOnce(&mut DopProps)) {
        self.edit(|d, _| f(&mut d.dop));
    }

    /// Põe um objeto por cima dos outros (desfazível); `chunk` é a cópia do
    /// conteúdo (`PICT`, `SPLE`…), guardada no documento com um número
    /// livre que substitui o de `o.object`. Devolve o índice.
    pub fn add_object(&mut self, mut o: FloatObject, chunk: Option<OwnedChunk>) -> usize {
        if let Some(c) = chunk {
            o.object = self.extra.add_object(c);
        }
        self.extra.frame_extra.push((o.object, Vec::new()));
        self.edit(|d, _| d.add_object(o))
    }

    /// Troca o objeto `k` (desfazível); `chunk` é o conteúdo novo, guardado
    /// com um número livre (o cavalete de palavras com forma reaberto).
    pub fn replace_object(&mut self, k: usize, mut o: FloatObject, chunk: Option<OwnedChunk>) {
        if k >= self.doc.objects.len() {
            return;
        }
        if let Some(c) = chunk {
            o.object = self.extra.add_object(c);
        }
        if let Some(e) = self.extra.frame_extra.get_mut(k) {
            e.0 = o.object;
        }
        self.edit(|d, _| d.objects[k] = o);
    }

    /// Tira o objeto `k` (desfazível).
    pub fn remove_object(&mut self, k: usize) -> Option<FloatObject> {
        if k >= self.doc.objects.len() {
            return None;
        }
        if k < self.extra.frame_extra.len() {
            self.extra.frame_extra.remove(k);
        }
        self.edit(|d, _| d.remove_object(k))
    }

    /// Objeto sob o ponto da tela (o de cima).
    pub fn object_at(&self, x: i32, y: i32) -> Option<usize> {
        let p = self.to_page(x, y)?;
        self.doc.object_at(p.page, p.x, p.y)
    }

    /// Cópia do `PICT` do adesivo `index` do rolo `roll` (número do `CARO`),
    /// como vem da biblioteca.
    pub fn sticker_chunk(&self, roll: u16, index: usize) -> Option<OwnedChunk> {
        let r = self.rolls.iter().find(|r| r.id == roll)?;
        let (_, bytes) = self.libs.iter().find(|(n, _)| *n == r.library)?;
        let f = chnk::parse(bytes).ok()?;
        let caro = f
            .chunks
            .iter()
            .find(|c| c.tag.0 == *b"CARO" && c.id == roll)?;
        let list = caro.parts.iter().find(|p| p.tag.0 == *b"CARO")?;
        let ids: Vec<u16> = parse_list(list.data)
            .into_iter()
            .filter(|id| f.chunks.iter().any(|c| c.tag.0 == *b"PICT" && c.id == *id))
            .collect();
        let id = *ids.get(index)?;
        let c = f
            .chunks
            .iter()
            .find(|c| c.tag.0 == *b"PICT" && c.id == id)?;
        Some(to_owned(c))
    }

    /// Cola o adesivo `index` do rolo `roll` como objeto solto com o canto
    /// em `(x, y)` da folha `page`, no tamanho natural; o contorno (`TWPB`)
    /// é calculado do desenho, como o original faz. Sem os dados, um
    /// quadrado reserva de 32 pontos.
    pub fn add_sticker(&mut self, roll: u16, index: usize, page: usize, x: i32, y: i32) -> usize {
        let chunk = self.sticker_chunk(roll, index);
        let img = chunk.as_ref().and_then(decode_pict);
        let (w, h) = img.as_ref().map_or((32, 32), |i| (i.width, i.height));
        let mut chunk =
            chunk.unwrap_or_else(|| OwnedChunk::single(b"PICT", 0, b"META", Vec::new()));
        let mut o = FloatObject::new(
            ObjectKind::Sticker,
            ObjRef::pict(0),
            page as u16,
            x.clamp(-(w as i32) + 8, i32::from(i16::MAX)) as i16,
            y.clamp(-(h as i32) + 8, i32::from(i16::MAX)) as i16,
            w as u16,
            h as u16,
        );
        if let Some(img) = &img {
            let mask: Vec<bool> = img.rgba.chunks_exact(4).map(|p| !is_white(p)).collect();
            let t = twpb::from_mask(&mask, img.width, img.height);
            o.outline = Some(convert::outline_of(&t));
            chunk.parts.retain(|p| p.tag.0 != *b"TWPB");
            chunk.parts.push(OwnedPart {
                tag: formats::Tag(*b"TWPB"),
                data: twpb::write(&t),
            });
        }
        self.add_object(o, Some(chunk))
    }

    /// Põe uma imagem própria (figura do Caderno ou do Quadro, desenho do
    /// cavalete) como objeto solto com o canto em `(x, y)` da folha `page`,
    /// no tamanho natural: o documento ganha um `PICT` com o metarquivo
    /// (`META` + `TWPB`, sem a prévia `PKBM`, como os desenhos do original).
    pub fn add_picture(&mut self, img: &Image, title: &str, page: usize, x: i32, y: i32) -> usize {
        let (chunk, outline) = picture_chunk(img, title, 0);
        let (w, h) = (img.width.clamp(1, 0x7fff), img.height.clamp(1, 0x7fff));
        let mut o = FloatObject::new(
            ObjectKind::Drawing,
            ObjRef::pict(0),
            page as u16,
            x as i16,
            y as i16,
            w as u16,
            h as u16,
        );
        o.outline = Some(outline);
        self.add_object(o, Some(chunk))
    }

    /// Troca a figura do objeto `k` pelo desenho do cavalete (Mover-modificar):
    /// um `PICT` novo no tamanho do desenho, no mesmo lugar, com o mesmo
    /// modo de flutuação, som e borda; o objeto passa a ser um desenho.
    pub fn replace_picture(&mut self, k: usize, img: &Image) -> bool {
        if k >= self.doc.objects.len() {
            return false;
        }
        let (chunk, outline) = picture_chunk(img, "", 0);
        let r = self.extra.add_object(chunk);
        let (w, h) = (
            img.width.clamp(1, 0x7fff) as u16,
            img.height.clamp(1, 0x7fff) as u16,
        );
        if let Some((o, _)) = self.extra.frame_extra.get_mut(k) {
            *o = r;
        }
        self.edit(|d, _| {
            let o = &mut d.objects[k];
            o.kind = ObjectKind::Drawing;
            o.object = r;
            (o.width, o.height, o.orig_width, o.orig_height) = (w, h, w, h);
            o.outline = Some(outline);
        });
        true
    }

    /// Cola o adesivo no ponto de inserção (a faixa dos Adesivos até a W10).
    pub fn insert_sticker(&mut self, roll: u16, index: usize) -> usize {
        let c = self.layout().caret(self.sel.caret);
        self.add_sticker(roll, index, c.page, c.x, c.top)
    }

    /// Desenho de um objeto no tamanho atual (em cache); `None` sem desenho.
    pub fn picture(&self, o: &FloatObject) -> Option<Rc<Image>> {
        let key = (o.object, o.width, o.height);
        if let Some(p) = self.pictures.borrow().get(&key) {
            return p.clone();
        }
        let (w, h) = (u32::from(o.width), u32::from(o.height));
        let img = self.extra.object(o.object).and_then(|c| match &c.tag.0 {
            // palavra com forma: os glifos desenhados já no tamanho da moldura
            b"SPLE" => c
                .parts
                .iter()
                .find(|p| p.tag.0 == *b"SPLE")
                .and_then(|p| formats::sple::read(&p.data).ok())
                .map(|s| Rc::new(crate::writer::splot::draw(&s, w, h))),
            _ => decode_pict(c).map(|i| Rc::new(scaled(&i, w, h))),
        });
        self.pictures.borrow_mut().insert(key, img.clone());
        img
    }

    /// Apaga tudo: texto e objetos (Jogar tudo fora; desfazível).
    pub fn clear_all(&mut self) -> bool {
        if self.doc.is_empty() && self.doc.objects.is_empty() {
            return false;
        }
        self.extra.frame_extra.clear();
        self.edit(|d, s| {
            d.clear();
            d.objects.clear();
            *s = Selection::default();
        });
        true
    }

    /// Recorta a seleção para a área de transferência.
    pub fn cut(&mut self) -> bool {
        if self.sel.is_empty() {
            return false;
        }
        self.record();
        let ok = self.clipboard.cut(&mut self.doc, &mut self.sel);
        self.after_edit();
        ok
    }

    /// Copia a seleção.
    pub fn copy(&mut self) -> bool {
        self.clipboard.copy(&self.doc, &self.sel)
    }

    /// Cola o texto da área de transferência no lugar da seleção.
    pub fn paste(&mut self) -> bool {
        if !matches!(self.clipboard.clip, Some(Clip::Text(_))) {
            return false;
        }
        self.record();
        let ok = self.clipboard.paste_text(&mut self.doc, &mut self.sel);
        self.after_edit();
        ok
    }

    fn restore(&mut self, s: Snapshot) {
        self.doc = s.doc;
        self.sel = s.sel;
        self.sel.clamp(&self.doc);
        self.pending = None;
        // as molduras desconhecidas não mudam com o Ovo; as listas acompanham
        self.extra
            .frame_extra
            .resize(self.doc.objects.len(), (ObjRef::pict(u16::MAX), Vec::new()));
        self.after_edit();
    }

    fn snapshot_step(
        &mut self,
        f: impl FnOnce(&mut UndoStack<Snapshot>, &mut Snapshot) -> bool,
    ) -> bool {
        let mut cur = Snapshot {
            doc: std::mem::take(&mut self.doc),
            sel: self.sel,
        };
        let ok = f(&mut self.undo, &mut cur);
        self.restore(cur);
        ok
    }

    /// DESFAZER.
    pub fn undo_step(&mut self) -> bool {
        self.snapshot_step(|u, c| u.undo(c))
    }

    /// REFAZER.
    pub fn redo_step(&mut self) -> bool {
        self.snapshot_step(|u, c| u.redo(c))
    }

    /// Ctrl+Z: alterna desfazer e refazer.
    pub fn toggle_undo(&mut self) -> bool {
        self.snapshot_step(|u, c| u.toggle(c))
    }

    // ---------------------------------------------------------------
    // Mouse na página

    /// Botão apertado na página: põe o ponto de inserção (com `shift`,
    /// estende a seleção) e começa o arrasto. `false` fora das folhas.
    pub fn press_page(&mut self, x: i32, y: i32, shift: bool) -> bool {
        let Some(p) = self.to_page(x, y) else {
            return inside(PAGE, x, y);
        };
        let pos = self.layout().pos_at(p.page, p.x, p.y);
        self.undo.close();
        self.pending = None;
        if shift {
            self.sel.drag_to(pos);
        } else {
            self.sel.begin_drag(pos);
        }
        self.dragging = true;
        self.blink = 0.0;
        true
    }

    /// Mouse movido com o botão apertado: estende a seleção.
    pub fn drag_page(&mut self, x: i32, y: i32) {
        if !self.dragging {
            return;
        }
        let (cx, cy) = (
            x.clamp(PAGE.0, PAGE.0 + PAGE.2 - 1),
            y.clamp(PAGE.1, PAGE.1 + PAGE.3 - 1),
        );
        // arrastar além da vista rola
        if y < PAGE.1 {
            self.scroll_by(0, y - PAGE.1);
        } else if y >= PAGE.1 + PAGE.3 {
            self.scroll_by(0, y - (PAGE.1 + PAGE.3) + 1);
        }
        if let Some(p) = self.to_page(cx, cy) {
            let pos = self.layout().pos_at(p.page, p.x, p.y);
            self.sel.drag_to(pos);
        }
    }

    /// Botão solto.
    pub fn release_page(&mut self) {
        self.dragging = false;
    }

    pub fn dragging(&self) -> bool {
        self.dragging
    }

    pub fn tick(&mut self, dt: f32) {
        if dt.is_finite() {
            self.blink = (self.blink + dt.max(0.0)) % CARET_PERIOD;
            self.effects_ms = self
                .effects_ms
                .wrapping_add((dt.max(0.0) * 1000.0).round() as u64);
            self.undo.tick(dt);
        }
    }

    // ---------------------------------------------------------------
    // Desenho

    /// As folhas visíveis, com a moldura, os objetos, o texto, a seleção
    /// (vídeo inverso), o número de página e o ponto de inserção. Fora da
    /// Visão normal em 100 % as folhas são desenhadas em tamanho real e
    /// reduzidas (ou ampliadas) para o zoom da vista.
    pub fn draw_page(&self, f: &mut Frame, a: &Assets) {
        let (vx, vy, vw, vh) = PAGE;
        let mut v = Frame::new(vw as u32, vh as u32);
        v.fill(if self.view.overview {
            OVERVIEW
        } else {
            BACKDROP
        });
        let l = self.layout();
        let s = self.sheets();
        let normal = self.view.is_normal();
        for page in 0..l.page_count() {
            let (ox, oy) = self.page_origin(page);
            let (ox, oy) = (ox - vx, oy - vy);
            if oy >= vh || oy + s.h <= 0 || ox >= vw || ox + s.w <= 0 {
                continue;
            }
            if normal {
                // a folha do port fica 1 px à esquerda da do original (x 21
                // em vez de 22): as molduras dos lados ficam de fora, só as
                // linhas de cima e de baixo separam as folhas
                self.draw_sheet(&mut v, a, page, ox, oy, true);
                super::effects_draw::draw(
                    &self.doc,
                    &l,
                    &mut v,
                    page,
                    ox,
                    oy,
                    (vx, vy),
                    self.effects_ms,
                );
                v.rect(ox, oy, s.w, 1, BLACK);
                v.rect(ox, oy + s.h - 1, s.w, 1, BLACK);
            } else {
                // as vistas reduzidas mostram a folha parada (sem os
                // efeitos animados)
                copy_into(&mut v, &self.scaled_sheet(a, page, s.w, s.h), ox, oy);
                v.frame_rect(ox, oy, s.w, s.h, 1, BLACK);
            }
        }
        if !normal && !self.view.overview {
            for r in l.selection_rects(self.sel.range()) {
                let (x0, y0, x1, y1) = r.rect;
                let (sx0, sy0) = self.to_screen(r.page, x0, y0);
                let (sx1, sy1) = self.to_screen(r.page, x1, y1);
                invert(&mut v, sx0 - vx, sy0 - vy, sx1 - sx0, sy1 - sy0);
            }
        }
        if self.caret_visible
            && !self.view.overview
            && self.sel.is_empty()
            && self.blink % CARET_PERIOD < CARET_PERIOD / 2.0
        {
            let c = l.caret(self.sel.caret);
            let (x, y) = self.to_screen(c.page, c.x, c.top);
            let h = (f64::from(c.height) * s.scale).round() as i32;
            v.rect(x - vx, y - vy, 1, h.max(1), BLACK);
        }
        copy_into(f, &v, vx, vy);
    }

    /// A folha `page` no tamanho `w × h` da vista (guardada até a próxima
    /// mudança do documento).
    fn scaled_sheet(&self, a: &Assets, page: usize, w: i32, h: i32) -> Rc<Frame> {
        if let Some(f) = self.scaled.borrow().get(&(page, w, h)) {
            return f.clone();
        }
        let f = Rc::new(resample(&self.render_page_with_art(a, page), w, h));
        self.scaled.borrow_mut().insert((page, w, h), f.clone());
        f
    }

    /// A folha `page` inteira, em tamanho real (1 ponto = 1 pixel), com o
    /// canto em `(ox, oy)` do quadro `v`: fundo branco, objetos, texto,
    /// número de página e (com `selection`) a seleção em vídeo inverso.
    /// O Visor de páginas e a impressão desenham por aqui.
    pub fn draw_sheet(
        &self,
        v: &mut Frame,
        a: &Assets,
        page: usize,
        ox: i32,
        oy: i32,
        selection: bool,
    ) {
        self.sheet(v, Some(a), page, ox, oy, selection, true);
    }

    /// [`Editor::draw_sheet`] sem os recursos (as figuras do Maçã ficam
    /// como retângulos): a miniatura gravada no arquivo.
    #[allow(clippy::too_many_arguments)]
    fn sheet(
        &self,
        v: &mut Frame,
        a: Option<&Assets>,
        page: usize,
        ox: i32,
        oy: i32,
        selection: bool,
        art: bool,
    ) {
        let l = self.layout();
        let (pw, ph) = (
            i32::from(self.doc.dop.page_width),
            i32::from(self.doc.dop.page_height),
        );
        v.rect(ox, oy, pw, ph, WHITE);
        if let Some(art) = self.art_ink().filter(|_| art) {
            for &(i, rgb) in &art.ink {
                v.put(
                    ox + (i % art.width) as i32,
                    oy + (i / art.width) as i32,
                    rgb,
                );
            }
        }
        let Some(sheet) = l.pages.get(page) else {
            return;
        };
        for &k in &sheet.objects {
            // na faixa, a palavra esticada continua nas folhas seguintes
            let o = &self.doc.objects[k];
            let shift = (page as i32 - i32::from(o.page)).max(0) * pw;
            let clip = (ox, oy, ox + pw, oy + ph);
            self.draw_object(v, o, ox - shift, oy, self.doc.dop.banner.then_some(clip));
        }
        self.draw_lines(v, a, &l, page, ox, oy);
        if selection {
            super::sounds::draw_word_boxes(&self.doc, &l, v, page, ox, oy);
        }
        if let Some(n) = &sheet.number {
            let font = RunFont::default();
            let w = self.fonts.text_width(&font, &n.text);
            let (x0, y0, x1, _) = n.rect;
            self.fonts.draw(
                v,
                ox + (x0 + x1 - w) / 2,
                oy + y0,
                &font,
                &n.text,
                BLACK,
                false,
            );
        }
        if selection {
            for r in l.selection_rects(self.sel.range()) {
                if r.page == page {
                    let (x0, y0, x1, y1) = r.rect;
                    invert(v, ox + x0, oy + y0, x1 - x0, y1 - y0);
                }
            }
        }
    }

    /// Fundo e borda da folha (Preparação da página), desenhados uma vez
    /// em tamanho real a partir das cópias guardadas no documento.
    fn art_ink(&self) -> Option<Rc<ArtInk>> {
        let d = &self.doc.dop;
        if d.border.is_none() && d.background.is_none() {
            return None;
        }
        let key: ArtKey = (
            d.border,
            d.background,
            [
                d.page_width,
                d.page_height,
                d.margin_top,
                d.margin_left,
                d.margin_bottom,
                d.margin_right,
            ],
        );
        if let Some((k, ink)) = self.art.borrow().as_ref()
            && *k == key
        {
            return Some(ink.clone());
        }
        let to_max = |r: ObjRef| max::ObjRef::new(&r.tag, r.id);
        let art =
            borders::PageArt::from_refs(d.border.map(to_max), d.background.map(to_max), &|r| {
                self.extra.object(ObjRef {
                    tag: r.tag.0,
                    id: r.id,
                })
            });
        let geom = borders::PageGeom {
            width: f64::from(d.page_width),
            height: f64::from(d.page_height),
            margin_top: f64::from(d.margin_top),
            margin_left: f64::from(d.margin_left),
            margin_bottom: f64::from(d.margin_bottom),
            margin_right: f64::from(d.margin_right),
        };
        let img = borders::render_page(&geom, &art, 1.0);
        let ink = Rc::new(ArtInk {
            width: img.width.max(1),
            ink: img
                .rgba
                .chunks_exact(4)
                .enumerate()
                .filter(|(_, p)| !is_white(p))
                .map(|(i, p)| (i as u32, [p[0], p[1], p[2]]))
                .collect(),
        });
        *self.art.borrow_mut() = Some((key, ink.clone()));
        Some(ink)
    }

    /// A folha `page` num quadro do tamanho dela (sem seleção nem cursor) e
    /// sem a borda e o fundo, que a impressão redesenha na resolução da
    /// folha (`writer::print`).
    pub fn render_page(&self, a: &Assets, page: usize) -> Frame {
        self.sheet_frame(a, page, false)
    }

    /// A folha `page` como na tela (com a borda e o fundo), em tamanho real.
    pub fn render_page_with_art(&self, a: &Assets, page: usize) -> Frame {
        self.sheet_frame(a, page, true)
    }

    fn sheet_frame(&self, a: &Assets, page: usize, art: bool) -> Frame {
        let mut f = Frame::new(
            u32::from(self.doc.dop.page_width),
            u32::from(self.doc.dop.page_height),
        );
        self.sheet(&mut f, Some(a), page, 0, 0, false, art);
        f
    }

    /// Desenha o objeto com a folha em `(ox, oy)`; com `clip` (x0, y0, x1,
    /// y1), só o que cair dentro (as folhas da faixa).
    fn draw_object(
        &self,
        v: &mut Frame,
        o: &FloatObject,
        ox: i32,
        oy: i32,
        clip: Option<(i32, i32, i32, i32)>,
    ) {
        let (x, y) = (ox + i32::from(o.x), oy + i32::from(o.y));
        if self.encrypted() {
            // Agente Secreto: o objeto vira um retângulo preto
            v.rect(x, y, i32::from(o.width), i32::from(o.height), BLACK);
            return;
        }
        let (cx0, cy0, cx1, cy1) = clip.unwrap_or((i32::MIN, i32::MIN, i32::MAX, i32::MAX));
        match self.picture(o) {
            Some(img) => {
                for (k, p) in img.rgba.chunks_exact(4).enumerate() {
                    let (px, py) = (
                        x + (k as u32 % img.width) as i32,
                        y + (k as u32 / img.width) as i32,
                    );
                    if !is_white(p) && (cx0..cx1).contains(&px) && (cy0..cy1).contains(&py) {
                        v.put(px, py, [p[0], p[1], p[2]]);
                    }
                }
            }
            None => {
                // sem desenho (figura sem metarquivo): o lugar dele
                let (w, h) = (i32::from(o.width), i32::from(o.height));
                v.frame_rect(x, y, w, h, 1, BACKDROP);
            }
        }
    }

    fn draw_lines(
        &self,
        v: &mut Frame,
        a: Option<&Assets>,
        l: &Layout,
        page: usize,
        ox: i32,
        oy: i32,
    ) {
        let vh = v.height() as i32;
        let scrambled = self.encrypted();
        for line in l.lines_on(page) {
            if oy + line.top > vh || oy + line.top + line.height < 0 {
                continue;
            }
            let para = &self.doc.paras[line.para];
            if let Some(bx) = line.bullet {
                let font = RunFont::of(&self.doc, &para.mark);
                let lm = self.fonts.line_metrics(&font);
                let t = crate::writer::layout::BULLET.to_string();
                self.fonts.draw(
                    v,
                    ox + bx,
                    oy + line.baseline - lm.ascent,
                    &font,
                    &t,
                    BLACK,
                    false,
                );
            }
            // trechos de itens seguidos com as mesmas propriedades e tamanho
            let mut k = 0;
            while k < line.boxes.len() {
                let b = line.boxes[k];
                let Some(item) = para.items.get(b.index) else {
                    k += 1;
                    continue;
                };
                match *item {
                    Item::Rebus(c) => {
                        // de pé na base, com a folga dos dois lados
                        let (w, h) = (b.w - 2 * REBUS_PAD, b.ascent);
                        let x = ox + b.x + REBUS_PAD;
                        let y = oy + b.baseline - h;
                        let drawn = if scrambled {
                            v.rect(x, y, w.max(1), h.max(1), BLACK);
                            Some(())
                        } else {
                            a.and_then(|a| crate::writer::ideas::rebus::draw(v, a, c, x, y))
                                .map(|_| ())
                        };
                        if drawn.is_none() {
                            v.frame_rect(x, y, w.max(1), h.max(1), 1, BACKDROP);
                        }
                        k += 1;
                    }
                    Item::Char(_, props) => {
                        let mut font = RunFont::of(&self.doc, &props);
                        font.points = b.size_pt;
                        // o trecho segue enquanto cada item começa onde o
                        // anterior acabou (o contorno dos objetos e o
                        // justificado abrem buracos)
                        let mut text = String::new();
                        let mut pen = b.x;
                        let mut j = k;
                        while j < line.boxes.len() {
                            let bj = line.boxes[j];
                            match para.items.get(bj.index) {
                                Some(Item::Char(c, p))
                                    if *p == props
                                        && bj.size_pt == b.size_pt
                                        && bj.baseline == b.baseline
                                        && bj.x == pen =>
                                {
                                    let c = shown(*c, scrambled);
                                    text.push(c);
                                    pen += self.fonts.advance(&font, c);
                                    j += 1;
                                }
                                _ => break,
                            }
                        }
                        if j == k {
                            // item fora do lugar sozinho: desenha só ele
                            if let Some(Item::Char(c, _)) = para.items.get(b.index) {
                                text.push(shown(*c, scrambled));
                            }
                            j = k + 1;
                        }
                        let lm = self.fonts.line_metrics(&font);
                        let last = line.boxes[j - 1];
                        if let Some(hl) = props.highlight {
                            // o marca-texto cobre a linha inteira (SAMP18 "calor")
                            v.rect(
                                ox + b.x,
                                oy + line.top,
                                last.x + last.w - b.x,
                                line.height,
                                Editor::rgb(hl),
                            );
                        }
                        let rgb = Editor::rgb(props.text_color());
                        self.fonts.draw(
                            v,
                            ox + b.x,
                            oy + b.baseline - lm.ascent,
                            &font,
                            &text,
                            rgb,
                            false,
                        );
                        k = j;
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // Arquivos

    /// Abre um `.MAX` no lugar do documento atual (o Ovo esquece tudo). Um
    /// arquivo de fora da pasta do usuário (os exemplos do Max) é aberto
    /// como cópia: salvar grava na pasta do usuário, nunca por cima dele.
    pub fn open(&mut self, path: &Path) -> Result<(), String> {
        let (doc, extra) = store::load(path)?;
        let own = self
            .dir
            .as_deref()
            .is_some_and(|d| path.parent() == Some(d));
        self.load(doc, extra, own.then(|| path.to_path_buf()));
        Ok(())
    }

    /// Abre a história começada na Oficina de Idéias (I10): o documento com
    /// a sentença ou a figura, o cursor no lugar dela e, ao salvar, `FLOR` 4
    /// e `WIZA` `plot`/`wpic`. É um documento novo (sem arquivo), já mudado.
    pub fn open_story(&mut self, story: IdeaStory) {
        let mut extra = MaxExtra::new();
        let mut doc = story.doc;
        for p in &story.pictures {
            let (chunk, outline) = picture_chunk(&p.image, &p.title, p.object.id);
            extra.objects.push(chunk);
            for o in doc.objects.iter_mut().filter(|o| o.object == p.object) {
                o.kind = ObjectKind::Drawing;
                o.outline = Some(outline.clone());
            }
        }
        extra.frame_extra = doc.objects.iter().map(|o| (o.object, Vec::new())).collect();
        extra.add.floor = story::FLOOR;
        extra.add.project = max::Project::from_bytes(story.source.wiza());
        self.load(doc, extra, None);
        self.sel = Selection::at(self.doc.clamp(story.cursor));
        self.changed = true;
        self.follow_caret();
    }

    /// Começa um documento em branco (NOVO).
    pub fn new_document(&mut self) {
        let (mut doc, extra) = convert::blank();
        NewStyle::load(self.dir.as_deref()).apply_to(&mut doc);
        self.load(doc, extra, None);
    }

    /// Começa um documento novo já montado (o projeto em branco da Oficina
    /// de Projetos, `writer::project::blank`).
    pub fn new_from(&mut self, doc: Document, extra: MaxExtra) {
        self.load(doc, extra, None);
    }

    fn load(&mut self, doc: Document, extra: MaxExtra, file: Option<PathBuf>) {
        self.doc = doc;
        self.extra = extra;
        self.file = file;
        self.undo.clear();
        self.pending = None;
        self.scroll = (0, 0);
        self.changed = false;
        self.pictures.get_mut().clear();
        *self.art.get_mut() = None;
        self.invalidate();
        self.sel = Selection::at(self.doc.start());
    }

    /// Esquece as mudanças (Jogar fora): nada será gravado ao sair.
    pub fn discard(&mut self) {
        self.changed = false;
    }

    /// Miniatura da primeira folha, como o original grava no `THMB`: o lado
    /// maior com 144 pixels, 4 bits na paleta `THUMB_PALETTE` (a pasta e o
    /// quadro da sala Escritório mostram).
    pub fn thumbnail(&self) -> pkbm::Bitmap {
        let dop = &self.doc.dop;
        let (pw, ph) = (dop.page_width, dop.page_height);
        let mut page = Frame::new(u32::from(pw), u32::from(ph));
        self.sheet(&mut page, None, 0, 0, 0, false, true);
        let (tw, th) = max::thumb_size(pw, ph);
        let mut pixels = Vec::with_capacity(usize::from(tw) * usize::from(th));
        for ty in 0..u32::from(th) {
            let (y0, y1) = (
                ty * u32::from(ph) / u32::from(th),
                (ty + 1) * u32::from(ph) / u32::from(th),
            );
            for tx in 0..u32::from(tw) {
                let (x0, x1) = (
                    tx * u32::from(pw) / u32::from(tw),
                    (tx + 1) * u32::from(pw) / u32::from(tw),
                );
                let mut sum = [0u32; 3];
                let mut n = 0;
                for y in y0..y1.max(y0 + 1) {
                    for x in x0..x1.max(x0 + 1) {
                        let p = page.pixel(x.min(u32::from(pw) - 1), y.min(u32::from(ph) - 1));
                        for c in 0..3 {
                            sum[c] += u32::from(p[c]);
                        }
                        n += 1;
                    }
                }
                let avg = sum.map(|v| (v / n.max(1)) as i32);
                let best = max::THUMB_PALETTE
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, c)| {
                        let d = [
                            i32::from(c.r) - avg[0],
                            i32::from(c.g) - avg[1],
                            i32::from(c.b) - avg[2],
                        ];
                        d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
                    })
                    .map_or(0, |(k, _)| k as u8);
                pixels.push(best);
            }
        }
        pkbm::Bitmap {
            width: tw,
            height: th,
            depth: 4,
            pixels,
            palette: Some(max::THUMB_PALETTE.to_vec()),
        }
    }

    fn write(&mut self, path: &Path) -> Result<(), String> {
        self.extra.thumbnail = Some(pkbm::encode_rows(&self.thumbnail(), (0, 0)));
        let l = self.layout();
        let pages = l.page_starts(&self.doc);
        store::save(path, &self.doc, &self.extra, &pages, Some(self.sel.range()))?;
        self.file = Some(path.to_path_buf());
        self.changed = false;
        Ok(())
    }

    /// Salva com o título `title` na pasta do usuário, sempre num arquivo
    /// novo (como o original: "Gato Teste" regravado vira `GATO_T11.MAX`).
    /// `keep_old` = "Salvar a versão antiga e a nova" (o arquivo anterior
    /// fica); senão ("Salvar só a nova") o anterior é apagado depois de
    /// gravar. Devolve o arquivo gravado.
    pub fn save_as(
        &mut self,
        title: &str,
        author: &str,
        keep_old: bool,
    ) -> Result<PathBuf, String> {
        let Some(dir) = self.dir.clone() else {
            return Err("sem pasta do usuário: documento não guardado".into());
        };
        let old = self.file.clone();
        let path = store::path_for(&dir, title, None);
        self.extra.add.title = title.to_string();
        if !author.is_empty() {
            self.extra.add.author = author.to_string();
        }
        self.write(&path)?;
        if let Some(o) = old.filter(|o| !keep_old && *o != path && o.starts_with(&dir)) {
            let _ = std::fs::remove_file(o);
        }
        Ok(path)
    }

    /// Grava ao sair sem ter escolhido no "Epa!": no arquivo do documento,
    /// ou num `KIDSnnnn.MAX` novo. `None` se não havia o que gravar.
    pub fn save_auto(&mut self) -> Option<Result<PathBuf, String>> {
        if !self.changed {
            return None;
        }
        let Some(dir) = self.dir.clone() else {
            return Some(Err("sem pasta do usuário: documento não guardado".into()));
        };
        let path = self.file.clone().unwrap_or_else(|| store::kids_path(&dir));
        Some(self.write(&path).map(|()| path))
    }

    /// "Salvar como texto": `<NOME>.TXT` ao lado do documento (ou na pasta).
    pub fn save_text(&self) -> Result<PathBuf, String> {
        let Some(dir) = self.dir.as_deref() else {
            return Err("sem pasta do usuário".into());
        };
        let stem = self
            .title()
            .and_then(store::file_stem)
            .unwrap_or_else(|| "TEXTO".into());
        let p = dir.join(format!("{stem}.TXT"));
        store::save_text(&p, &self.doc).map(|()| p)
    }
}

/// `PICT` de uma imagem própria (`META` + `TWPB`) e o contorno dela.
fn picture_chunk(
    img: &Image,
    title: &str,
    id: u16,
) -> (OwnedChunk, crate::writer::object::Outline) {
    let mask: Vec<bool> = img.rgba.chunks_exact(4).map(|p| !is_white(p)).collect();
    let t = twpb::from_mask(&mask, img.width, img.height);
    let chunk = OwnedChunk {
        tag: formats::Tag(*b"PICT"),
        id,
        flags: 0,
        name: (!title.is_empty()).then(|| title.to_string()),
        parts: vec![
            OwnedPart {
                tag: formats::Tag(*b"META"),
                data: wmf::meta_from_image(img),
            },
            OwnedPart {
                tag: formats::Tag(*b"TWPB"),
                data: twpb::write(&t),
            },
        ],
    };
    (chunk, convert::outline_of(&t))
}

/// Inverte as cores do retângulo (vídeo inverso da seleção).
/// Folha `src` em `w × h`: ampliar repete pixels; reduzir tira a média de
/// cada bloco (o texto miúdo fica cinza, como nas vistas reduzidas do
/// original, `cw/w_page/zm3`).
fn resample(src: &Frame, w: i32, h: i32) -> Frame {
    let (w, h) = (w.max(1) as u32, h.max(1) as u32);
    let (sw, sh) = (src.width().max(1), src.height().max(1));
    let px = src.pixels();
    let mut out = Frame::new(w, h);
    for y in 0..h {
        let (y0, y1) = (y * sh / h, ((y + 1) * sh / h).max(y * sh / h + 1).min(sh));
        for x in 0..w {
            let (x0, x1) = (x * sw / w, ((x + 1) * sw / w).max(x * sw / w + 1).min(sw));
            let mut sum = [0u32; 3];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let i = ((sy * sw + sx) * 4) as usize;
                    for (k, v) in sum.iter_mut().enumerate() {
                        *v += u32::from(px[i + k]);
                    }
                }
            }
            let n = ((y1 - y0) * (x1 - x0)).max(1);
            out.put(x as i32, y as i32, sum.map(|v| (v / n) as u8));
        }
    }
    out
}

fn invert(f: &mut Frame, x: i32, y: i32, w: i32, h: i32) {
    for yy in y.max(0)..(y + h).min(f.height() as i32) {
        for xx in x.max(0)..(x + w).min(f.width() as i32) {
            let p = f.pixel(xx as u32, yy as u32);
            f.put(xx, yy, [255 - p[0], 255 - p[1], 255 - p[2]]);
        }
    }
}

/// Copia o quadro `src` em `dst` com o canto em `(x, y)`.
fn copy_into(dst: &mut Frame, src: &Frame, x: i32, y: i32) {
    for sy in 0..src.height() {
        for sx in 0..src.width() {
            let p = src.pixel(sx, sy);
            dst.put(x + sx as i32, y + sy as i32, [p[0], p[1], p[2]]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    #[test]
    fn shrinking_keeps_thin_black_lines() {
        // linha preta de 1 px numa imagem 8×8, reduzida a 2×2
        let mut rgba = vec![255u8; 8 * 8 * 4];
        for y in 0..8 {
            let i = (y * 8 + 5) * 4;
            rgba[i..i + 3].copy_from_slice(&[0, 0, 0]);
        }
        let img = Image {
            width: 8,
            height: 8,
            rgba,
        };
        let s = scaled(&img, 2, 2);
        assert_eq!(&s.rgba[4..8], &[0, 0, 0, 255]);
        assert_eq!(&s.rgba[0..4], &[255, 255, 255, 255]);
        let big = scaled(&img, 16, 16);
        assert_eq!(&big.rgba[(10) * 4..(10) * 4 + 3], &[0, 0, 0]);
    }

    #[test]
    fn view_maps_page_points_like_the_original() {
        let (_d, a) = empty_assets();
        let ed = Editor::blank(None, &a);
        // texto com margens de 72 pontos em (93, 179)
        assert_eq!(ed.to_screen(0, 72, 72), (93, 179));
        assert_eq!(
            ed.to_page(93, 179),
            Some(PagePoint {
                page: 0,
                x: 72,
                y: 72
            })
        );
        assert_eq!(ed.to_page(10, 179), None);
        assert_eq!(ed.page(), 0);
    }

    #[test]
    fn scrolling_reaches_the_next_page_and_stops() {
        let (_d, a) = empty_assets();
        let mut ed = Editor::blank(None, &a);
        // duas folhas
        ed.type_text("a");
        ed.key(Key::Enter, false, false);
        ed.apply_para(ParaChange::Spacing(crate::writer::doc::Spacing::NewPage));
        assert_eq!(ed.layout().page_count(), 2);
        ed.scroll_by(0, 100_000);
        assert_eq!(ed.page(), 1);
        let y = ed.scroll.1;
        ed.scroll_by(0, 10);
        assert_eq!(ed.scroll.1, y);
        ed.show_page(0);
        assert_eq!(ed.page(), 0);
        // o cursor está na folha 2: segui-lo rola até ele aparecer
        ed.follow_caret();
        let c = ed.layout().caret(ed.sel.caret);
        let (_, y) = ed.to_screen(c.page, c.x, c.top);
        assert_eq!(c.page, 1);
        assert!(y >= PAGE.1 && y + c.height <= PAGE.1 + PAGE.3, "{y}");
    }
}
