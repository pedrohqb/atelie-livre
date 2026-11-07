//! Aspirador pelo caminho do mouse: no Aspira-tinta, no Aspira-cópias e no
//! Aspira-cores o bocal (3 × 3 pixels, medido no original) segue o arrasto
//! como um pincel; com Shift, continua o retângulo de canto a canto (a "tecla
//! de atalho" dos balões `WHAT 1008[5]` e `[8]`). Ao soltar, o que foi
//! aspirado (só os pixels por onde o bocal passou) vai para o Aspirador, para
//! o Lançador cuspir, e o McZee abre "Onde foi parar?" (`1008[16]`) ou "Iau!"
//! (`1008[15]`), com a dica "Fazer um adesivo" (`1008[17]`). O Aspira-coisas
//! também guarda o adesivo aspirado ("Chocante!", `1008[14]`).

use super::*;
use crate::paint::line_points;

/// Meia largura do bocal: 3 × 3 pixels.
const NOZZLE: i32 = 1;
/// Registros do bloco `WHAT_VACUUM` (1008) abertos depois de aspirar.
const REC_THING: usize = 14;
const REC_COPY: usize = 15;
const REC_PAINT: usize = 16;

/// Arrasto do aspirador em andamento.
pub struct VacuumPath {
    /// Pixels do quadro antes de aspirar.
    before: Vec<u8>,
    /// Por onde o bocal passou (um por pixel do quadro).
    mask: Vec<bool>,
    /// Retângulo `(x0, y0, x1, y1)` (inclusive) do que o bocal tocou.
    bbox: Option<(i32, i32, i32, i32)>,
    last: (i32, i32),
    /// Objeto sob o ponteiro ao apertar (o Aspira-cópias copia o objeto num clique).
    object: Option<u32>,
    moved: bool,
}

impl Paint {
    /// Aperto do aspirador no quadro.
    pub(super) fn vacuum_down(&mut self, cx: i32, cy: i32, assets: &Assets) {
        match self.vacuum {
            VacuumMode::Cut | VacuumMode::Copy | VacuumMode::Color if !self.shift => {
                self.vacuum_path = Some(VacuumPath {
                    before: self.canvas.px.clone(),
                    mask: vec![false; self.canvas.px.len()],
                    bbox: None,
                    last: (cx, cy),
                    object: self.objects.hit(cx, cy),
                    moved: false,
                });
                self.vacuum_nozzle(cx, cy);
            }
            VacuumMode::Stickers => {
                // o adesivo aspirado vai para o Aspirador
                if let Some(id) = self.objects.hit(cx, cy) {
                    self.clipboard = self.object_stamp(id);
                    self.vacuum_press(cx, cy);
                    self.vacuum_help(REC_THING, assets);
                }
            }
            _ => self.vacuum_press(cx, cy),
        }
    }

    /// Arrasto: o bocal segue o mouse.
    pub(super) fn vacuum_drag(&mut self, cx: i32, cy: i32) {
        let Some(path) = self.vacuum_path.as_mut() else {
            return;
        };
        if (cx, cy) == path.last {
            return;
        }
        let from = path.last;
        path.last = (cx, cy);
        path.moved = true;
        for (x, y) in line_points(from, (cx, cy)) {
            self.vacuum_nozzle(x, y);
        }
    }

    /// Passa o bocal em `(cx, cy)`: marca o caminho e aspira a tinta.
    fn vacuum_nozzle(&mut self, cx: i32, cy: i32) {
        let (w, h) = (self.canvas.w as i32, self.canvas.h as i32);
        let color = COLORS[self.color];
        let Some(path) = self.vacuum_path.as_mut() else {
            return;
        };
        for y in cy - NOZZLE..=cy + NOZZLE {
            for x in cx - NOZZLE..=cx + NOZZLE {
                if x < 0 || y < 0 || x >= w || y >= h || !self.canvas.can_paint(x, y) {
                    continue;
                }
                let i = (y * w + x) as usize;
                path.mask[i] = true;
                path.bbox = Some(match path.bbox {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
                let suck = match self.vacuum {
                    VacuumMode::Cut => true,
                    VacuumMode::Color => path.before[i] == color,
                    _ => false,
                };
                if suck {
                    self.canvas.px[i] = crate::paint::WHITE;
                }
            }
        }
    }

    /// Solta o aspirador: guarda o aspirado e abre o balão do McZee.
    pub(super) fn vacuum_up(&mut self, assets: &Assets) {
        let Some(path) = self.vacuum_path.take() else {
            return;
        };
        let w = self.canvas.w as i32;
        let only = (self.vacuum == VacuumMode::Color).then_some(COLORS[self.color]);
        let stamp = path.bbox.map(|(x0, y0, x1, y1)| {
            let px = (y0..=y1)
                .flat_map(|y| (x0..=x1).map(move |x| (y * w + x) as usize))
                .map(|i| {
                    let v = path.before[i];
                    (path.mask[i] && only.is_none_or(|c| c == v)).then_some(v)
                })
                .collect();
            Stamp {
                w: (x1 - x0 + 1) as u32,
                h: (y1 - y0 + 1) as u32,
                px,
            }
        });
        match self.vacuum {
            VacuumMode::Copy => {
                // um clique (sem arrastar) num objeto copia o objeto
                self.clipboard = match path.object {
                    Some(id) if !path.moved => self.object_stamp(id),
                    _ => stamp,
                };
                if self.clipboard.is_some() {
                    self.vacuum_help(REC_COPY, assets);
                }
            }
            VacuumMode::Cut | VacuumMode::Color => {
                if self.canvas.px != path.before {
                    self.history.record_previous(path.before);
                    self.changed = true;
                    if let Some(s) = stamp.filter(|s| s.px.iter().any(Option::is_some)) {
                        self.clipboard = Some(s);
                    }
                    self.vacuum_help(REC_PAINT, assets);
                } else if self.vacuum == VacuumMode::Cut {
                    // aspirou só branco: o Aspirador guarda a forma do traço
                    self.clipboard = stamp.or(self.clipboard.take());
                    self.vacuum_help(REC_PAINT, assets);
                }
            }
            _ => {}
        }
    }

    fn vacuum_help(&mut self, record: usize, assets: &Assets) {
        if self.help_on {
            self.open_help(WHAT_VACUUM, record, assets);
        }
    }

    /// Caminho do Aspira-cópias, pontilhado por cima do quadro.
    pub(super) fn draw_vacuum_path(&self, f: &mut Frame) {
        let Some(path) = &self.vacuum_path else {
            return;
        };
        if self.vacuum != VacuumMode::Copy || self.zoom.is_some() {
            return;
        }
        let Some((x0, y0, x1, y1)) = path.bbox else {
            return;
        };
        let w = self.canvas.w as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if path.mask[(y * w + x) as usize] {
                    let c = if (x + y) % 2 == 0 { BLACK } else { WHITE };
                    f.put(ORIGIN.0 + x, ORIGIN.1 + y, c);
                }
            }
        }
    }

    pub fn vacuuming(&self) -> bool {
        self.vacuum_path.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn mv(x: i32, y: i32) -> Input {
        Input::MouseMove { x, y }
    }

    fn up(x: i32, y: i32) -> Input {
        Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn black(p: &Paint) -> usize {
        p.canvas()
            .px
            .iter()
            .filter(|&&v| v != crate::paint::WHITE)
            .count()
    }

    /// Faixa preta horizontal de 20 px de altura no quadro (y 180..200).
    fn band(p: &mut Paint) {
        for y in 180 - ORIGIN.1..200 - ORIGIN.1 {
            for x in 0..p.canvas.w as i32 {
                p.canvas.set(x, y, COLORS[SWATCHES - 1]);
            }
        }
    }

    fn vacuum_option(p: &mut Paint, a: &Assets, k: usize) {
        p.update(
            &[click(320, 30), click(40 + 37 * k as i32 + 16, 79)],
            0.0,
            a,
        );
    }

    #[test]
    fn paint_sucker_follows_the_mouse_and_opens_where_did_it_go() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        band(&mut p);
        let all = black(&p);
        vacuum_option(&mut p, &a, 0);
        assert_eq!(p.vacuum_mode(), VacuumMode::Cut);
        // arrasto diagonal pela faixa: some só uma tira de 3 px, não o retângulo
        p.update(&[click(100, 170), mv(140, 210)], 0.0, &a);
        assert!(p.vacuuming());
        let during = black(&p);
        assert!(during < all, "aspira enquanto arrasta");
        p.update(&[up(140, 210)], 0.0, &a);
        let gone = all - black(&p);
        assert!((40..=120).contains(&gone), "{gone}");
        assert!(p.has_clipboard());
        let s = p.clipboard.as_ref().unwrap();
        assert!(
            s.px.iter().any(|v| v.is_none()),
            "o aspirado tem a forma do traço"
        );
        // o ovo traz tudo de volta
        p.update(&[click(270, 30)], 0.0, &a);
        assert_eq!(black(&p), all);
    }

    #[test]
    fn shift_keeps_the_rectangle() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        band(&mut p);
        vacuum_option(&mut p, &a, 0);
        p.update(
            &[
                Input::Shift(true),
                click(100, 170),
                mv(140, 210),
                up(140, 210),
                Input::Shift(false),
            ],
            0.0,
            &a,
        );
        // o retângulo 41 × 20 dentro da faixa sumiu inteiro
        assert_eq!(
            p.canvas().get(120 - ORIGIN.0, 190 - ORIGIN.1),
            Some(crate::paint::WHITE)
        );
        assert_eq!(
            p.canvas().get(101 - ORIGIN.0, 199 - ORIGIN.1),
            Some(crate::paint::WHITE)
        );
    }

    #[test]
    fn copy_and_color_suckers_follow_the_mouse() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        band(&mut p);
        let all = black(&p);
        vacuum_option(&mut p, &a, 4);
        assert_eq!(p.vacuum_mode(), VacuumMode::Copy);
        p.update(&[click(100, 190), mv(200, 190)], 0.0, &a);
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        p.update(&[up(200, 190)], 0.0, &a);
        assert_eq!(black(&p), all, "copiar não apaga");
        let s = p.clipboard.as_ref().unwrap();
        assert_eq!((s.w, s.h), (103, 3));

        // aspira-cores: só a cor escolhida (a preta, a última da coluna)
        vacuum_option(&mut p, &a, 5);
        p.color = SWATCHES - 1;
        p.update(&[click(100, 150), mv(100, 230), up(100, 230)], 0.0, &a);
        assert_eq!(all - black(&p), 3 * 20);
        let s = p.clipboard.as_ref().unwrap();
        assert_eq!(s.px.iter().filter(|v| v.is_some()).count(), 60);
    }

    #[test]
    fn thing_sucker_keeps_the_sticker_for_the_spewer() {
        let (_d, a) = empty_assets();
        let mut p = Paint::new(None, &a);
        let stamp = Rc::new(Stamp {
            w: 4,
            h: 4,
            px: vec![Some(COLORS[3]); 16],
        });
        let id = p.objects.place(stamp, 50, 50, Source::Png("x.png".into()));
        assert!(p.objects.get(id).is_some());
        vacuum_option(&mut p, &a, 1);
        p.update(&[click(ORIGIN.0 + 51, ORIGIN.1 + 51)], 0.0, &a);
        assert_eq!(p.objects().len(), 0);
        assert!(p.has_clipboard());
    }

    #[test]
    fn real_balloons_after_vacuuming() {
        let Some(a) = assets() else { return };
        let mut p = Paint::new(None, &a);
        band(&mut p);
        vacuum_option(&mut p, &a, 0);
        p.update(&[click(100, 170), mv(140, 210), up(140, 210)], 0.0, &a);
        let h = p.help().unwrap();
        assert_eq!((h.block, h.index), (WHAT_VACUUM, REC_PAINT));
        assert!(h.title().contains("paint sucker"));
        assert!(h.tail.is_some());
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "vacuum_where_did_it_go");
        vacuum_option(&mut p, &a, 4);
        p.update(&[click(100, 190), mv(200, 190), up(200, 190)], 0.0, &a);
        assert_eq!(p.help().map(|h| h.index), Some(REC_COPY));
        let stamp = Rc::new(Stamp {
            w: 4,
            h: 4,
            px: vec![Some(COLORS[3]); 16],
        });
        p.objects.place(stamp, 50, 50, Source::Png("x.png".into()));
        vacuum_option(&mut p, &a, 1);
        p.update(&[click(ORIGIN.0 + 51, ORIGIN.1 + 51)], 0.0, &a);
        assert_eq!(p.help().map(|h| h.index), Some(REC_THING));
        // o ovo: balão com o ícone e o rabicho
        p.update(&[click(270, 30)], 0.0, &a);
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "egg_balloon");
    }
}
