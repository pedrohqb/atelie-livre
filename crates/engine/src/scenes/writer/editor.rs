//! Estado do documento aberto no Escritório e a página: texto, adesivos
//! dentro do texto, ponto de inserção, digitação, desfazer e gravação.
//!
//! É a parte que as ferramentas leem e mudam (recebem `&mut Editor`). Usa o
//! modelo provisório (`crate::writer::legacy`, parágrafos de caracteres);
//! a integração do modelo rico (`writer::{doc,object,layout,selection,undo}`)
//! muda este arquivo — as ferramentas continuam falando com o `Editor`.

use super::tools::inside;
use crate::Frame;
use crate::assets::{Assets, Program};
use crate::clipart::{ClipRoll, load_rolls};
use crate::scene::Key;
use crate::writer::store::{LAST_NAME, load, save};
use crate::writer::{Document, FontSet, History, Item, Layout, Sticker, Style};
use formats::palette::Rgb;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// Bibliotecas de clip-art do Creative Writer (o Fine Artist tem também `MAG100.ART`).
pub const LIBRARIES: [&str; 2] = ["MAXMAG10.ART", "MAX100.ART"];

use crate::ui::Rect;

const BLACK: [u8; 3] = [0, 0, 0];

/// Página branca.
pub const PAGE: Rect = (23, 108, 593, 349);
/// Margens da página, dos quatro lados.
pub const MARGIN: i32 = 24;
/// Largura e altura da área de texto de uma página.
pub const TEXT_W: i32 = PAGE.2 - 2 * MARGIN;
pub const TEXT_H: i32 = PAGE.3 - 2 * MARGIN;
/// Período do piscar do cursor (meio período aceso).
pub const CARET_PERIOD: f32 = 1.0;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub struct Editor {
    texts: Option<PathBuf>,
    pub doc: Document,
    pub history: History,
    pub fonts: FontSet,
    pal: Vec<Rgb>,
    /// Página mostrada (0 = primeira).
    pub page: usize,
    /// Rolos de adesivos das bibliotecas do Writer.
    pub rolls: Vec<ClipRoll>,
    /// Desenhos dos adesivos já decodificados, por (rolo, número).
    art: HashMap<(u16, usize), Option<Rc<crate::Image>>>,
    /// Relógio do piscar do ponto de inserção.
    pub blink: f32,
    /// Ponto de inserção visível (as listas abertas o escondem).
    pub caret_visible: bool,
    pub changed: bool,
}

impl Editor {
    pub fn new(texts: Option<PathBuf>, assets: &Assets) -> Editor {
        let doc = texts
            .as_ref()
            .and_then(|d| load(&d.join(LAST_NAME)))
            .unwrap_or_default();
        Editor::with_doc(texts, doc, assets)
    }

    /// Editor com o documento dado (aberto pela Biblioteca).
    pub fn with_doc(texts: Option<PathBuf>, doc: Document, assets: &Assets) -> Editor {
        let pal = assets.palettes(Program::Writer).pal256.clone();
        let rolls = load_rolls(assets)
            .into_iter()
            .filter(|r| LIBRARIES.contains(&r.library))
            .collect();
        let mut ed = Editor {
            texts,
            doc,
            history: History::default(),
            fonts: FontSet {
                tiny: assets.font(6),
                small: assets.font(8),
                medium: assets.font(16),
            },
            pal,
            page: 0,
            rolls,
            art: HashMap::new(),
            blink: 0.0,
            caret_visible: true,
            changed: false,
        };
        // os adesivos do arquivo só guardam rolo e número: medidas vêm dos desenhos
        let keys: Vec<(u16, usize)> = ed
            .doc
            .paras
            .iter()
            .flat_map(|p| p.items.iter())
            .filter_map(|i| match i {
                Item::Sticker(s) => Some((s.roll, s.index)),
                Item::Char(..) => None,
            })
            .collect();
        let sizes: Vec<_> = keys
            .into_iter()
            .map(|(roll, n)| {
                let size = ed.sticker_art(roll, n).map(|i| (i.width, i.height));
                ((roll, n), size)
            })
            .collect();
        ed.doc
            .resize_stickers(|roll, n| sizes.iter().find(|(k, _)| *k == (roll, n))?.1);
        ed.follow_cursor();
        ed
    }

    pub fn layout(&self) -> Layout {
        Layout::build(&self.doc, &self.fonts, TEXT_W, TEXT_H)
    }

    /// Cor da paleta do Writer.
    pub fn rgb(&self, index: u8) -> [u8; 3] {
        crate::ui::pal_rgb(&self.pal, index)
    }

    /// Desenho do adesivo (em cache); `None` se o rolo ou o item não existem.
    pub fn sticker_art(&mut self, roll: u16, index: usize) -> Option<Rc<crate::Image>> {
        if let Some(a) = self.art.get(&(roll, index)) {
            return a.clone();
        }
        let img = self
            .rolls
            .iter()
            .find(|r| r.id == roll)
            .and_then(|r| r.items.get(index))
            .and_then(|i| i.art())
            .map(Rc::new);
        self.art.insert((roll, index), img.clone());
        img
    }

    /// Insere no cursor o adesivo `index` do rolo `roll` (sem desenho, um quadrado reserva).
    pub fn insert_sticker(&mut self, roll: u16, index: usize) {
        let s = match self.sticker_art(roll, index) {
            Some(img) => Sticker {
                roll,
                index,
                w: img.width,
                h: img.height,
            },
            None => Sticker::placeholder(roll, index),
        };
        self.history.record(&self.doc);
        self.doc.insert_sticker(s);
        self.after_edit();
    }

    /// Depois de mudar o documento: marca, acende o cursor e o acompanha.
    pub fn after_edit(&mut self) {
        self.changed = true;
        self.blink = 0.0;
        self.follow_cursor();
    }

    /// Mostra a página onde está o cursor.
    pub fn follow_cursor(&mut self) {
        let l = self.layout();
        let (k, _) = l.cursor_pos(self.doc.cursor);
        self.page = l.lines[k].page;
    }

    pub fn set_page(&mut self, delta: i32) {
        let pages = self.layout().pages as i32;
        self.page = (self.page as i32 + delta).clamp(0, pages - 1) as usize;
    }

    /// Desfaz ou refaz (o ovo alterna, como o Ctrl+Z do original).
    pub fn toggle_undo(&mut self) -> bool {
        if self.history.toggle(&mut self.doc) {
            self.doc.clamp_cursor();
            self.after_edit();
            true
        } else {
            false
        }
    }

    /// Apaga tudo (desfazível).
    pub fn clear_all(&mut self) -> bool {
        if self.doc.is_empty() {
            return false;
        }
        self.history.record(&self.doc);
        self.doc.clear();
        self.after_edit();
        true
    }

    pub fn type_text(&mut self, text: &str) {
        let clean: String = text
            .chars()
            .filter(|c| !c.is_control() && formats::cp1252::encode_char(*c).is_some())
            .collect();
        if clean.is_empty() {
            return;
        }
        self.history.record_typing(&self.doc);
        self.doc.insert_text(&clean);
        self.after_edit();
    }

    /// Teclas de edição e movimento do ponto de inserção.
    pub fn key(&mut self, key: Key) {
        self.blink = 0.0;
        match key {
            Key::Enter => {
                self.history.record(&self.doc);
                self.doc.newline();
                self.after_edit();
            }
            Key::Backspace => {
                self.history.record_typing(&self.doc);
                if self.doc.backspace() {
                    self.after_edit();
                }
            }
            Key::Left => {
                self.history.close();
                self.doc.move_left();
                self.follow_cursor();
            }
            Key::Right => {
                self.history.close();
                self.doc.move_right();
                self.follow_cursor();
            }
            Key::Up | Key::Down => {
                self.history.close();
                let delta = if key == Key::Up { -1 } else { 1 };
                if let Some(c) = self.layout().cursor_vertical(self.doc.cursor, delta) {
                    self.doc.cursor = c;
                }
                self.follow_cursor();
            }
            Key::Home => {
                self.history.close();
                self.doc.home();
            }
            Key::End => {
                self.history.close();
                self.doc.end();
            }
            Key::PageUp => self.set_page(-1),
            Key::PageDown => self.set_page(1),
            _ => {}
        }
    }

    /// Estilo de digitação.
    pub fn set_style(&mut self, style: Style) {
        self.history.close();
        self.doc.style = style;
    }

    /// Clique na página: põe o ponto de inserção. `false` fora da página.
    pub fn click_page(&mut self, x: i32, y: i32) -> bool {
        if !inside(PAGE, x, y) {
            return false;
        }
        let l = self.layout();
        let (tx, ty) = (x - PAGE.0 - MARGIN, y - PAGE.1 - MARGIN);
        if let Some(k) = l.line_at(self.page, ty) {
            self.history.close();
            self.doc.cursor = l.cursor_in_line(k, tx);
            self.blink = 0.0;
        }
        true
    }

    pub fn tick(&mut self, dt: f32) {
        if dt.is_finite() {
            self.blink = (self.blink + dt.max(0.0)) % CARET_PERIOD;
            self.history.tick(dt);
        }
    }

    /// Página branca com o texto da página atual, adesivos e cursor.
    pub fn draw_page(&self, f: &mut Frame) {
        f.rect(PAGE.0 - 1, PAGE.1, 1, PAGE.3, BLACK);
        f.rect(PAGE.0 + PAGE.2, PAGE.1, 1, PAGE.3, BLACK);
        f.rect(PAGE.0, PAGE.1, PAGE.2, PAGE.3, [255, 255, 255]);
        self.draw_text(f);
    }

    fn draw_text(&self, f: &mut Frame) {
        let l = self.layout();
        let (ox, oy) = (PAGE.0 + MARGIN, PAGE.1 + MARGIN);
        let mut put = |x: i32, y: i32, rgb: [u8; 3]| {
            if inside(PAGE, x, y) {
                f.put(x, y, rgb);
            }
        };
        for line in l.lines.iter().filter(|l| l.page == self.page) {
            let para = &self.doc.paras[line.para];
            let ly = oy + line.y;
            for b in &line.boxes {
                let bx = ox + b.x;
                match &para.items[b.item] {
                    Item::Char(c, style) => {
                        if c.is_whitespace() && !style.underline {
                            continue;
                        }
                        let g = self.fonts.glyph(*c, style);
                        let top = ly + line.ascent - g.ascent;
                        let rgb = self.rgb(style.color);
                        for (dx, dy) in &g.pixels {
                            put(bx + dx, top + dy, rgb);
                        }
                    }
                    Item::Sticker(s) => {
                        let top = ly + line.ascent - s.h as i32;
                        match self.art.get(&(s.roll, s.index)).and_then(|a| a.as_ref()) {
                            Some(img) => {
                                for y in 0..img.height as i32 {
                                    for x in 0..img.width as i32 {
                                        let i = ((y as u32 * img.width + x as u32) * 4) as usize;
                                        let p = &img.rgba[i..i + 3];
                                        if (p[0] as u32 + p[1] as u32 + p[2] as u32) < 384 {
                                            put(bx + x, top + y, BLACK);
                                        }
                                    }
                                }
                            }
                            None => {
                                // adesivo sem desenho: quadro com um X
                                let (w, h) = (s.w as i32, s.h as i32);
                                for k in 0..w.max(h) {
                                    put(bx + k.min(w - 1), top + k.min(h - 1), BLACK);
                                    put(bx + (w - 1 - k).max(0), top + k.min(h - 1), BLACK);
                                }
                                for x in 0..w {
                                    put(bx + x, top, BLACK);
                                    put(bx + x, top + h - 1, BLACK);
                                }
                                for y in 0..h {
                                    put(bx, top + y, BLACK);
                                    put(bx + w - 1, top + y, BLACK);
                                }
                            }
                        }
                    }
                }
            }
        }
        // cursor: linha de 2 px na cor atual, piscando
        if self.caret_visible && self.blink % CARET_PERIOD < CARET_PERIOD / 2.0 {
            let (k, x) = l.cursor_pos(self.doc.cursor);
            let line = &l.lines[k];
            if line.page == self.page {
                let st = self.doc.style_at_cursor();
                let (h, asc) = self.fonts.metrics(st.size);
                let top = oy + line.y + line.ascent - asc;
                let rgb = self.rgb(self.doc.style.color);
                for dy in 0..h {
                    put(ox + x, top + dy, rgb);
                    put(ox + x + 1, top + dy, rgb);
                }
            }
        }
    }

    /// Grava o texto (se mudou) na pasta de textos; devolve a mensagem.
    pub fn save(&mut self) -> String {
        let Some(dir) = &self.texts else {
            return "sem pasta de textos (HOME e XDG_DATA_HOME vazios): texto não guardado".into();
        };
        let msg = match save(dir, &self.doc, self.changed, now_secs()) {
            Ok(()) => format!("texto guardado em {}", dir.display()),
            Err(e) => e,
        };
        self.changed = false;
        msg
    }
}
