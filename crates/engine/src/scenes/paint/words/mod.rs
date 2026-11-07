//! Ferramenta Palavra, como no original: a faixa tem o botão **FORMA**
//! (abre o Cavalete de palavras com forma), o **Oi!** (abre o Cavalete de
//! balões de diálogo) e o **rolo de dipitidus** (ponteiros de balão).
//! Clicar no quadro vazio não faz nada; as palavras só entram pelos
//! cavaletes e viram objetos. Com a Palavra, arrastar uma palavra, balão ou
//! dipitidu move; com Shift, muda o tamanho do balão (largura) ou do
//! dipitidu; um clique sem mover reabre o cavalete (ou o rolo) para modificar.

pub mod balloon_easel;
pub mod shaped_easel;
pub mod ui;

#[cfg(test)]
mod tests;

use self::balloon_easel::BalloonEasel;
use self::shaped_easel::{Reply, ShapedEasel};
use self::ui::{List, ListHit, Rect, inside};
use super::{
    CANVAS_H, CANVAS_W, COLORS, MIN_BALLOON_W, Paint, Source, WHAT_WORDS, WordsStyle, sfx,
};
use crate::Frame;
use crate::assets::Assets;
use crate::clipart::ClipRoll;
use crate::help::HelpBalloon;
use crate::paint::objects::words_stamp;
use crate::paint::shaped::{self, Inks, ShapedWords};
use crate::paint::stamp::Stamp;
use crate::paint::store::nearest;
use crate::paint::{BLACK as INK_BLACK, WHITE as INK_WHITE, words_layout};
use crate::scene::{Effect, Input, MouseButton};
use formats::palette::Rgb;
use std::rc::Rc;
use std::sync::Mutex;

/// Botões da faixa (figura colorida, cinza e máscara) e o rolo com manivelas.
pub const FORMA_BTN: Rect = (83, 59, 43, 41);
pub const OI_BTN: Rect = (229, 59, 43, 41);
pub const ROLL: Rect = (375, 58, 78, 42);
const FORMA_PICS: (u16, u16) = (0x13ff, 0x1401);
const OI_PICS: (u16, u16) = (0x141f, 0x1421);
const ROLL_PICS: (u16, u16) = (0x10af, 0x10cd);
/// Janela do rolo, onde aparece o dipitidu escolhido.
const ROLL_WINDOW: Rect = (395, 64, 40, 30);
/// Coluna suspensa do rolo (como no original: x 400–444, embaixo da faixa).
const COLUMN_X: i32 = 398;
const COLUMN_TOP: i32 = 102;
const COLUMN_W: i32 = 48;
const COLUMN_ROW: i32 = 44;

/// Registros do `WHAT` 1009 (bloco da Palavra).
const HELP_SHAPED: usize = 1;
const HELP_BALLOON: usize = 5;
const HELP_ROLL: usize = 9;
const HELP_ROLL_PICK: usize = 10;
const HELP_TAIL: usize = 12;
const HELP_BALLOON_PLACED: usize = 14;
const HELP_WORD_PLACED: usize = 15;
/// Onde ficam os balões de ajuda dos cavaletes.
const HELP_SHAPED_POS: (i32, i32) = (146, 97);
const HELP_BALLOON_POS: (i32, i32) = (478, 124);
const HELP_ROTATE_POS: (i32, i32) = (390, 140);
/// Texto inicial do balão (o original abre com "Oi!").
const BALLOON_TEXT: &str = "Oi!";

static USER_NAME: Mutex<String> = Mutex::new(String::new());

/// Nome do usuário da sessão: é o texto inicial das palavras com forma
/// (no original, o "Ana" já selecionado no campo).
pub fn set_user_name(name: &str) {
    if let Ok(mut n) = USER_NAME.lock() {
        *n = name.to_string();
    }
}

fn user_name() -> String {
    USER_NAME
        .lock()
        .ok()
        .map(|n| n.clone())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "Palavras".to_string())
}

/// Cavalete aberto.
pub enum Easel {
    Shaped(ShapedEasel),
    Balloon(BalloonEasel),
}

/// Palavra, balão ou dipitidu sendo arrastado com a Palavra.
pub struct WordsDrag {
    id: u32,
    last: (i32, i32),
    resize: bool,
    /// Largura de quebra do balão no começo (0 = a largura do texto).
    width0: u32,
    start_x: i32,
    size0: (i32, i32),
    moved: bool,
    /// O ovo já guardou o estado anterior.
    recorded: bool,
}

/// O que não muda enquanto a pintura está aberta.
pub struct WordsData {
    inks: Inks,
    color_sets: Vec<Vec<u8>>,
    tails: Option<ClipRoll>,
}

pub struct WordsTool {
    pub easel: Option<Easel>,
    pub drag: Option<WordsDrag>,
    data: WordsData,
    /// Dipitidu que aparece no rolo.
    roll_item: usize,
    column: Option<List>,
    /// Um dipitidu escolhido espera o clique no quadro.
    armed: bool,
    /// Dipitidu do quadro que será trocado pelo escolhido na coluna.
    replace: Option<u32>,
    /// O balão "Para girar as palavras" já apareceu.
    rotate_help_shown: bool,
}

/// Cores com mais de uma tinta da lista de cores (alternadas por letra).
const COMBOS: [&[(u8, u8, u8)]; 7] = [
    &[(250, 0, 57), (71, 140, 80)],
    &[(140, 40, 160), (71, 140, 80)],
    &[(250, 74, 114), (130, 140, 230)],
    &[(92, 203, 83), (250, 74, 114), (71, 140, 80)],
    &[(255, 140, 50), (0, 0, 0), (250, 0, 57)],
    &[(60, 60, 220), (255, 238, 76)],
    &[(0, 0, 0), (250, 0, 57)],
];

impl WordsTool {
    pub fn new(assets: &Assets, pal: &[Rgb]) -> WordsTool {
        let mut color_sets: Vec<Vec<u8>> = COLORS.iter().map(|&c| vec![c]).collect();
        for combo in COMBOS {
            color_sets.push(
                combo
                    .iter()
                    .map(|&(r, g, b)| nearest(pal, r, g, b))
                    .collect(),
            );
        }
        WordsTool {
            easel: None,
            drag: None,
            data: WordsData {
                inks: Inks {
                    white: INK_WHITE,
                    black: INK_BLACK,
                    gray: nearest(pal, 128, 128, 128),
                    light: nearest(pal, 190, 190, 190),
                    dark: nearest(pal, 70, 70, 70),
                    pattern: [0; 8],
                },
                color_sets,
                tails: crate::clipart::load_tails(assets),
            },
            roll_item: 0,
            column: None,
            armed: false,
            replace: None,
            rotate_help_shown: false,
        }
    }

    /// Um cavalete ou a coluna do rolo está aberto (prende os cliques).
    pub fn modal(&self) -> bool {
        self.easel.is_some() || self.column.is_some()
    }

    fn tail_count(&self) -> usize {
        self.data.tails.as_ref().map_or(0, |t| t.items.len())
    }
}

/// Contexto do Cavalete de palavras com forma, pelos campos da pintura.
fn shaped_ctx<'a>(
    pal: &'a [Rgb],
    patterns: &'a [[u8; 8]],
    pablo: Option<&'a formats::fnt::Font>,
    ui_font: Option<&'a formats::fnt::Font>,
    data: &'a WordsData,
) -> shaped_easel::Ctx<'a> {
    shaped_easel::Ctx {
        pal,
        patterns,
        pablo,
        ui_font,
        inks: data.inks,
        color_sets: &data.color_sets,
    }
}

impl Paint {
    /// Texto no campo do cavalete aberto, se houver.
    pub fn words_text(&self) -> Option<&str> {
        match &self.words.easel {
            Some(Easel::Shaped(e)) => Some(e.field.text.as_str()),
            Some(Easel::Balloon(e)) => Some(e.field.text.as_str()),
            None => None,
        }
    }

    /// Cavalete aberto: `Some(true)` = palavras com forma, `Some(false)` = balões.
    pub fn words_easel(&self) -> Option<bool> {
        self.words
            .easel
            .as_ref()
            .map(|e| matches!(e, Easel::Shaped(_)))
    }

    /// Palavra com forma no cavalete aberto (para os testes e a ajuda).
    pub fn shaped_words(&self) -> Option<&ShapedWords> {
        match &self.words.easel {
            Some(Easel::Shaped(e)) => Some(&e.words),
            _ => None,
        }
    }

    pub fn tails_column_open(&self) -> bool {
        self.words.column.is_some()
    }

    /// Coluna do rolo aberta (posição das casas).
    pub fn tails_column(&self) -> Option<&List> {
        self.words.column.as_ref()
    }

    fn words_help(&self, index: usize, pos: (i32, i32), assets: &Assets) -> Option<HelpBalloon> {
        if !self.help_on || self.wizard.is_some() {
            return None;
        }
        let records =
            self.dialogs.get(&WHAT_WORDS).cloned().unwrap_or_else(|| {
                Rc::new(assets.dialogs(crate::assets::Program::Artist, WHAT_WORDS))
            });
        HelpBalloon::new(records, WHAT_WORDS, index, pos.0, pos.1)
    }

    /// Entrada com um cavalete (ou a coluna do rolo) aberto: fica toda com
    /// ele. Falso se não há nada aberto.
    pub(super) fn words_modal_input(
        &mut self,
        i: &Input,
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) -> bool {
        if self.words.column.is_some() {
            if let Input::Click {
                button: MouseButton::Left | MouseButton::Right,
                x,
                y,
            } = *i
            {
                self.column_click(x, y, fx, assets);
                return true;
            }
            return false;
        }
        let reply = match self.words.easel.as_mut() {
            None => return false,
            Some(Easel::Shaped(e)) => {
                let ctx = shaped_ctx(
                    &self.pal,
                    &self.patterns,
                    self.font.as_deref(),
                    self.font_small.as_deref(),
                    &self.words.data,
                );
                let r = e.input(i, assets, &ctx);
                e.refresh(&ctx);
                r
            }
            Some(Easel::Balloon(e)) => {
                let ctx = balloon_easel::Ctx {
                    pal: &self.pal,
                    big: self.font.as_deref(),
                    small: self.font_small.as_deref(),
                };
                let r = e.input(i, assets, &ctx);
                e.refresh(&ctx);
                r
            }
        };
        self.easel_reply(reply, fx, assets);
        true
    }

    fn easel_reply(&mut self, r: Reply, fx: &mut Vec<Effect>, assets: &Assets) {
        if let Some(id) = r.sound {
            fx.push(sfx::play(id));
        }
        if let Some(index) = r.help {
            let h = self.words_help(index, HELP_BALLOON_POS, assets);
            if let Some(Easel::Balloon(e)) = self.words.easel.as_mut() {
                e.help = h;
            }
        }
        if r.sound == Some(shaped_easel::SND_ROTATE)
            && let Some(Easel::Shaped(e)) = self.words.easel.as_ref()
            && e.help.is_none()
            && !self.words.rotate_help_shown
        {
            self.words.rotate_help_shown = true;
            let h = self.words_help(shaped_easel::HELP_ROTATE, HELP_ROTATE_POS, assets);
            if let Some(Easel::Shaped(e)) = self.words.easel.as_mut() {
                e.help = h;
            }
        }
        match r.done {
            Some(true) => self.finish_easel(fx, assets),
            Some(false) => self.words.easel = None,
            None => {}
        }
    }

    /// OK: a palavra (ou o balão) vai para o quadro como objeto — nova, no
    /// meio do quadro; modificada, no mesmo lugar (mesmo centro).
    fn finish_easel(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(easel) = self.words.easel.take() else {
            return;
        };
        let (stamp, source, editing, help) = match easel {
            Easel::Shaped(mut e) => {
                e.words.text = e.field.text.clone();
                if e.words.text.trim().is_empty() {
                    return;
                }
                let ctx = shaped_ctx(
                    &self.pal,
                    &self.patterns,
                    self.font.as_deref(),
                    self.font_small.as_deref(),
                    &self.words.data,
                );
                let stamp = shaped_easel::stamp_of(&e.words, &ctx);
                (
                    stamp,
                    Source::Shaped(Box::new(e.words)),
                    e.editing,
                    HELP_WORD_PLACED,
                )
            }
            Easel::Balloon(e) => {
                if e.field.text.trim().is_empty() {
                    return;
                }
                let ctx = balloon_easel::Ctx {
                    pal: &self.pal,
                    big: self.font.as_deref(),
                    small: self.font_small.as_deref(),
                };
                let style = e.final_style(&ctx);
                let stamp = words_stamp(ctx.font(&style), &e.field.text, &style, e.color);
                let source = Source::Words {
                    text: e.field.text.clone(),
                    style,
                    color: e.color,
                };
                (stamp, source, e.editing, HELP_BALLOON_PLACED)
            }
        };
        self.history.record_all(&self.canvas, &self.objects);
        match editing.filter(|&id| self.objects.get(id).is_some()) {
            Some(id) => replace_object(&mut self.objects, id, stamp, source),
            None => {
                let (w, h) = (stamp.w as i32, stamp.h as i32);
                let (cx, cy) = (CANVAS_W as i32 / 2, CANVAS_H as i32 / 2);
                self.objects
                    .place(Rc::new(stamp), cx - w / 2, cy - h / 2, source);
            }
        }
        self.changed = true;
        fx.push(sfx::play(sfx::WORDS_DONE));
        if self.help_on {
            self.open_help(WHAT_WORDS, help, assets);
        }
    }

    fn open_shaped_easel(&mut self, words: ShapedWords, editing: Option<u32>, assets: &Assets) {
        let mut e = ShapedEasel::new(words, editing);
        let ctx = shaped_ctx(
            &self.pal,
            &self.patterns,
            self.font.as_deref(),
            self.font_small.as_deref(),
            &self.words.data,
        );
        e.refresh(&ctx);
        e.help = self.words_help(HELP_SHAPED, HELP_SHAPED_POS, assets);
        self.help = None;
        self.words.easel = Some(Easel::Shaped(e));
    }

    fn open_balloon_easel(
        &mut self,
        text: &str,
        style: WordsStyle,
        color: u8,
        editing: Option<u32>,
        assets: &Assets,
    ) {
        let mut e = BalloonEasel::new(text, style, color, editing);
        let ctx = balloon_easel::Ctx {
            pal: &self.pal,
            big: self.font.as_deref(),
            small: self.font_small.as_deref(),
        };
        e.refresh(&ctx);
        e.help = self.words_help(HELP_BALLOON, HELP_BALLOON_POS, assets);
        self.help = None;
        self.words.easel = Some(Easel::Balloon(e));
    }

    /// Clique na faixa com a Palavra: FORMA, Oi! ou o rolo. Verdadeiro se acertou.
    pub(super) fn words_strip_click(
        &mut self,
        x: i32,
        y: i32,
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) -> bool {
        if inside(FORMA_BTN, x, y) {
            fx.push(sfx::play(sfx::WORDS_OPTION));
            let mut w = ShapedWords::new(&user_name(), COLORS[self.color]);
            w.font = shaped::default_font();
            self.open_shaped_easel(w, None, assets);
            return true;
        }
        if inside(OI_BTN, x, y) {
            fx.push(sfx::play(sfx::WORDS_OPTION));
            self.open_balloon_easel(
                BALLOON_TEXT,
                WordsStyle::balloon(),
                COLORS[self.color],
                None,
                assets,
            );
            return true;
        }
        if inside(ROLL, x, y) {
            fx.push(sfx::play(shaped_easel::SND_MENU));
            self.open_column(None, assets);
            return true;
        }
        false
    }

    fn open_column(&mut self, replace: Option<u32>, assets: &Assets) {
        let n = self.words.tail_count();
        if n == 0 {
            return;
        }
        let mut l = List::below(COLUMN_X, COLUMN_TOP, COLUMN_W, COLUMN_ROW, n, 470);
        l.show(self.words.roll_item.min(n - 1));
        self.words.column = Some(l);
        self.words.replace = replace;
        if self.help_on {
            self.open_help(WHAT_WORDS, HELP_ROLL, assets);
        }
    }

    fn column_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(mut l) = self.words.column.take() else {
            return;
        };
        match l.hit(x, y) {
            Some(ListHit::Item(k)) => {
                self.words.roll_item = k;
                self.help = None;
                match self.words.replace.take() {
                    Some(id) => {
                        if let Some(stamp) = self.tail_stamp(k) {
                            self.history.record_all(&self.canvas, &self.objects);
                            replace_object(&mut self.objects, id, stamp, Source::Tail(k));
                            self.changed = true;
                        }
                    }
                    None => {
                        self.words.armed = true;
                        if self.help_on {
                            self.open_help(WHAT_WORDS, HELP_ROLL_PICK, assets);
                        }
                    }
                }
            }
            Some(h @ (ListHit::Up | ListHit::Down)) => {
                fx.push(sfx::play(sfx::PAGE_ARROW));
                l.scroll_by(h == ListHit::Down);
                self.words.column = Some(l);
            }
            Some(ListHit::Blank) => self.words.column = Some(l),
            // clique fora fecha a coluna
            None => {
                self.words.replace = None;
                self.help = None;
            }
        }
    }

    /// Dipitidu `k` como imagem de objeto (traço preto, miolo branco).
    fn tail_stamp(&self, k: usize) -> Option<Stamp> {
        let art = self.words.data.tails.as_ref()?.items.get(k)?.art()?;
        Some(crate::scenes::stickers::line_art_stamp(&art, &self.pal))
    }

    /// Imagem de uma palavra com forma ou de um dipitidu lidos do arquivo.
    pub(super) fn words_stamp_for(&self, src: &Source) -> Option<Stamp> {
        match src {
            Source::Shaped(w) => {
                let ctx = shaped_ctx(
                    &self.pal,
                    &self.patterns,
                    self.font.as_deref(),
                    self.font_small.as_deref(),
                    &self.words.data,
                );
                Some(shaped_easel::stamp_of(w, &ctx))
            }
            Source::Tail(k) => self.tail_stamp(*k),
            _ => None,
        }
    }

    /// Clique no quadro com a Palavra: põe o dipitidu escolhido, ou pega uma
    /// palavra, balão ou dipitidu para arrastar. No quadro vazio, nada
    /// acontece (como no original).
    pub(super) fn words_canvas_press(
        &mut self,
        cx: i32,
        cy: i32,
        _fx: &mut Vec<Effect>,
        assets: &Assets,
    ) {
        if std::mem::take(&mut self.words.armed) {
            let k = self.words.roll_item;
            if let Some(stamp) = self.tail_stamp(k) {
                let (w, h) = (stamp.w as i32, stamp.h as i32);
                self.history.record_all(&self.canvas, &self.objects);
                self.objects
                    .place(Rc::new(stamp), cx - w / 2, cy - h / 2, Source::Tail(k));
                self.changed = true;
                if self.help_on {
                    self.open_help(WHAT_WORDS, HELP_TAIL, assets);
                }
            }
            return;
        }
        let hit = self.objects.hit(cx, cy).filter(|&id| {
            matches!(
                self.objects.get(id).map(|o| &o.source),
                Some(Source::Words { .. } | Source::Shaped(_) | Source::Tail(_))
            )
        });
        let Some(id) = hit else {
            return;
        };
        let Some(o) = self.objects.get(id) else {
            return;
        };
        let width0 = match &o.source {
            Source::Words { style, .. } => style.width,
            _ => 0,
        };
        self.words.drag = Some(WordsDrag {
            id,
            last: (cx, cy),
            resize: self.shift,
            width0,
            start_x: cx,
            size0: (o.w, o.h),
            moved: false,
            recorded: false,
        });
    }

    pub(super) fn drag_words_object(&mut self, cx: i32, cy: i32) {
        let Some(d) = self.words.drag.as_mut() else {
            return;
        };
        if (cx, cy) == d.last {
            return;
        }
        if !d.recorded {
            self.history.record_all(&self.canvas, &self.objects);
            d.recorded = true;
        }
        d.moved = true;
        let (id, resize, width0, start_x, size0, last) =
            (d.id, d.resize, d.width0, d.start_x, d.size0, d.last);
        d.last = (cx, cy);
        self.changed = true;
        let Some(o) = self.objects.get(id) else {
            return;
        };
        match o.source.clone() {
            Source::Words { text, style, color } if resize => {
                let font = if style.small {
                    self.font_small.as_deref().or(self.font.as_deref())
                } else {
                    self.font.as_deref()
                };
                let base = if width0 == 0 {
                    words_layout(font, &text, &style).tw as u32
                } else {
                    width0
                };
                let width = (base as i32 + cx - start_x).max(MIN_BALLOON_W as i32) as u32;
                let style = WordsStyle { width, ..style };
                let stamp = words_stamp(font, &text, &style, color);
                if let Some(o) = self.objects.get_mut(id) {
                    o.w = stamp.w as i32;
                    o.h = stamp.h as i32;
                    o.image = Rc::new(stamp);
                    o.source = Source::Words { text, style, color };
                }
            }
            Source::Tail(_) if resize => {
                // proporcional, pela distância horizontal desde o começo
                let w = (size0.0 + cx - start_x).max(crate::paint::objects::MIN_SIZE);
                let h = (size0.1 as i64 * w as i64 / size0.0.max(1) as i64) as i32;
                if let Some(o) = self.objects.get_mut(id) {
                    o.resize_centered(w, h);
                }
            }
            _ => self.objects.move_by(id, cx - last.0, cy - last.1),
        }
    }

    /// Solta o que estava sendo arrastado; um clique sem mover reabre o
    /// cavalete (palavras e balões) ou o rolo (dipitidus).
    pub(super) fn release_words_object(&mut self, assets: &Assets) {
        let Some(d) = self.words.drag.take() else {
            return;
        };
        if d.moved {
            return;
        }
        let Some(o) = self.objects.get(d.id) else {
            return;
        };
        match o.source.clone() {
            Source::Words { text, style, color } => {
                self.open_balloon_easel(&text, style, color, Some(d.id), assets);
            }
            Source::Shaped(w) => self.open_shaped_easel(*w, Some(d.id), assets),
            Source::Tail(_) => self.open_column(Some(d.id), assets),
            _ => {}
        }
    }

    /// Passagem do tempo com um cavalete aberto (cursor, giro segurado).
    pub(super) fn words_tick(&mut self, dt: f32) {
        match self.words.easel.as_mut() {
            Some(Easel::Shaped(e)) => {
                e.tick(dt);
                let ctx = shaped_ctx(
                    &self.pal,
                    &self.patterns,
                    self.font.as_deref(),
                    self.font_small.as_deref(),
                    &self.words.data,
                );
                e.refresh(&ctx);
            }
            Some(Easel::Balloon(e)) => e.tick(dt),
            None => {}
        }
    }

    /// Fecha o que estiver aberto, sem colocar nada (saída da tela, novo desenho).
    pub(super) fn words_reset(&mut self) {
        self.words.easel = None;
        self.words.column = None;
        self.words.drag = None;
        self.words.armed = false;
        self.words.replace = None;
    }

    /// Faixa: FORMA, Oi! e o rolo de dipitidus com o escolhido na janela.
    pub(super) fn draw_words_strip(&self, f: &mut Frame, a: &Assets) {
        ui::blit(f, a, FORMA_PICS, FORMA_BTN.0, FORMA_BTN.1);
        ui::blit(f, a, OI_PICS, OI_BTN.0, OI_BTN.1);
        ui::blit(f, a, ROLL_PICS, ROLL.0, ROLL.1);
        if let Some(img) = self
            .words
            .data
            .tails
            .as_ref()
            .and_then(|t| t.items.get(self.words.roll_item))
            .and_then(|i| i.preview.clone())
        {
            blit_centered(f, &img, ROLL_WINDOW);
        }
        if self.words.armed {
            let (x, y, w, h) = ROLL_WINDOW;
            f.frame_rect(x - 2, y - 2, w + 4, h + 4, 1, [250, 0, 57]);
        }
    }

    /// Coluna do rolo e cavalete aberto, por cima de tudo.
    pub(super) fn draw_words_overlay(&self, f: &mut Frame, a: &Assets) {
        if let Some(l) = &self.words.column {
            let tails = self.words.data.tails.as_ref();
            l.draw(f, a, |f, k, r| {
                if let Some(img) = tails
                    .and_then(|t| t.items.get(k))
                    .and_then(|i| i.preview.clone())
                {
                    blit_centered(f, &img, r);
                }
                if k == self.words.roll_item {
                    f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, [250, 0, 57]);
                }
            });
        }
        match &self.words.easel {
            Some(Easel::Shaped(e)) => {
                let ctx = shaped_ctx(
                    &self.pal,
                    &self.patterns,
                    self.font.as_deref(),
                    self.font_small.as_deref(),
                    &self.words.data,
                );
                e.draw(f, a, &ctx);
            }
            Some(Easel::Balloon(e)) => {
                let ctx = balloon_easel::Ctx {
                    pal: &self.pal,
                    big: self.font.as_deref(),
                    small: self.font_small.as_deref(),
                };
                e.draw(f, a, &ctx);
            }
            None => {}
        }
    }
}

/// Troca a imagem e a origem de um objeto, mantendo o centro (sem giros nem
/// inversões: a imagem nova já vem como deve aparecer).
fn replace_object(objects: &mut crate::paint::Objects, id: u32, stamp: Stamp, source: Source) {
    if let Some(o) = objects.get_mut(id) {
        let (cx, cy) = o.center();
        o.w = stamp.w as i32;
        o.h = stamp.h as i32;
        o.x = cx - o.w / 2;
        o.y = cy - o.h / 2;
        o.turns = 0;
        o.flip_h = false;
        o.flip_v = false;
        o.image = Rc::new(stamp);
        o.source = source;
    }
}

/// Imagem centrada em `r` (recortada a ele).
fn blit_centered(f: &mut Frame, img: &crate::Image, r: Rect) {
    let (w, h) = (img.width as i32, img.height as i32);
    let (x0, y0) = (r.0 + (r.2 - w) / 2, r.1 + (r.3 - h) / 2);
    for y in 0..h {
        for x in 0..w {
            let (px, py) = (x0 + x, y0 + y);
            if !inside(r, px, py) {
                continue;
            }
            let k = ((y * w + x) * 4) as usize;
            if let Some(p) = img.rgba.get(k..k + 3) {
                f.put(px, py, [p[0], p[1], p[2]]);
            }
        }
    }
}
