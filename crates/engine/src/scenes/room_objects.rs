//! Objetos clicáveis das salas do prédio, como no original (medidos no Wine):
//!
//! - **Bolsa Mágica**: fechada no chão; um clique a abre (som 0x2cf) e de
//!   dentro saem a maleta vermelha, o livro verde e a Varinha Mágica; outro
//!   clique a fecha (0x2ce).
//! - **maleta vermelha**: abre a pasta de trabalhos (0x2f1).
//! - **livro verde**: conta a história do McZee por cima da sala (0x2e2, 0x2f6).
//! - **Varinha Mágica**: vira o cursor (0x307) e a bolsa se fecha; o McZee
//!   explica o que ela faz. Com ela, um clique no mastro de bombeiro (ou na
//!   abertura dele no chão) faz um bicho descer ou subir pelo mastro (0x2f3) e
//!   um clique no buraco do rato faz sair uma surpresa (0x2ea); depois a bolsa
//!   se abre de novo com a varinha dentro. Clicar na bolsa devolve a varinha.
//!
//! As surpresas são os roteiros `ATAG` gravados em relação a um ponto (o
//! mastro ou o buraco do rato de cada sala); o bloco de cada sala e o ponto
//! foram conferidos casando as posições das figuras quadro a quadro nas
//! gravações do original (ver `docs/telas/animacoes.md`).

use crate::Frame;
use crate::anim::{self, Animator};
use crate::assets::{Assets, CursorRef, ImageRef, Program, SoundRef};
use crate::rng::XorShift;

type Rect = (i32, i32, i32, i32);

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

/// Imagem colorida, máscara, largura e altura de um objeto (`ARTIST.PIC`).
pub type Pic = (u16, u16, i32, i32);

/// Imagem colorida, máscara e tamanho de cada objeto.
pub const BAG_CLOSED: Pic = (0x6833, 0x6839, 113, 78);
pub const BAG_OPEN: Pic = (0x6873, 0x6879, 128, 56);
pub const SUITCASE: Pic = (0x685b, 0x6861, 99, 74);
pub const BOOK: Pic = (0x6863, 0x6869, 85, 60);
pub const WAND: Pic = (0x686b, 0x6871, 54, 54);

/// Sons (`ARTIST.SND`), identificados pelo tamanho no rastro do winmm.
pub const SND_BAG_OPEN: u16 = 0x2cf;
pub const SND_BAG_CLOSE: u16 = 0x2ce;
pub const SND_SUITCASE: u16 = 0x2f1;
pub const SND_BOOK: [u16; 2] = [0x2e2, 0x2f6];
pub const SND_WAND_TAKE: u16 = 0x307;
/// Varinha no buraco do rato. O rastro não separa 0x20a de 0x2ea (mesmo
/// tamanho); 0x20a é o som de escolher o Seleciona-adesivos na pintura.
pub const SND_WAND_HOLE: u16 = 0x2ea;
/// Mastro de bombeiro: subir, e a varinha no mastro.
pub const SND_POLE_UP: u16 = 0x2f3;
/// Abertura do mastro no chão: descer.
pub const SND_POLE_DOWN: u16 = 0x2f2;
/// Skate ("LÁ FORA" no saguão, "ELEVADOR" nos andares).
pub const SND_SKATE: u16 = 0x2fc;

/// Cursor da Varinha Mágica (`CURSOR` 0x00b7 de `ARTIST.EXE`).
pub const WAND_CURSOR: CursorRef = CursorRef(Program::Artist, 0xb7);

pub fn snd(id: u16) -> SoundRef {
    SoundRef::Snd(Program::Artist, id)
}

/// Onde ficam a bolsa e o que sai dela, em cada sala (medido no original).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BagLayout {
    pub closed: (i32, i32),
    pub open: (i32, i32),
    pub suitcase: (i32, i32),
    pub book: (i32, i32),
    pub wand: (i32, i32),
}

pub const LOBBY_BAG: BagLayout = BagLayout {
    closed: (97, 388),
    open: (97, 410),
    suitcase: (263, 394),
    book: (356, 367),
    wand: (222, 421),
};
pub const STUDIO_BAG: BagLayout = BagLayout {
    closed: (129, 392),
    open: (129, 414),
    suitcase: (305, 330),
    book: (265, 383),
    wand: (352, 410),
};
pub const PROJECTS_BAG: BagLayout = BagLayout {
    closed: (146, 392),
    open: (146, 414),
    suitcase: (330, 328),
    book: (247, 379),
    wand: (374, 407),
};
pub const TRICKS_BAG: BagLayout = BagLayout {
    closed: (149, 357),
    open: (149, 379),
    suitcase: (387, 306),
    book: (285, 282),
    wand: (284, 370),
};
pub const GALLERY_BAG: BagLayout = BagLayout {
    closed: (95, 378),
    open: (95, 400),
    suitcase: (304, 391),
    book: (235, 360),
    wand: (246, 422),
};

/// Parte da bolsa sob o ponteiro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BagPart {
    Bag,
    Suitcase,
    Book,
    Wand,
}

/// A Bolsa Mágica de uma sala.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicBag {
    pub layout: BagLayout,
    pub open: bool,
    /// A varinha está com a criança (fora da bolsa, no cursor).
    pub holding_wand: bool,
}

fn rect_at(pos: (i32, i32), img: Pic) -> Rect {
    (pos.0, pos.1, img.2, img.3)
}

impl MagicBag {
    pub fn new(layout: BagLayout, open: bool) -> MagicBag {
        MagicBag {
            layout,
            open,
            holding_wand: false,
        }
    }

    /// Partes visíveis, de baixo para cima (ordem de desenho).
    fn parts(&self) -> Vec<(BagPart, (i32, i32), Pic)> {
        let l = &self.layout;
        if !self.open {
            return vec![(BagPart::Bag, l.closed, BAG_CLOSED)];
        }
        let mut v = vec![
            (BagPart::Bag, l.open, BAG_OPEN),
            (BagPart::Suitcase, l.suitcase, SUITCASE),
            (BagPart::Book, l.book, BOOK),
        ];
        if !self.holding_wand {
            v.push((BagPart::Wand, l.wand, WAND));
        }
        v
    }

    /// O que está sob (x, y): o de cima primeiro.
    pub fn hit(&self, x: i32, y: i32) -> Option<BagPart> {
        self.parts()
            .into_iter()
            .rev()
            .find(|(_, pos, img)| inside(rect_at(*pos, *img), x, y))
            .map(|(p, _, _)| p)
    }

    pub fn draw(&self, frame: &mut Frame, assets: &Assets) {
        for (_, (x, y), (img, mask, _, _)) in self.parts() {
            frame.blit_masked(
                &assets.image(&ImageRef::Pic(Program::Artist, img)),
                &assets.image(&ImageRef::Pic(Program::Artist, mask)),
                x,
                y,
            );
        }
    }
}

/// Surpresas da Varinha Mágica numa sala: roteiros do mastro e do buraco do
/// rato, cada grupo com o ponto a que as origens gravadas se somam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Surprises {
    pub pole: &'static [u16],
    pub pole_anchor: (i32, i32),
    pub hole: &'static [u16],
    pub hole_anchor: (i32, i32),
}

/// Saguão: bichos no mastro 0x38b–0x394 (morsa conferida), buraco do rato
/// 0x3e9–0x3ef (avião conferido).
pub const LOBBY_SURPRISES: Surprises = Surprises {
    pole: &[
        0x38b, 0x38c, 0x38d, 0x38e, 0x38f, 0x390, 0x391, 0x392, 0x393, 0x394,
    ],
    pole_anchor: (596, 0),
    hole: &[0x3e9, 0x3ea, 0x3eb, 0x3ec, 0x3ed, 0x3ee, 0x3ef],
    hole_anchor: (513, 346),
};
/// Ateliê: mastro 0x70d–0x716 (Maggie, sapo, nós de corda conferidos),
/// buraco 0x76c–0x773 (pinguim e casa conferidos).
pub const STUDIO_SURPRISES: Surprises = Surprises {
    pole: &[
        0x70d, 0x70e, 0x70f, 0x710, 0x711, 0x712, 0x713, 0x714, 0x715, 0x716,
    ],
    pole_anchor: (575, -3),
    hole: &[0x76c, 0x76d, 0x76e, 0x76f, 0x770, 0x771, 0x772, 0x773],
    hole_anchor: (466, 328),
};
/// Oficina de Projetos: mastro 0x4b0–0x4b8 (rato, McZee e nós conferidos),
/// buraco 0x515–0x519 (rato espiando, casa e besouro conferidos).
pub const PROJECTS_SURPRISES: Surprises = Surprises {
    pole: &[
        0x4b0, 0x4b1, 0x4b2, 0x4b3, 0x4b4, 0x4b5, 0x4b6, 0x4b7, 0x4b8,
    ],
    pole_anchor: (573, -22),
    hole: &[0x515, 0x516, 0x517, 0x518, 0x519],
    hole_anchor: (479, 289),
};
/// Truques de Desenho: mastro 0xcd–0xd4 (sapo, rato e gosma conferidos),
/// buraco 0x12d–0x132 (barbatanas e roda conferidas).
pub const TRICKS_SURPRISES: Surprises = Surprises {
    pole: &[0xcd, 0xce, 0xcf, 0xd0, 0xd1, 0xd2, 0xd3, 0xd4],
    pole_anchor: (539, -32),
    hole: &[0x12d, 0x12e, 0x12f, 0x130, 0x131, 0x132],
    hole_anchor: (576, 273),
};
/// Galeria (sem mastro): buraco 0x259–0x25e, o bloco que sobra (por
/// eliminação; não conferido).
pub const GALLERY_SURPRISES: Surprises = Surprises {
    pole: &[],
    pole_anchor: (0, 0),
    hole: &[0x258, 0x259, 0x25a, 0x25b, 0x25c, 0x25d, 0x25e],
    hole_anchor: (445, 348),
};

/// Onde a varinha foi usada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Pole,
    Hole,
}

impl Surprises {
    /// Sorteia e carrega a surpresa do alvo, já com o ponto da sala somado.
    pub fn start(&self, target: Target, rng: &mut XorShift, assets: &Assets) -> Option<Animator> {
        let (list, anchor) = match target {
            Target::Pole => (self.pole, self.pole_anchor),
            Target::Hole => (self.hole, self.hole_anchor),
        };
        let id = *list.get(rng.below(list.len().max(1)))?;
        anim::load_exe(assets, Program::Artist, id).map(|a| a.with_offset(anchor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};

    #[test]
    fn bag_opens_and_shows_its_contents() {
        let mut b = MagicBag::new(LOBBY_BAG, false);
        // fechada: só a bolsa responde
        assert_eq!(b.hit(150, 420), Some(BagPart::Bag));
        assert_eq!(b.hit(300, 420), None);
        b.open = true;
        assert_eq!(b.hit(150, 440), Some(BagPart::Bag));
        assert_eq!(b.hit(310, 430), Some(BagPart::Suitcase));
        assert_eq!(b.hit(400, 400), Some(BagPart::Book));
        assert_eq!(b.hit(250, 445), Some(BagPart::Wand));
        // com a varinha na mão, ela não está mais no chão
        b.holding_wand = true;
        assert_ne!(b.hit(250, 445), Some(BagPart::Wand));
        let (_d, a) = empty_assets();
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
    }

    #[test]
    fn surprises_without_data_do_nothing() {
        let (_d, a) = empty_assets();
        let mut rng = XorShift::new(5);
        assert!(LOBBY_SURPRISES.start(Target::Hole, &mut rng, &a).is_none());
        assert!(
            GALLERY_SURPRISES
                .start(Target::Pole, &mut rng, &a)
                .is_none()
        );
    }

    /// Com os dados reais: todos os roteiros das surpresas existem, e a
    /// morsa do saguão (0x38b) passa pelo ponto medido na gravação: célula 3
    /// em (544, 23) e célula 6 em (544, 149).
    #[test]
    fn real_surprises_load_and_walrus_matches_capture() {
        let Some(a) = assets() else { return };
        for s in [
            LOBBY_SURPRISES,
            STUDIO_SURPRISES,
            PROJECTS_SURPRISES,
            TRICKS_SURPRISES,
            GALLERY_SURPRISES,
        ] {
            for &id in s.pole.iter().chain(s.hole) {
                assert!(
                    anim::load_exe(&a, Program::Artist, id).is_some(),
                    "ATAG {id:#x}"
                );
            }
        }
        let mut w = anim::load_exe(&a, Program::Artist, 0x38b)
            .unwrap()
            .with_offset(LOBBY_SURPRISES.pole_anchor);
        w.update(0.0);
        w.update(0.6);
        assert_eq!(w.current_cell(), Some(3));
        assert_eq!(w.position(), (544, 23));
        w.update(0.6 + 0.4);
        assert_eq!(w.current_cell(), Some(6));
        assert_eq!(w.position(), (544, 149));
        // o avião do buraco do rato do saguão (0x3ec): célula 3 em (310, 341)
        let mut p = anim::load_exe(&a, Program::Artist, 0x3ec)
            .unwrap()
            .with_offset(LOBBY_SURPRISES.hole_anchor);
        p.update(0.0);
        p.update(0.6);
        assert_eq!(p.current_cell(), Some(3));
        assert_eq!(p.position(), (310, 341));
    }
}
