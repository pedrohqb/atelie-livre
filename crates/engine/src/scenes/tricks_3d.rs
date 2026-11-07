//! Roteiro dos Truques de 3-D (a mesa do café da manhã): balões do
//! `WHAT 4000` e figuras do `ARTIST.PIC`. Reconstruído dos textos (o Wine
//! não mostra a lição); as figuras de cada passo são as cenas de 312×272
//! (`3e0b`…`4088`) e os quadros de 177×148 (`40bb`…`4111`) da mesa.

use super::wizard::{Advance, Goto, PanelAnim, Step, Target, Wizard};
use crate::scene::SceneId;

const BLOCK: u16 = 0x4000;

/// Índices em `TOOLS` da tela de pintura.
pub const TOOL_BRUSH: usize = 0;
pub const TOOL_STICKERS: usize = 2;
pub const TOOL_TRANSFORMERS: usize = 7;
pub const TOOL_SUPERTOOLS: usize = 8;
/// Casas da faixa das Super-ferramentas (`supertools::BUTTONS[..].k`).
pub const OPT_VP: usize = 5;
pub const OPT_VP_LINE: usize = 6;
pub const OPT_VP_PUSH: usize = 7;
/// Página e botão "Linha" do pincel.
pub const LINE_BRUSH_PAGE: u8 = 5;
pub const OPT_LINE_BRUSH: usize = 0;
/// Ícones dos balões "clique até ver [ícone]": o pincel Linha (página 5 do
/// pincel) e o Grudar ou desgrudar (cinza, como fica na faixa).
pub const ICON_LINE_BRUSH: u16 = 0x17b5;
pub const ICON_LINK: u16 = 0x14cc;
/// "Grudar ou desgrudar" na página 2 dos Transformadores.
pub const LINK_PAGE: u8 = 2;
pub const OPT_LINK: usize = 3;

// cenas do café da manhã (312×272)
const SCENE_DEPTH: u16 = 0x3e0b;
const SCENE_FLAT: u16 = 0x4008;
const SCENE_EYE_MIDDLE: u16 = 0x403b;
const SCENE_EYE_HIGH: u16 = 0x4053;
const SCENE_EYE_TOP: u16 = 0x4059;
const SCENE_BOXES: u16 = 0x4061;
const SCENE_BOWLS: u16 = 0x4083;
// recortes das cenas (com máscara em `id + 8`), para as animações do painel
/// A tigela de cereais da frente (141×54), em (27, 218) na cena `4083`.
const SPRITE_BOWL: u16 = 0x4069;
/// A embalagem de leite com a vaca e os detalhes (76×152), em (20, 73).
const SPRITE_MILK_DETAIL: u16 = 0x40a3;
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
                (2, Goto::Scene(SceneId::TricksFloor)),
            ]),
        ),
        // 1: Intro do ateliê de treinamento
        step(50, None, Advance::Next),
        // 2: Desenhar pode ser uma coisa mágica
        step(3, Some(SCENE_DEPTH), Advance::Next),
        // 3: com/sem profundidade (os botões trocam a figura; Segue continua)
        step(
            4,
            Some(SCENE_FLAT),
            Advance::Pictures(vec![SCENE_DEPTH, SCENE_FLAT]),
        ),
        // 4: como você dá a idéia de profundidade — ponto de vista (três botões)
        step(
            6,
            Some(SCENE_EYE_MIDDLE),
            Advance::Pictures(vec![SCENE_EYE_MIDDLE, SCENE_EYE_HIGH, SCENE_EYE_TOP]),
        ),
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
        // 8: sobreposição — clique na tigela mais perto (embaixo, à esquerda);
        // ela desliza para a frente do leite
        Step {
            anim: Some(PanelAnim {
                sprite: SPRITE_BOWL,
                from: (27, 218),
                to: (0, 150),
                wipe: false,
                secs: 1.0,
            }),
            ..step(
                29,
                Some(SCENE_BOWLS),
                Advance::Click(Target::PanelArea((0, 150, 170, 122))),
            )
        },
        // 9: detalhe — clique na tigela com menos detalhes
        step(
            36,
            Some(SCENE_BOWLS),
            Advance::Click(Target::PanelArea((120, 80, 120, 80))),
        ),
        // 10: clique na embalagem de leite; os detalhes aparecem aos poucos
        Step {
            anim: Some(PanelAnim {
                sprite: SPRITE_MILK_DETAIL,
                from: (20, 73),
                to: (20, 73),
                wipe: true,
                secs: 1.5,
            }),
            ..step(
                42,
                Some(SCENE_BOWLS),
                Advance::Click(Target::PanelArea((0, 40, 110, 130))),
            )
        },
        // 11: resumo dos truques (a embalagem detalhada fica até o Segue)
        step(44, Some(SCENE_BOWLS), Advance::Next),
        // 12: ferramentas especiais?
        step(
            47,
            Some(SCENE_DEPTH),
            Advance::Choice(vec![(0, Goto::Step(13)), (1, Goto::Step(54))]),
        ),
        // 13: primeiro, a linha do horizonte
        step(48, Some(TABLE_HORIZON), Advance::Next),
        // 14: clique nas Super-ferramentas
        step(49, None, Advance::Click(Target::Tool(TOOL_SUPERTOOLS))),
        // 15: clique na ferramenta Ponto de fuga
        step(52, None, supertool(OPT_VP)),
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
        // 19: interruptor mágico — desligue
        step(61, None, Advance::Click(Target::Switch)),
        // 20: ligue de novo
        step(62, None, Advance::Click(Target::Switch)),
        // 21: clique no Pincel
        step(63, None, Advance::Click(Target::Tool(TOOL_BRUSH))),
        // 22: a borda da mesa
        step(64, Some(TABLE_EDGE), Advance::Next),
        // 23: clique até ver o pincel Linha (página 5)
        Step {
            icons: &[ICON_LINE_BRUSH],
            ..step(
                65,
                None,
                Advance::Click(Target::Page(TOOL_BRUSH, LINE_BRUSH_PAGE)),
            )
        },
        // 24: clique no pincel Linha
        step(
            66,
            None,
            Advance::Click(Target::Option {
                tool: TOOL_BRUSH,
                page: Some(LINE_BRUSH_PAGE),
                k: OPT_LINE_BRUSH,
            }),
        ),
        // 25: desenhe uma linha como esta
        step(67, Some(TABLE_EDGE), Advance::PaintThenNext),
        // 26: já temos uma mesa
        step(70, Some(TABLE_EDGE), Advance::Next),
        // 27: Seleciona-adesivos
        step(71, None, Advance::Click(Target::Tool(TOOL_STICKERS))),
        // 28: ache o rolo do café da manhã
        step(73, Some(TABLE_STICKERS), Advance::Next),
        // 29: coloque os adesivos iguais aos meus
        step(77, Some(TABLE_STICKERS), Advance::Maggie),
        // 30: mudar o ponto de fuga?
        step(
            100,
            None,
            Advance::Choice(vec![(0, Goto::Step(31)), (1, Goto::Step(34))]),
        ),
        // 31: clique aqui (Super-ferramentas)
        step(101, None, Advance::Click(Target::Tool(TOOL_SUPERTOOLS))),
        // 32: clique na ferramenta Ponto de fuga
        step(102, None, supertool(OPT_VP)),
        // 33: arraste acima da mesa
        step(103, Some(TABLE_STICKERS), Advance::Maggie),
        // 34: arrume os adesivos
        step(126, Some(TABLE_STICKERS), Advance::Maggie),
        // 35: linhas guia do ponto de fuga
        step(137, Some(TABLE_GUIDE_1), Advance::Next),
        // 36: volte às Super-ferramentas
        step(138, None, Advance::Click(Target::Tool(TOOL_SUPERTOOLS))),
        // 37: ferramenta Linha do ponto de fuga
        step(140, None, supertool(OPT_VP_LINE)),
        // 38: primeira linha guia
        step(142, Some(TABLE_GUIDE_1), Advance::Maggie),
        // 39: as outras linhas guia
        step(144, Some(TABLE_GUIDES), Advance::Maggie),
        // 40: Aproximar do PF
        step(146, None, supertool(OPT_VP_PUSH)),
        // 41: a parte de trás da embalagem de leite
        step(148, Some(TABLE_BACK), Advance::Maggie),
        // 42: vamos juntar os dois adesivos
        step(155, Some(TABLE_BACK), Advance::Next),
        // 43: clique em Transformadores
        step(156, None, Advance::Click(Target::Tool(TOOL_TRANSFORMERS))),
        // 44: clique na seta até ver o Grudar (página 2)
        Step {
            icons: &[ICON_LINK],
            ..step(
                157,
                None,
                Advance::Click(Target::Page(TOOL_TRANSFORMERS, LINK_PAGE)),
            )
        },
        // 45: ferramenta Grudar ou Desgrudar
        step(
            159,
            None,
            Advance::Click(Target::Option {
                tool: TOOL_TRANSFORMERS,
                page: Some(LINK_PAGE),
                k: OPT_LINK,
            }),
        ),
        // 46: junte os dois adesivos da embalagem (a corrente mágica)
        step(161, Some(TABLE_BACK), Advance::Maggie),
        // 47: os lados da embalagem (sem o Cavalete de adesivos `[164]`–`[167]`,
        // que o port não tem: desenhe com o pincel)
        step(168, Some(TABLE_SIDES), Advance::Maggie),
        // 48: Uau! terminar sozinho / com os passos do leite como guia / próximo passo
        step(
            188,
            Some(TABLE_DONE),
            Advance::Choice(vec![
                (0, Goto::FreePaint),
                (1, Goto::Step(27)),
                (2, Goto::Step(49)),
            ]),
        ),
        // 49: pronto para ver sem as linhas guia?
        step(
            187,
            Some(TABLE_DONE),
            Advance::Choice(vec![(0, Goto::Step(50)), (1, Goto::Step(52))]),
        ),
        // 50: clique nas Super-ferramentas
        step(189, None, Advance::Click(Target::Tool(TOOL_SUPERTOOLS))),
        // 51: Interruptor mágico na posição desligado
        step(190, None, Advance::Click(Target::Switch)),
        // 52: cores e padrões da mesa e da parede
        step(191, Some(SCENE_DEPTH), Advance::Maggie),
        // 53: salvar?
        step(
            194,
            None,
            Advance::Choice(vec![(0, Goto::Step(55)), (1, Goto::DiscardThen(57))]),
        ),
        // 54: "Não, obrigado" às ferramentas: Tudo pronto!
        step(
            230,
            None,
            Advance::Choice(vec![
                (0, Goto::Step(1)),
                (1, Goto::Scene(SceneId::TricksFloor)),
            ]),
        ),
        // 55: digite um nome
        step(196, None, Advance::TypeName),
        // 56: imprimir?
        step(
            198,
            None,
            Advance::Choice(vec![(0, Goto::PrintThen(58)), (1, Goto::Step(58))]),
        ),
        // 57: imprimir antes de ir para o lixo?
        step(
            199,
            None,
            Advance::Choice(vec![(0, Goto::PrintThen(58)), (1, Goto::Step(58))]),
        ),
        // 58: Oba! o que fazer em seguida: nova figura / tornar 3-D um desenho
        // existente (abre a pasta) / voltar ao andar
        step(
            202,
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
    fn records_fit_block_4000() {
        for s in steps() {
            assert_eq!(s.record.0, 0x4000);
            assert!(s.record.1 < 250, "{}", s.record.1);
        }
        assert_eq!(steps().len(), 59);
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

    /// Avança até o passo `target` pelo caminho "sim" de cada passo.
    fn drive(w: &mut Wizard, target: usize) {
        while w.current() != target {
            let s = w.current();
            let ev = match w.step().advance.clone() {
                Advance::Choice(_) => E::Option(0),
                Advance::Next | Advance::Pictures(_) | Advance::PaintThenNext => E::Next,
                Advance::Click(Target::Tool(t)) => E::Tool(t),
                Advance::Click(Target::Option { tool, page, k }) => E::OptionButton {
                    tool,
                    page: page.unwrap_or(1),
                    k,
                },
                Advance::Click(Target::Page(t, p)) => E::PageShown(t, p),
                Advance::Click(Target::Switch) => E::Switch,
                Advance::Click(Target::PanelArea(r)) => E::Panel(r.0 + r.2 / 2, r.1 + r.3 / 2),
                Advance::Click(Target::Panel) | Advance::FindShapes(_) => E::Panel(200, 200),
                Advance::Maggie => E::McZee,
                Advance::TypeName => E::Enter,
            };
            w.on_event(ev);
            assert_ne!(w.current(), s, "preso no passo {s}");
        }
    }

    #[test]
    fn line_brush_needs_page_5_and_vanishing_point_needs_the_supertools() {
        let mut w = wizard();
        drive(&mut w, 24);
        assert_eq!(w.step().record.1, 66);
        let line = |page| E::OptionButton {
            tool: TOOL_BRUSH,
            page,
            k: OPT_LINE_BRUSH,
        };
        assert_eq!(w.on_event(line(1)), O::Stay);
        assert_eq!(w.on_event(line(LINE_BRUSH_PAGE)), O::Entered(25));
        // passo 32: a faixa dos adesivos aberta não vale
        drive(&mut w, 32);
        assert_eq!(w.step().record.1, 102);
        assert_eq!(
            w.on_event(E::OptionButton {
                tool: TOOL_STICKERS,
                page: 1,
                k: OPT_VP
            }),
            O::Stay
        );
        assert_eq!(
            w.on_event(E::OptionButton {
                tool: TOOL_SUPERTOOLS,
                page: 1,
                k: OPT_VP
            }),
            O::Entered(33)
        );
    }

    #[test]
    fn click_until_you_see_shows_the_port_icon() {
        // "clique até ver [ícone]": o ícone do WHAT não está no .PIC do port
        let st = steps();
        assert_eq!(st[23].record.1, 65);
        assert_eq!(st[23].icons, &[ICON_LINE_BRUSH]);
        assert_eq!(st[44].record.1, 157);
        assert_eq!(st[44].icons, &[ICON_LINK]);
    }

    #[test]
    fn the_two_milk_stickers_are_linked_with_the_transformers() {
        let mut w = wizard();
        drive(&mut w, 41);
        assert_eq!(w.step().record.1, 148);
        assert_eq!(w.on_event(E::McZee), O::Entered(42));
        assert_eq!(w.step().record.1, 155);
        assert_eq!(w.on_event(E::Next), O::Entered(43));
        assert_eq!(w.on_event(E::Tool(TOOL_TRANSFORMERS)), O::Entered(44));
        // [157] "clique na seta até ver": a página 1 não basta
        assert_eq!(w.on_event(E::PageShown(TOOL_TRANSFORMERS, 1)), O::Stay);
        assert_eq!(
            w.on_event(E::PageShown(TOOL_TRANSFORMERS, LINK_PAGE)),
            O::Entered(45)
        );
        assert_eq!(w.step().record.1, 159);
        let link = |tool, page| E::OptionButton {
            tool,
            page,
            k: OPT_LINK,
        };
        assert_eq!(w.on_event(link(TOOL_SUPERTOOLS, LINK_PAGE)), O::Stay);
        assert_eq!(w.on_event(link(TOOL_TRANSFORMERS, 1)), O::Stay);
        assert_eq!(
            w.on_event(link(TOOL_TRANSFORMERS, LINK_PAGE)),
            O::Entered(46)
        );
        assert_eq!(w.step().record.1, 161);
        assert_eq!(w.on_event(E::McZee), O::Entered(47));
        assert_eq!(w.step().record.1, 168);
    }

    #[test]
    fn the_end_is_guided_and_every_step_has_a_way_out() {
        for s in steps() {
            if let Advance::Choice(opts) = &s.advance {
                assert!(!opts.is_empty(), "{}", s.record.1);
            }
        }
        let mut w = wizard();
        drive(&mut w, 47);
        assert_eq!(w.step().record.1, 168);
        // 168: Maggie → 188 → próximo passo → 187 → sim → Super-ferramentas → interruptor
        assert_eq!(w.on_event(E::McZee), O::Entered(48));
        assert_eq!(w.on_event(E::Option(2)), O::Entered(49));
        assert_eq!(w.on_event(E::Option(0)), O::Entered(50));
        assert_eq!(w.on_event(E::Tool(TOOL_SUPERTOOLS)), O::Entered(51));
        assert_eq!(w.on_event(E::Switch), O::Entered(52));
        assert_eq!(w.on_event(E::McZee), O::Entered(53));
        // salvar: nome, imprimir, o que fazer em seguida
        assert_eq!(w.on_event(E::Option(0)), O::Entered(55));
        w.on_event(E::Typed('m'));
        assert_eq!(w.on_event(E::Enter), O::Save("m".into()));
        assert_eq!(w.current(), 56);
        assert_eq!(w.on_event(E::Option(0)), O::Print);
        assert_eq!(w.current(), 58);
        assert_eq!(w.step().record.1, 202);
        assert_eq!(w.on_event(E::Option(1)), O::OpenFolder);
        assert_eq!(w.on_event(E::Option(2)), O::Leave(SceneId::TricksFloor));
    }
}
