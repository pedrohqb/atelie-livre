//! Imaginópolis vista de fora (fundo `6cfd`, 641×480, cortado a 640): a porta
//! "Entre" do Museu Mágico leva à rua; de vez em quando o McZee atravessa
//! no carrinho da montanha-russa (roteiro `ATAG` 0x2bc).

use super::gags::GagPlayer;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, SCREEN_H, SCREEN_W};

pub const BACKGROUND: u16 = 0x6cfd;
/// Porta do museu (x, y, largura, altura), em volta de (302,172) do original.
pub const DOOR: (i32, i32, i32, i32) = (280, 130, 46, 76);
/// Roteiro do carrinho.
pub const CART: u16 = 0x2bc;
const GAG_FIRST: (f32, f32) = (2.0, 6.0);
const GAG_BETWEEN: (f32, f32) = (8.0, 20.0);

fn inside(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

pub struct Imaginopolis {
    common: KidCommon,
    gags: GagPlayer,
}

impl Imaginopolis {
    pub fn new(click: Option<SoundRef>) -> Imaginopolis {
        Imaginopolis {
            common: KidCommon::new(click),
            gags: GagPlayer::new(&[CART], GAG_FIRST, GAG_BETWEEN),
        }
    }

    pub fn gag_playing(&self) -> bool {
        self.gags.is_playing()
    }
}

impl Scene for Imaginopolis {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        fx.extend(
            self.gags
                .update(dt, assets)
                .into_iter()
                .map(Effect::PlaySound),
        );
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
        self.gags.draw(frame, assets);
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

    #[test]
    fn cart_crosses_with_real_data() {
        let Some(a) = assets() else { return };
        let mut s = Imaginopolis::new(None);
        let mut f = Frame::new(640, 480);
        let mut played = false;
        for _ in 0..200 {
            s.update(&[], 0.1, &a);
            if s.gag_playing() {
                played = true;
                s.draw(&mut f, &a);
            }
        }
        assert!(played);
        s.draw(&mut f, &a);
        // céu azul-arroxeado no alto
        let p = f.pixel(50, 20);
        assert!(p[2] > p[1], "{p:?}");
    }
}
