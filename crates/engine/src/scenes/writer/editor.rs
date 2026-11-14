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
//! com [`PAGE_GAP`] entre elas.

use super::tools::inside;
use crate::Frame;
use crate::Image;
use crate::assets::Assets;
use crate::clipart::{ClipRoll, load_rolls, parse_list};
use crate::scene::Key;
use crate::ui::Rect;
use crate::writer::convert::{self, MaxExtra};
use crate::writer::doc::{CharChange, CharProps, Document, DopProps, Item, ParaChange};
use crate::writer::fonts::{RunFont, WriterFonts};
use crate::writer::layout::Layout;
use crate::writer::object::{FloatObject, ObjRef, ObjectKind};
use crate::writer::selection::{Clip, Clipboard, Motion, Selection};
use crate::writer::store;
use crate::writer::undo::{Snapshot, UndoStack};
use formats::chnk::{self, OwnedChunk, OwnedPart};
use formats::{twpb, wmf};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Bibliotecas de clip-art do Creative Writer (o Fine Artist tem também `MAG100.ART`).
pub const LIBRARIES: [&str; 2] = ["MAXMAG10.ART", "MAX100.ART"];

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
/// Fundo da área da página fora das folhas (as laterais cinza).
const BACKDROP: [u8; 3] = [140, 140, 140];

/// Área da página na tela (a vista): x 23..615, y 108..456.
pub const PAGE: Rect = (23, 108, 593, 349);
/// Canto da primeira folha na tela, sem rolagem (Visão normal).
pub const ORIGIN: (i32, i32) = (21, 107);
/// Espaço entre duas folhas empilhadas na vista (provisório: não medido).
pub const PAGE_GAP: i32 = 8;
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

fn to_owned(c: &chnk::Chunk) -> OwnedChunk {
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

/// Imagem `w`×`h` por vizinho mais próximo.
fn scaled(img: &Image, w: u32, h: u32) -> Image {
    if (img.width, img.height) == (w, h) || w == 0 || h == 0 || img.width == 0 {
        return img.clone();
    }
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let sy = (y * img.height / h).min(img.height - 1);
        for x in 0..w {
            let sx = (x * img.width / w).min(img.width - 1);
            let i = ((sy * img.width + sx) * 4) as usize;
            rgba.extend_from_slice(&img.rgba[i..i + 4]);
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

fn is_white(p: &[u8]) -> bool {
    p[3] == 0 || (p[0] > 250 && p[1] > 250 && p[2] > 250)
}

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
    layout: RefCell<Option<Rc<Layout>>>,
    pictures: RefCell<HashMap<(ObjRef, u16, u16), Option<Rc<Image>>>>,
    /// Rolagem da vista, em pontos: (x, y) da pilha de folhas no canto.
    pub scroll: (i32, i32),
    /// Rolos de adesivos das bibliotecas do Writer.
    pub rolls: Vec<ClipRoll>,
    libs: Vec<(&'static str, Rc<Vec<u8>>)>,
    /// Relógio do piscar do ponto de inserção.
    pub blink: f32,
    /// Ponto de inserção visível (as listas abertas o escondem).
    pub caret_visible: bool,
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
        let (doc, extra) = convert::blank();
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
            layout: RefCell::new(None),
            pictures: RefCell::new(HashMap::new()),
            scroll: (0, 0),
            rolls,
            libs,
            blink: 0.0,
            caret_visible: true,
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
        let l = Rc::new(Layout::build(&self.doc, &*self.fonts));
        *self.layout.borrow_mut() = Some(l.clone());
        l
    }

    fn invalidate(&mut self) {
        *self.layout.get_mut() = None;
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

    /// Altura de uma folha mais o espaço até a seguinte.
    fn page_step(&self) -> i32 {
        i32::from(self.doc.dop.page_height) + PAGE_GAP
    }

    /// Canto da folha `page` na tela.
    pub fn page_origin(&self, page: usize) -> (i32, i32) {
        (
            ORIGIN.0 - self.scroll.0,
            ORIGIN.1 + page as i32 * self.page_step() - self.scroll.1,
        )
    }

    /// Ponto da folha → tela.
    pub fn to_screen(&self, page: usize, x: i32, y: i32) -> (i32, i32) {
        let (ox, oy) = self.page_origin(page);
        (ox + x, oy + y)
    }

    /// Tela → ponto da folha (`None` fora da vista ou entre folhas).
    pub fn to_page(&self, x: i32, y: i32) -> Option<PagePoint> {
        if !inside(PAGE, x, y) {
            return None;
        }
        let sy = y - ORIGIN.1 + self.scroll.1;
        if sy < 0 {
            return None;
        }
        let step = self.page_step();
        let page = (sy / step) as usize;
        let py = sy - page as i32 * step;
        let px = x - ORIGIN.0 + self.scroll.0;
        let (w, h) = (
            i32::from(self.doc.dop.page_width),
            i32::from(self.doc.dop.page_height),
        );
        (page < self.layout().page_count() && py < h && (0..w).contains(&px)).then_some(PagePoint {
            page,
            x: px,
            y: py,
        })
    }

    /// Folha no meio da vista ("Pág. N" da barra de baixo).
    pub fn page(&self) -> usize {
        let step = self.page_step();
        let mid = self.scroll.1 + (PAGE.1 - ORIGIN.1) + PAGE.3 / 2;
        (mid.max(0) / step).clamp(0, self.layout().page_count() as i32 - 1) as usize
    }

    /// Rola até mostrar o alto da folha `page`.
    pub fn show_page(&mut self, page: usize) {
        let n = self.layout().page_count();
        self.scroll.1 = page.min(n - 1) as i32 * self.page_step();
    }

    /// Rola a vista (setas da barra de baixo, Visor); limitada à pilha.
    pub fn scroll_by(&mut self, dx: i32, dy: i32) {
        let n = self.layout().page_count() as i32;
        let max_y = (n * self.page_step() - PAGE_GAP - PAGE.3 - (PAGE.1 - ORIGIN.1)).max(0);
        let max_x = (i32::from(self.doc.dop.page_width) - PAGE.2 - (PAGE.0 - ORIGIN.0)).max(0);
        self.scroll.0 = (self.scroll.0 + dx).clamp(0, max_x);
        self.scroll.1 = (self.scroll.1 + dy).clamp(0, max_y);
    }

    /// Rola o mínimo para o cursor aparecer.
    pub fn follow_caret(&mut self) {
        let l = self.layout();
        let c = l.caret(self.sel.caret);
        let (_, sy) = self.to_screen(c.page, c.x, c.top);
        let top = PAGE.1 + FOLLOW_MARGIN.min(PAGE.3 / 4);
        let bottom = PAGE.1 + PAGE.3 - FOLLOW_MARGIN.min(PAGE.3 / 4);
        if sy < top {
            self.scroll.1 -= top - sy;
        } else if sy + c.height > bottom {
            self.scroll.1 += sy + c.height - bottom;
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
        let img = self
            .extra
            .object(o.object)
            .and_then(decode_pict)
            .map(|i| Rc::new(scaled(&i, u32::from(o.width), u32::from(o.height))));
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
            self.undo.tick(dt);
        }
    }

    // ---------------------------------------------------------------
    // Desenho

    /// As folhas visíveis, com os objetos, o texto, a seleção (vídeo
    /// inverso), o número de página e o ponto de inserção.
    pub fn draw_page(&self, f: &mut Frame, a: &Assets) {
        let (vx, vy, vw, vh) = PAGE;
        let mut v = Frame::new(vw as u32, vh as u32);
        v.fill(BACKDROP);
        let l = self.layout();
        let (pw, ph) = (
            i32::from(self.doc.dop.page_width),
            i32::from(self.doc.dop.page_height),
        );
        for page in 0..l.page_count() {
            let (ox, oy) = self.page_origin(page);
            let (ox, oy) = (ox - vx, oy - vy);
            if oy >= vh || oy + ph <= 0 {
                continue;
            }
            v.rect(ox, oy, pw, ph, WHITE);
            for &k in &l.pages[page].objects {
                self.draw_object(&mut v, &self.doc.objects[k], ox, oy);
            }
            self.draw_lines(&mut v, a, &l, page, ox, oy);
            if let Some(n) = &l.pages[page].number {
                let font = RunFont::default();
                let w = self.fonts.text_width(&font, &n.text);
                let (x0, y0, x1, _) = n.rect;
                self.fonts.draw(
                    &mut v,
                    ox + (x0 + x1 - w) / 2,
                    oy + y0,
                    &font,
                    &n.text,
                    BLACK,
                    false,
                );
            }
            for r in l.selection_rects(self.sel.range()) {
                if r.page == page {
                    let (x0, y0, x1, y1) = r.rect;
                    invert(&mut v, ox + x0, oy + y0, x1 - x0, y1 - y0);
                }
            }
        }
        if self.caret_visible
            && self.sel.is_empty()
            && self.blink % CARET_PERIOD < CARET_PERIOD / 2.0
        {
            let c = l.caret(self.sel.caret);
            let (x, y) = self.to_screen(c.page, c.x, c.top);
            v.rect(x - vx, y - vy, 1, c.height.max(1), BLACK);
        }
        copy_into(f, &v, vx, vy);
    }

    fn draw_object(&self, v: &mut Frame, o: &FloatObject, ox: i32, oy: i32) {
        let (x, y) = (ox + i32::from(o.x), oy + i32::from(o.y));
        match self.picture(o) {
            Some(img) => {
                for (k, p) in img.rgba.chunks_exact(4).enumerate() {
                    if !is_white(p) {
                        let (dx, dy) =
                            ((k as u32 % img.width) as i32, (k as u32 / img.width) as i32);
                        v.put(x + dx, y + dy, [p[0], p[1], p[2]]);
                    }
                }
            }
            None => {
                // sem desenho (Splot, figura sem metarquivo): o lugar dele
                let (w, h) = (i32::from(o.width), i32::from(o.height));
                v.frame_rect(x, y, w, h, 1, BACKDROP);
            }
        }
    }

    fn draw_lines(&self, v: &mut Frame, a: &Assets, l: &Layout, page: usize, ox: i32, oy: i32) {
        let vh = v.height() as i32;
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
                        let (w, h) = (b.w, b.ascent);
                        let x = ox + b.x;
                        let y = oy + b.baseline - h;
                        if crate::writer::ideas::rebus::draw(v, a, c, x, y).is_none() {
                            v.frame_rect(x, y, w.max(1), h.max(1), 1, BACKDROP);
                        }
                        k += 1;
                    }
                    Item::Char(_, props) => {
                        let mut text = String::new();
                        let mut j = k;
                        while j < line.boxes.len() {
                            let bj = line.boxes[j];
                            match para.items.get(bj.index) {
                                Some(Item::Char(c, p))
                                    if *p == props
                                        && bj.size_pt == b.size_pt
                                        && bj.baseline == b.baseline =>
                                {
                                    text.push(*c);
                                    j += 1;
                                }
                                _ => break,
                            }
                        }
                        let mut font = RunFont::of(&self.doc, &props);
                        font.points = b.size_pt;
                        let lm = self.fonts.line_metrics(&font);
                        let last = line.boxes[j - 1];
                        if let Some(hl) = props.highlight {
                            v.rect(
                                ox + b.x,
                                oy + b.baseline - b.ascent,
                                last.x + last.w - b.x,
                                b.ascent + b.descent,
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

    /// Começa um documento em branco (NOVO).
    pub fn new_document(&mut self) {
        let (doc, extra) = convert::blank();
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
        self.invalidate();
        self.sel = Selection::at(self.doc.start());
    }

    /// Esquece as mudanças (Jogar fora): nada será gravado ao sair.
    pub fn discard(&mut self) {
        self.changed = false;
    }

    fn write(&mut self, path: &Path) -> Result<(), String> {
        let l = self.layout();
        let pages = l.page_starts(&self.doc);
        store::save(path, &self.doc, &self.extra, &pages, Some(self.sel.range()))?;
        self.file = Some(path.to_path_buf());
        self.changed = false;
        Ok(())
    }

    /// Salva com o título `title` na pasta do usuário. `keep_old` = "Salvar
    /// a versão antiga e a nova" (o arquivo anterior fica); senão o arquivo
    /// anterior é substituído. Devolve o arquivo gravado.
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
        let keep = if keep_old { None } else { old.as_deref() };
        let path = store::path_for(&dir, title, keep);
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

/// Inverte as cores do retângulo (vídeo inverso da seleção).
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
