//! Diretor de cenas para os testes de jogo: a mesma pilha de cenas e o mesmo
//! tratamento de efeitos do laço da plataforma (`crates/app/src/run.rs`), sem
//! janela nem som, com as pastas do usuário num diretório temporário.
//!
//! `make` espelha o `make_scene` do app; se uma cena nova entrar lá, ela
//! precisa entrar aqui também (o `match` sem `_` obriga).

#![allow(dead_code)]

use engine::Frame;
use engine::assets::{Assets, CursorRef, Program};
use engine::scene::{CwScene, Effect, Input, Key, MouseButton, Scene, SceneId};
use engine::scenes::room::DEFAULT_USER;
use engine::scenes::tricks_board::TricksBoard;
use engine::scenes::{
    Browser, Chooser, Entry, Floor, GalleryRoom, Imaginopolis, Paint, Room, Splash, Story, Street,
    Writer,
};
use std::path::{Path, PathBuf};

/// Assets reais, se `ATELIE_DATA` estiver definida.
pub fn real_assets() -> Option<Assets> {
    match std::env::var_os("ATELIE_DATA") {
        Some(d) => Some(Assets::open(Path::new(&d)).expect("ATELIE_DATA válida")),
        None => {
            eprintln!("aviso: ATELIE_DATA não definida; teste de jogo pulado");
            None
        }
    }
}

pub const DT: f32 = 1.0 / 60.0;

pub struct Director<'a> {
    pub assets: &'a Assets,
    /// Raiz do `XDG_DATA_HOME` do teste.
    pub root: PathBuf,
    pub stack: Vec<Box<dyn Scene>>,
    /// Ids das cenas da pilha (a trait não diz qual é).
    pub ids: Vec<SceneId>,
    pub user: Option<String>,
    /// Rastro no formato do `ATELIE_TRACE`, mais os `Log`.
    pub trace: Vec<String>,
    pub cursor: Option<CursorRef>,
    pub frame: Frame,
    pub quit: bool,
    /// Desenha a cada passo (pega pânicos do `draw`).
    pub draw: bool,
    pub steps: u64,
    /// Duração de cada quadro (simula uma máquina lenta).
    pub dt: f32,
}

impl<'a> Director<'a> {
    pub fn new(assets: &'a Assets, root: &Path, first: SceneId, user: Option<&str>) -> Self {
        let mut d = Director {
            assets,
            root: root.to_path_buf(),
            stack: Vec::new(),
            ids: Vec::new(),
            user: user.map(str::to_string),
            trace: Vec::new(),
            cursor: None,
            frame: Frame::new(engine::SCREEN_W, engine::SCREEN_H),
            quit: false,
            draw: true,
            steps: 0,
            dt: DT,
        };
        let s = d.make(first);
        d.stack.push(s);
        d.ids.push(first);
        d
    }

    fn base(&self) -> PathBuf {
        self.root.join("atelie-livre")
    }
    pub fn drawings(&self) -> PathBuf {
        self.base().join("desenhos")
    }
    pub fn users(&self) -> PathBuf {
        self.base().join("usuarios")
    }
    pub fn texts(&self) -> PathBuf {
        self.base().join("textos")
    }
    fn user_name(&self) -> String {
        self.user
            .clone()
            .unwrap_or_else(|| DEFAULT_USER.to_string())
    }
    fn animal(&self) -> Option<usize> {
        let u = self.user.as_deref()?;
        engine::users::load(&self.users(), u).map(|p| p.bicho)
    }

    /// Espelho do `make_scene` de `crates/app/src/run.rs`.
    pub fn make(&self, id: SceneId) -> Box<dyn Scene> {
        let assets = self.assets;
        let user = self.user_name();
        let drawings = Some(self.drawings());
        engine::scenes::paint::set_user_name(&user);
        let room = |floor| {
            Box::new(Room::new(floor, None, drawings.clone(), assets).with_user(&user, assets))
        };
        match id {
            SceneId::Paint => Box::new(Paint::new(drawings.clone(), assets).with_user(&user)),
            SceneId::PaintNew => {
                Box::new(Paint::new_blank(drawings.clone(), assets).with_user(&user))
            }
            SceneId::Gallery => Box::new(
                GalleryRoom::new(None, drawings.clone(), assets).with_user(&user, self.animal()),
            ),
            SceneId::Building => room(Floor::Lobby),
            SceneId::Studio => room(Floor::Studio),
            SceneId::TricksFloor => room(Floor::Tricks),
            SceneId::Projects => room(Floor::Projects),
            SceneId::Tricks(kind) => Box::new(TricksBoard::new(kind, None)),
            SceneId::TricksStudio(kind, start) => {
                Box::new(Paint::new_tricks(kind, start, drawings.clone(), assets))
            }
            SceneId::Project(kind) => Box::new(Paint::new_project(kind, drawings.clone(), assets)),
            SceneId::FlipbookFigure => {
                Box::new(Paint::new_flipbook_figure(drawings.clone(), assets))
            }
            SceneId::Chooser => Box::new(Chooser::new()),
            SceneId::Splash => Box::new(Splash::new(None)),
            SceneId::Entry => Box::new(Entry::new(Some(self.users()), None, assets)),
            SceneId::Story => Box::new(Story::new(None, assets)),
            SceneId::BookStory => Box::new(Story::new(None, assets).over_room()),
            SceneId::Portfolio => Box::new(
                GalleryRoom::portfolio(None, drawings.clone(), assets)
                    .with_user(&user, self.animal()),
            ),
            SceneId::Imaginopolis => Box::new(Imaginopolis::new(None)),
            SceneId::Street => Box::new(Street::new(None)),
            SceneId::Browser => Box::new(Browser::new()),
            SceneId::WriterSplash => Box::new(Splash::writer(None)),
            SceneId::Writer => Box::new(Writer::new(Some(self.texts()), assets)),
            SceneId::Cw(c) => {
                // as mesmas cenas, com os dados do Writer (sem os desenhos do FA)
                let p = Program::Writer;
                match c {
                    CwScene::Entry => {
                        Box::new(Entry::for_program(p, Some(self.users()), None, assets))
                    }
                    CwScene::Story => Box::new(Story::for_program(p, None, assets)),
                    CwScene::BookStory => Box::new(Story::for_program(p, None, assets).over_room()),
                    CwScene::Imaginopolis => Box::new(Imaginopolis::for_program(p, None)),
                    CwScene::Street => Box::new(Street::for_program(p, None)),
                    CwScene::Building(floor) => Box::new(
                        Room::for_program(p, floor, None, None, assets)
                            .with_user(&user, assets)
                            .with_users_dir(Some(self.users()), assets),
                    ),
                    CwScene::Library => Box::new(
                        GalleryRoom::for_program(p, None, None, assets)
                            .with_user(&user, self.animal()),
                    ),
                    CwScene::Portfolio => Box::new(
                        GalleryRoom::portfolio_for_program(p, None, None, assets)
                            .with_user(&user, self.animal()),
                    ),
                    // a Máquina de Idéias e o Quadro de Figuras ainda são a
                    // área de escrita (como em `run.rs`)
                    CwScene::NewDocument | CwScene::IdeaMachine | CwScene::PictureBoard => {
                        Box::new(Writer::new(Some(self.texts()), assets))
                    }
                }
            }
        }
    }

    pub fn top(&self) -> SceneId {
        *self.ids.last().expect("cena")
    }

    /// Um quadro do laço: atualiza a cena de cima, aplica os efeitos e desenha.
    pub fn step(&mut self, input: &[Input]) -> Vec<Effect> {
        self.steps += 1;
        let fx = self
            .stack
            .last_mut()
            .expect("cena")
            .update(input, self.dt, self.assets);
        for e in &fx {
            self.apply(e.clone());
        }
        if self.draw {
            self.paint();
        }
        fx
    }

    pub fn paint(&mut self) {
        let n = self.stack.len();
        if self.stack[n - 1].overlay() && n > 1 {
            self.stack[n - 2].draw(&mut self.frame, self.assets);
        }
        self.stack[n - 1].draw(&mut self.frame, self.assets);
    }

    fn leave_logs(&mut self, fx: Vec<Effect>) {
        for e in fx {
            match e {
                Effect::Log(s) => self.trace.push(format!("log {s}")),
                other => self.trace.push(format!("perdido-no-leave {other:?}")),
            }
        }
    }

    fn apply(&mut self, e: Effect) {
        match e {
            Effect::PlaySound(r) => self.trace.push(format!("TRACE som {r:?}")),
            Effect::PlayLoop(r) => self.trace.push(format!("TRACE laço {r:?}")),
            Effect::StopLoop => self.trace.push("TRACE fim-laço".into()),
            Effect::SetCursor(c) => self.cursor = Some(c),
            Effect::ChangeScene(id) => {
                self.trace.push(format!("TRACE cena {id:?}"));
                while self.stack.len() > 1 {
                    let mut s = self.stack.pop().expect("cena");
                    self.ids.pop();
                    let fx = s.leave();
                    self.leave_logs(fx);
                }
                let fx = self.stack.last_mut().expect("cena").leave();
                self.leave_logs(fx);
                let s = self.make(id);
                *self.stack.last_mut().expect("cena") = s;
                *self.ids.last_mut().expect("cena") = id;
            }
            Effect::Push(id) => {
                self.trace.push(format!("TRACE push {id:?}"));
                let s = self.make(id);
                self.stack.push(s);
                self.ids.push(id);
            }
            Effect::Back => {
                self.trace.push("TRACE back".into());
                if self.stack.len() > 1 {
                    let mut s = self.stack.pop().expect("cena");
                    self.ids.pop();
                    let fx = s.leave();
                    self.leave_logs(fx);
                }
            }
            Effect::SetVolume(v) => self.trace.push(format!("TRACE volume {v}")),
            Effect::RecordStart => self.trace.push("TRACE gravar".into()),
            Effect::RecordStop(p) => self.trace.push(format!("TRACE gravar-fim {}", p.display())),
            Effect::RecordCancel => self.trace.push("TRACE gravar-cancela".into()),
            Effect::SetUser(u) => self.user = Some(u),
            Effect::Log(s) => self.trace.push(format!("log {s}")),
            Effect::Quit => {
                self.trace.push("TRACE quit".into());
                self.quit = true;
            }
        }
    }

    /// Fecha tudo, como ao sair do programa.
    pub fn close(&mut self) {
        while let Some(mut s) = self.stack.pop() {
            self.ids.pop();
            let fx = s.leave();
            self.leave_logs(fx);
        }
    }

    // ---- gestos ---------------------------------------------------------

    pub fn idle(&mut self, frames: u32) {
        for _ in 0..frames {
            self.step(&[]);
        }
    }

    /// Espera (no máximo `frames` quadros) até a cena de cima ser `id`.
    pub fn wait_for(&mut self, id: SceneId, frames: u32) -> bool {
        for _ in 0..frames {
            if self.top() == id {
                return true;
            }
            self.step(&[]);
        }
        self.top() == id
    }

    pub fn mv(&mut self, x: i32, y: i32) {
        self.step(&[Input::MouseMove { x, y }]);
    }

    /// Clique simples: aperta num quadro, solta no seguinte, e deixa a cena andar um pouco.
    pub fn click(&mut self, x: i32, y: i32) {
        self.step(&[Input::MouseMove { x, y }]);
        self.step(&[Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }]);
        self.step(&[Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        }]);
        self.idle(3);
    }

    /// Clique rápido: aperta e solta no mesmo quadro (máquina lenta ou
    /// mouse rápido: os dois eventos chegam juntos ao `update`).
    pub fn quick_click(&mut self, x: i32, y: i32) {
        self.step(&[
            Input::MouseMove { x, y },
            Input::Click {
                button: MouseButton::Left,
                x,
                y,
            },
            Input::MouseUp {
                button: MouseButton::Left,
                x,
                y,
            },
        ]);
        self.idle(3);
    }

    pub fn double_click(&mut self, x: i32, y: i32) {
        let c = Input::Click {
            button: MouseButton::Left,
            x,
            y,
        };
        let u = Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        };
        self.step(&[Input::MouseMove { x, y }]);
        self.step(std::slice::from_ref(&c));
        self.step(std::slice::from_ref(&u));
        self.step(&[c]);
        self.step(&[u]);
        self.idle(3);
    }

    /// Arrasta com o botão apertado, em `n` passos.
    pub fn drag(&mut self, (x0, y0): (i32, i32), (x1, y1): (i32, i32), n: i32) {
        self.step(&[Input::MouseMove { x: x0, y: y0 }]);
        self.step(&[Input::Click {
            button: MouseButton::Left,
            x: x0,
            y: y0,
        }]);
        for k in 1..=n {
            let x = x0 + (x1 - x0) * k / n;
            let y = y0 + (y1 - y0) * k / n;
            self.step(&[Input::MouseMove { x, y }]);
        }
        self.step(&[Input::MouseUp {
            button: MouseButton::Left,
            x: x1,
            y: y1,
        }]);
        self.idle(3);
    }

    pub fn key(&mut self, key: Key) {
        self.step(&[Input::Key {
            key,
            ctrl: false,
            alt: false,
        }]);
    }

    pub fn text(&mut self, s: &str) {
        self.step(&[Input::Text(s.to_string())]);
    }

    /// Linhas do rastro a partir da posição `from`.
    pub fn trace_since(&self, from: usize) -> &[String] {
        &self.trace[from.min(self.trace.len())..]
    }

    pub fn sounds_since(&self, from: usize) -> usize {
        self.trace_since(from)
            .iter()
            .filter(|l| l.starts_with("TRACE som") || l.starts_with("TRACE laço"))
            .count()
    }

    /// Cor RGB de um ponto da tela já desenhada.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * self.frame.width() + x) * 4) as usize;
        let p = &self.frame.pixels()[i..i + 3];
        [p[0], p[1], p[2]]
    }

    /// Cópia do quadro (para comparar antes/depois de um clique).
    pub fn snapshot(&self) -> Vec<u8> {
        self.frame.pixels().to_vec()
    }

    pub fn changed_pixels(&self, before: &[u8]) -> usize {
        before
            .chunks(4)
            .zip(self.frame.pixels().chunks(4))
            .filter(|(a, b)| a[..3] != b[..3])
            .count()
    }
}

/// Gerador pseudoaleatório simples (xorshift64*), para o macaco ser reproduzível.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo).max(1) as u64) as i32
    }
}

/// Uma "rajada" de entrada aleatória de criança: clique, arrasto, duplo
/// clique, tecla, texto ou espera. Devolve a sequência de quadros.
pub fn kid_burst(rng: &mut Rng) -> Vec<Vec<Input>> {
    let x = rng.range(0, 640);
    let y = rng.range(0, 480);
    let left = MouseButton::Left;
    let c = Input::Click { button: left, x, y };
    let u = Input::MouseUp { button: left, x, y };
    match rng.below(100) {
        0..=44 => vec![vec![Input::MouseMove { x, y }], vec![c], vec![u], vec![]],
        45..=59 => {
            let (x1, y1) = (rng.range(0, 640), rng.range(0, 480));
            let mut v = vec![vec![Input::MouseMove { x, y }], vec![c]];
            for k in 1..=6 {
                v.push(vec![Input::MouseMove {
                    x: x + (x1 - x) * k / 6,
                    y: y + (y1 - y) * k / 6,
                }]);
            }
            v.push(vec![Input::MouseUp {
                button: left,
                x: x1,
                y: y1,
            }]);
            v
        }
        60..=67 => vec![vec![c.clone()], vec![u.clone()], vec![c], vec![u]],
        68..=71 => vec![
            vec![Input::Click {
                button: MouseButton::Right,
                x,
                y,
            }],
            vec![Input::MouseUp {
                button: MouseButton::Right,
                x,
                y,
            }],
        ],
        72..=77 => {
            let keys = [
                Key::Enter,
                Key::Escape,
                Key::Backspace,
                Key::Delete,
                Key::Left,
                Key::Right,
                Key::Up,
                Key::Down,
                Key::PageUp,
                Key::PageDown,
                Key::Home,
                Key::End,
                Key::Other,
            ];
            let key = keys[rng.below(keys.len() as u64) as usize];
            vec![vec![Input::Key {
                key,
                ctrl: false,
                alt: false,
            }]]
        }
        78..=81 => {
            let words = [
                "a",
                "Bia",
                "olá ção",
                "123",
                " ",
                "ZZZZZZZZZZZZZZZZZZZZZZZZZZZZ",
            ];
            vec![vec![Input::Text(
                words[rng.below(words.len() as u64) as usize].to_string(),
            )]]
        }
        82..=83 => vec![
            vec![Input::Shift(true)],
            vec![c],
            vec![u],
            vec![Input::Shift(false)],
        ],
        84..=85 => vec![
            vec![Input::Ctrl(true)],
            vec![c],
            vec![u],
            vec![Input::Ctrl(false)],
        ],
        _ => vec![vec![]; rng.range(5, 90) as usize],
    }
}
