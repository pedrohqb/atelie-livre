//! Roteiro do Max para a Faixa (`WHAT 3800`, avisos `3802`), transcrito de
//! `cw/report/projects-faixa.md`:
//!
//! ```text
//! [0] amostra ou do zero?
//!  ├─ amostra → pasta MAX › Faixas com [1] (mais amostras) e [2] (Já)
//!  │    → "Lado a lado" + [3] quantas páginas? [campo] Segue
//!  │    → [4] "Vá em frente! Modifique a faixa…" → Max → fim
//!  └─ zero → ([9]+[10] Palavra, se não estiver nela) → [11] Forma
//!       → [12]+[13] cavalete → OK → [16] "Chocante! Quantas páginas…"
//!       [campo 1…8; fora disso 3802[0] do Rex] Segue → "Lado a lado"
//!       → [17] "Vá em frente e termine a faixa…" → Max → fim
//! ```
//!
//! Fim comum: `[19]` salvar?, `0faa[14]` nome, `[22]` imprimir?, `[23]`
//! Imprimir, `[27]` McZee (`[28]` com a faixa aberta), `[20]` lixo, `[24]`
//! "Tudo pronto!" ("por conta própria" volta ao `[4]`). Faixa vazia ao
//! clicar no Max: `3802[1]`.

use super::assistant::Anchor::{Easel, Folder, Max};
use super::assistant::Wait::{On, Options, Pages};
use super::assistant::{
    Anchor, BANNER_MAX_PAGES, Button, Command, Cond, EndRecords, Kind, Pat, Script, Spot, Step,
    Tool, View, go, go_do, say, step,
};

const SIDE_BY_SIDE: &[Command] = &[Command::SetView(View::SideBySide)];

const STEPS: &[Step] = &[
    step(
        "início",
        &[say(0, Max)],
        Options(&[(0, go("amostra")), (1, go("palavra"))]),
    ),
    step(
        "amostra",
        &[say(1, Folder(Spot::More)), say(2, Folder(Spot::Ok))],
        On(&[(Pat::SampleChosen, go_do("páginas-amostra", SIDE_BY_SIDE))]),
    )
    .entering(&[Command::OpenSamples]),
    // [3] "Foi isto que eu comecei. Quantas páginas…" (valor da amostra)
    step(
        "páginas-amostra",
        &[say(3, Max)],
        Pages {
            max: BANNER_MAX_PAGES,
            next: go("mudar"),
        },
    ),
    // [4] dicas [21e:6]→[6] borda (Maggie), [21b:5]→[5], [21b:8]→[8], [222:14]→[7]
    step("mudar", &[say(4, Max)], On(&[(Pat::MaxDone, go("salvar"))])),
    // [9] "Para colocar palavras, clique na ferramenta Palavra" + [10] "Clique aqui."
    step(
        "palavra",
        &[say(9, Max), say(10, Anchor::Tool(Tool::Word))],
        On(&[(Pat::Tool(Tool::Word), go("forma"))]),
    )
    .skip_if(Cond::WordActive, "forma"),
    step(
        "forma",
        &[say(11, Anchor::Button(Button::Shape))],
        On(&[(Pat::Button(Button::Shape), go("mensagem"))]),
    ),
    step(
        "mensagem",
        &[say(12, Easel(Spot::Text)), say(13, Easel(Spot::Ok))],
        On(&[(Pat::ShapedOk, go("páginas"))]),
    ),
    step(
        "páginas",
        &[say(16, Max)],
        Pages {
            max: BANNER_MAX_PAGES,
            next: go_do("livre", SIDE_BY_SIDE),
        },
    ),
    step(
        "livre",
        &[say(17, Max)],
        On(&[(Pat::MaxDone, go("salvar"))]),
    ),
];

pub static BANNER: Script = Script {
    kind: Kind::Banner,
    block: 0x3800,
    alerts: 0x3802,
    blank_alert: 1,
    pages_alert: Some(0),
    fifth_page: None,
    help: 7,
    start: "início",
    free: "mudar",
    end: EndRecords {
        save: 19,
        trash: 20,
        print: 22,
        print_click: 23,
        print_mczee: 27,
        print_open: 28,
        done: 24,
    },
    steps: STEPS,
};

#[cfg(test)]
mod tests {
    use super::super::assistant::tests::{cmds, walk};
    use super::super::assistant::{Assistant, Context, Event as E};
    use super::*;

    fn banner(ctx: Context) -> Assistant {
        let mut a = Assistant::new(Kind::Banner, ctx);
        a.begin();
        a
    }

    #[test]
    fn from_scratch_shaped_words_then_pages() {
        let mut a = banner(Context {
            pages: 1,
            ..Context::default()
        });
        walk(&mut a, &[E::Option(1)]);
        assert_eq!(a.at(), "palavra");
        assert_eq!(a.shown().len(), 2, "[9] e o \"Clique aqui.\" [10]");
        walk(
            &mut a,
            &[
                E::ToolClicked(Tool::Word),
                E::ButtonClicked(Button::Shape),
                E::ShapedOk,
            ],
        );
        assert_eq!(a.at(), "páginas");
        assert_eq!(a.field(), Some("1"));
        assert!(a.buttons().back && a.buttons().next);
        // 9 páginas: o Rex (3802[0], @ = 8)
        walk(&mut a, &[E::Typed('9'), E::Next]);
        let al = a.alert().expect("aviso das páginas");
        assert_eq!((al.block, al.record, al.max), (0x3802, 0, Some(8)));
        walk(&mut a, &[E::Option(0)]);
        walk(&mut a, &[E::Backspace, E::Typed('3')]);
        let out = walk(&mut a, &[E::Next]);
        assert_eq!(
            cmds(&out),
            vec![Command::SetPages(3), Command::SetView(View::SideBySide)]
        );
        assert_eq!(a.at(), "livre");
        // não salvar, jogar fora, por conta própria → [4]
        let out = walk(&mut a, &[E::MaxClicked, E::Option(1), E::Option(1)]);
        assert_eq!(cmds(&out), vec![Command::Discard]);
        assert_eq!(a.shown()[0].record, 24);
        walk(&mut a, &[E::Option(0)]);
        assert_eq!(a.at(), "mudar");
    }

    #[test]
    fn sample_keeps_its_pages_and_word_tool_skip() {
        let mut a = banner(Context {
            pages: 4,
            word_active: true,
            ..Context::default()
        });
        walk(&mut a, &[E::Option(0)]);
        let out = walk(&mut a, &[E::SampleChosen]);
        assert_eq!(cmds(&out), vec![Command::SetView(View::SideBySide)]);
        assert_eq!(a.at(), "páginas-amostra");
        assert_eq!(a.field(), Some("4"));
        let out = walk(&mut a, &[E::Next]);
        assert_eq!(cmds(&out), vec![Command::SetPages(4)]);
        assert_eq!(a.at(), "mudar");
        // "Começar uma nova faixa" do fim: do começo, já com a Palavra
        walk(&mut a, &[E::MaxClicked, E::Option(1), E::Option(1)]);
        let out = walk(&mut a, &[E::Option(1)]);
        assert_eq!(cmds(&out), vec![Command::NewDocument]);
        walk(&mut a, &[E::Option(1)]);
        assert_eq!(a.at(), "forma");
    }

    #[test]
    fn blank_banner_needs_words() {
        let mut a = banner(Context {
            blank: true,
            ..Context::default()
        });
        walk(
            &mut a,
            &[E::Option(0), E::SampleChosen, E::Next, E::MaxClicked],
        );
        assert_eq!(a.alert().map(|x| (x.block, x.record)), Some((0x3802, 1)));
        assert_eq!(a.at(), "mudar");
    }

    #[test]
    fn script_is_closed_and_matches_the_writer_what() {
        super::super::assistant::tests::check_script(&BANNER);
    }
}
