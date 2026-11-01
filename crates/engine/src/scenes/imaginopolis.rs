//! Imaginópolis vista de fora (fundo `6cfd`, 641×480, cortado a 640): a porta
//! "Entre" do Museu Mágico leva à rua. Logo ao chegar, o McZee passa no
//! carrinho da montanha-russa (roteiro `ATAG` 0x2bc), uma vez só: no
//! original ele desce 0,2 s depois de a tela aparecer e, em 90 s de
//! gravação, não passa de novo.

use crate::anim::{self, Animator};
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, SCREEN_H, SCREEN_W};

pub const BACKGROUND: u16 = 0x6cfd;
/// Porta do museu (x, y, largura, altura), em volta de (302,172) do original.
pub const DOOR: (i32, i32, i32, i32) = (280, 130, 46, 76);
/// Roteiro do carrinho e quando ele começa depois de a tela aparecer (s).
pub const CART: u16 = 0x2bc;
const CART_DELAY: f32 = 0.2;

fn inside(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

pub struct Imaginopolis {
    common: KidCommon,
    clock: f32,
    /// O carrinho já começou (passa uma vez só).
    cart_started: bool,
    cart: Option<Animator>,
}

impl Imaginopolis {
    pub fn new(click: Option<SoundRef>) -> Imaginopolis {
        Imaginopolis {
            common: KidCommon::new(click),
            clock: 0.0,
            cart_started: false,
            cart: None,
        }
    }

    /// O carrinho está passando.
    pub fn gag_playing(&self) -> bool {
        self.cart.is_some()
    }
}

impl Scene for Imaginopolis {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        self.clock += dt;
        if !self.cart_started && self.clock >= CART_DELAY {
            self.cart_started = true;
            self.cart = anim::load_exe(assets, Program::Artist, CART);
            if let Some(a) = &mut self.cart {
                fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
            }
        } else if let Some(a) = &mut self.cart {
            fx.extend(a.update(dt).into_iter().map(Effect::PlaySound));
            if a.is_finished() {
                self.cart = None;
            }
        }
        for i in input {
            if let Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } = *i
                && inside(DOOR, x, y)
            {
                fx.push(Effect::ChangeScene(SceneId::Street));
                break;
            }
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        frame.fill([0, 0, 0]);
        let bg = assets.image(&ImageRef::Pic(Program::Artist, BACKGROUND));
        // 641 px de largura: alinhado à esquerda, a última coluna fica de fora
        let x = if bg.width > SCREEN_W {
            0
        } else {
            (SCREEN_W as i32 - bg.width as i32) / 2
        };
        frame.blit(&bg, x, (SCREEN_H as i32 - bg.height as i32) / 2);
        if let Some(a) = &self.cart {
            a.draw(frame, assets);
        }
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

    #[test]
    fn door_leads_to_street() {
        let (_d, a) = empty_assets();
        let mut s = Imaginopolis::new(None);
        let fx = s.update(&[click(302, 172)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Street)));
        let fx = s.update(&[click(100, 400)], 0.0, &a);
        assert!(!fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))));
        let mut f = Frame::new(640, 480);
        for _ in 0..30 {
            s.update(&[], 1.0, &a);
        }
        s.draw(&mut f, &a);
    }

    /// Como no original: o McZee passa logo ao chegar (já na terceira
    /// atualização de 0,1 s), uma vez só (nada em 90 s depois disso).
    #[test]
    fn cart_crosses_on_arrival_once_with_real_data() {
        let Some(a) = assets() else { return };
        let mut s = Imaginopolis::new(None);
        let mut f = Frame::new(640, 480);
        for _ in 0..3 {
            s.update(&[], 0.1, &a);
        }
        assert!(s.gag_playing());
        s.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "imaginopolis_cart");
        for _ in 0..40 {
            s.update(&[], 0.1, &a);
        }
        assert!(!s.gag_playing(), "o carrinho dura 3,5 s");
        for _ in 0..900 {
            s.update(&[], 0.1, &a);
            assert!(!s.gag_playing());
        }
        s.draw(&mut f, &a);
        // céu azul-arroxeado no alto
        let p = f.pixel(50, 20);
        assert!(p[2] > p[1], "{p:?}");
    }
}
