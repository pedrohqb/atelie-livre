//! Imaginópolis vista de fora (fundo `6cfd`, 641×480, cortado a 640). Logo ao
//! chegar (da Entrada ou da rua), a tela toca o 0x302 (trilha da chegada) e o
//! McZee passa no carrinho da montanha-russa (roteiro `ATAG` 0x2bc), uma vez
//! só: no original ele desce 0,2 s depois de a tela aparecer e, em 90 s de
//! gravação, não passa de novo.
//!
//! A tela é um "clique para continuar": depois do carrinho, um clique em
//! qualquer ponto (não só na porta "Entre") leva à rua, com o som 0x253.
//! Enquanto o carrinho passa, os cliques são ignorados.

use crate::anim::{self, Animator};
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, SCREEN_H, SCREEN_W};

pub const BACKGROUND: u16 = 0x6cfd;
/// Roteiro do carrinho e quando ele começa depois de a tela aparecer (s).
pub const CART: u16 = 0x2bc;
const CART_DELAY: f32 = 0.2;
/// Trilha da chegada (178304 bytes, ~16 s): `ARTIST.EXE` a toca logo antes
/// de começar o 0x2bc; continua depois de o carrinho sumir.
pub const SND_ARRIVE: u16 = 0x302;
/// Som da troca para a rua (1068 bytes; 0x2c0–0x2c5 são o mesmo som).
pub const SND_ENTER: u16 = 0x253;

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

    /// A tela já responde a cliques (o carrinho já passou).
    pub fn accepts_clicks(&self) -> bool {
        self.cart_started && self.cart.is_none()
    }
}

impl Scene for Imaginopolis {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        // cliques que chegam durante o carrinho (ou antes dele) são ignorados
        let ready = self.accepts_clicks();
        self.clock += dt;
        if !self.cart_started && self.clock >= CART_DELAY {
            self.cart_started = true;
            fx.push(Effect::PlaySound(SoundRef::Snd(
                Program::Artist,
                SND_ARRIVE,
            )));
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
        if ready
            && input.iter().any(|i| {
                matches!(
                    i,
                    Input::Click {
                        button: MouseButton::Left,
                        ..
                    }
                )
            })
        {
            fx.push(Effect::PlaySound(SoundRef::Snd(Program::Artist, SND_ENTER)));
            fx.push(Effect::ChangeScene(SceneId::Street));
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

    fn goes_to_street(fx: &[Effect]) -> bool {
        fx.contains(&Effect::ChangeScene(SceneId::Street))
    }

    #[test]
    fn any_click_leads_to_street_after_arrival() {
        let (_d, a) = empty_assets();
        let mut s = Imaginopolis::new(None);
        // antes do carrinho: ignorado
        let fx = s.update(&[click(302, 172)], 0.0, &a);
        assert!(!goes_to_street(&fx));
        // sem dados não há carrinho: a trilha toca e a tela já responde
        let fx = s.update(&[], 0.3, &a);
        assert!(fx.contains(&Effect::PlaySound(SoundRef::Snd(
            Program::Artist,
            SND_ARRIVE
        ))));
        for (x, y) in [
            (88, 282),
            (222, 236),
            (230, 330),
            (600, 245),
            (302, 148),
            (508, 200),
        ] {
            let mut t = Imaginopolis::new(None);
            t.update(&[], 0.3, &a);
            let fx = t.update(&[click(x, y)], 0.1, &a);
            assert!(goes_to_street(&fx), "({x},{y})");
            assert!(fx.contains(&Effect::PlaySound(SoundRef::Snd(
                Program::Artist,
                SND_ENTER
            ))));
        }
        let mut f = Frame::new(640, 480);
        s.draw(&mut f, &a);
    }

    #[test]
    fn right_click_does_nothing() {
        let (_d, a) = empty_assets();
        let mut s = Imaginopolis::new(None);
        s.update(&[], 0.3, &a);
        let fx = s.update(
            &[Input::Click {
                button: MouseButton::Right,
                x: 300,
                y: 200,
            }],
            0.1,
            &a,
        );
        assert!(!goes_to_street(&fx));
    }

    /// Como no original: o McZee passa logo ao chegar (já na terceira
    /// atualização de 0,1 s), uma vez só (nada em 90 s depois disso); os
    /// cliques durante o carrinho não fazem nada.
    #[test]
    fn cart_crosses_on_arrival_once_with_real_data() {
        let Some(a) = assets() else { return };
        let mut s = Imaginopolis::new(None);
        let mut f = Frame::new(640, 480);
        let mut arrive = 0;
        for _ in 0..3 {
            let fx = s.update(&[], 0.1, &a);
            arrive += fx
                .iter()
                .filter(|e| **e == Effect::PlaySound(SoundRef::Snd(Program::Artist, SND_ARRIVE)))
                .count();
        }
        assert_eq!(arrive, 1);
        assert!(s.gag_playing());
        s.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "imaginopolis_cart");
        for _ in 0..34 {
            let fx = s.update(&[click(302, 172)], 0.1, &a);
            assert!(!goes_to_street(&fx), "clique durante o carrinho");
        }
        for _ in 0..6 {
            s.update(&[], 0.1, &a);
        }
        assert!(!s.gag_playing(), "o carrinho dura 3,5 s");
        for _ in 0..900 {
            let fx = s.update(&[], 0.1, &a);
            assert!(!s.gag_playing());
            assert!(fx.is_empty());
        }
        s.draw(&mut f, &a);
        // céu azul-arroxeado no alto
        let p = f.pixel(50, 20);
        assert!(p[2] > p[1], "{p:?}");
        let fx = s.update(&[click(50, 20)], 0.1, &a);
        assert!(goes_to_street(&fx));
    }
}
