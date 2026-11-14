//! Sótão do Creative Writer (tarefa T10; `cw/report/building-salas.md`
//! §7 e a tabela de pontos quentes do `WRITER.EXE`,
//! `cw/report/building-hotspots.txt`): tela preta, aro `7937` (máscara
//! `7d01`) em (347,24) — leva ao saguão (`2f3`), com Ctrl ao Escritório —,
//! chegada com `287`.
//!
//! O escuro só responde em doze regiões (`4b0`–`4bb` na tabela; o
//! relatório contou onze), que não cobrem o alto do meio, em volta do aro:
//! ali o clique não faz nada. Um clique numa
//! região toca um som sorteado; com o de tatear (`2e6`, ~3 em 4 cliques)
//! acende por ~0,17 s um dos três objetos "de lanterna" da região, cada um
//! sempre no mesmo lugar (que nem sempre fica dentro da região).
//!
//! Medido no original (Wine, `:175`, 168 cliques nos centros das regiões,
//! quadros capturados a cada ~15 ms e comparados pixel a pixel com as
//! figuras `7d03`…`7d8d`: erro médio 0 na maioria, ≤ 2,2 nas outras): os
//! 36 objetos, o canto de cada um e a região; o lampejo aparece até
//! 10–30 ms depois do clique e some entre 150 e 180 ms. Os sons sem
//! lampejo (44 cliques) e o peso de cada um vêm da mesma contagem; os de
//! mesmo tamanho no `WRITER.SND` foram escolhidos pelos ids que só existem
//! no Writer (`282`, `286`, `28a`, `293`) ou pelos personagens (`281`
//! McZee, `283` Max, `2ff` bolinha), como o Fine Artist (`252`/`254`).

use super::attic::{AtticSpec, DarkRegion};
use super::room_objects;
use super::room_objects::Surprises;
use super::room_spec::{Action, RoomSpec, hot};
use super::writer_rooms::WRITER_KIT;
use crate::assets::Program;
use crate::ui::Rect;

/// Aro com o topo do mastro: cor, máscara e canto; e a área do clique.
pub const RING: (u16, u16, (i32, i32)) = (0x7937, 0x7d01, (347, 24));
pub const RING_RECT: Rect = (347, 24, 136, 75);
/// Som de chegada (depois do do mastro).
pub const SND_ARRIVE: u16 = 0x287;
/// Som de tatear no escuro: o único que acende um objeto.
pub const SND_GROPE: u16 = 0x2e6;

/// Sons do clique numa região e o peso de cada um (contagem no original).
pub const DARK_SOUNDS: [(u16, u32); 14] = [
    (SND_GROPE, 124),
    (0x293, 8),
    (0x281, 6),
    (0x20a, 5),
    (0x286, 4),
    (0x222, 4),
    (0x2dd, 3),
    (0x308, 3),
    (0x287, 3),
    (0x282, 2),
    (0x283, 2),
    (0x28a, 2),
    (0x2ff, 1),
    (0x20e, 1),
];

/// As doze regiões, na ordem da tabela (as de baixo ficam por cima), com
/// os três objetos de cada uma.
pub const REGIONS: [DarkRegion; 12] = [
    DarkRegion {
        rect: (1, 1, 143, 143),
        // Terra/lua, cachorro e peixe, relógio cuco
        objects: &[(0x7d15, (4, 15)), (0x7d25, (1, 2)), (0x7d7d, (5, 32))],
    },
    DarkRegion {
        rect: (145, 1, 175, 143),
        // âncora, sino, fumaça
        objects: &[
            (0x7d2d, (199, 2)),
            (0x7d49, (159, 18)),
            (0x7d89, (102, -22)),
        ],
    },
    DarkRegion {
        rect: (496, 1, 143, 143),
        // lesmas, colmeia, disco voador
        objects: &[
            (0x7d41, (484, 23)),
            (0x7d71, (483, 17)),
            (0x7d81, (527, -1)),
        ],
    },
    DarkRegion {
        rect: (496, 145, 143, 190),
        // sarcófago, monstro, caldeirão
        objects: &[
            (0x7d31, (454, 170)),
            (0x7d4d, (494, 189)),
            (0x7d79, (457, 214)),
        ],
    },
    DarkRegion {
        rect: (496, 336, 143, 143),
        // cerca, pilha de roupas, trevos
        objects: &[
            (0x7d09, (446, 314)),
            (0x7d0d, (326, 336)),
            (0x7d21, (539, 413)),
        ],
    },
    DarkRegion {
        rect: (321, 336, 175, 143),
        // baú, sapo, lâmpada mágica
        objects: &[
            (0x7d1d, (295, 308)),
            (0x7d29, (327, 364)),
            (0x7d6d, (349, 394)),
        ],
    },
    DarkRegion {
        rect: (145, 336, 175, 143),
        // chave, tigela "KILLER", fio de telefone
        objects: &[
            (0x7d35, (226, 407)),
            (0x7d3d, (169, 385)),
            (0x7d5d, (126, 397)),
        ],
    },
    DarkRegion {
        rect: (1, 336, 143, 143),
        // poça, caveira, frascos
        objects: &[(0x7d05, (4, 372)), (0x7d11, (4, 326)), (0x7d65, (7, 322))],
    },
    DarkRegion {
        rect: (1, 145, 175, 190),
        // vaso com flores, máscaras, cerca-viva
        objects: &[(0x7d55, (16, 252)), (0x7d59, (9, 181)), (0x7d69, (21, 254))],
    },
    DarkRegion {
        rect: (145, 145, 116, 190),
        // porta com luminária, boca, morcego na cortina
        objects: &[(0x7d19, (87, 72)), (0x7d51, (131, 176)), (0x7d85, (88, 87))],
    },
    DarkRegion {
        rect: (262, 145, 116, 190),
        // dinamite, túnel, castelo
        objects: &[
            (0x7d03, (143, 252)),
            (0x7d39, (222, 167)),
            (0x7d8d, (234, 178)),
        ],
    },
    DarkRegion {
        rect: (379, 145, 116, 190),
        // hieróglifos, painel de carro, Elvis
        objects: &[
            (0x7d45, (320, 158)),
            (0x7d61, (333, 219)),
            (0x7d75, (352, 161)),
        ],
    },
];

pub const WRITER_ATTIC: AtticSpec = AtticSpec {
    program: Program::Writer,
    ring: Some(RING),
    regions: &REGIONS,
    sounds: &DARK_SOUNDS,
    flash_sound: Some(SND_GROPE),
    flash_time: 0.17,
};

const ATTIC_ROOM: RoomSpec = RoomSpec {
    kit: &WRITER_KIT,
    background: 0,
    elements: &[],
    // o aro sobe ao saguão (Ctrl: ao Escritório), como o mastro
    hotspots: &[hot(RING_RECT, Action::PoleUp)],
    arrival: None,
    character: None,
    quit_pos: (208, 179),
    // sem bolsa no sótão (a sala não a desenha)
    bag: room_objects::TRICKS_BAG,
    bag_open: false,
    surprises: Surprises {
        pole: &[],
        pole_anchor: (0, 0),
        hole: &[],
        hole_anchor: (0, 0),
    },
    gags: &[],
    tips: &[],
    painting: None,
    studio: None,
    elevator_sound: 0x2fc,
    arrival_sound: Some(SND_ARRIVE),
    elevator_arrival_sound: None,
};

/// O sótão do Writer.
pub const ATTIC: &RoomSpec = &ATTIC_ROOM;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;
    use crate::assets::ImageRef;
    use crate::rng::XorShift;
    use crate::scenes::attic::{Dark, draw_in};
    use crate::scenes::testutil::{assets, empty_assets};

    #[test]
    fn regions_hold_the_36_objects_once() {
        let mut ids: Vec<u16> = REGIONS
            .iter()
            .flat_map(|r| r.objects.iter().map(|o| o.0))
            .collect();
        ids.sort();
        let mut want = vec![0x7d03];
        want.extend((0..35).map(|k| 0x7d05 + 4 * k));
        assert_eq!(ids, want);
        assert!(REGIONS.iter().all(|r| r.objects.len() == 3));
    }

    /// O alto do meio (em volta do aro) não responde; nas regiões, o
    /// `2e6` acende um objeto da região por ~0,17 s.
    #[test]
    fn grope_lights_an_object_of_the_region() {
        let mut rng = XorShift::new(11);
        let mut d = Dark::default();
        assert_eq!(d.click(Program::Writer, 330, 120, &mut rng), None);
        assert_eq!(d.click(Program::Writer, 480, 5, &mut rng), None);
        let (mut lit, mut other) = (0, 0);
        for _ in 0..400 {
            let id = d.click(Program::Writer, 72, 407, &mut rng).unwrap();
            if id == SND_GROPE {
                let (obj, _) = d.lit().expect("aceso");
                assert!(REGIONS[7].objects.iter().any(|o| o.0 == obj));
                lit += 1;
            } else {
                assert!(DARK_SOUNDS.iter().any(|s| s.0 == id));
                other += 1;
            }
            d.update(0.16);
            assert!(id != SND_GROPE || d.lit().is_some());
            d.update(0.02);
            assert!(d.lit().is_none());
        }
        assert!(lit > 250 && other > 60, "{lit} {other}");
        // a região de baixo na lista (145..261) vale sobre a anterior (1..176)
        let mut d = Dark::default();
        loop {
            if d.click(Program::Writer, 160, 200, &mut rng) == Some(SND_GROPE) {
                break;
            }
        }
        assert!(
            REGIONS[9]
                .objects
                .iter()
                .any(|o| Some(o.0) == d.lit().map(|l| l.0))
        );
    }

    #[test]
    fn fine_artist_dark_is_unchanged() {
        let mut a = XorShift::new(5);
        let mut b = XorShift::new(5);
        let mut d = Dark::default();
        for _ in 0..50 {
            assert_eq!(
                d.click(Program::Artist, 330, 120, &mut a),
                Some(crate::scenes::attic::dark_sound(&mut b))
            );
            assert!(d.lit().is_none());
        }
        let (_d, assets) = empty_assets();
        let mut f = Frame::new(640, 480);
        draw_in(Program::Writer, &d, &mut f, &assets);
    }

    fn click(x: i32, y: i32) -> crate::scene::Input {
        crate::scene::Input::Click {
            button: crate::scene::MouseButton::Left,
            x,
            y,
        }
    }

    /// Na sala do Writer: o aro leva ao saguão (Ctrl: ao Escritório); o
    /// escuro toca os sons do `WRITER.SND`, e o alto do meio, nada.
    #[test]
    fn writer_attic_room_ring_and_dark() {
        use crate::scene::{Effect, Input, Scene};
        use crate::scenes::room::{Floor, Room};
        let (_d, a) = empty_assets();
        let mut r = Room::for_program(Program::Writer, Floor::Attic, None, None, &a);
        let fx = r.update(&[click(100, 100)], 0.0, &a);
        assert!(fx.iter().any(|e| matches!(
            e,
            Effect::PlaySound(crate::assets::SoundRef::Snd(Program::Writer, id))
                if DARK_SOUNDS.iter().any(|s| s.0 == *id)
        )));
        let fx = r.update(&[click(330, 120)], 0.0, &a);
        assert!(!fx.iter().any(|e| matches!(e, Effect::PlaySound(_))));
        let mut f = Frame::new(640, 480);
        r.draw(&mut f, &a);
        r.update(&[click(400, 50)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        let mut r = Room::for_program(Program::Writer, Floor::Attic, None, None, &a);
        r.update(&[Input::Ctrl(true), click(400, 50)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Studio);
    }

    /// Com os dados reais: o aro e os objetos existem com o tamanho certo e
    /// o objeto aceso aparece no canto medido.
    #[test]
    fn writer_attic_with_real_data() {
        let Some(a) = assets() else { return };
        let ring = a.image(&ImageRef::Pic(Program::Writer, RING.0));
        assert_eq!((ring.width, ring.height), (136, 75));
        let mut rng = XorShift::new(3);
        let mut d = Dark::default();
        while d.click(Program::Writer, 567, 72, &mut rng) != Some(SND_GROPE) {}
        let (id, (x, y)) = d.lit().unwrap();
        let mut f = Frame::new(640, 480);
        draw_in(Program::Writer, &d, &mut f, &a);
        crate::scenes::testutil::dump(&f, "writer_attic_flash");
        let img = a.image(&ImageRef::Pic(Program::Writer, id));
        // o pixel mais claro da figura aparece no lugar
        let (mut best, mut at) = (0u32, (0, 0));
        for py in 0..img.height {
            for px in 0..img.width {
                let k = ((py * img.width + px) * 4) as usize;
                let v = img.rgba[k..k + 3].iter().map(|&c| c as u32).sum();
                if v > best && y + (py as i32) >= 0 {
                    (best, at) = (v, (px, py));
                }
            }
        }
        let k = ((at.1 * img.width + at.0) * 4) as usize;
        assert_eq!(
            f.pixel((x + at.0 as i32) as u32, (y + at.1 as i32) as u32)[..3],
            img.rgba[k..k + 3]
        );
        // o aro (laranja) em cima
        let p = f.pixel(347 + 30, 24 + 15);
        assert!(p[0] > 150, "{p:?}");
    }
}
