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

/// Figuras da Área de Desenho da Maggie.
/// O Chiquinho inteiro.
const PIG: u16 = 0x3827;
/// Contorno azul: o espaço negativo traçado.
const PIG_NEGATIVE: u16 = 0x3835;
/// Só o contorno preto (o espaço positivo).
const PIG_OUTLINE: u16 = 0x3838;
/// Contorno com as formas de dentro apagadas.
const PIG_SHAPES: u16 = 0x3830;
/// Formas de dentro bem claras, para o quadro da criança.
const PIG_FAINT: u16 = 0x3839;
/// Chiquinho com o quadriculado verde (178×149).
const PIG_GRID: u16 = 0x3859;

/// As "3 formas circulares meio achatadas" do Chiquinho, como elipses no
/// painel (`PIG_SHAPES`): a cabeça, o corpo e o focinho.
pub const PIG_CIRCLES: [(i32, i32, i32, i32); 3] =
    [(85, 35, 130, 110), (105, 105, 165, 125), (68, 108, 54, 46)];

fn step(k: usize, maggie: Option<u16>, advance: Advance) -> Step {
    Step {
        record: (BLOCK, k),
        maggie,
        your_area: None,
        advance,
        anim: None,
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
        // 0: "A garotada sempre me pergunta"
        step(
            0,
            None,
            Advance::Choice(vec![
                (0, Goto::Step(1)),
                (1, Goto::FreePaint),
                (2, Goto::Scene(SceneId::TricksFloor)),
            ]),
        ),
        // 1: Intro
        step(3, None, Advance::Next),
        // 2: É divertido desenhar
        step(4, None, Advance::Next),
        // 3: espaço negativo/positivo — clique no porco
        step(
            6,
            Some(PIG),
            Advance::Click(Target::PanelArea((40, 30, 240, 220))),
        ),
        // 4: trace o espaço negativo (a silhueta vai para o seu quadro)
        Step {
            your_area: Some(PIG_FAINT),
            ..step(7, Some(PIG_NEGATIVE), Advance::PaintThenNext)
        },
        // 5: olhe as formas menores — clique no rosto (metade esquerda)
        step(
            8,
            Some(PIG_OUTLINE),
            Advance::Click(Target::PanelArea((0, 0, 160, 272))),
        ),
        // 6: três formas circulares (a Maggie mostra onde estão)
        step(
            9,
            Some(PIG_SHAPES),
            Advance::FindShapes(PIG_CIRCLES.to_vec()),
        ),
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
        step(15, None, supertool(OPT_BIG_GRID)),
        // 12: olhe o Chiquinho com os quadrados
        step(18, Some(PIG_GRID), Advance::Maggie),
        // 13: eu contornei; preencha o resto
        Step {
            your_area: Some(PIG_OUTLINE),
            ..step(20, Some(PIG_GRID), Advance::Maggie)
        },
        // 14: Sem quadriculado
        step(23, None, supertool(OPT_NO_GRID)),
        // 15: Uau, D+ — ver sem o quadriculado (volta a 14) ou deixar
        step(
            40,
            None,
            Advance::Choice(vec![(0, Goto::Step(14)), (1, Goto::Step(17))]),
        ),
        // 16: Tudo pronto ("Não, ignorar" as ferramentas especiais)
        step(
            41,
            None,
            Advance::Choice(vec![
                (0, Goto::Step(3)),
                (1, Goto::Scene(SceneId::TricksFloor)),
            ]),
        ),
        // 17: salvar a figura?
        step(
            24,
            None,
            Advance::Choice(vec![(0, Goto::Step(18)), (1, Goto::DiscardThen(20))]),
        ),
        // 18: digite um nome
        step(25, None, Advance::TypeName),
        // 19: imprimir?
        step(
            27,
            None,
            Advance::Choice(vec![(0, Goto::PrintThen(21)), (1, Goto::Step(21))]),
        ),
        // 20: imprimir antes de jogar fora?
        step(
            26,
            None,
            Advance::Choice(vec![(0, Goto::PrintThen(21)), (1, Goto::Step(21))]),
        ),
        // 21: Chocante! o que fazer em seguida (ordem da tela: nova figura /
        // figura existente, pela pasta / voltar ao andar)
        step(
            29,
            None,
            Advance::Choice(vec![
                (0, Goto::Step(1)),
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
        assert_eq!(w.on_event(E::Option(0)), O::Entered(1));
        assert_eq!(w.on_event(E::Next), O::Entered(2));
        assert_eq!(w.on_event(E::Next), O::Entered(3));
        assert_eq!(w.on_event(E::Panel(150, 150)), O::Entered(4));
        assert_eq!(w.step().your_area, Some(0x3839));
        assert_eq!(w.on_event(E::Painted), O::Stay);
        assert_eq!(w.on_event(E::Next), O::Entered(5));
        assert_eq!(w.on_event(E::Panel(300, 100)), O::Stay);
        assert_eq!(w.on_event(E::Panel(50, 100)), O::Entered(6));
        // as três formas: um clique fora e um repetido não contam
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
        assert_eq!(w.on_event(E::McZee), O::Entered(9));
        assert_eq!(w.on_event(E::Option(0)), O::Entered(10));
        assert_eq!(w.on_event(E::Tool(0)), O::Stay);
        assert_eq!(w.on_event(E::Tool(TOOL_SUPERTOOLS)), O::Entered(11));
        let grid = |k| E::OptionButton {
            tool: TOOL_SUPERTOOLS,
            page: 1,
            k,
        };
        assert_eq!(w.on_event(grid(OPT_BIG_GRID)), O::Entered(12));
        assert_eq!(w.on_event(E::McZee), O::Entered(13));
        assert_eq!(w.on_event(E::McZee), O::Entered(14));
        assert_eq!(w.on_event(grid(OPT_NO_GRID)), O::Entered(15));
        // fim guiado: salvar com nome, imprimir, o que fazer em seguida
        assert_eq!(w.on_event(E::Option(1)), O::Entered(17));
        assert_eq!(w.on_event(E::Option(0)), O::Entered(18));
        for c in "porco".chars() {
            w.on_event(E::Typed(c));
        }
        assert_eq!(w.on_event(E::Enter), O::Save("porco".into()));
        assert_eq!(w.current(), 19);
        assert_eq!(w.on_event(E::Option(1)), O::Entered(21));
        assert_eq!(w.on_event(E::Option(1)), O::OpenFolder);
        assert_eq!(w.on_event(E::Option(2)), O::Leave(SceneId::TricksFloor));
    }

    #[test]
    fn throwing_away_offers_to_print_first() {
        let mut w = wizard();
        w.on_event(E::Option(0));
        for _ in 0..2 {
            w.on_event(E::Next);
        }
        w.on_event(E::Panel(150, 150));
        w.on_event(E::Next);
        w.on_event(E::Panel(50, 100));
        for (x, y, cw, ch) in PIG_CIRCLES {
            w.on_event(E::Panel(x + cw / 2, y + ch / 2));
        }
        assert_eq!(w.current(), 7);
        w.on_event(E::Next);
        w.on_event(E::McZee);
        // "Não, ignorar" as ferramentas: Tudo pronto
        assert_eq!(w.on_event(E::Option(1)), O::Entered(16));
        w.on_event(E::Back);
        w.on_event(E::Option(0));
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
