//! Sótão escuro, acima do andar dos Truques (só se chega pelo mastro): tela
//! preta com o topo do mastro atravessando um aro em (368,27)-(461,91). Ao
//! chegar toca `0x287`; clicar no escuro toca um som sorteado (quase sempre
//! `0x2e6`: a piada de tatear no escuro); clicar no aro sobe e dá a volta ao
//! saguão (Ctrl: ao Ateliê). Medido no original (`sweep/report/attic.md`).
//!
//! Não achamos no `ARTIST.PIC` a figura do aro do original; ele é montado com
//! a abertura do mastro dos andares (0x682b, virada de cabeça para baixo) e
//! um pedaço do mastro.
//!
//! O que muda por programa é um [`AtticSpec`]; o do Creative Writer
//! (`writer_attic`) tem o aro desenhado, só doze regiões que respondem no
//! escuro e, com o som de tatear, o lampejo de um objeto da região (como
//! se iluminado por uma lanterna), guardado num [`Dark`] pela sala.

use crate::assets::{Assets, ImageRef, Program};
use crate::rng::XorShift;
use crate::ui::{Rect, inside};
use crate::{Frame, Image};

/// Aro com o topo do mastro: a única área que leva a algum lugar.
pub const RING: Rect = (368, 27, 94, 65);
/// Som de chegada ao sótão (depois do do mastro).
pub const SND_ARRIVE: u16 = 0x287;

/// Sons do clique no escuro e o peso de cada um (contagem de 114 cliques
/// com som no original: `0x2e6` 87 vezes, os outros de 1 a 5).
const DARK_SOUNDS: [(u16, u32); 10] = [
    (0x2e6, 87),
    (0x222, 5),
    (0x2dd, 5),
    (0x252, 5),
    (0x20e, 3),
    (0x210, 3),
    (0x287, 2),
    (0x308, 2),
    (0x20a, 1),
    (0x254, 1),
];

/// Uma região do escuro: a área do clique e os objetos que podem acender
/// nela (figura colorida, desenhada opaca, e o canto na tela).
#[derive(Debug, Clone, Copy)]
pub struct DarkRegion {
    pub rect: Rect,
    pub objects: &'static [(u16, (i32, i32))],
}

/// O que muda no sótão de um programa.
#[derive(Debug, Clone, Copy)]
pub struct AtticSpec {
    pub program: Program,
    /// Aro do mastro (cor, máscara, canto); `None`: o aro montado do FA.
    pub ring: Option<(u16, u16, (i32, i32))>,
    /// Regiões que respondem no escuro (as de baixo na lista ficam por
    /// cima); vazio: a tela toda.
    pub regions: &'static [DarkRegion],
    /// Sons do clique no escuro e o peso de cada um.
    pub sounds: &'static [(u16, u32)],
    /// Som que acende um objeto da região clicada.
    pub flash_sound: Option<u16>,
    /// Quanto tempo o objeto fica aceso (s).
    pub flash_time: f32,
}

/// O sótão do Fine Artist.
pub const ARTIST_ATTIC: AtticSpec = AtticSpec {
    program: Program::Artist,
    ring: None,
    regions: &[],
    sounds: &DARK_SOUNDS,
    flash_sound: None,
    flash_time: 0.0,
};

/// O sótão do programa `p`.
pub fn spec(p: Program) -> &'static AtticSpec {
    match p {
        Program::Artist => &ARTIST_ATTIC,
        Program::Writer => &super::writer_attic::WRITER_ATTIC,
    }
}

/// O escuro do sótão: o objeto aceso agora (lampejo), se houver.
#[derive(Debug, Clone, Default)]
pub struct Dark {
    flash: Option<(u16, (i32, i32), f32)>,
}

impl Dark {
    /// Clique no escuro em (x, y) no sótão do programa `p`: o som a tocar
    /// (nenhum fora das regiões); com o som de tatear, acende um objeto.
    pub fn click(&mut self, p: Program, x: i32, y: i32, rng: &mut XorShift) -> Option<u16> {
        let spec = spec(p);
        let region = if spec.regions.is_empty() {
            None
        } else {
            Some(spec.regions.iter().rev().find(|r| inside(r.rect, x, y))?)
        };
        let id = weighted(spec.sounds, rng);
        if Some(id) == spec.flash_sound
            && let Some(r) = region.filter(|r| !r.objects.is_empty())
        {
            let (obj, pos) = r.objects[rng.below(r.objects.len())];
            self.flash = Some((obj, pos, spec.flash_time));
        }
        Some(id)
    }

    pub fn update(&mut self, dt: f32) {
        if let Some((_, _, t)) = &mut self.flash {
            *t -= dt;
            if *t <= 0.0 {
                self.flash = None;
            }
        }
    }

    /// Objeto aceso agora (figura e canto).
    pub fn lit(&self) -> Option<(u16, (i32, i32))> {
        self.flash.map(|(id, pos, _)| (id, pos))
    }
}

/// Sorteia o som de um clique no escuro (Fine Artist).
pub fn dark_sound(rng: &mut XorShift) -> u16 {
    weighted(&DARK_SOUNDS, rng)
}

fn weighted(sounds: &[(u16, u32)], rng: &mut XorShift) -> u16 {
    let total: u32 = sounds.iter().map(|s| s.1).sum();
    let mut n = rng.below(total as usize) as u32;
    for &(id, w) in sounds {
        if n < w {
            return id;
        }
        n -= w;
    }
    sounds[0].0
}

/// Abertura do mastro (cor, máscara) e mastro dos andares.
const HOLE: (u16, u16) = (0x682b, 0x6831);
const POLE: u16 = 0x6823;
/// Onde a abertura virada fica e o pedaço do mastro abaixo dela.
const HOLE_POS: (i32, i32) = (362, 27);
const POLE_X: i32 = 406;

fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Artist, id)
}

/// Copia a imagem virada de cabeça para baixo, só onde a máscara é preta.
fn blit_flipped(f: &mut Frame, img: &Image, mask: &Image, x: i32, y: i32) {
    let w = img.width.min(mask.width);
    let h = img.height.min(mask.height);
    for dy in 0..h {
        for dx in 0..w {
            let mi = ((dy * mask.width + dx) * 4) as usize;
            let m = &mask.rgba[mi..mi + 3];
            if (m[0] as u32 + m[1] as u32 + m[2] as u32) < 384 {
                let i = ((dy * img.width + dx) * 4) as usize;
                let p = &img.rgba[i..i + 3];
                f.put(x + dx as i32, y + (h - 1 - dy) as i32, [p[0], p[1], p[2]]);
            }
        }
    }
}

/// Desenha o sótão do programa `p`, com o objeto aceso de `dark`.
pub fn draw_in(p: Program, dark: &Dark, frame: &mut Frame, assets: &Assets) {
    let spec = spec(p);
    let Some((img, mask, (x, y))) = spec.ring else {
        return draw(frame, assets);
    };
    frame.fill([0, 0, 0]);
    let pic = |id| assets.image(&ImageRef::Pic(p, id));
    frame.blit_masked(&pic(img), &pic(mask), x, y);
    if let Some((id, (x, y))) = dark.lit() {
        frame.blit(&pic(id), x, y);
    }
}

pub fn draw(f: &mut Frame, a: &Assets) {
    f.fill([0, 0, 0]);
    let (img, mask) = (a.image(&pic(HOLE.0)), a.image(&pic(HOLE.1)));
    if (img.width, img.height) != (mask.width, mask.height) {
        return;
    }
    // o mastro desce do aro até a borda de baixo dele
    let pole = a.image(&pic(POLE));
    let top = HOLE_POS.1 + img.height as i32 - 6;
    let bottom = RING.1 + RING.3;
    for y in top..bottom {
        for dx in 0..pole.width.min(22) {
            let i = (((y - top) as u32 % pole.height.max(1) * pole.width + dx) * 4) as usize;
            let p = &pole.rgba[i..i + 4];
            if p[3] != 0 && p[..3] != [255, 255, 255] {
                f.put(POLE_X + dx as i32, y, [p[0], p[1], p[2]]);
            }
        }
    }
    blit_flipped(f, &img, &mask, HOLE_POS.0, HOLE_POS.1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_sounds_are_mostly_the_groping_one() {
        let mut rng = XorShift::new(7);
        let n = 1000;
        let groping = (0..n).filter(|_| dark_sound(&mut rng) == 0x2e6).count();
        assert!(groping > n * 6 / 10 && groping < n * 9 / 10, "{groping}");
    }
}
