//! Rua de Imaginópolis (ainda sem áreas clicáveis).

use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Input, KidCommon, Scene};
use crate::{Frame, SCREEN_H, SCREEN_W};

#[cfg(test)]
use crate::scene::{DEFAULT_CURSOR, MouseButton};

pub const BACKGROUND: u16 = 0x6d01;

pub struct Street {
    common: KidCommon,
}

impl Street {
    pub fn new(click: Option<SoundRef>) -> Street {
        Street {
            common: KidCommon::new(click),
        }
    }
}

impl Scene for Street {
    fn update(&mut self, input: &[Input], _dt: f32, _assets: &Assets) -> Vec<Effect> {
        self.common.update(input)
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        frame.fill([0, 0, 0]);
        let img = assets.image(&ImageRef::Pic(Program::Artist, BACKGROUND));
        frame.blit(
            &img,
            (SCREEN_W as i32 - img.width as i32) / 2,
            (SCREEN_H as i32 - img.height as i32) / 2,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    #[test]
    fn street_click_plays_sound_and_stays() {
        let (_d, a) = empty_assets();
        let snd = SoundRef::Snd(Program::Artist, 0x20a);
        let mut s = Street::new(Some(snd.clone()));
        let fx = s.update(
            &[Input::Click {
                button: MouseButton::Left,
                x: 1,
                y: 1,
            }],
            0.0,
            &a,
        );
        assert!(fx.contains(&Effect::PlaySound(snd)));
        assert!(fx.contains(&Effect::SetCursor(DEFAULT_CURSOR)));
        assert!(!fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))));
        assert!(s.update(&[], 100.0, &a).is_empty());
    }
}
