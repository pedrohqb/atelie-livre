//! Botões dos close-ups da Oficina de Idéias com estado apertado
//! (`office-complemento.md` §7, tarefa C9): ao apertar o botão mostra a
//! figura "apertada"; o som e a ação vêm ao **soltar sobre o botão**;
//! soltar fora cancela.

use super::wpic;
use crate::Frame;
use crate::assets::Assets;
use crate::ui::{Rect, inside};

/// Figura do `WRITER.PIC` com a máscara (`None` = opaca, ou com o branco
/// transparente se `keyed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Art {
    pub id: u16,
    pub mask: Option<u16>,
    pub keyed: bool,
}

impl Art {
    pub const fn masked(id: u16, mask: u16) -> Art {
        Art {
            id,
            mask: Some(mask),
            keyed: false,
        }
    }

    pub const fn opaque(id: u16) -> Art {
        Art {
            id,
            mask: None,
            keyed: false,
        }
    }

    /// Sem máscara no `.PIC` (as setas de 12×12): o branco fica transparente.
    pub const fn keyed(id: u16) -> Art {
        Art {
            id,
            mask: None,
            keyed: true,
        }
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets, x: i32, y: i32) {
        let img = a.image(&wpic(self.id));
        match self.mask {
            Some(m) => f.blit_masked(&img, &a.image(&wpic(m)), x, y),
            None if self.keyed => {
                for dy in 0..img.height {
                    for dx in 0..img.width {
                        let i = ((dy * img.width + dx) * 4) as usize;
                        let p = &img.rgba[i..i + 4];
                        if p[3] != 0 && p[..3] != [255, 255, 255] {
                            f.put(x + dx as i32, y + dy as i32, [p[0], p[1], p[2]]);
                        }
                    }
                }
            }
            None => f.blit(&img, x, y),
        }
    }
}

/// Botão: figura normal e apertada no mesmo lugar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressButton {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub normal: Art,
    pub pressed: Art,
}

impl PressButton {
    pub const fn new(x: i32, y: i32, w: i32, h: i32, normal: Art, pressed: Art) -> PressButton {
        PressButton {
            x,
            y,
            w,
            h,
            normal,
            pressed,
        }
    }

    /// O mesmo botão em outro lugar.
    pub const fn at(self, x: i32, y: i32) -> PressButton {
        PressButton { x, y, ..self }
    }

    pub fn rect(&self) -> Rect {
        (self.x, self.y, self.w, self.h)
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets, down: bool) {
        let art = if down { self.pressed } else { self.normal };
        art.draw(f, a, self.x, self.y);
    }
}

/// Qual botão está apertado (um por vez).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presser<K> {
    down: Option<K>,
}

impl<K> Default for Presser<K> {
    fn default() -> Self {
        Presser { down: None }
    }
}

impl<K: Copy + Eq> Presser<K> {
    pub fn press(&mut self, k: K) {
        self.down = Some(k);
    }

    pub fn is_down(&self, k: K) -> bool {
        self.down == Some(k)
    }

    pub fn down(&self) -> Option<K> {
        self.down
    }

    /// Botão solto sobre `under`: devolve o botão se for o mesmo que foi
    /// apertado (a ação acontece); soltar fora só cancela.
    pub fn release(&mut self, under: Option<K>) -> Option<K> {
        let k = self.down.take()?;
        (under == Some(k)).then_some(k)
    }
}

/// O botão de `buttons` sob `(x, y)`.
pub fn hit<K: Copy>(buttons: &[(K, PressButton)], x: i32, y: i32) -> Option<K> {
    buttons
        .iter()
        .find(|(_, b)| inside(b.rect(), x, y))
        .map(|(k, _)| *k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_only_when_released_over_the_same_button() {
        let mut p: Presser<u8> = Presser::default();
        assert_eq!(p.release(Some(1)), None);
        p.press(1);
        assert!(p.is_down(1));
        assert_eq!(p.release(Some(1)), Some(1));
        assert!(!p.is_down(1));
        p.press(1);
        assert_eq!(p.release(Some(2)), None);
        p.press(1);
        assert_eq!(p.release(None), None);
    }

    #[test]
    fn hit_finds_the_button() {
        let b = PressButton::new(10, 10, 5, 5, Art::opaque(1), Art::opaque(2));
        let list = [(7u8, b), (8u8, b.at(30, 10))];
        assert_eq!(hit(&list, 12, 12), Some(7));
        assert_eq!(hit(&list, 31, 14), Some(8));
        assert_eq!(hit(&list, 20, 12), None);
    }
}
