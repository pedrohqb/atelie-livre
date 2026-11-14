//! Roteiro dos Truques básicos (o porco Chiquinho): balões do `WHAT 3800`
//! e figuras do `ARTIST.PIC` (313×272). Tabela na spec
//! `docs/superpowers/specs/2026-10-31-truques-design.md`.

use super::wizard::{Advance, Goto, Step, Target, Wizard};
use crate::scene::SceneId;

const BLOCK: u16 = 0x3800;

/// Índices em `TOOLS` da tela de pintura.
pub const TOOL_SUPERTOOLS: usize = 8;
/// Casas da faixa das Super-ferramentas (`supertools::BUTTONS[..].k`).
pub const OPT_NO_GRID: usize = 0;
pub const OPT_BIG_GRID: usize = 2;

/// Figuras do quadro da apresentação (313×272), conferidas no original.
/// O Chiquinho inteiro.
const PIG: u16 = 0x3827;
/// O espaço em volta (negativo) pontilhado.
const PIG_AROUND: u16 = 0x382b;
/// As formas de dentro (positivo) sombreadas.
const PIG_INSIDE: u16 = 0x382d;
/// "Já comecei para você": o espaço negativo para traçar.
const PIG_TRACE: u16 = 0x3831;
/// O rosto: as formas de dentro, sem o contorno.
const PIG_FACE: u16 = 0x383c;
/// Só o contorno preto (o espaço positivo), pintado no quadro da criança.
const PIG_OUTLINE: u16 = 0x3838;
/// Figuras da Área de Desenho da Maggie na pintura (178×149): o Chiquinho e
/// o Chiquinho com o quadriculado verde.
const PIG_SMALL: u16 = 0x3865;
const PIG_GRID: u16 = 0x3859;

/// Primeiro passo na tela de pintura (o "ateliê de treinamento"); os
/// anteriores são da tela de apresentação (`tricks_board`).
pub const STUDIO: usize = 10;

/// Lugar do balão de cada registro na apresentação (canto superior
/// esquerdo, medido no original).
pub const BOARD_BALLOONS: &[(usize, i32, i32)] = &[
    (0, 425, 42),
    (2, 464, 46),
    (4, 459, 43),
    (6, 16, 32),
    (7, 463, 28),
    (8, 462, 8),
    (9, 460, 85),
    (10, 482, 125),
    (11, 439, 56),
    (13, 470, 106),
];

/// Maggie (e o Max) na apresentação, por registro: (figura, x, y).
pub const BOARD_POSES: &[(usize, &[(u16, i32, i32)])] = &[
    (0, &[(0x3803, 468, 262)]),
    (2, &[(0x3807, 483, 271)]),
    (4, &[(0x3807, 483, 271)]),
    (6, &[(0x380b, 22, 229), (0x380f, 55, 393)]),
    (7, &[(0x3813, 485, 181)]),
    (8, &[(0x3813, 485, 181)]),
    (9, &[(0x3807, 483, 271)]),
    (10, &[(0x3807, 483, 271)]),
    (11, &[(0x381f, 445, 277)]),
    (13, &[(0x3823, 405, 332)]),
];

/// As "3 formas circulares meio achatadas" do Chiquinho, como elipses no
/// quadro (`PIG_FACE`): a cabeça, o corpo e o focinho.
pub const PIG_CIRCLES: [(i32, i32, i32, i32); 3] =
    [(85, 35, 130, 110), (105, 105, 165, 125), (68, 108, 54, 46)];

fn step(k: usize, maggie: Option<u16>, advance: Advance) -> Step {
    Step {
        record: (BLOCK, k),
        maggie,
        your_area: None,
        advance,
        anim: None,
        icons: &[],
    }
}

fn supertool(k: usize) -> Advance {
    Advance::Click(Target::Option {
        tool: TOOL_SUPERTOOLS,
        page: None,
        k,
    })
}

/// Os passos, na ordem da tabela da spec.
pub fn steps() -> Vec<Step> {
    vec![
        // apresentação (quadro "Truques Básicos")
        // 0: "Maggie, como é que você aprendeu a desenhar?"
        step(
            0,
            Some(PIG),
            Advance::Choice(vec![
                (0, Goto::Step(1)),
                (1, Goto::Step(STUDIO)),
                (2, Goto::Scene(SceneId::TricksFloor)),
            ]),
        ),
        // 1-2: "É divertido desenhar!" (em volta; depois, dentro)
        step(2, Some(PIG_AROUND), Advance::Next),
        step(4, Some(PIG_INSIDE), Advance::Next),
        // 3: clique no espaço positivo
        step(
            6,
            Some(PIG),
            Advance::Click(Target::PanelArea((40, 30, 240, 220))),
        ),
        // 4: trace o espaço negativo (no próprio quadro)
        step(7, Some(PIG_TRACE), Advance::PaintThenNext),
        // 5: clique no rosto
        step(
            8,
            Some(PIG_FACE),
            Advance::Click(Target::PanelArea((0, 0, 160, 272))),
        ),
        // 6: as três formas
        step(9, Some(PIG_FACE), Advance::FindShapes(PIG_CIRCLES.to_vec())),
        step(10, Some(PIG_FACE), Advance::Next),
        step(11, Some(PIG_FACE), Advance::Next),
        // 9: ferramentas especiais?
        step(
            13,
            Some(PIG_FACE),
            Advance::Choice(vec![(0, Goto::Step(STUDIO)), (1, Goto::Step(18))]),
        ),
        // pintura ("ateliê de treinamento")
        // 10: "Seja bem-vindo ao meu ateliê de treinamento!"
        step(3, Some(PIG_SMALL), Advance::Next),
        // 11: "Vamos usar as Super-ferramentas de desenho…"
        step(1, Some(PIG_SMALL), Advance::Next),
        step(14, None, Advance::Click(Target::Tool(TOOL_SUPERTOOLS))),
        step(15, None, supertool(OPT_BIG_GRID)),
        step(18, Some(PIG_GRID), Advance::Maggie),
        Step {
            your_area: Some(PIG_OUTLINE),
            ..step(20, Some(PIG_GRID), Advance::Maggie)
        },
        step(23, None, supertool(OPT_NO_GRID)),
        // 17
        step(
            40,
            None,
            Advance::Choice(vec![(0, Goto::Step(16)), (1, Goto::Step(19))]),
        ),
        // 18
        step(
            41,
            None,
            Advance::Choice(vec![
                (0, Goto::Step(12)),
                (1, Goto::Scene(SceneId::TricksFloor)),
            ]),
        ),
        // 19
        step(
            24,
            None,
            Advance::Choice(vec![(0, Goto::Step(20)), (1, Goto::DiscardThen(22))]),
        ),
        // 20
        step(25, None, Advance::TypeName),
        // 21
        step(
            27,
            None,
            Advance::Choice(vec![(0, Goto::PrintThen(23)), (1, Goto::Step(23))]),
        ),
        // 22
        step(
            26,
            None,
            Advance::Choice(vec![(0, Goto::PrintThen(23)), (1, Goto::Step(23))]),
        ),
        // 23
        step(
            29,
            None,
            Advance::Choice(vec![
                (0, Goto::Step(STUDIO)),
                (1, Goto::Folder),
                (2, Goto::Scene(SceneId::TricksFloor)),
            ]),
        ),
    ]
}

pub fn wizard() -> Wizard {
    Wizard::new(steps())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::wizard::{WizardEvent as E, WizardOutcome as O};

    #[test]
    fn full_path_reaches_the_end() {
        let mut w = wizard();
        // apresentação
        assert_eq!(w.on_event(E::Option(0)), O::Entered(1));
        assert_eq!(w.on_event(E::Next), O::Entered(2));
        assert_eq!(w.on_event(E::Next), O::Entered(3));
        assert_eq!(w.on_event(E::Panel(150, 150)), O::Entered(4));
        assert_eq!(w.on_event(E::Painted), O::Stay);
        assert_eq!(w.on_event(E::Next), O::Entered(5));
        assert_eq!(w.on_event(E::Panel(300, 100)), O::Stay);
        assert_eq!(w.on_event(E::Panel(50, 100)), O::Entered(6));
        assert_eq!(w.on_event(E::Panel(10, 10)), O::Stay);
        let centers: Vec<(i32, i32)> = PIG_CIRCLES
            .iter()
            .map(|(x, y, cw, ch)| (x + cw / 2, y + ch / 2))
            .collect();
        assert_eq!(w.on_event(E::Panel(centers[0].0, centers[0].1)), O::Stay);
        assert_eq!(w.on_event(E::Panel(centers[0].0, centers[0].1)), O::Stay);
        assert_eq!(w.on_event(E::Panel(centers[1].0, centers[1].1)), O::Stay);
        assert_eq!(w.shapes_found().0, &[0, 1]);
        assert_eq!(
            w.on_event(E::Panel(centers[2].0, centers[2].1)),
            O::Entered(7)
        );
        assert_eq!(w.on_event(E::Next), O::Entered(8));
        assert_eq!(w.on_event(E::Next), O::Entered(9));
        // "Sim, vamos experimentar": o ateliê de treinamento
        assert_eq!(w.on_event(E::Option(0)), O::Entered(STUDIO));
        assert_eq!(w.step().record.1, 3);
        // a pintura começa ali, sem o Volta para a apresentação
        let mut w = wizard().starting_at(STUDIO);
        assert!(!w.has_back());
        assert_eq!(w.on_event(E::Next), O::Entered(11));
        assert_eq!(w.on_event(E::Next), O::Entered(12));
        assert_eq!(w.on_event(E::Tool(0)), O::Stay);
        assert_eq!(w.on_event(E::Tool(TOOL_SUPERTOOLS)), O::Entered(13));
        let grid = |k| E::OptionButton {
            tool: TOOL_SUPERTOOLS,
            page: 1,
            k,
        };
        assert_eq!(w.on_event(grid(OPT_BIG_GRID)), O::Entered(14));
        assert_eq!(w.on_event(E::McZee), O::Entered(15));
        assert_eq!(w.step().your_area, Some(PIG_OUTLINE));
        assert_eq!(w.on_event(E::McZee), O::Entered(16));
        assert_eq!(w.on_event(grid(OPT_NO_GRID)), O::Entered(17));
        assert_eq!(w.on_event(E::Option(1)), O::Entered(19));
        assert_eq!(w.on_event(E::Option(0)), O::Entered(20));
        for c in "porco".chars() {
            w.on_event(E::Typed(c));
        }
        assert_eq!(w.on_event(E::Enter), O::Save("porco".into()));
        assert_eq!(w.current(), 21);
        assert_eq!(w.on_event(E::Option(1)), O::Entered(23));
        assert_eq!(w.on_event(E::Option(1)), O::OpenFolder);
        assert_eq!(w.on_event(E::Option(2)), O::Leave(SceneId::TricksFloor));
    }

    #[test]
    fn board_steps_have_pictures_poses_and_places() {
        for (i, s) in steps().iter().enumerate().take(STUDIO) {
            assert!(s.maggie.is_some(), "passo {i}");
            let k = s.record.1;
            assert!(BOARD_POSES.iter().any(|p| p.0 == k), "pose {k}");
            assert!(BOARD_BALLOONS.iter().any(|p| p.0 == k), "balão {k}");
        }
        // "Não, ignorar" vai ao fim ("Tudo pronto!")
        let mut w = wizard();
        for e in [E::Option(0), E::Next, E::Next] {
            w.on_event(e);
        }
        let steps = steps();
        let Advance::Choice(c) = &steps[9].advance else {
            panic!()
        };
        assert_eq!(c[1], (1, Goto::Step(18)));
        assert_eq!(steps[18].record.1, 41);
    }

    #[test]
    fn throwing_away_offers_to_print_first() {
        let mut w = wizard().starting_at(STUDIO);
        w.on_event(E::Next);
        w.on_event(E::Next);
        w.on_event(E::Tool(TOOL_SUPERTOOLS));
        w.on_event(E::OptionButton {
            tool: TOOL_SUPERTOOLS,
            page: 1,
            k: OPT_BIG_GRID,
        });
        w.on_event(E::McZee);
        w.on_event(E::McZee);
        w.on_event(E::OptionButton {
            tool: TOOL_SUPERTOOLS,
            page: 1,
            k: OPT_NO_GRID,
        });
        w.on_event(E::Option(1));
        assert_eq!(w.step().record.1, 24);
        assert_eq!(w.on_event(E::Option(1)), O::Discard);
        assert_eq!(w.step().record.1, 26);
        assert_eq!(w.on_event(E::Option(0)), O::Print);
        assert_eq!(w.step().record.1, 29);
    }

    #[test]
    fn records_exist_in_block_3800() {
        // o bloco tem 47 balões; todos os índices usados cabem
        for s in steps() {
            assert_eq!(s.record.0, 0x3800);
            assert!(s.record.1 < 47, "{}", s.record.1);
        }
    }
}
