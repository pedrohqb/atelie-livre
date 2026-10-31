//! Abertura: tela de título do Fine Artist; clique ou 5 s levam à rua.

use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, SCREEN_H, SCREEN_W};

#[cfg(test)]
use crate::scene::DEFAULT_CURSOR;

pub const TITLE: u16 = 0x6ca7;
const TIMEOUT: f32 = 5.0;

pub struct Splash {
    common: KidCommon,
    elapsed: f32,
    done: bool,
}

impl Splash {
    pub fn new(click: Option<SoundRef>) -> Splash {
        Splash {
            common: KidCommon::new(click),
            elapsed: 0.0,
            done: false,
        }
    }
}

impl Scene for Splash {
    fn update(&mut self, input: &[Input], dt: f32, _assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        self.elapsed += dt;
        let clicked = input.iter().any(|i| {
            matches!(
                i,
                Input::Click {
                    button: MouseButton::Left,
                    ..
                }
            )
        });
        if !self.done && (clicked || self.elapsed >= TIMEOUT) {
            self.done = true;
            fx.push(Effect::ChangeScene(SceneId::Street));
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        frame.fill([0, 0, 0]);
        let img = assets.image(&ImageRef::Pic(Program::Artist, TITLE));
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
    use crate::scenes::testutil::{assets, empty_assets};

    fn click() -> Input {
        Input::Click {
            button: MouseButton::Left,
            x: 10,
            y: 10,
        }
    }

    #[test]
    fn first_update_sets_cursor() {
        let (_d, a) = empty_assets();
        let mut s = Splash::new(None);
        assert!(
            s.update(&[], 0.0, &a)
                .contains(&Effect::SetCursor(DEFAULT_CURSOR))
        );
        assert!(
            !s.update(&[], 0.0, &a)
                .contains(&Effect::SetCursor(DEFAULT_CURSOR))
        );
    }

    #[test]
    fn splash_click_goes_to_street() {
        let (_d, a) = empty_assets();
        let mut s = Splash::new(None);
        assert!(
            s.update(&[click()], 0.016, &a)
                .contains(&Effect::ChangeScene(SceneId::Street))
        );
    }

    #[test]
    fn splash_timeout_goes_to_street_once() {
        let (_d, a) = empty_assets();
        let mut s = Splash::new(None);
        let mut changes = 0;
        for _ in 0..8 {
            changes += s
                .update(&[], 1.0, &a)
                .iter()
                .filter(|e| **e == Effect::ChangeScene(SceneId::Street))
                .count();
        }
        assert_eq!(changes, 1);
    }

    #[test]
    fn click_plays_configured_sound() {
        let (_d, a) = empty_assets();
        let snd = SoundRef::Snd(Program::Artist, 0x209);
        let mut s = Splash::new(Some(snd.clone()));
        assert!(
            s.update(&[click()], 0.0, &a)
                .contains(&Effect::PlaySound(snd))
        );
        let mut quiet = Splash::new(None);
        assert!(
            !quiet
                .update(&[click()], 0.0, &a)
                .iter()
                .any(|e| matches!(e, Effect::PlaySound(_)))
        );
    }

    #[test]
    fn splash_draws_title() {
        let Some(a) = assets() else { return };
        let s = Splash::new(None);
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        s.draw(&mut f, &a);
        let img = a.image(&ImageRef::Pic(Program::Artist, 0x6ca7));
        let i = ((240 * img.width + 320) * 4) as usize;
        assert_eq!(
            f.pixel(320, 240),
            <[u8; 4]>::try_from(&img.rgba[i..i + 4]).unwrap()
        );
    }
}
