//! História em quadrinhos: molduras sobre o quadro e recorte ao quadrinho.
//!
//! O original permite "até quatro painéis de comprimento" (roteiro de ajuda
//! `WHAT` 3e00 do `ARTIST.EXE`). As molduras são linhas pretas de 3 px
//! desenhadas por cima do quadro (overlay): não entram no desenho guardado.
//!
//! O painel do nicho tem, além dos dígitos, os fundos prontos (rolo "Comic
//! Backgrounds" de `SHARED/STUFF/C-MAG100.ART`: onze desenhos de 271×219 em
//! preto e branco, que se pintam num quadrinho) e os cartões de título e
//! autor (`WHAT` 3e00 [158] e [160]), que vão para a barra inferior e para a
//! página impressa.

use super::{BLACK, NICHE, RED, STRIP, WHITE, card, clear_niche, inside, label, pic};
use crate::assets::{Assets, Program};
use crate::paint::{
    BLACK as INK_BLACK, CANVAS_H, CANVAS_W, Canvas, History, ORIGIN, WHITE as INK_WHITE,
};
use crate::scene::Input;
use crate::text_balloon::{Button, Part, TextBalloon};
use crate::{Frame, Image};
use formats::fnt::Font;
use formats::palette::Rgb;
use std::rc::Rc;

/// Biblioteca e rolo dos fundos prontos.
pub const BACKGROUNDS_LIBRARY: &str = "C-MAG100.ART";
pub const BACKGROUNDS_ROLL: u16 = 28;
/// Bloco `WHAT` dos quadrinhos; registros do título e do nome do autor.
const WHAT_COMICS: u16 = 0x3e00;
const WHAT_TITLE: usize = 158;
const WHAT_AUTHOR: usize = 160;
/// Posição dos balões de título e autor.
const FIELD_POS: (i32, i32) = (200, 150);
/// Folga vertical das faixas do título (em cima) e do autor (embaixo) na
/// página impressa, além da altura da fonte.
pub const BAND_PAD: u32 = 8;

/// Seta esquerda, miniatura do fundo e seta direita no painel.
pub const BG_LEFT: (i32, i32, i32, i32) = (186, 62, 14, 30);
pub const BG_THUMB: (i32, i32, i32, i32) = (204, 58, 56, 36);
pub const BG_RIGHT: (i32, i32, i32, i32) = (264, 62, 14, 30);
/// Cartões "Título" e "Autor".
pub const TITLE_CARD: (i32, i32, i32, i32) = (296, 62, 60, 36);
pub const AUTHOR_CARD: (i32, i32, i32, i32) = (364, 62, 60, 36);

/// Número máximo de quadrinhos.
pub const MAX_PANELS: usize = 4;
/// Margem até a borda do quadro.
pub const MARGIN: i32 = 6;
/// Calha entre quadrinhos.
pub const GUTTER: i32 = 8;
/// Espessura da moldura.
pub const LINE: i32 = 3;

/// Dígitos 1–4 do `ARTIST.PIC`: (colorido, mono, máscara). O 1 não tem
/// versão colorida.
const DIGITS: [(Option<u16>, u16, u16); MAX_PANELS] = [
    (None, 0x3a78, 0x3a79),
    (Some(0x3a7b), 0x3a7c, 0x3a7d),
    (Some(0x3a7f), 0x3a80, 0x3a81),
    (Some(0x3a83), 0x3a84, 0x3a85),
];
/// Largura de cada casa dos dígitos na faixa.
const SLOT_W: i32 = 42;

/// Casa do dígito `k` (0-based) na faixa de opções, à esquerda.
pub fn digit_rect(k: usize) -> (i32, i32, i32, i32) {
    let x0 = STRIP.0 + 8;
    (x0 + SLOT_W * k as i32, STRIP.1, SLOT_W, STRIP.3)
}

/// Um fundo pronto: nome e desenho em preto e branco.
#[derive(Debug, Clone)]
pub struct Background {
    pub name: String,
    pub image: Rc<Image>,
}

/// Lê os fundos do rolo "Comic Backgrounds"; sem a biblioteca, lista vazia.
pub fn load_backgrounds(assets: &Assets) -> Vec<Background> {
    let Some(data) = assets.library_bytes(BACKGROUNDS_LIBRARY) else {
        return Vec::new();
    };
    let Ok(f) = formats::chnk::parse(data) else {
        return Vec::new();
    };
    let tag_is = |t: &formats::tag::Tag, s: &[u8; 4]| &t.0 == s;
    let Some(roll) = f
        .chunks
        .iter()
        .find(|c| tag_is(&c.tag, b"CARO") && c.id == BACKGROUNDS_ROLL)
    else {
        return Vec::new();
    };
    let Some(list) = roll.parts.iter().find(|p| tag_is(&p.tag, b"CARO")) else {
        return Vec::new();
    };
    crate::clipart::parse_list(list.data)
        .into_iter()
        .filter_map(|id| {
            let c = f
                .chunks
                .iter()
                .find(|c| tag_is(&c.tag, b"PICT") && c.id == id)?;
            let meta = c.parts.iter().find(|p| tag_is(&p.tag, b"META"))?;
            let image = formats::wmf::meta_dib(meta.data).ok()?;
            Some(Background {
                name: c.name.clone().unwrap_or_default(),
                image: Rc::new(image),
            })
        })
        .collect()
}

/// Campo de texto aberto: título ou autor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Title,
    Author,
}

/// Desenha `img` esticada (vizinho mais próximo) para caber em `w`×`h`
/// mantendo a proporção, centrada em `(x, y, w, h)`; fundo branco.
fn blit_fit(f: &mut Frame, img: &Image, r: (i32, i32, i32, i32)) {
    f.rect(r.0, r.1, r.2, r.3, WHITE);
    let Some((dx, dy, dw, dh)) = fit(img, r.2, r.3) else {
        return;
    };
    for y in 0..dh {
        for x in 0..dw {
            let sx = (x as f32 / dw as f32 * img.width as f32) as usize;
            let sy = (y as f32 / dh as f32 * img.height as f32) as usize;
            let k = (sy * img.width as usize + sx) * 4;
            if img.rgba.get(k).is_some_and(|&v| v < 128) {
                f.put(r.0 + dx + x, r.1 + dy + y, BLACK);
            }
        }
    }
}

/// Caixa `(dx, dy, dw, dh)` em que `img` cabe num espaço `w`×`h`, centrada.
fn fit(img: &Image, w: i32, h: i32) -> Option<(i32, i32, i32, i32)> {
    if img.width == 0 || img.height == 0 || w <= 0 || h <= 0 {
        return None;
    }
    let scale = (w as f32 / img.width as f32).min(h as f32 / img.height as f32);
    let dw = ((img.width as f32 * scale) as i32).max(1);
    let dh = ((img.height as f32 * scale) as i32).max(1);
    Some(((w - dw) / 2, (h - dh) / 2, dw, dh))
}

/// Retângulos `(x, y, w, h)` dos quadrinhos (coordenadas do quadro, com a
/// moldura incluída), para `n` quadrinhos: 1 inteiro; 2 lado a lado;
/// 3 colunas; 4 = 2×2.
pub fn layout(n: usize) -> Vec<(i32, i32, i32, i32)> {
    let (cols, rows) = match n.clamp(1, MAX_PANELS) {
        1 => (1, 1),
        2 => (2, 1),
        3 => (3, 1),
        _ => (2, 2),
    };
    let area_w = CANVAS_W as i32 - 2 * MARGIN;
    let area_h = CANVAS_H as i32 - 2 * MARGIN;
    let w = (area_w - GUTTER * (cols - 1)) / cols;
    let h = (area_h - GUTTER * (rows - 1)) / rows;
    let mut out = Vec::with_capacity(n);
    for r in 0..rows {
        for c in 0..cols {
            out.push((MARGIN + c * (w + GUTTER), MARGIN + r * (h + GUTTER), w, h));
        }
    }
    out
}

/// Interior de um quadrinho (sem a moldura): o recorte da pintura.
fn interior(r: (i32, i32, i32, i32)) -> (i32, i32, u32, u32) {
    (
        r.0 + LINE,
        r.1 + LINE,
        (r.2 - 2 * LINE).max(0) as u32,
        (r.3 - 2 * LINE).max(0) as u32,
    )
}

/// Estado da história em quadrinhos.
#[derive(Clone)]
pub struct Comics {
    /// Número de quadrinhos (1 a 4).
    pub panels: usize,
    /// O painel do nicho (dígitos, fundos, título) está aberto na faixa.
    pub asking: bool,
    /// Fundos prontos e o mostrado no painel.
    pub backgrounds: Vec<Background>,
    pub background: usize,
    /// Fundo escolhido, à espera do clique no quadrinho.
    pub pending_background: bool,
    pub title: String,
    pub author: String,
    /// Balão de título ou autor aberto.
    pub field: Option<(Field, TextBalloon)>,
}

impl std::fmt::Debug for Comics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Comics")
            .field("panels", &self.panels)
            .field("asking", &self.asking)
            .field("backgrounds", &self.backgrounds.len())
            .field("title", &self.title)
            .field("author", &self.author)
            .finish()
    }
}

impl Default for Comics {
    fn default() -> Comics {
        Comics {
            panels: 1,
            asking: true,
            backgrounds: Vec::new(),
            background: 0,
            pending_background: false,
            title: String::new(),
            author: String::new(),
            field: None,
        }
    }
}

impl Comics {
    pub fn set_panels(&mut self, n: usize) {
        self.panels = n.clamp(1, MAX_PANELS);
    }

    /// Carrega os fundos prontos da biblioteca.
    pub fn load_assets(&mut self, assets: &Assets) {
        self.backgrounds = load_backgrounds(assets);
    }

    /// Metadados gravados ao lado do trabalho.
    pub fn meta(&self) -> Vec<(String, String)> {
        let mut v = vec![("quadrinhos".to_string(), self.panels.to_string())];
        if !self.title.is_empty() {
            v.push(("titulo".into(), self.title.clone()));
        }
        if !self.author.is_empty() {
            v.push(("autor".into(), self.author.clone()));
        }
        v
    }

    pub fn apply_meta(&mut self, key: &str, value: &str) {
        match key {
            "quadrinhos" => {
                if let Ok(n) = value.trim().parse::<usize>() {
                    self.set_panels(n);
                }
            }
            "titulo" => self.title = value.to_string(),
            "autor" => self.author = value.to_string(),
            _ => {}
        }
    }

    /// Texto da barra inferior: o título (e o autor), se houver.
    pub fn display_name(&self) -> Option<String> {
        match (self.title.is_empty(), self.author.is_empty()) {
            (true, true) => None,
            (false, true) => Some(self.title.clone()),
            (true, false) => Some(format!("por {}", self.author)),
            (false, false) => Some(format!("{} — por {}", self.title, self.author)),
        }
    }

    /// Abre o balão de título ou autor (texto do `WHAT`, ou um simples).
    pub fn open_field(&mut self, field: Field, assets: &Assets) {
        let (record, text, current) = match field {
            Field::Title => (WHAT_TITLE, "Digite o título aqui.", &self.title),
            Field::Author => (
                WHAT_AUTHOR,
                "Digite o seu nome aqui — ou invente um!",
                &self.author,
            ),
        };
        let records = assets.dialogs(Program::Artist, WHAT_COMICS);
        let buttons = vec![Button::Stop, Button::Next];
        let mut b = match records.get(record) {
            Some(r) => TextBalloon::from_record(r, FIELD_POS.0, FIELD_POS.1, "", buttons),
            None => TextBalloon::new(
                FIELD_POS.0,
                FIELD_POS.1,
                220,
                vec![
                    Part::Text(text.into()),
                    Part::Field {
                        field: 0,
                        width: 160,
                    },
                ],
                buttons,
            ),
        };
        if b.fields.is_empty() {
            b.parts.push(Part::Field {
                field: 0,
                width: 160,
            });
            b.fields.push(String::new());
        }
        b.fields[0] = current.clone();
        self.field = Some((field, b));
    }

    /// Entrada com o balão de campo aberto; `true` se o balão a consumiu.
    pub fn field_input(&mut self, input: &Input, assets: &Assets) -> bool {
        let Some((field, b)) = self.field.as_mut() else {
            return false;
        };
        if let Input::Click { x, y, .. } = input
            && !b.contains(assets, *x, *y)
        {
            // clique fora: fecha sem mudar
            self.field = None;
            return true;
        }
        match b.handle(assets, input) {
            Some(Button::Next) => {
                let text = b.fields.first().cloned().unwrap_or_default();
                let text = text.trim().to_string();
                match field {
                    Field::Title => self.title = text,
                    Field::Author => self.author = text,
                }
                self.field = None;
            }
            Some(Button::Stop) | Some(Button::Back) => self.field = None,
            None => {}
        }
        true
    }

    /// Pinta o fundo escolhido no quadrinho que contém `(cx, cy)`: o interior
    /// fica branco e recebe o desenho esticado (uma ação do ovo).
    pub fn paint_background(&self, c: &mut Canvas, h: &mut History, cx: i32, cy: i32) -> bool {
        let Some(bg) = self.backgrounds.get(self.background) else {
            return false;
        };
        let (x, y, w, hh) = self.clip_for(cx, cy);
        if w == 0 || hh == 0 {
            return false;
        }
        let Some((dx, dy, dw, dh)) = fit(&bg.image, w as i32, hh as i32) else {
            return false;
        };
        h.record(c);
        let (clip, mask) = (c.clip.take(), c.clip_mask.take());
        for yy in y..y + hh as i32 {
            for xx in x..x + w as i32 {
                c.set(xx, yy, INK_WHITE);
            }
        }
        let img = &bg.image;
        for py in 0..dh {
            for px in 0..dw {
                let sx = (px as f32 / dw as f32 * img.width as f32) as usize;
                let sy = (py as f32 / dh as f32 * img.height as f32) as usize;
                let k = (sy * img.width as usize + sx) * 4;
                if img.rgba.get(k).is_some_and(|&v| v < 128) {
                    c.set(x + dx + px, y + dy + py, INK_BLACK);
                }
            }
        }
        c.clip = clip;
        c.clip_mask = mask;
        true
    }

    pub fn rects(&self) -> Vec<(i32, i32, i32, i32)> {
        layout(self.panels)
    }

    /// Recorte do quadrinho que contém `(cx, cy)`; na calha ou na margem,
    /// recorte vazio (nada pinta).
    pub fn clip_for(&self, cx: i32, cy: i32) -> (i32, i32, u32, u32) {
        self.rects()
            .into_iter()
            .map(interior)
            .find(|&(x, y, w, h)| cx >= x && cy >= y && cx < x + w as i32 && cy < y + h as i32)
            .unwrap_or((0, 0, 0, 0))
    }

    pub fn clip_at(&self, c: &mut Canvas, cx: i32, cy: i32) {
        c.clip = Some(self.clip_for(cx, cy));
    }

    /// Clique: nicho abre/fecha o painel; com o painel aberto, um dígito
    /// escolhe o número de quadrinhos, as setas trocam o fundo, a miniatura
    /// escolhe o fundo (o próximo clique num quadrinho pinta) e os cartões
    /// abrem o título ou o autor. `None` se não era do projeto.
    pub fn click(
        &mut self,
        x: i32,
        y: i32,
        c: &mut Canvas,
        h: &mut History,
        assets: &Assets,
    ) -> Option<bool> {
        if inside(NICHE, x, y) {
            self.asking = !self.asking;
            self.pending_background = false;
            return Some(false);
        }
        if self.asking && inside(STRIP, x, y) {
            if let Some(k) = (0..MAX_PANELS).find(|&k| inside(digit_rect(k), x, y)) {
                self.set_panels(k + 1);
                self.asking = false;
            } else if !self.backgrounds.is_empty() && inside(BG_LEFT, x, y) {
                self.background =
                    (self.background + self.backgrounds.len() - 1) % self.backgrounds.len();
            } else if !self.backgrounds.is_empty() && inside(BG_RIGHT, x, y) {
                self.background = (self.background + 1) % self.backgrounds.len();
            } else if !self.backgrounds.is_empty() && inside(BG_THUMB, x, y) {
                self.pending_background = true;
                self.asking = false;
            } else if inside(TITLE_CARD, x, y) {
                self.open_field(Field::Title, assets);
                self.asking = false;
            } else if inside(AUTHOR_CARD, x, y) {
                self.open_field(Field::Author, assets);
                self.asking = false;
            }
            return Some(false);
        }
        self.asking = false;
        let (cx, cy) = (x - ORIGIN.0, y - ORIGIN.1);
        if self.pending_background {
            self.pending_background = false;
            if cx >= 0 && cy >= 0 && cx < CANVAS_W as i32 && cy < CANVAS_H as i32 {
                return Some(self.paint_background(c, h, cx, cy));
            }
        }
        None
    }

    fn blit_digit(
        &self,
        f: &mut Frame,
        a: &Assets,
        k: usize,
        r: (i32, i32, i32, i32),
        color: bool,
    ) {
        let (col, mono, mask) = DIGITS[k];
        let id = if color { col.unwrap_or(mono) } else { mono };
        let img = a.image(&pic(id));
        let m = a.image(&pic(mask));
        let x = r.0 + (r.2 - img.width as i32) / 2;
        let y = r.1 + (r.3 - img.height as i32) / 2;
        f.blit_masked(&img, &m, x, y);
    }

    /// Painel do nicho: dígitos 1–4 (o escolhido em cor), o fundo pronto com
    /// as setas e os cartões de título e autor.
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        for k in 0..MAX_PANELS {
            let r = digit_rect(k);
            let chosen = k + 1 == self.panels;
            self.blit_digit(f, a, k, r, chosen);
            if chosen {
                f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, RED);
            }
        }
        if let Some(bg) = self.backgrounds.get(self.background) {
            for (r, left) in [(BG_LEFT, true), (BG_RIGHT, false)] {
                f.rect(r.0, r.1, r.2, r.3, WHITE);
                f.frame_rect(r.0, r.1, r.2, r.3, 1, BLACK);
                // seta: triângulo com a base vertical
                for dy in -5..=5i32 {
                    let len = 8 - dy.abs();
                    let x = if left { r.0 + 3 + dy.abs() } else { r.0 + 3 };
                    f.rect(x, r.1 + r.3 / 2 + dy, len, 1, BLACK);
                }
            }
            let t = BG_THUMB;
            blit_fit(f, &bg.image, (t.0 + 1, t.1 + 1, t.2 - 2, t.3 - 2));
            f.frame_rect(t.0, t.1, t.2, t.3, 1, BLACK);
            if self.pending_background {
                f.frame_rect(t.0 + 1, t.1 + 1, t.2 - 2, t.3 - 2, 1, RED);
            }
            if let Some(font) = a.font(8) {
                let w = font.width(&bg.name) as i32;
                let x = (t.0 + t.2 / 2 - w / 2).max(BG_LEFT.0);
                f.text_font(&font, x, t.1 + t.3, &bg.name, BLACK);
            }
        }
        card(f, a, TITLE_CARD, "Título", !self.title.is_empty());
        card(f, a, AUTHOR_CARD, "Autor", !self.author.is_empty());
    }

    /// Molduras por cima do quadro, o dígito atual (ou o fundo à espera do
    /// quadrinho) no nicho e o balão de título/autor, se aberto.
    pub fn draw_overlay(&self, f: &mut Frame, a: &Assets) {
        for (x, y, w, h) in self.rects() {
            f.frame_rect(ORIGIN.0 + x, ORIGIN.1 + y, w, h, LINE, BLACK);
        }
        clear_niche(f);
        match self.backgrounds.get(self.background) {
            Some(bg) if self.pending_background => {
                blit_fit(
                    f,
                    &bg.image,
                    (NICHE.0 + 3, NICHE.1 + 8, NICHE.2 - 6, NICHE.3 - 16),
                );
                label(
                    f,
                    a,
                    (NICHE.0, NICHE.1 + NICHE.3 - 12, NICHE.2, 10),
                    "clique",
                );
            }
            _ => self.blit_digit(f, a, self.panels - 1, NICHE, false),
        }
        if let Some((_, b)) = &self.field {
            b.draw(f, a);
        }
    }

    /// Página impressa: o quadro com as molduras e, se houver, o título em
    /// cima e o autor embaixo.
    pub fn print_page(&self, c: &Canvas, pal: &[Rgb], font: Option<&Font>) -> Canvas {
        let body = self.compose(c);
        if self.title.is_empty() && self.author.is_empty() {
            return body;
        }
        let th = crate::paint::text_size(font, "").1;
        let band = th + BAND_PAD;
        let top = if self.title.is_empty() { 0 } else { band };
        let bottom = if self.author.is_empty() { 0 } else { band };
        let mut page = Canvas::new(body.w, body.h + top + bottom);
        page.px
            .fill(crate::paint::store::nearest(pal, 255, 255, 255));
        for y in 0..body.h {
            for x in 0..body.w {
                if let Some(v) = body.get(x as i32, y as i32) {
                    page.set(x as i32, (y + top) as i32, v);
                }
            }
        }
        let black = crate::paint::store::nearest(pal, 0, 0, 0);
        if top > 0 {
            let (tw, _) = crate::paint::text_size(font, &self.title);
            let x = (body.w as i32 - tw as i32) / 2;
            page.draw_text(font, x, BAND_PAD as i32 / 2, &self.title, black);
        }
        if bottom > 0 {
            let text = format!("por {}", self.author);
            let (tw, _) = crate::paint::text_size(font, &text);
            let x = body.w as i32 - tw as i32 - MARGIN;
            let y = (body.h + top) as i32 + BAND_PAD as i32 / 2;
            page.draw_text(font, x, y, &text, black);
        }
        page
    }

    /// Cópia do quadro com as molduras em preto (para imprimir).
    pub fn compose(&self, c: &Canvas) -> Canvas {
        let mut out = c.clone();
        out.clip = None;
        out.clip_mask = None;
        for (x, y, w, h) in self.rects() {
            for yy in y..y + h {
                for xx in x..x + w {
                    let edge =
                        xx < x + LINE || xx >= x + w - LINE || yy < y + LINE || yy >= y + h - LINE;
                    if edge {
                        out.set(xx, yy, INK_BLACK);
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_fit_in_canvas_with_margin_and_gutter() {
        for n in 1..=MAX_PANELS {
            let rs = layout(n);
            assert_eq!(rs.len(), n);
            for &(x, y, w, h) in &rs {
                assert!(x >= MARGIN && y >= MARGIN);
                assert!(x + w <= CANVAS_W as i32 - MARGIN);
                assert!(y + h <= CANVAS_H as i32 - MARGIN);
            }
        }
        let two = layout(2);
        assert_eq!(two[1].0 - (two[0].0 + two[0].2), GUTTER);
        let four = layout(4);
        assert_eq!(four[2].1 - (four[0].1 + four[0].3), GUTTER);
        assert_eq!(four[0].0, four[2].0);
        assert_eq!(layout(1)[0], (6, 6, 582, 335));
        assert_eq!(layout(9).len(), MAX_PANELS);
    }

    #[test]
    fn clip_is_panel_interior_or_empty_in_gutter() {
        let mut k = Comics::default();
        k.set_panels(4);
        let r = layout(4)[0];
        assert_eq!(
            k.clip_for(r.0 + 10, r.1 + 10),
            (r.0 + LINE, r.1 + LINE, (r.2 - 6) as u32, (r.3 - 6) as u32)
        );
        // calha entre as duas colunas
        assert_eq!(k.clip_for(r.0 + r.2 + 4, r.1 + 10), (0, 0, 0, 0));
        // em cima da própria moldura também não pinta
        assert_eq!(k.clip_for(r.0 + 1, r.1 + 1), (0, 0, 0, 0));
        assert_eq!(k.clip_for(0, 0), (0, 0, 0, 0));
    }

    #[test]
    fn click_digits_and_niche() {
        let (_d, a) = crate::scenes::testutil::empty_assets();
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        let mut h = History::default();
        let mut k = Comics::default();
        assert!(k.asking);
        let r = digit_rect(3);
        assert_eq!(k.click(r.0 + 5, r.1 + 5, &mut c, &mut h, &a), Some(false));
        assert_eq!(k.panels, 4);
        assert!(!k.asking);
        // fechado, a faixa segue para as opções da ferramenta
        assert_eq!(k.click(r.0 + 5, r.1 + 5, &mut c, &mut h, &a), None);
        assert_eq!(
            k.click(NICHE.0 + 3, NICHE.1 + 3, &mut c, &mut h, &a),
            Some(false)
        );
        assert!(k.asking);
        // clique fora fecha o painel sem tratar
        assert_eq!(k.click(300, 300, &mut c, &mut h, &a), None);
        assert!(!k.asking);
        assert!(digit_rect(0).0 > STRIP.0);
        assert!(digit_rect(3).0 + SLOT_W < BG_LEFT.0);
        assert!(AUTHOR_CARD.0 + AUTHOR_CARD.2 < STRIP.0 + STRIP.2);
    }

    fn bg(w: u32, h: u32) -> Background {
        // metade esquerda preta, direita branca
        let mut rgba = vec![255u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w / 2 {
                let k = ((y * w + x) * 4) as usize;
                rgba[k] = 0;
                rgba[k + 1] = 0;
                rgba[k + 2] = 0;
            }
        }
        Background {
            name: "Teste".into(),
            image: Rc::new(Image {
                width: w,
                height: h,
                rgba,
            }),
        }
    }

    #[test]
    fn background_paints_the_clicked_panel_only() {
        let (_d, a) = crate::scenes::testutil::empty_assets();
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(7);
        let mut h = History::default();
        let mut k = Comics::default();
        k.set_panels(2);
        k.asking = true;
        k.backgrounds = vec![bg(100, 50), bg(10, 10)];
        // setas circulam; a miniatura arma o fundo e fecha o painel
        k.click(BG_RIGHT.0 + 2, BG_RIGHT.1 + 2, &mut c, &mut h, &a);
        assert_eq!(k.background, 1);
        k.click(BG_LEFT.0 + 2, BG_LEFT.1 + 2, &mut c, &mut h, &a);
        assert_eq!(k.background, 0);
        k.click(BG_THUMB.0 + 2, BG_THUMB.1 + 2, &mut c, &mut h, &a);
        assert!(k.pending_background && !k.asking);
        // clique no 2.º quadrinho: interior branco com a metade esquerda preta
        let r = layout(2)[1];
        let (ix, iy, iw, ih) = interior(r);
        assert_eq!(
            k.click(ORIGIN.0 + ix + 10, ORIGIN.1 + iy + 10, &mut c, &mut h, &a),
            Some(true)
        );
        assert!(!k.pending_background);
        assert_eq!(c.get(ix + 2, iy + ih as i32 / 2), Some(INK_BLACK));
        assert_eq!(
            c.get(ix + iw as i32 - 3, iy + ih as i32 / 2),
            Some(INK_WHITE)
        );
        // o 1.º quadrinho não mudou; o ovo desfaz
        let r0 = layout(2)[0];
        assert_eq!(c.get(r0.0 + 10, r0.1 + 10), Some(7));
        assert!(h.toggle(&mut c, &mut crate::paint::Objects::default()));
        assert_eq!(c.get(ix + 2, iy + ih as i32 / 2), Some(7));
        // na calha nada acontece
        k.pending_background = true;
        assert_eq!(
            k.click(
                ORIGIN.0 + r0.0 + r0.2 + 4,
                ORIGIN.1 + 100,
                &mut c,
                &mut h,
                &a
            ),
            Some(false)
        );
    }

    #[test]
    fn title_and_author_fields_and_print_page() {
        let (_d, a) = crate::scenes::testutil::empty_assets();
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        let mut h = History::default();
        let mut k = Comics::default();
        k.click(TITLE_CARD.0 + 2, TITLE_CARD.1 + 2, &mut c, &mut h, &a);
        assert!(matches!(&k.field, Some((Field::Title, _))));
        assert!(k.field_input(&Input::Text("Meu gibi".into()), &a));
        assert!(k.field_input(
            &Input::Key {
                key: crate::scene::Key::Enter,
                ctrl: false,
                alt: false
            },
            &a
        ));
        assert!(k.field.is_none());
        assert_eq!(k.title, "Meu gibi");
        assert_eq!(k.display_name().as_deref(), Some("Meu gibi"));
        k.asking = true;
        k.click(AUTHOR_CARD.0 + 2, AUTHOR_CARD.1 + 2, &mut c, &mut h, &a);
        k.field_input(&Input::Text("Ana".into()), &a);
        // clique fora fecha sem gravar
        k.field_input(
            &Input::Click {
                button: crate::scene::MouseButton::Left,
                x: 5,
                y: 470,
            },
            &a,
        );
        assert_eq!(k.author, "");
        assert!(k.field.is_none());
        k.author = "Ana".into();
        assert_eq!(k.display_name().as_deref(), Some("Meu gibi — por Ana"));
        assert_eq!(
            k.meta(),
            vec![
                ("quadrinhos".to_string(), "1".to_string()),
                ("titulo".to_string(), "Meu gibi".to_string()),
                ("autor".to_string(), "Ana".to_string()),
            ]
        );
        let pal: Vec<Rgb> = (0..256)
            .map(|i| Rgb {
                r: i as u8,
                g: i as u8,
                b: i as u8,
            })
            .collect();
        let page = k.print_page(&c, &pal, None);
        // fonte reserva de 8 px: faixas de 16
        let band = 8 + BAND_PAD;
        assert_eq!(page.h, CANVAS_H + 2 * band);
        assert_eq!(page.w, CANVAS_W);
        // há texto preto na faixa de cima e na de baixo
        let has_ink = |y0: u32, y1: u32| {
            (y0..y1).any(|y| (0..page.w).any(|x| page.get(x as i32, y as i32) == Some(0)))
        };
        assert!(has_ink(0, band));
        assert!(has_ink(CANVAS_H + band, page.h));
    }

    #[test]
    fn compose_draws_black_frames_only_on_edges() {
        let mut k = Comics::default();
        k.set_panels(2);
        let c = Canvas::new(CANVAS_W, CANVAS_H);
        let out = k.compose(&c);
        let r = layout(2)[0];
        assert_eq!(out.get(r.0, r.1), Some(INK_BLACK));
        assert_eq!(out.get(r.0 + 2, r.1 + 100), Some(INK_BLACK));
        assert_eq!(out.get(r.0 + 3, r.1 + 100), Some(0));
        assert_eq!(out.get(0, 0), Some(0));
        let black = out.px.iter().filter(|&&v| v == INK_BLACK).count();
        let per = 2 * (r.2 * LINE) + 2 * ((r.3 - 2 * LINE) * LINE);
        assert_eq!(black, 2 * per as usize);
    }
}
