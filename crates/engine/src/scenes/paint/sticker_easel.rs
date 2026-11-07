//! Cavalete de adesivos ("Fazer o seu") da ferramenta Seleciona-adesivos:
//! um mini-quadro com moldura laranja sobre o quadro, barra própria embaixo
//! (pincel, lata, ovo, aspirador, lupa, NÃO, OK) e a faixa de cima trocada
//! pelas opções do pincel. O resultado vira um adesivo do usuário
//! (`adesivo-<data>.png` na pasta de desenhos), no último rolo da faixa.
//! Clicar num adesivo colocado (sem arrastar) o traz para cá para modificar.
//! Geometria medida no original (varredura `paint-stickers`, `easel_*.png`).
//!
//! Este módulo também guarda a cola da ferramenta de adesivos com a tela:
//! arrasto de adesivos colocados, balões de dica e os pedidos da faixa
//! (Cachorrinho, Cavalete).

use super::*;
use crate::paint::brush::Stroke;
use crate::paint::store::{save_stamp_png, timestamp};
use crate::paint::{Canvas, History, Objects, WHITE as WHITE_IX};
use crate::scenes::stickers::USER_PREFIX;

type Rect = (i32, i32, i32, i32);

/// Moldura externa do cavalete (5 px de banda) e área branca.
const FRAME: Rect = (149, 172, 341, 221);
const AREA: Rect = (154, 177, 331, 166);
pub const EASEL_W: u32 = 331;
pub const EASEL_H: u32 = 166;
/// Barra de ferramentas do cavalete, sob a área.
const BAR: Rect = (154, 348, 331, 40);
/// Faixa vertical entre a área e a barra, e divisórias da barra.
const BAND_MID_Y: i32 = 343;
const SEP_X: [i32; 2] = [362, 408];
/// Botões da barra: (ferramenta, centro x, imagem, máscara).
const BUTTONS: [(Button, i32, u16, u16); 7] = [
    (Button::Brush, 182, 0x146f, 0x1471),
    (Button::Bucket, 230, 0x1427, 0x1429),
    (Button::Egg, 282, 0x143b, 0x143d),
    (Button::Vacuum, 330, 0x1443, 0x1445),
    (Button::Zoom, 387, 0x1403, 0x1405),
    (Button::No, 437, 0x0271, 0x0273),
    (Button::Ok, 468, 0x026d, 0x026f),
];
/// Lupa "apertada" (cavalete ampliado).
const ZOOM_ON: (u16, u16) = (0x1407, 0x1409);
/// Setas de rolagem no lugar da lupa, com o cavalete ampliado: (dx, dy, imagem, máscara).
const PAN: [((i32, i32), u16, u16); 4] = [
    ((0, -1), 0x151f, 0x1521),
    ((-1, 0), 0x14f3, 0x1519),
    ((1, 0), 0x151b, 0x151d),
    ((0, 1), 0x1523, 0x1525),
];
const PAN_POS: [(i32, i32); 4] = [(382, 349), (368, 362), (394, 362), (382, 375)];
const ZOOM: i32 = 2;
/// Passo da rolagem ampliada, em pixels do cavalete.
const PAN_STEP: i32 = 40;
/// Balão "Este é o Cavalete de adesivos" e "UAU! …" (`WHAT 1005`, 6 e 14).
const HELP_EASEL: (usize, (i32, i32)) = (6, (229, 149));
const HELP_ZOOM: (usize, (i32, i32)) = (14, (216, 224));
/// Dicas: "Clique no adesivo que você quer usar..." (2) e "...então, clique
/// no lugar" (3); balões depois de colocar: estático (4) e animado (9).
const TIP_FILM: usize = 2;
const TIP_PICK: usize = 3;
const PLACED_STATIC: usize = 4;
const PLACED_ANIMATED: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Brush,
    Bucket,
    Egg,
    Vacuum,
    Zoom,
    No,
    Ok,
}

fn button_rect(k: usize) -> Rect {
    let cx = BUTTONS[k].1;
    match BUTTONS[k].0 {
        Button::Zoom => (SEP_X[0] + 5, BAR.1, SEP_X[1] - SEP_X[0] - 5, BAR.3),
        Button::No => (SEP_X[1] + 5, BAR.1, 452 - SEP_X[1] - 5, BAR.3),
        Button::Ok => (452, BAR.1, BAR.0 + BAR.2 - 452, BAR.3),
        _ => (cx - 24, BAR.1, 48, BAR.3),
    }
}

pub struct StickerEasel {
    pub canvas: Canvas,
    history: History,
    /// Sem objetos: só para a interface do histórico.
    none: Objects,
    stroke: Option<Stroke>,
    tool: Button,
    zoom: bool,
    /// Canto do trecho visível, ampliado (coordenadas do cavalete).
    view: (i32, i32),
    /// Objeto do quadro que veio para cá, para trocar a imagem no OK.
    editing: Option<u32>,
}

impl StickerEasel {
    pub fn new() -> StickerEasel {
        StickerEasel {
            canvas: Canvas::new(EASEL_W, EASEL_H),
            history: History::default(),
            none: Objects::default(),
            stroke: None,
            tool: Button::Brush,
            zoom: false,
            view: (0, 0),
            editing: None,
        }
    }

    /// Cavalete com um adesivo colocado (como aparece) no meio, para modificá-lo.
    pub fn editing(id: u32, st: &Stamp) -> StickerEasel {
        let mut e = StickerEasel::new();
        let (x0, y0) = (
            (EASEL_W as i32 - st.w as i32) / 2,
            (EASEL_H as i32 - st.h as i32) / 2,
        );
        e.canvas.paste(st, x0, y0);
        e.editing = Some(id);
        e
    }

    pub fn zoomed(&self) -> bool {
        self.zoom
    }

    pub fn view(&self) -> (i32, i32) {
        self.view
    }

    pub fn tool(&self) -> Button {
        self.tool
    }

    pub fn edited(&self) -> Option<u32> {
        self.editing
    }

    /// Ponto da tela em coordenadas do cavalete (também fora da área).
    pub fn to_canvas(&self, sx: i32, sy: i32) -> (i32, i32) {
        if self.zoom {
            (
                self.view.0 + (sx - AREA.0).div_euclid(ZOOM),
                self.view.1 + (sy - AREA.1).div_euclid(ZOOM),
            )
        } else {
            (sx - AREA.0, sy - AREA.1)
        }
    }

    fn pan(&mut self, dx: i32, dy: i32) {
        let max_x = EASEL_W as i32 - AREA.2 / ZOOM;
        let max_y = EASEL_H as i32 - AREA.3 / ZOOM;
        self.view = (
            (self.view.0 + dx * PAN_STEP).clamp(0, max_x),
            (self.view.1 + dy * PAN_STEP).clamp(0, max_y),
        );
    }

    /// Botão da barra em `(x, y)` (as setas de rolagem têm precedência).
    pub fn button_at(&self, x: i32, y: i32) -> Option<Button> {
        if self.pan_at(x, y).is_some() {
            return None;
        }
        (0..BUTTONS.len())
            .find(|&k| inside(button_rect(k), x, y))
            .map(|k| BUTTONS[k].0)
    }

    fn pan_at(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        if !self.zoom {
            return None;
        }
        PAN_POS
            .iter()
            .position(|&(px, py)| inside((px, py, 12, 12), x, y))
            .map(|k| PAN[k].0)
    }

    /// Adesivo pronto: o desenho sem o branco de fora, cortado ao que foi
    /// pintado; `None` se o cavalete está em branco.
    pub fn stamp(&self) -> Option<Stamp> {
        canvas_to_stamp(&self.canvas)
    }
}

/// Converte o cavalete num carimbo: o branco ligado à borda fica transparente,
/// o resto (inclusive o branco de dentro) opaco; cortado ao retângulo pintado.
pub fn canvas_to_stamp(c: &Canvas) -> Option<Stamp> {
    let (w, h) = (c.w as usize, c.h as usize);
    let mut outside = vec![false; w * h];
    let mut stack: Vec<usize> = Vec::new();
    for x in 0..w {
        stack.push(x);
        stack.push((h - 1) * w + x);
    }
    for y in 0..h {
        stack.push(y * w);
        stack.push(y * w + w - 1);
    }
    while let Some(i) = stack.pop() {
        if outside[i] || c.px[i] != WHITE_IX {
            continue;
        }
        outside[i] = true;
        let (x, y) = (i % w, i / w);
        if x > 0 {
            stack.push(i - 1);
        }
        if x + 1 < w {
            stack.push(i + 1);
        }
        if y > 0 {
            stack.push(i - w);
        }
        if y + 1 < h {
            stack.push(i + w);
        }
    }
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (i, &out) in outside.iter().enumerate() {
        if !out {
            let (x, y) = (i % w, i / w);
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + 1);
            y1 = y1.max(y + 1);
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let (sw, sh) = (x1 - x0, y1 - y0);
    let mut px = Vec::with_capacity(sw * sh);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = y * w + x;
            px.push((!outside[i]).then_some(c.px[i]));
        }
    }
    Some(Stamp {
        w: sw as u32,
        h: sh as u32,
        px,
    })
}

impl Paint {
    /// Balão automático da ferramenta de adesivos (não abre com o McZee fora).
    pub(super) fn auto_help(&mut self, block: u16, index: usize, assets: &Assets) {
        if self.help_on {
            self.open_help(block, index, assets);
        }
    }

    /// Como `auto_help`, com o balão em `pos`.
    pub(super) fn auto_help_at(
        &mut self,
        block: u16,
        index: usize,
        pos: (i32, i32),
        assets: &Assets,
    ) {
        if self.help_on {
            self.open_help_at(block, index, pos, assets);
        }
    }

    pub fn easel(&self) -> Option<&StickerEasel> {
        self.easel.as_ref()
    }

    /// A ferramenta de adesivos mexe nos adesivos colocados (contorno pisca,
    /// arrasto) quando não há nada na mão nem tira aberta.
    pub(super) fn sticker_handles_objects(&self) -> bool {
        !self.stickers.holding()
            && !self.stickers.film_open()
            && self.easel.is_none()
            && self.finder.is_none()
    }

    /// Depois de um clique na faixa dos adesivos: dicas e pedidos (Cachorrinho,
    /// Cavalete). `before`/`after` = (segurando, tira aberta).
    pub(super) fn sticker_strip_after(
        &mut self,
        before: (bool, bool),
        after: (bool, bool),
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) {
        match self.stickers.take_request() {
            Some(Request::Easel) => {
                fx.push(sfx::play(sfx::OPTION));
                self.open_easel(None, assets);
            }
            Some(Request::FindPictures) => {
                fx.push(sfx::play(sfx::TOOLS[2]));
                self.open_finder(assets);
            }
            None => {
                if after.1 && !before.1 {
                    let fx0 = self.stickers.film_x();
                    self.auto_help_at(WHAT_STICKERS, TIP_FILM, (fx0 + 70, 250), assets);
                } else if after.0 && !before.0 {
                    self.auto_help(WHAT_STICKERS, TIP_PICK, assets);
                }
            }
        }
    }

    /// Botão solto na ferramenta de adesivos: um clique parado num bicho
    /// colocado o anima de novo ("Para animar um adesivo outra vez agora
    /// mesmo: Clique nele", `1005[9]`); num adesivo estático, o traz para o
    /// Cavalete ("Para modificar um adesivo: Clique nele", `1005[4]`).
    pub(super) fn sticker_release(&mut self, assets: &Assets) {
        let Some(id) = self.stickers.release() else {
            return;
        };
        if self.tool != Tool::Stickers {
            return;
        }
        let Some(o) = self.objects.get(id) else {
            return;
        };
        if let Source::Roll { roll: 0, item } = o.source {
            let (pos, sound) = ((o.x, o.y), o.sound.clone());
            if self.stickers.replay(item, pos, sound, assets, &self.pal) {
                self.objects.remove(id);
                self.changed = true;
            }
            return;
        }
        self.open_easel(Some(id), assets);
    }

    /// Balão depois de colocar um adesivo (estático ou animado).
    pub(super) fn sticker_notices(&mut self, assets: &Assets) {
        if let Some(roll) = self.stickers.take_placed_notice() {
            let k = if roll == 0 {
                PLACED_ANIMATED
            } else {
                PLACED_STATIC
            };
            self.auto_help(WHAT_STICKERS, k, assets);
        }
    }

    /// Abre o Cavalete, vazio ou com o objeto `edit` do quadro.
    pub(super) fn open_easel(&mut self, edit: Option<u32>, assets: &Assets) {
        self.stickers.reset();
        self.stroke = None;
        self.easel = Some(
            match edit.and_then(|id| Some((id, self.object_stamp(id)?))) {
                Some((id, st)) => StickerEasel::editing(id, &st),
                None => StickerEasel::new(),
            },
        );
        self.auto_help_at(WHAT_STICKERS, HELP_EASEL.0, HELP_EASEL.1, assets);
    }

    /// Fecha o Cavalete sem guardar e reabre o balão Seleciona-adesivos.
    fn close_easel(&mut self, assets: &Assets) {
        self.easel = None;
        self.auto_help(WHAT_STICKERS, 0, assets);
    }

    /// OK: grava o adesivo, troca a imagem do objeto editado ou mostra o
    /// adesivo novo na tira do rolo do usuário.
    fn easel_ok(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(e) = self.easel.take() else {
            return;
        };
        let Some(stamp) = e.stamp() else {
            self.close_easel(assets);
            return;
        };
        let name = format!("{USER_PREFIX}{}.png", timestamp(now_secs()));
        let saved = match &self.drawings {
            Some(dir) => std::fs::create_dir_all(dir)
                .and_then(|_| save_stamp_png(&stamp, &self.pal, &dir.join(&name)))
                .map_err(|err| format!("adesivo: não deu para gravar {name}: {err}")),
            None => Err("adesivo: sem pasta de desenhos; o adesivo não foi guardado".to_string()),
        };
        if let Err(msg) = &saved {
            fx.push(Effect::Log(msg.clone()));
        }
        match e
            .editing
            .and_then(|id| self.objects.get(id).map(|o| o.center()))
        {
            Some((cx, cy)) => {
                self.history.record_all(&self.canvas, &self.objects);
                let id = e.editing.unwrap_or_default();
                if let Some(o) = self.objects.get_mut(id) {
                    let (w, h) = (stamp.w as i32, stamp.h as i32);
                    o.image = Rc::new(stamp.clone());
                    o.w = w;
                    o.h = h;
                    o.x = cx - w / 2;
                    o.y = cy - h / 2;
                    // o giro e a inversão já estão na imagem editada
                    o.flip_h = false;
                    o.flip_v = false;
                    o.turns = 0;
                    if saved.is_ok() {
                        o.source = Source::Png(name.clone());
                    }
                }
                self.changed = true;
                if saved.is_ok() {
                    self.stickers.add_user(name, stamp, &self.pal);
                }
                self.auto_help(WHAT_STICKERS, 0, assets);
            }
            None => {
                let item = self.stickers.add_user(name, stamp, &self.pal);
                self.stickers.show_user(item);
                let fx0 = self.stickers.film_x();
                self.auto_help_at(WHAT_STICKERS, TIP_FILM, (fx0 + 70, 250), assets);
            }
        }
    }

    pub(super) fn easel_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(e) = self.easel.as_mut() else {
            return;
        };
        e.stroke = None;
        if let Some((dx, dy)) = e.pan_at(x, y) {
            e.pan(dx, dy);
            fx.push(sfx::play(sfx::PAGE_ARROW));
            return;
        }
        if let Some(b) = e.button_at(x, y) {
            match b {
                Button::Brush | Button::Bucket => {
                    e.tool = b;
                    fx.push(sfx::play(sfx::OPTION));
                }
                Button::Egg => {
                    let mut none = std::mem::take(&mut e.none);
                    e.history.toggle(&mut e.canvas, &mut none);
                    e.none = none;
                    fx.push(sfx::play(sfx::TOOLS[5]));
                }
                Button::Vacuum => {
                    e.history.record(&e.canvas);
                    e.canvas.clear();
                    fx.push(sfx::play(sfx::TOOLS[6]));
                }
                Button::Zoom => {
                    e.zoom = !e.zoom;
                    fx.push(sfx::play(sfx::MAGNIFIER));
                    if e.zoom {
                        self.auto_help_at(WHAT_STICKERS, HELP_ZOOM.0, HELP_ZOOM.1, assets);
                    }
                }
                Button::No => {
                    fx.push(sfx::play(sfx::OPTION));
                    self.close_easel(assets);
                }
                Button::Ok => {
                    fx.push(sfx::play(sfx::OPTION));
                    self.easel_ok(fx, assets);
                }
            }
            return;
        }
        if inside(AREA, x, y) {
            let (cx, cy) = e.to_canvas(x, y);
            let ink = self.ink();
            let e = self.easel.as_mut().expect("cavalete aberto");
            match e.tool {
                Button::Bucket => {
                    e.history.record(&e.canvas);
                    e.canvas.fill(cx, cy, &ink);
                    fx.push(sfx::play(sfx::bucket(self.bucket_option)));
                }
                _ => {
                    let elastic = self.pen.elastic();
                    if !elastic {
                        e.history.record(&e.canvas);
                    }
                    let seed = (cx as u32).wrapping_mul(2654435761)
                        ^ (cy as u32).wrapping_mul(40503)
                        ^ (now_secs() as u32);
                    e.stroke = self.pen.press(&mut e.canvas, (cx, cy), &ink, seed);
                    if e.stroke.is_some() {
                        let id = sfx::stroke(self.brush.0, self.brush.1);
                        fx.push(self.sfx.start_loop(id));
                    }
                }
            }
            return;
        }
        // a faixa de cima é a do pincel: setas e formas
        if inside(ARROW_UP, x, y) {
            fx.push(sfx::play(sfx::PAGE_ARROW));
            self.page = if self.page == 1 {
                PAGE_COUNT
            } else {
                self.page - 1
            };
            return;
        }
        if inside(ARROW_DOWN, x, y) {
            fx.push(sfx::play(sfx::PAGE_ARROW));
            self.page = self.page % PAGE_COUNT + 1;
            return;
        }
        if let Some(k) = (0..12).find(|&k| inside(option_rect(25, k), x, y)) {
            fx.push(sfx::play(sfx::OPTION));
            self.set_brush(self.page, k);
            return;
        }
        if let Some(k) = (0..SWATCHES).find(|&k| inside(color_rect(k), x, y)) {
            self.color = k;
            return;
        }
        if let Some(k) = (0..SWATCHES).find(|&k| inside(pattern_rect(k), x, y)) {
            self.pattern = k;
        }
        // o resto da tela (barra de ferramentas, saída) fica desligado
    }

    pub(super) fn easel_move(&mut self, x: i32, y: i32) {
        let ink = self.ink();
        let Some(e) = self.easel.as_mut() else {
            return;
        };
        let (cx, cy) = e.to_canvas(x, y);
        if let Some(s) = e.stroke.as_mut() {
            self.pen.drag(s, &mut e.canvas, (cx, cy), &ink);
        }
    }

    pub(super) fn easel_release(&mut self, x: i32, y: i32) {
        let ink = self.ink();
        let Some(e) = self.easel.as_mut() else {
            return;
        };
        let (cx, cy) = e.to_canvas(x, y);
        let Some(mut s) = e.stroke.take() else {
            return;
        };
        self.pen.drag(&mut s, &mut e.canvas, (cx, cy), &ink);
        let before = s.is_elastic().then(|| e.canvas.px.clone());
        self.pen.release(s, &mut e.canvas, (cx, cy), &ink);
        if let Some(before) = before
            && before != e.canvas.px
        {
            e.history.record_previous(before);
        }
    }

    /// Uma atualização com o botão apertado (o spray borrifa no cavalete).
    pub(super) fn easel_tick(&mut self) {
        let ink = self.ink();
        let Some(e) = self.easel.as_mut() else {
            return;
        };
        if let Some(s) = e.stroke.as_mut() {
            self.pen.tick(s, &mut e.canvas, &ink);
        }
    }

    pub(super) fn easel_draw(&self, f: &mut Frame, a: &Assets) {
        let Some(e) = &self.easel else {
            return;
        };
        // barra de ferramentas principal, nicho e saída desligados (cinza)
        gray_region(f, (0, 0, 640, 52));
        gray_region(f, (533, 52, 107, 56));
        // moldura
        band(f, FRAME.0, FRAME.1, FRAME.2, 5, false);
        band(f, FRAME.0, FRAME.1 + FRAME.3 - 5, FRAME.2, 5, true);
        band_v(f, FRAME.0, FRAME.1, 5, FRAME.3, true);
        band_v(f, FRAME.0 + FRAME.2 - 5, FRAME.1, 5, FRAME.3, false);
        band(f, FRAME.0, BAND_MID_Y, FRAME.2, 5, false);
        f.rect(BAR.0, BAR.1, BAR.2, BAR.3, DARK);
        for sx in SEP_X {
            band_v(f, sx, BAR.1, 5, BAR.3, true);
        }
        // área branca e desenho
        if e.zoom {
            for dy in 0..AREA.3 {
                for dx in 0..AREA.2 {
                    let (cx, cy) = (e.view.0 + dx / ZOOM, e.view.1 + dy / ZOOM);
                    let v = e.canvas.get(cx, cy).unwrap_or(WHITE_IX);
                    f.put(AREA.0 + dx, AREA.1 + dy, self.rgb(v));
                }
            }
        } else {
            for (k, &i) in e.canvas.px.iter().enumerate() {
                let x = AREA.0 + (k as u32 % e.canvas.w) as i32;
                let y = AREA.1 + (k as u32 / e.canvas.w) as i32;
                f.put(x, y, self.rgb(i));
            }
        }
        // prévia do traço elástico
        if let Some(s) = &e.stroke {
            for (px, py) in self.pen.preview(s) {
                let (sx, sy) = if e.zoom {
                    (
                        AREA.0 + (px - e.view.0) * ZOOM,
                        AREA.1 + (py - e.view.1) * ZOOM,
                    )
                } else {
                    (AREA.0 + px, AREA.1 + py)
                };
                if inside(AREA, sx, sy) {
                    f.put(sx, sy, BLACK);
                }
            }
        }
        // botões
        for (k, &(b, cx, img, mask)) in BUTTONS.iter().enumerate() {
            if b == e.tool {
                let r = button_rect(k);
                f.rect(cx - 19, r.1, 38, r.3, LIGHT);
            }
            if b == Button::Zoom && e.zoom {
                for (k, &(_, img, mask)) in PAN.iter().enumerate() {
                    let (px, py) = PAN_POS[k];
                    f.blit_masked(&a.image(&pic(img)), &a.image(&pic(mask)), px, py);
                }
                let (img, mask) = ZOOM_ON;
                let im = a.image(&pic(img));
                f.blit_masked(
                    &im,
                    &a.image(&pic(mask)),
                    cx - im.width as i32 / 2,
                    BAR.1 + BAR.3 / 2 - im.height as i32 / 2,
                );
                continue;
            }
            let im = a.image(&pic(img));
            f.blit_masked(
                &im,
                &a.image(&pic(mask)),
                cx - im.width as i32 / 2,
                BAR.1 + BAR.3 / 2 - im.height as i32 / 2,
            );
        }
    }
}

/// Banda horizontal da moldura: preto, laranja, preto, com a linha marrom
/// do lado de fora (`outer`) ou de dentro.
fn band(f: &mut Frame, x: i32, y: i32, w: i32, h: i32, outer: bool) {
    f.rect(x, y, w, h, ORANGE);
    f.rect(x, y, w, 1, BLACK);
    f.rect(x, y + h - 1, w, 1, BLACK);
    let by = if outer { y + h - 2 } else { y + 1 };
    f.rect(x, by, w, 1, BROWN);
}

fn band_v(f: &mut Frame, x: i32, y: i32, w: i32, h: i32, outer: bool) {
    f.rect(x, y, w, h, ORANGE);
    f.rect(x, y, 1, h, BLACK);
    f.rect(x + w - 1, y, 1, h, BLACK);
    let bx = if outer { x + 1 } else { x + w - 2 };
    f.rect(bx, y, 1, h, BROWN);
}

/// Tira a cor de um trecho da tela (ferramentas desligadas).
pub(super) fn gray_region(f: &mut Frame, r: Rect) {
    for y in r.1.max(0)..(r.1 + r.3).min(f.height() as i32) {
        for x in r.0.max(0)..(r.0 + r.2).min(f.width() as i32) {
            let p = f.pixel(x as u32, y as u32);
            let g = ((p[0] as u32 * 77 + p[1] as u32 * 151 + p[2] as u32 * 28) >> 8) as u8;
            let g = 96 + (g as u32 * 128 / 255) as u8;
            f.put(x, y, [g, g, g]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_to_stamp_crops_and_keeps_inner_white() {
        let mut c = Canvas::new(10, 8);
        // um anel 3×3 de tinta 7 com branco dentro, em (2..5, 2..5)
        for (x, y) in [
            (2, 2),
            (3, 2),
            (4, 2),
            (2, 3),
            (4, 3),
            (2, 4),
            (3, 4),
            (4, 4),
        ] {
            c.set(x, y, 7);
        }
        let s = canvas_to_stamp(&c).unwrap();
        assert_eq!((s.w, s.h), (3, 3));
        assert_eq!(s.px[0], Some(7));
        assert_eq!(s.px[4], Some(WHITE_IX), "o branco de dentro fica opaco");
        assert!(canvas_to_stamp(&Canvas::new(4, 4)).is_none());
    }

    #[test]
    fn zoom_maps_screen_to_canvas_and_pans_within_bounds() {
        let mut e = StickerEasel::new();
        assert_eq!(e.to_canvas(AREA.0 + 10, AREA.1 + 5), (10, 5));
        e.zoom = true;
        assert_eq!(e.to_canvas(AREA.0 + 10, AREA.1 + 5), (5, 2));
        e.pan(1, 1);
        assert_eq!(e.view, (PAN_STEP, PAN_STEP));
        for _ in 0..20 {
            e.pan(1, 1);
        }
        assert_eq!(
            e.view,
            (
                EASEL_W as i32 - AREA.2 / ZOOM,
                EASEL_H as i32 - AREA.3 / ZOOM
            )
        );
        e.pan(-50, -50);
        assert_eq!(e.view, (0, 0));
    }

    #[test]
    fn buttons_have_the_measured_centers() {
        let e = StickerEasel::new();
        assert_eq!(e.button_at(182, 368), Some(Button::Brush));
        assert_eq!(e.button_at(230, 368), Some(Button::Bucket));
        assert_eq!(e.button_at(282, 368), Some(Button::Egg));
        assert_eq!(e.button_at(330, 368), Some(Button::Vacuum));
        assert_eq!(e.button_at(387, 368), Some(Button::Zoom));
        assert_eq!(e.button_at(437, 368), Some(Button::No));
        assert_eq!(e.button_at(468, 368), Some(Button::Ok));
        assert_eq!(e.button_at(300, 300), None);
    }
}
