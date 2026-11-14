//! Caderno de Idéias (`ideas.md` §5, `office-complemento.md` §6.2): uma
//! sobreposição com três abas — "Máquina de Idéias" (sentenças com
//! marcador), "Figuras" (uma página por figura, setas → / ←) e "Lembretes"
//! (texto livre) —, tesoura e câmera, campo de título, NÃO e OK.
//!
//! O Caderno trabalha numa **cópia**: OK grava as três abas
//! (`writer::ideas::store::Notebook::save`) e NÃO fecha na hora, sem
//! perguntar, descartando tudo desde a abertura (inclusive o item que a
//! Máquina ou o Quadro acabaram de pôr). Clique fora do caderno: com um
//! balão de ajuda aberto só o fecha; senão o "Epa! Você já terminou?"
//! (`0805[15]`) e a moldura laranja em volta de NÃO/OK. Tesoura e câmera:
//! nos Lembretes agem na hora sobre a seleção (arrasto); nas outras abas
//! armam o modo e o próximo clique numa entrada a recorta/copia inteira
//! ("OK! Foi copiado.", `0bb8[64]`). O que é copiado vai para a área de
//! transferência da sessão (`session::set_clip`), que o editor cola.
//!
//! É um componente (não uma cena): a Máquina e o Quadro o abrem por cima de
//! si; a Bolsa Mágica do editor pode abri-lo do mesmo jeito.

use super::buttons::Art;
use super::{Owner, max_balloon, outline, play, rex_alert, snd};
use crate::Frame;
use crate::Image;
use crate::assets::Assets;
use crate::balloon::{Balloon, draw_bullet, line_height};
use crate::scene::{Effect, Input, Key, MouseButton};
use crate::text_balloon::TextBalloon;
use crate::ui::{Pen, Rect, inside};
use crate::writer::ideas::machine::Piece;
use crate::writer::ideas::notes::NoteText;
use crate::writer::ideas::session::{self, Clip, Tab};
use crate::writer::ideas::store::{Notebook, TitledPicture};

/// Canto do caderno ao abrir (`7929`, 314×335).
pub const ORIGIN: (i32, i32) = (163, 73);
pub const SIZE: (i32, i32) = (314, 335);
const BODY: Art = Art::masked(0x7929, 0x792d);
/// Abas ativas (relativas ao caderno) e áreas de clique.
const TABS: [(Tab, Art, (i32, i32), Rect); 3] = [
    (
        Tab::Notes,
        Art::masked(0x791f, 0x7921),
        (279, 8),
        (278, 8, 36, 62),
    ),
    (
        Tab::Pictures,
        Art::opaque(0x791b),
        (278, 70),
        (278, 70, 36, 66),
    ),
    (
        Tab::Machine,
        Art::masked(0x7727, 0x7919),
        (278, 136),
        (278, 136, 36, 76),
    ),
];
/// Página (relativa).
const PAGE: Rect = (27, 12, 250, 262);
const SCISSORS: (Art, Rect) = (Art::opaque(0x7923), (16, 286, 32, 32));
const CAMERA: (Art, Rect) = (Art::opaque(0x7927), (53, 286, 32, 32));
const TITLE: Rect = (89, 286, 95, 31);
const NO: (Art, Rect) = (Art::masked(0x0271, 0x0273), (198, 289, 28, 28));
const OK: (Art, Rect) = (Art::masked(0x026d, 0x026f), (230, 289, 26, 27));
const NEXT: Rect = (265, 284, 20, 17);
const PREV: Rect = (265, 301, 20, 18);
/// Faixas por onde se arrasta o caderno (em cima e embaixo).
const GRIP_TOP: Rect = (0, 0, 290, 12);
const GRIP_BOTTOM: Rect = (0, 322, 290, 13);
/// Onde o Max fala.
const HELP_POS: (i32, i32) = (72, 155);
/// "OK! Foi copiado." (`0bb8[64]`), quanto tempo fica.
const COPIED_S: f32 = 1.5;
const ORANGE: [u8; 3] = [255, 140, 50];
const BLACK: [u8; 3] = [0, 0, 0];

/// Como o caderno foi fechado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// OK: as abas foram gravadas.
    Ok,
    /// NÃO: tudo descartado.
    No,
}

/// Tesoura ou câmera armadas (abas Máquina e Figuras).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    Cut,
    Copy,
}

pub struct NotebookView {
    owner: Owner,
    data: Notebook,
    tab: Tab,
    page: usize,
    notes: NoteText,
    offset: (i32, i32),
    /// Arrasto do caderno: ponto apertado relativo ao canto.
    drag: Option<(i32, i32)>,
    /// Arrastando sobre o texto dos Lembretes.
    selecting: bool,
    mode: Option<ToolMode>,
    help: Option<Balloon>,
    alert: Option<TextBalloon>,
    /// Moldura laranja em NÃO/OK (depois do "Você já terminou?").
    warn: bool,
    /// "OK! Foi copiado." com o tempo que falta.
    copied: Option<(Balloon, f32)>,
}

impl NotebookView {
    /// Abre o caderno gravado do usuário (vazio se não houver) na última
    /// aba usada, com o balão "Caderno de Idéias" (`0bb8[59]`).
    pub fn open(owner: &Owner, a: &Assets) -> NotebookView {
        let data = owner.dir.as_deref().map(Notebook::load).unwrap_or_default();
        let notes = NoteText::new(&data.notes);
        let help = max_balloon(
            a,
            59,
            HELP_POS,
            &owner.user,
            225,
            "Caderno de Idéias\nNão abandone nenhuma das suas boas idéias! Anote-as!\nPara fechar o caderno: clique em OK.",
            &[],
        );
        NotebookView {
            owner: owner.clone(),
            tab: session::tab(&owner.user),
            data,
            page: 0,
            notes,
            offset: ORIGIN,
            drag: None,
            selecting: false,
            mode: None,
            help: Some(help),
            alert: None,
            warn: false,
            copied: None,
        }
    }

    pub fn tab(&self) -> Tab {
        self.tab
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn origin(&self) -> (i32, i32) {
        self.offset
    }

    pub fn sentences(&self) -> &[Vec<Piece>] {
        &self.data.sentences
    }

    pub fn pictures(&self) -> &[TitledPicture] {
        &self.data.pictures
    }

    pub fn notes(&self) -> &NoteText {
        &self.notes
    }

    pub fn mode(&self) -> Option<ToolMode> {
        self.mode
    }

    pub fn alert_open(&self) -> bool {
        self.alert.is_some()
    }

    pub fn warned(&self) -> bool {
        self.warn
    }

    /// Retângulo de uma parte do caderno na tela.
    pub fn at(&self, r: Rect) -> Rect {
        (self.offset.0 + r.0, self.offset.1 + r.1, r.2, r.3)
    }

    /// "Colocar essa sentença no Caderno de Idéias": entra como item da aba
    /// Máquina, que fica à mostra.
    pub fn add_sentence(&mut self, s: Vec<Piece>) {
        self.data.sentences.push(s);
        self.set_tab(Tab::Machine);
    }

    /// "Colocar essa figura no Caderno de Idéias": página nova da aba
    /// Figuras, à mostra.
    pub fn add_picture(&mut self, title: &str, image: Image) {
        self.data.pictures.push(TitledPicture::new(title, image));
        self.set_tab(Tab::Pictures);
        self.page = self.data.pictures.len() - 1;
    }

    fn set_tab(&mut self, t: Tab) {
        self.tab = t;
        self.mode = None;
        session::set_tab(&self.owner.user, t);
    }

    fn rel(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.offset.0, y - self.offset.1)
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        let (rx, ry) = self.rel(x, y);
        inside((0, 0, SIZE.0, SIZE.1), rx, ry)
    }

    /// Trata uma entrada; devolve como o caderno fechou, se fechou.
    pub fn handle(&mut self, i: &Input, fx: &mut Vec<Effect>, a: &Assets) -> Option<Outcome> {
        match i {
            Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } => self.click(*x, *y, fx, a),
            Input::MouseMove { x, y } => {
                self.motion(*x, *y, a);
                None
            }
            Input::MouseUp {
                button: MouseButton::Left,
                x,
                y,
            } => {
                self.motion(*x, *y, a);
                self.drag = None;
                self.selecting = false;
                None
            }
            Input::Text(s) if self.tab == Tab::Notes && self.alert.is_none() => {
                self.notes.insert(s);
                None
            }
            Input::Key { key, .. } if self.tab == Tab::Notes && self.alert.is_none() => {
                match key {
                    Key::Enter => self.notes.insert("\n"),
                    Key::Backspace => self.notes.backspace(),
                    Key::Delete => self.notes.delete(),
                    Key::Left => self.notes.left(),
                    Key::Right => self.notes.right(),
                    _ => {}
                }
                None
            }
            _ => None,
        }
    }

    /// Avança o tempo (o "OK! Foi copiado." some sozinho).
    pub fn tick(&mut self, dt: f32) {
        if let Some((_, t)) = self.copied.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.copied = None;
            }
        }
    }

    fn motion(&mut self, x: i32, y: i32, a: &Assets) {
        if let Some((dx, dy)) = self.drag {
            self.offset = (
                (x - dx).clamp(-SIZE.0 / 2, 640 - SIZE.0 / 2),
                (y - dy).clamp(0, 480 - 40),
            );
        } else if self.selecting {
            let k = self.note_index(x, y, a);
            self.notes.drag_to(k);
        }
    }

    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) -> Option<Outcome> {
        // aviso aberto: qualquer clique o fecha
        if self.alert.take().is_some() {
            fx.push(play(snd::BALLOON));
            return None;
        }
        self.copied = None;
        if !self.contains(x, y) {
            if self.help.take().is_some() {
                return None;
            }
            fx.push(play(snd::EPA));
            self.alert = Some(rex_alert(
                a,
                15,
                (self.offset.0 + 52, self.offset.1 + 70),
                210,
                "Você já terminou?\nSe você já tiver terminado de usar o Caderno de Idéias, precisa fechá-lo. Depois pode trabalhar em outra coisa.\nPara fechar o Caderno de Idéias: clique em OK.",
                &[],
            ));
            self.warn = true;
            return None;
        }
        self.help = None;
        let (rx, ry) = self.rel(x, y);
        if inside(OK.1, rx, ry) {
            fx.push(play(snd::DONE));
            self.data.notes = self.notes.text();
            if let Some(dir) = &self.owner.dir
                && let Err(e) = self.data.save(dir)
            {
                fx.push(Effect::Log(format!("Caderno de Idéias: {e}")));
            }
            return Some(Outcome::Ok);
        }
        if inside(NO.1, rx, ry) {
            fx.push(play(snd::NO));
            return Some(Outcome::No);
        }
        if inside(GRIP_TOP, rx, ry) || inside(GRIP_BOTTOM, rx, ry) {
            self.drag = Some((rx, ry));
            return None;
        }
        if let Some((t, ..)) = TABS.iter().find(|(_, _, _, r)| inside(*r, rx, ry)) {
            if *t != self.tab {
                fx.push(play(match t {
                    Tab::Machine => snd::MACHINE,
                    Tab::Pictures => snd::PICTURES,
                    Tab::Notes => snd::NOTEBOOK,
                }));
                self.set_tab(*t);
            }
            return None;
        }
        if inside(SCISSORS.1, rx, ry) {
            self.tool(ToolMode::Cut, fx, a);
            return None;
        }
        if inside(CAMERA.1, rx, ry) {
            self.tool(ToolMode::Copy, fx, a);
            return None;
        }
        if self.tab == Tab::Pictures && !self.data.pictures.is_empty() {
            let n = self.data.pictures.len();
            if inside(NEXT, rx, ry) {
                fx.push(play(snd::ROUND));
                self.page = (self.page + 1) % n;
                return None;
            }
            if inside(PREV, rx, ry) {
                fx.push(play(snd::ROUND));
                self.page = (self.page + n - 1) % n;
                return None;
            }
        }
        if inside(PAGE, rx, ry) {
            self.click_page(x, y, fx, a);
        }
        None
    }

    fn tool(&mut self, m: ToolMode, fx: &mut Vec<Effect>, a: &Assets) {
        fx.push(play(match m {
            ToolMode::Cut => snd::CUT,
            ToolMode::Copy => snd::BOARD,
        }));
        let user = self.owner.user.clone();
        let (k, w, text) = match (self.tab, m) {
            (Tab::Notes, ToolMode::Cut) => (
                72,
                196,
                "Recortar\nJogue fora palavras do seu caderno\n• Arraste o mouse sobre a parte que você quer recortar.",
            ),
            (Tab::Notes, ToolMode::Copy) => (
                73,
                179,
                "Copiar\nPara copiar palavras que estão no seu caderno\n• Arraste o mouse sobre a parte que você quer copiar.",
            ),
            (_, ToolMode::Cut) => (
                61,
                179,
                "Recortar\nJogue fora parte do conteúdo do seu caderno!\n• Clique na entrada que você quer jogar fora.",
            ),
            (_, ToolMode::Copy) => (
                60,
                200,
                "Copiar\nVocê pode copiar idéias do seu Caderno de Idéias para as páginas de um documento.\n• Clique na entrada que você quer copiar.",
            ),
        };
        self.help = Some(max_balloon(a, k, HELP_POS, &user, w, text, &[]));
        if self.tab == Tab::Notes {
            // nos Lembretes a ferramenta age na hora sobre a seleção
            let taken = match m {
                ToolMode::Cut => self.notes.cut(),
                ToolMode::Copy => self.notes.copied(),
            };
            if let Some(t) = taken {
                session::set_clip(&user, Clip::Text(vec![Piece::Text(t)]));
            }
        } else {
            self.mode = Some(m);
        }
    }

    fn click_page(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        match self.tab {
            Tab::Notes => {
                let k = self.note_index(x, y, a);
                self.notes.set_cursor(k);
                self.selecting = true;
            }
            Tab::Machine => {
                let Some(mode) = self.mode else { return };
                let Some(k) = self.sentence_at(y, a) else {
                    return;
                };
                self.take_entry(mode, fx, a, (x, y), |nb| {
                    let s = nb.data.sentences[k].clone();
                    if mode == ToolMode::Cut {
                        nb.data.sentences.remove(k);
                    }
                    Clip::Text(s)
                });
            }
            Tab::Pictures => {
                let Some(mode) = self.mode else { return };
                if self.page >= self.data.pictures.len() {
                    return;
                }
                let k = self.page;
                self.take_entry(mode, fx, a, (x, y), |nb| {
                    let p = &nb.data.pictures[k];
                    let clip = Clip::Picture(p.image.clone(), p.title.clone());
                    if mode == ToolMode::Cut {
                        nb.data.pictures.remove(k);
                        nb.page = nb.page.min(nb.data.pictures.len().saturating_sub(1));
                    }
                    clip
                });
            }
        }
    }

    fn take_entry(
        &mut self,
        mode: ToolMode,
        fx: &mut Vec<Effect>,
        a: &Assets,
        (x, y): (i32, i32),
        take: impl FnOnce(&mut NotebookView) -> Clip,
    ) {
        let clip = take(self);
        session::set_clip(&self.owner.user, clip);
        self.mode = None;
        self.help = None;
        fx.push(play(match mode {
            ToolMode::Cut => snd::CUT,
            ToolMode::Copy => snd::BOARD,
        }));
        if mode == ToolMode::Copy {
            let b = max_balloon(a, 64, (x - 50, y - 30), "", 100, "OK! Foi copiado.", &[]);
            self.copied = Some((b, COPIED_S));
        }
    }

    // --- desenho e geometria das páginas -------------------------------------

    fn page_rect(&self) -> Rect {
        self.at(PAGE)
    }

    fn note_index(&self, x: i32, y: i32, a: &Assets) -> usize {
        let pen = Pen::new(a);
        let lh = line_height(pen.font());
        let r = self.page_rect();
        let m = |s: &str| pen.width(s);
        let lines = self.notes.lines(r.2 - 4, m);
        self.notes.hit(&lines, lh, x - r.0, y - r.1, m)
    }

    /// Linhas de cada sentença da aba Máquina: (índice, y0, y1) na tela.
    fn sentence_rows(&self, a: &Assets) -> Vec<(usize, i32, i32)> {
        let pen = Pen::new(a);
        let lh = line_height(pen.font());
        let r = self.page_rect();
        let mut y = r.1 + 4;
        let mut out = Vec::new();
        for (k, s) in self.data.sentences.iter().enumerate() {
            let h = layout_pieces(s, r.2 - 22, &pen, a)
                .iter()
                .map(|l| l.height.max(lh))
                .sum::<i32>();
            out.push((k, y, y + h));
            y += h + 4;
        }
        out
    }

    fn sentence_at(&self, y: i32, a: &Assets) -> Option<usize> {
        self.sentence_rows(a)
            .into_iter()
            .find(|&(_, y0, y1)| y >= y0 && y < y1)
            .map(|(k, ..)| k)
    }

    fn title(&self) -> String {
        match self.tab {
            Tab::Machine => "Máquina de Idéias".into(),
            Tab::Notes => "Lembretes".into(),
            Tab::Pictures => self
                .data
                .pictures
                .get(self.page)
                .map(|p| p.title.clone())
                .unwrap_or_default(),
        }
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let (ox, oy) = self.offset;
        BODY.draw(f, a, ox, oy);
        if let Some((_, art, (dx, dy), _)) = TABS.iter().find(|(t, ..)| *t == self.tab) {
            art.draw(f, a, ox + dx, oy + dy);
        }
        let pen = Pen::new(a);
        let lh = line_height(pen.font());
        let r = self.page_rect();
        match self.tab {
            Tab::Machine => {
                for (k, y0, _) in self.sentence_rows(a) {
                    draw_bullet(f, r.0 + 2, y0 + 3);
                    let mut y = y0;
                    for line in layout_pieces(&self.data.sentences[k], r.2 - 22, &pen, a) {
                        let h = line.height.max(lh);
                        for (x, tok) in &line.tokens {
                            match tok {
                                Tok::Text(t) => {
                                    pen.text(f, r.0 + 18 + x, y + h - lh, t, BLACK);
                                }
                                Tok::Rebus(img) => {
                                    f.blit(img, r.0 + 18 + x, y + h - img.height as i32);
                                }
                            }
                        }
                        y += h;
                    }
                }
            }
            Tab::Pictures => {
                if let Some(p) = self.data.pictures.get(self.page) {
                    blit_fit(f, &p.image, (r.0 + 4, r.1 + 4, r.2 - 8, r.3 - 8));
                }
            }
            Tab::Notes => {
                let m = |s: &str| pen.width(s);
                let lines = self.notes.lines(r.2 - 4, m);
                let sel = self.notes.selection();
                for (k, &(s, e)) in lines.iter().enumerate() {
                    let y = r.1 + lh * k as i32;
                    if y + lh > r.1 + r.3 {
                        break;
                    }
                    pen.text(f, r.0, y, &self.notes.slice(s, e), BLACK);
                    if let Some((sa, sb)) = sel {
                        let (a0, b0) = (sa.max(s), sb.min(e));
                        if a0 < b0 {
                            let x0 = r.0 + pen.width(&self.notes.slice(s, a0));
                            let x1 = r.0 + pen.width(&self.notes.slice(s, b0));
                            invert(f, (x0, y, x1 - x0, lh));
                        }
                    }
                }
                if sel.is_none() {
                    let (line, cx) = self.notes.caret(&lines, m);
                    f.rect(r.0 + cx, r.1 + lh * line as i32, 1, lh, BLACK);
                }
            }
        }
        SCISSORS.0.draw(f, a, ox + SCISSORS.1.0, oy + SCISSORS.1.1);
        CAMERA.0.draw(f, a, ox + CAMERA.1.0, oy + CAMERA.1.1);
        if let Some(m) = self.mode {
            let r = match m {
                ToolMode::Cut => SCISSORS.1,
                ToolMode::Copy => CAMERA.1,
            };
            outline(f, self.at(r), 2, [255, 238, 76]);
        }
        let t = self.at(TITLE);
        let title = pen.clip(&self.title(), t.2 - 6);
        pen.centered(f, t.0 + t.2 / 2, t.1 + (t.3 - lh) / 2, &title, BLACK);
        NO.0.draw(f, a, ox + NO.1.0, oy + NO.1.1);
        OK.0.draw(f, a, ox + OK.1.0, oy + OK.1.1);
        draw_arrow(f, self.at(NEXT), true);
        draw_arrow(f, self.at(PREV), false);
        if self.warn {
            let (n, k) = (self.at(NO.1), self.at(OK.1));
            outline(
                f,
                (n.0 - 3, n.1 - 3, k.0 + k.2 - n.0 + 6, n.3 + 6),
                2,
                ORANGE,
            );
        }
        if let Some(b) = &self.help {
            b.draw(f, a);
        }
        if let Some((b, _)) = &self.copied {
            b.draw(f, a);
        }
        if let Some(b) = &self.alert {
            b.draw(f, a);
        }
    }
}

/// Seta → (laranja) ou ← (verde) das páginas da aba Figuras, desenhada
/// (a figura do original não foi identificada no `WRITER.PIC`).
fn draw_arrow(f: &mut Frame, r: Rect, right: bool) {
    let color = if right { ORANGE } else { [92, 203, 83] };
    let (cx, cy) = (r.0 + r.2 / 2, r.1 + r.3 / 2);
    for dy in -5i32..=5 {
        let half = 5 - dy.abs();
        for dx in 0..=half {
            let x = if right { cx + dx } else { cx - dx };
            f.put(x, cy + dy, if dx == half { BLACK } else { color });
        }
    }
    let (x0, x1) = if right { (cx - 6, cx) } else { (cx, cx + 6) };
    f.rect(x0, cy - 2, x1 - x0, 5, BLACK);
    f.rect(x0 + 1, cy - 1, x1 - x0 - 1, 3, color);
}

/// Vídeo inverso (seleção nos Lembretes).
fn invert(f: &mut Frame, r: Rect) {
    for y in r.1..r.1 + r.3 {
        for x in r.0..r.0 + r.2 {
            if x >= 0 && y >= 0 && (x as u32) < f.width() && (y as u32) < f.height() {
                let p = f.pixel(x as u32, y as u32);
                f.put(x, y, [255 - p[0], 255 - p[1], 255 - p[2]]);
            }
        }
    }
}

/// Figura reduzida (nunca ampliada) para caber em `r`, centrada.
pub fn blit_fit(f: &mut Frame, img: &Image, r: Rect) {
    let (w, h) = (img.width as i32, img.height as i32);
    if w == 0 || h == 0 {
        return;
    }
    let scale = (r.2 as f32 / w as f32).min(r.3 as f32 / h as f32).min(1.0);
    let (sw, sh) = (
        ((w as f32 * scale) as i32).max(1),
        ((h as f32 * scale) as i32).max(1),
    );
    let (x0, y0) = (r.0 + (r.2 - sw) / 2, r.1 + (r.3 - sh) / 2);
    for y in 0..sh {
        for x in 0..sw {
            let (ix, iy) = (x * w / sw, y * h / sh);
            let i = ((iy * w + ix) * 4) as usize;
            let p = &img.rgba[i..i + 4];
            if p[3] != 0 {
                f.put(x0 + x, y0 + y, [p[0], p[1], p[2]]);
            }
        }
    }
}

/// Pedaço de uma linha de sentença: texto ou figura-rebus.
enum Tok {
    Text(String),
    Rebus(std::rc::Rc<Image>),
}

struct PieceLine {
    tokens: Vec<(i32, Tok)>,
    height: i32,
}

/// Quebra uma sentença (texto e figuras-rebus) em linhas de `width` px.
fn layout_pieces(pieces: &[Piece], width: i32, pen: &Pen, a: &Assets) -> Vec<PieceLine> {
    // palavras e figuras, na ordem
    let mut words: Vec<Tok> = Vec::new();
    for p in pieces {
        match p {
            Piece::Text(t) => {
                for w in t.split(' ') {
                    words.push(Tok::Text(w.to_string()));
                }
            }
            Piece::Rebus(c) => {
                if let Some(img) = a.rebus(*c) {
                    words.push(Tok::Rebus(std::rc::Rc::new(img)));
                }
            }
        }
    }
    let space = pen.width(" ").max(3);
    let mut lines = vec![PieceLine {
        tokens: Vec::new(),
        height: 0,
    }];
    let mut x = 0;
    for w in words {
        let (ww, wh) = match &w {
            Tok::Text(t) if t.is_empty() => continue,
            Tok::Text(t) => (pen.width(t), 0),
            Tok::Rebus(img) => (img.width as i32, img.height as i32),
        };
        if x > 0 && x + ww > width {
            lines.push(PieceLine {
                tokens: Vec::new(),
                height: 0,
            });
            x = 0;
        }
        let line = lines.last_mut().expect("uma linha");
        line.height = line.height.max(wh);
        line.tokens.push((x, w));
        x += ww + space;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;
    use crate::ui::center;

    fn owner(dir: &std::path::Path, user: &str) -> Owner {
        Owner {
            user: user.into(),
            dir: Some(dir.to_path_buf()),
        }
    }

    fn click(nb: &mut NotebookView, a: &Assets, (x, y): (i32, i32)) -> Option<Outcome> {
        let mut fx = Vec::new();
        let out = nb.handle(
            &Input::Click {
                button: MouseButton::Left,
                x,
                y,
            },
            &mut fx,
            a,
        );
        nb.handle(
            &Input::MouseUp {
                button: MouseButton::Left,
                x,
                y,
            },
            &mut fx,
            a,
        );
        out
    }

    #[test]
    fn ok_saves_and_no_discards_everything() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let o = owner(dir.path(), "nb1");
        let mut nb = NotebookView::open(&o, &a);
        nb.add_sentence(vec![Piece::Text("Primeira".into())]);
        let ok = center(nb.at(OK.1));
        assert_eq!(click(&mut nb, &a, ok), Some(Outcome::Ok));
        let mut nb = NotebookView::open(&o, &a);
        assert_eq!(nb.sentences().len(), 1);
        nb.add_sentence(vec![Piece::Text("Segunda".into())]);
        let no = center(nb.at(NO.1));
        assert_eq!(click(&mut nb, &a, no), Some(Outcome::No));
        let nb = NotebookView::open(&o, &a);
        assert_eq!(nb.sentences().len(), 1, "o NÃO descarta o item novo");
    }

    #[test]
    fn click_outside_first_closes_help_then_warns() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut nb = NotebookView::open(&owner(dir.path(), "nb2"), &a);
        assert_eq!(click(&mut nb, &a, (5, 5)), None);
        assert!(!nb.alert_open() && !nb.warned(), "só fechou o balão");
        click(&mut nb, &a, (5, 5));
        assert!(nb.alert_open() && nb.warned());
        click(&mut nb, &a, (5, 5));
        assert!(!nb.alert_open());
    }

    #[test]
    fn notes_cut_and_copy_act_on_the_selection() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let o = owner(dir.path(), "nb3");
        let mut nb = NotebookView::open(&o, &a);
        let tab = TABS.iter().find(|t| t.0 == Tab::Notes).unwrap().3;
        let p = center(nb.at(tab));
        click(&mut nb, &a, p);
        assert_eq!(nb.tab(), Tab::Notes);
        let mut fx = Vec::new();
        nb.handle(&Input::Text("gato astronauta".into()), &mut fx, &a);
        nb.notes.set_cursor(1);
        nb.notes.drag_to(8);
        let p = center(nb.at(SCISSORS.1));
        click(&mut nb, &a, p);
        assert_eq!(nb.notes().text(), "gronauta");
        match session::clip("nb3") {
            Some(Clip::Text(p)) => assert_eq!(p, vec![Piece::Text("ato ast".into())]),
            other => panic!("{other:?}"),
        }
        // a aba aberta fica para a próxima vez
        assert_eq!(session::tab("nb3"), Tab::Notes);
    }

    #[test]
    fn camera_copies_a_whole_sentence() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut nb = NotebookView::open(&owner(dir.path(), "nb4"), &a);
        nb.add_sentence(vec![Piece::Text("Uma".into())]);
        nb.add_sentence(vec![Piece::Text("Duas".into())]);
        let p = center(nb.at(CAMERA.1));
        click(&mut nb, &a, p);
        assert_eq!(nb.mode(), Some(ToolMode::Copy));
        let rows = nb.sentence_rows(&a);
        let (_, y0, y1) = rows[1];
        let x = nb.page_rect().0 + 30;
        click(&mut nb, &a, (x, (y0 + y1) / 2));
        assert_eq!(nb.mode(), None);
        match session::clip("nb4") {
            Some(Clip::Text(p)) => assert_eq!(p, vec![Piece::Text("Duas".into())]),
            other => panic!("{other:?}"),
        }
        assert_eq!(nb.sentences().len(), 2);
        // tesoura recorta
        let p = center(nb.at(SCISSORS.1));
        click(&mut nb, &a, p);
        click(&mut nb, &a, (x, (rows[0].1 + rows[0].2) / 2));
        assert_eq!(nb.sentences().len(), 1);
    }

    #[test]
    fn pictures_have_pages_and_the_notebook_can_be_dragged() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut nb = NotebookView::open(&owner(dir.path(), "nb5"), &a);
        let img = Image {
            width: 4,
            height: 4,
            rgba: vec![0; 64],
        };
        nb.add_picture("A", img.clone());
        nb.add_picture("B", img);
        assert_eq!((nb.tab(), nb.page()), (Tab::Pictures, 1));
        let p = center(nb.at(NEXT));
        click(&mut nb, &a, p);
        assert_eq!(nb.page(), 0);
        let p = center(nb.at(PREV));
        click(&mut nb, &a, p);
        assert_eq!(nb.page(), 1);
        let (x, y) = center(nb.at(GRIP_TOP));
        let mut fx = Vec::new();
        nb.handle(
            &Input::Click {
                button: MouseButton::Left,
                x,
                y,
            },
            &mut fx,
            &a,
        );
        nb.handle(
            &Input::MouseMove {
                x: x - 20,
                y: y + 10,
            },
            &mut fx,
            &a,
        );
        nb.handle(
            &Input::MouseUp {
                button: MouseButton::Left,
                x: x - 20,
                y: y + 10,
            },
            &mut fx,
            &a,
        );
        assert_eq!(nb.origin(), (ORIGIN.0 - 20, ORIGIN.1 + 10));
    }
}
