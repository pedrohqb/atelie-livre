//! Abertura: tela de título do Fine Artist (clique ou 5 s levam à Entrada) ou do
//! Creative Writer (leva ao Escritório). Enquanto espera, no Fine Artist
//! tocam as animações do título: o pássaro atravessa a tela e depois os
//! demais roteiros (McZee saindo do buraco com chapéus, porco, gafanhoto...)
//! em ordem sorteada.

use super::gags::{GagPlayer, TITLE_BIRD, TITLE_SCRIPTS};
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, SCREEN_H, SCREEN_W};

#[cfg(test)]
use crate::scene::DEFAULT_CURSOR;

pub const TITLE: u16 = 0x6ca7;
/// Tela de título do Creative Writer (`WRITER.PIC`, 640×480).
pub const WRITER_TITLE: u16 = 0x6737;
const TIMEOUT: f32 = 5.0;

pub struct Splash {
    common: KidCommon,
    elapsed: f32,
    done: bool,
    program: Program,
    title: u16,
    next: SceneId,
    gags: GagPlayer,
}

impl Splash {
    /// Abertura do Fine Artist.
    pub fn new(click: Option<SoundRef>) -> Splash {
        let mut gags = GagPlayer::new(TITLE_SCRIPTS, (0.3, 0.3), (0.5, 1.5));
        gags.queue(TITLE_BIRD);
        Splash {
            common: KidCommon::new(click),
            elapsed: 0.0,
            done: false,
            program: Program::Artist,
            title: TITLE,
            next: SceneId::Entry,
            gags,
        }
    }

    /// Abertura do Creative Writer: vai direto ao Escritório, sem animações.
    pub fn writer(click: Option<SoundRef>) -> Splash {
        Splash {
            program: Program::Writer,
            title: WRITER_TITLE,
            next: SceneId::Writer,
            gags: GagPlayer::new(&[], (0.3, 0.3), (0.5, 1.5)),
            ..Splash::new(click)
        }
    }
}

impl Scene for Splash {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        self.elapsed += dt;
        if !self.done {
            fx.extend(
                self.gags
                    .update(dt, assets)
                    .into_iter()
                    .map(Effect::PlaySound),
            );
        }
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
            fx.push(Effect::ChangeScene(self.next));
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        frame.fill([0, 0, 0]);
        let img = assets.image(&ImageRef::Pic(self.program, self.title));
        frame.blit(
            &img,
            (SCREEN_W as i32 - img.width as i32) / 2,
            (SCREEN_H as i32 - img.height as i32) / 2,
        );
        self.gags.draw(frame, assets);
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
    fn splash_click_goes_to_entry() {
        let (_d, a) = empty_assets();
        let mut s = Splash::new(None);
        assert!(
            s.update(&[click()], 0.016, &a)
                .contains(&Effect::ChangeScene(SceneId::Entry))
        );
    }

    #[test]
    fn splash_timeout_goes_to_entry_once() {
        let (_d, a) = empty_assets();
        let mut s = Splash::new(None);
        let mut changes = 0;
        for _ in 0..8 {
            changes += s
                .update(&[], 1.0, &a)
                .iter()
                .filter(|e| **e == Effect::ChangeScene(SceneId::Entry))
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
    fn writer_splash_goes_to_writer() {
        let (_d, a) = empty_assets();
        let mut s = Splash::writer(None);
        assert!(
            s.update(&[click()], 0.016, &a)
                .contains(&Effect::ChangeScene(SceneId::Writer))
        );
    }

    #[test]
    fn splash_without_data_runs_gags_quietly() {
        let (_d, a) = empty_assets();
        let mut s = Splash::new(None);
        for _ in 0..30 {
            let fx = s.update(&[], 0.1, &a);
            assert!(!fx.iter().any(|e| matches!(e, Effect::PlaySound(_))));
        }
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        s.draw(&mut f, &a);
    }

    #[test]
    fn splash_plays_bird_first_with_real_data() {
        let Some(a) = assets() else { return };
        let mut s = Splash::new(None);
        let mut sounds = Vec::new();
        for _ in 0..12 {
            sounds.extend(s.update(&[], 0.1, &a).into_iter().filter_map(|e| match e {
                Effect::PlaySound(snd) => Some(snd),
                _ => None,
            }));
        }
        // o pássaro (0x67) começa aos 0,3 s e sua primeira célula toca o som 0x217
        assert!(s.gags.is_playing());
        assert!(sounds.contains(&SoundRef::Snd(Program::Artist, 0x217)));
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        s.draw(&mut f, &a);
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
