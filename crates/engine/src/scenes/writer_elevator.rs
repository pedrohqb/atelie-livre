//! Elevador do Creative Writer (tarefa T8; `cw/report/building-salas.md`
//! §2 e a tabela de pontos quentes do `WRITER.EXE`,
//! `cw/report/building-hotspots.txt`).
//!
//! - Fundo `680b` (parede verde com espirais); as portas com a escotilha
//!   são uma figura só, `69b7` (365×438, máscara `69b9`) em (143,42): é
//!   também a área da porta (abre no andar em que se está, `2d4`).
//! - Mostrador `695b` (máscara `695d`) em (240,4): os números S/2/3/4 são
//!   botões de 25×25 (som `233`); aceso, o número do andar ganha a figura
//!   amarela opaca do mesmo tamanho: S `6979`, 2 `6975`, 3 `6973`, 4
//!   `696f` (este com máscara, `6971`, por ser um losango). Identificadas
//!   pela forma e conferidas na captura `building_55b_elev_base`.
//! - Seta `6957` (máscara `6959`) em (516,81) com os botões S (560,240)
//!   19×19, 2 (571,203) 19×19, 3 (558,167) 20×20 e 4 (553,127) 22×23 (som
//!   `2d4`); acesos: S `696b`, 2 `6967`, 3 `6963`, 4 `695f` (máscara duas
//!   figuras adiante).
//! - Viagem: roteiro `0822` (sobe) / `0823` (desce), 1,4 s por andar, na
//!   escotilha (origem relativa ao canto das portas). O motor `2d8` tem um
//!   bloco `loop`: no original o `WRITER.EXE` manda os três trechos
//!   (16416 + 11712 + 31868 B) com o do meio em laço e chama
//!   `waveOutBreakLoop` ao chegar — o mesmo número de trechos numa viagem
//!   de um andar ou de três (medido no Wine, S→4 e 4→3).
//! - McZee (fantasia da sala) em (−5,180); balão `0bb8[63]` "Subindo? Ou
//!   descendo?" em (140,193); luminária "Tchau" `6917` em (46,6).

use super::elevator::{ElevatorSounds, ElevatorSpec};
use super::writer_gags::{MCZEE_COSTUMES, MCZEE_ELEVATOR};
use crate::assets::Program;

pub const WRITER_ELEVATOR: ElevatorSpec = ElevatorSpec {
    program: Program::Writer,
    background: 0x680b,
    doors: (0x69b7, 0x69b9),
    doors_pos: (143, 42),
    porthole: None,
    indicator: (0x695b, Some(0x695d), (240, 4)),
    lit: [
        (0x6979, None, (254, 10)),
        (0x6975, None, (284, 10)),
        (0x6973, None, (314, 10)),
        (0x696f, Some(0x6971), (345, 10)),
    ],
    panel: ((0x6957, 0x6959), (516, 81)),
    panel_lit: [
        (0x696b, (560, 240)),
        (0x6967, (571, 203)),
        (0x6963, (558, 167)),
        (0x695f, (553, 127)),
    ],
    buttons: [
        (560, 240, 19, 19),
        (571, 203, 19, 19),
        (558, 167, 20, 20),
        (553, 127, 22, 23),
    ],
    indicator_buttons: [
        (254, 10, 25, 25),
        (284, 10, 25, 25),
        (314, 10, 25, 25),
        (345, 10, 25, 25),
    ],
    door: (143, 42, 364, 438),
    tchau: ((0x6917, 0x6919), (46, 6)),
    tchau_rect: (46, 6, 56, 132),
    mczee_pos: MCZEE_ELEVATOR,
    costumes: MCZEE_COSTUMES,
    ride: (0x0822, 0x0823),
    floor_room: super::writer_rooms::room,
    doorway_rim: 12,
    what: 0x0bb8,
    ask: 63,
    ask_pos: (140, 193),
    snd: ElevatorSounds {
        motor: 0x2d8,
        motor_loops: true,
        doors: None,
        arrive: 0x311,
        button: Some(0x2d4),
        indicator_button: Some(0x233),
        door: Some(0x2d4),
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;
    use crate::assets::{Assets, ImageRef, SoundRef};
    use crate::scene::{Effect, Input, MouseButton};
    use crate::scenes::elevator::{Elevator, Outcome};
    use crate::scenes::room::Floor;
    use crate::scenes::testutil::{assets, empty_assets};

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn sounds(fx: &[Effect]) -> Vec<Effect> {
        fx.iter()
            .filter(|e| {
                matches!(
                    e,
                    Effect::PlaySound(_) | Effect::PlayLoop(_) | Effect::StopLoop
                )
            })
            .cloned()
            .collect()
    }

    fn snd(id: u16) -> SoundRef {
        SoundRef::Snd(Program::Writer, id)
    }

    fn run(e: &mut Elevator, secs: f32, a: &Assets, fx: &mut Vec<Effect>) -> Option<Outcome> {
        let mut out = None;
        for _ in 0..(secs / 0.1).round() as usize {
            if let Some(o) = e.update(&[], 0.1, a, fx) {
                out = Some(o);
            }
        }
        out
    }

    /// Seta do andar 4 a partir do saguão: `2d4` do botão, o motor em laço
    /// durante os três andares, a freada (`StopLoop`) ao chegar e o `311`
    /// com as portas abertas.
    #[test]
    fn ride_loops_the_motor_until_arrival() {
        let (_d, a) = empty_assets();
        let mut e = Elevator::for_program(Program::Writer, Floor::Lobby, None, &a, "ana");
        assert!(e.balloon_open());
        let mut fx = Vec::new();
        e.update(&[click(560, 135)], 0.0, &a, &mut fx);
        assert_eq!(
            sounds(&fx),
            [Effect::PlaySound(snd(0x2d4)), Effect::PlayLoop(snd(0x2d8))]
        );
        // um andar a cada 1,4 s; o laço só para no último
        let mut t = 0.0;
        while e.at() != Floor::Tricks {
            let mut fx = Vec::new();
            assert_eq!(e.update(&[], 0.1, &a, &mut fx), None);
            t += 0.1;
            let stop = sounds(&fx) == [Effect::StopLoop];
            assert_eq!(stop, e.at() == Floor::Tricks, "{t}");
            assert!(t < 5.0);
        }
        assert!((4.1..4.6).contains(&t), "{t}");
        let mut fx = Vec::new();
        assert_eq!(
            run(&mut e, 1.1, &a, &mut fx),
            Some(Outcome::Arrived(Floor::Tricks))
        );
        assert_eq!(sounds(&fx), [Effect::PlaySound(snd(0x311))]);
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
    }

    /// Números do mostrador (`233`), porta (`2d4`: só abre) e luminária.
    #[test]
    fn indicator_door_and_tchau() {
        let (_d, a) = empty_assets();
        let mut e = Elevator::for_program(Program::Writer, Floor::Lobby, None, &a, "ana");
        let mut fx = Vec::new();
        e.update(&[click(296, 20)], 0.0, &a, &mut fx);
        assert!(e.moving());
        assert_eq!(sounds(&fx)[0], Effect::PlaySound(snd(0x233)));
        let mut e = Elevator::for_program(Program::Writer, Floor::Studio, None, &a, "ana");
        let mut fx = Vec::new();
        e.update(&[click(320, 300)], 0.0, &a, &mut fx);
        assert_eq!(sounds(&fx), [Effect::PlaySound(snd(0x2d4))]);
        assert_eq!(
            run(&mut e, 1.1, &a, &mut fx),
            Some(Outcome::Arrived(Floor::Studio))
        );
        let mut fx = Vec::new();
        assert_eq!(
            e.update(&[click(70, 120)], 0.0, &a, &mut fx),
            Some(Outcome::Quit)
        );
        // os rótulos da seta não são botões (só os círculos)
        let mut e = Elevator::for_program(Program::Writer, Floor::Lobby, None, &a, "ana");
        let mut fx = Vec::new();
        e.update(&[click(600, 255)], 0.0, &a, &mut fx);
        assert!(!e.moving());
    }

    /// Com os dados reais: as figuras existem com as medidas da tabela, o
    /// balão é o `0bb8[63]`, a escotilha passa pelo canto das portas e o
    /// mostrador acende o andar.
    #[test]
    fn writer_elevator_with_real_data() {
        let Some(a) = assets() else { return };
        let s = &WRITER_ELEVATOR;
        let size = |id: u16| {
            let i = a.image(&ImageRef::Pic(Program::Writer, id));
            (i.width, i.height)
        };
        assert_eq!(size(s.background), (640, 480));
        assert_eq!(size(s.doors.0), (365, 438));
        assert_eq!(size(s.doors.1), (365, 438));
        assert_eq!(size(s.indicator.0), (140, 44));
        assert_eq!(size(s.panel.0.0), (92, 240));
        for (id, ..) in s.lit {
            assert_eq!(size(id), (25, 25));
        }
        for (k, (id, _)) in s.panel_lit.iter().enumerate() {
            let r = s.buttons[k];
            assert_eq!(size(*id), (r.2 as u32, r.3 as u32), "{id:x}");
        }
        let mut e = Elevator::for_program(Program::Writer, Floor::Studio, None, &a, "ana");
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "writer_elevator");
        // "2" aceso: amarelo no mostrador e na seta
        let p = f.pixel(284 + 6, 10 + 12);
        assert!(p[0] > 200 && p[1] > 200 && p[2] < 100, "{p:?}");
        let p = f.pixel(571 + 4, 203 + 9);
        assert!(p[0] > 200 && p[1] > 200 && p[2] < 100, "{p:?}");
        let mut fx = Vec::new();
        e.choose(Floor::Projects, &a, &mut fx);
        assert!(e.moving());
        e.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "writer_elevator_ride");
        run(&mut e, 1.5, &a, &mut fx);
        e.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "writer_elevator_open");
    }
}
