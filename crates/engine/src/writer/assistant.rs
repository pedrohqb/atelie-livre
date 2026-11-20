//! Motor dos assistentes do Max na Oficina de Projetos do Creative Writer
//! (Jornal `WHAT 3a00`, Cartão `3c00`, Faixa `3800`), dirigido por dados.
//!
//! Um roteiro ([`Script`]) é uma lista de passos ([`Step`]): cada passo
//! mostra um ou mais balões do `WHAT` ([`Say`]: registro e [`Anchor`], para
//! onde a cauda aponta), espera alguma coisa ([`Wait`]: Segue, uma opção do
//! balão, rádios, o número de páginas, ou ações reais na tela — [`Pat`]) e
//! vai a outro passo ([`Go`]), pedindo à tela o que ela precisa fazer
//! ([`Command`]: abrir a pasta de amostras, trocar a visão, pedir o nome,
//! jogar fora, voltar ao andar…). Os roteiros são só dados, em
//! `assistant_news`, `assistant_card` e `assistant_banner`; o fim comum
//! (salvar → nome → imprimir → "Tudo pronto!") é [`END_FLOW`], com os
//! registros de cada projeto em [`EndRecords`].
//!
//! O esqueleto (passo atual, histórico do **Volta**, `3803[0]` e "trabalhar
//! por conta própria") é o mesmo da lição da Maggie no Fine Artist:
//! [`crate::guide::Guide`]. O que é só do Max fica aqui:
//!
//! - **Pára** abre `3803[0]` (no Fine Artist abre `3803[4]`); clicar no Max
//!   num passo que não o espera também (como a Maggie fora de hora);
//! - as **dicas** `[21b:N]`/`[21d:N]`/`[21e:N]` abrem o registro N ao lado
//!   do balão ([`Event::Tip`], som `0307`) e "Precisa de ajuda?" `[222]`
//!   troca o balão pelo da Bolsa Mágica ([`Event::NeedHelp`]);
//! - os avisos do **Rex**: documento vazio ao clicar no Max no fim
//!   (`3a02[1]`/`3c02[1]`/`3802[1]`), número de páginas fora de 1…8
//!   (`3802[0]`, o `@` vira o máximo) e quinta página do cartão
//!   (`3c00[106]`);
//! - passos que se pulam conforme a tela ([`Cond`]: a Palavra já ativa, a
//!   faixa de uma ferramenta já aberta);
//! - a figura da dobra do cartão ([`super::fold_figure`]), que anda com os
//!   passos e desdobra no Volta.
//!
//! Sem tela: a cena do editor em modo projeto traduz os cliques em
//! [`Event`]s (ou usa [`Assistant::click`]), mantém o [`Context`] em dia e
//! executa os [`Out`]s. Ver `docs/telas/creative-writer-projetos.md`.

use super::fold_figure::{FoldFigure, FoldStage};
use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::balloon::draw_tail_box_in;
use crate::guide::{Answer, Ask, Guide, QUIT_ANSWERS, QUIT_BLOCK};
use crate::rich::Hot;
use crate::text_balloon::{Button as BalloonButton, TextBalloon};
use crate::ui::{Rect, inside};
use formats::what::{Item, PIC_OPTION, PIC_OPTION_PICTURE, Record};

/// `3803[0]`: "O que você quer que eu faça agora?" (Pára e Max fora de hora).
pub const QUIT_RECORD: usize = 0;
/// Balão genérico do nome ao salvar (`0faa[14]` "Digite um nome para o seu
/// documento…"): o original o mostra no lugar dos `[9]`/`[21]`/`[38]` de
/// cada bloco.
pub const NAME_BLOCK: u16 = 0x0faa;
pub const NAME_RECORD: usize = 14;
/// Máximo de páginas de uma faixa (`3802[0]`: "Digite um número entre 1 e @").
pub const BANNER_MAX_PAGES: u8 = 8;

/// Max no lugar da porta Saída durante o projeto (`7e13`, 55×87) e com a
/// placa "Clique aqui!" (`7e9f`), em (581, 16).
pub const MAX_DOOR: u16 = 0x7e13;
pub const MAX_SIGN: u16 = 0x7e9f;
pub const MAX_RECT: Rect = (581, 16, 55, 87);
/// Ponta da cauda dos balões que apontam para o Max.
pub const MAX_TAIL: (i32, i32) = (598, 103);

/// Sons do `WRITER.SND` (`projects.md` §7).
pub mod snd {
    /// Clique numa opção ou num rádio do balão.
    pub const OPTION: u16 = 0x02ff;
    pub const NEXT: u16 = 0x02eb;
    pub const STOP: u16 = 0x02ec;
    /// Rex (avisos e o `3803[0]` do Pára).
    pub const REX: u16 = 0x02fe;
    /// Dica `[21b]`/`[21d]`/`[21e]`.
    pub const TIP: u16 = 0x0307;
    /// "Precisa de ajuda?".
    pub const HELP: u16 = 0x02d3;
    /// Clique no Max (`0226`/`0283`/`0312` têm o mesmo tamanho).
    pub const MAX: u16 = 0x0283;
    /// A pasta de amostras abre.
    pub const FOLDER: u16 = 0x02f1;
}

/// Qual projeto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    News,
    Card,
    Banner,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::News, Kind::Card, Kind::Banner];

    /// Valor do chunk `WIZA` do `.MAX` (e dos 4 primeiros bytes do `ADD `).
    pub fn wiza(self) -> [u8; 4] {
        match self {
            Kind::News => *b"news",
            Kind::Card => *b"card",
            Kind::Banner => *b"banr",
        }
    }

    pub fn from_wiza(w: [u8; 4]) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.wiza() == w)
    }

    /// Texto do meio da barra inferior.
    pub fn doing(self) -> &'static str {
        match self {
            Kind::News => "Estamos fazendo um Jornal",
            Kind::Card => "Estamos fazendo um Cartão",
            Kind::Banner => "Estamos fazendo uma Faixa",
        }
    }

    /// Pasta da criança onde a pasta de trabalhos filtra o projeto.
    pub fn folder(self) -> &'static str {
        match self {
            Kind::News => "Jornais",
            Kind::Card => "Cartões",
            Kind::Banner => "Faixas",
        }
    }

    pub fn script(self) -> &'static Script {
        match self {
            Kind::News => &super::assistant_news::NEWS,
            Kind::Card => &super::assistant_card::CARD,
            Kind::Banner => &super::assistant_banner::BANNER,
        }
    }
}

/// Ferramentas da barra do Escritório, com os nomes de
/// `scenes::writer::tools::ToolId` (ramo do Escritório integrado).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    Word,
    Stickers,
    Sounds,
    Page,
    Speller,
    Egg,
    Removers,
    Transformers,
    Viewer,
    Print,
}

/// Botões das faixas de opções (e da pasta/cavalete) que os roteiros
/// mandam clicar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    /// Forma (palavras com forma), na faixa da Palavra.
    Shape,
    /// Cria-colunas, na Preparação da página.
    Columns,
    /// Alto/Largo, na Preparação da página.
    Orientation,
    /// Número de página / etiquetas, na Preparação da página.
    PageNumber,
    /// Alavanca de mais escolhas (2ª página da faixa).
    Lever,
    /// Um dos rolos BORDA (2ª página da Preparação da página).
    BorderRoll,
    /// Visão normal (uma página de cada vez), nos Visores.
    ViewNormal,
    /// Um rolo de adesivos.
    StickerRoll,
    /// Lápis de cera ("desenhar a sua própria figura", `[80a]`).
    Crayon,
    /// Imprimir com o McZee (máquina de impressão), na faixa do Imprimir.
    PrintMcZee,
}

/// Menus que se abrem de um botão de faixa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Menu {
    /// 1 a 4 colunas (item = número de colunas).
    Columns,
    /// [`TALL`] / [`WIDE`].
    Orientation,
    /// Etiquetas: nada / "Pág. 1" ([`LABEL_PAGE`]) / "1".
    PageNumber,
}

pub const TALL: u8 = 0;
pub const WIDE: u8 = 1;
pub const LABEL_NONE: u8 = 0;
pub const LABEL_PAGE: u8 = 1;
pub const LABEL_NUMBER: u8 = 2;

/// Lugar de um diálogo (pasta de amostras, cavalete de palavras com forma).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Spot {
    /// Seta "mais amostras" (Título) da pasta.
    More,
    /// "Já" da pasta / OK do cavalete.
    Ok,
    /// Campo de texto do cavalete.
    Text,
}

/// Para onde a cauda do balão aponta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
    /// O Max na porta (cauda em [`MAX_TAIL`]).
    Max,
    Tool(Tool),
    Button(Button),
    Menu(Menu, u8),
    /// A página (qualquer lugar do cartão).
    Page,
    Folder(Spot),
    Easel(Spot),
    /// Bolsa Mágica ("Precisa de ajuda?").
    Bag,
    /// Título na barra inferior (nome ao salvar).
    NameBar,
    /// Sem cauda, no meio da tela (avisos).
    Center,
}

impl Anchor {
    /// Ponto da tela (640×480) para onde a cauda aponta, quando foi medido
    /// no original (`projects*.md`); os outros a tela resolve.
    pub fn point(self) -> Option<(i32, i32)> {
        Some(match self {
            Anchor::Max => MAX_TAIL,
            Anchor::Tool(Tool::Word) => (36, 26),
            Anchor::Tool(Tool::Stickers) => (80, 28),
            Anchor::Tool(Tool::Page) => (180, 28),
            Anchor::Tool(Tool::Transformers) => (388, 26),
            Anchor::Tool(Tool::Viewer) => (440, 26),
            Anchor::Tool(Tool::Print) => (492, 26),
            Anchor::Button(Button::Shape) => (466, 76),
            Anchor::Button(Button::Orientation) => (290, 76),
            Anchor::Button(Button::PageNumber) => (450, 76),
            Anchor::Button(Button::Lever) => (516, 76),
            Anchor::Button(Button::ViewNormal) => (88, 76),
            Anchor::Button(Button::PrintMcZee) => (370, 76),
            // primeiro rolo BORDA (104, 57), 108×42
            Anchor::Button(Button::BorderRoll) => (158, 78),
            Anchor::Bag => (555, 95),
            // seta "mais amostras" do Título e o Já da pasta
            // (`projects_07_jornal_amostra`, `projects_95_seta`)
            Anchor::Folder(Spot::More) => (496, 287),
            Anchor::Folder(Spot::Ok) => (263, 356),
            Anchor::NameBar => (320, 461),
            _ => return None,
        })
    }
}

/// Onde pôr um balão de `w`×`h` que aponta para `anchor`: canto superior
/// esquerdo e ponta da cauda. Medido nas capturas: o do Max fica à
/// esquerda e abaixo dele (borda direita em 620, topo em 115;
/// `projects_35`, `45_dobra_1`); os que apontam a barra ficam logo abaixo
/// do ponto, estendendo-se para a direita na metade esquerda da tela
/// (`28_colunas_2`) e para a esquerda na direita (`51_visor_1`,
/// `52_visor_2`); os de baixo da tela, acima dele.
pub fn place(anchor: Anchor, w: i32, h: i32) -> ((i32, i32), Option<(i32, i32)>) {
    let clamp_x = |x: i32| x.clamp(4, (636 - w).max(4));
    match anchor.point() {
        Some(tail) if anchor == Anchor::Max => ((clamp_x(620 - w), 115), Some(tail)),
        // na pasta: à direita da seta (522,277) e embaixo do Já (240,380)
        Some((px, py)) if anchor == Anchor::Folder(Spot::More) => {
            ((clamp_x(px + 26), py - 10), Some((px, py)))
        }
        Some((px, py)) if anchor == Anchor::Folder(Spot::Ok) => {
            ((clamp_x(px - 23), py + 24), Some((px, py)))
        }
        Some((px, py)) if py > 240 => ((clamp_x(px - w / 2), py - 16 - h), Some((px, py))),
        Some((px, py)) if px < 320 => ((clamp_x(px - 25), py + 44), Some((px, py + 14))),
        Some((px, py)) => ((clamp_x(px + 22 - w), py + 44), Some((px, py + 14))),
        None => ((clamp_x(320 - w / 2), (240 - h / 2).max(110)), None),
    }
}

/// O `3803[0]` abre ao lado do balão do passo, que continua à vista:
/// 31 px à esquerda dele e 22 px abaixo do topo (`projects_36_para`).
pub const QUIT_BESIDE: (i32, i32) = (31, 22);

/// Registro de um balão: do bloco do roteiro, de outro bloco ou do fim comum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rec {
    /// Registro do bloco do roteiro.
    N(usize),
    /// Registro de outro bloco (`0faa[14]`).
    In(u16, usize),
    /// Registro do fim comum, resolvido por [`EndRecords`].
    End(End),
}

/// Balões do fim comum, um registro por projeto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Save,
    Trash,
    Print,
    PrintClick,
    PrintMcZee,
    /// Variante quando a faixa do Imprimir já está aberta.
    PrintOpen,
    Done,
}

/// Registros do fim comum de um projeto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndRecords {
    /// "Você quer salvar…?" (É, salvar / Não salvar, não).
    pub save: usize,
    /// "…imprimir antes que vá para o lixo?" (imprimir / jogar fora).
    pub trash: usize,
    /// "Você quer imprimir…?" (É / Não).
    pub print: usize,
    /// "Para imprimir, clique aqui…" (ferramenta Imprimir).
    pub print_click: usize,
    /// "…então, clique aqui. O meu amigo McZee…".
    pub print_mczee: usize,
    /// "Para imprimir, clique aqui. O meu amigo McZee…" (faixa já aberta).
    pub print_open: usize,
    /// "Tudo pronto!…" (por conta própria / novo / voltar ao andar).
    pub done: usize,
}

impl EndRecords {
    pub fn get(&self, e: End) -> usize {
        match e {
            End::Save => self.save,
            End::Trash => self.trash,
            End::Print => self.print,
            End::PrintClick => self.print_click,
            End::PrintMcZee => self.print_mczee,
            End::PrintOpen => self.print_open,
            End::Done => self.done,
        }
    }
}

/// Um balão de um passo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Say {
    pub rec: Rec,
    pub anchor: Anchor,
}

pub const fn say(record: usize, anchor: Anchor) -> Say {
    Say {
        rec: Rec::N(record),
        anchor,
    }
}

/// Ação real na tela que um passo espera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pat {
    Tool(Tool),
    Button(Button),
    /// Item do menu (`None`: qualquer um).
    Menu(Menu, Option<u8>),
    Page,
    ShapedOk,
    /// Clique no Max que só avança.
    Max,
    /// Clique no Max que encerra o projeto: com o documento vazio, o Rex
    /// avisa (`x02[1]`) e o passo fica.
    MaxDone,
    SampleChosen,
    StickerPicked,
    StickerPlaced,
    BorderPicked,
    Saved,
    Printed,
}

impl Pat {
    pub fn matches(self, ev: &Event) -> bool {
        match (self, ev) {
            (Pat::Tool(t), Event::ToolClicked(u)) => t == *u,
            (Pat::Button(b), Event::ButtonClicked(c)) => b == *c,
            (Pat::Menu(m, item), Event::MenuItem(n, k)) => m == *n && item.is_none_or(|i| i == *k),
            (Pat::Page, Event::PageClicked) => true,
            (Pat::ShapedOk, Event::ShapedOk) => true,
            (Pat::Max | Pat::MaxDone, Event::MaxClicked) => true,
            (Pat::SampleChosen, Event::SampleChosen) => true,
            (Pat::StickerPicked, Event::StickerPicked) => true,
            (Pat::StickerPlaced, Event::StickerPlaced) => true,
            (Pat::BorderPicked, Event::BorderPicked) => true,
            (Pat::Saved, Event::Saved) => true,
            (Pat::Printed, Event::Printed) => true,
            _ => false,
        }
    }
}

/// Visões de página.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Normal,
    TwoPages,
    AllPages,
    SideBySide,
}

/// O que o assistente pede à tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Abrir a pasta de trabalhos em `MAX › <Kind::folder>` (amostras do Max).
    OpenSamples,
    SetView(View),
    /// Faixa com `n` páginas largas.
    SetPages(u8),
    /// Mostrar `0faa[14]` e deixar editar o título na barra inferior; Enter
    /// grava e manda [`Event::Saved`].
    AskName,
    /// O documento vai para o lixo: não gravar ao sair.
    Discard,
    /// Começar um projeto novo do mesmo tipo (documento em branco).
    NewDocument,
    /// Voltar ao Andar dos Projetos (sons `02d7` + `02c0`).
    Leave,
}

/// Para onde ir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Go {
    /// Ao passo com esse id (ou [`FREE`]/[`START`]), depois de pedir os
    /// comandos à tela.
    Step(&'static str, &'static [Command]),
    /// Sai para o andar ([`Command::Leave`]).
    Leave,
}

pub const fn go(to: &'static str) -> Go {
    Go::Step(to, &[])
}

pub const fn go_do(to: &'static str, cmds: &'static [Command]) -> Go {
    Go::Step(to, cmds)
}

/// Passo de trabalho livre do roteiro ([`Script::free`]).
pub const FREE: &str = "@livre";
/// Primeiro passo do roteiro ([`Script::start`]).
pub const START: &str = "@início";

/// O que o passo espera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// Segue.
    Next(Go),
    /// Uma opção do balão (pelo índice do original, `[22c:N]`).
    Options(&'static [(u16, Go)]),
    /// Rádios `[22e:N]` (com o marcado de início) e Segue.
    Radios {
        default: u16,
        next: &'static [(u16, Go)],
    },
    /// Campo numérico `[0:19]` de 1 a `max` e Segue (páginas da faixa):
    /// pede [`Command::SetPages`]; fora disso, o Rex (`alerts[pages_alert]`).
    Pages { max: u8, next: Go },
    /// Ações na tela; vale a primeira que casar.
    On(&'static [(Pat, Go)]),
}

/// Condição da tela para pular um passo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cond {
    /// A ferramenta Palavra já está escolhida.
    WordActive,
    /// A faixa de opções da ferramenta já está aberta.
    StripOpen(Tool),
}

/// Um passo do roteiro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// Nome do passo (para os [`Go`] e o registro de atividade).
    pub id: &'static str,
    /// Balões: o primeiro tem as opções e os botões; os outros são os
    /// "Clique aqui." que o acompanham. Vazio: passo invisível (a máquina
    /// de impressão por cima).
    pub say: &'static [Say],
    pub wait: Wait,
    /// Comandos ao entrar no passo (também ao voltar a ele).
    pub on_enter: &'static [Command],
    /// Se a condição vale ao entrar, vai direto a este passo.
    pub skip: Option<(Cond, &'static str)>,
    /// Figura da dobra do cartão neste passo.
    pub fold: Option<FoldStage>,
    /// Registro do "Precisa de ajuda?" neste passo (senão o do roteiro).
    pub help: Option<usize>,
}

pub const fn step(id: &'static str, say: &'static [Say], wait: Wait) -> Step {
    Step {
        id,
        say,
        wait,
        on_enter: &[],
        skip: None,
        fold: None,
        help: None,
    }
}

impl Step {
    pub const fn entering(mut self, cmds: &'static [Command]) -> Step {
        self.on_enter = cmds;
        self
    }

    pub const fn skip_if(mut self, cond: Cond, to: &'static str) -> Step {
        self.skip = Some((cond, to));
        self
    }

    pub const fn folded(mut self, stage: FoldStage) -> Step {
        self.fold = Some(stage);
        self
    }

    pub const fn help(mut self, record: usize) -> Step {
        self.help = Some(record);
        self
    }

    /// O passo mostra Segue.
    pub fn has_next(&self) -> bool {
        matches!(
            self.wait,
            Wait::Next(_) | Wait::Radios { .. } | Wait::Pages { .. }
        )
    }
}

/// Fim comum dos três projetos (`projects.md` §4): clique no Max →
/// salvar? → nome (`0faa[14]`) → imprimir? → ferramenta Imprimir → botão
/// do McZee → máquina de impressão → "Tudo pronto!"; sem salvar →
/// imprimir antes do lixo? → "Tudo pronto!".
pub const END_FLOW: &[Step] = &[
    step(
        "salvar",
        &[Say {
            rec: Rec::End(End::Save),
            anchor: Anchor::Max,
        }],
        Wait::Options(&[(0, go("nome")), (1, go("lixo"))]),
    ),
    step(
        "nome",
        &[Say {
            rec: Rec::In(NAME_BLOCK, NAME_RECORD),
            anchor: Anchor::NameBar,
        }],
        Wait::On(&[(Pat::Saved, go("imprimir?"))]),
    )
    .entering(&[Command::AskName]),
    step(
        "imprimir?",
        &[Say {
            rec: Rec::End(End::Print),
            anchor: Anchor::Max,
        }],
        Wait::Options(&[(0, go("imprimir")), (1, go("pronto"))]),
    ),
    step(
        "imprimir",
        &[Say {
            rec: Rec::End(End::PrintClick),
            anchor: Anchor::Tool(Tool::Print),
        }],
        Wait::On(&[(Pat::Tool(Tool::Print), go("imprimir-mczee"))]),
    )
    .skip_if(Cond::StripOpen(Tool::Print), "imprimir-aberto"),
    step(
        "imprimir-mczee",
        &[Say {
            rec: Rec::End(End::PrintMcZee),
            anchor: Anchor::Button(Button::PrintMcZee),
        }],
        Wait::On(&[(Pat::Button(Button::PrintMcZee), go("imprimindo"))]),
    ),
    step(
        "imprimir-aberto",
        &[Say {
            rec: Rec::End(End::PrintOpen),
            anchor: Anchor::Button(Button::PrintMcZee),
        }],
        Wait::On(&[(Pat::Button(Button::PrintMcZee), go("imprimindo"))]),
    ),
    // a máquina de impressão por cima; quando ela fecha, "Tudo pronto!"
    step("imprimindo", &[], Wait::On(&[(Pat::Printed, go("pronto"))])),
    step(
        "lixo",
        &[Say {
            rec: Rec::End(End::Trash),
            anchor: Anchor::Max,
        }],
        Wait::Options(&[
            (0, go_do("imprimir", &[Command::Discard])),
            (1, go_do("pronto", &[Command::Discard])),
        ]),
    ),
    step(
        "pronto",
        &[Say {
            rec: Rec::End(End::Done),
            anchor: Anchor::Max,
        }],
        Wait::Options(&[
            (0, go(FREE)),
            (1, go_do(START, &[Command::NewDocument])),
            (2, Go::Leave),
        ]),
    ),
];

/// Um roteiro de projeto.
#[derive(Debug)]
pub struct Script {
    pub kind: Kind,
    /// Bloco `WHAT` dos balões (`3a00`, `3c00`, `3800`).
    pub block: u16,
    /// Bloco dos avisos do Rex (`3a02`, `3c02`, `3802`).
    pub alerts: u16,
    /// Aviso de documento vazio em `alerts`.
    pub blank_alert: usize,
    /// Aviso de número de páginas em `alerts` (faixa).
    pub pages_alert: Option<usize>,
    /// Aviso de quinta página no `block` (cartão, `[106]`).
    pub fifth_page: Option<usize>,
    /// "Precisa de ajuda?" (`[222:14]`): registro com a Bolsa Mágica.
    pub help: usize,
    pub start: &'static str,
    /// Passo de "Trabalhar … por conta própria" do "Tudo pronto!".
    pub free: &'static str,
    pub end: EndRecords,
    pub steps: &'static [Step],
}

impl Script {
    /// Número de passos, contando o fim comum.
    pub fn len(&self) -> usize {
        self.steps.len() + END_FLOW.len()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    /// Passo `i` (os do roteiro, depois os de [`END_FLOW`]).
    pub fn step(&self, i: usize) -> &'static Step {
        if i < self.steps.len() {
            &self.steps[i]
        } else {
            &END_FLOW[i - self.steps.len()]
        }
    }

    /// Índice do passo com esse id ([`FREE`] e [`START`] incluídos).
    pub fn find(&self, id: &str) -> Option<usize> {
        let id = match id {
            FREE => self.free,
            START => self.start,
            other => other,
        };
        (0..self.len()).find(|&i| self.step(i).id == id)
    }

    /// Bloco e registro de um balão.
    pub fn record(&self, r: Rec) -> (u16, usize) {
        match r {
            Rec::N(k) => (self.block, k),
            Rec::In(b, k) => (b, k),
            Rec::End(e) => (self.block, self.end.get(e)),
        }
    }

    /// Ids de passo referidos e não definidos (para os testes dos roteiros).
    pub fn dangling(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        for i in 0..self.len() {
            let s = self.step(i);
            let mut gos: Vec<Go> = match s.wait {
                Wait::Next(g) | Wait::Pages { next: g, .. } => vec![g],
                Wait::Options(t) | Wait::Radios { next: t, .. } => t.iter().map(|o| o.1).collect(),
                Wait::On(t) => t.iter().map(|o| o.1).collect(),
            };
            if let Some((_, to)) = s.skip {
                gos.push(go(to));
            }
            for g in gos {
                if let Go::Step(id, _) = g
                    && self.find(id).is_none()
                {
                    out.push(id);
                }
            }
        }
        out
    }
}

/// O que a tela sabe e o assistente precisa saber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Context {
    /// O documento está vazio (sem palavras).
    pub blank: bool,
    /// A ferramenta Palavra está escolhida.
    pub word_active: bool,
    /// Ferramenta cuja faixa de opções está aberta.
    pub open_strip: Option<Tool>,
    /// Número de páginas do documento (valor inicial do campo da faixa).
    pub pages: u8,
}

impl Context {
    fn holds(&self, c: Cond) -> bool {
        match c {
            Cond::WordActive => self.word_active,
            Cond::StripOpen(t) => self.open_strip == Some(t),
        }
    }
}

/// O que a tela manda ao assistente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    ToolClicked(Tool),
    ButtonClicked(Button),
    MenuItem(Menu, u8),
    PageClicked,
    /// OK do cavalete de palavras com forma (a palavra foi posta).
    ShapedOk,
    MaxClicked,
    Next,
    Back,
    Stop,
    /// Opção do balão, pelo índice do original (`[22c:N]`).
    Option(u16),
    /// Rádio do balão, pelo índice do original (`[22e:N]`).
    Radio(u16),
    /// Tecla no campo numérico.
    Typed(char),
    Backspace,
    /// Dica: registro N do bloco (`[21b:N]`, `[21d:N]`, `[21e:N]`).
    Tip(usize),
    /// "Precisa de ajuda?" (`[222]`).
    NeedHelp,
    /// Clique fora dos balões (fecha dica, aviso ou `3803[0]`).
    Dismiss,
    /// "Já" na pasta de amostras: a amostra foi aberta.
    SampleChosen,
    StickerPicked,
    StickerPlaced,
    BorderPicked,
    /// O documento foi gravado com o nome digitado.
    Saved,
    /// A máquina de impressão terminou (ou foi fechada).
    Printed,
    /// O cartão ganhou uma quinta página.
    FifthPage,
}

/// O que o assistente devolve a cada evento, na ordem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Out {
    /// Entrou no passo (índice no [`Script`]); a tela redesenha os balões.
    Entered(usize),
    Do(Command),
    /// Som do `WRITER.SND`.
    Sound(u16),
}

/// Dica aberta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tip {
    pub record: usize,
    /// "Precisa de ajuda?": troca o balão do passo (aponta a Bolsa).
    pub replaces: bool,
}

/// Aviso do Rex aberto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alert {
    pub block: u16,
    pub record: usize,
    /// O `@` do texto vira este número (máximo de páginas).
    pub max: Option<u8>,
}

/// Um balão à vista.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// O balão do passo (opções, campo e botões).
    Step,
    /// "Clique aqui." que acompanha o do passo.
    Companion,
    /// `3803[0]`.
    Quit,
    Tip,
    Alert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shown {
    pub block: u16,
    pub record: usize,
    pub anchor: Anchor,
    pub role: Role,
}

/// Botões do balão do passo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Buttons {
    pub back: bool,
    pub stop: bool,
    pub next: bool,
}

/// Balão montado e posto na tela.
pub struct Placed {
    pub shown: Shown,
    pub balloon: TextBalloon,
    pub tail: Option<(i32, i32)>,
    /// Índices do original das opções, na ordem da tela, e se cada uma é
    /// rádio (`22e`).
    pub options: Vec<(u16, bool)>,
}

/// Assistente de um projeto em andamento.
#[derive(Debug, Clone)]
pub struct Assistant {
    script: &'static Script,
    guide: Guide,
    /// Mantido em dia pela tela.
    pub ctx: Context,
    tip: Option<Tip>,
    alert: Option<Alert>,
    radio: Option<u16>,
    field: String,
    /// O campo ainda tem o valor inicial (selecionado: digitar substitui).
    field_fresh: bool,
    fold: Option<FoldFigure>,
}

impl Assistant {
    pub fn new(kind: Kind, ctx: Context) -> Assistant {
        Assistant::with_script(kind.script(), ctx)
    }

    /// Assistente de um roteiro qualquer (testes). Chame [`Self::begin`].
    pub fn with_script(script: &'static Script, ctx: Context) -> Assistant {
        Assistant {
            script,
            guide: Guide::new(script.find(START).unwrap_or(0)),
            ctx,
            tip: None,
            alert: None,
            radio: None,
            field: String::new(),
            field_fresh: false,
            fold: None,
        }
    }

    /// Entra no primeiro passo.
    pub fn begin(&mut self) -> Vec<Out> {
        let mut out = Vec::new();
        self.enter(&mut out);
        out
    }

    pub fn script(&self) -> &'static Script {
        self.script
    }

    pub fn kind(&self) -> Kind {
        self.script.kind
    }

    pub fn current(&self) -> usize {
        self.guide.current()
    }

    pub fn step(&self) -> &'static Step {
        self.script.step(self.guide.current())
    }

    /// Id do passo atual.
    pub fn at(&self) -> &'static str {
        self.step().id
    }

    pub fn paused(&self) -> bool {
        self.guide.paused()
    }

    /// `3803[0]` aberto.
    pub fn quitting(&self) -> bool {
        self.guide.asking().is_some()
    }

    pub fn tip(&self) -> Option<Tip> {
        self.tip
    }

    pub fn alert(&self) -> Option<Alert> {
        self.alert
    }

    /// Rádio marcado no passo (índice do original).
    pub fn radio(&self) -> Option<u16> {
        self.radio
    }

    /// Texto do campo numérico, se o passo tem um.
    pub fn field(&self) -> Option<&str> {
        matches!(self.step().wait, Wait::Pages { .. }).then_some(self.field.as_str())
    }

    pub fn fold(&self) -> Option<&FoldFigure> {
        self.fold.as_ref()
    }

    /// Figura do Max na porta: com a placa "Clique aqui!" quando o passo
    /// espera um clique nele.
    pub fn max_picture(&self) -> u16 {
        let waits_max = matches!(self.step().wait, Wait::On(t)
            if t.iter().any(|(p, _)| matches!(p, Pat::Max | Pat::MaxDone)));
        if self.guide.active() && waits_max {
            MAX_SIGN
        } else {
            MAX_DOOR
        }
    }

    /// Ações da tela que o passo espera (para destacar o alvo).
    pub fn expects(&self) -> Vec<Pat> {
        match self.step().wait {
            Wait::On(t) if self.guide.active() && self.alert.is_none() => {
                t.iter().map(|o| o.0).collect()
            }
            _ => Vec::new(),
        }
    }

    /// Avança o relógio (animação da dobra).
    pub fn tick(&mut self, dt: f32) {
        if let Some(f) = self.fold.as_mut() {
            f.tick(dt);
        }
    }

    /// Botões do balão do passo. Só os balões que apontam para o Max têm
    /// botões; os "Clique aqui." que apontam a tela não (`projects_28`,
    /// `51`). Com o `3803[0]` aberto eles continuam à vista, sem efeito.
    pub fn buttons(&self) -> Buttons {
        let st = self.step();
        if self.guide.paused() || st.say.first().is_none_or(|s| s.anchor != Anchor::Max) {
            return Buttons::default();
        }
        Buttons {
            back: self.guide.has_back(),
            stop: true,
            next: st.has_next(),
        }
    }

    /// Balões à vista, na ordem de desenho (o aviso por último).
    pub fn shown(&self) -> Vec<Shown> {
        let mut v = Vec::new();
        if self.guide.paused() {
            return v;
        }
        {
            let st = self.step();
            for (k, s) in st.say.iter().enumerate() {
                let (block, record) = self.script.record(s.rec);
                let role = if k == 0 { Role::Step } else { Role::Companion };
                if k == 0
                    && let Some(Tip {
                        record,
                        replaces: true,
                    }) = self.tip
                {
                    v.push(Shown {
                        block: self.script.block,
                        record,
                        anchor: Anchor::Bag,
                        role: Role::Tip,
                    });
                    continue;
                }
                v.push(Shown {
                    block,
                    record,
                    anchor: s.anchor,
                    role,
                });
            }
            if let Some(Tip {
                record,
                replaces: false,
            }) = self.tip
            {
                v.push(Shown {
                    block: self.script.block,
                    record,
                    anchor: Anchor::Center,
                    role: Role::Tip,
                });
            }
        }
        if self.guide.asking().is_some() {
            v.push(Shown {
                block: QUIT_BLOCK,
                record: QUIT_RECORD,
                anchor: Anchor::Max,
                role: Role::Quit,
            });
        }
        if let Some(a) = self.alert {
            v.push(Shown {
                block: a.block,
                record: a.record,
                anchor: Anchor::Center,
                role: Role::Alert,
            });
        }
        v
    }

    fn enter(&mut self, out: &mut Vec<Out>) {
        for _ in 0..8 {
            let Some((cond, to)) = self.step().skip else {
                break;
            };
            match self.script.find(to) {
                Some(i) if self.ctx.holds(cond) => self.guide.replace(i),
                _ => break,
            }
        }
        let st = self.step();
        self.tip = None;
        self.radio = match st.wait {
            Wait::Radios { default, .. } => Some(default),
            _ => None,
        };
        self.field = match st.wait {
            Wait::Pages { max, .. } => self.ctx.pages.clamp(1, max.max(1)).to_string(),
            _ => String::new(),
        };
        self.field_fresh = true;
        for c in st.on_enter {
            out.push(Out::Do(*c));
            if *c == Command::OpenSamples {
                out.push(Out::Sound(snd::FOLDER));
            }
        }
        match (st.fold, self.fold.as_mut()) {
            (Some(stage), Some(f)) => f.go_to(stage),
            (Some(stage), None) => self.fold = Some(FoldFigure::new(stage)),
            (None, _) => self.fold = None,
        }
        out.push(Out::Entered(self.guide.current()));
    }

    fn go(&mut self, g: Go, out: &mut Vec<Out>) {
        match g {
            Go::Leave => out.push(Out::Do(Command::Leave)),
            Go::Step(id, cmds) => {
                let Some(i) = self.script.find(id) else {
                    return;
                };
                out.extend(cmds.iter().map(|c| Out::Do(*c)));
                if cmds.contains(&Command::NewDocument) {
                    self.fold = None;
                    self.guide.restart(i);
                } else {
                    self.guide.forward(i);
                }
                self.enter(out);
            }
        }
    }

    fn rex(&mut self, block: u16, record: usize, max: Option<u8>, out: &mut Vec<Out>) {
        self.alert = Some(Alert { block, record, max });
        out.push(Out::Sound(snd::REX));
    }

    /// Trata um evento da tela.
    pub fn on_event(&mut self, ev: Event) -> Vec<Out> {
        let mut out = Vec::new();
        // aviso do Rex: qualquer resposta o fecha
        if self.alert.is_some() {
            if matches!(
                ev,
                Event::Option(_)
                    | Event::Dismiss
                    | Event::Next
                    | Event::Back
                    | Event::Stop
                    | Event::MaxClicked
            ) {
                self.alert = None;
            }
            return out;
        }
        if ev == Event::FifthPage {
            if let Some(r) = self.script.fifth_page {
                self.rex(self.script.block, r, None, &mut out);
            }
            return out;
        }
        if self.guide.paused() {
            if ev == Event::MaxClicked && self.guide.resume() {
                out.push(Out::Sound(snd::MAX));
                out.push(Out::Entered(self.guide.current()));
            }
            return out;
        }
        if self.guide.asking().is_some() {
            let option = match ev {
                Event::Option(k) => Some(k as usize),
                Event::MaxClicked | Event::Stop | Event::Dismiss => None,
                _ => return out,
            };
            if option.is_some() {
                out.push(Out::Sound(snd::OPTION));
            }
            if self.guide.answer(option, QUIT_ANSWERS) == Answer::Leave {
                out.push(Out::Do(Command::Leave));
            }
            return out;
        }
        // dicas: abrir, fechar; qualquer outro evento fecha a dica e segue
        match ev {
            Event::Tip(0) => return out,
            Event::Tip(k) => {
                self.tip = Some(Tip {
                    record: k,
                    replaces: false,
                });
                out.push(Out::Sound(snd::TIP));
                return out;
            }
            Event::NeedHelp => {
                self.tip = Some(Tip {
                    record: self.step().help.unwrap_or(self.script.help),
                    replaces: true,
                });
                out.push(Out::Sound(snd::HELP));
                return out;
            }
            Event::Dismiss => {
                self.tip = None;
                return out;
            }
            _ => self.tip = None,
        }
        match ev {
            Event::Stop => {
                self.guide.ask(Ask::Stop);
                out.push(Out::Sound(snd::STOP));
                out.push(Out::Sound(snd::REX));
                return out;
            }
            Event::Back => {
                if self.guide.back().is_some() {
                    self.enter(&mut out);
                }
                return out;
            }
            _ => {}
        }
        let st = self.step();
        match (st.wait, ev) {
            (Wait::Next(g), Event::Next) => {
                out.push(Out::Sound(snd::NEXT));
                self.go(g, &mut out);
            }
            (Wait::Options(t), Event::Option(id)) => {
                if let Some(&(_, g)) = t.iter().find(|o| o.0 == id) {
                    out.push(Out::Sound(snd::OPTION));
                    self.go(g, &mut out);
                }
            }
            (Wait::Radios { next, .. }, Event::Radio(id)) => {
                if next.iter().any(|o| o.0 == id) {
                    self.radio = Some(id);
                    out.push(Out::Sound(snd::OPTION));
                }
            }
            (Wait::Radios { next, .. }, Event::Next) => {
                let chosen = next.iter().find(|o| Some(o.0) == self.radio);
                if let Some(&(_, g)) = chosen {
                    out.push(Out::Sound(snd::NEXT));
                    self.go(g, &mut out);
                }
            }
            (Wait::Pages { .. }, Event::Typed(c)) if c.is_ascii_digit() => {
                if std::mem::take(&mut self.field_fresh) {
                    self.field.clear();
                }
                if self.field.len() < 2 {
                    self.field.push(c);
                }
            }
            (Wait::Pages { .. }, Event::Backspace) => {
                if std::mem::take(&mut self.field_fresh) {
                    self.field.clear();
                }
                self.field.pop();
            }
            (Wait::Pages { max, next }, Event::Next) => match self.field.parse::<u8>() {
                Ok(n) if (1..=max).contains(&n) => {
                    out.push(Out::Sound(snd::NEXT));
                    out.push(Out::Do(Command::SetPages(n)));
                    self.ctx.pages = n;
                    self.go(next, &mut out);
                }
                _ => {
                    let r = self.script.pages_alert.unwrap_or(0);
                    self.rex(self.script.alerts, r, Some(max), &mut out);
                }
            },
            (Wait::On(t), ev) => match t.iter().find(|o| o.0.matches(&ev)) {
                Some(&(Pat::MaxDone, _)) if self.ctx.blank => {
                    out.push(Out::Sound(snd::MAX));
                    let r = self.script.blank_alert;
                    self.rex(self.script.alerts, r, None, &mut out);
                }
                Some(&(p, g)) => {
                    if matches!(p, Pat::Max | Pat::MaxDone) {
                        out.push(Out::Sound(snd::MAX));
                    }
                    self.go(g, &mut out);
                }
                None => self.out_of_turn(ev, &mut out),
            },
            (_, ev) => self.out_of_turn(ev, &mut out),
        }
        out
    }

    /// Clique no Max num passo que não o espera: `3803[0]`.
    fn out_of_turn(&mut self, ev: Event, out: &mut Vec<Out>) {
        if ev == Event::MaxClicked {
            self.guide.ask(Ask::Quit);
            out.push(Out::Sound(snd::MAX));
        }
    }

    /// Monta os balões à vista com os textos do `WHAT` do Writer e os põe
    /// na tela. Registros que faltam (ou os marcadores vazios de 1 px) não
    /// aparecem.
    pub fn placed(&self, a: &Assets) -> Vec<Placed> {
        let mut v: Vec<Placed> = Vec::new();
        for s in self.shown() {
            let records = a.dialogs(Program::Writer, s.block);
            let Some(r) = records.get(s.record) else {
                continue;
            };
            if r.height <= 3 {
                continue;
            }
            let buttons = if s.role == Role::Step {
                let b = self.buttons();
                [
                    (b.back, BalloonButton::Back),
                    (b.stop, BalloonButton::Stop),
                    (b.next, BalloonButton::Next),
                ]
                .into_iter()
                .filter_map(|(on, k)| on.then_some(k))
                .collect()
            } else {
                Vec::new()
            };
            let name = self
                .alert
                .filter(|_| s.role == Role::Alert)
                .and_then(|al| al.max)
                .map(|m| m.to_string())
                .unwrap_or_default();
            let mut b = TextBalloon::from_record(r, 0, 0, &name, buttons);
            let options = option_kinds(r);
            if s.role == Role::Step {
                if let Some(id) = self.radio {
                    let radios: Vec<u16> = options.iter().filter(|o| o.1).map(|o| o.0).collect();
                    b.selected = radios.iter().position(|&k| k == id).unwrap_or(0);
                }
                if let Some(text) = self.field()
                    && let Some(f) = b.fields.first_mut()
                {
                    *f = text.to_string();
                    b.select_all = self.field_fresh;
                }
            }
            if s.role == Role::Alert {
                b = b.as_alert();
            }
            let (w, h) = (b.box_width(), b.height(a));
            let ((x, y), tail) = match (s.role, v.iter().find(|p| p.shown.role == Role::Step)) {
                // a dica fica ao lado do balão do passo (à esquerda, se couber)
                (Role::Tip, Some(main)) if s.anchor == Anchor::Center => {
                    let mx = main.balloon.x;
                    let x = if mx - 6 - w >= 4 {
                        mx - 6 - w
                    } else {
                        mx + main.balloon.box_width() + 6
                    };
                    ((x.clamp(4, (636 - w).max(4)), main.balloon.y), None)
                }
                (Role::Quit, Some(main)) => {
                    let x = main.balloon.x - QUIT_BESIDE.0 - w;
                    (
                        (x.clamp(4, (636 - w).max(4)), main.balloon.y + QUIT_BESIDE.1),
                        None,
                    )
                }
                _ => place(s.anchor, w, h),
            };
            b.x = x;
            b.y = y.clamp(4, (476 - h).max(4));
            v.push(Placed {
                shown: s,
                balloon: b,
                tail,
                options,
            });
        }
        v
    }

    /// Desenha o Max na porta ([`MAX_DOOR`] ou, quando o passo espera um
    /// clique nele, [`MAX_SIGN`]).
    pub fn draw_max(&self, f: &mut Frame, a: &Assets) {
        let img = ImageRef::Pic(Program::Writer, self.max_picture());
        if a.has_image(&img) {
            f.blit(&a.image(&img), MAX_RECT.0, MAX_RECT.1);
        }
    }

    /// Desenha a figura da dobra e os balões à vista (com a cauda para a
    /// âncora), por cima do editor.
    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        if let Some(fold) = &self.fold
            && !self.guide.paused()
        {
            fold.draw(f, a);
        }
        for p in self.placed(a) {
            let b = &p.balloon;
            if let Some(tip) = p.tail {
                let r = (b.x, b.y, b.box_width(), b.height(a));
                draw_tail_box_in(f, Program::Writer, r, tip);
            }
            b.draw(f, a);
        }
    }

    /// Traduz um clique na tela no evento do assistente, se o clique for
    /// dele (balões, botões, dicas, o Max na porta); `None` deixa o clique
    /// para o editor. Clique fora com dica, aviso ou `3803[0]` abertos, ou
    /// dentro de um balão sem acertar nada, é [`Event::Dismiss`].
    pub fn click(&self, a: &Assets, x: i32, y: i32) -> Option<Event> {
        let mut placed = self.placed(a);
        if self.quitting() {
            // o `3803[0]` é o único que responde; o resto é "ignorar"
            placed.retain(|p| p.shown.role == Role::Quit);
        }
        // de cima para baixo: o último desenhado primeiro
        for p in placed.iter().rev() {
            let b = &p.balloon;
            if let Some(btn) = b.hit_button(a, x, y) {
                return Some(match btn {
                    BalloonButton::Back => Event::Back,
                    BalloonButton::Stop => Event::Stop,
                    BalloonButton::Next => Event::Next,
                });
            }
            if let Some(k) = b.hit_option(a, x, y)
                && let Some(&(id, radio)) = p.options.get(k)
            {
                return Some(if radio {
                    Event::Radio(id)
                } else {
                    Event::Option(id)
                });
            }
            let radios: Vec<u16> = p.options.iter().filter(|o| o.1).map(|o| o.0).collect();
            for (k, id) in radios.iter().enumerate() {
                if b.cell_rect(a, k).is_some_and(|r| inside(r, x, y)) {
                    return Some(Event::Radio(*id));
                }
            }
            match b.hot_at(a, x, y) {
                Some(Hot::Tip(t) | Hot::Link(t)) => return Some(Event::Tip(t)),
                Some(Hot::Glossary(_)) | None => {}
            }
            if b.help_at(a, x, y) {
                return Some(Event::NeedHelp);
            }
            if b.contains(a, x, y) {
                // dica ou aviso: clicar nele o fecha; o do passo é opaco
                // (o clique não vai ao editor e não faz nada)
                return Some(Event::Dismiss);
            }
        }
        if self.quitting() && !inside(MAX_RECT, x, y) {
            return Some(Event::Dismiss);
        }
        if inside(MAX_RECT, x, y) {
            return Some(Event::MaxClicked);
        }
        (self.tip.is_some() || self.alert.is_some() || self.quitting()).then_some(Event::Dismiss)
    }
}

/// Opções de um registro na ordem da tela: índice do original e se é rádio
/// (`22e`); a mesma ordem de [`Record::options`].
pub fn option_kinds(r: &Record) -> Vec<(u16, bool)> {
    let opts = r.options();
    let kinds: Vec<(u16, bool)> = r
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Picture { id, param } if *id == PIC_OPTION => Some((*param, false)),
            Item::Picture { id, param } if *id == PIC_OPTION_PICTURE => Some((*param, true)),
            _ => None,
        })
        .collect();
    opts.iter()
        .map(|(k, _)| {
            let radio = kinds.iter().find(|o| o.0 == *k).is_some_and(|o| o.1);
            (*k, radio)
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Percorre um roteiro: aplica os eventos e confere o passo de chegada.
    pub(crate) fn walk(a: &mut Assistant, evs: &[Event]) -> Vec<Out> {
        evs.iter().flat_map(|e| a.on_event(*e)).collect()
    }

    pub(crate) fn cmds(out: &[Out]) -> Vec<Command> {
        out.iter()
            .filter_map(|o| match o {
                Out::Do(c) => Some(*c),
                _ => None,
            })
            .collect()
    }

    /// Confere um roteiro: ids únicos, todo `Go` leva a um passo, todo passo
    /// é alcançável do início e, com os dados (`ATELIE_DATA`), cada balão
    /// existe no `WHAT` do Writer e não é marcador vazio; as opções, os
    /// rádios e o campo do balão são os que o passo espera; as dicas, a
    /// ajuda e os avisos existem.
    pub(crate) fn check_script(s: &'static Script) {
        let ids: Vec<&str> = (0..s.len()).map(|i| s.step(i).id).collect();
        for (i, id) in ids.iter().enumerate() {
            assert!(!ids[..i].contains(id), "passo repetido: {id}");
        }
        assert!(s.dangling().is_empty(), "sem destino: {:?}", s.dangling());
        let mut seen = vec![false; s.len()];
        let mut todo = vec![s.find(START).expect("início")];
        while let Some(i) = todo.pop() {
            if std::mem::replace(&mut seen[i], true) {
                continue;
            }
            let st = s.step(i);
            let mut gos: Vec<Go> = match st.wait {
                Wait::Next(g) | Wait::Pages { next: g, .. } => vec![g],
                Wait::Options(t) | Wait::Radios { next: t, .. } => t.iter().map(|o| o.1).collect(),
                Wait::On(t) => t.iter().map(|o| o.1).collect(),
            };
            if let Some((_, to)) = st.skip {
                gos.push(go(to));
            }
            for g in gos {
                if let Go::Step(id, _) = g {
                    todo.push(s.find(id).unwrap());
                }
            }
        }
        let lost: Vec<&str> = (0..s.len()).filter(|&i| !seen[i]).map(|i| ids[i]).collect();
        assert!(lost.is_empty(), "passos inalcançáveis: {lost:?}");
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let get = |block: u16, k: usize| -> Record {
            let recs = a.dialogs(Program::Writer, block);
            let r = recs
                .get(k)
                .unwrap_or_else(|| panic!("{block:04x}[{k}] não existe"))
                .clone();
            assert!(r.height > 3, "{block:04x}[{k}] é marcador vazio");
            r
        };
        let mut shown = Vec::new();
        for i in 0..s.len() {
            let st = s.step(i);
            for (k, say) in st.say.iter().enumerate() {
                let (block, rec) = s.record(say.rec);
                let r = get(block, rec);
                shown.push((block, r.clone()));
                if k > 0 {
                    continue;
                }
                let mut want: Vec<(u16, bool)> = match st.wait {
                    Wait::Options(t) => t.iter().map(|o| (o.0, false)).collect(),
                    Wait::Radios { next, .. } => next.iter().map(|o| (o.0, true)).collect(),
                    _ => Vec::new(),
                };
                let mut got = option_kinds(&r);
                want.sort();
                got.sort();
                assert_eq!(got, want, "opções de {} ({block:04x}[{rec}])", st.id);
                if let Wait::Radios { default, .. } = st.wait {
                    assert!(got.iter().any(|o| o.0 == default), "{}", st.id);
                }
                let field = r
                    .items
                    .iter()
                    .any(|it| matches!(it, Item::Picture { id: 0, .. }));
                assert_eq!(
                    field,
                    matches!(st.wait, Wait::Pages { .. }),
                    "campo de {}",
                    st.id
                );
            }
            if let Some(h) = st.help {
                get(s.block, h);
            }
        }
        // dicas dos balões mostrados
        for (block, r) in shown {
            for it in &r.items {
                if let Item::Picture { id, param } = it
                    && matches!(*id, 0x21b | 0x21d | 0x21e)
                    && param & 0x7fff != 0
                {
                    get(block, (param & 0x7fff) as usize);
                }
            }
        }
        get(s.block, s.help);
        get(s.alerts, s.blank_alert);
        if let Some(k) = s.pages_alert {
            let r = get(s.alerts, k);
            assert!(r.plain().contains('@'), "o aviso das páginas tem o @");
        }
        if let Some(k) = s.fifth_page {
            get(s.block, k);
        }
        let quit = get(QUIT_BLOCK, QUIT_RECORD);
        assert_eq!(
            option_kinds(&quit),
            vec![(0, false), (1, false), (2, false)]
        );
    }

    static TEST: Script = Script {
        kind: Kind::News,
        block: 0x3a00,
        alerts: 0x3a02,
        blank_alert: 1,
        pages_alert: Some(0),
        fifth_page: Some(106),
        help: 6,
        start: "a",
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
        steps: &[
            step(
                "a",
                &[say(1, Anchor::Max)],
                Wait::Options(&[(1, go("b")), (0, go_do("c", &[Command::OpenSamples]))]),
            ),
            step(
                "b",
                &[say(16, Anchor::Tool(Tool::Page)), say(17, Anchor::Max)],
                Wait::On(&[
                    (Pat::Tool(Tool::Page), go("r")),
                    (Pat::Menu(Menu::Columns, None), go("c")),
                ]),
            ),
            step(
                "c",
                &[say(2, Anchor::Max)],
                Wait::On(&[(Pat::SampleChosen, go("livre"))]),
            )
            .entering(&[Command::OpenSamples]),
            step(
                "r",
                &[say(14, Anchor::Max)],
                Wait::Radios {
                    default: 1,
                    next: &[(1, go("p")), (0, go("w"))],
                },
            ),
            step(
                "w",
                &[say(17, Anchor::Tool(Tool::Word))],
                Wait::On(&[(Pat::Tool(Tool::Word), go("p"))]),
            )
            .skip_if(Cond::WordActive, "p"),
            step(
                "p",
                &[say(16, Anchor::Max)],
                Wait::Pages {
                    max: 8,
                    next: go_do("livre", &[Command::SetView(View::SideBySide)]),
                },
            ),
            step(
                "livre",
                &[say(0, Anchor::Max)],
                Wait::On(&[(Pat::MaxDone, go("salvar"))]),
            ),
        ],
    };

    fn ctx() -> Context {
        Context {
            pages: 1,
            ..Context::default()
        }
    }

    fn new() -> Assistant {
        let mut a = Assistant::with_script(&TEST, ctx());
        assert_eq!(a.begin(), vec![Out::Entered(0)]);
        a
    }

    #[test]
    fn options_go_by_original_index_and_back_returns() {
        let mut a = new();
        assert_eq!(
            a.buttons(),
            Buttons {
                back: false,
                stop: true,
                next: false
            }
        );
        assert!(a.on_event(Event::Option(7)).is_empty());
        let out = a.on_event(Event::Option(1));
        assert_eq!(out, vec![Out::Sound(snd::OPTION), Out::Entered(1)]);
        assert_eq!(a.at(), "b");
        // "Clique aqui." que aponta a ferramenta: sem botões
        assert_eq!(a.buttons(), Buttons::default());
        let back = a.on_event(Event::Back);
        assert_eq!(back, vec![Out::Entered(0)]);
        assert_eq!(a.at(), "a");
        // comandos do Go e do passo, nessa ordem, com o som da pasta
        let out = a.on_event(Event::Option(0));
        assert_eq!(
            out,
            vec![
                Out::Sound(snd::OPTION),
                Out::Do(Command::OpenSamples),
                Out::Do(Command::OpenSamples),
                Out::Sound(snd::FOLDER),
                Out::Entered(2),
            ]
        );
    }

    #[test]
    fn screen_events_advance_only_on_the_expected_target() {
        let mut a = new();
        a.on_event(Event::Option(1));
        assert_eq!(
            a.expects(),
            vec![Pat::Tool(Tool::Page), Pat::Menu(Menu::Columns, None)]
        );
        assert!(a.on_event(Event::ToolClicked(Tool::Word)).is_empty());
        assert!(a.on_event(Event::PageClicked).is_empty());
        assert_eq!(a.at(), "b");
        a.on_event(Event::MenuItem(Menu::Columns, 3));
        assert_eq!(a.at(), "c");
    }

    #[test]
    fn radios_pick_then_next_and_skip_when_word_is_active() {
        let mut a = new();
        walk(&mut a, &[Event::Option(1), Event::ToolClicked(Tool::Page)]);
        assert_eq!(a.at(), "r");
        assert_eq!(a.radio(), Some(1));
        assert_eq!(
            a.buttons(),
            Buttons {
                back: true,
                stop: true,
                next: true
            }
        );
        a.on_event(Event::Radio(5));
        assert_eq!(a.radio(), Some(1));
        a.on_event(Event::Radio(0));
        assert_eq!(a.radio(), Some(0));
        a.on_event(Event::Next);
        assert_eq!(a.at(), "w");
        a.on_event(Event::Back);
        assert_eq!(a.radio(), Some(1), "voltar ao passo restaura o padrão");
        a.ctx.word_active = true;
        walk(&mut a, &[Event::Radio(0), Event::Next]);
        assert_eq!(a.at(), "p", "Palavra já ativa: pula o \"Clique aqui\"");
        // o Volta não passa pelo passo pulado
        a.on_event(Event::Back);
        assert_eq!(a.at(), "r");
    }

    #[test]
    fn pages_field_validates_with_the_rex() {
        let mut a = new();
        a.ctx.pages = 4;
        walk(
            &mut a,
            &[
                Event::Option(1),
                Event::ToolClicked(Tool::Page),
                Event::Next,
            ],
        );
        assert_eq!(a.at(), "p");
        assert_eq!(a.field(), Some("4"));
        // o valor inicial está selecionado: digitar substitui
        walk(&mut a, &[Event::Typed('9'), Event::Typed('x')]);
        assert_eq!(a.field(), Some("9"));
        let out = a.on_event(Event::Next);
        assert_eq!(out, vec![Out::Sound(snd::REX)]);
        assert_eq!(
            a.alert(),
            Some(Alert {
                block: 0x3a02,
                record: 0,
                max: Some(8)
            })
        );
        // com o aviso aberto, nada mais anda; qualquer resposta o fecha
        assert!(a.on_event(Event::Typed('3')).is_empty());
        a.on_event(Event::Dismiss);
        assert!(a.alert().is_none());
        walk(&mut a, &[Event::Backspace, Event::Typed('3')]);
        let out = a.on_event(Event::Next);
        assert_eq!(
            cmds(&out),
            vec![Command::SetPages(3), Command::SetView(View::SideBySide)]
        );
        assert_eq!(a.at(), "livre");
        assert_eq!(a.ctx.pages, 3);
    }

    #[test]
    fn max_done_with_blank_document_warns_and_stays() {
        let mut a = new();
        walk(&mut a, &[Event::Option(0), Event::SampleChosen]);
        assert_eq!(a.at(), "livre");
        assert_eq!(a.max_picture(), MAX_SIGN);
        a.ctx.blank = true;
        let out = a.on_event(Event::MaxClicked);
        assert_eq!(out, vec![Out::Sound(snd::MAX), Out::Sound(snd::REX)]);
        assert_eq!(a.alert().map(|al| (al.block, al.record)), Some((0x3a02, 1)));
        a.on_event(Event::MaxClicked);
        assert!(a.alert().is_none());
        a.ctx.blank = false;
        a.on_event(Event::MaxClicked);
        assert_eq!(a.at(), "salvar");
        assert_eq!(a.max_picture(), MAX_DOOR);
    }

    #[test]
    fn stop_and_max_out_of_turn_open_3803_and_pause() {
        let mut a = new();
        let out = a.on_event(Event::Stop);
        assert_eq!(out, vec![Out::Sound(snd::STOP), Out::Sound(snd::REX)]);
        assert!(a.quitting());
        // o balão do passo continua à vista, com o `3803[0]` ao lado
        let s = a.shown();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].role, Role::Step);
        assert_eq!(
            s[1],
            Shown {
                block: QUIT_BLOCK,
                record: 0,
                anchor: Anchor::Max,
                role: Role::Quit
            }
        );
        // ignorar
        a.on_event(Event::Option(2));
        assert!(!a.quitting());
        // Max fora de hora
        a.on_event(Event::MaxClicked);
        assert!(a.quitting());
        a.on_event(Event::Dismiss);
        assert!(!a.quitting());
        // trabalhar por conta própria: some até clicar no Max
        a.on_event(Event::Stop);
        a.on_event(Event::Option(0));
        assert!(a.paused());
        assert!(a.shown().is_empty());
        assert!(a.on_event(Event::Option(1)).is_empty());
        assert_eq!(
            a.on_event(Event::MaxClicked),
            vec![Out::Sound(snd::MAX), Out::Entered(0)]
        );
        // parar de trabalhar aqui: volta ao andar
        a.on_event(Event::Stop);
        assert_eq!(cmds(&a.on_event(Event::Option(1))), vec![Command::Leave]);
    }

    #[test]
    fn tips_open_beside_and_help_replaces_the_balloon() {
        let mut a = new();
        walk(&mut a, &[Event::Option(0), Event::SampleChosen]);
        assert!(a.on_event(Event::Tip(0)).is_empty(), "lâmpada do título");
        assert_eq!(a.on_event(Event::Tip(5)), vec![Out::Sound(snd::TIP)]);
        let roles: Vec<Role> = a.shown().iter().map(|s| s.role).collect();
        assert_eq!(roles, vec![Role::Step, Role::Tip]);
        a.on_event(Event::Dismiss);
        assert!(a.tip().is_none());
        assert_eq!(a.on_event(Event::NeedHelp), vec![Out::Sound(snd::HELP)]);
        let s = a.shown();
        assert_eq!(s.len(), 1);
        assert_eq!(
            (s[0].record, s[0].anchor, s[0].role),
            (6, Anchor::Bag, Role::Tip)
        );
        // outro evento fecha a dica e vale
        a.on_event(Event::MaxClicked);
        assert!(a.tip().is_none());
        assert_eq!(a.at(), "salvar");
    }

    #[test]
    fn end_flow_save_print_and_done() {
        let mut a = new();
        walk(
            &mut a,
            &[Event::Option(0), Event::SampleChosen, Event::MaxClicked],
        );
        assert_eq!(a.shown()[0].record, 7);
        let out = a.on_event(Event::Option(0));
        assert_eq!(cmds(&out), vec![Command::AskName]);
        assert_eq!(
            (a.shown()[0].block, a.shown()[0].record),
            (NAME_BLOCK, NAME_RECORD)
        );
        walk(&mut a, &[Event::Saved, Event::Option(0)]);
        assert_eq!(a.at(), "imprimir");
        walk(&mut a, &[Event::ToolClicked(Tool::Print)]);
        assert_eq!(a.shown()[0].record, 20);
        a.on_event(Event::ButtonClicked(Button::PrintMcZee));
        assert_eq!(a.at(), "imprimindo");
        assert!(a.shown().is_empty() && a.buttons() == Buttons::default());
        a.on_event(Event::Printed);
        assert_eq!(a.shown()[0].record, 12);
        // "Começar um novo": documento novo e roteiro do começo, sem Volta
        let out = a.on_event(Event::Option(1));
        assert_eq!(cmds(&out), vec![Command::NewDocument]);
        assert_eq!(a.at(), "a");
        assert!(!a.buttons().back);
    }

    #[test]
    fn end_flow_trash_and_print_strip_already_open() {
        let mut a = new();
        walk(
            &mut a,
            &[Event::Option(0), Event::SampleChosen, Event::MaxClicked],
        );
        a.ctx.open_strip = Some(Tool::Print);
        let out = walk(&mut a, &[Event::Option(1), Event::Option(0)]);
        assert_eq!(cmds(&out), vec![Command::Discard]);
        assert_eq!(a.at(), "imprimir-aberto");
        assert_eq!(a.shown()[0].record, 21);
        walk(
            &mut a,
            &[Event::ButtonClicked(Button::PrintMcZee), Event::Printed],
        );
        assert_eq!(a.at(), "pronto");
        a.on_event(Event::Option(0));
        assert_eq!(a.at(), "livre", "por conta própria");
        a.on_event(Event::MaxClicked);
        walk(&mut a, &[Event::Option(1), Event::Option(1)]);
        assert_eq!(a.at(), "pronto");
        assert_eq!(cmds(&a.on_event(Event::Option(2))), vec![Command::Leave]);
    }

    #[test]
    fn fifth_page_warns_whenever() {
        let mut a = new();
        assert_eq!(a.on_event(Event::FifthPage), vec![Out::Sound(snd::REX)]);
        assert_eq!(a.alert().map(|x| x.record), Some(106));
        a.on_event(Event::Option(0));
        assert!(a.alert().is_none());
        assert_eq!(a.at(), "a");
    }

    #[test]
    fn placement_follows_the_anchor() {
        // Max: à esquerda e abaixo, borda direita em 620
        assert_eq!(place(Anchor::Max, 180, 100), ((440, 115), Some(MAX_TAIL)));
        // ferramenta na direita: logo abaixo, para a esquerda, cauda no pé do ícone
        assert_eq!(
            place(Anchor::Tool(Tool::Viewer), 98, 60),
            ((364, 70), Some((440, 40)))
        );
        // na esquerda, para a direita
        assert_eq!(place(Anchor::Tool(Tool::Page), 130, 50).0, (155, 72));
        // não sai da tela
        assert_eq!(place(Anchor::Tool(Tool::Word), 130, 50).0, (11, 70));
        assert_eq!(place(Anchor::Button(Button::Lever), 300, 50).0.0, 238);
        // barra de baixo: acima do ponto
        assert_eq!(place(Anchor::NameBar, 160, 100).0, (240, 345));
        assert_eq!(place(Anchor::Page, 100, 60), ((270, 210), None));
    }

    fn center(r: Rect) -> (i32, i32) {
        (r.0 + r.2 / 2, r.1 + r.3 / 2)
    }

    #[test]
    fn clicks_without_data_only_reach_the_max() {
        let (_d, assets) = crate::scenes::testutil::empty_assets();
        let a = Assistant::new(Kind::News, Context::default());
        assert!(a.placed(&assets).is_empty());
        assert_eq!(a.click(&assets, 600, 50), Some(Event::MaxClicked));
        assert_eq!(a.click(&assets, 300, 300), None);
    }

    #[test]
    fn placed_balloons_translate_clicks_into_events() {
        let Some(assets) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut a = Assistant::new(Kind::News, Context::default());
        a.begin();
        let p = a.placed(&assets);
        assert_eq!(p.len(), 1);
        let b = &p[0].balloon;
        assert_eq!(p[0].options, vec![(0, false), (1, false)]);
        assert_eq!(b.x + b.box_width(), 620, "encostado à esquerda do Max");
        assert_eq!(p[0].tail, Some(MAX_TAIL));
        let o1 = center(b.option_rect(&assets, 1).unwrap());
        assert_eq!(a.click(&assets, o1.0, o1.1), Some(Event::Option(1)));
        let stop = center(b.button_rect(&assets, BalloonButton::Stop).unwrap());
        assert_eq!(a.click(&assets, stop.0, stop.1), Some(Event::Stop));
        assert!(b.button_rect(&assets, BalloonButton::Back).is_none());
        // clique no balão fora das opções não chega ao editor nem faz nada
        assert_eq!(a.click(&assets, b.x + 3, b.y + 3), Some(Event::Dismiss));
        assert!(a.clone().on_event(Event::Dismiss).is_empty());
        assert_eq!(a.click(&assets, 100, 300), None);
        // [0]: dicas e "Precisa de ajuda?"
        walk(
            &mut a,
            &[Event::Option(1), Event::Option(1), Event::Option(1)],
        );
        assert_eq!(a.at(), "livre");
        let p = a.placed(&assets);
        let b = &p[0].balloon;
        let mut tips = Vec::new();
        let mut help = false;
        for y in (b.y..b.y + b.height(&assets)).step_by(2) {
            for x in (b.x..b.x + b.box_width()).step_by(4) {
                match a.click(&assets, x, y) {
                    Some(Event::Tip(t)) if !tips.contains(&t) => tips.push(t),
                    Some(Event::NeedHelp) => help = true,
                    _ => {}
                }
            }
        }
        tips.sort();
        assert_eq!(tips, vec![5, 15, 23]);
        assert!(help);
        // a dica abre ao lado; clicar nela fecha
        a.on_event(Event::Tip(5));
        let p = a.placed(&assets);
        assert_eq!(p.len(), 2);
        let tip = &p[1].balloon;
        assert!(tip.x + tip.box_width() <= p[0].balloon.x || tip.x >= p[0].balloon.x);
        assert_eq!(
            a.click(&assets, tip.x + 5, tip.y + tip.height(&assets) - 3),
            Some(Event::Dismiss)
        );
        assert_eq!(a.click(&assets, 100, 300), Some(Event::Dismiss));
    }

    #[test]
    fn radios_field_and_alert_are_placed() {
        let Some(assets) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut c = Assistant::new(Kind::Card, Context::default());
        c.begin();
        walk(&mut c, &[Event::Option(1), Event::Option(1)]);
        let p = c.placed(&assets);
        assert_eq!(p[0].options, vec![(1, true), (0, true)]);
        assert_eq!(p[0].balloon.selected, 0, "Todas (22e:1) marcado");
        let cell = center(p[0].balloon.cell_rect(&assets, 1).unwrap());
        assert_eq!(c.click(&assets, cell.0, cell.1), Some(Event::Radio(0)));
        c.on_event(Event::Radio(0));
        assert_eq!(c.placed(&assets)[0].balloon.selected, 1);
        let next = c.placed(&assets)[0]
            .balloon
            .button_rect(&assets, BalloonButton::Next)
            .map(center)
            .unwrap();
        assert_eq!(c.click(&assets, next.0, next.1), Some(Event::Next));
        // faixa: campo das páginas e o Rex com o máximo
        let mut f = Assistant::new(Kind::Banner, Context::default());
        f.begin();
        walk(
            &mut f,
            &[
                Event::Option(1),
                Event::ToolClicked(Tool::Word),
                Event::ButtonClicked(Button::Shape),
                Event::ShapedOk,
            ],
        );
        let p = f.placed(&assets);
        assert_eq!(p[0].balloon.fields, vec!["1".to_string()]);
        assert!(p[0].balloon.select_all);
        walk(&mut f, &[Event::Typed('0'), Event::Next]);
        let p = f.placed(&assets);
        let alert = p.last().unwrap();
        assert_eq!(alert.shown.role, Role::Alert);
        assert!(alert.balloon.alert);
        assert!(alert.balloon.plain_text().contains("entre 1 e 8"));
        // os balões cabem na tela
        for p in p {
            let b = &p.balloon;
            assert!(b.x >= 4 && b.x + b.box_width() <= 640, "{:?}", p.shown);
            assert!(b.y >= 4 && b.y + b.height(&assets) <= 480, "{:?}", p.shown);
        }
    }

    #[test]
    fn every_balloon_of_every_script_fits_the_screen() {
        let Some(assets) = crate::scenes::testutil::assets() else {
            return;
        };
        for kind in Kind::ALL {
            let s = kind.script();
            for i in 0..s.len() {
                let mut a = Assistant::with_script(s, Context::default());
                a.guide.replace(i);
                a.enter(&mut Vec::new());
                for p in a.placed(&assets) {
                    let b = &p.balloon;
                    let (w, h) = (b.box_width(), b.height(&assets));
                    assert!(
                        b.x >= 4 && b.x + w <= 640 && b.y >= 4 && b.y + h <= 480,
                        "{kind:?} {}: {:?} em {},{} {w}×{h}",
                        a.at(),
                        p.shown,
                        b.x,
                        b.y
                    );
                }
            }
        }
    }

    /// Quadros para comparar com as capturas do original
    /// (`ATELIE_DUMP=<pasta>`): `projects_45_dobra_1`, `51_visor_1`, …
    #[test]
    fn dump_balloons_for_comparison() {
        let Some(assets) = crate::scenes::testutil::assets() else {
            return;
        };
        use crate::scenes::testutil::dump;
        let shot = |a: &Assistant, name: &str| {
            let mut f = Frame::new(640, 480);
            f.fill([255, 255, 255]);
            a.draw_max(&mut f, &assets);
            a.draw(&mut f, &assets);
            dump(&f, name);
        };
        let mut c = Assistant::new(Kind::Card, Context::default());
        c.begin();
        shot(&c, "cw_assist_card_0");
        walk(&mut c, &[Event::Option(0)]);
        shot(&c, "cw_assist_card_dobra_1");
        walk(&mut c, &[Event::Next, Event::Next]);
        c.tick(2.0);
        shot(&c, "cw_assist_card_dobra_3");
        let mut c = Assistant::new(Kind::Card, Context::default());
        c.begin();
        walk(
            &mut c,
            &[
                Event::Option(1),
                Event::Option(1),
                Event::Radio(0),
                Event::Next,
            ],
        );
        shot(&c, "cw_assist_card_visores");
        let mut n = Assistant::new(Kind::News, Context::default());
        n.begin();
        walk(
            &mut n,
            &[Event::Option(1), Event::Option(1), Event::Option(1)],
        );
        n.on_event(Event::Tip(5));
        shot(&n, "cw_assist_news_livre_dica");
        let mut f = Assistant::new(Kind::Banner, Context::default());
        f.begin();
        walk(&mut f, &[Event::Option(1), Event::ToolClicked(Tool::Word)]);
        walk(
            &mut f,
            &[Event::ButtonClicked(Button::Shape), Event::ShapedOk],
        );
        shot(&f, "cw_assist_banner_paginas");
        walk(&mut f, &[Event::Typed('9'), Event::Next]);
        shot(&f, "cw_assist_banner_rex");
        f.on_event(Event::Dismiss);
        f.on_event(Event::Stop);
        shot(&f, "cw_assist_banner_para");
    }

    #[test]
    fn option_kinds_follow_the_record() {
        use formats::what::Item as I;
        let txt = |s: &str| I::Text {
            text: s.into(),
            bold: false,
            color: 0xf,
            indent: 0,
            key: false,
            signature: false,
        };
        let r = Record {
            title: "t".into(),
            width: 100,
            height: 50,
            style: 2,
            items: vec![
                txt("Você quer:"),
                I::Newline,
                I::Picture {
                    id: 0x22e,
                    param: 1,
                },
                txt("Todas"),
                I::Newline,
                I::Picture {
                    id: 0x22e,
                    param: 0,
                },
                txt("Uma"),
                I::Newline,
                I::Picture {
                    id: 0x22c,
                    param: 2,
                },
                txt("Outra"),
            ],
        };
        assert_eq!(option_kinds(&r), vec![(1, true), (0, true), (2, false)]);
    }
}
