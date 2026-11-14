//! Roteiro do Max para o Cartão francês (`WHAT 3c00`, avisos `3c02`),
//! transcrito de `cw/report/projects-cartao.md`:
//!
//! ```text
//! [0] começar (22c:1) ou ver como se dobra (22c:0)?
//!  └─ dobra: [2] folha → Segue (anima) → [3] "Dobro no meio…" → Segue
//!            (anima) → [4] "…no meio de novo." → Segue ─┐
//! [19] cartão pronto do Max ou em branco? ◄─────────────┘
//!  ├─ pronto → pasta MAX › Cartões com [22] (Já) e [23] (mais) → "Duas
//!  │    páginas" → [24] usar / ver outro / do zero
//!  │      usar → [26] "Use qualquer uma das ferramentas…" → Max → [87]
//!  └─ branco → "Todas as páginas" → [14] rádios Todas / Uma de cada vez
//!       Uma → [15] Visores → [11] alavanca → [18] Visão normal
//!     [42] rádios Alto / Largo — Largo → [10] Preparação da página →
//!       [45] Alto/Largo → [46] Largo
//!     [48] borda? — quero → ([10]) → [50] alavanca → [44] rolo BORDA →
//!       [51] escolha → [52] clique no cartão
//!     [53] figura? — é → [54] Seleciona-adesivos → [55] rolo ou lápis
//!       rolo → [57] adesivo → [58] clique no cartão; lápis → [59]+[60] → Max
//!       → [61] tamanho? — é → [62] Transformadores → [63] … Max
//!       → [65] "Coloque quantos adesivos quiser…" → Max
//!     [67] palavras com forma? — é → ([68] Palavra) → [69] Forma →
//!       [70]+[71] cavalete → OK → [75] mais palavras (Forma) ou Max
//!                                 — não → [26]
//!     [87] "Estamos quase no fim…" — mudar → [88] → Max
//!     [31] rádios etiquetas — quero → ([10]) → [12] Nº de página →
//!       [13] "Pág. 1"
//! ```
//!
//! Fim comum: `[36]` salvar?, `0faa[14]` nome, `[39]` imprimir?, `[40]`
//! Imprimir, `[108]` McZee (`[109]` com a faixa aberta), `[37]` lixo,
//! `[41]` "Foi divertido…" ("por conta própria" volta ao `[26]`). Cartão
//! vazio ao clicar no Max: `3c02[1]`; quinta página: `3c00[106]`.
//!
//! Não estão no roteiro (sobras do original ou não observados): `[5]`,
//! `[7]`, `[20]`/`[21]` (pasta que não abre sozinha), `[25]`, `[49]`
//! ("Clique em NADA"), `[56]` (é a dica `[21b:38]` do `[55]`), `[72]`–`[74]`
//! (girar/preferidos, sobre os botões do cavalete), `[78]`–`[80]`,
//! `[104]`, `[107]`, e os marcadores "same as #N".

use super::assistant::Anchor::{Easel, Folder, Max};
use super::assistant::Wait::{Next, On, Options, Radios};
use super::assistant::{
    Anchor, Button, Command, Cond, EndRecords, Kind, LABEL_PAGE, Menu, Pat, Script, Spot, Step,
    Tool, View, WIDE, go, go_do, say, step,
};
use super::fold_figure::FoldStage;

const STEPS: &[Step] = &[
    step(
        "início",
        &[say(0, Max)],
        Options(&[(1, go("branco?")), (0, go("dobra"))]),
    ),
    // a figura da dobra: o Segue anima, o Volta desdobra
    step("dobra", &[say(2, Max)], Next(go("dobra-meio"))).folded(FoldStage::Sheet),
    step("dobra-meio", &[say(3, Max)], Next(go("dobra-de-novo"))).folded(FoldStage::Half),
    step("dobra-de-novo", &[say(4, Max)], Next(go("branco?"))).folded(FoldStage::Quarter),
    step(
        "branco?",
        &[say(19, Max)],
        Options(&[
            (0, go("amostra")),
            (1, go_do("todas?", &[Command::SetView(View::AllPages)])),
        ]),
    ),
    step(
        "amostra",
        &[say(22, Folder(Spot::Ok)), say(23, Folder(Spot::More))],
        On(&[(
            Pat::SampleChosen,
            go_do("gostou?", &[Command::SetView(View::TwoPages)]),
        )]),
    )
    .entering(&[Command::OpenSamples]),
    step(
        "gostou?",
        &[say(24, Max)],
        Options(&[
            (0, go("usar")),
            (1, go("amostra")),
            (
                2,
                go_do(
                    "todas?",
                    &[Command::NewDocument, Command::SetView(View::AllPages)],
                ),
            ),
        ]),
    ),
    // [26] dicas [21b:1b]→[27] Mudar visão, [21b:9]→[9] som, [222:14]→[28]
    step("usar", &[say(26, Max)], On(&[(Pat::MaxDone, go("quase"))])),
    // rádios [22e:1] Todas (marcado) / [22e:0] Uma de cada vez
    step(
        "todas?",
        &[say(14, Max)],
        Radios {
            default: 1,
            next: &[(1, go("alto?")), (0, go("visores"))],
        },
    ),
    step(
        "visores",
        &[say(15, Anchor::Tool(Tool::Viewer))],
        On(&[(Pat::Tool(Tool::Viewer), go("alavanca-visores"))]),
    )
    .skip_if(Cond::StripOpen(Tool::Viewer), "alavanca-visores"),
    step(
        "alavanca-visores",
        &[say(11, Anchor::Button(Button::Lever))],
        On(&[(Pat::Button(Button::Lever), go("visão-normal"))]),
    ),
    step(
        "visão-normal",
        &[say(18, Anchor::Button(Button::ViewNormal))],
        On(&[(Pat::Button(Button::ViewNormal), go("alto?"))]),
    ),
    // rádios [22e:1] Alto (marcado) / [22e:0] Largo
    step(
        "alto?",
        &[say(42, Max)],
        Radios {
            default: 1,
            next: &[(1, go("borda?")), (0, go("largo"))],
        },
    ),
    step(
        "largo",
        &[say(10, Anchor::Tool(Tool::Page))],
        On(&[(Pat::Tool(Tool::Page), go("visual"))]),
    )
    .skip_if(Cond::StripOpen(Tool::Page), "visual"),
    step(
        "visual",
        &[say(45, Anchor::Button(Button::Orientation))],
        On(&[(Pat::Button(Button::Orientation), go("clique-largo"))]),
    ),
    step(
        "clique-largo",
        &[say(46, Anchor::Menu(Menu::Orientation, WIDE))],
        On(&[(Pat::Menu(Menu::Orientation, Some(WIDE)), go("borda?"))]),
    ),
    step(
        "borda?",
        &[say(48, Max)],
        Options(&[(0, go("borda")), (1, go("figura?")), (2, go("figura?"))]),
    ),
    step(
        "borda",
        &[say(10, Anchor::Tool(Tool::Page))],
        On(&[(Pat::Tool(Tool::Page), go("alavanca-borda"))]),
    )
    .skip_if(Cond::StripOpen(Tool::Page), "alavanca-borda"),
    step(
        "alavanca-borda",
        &[say(50, Anchor::Button(Button::Lever))],
        On(&[(Pat::Button(Button::Lever), go("rolo-borda"))]),
    ),
    step(
        "rolo-borda",
        &[say(44, Anchor::Button(Button::BorderRoll))],
        On(&[(Pat::Button(Button::BorderRoll), go("escolha-borda"))]),
    ),
    step(
        "escolha-borda",
        &[say(51, Anchor::Button(Button::BorderRoll))],
        On(&[(Pat::BorderPicked, go("aplica-borda"))]),
    ),
    // a borda é aplicada com o clique (som 0220, da tela)
    step(
        "aplica-borda",
        &[say(52, Anchor::Page)],
        On(&[(Pat::Page, go("figura?"))]),
    ),
    step(
        "figura?",
        &[say(53, Max)],
        Options(&[(0, go("adesivos")), (1, go("palavras?"))]),
    ),
    step(
        "adesivos",
        &[say(54, Anchor::Tool(Tool::Stickers))],
        On(&[(Pat::Tool(Tool::Stickers), go("rolo-adesivos"))]),
    )
    .skip_if(Cond::StripOpen(Tool::Stickers), "rolo-adesivos"),
    // [55] um rolo ([21b:38]→[56] mais escolhas) ou o lápis de cera [80a]
    step(
        "rolo-adesivos",
        &[say(55, Max)],
        On(&[
            (Pat::Button(Button::StickerRoll), go("adesivo")),
            (Pat::Button(Button::Crayon), go("lápis")),
        ]),
    ),
    step(
        "adesivo",
        &[say(57, Anchor::Button(Button::StickerRoll))],
        On(&[(Pat::StickerPicked, go("põe-adesivo"))]),
    ),
    step(
        "põe-adesivo",
        &[say(58, Anchor::Page)],
        On(&[(Pat::StickerPlaced, go("tamanho?"))]),
    ),
    step(
        "lápis",
        &[say(59, Anchor::Page), say(60, Max)],
        On(&[(Pat::Max, go("tamanho?"))]),
    ),
    step(
        "tamanho?",
        &[say(61, Max)],
        Options(&[(0, go("transformadores")), (1, go("mais-adesivos"))]),
    ),
    step(
        "transformadores",
        &[say(62, Anchor::Tool(Tool::Transformers))],
        On(&[(Pat::Tool(Tool::Transformers), go("transformar"))]),
    ),
    // [63] "[222:14] Precisa de ajuda?" aqui é o [64]
    step(
        "transformar",
        &[say(63, Max)],
        On(&[(Pat::Max, go("mais-adesivos"))]),
    )
    .help(64),
    // [65] [21e:42]→[66] (Maggie)
    step(
        "mais-adesivos",
        &[say(65, Max)],
        On(&[(Pat::Max, go("palavras?"))]),
    ),
    step(
        "palavras?",
        &[say(67, Max)],
        Options(&[(0, go("palavra")), (1, go("usar"))]),
    ),
    step(
        "palavra",
        &[say(68, Anchor::Tool(Tool::Word))],
        On(&[(Pat::Tool(Tool::Word), go("forma"))]),
    )
    .skip_if(Cond::WordActive, "forma"),
    step(
        "forma",
        &[say(69, Anchor::Button(Button::Shape))],
        On(&[(Pat::Button(Button::Shape), go("mensagem"))]),
    ),
    step(
        "mensagem",
        &[say(70, Easel(Spot::Text)), say(71, Easel(Spot::Ok))],
        On(&[(Pat::ShapedOk, go("mais-palavras"))]),
    ),
    // [75] Forma outra vez põe mais palavras; [21b:804c]→[76]
    step(
        "mais-palavras",
        &[say(75, Max)],
        On(&[
            (Pat::MaxDone, go("quase")),
            (Pat::Button(Button::Shape), go("mensagem")),
        ]),
    ),
    step(
        "quase",
        &[say(87, Max)],
        Options(&[(0, go("etiquetas?")), (1, go("mudar"))]),
    ),
    step(
        "mudar",
        &[say(88, Max)],
        On(&[(Pat::MaxDone, go("etiquetas?"))]),
    ),
    // rádios [22e:0] É, eu quero / [22e:1] Sem etiquetas (marcado)
    step(
        "etiquetas?",
        &[say(31, Max)],
        Radios {
            default: 1,
            next: &[(0, go("etiquetas")), (1, go("salvar"))],
        },
    ),
    step(
        "etiquetas",
        &[say(10, Anchor::Tool(Tool::Page))],
        On(&[(Pat::Tool(Tool::Page), go("botão-etiquetas"))]),
    )
    .skip_if(Cond::StripOpen(Tool::Page), "botão-etiquetas"),
    step(
        "botão-etiquetas",
        &[say(12, Anchor::Button(Button::PageNumber))],
        On(&[(Pat::Button(Button::PageNumber), go("pág-1"))]),
    ),
    // menu nada / "Pág. 1" / "1": [13] aponta "Pág. 1"
    step(
        "pág-1",
        &[say(13, Anchor::Menu(Menu::PageNumber, LABEL_PAGE))],
        On(&[(Pat::Menu(Menu::PageNumber, Some(LABEL_PAGE)), go("salvar"))]),
    ),
];

pub static CARD: Script = Script {
    kind: Kind::Card,
    block: 0x3c00,
    alerts: 0x3c02,
    blank_alert: 1,
    pages_alert: None,
    fifth_page: Some(106),
    help: 28,
    start: "início",
    free: "usar",
    end: EndRecords {
        save: 36,
        trash: 37,
        print: 39,
        print_click: 40,
        print_mczee: 108,
        print_open: 109,
        done: 41,
    },
    steps: STEPS,
};

#[cfg(test)]
mod tests {
    use super::super::assistant::tests::{cmds, walk};
    use super::super::assistant::{Assistant, Context, Event as E};
    use super::super::fold_figure::FRAME_SECS;
    use super::*;

    fn card(ctx: Context) -> Assistant {
        let mut a = Assistant::new(Kind::Card, ctx);
        a.begin();
        a
    }

    #[test]
    fn fold_explanation_animates_and_back_unfolds() {
        let mut a = card(Context::default());
        assert!(a.fold().is_none());
        // a primeira opção na tela é "Começar" (22c:1); a dobra é a 22c:0
        walk(&mut a, &[E::Option(0)]);
        assert_eq!(a.at(), "dobra");
        assert_eq!(a.fold().map(|f| f.frame().pic), Some(0x0c65));
        walk(&mut a, &[E::Next]);
        assert_eq!(a.at(), "dobra-meio");
        assert!(a.fold().unwrap().animating());
        a.tick(FRAME_SECS + 0.001);
        assert_eq!(a.fold().unwrap().frame().pic, 0x3c0b);
        a.tick(1.0);
        assert_eq!(a.fold().unwrap().frame().pic, 0x3c1b);
        // Volta desfaz a dobra
        walk(&mut a, &[E::Back]);
        assert_eq!(a.at(), "dobra");
        a.tick(1.0);
        assert_eq!(a.fold().unwrap().frame().pic, 0x0c65);
        walk(&mut a, &[E::Next, E::Next]);
        a.tick(2.0);
        assert_eq!(a.fold().unwrap().frame().pic, 0x3c33);
        walk(&mut a, &[E::Next]);
        assert_eq!(a.at(), "branco?");
        assert!(a.fold().is_none());
    }

    #[test]
    fn blank_card_whole_path_to_print() {
        let mut a = card(Context::default());
        let out = walk(&mut a, &[E::Option(1), E::Option(1)]);
        assert_eq!(cmds(&out), vec![Command::SetView(View::AllPages)]);
        assert_eq!((a.at(), a.radio()), ("todas?", Some(1)));
        // uma página de cada vez
        walk(
            &mut a,
            &[
                E::Radio(0),
                E::Next,
                E::ToolClicked(Tool::Viewer),
                E::ButtonClicked(Button::Lever),
                E::ButtonClicked(Button::ViewNormal),
            ],
        );
        assert_eq!(a.at(), "alto?");
        // largo
        walk(
            &mut a,
            &[
                E::Radio(0),
                E::Next,
                E::ToolClicked(Tool::Page),
                E::ButtonClicked(Button::Orientation),
                E::MenuItem(Menu::Orientation, 0),
            ],
        );
        assert_eq!(a.at(), "clique-largo", "Alto não serve");
        walk(&mut a, &[E::MenuItem(Menu::Orientation, WIDE)]);
        assert_eq!(a.at(), "borda?");
        // a faixa da Preparação da página continua aberta: pula o [10]
        a.ctx.open_strip = Some(Tool::Page);
        walk(&mut a, &[E::Option(0)]);
        assert_eq!(a.at(), "alavanca-borda");
        walk(
            &mut a,
            &[
                E::ButtonClicked(Button::Lever),
                E::ButtonClicked(Button::BorderRoll),
                E::BorderPicked,
                E::PageClicked,
            ],
        );
        assert_eq!(a.at(), "figura?");
        walk(
            &mut a,
            &[
                E::Option(0),
                E::ToolClicked(Tool::Stickers),
                E::ButtonClicked(Button::StickerRoll),
                E::StickerPicked,
                E::StickerPlaced,
            ],
        );
        assert_eq!(a.at(), "tamanho?");
        walk(&mut a, &[E::Option(0), E::ToolClicked(Tool::Transformers)]);
        assert_eq!(a.at(), "transformar");
        // "Precisa de ajuda?" deste passo é o [64]
        walk(&mut a, &[E::NeedHelp]);
        assert_eq!(a.tip().map(|t| t.record), Some(64));
        walk(&mut a, &[E::MaxClicked, E::MaxClicked]);
        assert_eq!(a.at(), "palavras?");
        a.ctx.open_strip = None;
        walk(
            &mut a,
            &[
                E::Option(0),
                E::ToolClicked(Tool::Word),
                E::ButtonClicked(Button::Shape),
                E::ShapedOk,
            ],
        );
        assert_eq!(a.at(), "mais-palavras");
        // Forma de novo: mais palavras
        walk(&mut a, &[E::ButtonClicked(Button::Shape), E::ShapedOk]);
        assert_eq!(a.at(), "mais-palavras");
        walk(&mut a, &[E::MaxClicked, E::Option(1), E::MaxClicked]);
        assert_eq!(a.at(), "etiquetas?");
        walk(
            &mut a,
            &[
                E::Radio(0),
                E::Next,
                E::ToolClicked(Tool::Page),
                E::ButtonClicked(Button::PageNumber),
                E::MenuItem(Menu::PageNumber, LABEL_PAGE),
            ],
        );
        assert_eq!(a.at(), "salvar");
        assert_eq!(a.shown()[0].record, 36);
        walk(&mut a, &[E::Option(0), E::Saved, E::Option(0)]);
        assert_eq!(a.shown()[0].record, 40);
        walk(&mut a, &[E::ToolClicked(Tool::Print)]);
        assert_eq!(a.shown()[0].record, 108);
        walk(&mut a, &[E::ButtonClicked(Button::PrintMcZee), E::Printed]);
        assert_eq!(a.shown()[0].record, 41);
        walk(&mut a, &[E::Option(0)]);
        assert_eq!(a.at(), "usar");
    }

    #[test]
    fn crayon_and_skipping_everything() {
        let mut a = card(Context::default());
        walk(
            &mut a,
            &[
                E::Option(1),
                E::Option(1),
                E::Next,
                E::Next,
                E::Option(2),
                E::Option(0),
                E::ToolClicked(Tool::Stickers),
                E::ButtonClicked(Button::Crayon),
            ],
        );
        assert_eq!(a.at(), "lápis");
        walk(&mut a, &[E::MaxClicked, E::Option(1), E::MaxClicked]);
        assert_eq!(a.at(), "palavras?");
        walk(&mut a, &[E::Option(1)]);
        assert_eq!(a.at(), "usar");
        // etiquetas: o padrão é "Sem etiquetas", direto ao salvar
        walk(&mut a, &[E::MaxClicked, E::Option(0), E::Next]);
        assert_eq!(a.at(), "salvar");
    }

    #[test]
    fn sample_card_use_another_or_restart() {
        let mut a = card(Context::default());
        walk(&mut a, &[E::Option(1)]);
        let out = walk(&mut a, &[E::Option(0)]);
        assert_eq!(cmds(&out), vec![Command::OpenSamples]);
        let out = walk(&mut a, &[E::SampleChosen]);
        assert_eq!(cmds(&out), vec![Command::SetView(View::TwoPages)]);
        // ver outro cartão: a pasta abre de novo
        let out = walk(&mut a, &[E::Option(1)]);
        assert_eq!(cmds(&out), vec![Command::OpenSamples]);
        walk(&mut a, &[E::SampleChosen]);
        let out = walk(&mut a, &[E::Option(2)]);
        assert_eq!(
            cmds(&out),
            vec![Command::NewDocument, Command::SetView(View::AllPages)]
        );
        assert_eq!(a.at(), "todas?");
        assert!(!a.buttons().back, "documento novo: sem Volta");
        // usar uma amostra
        let mut b = card(Context::default());
        walk(
            &mut b,
            &[E::Option(1), E::Option(0), E::SampleChosen, E::Option(0)],
        );
        assert_eq!(b.at(), "usar");
        walk(&mut b, &[E::Tip(27)]);
        assert_eq!(b.tip().map(|t| t.record), Some(27));
    }

    #[test]
    fn fifth_page_and_blank_card() {
        let mut a = card(Context {
            blank: true,
            ..Context::default()
        });
        walk(&mut a, &[E::FifthPage]);
        assert_eq!(a.alert().map(|x| (x.block, x.record)), Some((0x3c00, 106)));
        walk(&mut a, &[E::Option(1)]);
        assert!(a.alert().is_none());
        walk(
            &mut a,
            &[E::Option(1), E::Option(0), E::SampleChosen, E::Option(0)],
        );
        walk(&mut a, &[E::MaxClicked]);
        assert_eq!(a.alert().map(|x| (x.block, x.record)), Some((0x3c02, 1)));
        assert_eq!(a.at(), "usar");
    }

    #[test]
    fn script_is_closed_and_matches_the_writer_what() {
        super::super::assistant::tests::check_script(&CARD);
    }
}
