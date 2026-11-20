//! Abertura: tela de título do Fine Artist ou do Creative Writer (clique ou
//! 5 s levam à Entrada do programa). Nos dois programas o título é parado e
//! mudo: no original do Fine Artist (gravação no Wine a ~10 quadros/s) ele
//! fica 3 s sem nenhum quadro diferente e vai à Entrada; o pássaro vermelho
//! (roteiro 0x67) e os outros roteiros 0x65–0x74, que o port tocava aqui por
//! suposição, não aparecem (`docs/telas/animacoes.md`).

use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{CwScene, Effect, Input, KidCommon, MouseButton, Scene, SceneId};
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
}

impl Splash {
    /// Abertura do Fine Artist.
    pub fn new(click: Option<SoundRef>) -> Splash {
        Splash {
            common: KidCommon::new(click),
            elapsed: 0.0,
            done: false,
            program: Program::Artist,
            title: TITLE,
            next: SceneId::Entry,
        }
    }

    /// Abertura do Creative Writer: vai à Entrada do Writer (o título também
    /// é estático: não há roteiros de título no `WRITER.EXE`).
    pub fn writer(click: Option<SoundRef>) -> Splash {
        Splash {
            program: Program::Writer,
            title: WRITER_TITLE,
            next: SceneId::Cw(CwScene::Entry),
            ..Splash::new(click)
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
    fn writer_splash_goes_to_writer_entry() {
        let (_d, a) = empty_assets();
        let mut s = Splash::writer(None);
        assert!(
            s.update(&[click()], 0.016, &a)
                .contains(&Effect::ChangeScene(SceneId::Cw(CwScene::Entry)))
        );
    }

    /// No original o título do Fine Artist fica parado e mudo até a Entrada
    /// (gravação no Wine: 3 s de título sem nenhum quadro diferente); o
    /// pássaro 0x67 e os outros roteiros 0x65–0x74 não tocam nele.
    #[test]
    fn artist_title_is_static_and_silent_with_real_data() {
        let Some(a) = assets() else { return };
        let mut s = Splash::new(None);
        let mut first = Frame::new(SCREEN_W, SCREEN_H);
        s.update(&[], 0.0, &a);
        s.draw(&mut first, &a);
        let mut t = 0.0;
        while t < TIMEOUT - 0.2 {
            let fx = s.update(&[], 0.1, &a);
            assert!(
                !fx.iter().any(|e| matches!(e, Effect::PlaySound(_))),
                "som aos {t:.1} s"
            );
            let mut f = Frame::new(SCREEN_W, SCREEN_H);
            s.draw(&mut f, &a);
            assert!(f.pixels() == first.pixels(), "quadro mudou aos {t:.1} s");
            t += 0.1;
        }
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
