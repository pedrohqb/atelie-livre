//! Escolha do programa ao abrir: Fine Artist ou Creative Writer, com as telas
//! de título reduzidas à metade, lado a lado (tela nossa, não do original).

use crate::assets::{Assets, ImageRef, Program};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, Image};

const ARTIST_TITLE: u16 = 0x6ca7;
const WRITER_TITLE: u16 = 0x6737;
const CARD_W: i32 = 300;
const CARD_H: i32 = 225;
const LEFT: (i32, i32) = (12, 120);
const RIGHT: (i32, i32) = (328, 120);
const BG: [u8; 3] = [136, 136, 230];
const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const YELLOW: [u8; 3] = [255, 238, 76];

pub struct Chooser {
    common: KidCommon,
    hover: Option<usize>,
}

impl Chooser {
    pub fn new() -> Chooser {
        Chooser {
            common: KidCommon::new(None),
            hover: None,
        }
    }

    fn card(k: usize) -> (i32, i32, i32, i32) {
        let (x, y) = if k == 0 { LEFT } else { RIGHT };
        (x, y, CARD_W, CARD_H)
    }

    fn hit(x: i32, y: i32) -> Option<usize> {
        (0..2).find(|&k| {
            let r = Self::card(k);
            x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
        })
    }

    /// Copia a imagem reduzida por `div` (vizinho mais próximo).
    fn blit_scaled(f: &mut Frame, img: &Image, x: i32, y: i32, div: u32, max: (u32, u32)) {
        for dy in 0..(img.height / div).min(max.1) {
            for dx in 0..(img.width / div).min(max.0) {
                let i = (((dy * div) * img.width + dx * div) * 4) as usize;
                let p = &img.rgba[i..i + 3];
                f.put(x + dx as i32, y + dy as i32, [p[0], p[1], p[2]]);
            }
        }
    }
}

impl Default for Chooser {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for Chooser {
    fn update(&mut self, input: &[Input], _dt: f32, _assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        for i in input {
            match *i {
                Input::MouseMove { x, y } => self.hover = Self::hit(x, y),
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => match Self::hit(x, y) {
                    Some(0) => fx.push(Effect::ChangeScene(SceneId::Splash)),
                    Some(_) => fx.push(Effect::ChangeScene(SceneId::WriterSplash)),
                    None => {}
                },
                _ => {}
            }
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        frame.fill(BG);
        let title = "Ateliê Livre";
        let sub = "Clique no programa que você quer usar";
        match assets.font(16) {
            Some(fo) => {
                let w = fo.width(title) as i32;
                frame.text_font(&fo, 320 - w / 2, 30, title, BLACK);
            }
            None => frame.text(270, 34, title, BLACK),
        }
        match assets.font(8) {
            Some(fo) => {
                let w = fo.width(sub) as i32;
                frame.text_font(&fo, 320 - w / 2, 70, sub, BLACK);
            }
            None => frame.text(170, 74, sub, BLACK),
        }
        for (k, id, label) in [
            (0, ARTIST_TITLE, "Fine Artist"),
            (1, WRITER_TITLE, "Creative Writer"),
        ] {
            let r = Self::card(k);
            let hot = self.hover == Some(k);
            frame.rect(
                r.0 - 4,
                r.1 - 4,
                r.2 + 8,
                r.3 + 8,
                if hot { YELLOW } else { BLACK },
            );
            frame.rect(r.0 - 2, r.1 - 2, r.2 + 4, r.3 + 4, WHITE);
            let program = if k == 0 {
                Program::Artist
            } else {
                Program::Writer
            };
            let img = assets.image(&ImageRef::Pic(program, id));
            Self::blit_scaled(frame, &img, r.0, r.1, 2, (r.2 as u32, r.3 as u32));
            match assets.font(8) {
                Some(fo) => {
                    let w = fo.width(label) as i32;
                    frame.text_font(&fo, r.0 + r.2 / 2 - w / 2, r.1 + r.3 + 12, label, BLACK);
                }
                None => frame.text(r.0 + 100, r.1 + r.3 + 14, label, BLACK),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    #[test]
    fn cards_choose_program() {
        let (_d, a) = empty_assets();
        let mut c = Chooser::new();
        let click = |x, y| Input::Click {
            button: MouseButton::Left,
            x,
            y,
        };
        let fx = c.update(&[click(100, 200)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Splash)));
        let fx = c.update(&[click(400, 200)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::WriterSplash)));
        let fx = c.update(&[click(320, 460)], 0.0, &a);
        assert!(!fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))));
        let mut f = Frame::new(640, 480);
        c.draw(&mut f, &a);
    }
}
