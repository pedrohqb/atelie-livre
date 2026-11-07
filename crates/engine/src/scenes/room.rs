//! Salas do prédio do Fine Artist: saguão, Ateliê de Pintura, Oficina de
//! Projetos, Truques de Desenho e o sótão escuro. Fundo e elementos fixos vêm
//! de `rooms_data` (medidos no original); as áreas clicáveis estão aqui. Em
//! cada andar há um personagem parado à esquerda (McZee com uma fantasia
//! sorteada no saguão e no Ateliê; Maggie nos andares 3 e 4) e, de tempos em
//! tempos, uma animação ambiente do andar (`gags`). Os objetos do chão
//! (Bolsa Mágica, maleta, livro, Varinha Mágica) estão em `room_objects`; o
//! elevador, em `elevator`; os balões, em `room_talk`; o sótão, em `attic`.
//!
//! Como no original, as opções do balão de chegada só apontam (abrem uma
//! dica sobre o objeto); os objetos da Oficina e dos Truques abrem um balão
//! de escolha, que Ctrl+clique pula; o cavalete e o quadro da parede do
//! Ateliê mostram a dica no clique simples e abrem a pintura no duplo; o
//! mastro de bombeiro dá a volta por cinco posições (saguão, Ateliê,
//! Projetos, Truques, sótão): o mastro sobe uma, a abertura no chão desce
//! uma e Ctrl anda duas.

use super::attic;
use super::elevator::{self, Elevator, Outcome};
use super::gags::{self, Character, GagPlayer};
use super::projects::ProjectKind;
use super::room_objects::{self as objects, BagLayout, BagPart, MagicBag, Surprises, Target, snd};
use super::room_talk::{self as talk, Helper, Pick, Start, rec, sound};
use super::rooms_data::{self, Element};
use super::studio_wall::{self, End, NameField};
use crate::anim::Animator;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::Balloon;
use crate::paint::Canvas;
use crate::paint::store::{LAST_NAME, load_name, load_png};
use crate::rng::XorShift;
use crate::scene::{DEFAULT_CURSOR, Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::text_balloon::TextBalloon;
use crate::{Frame, SCREEN_H, SCREEN_W};
use formats::palette::Rgb;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Floor {
    Lobby,
    Studio,
    Projects,
    Tricks,
    /// Sótão escuro acima dos Truques (só pelo mastro).
    Attic,
}

type Rect = (i32, i32, i32, i32);

/// O que um clique numa área faz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// Fecha o balão.
    Close,
    /// Andar do quadro do saguão (nome ou figura): fala `304`.
    Floor(Floor),
    /// Galeria pelo saguão, com o som (quadro `304`, seta `2fb`).
    Gallery(u16),
    /// Luminária ou "Tchau": o "EPA!" (Sair / Continuar).
    Quit,
    /// Pergunta "Botão ou adesivo?" num segundo balão.
    AskButton,
    /// Mastro de bombeiro: sobe um andar (Ctrl: dois).
    PoleUp,
    /// Abertura do mastro no chão: desce um andar (Ctrl: dois).
    PoleDown,
    /// Buraco do rato (com a Varinha Mágica, a surpresa).
    MouseHole,
    /// Portas do elevador, botão de chamar ou skate "ELEVADOR".
    Elevator,
    /// Skate "LÁ FORA" do saguão: volta para a rua.
    Outside,
    /// Cavalete do Ateliê: dica; clique duplo abre um quadro novo.
    Easel,
    /// Quadro da parede do Ateliê: dica; clique duplo abre o último quadro.
    Wall,
    /// Plaqueta do quadro da parede: troca o nome do último quadro.
    WallLabel,
    /// Objeto com balão de escolha (vaca, cobra, porco, estátuas).
    Pick(Pick),
    /// Moldura dos Truques: troca a obra-prima.
    Painting,
    /// Dica do `WHAT 100d` (opções do balão de chegada).
    Tip(usize),
}

impl Action {
    /// Objetos que respondem com balão: clicar num deles com outro balão
    /// aberto troca direto para o balão dele.
    fn talks(self) -> bool {
        matches!(
            self,
            Action::Easel | Action::Wall | Action::Pick(_) | Action::Painting | Action::Quit
        )
    }
}

/// De que é o balão aberto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Talk {
    Arrival,
    Tip,
    Pick(Pick),
    /// Menu do McZee ou da Maggie; `off` = "Que bom que você me chamou de novo".
    Helper {
        off: bool,
    },
    Wand,
    AskButton,
}

struct Hotspot {
    rect: Rect,
    action: Action,
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

/// As cinco posições do mastro, na ordem em que ele sobe.
const CYCLE: [Floor; 5] = [
    Floor::Lobby,
    Floor::Studio,
    Floor::Projects,
    Floor::Tricks,
    Floor::Attic,
];

impl Floor {
    pub(crate) fn background(self) -> u16 {
        match self {
            Floor::Lobby => rooms_data::LOBBY_BG,
            Floor::Studio => rooms_data::STUDIO_BG,
            Floor::Projects => rooms_data::PROJECTS_BG,
            Floor::Tricks | Floor::Attic => rooms_data::TRICKS_BG,
        }
    }

    fn elements(self) -> &'static [Element] {
        match self {
            Floor::Lobby => rooms_data::LOBBY,
            Floor::Studio => rooms_data::STUDIO,
            Floor::Projects => rooms_data::PROJECTS,
            Floor::Tricks => rooms_data::TRICKS,
            Floor::Attic => &[],
        }
    }

    /// O andar `d` posições acima pelo mastro (negativo: abaixo), com volta.
    pub fn step(self, d: i32) -> Floor {
        let i = CYCLE.iter().position(|f| *f == self).unwrap_or(0) as i32;
        CYCLE[(i + d).rem_euclid(CYCLE.len() as i32) as usize]
    }

    /// Áreas clicáveis, medidas no original; a primeira que contém o ponto vale.
    fn hotspots(self) -> Vec<Hotspot> {
        let h = |rect, action| Hotspot { rect, action };
        match self {
            Floor::Lobby => vec![
                // quadro de andares: os nomes e os andares da figura do
                // prédio (a setinha "Galeria" fica no térreo dela)
                h((98, 38, 90, 18), Action::Floor(Floor::Tricks)),
                h((98, 58, 90, 18), Action::Floor(Floor::Projects)),
                h((98, 78, 90, 18), Action::Floor(Floor::Studio)),
                h((98, 98, 90, 18), Action::Gallery(talk::snd::STUDIO_ARRIVAL)),
                h((12, 18, 86, 35), Action::Floor(Floor::Tricks)),
                h((12, 53, 86, 22), Action::Floor(Floor::Projects)),
                h((12, 75, 86, 21), Action::Floor(Floor::Studio)),
                h((10, 96, 88, 22), Action::Gallery(talk::snd::STUDIO_ARRIVAL)),
                h((513, 346, 54, 47), Action::MouseHole),
                // abertura do mastro no chão, à direita do skate
                h((560, 390, 80, 20), Action::PoleDown),
                h((584, 410, 56, 16), Action::PoleDown),
                h((502, 411, 81, 66), Action::Outside),
                h((480, 202, 36, 71), Action::Elevator),
                h((210, 130, 260, 270), Action::Elevator),
                // topo do arco da porta
                h((236, 64, 210, 66), Action::Elevator),
                h((146, 290, 48, 20), Action::Quit),
                h((148, 183, 52, 105), Action::Quit),
                h((95, 316, 95, 52), Action::Gallery(talk::snd::GALLERY_ARROW)),
                h((594, 0, 26, 390), Action::PoleUp),
            ],
            Floor::Studio => vec![
                h((283, 63, 158, 120), Action::Wall),
                h(studio_wall::LABEL, Action::WallLabel),
                // tela e plaqueta do cavalete, poste de cima e pés
                h((105, 135, 186, 165), Action::Easel),
                h((150, 300, 100, 92), Action::Easel),
                h((466, 328, 54, 47), Action::MouseHole),
                h((466, 407, 81, 66), Action::Elevator),
                h((531, 390, 105, 43), Action::PoleDown),
                h((575, 0, 24, 395), Action::PoleUp),
                h((474, 250, 46, 20), Action::Quit),
                h((473, 146, 52, 104), Action::Quit),
            ],
            Floor::Projects => vec![
                h((479, 289, 54, 47), Action::MouseHole),
                h((466, 407, 81, 66), Action::Elevator),
                h((529, 371, 105, 43), Action::PoleDown),
                h((573, 0, 24, 375), Action::PoleUp),
                h((478, 185, 50, 20), Action::Quit),
                h((478, 72, 54, 112), Action::Quit),
                h((40, 20, 145, 165), Action::Pick(Pick::Comics)),
                h((255, 0, 135, 120), Action::Pick(Pick::Flipbook)),
                h((150, 165, 240, 120), Action::Pick(Pick::Sticker)),
            ],
            Floor::Tricks => vec![
                h((8, 8, 110, 134), Action::Painting),
                h((576, 273, 54, 47), Action::MouseHole),
                h((457, 409, 81, 66), Action::Elevator),
                h((495, 355, 105, 43), Action::PoleDown),
                h((539, 0, 24, 360), Action::PoleUp),
                h((564, 114, 50, 20), Action::Quit),
                h((565, 5, 50, 108), Action::Quit),
                h((325, 100, 200, 245), Action::Pick(Pick::ThreeD)),
                h((150, 40, 150, 290), Action::Pick(Pick::Basic)),
            ],
            // o resto do escuro só toca um som
            Floor::Attic => vec![h(attic::RING, Action::PoleUp)],
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

/// Nome usado no lugar do `@` dos balões quando não há usuário escolhido.
pub const DEFAULT_USER: &str = "artista";

impl Floor {
    /// Balão de chegada ao andar: o do `WHAT` do original quando os dados
    /// estão instalados, senão o texto reserva. O sótão não tem.
    fn balloon(self, assets: &Assets, user: &str) -> Option<Balloon> {
        let k = match self {
            Floor::Lobby => 1,
            Floor::Studio => 26,
            Floor::Projects => 31,
            Floor::Tricks => 42,
            Floor::Attic => return None,
        };
        let fallback = self.fallback_balloon();
        let records = assets.dialogs(Program::Artist, talk::ROOMS_WHAT);
        Some(match records.get(k) {
            Some(r) if r.options().len() == fallback.options.len() => {
                Balloon::from_block(&records, k, fallback.x, fallback.y, user).unwrap_or(fallback)
            }
            _ => fallback,
        })
    }

    /// Balão do McZee ou da Maggie ao chegar ao andar (textos do original,
    /// usados quando o `ARTIST.EXE` não está instalado).
    fn fallback_balloon(self) -> Balloon {
        let (x, y, w) = match self {
            Floor::Lobby => (300, 20, 300),
            Floor::Studio => (130, 230, 200),
            Floor::Projects => (70, 150, 300),
            Floor::Tricks | Floor::Attic => (70, 120, 260),
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
            Floor::Tricks | Floor::Attic => (
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
            figures: Vec::new(),
        }
    }

    /// Ação da opção `k` do balão de chegada: no saguão leva ao andar; nos
    /// andares só aponta (dica ou balão do objeto), como no original.
    fn balloon_action(self, k: usize) -> Action {
        match (self, k) {
            (Floor::Lobby, 0) => Action::Tip(rec::LOBBY_STUDIO),
            (Floor::Lobby, 1) => Action::Tip(rec::LOBBY_PROJECTS),
            (Floor::Lobby, 2) => Action::Tip(rec::LOBBY_TRICKS),
            (Floor::Lobby, _) => Action::Tip(rec::LOBBY_GALLERY),
            (Floor::Studio, 0) => Action::Tip(rec::STUDIO_NEW),
            (Floor::Studio, 1) => Action::Tip(rec::STUDIO_LAST),
            (Floor::Studio, 2) => Action::Tip(rec::STUDIO_PORTFOLIO),
            (Floor::Studio, _) => Action::Tip(rec::STUDIO_POLE),
            (Floor::Projects, 0) => Action::Pick(Pick::Comics),
            (Floor::Projects, 1) => Action::Pick(Pick::Sticker),
            (Floor::Projects, 2) => Action::Pick(Pick::Flipbook),
            (Floor::Projects, _) => Action::Tip(rec::PROJECTS_POLE),
            (Floor::Tricks, 0) => Action::Pick(Pick::Basic),
            (Floor::Tricks, 1) => Action::Pick(Pick::ThreeD),
            (Floor::Tricks, _) => Action::Tip(rec::TRICKS_POLE),
            (Floor::Attic, _) => Action::Close,
        }
    }

    /// Roteiros `ATAG` das animações ambientes do andar.
    fn gag_scripts(self) -> &'static [u16] {
        match self {
            Floor::Lobby => gags::LOBBY_SCRIPTS,
            Floor::Studio => gags::STUDIO_SCRIPTS,
            Floor::Projects => gags::PROJECTS_SCRIPTS,
            Floor::Tricks => gags::TRICKS_SCRIPTS,
            Floor::Attic => &[],
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
            Floor::Attic => None,
        }
    }

    /// Onde fica o menu do personagem (medido no original).
    fn helper_pos(self) -> (i32, i32) {
        match self {
            Floor::Lobby => (119, 221),
            Floor::Studio => (130, 232),
            Floor::Projects | Floor::Tricks | Floor::Attic => (109, 132),
        }
    }

    fn helper(self) -> Option<Helper> {
        match self {
            Floor::Lobby | Floor::Studio => Some(Helper::McZee),
            Floor::Projects | Floor::Tricks => Some(Helper::Maggie),
            Floor::Attic => None,
        }
    }

    /// Onde fica o "EPA!" da luminária (medido no original).
    fn quit_pos(self) -> (i32, i32) {
        match self {
            Floor::Studio => (215, 180),
            Floor::Projects => (214, 180),
            Floor::Tricks => (214, 200),
            Floor::Lobby | Floor::Attic => (214, 179),
        }
    }

    pub fn number(self) -> u8 {
        match self {
            Floor::Lobby => 1,
            Floor::Studio => 2,
            Floor::Projects => 3,
            Floor::Tricks => 4,
            Floor::Attic => 5,
        }
    }

    fn bag_layout(self) -> BagLayout {
        match self {
            Floor::Lobby => objects::LOBBY_BAG,
            Floor::Studio => objects::STUDIO_BAG,
            Floor::Projects => objects::PROJECTS_BAG,
            Floor::Tricks | Floor::Attic => objects::TRICKS_BAG,
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
            Floor::Attic => Surprises {
                pole: &[],
                pole_anchor: (0, 0),
                hole: &[],
                hole_anchor: (0, 0),
            },
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
            figures: Vec::new(),
        };
        let mut b = Balloon::from_block(
            &assets.dialogs(Program::Artist, talk::ROOMS_WHAT),
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

/// Quadro da parede do Ateliê: o último desenho, reduzido a 1/4, e a
/// plaqueta com o nome; plaqueta do cavalete ("Quadro Novo"). Medido no
/// original: miolo branco da parede (287,65)-(434,151); texto das
/// plaquetas centrado na parte branca.
const WALL_PIC: (i32, i32) = (287, 65);
const WALL_SCALE: u32 = 4;
const WALL_LABEL: (i32, i32) = (363, 190);
const EASEL_LABEL: (i32, i32) = (197, 277);

/// Por onde se chegou a um andar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Via {
    Pole,
    Board,
    Elevator,
}

/// Menu do McZee dentro do elevador.
const ELEVATOR_HELPER: (i32, i32) = (117, 102);

/// Dois cliques no mesmo ponto dentro deste tempo são um clique duplo.
const DOUBLE_CLICK: f32 = 0.4;
const DOUBLE_SLACK: i32 = 6;

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
    /// Nome do último quadro (plaqueta da parede do Ateliê).
    last_name: Option<String>,
    pal: Vec<Rgb>,
    drawings: Option<PathBuf>,
    balloon: Option<Balloon>,
    /// De que é o balão aberto.
    talk: Talk,
    /// Dica aberta por cima do balão (lâmpada, tecla de atalho, glossário):
    /// o próximo clique a fecha.
    sub: Option<Balloon>,
    /// "EPA!" da luminária e do "Tchau": modal, só as bolinhas respondem.
    quit: Option<TextBalloon>,
    /// Ctrl apertado.
    ctrl: bool,
    /// Relógio da sala (s), para o clique duplo.
    clock: f32,
    /// Último clique simples no cavalete ou na parede: ação, hora e ponto.
    last_click: Option<(Action, f32, i32, i32)>,
    /// "Você quer me ver menos? — Sim": sem balões de chegada até religar.
    help_off: bool,
    /// Plaqueta da parede em edição (novo nome do último quadro).
    naming: Option<NameField>,
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
        // um último quadro todo em branco é como nenhum (parede vazia)
        let last_drawing = drawings
            .as_ref()
            .and_then(|d| load_png(&d.join(LAST_NAME), &pal))
            .filter(|c| c.px.iter().any(|&v| v != crate::paint::WHITE));
        let last_name = drawings.as_deref().and_then(load_name);
        let mut rng = XorShift::from_clock();
        let character = floor.character(&mut rng);
        let gags = GagPlayer::with_rng(floor.gag_scripts(), GAG_FIRST, GAG_BETWEEN, rng.clone());
        Room {
            common: KidCommon::new(click),
            hotspots: floor.hotspots(),
            last_drawing,
            last_name,
            pal,
            drawings,
            balloon: floor.balloon(assets, DEFAULT_USER),
            talk: Talk::Arrival,
            sub: None,
            quit: None,
            ctrl: false,
            clock: 0.0,
            last_click: None,
            help_off: false,
            naming: None,
            floor,
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
        self.balloon = self.floor.balloon(assets, &self.user);
        self.talk = Talk::Arrival;
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

    /// O "EPA!" (Sair / Continuar) está aberto.
    pub fn quit_open(&self) -> bool {
        self.quit.is_some()
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

    /// A ajuda do McZee / da Maggie está desligada.
    pub fn help_off(&self) -> bool {
        self.help_off
    }

    fn show(&mut self, b: Balloon, t: Talk) {
        self.balloon = Some(b);
        self.talk = t;
        self.sub = None;
    }

    fn close(&mut self) {
        self.balloon = None;
        self.sub = None;
    }

    fn show_arrival(&mut self, assets: &Assets) {
        match self.floor.balloon(assets, &self.user) {
            Some(b) => self.show(b, Talk::Arrival),
            None => self.close(),
        }
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
                let b = self.floor.wand_balloon(assets, &self.user);
                self.show(b, Talk::Wand);
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
        let mut e = Elevator::new(self.floor, costume, assets, &self.user);
        e.set_ask(!self.help_off);
        self.elevator = Some(e);
        self.close();
        self.bag.holding_wand = false;
    }

    /// Abre o "EPA!" por cima da sala, com a vinheta.
    fn open_quit(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        fx.extend(talk::snd::QUIT.map(|id| Effect::PlaySound(sound(id))));
        self.close();
        self.quit = Some(talk::quit_balloon(assets, self.floor.quit_pos()));
    }

    /// Clique com o "EPA!" aberto: só as bolinhas respondem.
    fn quit_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(q) = &self.quit else { return };
        match q.hit_option(assets, x, y) {
            Some(0) => {
                fx.push(Effect::PlaySound(sound(talk::snd::CHOICE)));
                fx.push(Effect::Quit);
            }
            Some(_) => {
                fx.push(Effect::PlaySound(sound(talk::snd::CHOICE)));
                self.quit = None;
            }
            None => {
                if !q.contains(assets, x, y) {
                    fx.push(Effect::PlaySound(sound(talk::snd::REFUSE)));
                }
            }
        }
    }

    /// Clique no McZee ou na Maggie (na sala ou no elevador): o menu
    /// "Você quer me ver menos?" (ou "Que bom que você me chamou de novo").
    fn open_helper(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        let (h, pos) = if self.elevator.is_some() {
            (Helper::McZee, ELEVATOR_HELPER)
        } else {
            match self.floor.helper() {
                Some(h) => (h, self.floor.helper_pos()),
                None => return,
            }
        };
        fx.push(Effect::PlaySound(sound(h.sound())));
        let off = self.help_off;
        let b = h.balloon(assets, &self.user, off, pos);
        self.show(b, Talk::Helper { off });
    }

    /// "Diga de novo" e "Ajudar": o balão de chegada (no elevador, o
    /// "Subindo? Ou descendo?").
    fn repeat_arrival(&mut self, assets: &Assets) {
        match self.elevator.as_mut() {
            Some(e) => {
                self.balloon = None;
                e.set_ask(true);
            }
            None => self.show_arrival(assets),
        }
    }

    /// Começa um trabalho novo do objeto (escolha "criar novo" ou Ctrl+clique).
    fn start_pick(&mut self, p: Pick, fx: &mut Vec<Effect>, assets: &Assets) {
        match p.start() {
            Start::Scene(id) => fx.push(Effect::ChangeScene(id)),
            Start::AskButton => self.act(Action::AskButton, fx, assets),
        }
    }

    /// Opção `k` do balão aberto.
    fn choose(&mut self, k: usize, fx: &mut Vec<Effect>, assets: &Assets) {
        // "Ignorar isto" (última linha do balão de escolha): a dica do Ctrl
        if let Talk::Pick(p) = self.talk
            && k == talk::IGNORE
        {
            self.open_link(p.skip_tip(), fx, assets);
            return;
        }
        fx.push(Effect::PlaySound(sound(talk::snd::CHOICE)));
        match self.talk {
            Talk::Arrival => {
                let action = self.floor.balloon_action(k);
                self.act(action, fx, assets);
            }
            Talk::Pick(p) => {
                self.close();
                if k == 0 {
                    self.start_pick(p, fx, assets);
                } else {
                    fx.push(Effect::PlaySound(sound(talk::snd::PORTFOLIO)));
                    fx.push(Effect::Push(SceneId::Portfolio));
                }
            }
            Talk::Helper { off: false } => match k {
                0 => {
                    self.help_off = true;
                    self.close();
                }
                1 => self.repeat_arrival(assets),
                _ => self.close(),
            },
            Talk::Helper { off: true } => {
                if k == 0 {
                    self.help_off = false;
                    self.repeat_arrival(assets);
                } else {
                    self.close();
                }
            }
            Talk::AskButton => {
                let kind = if k == 0 {
                    ProjectKind::Button
                } else {
                    ProjectKind::Sticker
                };
                self.close();
                fx.push(Effect::ChangeScene(SceneId::Project(kind)));
            }
            Talk::Tip | Talk::Wand => self.close(),
        }
    }

    /// Ligação (dica, tecla de atalho, glossário): abre por cima do balão.
    fn open_link(&mut self, k: usize, fx: &mut Vec<Effect>, assets: &Assets) {
        fx.push(Effect::PlaySound(sound(talk::link_sound(k))));
        self.sub = Some(talk::tip(assets, k, &self.user));
    }

    /// Palavra "tridimensional" do balão de chegada dos Truques (glossário).
    fn glossary_hit(&self, b: &Balloon, x: i32, y: i32, assets: &Assets) -> bool {
        self.talk == Talk::Arrival
            && self.floor == Floor::Tricks
            && b.option_word_rect(assets, 1, "tridimensional")
                .is_some_and(|r| inside(r, x, y))
    }

    fn act(&mut self, action: Action, fx: &mut Vec<Effect>, assets: &Assets) {
        match action {
            Action::Floor(fl) => {
                fx.push(Effect::PlaySound(sound(talk::snd::STUDIO_ARRIVAL)));
                self.set_floor(fl, Via::Board, fx, assets);
            }
            Action::Gallery(id) => {
                fx.push(Effect::PlaySound(sound(id)));
                fx.push(Effect::ChangeScene(SceneId::Gallery));
            }
            Action::PoleUp => {
                fx.push(Effect::PlaySound(snd(objects::SND_POLE_UP)));
                let to = self.floor.step(if self.ctrl { 2 } else { 1 });
                self.set_floor(to, Via::Pole, fx, assets);
            }
            Action::PoleDown => {
                fx.push(Effect::PlaySound(snd(objects::SND_POLE_DOWN)));
                let to = self.floor.step(if self.ctrl { -2 } else { -1 });
                self.set_floor(to, Via::Pole, fx, assets);
            }
            Action::MouseHole => fx.push(Effect::PlaySound(sound(talk::snd::MOUSE_HOLE))),
            Action::Elevator => self.enter_elevator(fx, assets),
            Action::Outside => {
                fx.push(Effect::PlaySound(snd(objects::SND_SKATE)));
                fx.push(Effect::ChangeScene(SceneId::Street));
            }
            Action::Quit => self.open_quit(fx, assets),
            Action::Close => self.close(),
            Action::AskButton => {
                self.show(
                    Balloon {
                        x: 183,
                        y: 239,
                        w: 200,
                        text: "Legal! O que você quer fazer?".into(),
                        options: vec!["Um botão".into(), "Um adesivo".into()],
                        links: Vec::new(),
                        figures: Vec::new(),
                    },
                    Talk::AskButton,
                );
            }
            Action::Easel => {
                fx.push(Effect::PlaySound(sound(talk::snd::OBJECT)));
                let b = talk::tip(assets, rec::STUDIO_NEW, &self.user);
                self.show(b, Talk::Tip);
            }
            Action::Wall => {
                fx.push(Effect::PlaySound(sound(talk::snd::OBJECT)));
                let b = talk::tip(assets, rec::STUDIO_LAST, &self.user);
                self.show(b, Talk::Tip);
            }
            Action::Pick(p) => {
                fx.push(Effect::PlaySound(sound(talk::snd::OBJECT)));
                if self.ctrl {
                    self.close();
                    self.start_pick(p, fx, assets);
                } else {
                    fx.push(Effect::PlaySound(sound(talk::snd::PICK)));
                    let b = p.balloon(assets, &self.user);
                    self.show(b, Talk::Pick(p));
                }
            }
            Action::Painting => {
                fx.push(Effect::PlaySound(sound(talk::snd::PAINTING)));
                self.painting = self.floor.painting(&mut self.rng);
            }
            Action::WallLabel => {
                if self.last_drawing.is_some() {
                    fx.push(Effect::PlaySound(sound(talk::snd::OBJECT)));
                    let name = self.wall_name().unwrap_or_default();
                    self.naming = Some(NameField::new(&name));
                    let b = studio_wall::balloon(assets, &self.user);
                    self.show(b, Talk::Tip);
                }
            }
            Action::Tip(k) => {
                let b = talk::tip(assets, k, &self.user);
                self.show(b, Talk::Tip);
            }
        }
    }

    /// Clique duplo no cavalete (quadro novo) ou no quadro da parede (o
    /// último quadro), se este clique completa um.
    fn double_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>) -> bool {
        let Some((action, t, px, py)) = self.last_click else {
            return false;
        };
        if self.clock - t > DOUBLE_CLICK
            || (x - px).abs() > DOUBLE_SLACK
            || (y - py).abs() > DOUBLE_SLACK
        {
            return false;
        }
        self.last_click = None;
        fx.push(Effect::PlaySound(sound(talk::snd::OBJECT)));
        fx.push(Effect::ChangeScene(match action {
            Action::Easel => SceneId::PaintNew,
            _ => SceneId::Paint,
        }));
        true
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

    /// Muda de andar; `via` diz por onde (a fala `304` do quadro do saguão
    /// toca no clique; pelo elevador, ao chegar ao Ateliê).
    fn set_floor(&mut self, floor: Floor, via: Via, fx: &mut Vec<Effect>, assets: &Assets) {
        self.floor = floor;
        self.hotspots = floor.hotspots();
        self.close();
        self.quit = None;
        if !self.help_off {
            self.show_arrival(assets);
        }
        self.character = floor.character(&mut self.rng);
        self.painting = floor.painting(&mut self.rng);
        self.bag = floor.bag();
        self.surprise = None;
        self.last_click = None;
        self.naming = None;
        self.gags = GagPlayer::with_rng(
            floor.gag_scripts(),
            GAG_FIRST,
            GAG_BETWEEN,
            XorShift::new(self.rng.next_u32()),
        );
        match floor {
            Floor::Attic => fx.push(Effect::PlaySound(sound(attic::SND_ARRIVE))),
            Floor::Studio if via == Via::Elevator => {
                fx.push(Effect::PlaySound(sound(talk::snd::STUDIO_ARRIVAL)))
            }
            _ => {}
        }
    }

    /// Um clique de botão esquerdo em (x, y) fora do elevador; `true` se ele
    /// foi usado (os seguintes do mesmo quadro esperam).
    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) -> bool {
        if self.quit.is_some() {
            self.quit_click(x, y, fx, assets);
            return true;
        }
        if self.double_click(x, y, fx) {
            return true;
        }
        if self.sub.take().is_some() {
            return true;
        }
        if let Some(b) = &self.balloon {
            if self.glossary_hit(b, x, y, assets) {
                self.open_link(rec::GLOSS_3D, fx, assets);
                return true;
            }
            if let Some(k) = b.hit(assets, x, y) {
                self.choose(k, fx, assets);
                return true;
            }
            if let Some(k) = b.link_hit(assets, x, y) {
                self.open_link(k, fx, assets);
                return true;
            }
            if b.contains(assets, x, y) {
                return false;
            }
            // o menu do personagem é modal: fora dele, a vinheta de recusa
            if matches!(self.talk, Talk::Helper { .. }) {
                fx.push(Effect::PlaySound(sound(talk::snd::REFUSE)));
                return true;
            }
            // clicar fora do balão o fecha; o da varinha deixa o clique
            // seguir (ela já está na mão), e um objeto com balão abre o dele
            let pass = (self.talk == Talk::Wand && self.bag.holding_wand)
                || self.helper_hit(x, y, assets)
                || self.hotspot_at(x, y).is_some_and(Action::talks);
            self.close();
            if !pass {
                return false;
            }
        }
        if self.elevator.is_some() {
            return false;
        }
        if self.floor != Floor::Attic
            && let Some(part) = self.bag.hit(x, y)
        {
            self.use_bag(part, fx, assets);
            return true;
        }
        if !self.bag.holding_wand && self.helper_hit(x, y, assets) {
            self.open_helper(fx, assets);
            return true;
        }
        let Some(action) = self.hotspot_at(x, y) else {
            if self.floor == Floor::Attic {
                fx.push(Effect::PlaySound(sound(attic::dark_sound(&mut self.rng))));
                return true;
            }
            return false;
        };
        match (self.bag.holding_wand, action) {
            (true, Action::PoleUp | Action::PoleDown) => {
                self.start_surprise(Target::Pole, fx, assets)
            }
            (true, Action::MouseHole) => self.start_surprise(Target::Hole, fx, assets),
            (_, action) => {
                if matches!(action, Action::Easel | Action::Wall) {
                    self.last_click = Some((action, self.clock, x, y));
                }
                self.act(action, fx, assets)
            }
        }
        true
    }

    fn hotspot_at(&self, x: i32, y: i32) -> Option<Action> {
        self.hotspots
            .iter()
            .find(|h| inside(h.rect, x, y))
            .map(|h| h.action)
    }

    fn helper_hit(&self, x: i32, y: i32, assets: &Assets) -> bool {
        self.character.as_ref().is_some_and(|c| c.hit(assets, x, y))
    }

    /// Nome na plaqueta da parede: o do último quadro, ou "Quadro Novo" se
    /// ele não tem nome; nada sem último quadro.
    fn wall_name(&self) -> Option<String> {
        self.last_drawing.as_ref().map(|_| {
            self.last_name
                .clone()
                .unwrap_or_else(|| super::paint::rename::DEFAULT_NAME.to_string())
        })
    }

    /// Termina a edição da plaqueta: troca o nome (ou desiste).
    fn finish_naming(&mut self, end: End, fx: &mut Vec<Effect>) {
        let Some(field) = self.naming.take() else {
            return;
        };
        if self.talk == Talk::Tip {
            self.close();
        }
        let name = field.name();
        if end == End::Cancel || name.is_empty() || Some(&name) == self.last_name.as_ref() {
            return;
        }
        let Some(dir) = &self.drawings else { return };
        match studio_wall::rename_last(dir, &self.pal, &name) {
            Ok(()) => {
                fx.push(Effect::Log(format!(
                    "último quadro agora se chama \"{name}\""
                )));
                self.last_name = Some(name);
            }
            Err(e) => fx.push(Effect::Log(e)),
        }
    }

    /// Último quadro reduzido no miolo do quadro da parede.
    fn draw_wall_picture(&self, frame: &mut Frame) {
        let Some(c) = &self.last_drawing else { return };
        for y in 0..c.h / WALL_SCALE {
            for x in 0..c.w / WALL_SCALE {
                let i = c.px[((y * WALL_SCALE) * c.w + x * WALL_SCALE) as usize];
                let p = self
                    .pal
                    .get(i as usize)
                    .copied()
                    .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                frame.put(
                    WALL_PIC.0 + x as i32,
                    WALL_PIC.1 + y as i32,
                    [p.r, p.g, p.b],
                );
            }
        }
    }

    /// Nome na plaqueta: o do último quadro (ou "Quadro Novo" se ele não
    /// tem nome) na parede; "Quadro Novo" no cavalete.
    fn draw_labels(&self, frame: &mut Frame, assets: &Assets) {
        let wall = if self.naming.is_some() {
            None
        } else {
            self.wall_name()
        };
        let labels = [
            (wall, WALL_LABEL),
            (
                Some(super::paint::rename::DEFAULT_NAME.to_string()),
                EASEL_LABEL,
            ),
        ];
        let font = assets.font(8);
        for (text, (cx, y)) in labels {
            let Some(t) = text else { continue };
            match &font {
                Some(f) => {
                    let w = f.width(&t) as i32;
                    frame.text_font(f, cx - w / 2, y, &t, [0, 0, 0]);
                }
                None => frame.text(cx - 4 * t.chars().count() as i32, y + 2, &t, [0, 0, 0]),
            }
        }
        if let Some(n) = &self.naming {
            n.draw(frame, font.as_deref());
        }
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
        self.clock += dt;
        for i in input {
            if let Input::Ctrl(down) = i {
                self.ctrl = *down;
            }
        }
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
                // a varinha volta para a bolsa, que fica aberta, e o balão
                // de chegada reaparece
                self.surprise = None;
                self.bag.open = true;
                if !self.help_off && self.balloon.is_none() {
                    self.show_arrival(assets);
                }
            }
        }
        if self.elevator.is_some() && self.balloon.is_none() && self.quit.is_none() {
            let outcome = self
                .elevator
                .as_mut()
                .and_then(|e| e.update(input, dt, assets, &mut fx));
            match outcome {
                Some(Outcome::Arrived(fl)) => {
                    self.elevator = None;
                    self.set_floor(fl, Via::Elevator, &mut fx, assets);
                }
                Some(Outcome::Quit) => self.open_quit(&mut fx, assets),
                Some(Outcome::Helper) => self.open_helper(&mut fx, assets),
                None => {}
            }
            self.sync_cursor(&mut fx);
            return fx;
        }
        for i in input {
            if let Some(n) = self.naming.as_mut() {
                if let Some(end) = n.input(i) {
                    self.finish_naming(end, &mut fx);
                    continue;
                }
                // clicar fora da plaqueta guarda o nome digitado
                if let Input::Click { x, y, .. } = *i
                    && !inside(studio_wall::LABEL, x, y)
                {
                    self.finish_naming(End::Commit, &mut fx);
                }
            }
            let Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } = *i
            else {
                continue;
            };
            if self.naming.is_some() {
                continue;
            }
            // durante a surpresa da varinha os cliques esperam
            if self.surprise.is_some() {
                continue;
            }
            if self.click(x, y, &mut fx, assets) {
                break;
            }
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
            if let Some(q) = &self.quit {
                q.draw(frame, assets);
            }
            return;
        }
        if self.floor == Floor::Attic {
            attic::draw(frame, assets);
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
        if self.floor == Floor::Studio {
            self.draw_wall_picture(frame);
            self.draw_labels(frame, assets);
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
        // por cima de tudo, menos dos balões
        self.gags.draw(frame, assets);
        if let Some(a) = &self.surprise {
            a.draw(frame, assets);
        }
        if let Some(b) = &self.balloon {
            b.draw(frame, assets);
        }
        if let Some(b) = &self.sub {
            b.draw(frame, assets);
        }
        if let Some(q) = &self.quit {
            q.draw(frame, assets);
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

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(SoundRef::Snd(_, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    fn option(r: &Room, a: &Assets, k: usize) -> Input {
        let (bx, by, _, _) = r.balloon.as_ref().unwrap().option_rect(a, k);
        click(bx + 20, by + 3)
    }

    fn quit_option(r: &Room, a: &Assets, k: usize) -> Input {
        let (bx, by, _, _) = r.quit.as_ref().unwrap().option_rect(a, k).unwrap();
        click(bx + 4, by + 3)
    }

    fn link(r: &Room, a: &Assets, k: usize) -> Input {
        let (bx, by, _, _) = r.balloon.as_ref().unwrap().link_rect(a, k);
        click(bx + 20, by + 3)
    }

    fn changes(fx: &[Effect]) -> Vec<SceneId> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::ChangeScene(id) => Some(*id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn lobby_directory_and_pole_tour() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        assert!(r.balloon_open());
        // clique fora do balão fecha; depois os pontos quentes funcionam
        r.update(&[click(140, 300)], 0.0, &a);
        assert!(!r.balloon_open());
        let fx = r.update(&[click(140, 86)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Studio);
        assert!(r.balloon_open());
        // pelo saguão, a fala do McZee no Ateliê
        assert_eq!(sounds(&fx), [talk::snd::STUDIO_ARRIVAL]);
        r.update(&[click(600, 470), click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
        r.update(&[click(600, 470), click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Tricks);
        // o mastro dos Truques sobe ao sótão escuro; o aro, ao saguão
        let fx = r.update(&[click(600, 470), click(550, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Attic);
        assert!(!r.balloon_open());
        assert_eq!(sounds(&fx), [objects::SND_POLE_UP, attic::SND_ARRIVE]);
        let fx = r.update(&[click(100, 300)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Attic);
        assert_eq!(sounds(&fx).len(), 1, "um som no escuro");
        r.update(&[click(410, 50)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        // opção 2 do balão do saguão: só a dica apontando a Oficina
        let fx = r.update(&[option(&r, &a, 1)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Lobby);
        assert_eq!(sounds(&fx), [talk::snd::CHOICE]);
        assert_eq!((r.talk, r.balloon.as_ref().unwrap().x), (Talk::Tip, 30));
        // a figura do prédio no quadro também leva aos andares, com a fala
        r.update(&[click(600, 470)], 0.0, &a);
        let fx = r.update(&[click(40, 70)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Projects);
        assert_eq!(sounds(&fx), [talk::snd::STUDIO_ARRIVAL]);
    }

    /// Setinha "Galeria" do quadro (fala `304`) e seta grande (`2fb`).
    #[test]
    fn lobby_gallery_arrows() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.update(&[click(300, 470)], 0.0, &a);
        let fx = r.update(&[click(25, 108)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Gallery]);
        assert_eq!(sounds(&fx), [talk::snd::STUDIO_ARRIVAL]);
        let fx = r.update(&[click(140, 340)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Gallery]);
        assert_eq!(sounds(&fx), [talk::snd::GALLERY_ARROW]);
        // topo do arco da porta também abre o elevador
        r.update(&[click(340, 90)], 0.0, &a);
        assert!(r.in_elevator());
    }

    /// No elevador, o McZee abre o mesmo menu (modal); "Sim" desliga a
    /// ajuda e o "Subindo?" não aparece mais.
    #[test]
    fn elevator_mczee_menu() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.update(&[click(300, 470), click(340, 300)], 0.0, &a);
        assert!(r.in_elevator());
        let (mx, my) = (elevator::MCZEE_POS.0 + 10, elevator::MCZEE_POS.1 + 10);
        let fx = r.update(&[click(mx, my)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::MCZEE]);
        assert_eq!(r.talk, Talk::Helper { off: false });
        assert_eq!(
            r.balloon.as_ref().map(|b| (b.x, b.y)),
            Some(ELEVATOR_HELPER)
        );
        // modal: clique fora só dá a recusa
        let fx = r.update(&[click(400, 400)], 0.0, &a);
        assert!(r.balloon_open());
        assert_eq!(sounds(&fx), [talk::snd::REFUSE]);
        r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert!(r.help_off() && !r.balloon_open());
        // de volta ao saguão pela porta: sem balão de chegada
        r.update(&[click(320, 230)], 0.0, &a);
        for _ in 0..15 {
            r.update(&[], 0.1, &a);
        }
        assert!(!r.in_elevator());
        assert_eq!(r.floor(), Floor::Lobby);
        assert!(!r.balloon_open());
    }

    /// O mastro é um ciclo de cinco posições: sobe uma, a abertura desce
    /// uma, Ctrl anda duas, com volta.
    #[test]
    fn pole_cycles_through_five_floors() {
        assert_eq!(Floor::Tricks.step(1), Floor::Attic);
        assert_eq!(Floor::Attic.step(1), Floor::Lobby);
        assert_eq!(Floor::Lobby.step(-1), Floor::Attic);
        assert_eq!(Floor::Lobby.step(-2), Floor::Tricks);
        assert_eq!(Floor::Projects.step(2), Floor::Attic);
        assert_eq!(Floor::Lobby.step(2), Floor::Projects);
        assert_eq!(Floor::Attic.step(2), Floor::Studio);
        let (_d, a) = empty_assets();
        // abertura do mastro do saguão: sótão; com Ctrl, os Truques
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.update(&[click(300, 450), click(620, 398)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Attic);
        let mut r = Room::new(Floor::Lobby, None, None, &a);
        r.update(&[click(300, 450)], 0.0, &a);
        r.update(&[Input::Ctrl(true), click(620, 398)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Tricks);
        // Ctrl+mastro dos Projetos: sótão; Ctrl+aro: Ateliê
        r.update(
            &[Input::Ctrl(false), click(600, 470), click(520, 370)],
            0.0,
            &a,
        );
        assert_eq!(r.floor(), Floor::Projects);
        r.update(&[click(600, 470)], 0.0, &a);
        r.update(&[Input::Ctrl(true), click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Attic);
        r.update(&[click(410, 50)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Studio);
    }

    /// Cavalete: clique simples só mostra a dica; o duplo abre um quadro
    /// novo. Parede: dica; duplo abre o último quadro.
    #[test]
    fn easel_and_wall_need_a_double_click() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[click(600, 470)], 0.0, &a);
        let fx = r.update(&[click(190, 200)], 0.0, &a);
        assert!(changes(&fx).is_empty());
        assert!(r.balloon_open());
        assert_eq!(sounds(&fx), [talk::snd::OBJECT]);
        let fx = r.update(&[click(191, 201)], 0.2, &a);
        assert_eq!(changes(&fx), [SceneId::PaintNew]);
        // devagar demais não é clique duplo
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[click(600, 470), click(350, 100)], 0.0, &a);
        assert!(r.balloon_open());
        let fx = r.update(&[click(350, 100)], 1.0, &a);
        assert!(changes(&fx).is_empty());
        let fx = r.update(&[click(350, 100)], 0.1, &a);
        assert_eq!(changes(&fx), [SceneId::Paint]);
    }

    /// As opções do balão de chegada do Ateliê só apontam (dicas); nenhuma
    /// grava um quadro em branco por cima do último.
    #[test]
    fn studio_arrival_options_only_point() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = a.palettes(Program::Artist).pal256.clone();
        let mut c = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
        c.px.fill(255);
        crate::paint::store::save_drawing(dir.path(), &c, &pal, false, 0).unwrap();
        for k in 0..4 {
            let mut r = Room::new(Floor::Studio, None, Some(dir.path().to_path_buf()), &a);
            let fx = r.update(&[option(&r, &a, k)], 0.0, &a);
            assert!(changes(&fx).is_empty(), "opção {k}");
            assert!(r.balloon_open(), "opção {k}");
            assert_eq!(sounds(&fx), [talk::snd::CHOICE]);
        }
        let last = load_png(&dir.path().join(LAST_NAME), &pal).unwrap();
        assert!(last.px.iter().all(|&v| v == 255));
        // "Outra coisa?": dica do mastro com "Tecla de atalho" e "Descendo?"
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[option(&r, &a, 3)], 0.0, &a);
        assert_eq!(r.balloon.as_ref().unwrap().links.len(), 2);
        let fx = r.update(&[link(&r, &a, 1)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::TIP]);
        assert!(r.sub.is_some());
        // o próximo clique fecha só a dica de cima
        r.update(&[click(5, 5)], 0.0, &a);
        assert!(r.sub.is_none() && r.balloon_open());
    }

    /// Último quadro no quadro da parede, com o nome na plaqueta; o
    /// cavalete fica em branco.
    #[test]
    fn studio_shows_last_drawing_on_the_wall() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut c = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
        c.px.fill(128); // cinza na rampa dos testes
        crate::paint::store::save_drawing_named(
            dir.path(),
            &c,
            &crate::paint::Objects::default(),
            &a.palettes(Program::Artist).pal256,
            true,
            0,
            Some("Fundo Lunar"),
        )
        .unwrap();
        let r = Room::new(Floor::Studio, None, Some(dir.path().to_path_buf()), &a);
        assert_eq!(r.last_name.as_deref(), Some("Fundo Lunar"));
        let mut f = Frame::new(640, 480);
        r.draw(&mut f, &a);
        // o cavalete não muda com o último quadro
        let mut g = Frame::new(640, 480);
        Room::new(Floor::Studio, None, None, &a).draw(&mut g, &a);
        for (x, y) in [(150, 200), (190, 220), (260, 260)] {
            assert_eq!(f.pixel(x, y), g.pixel(x, y));
        }
        assert_ne!(f.pixel(360, 110), g.pixel(360, 110));
    }

    /// Plaqueta da parede: clique abre o campo com o nome selecionado;
    /// digitar e Return trocam o nome do último quadro.
    #[test]
    fn wall_label_renames_the_last_picture() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut c = Canvas::new(crate::paint::CANVAS_W, crate::paint::CANVAS_H);
        c.px.fill(3);
        crate::paint::store::save_drawing_named(
            dir.path(),
            &c,
            &crate::paint::Objects::default(),
            &a.palettes(Program::Artist).pal256,
            true,
            0,
            Some("Casa"),
        )
        .unwrap();
        let mut r = Room::new(Floor::Studio, None, Some(dir.path().to_path_buf()), &a);
        r.update(&[click(600, 470)], 0.0, &a);
        r.update(&[click(360, 196)], 0.0, &a);
        assert_eq!(r.naming.as_ref().map(|n| n.text.as_str()), Some("Casa"));
        assert!(r.balloon_open());
        let enter = Input::Key {
            key: crate::scene::Key::Enter,
            ctrl: false,
            alt: false,
        };
        r.update(&[Input::Text("Lua".into()), enter], 0.0, &a);
        assert!(r.naming.is_none() && !r.balloon_open());
        assert_eq!(r.last_name.as_deref(), Some("Lua"));
        assert!(dir.path().join("Lua.png").exists());
        assert_eq!(load_name(dir.path()).as_deref(), Some("Lua"));
    }

    /// Vaca, porco, cobra e estátuas abrem o balão de escolha; "criar novo"
    /// entra, "trabalhar com um que você já criou" abre a pasta, "Ignorar
    /// isto" mostra a dica do Ctrl; Ctrl+clique entra direto.
    #[test]
    fn project_objects_ask_first() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Projects, None, None, &a);
        r.update(&[click(600, 470)], 0.0, &a);
        let fx = r.update(&[click(100, 80)], 0.0, &a);
        assert!(changes(&fx).is_empty());
        assert_eq!(sounds(&fx), [talk::snd::OBJECT, talk::snd::PICK]);
        assert_eq!(r.talk, Talk::Pick(Pick::Comics));
        let fx = r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Project(ProjectKind::Comics)]);
        // porco: "trabalhar com um que você já criou" → pasta
        let mut r = Room::new(Floor::Projects, None, None, &a);
        r.update(&[click(600, 470), click(320, 50)], 0.0, &a);
        assert_eq!(r.talk, Talk::Pick(Pick::Flipbook));
        let fx = r.update(&[option(&r, &a, 1)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::Portfolio)));
        assert!(!r.balloon_open());
        // cobra: "Ignorar isto" (a última linha) → dica [38]
        r.update(&[click(300, 230)], 0.0, &a);
        assert_eq!(r.talk, Talk::Pick(Pick::Sticker));
        let fx = r.update(&[option(&r, &a, talk::IGNORE)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::TIP]);
        assert!(r.sub.is_some());
        // com outro balão aberto, clicar na vaca troca direto de balão
        r.update(&[click(5, 5)], 0.0, &a);
        r.update(&[click(100, 80)], 0.0, &a);
        assert_eq!(r.talk, Talk::Pick(Pick::Comics));
        // Ctrl+clique na vaca: direto ao projeto
        let fx = r.update(&[Input::Ctrl(true), click(100, 80)], 0.0, &a);
        assert_eq!(changes(&fx), [SceneId::Project(ProjectKind::Comics)]);
        // opção 1 do balão de chegada dos Truques: o balão da estátua
        let mut r = Room::new(Floor::Tricks, None, None, &a);
        let fx = r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert!(changes(&fx).is_empty());
        assert_eq!(r.talk, Talk::Pick(Pick::Basic));
        let fx = r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert_eq!(
            changes(&fx),
            [SceneId::Tricks(super::super::wizard::TrickKind::Basic)]
        );
    }

    /// "tridimensional" no balão de chegada dos Truques abre o glossário
    /// sem fechar o balão; o resto da opção abre o balão do Pensador.
    #[test]
    fn tricks_glossary_word_and_frame() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Tricks, None, None, &a);
        let b = r.balloon.clone().unwrap();
        let (wx, wy, _, _) = b.option_word_rect(&a, 1, "tridimensional").unwrap();
        let fx = r.update(&[click(wx + 2, wy + 2)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::GLOSSARY]);
        assert!(r.sub.is_some() && r.balloon_open());
        r.update(&[click(5, 470)], 0.0, &a);
        let (ox, oy, _, _) = b.option_rect(&a, 1);
        r.update(&[click(ox + 20, oy + 3)], 0.0, &a);
        assert_eq!(r.talk, Talk::Pick(Pick::ThreeD));
        // a moldura troca de quadro a cada clique
        r.update(&[click(5, 470)], 0.0, &a);
        let fx = r.update(&[click(60, 60)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::PAINTING]);
        assert!(r.painting.is_some());
        // buraco do rato sem a varinha: só o som
        let fx = r.update(&[click(600, 300)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::MOUSE_HOLE]);
    }

    /// Luminária e "Tchau" abrem o "EPA!" modal: fora das bolinhas, a
    /// vinheta de recusa; "Continuar" fecha; "Sair" sai.
    #[test]
    fn lamp_and_tchau_open_the_modal_epa() {
        let (_d, a) = empty_assets();
        for (fl, lamp) in [
            (Floor::Lobby, (170, 220)),
            (Floor::Studio, (496, 180)),
            (Floor::Projects, (500, 120)),
            (Floor::Tricks, (590, 50)),
        ] {
            let mut r = Room::new(fl, None, None, &a);
            r.update(&[click(300, 470)], 0.0, &a);
            let fx = r.update(&[click(lamp.0, lamp.1)], 0.0, &a);
            assert!(r.quit_open(), "{fl:?}");
            assert_eq!(sounds(&fx), talk::snd::QUIT);
            let fx = r.update(&[click(5, 470)], 0.0, &a);
            assert!(r.quit_open());
            assert_eq!(sounds(&fx), [talk::snd::REFUSE]);
            let fx = r.update(&[quit_option(&r, &a, 1)], 0.0, &a);
            assert!(!r.quit_open());
            assert_eq!(sounds(&fx), [talk::snd::CHOICE]);
        }
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[click(600, 470), click(496, 260)], 0.0, &a);
        let fx = r.update(&[quit_option(&r, &a, 0)], 0.0, &a);
        assert!(fx.contains(&Effect::Quit));
    }

    /// McZee e Maggie: "Você quer me ver menos?"; "Sim" desliga a ajuda (sem
    /// balão de chegada) e o próximo clique pergunta se é para ajudar de novo.
    #[test]
    fn helper_turns_help_off_and_on() {
        let (_d, a) = empty_assets();
        let mut r = Room::new(Floor::Projects, None, None, &a);
        r.update(&[click(600, 470)], 0.0, &a);
        // sem dados, a Maggie é o quadrado de 32×32 em MAGGIE_POS
        let (mx, my) = (gags::MAGGIE_POS.0 + 10, gags::MAGGIE_POS.1 + 10);
        let fx = r.update(&[click(mx, my)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::MAGGIE]);
        assert_eq!(r.talk, Talk::Helper { off: false });
        r.update(&[option(&r, &a, 1)], 0.0, &a);
        assert_eq!(r.talk, Talk::Arrival, "diga de novo");
        r.update(&[click(600, 470), click(mx, my)], 0.0, &a);
        r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert!(r.help_off() && !r.balloon_open());
        // sem ajuda, o andar seguinte chega sem balão
        r.update(&[click(585, 100)], 0.0, &a);
        assert_eq!(r.floor(), Floor::Tricks);
        assert!(!r.balloon_open());
        r.update(&[click(mx, my)], 0.0, &a);
        assert_eq!(r.talk, Talk::Helper { off: true });
        r.update(&[option(&r, &a, 0)], 0.0, &a);
        assert!(!r.help_off());
        assert_eq!(r.talk, Talk::Arrival);
        // McZee no Ateliê, com a fala dele
        let mut r = Room::new(Floor::Studio, None, None, &a);
        r.update(&[click(600, 470)], 0.0, &a);
        let (cx, cy) = (gags::MCZEE_STUDIO.0 + 20, gags::MCZEE_STUDIO.1 + 10);
        let fx = r.update(&[click(cx, cy)], 0.0, &a);
        assert_eq!(sounds(&fx), [talk::snd::MCZEE]);
        assert_eq!(r.talk, Talk::Helper { off: false });
    }

    /// Com os dados reais: "tridimensional" é uma palavra dentro da opção 2
    /// do balão de chegada dos Truques.
    #[test]
    fn tricks_glossary_word_with_real_data() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let r = Room::new(Floor::Tricks, None, None, &a);
        let b = r.balloon.as_ref().unwrap();
        let w = b.option_word_rect(&a, 1, "tridimensional").unwrap();
        let o = b.option_rect(&a, 1);
        assert!(w.0 >= o.0 && w.1 >= o.1 && w.2 > 30, "{w:?} {o:?}");
    }

    #[test]
    fn draws_without_data() {
        let (_d, a) = empty_assets();
        let mut f = Frame::new(640, 480);
        for fl in CYCLE {
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
        assert!(r.quit_open());
        let fx = r.update(&[quit_option(&r, &a, 0)], 0.0, &a);
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
