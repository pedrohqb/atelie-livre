//! Salas do prédio do Fine Artist: saguão, Ateliê de Pintura, Oficina de
//! Projetos e Truques de Desenho. Fundo e elementos fixos vêm de `rooms_data`
//! (medidos no original); as áreas clicáveis estão aqui.

use super::rooms_data::{self, Element};
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::Balloon;
use crate::paint::Canvas;
use crate::paint::store::{LAST_NAME, load_png};
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, SCREEN_H, SCREEN_W};
use formats::palette::Rgb;
use std::path::PathBuf;

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
    /// Quadro novo: apaga o último desenho e abre a pintura.
    NewPicture,
    /// Fecha o balão.
    Close,
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
                h((98, 98, 90, 18), Action::Go(SceneId::Gallery)),
                h((210, 130, 260, 270), Action::Floor(Floor::Studio)),
                h((146, 290, 48, 20), Action::Quit),
                h((95, 316, 95, 52), Action::Go(SceneId::Gallery)),
            ],
            Floor::Studio => vec![
                h((110, 176, 170, 100), Action::Go(SceneId::Paint)),
                h((264, 63, 190, 145), Action::Go(SceneId::Gallery)),
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

    /// Balão do McZee ou da Maggie ao chegar ao andar (textos do original).
    fn balloon(self) -> Balloon {
        let (x, y, w) = match self {
            Floor::Lobby => (300, 20, 300),
            Floor::Studio => (130, 230, 200),
            Floor::Projects => (70, 150, 300),
            Floor::Tricks => (70, 120, 260),
        };
        let (text, options): (&str, &[&str]) = match self {
            Floor::Lobby => (
                "Oi! Eu sou o McZee. O que você quer fazer?",
                &[
                    "Começar a desenhar por conta própria",
                    "Fazer uma história em quadrinhos, um botão ou adesivo, ou montar um Desenho Animado",
                    "Aprender alguns truques de desenho",
                    "Copiar, apagar ou limpar desenhos",
                ],
            ),
            Floor::Studio => (
                "Oba! Este é o Ateliê de Pintura! O que você quer fazer?",
                &[
                    "Pintar alguma coisa nova",
                    "Trabalhar na minha última obra",
                    "Trabalhar em um quadro mais antigo",
                    "Outra coisa?",
                ],
            ),
            Floor::Projects => (
                "Aqui é a Maggie! Estamos no Andar dos Projetos! Vamos fazer alguma coisa juntos?",
                &[
                    "É, uma história em quadrinhos",
                    "OK, vamos fazer um botão ou um adesivo",
                    "Quero fazer um desenho animado",
                    "Outra coisa?",
                ],
            ),
            Floor::Tricks => (
                "Seja bem-vindo ao andar dos Truques de Desenho! Eu vou te mostrar os meus truques secretos para desenhar. Com que truque você quer que eu comece?",
                &[
                    "Um truque básico",
                    "Fazendo uma coisa parecer tridimensional",
                    "Quero fazer outra coisa",
                ],
            ),
        };
        Balloon {
            x,
            y,
            w,
            text: text.into(),
            options: options.iter().map(|o| o.to_string()).collect(),
        }
    }

    /// Ação da opção `k` do balão.
    fn balloon_action(self, k: usize) -> Action {
        match (self, k) {
            (Floor::Lobby, 0) => Action::Floor(Floor::Studio),
            (Floor::Lobby, 1) => Action::Floor(Floor::Projects),
            (Floor::Lobby, 2) => Action::Floor(Floor::Tricks),
            (Floor::Lobby, _) => Action::Go(SceneId::Gallery),
            (Floor::Studio, 0) => Action::NewPicture,
            (Floor::Studio, 1) => Action::Go(SceneId::Paint),
            (Floor::Studio, 2) => Action::Go(SceneId::Gallery),
            (Floor::Projects, 0) => Action::Soon("história em quadrinhos"),
            (Floor::Projects, 1) => Action::Soon("botão ou adesivo"),
            (Floor::Projects, 2) => Action::Soon("desenho animado"),
            (Floor::Tricks, 0) => Action::Soon("truques básicos"),
            (Floor::Tricks, 1) => Action::Soon("truques 3D"),
            _ => Action::Close,
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

/// Quadro no cavalete do Ateliê: o último desenho, reduzido a 1/4.
const EASEL: (i32, i32) = (120, 179);
const EASEL_SCALE: u32 = 4;

pub struct Room {
    common: KidCommon,
    floor: Floor,
    hotspots: Vec<Hotspot>,
    last_drawing: Option<Canvas>,
    pal: Vec<Rgb>,
    drawings: Option<PathBuf>,
    balloon: Option<Balloon>,
}

impl Room {
    pub fn new(
        floor: Floor,
        click: Option<SoundRef>,
        drawings: Option<PathBuf>,
        assets: &Assets,
    ) -> Room {
        let pal = assets.palettes(Program::Artist).pal256.clone();
        let last_drawing = drawings
            .as_ref()
            .and_then(|d| load_png(&d.join(LAST_NAME), &pal));
        Room {
            common: KidCommon::new(click),
            hotspots: floor.hotspots(),
            last_drawing,
            pal,
            drawings,
            balloon: Some(floor.balloon()),
            floor,
        }
    }

    pub fn balloon_open(&self) -> bool {
        self.balloon.is_some()
    }

    fn act(&mut self, action: Action, fx: &mut Vec<Effect>) {
        match action {
            Action::Go(id) => fx.push(Effect::ChangeScene(id)),
            Action::Floor(fl) => self.set_floor(fl),
            Action::Quit => fx.push(Effect::Quit),
            Action::Close => self.balloon = None,
            Action::NewPicture => {
                if let Some(d) = &self.drawings {
                    let blank = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
                    if let Err(e) =
                        crate::paint::store::save_png(&blank, &self.pal, &d.join(LAST_NAME))
                    {
                        fx.push(Effect::Log(format!("quadro novo: {e}")));
                    }
                }
                fx.push(Effect::ChangeScene(SceneId::Paint));
            }
            Action::Soon(what) => {
                fx.push(Effect::Log(format!("{what}: ainda não disponível")));
                self.balloon = None;
            }
        }
    }

    pub fn floor(&self) -> Floor {
        self.floor
    }

    fn set_floor(&mut self, floor: Floor) {
        self.floor = floor;
        self.hotspots = floor.hotspots();
        self.balloon = Some(floor.balloon());
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
    fn update(&mut self, input: &[Input], _dt: f32, assets: &Assets) -> Vec<Effect> {
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
            if let Some(b) = &self.balloon {
                if let Some(k) = b.hit(assets, x, y) {
                    let action = self.floor.balloon_action(k);
                    self.act(action, &mut fx);
                    break;
                }
                if b.contains(assets, x, y) {
                    continue;
                }
                // clicar fora do balão só o fecha
                self.balloon = None;
                continue;
            }
            let Some(hs) = self.hotspots.iter().find(|h| inside(h.rect, x, y)) else {
                continue;
            };
            let action = hs.action.clone();
            self.act(action, &mut fx);
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
        if self.floor == Floor::Studio
            && let Some(c) = &self.last_drawing
        {
            // o desenho no cavalete, reduzido pelo vizinho mais próximo
            for y in 0..c.h / EASEL_SCALE {
                for x in 0..c.w / EASEL_SCALE {
                    let i = c.px[((y * EASEL_SCALE) * c.w + x * EASEL_SCALE) as usize];
                    let p = self
                        .pal
                        .get(i as usize)
                        .copied()
                        .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                    frame.put(EASEL.0 + x as i32, EASEL.1 + y as i32, [p.r, p.g, p.b]);
                }
            }
        }
        if let Some(b) = &self.balloon {
            b.draw(frame, assets);
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
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        assert!(r.balloon_open());
        // clique fora do balão fecha; depois os pontos quentes funcionam
        r.update(&[click(140, 300)], 0.0, &a);
        assert!(!r.balloon_open());
        r.update(&[click(140, 86)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Studio);
        assert!(r.balloon_open());
        r.update(&[click(600, 470), click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
        r.update(&[click(600, 470), click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Tricks);
        r.update(&[click(600, 470), click(550, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        // opção 1 do balão do saguão: Oficina de Projetos
        let b = r.balloon.as_ref().unwrap();
        let (bx, by, _, _) = b.option_rect(&a, 1);
        r.update(&[click(bx + 20, by + 3)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
    }

    #[test]
    fn easel_opens_paint_and_tchau_quits() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[click(600, 470)], 0.0, &a);
        let fx = r.update(&[click(190, 220)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        let fx = r.update(&[click(496, 260)], 0.0, &a);
        assert!(fx.contains(&Effect::Quit));
        let fx = r.update(&[click(350, 100)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Gallery)));
    }

    #[test]
    fn studio_shows_last_drawing_on_easel() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut c = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
        c.px.fill(255); // preto na rampa de cinzas dos testes
        crate::paint::store::save_drawing(
            dir.path(),
            &c,
            &a.palettes(Program::Artist).pal256,
            false,
            0,
        )
        .unwrap();
        let r = Room::new(Floor::Studio, None, Some(dir.path().to_path_buf()), &a);
        let mut f = Frame::new(640, 480);
        r.draw(&mut f, &a);
        assert_eq!(f.pixel(190, 220)[..3], [0, 0, 0]);
    }

    #[test]
    fn studio_balloon_new_picture_blanks_last() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = a.palettes(Program::Artist).pal256.clone();
        let mut c = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
        c.px.fill(255);
        crate::paint::store::save_drawing(dir.path(), &c, &pal, false, 0).unwrap();
        let mut r = Room::new(Floor::Studio, None, Some(dir.path().to_path_buf()), &a);
        let (bx, by, _, _) = r.balloon.as_ref().unwrap().option_rect(&a, 0);
        let fx = r.update(&[click(bx + 20, by + 3)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        let last = load_png(&dir.path().join(LAST_NAME), &pal).unwrap();
        assert!(last.px.iter().all(|&v| v == 0));
    }

    #[test]
    fn draws_without_data() {
        let (_d, a) = empty_assets();
        let mut f = Frame::new(640, 480);
        for fl in [Floor::Lobby, Floor::Studio, Floor::Projects, Floor::Tricks] {
            Room::new(fl, None, None, &a).draw(&mut f, &a);
        }
    }
}
