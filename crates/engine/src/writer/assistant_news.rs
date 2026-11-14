//! Roteiro do Max para o Jornal (`WHAT 3a00`, avisos `3a02`), transcrito de
//! `cw/report/projects-jornal.md`:
//!
//! ```text
//! [1] amostra ou do zero?
//!  ├─ amostra → pasta MAX › Jornais com [2] (mais amostras) e [3] (Já)
//!  │            → [4] "OK, faça as mudanças…" → Max → fim
//!  └─ zero → [22] mudar as colunas?
//!        ├─ é → [16] Preparação da página → [18] Cria-colunas → [19] número
//!        └─ não ─┐
//!      [24] manchete? ─ é → [17] Palavra → [25] Forma → [26]+[27] cavalete → OK
//!      [0] "OK, vá em frente e faça o seu jornal…" → Max → fim
//! ```
//!
//! Fim comum: `[7]` salvar?, `0faa[14]` nome, `[10]` imprimir?, `[11]`
//! Imprimir, `[20]` McZee (`[21]` com a faixa aberta), `[8]` lixo, `[12]`
//! "Tudo pronto!" ("por conta própria" volta ao `[0]`).

use super::assistant::Anchor::{Easel, Folder, Max};
use super::assistant::Wait::{On, Options};
use super::assistant::{
    Anchor, Button, Command, Cond, EndRecords, Kind, Menu, Pat, Script, Spot, Step, Tool, go, say,
    step,
};

const STEPS: &[Step] = &[
    step(
        "início",
        &[say(1, Max)],
        Options(&[(0, go("amostra")), (1, go("colunas?"))]),
    ),
    step(
        "amostra",
        &[say(2, Folder(Spot::More)), say(3, Folder(Spot::Ok))],
        On(&[(Pat::SampleChosen, go("amostra-ok"))]),
    )
    .entering(&[Command::OpenSamples]),
    step(
        "amostra-ok",
        &[say(4, Max)],
        On(&[(Pat::MaxDone, go("salvar"))]),
    ),
    step(
        "colunas?",
        &[say(22, Max)],
        Options(&[(0, go("colunas")), (1, go("manchete?"))]),
    ),
    // [16] "Primeiro, clique aqui." (Preparação da página, 180,28)
    step(
        "colunas",
        &[say(16, Anchor::Tool(Tool::Page))],
        On(&[(Pat::Tool(Tool::Page), go("cria-colunas"))]),
    )
    .skip_if(Cond::StripOpen(Tool::Page), "cria-colunas"),
    step(
        "cria-colunas",
        &[say(18, Anchor::Button(Button::Columns))],
        On(&[(Pat::Button(Button::Columns), go("n-colunas"))]),
    ),
    // menu de 1 a 4 colunas (sons 022d + 021c, da tela)
    step(
        "n-colunas",
        &[say(19, Anchor::Menu(Menu::Columns, 1))],
        On(&[(Pat::Menu(Menu::Columns, None), go("manchete?"))]),
    ),
    step(
        "manchete?",
        &[say(24, Max)],
        Options(&[(0, go("palavra")), (1, go("livre"))]),
    ),
    // [17] "Clique aqui." (ferramenta Palavra, 36,26), se não estiver nela
    step(
        "palavra",
        &[say(17, Anchor::Tool(Tool::Word))],
        On(&[(Pat::Tool(Tool::Word), go("forma"))]),
    )
    .skip_if(Cond::WordActive, "forma"),
    // [25] "Clique na ferramenta Forma…" (466,76)
    step(
        "forma",
        &[say(25, Anchor::Button(Button::Shape))],
        On(&[(Pat::Button(Button::Shape), go("manchete"))]),
    ),
    // cavalete com o nome do usuário: [26] digite, [27] OK
    step(
        "manchete",
        &[say(26, Easel(Spot::Text)), say(27, Easel(Spot::Ok))],
        On(&[(Pat::ShapedOk, go("livre"))]),
    ),
    // dicas [21d:5]→[5], [21b:f]→[15], [21b:17]→[23], [222:14]→[6]
    step("livre", &[say(0, Max)], On(&[(Pat::MaxDone, go("salvar"))])),
];

pub static NEWS: Script = Script {
    kind: Kind::News,
    block: 0x3a00,
    alerts: 0x3a02,
    blank_alert: 1,
    pages_alert: None,
    fifth_page: None,
    help: 6,
    start: "início",
    free: "livre",
    end: EndRecords {
        save: 7,
        trash: 8,
        print: 10,
        print_click: 11,
        print_mczee: 20,
        print_open: 21,
        done: 12,
    },
    steps: STEPS,
};

#[cfg(test)]
mod tests {
    use super::super::assistant::tests::{cmds, walk};
    use super::super::assistant::{Assistant, Command, Context, Event as E};
    use super::*;

    fn news() -> Assistant {
        let mut a = Assistant::new(Kind::News, Context::default());
        a.begin();
        a
    }

    #[test]
    fn from_scratch_with_columns_and_headline_to_print() {
        let mut a = news();
        assert_eq!(a.at(), "início");
        walk(&mut a, &[E::Option(1), E::Option(0)]);
        assert_eq!(a.at(), "colunas");
        assert_eq!(a.expects(), vec![Pat::Tool(Tool::Page)]);
        walk(
            &mut a,
            &[
                E::ToolClicked(Tool::Page),
                E::ButtonClicked(Button::Columns),
                E::MenuItem(Menu::Columns, 3),
            ],
        );
        assert_eq!(a.at(), "manchete?");
        walk(&mut a, &[E::Option(0), E::ToolClicked(Tool::Word)]);
        assert_eq!(a.at(), "forma");
        walk(&mut a, &[E::ButtonClicked(Button::Shape)]);
        assert_eq!(a.shown().len(), 2, "[26] e [27] juntos");
        walk(&mut a, &[E::ShapedOk]);
        assert_eq!(a.at(), "livre");
        // dica das ferramentas Palavra
        walk(&mut a, &[E::Tip(15), E::MaxClicked]);
        assert_eq!(a.at(), "salvar");
        let out = walk(
            &mut a,
            &[
                E::Option(0),
                E::Saved,
                E::Option(0),
                E::ToolClicked(Tool::Print),
                E::ButtonClicked(Button::PrintMcZee),
                E::Printed,
            ],
        );
        assert_eq!(cmds(&out), vec![Command::AskName]);
        assert_eq!(a.at(), "pronto");
        assert_eq!(a.shown()[0].record, 12);
        // "Trabalhar no jornal por conta própria" → [0]
        walk(&mut a, &[E::Option(0)]);
        assert_eq!(a.at(), "livre");
    }

    #[test]
    fn sample_path_opens_the_folder_and_skips_steps_already_done() {
        let mut a = news();
        let out = walk(&mut a, &[E::Option(0)]);
        assert_eq!(cmds(&out), vec![Command::OpenSamples]);
        walk(&mut a, &[E::SampleChosen]);
        assert_eq!(a.shown()[0].record, 4);
        walk(&mut a, &[E::MaxClicked]);
        assert_eq!(a.at(), "salvar");
        // com a Preparação da página aberta e a Palavra ativa, os
        // "Clique aqui" da ferramenta são pulados
        let mut b = Assistant::new(
            Kind::News,
            Context {
                word_active: true,
                open_strip: Some(Tool::Page),
                ..Context::default()
            },
        );
        b.begin();
        walk(&mut b, &[E::Option(1), E::Option(0)]);
        assert_eq!(b.at(), "cria-colunas");
        walk(
            &mut b,
            &[
                E::ButtonClicked(Button::Columns),
                E::MenuItem(Menu::Columns, 2),
                E::Option(0),
            ],
        );
        assert_eq!(b.at(), "forma");
        walk(&mut b, &[E::Back]);
        assert_eq!(b.at(), "manchete?");
    }

    #[test]
    fn blank_newspaper_and_leave_from_done() {
        let mut a = news();
        a.ctx.blank = true;
        walk(&mut a, &[E::Option(1), E::Option(1), E::Option(1)]);
        assert_eq!(a.at(), "livre");
        walk(&mut a, &[E::MaxClicked]);
        assert_eq!(a.alert().map(|x| (x.block, x.record)), Some((0x3a02, 1)));
        a.ctx.blank = false;
        walk(
            &mut a,
            &[E::Dismiss, E::MaxClicked, E::Option(1), E::Option(1)],
        );
        assert_eq!(a.at(), "pronto");
        assert_eq!(cmds(&walk(&mut a, &[E::Option(2)])), vec![Command::Leave]);
    }

    #[test]
    fn script_is_closed_and_matches_the_writer_what() {
        super::super::assistant::tests::check_script(&NEWS);
    }
}
