//! Salas do prédio do Fine Artist: saguão, Ateliê de Pintura, Oficina de
//! Projetos e Truques de Desenho. Fundo e elementos fixos vêm de `rooms_data`
//! (medidos no original); as áreas clicáveis estão aqui. Em cada andar há um
//! personagem parado à esquerda (McZee com uma fantasia sorteada no saguão e
//! no Ateliê; Maggie nos andares 3 e 4) e, de tempos em tempos, uma animação
//! ambiente do andar (`gags`). Os objetos do chão (Bolsa Mágica, maleta,
//! livro, Varinha Mágica) estão em `room_objects`; o elevador, em `elevator`.

use super::elevator::{self, Elevator, Outcome};
use super::gags::{self, Character, GagPlayer};
use super::projects::ProjectKind;
use super::room_objects::{self as objects, BagLayout, BagPart, MagicBag, Surprises, Target, snd};
use super::rooms_data::{self, Element};
use super::wizard::TrickKind;
use crate::anim::Animator;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::Balloon;
use crate::paint::Canvas;
use crate::paint::store::{LAST_NAME, load_png};
use crate::rng::XorShift;
use crate::scene::{DEFAULT_CURSOR, Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::{Frame, SCREEN_H, SCREEN_W};
use formats::palette::Rgb;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Floor {
    Lobby,
    Studio,
    Projects,
    Tricks,
}

type Rect = (i32, i32, i32, i32);

/// O que um clique numa área faz.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Go(SceneId),
    /// Quadro novo: apaga o último desenho e abre a pintura.
    NewPicture,
    /// Fecha o balão.
    Close,
    Floor(Floor),
    Quit,
    /// Pergunta "Botão ou adesivo?" num segundo balão.
    AskButton,
    /// Mastro de bombeiro: sobe para o andar.
    PoleUp(Floor),
    /// Abertura do mastro no chão: desce para o andar.
    PoleDown(Floor),
    /// Buraco do rato (só faz algo com a Varinha Mágica).
    MouseHole,
    /// Portas do elevador, botão de chamar ou skate "ELEVADOR".
    Elevator,
    /// Skate "LÁ FORA" do saguão: volta para a rua.
    Outside,
}

struct Hotspot {
    rect: Rect,
    action: Action,
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

impl Floor {
    pub(crate) fn background(self) -> u16 {
        match self {
            Floor::Lobby => rooms_data::LOBBY_BG,
            Floor::Studio => rooms_data::STUDIO_BG,
            Floor::Projects => rooms_data::PROJECTS_BG,
            Floor::Tricks => rooms_data::TRICKS_BG,
        }
    }

    fn elements(self) -> &'static [Element] {
        match self {
            Floor::Lobby => rooms_data::LOBBY,
            Floor::Studio => rooms_data::STUDIO,
            Floor::Projects => rooms_data::PROJECTS,
            Floor::Tricks => rooms_data::TRICKS,
        }
    }

    /// Áreas clicáveis, medidas no original.
    fn hotspots(self) -> Vec<Hotspot> {
        let h = |rect, action| Hotspot { rect, action };
        match self {
            Floor::Lobby => vec![
                h((98, 38, 90, 18), Action::Floor(Floor::Tricks)),
                h((98, 58, 90, 18), Action::Floor(Floor::Projects)),
                h((98, 78, 90, 18), Action::Floor(Floor::Studio)),
                h((98, 98, 90, 18), Action::Go(SceneId::Gallery)),
                h((513, 346, 54, 47), Action::MouseHole),
                h((502, 411, 81, 66), Action::Outside),
                h((480, 202, 36, 71), Action::Elevator),
                h((210, 130, 260, 270), Action::Elevator),
                h((146, 290, 48, 20), Action::Quit),
                h((95, 316, 95, 52), Action::Go(SceneId::Gallery)),
                h((594, 0, 26, 400), Action::PoleUp(Floor::Studio)),
            ],
            Floor::Studio => vec![
                h((110, 176, 170, 100), Action::Go(SceneId::Paint)),
                h((264, 63, 190, 145), Action::Go(SceneId::Gallery)),
                h((466, 328, 54, 47), Action::MouseHole),
                h((466, 407, 81, 66), Action::Elevator),
                h((531, 390, 105, 43), Action::PoleDown(Floor::Lobby)),
                h((575, 0, 24, 395), Action::PoleUp(Floor::Projects)),
                h((474, 250, 46, 20), Action::Quit),
            ],
            Floor::Projects => vec![
                h((479, 289, 54, 47), Action::MouseHole),
                h((466, 407, 81, 66), Action::Elevator),
                h((529, 371, 105, 43), Action::PoleDown(Floor::Studio)),
                h((573, 0, 24, 375), Action::PoleUp(Floor::Tricks)),
                h((478, 185, 50, 20), Action::Quit),
                h(
                    (40, 20, 170, 150),
                    Action::Go(SceneId::Project(ProjectKind::Comics)),
                ),
                h(
                    (250, 0, 160, 120),
                    Action::Go(SceneId::Project(ProjectKind::Flipbook)),
                ),
                h((150, 170, 240, 110), Action::AskButton),
            ],
            Floor::Tricks => vec![
                h((576, 273, 54, 47), Action::MouseHole),
                h((457, 409, 81, 66), Action::Elevator),
                h((495, 355, 105, 43), Action::PoleDown(Floor::Projects)),
                // no original o mastro sobe para um sótão escuro de onde se
                // desce ao saguão; aqui vai direto ao saguão
                h((539, 0, 24, 360), Action::PoleUp(Floor::Lobby)),
                h((564, 114, 50, 20), Action::Quit),
                h(
                    (330, 80, 190, 250),
                    Action::Go(SceneId::Tricks(TrickKind::ThreeD)),
                ),
                h(
                    (150, 40, 150, 290),
                    Action::Go(SceneId::Tricks(TrickKind::Basic)),
                ),
            ],
        }
    }
}

/// Obras-primas do McZee (cor, máscara) que aparecem na moldura do andar 4.
const PAINTINGS: [(u16, u16); 8] = [
    (0x6d37, 0x6d39),
    (0x6d3b, 0x6d3d),
    (0x6d3f, 0x6d45),
    (0x6d47, 0x6d4d),
    (0x6d4f, 0x6d51),
    (0x6d53, 0x6d55),
    (0x6d57, 0x6d59),
    (0x6d5b, 0x6d5d),
];
const PAINTING_POS: (i32, i32) = (1, 4);

impl Floor {
    fn painting(self, rng: &mut XorShift) -> Option<(u16, u16)> {
        (self == Floor::Tricks).then(|| PAINTINGS[rng.below(PAINTINGS.len())])
    }
}

/// Bloco `WHAT` com os balões das salas e índice do balão de chegada de cada andar.
const ROOMS_WHAT: u16 = 0x100d;

/// Nome usado no lugar do `@` dos balões quando não há usuário escolhido.
pub const DEFAULT_USER: &str = "artista";

impl Floor {
    /// Balão de chegada ao andar: o do `WHAT` do original quando os dados
    /// estão instalados, senão o texto reserva.
    fn balloon(self, assets: &Assets, user: &str) -> Balloon {
        let k = match self {
            Floor::Lobby => 1,
            Floor::Studio => 26,
            Floor::Projects => 31,
            Floor::Tricks => 42,
        };
        let fallback = self.fallback_balloon();
        let records = assets.dialogs(Program::Artist, ROOMS_WHAT);
        match records.get(k) {
            Some(r) if r.options().len() == fallback.options.len() => {
                Balloon::from_block(&records, k, fallback.x, fallback.y, user).unwrap_or(fallback)
            }
            _ => fallback,
        }
    }

    /// Balão do McZee ou da Maggie ao chegar ao andar (textos do original,
    /// usados quando o `ARTIST.EXE` não está instalado).
    fn fallback_balloon(self) -> Balloon {
        let (x, y, w) = match self {
            Floor::Lobby => (300, 20, 300),
            Floor::Studio => (130, 230, 200),
            Floor::Projects => (70, 150, 300),
            Floor::Tricks => (70, 120, 260),
        };
        let (text, options): (&str, &[&str]) = match self {
            Floor::Lobby => (
                "Oi! Eu sou o McZee. O que você quer fazer?",
                &[
                    "Começar a desenhar por conta própria",
                    "Fazer uma história em quadrinhos, um botão ou adesivo, ou montar um Desenho Animado",
                    "Aprender alguns truques de desenho",
                    "Copiar, apagar ou limpar desenhos",
                ],
            ),
            Floor::Studio => (
                "Oba! Este é o Ateliê de Pintura! O que você quer fazer?",
                &[
                    "Pintar alguma coisa nova",
                    "Trabalhar na minha última obra",
                    "Trabalhar em um quadro mais antigo",
                    "Outra coisa?",
                ],
            ),
            Floor::Projects => (
                "Aqui é a Maggie! Estamos no Andar dos Projetos! Vamos fazer alguma coisa juntos?",
                &[
                    "É, uma história em quadrinhos",
                    "OK, vamos fazer um botão ou um adesivo",
                    "Quero fazer um desenho animado",
                    "Outra coisa?",
                ],
            ),
            Floor::Tricks => (
                "Seja bem-vindo ao andar dos Truques de Desenho! Eu vou te mostrar os meus truques secretos para desenhar. Com que truque você quer que eu comece?",
                &[
                    "Um truque básico",
                    "Fazendo uma coisa parecer tridimensional",
                    "Quero fazer outra coisa",
                ],
            ),
        };
        Balloon {
            x,
            y,
            w,
            text: text.into(),
            options: options.iter().map(|o| o.to_string()).collect(),
            links: Vec::new(),
        }
    }

    /// Ação da opção `k` do balão.
    fn balloon_action(self, k: usize) -> Action {
        match (self, k) {
            (Floor::Lobby, 0) => Action::Floor(Floor::Studio),
            (Floor::Lobby, 1) => Action::Floor(Floor::Projects),
            (Floor::Lobby, 2) => Action::Floor(Floor::Tricks),
            (Floor::Lobby, _) => Action::Go(SceneId::Gallery),
            (Floor::Studio, 0) => Action::NewPicture,
            (Floor::Studio, 1) => Action::Go(SceneId::Paint),
            (Floor::Studio, 2) => Action::Go(SceneId::Gallery),
            (Floor::Projects, 0) => Action::Go(SceneId::Project(ProjectKind::Comics)),
            (Floor::Projects, 1) => Action::AskButton,
            (Floor::Projects, 2) => Action::Go(SceneId::Project(ProjectKind::Flipbook)),
            (Floor::Tricks, 0) => Action::Go(SceneId::Tricks(TrickKind::Basic)),
            (Floor::Tricks, 1) => Action::Go(SceneId::Tricks(TrickKind::ThreeD)),
            _ => Action::Close,
        }
    }

    /// Roteiros `ATAG` das animações ambientes do andar.
    fn gag_scripts(self) -> &'static [u16] {
        match self {
            Floor::Lobby => gags::LOBBY_SCRIPTS,
            Floor::Studio => gags::STUDIO_SCRIPTS,
            Floor::Projects => gags::PROJECTS_SCRIPTS,
            Floor::Tricks => gags::TRICKS_SCRIPTS,
        }
    }

    /// Personagem parado do andar, com a fantasia ou pose sorteada.
    fn character(self, rng: &mut XorShift) -> Option<Character> {
        match self {
            Floor::Lobby => Character::pick(gags::MCZEE_COSTUMES, gags::MCZEE_LOBBY, rng),
            Floor::Studio => Character::pick(gags::MCZEE_COSTUMES, gags::MCZEE_STUDIO, rng),
            Floor::Projects | Floor::Tricks => {
                Character::pick(gags::MAGGIE_POSES, gags::MAGGIE_POS, rng)
            }
        }
    }

    pub fn number(self) -> u8 {
        match self {
            Floor::Lobby => 1,
            Floor::Studio => 2,
            Floor::Projects => 3,
            Floor::Tricks => 4,
        }
    }

    fn bag_layout(self) -> BagLayout {
        match self {
            Floor::Lobby => objects::LOBBY_BAG,
            Floor::Studio => objects::STUDIO_BAG,
            Floor::Projects => objects::PROJECTS_BAG,
            Floor::Tricks => objects::TRICKS_BAG,
        }
    }

    /// No original a bolsa já está aberta ao chegar ao Ateliê; nos outros
    /// andares, fechada.
    fn bag(self) -> MagicBag {
        MagicBag::new(self.bag_layout(), self == Floor::Studio)
    }

    fn surprises(self) -> Surprises {
        match self {
            Floor::Lobby => objects::LOBBY_SURPRISES,
            Floor::Studio => objects::STUDIO_SURPRISES,
            Floor::Projects => objects::PROJECTS_SURPRISES,
            Floor::Tricks => objects::TRICKS_SURPRISES,
        }
    }

    /// Balão da Varinha Mágica (`WHAT 100d`): nos andares fala do mastro e
    /// do buraco do rato.
    fn wand_balloon(self, assets: &Assets, user: &str) -> Balloon {
        let fallback = Balloon {
            x: 0,
            y: 0,
            w: 168,
            text: "Varinha Mágica\nQuando você clica no mastro de bombeiro ou no buraco do rato com a Varinha Mágica, coisas incríveis acontecem!".into(),
            options: Vec::new(),
            links: Vec::new(),
        };
        let mut b = Balloon::from_block(
            &assets.dialogs(Program::Artist, ROOMS_WHAT),
            WAND_FLOORS,
            0,
            0,
            user,
        )
        .filter(|b| !b.text.is_empty())
        .unwrap_or(fallback);
        // a "Tecla de atalho" (Ctrl) não existe aqui
        b.options.clear();
        b.x = 120;
        b.y = 235;
        b
    }
}

/// Balão "Wand - floors" do `WHAT 100d`.
const WAND_FLOORS: usize = 59;

/// Quadro no cavalete do Ateliê: o último desenho, reduzido a 1/4.
const EASEL: (i32, i32) = (120, 179);
const EASEL_SCALE: u32 = 4;

/// Espera antes da primeira animação ambiente e entre as seguintes (s).
/// Animações de passagem: só depois de um tempo sem ninguém clicar (como no
/// original); cada clique ou tecla recomeça a espera.
const GAG_FIRST: (f32, f32) = (20.0, 30.0);
const GAG_BETWEEN: (f32, f32) = (20.0, 40.0);

pub struct Room {
    common: KidCommon,
    floor: Floor,
    hotspots: Vec<Hotspot>,
    last_drawing: Option<Canvas>,
    pal: Vec<Rgb>,
    drawings: Option<PathBuf>,
    balloon: Option<Balloon>,
    /// O balão aberto é o de confirmação de saída ("Tchau").
    quitting: bool,
    /// Balão "Botão ou adesivo?" aberto.
    asking_button: bool,
    rng: XorShift,
    character: Option<Character>,
    /// Obra-prima do McZee na moldura do andar dos Truques (sorteada).
    painting: Option<(u16, u16)>,
    gags: GagPlayer,
    /// Apelido do usuário, o `@` dos balões.
    user: String,
    /// Bolsa Mágica do andar (e a Varinha Mágica).
    bag: MagicBag,
    /// Surpresa da Varinha Mágica em curso (mastro ou buraco do rato).
    surprise: Option<Animator>,
    /// Dentro do elevador.
    elevator: Option<Elevator>,
    /// O cursor atual é o da varinha.
    wand_cursor: bool,
}

impl Room {
    pub fn new(
        floor: Floor,
        click: Option<SoundRef>,
        drawings: Option<PathBuf>,
        assets: &Assets,
    ) -> Room {
        let pal = assets.palettes(Program::Artist).pal256.clone();
        let last_drawing = drawings
            .as_ref()
            .and_then(|d| load_png(&d.join(LAST_NAME), &pal));
        let mut rng = XorShift::from_clock();
        let character = floor.character(&mut rng);
        let gags = GagPlayer::with_rng(floor.gag_scripts(), GAG_FIRST, GAG_BETWEEN, rng.clone());
        Room {
            common: KidCommon::new(click),
            hotspots: floor.hotspots(),
            last_drawing,
            pal,
            drawings,
            balloon: Some(floor.balloon(assets, DEFAULT_USER)),
            floor,
            quitting: false,
            asking_button: false,
            painting: floor.painting(&mut rng),
            rng,
            character,
            gags,
            user: DEFAULT_USER.to_string(),
            bag: floor.bag(),
            surprise: None,
            elevator: None,
            wand_cursor: false,
        }
    }

    /// Com o apelido do usuário no lugar do `@` dos balões.
    pub fn with_user(mut self, user: &str, assets: &Assets) -> Room {
        self.user = user.to_string();
        self.balloon = Some(self.floor.balloon(assets, &self.user));
        self
    }

    pub fn character(&self) -> Option<&Character> {
        self.character.as_ref()
    }

    /// Há uma animação ambiente em curso.
    pub fn gag_playing(&self) -> bool {
        self.gags.is_playing()
    }

    pub fn balloon_open(&self) -> bool {
        self.balloon.is_some()
    }

    pub fn bag(&self) -> &MagicBag {
        &self.bag
    }

    /// Há uma surpresa da Varinha Mágica em curso.
    pub fn surprise_playing(&self) -> bool {
        self.surprise.is_some()
    }

    pub fn in_elevator(&self) -> bool {
        self.elevator.is_some()
    }

    /// Clique na Bolsa Mágica ou no que saiu dela.
    fn use_bag(&mut self, part: BagPart, fx: &mut Vec<Effect>, assets: &Assets) {
        match part {
            BagPart::Bag => {
                // com a varinha na mão, a bolsa a recebe de volta e se abre
                let open = self.bag.holding_wand || !self.bag.open;
                self.bag.holding_wand = false;
                self.bag.open = open;
                fx.push(Effect::PlaySound(snd(if open {
                    objects::SND_BAG_OPEN
                } else {
                    objects::SND_BAG_CLOSE
                })));
            }
            BagPart::Suitcase => {
                fx.push(Effect::PlaySound(snd(objects::SND_SUITCASE)));
                fx.push(Effect::Push(SceneId::Portfolio));
            }
            BagPart::Book => {
                fx.extend(objects::SND_BOOK.map(|id| Effect::PlaySound(snd(id))));
                fx.push(Effect::Push(SceneId::BookStory));
            }
            BagPart::Wand => {
                self.bag.holding_wand = true;
                self.bag.open = false;
                fx.push(Effect::PlaySound(snd(objects::SND_WAND_TAKE)));
                self.balloon = Some(self.floor.wand_balloon(assets, &self.user));
            }
        }
    }

    /// A varinha tocou o mastro ou o buraco do rato: a surpresa do andar.
    /// Depois dela a bolsa se abre com a varinha dentro.
    fn start_surprise(&mut self, target: Target, fx: &mut Vec<Effect>, assets: &Assets) {
        self.bag.holding_wand = false;
        fx.push(Effect::PlaySound(snd(match target {
            Target::Pole => objects::SND_POLE_UP,
            Target::Hole => objects::SND_WAND_HOLE,
        })));
        match self.floor.surprises().start(target, &mut self.rng, assets) {
            Some(mut a) => {
                fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
                self.surprise = Some(a);
            }
            None => self.bag.open = true,
        }
    }

    fn enter_elevator(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        fx.push(Effect::PlaySound(snd(if self.floor == Floor::Lobby {
            elevator::SND_DOORS
        } else {
            objects::SND_SKATE
        })));
        fx.push(Effect::PlaySound(snd(elevator::SND_ARRIVE)));
        let costume = self
            .character
            .as_ref()
            .map(|c| c.image)
            .filter(|i| gags::MCZEE_COSTUMES.contains(i));
        self.elevator = Some(Elevator::new(self.floor, costume, assets, &self.user));
        self.balloon = None;
        self.bag.holding_wand = false;
    }

    fn act(&mut self, action: Action, fx: &mut Vec<Effect>, assets: &Assets) {
        match action {
            Action::Go(id) => fx.push(Effect::ChangeScene(id)),
            Action::Floor(fl) => self.set_floor(fl, assets),
            Action::PoleUp(fl) => {
                fx.push(Effect::PlaySound(snd(objects::SND_POLE_UP)));
                self.set_floor(fl, assets);
            }
            Action::PoleDown(fl) => {
                fx.push(Effect::PlaySound(snd(objects::SND_POLE_DOWN)));
                self.set_floor(fl, assets);
            }
            Action::MouseHole => {}
            Action::Elevator => self.enter_elevator(fx, assets),
            Action::Outside => {
                fx.push(Effect::PlaySound(snd(objects::SND_SKATE)));
                fx.push(Effect::ChangeScene(SceneId::Street));
            }
            Action::Quit => {
                // como no original: "O que você quer fazer?"
                self.quitting = true;
                self.balloon = Some(Balloon {
                    x: 210,
                    y: 180,
                    w: 220,
                    text: "Epa! O que você quer fazer?".into(),
                    options: vec!["Sair do Fine Artist".into(), "Continuar trabalhando".into()],
                    links: Vec::new(),
                });
            }
            Action::Close => {
                self.balloon = None;
                self.quitting = false;
                self.asking_button = false;
            }
            Action::AskButton => {
                self.asking_button = true;
                self.balloon = Some(Balloon {
                    x: 70,
                    y: 150,
                    w: 260,
                    text: "Legal! O que você quer fazer?".into(),
                    options: vec!["Um botão".into(), "Um adesivo".into()],
                    links: Vec::new(),
                });
            }
            Action::NewPicture => {
                if let Some(d) = &self.drawings {
                    let blank = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
                    if let Err(e) =
                        crate::paint::store::save_png(&blank, &self.pal, &d.join(LAST_NAME))
                    {
                        fx.push(Effect::Log(format!("quadro novo: {e}")));
                    }
                }
                fx.push(Effect::ChangeScene(SceneId::Paint));
            }
        }
    }

    /// Cursor da varinha enquanto ela está na mão; senão, a mão.
    fn sync_cursor(&mut self, fx: &mut Vec<Effect>) {
        if self.bag.holding_wand != self.wand_cursor {
            self.wand_cursor = self.bag.holding_wand;
            fx.push(Effect::SetCursor(if self.wand_cursor {
                objects::WAND_CURSOR
            } else {
                DEFAULT_CURSOR
            }));
        }
    }

    pub fn floor(&self) -> Floor {
        self.floor
    }

    fn set_floor(&mut self, floor: Floor, assets: &Assets) {
        self.floor = floor;
        self.hotspots = floor.hotspots();
        self.balloon = Some(floor.balloon(assets, &self.user));
        self.character = floor.character(&mut self.rng);
        self.painting = floor.painting(&mut self.rng);
        self.bag = floor.bag();
        self.surprise = None;
        self.gags = GagPlayer::with_rng(
            floor.gag_scripts(),
            GAG_FIRST,
            GAG_BETWEEN,
            XorShift::new(self.rng.next_u32()),
        );
    }
}

/// Copia a imagem tratando branco puro como transparente (elementos sem máscara).
fn blit_white_transparent(f: &mut Frame, img: &crate::Image, x: i32, y: i32) {
    for dy in 0..img.height {
        for dx in 0..img.width {
            let i = ((dy * img.width + dx) * 4) as usize;
            let p = &img.rgba[i..i + 4];
            if p[3] != 0 && p[..3] != [255, 255, 255] {
                f.put(x + dx as i32, y + dy as i32, [p[0], p[1], p[2]]);
            }
        }
    }
}

impl Scene for Room {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        if input
            .iter()
            .any(|i| matches!(i, Input::Click { .. } | Input::Key { .. }))
        {
            self.gags.poke();
        }
        fx.extend(
            self.gags
                .update(dt, assets)
                .into_iter()
                .map(Effect::PlaySound),
        );
        if let Some(a) = &mut self.surprise {
            fx.extend(a.update(dt).into_iter().map(Effect::PlaySound));
            if a.is_finished() {
                // a varinha volta para a bolsa, que fica aberta
                self.surprise = None;
                self.bag.open = true;
            }
        }
        if self.elevator.is_some() && self.balloon.is_none() {
            let outcome = self
                .elevator
                .as_mut()
                .and_then(|e| e.update(input, dt, assets, &mut fx));
            match outcome {
                Some(Outcome::Arrived(fl)) => {
                    self.elevator = None;
                    self.set_floor(fl, assets);
                }
                Some(Outcome::Quit) => self.act(Action::Quit, &mut fx, assets),
                None => {}
            }
            self.sync_cursor(&mut fx);
            return fx;
        }
        for i in input {
            let Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } = *i
            else {
                continue;
            };
            // durante a surpresa da varinha os cliques esperam
            if self.surprise.is_some() {
                continue;
            }
            if let Some(b) = &self.balloon {
                if let Some(k) = b.hit(assets, x, y) {
                    if self.quitting {
                        if k == 0 {
                            fx.push(Effect::Quit);
                        } else {
                            self.act(Action::Close, &mut fx, assets);
                        }
                        break;
                    }
                    let action = if self.asking_button {
                        let kind = if k == 0 {
                            ProjectKind::Button
                        } else {
                            ProjectKind::Sticker
                        };
                        Action::Go(SceneId::Project(kind))
                    } else {
                        self.floor.balloon_action(k)
                    };
                    self.act(action, &mut fx, assets);
                    break;
                }
                if b.contains(assets, x, y) {
                    continue;
                }
                // clicar fora do balão só o fecha; o da varinha deixa o
                // clique seguir (ela já está na mão)
                let pass = self.bag.holding_wand && b.options.is_empty();
                self.balloon = None;
                self.quitting = false;
                self.asking_button = false;
                if !pass {
                    continue;
                }
            }
            if self.elevator.is_some() {
                continue;
            }
            if let Some(part) = self.bag.hit(x, y) {
                self.use_bag(part, &mut fx, assets);
                break;
            }
            let Some(hs) = self.hotspots.iter().find(|h| inside(h.rect, x, y)) else {
                continue;
            };
            let action = hs.action.clone();
            match (self.bag.holding_wand, action) {
                (true, Action::PoleUp(_) | Action::PoleDown(_)) => {
                    self.start_surprise(Target::Pole, &mut fx, assets)
                }
                (true, Action::MouseHole) => self.start_surprise(Target::Hole, &mut fx, assets),
                (_, action) => self.act(action, &mut fx, assets),
            }
            break;
        }
        self.sync_cursor(&mut fx);
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        if let Some(e) = &self.elevator {
            e.draw(frame, assets);
            if let Some(b) = &self.balloon {
                b.draw(frame, assets);
            }
            return;
        }
        frame.fill([0, 0, 0]);
        let bg = assets.image(&ImageRef::Pic(Program::Artist, self.floor.background()));
        frame.blit(
            &bg,
            (SCREEN_W as i32 - bg.width as i32) / 2,
            (SCREEN_H as i32 - bg.height as i32) / 2,
        );
        for e in self.floor.elements() {
            let img = assets.image(&ImageRef::Pic(Program::Artist, e.id));
            if e.mask != 0 {
                let mask = assets.image(&ImageRef::Pic(Program::Artist, e.mask));
                frame.blit_masked(&img, &mask, e.x, e.y);
            } else {
                blit_white_transparent(frame, &img, e.x, e.y);
            }
        }
        self.bag.draw(frame, assets);
        if let Some((img, mask)) = self.painting {
            frame.blit_masked(
                &assets.image(&ImageRef::Pic(Program::Artist, img)),
                &assets.image(&ImageRef::Pic(Program::Artist, mask)),
                PAINTING_POS.0,
                PAINTING_POS.1,
            );
        }
        if let Some(c) = &self.character {
            c.draw(frame, assets);
        }
        if self.floor == Floor::Studio
            && let Some(c) = &self.last_drawing
        {
            // o desenho no cavalete, reduzido pelo vizinho mais próximo
            for y in 0..c.h / EASEL_SCALE {
                for x in 0..c.w / EASEL_SCALE {
                    let i = c.px[((y * EASEL_SCALE) * c.w + x * EASEL_SCALE) as usize];
                    let p = self
                        .pal
                        .get(i as usize)
                        .copied()
                        .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                    frame.put(EASEL.0 + x as i32, EASEL.1 + y as i32, [p.r, p.g, p.b]);
                }
            }
        }
        // por cima de tudo (até do quadro no cavalete), menos dos balões
        self.gags.draw(frame, assets);
        if let Some(a) = &self.surprise {
            a.draw(frame, assets);
        }
        if let Some(b) = &self.balloon {
            b.draw(frame, assets);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    #[test]
    fn lobby_directory_and_pole_tour() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        assert!(r.balloon_open());
        // clique fora do balão fecha; depois os pontos quentes funcionam
        r.update(&[click(140, 300)], 0.0, &a);
        assert!(!r.balloon_open());
        r.update(&[click(140, 86)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Studio);
        assert!(r.balloon_open());
        r.update(&[click(600, 470), click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
        r.update(&[click(600, 470), click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Tricks);
        r.update(&[click(600, 470), click(550, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        // opção 1 do balão do saguão: Oficina de Projetos
        let b = r.balloon.as_ref().unwrap();
        let (bx, by, _, _) = b.option_rect(&a, 1);
        r.update(&[click(bx + 20, by + 3)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
    }

    #[test]
    fn easel_opens_paint_and_tchau_quits() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[click(600, 470)], 0.0, &a);
        let fx = r.update(&[click(190, 220)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        r.update(&[click(496, 260)], 0.0, &a);
        assert!(r.balloon_open());
        let (bx, by, _, _) = r.balloon.as_ref().unwrap().option_rect(&a, 1);
        r.update(&[click(bx + 20, by + 3)], 0.0, &a);
        assert!(!r.balloon_open());
        r.update(&[click(496, 260)], 0.0, &a);
        let (bx, by, _, _) = r.balloon.as_ref().unwrap().option_rect(&a, 0);
        let fx = r.update(&[click(bx + 20, by + 3)], 0.0, &a);
        assert!(fx.contains(&Effect::Quit));
        r.update(&[click(600, 470)], 0.0, &a);
        let fx = r.update(&[click(350, 100)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Gallery)));
    }

    #[test]
    fn studio_shows_last_drawing_on_easel() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut c = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
        c.px.fill(255); // preto na rampa de cinzas dos testes
        crate::paint::store::save_drawing(
            dir.path(),
            &c,
            &a.palettes(Program::Artist).pal256,
            false,
            0,
        )
        .unwrap();
        let r = Room::new(Floor::Studio, None, Some(dir.path().to_path_buf()), &a);
        let mut f = Frame::new(640, 480);
        r.draw(&mut f, &a);
        assert_eq!(f.pixel(190, 220)[..3], [0, 0, 0]);
    }

    #[test]
    fn studio_balloon_new_picture_blanks_last() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = a.palettes(Program::Artist).pal256.clone();
        let mut c = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
        c.px.fill(255);
        crate::paint::store::save_drawing(dir.path(), &c, &pal, false, 0).unwrap();
        let mut r = Room::new(Floor::Studio, None, Some(dir.path().to_path_buf()), &a);
        let (bx, by, _, _) = r.balloon.as_ref().unwrap().option_rect(&a, 0);
        let fx = r.update(&[click(bx + 20, by + 3)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        let last = load_png(&dir.path().join(LAST_NAME), &pal).unwrap();
        assert!(last.px.iter().all(|&v| v == 0));
    }

    #[test]
    fn draws_without_data() {
        let (_d, a) = empty_assets();
        let mut f = Frame::new(640, 480);
        for fl in [Floor::Lobby, Floor::Studio, Floor::Projects, Floor::Tricks] {
            let mut r = Room::new(fl, None, None, &a);
            // as animações ambientes tentam carregar e falham sem quebrar
            for _ in 0..40 {
                let fx = r.update(&[], 1.0, &a);
                assert!(!fx.iter().any(|e| matches!(e, Effect::PlaySound(_))));
            }
            r.draw(&mut f, &a);
        }
    }

    #[test]
    fn each_floor_has_its_character() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        let c = r.character().unwrap().clone();
        assert!(gags::MCZEE_COSTUMES.contains(&c.image));
        assert_eq!((c.x, c.y), gags::MCZEE_LOBBY);
        r.update(&[click(140, 300), click(140, 66)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
        let c = r.character().unwrap();
        assert!(gags::MAGGIE_POSES.contains(&c.image));
        assert_eq!((c.x, c.y), gags::MAGGIE_POS);
    }

    #[test]
    fn room_gags_play_with_real_data() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        let mut f = Frame::new(640, 480);
        let mut played = 0;
        // nada nos primeiros 15 s sem cliques; depois de 30 s já começou
        for _ in 0..150 {
            r.update(&[], 0.1, &a);
            assert!(!r.gags.is_playing());
        }
        for _ in 0..400 {
            r.update(&[], 0.1, &a);
            if r.gags.is_playing() {
                played += 1;
                r.draw(&mut f, &a);
            }
        }
        assert!(played > 0);
        // McZee fantasiado à esquerda do saguão: pixel não branco na barriga
        r.draw(&mut f, &a);
        assert_ne!(f.pixel(60, 300)[..3], [255, 255, 255]);
    }

    #[test]
    fn clicks_postpone_room_gags() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        // um clique a cada 10 s: nunca fica ocioso o bastante
        for k in 0..600 {
            let input = if k % 100 == 0 {
                vec![click(600, 470)]
            } else {
                vec![]
            };
            r.update(&input, 0.1, &a);
            assert!(!r.gags.is_playing(), "tocou no passo {k}");
        }
    }
    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(SoundRef::Snd(_, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// Bolsa Mágica: abre e fecha; a maleta abre a pasta por cima da sala, o
    /// livro conta a história e a varinha vira o cursor, com o balão dela.
    #[test]
    fn magic_bag_and_its_contents() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        assert!(!r.bag().open);
        r.update(&[click(300, 450)], 0.0, &a); // fecha o balão de chegada
        let fx = r.update(&[click(150, 425)], 0.0, &a);
        assert!(r.bag().open);
        assert_eq!(sounds(&fx), [objects::SND_BAG_OPEN]);
        let fx = r.update(&[click(310, 430)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::Portfolio)));
        assert_eq!(sounds(&fx), [objects::SND_SUITCASE]);
        let fx = r.update(&[click(400, 400)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::BookStory)));
        let fx = r.update(&[click(250, 445)], 0.0, &a);
        assert!(r.bag().holding_wand);
        assert!(!r.bag().open);
        assert!(r.balloon_open());
        assert!(fx.contains(&Effect::SetCursor(objects::WAND_CURSOR)));
        assert_eq!(sounds(&fx), [objects::SND_WAND_TAKE]);
        // a bolsa recebe a varinha de volta e se abre
        let fx = r.update(&[click(150, 410)], 0.0, &a);
        assert!(!r.bag().holding_wand);
        assert!(r.bag().open);
        assert!(fx.contains(&Effect::SetCursor(DEFAULT_CURSOR)));
        let fx = r.update(&[click(150, 440)], 0.0, &a);
        assert!(!r.bag().open);
        assert_eq!(sounds(&fx), [objects::SND_BAG_CLOSE]);
    }

    /// Sem os roteiros, a varinha no buraco do rato só volta para a bolsa;
    /// o clique atravessa o balão dela, como no original.
    #[test]
    fn wand_without_data_returns_to_the_bag() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Studio, None, None, &a);
        assert!(r.bag().open, "no Ateliê a bolsa já chega aberta");
        r.update(&[click(600, 470)], 0.0, &a);
        r.update(&[click(370, 430)], 0.0, &a);
        assert!(r.bag().holding_wand && r.balloon_open());
        let fx = r.update(&[click(490, 350)], 0.0, &a);
        assert!(!r.balloon_open());
        assert!(!r.bag().holding_wand);
        assert!(r.bag().open);
        assert_eq!(sounds(&fx), [objects::SND_WAND_HOLE]);
        assert!(fx.contains(&Effect::SetCursor(DEFAULT_CURSOR)));
        assert_eq!(r.floor(), Floor::Studio);
    }

    /// Mastro sobe, abertura no chão desce, cada um com o seu som; skates
    /// levam à rua (saguão) e ao elevador (andares).
    #[test]
    fn pole_hole_and_skates() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        let fx = r.update(&[click(300, 450), click(606, 200)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Studio);
        assert_eq!(sounds(&fx), [objects::SND_POLE_UP]);
        let fx = r.update(&[click(300, 30), click(610, 412)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        assert_eq!(sounds(&fx), [objects::SND_POLE_DOWN]);
        let fx = r.update(&[click(300, 450), click(540, 445)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Street)));
        assert_eq!(sounds(&fx), [objects::SND_SKATE]);
        // portas do elevador
        let fx = r.update(&[click(330, 250)], 0.0, &a);
        assert!(r.in_elevator());
        assert_eq!(sounds(&fx), [elevator::SND_DOORS, elevator::SND_ARRIVE]);
        let mut f = Frame::new(640, 480);
        r.draw(&mut f, &a);
        // skate "ELEVADOR" da Oficina, depois o andar 2 no painel
        let mut r = Room::new(Floor::Projects, None, None, &a);
        let fx = r.update(&[click(300, 450), click(500, 440)], 0.0, &a);
        assert!(r.in_elevator());
        assert_eq!(sounds(&fx), [objects::SND_SKATE, elevator::SND_ARRIVE]);
        r.update(&[click(595, 212)], 0.0, &a);
        for _ in 0..30 {
            r.update(&[], 0.1, &a);
        }
        assert!(!r.in_elevator());
        assert_eq!(r.floor(), Floor::Studio);
        assert!(r.balloon_open(), "balão de chegada do Ateliê");
    }

    /// "Tchau" no elevador pergunta antes de sair.
    #[test]
    fn elevator_tchau_asks() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Tricks, None, None, &a);
        r.update(&[click(300, 450), click(490, 440)], 0.0, &a);
        assert!(r.in_elevator());
        r.update(&[click(70, 110)], 0.0, &a);
        assert!(r.balloon_open());
        let (bx, by, _, _) = r.balloon.as_ref().unwrap().option_rect(&a, 0);
        let fx = r.update(&[click(bx + 20, by + 3)], 0.0, &a);
        assert!(fx.contains(&Effect::Quit));
    }

    /// Com os dados reais: a varinha no mastro do saguão faz um bicho descer
    /// (ou subir) pelo mastro sem mudar de andar; cliques esperam; no fim a
    /// bolsa está aberta com a varinha dentro.
    #[test]
    fn wand_surprise_on_the_pole_with_real_data() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.update(&[click(300, 450), click(150, 425)], 0.0, &a);
        r.update(&[click(250, 445)], 0.0, &a);
        let fx = r.update(&[click(606, 200)], 0.0, &a);
        assert!(r.surprise_playing());
        assert_eq!(r.floor(), Floor::Lobby);
        assert!(sounds(&fx).contains(&objects::SND_POLE_UP));
        let mut f = Frame::new(640, 480);
        r.update(&[], 0.5, &a);
        r.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "lobby_wand_pole");
        // clique durante a surpresa não troca de andar
        r.update(&[click(140, 66)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        for _ in 0..40 {
            r.update(&[], 0.1, &a);
        }
        assert!(!r.surprise_playing());
        assert!(r.bag().open && !r.bag().holding_wand);
        r.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "lobby_bag_open");
        // a bolsa aberta no chão: a maleta vermelha em (300, 430)
        let p = f.pixel(300, 430);
        assert!(p[0] > 150 && p[1] < 80, "{p:?}");
    }
}
