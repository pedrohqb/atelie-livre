//! Rua diante do Museu Mágico (fundo `6d01`). Não há área morta:
//!
//! - a porta giratória gira (roteiro `ATAG` 0x640, 1,8 s) e leva ao saguão;
//! - a fachada em volta da porta e o toldo levam direto ao saguão;
//! - o resto da fachada e a calçada da frente tocam o gag da bola de
//!   beisebol (0x5aa: três bolas racham três janelas, 5,85 s);
//! - o hidrante e a lata de lixo tocam a animação deles (0x5dc e 0x672);
//! - qualquer outro ponto (céu, prédios do lado, rua, amarelinha) volta a
//!   Imaginópolis, que toca o 0x302 e o carrinho do McZee.
//!
//! Enquanto um roteiro toca, os cliques são ignorados, como no original.
//! As áreas vêm da tabela de pontos quentes da rua em `ARTIST.EXE`
//! (`docs/telas/fine-artist.md`, "Rua").

use super::gags::{GagPlayer, HYDRANT_RECT, STREET_HYDRANT, STREET_TRASH, TRASH_RECT};
use crate::anim::{self, Animator};
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::rng::XorShift;
use crate::scene::{Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::text_balloon::blit_pic_in;
use crate::ui::{Rect, inside};
use crate::{Frame, SCREEN_H, SCREEN_W};

pub const BACKGROUND: u16 = 0x6d01;
/// Porta giratória (x, y, largura, altura); o canto é também o ponto do
/// roteiro 0x640 (origem (0,−2): a porta é desenhada em (296,313)).
pub const DOOR: (i32, i32, i32, i32) = (296, 315, 89, 110);
/// Fachada em volta da porta e toldo: vão direto ao saguão.
pub const FACADE: (i32, i32, i32, i32) = (224, 280, 221, 141);
pub const AWNING: (i32, i32, i32, i32) = (289, 264, 100, 46);
/// Faixas do gag da bola: alto da fachada, meio e a parte de baixo com a
/// calçada da frente (a de baixo fica por trás da fachada e da porta).
pub const BALL_RECTS: [(i32, i32, i32, i32); 3] = [
    (195, 23, 260, 92),
    (194, 115, 250, 75),
    (209, 189, 250, 278),
];
/// Roteiros da porta giratória e da bola de beisebol.
pub const DOOR_SCRIPT: u16 = 0x640;
pub const BALL_SCRIPT: u16 = 0x5aa;
/// Som do clique na fachada (6700 bytes).
pub const SND_FACADE: u16 = 0x301;
/// Som da troca para o saguão (1068 bytes; 0x2c0–0x2c5 são o mesmo som).
pub const SND_ENTER: u16 = 0x253;

/// Bichos que passam pela rua quando ninguém mexe: espera antes do
/// primeiro e entre os outros (s), como nas salas.
const GAG_FIRST: (f32, f32) = (20.0, 30.0);
const GAG_BETWEEN: (f32, f32) = (20.0, 40.0);

/// O que muda na rua de um programa (ids do `.PIC`/`.SND`/roteiros dele).
/// Um roteiro com âncora começa com a origem gravada somada à âncora.
#[derive(Debug, Clone, Copy)]
pub struct StreetSpec {
    pub program: Program,
    pub background: u16,
    /// Figuras por cima do fundo (cor, canto; máscara duas figuras adiante).
    pub elements: &'static [(u16, (i32, i32))],
    /// Porta giratória: roteiro e área (o canto é a âncora).
    pub door: (u16, Rect),
    /// Fachada em volta da porta: direto ao saguão, com `snd_lobby`.
    pub lobby: &'static [Rect],
    /// Bola de beisebol: roteiro, âncora e faixas da fachada.
    pub ball: (u16, (i32, i32), &'static [Rect]),
    pub hydrant: (u16, (i32, i32), Rect),
    pub trash: (u16, (i32, i32), Rect),
    /// Bichos que passam sozinhos (nenhum no Fine Artist).
    pub gags: &'static [u16],
    /// Clique na fachada da bola e na que leva ao saguão.
    pub snd_ball: u16,
    pub snd_lobby: u16,
    /// Troca para o saguão.
    pub snd_enter: u16,
}

/// A rua do Fine Artist.
pub const ARTIST_STREET: StreetSpec = StreetSpec {
    program: Program::Artist,
    background: BACKGROUND,
    elements: &[],
    door: (DOOR_SCRIPT, DOOR),
    lobby: &[AWNING, FACADE],
    ball: (BALL_SCRIPT, (0, 0), &BALL_RECTS),
    hydrant: (STREET_HYDRANT.0, STREET_HYDRANT.1, HYDRANT_RECT),
    trash: (STREET_TRASH.0, STREET_TRASH.1, TRASH_RECT),
    gags: &[],
    snd_ball: SND_FACADE,
    snd_lobby: SND_FACADE,
    snd_enter: SND_ENTER,
};

impl StreetSpec {
    /// Classifica o ponto (x, y); a ordem é a do original, de cima para baixo.
    pub fn spot_at(&self, x: i32, y: i32) -> Spot {
        if inside(self.hydrant.2, x, y) {
            Spot::Hydrant
        } else if inside(self.trash.2, x, y) {
            Spot::Trash
        } else if inside(self.door.1, x, y) {
            Spot::Door
        } else if self.lobby.iter().any(|r| inside(*r, x, y)) {
            Spot::Lobby
        } else if self.ball.2.iter().any(|r| inside(*r, x, y)) {
            Spot::Ball
        } else {
            Spot::Town
        }
    }
}

/// O que um clique na rua faz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spot {
    Door,
    Lobby,
    Ball,
    Hydrant,
    Trash,
    /// Fora do prédio e da calçada da frente.
    Town,
}

/// Classifica o ponto (x, y) na rua do Fine Artist.
pub fn spot_at(x: i32, y: i32) -> Spot {
    ARTIST_STREET.spot_at(x, y)
}

pub struct Street {
    spec: &'static StreetSpec,
    common: KidCommon,
    /// Bichos que passam sozinhos (só no Writer).
    idle: GagPlayer,
    /// Roteiro em curso (porta, bola, hidrante ou lata).
    gag: Option<Animator>,
    /// Ao fim do roteiro, entra no saguão (porta giratória).
    then_lobby: bool,
    /// Âncora do roteiro em curso: a figura fixa nesse canto (hidrante,
    /// lata do Writer) some enquanto o roteiro a desenha.
    hidden: Option<(i32, i32)>,
    /// Programa: a porta leva ao saguão dele; o resto, à Imaginópolis dele.
    program: Program,
}

impl Street {
    /// Rua do Fine Artist.
    pub fn new(click: Option<SoundRef>) -> Street {
        Street::for_program(Program::Artist, click)
    }

    /// Rua do programa `p` (`ARTIST_STREET` ou `writer_street::WRITER_STREET`).
    pub fn for_program(p: Program, click: Option<SoundRef>) -> Street {
        let spec = match p {
            Program::Artist => &ARTIST_STREET,
            Program::Writer => &super::writer_street::WRITER_STREET,
        };
        Street {
            spec,
            idle: GagPlayer::with_rng(spec.gags, GAG_FIRST, GAG_BETWEEN, XorShift::from_clock())
                .in_program(spec.program),
            program: p,
            common: KidCommon::new(click),
            gag: None,
            then_lobby: false,
            hidden: None,
        }
    }

    pub fn is_animating(&self) -> bool {
        self.gag.is_some()
    }

    /// Começa o roteiro `id` com a origem gravada somada ao ponto `anchor`;
    /// `false` se o roteiro não carregou.
    fn start_gag(
        &mut self,
        id: u16,
        anchor: (i32, i32),
        assets: &Assets,
        fx: &mut Vec<Effect>,
    ) -> bool {
        let Some(a) = anim::load_exe(assets, self.spec.program, id) else {
            return false;
        };
        let mut a = a.with_offset(anchor);
        fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
        self.gag = Some(a);
        self.hidden = Some(anchor);
        true
    }

    fn snd(&self, id: u16) -> Effect {
        Effect::PlaySound(SoundRef::Snd(self.spec.program, id))
    }

    fn enter_lobby(&self, fx: &mut Vec<Effect>) {
        fx.push(self.snd(self.spec.snd_enter));
        fx.push(Effect::ChangeScene(
            SceneId::Building.in_program(self.program),
        ));
    }

    fn click(&mut self, x: i32, y: i32, assets: &Assets, fx: &mut Vec<Effect>) {
        let spec = self.spec;
        match spec.spot_at(x, y) {
            Spot::Door => {
                let (id, r) = spec.door;
                // sem os dados da porta, entra direto
                if self.start_gag(id, (r.0, r.1), assets, fx) {
                    self.then_lobby = true;
                } else {
                    self.enter_lobby(fx);
                }
            }
            Spot::Lobby => {
                fx.push(self.snd(spec.snd_lobby));
                self.enter_lobby(fx);
            }
            Spot::Ball => {
                fx.push(self.snd(spec.snd_ball));
                // no FA a origem é absoluta: o roteiro já está em coordenadas da tela
                self.start_gag(spec.ball.0, spec.ball.1, assets, fx);
            }
            Spot::Hydrant => {
                self.start_gag(spec.hydrant.0, spec.hydrant.1, assets, fx);
            }
            Spot::Trash => {
                self.start_gag(spec.trash.0, spec.trash.1, assets, fx);
            }
            // o som e o carrinho são de Imaginópolis, ao chegar
            Spot::Town => fx.push(Effect::ChangeScene(
                SceneId::Imaginopolis.in_program(self.program),
            )),
        }
    }
}

impl Scene for Street {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        if input
            .iter()
            .any(|i| matches!(i, Input::Click { .. } | Input::Key { .. }))
        {
            self.idle.poke();
        }
        if self.gag.is_none() || self.idle.is_playing() {
            fx.extend(
                self.idle
                    .update(dt, assets)
                    .into_iter()
                    .map(Effect::PlaySound),
            );
        }
        if let Some(a) = &mut self.gag {
            fx.extend(a.update(dt).into_iter().map(Effect::PlaySound));
            if a.is_finished() {
                self.gag = None;
                self.hidden = None;
                if std::mem::take(&mut self.then_lobby) {
                    self.enter_lobby(&mut fx);
                    return fx;
                }
            }
            // durante um roteiro, os cliques são ignorados
            return fx;
        }
        for i in input {
            if let Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } = *i
            {
                self.click(x, y, assets, &mut fx);
                // um clique por vez: o que vem depois cai no roteiro ou na
                // tela seguinte
                if self.gag.is_some() || fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))) {
                    break;
                }
            }
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        frame.fill([0, 0, 0]);
        let p = self.spec.program;
        let img = assets.image(&ImageRef::Pic(p, self.spec.background));
        frame.blit(
            &img,
            (SCREEN_W as i32 - img.width as i32) / 2,
            (SCREEN_H as i32 - img.height as i32) / 2,
        );
        for &(id, (x, y)) in self.spec.elements {
            if self.gag.is_none() || self.hidden != Some((x, y)) {
                blit_pic_in(frame, assets, p, id, x, y, true);
            }
        }
        self.idle.draw(frame, assets);
        if let Some(a) = &self.gag {
            a.draw(frame, assets);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::DEFAULT_CURSOR;
    use crate::scenes::testutil::{assets, empty_assets};

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn changes(fx: &[Effect]) -> Vec<SceneId> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::ChangeScene(s) => Some(*s),
                _ => None,
            })
            .collect()
    }

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(SoundRef::Snd(_, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// Fronteiras conferidas uma a uma no original (`report/street.md`).
    #[test]
    fn spots_match_the_original_boundaries() {
        for (x, y) in [(472, 392), (480, 360), (200, 200), (464, 240), (464, 432)] {
            assert_eq!(spot_at(x, y), Spot::Town, "({x},{y})");
        }
        // céu, rua, amarelinha, prédios dos lados, cornija
        for (x, y) in [
            (10, 10),
            (320, 10),
            (320, 475),
            (100, 450),
            (600, 200),
            (560, 300),
        ] {
            assert_eq!(spot_at(x, y), Spot::Town, "({x},{y})");
        }
        for (x, y) in [(456, 392), (216, 450), (320, 60), (400, 150), (250, 250)] {
            assert_eq!(spot_at(x, y), Spot::Ball, "({x},{y})");
        }
        for (x, y) in [(224, 350), (232, 300), (440, 300), (440, 420), (330, 270)] {
            assert_eq!(spot_at(x, y), Spot::Lobby, "({x},{y})");
        }
        for (x, y) in [(338, 370), (300, 320), (380, 420)] {
            assert_eq!(spot_at(x, y), Spot::Door, "({x},{y})");
        }
        // grade manual do original em volta do hidrante e da lata
        for (x, y) in [(496, 400), (528, 400), (496, 432), (528, 432)] {
            assert_eq!(spot_at(x, y), Spot::Hydrant, "({x},{y})");
        }
        for (x, y) in [(464, 400), (560, 400), (528, 368), (144, 368), (56, 344)] {
            assert_eq!(spot_at(x, y), Spot::Town, "({x},{y})");
        }
        assert_eq!(spot_at(100, 360), Spot::Trash);
    }

    #[test]
    fn outside_goes_back_to_town() {
        let (_d, a) = empty_assets();
        let mut s = Street::new(None);
        let fx = s.update(&[click(320, 10)], 0.0, &a);
        assert_eq!(changes(&fx), vec![SceneId::Imaginopolis]);
        assert!(sounds(&fx).is_empty(), "o 0x302 é de Imaginópolis");
    }

    #[test]
    fn facade_enters_lobby_with_sounds() {
        let (_d, a) = empty_assets();
        let mut s = Street::new(None);
        let fx = s.update(&[click(232, 300)], 0.0, &a);
        assert_eq!(changes(&fx), vec![SceneId::Building]);
        assert_eq!(sounds(&fx), vec![SND_FACADE, SND_ENTER]);
    }

    #[test]
    fn door_without_data_enters_at_once() {
        let (_d, a) = empty_assets();
        let mut s = Street::new(None);
        let fx = s.update(&[click(338, 370)], 0.0, &a);
        assert_eq!(changes(&fx), vec![SceneId::Building]);
        assert_eq!(sounds(&fx), vec![SND_ENTER]);
    }

    #[test]
    fn click_sound_and_cursor_still_come_from_common() {
        let (_d, a) = empty_assets();
        let snd = SoundRef::Snd(Program::Artist, 0x20a);
        let mut s = Street::new(Some(snd.clone()));
        let fx = s.update(&[click(500, 400)], 0.0, &a);
        assert!(fx.contains(&Effect::PlaySound(snd)));
        assert!(fx.contains(&Effect::SetCursor(DEFAULT_CURSOR)));
        assert!(changes(&fx).is_empty());
        assert!(s.update(&[], 100.0, &a).is_empty());
    }

    #[test]
    fn hydrant_click_without_data_does_nothing() {
        let (_d, a) = empty_assets();
        let mut s = Street::new(None);
        s.update(&[click(500, 400)], 0.0, &a);
        assert!(!s.is_animating());
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        s.draw(&mut f, &a);
    }

    #[test]
    fn hydrant_click_plays_script_with_real_data() {
        let Some(a) = assets() else {
            return;
        };
        let mut s = Street::new(None);
        let fx = s.update(&[click(500, 400)], 0.0, &a);
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

    /// Porta giratória: 9 células de 200 ms em (296,313), som 0x2f9 no
    /// começo; só ao fim entra no saguão, com o 0x253.
    #[test]
    fn revolving_door_spins_then_enters_with_real_data() {
        let Some(a) = assets() else { return };
        let mut s = Street::new(None);
        let fx = s.update(&[click(338, 370)], 0.0, &a);
        assert!(changes(&fx).is_empty());
        assert_eq!(sounds(&fx), vec![0x2f9]);
        assert_eq!(s.gag.as_ref().unwrap().position(), (296, 313));
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        s.update(&[], 0.1, &a);
        s.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "street_door");
        let mut t = 0.1;
        let mut entered = None;
        while t < 3.0 {
            let fx = s.update(&[], 0.1, &a);
            t += 0.1;
            if !changes(&fx).is_empty() {
                assert_eq!(changes(&fx), vec![SceneId::Building]);
                assert_eq!(sounds(&fx), vec![SND_ENTER]);
                entered = Some(t);
                break;
            }
        }
        let t = entered.expect("entrou no saguão");
        assert!((1.75..1.95).contains(&t), "{t}");
    }

    /// Bola de beisebol: 0x301 no clique; três arremessos com 0x2e9 e a
    /// janela rachada com 0x25c; 5,85 s; cliques ignorados durante o gag.
    #[test]
    fn baseball_cracks_three_windows_with_real_data() {
        let Some(a) = assets() else { return };
        let mut s = Street::new(None);
        let mut all = sounds(&s.update(&[click(320, 60)], 0.0, &a));
        assert!(s.is_animating());
        let mut cracks = Vec::new();
        let mut t = 0.0;
        while s.is_animating() {
            // céu e porta clicados durante o gag não fazem nada
            let fx = s.update(&[click(10, 10), click(338, 370)], 0.05, &a);
            t += 0.05;
            assert!(changes(&fx).is_empty());
            let snd = sounds(&fx);
            if snd.contains(&0x25c) {
                cracks.push(s.gag.as_ref().unwrap().position());
            }
            all.extend(snd);
            assert!(t < 7.0);
        }
        assert_eq!(
            all,
            vec![SND_FACADE, 0x2e9, 0x25c, 0x2e9, 0x25c, 0x2e9, 0x25c]
        );
        assert_eq!(cracks, vec![(254, 246), (390, 151), (247, 128)]);
        assert!((5.8..5.95).contains(&t), "{t}");
        // acabou: a rua volta a responder
        let fx = s.update(&[click(10, 10)], 0.05, &a);
        assert_eq!(changes(&fx), vec![SceneId::Imaginopolis]);
    }

    /// Durante o hidrante, a porta não abre (no port antigo abria).
    #[test]
    fn clicks_ignored_during_hydrant_with_real_data() {
        let Some(a) = assets() else { return };
        let mut s = Street::new(None);
        s.update(&[click(500, 400)], 0.0, &a);
        let fx = s.update(&[click(338, 370)], 0.1, &a);
        assert!(changes(&fx).is_empty());
        assert!(!sounds(&fx).contains(&0x2f9));
    }
}
