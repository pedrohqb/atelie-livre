//! Imaginópolis e rua do Creative Writer (tarefa T3;
//! `cw/report/building.md` §6–7).
//!
//! - Imaginópolis `6801` (641×480): ao chegar, `302` e o carrinho
//!   `ATAG 0861` (os mesmos números do `02bc` do FA); qualquer clique leva à
//!   rua com o `2c0`.
//! - Rua `6803`: os pontos quentes vêm da tabela do `WRITER.EXE` em
//!   0x0b9fd6 (`cw/report/building-hotspots.txt`), na ordem do arquivo; os
//!   de baixo ficam por cima (porta > hidrante > lata > fachada do saguão >
//!   faixas da bola):
//!
//!   | área | som | ação |
//!   |---|---|---|
//!   | fachada (118,23) 372×92, (165,114) 302×75, (179,188) 287×76 | `301` | bola `080c` |
//!   | fachada de baixo (198,263) 269×166 | `304` | saguão (+ `2c0`) |
//!   | lata `699f` (461,309) 63×72 | — | roteiro `081b` |
//!   | hidrante `697d` (83,392) 49×60 | — | roteiro `0816` |
//!   | porta giratória (296,312) 90×110 | — | roteiro `0811` → saguão (`2c0`) |
//!   | o resto | — | Imaginópolis |
//!
//!   O hidrante e a lata não estão no fundo: são figuras por cima dele, que
//!   somem enquanto o roteiro delas toca (o roteiro as desenha). Os
//!   roteiros da porta, do hidrante e da lata têm a origem relativa ao
//!   canto da área; o da bola, absoluta. Bichos que passam sozinhos:
//!   `0800`–`0805` (`writer_gags::STREET_GAGS`).

use super::imaginopolis::ImaginopolisSpec;
use super::street::StreetSpec;
use super::writer_gags::{IMAGINOPOLIS_CAR, STREET_GAGS};
use crate::assets::Program;

/// Som da troca de tela (1068 B; `2c0`–`2c5` são o mesmo som).
pub const SND_CHANGE: u16 = 0x2c0;

pub const WRITER_IMAGINOPOLIS: ImaginopolisSpec = ImaginopolisSpec {
    program: Program::Writer,
    background: 0x6801,
    cart: IMAGINOPOLIS_CAR,
    snd_arrive: 0x302,
    snd_enter: SND_CHANGE,
};

/// Hidrante e lata de lixo (cor; máscara duas figuras adiante no `.PIC`).
pub const HYDRANT: (u16, (i32, i32)) = (0x697d, (83, 392));
pub const TRASH: (u16, (i32, i32)) = (0x699f, (461, 309));

pub const WRITER_STREET: StreetSpec = StreetSpec {
    program: Program::Writer,
    background: 0x6803,
    elements: &[TRASH, HYDRANT],
    door: (0x0811, (296, 312, 90, 110)),
    lobby: &[(198, 263, 269, 166)],
    ball: (
        0x080c,
        (0, 0),
        &[(118, 23, 372, 92), (165, 114, 302, 75), (179, 188, 287, 76)],
    ),
    hydrant: (0x0816, HYDRANT.1, (83, 392, 49, 60)),
    trash: (0x081b, TRASH.1, (461, 309, 63, 72)),
    gags: STREET_GAGS,
    snd_ball: 0x301,
    snd_lobby: 0x304,
    snd_enter: SND_CHANGE,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;
    use crate::assets::{Assets, ImageRef, SoundRef};
    use crate::scene::{Effect, Input, MouseButton, Scene, SceneId};
    use crate::scenes::imaginopolis::Imaginopolis;
    use crate::scenes::street::{Spot, Street};
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
                Effect::PlaySound(SoundRef::Snd(p, id)) => {
                    assert_eq!(*p, Program::Writer, "som {id:x} do programa errado");
                    Some(*id)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn spots_follow_the_writer_hotspot_table() {
        let s = &WRITER_STREET;
        // porta por cima da fachada do saguão
        assert_eq!(s.spot_at(300, 320), Spot::Door);
        assert_eq!(s.spot_at(385, 421), Spot::Door);
        assert_eq!(s.spot_at(386, 421), Spot::Lobby);
        assert_eq!(s.spot_at(200, 270), Spot::Lobby);
        // lata por cima da fachada; hidrante sozinho na calçada
        assert_eq!(s.spot_at(462, 330), Spot::Trash);
        assert_eq!(s.spot_at(100, 420), Spot::Hydrant);
        assert_eq!(s.spot_at(82, 420), Spot::Town);
        // faixas da bola
        assert_eq!(s.spot_at(120, 30), Spot::Ball);
        assert_eq!(s.spot_at(170, 150), Spot::Ball);
        assert_eq!(s.spot_at(180, 200), Spot::Ball);
        assert_eq!(s.spot_at(178, 200), Spot::Town);
        assert_eq!(s.spot_at(320, 450), Spot::Town);
        assert_eq!(s.spot_at(600, 300), Spot::Town);
    }

    #[test]
    fn writer_street_goes_to_writer_screens() {
        let (_d, a) = empty_assets();
        let mut s = Street::for_program(Program::Writer, None);
        let fx = s.update(&[click(250, 300)], 0.0, &a);
        assert_eq!(
            changes(&fx),
            [SceneId::Building.in_program(Program::Writer)]
        );
        assert_eq!(sounds(&fx), [0x304, SND_CHANGE]);
        let mut s = Street::for_program(Program::Writer, None);
        let fx = s.update(&[click(10, 10)], 0.0, &a);
        assert_eq!(
            changes(&fx),
            [SceneId::Imaginopolis.in_program(Program::Writer)]
        );
        // porta sem dados: entra direto
        let mut s = Street::for_program(Program::Writer, None);
        let fx = s.update(&[click(330, 380)], 0.0, &a);
        assert_eq!(sounds(&fx), [SND_CHANGE]);
        let mut f = Frame::new(640, 480);
        s.draw(&mut f, &a);
    }

    #[test]
    fn writer_imaginopolis_leads_to_writer_street() {
        let (_d, a) = empty_assets();
        let mut s = Imaginopolis::for_program(Program::Writer, None);
        let fx = s.update(&[], 0.3, &a);
        assert_eq!(sounds(&fx), [0x302]);
        let fx = s.update(&[click(300, 200)], 0.1, &a);
        assert_eq!(sounds(&fx), [SND_CHANGE]);
        assert_eq!(changes(&fx), [SceneId::Street.in_program(Program::Writer)]);
    }

    fn run(s: &mut Street, a: &Assets, secs: f32) -> Vec<Effect> {
        let mut fx = Vec::new();
        for _ in 0..(secs / 0.05) as usize {
            fx.extend(s.update(&[], 0.05, a));
        }
        fx
    }

    /// Com os dados reais: os roteiros do Writer tocam nos lugares da
    /// tabela, com as durações e os sons medidos (`building-atag.txt`).
    #[test]
    fn writer_street_scripts_with_real_data() {
        let Some(a) = assets() else { return };
        for (id, _) in WRITER_STREET.elements {
            assert!(a.pic_ids(Program::Writer).binary_search(id).is_ok());
        }
        let bg = a.image(&ImageRef::Pic(Program::Writer, WRITER_STREET.background));
        assert_eq!((bg.width, bg.height), (640, 480));
        // hidrante vermelho por cima da calçada
        let mut s = Street::for_program(Program::Writer, None);
        let mut f = Frame::new(640, 480);
        s.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "writer_street");
        let p = f.pixel(83 + 22, 392 + 24);
        assert!(p[0] > 150 && p[1] < 80, "{p:?}");
        // porta: 1,8 s com o `2f9`, depois o saguão
        let fx = s.update(&[click(330, 380)], 0.0, &a);
        assert_eq!(sounds(&fx), [0x2f9]);
        let fx = run(&mut s, &a, 1.75);
        assert!(changes(&fx).is_empty());
        let fx = run(&mut s, &a, 0.2);
        assert_eq!(
            changes(&fx),
            [SceneId::Building.in_program(Program::Writer)]
        );
        // lata: 3,6 s, cliques ignorados
        let mut s = Street::for_program(Program::Writer, None);
        s.update(&[click(480, 340)], 0.0, &a);
        assert!(s.is_animating());
        let fx = s.update(&[click(10, 10)], 0.05, &a);
        assert!(changes(&fx).is_empty());
        run(&mut s, &a, 3.7);
        assert!(!s.is_animating());
        // bola: `301` e 7,8 s
        let mut s = Street::for_program(Program::Writer, None);
        let fx = s.update(&[click(300, 60)], 0.0, &a);
        assert_eq!(sounds(&fx)[0], 0x301);
        run(&mut s, &a, 7.7);
        assert!(s.is_animating());
        run(&mut s, &a, 0.2);
        assert!(!s.is_animating());
    }
}
