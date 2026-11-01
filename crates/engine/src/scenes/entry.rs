//! Entrada do Fine Artist: o McZee de livro na mão sobre as nuvens (fundo
//! `4139`), o quadro-negro com a lista de usuários e os botões Sou eu /
//! Primeira vez / Procurar; o cadastro (nome, bicho preferido, apelido) em
//! balões sucessivos do `WHAT 4400`, com os avisos do `WHAT 4402`. Posições
//! medidas nas capturas do original.

use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::{Balloon, balloon_font, draw_line};
use crate::scene::{Effect, Input, Key, KidCommon, MouseButton, Scene, SceneId};
use crate::text_balloon::{Button, PIC_ANIMAL0, PIC_MCZEE_HEAD, Part, TextBalloon, blit_pic};
use crate::users::{self, ANIMALS, Profile};
use crate::{Frame, SCREEN_H, SCREEN_W};
use formats::what::Record;
use std::path::PathBuf;

pub const BACKGROUND: u16 = 0x4139;
/// McZee de livro na mão (123×309; 1 bit e máscara são as duas seguintes).
pub const MCZEE: (u16, i32, i32) = (0x42f9, 435, 112);
/// Botões do quadro-negro (figura colorida; a máscara é duas adiante).
pub const BTN_ME: (u16, i32, i32) = (0x473, 280, 215);
pub const BTN_FIRST: (u16, i32, i32) = (0x477, 280, 253);
pub const BTN_BROWSE: (u16, i32, i32) = (0x47b, 280, 288);
/// Setas da lista (12×12).
pub const ARROW_UP: (u16, i32, i32) = (0x2db, 169, 343);
pub const ARROW_DOWN: (u16, i32, i32) = (0x2df, 220, 343);
/// Quadro-negro (moldura externa) e caixa branca da lista.
const BOARD: Rect = (120, 170, 311, 200);
const LIST: Rect = (130, 179, 142, 163);
const ROW_H: i32 = 18;
/// Linhas visíveis de usuários (a primeira linha é o caminho da pasta).
const ROWS: usize = 8;
/// Blocos `WHAT` da entrada e dos avisos.
const ENTRY_WHAT: u16 = 0x4400;
const ERRORS_WHAT: u16 = 0x4402;
/// Posição dos balões do McZee: o grande (lista e boas-vindas) e os do cadastro.
const BIG: (i32, i32) = (84, 24);
const FORM: (i32, i32) = (112, 78);

type Rect = (i32, i32, i32, i32);

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn pic_rect(p: (u16, i32, i32), w: i32, h: i32) -> Rect {
    (p.1, p.2, w, h)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    List,
    Welcome,
    Name,
    Animal,
    Nickname,
    Thanks,
    Warn(Warning),
}

pub struct Entry {
    common: KidCommon,
    dir: Option<PathBuf>,
    users: Vec<String>,
    selected: Option<usize>,
    scroll: usize,
    step: Step,
    /// Balão da lista (`4400[1]`, com o botão Para).
    intro: TextBalloon,
    /// Balões com opções: boas-vindas (`4400[2]`) e agradecimento (`4400[3]`).
    options: Option<Balloon>,
    name_form: TextBalloon,
    animal_form: TextBalloon,
    nick_form: TextBalloon,
    warning: Option<TextBalloon>,
    draft: Profile,
    /// Apelido original quando se altera um cadastro existente.
    editing: Option<String>,
    /// Usuário dado na configuração: pula a entrada.
    preset: Option<String>,
    done: bool,
}

/// Balão com opções do `WHAT`, se o registro existir com o número esperado
/// de opções; senão o reserva.
fn options_balloon(assets: &Assets, k: usize, name: &str, fallback: Balloon) -> Balloon {
    match assets.dialogs(Program::Artist, ENTRY_WHAT).get(k) {
        Some(r) if r.options().len() == fallback.options.len() => {
            Balloon::from_record(r, fallback.x, fallback.y, name)
        }
        _ => fallback,
    }
}

fn record(assets: &Assets, block: u16, k: usize) -> Option<Record> {
    assets.dialogs(Program::Artist, block).into_iter().nth(k)
}

fn form_balloon(
    assets: &Assets,
    block: u16,
    k: usize,
    name: &str,
    buttons: &[Button],
    fallback: TextBalloon,
) -> TextBalloon {
    match record(assets, block, k) {
        Some(r) => {
            let b = TextBalloon::from_record(&r, fallback.x, fallback.y, name, buttons.to_vec());
            if b.fields.len() == fallback.fields.len() {
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
            intro: Self::intro_balloon(assets),
            options: None,
            name_form: Self::name_balloon(assets),
            animal_form: Self::animal_balloon(assets, ""),
            nick_form: Self::nick_balloon(assets),
            warning: None,
            draft: Profile::default(),
            editing: None,
            preset: None,
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
            Step::Nickname => "apelido",
            Step::Thanks => "obrigado",
            Step::Warn(_) => "aviso",
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
        form_balloon(assets, ENTRY_WHAT, 1, "", &[Button::Stop], fallback)
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
        form_balloon(
            assets,
            ENTRY_WHAT,
            4,
            "",
            &[Button::Back, Button::Stop, Button::Next],
            fallback,
        )
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
        form_balloon(
            assets,
            ENTRY_WHAT,
            8,
            name,
            &[Button::Back, Button::Stop, Button::Next],
            fallback,
        )
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
        form_balloon(
            assets,
            ENTRY_WHAT,
            5,
            "",
            &[Button::Back, Button::Stop, Button::Next],
            fallback,
        )
    }

    fn welcome_balloon(assets: &Assets, name: &str) -> Balloon {
        let fallback = Balloon {
            x: BIG.0,
            y: BIG.1,
            w: 344,
            text: format!("{name}! Que bom ver você de novo! O que você quer fazer?"),
            options: [
                "Ir direto para Imaginópolis",
                "Mudar o que você sabe sobre mim",
                "Conte mais sobre o seu mundo",
                "Ir direto para o Ateliê de Pintura",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        };
        options_balloon(assets, 2, name, fallback)
    }

    fn thanks_balloon(assets: &Assets, name: &str) -> Balloon {
        let fallback = Balloon {
            x: BIG.0,
            y: BIG.1,
            w: 340,
            text: format!(
                "Obrigado por ter me contado coisas sobre você, {name}. Você quer saber um pouco mais sobre mim?"
            ),
            options: [
                "É, por favor conte mais",
                "Não precisa, vamos direto para Imaginópolis",
                "Quero mudar o que você sabe sobre mim",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        };
        options_balloon(assets, 3, name, fallback)
    }

    fn warning_balloon(assets: &Assets, w: Warning, name: &str) -> TextBalloon {
        let (k, msg, width) = match w {
            Warning::Duplicate => (
                0,
                format!(
                    "Ops! Você não pode usar esse nome!\nEsse apelido ({name}) já está sendo usado por outra pessoa. Mude o seu apelido um pouquinho — use as suas iniciais ou junte a primeira letra do seu sobrenome."
                ),
                258,
            ),
            Warning::BadName => (
                1,
                "Tem certeza de que esse é o seu nome?\nPara mim, isso não parece um nome. Vamos tentar de novo? Digite o seu nome — você pode usar letras ou números. Não deixe de digitar coisas nas duas caixas.".to_string(),
                230,
            ),
            Warning::BadNickname => (
                2,
                "Experimente outro apelido.\nEu preciso de um apelido para você — digite um nome com 8 caracteres ou menos, letras ou números, sem espaços e sem pontuação.".to_string(),
                212,
            ),
            Warning::NoLogin => (
                3,
                "Eu preciso saber o seu nome.\nClique primeiro no seu nome que aparece na lista! Depois, clique em Sou eu.".to_string(),
                213,
            ),
        };
        let fallback = TextBalloon::new(
            FORM.0,
            FORM.1,
            width,
            vec![Part::Figure(PIC_MCZEE_HEAD), Part::Text(msg)],
            vec![Button::Next],
        );
        form_balloon(assets, ERRORS_WHAT, k, name, &[Button::Next], fallback)
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

    fn go(&mut self, step: Step, assets: &Assets) {
        self.step = step;
        self.options = None;
        self.warning = None;
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
            Step::Animal => {
                let sel = self.animal_form.selected;
                self.animal_form = Self::animal_balloon(assets, &self.draft.nome);
                self.animal_form.selected = sel.min(ANIMALS - 1);
            }
            _ => {}
        }
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

    fn start_registration(&mut self, prefill: Option<Profile>, assets: &Assets) {
        self.editing = prefill.as_ref().map(|p| p.apelido.clone());
        self.draft = prefill.unwrap_or_default();
        self.name_form = Self::name_balloon(assets);
        self.name_form.fields[0] = self.draft.nome.clone();
        self.name_form.fields[1] = self.draft.sobrenome.clone();
        self.nick_form = Self::nick_balloon(assets);
        self.nick_form.fields[0] = self.draft.apelido.clone();
        self.animal_form.selected = self.draft.bicho;
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

    fn list_input(&mut self, i: &Input, fx: &mut Vec<Effect>, assets: &Assets) {
        if let Some(Button::Stop) = self.intro.handle(assets, i) {
            fx.push(Effect::Quit);
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
                    if idx < self.users.len() {
                        self.selected = Some(idx);
                    }
                } else if inside(pic_rect(BTN_ME, 114, 21), x, y) {
                    if self.selected.is_some() {
                        self.go(Step::Welcome, assets);
                    } else {
                        self.go(Step::Warn(Warning::NoLogin), assets);
                    }
                } else if inside(pic_rect(BTN_FIRST, 143, 20), x, y) {
                    self.start_registration(None, assets);
                } else if inside(pic_rect(BTN_BROWSE, 140, 24), x, y) {
                    fx.push(Effect::Log(
                        "Procurar: não há discos para procurar no Ateliê Livre".into(),
                    ));
                } else if inside(pic_rect(ARROW_UP, 12, 12), x, y) {
                    self.scroll = self.scroll.saturating_sub(1);
                } else if inside(pic_rect(ARROW_DOWN, 12, 12), x, y)
                    && self.scroll + ROWS < self.users.len()
                {
                    self.scroll += 1;
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
            } if self.selected.is_some() => self.go(Step::Welcome, assets),
            _ => {}
        }
    }

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
        let (hit, inside_balloon) = (b.hit(assets, x, y), b.contains(assets, x, y));
        let user = self.selected().unwrap_or("artista").to_string();
        match hit {
            Some(0) => self.finish(&user, SceneId::Imaginopolis, fx),
            Some(1) => {
                let prof = self
                    .dir
                    .as_deref()
                    .and_then(|d| users::load(d, &user))
                    .unwrap_or(Profile {
                        apelido: user.clone(),
                        ..Profile::default()
                    });
                self.start_registration(Some(prof), assets);
            }
            Some(2) => self.finish(&user, SceneId::Story, fx),
            Some(_) => self.finish(&user, SceneId::Studio, fx),
            None => {
                if !inside_balloon {
                    self.go(Step::List, assets);
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
        let hit = b.hit(assets, x, y);
        let user = self.draft.apelido.clone();
        match hit {
            Some(0) => self.finish(&user, SceneId::Story, fx),
            Some(1) => self.finish(&user, SceneId::Imaginopolis, fx),
            Some(_) => {
                let prof = self.draft.clone();
                self.start_registration(Some(prof), assets);
            }
            None => {}
        }
    }

    fn form_input(&mut self, i: &Input, fx: &mut Vec<Effect>, assets: &Assets) {
        let step = self.step;
        let form = match step {
            Step::Name => &mut self.name_form,
            Step::Animal => &mut self.animal_form,
            _ => &mut self.nick_form,
        };
        let Some(b) = form.handle(assets, i) else {
            return;
        };
        match (step, b) {
            (_, Button::Stop) => self.go(Step::List, assets),
            (Step::Name, Button::Back) => self.go(Step::List, assets),
            (Step::Name, Button::Next) => {
                let nome = self.name_form.fields[0].trim().to_string();
                let sobrenome = self.name_form.fields[1].trim().to_string();
                if nome.is_empty() || sobrenome.is_empty() {
                    self.go(Step::Warn(Warning::BadName), assets);
                } else {
                    self.draft.nome = nome;
                    self.draft.sobrenome = sobrenome;
                    self.go(Step::Animal, assets);
                }
            }
            (Step::Animal, Button::Back) => self.go(Step::Name, assets),
            (Step::Animal, Button::Next) => {
                self.draft.bicho = self.animal_form.selected.min(ANIMALS - 1);
                if self.nick_form.fields[0].is_empty() {
                    self.nick_form.fields[0] = users::suggest_nickname(&self.draft.nome);
                }
                self.go(Step::Nickname, assets);
            }
            (_, Button::Back) => self.go(Step::Animal, assets),
            (_, Button::Next) => match self.save(fx) {
                Ok(()) => self.go(Step::Thanks, assets),
                Err(w) => self.go(Step::Warn(w), assets),
            },
        }
    }

    fn warn_input(&mut self, i: &Input, assets: &Assets) {
        let Step::Warn(w) = self.step else { return };
        let Some(b) = &mut self.warning else { return };
        let clicked = matches!(
            i,
            Input::Click {
                button: MouseButton::Left,
                ..
            }
        );
        if b.handle(assets, i) == Some(Button::Next) || clicked {
            let back = match w {
                Warning::Duplicate | Warning::BadNickname => Step::Nickname,
                Warning::BadName => Step::Name,
                Warning::NoLogin => Step::List,
            };
            self.go(back, assets);
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
        let folder = self
            .dir
            .as_ref()
            .and_then(|d| d.file_name())
            .and_then(|n| n.to_str())
            .map_or("(sem pasta)".to_string(), |n| format!("{n}/"));
        draw_line(frame, font.as_deref(), lx + 4, ly + 4, &folder);
        for (k, r) in self.list_rows().iter().enumerate() {
            frame.rect(r.0, r.1 - 1, r.2, 1, [0, 0, 0]);
            let idx = self.scroll + k;
            let Some(name) = self.users.get(idx) else {
                continue;
            };
            if self.selected == Some(idx) {
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
        for p in [BTN_ME, BTN_FIRST, BTN_BROWSE] {
            blit_pic(frame, assets, p.0, p.1, p.2, true);
        }
    }
}

impl Scene for Entry {
    fn update(&mut self, input: &[Input], _dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        if let Some(u) = self.preset.take() {
            self.finish(&u, SceneId::Imaginopolis, &mut fx);
            return fx;
        }
        for i in input {
            if self.done {
                break;
            }
            match self.step {
                Step::List => self.list_input(i, &mut fx, assets),
                Step::Welcome => self.welcome_input(i, &mut fx, assets),
                Step::Thanks => self.thanks_input(i, &mut fx, assets),
                Step::Name | Step::Animal | Step::Nickname => self.form_input(i, &mut fx, assets),
                Step::Warn(_) => self.warn_input(i, assets),
            }
        }
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
        match self.step {
            Step::List | Step::Welcome | Step::Warn(Warning::NoLogin) => {
                self.draw_board(frame, assets);
            }
            _ => {}
        }
        match self.step {
            Step::List => self.intro.draw(frame, assets),
            Step::Name => self.name_form.draw(frame, assets),
            Step::Animal => self.animal_form.draw(frame, assets),
            Step::Nickname => self.nick_form.draw(frame, assets),
            Step::Welcome | Step::Thanks => {
                if let Some(b) = &self.options {
                    b.draw(frame, assets);
                }
            }
            Step::Warn(_) => {
                if let Some(b) = &self.warning {
                    b.draw(frame, assets);
                }
            }
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
        let r = e.options.as_ref().unwrap().option_rect(a, k);
        e.update(&[click(r.0 + 20, r.1 + 3)], 0.0, a)
    }

    /// Cadastro completo: nome, bicho e apelido.
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
        let fx = option(&mut e2, &a, 3);
        assert!(fx.contains(&Effect::SetUser("aninha".into())));
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Studio)));
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
        press(&mut e, &a, |e| e.warning.as_ref().unwrap(), Button::Next);
        assert_eq!(e.step_name(), "apelido");
        // nome vazio
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        e.update(&[click(BTN_FIRST.1 + 5, BTN_FIRST.2 + 5)], 0.0, &a);
        press(&mut e, &a, |e| &e.name_form, Button::Next);
        assert_eq!(e.step, Step::Warn(Warning::BadName));
        // Sou eu sem nome escolhido
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        e.update(&[click(BTN_ME.1 + 5, BTN_ME.2 + 5)], 0.0, &a);
        assert_eq!(e.step, Step::Warn(Warning::NoLogin));
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
                .any(|e| matches!(e, Effect::ChangeScene(_)))
        );
    }

    #[test]
    fn stop_quits_and_browse_only_logs() {
        let (_d, a) = empty_assets();
        let mut e = Entry::new(None, None, &a);
        let fx = e.update(&[click(BTN_BROWSE.1 + 5, BTN_BROWSE.2 + 5)], 0.0, &a);
        assert!(fx.iter().any(|e| matches!(e, Effect::Log(_))));
        assert_eq!(e.step_name(), "lista");
        let r = e.intro.button_rect(&a, Button::Stop).unwrap();
        let fx = e.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert!(fx.contains(&Effect::Quit));
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
        option(&mut e, &a, 1);
        assert_eq!(e.step_name(), "nome");
        assert_eq!(e.name_form.fields[0], "BIA");
        press(&mut e, &a, |e| &e.name_form, Button::Next);
        assert_eq!(e.animal_form.selected, 1);
        press(&mut e, &a, |e| &e.animal_form, Button::Next);
        assert_eq!(e.nick_form.fields[0], "bia");
        e.update(&[Input::Text("2".into())], 0.0, &a);
        press(&mut e, &a, |e| &e.nick_form, Button::Next);
        assert_eq!(e.step_name(), "obrigado");
        assert_eq!(users::list(dir.path()), ["ana", "bia2", "caio"]);
        assert_eq!(users::load(dir.path(), "bia2").unwrap().nome, "BIA");
        // "Quero mudar o que você sabe sobre mim" volta ao nome
        option(&mut e, &a, 2);
        assert_eq!(e.step_name(), "nome");
        press(&mut e, &a, |e| &e.name_form, Button::Stop);
        assert_eq!(e.step_name(), "lista");
    }

    #[test]
    fn balloons_come_from_what_with_real_data() {
        let Some(a) = assets() else { return };
        let dir = tempfile::tempdir().unwrap();
        let mut e = Entry::new(Some(dir.path().to_path_buf()), None, &a);
        assert!(matches!(&e.intro.parts[0], Part::Text(t) if t.starts_with("Oi, eu sou o McZee!")));
        register(&mut e, &a, "Ana", "Teste", "ana");
        assert!(e.options.as_ref().unwrap().text.contains("ana"));
        assert_eq!(e.options.as_ref().unwrap().options.len(), 3);
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
    }
}
