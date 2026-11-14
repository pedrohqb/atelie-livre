//! Figura da dobra do cartão francês, mostrada pelo assistente do Cartão
//! (`WHAT 3c00[2]`–`[4]`, "Ver como se dobra o cartão?").
//!
//! A folha `0c65` (páginas 1 e 4 de cabeça para baixo em cima, 2 e 3
//! embaixo) dobra ao meio (`3c1b`, "2 3") e ao meio de novo (`3c33`, "1").
//! O original anima a dobra pelo código (nenhum `ATAG` usa essas figuras):
//! um quadro a cada ≈150 ms, `0c65 → 3c0b → 3c13 → 3c1b` no Segue de `[2]`
//! e `3c1b → 3c23 → 3c2b → 3c33` no de `[3]`. O Volta desfaz: os mesmos
//! quadros de trás para a frente. Cada quadro tem máscara 6 figuras adiante
//! (`3c0b`/`3c11` … `3c33`/`3c39`); a folha usa `3c09`. `3c3b` (189×77) não
//! entra na sequência.
//!
//! Posições: `0c65`, `3c1b` e `3c33` medidos nas capturas do original
//! (`projects_45_dobra_*`, `46_dobra_segue_*`, `47_dobra3*`); os quadros
//! intermediários, que passam rápido demais para a captura, ficam alinhados
//! pelo canto inferior direito dos medidos (≈ 338, 284).
//!
//! Sem tela: quem hospeda o assistente chama [`FoldFigure::tick`] e
//! [`FoldFigure::draw`].

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};

/// Um quadro da dobra: figura e máscara do `WRITER.PIC`, canto superior
/// esquerdo na tela e tamanho.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldFrame {
    pub pic: u16,
    pub mask: u16,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

const fn fr(pic: u16, mask: u16, x: i32, y: i32, w: i32, h: i32) -> FoldFrame {
    FoldFrame {
        pic,
        mask,
        x,
        y,
        w,
        h,
    }
}

/// Os sete quadros, da folha aberta ao cartão dobrado.
pub const FRAMES: [FoldFrame; 7] = [
    fr(0x0c65, 0x3c09, 183, 127, 152, 153), // folha 1 4 / 2 3 (medida)
    fr(0x3c0b, 0x3c11, 174, 171, 164, 113),
    fr(0x3c13, 0x3c19, 179, 207, 159, 77),
    fr(0x3c1b, 0x3c21, 193, 207, 145, 77), // dobrada ao meio, "2 3" (medida)
    fr(0x3c23, 0x3c29, 213, 188, 124, 95),
    fr(0x3c2b, 0x3c31, 253, 186, 84, 97),
    fr(0x3c33, 0x3c39, 253, 205, 84, 77), // dobrada de novo, "1" (medida)
];

/// Duração de cada quadro da animação (s).
pub const FRAME_SECS: f32 = 0.15;

/// Os três estados parados da figura, um por balão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldStage {
    /// `3c00[2]`: a folha aberta.
    Sheet,
    /// `3c00[3]`: "Dobro no meio…".
    Half,
    /// `3c00[4]`: "… depois, dobro no meio de novo."
    Quarter,
}

impl FoldStage {
    /// Índice do quadro em [`FRAMES`].
    pub const fn frame(self) -> usize {
        match self {
            FoldStage::Sheet => 0,
            FoldStage::Half => 3,
            FoldStage::Quarter => 6,
        }
    }
}

/// A figura na tela: o quadro mostrado e o estado para onde ela anda.
#[derive(Debug, Clone, PartialEq)]
pub struct FoldFigure {
    frame: usize,
    target: usize,
    t: f32,
}

impl FoldFigure {
    /// Parada no estado `stage`.
    pub fn new(stage: FoldStage) -> FoldFigure {
        FoldFigure {
            frame: stage.frame(),
            target: stage.frame(),
            t: 0.0,
        }
    }

    /// Anda (dobrando ou desdobrando) até `stage`, a partir do quadro que
    /// está à vista — inclusive no meio de outra animação.
    pub fn go_to(&mut self, stage: FoldStage) {
        if self.target != stage.frame() {
            self.target = stage.frame();
            self.t = 0.0;
        }
    }

    /// Ainda trocando de quadro.
    pub fn animating(&self) -> bool {
        self.frame != self.target
    }

    /// Índice do quadro à vista em [`FRAMES`].
    pub fn frame_index(&self) -> usize {
        self.frame
    }

    /// O quadro à vista.
    pub fn frame(&self) -> &'static FoldFrame {
        &FRAMES[self.frame]
    }

    /// Avança o relógio: um quadro a cada [`FRAME_SECS`].
    pub fn tick(&mut self, dt: f32) {
        if !self.animating() || !dt.is_finite() {
            return;
        }
        self.t += dt.max(0.0);
        while self.t >= FRAME_SECS && self.animating() {
            self.t -= FRAME_SECS;
            if self.target > self.frame {
                self.frame += 1;
            } else {
                self.frame -= 1;
            }
        }
        if !self.animating() {
            self.t = 0.0;
        }
    }

    /// Desenha o quadro à vista com a máscara; sem os dados, uma folha
    /// branca contornada do mesmo tamanho.
    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let q = self.frame();
        let img = ImageRef::Pic(Program::Writer, q.pic);
        if a.has_image(&img) {
            let mask = ImageRef::Pic(Program::Writer, q.mask);
            let img = a.image(&img);
            if a.has_image(&mask) {
                f.blit_masked(&img, &a.image(&mask), q.x, q.y);
            } else {
                f.blit(&img, q.x, q.y);
            }
        } else {
            f.rect(q.x, q.y, q.w, q.h, [0, 0, 0]);
            f.rect(q.x + 1, q.y + 1, q.w - 2, q.h - 2, [255, 255, 255]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};

    #[test]
    fn folds_one_frame_every_150_ms() {
        let mut f = FoldFigure::new(FoldStage::Sheet);
        assert!(!f.animating());
        assert_eq!(f.frame().pic, 0x0c65);
        f.go_to(FoldStage::Half);
        assert!(f.animating());
        f.tick(0.1);
        assert_eq!(f.frame_index(), 0);
        f.tick(0.06);
        assert_eq!(f.frame().pic, 0x3c0b);
        f.tick(0.15);
        assert_eq!(f.frame().pic, 0x3c13);
        f.tick(1.0);
        assert_eq!(f.frame().pic, 0x3c1b);
        assert!(!f.animating());
        // de novo ao meio: 3c23, 3c2b, 3c33
        f.go_to(FoldStage::Quarter);
        let mut seen = Vec::new();
        for _ in 0..3 {
            f.tick(FRAME_SECS + 0.001);
            seen.push(f.frame().pic);
        }
        assert_eq!(seen, [0x3c23, 0x3c2b, 0x3c33]);
        assert!(!f.animating());
    }

    #[test]
    fn back_unfolds_and_can_reverse_midway() {
        let mut f = FoldFigure::new(FoldStage::Half);
        f.go_to(FoldStage::Sheet);
        f.tick(FRAME_SECS + 0.001);
        assert_eq!(f.frame().pic, 0x3c13);
        // Segue de novo no meio da volta: dobra a partir de onde está
        f.go_to(FoldStage::Half);
        f.tick(FRAME_SECS + 0.001);
        assert_eq!(f.frame().pic, 0x3c1b);
        assert!(!f.animating());
        f.go_to(FoldStage::Sheet);
        f.tick(10.0);
        assert_eq!(f.frame().pic, 0x0c65);
        // relógio estranho não anda
        f.go_to(FoldStage::Quarter);
        f.tick(f32::NAN);
        f.tick(-1.0);
        assert_eq!(f.frame_index(), 0);
    }

    #[test]
    fn measured_frames_share_the_bottom_right_corner() {
        for q in FRAMES {
            let (r, b) = (q.x + q.w, q.y + q.h);
            assert!((335..=338).contains(&r), "{:04x}: direita {r}", q.pic);
            assert!((280..=284).contains(&b), "{:04x}: pé {b}", q.pic);
            assert_eq!(q.mask, if q.pic == 0x0c65 { 0x3c09 } else { q.pic + 6 });
        }
    }

    #[test]
    fn draws_a_placeholder_without_data() {
        let (_d, a) = empty_assets();
        let mut f = Frame::new(640, 480);
        f.fill([9, 9, 9]);
        FoldFigure::new(FoldStage::Quarter).draw(&mut f, &a);
        let q = FRAMES[6];
        assert_eq!(f.pixel(q.x as u32, q.y as u32), [0, 0, 0, 255]);
        assert_eq!(
            f.pixel(q.x as u32 + 5, q.y as u32 + 5),
            [255, 255, 255, 255]
        );
    }

    #[test]
    fn frames_and_masks_exist_with_the_listed_sizes() {
        let Some(a) = assets() else { return };
        for q in FRAMES {
            for id in [q.pic, q.mask] {
                let img = a.image(&ImageRef::Pic(Program::Writer, id));
                assert_eq!(
                    (img.width as i32, img.height as i32),
                    (q.w, q.h),
                    "{id:04x}"
                );
            }
        }
        let mut f = Frame::new(640, 480);
        f.fill([9, 9, 9]);
        FoldFigure::new(FoldStage::Sheet).draw(&mut f, &a);
        // a máscara deixa o fundo fora da folha (canto da sombra)
        assert_eq!(f.pixel(184, 278), [9, 9, 9, 255]);
        assert_ne!(f.pixel(260, 200), [9, 9, 9, 255]);
    }
}
