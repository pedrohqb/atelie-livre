//! Animações ambientes das telas do Fine Artist ("gags"): quais roteiros
//! `ATAG` de `ARTIST.EXE` tocam em cada tela, os personagens parados das
//! salas e o sorteador que os toca de tempos em tempos. O mapeamento e a
//! confiança de cada item estão em `docs/telas/animacoes.md`.

use crate::Frame;
use crate::anim::{self, Animator, blit_masked};
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::rng::XorShift;

/// Roteiros do título (recursos 0x65–0x74): porco, pássaro, gafanhoto,
/// patinador e McZee saindo do buraco com onze chapéus.
pub const TITLE_SCRIPTS: &[u16] = &[
    0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74,
];
/// Pássaro atravessando o título (origem (628, 134)).
pub const TITLE_BIRD: u16 = 0x67;

/// Saguão (0x324–0x328): canguru, pássaro, tiranossauro, patinador e a
/// menina de skate. Conferido no original (canguru).
pub const LOBBY_SCRIPTS: &[u16] = &[0x324, 0x325, 0x326, 0x327, 0x328];
/// Ateliê de Pintura (0x579–0x57e): provável, por eliminação.
pub const STUDIO_SCRIPTS: &[u16] = &[0x579, 0x57a, 0x57b, 0x57c, 0x57d, 0x57e];
/// Oficina de Projetos (0x45c–0x469): dragão, pássaro, patinador e McZee
/// saindo do buraco da parede com chapéus. Conferido no original (chapéus).
pub const PROJECTS_SCRIPTS: &[u16] = &[
    0x45c, 0x45d, 0x45e, 0x45f, 0x460, 0x461, 0x462, 0x463, 0x464, 0x465, 0x466, 0x467, 0x468,
    0x469,
];
/// Truques de Desenho (0x6a6–0x6ab): provável, por eliminação.
pub const TRICKS_SCRIPTS: &[u16] = &[0x6a6, 0x6a7, 0x6a8, 0x6a9, 0x6aa, 0x6ab];

/// Fantasias do McZee (imagens coloridas de 149×354 de `ARTIST.PIC`); a
/// versão de 1 bit e a máscara são as duas imagens seguintes.
pub const MCZEE_COSTUMES: &[u16] = &[
    0x6d93, 0x6d9f, 0x6da3, 0x6da7, 0x6dab, 0x6daf, 0x6db3, 0x6db7, 0x6dbb, 0x6dbf, 0x6dc3, 0x6dc7,
    0x6dcb, 0x6dcf,
];
/// Posição do McZee no saguão e no Ateliê (medida no original).
pub const MCZEE_LOBBY: (i32, i32) = (-28, 129);
pub const MCZEE_STUDIO: (i32, i32) = (-18, 129);

/// Poses da Maggie nos andares 3 e 4 (imagens coloridas; 1 bit e máscara
/// são as duas seguintes).
pub const MAGGIE_POSES: &[u16] = &[0x6d5f, 0x6d6b, 0x6d6f, 0x6d73];
/// Posição da Maggie na Oficina de Projetos e nos Truques (medida no original).
pub const MAGGIE_POS: (i32, i32) = (-2, 253);

/// Rua: hidrante (roteiro 0x5dc) e lata de lixo (0x672), gravados em relação
/// a um ponto; o ponto foi medido no original.
pub const STREET_HYDRANT: (u16, (i32, i32)) = (0x5dc, (489, 387));
pub const STREET_TRASH: (u16, (i32, i32)) = (0x672, (69, 333));
/// Áreas clicáveis do hidrante e da lata (x, y, largura, altura).
pub const HYDRANT_RECT: (i32, i32, i32, i32) = (470, 370, 60, 80);
pub const TRASH_RECT: (i32, i32, i32, i32) = (60, 330, 90, 80);

/// Personagem parado de uma sala: figura colorida com a máscara duas
/// imagens adiante no `.PIC`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    pub image: u16,
    pub x: i32,
    pub y: i32,
}

impl Character {
    /// Sorteia a fantasia (ou pose) entre `choices`.
    pub fn pick(choices: &[u16], pos: (i32, i32), rng: &mut XorShift) -> Option<Character> {
        let image = *choices.get(rng.below(choices.len()))?;
        Some(Character {
            image,
            x: pos.0,
            y: pos.1,
        })
    }

    pub fn draw(&self, frame: &mut Frame, assets: &Assets) {
        let img = assets.image(&ImageRef::Pic(Program::Artist, self.image));
        let mask = assets
            .pic_next_id(Program::Artist, self.image, 2)
            .map(|m| assets.image(&ImageRef::Pic(Program::Artist, m)))
            .filter(|m| m.height == img.height && m.width.abs_diff(img.width) <= 1);
        match mask {
            Some(m) => blit_masked(frame, &img, &m, self.x, self.y),
            None => frame.blit(&img, self.x, self.y),
        }
    }
}

/// Toca um roteiro de cada vez, sorteado da lista, com pausas aleatórias.
pub struct GagPlayer {
    scripts: Vec<u16>,
    program: Program,
    rng: XorShift,
    current: Option<Animator>,
    /// Segundos até o próximo roteiro.
    wait: f32,
    /// Intervalo (mín, máx) entre roteiros.
    between: (f32, f32),
    /// Próximo roteiro a tocar, se já escolhido (ex.: o pássaro do título).
    next: Option<u16>,
    /// Quantos roteiros já começaram (para testes e depuração).
    started: usize,
}

impl GagPlayer {
    /// `first`: espera antes do primeiro; `between`: pausa entre os demais.
    pub fn new(scripts: &[u16], first: (f32, f32), between: (f32, f32)) -> GagPlayer {
        GagPlayer::with_rng(scripts, first, between, XorShift::from_clock())
    }

    pub fn with_rng(
        scripts: &[u16],
        first: (f32, f32),
        between: (f32, f32),
        mut rng: XorShift,
    ) -> GagPlayer {
        let wait = rng.range(first.0, first.1);
        GagPlayer {
            scripts: scripts.to_vec(),
            program: Program::Artist,
            rng,
            current: None,
            wait,
            between,
            next: None,
            started: 0,
        }
    }

    /// Fixa o próximo roteiro em vez de sortear.
    pub fn queue(&mut self, id: u16) {
        self.next = Some(id);
    }

    pub fn is_playing(&self) -> bool {
        self.current.is_some()
    }

    pub fn started(&self) -> usize {
        self.started
    }

    pub fn update(&mut self, dt: f32, assets: &Assets) -> Vec<SoundRef> {
        if let Some(a) = &mut self.current {
            let sounds = a.update(dt);
            if a.is_finished() {
                self.current = None;
                self.wait = self.rng.range(self.between.0, self.between.1);
            }
            return sounds;
        }
        self.wait -= dt;
        if self.wait > 0.0 || self.scripts.is_empty() {
            return Vec::new();
        }
        let id = self
            .next
            .take()
            .unwrap_or_else(|| self.scripts[self.rng.below(self.scripts.len())]);
        self.started += 1;
        match anim::load_exe(assets, self.program, id) {
            Some(mut a) => {
                let sounds = a.update(0.0);
                self.current = Some(a);
                sounds
            }
            None => {
                // sem o recurso (dados ausentes): tenta de novo mais tarde
                self.wait = self.rng.range(self.between.0, self.between.1);
                Vec::new()
            }
        }
    }

    pub fn draw(&self, frame: &mut Frame, assets: &Assets) {
        if let Some(a) = &self.current {
            a.draw(frame, assets);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};
    use formats::atag;

    #[test]
    fn player_without_resources_keeps_trying_without_panicking() {
        let (_d, a) = empty_assets();
        let mut g = GagPlayer::with_rng(LOBBY_SCRIPTS, (0.0, 0.0), (0.1, 0.1), XorShift::new(3));
        for _ in 0..20 {
            assert!(g.update(0.5, &a).is_empty());
            assert!(!g.is_playing());
        }
        assert!(g.started() >= 10);
        let mut f = Frame::new(640, 480);
        g.draw(&mut f, &a);
        let mut empty = GagPlayer::new(&[], (0.0, 0.0), (0.0, 0.0));
        assert!(empty.update(1.0, &a).is_empty());
    }

    #[test]
    fn character_draws_placeholder_without_data() {
        let (_d, a) = empty_assets();
        let mut rng = XorShift::new(1);
        let c = Character::pick(MCZEE_COSTUMES, MCZEE_LOBBY, &mut rng).unwrap();
        assert!(MCZEE_COSTUMES.contains(&c.image));
        let mut f = Frame::new(640, 480);
        c.draw(&mut f, &a);
        assert!(Character::pick(&[], (0, 0), &mut rng).is_none());
    }

    /// Com os dados reais: todos os roteiros escolhidos decodificam e as
    /// imagens (colorida e máscara) existem no `.PIC`.
    #[test]
    fn real_scripts_parse_and_images_exist() {
        let Some(a) = assets() else { return };
        let all: Vec<u16> = TITLE_SCRIPTS
            .iter()
            .chain(LOBBY_SCRIPTS)
            .chain(STUDIO_SCRIPTS)
            .chain(PROJECTS_SCRIPTS)
            .chain(TRICKS_SCRIPTS)
            .chain([STREET_HYDRANT.0, STREET_TRASH.0].iter())
            .copied()
            .collect();
        for id in all {
            let data = a
                .exe_resource(Program::Artist, "ATAG", id)
                .unwrap_or_else(|| panic!("ATAG {id:#x} ausente"));
            let sc = atag::parse(&data).unwrap_or_else(|e| panic!("ATAG {id:#x}: {e}"));
            assert_eq!(sc.path_kind, 5, "ATAG {id:#x}");
            for c in &sc.cells {
                let ci = anim::cell_images(&a, Program::Artist, c);
                let ImageRef::Pic(_, colour) = ci.image else {
                    unreachable!()
                };
                assert_ne!(
                    colour, c.image,
                    "ATAG {id:#x}: célula {:#x} sem versão colorida",
                    c.image
                );
                assert!(
                    ci.mask.is_some(),
                    "ATAG {id:#x}: célula {:#x} sem máscara",
                    c.image
                );
                let img = a.image(&ci.image);
                assert!(img.width > 1 || img.height > 1);
            }
            let an = anim::load_exe(&a, Program::Artist, id).unwrap();
            assert!(!an.is_finished());
        }
        for &c in MCZEE_COSTUMES.iter().chain(MAGGIE_POSES) {
            let img = a.image(&ImageRef::Pic(Program::Artist, c));
            let m = a.pic_next_id(Program::Artist, c, 2).expect("máscara");
            let mask = a.image(&ImageRef::Pic(Program::Artist, m));
            assert_eq!(mask.height, img.height, "{c:#x}");
        }
    }

    /// O canguru do saguão (0x324) passa pelas posições medidas na gravação
    /// do original: célula 1 em (534, 234) e célula 3 em (459, 355).
    #[test]
    fn real_lobby_kangaroo_matches_capture() {
        let Some(a) = assets() else { return };
        let mut an = anim::load_exe(&a, Program::Artist, 0x324).unwrap();
        an.update(0.0);
        assert_eq!(an.position(), (586, 220));
        an.update(0.2);
        assert_eq!(an.current_cell(), Some(1));
        assert_eq!(an.position(), (534, 234));
        an.update(0.4);
        assert_eq!(an.current_cell(), Some(3));
        assert_eq!(an.position(), (459, 355));
    }
}
