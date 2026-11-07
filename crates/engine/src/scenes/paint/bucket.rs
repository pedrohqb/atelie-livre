//! Lata de tinta na tela de pintura: o rolo de papéis de parede (coluna
//! suspensa à esquerda da faixa), o despejo de cada uma das dez opções e o
//! preenchimento surpresa, que só aparece por cima do quadro. Medidas e sons
//! tirados das capturas do original (ver `docs/telas/fine-artist.md`).

use super::sfx;
use super::{BLACK, Paint, SND_ZOOM_SCROLL, Tool, WHAT_BUCKET, WHITE, canvas_rect, inside, pic};
use crate::Frame;
use crate::assets::Assets;
use crate::paint::Source;
use crate::paint::bucket_fx::{self, Moving, Pour, TILE, Wallpaper, load_wallpapers};
use crate::rng::XorShift;
use crate::scene::Effect;
use formats::palette::Rgb;

/// Opções da faixa, na ordem dos botões.
const NORMAL: usize = 0;
const SWAP_COLORS: usize = 1;
const SWAP_PATTERNS: usize = 2;
const OUTLINE: usize = 3;
const BLEND: usize = 4;
const POINTS: usize = 5;
const POP: usize = 6;
const BUBBLES: usize = 7;
const SPIN: usize = 8;
const SURPRISE: usize = 9;

/// Rolo de papéis de parede: área do clique (as manivelas e a amostra) e a
/// figura do rolo com as duas manivelas.
const ROLL: (i32, i32, i32, i32) = (8, 56, 73, 45);
const ROLL_PIC: (u16, i32, i32) = (0x10af, 9, 58);
/// Amostra dentro do rolo (cantos arredondados).
const SWATCH: (i32, i32, i32) = (39, 65, 30);
/// Coluna suspensa: bordas pretas em x = 33 e 74, de y = 99 a 442.
const COLUMN: (i32, i32, i32, i32) = (33, 99, 42, 344);
const CELL_X: i32 = 34;
const CELL_W: i32 = 40;
/// Casa "Normal" e primeira casa de papel; cada casa tem 40 px e 1 de divisa.
const NORMAL_Y: i32 = 100;
const FIRST_Y: i32 = 141;
const CELL_STEP: i32 = 41;
const THUMB_DX: i32 = 4;
const THUMB_DY: i32 = 4;
pub const VISIBLE: usize = 7;
/// Setas ▲ ▼ da coluna (12×12).
const ARROW_UP: (u16, i32, i32) = (0x02db, 39, 429);
const ARROW_DOWN: (u16, i32, i32) = (0x02df, 57, 429);
const ARROW_SIZE: i32 = 12;
/// Sons: abrir a coluna e escolher um papel (fechar não tem som).
const SND_OPEN: u16 = 0x0233;
const SND_PICK: u16 = 0x022d;
/// Registro do balão "Rolo de papéis de parede".
const WHAT_ROLL: usize = 18;
/// Largura do contorno, em pixels.
const OUTLINE_W: i32 = 3;
/// Raio do sorteio dos pontinhos e número de bolhas por clique.
const POINTS_RADIUS: i32 = 3;
const BUBBLES_PER_CLICK: usize = 6;
/// Tempo de cada quadro dos papéis que se mexem, em segundos.
const FRAME_S: f32 = 0.25;
/// No máximo tantas áreas se mexendo ao mesmo tempo (as mais novas ficam).
const MAX_MOVING: usize = 16;
/// Figuras do preenchimento surpresa (desenho, máscara): pegadas e manchas,
/// rastro de passarinho, patas e passos de dança. Desenhadas a partir de
/// (21, 106) da tela, recortadas no quadro.
const SURPRISES: [(u16, u16); 4] = [
    (0x122f, 0x1231),
    (0x1233, 0x1235),
    (0x123b, 0x123d),
    (0x122b, 0x122d),
];
const SURPRISE_POS: (i32, i32) = (21, 106);
/// Duração do aparecimento da surpresa, do canto de baixo à esquerda para o
/// de cima à direita (como o rastro anda).
const REVEAL_S: f32 = 1.2;

/// Estado da lata que não cabe numa opção: papéis, coluna, animação e surpresa.
pub struct Bucket {
    wallpapers: Vec<Wallpaper>,
    /// Papel escolhido; `None` = "Normal" (cor e padrão).
    chosen: Option<usize>,
    /// Cor e padrão quando o papel foi escolhido: escolher outra cor ou
    /// padrão nas tiras volta ao "Normal".
    chosen_ink: (usize, usize),
    open: bool,
    /// Primeiro papel visível na coluna.
    top: usize,
    scroll_sound: usize,
    moving: Vec<Moving>,
    anim_t: f32,
    /// Surpresa na tela: qual e há quanto tempo.
    surprise: Option<(usize, f32)>,
    next_surprise: usize,
    rng: XorShift,
}

impl Bucket {
    pub fn new(assets: &Assets, pal: &[Rgb]) -> Bucket {
        Bucket {
            wallpapers: load_wallpapers(assets, pal),
            chosen: None,
            chosen_ink: (0, 0),
            open: false,
            top: 0,
            scroll_sound: 0,
            moving: Vec::new(),
            anim_t: 0.0,
            surprise: None,
            next_surprise: 0,
            rng: XorShift::from_clock(),
        }
    }

    pub fn chosen(&self) -> Option<&Wallpaper> {
        self.chosen.and_then(|k| self.wallpapers.get(k))
    }

    fn max_top(&self) -> usize {
        self.wallpapers.len().saturating_sub(VISIBLE)
    }
}

fn cell_rect(k: usize) -> (i32, i32, i32, i32) {
    (CELL_X, FIRST_Y + CELL_STEP * k as i32, CELL_W, CELL_W)
}

/// O que há na coluna sob `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnHit {
    Normal,
    Cell(usize),
    Up,
    Down,
    Frame,
}

fn column_hit(x: i32, y: i32) -> Option<ColumnHit> {
    if !inside(COLUMN, x, y) {
        return None;
    }
    let arrow = |(_, ax, ay): (u16, i32, i32)| inside((ax, ay, ARROW_SIZE, ARROW_SIZE), x, y);
    Some(if inside((CELL_X, NORMAL_Y, CELL_W, CELL_W), x, y) {
        ColumnHit::Normal
    } else if let Some(k) = (0..VISIBLE).find(|&k| inside(cell_rect(k), x, y)) {
        ColumnHit::Cell(k)
    } else if arrow(ARROW_UP) {
        ColumnHit::Up
    } else if arrow(ARROW_DOWN) {
        ColumnHit::Down
    } else {
        ColumnHit::Frame
    })
}

/// Na amostra, o ladrilho de 32 encolhe para 30 pulando as colunas 8 e 24 e
/// as linhas 7 e 23 (ajustado à captura do original: 98,5% dos pixels iguais).
/// Os números são os da amostra a partir dos quais se pula uma a mais.
const SHRINK_COLS: (i32, i32) = (8, 23);
const SHRINK_ROWS: (i32, i32) = (7, 22);

fn shrink(v: i32, (a, b): (i32, i32)) -> i32 {
    v + (v >= a) as i32 + (v >= b) as i32
}

/// O pixel `(x, y)` da amostra (lado `n`) fica de fora no canto arredondado?
fn rounded_out(x: i32, y: i32, n: i32) -> bool {
    let edge = |v: i32| v.min(n - 1 - v);
    matches!((edge(x), edge(y)), (0, 0) | (0, 1) | (1, 0))
}

impl Paint {
    /// Clique da lata fora das opções: rolo e coluna. Devolve `true` se o
    /// clique foi usado.
    pub(super) fn bucket_click(
        &mut self,
        x: i32,
        y: i32,
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) -> bool {
        if self.tool != Tool::Bucket {
            return false;
        }
        if self.bucket.open {
            if let Some(hit) = column_hit(x, y) {
                self.bucket_column_click(hit, fx);
                return true;
            }
            // qualquer outro clique fecha a coluna; no quadro, só fecha
            self.bucket.open = false;
            return inside(ROLL, x, y) || inside(canvas_rect(), x, y);
        }
        if inside(ROLL, x, y) {
            self.bucket.open = true;
            fx.push(sfx::play(SND_OPEN));
            self.open_help(WHAT_BUCKET, WHAT_ROLL, assets);
            return true;
        }
        false
    }

    fn bucket_column_click(&mut self, hit: ColumnHit, fx: &mut Vec<Effect>) {
        let b = &mut self.bucket;
        match hit {
            ColumnHit::Normal => {
                b.chosen = None;
                b.open = false;
                fx.push(sfx::play(SND_PICK));
            }
            ColumnHit::Cell(k) => {
                if b.top + k < b.wallpapers.len() {
                    b.chosen = Some(b.top + k);
                    b.chosen_ink = (self.color, self.pattern);
                    b.open = false;
                    fx.push(sfx::play(SND_PICK));
                }
            }
            ColumnHit::Up | ColumnHit::Down => {
                let top = if hit == ColumnHit::Up {
                    b.top.saturating_sub(1)
                } else {
                    (b.top + 1).min(b.max_top())
                };
                if top != b.top {
                    b.top = top;
                    fx.push(sfx::play(SND_ZOOM_SCROLL[b.scroll_sound]));
                    b.scroll_sound = (b.scroll_sound + 1) % SND_ZOOM_SCROLL.len();
                }
            }
            ColumnHit::Frame => {}
        }
    }

    /// Despeja a lata em `(cx, cy)` do quadro com a opção escolhida.
    pub(super) fn pour_bucket(&mut self, cx: i32, cy: i32) {
        let option = self.bucket_option;
        if option == SURPRISE {
            let b = &mut self.bucket;
            b.surprise = Some((b.next_surprise, 0.0));
            b.next_surprise = (b.next_surprise + 1) % SURPRISES.len();
            return;
        }
        let ink = self.ink();
        let chosen = self.bucket.chosen;
        let wallpaper = self.bucket.chosen().cloned();
        let pour = match &wallpaper {
            Some(w) => Pour::Wallpaper(w),
            None => Pour::Ink(ink),
        };
        // dentro de um adesivo ou palavra, as opções de preencher pintam o
        // próprio objeto (como no original)
        if matches!(option, NORMAL | SWAP_COLORS | OUTLINE)
            && let Some(id) = self.objects.hit(cx, cy)
        {
            self.history.record_all(&self.canvas, &self.objects);
            let Some(o) = self.objects.get_mut(id) else {
                return;
            };
            let (ox, oy) = (cx - o.x, cy - o.y);
            let changed = match option {
                NORMAL => bucket_fx::object_fill(o, ox, oy, pour),
                SWAP_COLORS => bucket_fx::object_swap_color(o, ox, oy, pour),
                _ => bucket_fx::object_outline(o, OUTLINE_W, pour),
            };
            if changed {
                self.keep_user_sticker(id);
                self.changed = true;
            }
            return;
        }
        let seen = self.composed();
        let before = self.canvas.px.clone();
        let region = bucket_fx::region(&self.canvas, &seen.px, cx, cy);
        let c = &mut self.canvas;
        match (option, region) {
            (NORMAL, Some(r)) => {
                bucket_fx::fill(c, &r, pour);
                if let (Some(k), Some(w)) = (chosen, &wallpaper)
                    && w.animated()
                {
                    let moving = &mut self.bucket.moving;
                    moving.push(Moving::new(k, &r));
                    if moving.len() > MAX_MOVING {
                        moving.remove(0);
                    }
                }
            }
            (SWAP_COLORS, _) => {
                if let Some(target) = c.get(cx, cy) {
                    bucket_fx::swap_color(c, target, pour);
                }
            }
            (SWAP_PATTERNS, _) => {
                bucket_fx::swap_pattern(c, cx, cy, &self.patterns, pour);
            }
            (OUTLINE, Some(r)) => {
                bucket_fx::outline(c, &r, OUTLINE_W, pour);
            }
            (BLEND, Some(r)) => {
                let to = seen.get(cx, cy).unwrap_or(WHITE_INDEX);
                bucket_fx::blend(c, &r, (cx, cy), ink.color, to);
            }
            (POINTS, Some(r)) => {
                bucket_fx::pointillize(c, &r, &mut self.bucket.rng, POINTS_RADIUS);
            }
            (POP, _) => {
                bucket_fx::pop(c);
            }
            (BUBBLES, _) => {
                bucket_fx::bubbles(c, (cx, cy), &mut self.bucket.rng, BUBBLES_PER_CLICK, pour);
            }
            (SPIN, _) => {
                bucket_fx::spin(c, &self.pal);
            }
            _ => {}
        }
        if self.canvas.px != before {
            self.history.record_previous(before);
            self.changed = true;
        }
    }

    /// O adesivo `id` mudou: passa a ser um adesivo do usuário, gravado ao lado.
    fn keep_user_sticker(&mut self, id: u32) {
        let Some(o) = self.objects.get_mut(id) else {
            return;
        };
        let name = format!(
            "adesivo-{}-{id}.png",
            crate::paint::store::timestamp(super::now_secs())
        );
        if let Some(dir) = &self.drawings
            && std::fs::create_dir_all(dir).is_ok()
            && crate::paint::store::save_stamp_png(&o.image, &self.pal, &dir.join(&name)).is_ok()
        {
            o.source = Source::Png(name);
        }
    }

    /// Relógio da lata: papéis que se mexem, surpresa e coluna.
    pub(super) fn bucket_tick(&mut self, dt: f32) {
        let b = &mut self.bucket;
        if self.tool != Tool::Bucket {
            b.open = false;
        }
        // a surpresa some quando se usa outra ferramenta ou outra opção
        if self.tool != Tool::Bucket || self.bucket_option != SURPRISE {
            b.surprise = None;
        }
        if b.chosen.is_some() && b.chosen_ink != (self.color, self.pattern) {
            b.chosen = None;
        }
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        if let Some((_, t)) = b.surprise.as_mut() {
            *t += dt;
        }
        if b.moving.is_empty() {
            b.anim_t = 0.0;
            return;
        }
        b.anim_t += dt;
        while b.anim_t >= FRAME_S {
            b.anim_t -= FRAME_S;
            let (wallpapers, canvas) = (&b.wallpapers, &mut self.canvas);
            b.moving.retain_mut(|m| {
                wallpapers
                    .get(m.wallpaper)
                    .is_some_and(|w| m.step(canvas, w))
            });
        }
    }

    /// Faixa da lata: rolo com a amostra, moldura verde e as dez opções.
    pub(super) fn draw_bucket_strip(&self, f: &mut Frame, a: &Assets, buttons: &[u16]) {
        f.blit_masked(&a.image(&pic(0x1121)), &a.image(&pic(0x1123)), 92, 56);
        self.draw_roll(f, a);
        for (k, id) in buttons.iter().enumerate() {
            let r = super::option_rect(113, k);
            if k == NORMAL {
                draw_normal_icon(f, a, r.0, r.1);
            } else {
                f.blit(&a.image(&pic(*id)), r.0, r.1);
            }
        }
    }

    fn draw_roll(&self, f: &mut Frame, a: &Assets) {
        let (id, rx, ry) = ROLL_PIC;
        let img = a.image(&pic(id));
        let (w, h) = (img.width as i32, img.height as i32);
        // o branco ligado à borda da figura é transparente (fundo da faixa)
        let white = |x: i32, y: i32| {
            let i = ((y * w + x) * 4) as usize;
            img.rgba[i..i + 3] == WHITE
        };
        let mut clear = vec![false; (w * h) as usize];
        let mut stack: Vec<(i32, i32)> = (0..w)
            .flat_map(|x| [(x, 0), (x, h - 1)])
            .chain((0..h).flat_map(|y| [(0, y), (w - 1, y)]))
            .filter(|&(x, y)| white(x, y))
            .collect();
        for &(x, y) in &stack {
            clear[(y * w + x) as usize] = true;
        }
        while let Some((x, y)) = stack.pop() {
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    let i = (ny * w + nx) as usize;
                    if !clear[i] && white(nx, ny) {
                        clear[i] = true;
                        stack.push((nx, ny));
                    }
                }
            }
        }
        for y in 0..h {
            for x in 0..w {
                if !clear[(y * w + x) as usize] {
                    let i = ((y * w + x) * 4) as usize;
                    let p = &img.rgba[i..i + 3];
                    f.put(rx + x, ry + y, [p[0], p[1], p[2]]);
                }
            }
        }
        // amostra: a tinta, ou o papel escolhido (ladrilho 32×32 encolhido)
        let (sx, sy, n) = SWATCH;
        let ink = self.ink();
        let chosen = self.bucket.chosen();
        for y in 0..n {
            for x in 0..n {
                if rounded_out(x, y, n) {
                    continue;
                }
                let v = match chosen {
                    Some(w) => {
                        let (tx, ty) = (shrink(x, SHRINK_COLS), shrink(y, SHRINK_ROWS));
                        w.frames[0][ty as usize * TILE + tx as usize]
                    }
                    None => ink.index_at(sx + x, sy + y),
                };
                f.put(sx + x, sy + y, self.rgb(v));
            }
        }
    }

    /// Por cima do quadro: a surpresa e a coluna de papéis de parede.
    pub(super) fn draw_bucket_overlay(&self, f: &mut Frame, a: &Assets) {
        if self.tool != Tool::Bucket {
            return;
        }
        if let Some((k, t)) = self.bucket.surprise {
            draw_surprise(f, a, k, t);
        }
        if self.bucket.open {
            self.draw_column(f, a);
        }
    }

    fn draw_column(&self, f: &mut Frame, a: &Assets) {
        let (x0, y0, w, h) = COLUMN;
        f.rect(x0, y0, w, h, BLACK);
        f.rect(CELL_X, NORMAL_Y, CELL_W, CELL_W, WHITE);
        if let Some(font) = a.font(8) {
            let label = "Normal";
            let width: i32 = font.glyphs(label).map(|g| g.width as i32).sum();
            f.text_font(
                &font,
                CELL_X + (CELL_W - width) / 2 + 2,
                NORMAL_Y + 13,
                label,
                BLACK,
            );
        } else {
            f.text(CELL_X + 2, NORMAL_Y + 16, "Normal", BLACK);
        }
        for k in 0..VISIBLE {
            let (cx, cy, cw, ch) = cell_rect(k);
            f.rect(cx, cy, cw, ch, WHITE);
            let Some(wp) = self.bucket.wallpapers.get(self.bucket.top + k) else {
                continue;
            };
            for (i, &v) in wp.frames[0].iter().enumerate() {
                let (tx, ty) = ((i % TILE) as i32, (i / TILE) as i32);
                f.put(cx + THUMB_DX + tx, cy + THUMB_DY + ty, self.rgb(v));
            }
        }
        let arrows_y = FIRST_Y + CELL_STEP * VISIBLE as i32;
        f.rect(CELL_X, arrows_y, CELL_W, y0 + h - 1 - arrows_y, WHITE);
        for (id, ax, ay) in [ARROW_UP, ARROW_DOWN] {
            f.blit(&a.image(&pic(id)), ax, ay);
        }
    }
}

/// Ícone da opção "normal": no `.PIC` só há a versão de 1 bit (`1590`); o
/// original a desenha em preto sobre o fundo do ícone da mistura (`1597`),
/// com a faixa da tinta em xadrez lilás e preto (confere com a captura em
/// 1023 dos 1024 pixels).
const NORMAL_ICON: (u16, u16) = (0x1590, 0x1597);
const NORMAL_BAND: (i32, i32, i32, i32) = (5, 21, 28, 26);
const LILAC: [u8; 3] = [136, 136, 230];

fn draw_normal_icon(f: &mut Frame, a: &Assets, x0: i32, y0: i32) {
    let mono = a.image(&pic(NORMAL_ICON.0));
    let back = a.image(&pic(NORMAL_ICON.1));
    if (mono.width, mono.height) != (back.width, back.height) {
        f.blit(&mono, x0, y0);
        return;
    }
    let (bx0, by0, bx1, by1) = NORMAL_BAND;
    for y in 0..back.height as i32 {
        for x in 0..back.width as i32 {
            let i = ((y * back.width as i32 + x) * 4) as usize;
            let c = if mono.rgba[i..i + 3] == BLACK {
                BLACK
            } else if (bx0..=bx1).contains(&x) && (by0..=by1).contains(&y) {
                if (x + y) % 2 == 0 { LILAC } else { BLACK }
            } else {
                [back.rgba[i], back.rgba[i + 1], back.rgba[i + 2]]
            };
            f.put(x0 + x, y0 + y, c);
        }
    }
}

/// Índice do branco na paleta de 256 cores.
const WHITE_INDEX: u8 = crate::paint::WHITE;

fn draw_surprise(f: &mut Frame, a: &Assets, k: usize, t: f32) {
    let (img_id, mask_id) = SURPRISES[k % SURPRISES.len()];
    let img = a.image(&pic(img_id));
    let mask = a.image(&pic(mask_id));
    if (img.width, img.height) != (mask.width, mask.height) {
        return;
    }
    let (w, h) = (img.width as i32, img.height as i32);
    let reach = ((t / REVEAL_S).min(1.0) * (w + h) as f32) as i32;
    let r = canvas_rect();
    for y in 0..h {
        for x in 0..w {
            // aparece do canto de baixo à esquerda para o de cima à direita
            if x + (h - 1 - y) > reach {
                continue;
            }
            let i = ((y * w + x) * 4) as usize;
            let m = &mask.rgba[i..i + 3];
            if m[0] as u32 + m[1] as u32 + m[2] as u32 >= 384 {
                continue;
            }
            let (sx, sy) = (SURPRISE_POS.0 + x, SURPRISE_POS.1 + y);
            if inside(r, sx, sy) {
                let p = &img.rgba[i..i + 3];
                f.put(sx, sy, [p[0], p[1], p[2]]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{CANVAS_H, CANVAS_W};
    use crate::scene::{Input, MouseButton, Scene};
    use crate::scenes::testutil::empty_assets;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    const BUCKET_TOOL: (i32, i32) = (70, 30);
    const CANVAS_PT: (i32, i32) = (300, 300);

    fn option(k: usize) -> Input {
        let r = super::super::option_rect(113, k);
        click(r.0 + 16, r.1 + 16)
    }

    fn wallpaper(frames: &[u8]) -> Wallpaper {
        Wallpaper {
            name: "teste".into(),
            frames: frames.iter().map(|&v| vec![v; TILE * TILE]).collect(),
        }
    }

    /// Tela com a lata escolhida e dois papéis de teste (o 2º se mexe).
    fn bucket_scene() -> (tempfile::TempDir, Assets, Paint) {
        let (d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        p.bucket.wallpapers = vec![wallpaper(&[7]), wallpaper(&[11, 12])];
        p.update(&[click(BUCKET_TOOL.0, BUCKET_TOOL.1)], 0.0, &a);
        assert_eq!(p.tool(), Tool::Bucket);
        (d, a, p)
    }

    fn canvas_at(p: &Paint, x: i32, y: i32) -> u8 {
        p.canvas.px[(y as u32 * CANVAS_W + x as u32) as usize]
    }

    #[test]
    fn roll_opens_column_and_picks_wallpaper() {
        let (_d, a, mut p) = bucket_scene();
        let fx = p.update(&[click(20, 80)], 0.0, &a);
        assert!(p.bucket.open);
        assert!(fx.contains(&sfx::play(SND_OPEN)));
        // com a coluna aberta, o quadro não recebe tinta: só fecha
        let fx = p.update(&[click(CANVAS_PT.0, CANVAS_PT.1)], 0.0, &a);
        assert!(!p.bucket.open);
        assert!(!fx.contains(&sfx::play(SND_OPEN)));
        assert!(p.canvas.px.iter().all(|&v| v == WHITE_INDEX));
        p.update(&[click(70, 80)], 0.0, &a);
        let fx = p.update(&[click(50, 160)], 0.0, &a);
        assert!(fx.contains(&sfx::play(SND_PICK)));
        assert!(!p.bucket.open);
        assert_eq!(p.bucket.chosen, Some(0));
        p.update(&[click(CANVAS_PT.0, CANVAS_PT.1)], 0.0, &a);
        assert!(p.canvas.px.iter().all(|&v| v == 7));
        // "Normal" volta à cor e ao padrão
        p.update(&[click(20, 80), click(50, 120)], 0.0, &a);
        assert_eq!(p.bucket.chosen, None);
    }

    #[test]
    fn moving_wallpaper_animates_until_painted_over() {
        let (_d, a, mut p) = bucket_scene();
        p.update(&[click(20, 80), click(50, 200)], 0.0, &a);
        assert_eq!(p.bucket.chosen, Some(1));
        p.update(&[click(CANVAS_PT.0, CANVAS_PT.1)], 0.0, &a);
        assert_eq!(canvas_at(&p, 10, 10), 11);
        p.update(&[], FRAME_S + 0.01, &a);
        assert_eq!(canvas_at(&p, 10, 10), 12);
        p.update(&[], FRAME_S, &a);
        assert_eq!(canvas_at(&p, 10, 10), 11);
        // escolher outra cor na tira volta ao normal; despejar por cima para a área
        p.update(&[click(10, 115)], 0.0, &a);
        assert_eq!(p.bucket.chosen, None);
        p.update(&[click(CANVAS_PT.0, CANVAS_PT.1)], 0.0, &a);
        let painted = canvas_at(&p, 10, 10);
        p.update(&[], FRAME_S * 3.0, &a);
        assert_eq!(canvas_at(&p, 10, 10), painted);
        assert!(p.bucket.moving.is_empty());
    }

    #[test]
    fn column_scrolls_one_line_with_sound() {
        let (_d, a, mut p) = bucket_scene();
        p.bucket.wallpapers = (0..10).map(|k| wallpaper(&[k as u8 + 2])).collect();
        p.update(&[click(20, 80)], 0.0, &a);
        let fx = p.update(&[click(62, 434)], 0.0, &a);
        assert_eq!(p.bucket.top, 1);
        assert!(fx.contains(&sfx::play(SND_ZOOM_SCROLL[0])));
        p.update(&[click(62, 434), click(62, 434), click(62, 434)], 0.0, &a);
        assert_eq!(p.bucket.top, 3, "10 papéis, 7 à vista");
        p.update(&[click(44, 434)], 0.0, &a);
        assert_eq!(p.bucket.top, 2);
        assert!(p.bucket.open);
        p.update(&[click(50, 160)], 0.0, &a);
        assert_eq!(p.bucket.chosen, Some(2));
    }

    #[test]
    fn options_have_their_own_effects() {
        let (_d, a, mut p) = bucket_scene();
        // um traço preto no quadro branco
        for x in 50..60 {
            p.canvas.set(x, 40, crate::paint::BLACK);
        }
        let base = p.canvas.px.clone();
        // pop: o traço aparece em quatro lugares
        p.update(&[option(POP), click(CANVAS_PT.0, CANVAS_PT.1)], 0.0, &a);
        assert_eq!(canvas_at(&p, 26, 20), crate::paint::BLACK);
        assert_eq!(
            canvas_at(&p, 26 + CANVAS_W as i32 / 2 + 1, 20),
            crate::paint::BLACK
        );
        // seqüência: branco e preto mudam de cor
        p.canvas.px = base.clone();
        p.update(&[option(SPIN), click(CANVAS_PT.0, CANVAS_PT.1)], 0.0, &a);
        assert_ne!(canvas_at(&p, 0, 0), WHITE_INDEX);
        assert_ne!(canvas_at(&p, 55, 40), crate::paint::BLACK);
        // o ovo desfaz
        p.update(&[click(279, 30)], 0.0, &a);
        assert_eq!(p.canvas.px, base);
        // troca-cores: o preto vira a cor da tinta em todo o quadro
        p.update(&[click(10, 158)], 0.0, &a);
        p.update(
            &[option(SWAP_COLORS), click(ORIGIN_X + 55, ORIGIN_Y + 40)],
            0.0,
            &a,
        );
        let ink = p.ink().color;
        assert_ne!(ink, crate::paint::BLACK);
        assert_eq!(canvas_at(&p, 50, 40), ink);
        assert_eq!(canvas_at(&p, 0, 0), WHITE_INDEX);
    }

    const ORIGIN_X: i32 = crate::paint::ORIGIN.0;
    const ORIGIN_Y: i32 = crate::paint::ORIGIN.1;

    #[test]
    fn surprise_is_temporary() {
        let (_d, a, mut p) = bucket_scene();
        let base = p.canvas.px.clone();
        p.update(
            &[option(SURPRISE), click(CANVAS_PT.0, CANVAS_PT.1)],
            0.0,
            &a,
        );
        assert_eq!(p.bucket.surprise.map(|s| s.0), Some(0));
        assert_eq!(p.canvas.px, base);
        p.update(&[click(CANVAS_PT.0, CANVAS_PT.1)], 0.5, &a);
        assert_eq!(p.bucket.surprise.map(|s| s.0), Some(1));
        // outra ferramenta: a figura volta ao que era
        p.update(&[click(28, 30)], 0.0, &a);
        assert_eq!(p.bucket.surprise, None);
        assert_eq!(p.canvas.px, base);
        assert_eq!(p.canvas.px.len(), (CANVAS_W * CANVAS_H) as usize);
    }

    #[test]
    fn column_cells_match_capture() {
        // medidas da captura `tools_o_bucket_wp_open.png`
        assert_eq!(column_hit(50, 120), Some(ColumnHit::Normal));
        assert_eq!(column_hit(50, 160), Some(ColumnHit::Cell(0)));
        assert_eq!(column_hit(50, 405), Some(ColumnHit::Cell(6)));
        assert_eq!(column_hit(44, 434), Some(ColumnHit::Up));
        assert_eq!(column_hit(62, 434), Some(ColumnHit::Down));
        assert_eq!(column_hit(33, 300), Some(ColumnHit::Frame));
        assert_eq!(column_hit(80, 300), None);
        assert_eq!(column_hit(50, 450), None);
        // as casas não passam da divisa de baixo (y = 427)
        let (_, y, _, h) = cell_rect(VISIBLE - 1);
        assert_eq!(y + h, 427);
    }

    #[test]
    fn swatch_shrink_skips_two_lines() {
        let cols: Vec<i32> = (0..30).map(|v| shrink(v, SHRINK_COLS)).collect();
        assert_eq!(cols[0], 0);
        assert_eq!(cols[29], 31);
        let skipped: Vec<i32> = (0..32).filter(|v| !cols.contains(v)).collect();
        assert_eq!(skipped, [8, 24]);
        let rows: Vec<i32> = (0..30).map(|v| shrink(v, SHRINK_ROWS)).collect();
        let skipped: Vec<i32> = (0..32).filter(|v| !rows.contains(v)).collect();
        assert_eq!(skipped, [7, 23]);
    }

    #[test]
    fn swatch_corners_are_rounded() {
        assert!(rounded_out(0, 0, 30) && rounded_out(1, 0, 30) && rounded_out(0, 1, 30));
        assert!(rounded_out(29, 29, 30) && rounded_out(28, 29, 30));
        assert!(!rounded_out(1, 1, 30) && !rounded_out(2, 0, 30) && !rounded_out(15, 15, 30));
    }
}
