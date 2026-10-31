//! Salas do prédio do Fine Artist: saguão, Ateliê de Pintura, Oficina de
//! Projetos e Truques de Desenho. Fundo e elementos fixos vêm de `rooms_data`
//! (medidos no original); as áreas clicáveis estão aqui.

use super::rooms_data::{self, Element};
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, SCREEN_H, SCREEN_W};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Floor {
    Lobby,
    Studio,
    Projects,
    Tricks,
}

type Rect = (i32, i32, i32, i32);

/// O que um clique numa área faz.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Go(SceneId),
    Floor(Floor),
    Quit,
    Soon(&'static str),
}

struct Hotspot {
    rect: Rect,
    action: Action,
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

impl Floor {
    fn background(self) -> u16 {
        match self {
            Floor::Lobby => rooms_data::LOBBY_BG,
            Floor::Studio => rooms_data::STUDIO_BG,
            Floor::Projects => rooms_data::PROJECTS_BG,
            Floor::Tricks => rooms_data::TRICKS_BG,
        }
    }

    fn elements(self) -> &'static [Element] {
        match self {
            Floor::Lobby => rooms_data::LOBBY,
            Floor::Studio => rooms_data::STUDIO,
            Floor::Projects => rooms_data::PROJECTS,
            Floor::Tricks => rooms_data::TRICKS,
        }
    }

    /// Áreas clicáveis, medidas no original.
    fn hotspots(self) -> Vec<Hotspot> {
        let h = |rect, action| Hotspot { rect, action };
        match self {
            Floor::Lobby => vec![
                h((98, 38, 90, 18), Action::Floor(Floor::Tricks)),
                h((98, 58, 90, 18), Action::Floor(Floor::Projects)),
                h((98, 78, 90, 18), Action::Floor(Floor::Studio)),
                h((98, 98, 90, 18), Action::Soon("Galeria")),
                h((210, 130, 260, 270), Action::Floor(Floor::Studio)),
                h((146, 290, 48, 20), Action::Quit),
                h((95, 316, 95, 52), Action::Soon("Galeria")),
            ],
            Floor::Studio => vec![
                h((110, 176, 170, 100), Action::Go(SceneId::Paint)),
                h((264, 63, 190, 145), Action::Soon("quadro da estante")),
                h((575, 0, 24, 395), Action::Floor(Floor::Projects)),
                h((474, 250, 46, 20), Action::Quit),
            ],
            Floor::Projects => vec![
                h((573, 0, 24, 395), Action::Floor(Floor::Tricks)),
                h((478, 185, 50, 20), Action::Quit),
                h((40, 20, 170, 150), Action::Soon("história em quadrinhos")),
                h((250, 0, 160, 120), Action::Soon("desenho animado")),
            ],
            Floor::Tricks => vec![
                h((539, 0, 24, 395), Action::Floor(Floor::Lobby)),
                h((564, 114, 50, 20), Action::Quit),
                h((330, 80, 190, 250), Action::Soon("truques de desenho 3D")),
                h((150, 40, 150, 100), Action::Soon("truques básicos")),
            ],
        }
    }

    pub fn number(self) -> u8 {
        match self {
            Floor::Lobby => 1,
            Floor::Studio => 2,
            Floor::Projects => 3,
            Floor::Tricks => 4,
        }
    }
}

pub struct Room {
    common: KidCommon,
    floor: Floor,
    hotspots: Vec<Hotspot>,
}

impl Room {
    pub fn new(floor: Floor, click: Option<SoundRef>) -> Room {
        Room {
            common: KidCommon::new(click),
            floor,
            hotspots: floor.hotspots(),
        }
    }

    pub fn floor(&self) -> Floor {
        self.floor
    }

    fn set_floor(&mut self, floor: Floor) {
        self.floor = floor;
        self.hotspots = floor.hotspots();
    }
}

/// Copia a imagem tratando branco puro como transparente (elementos sem máscara).
fn blit_white_transparent(f: &mut Frame, img: &crate::Image, x: i32, y: i32) {
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

impl Scene for Room {
    fn update(&mut self, input: &[Input], _dt: f32, _assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        for i in input {
            let Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } = *i
            else {
                continue;
            };
            let Some(hs) = self.hotspots.iter().find(|h| inside(h.rect, x, y)) else {
                continue;
            };
            match hs.action.clone() {
                Action::Go(id) => fx.push(Effect::ChangeScene(id)),
                Action::Floor(fl) => self.set_floor(fl),
                Action::Quit => fx.push(Effect::Quit),
                Action::Soon(what) => fx.push(Effect::Log(format!("{what}: ainda não disponível"))),
            }
            break;
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        frame.fill([0, 0, 0]);
        let bg = assets.image(&ImageRef::Pic(Program::Artist, self.floor.background()));
        frame.blit(
            &bg,
            (SCREEN_W as i32 - bg.width as i32) / 2,
            (SCREEN_H as i32 - bg.height as i32) / 2,
        );
        for e in self.floor.elements() {
            let img = assets.image(&ImageRef::Pic(Program::Artist, e.id));
            if e.mask != 0 {
                let mask = assets.image(&ImageRef::Pic(Program::Artist, e.mask));
                frame.blit_masked(&img, &mask, e.x, e.y);
            } else {
                blit_white_transparent(frame, &img, e.x, e.y);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    #[test]
    fn lobby_directory_and_pole_tour() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None);
        r.update(&[click(140, 86)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Studio);
        r.update(&[click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
        r.update(&[click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Tricks);
        r.update(&[click(550, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
    }

    #[test]
    fn easel_opens_paint_and_tchau_quits() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Studio, None);
        let fx = r.update(&[click(190, 220)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        let fx = r.update(&[click(496, 260)], 0.0, &a);
        assert!(fx.contains(&Effect::Quit));
        let fx = r.update(&[click(350, 100)], 0.0, &a);
        assert!(
            fx.iter()
                .any(|e| matches!(e, Effect::Log(s) if s.contains("não disponível")))
        );
    }

    #[test]
    fn draws_without_data() {
        let (_d, a) = empty_assets();
        let mut f = Frame::new(640, 480);
        for fl in [Floor::Lobby, Floor::Studio, Floor::Projects, Floor::Tricks] {
            Room::new(fl, None).draw(&mut f, &a);
        }
    }
}
