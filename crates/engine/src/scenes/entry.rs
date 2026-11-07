//! Entrada do Fine Artist: o McZee de livro na mão sobre as nuvens (fundo
//! `4139`), o quadro-negro com a lista de usuários e os botões Sou eu /
//! Primeira vez / Procurar; o cadastro (nome, bicho preferido, pasta,
//! apelido) em balões sucessivos do `WHAT 4400`, com os avisos "EPA!" do
//! `WHAT 4402` e o balão de saída do `WHAT 1001[2]`. Posições e sons
//! medidos no original (ver `docs/telas/fine-artist.md`).

use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::{balloon_font, draw_line};
use crate::scene::{Effect, Input, Key, KidCommon, MouseButton, Scene, SceneId};
use crate::text_balloon::{Button, PIC_ALERT, PIC_ANIMAL0, Part, TextBalloon, blit_pic};
use crate::users::{self, ANIMALS, Profile};
use crate::{Frame, SCREEN_H, SCREEN_W};
use formats::what::Record;
use std::collections::VecDeque;
use std::path::PathBuf;

pub const BACKGROUND: u16 = 0x4139;
/// McZee de livro na mão (123×309; 1 bit e máscara são as duas seguintes).
pub const MCZEE: (u16, i32, i32) = (0x42f9, 435, 112);
/// Botões do quadro-negro (figura colorida; a máscara é duas adiante).
pub const BTN_ME: (u16, i32, i32) = (0x473, 280, 215);
pub const BTN_FIRST: (u16, i32, i32) = (0x477, 280, 253);
pub const BTN_BROWSE: (u16, i32, i32) = (0x47b, 280, 288);
/// "Coloque aqui" do passo da pasta (109×32).
pub const BTN_PUT_HERE: (u16, i32, i32) = (0x47f, 282, 218);
/// Setas da lista (12×12).
pub const ARROW_UP: (u16, i32, i32) = (0x2db, 169, 343);
pub const ARROW_DOWN: (u16, i32, i32) = (0x2df, 220, 343);
/// Quadro-negro (moldura externa) e caixa branca da lista.
const BOARD: Rect = (120, 170, 311, 200);
const LIST: Rect = (130, 179, 142, 163);
const ROW_H: i32 = 18;
/// Linhas visíveis de usuários (a primeira linha é o caminho da pasta).
const ROWS: usize = 8;
/// Blocos `WHAT` da entrada, dos avisos e do balão de saída.
const ENTRY_WHAT: u16 = 0x4400;
const ERRORS_WHAT: u16 = 0x4402;
const QUIT_WHAT: u16 = 0x1001;
/// Posição dos balões do McZee: o grande (lista), os de opções (boas-vindas
/// e agradecimento), os do cadastro, e os avisos "EPA!" sobre o quadro-negro,
/// sobre o formulário e o de saída.
const BIG: (i32, i32) = (84, 24);
const OPTIONS: (i32, i32) = (64, 76);
const FORM: (i32, i32) = (112, 78);
const ALERT_BOARD: (i32, i32) = (213, 173);
const ALERT_FORM: (i32, i32) = (205, 148);
const ALERT_QUIT: (i32, i32) = (214, 179);

/// Sons da Entrada (ids do `ARTIST.SND`, medidos no original pelo tamanho
/// dos buffers do `waveOutWrite`; quando vários ids têm o mesmo som, o
/// primeiro).
pub mod snd {
    /// Ao entrar na tela, em sequência.
    pub const OPENING: [u16; 3] = [0x300, 0x2f6, 0x2fd];
    /// Tique ao escolher um nome da lista.
    pub const SELECT: u16 = 0x230;
    /// Tique ao escolher a pasta no passo "Onde você quer que eu guarde".
    pub const SELECT_HOME: u16 = 0x22d;
    /// Setas da lista.
    pub const ARROW: u16 = 0x2bc;
    /// Fala curta do McZee ao clicar nele.
    pub const MCZEE: u16 = 0x252;
    /// "Sou eu" e "Coloque aqui".
    pub const ME: u16 = 0x239;
    /// "Primeira vez".
    pub const FIRST: u16 = 0x2fb;
    /// "Procurar" (fala longa).
    pub const BROWSE: u16 = 0x26d;
    /// Botão Para.
    pub const STOP: u16 = 0x2ec;
    /// Vinheta dos avisos "EPA!".
    pub const ALERT: u16 = 0x2fe;
    /// Vinheta de recusa: clique fora de um balão modal.
    pub const REFUSE: u16 = 0x217;
    /// Opção escolhida (bolinha), inclusive "Continuar trabalhando".
    pub const CHOICE: u16 = 0x254;
    /// Botão Segue.
    pub const NEXT: u16 = 0x2eb;
    /// Botão Volta.
    pub const BACK: u16 = 0x2f5;
}

type Rect = (i32, i32, i32, i32);

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn pic_rect(p: (u16, i32, i32), w: i32, h: i32) -> Rect {
    (p.1, p.2, w, h)
}

fn sound(id: u16) -> SoundRef {
    SoundRef::Snd(Program::Artist, id)
}

/// Avisos do `WHAT 4402`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Warning {
    /// Apelido repetido (`4402[0]`): volta ao apelido.
    Duplicate,
    /// Nome vazio (`4402[1]`): volta ao nome.
    BadName,
    /// Apelido inválido (`4402[2]`): volta ao apelido.
    BadNickname,
    /// "Sou eu" sem nome escolhido (`4402[3]`): volta à lista.
    NoLogin,
}

impl Warning {
    /// Passo que fica por baixo do aviso e ao qual ele volta.
    fn under(self) -> Step {
        match self {
            Warning::Duplicate | Warning::BadNickname => Step::Nickname,
            Warning::BadName => Step::Name,
            Warning::NoLogin => Step::List,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    List,
    /// Balão de opções depois de "Sou eu" (modal; o quadro-negro some).
    Welcome,
    Name,
    Animal,
    /// "Onde você quer que eu guarde o seu trabalho?": o quadro-negro com a
    /// pasta e o botão "Coloque aqui".
    Home,
    Nickname,
    Thanks,
    Warn(Warning),
    /// "EPA! O que você quer fazer?" (Sair / Continuar), por cima de
    /// `Entry::quit_from`.
    Quit,
}

pub struct Entry {
    common: KidCommon,
    dir: Option<PathBuf>,
    users: Vec<String>,
    selected: Option<usize>,
    scroll: usize,
    step: Step,
    /// Passo por baixo do balão de saída.
    quit_from: Step,
    /// Balão da lista (`4400[1]`, com o botão Para).
    intro: TextBalloon,
    /// Balões com opções: boas-vindas (`4400[2]`) e agradecimento (`4400[3]`).
    options: Option<TextBalloon>,
    name_form: TextBalloon,
    animal_form: TextBalloon,
    home_form: TextBalloon,
    nick_form: TextBalloon,
    warning: Option<TextBalloon>,
    quit: Option<TextBalloon>,
    draft: Profile,
    /// Apelido original quando se altera um cadastro existente.
    editing: Option<String>,
    /// Para onde volta o Volta do nome: lista ou "Obrigado…".
    name_back: Step,
    /// A pasta foi escolhida no passo `Home`.
    home_selected: bool,
    /// Usuário dado na configuração: pula a entrada.
    preset: Option<String>,
    /// Os sons de abertura já foram pedidos.
    opened: bool,
    /// Sons a tocar em sequência e quanto falta para o próximo.
    queue: VecDeque<u16>,
    queue_wait: f32,
    done: bool,
}

fn record(assets: &Assets, block: u16, k: usize) -> Option<Record> {
    assets.dialogs(Program::Artist, block).into_iter().nth(k)
}

/// Balão do `WHAT` com os campos e as opções no número esperado; senão o
/// reserva. Posição, botões e fundo vêm do reserva.
fn form_balloon(
    assets: &Assets,
    block: u16,
    k: usize,
    name: &str,
    fallback: TextBalloon,
) -> TextBalloon {
    match record(assets, block, k) {
        Some(r) => {
            let mut b = TextBalloon::from_record(
                &r,
                fallback.x,
                fallback.y,
                name,
                fallback.buttons.clone(),
            );
            b.alert = fallback.alert;
            if b.fields.len() == fallback.fields.len()
                && b.options().len() == fallback.options().len()
            {
                b
            } else {
                fallback
            }
        }
        None => fallback,
    }
}

fn text(s: &str) -> Part {
    Part::Text(s.to_string())
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

impl Entry {
    pub fn new(dir: Option<PathBuf>, click: Option<SoundRef>, assets: &Assets) -> Entry {
        let users = dir.as_deref().map(users::list).unwrap_or_default();
        let mut e = Entry {
            common: KidCommon::new(click),
            dir,
            users,
            selected: None,
            scroll: 0,
            step: Step::List,
            quit_from: Step::List,
            intro: Self::intro_balloon(assets),
            options: None,
            name_form: Self::name_balloon(assets),
            animal_form: Self::animal_balloon(assets, ""),
            home_form: Self::home_balloon(assets),
            nick_form: Self::nick_balloon(assets),
            warning: None,
            quit: None,
            draft: Profile::default(),
            editing: None,
            name_back: Step::List,
            home_selected: false,
            preset: None,
            opened: false,
            queue: VecDeque::new(),
            queue_wait: 0.0,
            done: false,
        };
        e.name_form.focus = 0;
        e
    }

    /// Entrada com o usuário já escolhido (`--user` ou `usuario` no config):
    /// vai direto a Imaginópolis.
    pub fn with_user(mut self, user: &str) -> Entry {
        self.preset = Some(user.to_string());
        self
    }

    pub fn step_name(&self) -> &'static str {
        match self.step {
            Step::List => "lista",
            Step::Welcome => "boas-vindas",
            Step::Name => "nome",
            Step::Animal => "bicho",
            Step::Home => "pasta",
            Step::Nickname => "apelido",
            Step::Thanks => "obrigado",
            Step::Warn(_) => "aviso",
            Step::Quit => "sair",
        }
    }

    pub fn users(&self) -> &[String] {
        &self.users
    }

    pub fn selected(&self) -> Option<&str> {
        self.selected
            .and_then(|k| self.users.get(k))
            .map(String::as_str)
    }

    fn intro_balloon(assets: &Assets) -> TextBalloon {
        let fallback = TextBalloon::new(
            BIG.0,
            BIG.1,
            327,
            vec![
                text("Oi, eu sou o McZee! Diga quem é você!"),
                text(
                    "Se você estiver vendo o seu nome aí embaixo, clique nele e depois em \"Sou eu\".",
                ),
                text("Se o meu mundo for novidade para você, clique no botão \"Primeira vez\"."),
            ],
            vec![Button::Stop],
        );
        form_balloon(assets, ENTRY_WHAT, 1, "", fallback)
    }

    fn name_balloon(assets: &Assets) -> TextBalloon {
        let fallback = TextBalloon::new(
            FORM.0,
            FORM.1,
            297,
            vec![
                text("Preciso saber seu nome e sobrenome.\nDigite o seu nome aqui:"),
                Part::Field {
                    field: 0,
                    width: 144,
                },
                text("Digite o seu sobrenome aqui:"),
                Part::Field {
                    field: 1,
                    width: 144,
                },
                text("Para continuar, clique no botão Segue"),
            ],
            vec![Button::Back, Button::Stop, Button::Next],
        );
        form_balloon(assets, ENTRY_WHAT, 4, "", fallback)
    }

    fn animal_balloon(assets: &Assets, name: &str) -> TextBalloon {
        let fallback = TextBalloon::new(
            FORM.0,
            FORM.1,
            277,
            vec![
                Part::Text(format!(
                    "Oi, {name}! Que bom te conhecer! Qual dos animais da lista aí embaixo é o seu preferido?"
                )),
                Part::Grid {
                    pictures: (0..ANIMALS as u16).map(|k| PIC_ANIMAL0 + 2 * k).collect(),
                    cols: 3,
                },
                text("Para continuar, clique no botão Segue."),
            ],
            vec![Button::Back, Button::Stop, Button::Next],
        );
        form_balloon(assets, ENTRY_WHAT, 8, name, fallback)
    }

    /// "Onde você quer que eu guarde o seu trabalho?" (`4400[6]`), sem a
    /// dica do disquete (o Ateliê Livre guarda tudo na pasta de dados).
    fn home_balloon(assets: &Assets) -> TextBalloon {
        let fallback = TextBalloon::new(
            FORM.0,
            FORM.1,
            264,
            vec![
                text("Onde você quer que eu guarde o seu trabalho?"),
                text(
                    "1 Na lista abaixo, clique no nome da pasta.\n2 Clique no botão \"Coloque aqui\".",
                ),
                text("Se você não souber o que fazer, basta clicar em \"Coloque aqui\"."),
            ],
            vec![Button::Back, Button::Stop],
        );
        let mut b = form_balloon(assets, ENTRY_WHAT, 6, "", fallback);
        for p in &mut b.parts {
            if let Part::Text(t) = p {
                *t = t
                    .lines()
                    .filter(|l| !l.contains("disquete"))
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string();
            }
        }
        b.parts
            .retain(|p| !matches!(p, Part::Text(t) if t.is_empty()));
        b
    }

    fn nick_balloon(assets: &Assets) -> TextBalloon {
        let fallback = TextBalloon::new(
            FORM.0,
            FORM.1,
            259,
            vec![
                text(
                    "Mais uma coisa — se você tiver algum apelido que prefere usar, digite-o aqui:",
                ),
                Part::Field {
                    field: 0,
                    width: 100,
                },
                text("Para continuar, clique no botão Segue."),
            ],
            vec![Button::Back, Button::Stop, Button::Next],
        );
        form_balloon(assets, ENTRY_WHAT, 5, "", fallback)
    }

    fn welcome_balloon(assets: &Assets, name: &str) -> TextBalloon {
        let fallback = TextBalloon::new(
            OPTIONS.0,
            OPTIONS.1,
            344,
            vec![
                Part::Text(format!(
                    "{name}! Que bom ver você de novo! O que você quer fazer?"
                )),
                Part::Options(strings(&[
                    "Ir direto para Imaginópolis",
                    "Mudar o que você sabe sobre mim",
                    "Conte mais sobre o seu mundo",
                    "Ir direto para o Ateliê de Pintura",
                ])),
            ],
            vec![Button::Back, Button::Stop],
        );
        form_balloon(assets, ENTRY_WHAT, 2, name, fallback)
    }

    fn thanks_balloon(assets: &Assets, name: &str) -> TextBalloon {
        let fallback = TextBalloon::new(
            OPTIONS.0 + 6,
            OPTIONS.1,
            340,
            vec![
                Part::Text(format!(
                    "Obrigado por ter me contado coisas sobre você, {name}. Você quer saber um pouco mais sobre mim?"
                )),
                Part::Options(strings(&[
                    "É, por favor conte mais",
                    "Não precisa, vamos direto para Imaginópolis",
                    "Quero mudar o que você sabe sobre mim",
                ])),
            ],
            vec![Button::Stop],
        );
        form_balloon(assets, ENTRY_WHAT, 3, name, fallback)
    }

    /// "EPA! O que você quer fazer?" (`1001[2]`): Sair / Continuar.
    fn quit_balloon(assets: &Assets) -> TextBalloon {
        let fallback = TextBalloon::new(
            ALERT_QUIT.0,
            ALERT_QUIT.1,
            210,
            vec![
                Part::Figure(PIC_ALERT),
                text("O que você quer fazer?"),
                Part::Options(strings(&["Sair do Fine Artist", "Continuar trabalhando"])),
            ],
            vec![],
        )
        .as_alert();
        form_balloon(assets, QUIT_WHAT, 2, "", fallback)
    }

    fn warning_balloon(assets: &Assets, w: Warning, name: &str) -> TextBalloon {
        let (k, msg, width, pos) = match w {
            Warning::Duplicate => (
                0,
                format!(
                    "Ops! Você não pode usar esse nome!\nEsse apelido ({name}) já está sendo usado por outra pessoa. Mude o seu apelido um pouquinho — use as suas iniciais ou junte a primeira letra do seu sobrenome."
                ),
                258,
                ALERT_FORM,
            ),
            Warning::BadName => (
                1,
                "Tem certeza de que esse é o seu nome?\nPara mim, isso não parece um nome. Vamos tentar de novo? Digite o seu nome — você pode usar letras ou números. Não deixe de digitar coisas nas duas caixas.".to_string(),
                230,
                ALERT_FORM,
            ),
            Warning::BadNickname => (
                2,
                "Experimente outro apelido.\nEu preciso de um apelido para você — digite um nome com 8 caracteres ou menos, letras ou números, sem espaços e sem pontuação.".to_string(),
                212,
                ALERT_FORM,
            ),
            Warning::NoLogin => (
                3,
                "Eu preciso saber o seu nome.\nClique primeiro no seu nome que aparece na lista! Depois, clique em Sou eu.".to_string(),
                213,
                ALERT_BOARD,
            ),
        };
        let fallback = TextBalloon::new(
            pos.0,
            pos.1,
            width,
            vec![Part::Figure(PIC_ALERT), Part::Text(msg)],
            vec![],
        )
        .as_alert();
        form_balloon(assets, ERRORS_WHAT, k, name, fallback)
    }

    fn list_rows(&self) -> Vec<Rect> {
        (0..ROWS)
            .map(|k| {
                (
                    LIST.0 + 1,
                    LIST.1 + ROW_H * (k as i32 + 1),
                    LIST.2 - 2,
                    ROW_H - 1,
                )
            })
            .collect()
    }

    fn reload_users(&mut self) {
        self.users = self.dir.as_deref().map(users::list).unwrap_or_default();
        if self.selected.is_some_and(|k| k >= self.users.len()) {
            self.selected = None;
        }
    }

    /// Nome da pasta de dados, como aparece no quadro-negro.
    fn folder_name(&self) -> String {
        self.dir
            .as_ref()
            .and_then(|d| d.file_name())
            .and_then(|n| n.to_str())
            .map_or("(sem pasta)".to_string(), |n| format!("{n}/"))
    }

    /// Toca sons em sequência: o primeiro já, os outros quando o anterior
    /// acabar. Uma ação nova interrompe a sequência pendente.
    fn play(&mut self, ids: &[u16]) {
        self.queue.clear();
        self.queue.extend(ids.iter().copied());
        self.queue_wait = 0.0;
    }

    /// Toca o próximo som da fila quando o anterior acabou.
    fn pump_sounds(&mut self, dt: f32, fx: &mut Vec<Effect>, assets: &Assets) {
        self.queue_wait -= dt;
        if self.queue_wait > 0.0 {
            return;
        }
        if let Some(id) = self.queue.pop_front() {
            let r = sound(id);
            self.queue_wait = assets.sound(&r).map_or(0.5, |p| {
                p.samples.len() as f32 / (p.sample_rate.max(1) as f32 * p.channels.max(1) as f32)
            });
            fx.push(Effect::PlaySound(r));
        } else {
            self.queue_wait = 0.0;
        }
    }

    fn go(&mut self, step: Step, assets: &Assets) {
        self.step = step;
        // o balão de saída fica por cima do de opções, que continua visível
        if step != Step::Quit {
            self.options = None;
        }
        self.warning = None;
        self.quit = None;
        let name = self.display_name();
        match step {
            Step::Welcome => self.options = Some(Self::welcome_balloon(assets, &name)),
            Step::Thanks => self.options = Some(Self::thanks_balloon(assets, &name)),
            Step::Warn(w) => {
                let n = if w == Warning::Duplicate {
                    self.nick_form.fields[0].clone()
                } else {
                    name
                };
                self.warning = Some(Self::warning_balloon(assets, w, &n));
            }
            Step::Quit => self.quit = Some(Self::quit_balloon(assets)),
            Step::Animal => {
                let sel = self.animal_form.selected;
                self.animal_form = Self::animal_balloon(assets, &self.draft.nome);
                self.animal_form.selected = sel.min(ANIMALS - 1);
            }
            Step::Nickname => {
                // o apelido sugerido vem selecionado: digitar substitui
                self.nick_form.focus = 0;
                self.nick_form.select_all = !self.nick_form.fields[0].is_empty();
            }
            _ => {}
        }
    }

    /// Abre o balão de saída por cima do passo atual.
    fn ask_quit(&mut self, assets: &Assets) {
        self.play(&[snd::STOP, snd::ALERT]);
        self.quit_from = self.step;
        self.go(Step::Quit, assets);
    }

    /// Nome para o `@`: o apelido escolhido ou o primeiro nome digitado.
    fn display_name(&self) -> String {
        if let Some(s) = self.selected()
            && matches!(
                self.step,
                Step::List | Step::Welcome | Step::Warn(Warning::NoLogin)
            )
        {
            return s.to_string();
        }
        if !self.draft.apelido.is_empty() && matches!(self.step, Step::Thanks) {
            return self.draft.apelido.clone();
        }
        if !self.draft.nome.is_empty() {
            return self.draft.nome.clone();
        }
        self.selected().unwrap_or("artista").to_string()
    }

    fn start_registration(&mut self, prefill: Option<Profile>, back: Step, assets: &Assets) {
        self.editing = prefill.as_ref().map(|p| p.apelido.clone());
        self.draft = prefill.unwrap_or_default();
        self.name_form = Self::name_balloon(assets);
        self.name_form.fields[0] = self.draft.nome.clone();
        self.name_form.fields[1] = self.draft.sobrenome.clone();
        self.nick_form = Self::nick_balloon(assets);
        self.nick_form.fields[0] = self.draft.apelido.clone();
        self.animal_form.selected = self.draft.bicho;
        self.home_selected = false;
        self.name_back = back;
        self.go(Step::Name, assets);
    }

    fn finish(&mut self, user: &str, next: SceneId, fx: &mut Vec<Effect>) {
        self.done = true;
        fx.push(Effect::SetUser(user.to_string()));
        fx.push(Effect::ChangeScene(next));
    }

    /// Grava o cadastro; `Err` com o aviso a mostrar.
    fn save(&mut self, fx: &mut Vec<Effect>) -> Result<(), Warning> {
        let apelido = self.nick_form.fields[0].trim().to_string();
        if !users::valid_nickname(&apelido) {
            return Err(Warning::BadNickname);
        }
        let taken = self
            .users
            .iter()
            .any(|u| u.eq_ignore_ascii_case(&apelido) && Some(u) != self.editing.as_ref());
        if taken {
            return Err(Warning::Duplicate);
        }
        self.draft.apelido = apelido;
        self.draft.bicho = self.animal_form.selected.min(ANIMALS - 1);
        if let Some(dir) = &self.dir {
            if let Some(old) = &self.editing
                && *old != self.draft.apelido
                && let Err(e) = users::remove(dir, old)
            {
                fx.push(Effect::Log(format!("cadastro: apagar {old}: {e}")));
            }
            if let Err(e) = users::save(dir, &self.draft) {
                fx.push(Effect::Log(format!("cadastro: {e}")));
            }
        } else {
            fx.push(Effect::Log(
                "cadastro: sem pasta de dados; o usuário não foi guardado".into(),
            ));
        }
        self.editing = Some(self.draft.apelido.clone());
        self.reload_users();
        self.selected = self.users.iter().position(|u| *u == self.draft.apelido);
        Ok(())
    }

    /// Clique no McZee: uma fala curta.
    fn mczee_hit(x: i32, y: i32) -> bool {
        inside((440, 112, 120, 288), x, y)
    }

    fn list_input(&mut self, i: &Input, fx: &mut Vec<Effect>, assets: &Assets) {
        if let Some(Button::Stop) = self.intro.handle(assets, i) {
            self.ask_quit(assets);
            return;
        }
        match *i {
            Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } => {
                if let Some(k) = self.list_rows().iter().position(|r| inside(*r, x, y)) {
                    let idx = self.scroll + k;
                    if idx < self.users.len() && self.selected != Some(idx) {
                        self.selected = Some(idx);
                        self.play(&[snd::SELECT]);
                    }
                } else if inside(pic_rect(BTN_ME, 114, 21), x, y) {
                    if self.selected.is_some() {
                        self.play(&[snd::ME]);
                        self.go(Step::Welcome, assets);
                    } else {
                        self.play(&[snd::ME, snd::ALERT]);
                        self.go(Step::Warn(Warning::NoLogin), assets);
                    }
                } else if inside(pic_rect(BTN_FIRST, 143, 20), x, y) {
                    self.play(&[snd::FIRST]);
                    self.start_registration(None, Step::List, assets);
                } else if inside(pic_rect(BTN_BROWSE, 140, 24), x, y) {
                    // procura usuários de novo: a lista é relida e perde a escolha
                    self.play(&[snd::BROWSE]);
                    self.reload_users();
                    self.selected = None;
                    fx.push(Effect::Log(
                        "Procurar: não há discos para procurar no Ateliê Livre; a lista foi relida"
                            .into(),
                    ));
                } else if inside(pic_rect(ARROW_UP, 12, 12), x, y) {
                    self.play(&[snd::ARROW]);
                    self.scroll = self.scroll.saturating_sub(1);
                } else if inside(pic_rect(ARROW_DOWN, 12, 12), x, y) {
                    self.play(&[snd::ARROW]);
                    if self.scroll + ROWS < self.users.len() {
                        self.scroll += 1;
                    }
                } else if Self::mczee_hit(x, y) {
                    self.play(&[snd::MCZEE]);
                }
            }
            Input::Key { key: Key::Down, .. } if !self.users.is_empty() => {
                let k = self
                    .selected
                    .map_or(0, |k| (k + 1).min(self.users.len() - 1));
                self.selected = Some(k);
                if k >= self.scroll + ROWS {
                    self.scroll = k + 1 - ROWS;
                }
            }
            Input::Key { key: Key::Up, .. } if !self.users.is_empty() => {
                let k = self.selected.map_or(0, |k| k.saturating_sub(1));
                self.selected = Some(k);
                if k < self.scroll {
                    self.scroll = k;
                }
            }
            Input::Key {
                key: Key::Enter, ..
            } if self.selected.is_some() => {
                self.play(&[snd::ME]);
                self.go(Step::Welcome, assets);
            }
            _ => {}
        }
    }

    /// Balão de opções modal: a opção, um botão, ou a vinheta de recusa fora dele.
    fn welcome_input(&mut self, i: &Input, fx: &mut Vec<Effect>, assets: &Assets) {
        let Input::Click {
            button: MouseButton::Left,
            x,
            y,
        } = *i
        else {
            return;
        };
        let Some(b) = &self.options else { return };
        let user = self.selected().unwrap_or("artista").to_string();
        if let Some(k) = b.hit_option(assets, x, y) {
            self.play(&[snd::CHOICE]);
            match k {
                0 => self.finish(&user, SceneId::Imaginopolis, fx),
                1 => {
                    let prof = self
                        .dir
                        .as_deref()
                        .and_then(|d| users::load(d, &user))
                        .unwrap_or(Profile {
                            apelido: user.clone(),
                            ..Profile::default()
                        });
                    self.start_registration(Some(prof), Step::List, assets);
                }
                2 => self.finish(&user, SceneId::Story, fx),
                // direto à tela de pintura, sem passar pelo cômodo
                _ => self.finish(&user, SceneId::Paint, fx),
            }
            return;
        }
        match b.hit_button(assets, x, y) {
            Some(Button::Back) => {
                self.play(&[snd::BACK]);
                self.selected = None;
                self.go(Step::List, assets);
            }
            Some(Button::Stop) => self.ask_quit(assets),
            _ => {
                if !b.contains(assets, x, y) {
                    self.play(&[snd::REFUSE]);
                }
            }
        }
    }

    fn thanks_input(&mut self, i: &Input, fx: &mut Vec<Effect>, assets: &Assets) {
        let Input::Click {
            button: MouseButton::Left,
            x,
            y,
        } = *i
        else {
            return;
        };
        let Some(b) = &self.options else { return };
        let user = self.draft.apelido.clone();
        if let Some(k) = b.hit_option(assets, x, y) {
            self.play(&[snd::CHOICE]);
            match k {
                0 => self.finish(&user, SceneId::Story, fx),
                1 => self.finish(&user, SceneId::Imaginopolis, fx),
                _ => {
                    let prof = self.draft.clone();
                    self.start_registration(Some(prof), Step::Thanks, assets);
                }
            }
            return;
        }
        if b.hit_button(assets, x, y) == Some(Button::Stop) {
            self.ask_quit(assets);
        }
    }

    fn form_input(&mut self, i: &Input, fx: &mut Vec<Effect>, assets: &Assets) {
        let step = self.step;
        let form = match step {
            Step::Name => &mut self.name_form,
            Step::Animal => &mut self.animal_form,
            Step::Home => &mut self.home_form,
            _ => &mut self.nick_form,
        };
        let Some(b) = form.handle(assets, i) else {
            if step == Step::Home {
                self.home_input(i, fx, assets);
            }
            return;
        };
        match (step, b) {
            (_, Button::Stop) => self.ask_quit(assets),
            (Step::Name, Button::Back) => {
                self.play(&[snd::BACK]);
                let back = self.name_back;
                self.go(back, assets);
            }
            (Step::Name, Button::Next) => {
                let nome = self.name_form.fields[0].trim().to_string();
                let sobrenome = self.name_form.fields[1].trim().to_string();
                if nome.is_empty() || sobrenome.is_empty() {
                    self.play(&[snd::NEXT, snd::ALERT]);
                    self.go(Step::Warn(Warning::BadName), assets);
                } else {
                    self.play(&[snd::NEXT]);
                    self.draft.nome = nome;
                    self.draft.sobrenome = sobrenome;
                    self.go(Step::Animal, assets);
                }
            }
            (Step::Animal, Button::Back) => {
                self.play(&[snd::BACK]);
                self.go(Step::Name, assets);
            }
            (Step::Animal, Button::Next) => {
                self.play(&[snd::NEXT]);
                self.draft.bicho = self.animal_form.selected.min(ANIMALS - 1);
                self.go(Step::Home, assets);
            }
            (Step::Home, _) => {
                self.play(&[snd::BACK]);
                self.go(Step::Animal, assets);
            }
            (_, Button::Back) => {
                self.play(&[snd::BACK]);
                self.go(Step::Home, assets);
            }
            (_, Button::Next) => match self.save(fx) {
                Ok(()) => {
                    self.play(&[snd::NEXT]);
                    self.go(Step::Thanks, assets);
                }
                Err(w) => {
                    self.play(&[snd::NEXT, snd::ALERT]);
                    self.go(Step::Warn(w), assets);
                }
            },
        }
    }

    /// Quadro-negro do passo da pasta: a pasta, "Coloque aqui", Procurar e setas.
    fn home_input(&mut self, i: &Input, fx: &mut Vec<Effect>, assets: &Assets) {
        let Input::Click {
            button: MouseButton::Left,
            x,
            y,
        } = *i
        else {
            return;
        };
        if inside(self.list_rows()[0], x, y) {
            if !self.home_selected {
                self.home_selected = true;
                self.play(&[snd::SELECT_HOME]);
            }
        } else if inside(pic_rect(BTN_PUT_HERE, 109, 32), x, y) {
            self.play(&[snd::ME]);
            if self.nick_form.fields[0].is_empty() {
                self.nick_form.fields[0] = users::suggest_nickname(&self.draft.nome);
            }
            self.go(Step::Nickname, assets);
        } else if inside(pic_rect(BTN_BROWSE, 140, 24), x, y) {
            fx.push(Effect::Log(
                "Procurar: não há discos para procurar no Ateliê Livre".into(),
            ));
        } else if inside(pic_rect(ARROW_UP, 12, 12), x, y)
            || inside(pic_rect(ARROW_DOWN, 12, 12), x, y)
        {
            self.play(&[snd::ARROW]);
        }
    }

    /// Avisos "EPA!": qualquer clique fecha, sem som.
    fn warn_input(&mut self, i: &Input, assets: &Assets) {
        let Step::Warn(w) = self.step else { return };
        if matches!(
            i,
            Input::Click {
                button: MouseButton::Left,
                ..
            }
        ) {
            self.go(w.under(), assets);
        }
    }

    /// Balão de saída: só as opções respondem; fora dele, a vinheta de recusa.
    fn quit_input(&mut self, i: &Input, fx: &mut Vec<Effect>, assets: &Assets) {
        let Input::Click {
            button: MouseButton::Left,
            x,
            y,
        } = *i
        else {
            return;
        };
        let Some(b) = &self.quit else { return };
        match b.hit_option(assets, x, y) {
            Some(0) => {
                self.play(&[snd::CHOICE]);
                fx.push(Effect::Quit);
            }
            Some(_) => {
                self.play(&[snd::CHOICE]);
                let back = self.quit_from;
                self.go(back, assets);
            }
            None => {
                if !b.contains(assets, x, y) {
                    self.play(&[snd::REFUSE]);
                }
            }
        }
    }

    /// O quadro-negro aparece na lista, no aviso sobre ela e no passo da pasta.
    fn board_visible(&self, step: Step) -> bool {
        match step {
            Step::List | Step::Warn(Warning::NoLogin) | Step::Home => true,
            Step::Quit => self.board_visible(self.quit_from),
            _ => false,
        }
    }

    fn draw_board(&self, frame: &mut Frame, assets: &Assets) {
        let (bx, by, bw, bh) = BOARD;
        frame.rect(bx, by, bw, bh, [0, 0, 0]);
        frame.rect(bx + 1, by + 1, bw - 2, bh - 2, [185, 101, 36]);
        frame.rect(bx + 2, by + 2, bw - 4, bh - 4, [255, 140, 50]);
        frame.rect(bx + 4, by + 4, bw - 8, bh - 8, [0, 0, 0]);
        frame.rect(bx + 5, by + 5, bw - 10, bh - 10, [71, 140, 80]);
        let (lx, ly, lw, lh) = LIST;
        frame.rect(lx, ly, lw, lh, [0, 0, 0]);
        frame.rect(lx + 1, ly + 1, lw - 2, lh - 2, [255, 255, 255]);
        let font = balloon_font(assets);
        let home =
            self.step == Step::Home || (self.step == Step::Quit && self.quit_from == Step::Home);
        let folder = self.folder_name();
        // no passo da pasta a lista mostra as pastas possíveis (só a de dados)
        let rows: Vec<(String, bool)> = if home {
            vec![(folder.clone(), self.home_selected)]
        } else {
            draw_line(frame, font.as_deref(), lx + 4, ly + 4, &folder);
            self.users
                .iter()
                .enumerate()
                .skip(self.scroll)
                .take(ROWS)
                .map(|(k, u)| (u.clone(), self.selected == Some(k)))
                .collect()
        };
        for (k, r) in self.list_rows().iter().enumerate() {
            frame.rect(r.0, r.1 - 1, r.2, 1, [0, 0, 0]);
            let Some((name, sel)) = rows.get(k) else {
                continue;
            };
            if *sel {
                frame.rect(r.0, r.1, r.2, r.3, [0, 0, 0]);
                match &font {
                    Some(fo) => {
                        frame.text_font(fo, r.0 + 3, r.1 + 3, name, [255, 255, 255]);
                    }
                    None => frame.text(r.0 + 3, r.1 + 3, name, [255, 255, 255]),
                }
            } else {
                draw_line(frame, font.as_deref(), r.0 + 3, r.1 + 3, name);
            }
        }
        for p in [ARROW_UP, ARROW_DOWN] {
            blit_pic(frame, assets, p.0, p.1, p.2, true);
        }
        let buttons: &[(u16, i32, i32)] = if home {
            &[BTN_PUT_HERE, BTN_BROWSE]
        } else {
            &[BTN_ME, BTN_FIRST, BTN_BROWSE]
        };
        for p in buttons {
            blit_pic(frame, assets, p.0, p.1, p.2, true);
        }
    }

    /// Balão principal de um passo (sem os avisos e o balão de saída).
    fn draw_step(&self, step: Step, frame: &mut Frame, assets: &Assets) {
        match step {
            Step::List | Step::Warn(Warning::NoLogin) => self.intro.draw(frame, assets),
            Step::Name | Step::Warn(Warning::BadName) => self.name_form.draw(frame, assets),
            Step::Animal => self.animal_form.draw(frame, assets),
            Step::Home => self.home_form.draw(frame, assets),
            Step::Nickname | Step::Warn(_) => self.nick_form.draw(frame, assets),
            Step::Welcome | Step::Thanks => {
                if let Some(b) = &self.options {
                    b.draw(frame, assets);
                }
            }
            Step::Quit => self.draw_step(self.quit_from, frame, assets),
        }
    }
}

impl Scene for Entry {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        if let Some(u) = self.preset.take() {
            self.finish(&u, SceneId::Imaginopolis, &mut fx);
            return fx;
        }
        if !self.opened && !self.done {
            self.opened = true;
            self.play(&snd::OPENING);
        }
        for i in input {
            if self.done {
                break;
            }
            match self.step {
                Step::List => self.list_input(i, &mut fx, assets),
                Step::Welcome => self.welcome_input(i, &mut fx, assets),
                Step::Thanks => self.thanks_input(i, &mut fx, assets),
                Step::Name | Step::Animal | Step::Home | Step::Nickname => {
                    self.form_input(i, &mut fx, assets)
                }
                Step::Warn(_) => self.warn_input(i, assets),
                Step::Quit => self.quit_input(i, &mut fx, assets),
            }
        }
        self.pump_sounds(dt, &mut fx, assets);
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        frame.fill([136, 136, 230]);
        let bg = assets.image(&ImageRef::Pic(Program::Artist, BACKGROUND));
        frame.blit(
            &bg,
            (SCREEN_W as i32 - bg.width as i32) / 2,
            (SCREEN_H as i32 - bg.height as i32) / 2,
        );
        blit_pic(frame, assets, MCZEE.0, MCZEE.1, MCZEE.2, true);
        if self.board_visible(self.step) {
            self.draw_board(frame, assets);
        }
        self.draw_step(self.step, frame, assets);
        if let Some(b) = &self.warning {
            b.draw(frame, assets);
        }
        if let Some(b) = &self.quit {
            b.draw(frame, assets);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn press(
        e: &mut Entry,
        a: &Assets,
        form: fn(&Entry) -> &TextBalloon,
        b: Button,
    ) -> Vec<Effect> {
        let r = form(e).button_rect(a, b).unwrap();
        e.update(&[click(r.0 + 2, r.1 + 2)], 0.0, a)
    }

    fn option(e: &mut Entry, a: &Assets, k: usize) -> Vec<Effect> {
        let r = e.options.as_ref().unwrap().option_rect(a, k).unwrap();
        e.update(&[click(r.0 + 20, r.1 + 3)], 0.0, a)
    }

    fn played(fx: &[Effect], id: u16) -> bool {
        fx.contains(&Effect::PlaySound(sound(id)))
    }

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|f| match f {
                Effect::PlaySound(SoundRef::Snd(_, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// Esvazia a fila de sons, devolvendo os ids na ordem.
    fn drain(e: &mut Entry, a: &Assets) -> Vec<u16> {
        let mut out = Vec::new();
        for _ in 0..8 {
            out.extend(sounds(&e.update(&[], 1.0, a)));
        }
        out
    }

    /// Sons de um clique: os imediatos e os que vêm na sequência.
    fn seq(e: &mut Entry, a: &Assets, i: Input) -> Vec<u16> {
        let mut out = sounds(&e.update(&[i], 0.0, a));
        out.extend(drain(e, a));
        out
    }

    /// Cadastro completo: nome, bicho, pasta e apelido.
    fn register(
        e: &mut Entry,
        a: &Assets,
        nome: &str,
        sobrenome: &str,
        apelido: &str,
    ) -> Vec<Effect> {
        e.update(&[click(BTN_FIRST.1 + 5, BTN_FIRST.2 + 5)], 0.0, a);
        assert_eq!(e.step_name(), "nome");
        e.update(&[Input::Text(nome.into())], 0.0, a);
        let r1 = e.name_form.field_rect(a, 1).unwrap();
        e.update(
            &[click(r1.0 + 3, r1.1 + 3), Input::Text(sobrenome.into())],
            0.0,
            a,
        );
        press(e, a, |e| &e.name_form, Button::Next);
        assert_eq!(e.step_name(), "bicho");
        let c = e.animal_form.cell_rect(a, 5).unwrap();
        e.update(&[click(c.0 + 3, c.1 + 3)], 0.0, a);
        press(e, a, |e| &e.animal_form, Button::Next);
        assert_eq!(e.step_name(), "pasta");
        e.update(&[click(BTN_PUT_HERE.1 + 5, BTN_PUT_HERE.2 + 5)], 0.0, a);
        assert_eq!(e.step_name(), "apelido");
        e.nick_form.fields[0].clear();
        e.update(&[Input::Text(apelido.into())], 0.0, a);
        press(e, a, |e| &e.nick_form, Button::Next)
    }

    #[test]
    fn registration_saves_profile_and_lists_it_next_time() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        assert!(e.users().is_empty());
        register(&mut e, &a, "Ana", "Teste", "aninha");
        assert_eq!(e.step_name(), "obrigado");
        let p = users::load(dir.path(), "aninha").unwrap();
        assert_eq!(p.nome, "Ana");
        assert_eq!(p.sobrenome, "Teste");
        assert_eq!(p.bicho, 5);
        // "Não precisa, vamos direto para Imaginópolis"
        let fx = option(&mut e, &a, 1);
        assert!(fx.contains(&Effect::SetUser("aninha".into())));
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Imaginopolis)));
        // na próxima entrada o apelido está na lista; Sou eu abre as boas-vindas
        let mut e2 = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        assert_eq!(e2.users(), ["aninha".to_string()]);
        let row = e2.list_rows()[0];
        e2.update(&[click(row.0 + 5, row.1 + 5)], 0.0, &a);
        assert_eq!(e2.selected(), Some("aninha"));
        e2.update(&[click(BTN_ME.1 + 5, BTN_ME.2 + 5)], 0.0, &a);
        assert_eq!(e2.step_name(), "boas-vindas");
        // "Ir direto para o Ateliê de Pintura" abre a tela de pintura
        let fx = option(&mut e2, &a, 3);
        assert!(fx.contains(&Effect::SetUser("aninha".into())));
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        let mut f = Frame::new(640, 480);
        e2.draw(&mut f, &a);
    }

    #[test]
    fn nine_letter_nickname_is_refused() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        register(&mut e, &a, "Ana", "Teste", "abcdefghi");
        assert_eq!(e.step, Step::Warn(Warning::BadNickname));
        assert!(users::list(dir.path()).is_empty());
        // o aviso volta ao apelido, que continua digitado
        e.update(&[click(5, 5)], 0.0, &a);
        assert_eq!(e.step_name(), "apelido");
        assert_eq!(e.nick_form.fields[0], "abcdefghi");
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
    }

    #[test]
    fn duplicate_nickname_and_empty_name_warn() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        users::save(
            dir.path(),
            &Profile {
                apelido: "ana".into(),
                ..Profile::default()
            },
        )
        .unwrap();
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        register(&mut e, &a, "Ana", "Teste", "ana");
        assert_eq!(e.step, Step::Warn(Warning::Duplicate));
        // o aviso não tem botão: qualquer clique fecha
        assert!(e.warning.as_ref().unwrap().buttons.is_empty());
        assert!(e.warning.as_ref().unwrap().alert);
        e.update(&[click(300, 200)], 0.0, &a);
        assert_eq!(e.step_name(), "apelido");
        // nome vazio: Segue + vinheta
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        e.update(&[click(BTN_FIRST.1 + 5, BTN_FIRST.2 + 5)], 0.0, &a);
        let r = e.name_form.button_rect(&a, Button::Next).unwrap();
        let ids = seq(&mut e, &a, click(r.0 + 2, r.1 + 2));
        assert_eq!(e.step, Step::Warn(Warning::BadName));
        assert_eq!(ids, [snd::NEXT, snd::ALERT]);
        // Sou eu sem nome escolhido
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        let ids = seq(&mut e, &a, click(BTN_ME.1 + 5, BTN_ME.2 + 5));
        assert_eq!(e.step, Step::Warn(Warning::NoLogin));
        assert_eq!(ids, [snd::ME, snd::ALERT]);
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
        e.update(&[click(5, 5)], 0.0, &a);
        assert_eq!(e.step_name(), "lista");
    }

    #[test]
    fn preset_user_skips_to_imaginopolis() {
        let (_d, a) = empty_assets();
        let mut e = Entry::new(None, None, &a).with_user("zeca");
        let fx = e.update(&[], 0.0, &a);
        assert!(fx.contains(&Effect::SetUser("zeca".into())));
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Imaginopolis)));
        assert!(
            !e.update(&[], 0.0, &a)
                .iter()
                .any(|e| matches!(e, Effect::ChangeScene(_) | Effect::PlaySound(_)))
        );
    }

    #[test]
    fn opening_sounds_play_in_sequence() {
        let (_d, a) = empty_assets();
        let mut e = Entry::new(None, None, &a);
        let fx = e.update(&[], 0.0, &a);
        assert!(played(&fx, snd::OPENING[0]));
        assert!(!played(&fx, snd::OPENING[1]));
        assert_eq!(drain(&mut e, &a), &snd::OPENING[1..]);
        assert!(drain(&mut e, &a).is_empty());
    }

    #[test]
    fn stop_asks_before_quitting() {
        let (_d, a) = empty_assets();
        let mut e = Entry::new(None, None, &a);
        drain(&mut e, &a);
        let r = e.intro.button_rect(&a, Button::Stop).unwrap();
        let fx = e.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert!(!fx.contains(&Effect::Quit));
        assert_eq!(e.step_name(), "sair");
        assert!(played(&fx, snd::STOP));
        assert_eq!(drain(&mut e, &a), [snd::ALERT]);
        let q = e.quit.as_ref().unwrap();
        assert!(q.alert);
        assert_eq!(q.options().len(), 2);
        let r1 = q.option_rect(&a, 1).unwrap();
        // clique fora: recusa, o balão fica
        let fx = e.update(&[click(5, 5)], 0.0, &a);
        assert!(played(&fx, snd::REFUSE));
        assert_eq!(e.step_name(), "sair");
        // "Continuar trabalhando" volta à lista com o quadro-negro
        let fx = e.update(&[click(r1.0 + 5, r1.1 + 3)], 0.0, &a);
        assert!(played(&fx, snd::CHOICE));
        assert!(!fx.contains(&Effect::Quit));
        assert_eq!(e.step_name(), "lista");
        assert!(e.board_visible(e.step));
        // "Sair do Fine Artist" sai
        let r = e.intro.button_rect(&a, Button::Stop).unwrap();
        e.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
        let r0 = e.quit.as_ref().unwrap().option_rect(&a, 0).unwrap();
        let fx = e.update(&[click(r0.0 + 5, r0.1 + 3)], 0.0, &a);
        assert!(fx.contains(&Effect::Quit));
    }

    #[test]
    fn browse_reloads_and_board_buttons_make_sounds() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        users::save(
            dir.path(),
            &Profile {
                apelido: "ana".into(),
                ..Profile::default()
            },
        )
        .unwrap();
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        drain(&mut e, &a);
        let row = e.list_rows()[0];
        let fx = e.update(&[click(row.0 + 5, row.1 + 5)], 0.0, &a);
        assert!(played(&fx, snd::SELECT));
        // segundo clique no mesmo nome: nada
        let fx = e.update(&[click(row.0 + 5, row.1 + 5)], 0.0, &a);
        assert!(!played(&fx, snd::SELECT));
        // Procurar: fala, relê a lista (com o usuário novo) e perde a escolha
        users::save(
            dir.path(),
            &Profile {
                apelido: "bia".into(),
                ..Profile::default()
            },
        )
        .unwrap();
        let fx = e.update(&[click(BTN_BROWSE.1 + 5, BTN_BROWSE.2 + 5)], 0.0, &a);
        assert!(played(&fx, snd::BROWSE));
        assert!(fx.iter().any(|e| matches!(e, Effect::Log(_))));
        assert_eq!(e.users(), ["ana", "bia"]);
        assert_eq!(e.selected(), None);
        assert_eq!(e.step_name(), "lista");
        drain(&mut e, &a);
        let fx = e.update(&[click(ARROW_UP.1 + 3, ARROW_UP.2 + 3)], 0.0, &a);
        assert!(played(&fx, snd::ARROW));
        let fx = e.update(&[click(500, 250)], 0.0, &a);
        assert!(played(&fx, snd::MCZEE));
        let fx = e.update(&[click(BTN_FIRST.1 + 5, BTN_FIRST.2 + 5)], 0.0, &a);
        assert!(played(&fx, snd::FIRST));
        assert_eq!(e.step_name(), "nome");
    }

    #[test]
    fn welcome_is_modal_with_back_and_stop() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        users::save(
            dir.path(),
            &Profile {
                apelido: "ana".into(),
                ..Profile::default()
            },
        )
        .unwrap();
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        drain(&mut e, &a);
        let row = e.list_rows()[0];
        e.update(&[click(row.0 + 5, row.1 + 5)], 0.0, &a);
        drain(&mut e, &a);
        let fx = e.update(&[click(BTN_ME.1 + 5, BTN_ME.2 + 5)], 0.0, &a);
        assert!(played(&fx, snd::ME));
        assert_eq!(e.step_name(), "boas-vindas");
        assert!(!e.board_visible(e.step));
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
        // fora do balão: recusa, continua aberto
        let fx = e.update(&[click(300, 400)], 0.0, &a);
        assert!(played(&fx, snd::REFUSE));
        assert_eq!(e.step_name(), "boas-vindas");
        // Para: balão de saída por cima; Continuar volta às boas-vindas
        press(&mut e, &a, |e| e.options.as_ref().unwrap(), Button::Stop);
        assert_eq!(e.step_name(), "sair");
        assert!(!e.board_visible(e.step));
        assert!(e.options.is_some(), "as boas-vindas ficam por baixo");
        e.draw(&mut f, &a);
        let r1 = e.quit.as_ref().unwrap().option_rect(&a, 1).unwrap();
        e.update(&[click(r1.0 + 5, r1.1 + 3)], 0.0, &a);
        assert_eq!(e.step_name(), "boas-vindas");
        assert_eq!(e.selected(), Some("ana"));
        // Volta: lista sem nome escolhido
        let fx = press(&mut e, &a, |e| e.options.as_ref().unwrap(), Button::Back);
        assert!(played(&fx, snd::BACK));
        assert_eq!(e.step_name(), "lista");
        assert_eq!(e.selected(), None);
    }

    #[test]
    fn keys_move_selection_and_edit_keeps_profile() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        for n in ["ana", "bia", "caio"] {
            users::save(
                dir.path(),
                &Profile {
                    nome: n.to_uppercase(),
                    sobrenome: "X".into(),
                    apelido: n.into(),
                    bicho: 1,
                },
            )
            .unwrap();
        }
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        let down = Input::Key {
            key: Key::Down,
            ctrl: false,
            alt: false,
        };
        e.update(&[down.clone(), down.clone()], 0.0, &a);
        assert_eq!(e.selected(), Some("bia"));
        e.update(
            &[Input::Key {
                key: Key::Enter,
                ctrl: false,
                alt: false,
            }],
            0.0,
            &a,
        );
        assert_eq!(e.step_name(), "boas-vindas");
        // "Mudar o que você sabe sobre mim": campos preenchidos; troca o apelido
        let fx = option(&mut e, &a, 1);
        assert!(played(&fx, snd::CHOICE));
        assert_eq!(e.step_name(), "nome");
        assert_eq!(e.name_form.fields[0], "BIA");
        press(&mut e, &a, |e| &e.name_form, Button::Next);
        assert_eq!(e.animal_form.selected, 1);
        press(&mut e, &a, |e| &e.animal_form, Button::Next);
        assert_eq!(e.step_name(), "pasta");
        // a pasta: escolher a linha faz o tique; Volta vai ao bicho
        let row = e.list_rows()[0];
        drain(&mut e, &a);
        let fx = e.update(&[click(row.0 + 5, row.1 + 5)], 0.0, &a);
        assert!(played(&fx, snd::SELECT_HOME));
        assert!(e.home_selected);
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
        press(&mut e, &a, |e| &e.home_form, Button::Back);
        assert_eq!(e.step_name(), "bicho");
        press(&mut e, &a, |e| &e.animal_form, Button::Next);
        e.update(&[click(BTN_PUT_HERE.1 + 5, BTN_PUT_HERE.2 + 5)], 0.0, &a);
        assert_eq!(e.step_name(), "apelido");
        // o apelido vem selecionado: digitar substitui
        assert_eq!(e.nick_form.fields[0], "bia");
        assert!(e.nick_form.select_all);
        e.update(&[Input::Text("bia2".into())], 0.0, &a);
        assert_eq!(e.nick_form.fields[0], "bia2");
        // Volta do apelido vai à pasta
        press(&mut e, &a, |e| &e.nick_form, Button::Back);
        assert_eq!(e.step_name(), "pasta");
        e.update(&[click(BTN_PUT_HERE.1 + 5, BTN_PUT_HERE.2 + 5)], 0.0, &a);
        assert_eq!(e.nick_form.fields[0], "bia2");
        press(&mut e, &a, |e| &e.nick_form, Button::Next);
        assert_eq!(e.step_name(), "obrigado");
        assert_eq!(users::list(dir.path()), ["ana", "bia2", "caio"]);
        assert_eq!(users::load(dir.path(), "bia2").unwrap().nome, "BIA");
        // "Quero mudar o que você sabe sobre mim" volta ao nome; Volta ao obrigado
        option(&mut e, &a, 2);
        assert_eq!(e.step_name(), "nome");
        press(&mut e, &a, |e| &e.name_form, Button::Back);
        assert_eq!(e.step_name(), "obrigado");
        // Para no obrigado pergunta; Para no formulário também
        press(&mut e, &a, |e| e.options.as_ref().unwrap(), Button::Stop);
        assert_eq!(e.step_name(), "sair");
        let r1 = e.quit.as_ref().unwrap().option_rect(&a, 1).unwrap();
        e.update(&[click(r1.0 + 5, r1.1 + 3)], 0.0, &a);
        assert_eq!(e.step_name(), "obrigado");
        option(&mut e, &a, 2);
        press(&mut e, &a, |e| &e.name_form, Button::Stop);
        assert_eq!(e.step_name(), "sair");
        e.draw(&mut f, &a);
    }

    #[test]
    fn balloons_come_from_what_with_real_data() {
        let Some(a) = assets() else { return };
        let dir = tempfile::tempdir().unwrap();
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        assert!(matches!(&e.intro.parts[0], Part::Text(t) if t.starts_with("Oi, eu sou o McZee!")));
        // o balão de saída e os avisos vêm do WHAT, com a arte "EPA!"
        let q = Entry::quit_balloon(&a);
        assert_eq!(q.parts[0], Part::Figure(PIC_ALERT));
        assert_eq!(q.options()[0], "Sair do Fine Artist");
        assert!(q.alert && q.buttons.is_empty());
        let h = Entry::home_balloon(&a);
        assert!(matches!(&h.parts[0], Part::Text(t) if t.starts_with("Onde você quer")));
        assert!(
            !h.parts
                .iter()
                .any(|p| matches!(p, Part::Text(t) if t.contains("disquete")))
        );
        register(&mut e, &a, "Ana", "Teste", "ana");
        let o = e.options.as_ref().unwrap();
        assert!(matches!(&o.parts[0], Part::Text(t) if t.contains("ana")));
        assert_eq!(o.options().len(), 3);
        assert_eq!(o.buttons, [Button::Stop]);
        let mut f = Frame::new(640, 480);
        e.draw(&mut f, &a);
        let e2 = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        e2.draw(&mut f, &a);
        // o McZee de livro na mão está à direita (camisa laranja)
        let orange = (240..290)
            .flat_map(|y| (480..540).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let p = f.pixel(x, y);
                p[0] > 200 && p[1] > 100 && p[2] < 120
            })
            .count();
        assert!(orange > 100, "{orange}");
        // a lista mostra "ana" em preto sobre branco
        let row = e2.list_rows()[0];
        assert_eq!(
            f.pixel((row.0 + 60) as u32, (row.1 + 5) as u32)[..3],
            [255, 255, 255]
        );
        // as boas-vindas do WHAT têm as quatro opções e Volta/Para
        let w = Entry::welcome_balloon(&a, "Ana");
        assert_eq!(w.options().len(), 4);
        assert_eq!(w.buttons, [Button::Back, Button::Stop]);
    }
}
