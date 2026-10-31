//! Rua de Imaginópolis: a porta do prédio leva ao saguão; clicar no hidrante
//! ou na lata de lixo toca a animação deles (roteiros `ATAG` 0x5dc e 0x672).

use super::gags::{HYDRANT_RECT, STREET_HYDRANT, STREET_TRASH, TRASH_RECT};
use crate::anim::{self, Animator};
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, SCREEN_H, SCREEN_W};

#[cfg(test)]
use crate::scene::DEFAULT_CURSOR;

pub const BACKGROUND: u16 = 0x6d01;
/// Porta do prédio (x, y, largura, altura), medida no original.
pub const DOOR: (i32, i32, i32, i32) = (296, 300, 88, 120);

fn inside(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

pub struct Street {
    common: KidCommon,
    /// Animação do hidrante ou da lata em curso.
    gag: Option<Animator>,
}

impl Street {
    pub fn new(click: Option<SoundRef>) -> Street {
        Street {
            common: KidCommon::new(click),
            gag: None,
        }
    }

    pub fn is_animating(&self) -> bool {
        self.gag.is_some()
    }

    /// Começa o roteiro `id` com a origem gravada somada ao ponto `anchor`.
    fn start_gag(&mut self, id: u16, anchor: (i32, i32), assets: &Assets, fx: &mut Vec<Effect>) {
        if self.gag.is_some() {
            return;
        }
        if let Some(mut a) = anim::load_exe(assets, Program::Artist, id) {
            a = a.with_offset(anchor);
            fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
            self.gag = Some(a);
        }
    }
}

impl Scene for Street {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        if let Some(a) = &mut self.gag {
            fx.extend(a.update(dt).into_iter().map(Effect::PlaySound));
            if a.is_finished() {
                self.gag = None;
            }
        }
        for i in input {
            let Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } = *i
            else {
                continue;
            };
            if inside(DOOR, x, y) {
                fx.push(Effect::ChangeScene(SceneId::Building));
            } else if inside(HYDRANT_RECT, x, y) {
                self.start_gag(STREET_HYDRANT.0, STREET_HYDRANT.1, assets, &mut fx);
            } else if inside(TRASH_RECT, x, y) {
                self.start_gag(STREET_TRASH.0, STREET_TRASH.1, assets, &mut fx);
            }
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        frame.fill([0, 0, 0]);
        let img = assets.image(&ImageRef::Pic(Program::Artist, BACKGROUND));
        frame.blit(
            &img,
            (SCREEN_W as i32 - img.width as i32) / 2,
            (SCREEN_H as i32 - img.height as i32) / 2,
        );
        if let Some(a) = &self.gag {
            a.draw(frame, assets);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    #[test]
    fn street_door_opens_building() {
        let (_d, a) = empty_assets();
        let mut s = Street::new(None);
        let fx = s.update(
            &[Input::Click {
                button: MouseButton::Left,
                x: 338,
                y: 370,
            }],
            0.0,
            &a,
        );
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Building)));
    }

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

    #[test]
    fn hydrant_click_without_data_does_nothing() {
        let (_d, a) = empty_assets();
        let mut s = Street::new(None);
        s.update(
            &[Input::Click {
                button: MouseButton::Left,
                x: 500,
                y: 400,
            }],
            0.0,
            &a,
        );
        assert!(!s.is_animating());
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        s.draw(&mut f, &a);
    }

    #[test]
    fn hydrant_click_plays_script_with_real_data() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut s = Street::new(None);
        let fx = s.update(
            &[Input::Click {
                button: MouseButton::Left,
                x: 500,
                y: 400,
            }],
            0.0,
            &a,
        );
        assert!(s.is_animating());
        // primeira célula do hidrante toca o som 0x2bc
        assert!(fx.contains(&Effect::PlaySound(SoundRef::Snd(Program::Artist, 0x2bc))));
        let pos = s.gag.as_ref().unwrap().position();
        assert_eq!(pos, (479, 383));
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        s.draw(&mut f, &a);
        s.update(&[], 10.0, &a);
        assert!(!s.is_animating());
    }
}
