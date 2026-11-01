//! Roteiro dos Truques básicos (o porco Chiquinho): balões do `WHAT 3800`
//! e figuras do `ARTIST.PIC` (313×272). Tabela na spec
//! `docs/superpowers/specs/2026-10-31-truques-design.md`.

use super::wizard::{Advance, Goto, Step, Target, Wizard};
use crate::scene::SceneId;

const BLOCK: u16 = 0x3800;

/// Índices em `TOOLS` da tela de pintura.
pub const TOOL_SUPERTOOLS: usize = 8;
/// Botões da faixa das Super-ferramentas.
pub const OPT_NO_GRID: usize = 0;
pub const OPT_BIG_GRID: usize = 1;

/// Figuras da Área de Desenho da Maggie.
const PIG_FAINT: u16 = 0x3827;
const PIG_OUTLINE: u16 = 0x382c;
const PIG_GRID: u16 = 0x382d;
const PIG_GRID_BLACK: u16 = 0x3830;
const PIG_SILHOUETTE: u16 = 0x3831;
const PIG_SHAPES: u16 = 0x3838;

fn step(k: usize, maggie: Option<u16>, advance: Advance) -> Step {
    Step {
        record: (BLOCK, k),
        maggie,
        your_area: None,
        advance,
    }
}

/// Os passos, na ordem da tabela da spec.
pub fn steps() -> Vec<Step> {
    vec![
        // 0: "A garotada sempre me pergunta"
        step(
            0,
            None,
            Advance::Choice(vec![
                (0, Goto::Step(1)),
                (1, Goto::FreePaint),
                (2, Goto::Scene(SceneId::Building)),
            ]),
        ),
        // 1: Intro
        step(3, None, Advance::Next),
        // 2: É divertido desenhar
        step(4, None, Advance::Next),
        // 3: espaço negativo/positivo — clique no porco
        step(
            6,
            Some(PIG_FAINT),
            Advance::Click(Target::PanelArea((40, 30, 240, 220))),
        ),
        // 4: trace o espaço negativo (a silhueta vai para o seu quadro)
        Step {
            record: (BLOCK, 7),
            maggie: Some(PIG_OUTLINE),
            your_area: Some(PIG_SILHOUETTE),
            advance: Advance::PaintThenNext,
        },
        // 5: olhe as formas menores — clique no rosto (metade esquerda)
        step(
            8,
            Some(PIG_OUTLINE),
            Advance::Click(Target::PanelArea((0, 0, 160, 272))),
        ),
        // 6: três formas circulares
        step(9, Some(PIG_SHAPES), Advance::ClicksThenMaggie(3)),
        // 7: acabe de desenhar
        step(10, Some(PIG_SHAPES), Advance::Next),
        // 8: Oba! truques 1 e 2
        step(11, None, Advance::Maggie),
        // 9: ferramentas especiais?
        step(
            13,
            None,
            Advance::Choice(vec![(0, Goto::Step(10)), (1, Goto::Step(16))]),
        ),
        // 10: clique nas Super-ferramentas
        step(14, None, Advance::Click(Target::Tool(TOOL_SUPERTOOLS))),
        // 11: Quadriculado grande
        step(15, None, Advance::Click(Target::Option(OPT_BIG_GRID))),
        // 12: olhe o Chiquinho com os quadrados
        step(18, Some(PIG_GRID), Advance::Maggie),
        // 13: eu contornei; preencha o resto
        Step {
            record: (BLOCK, 20),
            maggie: Some(PIG_GRID_BLACK),
            your_area: Some(PIG_SILHOUETTE),
            advance: Advance::Maggie,
        },
        // 14: Sem quadriculado
        step(23, None, Advance::Click(Target::Option(OPT_NO_GRID))),
        // 15: Uau, D+
        step(
            40,
            None,
            Advance::Choice(vec![(0, Goto::Step(14)), (1, Goto::Step(16))]),
        ),
        // 16: Tudo pronto
        step(
            41,
            None,
            Advance::Choice(vec![
                (0, Goto::Step(3)),
                (1, Goto::Scene(SceneId::Building)),
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
        assert_eq!(w.on_event(E::Option(0)), O::Entered(1));
        assert_eq!(w.on_event(E::Next), O::Entered(2));
        assert_eq!(w.on_event(E::Next), O::Entered(3));
        assert_eq!(w.on_event(E::Panel(150, 150)), O::Entered(4));
        assert_eq!(w.step().your_area, Some(0x3831));
        assert_eq!(w.on_event(E::Painted), O::Stay);
        assert_eq!(w.on_event(E::Next), O::Entered(5));
        assert_eq!(w.on_event(E::Panel(300, 100)), O::Stay);
        assert_eq!(w.on_event(E::Panel(50, 100)), O::Entered(6));
        for _ in 0..3 {
            w.on_event(E::Panel(10, 10));
        }
        assert_eq!(w.current(), 7);
        assert_eq!(w.on_event(E::Next), O::Entered(8));
        assert_eq!(w.on_event(E::McZee), O::Entered(9));
        assert_eq!(w.on_event(E::Option(0)), O::Entered(10));
        assert_eq!(w.on_event(E::Tool(0)), O::Stay);
        assert_eq!(w.on_event(E::Tool(TOOL_SUPERTOOLS)), O::Entered(11));
        assert_eq!(w.on_event(E::OptionButton(OPT_BIG_GRID)), O::Entered(12));
        assert_eq!(w.on_event(E::McZee), O::Entered(13));
        assert_eq!(w.on_event(E::McZee), O::Entered(14));
        assert_eq!(w.on_event(E::OptionButton(OPT_NO_GRID)), O::Entered(15));
        assert_eq!(w.on_event(E::Option(1)), O::Entered(16));
        assert_eq!(w.on_event(E::Option(1)), O::Leave(SceneId::Building));
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
