//! Transformadores da tela de pintura (`1045`; ajuda `WHAT 1006`): faixa de
//! duas páginas de ações sobre o objeto (adesivo ou palavra) sob o ponteiro —
//! mover, dimensionar, aumentar/diminuir segurando, empurrar/trazer, girar,
//! grudar/desgrudar com corrente e inverter. Ícones e posições medidos no
//! original (`scratch/orig_tr1..4.png`, `tools/wine/locate.py`).

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::paint::objects::MIN_SIZE;
use crate::paint::{Canvas, History, ORIGIN, Objects};

type Rect = (i32, i32, i32, i32);

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const RED: [u8; 3] = [250, 0, 57];
const GRAY: [u8; 3] = [100, 100, 100];

/// Tampa esquerda da faixa verde (19×46 em x 4, y 57).
pub const CAP_LEFT: u16 = 0x14db;
/// Ponta direita da faixa verde (29×48) e sua máscara.
pub const CAP_RIGHT: (u16, u16) = (0x14ef, 0x14f1);
const ARROW_UP: Rect = (504, 58, 16, 14);
const ARROW_DOWN: Rect = (504, 90, 16, 14);
/// Crescimento por quadro ao segurar Aumentar/Diminuir.
const GROW_STEP: f32 = 1.04;
/// Intervalo entre repetições ao segurar Empurrar/Trazer, em segundos.
const REPEAT: f32 = 0.25;
/// Comprimento de cada elo da corrente, em pixels.
const LINK_LEN: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Mover-modificar: arrastar move o objeto (e os grudados a ele).
    Move,
    /// Dimensionar por arraste: largura e altura seguem o ponteiro.
    DragSize,
    /// Dimensionar lateral: só a largura.
    SideSize,
    /// Dimensionar vertical: só a altura.
    VertSize,
    /// Aumentar: cresce enquanto o botão fica apertado.
    Grow,
    /// Diminuir: encolhe enquanto o botão fica apertado.
    Shrink,
    /// Empurrar para trás um nível (segurar continua).
    PushBack,
    /// Trazer para a frente um nível (segurar continua).
    BringFront,
    RotateLeft,
    RotateRight,
    /// Grudar ou desgrudar: dois cliques ligam; clique na corrente desliga.
    Link,
    FlipH,
    FlipV,
    /// Coloca-adesivos (cavalete de adesivos): fora desta fase.
    AddSticker,
}

/// Botão da faixa: ação, ícone colorido (escolhido) e mono (normal).
pub struct Button {
    pub op: Op,
    pub color: u16,
    pub mono: u16,
}

const fn b(op: Op, color: u16, mono: u16) -> Button {
    Button { op, color, mono }
}

pub const PAGE1: [Button; 8] = [
    b(Op::Move, 0x1470, 0x1471),
    b(Op::DragSize, 0x149d, 0x14a0),
    b(Op::SideSize, 0x14a1, 0x14a4),
    b(Op::VertSize, 0x14a5, 0x14a8),
    b(Op::Grow, 0x14b1, 0x14b4),
    b(Op::Shrink, 0x14b5, 0x14b8),
    b(Op::PushBack, 0x14b9, 0x14bc),
    b(Op::BringFront, 0x14bd, 0x14c0),
];

pub const PAGE2: [Button; 7] = [
    b(Op::Move, 0x1470, 0x1471),
    b(Op::RotateLeft, 0x14c1, 0x14c4),
    b(Op::RotateRight, 0x14c5, 0x14c8),
    b(Op::Link, 0x14c9, 0x14cc),
    b(Op::FlipH, 0x14cd, 0x14d0),
    b(Op::FlipV, 0x14d1, 0x14d4),
    b(Op::AddSticker, 0x14d9, 0x14da),
];

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Artist, id)
}

/// Retângulo do `k`-ésimo botão da faixa (32×32 em x = 25 + 37k, y = 63).
pub fn option_rect(k: usize) -> Rect {
    (25 + 37 * k as i32, 63, 32, 32)
}

/// Arrasto em curso sobre um objeto.
struct Drag {
    id: u32,
    /// Ponto do quadro onde o arrasto começou e retângulo do objeto então.
    start: (i32, i32),
    rect: Rect,
    last: (i32, i32),
}

/// Botão apertado sobre um objeto (Aumentar, Diminuir, Empurrar, Trazer).
struct Hold {
    id: u32,
    /// Tempo desde a última repetição.
    t: f32,
}

pub struct Transformers {
    page: u8,
    op: Op,
    drag: Option<Drag>,
    hold: Option<Hold>,
    /// Primeiro objeto clicado com Grudar, à espera do segundo.
    pending_link: Option<u32>,
}

impl Default for Transformers {
    fn default() -> Self {
        Self::new()
    }
}

impl Transformers {
    pub fn new() -> Transformers {
        Transformers {
            page: 1,
            op: Op::Move,
            drag: None,
            hold: None,
            pending_link: None,
        }
    }

    pub fn page(&self) -> u8 {
        self.page
    }

    pub fn op(&self) -> Op {
        self.op
    }

    pub fn pending_link(&self) -> Option<u32> {
        self.pending_link
    }

    fn buttons(&self) -> &'static [Button] {
        if self.page == 1 { &PAGE1 } else { &PAGE2 }
    }

    /// Botão escolhido na página atual, para o realce vermelho.
    pub fn selected(&self) -> Option<usize> {
        self.buttons().iter().position(|b| b.op == self.op)
    }

    /// Índice do registro de ajuda (`WHAT 1006`) da ação escolhida.
    pub fn help_record(&self) -> usize {
        match self.op {
            Op::Move => 1,
            Op::DragSize => 8,
            Op::SideSize => 11,
            Op::VertSize => 12,
            Op::Grow => 5,
            Op::Shrink => 6,
            Op::PushBack => 18,
            Op::BringFront => 19,
            Op::RotateLeft => 14,
            Op::RotateRight => 13,
            Op::Link => 17,
            Op::FlipH => 15,
            Op::FlipV => 16,
            Op::AddSticker => 20,
        }
    }

    /// Solta tudo (troca de ferramenta, saída).
    pub fn reset(&mut self) {
        self.drag = None;
        self.hold = None;
        self.pending_link = None;
    }

    /// Clique na faixa de opções: `Some(aviso)` se foi tratado aqui.
    pub fn click_strip(&mut self, x: i32, y: i32) -> Option<Option<&'static str>> {
        if inside(ARROW_UP, x, y) || inside(ARROW_DOWN, x, y) {
            self.page = 3 - self.page;
            self.pending_link = None;
            return Some(None);
        }
        let k = (0..self.buttons().len()).find(|&k| inside(option_rect(k), x, y))?;
        self.op = self.buttons()[k].op;
        self.pending_link = None;
        if self.op == Op::AddSticker {
            return Some(Some("coloca-adesivos: ainda não disponível"));
        }
        Some(None)
    }

    /// Botão apertado em `(cx, cy)` do quadro. Verdadeiro se mudou algo.
    pub fn press(
        &mut self,
        cx: i32,
        cy: i32,
        objs: &mut Objects,
        canvas: &Canvas,
        history: &mut History,
    ) -> bool {
        if self.op == Op::Link
            && let Some(holder) = objs.chain_hit(cx, cy)
        {
            history.record_all(canvas, objs);
            objs.unlink(holder);
            self.pending_link = None;
            return true;
        }
        let Some(id) = objs.hit(cx, cy) else {
            self.pending_link = None;
            return false;
        };
        match self.op {
            Op::Move | Op::DragSize | Op::SideSize | Op::VertSize => {
                history.record_all(canvas, objs);
                let rect = objs.get(id).map(|o| o.rect()).unwrap_or_default();
                self.drag = Some(Drag {
                    id,
                    start: (cx, cy),
                    rect,
                    last: (cx, cy),
                });
                false
            }
            Op::Grow | Op::Shrink | Op::PushBack | Op::BringFront => {
                history.record_all(canvas, objs);
                self.hold = Some(Hold { id, t: 0.0 });
                self.step(id, objs)
            }
            Op::RotateLeft | Op::RotateRight => {
                history.record_all(canvas, objs);
                if let Some(o) = objs.get_mut(id) {
                    o.turn(self.op == Op::RotateRight);
                }
                true
            }
            Op::FlipH | Op::FlipV => {
                history.record_all(canvas, objs);
                if let Some(o) = objs.get_mut(id) {
                    if self.op == Op::FlipH {
                        o.flip_h = !o.flip_h;
                    } else {
                        o.flip_v = !o.flip_v;
                    }
                }
                true
            }
            Op::Link => match self.pending_link.take() {
                Some(first) if first != id => {
                    history.record_all(canvas, objs);
                    objs.link(first, id)
                }
                _ => {
                    self.pending_link = Some(id);
                    false
                }
            },
            Op::AddSticker => false,
        }
    }

    /// Um passo das ações de segurar.
    fn step(&self, id: u32, objs: &mut Objects) -> bool {
        match self.op {
            Op::Grow | Op::Shrink => {
                let Some(o) = objs.get_mut(id) else {
                    return false;
                };
                let f = if self.op == Op::Grow {
                    GROW_STEP
                } else {
                    1.0 / GROW_STEP
                };
                let w = ((o.w as f32 * f).round() as i32).max(MIN_SIZE);
                let h = ((o.h as f32 * f).round() as i32).max(MIN_SIZE);
                // garante ao menos um pixel de mudança enquanto couber
                let w = if w == o.w && self.op == Op::Grow {
                    w + 1
                } else {
                    w
                };
                let h = if h == o.h && self.op == Op::Grow {
                    h + 1
                } else {
                    h
                };
                let w = if w == o.w && self.op == Op::Shrink {
                    (w - 1).max(MIN_SIZE)
                } else {
                    w
                };
                let h = if h == o.h && self.op == Op::Shrink {
                    (h - 1).max(MIN_SIZE)
                } else {
                    h
                };
                let changed = (w, h) != (o.w, o.h);
                o.resize_centered(w, h);
                changed
            }
            Op::PushBack => objs.push_back(id),
            Op::BringFront => objs.bring_front(id),
            _ => false,
        }
    }

    /// Ponteiro movido com o botão apertado. Verdadeiro se mudou algo.
    pub fn drag(&mut self, cx: i32, cy: i32, objs: &mut Objects) -> bool {
        let Some(d) = self.drag.as_mut() else {
            return false;
        };
        if (cx, cy) == d.last {
            return false;
        }
        let (dx, dy) = (cx - d.start.0, cy - d.start.1);
        match self.op {
            Op::Move => {
                objs.move_by(d.id, cx - d.last.0, cy - d.last.1);
            }
            Op::DragSize | Op::SideSize | Op::VertSize => {
                let Some(o) = objs.get_mut(d.id) else {
                    return false;
                };
                let (x0, y0, w0, h0) = d.rect;
                if self.op != Op::VertSize {
                    o.w = (w0 + dx).max(MIN_SIZE);
                }
                if self.op != Op::SideSize {
                    o.h = (h0 + dy).max(MIN_SIZE);
                }
                o.x = x0;
                o.y = y0;
            }
            _ => {}
        }
        d.last = (cx, cy);
        true
    }

    pub fn release(&mut self) {
        self.drag = None;
        self.hold = None;
    }

    /// Repete a ação de segurar. Verdadeiro se mudou algo.
    pub fn update(&mut self, dt: f32, objs: &mut Objects) -> bool {
        let Some(h) = self.hold.as_mut() else {
            return false;
        };
        let id = h.id;
        match self.op {
            Op::Grow | Op::Shrink => self.step(id, objs),
            Op::PushBack | Op::BringFront => {
                h.t += dt.max(0.0);
                if h.t >= REPEAT {
                    h.t = 0.0;
                    self.step(id, objs)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Faixa de opções: tampas verdes, botões, setas e número da página.
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        f.blit(&a.image(&pic(CAP_LEFT)), 4, 57);
        let n = self.buttons().len();
        f.blit_masked(
            &a.image(&pic(CAP_RIGHT.0)),
            &a.image(&pic(CAP_RIGHT.1)),
            25 + 37 * n as i32 - 2,
            56,
        );
        for (k, btn) in self.buttons().iter().enumerate() {
            let r = option_rect(k);
            let id = if btn.op == self.op {
                btn.color
            } else {
                btn.mono
            };
            f.blit(&a.image(&pic(id)), r.0, r.1);
        }
        if let Some(k) = self.selected() {
            let r = option_rect(k);
            f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, RED);
        }
        f.blit_masked(
            &a.image(&pic(0x03b3)),
            &a.image(&pic(0x03b5)),
            ARROW_UP.0,
            ARROW_UP.1,
        );
        f.blit_masked(
            &a.image(&pic(0x03b7)),
            &a.image(&pic(0x03b9)),
            ARROW_DOWN.0,
            ARROW_DOWN.1,
        );
        f.rect(504, 75, 16, 12, BLACK);
        f.rect(505, 76, 14, 10, WHITE);
        f.text(508, 77, &self.page.to_string(), BLACK);
    }

    /// Correntes entre objetos grudados e a marca do primeiro objeto a grudar.
    pub fn draw_overlay(&self, f: &mut Frame, objs: &Objects) {
        for (a, b, _) in objs.chains() {
            draw_chain(f, a, b);
        }
        if let Some(o) = self.pending_link.and_then(|id| objs.get(id)) {
            let (cx, cy) = o.center();
            f.frame_rect(ORIGIN.0 + cx - 4, ORIGIN.1 + cy - 4, 9, 9, 1, GRAY);
        }
    }
}

/// Corrente de elos entre dois pontos do quadro.
pub fn draw_chain(f: &mut Frame, a: (i32, i32), b: (i32, i32)) {
    let (ax, ay) = (a.0 as f32, a.1 as f32);
    let (dx, dy) = (b.0 as f32 - ax, b.1 as f32 - ay);
    let len = (dx * dx + dy * dy).sqrt();
    let n = (len / LINK_LEN).ceil().max(1.0) as i32;
    for k in 0..=n {
        let t = k as f32 / n as f32;
        let (x, y) = (
            ORIGIN.0 + (ax + dx * t).round() as i32,
            ORIGIN.1 + (ay + dy * t).round() as i32,
        );
        // elo: anel de 5 px, branco com borda preta
        for oy in -2..=2 {
            for ox in -2..=2 {
                let d = ox * ox + oy * oy;
                if d <= 5 {
                    f.put(x + ox, y + oy, if d <= 1 { WHITE } else { BLACK });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::stamp::Stamp;
    use crate::paint::{CANVAS_H, CANVAS_W, Source};
    use std::rc::Rc;

    fn stamp() -> Rc<Stamp> {
        Rc::new(Stamp {
            w: 4,
            h: 2,
            px: vec![Some(3); 8],
        })
    }

    fn setup() -> (Transformers, Objects, Canvas, History) {
        let mut objs = Objects::default();
        objs.place(stamp(), 100, 100, Source::Roll { roll: 1, item: 0 });
        (
            Transformers::new(),
            objs,
            Canvas::new(CANVAS_W, CANVAS_H),
            History::default(),
        )
    }

    fn pick(t: &mut Transformers, op: Op) {
        let page = if PAGE1.iter().any(|b| b.op == op) {
            1
        } else {
            2
        };
        while t.page != page {
            t.click_strip(510, 95);
        }
        let k = t.buttons().iter().position(|b| b.op == op).unwrap();
        let r = option_rect(k);
        t.click_strip(r.0 + 5, r.1 + 5);
        assert_eq!(t.op(), op);
    }

    #[test]
    fn pages_wrap_and_buttons_select() {
        let mut t = Transformers::new();
        assert_eq!(t.page(), 1);
        assert_eq!(t.click_strip(510, 95), Some(None));
        assert_eq!(t.page(), 2);
        assert_eq!(t.click_strip(510, 60), Some(None));
        assert_eq!(t.page(), 1);
        assert_eq!(t.click_strip(300, 300), None);
        pick(&mut t, Op::Shrink);
        assert_eq!(t.selected(), Some(5));
        pick(&mut t, Op::AddSticker);
        assert!(matches!(t.click_strip(25 + 37 * 6 + 5, 70), Some(Some(_))));
        assert_eq!(t.help_record(), 20);
    }

    #[test]
    fn move_drags_object_and_egg_undoes() {
        let (mut t, mut objs, c, mut h) = setup();
        assert!(!t.press(101, 100, &mut objs, &c, &mut h));
        assert!(t.drag(121, 110, &mut objs));
        assert!(!t.drag(121, 110, &mut objs));
        t.release();
        assert_eq!(objs.items[0].rect(), (120, 110, 4, 2));
        assert!(h.toggle(&mut Canvas::new(CANVAS_W, CANVAS_H), &mut objs));
        assert_eq!(objs.items[0].rect(), (100, 100, 4, 2));
        // clique fora de qualquer objeto não arrasta nada
        assert!(!t.press(0, 0, &mut objs, &c, &mut h));
        assert!(!t.drag(10, 10, &mut objs));
    }

    #[test]
    fn sizers_change_the_right_dimensions() {
        let (mut t, mut objs, c, mut h) = setup();
        pick(&mut t, Op::DragSize);
        t.press(101, 100, &mut objs, &c, &mut h);
        t.drag(111, 105, &mut objs);
        t.release();
        assert_eq!(objs.items[0].rect(), (100, 100, 14, 7));
        pick(&mut t, Op::SideSize);
        t.press(101, 100, &mut objs, &c, &mut h);
        t.drag(91, 150, &mut objs);
        t.release();
        assert_eq!(objs.items[0].rect(), (100, 100, 4, 7));
        pick(&mut t, Op::VertSize);
        t.press(101, 100, &mut objs, &c, &mut h);
        t.drag(50, 90, &mut objs);
        t.release();
        assert_eq!(objs.items[0].rect(), (100, 100, 4, MIN_SIZE));
    }

    #[test]
    fn grow_and_shrink_while_held() {
        let (mut t, mut objs, c, mut h) = setup();
        objs.items[0].w = 40;
        objs.items[0].h = 20;
        pick(&mut t, Op::Grow);
        assert!(t.press(101, 101, &mut objs, &c, &mut h));
        for _ in 0..10 {
            t.update(0.016, &mut objs);
        }
        t.release();
        assert!(objs.items[0].w > 40 && objs.items[0].h > 20);
        assert!(!t.update(0.016, &mut objs));
        pick(&mut t, Op::Shrink);
        let (cx, cy) = objs.items[0].center();
        assert!(t.press(cx, cy, &mut objs, &c, &mut h));
        for _ in 0..200 {
            t.update(0.016, &mut objs);
        }
        t.release();
        assert_eq!((objs.items[0].w, objs.items[0].h), (MIN_SIZE, MIN_SIZE));
    }

    #[test]
    fn push_back_and_bring_front_repeat_while_held() {
        let (mut t, mut objs, c, mut h) = setup();
        objs.place(stamp(), 100, 100, Source::Roll { roll: 1, item: 1 });
        objs.place(stamp(), 100, 100, Source::Roll { roll: 1, item: 2 });
        let top = objs.items[2].id;
        pick(&mut t, Op::PushBack);
        assert!(t.press(101, 101, &mut objs, &c, &mut h));
        assert_eq!(objs.items[1].id, top);
        assert!(!t.update(0.1, &mut objs));
        assert!(t.update(0.2, &mut objs));
        assert_eq!(objs.items[0].id, top);
        t.release();
        pick(&mut t, Op::BringFront);
        // o de baixo não está sob o ponteiro; o hit acha o de cima
        let top_now = objs.items[2].id;
        assert!(!t.press(101, 101, &mut objs, &c, &mut h));
        assert_eq!(objs.items[2].id, top_now);
    }

    #[test]
    fn rotate_four_times_and_flip_twice_restore() {
        let (mut t, mut objs, c, mut h) = setup();
        let before = objs.items[0].clone();
        pick(&mut t, Op::RotateRight);
        for _ in 0..4 {
            let (cx, cy) = objs.items[0].center();
            assert!(t.press(cx, cy, &mut objs, &c, &mut h));
        }
        assert_eq!(objs.items[0], before);
        pick(&mut t, Op::RotateLeft);
        let (cx, cy) = objs.items[0].center();
        t.press(cx, cy, &mut objs, &c, &mut h);
        assert_eq!(objs.items[0].turns, 3);
        assert_eq!((objs.items[0].w, objs.items[0].h), (2, 4));
        t.press(cx, cy, &mut objs, &c, &mut h);
        t.press(cx, cy, &mut objs, &c, &mut h);
        t.press(cx, cy, &mut objs, &c, &mut h);
        assert_eq!(objs.items[0], before);
        for op in [Op::FlipH, Op::FlipV] {
            pick(&mut t, op);
            t.press(101, 101, &mut objs, &c, &mut h);
            assert!(objs.items[0].flip_h || objs.items[0].flip_v);
            t.press(101, 101, &mut objs, &c, &mut h);
            assert_eq!(objs.items[0], before);
        }
    }

    #[test]
    fn link_two_objects_then_move_together_and_unlink_on_chain() {
        let (mut t, mut objs, c, mut h) = setup();
        let a = objs.items[0].id;
        let b = objs.place(stamp(), 200, 100, Source::Roll { roll: 1, item: 1 });
        pick(&mut t, Op::Link);
        assert!(!t.press(101, 101, &mut objs, &c, &mut h));
        assert_eq!(t.pending_link(), Some(a));
        // clicar fora desiste
        assert!(!t.press(0, 0, &mut objs, &c, &mut h));
        assert_eq!(t.pending_link(), None);
        t.press(101, 101, &mut objs, &c, &mut h);
        assert!(t.press(201, 101, &mut objs, &c, &mut h));
        assert_eq!(objs.group(a).len(), 2);
        pick(&mut t, Op::Move);
        t.press(101, 101, &mut objs, &c, &mut h);
        t.drag(111, 121, &mut objs);
        t.release();
        assert_eq!(objs.get(b).unwrap().rect(), (210, 120, 4, 2));
        // clique na corrente (meio do caminho entre os centros) desgruda
        pick(&mut t, Op::Link);
        let (ca, cb) = (objs.get(a).unwrap().center(), objs.get(b).unwrap().center());
        let mid = ((ca.0 + cb.0) / 2, (ca.1 + cb.1) / 2);
        assert!(t.press(mid.0, mid.1, &mut objs, &c, &mut h));
        assert_eq!(objs.chains().len(), 0);
        let mut f = Frame::new(640, 480);
        objs.link(a, b);
        t.pending_link = Some(a);
        t.draw_overlay(&mut f, &objs);
    }

    #[test]
    fn strip_draws_with_empty_assets() {
        let (_d, a) = crate::scenes::testutil::empty_assets();
        let mut t = Transformers::new();
        let mut f = Frame::new(640, 480);
        t.draw_strip(&mut f, &a);
        t.click_strip(510, 95);
        t.draw_strip(&mut f, &a);
        assert_eq!(f.pixel(505, 77)[..3], [255, 255, 255]);
    }
}
