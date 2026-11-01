//! Roteiro dos Truques de 3-D (a mesa do café da manhã): balões do
//! `WHAT 4000` e figuras do `ARTIST.PIC`. Reconstruído dos textos (o Wine
//! não mostra a lição); as figuras de cada passo são as cenas de 312×272
//! (`3e0b`…`4088`) e os quadros de 177×148 (`40bb`…`4111`) da mesa.

use super::wizard::{Advance, Goto, Step, Target, Wizard};
use crate::scene::SceneId;

const BLOCK: u16 = 0x4000;

/// Índices em `TOOLS` da tela de pintura.
pub const TOOL_BRUSH: usize = 0;
pub const TOOL_STICKERS: usize = 2;
pub const TOOL_TRANSFORMERS: usize = 7;
pub const TOOL_SUPERTOOLS: usize = 8;
/// Botões da faixa das Super-ferramentas (ordem da spec de objetos).
pub const OPT_VP: usize = 3;
pub const OPT_VP_LINE: usize = 4;
pub const OPT_VP_PUSH: usize = 6;
/// Botão "Linha" da página 5 do pincel.
pub const OPT_LINE_BRUSH: usize = 0;
/// "Grudar ou desgrudar" na página 2 dos Transformadores.
pub const OPT_LINK: usize = 3;

// cenas do café da manhã (312×272)
const SCENE_DEPTH: u16 = 0x3e0b;
const SCENE_FLAT: u16 = 0x4008;
const SCENE_EYE_MIDDLE: u16 = 0x403b;
const SCENE_EYE_HIGH: u16 = 0x4053;
const SCENE_EYE_TOP: u16 = 0x4059;
const SCENE_BOXES: u16 = 0x4061;
const SCENE_BOWLS: u16 = 0x4083;
// quadros da mesa (177×148)
const TABLE_HORIZON: u16 = 0x40bb;
const TABLE_EDGE: u16 = 0x40c8;
const TABLE_STICKERS: u16 = 0x40d1;
const TABLE_GUIDE_1: u16 = 0x40d9;
const TABLE_GUIDES: u16 = 0x40dd;
const TABLE_BACK: u16 = 0x40e8;
const TABLE_SIDES: u16 = 0x40f1;
const TABLE_DONE: u16 = 0x40f8;

fn step(k: usize, maggie: Option<u16>, advance: Advance) -> Step {
    Step {
        record: (BLOCK, k),
        maggie,
        your_area: None,
        advance,
    }
}

/// Os passos; os índices dos balões são os do bloco `4000`.
pub fn steps() -> Vec<Step> {
    vec![
        // 0: "Maggie, como é que você faz para os seus desenhos parecerem tridimensionais?"
        step(
            0,
            None,
            Advance::Choice(vec![
                (0, Goto::Step(1)),
                (1, Goto::FreePaint),
                (2, Goto::Scene(SceneId::Building)),
            ]),
        ),
        // 1: Intro do ateliê de treinamento
        step(50, None, Advance::Next),
        // 2: Desenhar pode ser uma coisa mágica
        step(3, Some(SCENE_DEPTH), Advance::Next),
        // 3: com/sem profundidade (as opções trocam a figura; Segue continua)
        step(4, Some(SCENE_FLAT), Advance::Next),
        // 4: como você dá a idéia de profundidade — ponto de vista
        step(6, Some(SCENE_EYE_MIDDLE), Advance::Next),
        // 5: um pouco mais para cima
        step(9, Some(SCENE_EYE_HIGH), Advance::Next),
        // 6: bem lá em cima
        step(11, Some(SCENE_EYE_TOP), Advance::Next),
        // 7: clique na caixa mais perto (a da frente, à direita)
        step(
            22,
            Some(SCENE_BOXES),
            Advance::Click(Target::PanelArea((170, 40, 140, 160))),
        ),
        // 8: sobreposição — clique na tigela mais perto (embaixo, à esquerda)
        step(
            29,
            Some(SCENE_BOWLS),
            Advance::Click(Target::PanelArea((0, 150, 170, 122))),
        ),
        // 9: detalhe — clique na tigela com menos detalhes
        step(
            36,
            Some(SCENE_BOWLS),
            Advance::Click(Target::PanelArea((120, 80, 120, 80))),
        ),
        // 10: clique na embalagem de leite
        step(
            42,
            Some(SCENE_BOWLS),
            Advance::Click(Target::PanelArea((0, 40, 110, 130))),
        ),
        // 11: resumo dos truques
        step(44, Some(SCENE_DEPTH), Advance::Next),
        // 12: ferramentas especiais?
        step(
            47,
            None,
            Advance::Choice(vec![(0, Goto::Step(13)), (1, Goto::Step(40))]),
        ),
        // 13: primeiro, a linha do horizonte
        step(48, Some(TABLE_HORIZON), Advance::Next),
        // 14: clique nas Super-ferramentas
        step(49, None, Advance::Click(Target::Tool(TOOL_SUPERTOOLS))),
        // 15: clique na ferramenta Ponto de fuga
        step(52, None, Advance::Click(Target::Option(OPT_VP))),
        // 16: clique na sua área para posicionar
        step(53, Some(TABLE_HORIZON), Advance::PaintThenNext),
        // 17: o círculo pequeno é o ponto de fuga
        step(55, Some(TABLE_HORIZON), Advance::Maggie),
        // 18: linhas guia ligam e desligam?
        step(
            60,
            None,
            Advance::Choice(vec![(0, Goto::Step(19)), (1, Goto::Step(21))]),
        ),
        // 19: interruptor mágico
        step(61, None, Advance::Next),
        // 20: ligue de novo
        step(62, None, Advance::Next),
        // 21: clique no Pincel
        step(63, None, Advance::Click(Target::Tool(TOOL_BRUSH))),
        // 22: a borda da mesa
        step(64, Some(TABLE_EDGE), Advance::Next),
        // 23: clique no pincel Linha
        step(66, None, Advance::Click(Target::Option(OPT_LINE_BRUSH))),
        // 24: desenhe uma linha como esta
        step(67, Some(TABLE_EDGE), Advance::PaintThenNext),
        // 25: já temos uma mesa
        step(70, Some(TABLE_EDGE), Advance::Next),
        // 26: Seleciona-adesivos
        step(71, None, Advance::Click(Target::Tool(TOOL_STICKERS))),
        // 27: ache o rolo do café da manhã
        step(73, Some(TABLE_STICKERS), Advance::Next),
        // 28: coloque os adesivos iguais aos meus
        step(77, Some(TABLE_STICKERS), Advance::Maggie),
        // 29: mudar o ponto de fuga?
        step(
            100,
            None,
            Advance::Choice(vec![(0, Goto::Step(30)), (1, Goto::Step(32))]),
        ),
        // 30: clique na ferramenta Ponto de fuga
        step(102, None, Advance::Click(Target::Option(OPT_VP))),
        // 31: arraste acima da mesa
        step(103, Some(TABLE_STICKERS), Advance::Maggie),
        // 32: arrume os adesivos
        step(126, Some(TABLE_STICKERS), Advance::Maggie),
        // 33: linhas guia do ponto de fuga
        step(137, Some(TABLE_GUIDE_1), Advance::Next),
        // 34: volte às Super-ferramentas
        step(138, None, Advance::Click(Target::Tool(TOOL_SUPERTOOLS))),
        // 35: ferramenta Linha do ponto de fuga
        step(140, None, Advance::Click(Target::Option(OPT_VP_LINE))),
        // 36: primeira linha guia
        step(142, Some(TABLE_GUIDE_1), Advance::Maggie),
        // 37: as outras linhas guia
        step(144, Some(TABLE_GUIDES), Advance::Maggie),
        // 38: Aproximar do PF
        step(146, None, Advance::Click(Target::Option(OPT_VP_PUSH))),
        // 39: a parte de trás da embalagem de leite
        step(148, Some(TABLE_BACK), Advance::Maggie),
        // 40: pronto para os lados — fim da lição (volta ao andar)
        step(
            168,
            Some(TABLE_SIDES),
            Advance::Choice(vec![
                (0, Goto::Step(41)),
                (1, Goto::Scene(SceneId::Building)),
            ]),
        ),
        // 41: olhe os lados prontos
        step(170, Some(TABLE_DONE), Advance::Maggie),
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
    fn records_fit_block_4000() {
        for s in steps() {
            assert_eq!(s.record.0, 0x4000);
            assert!(s.record.1 < 250, "{}", s.record.1);
        }
        assert_eq!(steps().len(), 42);
    }

    #[test]
    fn choices_point_inside_the_script() {
        let n = steps().len();
        for s in steps() {
            if let Advance::Choice(opts) = s.advance {
                for (_, g) in opts {
                    if let Goto::Step(k) = g {
                        assert!(k < n, "{k}");
                    }
                }
            }
        }
    }

    #[test]
    fn first_steps_run() {
        let mut w = wizard();
        assert_eq!(w.on_event(E::Option(0)), O::Entered(1));
        for k in 2..=7 {
            assert_eq!(w.on_event(E::Next), O::Entered(k));
        }
        assert_eq!(w.on_event(E::Panel(200, 100)), O::Entered(8));
    }
}
