//! Galeria do saguão como no original: três molduras na parede com os
//! desenhos, a pasta no chão com todos os trabalhos, a impressora e a
//! lixeira. Arrastar um quadro troca de moldura, vai para a impressora ou
//! para a lixeira; clique duplo abre o quadro na pintura. Medido na captura
//! `scratch/orig_gallery1.png`/`orig_gallery2.png` (fora do repositório); o
//! papel de parede de corações não está no `ARTIST.PIC` e é desenhado aqui.
//!
//! A Bolsa Mágica fica aberta no chão, como no original: a maleta vermelha
//! abre a pasta, o livro verde conta a história do McZee e a Varinha Mágica,
//! no buraco do rato, faz sair uma surpresa (`room_objects`). A pasta também
//! abre sozinha por cima das salas ([`GalleryRoom::portfolio`]).

use super::gags::{self, Character};
use super::gallery_print::PrintPreview;
use super::portfolio::{self, Action, Binder};
use super::room_objects::{self as objects, BagPart, MagicBag, Target, snd};
use crate::Frame;
use crate::anim::Animator;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::Balloon;
use crate::paint::store::{LAST_NAME, MAG_TITLE, NAME_FILE, load_picture, save_png};
use crate::paint::{CANVAS_H, CANVAS_W, Canvas};
use crate::rng::XorShift;
use crate::scene::{DEFAULT_CURSOR, Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::text_balloon::{PIC_ALERT, Part, TextBalloon};
use formats::palette::Rgb;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

type Rect = (i32, i32, i32, i32);

/// Molduras da parede (`6c79`, máscara `6c7d`, 200×140).
pub const FRAMES: [(i32, i32); 3] = [(95, 50), (323, 50), (211, 195)];
const FRAME_IMG: (u16, u16) = (0x6c79, 0x6c7d);
/// Área do desenho dentro da moldura (1/4 do quadro) e a plaqueta do nome.
const PICTURE_OFF: (i32, i32) = (24, 25);
const PLATE: Rect = (32, 117, 140, 12);
const THUMB_W: u32 = CANVAS_W / 4;
const THUMB_H: u32 = CANVAS_H / 4;

/// Elementos fixos: (cor, máscara, x, y). A Bolsa Mágica e o que sai dela
/// (livro verde, maleta e varinha) vêm de `room_objects::GALLERY_BAG`.
const ELEMENTS: [(u16, u16, i32, i32); 5] = [
    (0x681b, 0x6821, 16, 3),    // Tchau
    (0x6c87, 0x6c89, 430, 193), // impressora no quadro da parede
    (0x6c7f, 0x6c81, 527, 276), // seta "Saguão"
    (0x683b, 0x6841, 445, 348), // buraco do rato
    (0x6c83, 0x6c85, 523, 359), // lixeira
];
const TCHAU: Rect = (16, 90, 56, 45);
const PRINTER: Rect = (430, 193, 60, 74);
const LOBBY_ARROW: Rect = (527, 276, 91, 46);
const TRASH: Rect = (523, 359, 92, 108);
const MOUSE_HOLE: Rect = (445, 348, 54, 47);
/// A maleta vermelha (a pasta de trabalhos).
#[cfg(test)]
const FOLDER: Rect = (304, 391, 99, 74);

/// Bloco `WHAT` das salas: boas-vindas da galeria e reciclagem.
const WHAT: u16 = 0x100d;
const WELCOME: usize = 6;
const WELCOME2: usize = 7;
/// Dicas mostradas pelas opções do balão de boas-vindas, pela impressora e
/// pela lixeira (`gallery-printer`, `gallery-Recycle bin`).
const TIP_REORGANIZE: usize = 8;
const TIP_OPEN: usize = 9;
const TIP_PRINT: usize = 10;
const TIP_TRASH: usize = 11;
/// "A pasta é o lugar onde são guardados todos os seus quadros…".
const TIP_PORTFOLIO: usize = 12;
/// "Você quer me ver menos?" e "Que bom que você me chamou de novo".
const HELP_OFF: usize = 29;
const HELP_ON: usize = 30;
/// "Se você quiser trabalhar neste quadro…" (clique simples na moldura).
const WORK_ON: usize = 41;
/// "Wand - library": a varinha no buraco do rato da Galeria.
const WAND_LIBRARY: usize = 57;
/// Avisos "EPA!" da lixeira: desenho da pasta (2 opções), exemplo da parede
/// ("Não, obrigado..."), exemplo da pasta e desenho da parede (3 opções).
const RECYCLE: usize = 73;
const NOT_YOURS_WALL: usize = 74;
const NOT_YOURS: usize = 75;
const RECYCLE_WALL: usize = 76;
/// "O que você quer fazer? Sair do Fine Artist / Continuar trabalhando".
const QUIT_WHAT: (u16, usize) = (0x1001, 2);
/// Índices do original nas opções da lixeira (`22C000n`).
const OPT_KEEP: u16 = 0;
const OPT_TRASH: u16 = 1;
const OPT_QUIT: u16 = 1;
/// Posição dos avisos "EPA!" (medida em `gallery_o_dtrash_own_1.png`).
const ALERT_POS: (i32, i32) = (178, 145);
const QUIT_POS: (i32, i32) = (214, 179);
/// Balão de boas-vindas (medido na chegada do original).
const WELCOME_POS: (i32, i32) = (130, 240);

/// Sons da Galeria (ids do `ARTIST.SND`, identificados pelo tamanho no
/// original; ver `report/gallery.md` da varredura).
pub mod sfx {
    /// Fala de chegada.
    pub const ARRIVAL: u16 = 0x304;
    /// Clique ao pegar um quadro (ou clicar nele).
    pub const PICK: u16 = 0x21c;
    /// Opção escolhida num balão.
    pub const CHOICE: u16 = 0x254;
    /// Impressora (clique ou quadro solto nela) e a abertura da prévia.
    pub const PRINTER: u16 = 0x212;
    pub const PREVIEW: u16 = 0x2f6;
    /// Lixeira.
    pub const TRASH: u16 = 0x2f8;
    /// Vinheta dos avisos "EPA!".
    pub const ALERT: u16 = 0x2fe;
    /// Vinheta de recusa: clique fora de um aviso modal.
    pub const REFUSE: u16 = 0x217;
    /// Tchau.
    pub const BYE: u16 = 0x264;
    /// Fala curta do McZee ao clicar nele.
    pub const MCZEE: u16 = 0x252;
    /// Seta "Saguão".
    pub const LOBBY: u16 = 0x2fb;
    /// Buraco do rato sem a varinha.
    pub const MOUSE_HOLE: u16 = 0x20a;
}

/// Programa que imprime a folha (o `lp` do CUPS).
const PRINT_COMMAND: &str = "lp";

const DOUBLE_CLICK: f32 = 0.4;
/// Arquivo com os nomes dos quadros das três molduras.
pub const WALL_FILE: &str = "galeria.txt";

const BLACK: [u8; 3] = [0, 0, 0];
const WALL_A: [u8; 3] = [136, 136, 230];
const WALL_B: [u8; 3] = [71, 140, 80];
const FLOOR_A: [u8; 3] = [100, 100, 100];
const BASEBOARD: (i32, i32) = (375, 395);

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn frame_rect(k: usize) -> Rect {
    (FRAMES[k].0, FRAMES[k].1, 200, 140)
}

/// Um quadro: arquivo, miniatura 1/4 e nome para a plaqueta.
pub struct Item {
    pub path: PathBuf,
    pub name: String,
    /// Título dado ao quadro (o do `.MAG`, sem o "Quadro Novo"): vai para o
    /// rótulo da pintura quando ele é aberto.
    pub title: Option<String>,
    /// Autor gravado no `.MAG` (os exemplos são da Maggie).
    pub author: String,
    pub sample: bool,
    thumb: Canvas,
}

impl Item {
    /// Miniatura 1/4 do quadro.
    pub fn thumb(&self) -> &Canvas {
        &self.thumb
    }
}

/// Título e autor do `.MAG` do quadro (o próprio arquivo ou o gêmeo do `.png`).
fn mag_info(path: &Path) -> (Option<String>, String) {
    let mag = if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mag"))
    {
        path.to_path_buf()
    } else {
        path.with_extension("mag")
    };
    let Some(p) = std::fs::read(&mag)
        .ok()
        .and_then(|d| formats::mag::read(&d).ok())
    else {
        return (None, String::new());
    };
    let t = p.title.trim();
    let title = (!t.is_empty() && t != MAG_TITLE).then(|| t.to_string());
    (title, p.author.trim().to_string())
}

/// Nome para mostrar: o título do `.MAG`, ou o nome do arquivo sem o carimbo.
fn display_name(path: &Path) -> String {
    if let (Some(t), _) = mag_info(path) {
        return t;
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // desenho-AAAAMMDD-HHMMSS → "Desenho 31/10 14:05"
    let parts: Vec<&str> = stem.splitn(3, '-').collect();
    if parts.len() == 3 && parts[1].len() == 8 && parts[2].len() >= 4 {
        let (d, t) = (parts[1], parts[2]);
        let mut kind = parts[0].to_string();
        if let Some(c) = kind.get_mut(0..1) {
            c.make_ascii_uppercase();
        }
        return format!("{kind} {}/{} {}:{}", &d[6..8], &d[4..6], &t[0..2], &t[2..4]);
    }
    stem
}

/// Miniatura 1/4: de cada bloco 4×4 fica o pixel mais escuro, para as
/// linhas finas não sumirem (o original reduz com o `StretchBlt` do
/// Windows, que preserva o preto).
fn thumbnail(c: &Canvas, pal: &[Rgb]) -> Canvas {
    let (w, h) = (THUMB_W, THUMB_H);
    let luma = |i: u8| {
        pal.get(i as usize)
            .map_or(0, |p| 3 * p.r as u32 + 6 * p.g as u32 + p.b as u32)
    };
    let mut t = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let mut best = c.px[((y * 4) * c.w + x * 4) as usize];
            for dy in 0..4 {
                for dx in 0..4 {
                    let (sx, sy) = (x * 4 + dx, y * 4 + dy);
                    if sx < c.w && sy < c.h {
                        let v = c.px[(sy * c.w + sx) as usize];
                        if luma(v) < luma(best) {
                            best = v;
                        }
                    }
                }
            }
            t.px[(y * w + x) as usize] = best;
        }
    }
    t
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Wall(usize),
    Folder(usize),
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    from: Source,
    start: (i32, i32),
    pos: (i32, i32),
    moved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ask {
    /// Boas-vindas (`100d[6]`).
    Welcome,
    /// "Outra coisa..." (`100d[7]`).
    Welcome2,
    /// Dica sem opções.
    Tip,
    /// "Se você quiser trabalhar neste quadro…" (índice em `items`).
    WorkOn(usize),
    /// "Você quer me ver menos?".
    HelpOff,
    /// "Que bom que você me chamou de novo".
    HelpOn,
}

/// Avisos "EPA!" modais (casinha do Rex): só as opções respondem; um clique
/// fora toca a vinheta de recusa. Os que não têm opções fecham com qualquer
/// clique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modal {
    /// Sair do Fine Artist / Continuar trabalhando.
    Quit,
    /// Um exemplo na lixeira: "Não, obrigado..."; a moldura de onde ele
    /// veio, se veio de uma, recebe outro exemplo.
    NotYours(Option<usize>),
    /// Um desenho da criança na lixeira (item, moldura de onde veio).
    Trash(usize, Option<usize>),
}

struct Alert {
    kind: Modal,
    balloon: TextBalloon,
    /// Índice do original de cada opção, na ordem em que aparecem.
    ids: Vec<u16>,
}

pub struct GalleryRoom {
    common: KidCommon,
    dir: Option<PathBuf>,
    pal: Vec<Rgb>,
    items: Vec<Item>,
    /// Índice em `items` do quadro de cada moldura.
    wall: [Option<usize>; 3],
    folder_open: bool,
    /// O fichário verde (a pasta de trabalhos).
    binder: Binder,
    drag: Option<Drag>,
    last_click: Option<(Source, f32)>,
    clock: f32,
    balloon: Option<(Ask, Balloon)>,
    alert: Option<Alert>,
    print: Option<PrintPreview>,
    /// Programa de impressão (os testes trocam o `lp`).
    print_command: String,
    character: Option<Character>,
    /// "Sim, por favor" em "Você quer me ver menos?": sem as dicas, e o
    /// próximo clique no McZee pergunta se ele volta a ajudar.
    help_off: bool,
    mouse: (i32, i32),
    /// Só a pasta, aberta pela maleta de uma sala (por cima dela).
    overlay: bool,
    bag: MagicBag,
    surprise: Option<Animator>,
    rng: XorShift,
    wand_cursor: bool,
    /// A fala de chegada já foi pedida.
    arrived: bool,
    /// Sons em sequência e quanto falta para o próximo.
    queue: VecDeque<u16>,
    queue_wait: f32,
}

impl GalleryRoom {
    pub fn new(click: Option<SoundRef>, dir: Option<PathBuf>, assets: &Assets) -> GalleryRoom {
        let pal = assets.palettes(Program::Artist).pal256.clone();
        let mut items = Vec::new();
        let mut add = |paths: Vec<PathBuf>, sample: bool| {
            for path in paths {
                if let Some(c) = load_picture(&path, &pal) {
                    let (title, author) = mag_info(&path);
                    items.push(Item {
                        name: display_name(&path),
                        title,
                        author,
                        thumb: thumbnail(&c, &pal),
                        path,
                        sample,
                    });
                }
            }
        };
        add(
            dir.as_deref()
                .map(super::gallery::list_drawings)
                .unwrap_or_default(),
            false,
        );
        add(super::gallery::list_samples(assets.data_dir()), true);
        let wall = load_wall(dir.as_deref(), &items);
        let folder = dir
            .as_ref()
            .and_then(|d| d.file_name())
            .map_or("(sem pasta)".to_string(), |n| {
                format!("{}/", n.to_string_lossy())
            });
        let mut binder = Binder::new(folder);
        binder.rebuild(&items);
        let mut rng = XorShift::from_clock();
        let character = Character::pick(gags::MCZEE_COSTUMES, gags::MCZEE_LOBBY, &mut rng);
        let mut g = GalleryRoom {
            common: KidCommon::new(click),
            dir,
            pal,
            items,
            wall,
            folder_open: false,
            binder,
            drag: None,
            last_click: None,
            clock: 0.0,
            balloon: None,
            alert: None,
            print: None,
            print_command: PRINT_COMMAND.to_string(),
            character,
            help_off: false,
            mouse: (0, 0),
            overlay: false,
            bag: MagicBag::new(objects::GALLERY_BAG, true),
            surprise: None,
            rng,
            wand_cursor: false,
            arrived: false,
            queue: VecDeque::new(),
            queue_wait: 0.0,
        };
        g.ask(Ask::Welcome, WELCOME, assets);
        g
    }

    /// Toca sons em sequência: o primeiro já, os outros quando o anterior
    /// acabar. Uma ação nova interrompe a sequência pendente.
    fn play(&mut self, ids: &[u16]) {
        self.queue.clear();
        self.queue.extend(ids.iter().copied());
        self.queue_wait = 0.0;
    }

    fn pump_sounds(&mut self, dt: f32, fx: &mut Vec<Effect>, assets: &Assets) {
        self.queue_wait -= dt;
        if self.queue_wait > 0.0 {
            return;
        }
        if let Some(id) = self.queue.pop_front() {
            let r = snd(id);
            self.queue_wait = assets.sound(&r).map_or(0.5, |p| {
                p.samples.len() as f32 / (p.sample_rate.max(1) as f32 * p.channels.max(1) as f32)
            });
            fx.push(Effect::PlaySound(r));
        } else {
            self.queue_wait = 0.0;
        }
    }

    /// Aviso "EPA!" do bloco `block`, registro `k`, em `pos`; o reserva vale
    /// sem o `ARTIST.EXE`.
    fn alert(&mut self, kind: Modal, block: u16, k: usize, pos: (i32, i32), assets: &Assets) {
        let (fallback, opts): (&str, &[&str]) = match kind {
            Modal::Quit => (
                "O que você quer fazer?",
                &["Sair do Fine Artist", "Continuar trabalhando"],
            ),
            Modal::NotYours(_) => (
                "Não, obrigado... Não posso reciclar isso — ele não é seu.",
                &[],
            ),
            Modal::Trash(_, Some(_)) => (
                "Tem certeza de que você quer que o Rex jogue isso fora?",
                &[
                    "Jogue no lixo! Não quero ver isso nunca mais!",
                    "Tire da parede, posso precisar mais tarde",
                    "Ops! Esquece, pode deixar na parede",
                ],
            ),
            Modal::Trash(_, None) => (
                "O Rex está se preparando para jogar isto fora... Você tem certeza?",
                &[
                    "Jogue no lixo! Não quero ver isso nunca mais!",
                    "Tira a mão! Posso precisar mais tarde",
                ],
            ),
        };
        let fallback_ids: Vec<u16> = match kind {
            Modal::Trash(_, Some(_)) => vec![OPT_TRASH, OPT_KEEP, 2],
            Modal::Trash(_, None) => vec![OPT_TRASH, OPT_KEEP],
            Modal::Quit => vec![OPT_QUIT, 0],
            Modal::NotYours(_) => Vec::new(),
        };
        let (balloon, ids) = match assets.dialogs(Program::Artist, block).get(k) {
            Some(r) => (
                TextBalloon::from_record(r, pos.0, pos.1, "", Vec::new()),
                r.options().into_iter().map(|o| o.0).collect(),
            ),
            None => {
                let mut parts = vec![Part::Figure(PIC_ALERT), Part::Text(fallback.into())];
                if !opts.is_empty() {
                    parts.push(Part::Options(opts.iter().map(|o| o.to_string()).collect()));
                }
                (
                    TextBalloon::new(pos.0, pos.1, 260, parts, Vec::new()),
                    fallback_ids,
                )
            }
        };
        self.balloon = None;
        self.alert = Some(Alert { kind, balloon, ids });
    }

    /// Só a pasta de trabalhos, aberta pela maleta vermelha de uma sala:
    /// desenhada por cima da sala; fechar (ou clicar fora) volta a ela, e um
    /// clique duplo abre o quadro na pintura.
    pub fn portfolio(
        click: Option<SoundRef>,
        dir: Option<PathBuf>,
        assets: &Assets,
    ) -> GalleryRoom {
        let mut g = GalleryRoom::new(click, dir, assets);
        g.balloon = None;
        g.overlay = true;
        g.open_folder(assets);
        g
    }

    /// A criança (apelido e bicho), para a árvore do fichário.
    pub fn with_user(mut self, user: &str, animal: Option<usize>) -> GalleryRoom {
        self.binder.set_user(user, animal, &self.items);
        self
    }

    fn open_folder(&mut self, assets: &Assets) {
        self.folder_open = true;
        self.binder.open(!self.help_off, assets);
    }

    fn close_folder(&mut self, fx: &mut Vec<Effect>) {
        self.folder_open = false;
        if self.overlay {
            fx.push(Effect::Back);
        }
    }

    pub fn bag(&self) -> &MagicBag {
        &self.bag
    }

    pub fn surprise_playing(&self) -> bool {
        self.surprise.is_some()
    }

    fn use_bag(&mut self, part: BagPart, fx: &mut Vec<Effect>, assets: &Assets) {
        match part {
            BagPart::Bag => {
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
                self.open_folder(assets);
            }
            BagPart::Book => {
                fx.extend(objects::SND_BOOK.map(|id| Effect::PlaySound(snd(id))));
                fx.push(Effect::Push(SceneId::BookStory));
            }
            BagPart::Wand => {
                self.bag.holding_wand = true;
                self.bag.open = false;
                fx.push(Effect::PlaySound(snd(objects::SND_WAND_TAKE)));
                self.ask(Ask::Tip, WAND_LIBRARY, assets);
                if let Some((_, b)) = &mut self.balloon {
                    b.options.clear();
                }
            }
        }
    }

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

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn wall(&self) -> [Option<usize>; 3] {
        self.wall
    }

    pub fn folder_open(&self) -> bool {
        self.folder_open
    }

    fn ask(&mut self, kind: Ask, k: usize, assets: &Assets) {
        let fallback = match kind {
            Ask::Welcome => Balloon {
                x: 0,
                y: 0,
                w: 239,
                text: "Seja bem-vindo à Galeria! O que você quer fazer?".into(),
                options: vec![
                    "Reorganizar a parede".into(),
                    "Abrir um quadro".into(),
                    "Jogar fora alguns quadros".into(),
                    "Ver outros quadros".into(),
                    "Outra coisa...".into(),
                ],
                links: Vec::new(),
                figures: Vec::new(),
            },
            Ask::Welcome2 => Balloon {
                x: 0,
                y: 0,
                w: 206,
                text: "O que você quer fazer?".into(),
                options: vec![
                    "Imprimir um desenho".into(),
                    "Voltar para outras escolhas...".into(),
                ],
                links: Vec::new(),
                figures: Vec::new(),
            },
            Ask::WorkOn(_) => Balloon {
                x: 0,
                y: 0,
                w: 225,
                text: "Se você quiser trabalhar neste quadro, eu vou levar você para o Ateliê de Pintura na Bolsa Mágica.".into(),
                options: vec![
                    "Estou pronto para ir!".into(),
                    "Ops! Não quero trabalhar com isto!".into(),
                ],
                links: Vec::new(),
                figures: Vec::new(),
            },
            Ask::HelpOff => Balloon {
                x: 0,
                y: 0,
                w: 162,
                text: "Você quer me ver menos?".into(),
                options: vec![
                    "Sim, por favor.".into(),
                    "Diga de novo o que eu posso fazer aqui.".into(),
                    "Deixa para lá.".into(),
                ],
                links: Vec::new(),
                figures: Vec::new(),
            },
            Ask::HelpOn => Balloon {
                x: 0,
                y: 0,
                w: 203,
                text: "Que bom que você me chamou de novo. O que você quer que eu faça?".into(),
                options: vec![
                    "Ajudar você mais um pouco?".into(),
                    "Não, eu não preciso de mais ajuda.".into(),
                ],
                links: Vec::new(),
                figures: Vec::new(),
            },
            Ask::Tip => Balloon {
                x: 0,
                y: 0,
                w: 200,
                text: String::new(),
                options: Vec::new(),
                links: Vec::new(),
                figures: Vec::new(),
            },
        };
        let records = assets.dialogs(Program::Artist, WHAT);
        let mut b =
            Balloon::from_block(&records, k, 0, 0, super::room::DEFAULT_USER).unwrap_or(fallback);
        // as opções de disquete do original não existem aqui
        if kind == Ask::Welcome2 {
            b.options
                .retain(|o| !o.contains("disquete") && !o.contains("unidades"));
        }
        (b.x, b.y) = match kind {
            Ask::Welcome | Ask::Welcome2 | Ask::HelpOff | Ask::HelpOn => WELCOME_POS,
            Ask::WorkOn(_) => (0, 0),
            Ask::Tip => (150, 170),
        };
        self.balloon = Some((kind, b));
    }

    /// Dica do McZee (sem opções), se ele não foi dispensado, em `pos`.
    fn tip(&mut self, k: usize, pos: (i32, i32), assets: &Assets) {
        if self.help_off {
            return;
        }
        self.ask(Ask::Tip, k, assets);
        if let Some((_, b)) = &mut self.balloon {
            (b.x, b.y) = pos;
        }
    }

    /// "Se você quiser trabalhar neste quadro…" ao lado da moldura `k`: à
    /// direita se couber, senão à esquerda, senão encostado na borda.
    fn ask_work_on(&mut self, i: usize, k: usize, assets: &Assets) {
        self.ask(Ask::WorkOn(i), WORK_ON, assets);
        let (fx, fy, fw, _) = frame_rect(k);
        if let Some((_, b)) = &mut self.balloon {
            let right = fx + fw + 23;
            let left = fx - b.w - 23;
            b.x = if right + b.w <= 636 {
                right
            } else if left >= 4 {
                left
            } else {
                636 - b.w
            };
            b.y = fy + 2;
        }
    }

    /// Arquivo do quadro da moldura `k`.
    pub fn wall_path(&self, k: usize) -> Option<&Path> {
        self.wall[k].map(|i| self.items[i].path.as_path())
    }

    fn save_wall(&self) -> Option<Effect> {
        let dir = self.dir.as_ref()?;
        let text: String = self
            .wall
            .iter()
            .map(|w| {
                w.map(|i| self.items[i].path.to_string_lossy().to_string())
                    .unwrap_or_default()
                    + "\n"
            })
            .collect();
        std::fs::create_dir_all(dir)
            .and_then(|()| std::fs::write(dir.join(WALL_FILE), text))
            .err()
            .map(|e| Effect::Log(format!("galeria: {e}")))
    }

    /// Abre o quadro na pintura: ele vira o último desenho, com o seu título
    /// no rótulo (ou "Quadro Novo", se não tiver um).
    fn open(&self, i: usize) -> Vec<Effect> {
        let (Some(dir), Some(it)) = (&self.dir, self.items.get(i)) else {
            return Vec::new();
        };
        let Some(c) = load_picture(&it.path, &self.pal) else {
            return vec![Effect::Log(format!(
                "galeria: não consegui abrir {}",
                it.path.display()
            ))];
        };
        if let Err(e) = std::fs::create_dir_all(dir)
            .and_then(|()| save_png(&c, &self.pal, &dir.join(LAST_NAME)))
        {
            return vec![Effect::Log(format!("galeria: {e}"))];
        }
        let name_file = dir.join(NAME_FILE);
        let named = match &it.title {
            Some(t) => std::fs::write(&name_file, t),
            None => match std::fs::remove_file(&name_file) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            },
        };
        let mut fx = Vec::new();
        if let Err(e) = named {
            fx.push(Effect::Log(format!("galeria: {e}")));
        }
        fx.push(Effect::ChangeScene(SceneId::Paint));
        fx
    }

    /// Quadro solto na impressora: a prévia com o McZee.
    fn print(&mut self, i: usize, assets: &Assets) -> Option<Effect> {
        let it = self.items.get(i)?;
        let Some(c) = load_picture(&it.path, &self.pal) else {
            return Some(Effect::Log(format!(
                "imprimir: não consegui abrir {}",
                it.path.display()
            )));
        };
        self.balloon = None;
        self.play(&[sfx::PRINTER, sfx::PREVIEW]);
        self.print = Some(PrintPreview::new(
            c,
            self.pal.clone(),
            self.dir.clone(),
            &self.print_command,
            assets,
        ));
        None
    }

    /// Outro quadro para a moldura que ficou vazia: um exemplo que não está
    /// na parede (sorteado), ou qualquer quadro fora dela; nunca `except`.
    fn replacement(&mut self, except: Option<usize>) -> Option<usize> {
        let free = |s: bool| -> Vec<usize> {
            (0..self.items.len())
                .filter(|&i| {
                    self.items[i].sample == s && !self.wall.contains(&Some(i)) && Some(i) != except
                })
                .collect()
        };
        let mut pool = free(true);
        if pool.is_empty() {
            pool = free(false);
        }
        if pool.is_empty() {
            return None;
        }
        Some(pool[self.rng.below(pool.len())])
    }

    /// Põe outro quadro na moldura `k` e grava a parede.
    fn refill(&mut self, k: usize) -> Option<Effect> {
        let old = self.wall[k].take();
        self.wall[k] = self.replacement(old);
        self.save_wall()
    }

    /// Joga fora o quadro: só desenhos da criança (os exemplos ficam).
    fn recycle(&mut self, i: usize) -> Vec<Effect> {
        let Some(it) = self.items.get(i) else {
            return Vec::new();
        };
        if it.sample {
            return vec![Effect::Log(
                "os exemplos do Fine Artist não podem ir para a lixeira".into(),
            )];
        }
        let mut fx = Vec::new();
        if let Err(e) = std::fs::remove_file(&it.path) {
            fx.push(Effect::Log(format!("lixeira: {e}")));
            return fx;
        }
        // o .mag gêmeo do .png vai junto
        let twin = it.path.with_extension("mag");
        if twin != it.path {
            let _ = std::fs::remove_file(twin);
        }
        self.items.remove(i);
        self.binder.rebuild(&self.items);
        for w in self.wall.iter_mut() {
            *w = match *w {
                Some(j) if j == i => None,
                Some(j) if j > i => Some(j - 1),
                other => other,
            };
        }
        fx.extend(self.save_wall());
        fx
    }

    /// Quadro solto na lixeira: os exemplos são recusados pelo Rex; os
    /// desenhos da criança pedem confirmação.
    fn to_trash(&mut self, i: usize, from: Source, assets: &Assets) {
        self.play(&[sfx::TRASH, sfx::ALERT]);
        let slot = match from {
            Source::Wall(k) => Some(k),
            Source::Folder(_) => None,
        };
        if self.items[i].sample {
            let k = if slot.is_some() {
                NOT_YOURS_WALL
            } else {
                NOT_YOURS
            };
            self.alert(Modal::NotYours(slot), WHAT, k, ALERT_POS, assets);
        } else {
            let k = if slot.is_some() {
                RECYCLE_WALL
            } else {
                RECYCLE
            };
            self.alert(Modal::Trash(i, slot), WHAT, k, ALERT_POS, assets);
        }
    }

    /// Clique com um aviso "EPA!" aberto.
    fn alert_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(a) = &self.alert else { return };
        let kind = a.kind;
        if a.ids.is_empty() {
            // sem opções: qualquer clique fecha
            self.alert = None;
            if let Modal::NotYours(Some(k)) = kind {
                fx.extend(self.refill(k));
            }
            return;
        }
        let Some(pos) = a.balloon.hit_option(assets, x, y) else {
            self.play(&[sfx::REFUSE]);
            return;
        };
        let id = a.ids.get(pos).copied().unwrap_or(pos as u16);
        self.alert = None;
        self.play(&[sfx::CHOICE]);
        match kind {
            Modal::Quit => {
                // no original, "Sair" é a opção 1 e "Continuar" a 0
                if id == OPT_QUIT {
                    fx.push(Effect::Quit);
                }
            }
            Modal::NotYours(_) => {}
            Modal::Trash(i, slot) => match id {
                OPT_TRASH => {
                    fx.extend(self.recycle(i));
                    if let Some(k) = slot {
                        fx.extend(self.refill(k));
                    }
                }
                OPT_KEEP => {
                    // "Tire da parede": o arquivo fica na pasta
                    if let Some(k) = slot {
                        fx.extend(self.refill(k));
                    }
                }
                _ => {}
            },
        }
    }

    fn hit(&self, x: i32, y: i32) -> Option<Source> {
        if self.folder_open {
            // a miniatura da página direita pode ir para uma moldura
            if self.binder.search_open() || !inside(portfolio::THUMB, x, y) {
                return None;
            }
            return self.binder.selected().map(Source::Folder);
        }
        (0..3)
            .rev()
            .find(|&k| inside(frame_rect(k), x, y))
            .map(Source::Wall)
    }

    fn item_of(&self, s: Source) -> Option<usize> {
        match s {
            Source::Wall(k) => self.wall[k],
            Source::Folder(i) => Some(i),
        }
    }

    fn press(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        if self.alert.is_some() {
            self.alert_click(x, y, fx, assets);
            return;
        }
        if let Some((kind, b)) = &self.balloon {
            let kind = *kind;
            if let Some(k) = b.hit(assets, x, y) {
                self.balloon = None;
                self.answer(kind, k, fx, assets);
                return;
            }
            let inside_balloon = b.contains(assets, x, y);
            self.balloon = None;
            // fora do balão, um quadro (clique duplo) e a varinha seguem
            let on_frame = !self.folder_open && self.hit(x, y).is_some();
            if inside_balloon || !(self.bag.holding_wand || on_frame) {
                return;
            }
        }
        if self.folder_open && self.hit(x, y).is_none() {
            if !inside(portfolio::BINDER, x, y) && !self.binder.search_open() {
                self.close_folder(fx);
                return;
            }
            let clock = self.clock;
            let mut sounds = Vec::new();
            let action = self
                .binder
                .click(x, y, clock, &self.items, &mut sounds, assets);
            fx.extend(sounds);
            match action {
                Some(Action::Close) => self.close_folder(fx),
                Some(Action::Open(i)) => fx.extend(self.open(i)),
                None => {}
            }
            return;
        }
        if let Some(src) = self.hit(x, y) {
            if self.item_of(src).is_some() {
                if matches!(src, Source::Wall(_)) {
                    self.play(&[sfx::PICK]);
                }
                self.drag = Some(Drag {
                    from: src,
                    start: (x, y),
                    pos: (x, y),
                    moved: false,
                });
            }
            return;
        }
        if let Some(part) = self.bag.hit(x, y) {
            self.use_bag(part, fx, assets);
        } else if inside(MOUSE_HOLE, x, y) {
            if !self.bag.holding_wand {
                self.play(&[sfx::MOUSE_HOLE]);
                return;
            }
            // a varinha no buraco do rato: uma surpresa, e ela volta à bolsa
            self.bag.holding_wand = false;
            fx.push(Effect::PlaySound(snd(objects::SND_WAND_HOLE)));
            match objects::GALLERY_SURPRISES.start(Target::Hole, &mut self.rng, assets) {
                Some(mut a) => {
                    fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
                    self.surprise = Some(a);
                }
                None => self.bag.open = true,
            }
        } else if inside(LOBBY_ARROW, x, y) {
            fx.push(Effect::PlaySound(snd(sfx::LOBBY)));
            fx.push(Effect::ChangeScene(SceneId::Building));
        } else if inside(TCHAU, x, y) {
            self.play(&[sfx::BYE, sfx::ALERT]);
            self.alert(Modal::Quit, QUIT_WHAT.0, QUIT_WHAT.1, QUIT_POS, assets);
        } else if inside(PRINTER, x, y) {
            self.play(&[sfx::PRINTER]);
            self.tip(TIP_PRINT, (PRINTER.0 - 190, PRINTER.1 - 40), assets);
        } else if inside(TRASH, x, y) {
            self.play(&[sfx::TRASH]);
            self.tip(TIP_TRASH, (TRASH.0 - 180, TRASH.1 - 120), assets);
        } else if let Some(c) = &self.character
            && x < 120
            && y > c.y
        {
            self.play(&[sfx::MCZEE]);
            if self.help_off {
                self.ask(Ask::HelpOn, HELP_ON, assets);
            } else {
                self.ask(Ask::HelpOff, HELP_OFF, assets);
            }
        }
    }

    fn release(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(d) = self.drag.take() else { return };
        let Some(i) = self.item_of(d.from) else {
            return;
        };
        if !d.moved {
            // clique: duplo abre o quadro; simples, na parede, pergunta
            if let Some((s, t)) = self.last_click
                && s == d.from
                && self.clock - t < DOUBLE_CLICK
            {
                self.last_click = None;
                self.balloon = None;
                fx.extend(self.open(i));
                return;
            }
            self.last_click = Some((d.from, self.clock));
            if let Source::Wall(k) = d.from {
                self.ask_work_on(i, k, assets);
            }
            return;
        }
        if inside(PRINTER, x, y) {
            fx.extend(self.print(i, assets));
            return;
        }
        if inside(TRASH, x, y) {
            self.to_trash(i, d.from, assets);
            return;
        }
        if let Some(k) = (0..3).rev().find(|&k| inside(frame_rect(k), x, y)) {
            match d.from {
                Source::Wall(j) => self.wall.swap(j, k),
                Source::Folder(_) => self.wall[k] = Some(i),
            }
            fx.extend(self.save_wall());
        }
    }

    fn answer(&mut self, kind: Ask, k: usize, fx: &mut Vec<Effect>, assets: &Assets) {
        if kind != Ask::Tip {
            self.play(&[sfx::CHOICE]);
        }
        match (kind, k) {
            (Ask::Welcome, 0) => self.ask(Ask::Tip, TIP_REORGANIZE, assets),
            (Ask::Welcome, 1) => self.ask(Ask::Tip, TIP_OPEN, assets),
            (Ask::Welcome, 2) => self.ask(Ask::Tip, TIP_TRASH, assets),
            (Ask::Welcome, 3) => self.ask(Ask::Tip, TIP_PORTFOLIO, assets),
            (Ask::Welcome, _) => self.ask(Ask::Welcome2, WELCOME2, assets),
            (Ask::Welcome2, 0) => self.ask(Ask::Tip, TIP_PRINT, assets),
            (Ask::Welcome2, _) => self.ask(Ask::Welcome, WELCOME, assets),
            (Ask::WorkOn(i), 0) => fx.extend(self.open(i)),
            (Ask::HelpOff, 0) => self.help_off = true,
            (Ask::HelpOff, 1) => self.ask(Ask::Welcome, WELCOME, assets),
            (Ask::HelpOn, 0) => {
                self.help_off = false;
                self.ask(Ask::Welcome, WELCOME, assets);
            }
            _ => {}
        }
    }

    fn draw_background(f: &mut Frame) {
        for y in 0..480 {
            for x in 0..640 {
                let even = (x + y) % 2 == 0;
                let c = if y < BASEBOARD.0 {
                    if even { WALL_B } else { WALL_A }
                } else if y <= BASEBOARD.1 {
                    BLACK
                } else if even {
                    WALL_B
                } else {
                    FLOOR_A
                };
                f.put(x, y, c);
            }
        }
        // corações do papel de parede (fileiras alternadas, medidos na captura)
        const HEART: [(i32, i32); 12] = [
            (1, 0),
            (5, 0),
            (0, 1),
            (2, 1),
            (4, 1),
            (6, 1),
            (1, 2),
            (3, 2),
            (5, 2),
            (2, 3),
            (4, 3),
            (3, 4),
        ];
        let mut row = 0;
        let mut y = 7;
        while y < BASEBOARD.0 - 8 {
            let x0 = if row % 2 == 0 { 10 } else { 46 };
            let mut x = x0;
            while x < 640 {
                for (dx, dy) in HEART {
                    f.put(x + dx, y + dy, BLACK);
                }
                x += 71;
            }
            y += if row % 2 == 0 { 22 } else { 20 };
            row += 1;
        }
    }

    fn draw_thumb(&self, f: &mut Frame, t: &Canvas, x: i32, y: i32) {
        for ty in 0..t.h {
            for tx in 0..t.w {
                let p = self
                    .pal
                    .get(t.px[(ty * t.w + tx) as usize] as usize)
                    .copied()
                    .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                f.put(x + tx as i32, y + ty as i32, [p.r, p.g, p.b]);
            }
        }
    }

    fn draw_text_centered(f: &mut Frame, a: &Assets, r: Rect, s: &str) {
        match a.font(8) {
            Some(font) => {
                let mut s = s.to_string();
                if font.width(&s) as i32 > r.2 {
                    let mut chars: Vec<char> = s.chars().collect();
                    while chars.len() > 1 {
                        chars.pop();
                        let t: String = chars.iter().collect::<String>() + "…";
                        if font.width(&t) as i32 <= r.2 {
                            break;
                        }
                    }
                    s = chars.iter().collect::<String>() + "…";
                }
                let w = font.width(&s) as i32;
                f.text_font(&font, r.0 + (r.2 - w) / 2, r.1, &s, BLACK);
            }
            None => f.text(r.0 + 2, r.1 + 2, s, BLACK),
        }
    }
}

/// Molduras guardadas em `galeria.txt`; o que faltar é preenchido com os
/// desenhos mais recentes (e depois os exemplos).
fn load_wall(dir: Option<&Path>, items: &[Item]) -> [Option<usize>; 3] {
    let mut wall = [None; 3];
    if let Some(text) = dir.and_then(|d| std::fs::read_to_string(d.join(WALL_FILE)).ok()) {
        for (k, line) in text.lines().take(3).enumerate() {
            wall[k] = items
                .iter()
                .position(|it| it.path.to_string_lossy() == line.trim());
        }
    }
    let mut next = 0;
    for w in wall.iter_mut() {
        if w.is_none() {
            while next < items.len() && wall_contains(&[*w], next) {
                next += 1;
            }
            if next < items.len() {
                *w = Some(next);
                next += 1;
            }
        }
    }
    // sem repetição quando o arquivo e o preenchimento coincidem
    if wall[0].is_some() && wall[0] == wall[1] {
        wall[1] = None;
    }
    wall
}

fn wall_contains(w: &[Option<usize>], i: usize) -> bool {
    w.contains(&Some(i))
}

impl Scene for GalleryRoom {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        self.clock += dt;
        let mut fx = self.common.update(input);
        if !self.arrived {
            self.arrived = true;
            if !self.overlay {
                self.play(&[sfx::ARRIVAL]);
            }
        }
        if let Some(p) = &mut self.print {
            // a prévia de impressão cobre a sala e recebe tudo
            fx.extend(p.update(dt, assets));
            let mut closed = false;
            for i in input {
                if p.input(i, assets, &mut fx) {
                    closed = true;
                    break;
                }
            }
            if closed {
                self.print = None;
            }
            self.pump_sounds(dt, &mut fx, assets);
            return fx;
        }
        if let Some(a) = &mut self.surprise {
            fx.extend(a.update(dt).into_iter().map(Effect::PlaySound));
            if a.is_finished() {
                self.surprise = None;
                self.bag.open = true;
                // como no original, o balão de boas-vindas volta
                self.ask(Ask::Welcome, WELCOME, assets);
            }
        }
        for i in input {
            match *i {
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.mouse = (x, y);
                    if self.surprise.is_none() {
                        self.press(x, y, &mut fx, assets);
                    }
                }
                Input::MouseMove { x, y } => {
                    self.mouse = (x, y);
                    if let Some(d) = self.drag.as_mut() {
                        d.pos = (x, y);
                        // só a pasta (por cima de uma sala): não há parede para arrastar
                        if !self.overlay && (x - d.start.0).abs() + (y - d.start.1).abs() > 4 {
                            d.moved = true;
                            // arrastando da pasta: ela se fecha para mostrar a parede
                            if matches!(d.from, Source::Folder(_)) {
                                self.folder_open = false;
                            }
                        }
                    }
                }
                Input::MouseUp {
                    button: MouseButton::Left,
                    x,
                    y,
                } => self.release(x, y, &mut fx, assets),
                Input::Key { .. } | Input::Text(_) if self.folder_open => {
                    self.binder.key(i, &self.items, &mut fx, assets);
                }
                _ => {}
            }
        }
        self.sync_cursor(&mut fx);
        self.pump_sounds(dt, &mut fx, assets);
        fx
    }

    fn overlay(&self) -> bool {
        self.overlay
    }

    fn draw(&self, f: &mut Frame, a: &Assets) {
        if self.overlay {
            self.binder.draw(f, a, &self.items, &self.pal);
            return;
        }
        Self::draw_background(f);
        for (img, mask, x, y) in ELEMENTS {
            f.blit_masked(
                &a.image(&ImageRef::Pic(Program::Artist, img)),
                &a.image(&ImageRef::Pic(Program::Artist, mask)),
                x,
                y,
            );
        }
        let frame_img = a.image(&ImageRef::Pic(Program::Artist, FRAME_IMG.0));
        let frame_mask = a.image(&ImageRef::Pic(Program::Artist, FRAME_IMG.1));
        for (k, &(x, y)) in FRAMES.iter().enumerate() {
            f.blit_masked(&frame_img, &frame_mask, x, y);
            let dragging = matches!(self.drag, Some(d) if d.moved && d.from == Source::Wall(k));
            if let Some(i) = self.wall[k]
                && !dragging
            {
                let it = &self.items[i];
                self.draw_thumb(f, &it.thumb, x + PICTURE_OFF.0, y + PICTURE_OFF.1);
                Self::draw_text_centered(
                    f,
                    a,
                    (x + PLATE.0, y + PLATE.1, PLATE.2, PLATE.3),
                    &it.name,
                );
            }
        }
        self.bag.draw(f, a);
        if let Some(c) = &self.character {
            c.draw(f, a);
        }
        if let Some(s) = &self.surprise {
            s.draw(f, a);
        }
        if self.folder_open {
            self.binder.draw(f, a, &self.items, &self.pal);
        }
        if let Some(d) = self.drag
            && d.moved
            && let Some(i) = self.item_of(d.from)
        {
            let t = &self.items[i].thumb;
            let (x, y) = (d.pos.0 - t.w as i32 / 2, d.pos.1 - t.h as i32 / 2);
            f.rect(x - 2, y - 2, t.w as i32 + 4, t.h as i32 + 4, BLACK);
            self.draw_thumb(f, t, x, y);
        }
        if let Some((_, b)) = &self.balloon {
            b.draw(f, a);
        }
        if let Some(al) = &self.alert {
            al.balloon.draw(f, a);
        }
        if let Some(p) = &self.print {
            p.draw(f, a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::store::save_drawing;
    use crate::scenes::testutil::empty_assets;

    fn down(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }
    fn up(x: i32, y: i32) -> Input {
        Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        }
    }
    fn mv(x: i32, y: i32) -> Input {
        Input::MouseMove { x, y }
    }

    fn setup(n: u64) -> (tempfile::TempDir, tempfile::TempDir, Assets) {
        let (d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = a.palettes(Program::Artist).pal256.clone();
        for k in 0..n {
            let mut c = Canvas::new(CANVAS_W, CANVAS_H);
            c.px.fill(k as u8 + 2);
            save_drawing(dir.path(), &c, &pal, true, 1_793_404_800 + 60 * k).unwrap();
        }
        (d, dir, a)
    }

    fn room(dir: &Path, a: &Assets) -> GalleryRoom {
        let mut g = GalleryRoom::new(None, Some(dir.to_path_buf()), a);
        g.balloon = None;
        g
    }

    #[test]
    fn wall_shows_most_recent_and_names() {
        let (_d, dir, a) = setup(4);
        let g = room(dir.path(), &a);
        assert_eq!(g.items().len(), 4);
        assert_eq!(g.wall(), [Some(0), Some(1), Some(2)]);
        assert!(
            g.items()[0].name.starts_with("Desenho "),
            "{}",
            g.items()[0].name
        );
    }

    #[test]
    fn double_click_opens_in_paint() {
        let (_d, dir, a) = setup(2);
        let mut g = room(dir.path(), &a);
        let (x, y) = (FRAMES[0].0 + 100, FRAMES[0].1 + 60);
        let first = g.update(&[down(x, y), up(x, y)], 0.05, &a);
        assert!(!first.contains(&Effect::ChangeScene(SceneId::Paint)));
        let fx = g.update(&[down(x, y), up(x, y)], 0.1, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        assert!(dir.path().join(LAST_NAME).exists());
    }

    #[test]
    fn drag_swaps_frames_and_persists() {
        let (_d, dir, a) = setup(3);
        let mut g = room(dir.path(), &a);
        let (x0, y0) = (FRAMES[0].0 + 100, FRAMES[0].1 + 60);
        let (x1, y1) = (FRAMES[1].0 + 100, FRAMES[1].1 + 60);
        g.update(
            &[down(x0, y0), mv(x0 + 50, y0), mv(x1, y1), up(x1, y1)],
            0.1,
            &a,
        );
        assert_eq!(g.wall(), [Some(1), Some(0), Some(2)]);
        let g2 = room(dir.path(), &a);
        assert_eq!(g2.wall(), [Some(1), Some(0), Some(2)]);
    }

    fn drag_to(g: &mut GalleryRoom, k: usize, to: (i32, i32), a: &Assets) -> Vec<Effect> {
        let (x0, y0) = (FRAMES[k].0 + 100, FRAMES[k].1 + 60);
        g.update(&[down(x0, y0), mv(to.0, to.1), up(to.0, to.1)], 0.1, a)
    }

    fn pick_option(g: &mut GalleryRoom, pos: usize, a: &Assets) -> Vec<Effect> {
        let r = g
            .alert
            .as_ref()
            .unwrap()
            .balloon
            .option_rect(a, pos)
            .unwrap();
        g.update(&[down(r.0 + 4, r.1 + 4), up(r.0 + 4, r.1 + 4)], 0.1, a)
    }

    const TRASH_AT: (i32, i32) = (TRASH.0 + 40, TRASH.1 + 50);

    #[test]
    fn trash_asks_and_deletes() {
        let (_d, dir, a) = setup(2);
        let mut g = room(dir.path(), &a);
        let path = g.wall_path(0).unwrap().to_path_buf();
        let fx = drag_to(&mut g, 0, TRASH_AT, &a);
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::TRASH))));
        let al = g.alert.as_ref().unwrap();
        assert_eq!(al.kind, Modal::Trash(0, Some(0)));
        assert_eq!(al.balloon.options().len(), 3);
        // fora das opções: a vinheta de recusa, e o aviso fica
        let fx = g.update(&[down(5, 470), up(5, 470)], 0.1, &a);
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::REFUSE))));
        assert!(g.alert.is_some());
        // "Jogue no lixo!"
        pick_option(&mut g, 0, &a);
        assert!(g.alert.is_none());
        assert!(!path.exists());
        assert_eq!(g.items().len(), 1);
        assert_eq!(g.wall(), [None, Some(0), None]);
    }

    /// "Tire da parede": o arquivo fica na pasta e a moldura recebe outro.
    #[test]
    fn trash_take_off_the_wall_keeps_the_file() {
        let (_d, dir, a) = setup(4);
        let mut g = room(dir.path(), &a);
        let path = g.wall_path(0).unwrap().to_path_buf();
        drag_to(&mut g, 0, TRASH_AT, &a);
        pick_option(&mut g, 1, &a);
        assert!(path.exists());
        assert_eq!(g.items().len(), 4);
        assert_eq!(g.wall(), [Some(3), Some(1), Some(2)]);
        // "Ops! Esquece": nada muda
        drag_to(&mut g, 1, TRASH_AT, &a);
        pick_option(&mut g, 2, &a);
        assert_eq!(g.wall(), [Some(3), Some(1), Some(2)]);
    }

    /// Um exemplo na lixeira: o Rex recusa ("Não, obrigado…"); qualquer
    /// clique fecha e a moldura ganha outro quadro.
    #[test]
    fn trash_refuses_samples() {
        let (_d, dir, a) = setup(4);
        let mut g = room(dir.path(), &a);
        g.items[0].sample = true;
        let path = g.wall_path(0).unwrap().to_path_buf();
        let fx = drag_to(&mut g, 0, TRASH_AT, &a);
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::TRASH))));
        assert_eq!(g.alert.as_ref().unwrap().kind, Modal::NotYours(Some(0)));
        assert!(g.alert.as_ref().unwrap().balloon.options().is_empty());
        g.update(&[down(5, 470), up(5, 470)], 0.1, &a);
        assert!(g.alert.is_none());
        assert!(path.exists());
        assert_eq!(g.wall(), [Some(3), Some(1), Some(2)]);
    }

    /// Tchau: o "EPA!" modal com Sair / Continuar.
    #[test]
    fn tchau_asks_before_quitting() {
        let (_d, dir, a) = setup(0);
        let mut g = room(dir.path(), &a);
        let (x, y) = (TCHAU.0 + 20, TCHAU.1 + 20);
        let fx = g.update(&[down(x, y), up(x, y)], 0.1, &a);
        assert!(!fx.contains(&Effect::Quit));
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::BYE))));
        assert_eq!(g.alert.as_ref().unwrap().kind, Modal::Quit);
        // a seta do saguão não responde enquanto o aviso está aberto
        let fx = g.update(&[down(560, 295), up(560, 295)], 0.1, &a);
        assert!(!fx.contains(&Effect::ChangeScene(SceneId::Building)));
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::REFUSE))));
        // Continuar trabalhando
        let fx = pick_option(&mut g, 1, &a);
        assert!(!fx.contains(&Effect::Quit));
        assert!(g.alert.is_none());
        g.update(&[down(x, y), up(x, y)], 0.1, &a);
        let fx = pick_option(&mut g, 0, &a);
        assert!(fx.contains(&Effect::Quit));
    }

    /// Clique simples na moldura: "Se você quiser trabalhar neste quadro…";
    /// "Estou pronto para ir!" abre a pintura com o título do quadro.
    #[test]
    fn single_click_asks_to_work_on_it() {
        let (_d, dir, a) = setup(1);
        let pal = a.palettes(Program::Artist).pal256.clone();
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(5);
        crate::paint::store::save_mag_titled(&c, &pal, &dir.path().join("Casa.mag"), "Casa")
            .unwrap();
        let mut g = room(dir.path(), &a);
        let k = (0..3)
            .find(|&k| g.wall_path(k).is_some_and(|p| p.ends_with("Casa.mag")))
            .unwrap();
        let (x, y) = (FRAMES[k].0 + 100, FRAMES[k].1 + 60);
        let fx = g.update(&[down(x, y), up(x, y)], 0.1, &a);
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::PICK))));
        let (kind, b) = g.balloon.as_ref().unwrap();
        assert!(matches!(kind, Ask::WorkOn(_)));
        // ao lado da moldura, dentro da tela
        assert!(b.x >= 0 && b.x + b.w <= 640);
        let r = b.option_rect(&a, 0);
        let fx = g.update(&[down(r.0 + 4, r.1 + 4), up(r.0 + 4, r.1 + 4)], 0.1, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        assert_eq!(
            crate::paint::store::load_name(dir.path()).as_deref(),
            Some("Casa")
        );
        // um desenho sem título abre como "Quadro Novo"
        let j = (0..3)
            .find(|&j| {
                g.wall_path(j)
                    .is_some_and(|p| p.extension().is_some_and(|e| e == "png"))
            })
            .unwrap();
        let (x, y) = (FRAMES[j].0 + 100, FRAMES[j].1 + 60);
        g.update(&[down(x, y), up(x, y)], 0.1, &a);
        let r = g.balloon.as_ref().unwrap().1.option_rect(&a, 0);
        g.update(&[down(r.0 + 4, r.1 + 4), up(r.0 + 4, r.1 + 4)], 0.1, &a);
        assert_eq!(crate::paint::store::load_name(dir.path()), None);
    }

    /// Impressora e lixeira com clique simples: som e dica (sem dica depois
    /// de "Você quer me ver menos? Sim").
    #[test]
    fn printer_and_trash_clicks_show_tips() {
        let (_d, dir, a) = setup(0);
        let mut g = room(dir.path(), &a);
        let (px, py) = (PRINTER.0 + 30, PRINTER.1 + 30);
        let fx = g.update(&[down(px, py), up(px, py)], 0.1, &a);
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::PRINTER))));
        assert!(matches!(g.balloon, Some((Ask::Tip, _))));
        g.balloon = None;
        let fx = g.update(
            &[down(TRASH_AT.0, TRASH_AT.1), up(TRASH_AT.0, TRASH_AT.1)],
            0.1,
            &a,
        );
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::TRASH))));
        assert!(matches!(g.balloon, Some((Ask::Tip, _))));
        g.balloon = None;
        g.help_off = true;
        g.update(&[down(px, py), up(px, py)], 0.1, &a);
        assert!(g.balloon.is_none());
    }

    /// McZee: "Você quer me ver menos?"; com "Sim", o próximo clique
    /// pergunta se ele volta a ajudar.
    #[test]
    fn mczee_asks_to_be_seen_less() {
        let (_d, dir, a) = setup(0);
        let mut g = room(dir.path(), &a);
        let Some(c) = &g.character else { return };
        let (x, y) = (40, c.y + 60);
        let fx = g.update(&[down(x, y), up(x, y)], 0.1, &a);
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::MCZEE))));
        let r = match &g.balloon {
            Some((Ask::HelpOff, b)) => b.option_rect(&a, 0),
            _ => panic!("sem o balão"),
        };
        g.update(&[down(r.0 + 4, r.1 + 4), up(r.0 + 4, r.1 + 4)], 0.1, &a);
        assert!(g.help_off);
        g.update(&[down(x, y), up(x, y)], 0.1, &a);
        let r = match &g.balloon {
            Some((Ask::HelpOn, b)) => b.option_rect(&a, 0),
            _ => panic!("sem o balão de volta"),
        };
        g.update(&[down(r.0 + 4, r.1 + 4), up(r.0 + 4, r.1 + 4)], 0.1, &a);
        assert!(!g.help_off);
        assert!(matches!(g.balloon, Some((Ask::Welcome, _))));
    }

    #[test]
    fn arrival_speech_once() {
        let (_d, dir, a) = setup(0);
        let mut g = room(dir.path(), &a);
        let fx = g.update(&[], 0.1, &a);
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::ARRIVAL))));
        for _ in 0..200 {
            let fx = g.update(&[], 0.1, &a);
            assert!(!fx.contains(&Effect::PlaySound(snd(sfx::ARRIVAL))));
        }
    }

    fn click_at(g: &mut GalleryRoom, p: (i32, i32), a: &Assets) -> Vec<Effect> {
        g.update(&[down(p.0, p.1), up(p.0, p.1)], 0.1, a)
    }

    fn center(r: Rect) -> (i32, i32) {
        (r.0 + r.2 / 2, r.1 + r.3 / 2)
    }

    /// A maleta abre o fichário; a miniatura do quadro escolhido vai para
    /// uma moldura (o fichário some ao arrastar).
    #[test]
    fn folder_places_on_wall() {
        let (_d, dir, a) = setup(5);
        let mut g = room(dir.path(), &a);
        click_at(&mut g, (FOLDER.0 + 20, FOLDER.1 + 20), &a);
        assert!(g.folder_open());
        // escolhe o quinto quadro na lista
        let p = g.binder.row_point(4).unwrap();
        let fx = click_at(&mut g, p, &a);
        assert!(fx.contains(&Effect::PlaySound(snd(portfolio::sfx::SELECT))));
        assert_eq!(g.binder.selected(), Some(4));
        let (cx, cy) = center(portfolio::THUMB);
        let (wx, wy) = (FRAMES[2].0 + 100, FRAMES[2].1 + 60);
        g.update(
            &[down(cx, cy), mv(cx + 30, cy), mv(wx, wy), up(wx, wy)],
            0.1,
            &a,
        );
        assert!(!g.folder_open());
        assert_eq!(g.wall(), [Some(0), Some(1), Some(4)]);
    }

    /// JÁ abre o escolhido; clique duplo numa linha também; X fecha.
    #[test]
    fn folder_buttons() {
        let (_d, dir, a) = setup(3);
        let mut g = room(dir.path(), &a);
        click_at(&mut g, (FOLDER.0 + 20, FOLDER.1 + 20), &a);
        let p = g.binder.row_point(2).unwrap();
        click_at(&mut g, p, &a);
        let fx = click_at(&mut g, center(portfolio::BUTTON_GO), &a);
        assert!(fx.contains(&Effect::PlaySound(snd(portfolio::sfx::GO))));
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        let fx = g.update(
            &[down(p.0, p.1), up(p.0, p.1), down(p.0, p.1), up(p.0, p.1)],
            0.1,
            &a,
        );
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        // seta do título: próximo quadro
        click_at(&mut g, center(portfolio::TITLE_NEXT), &a);
        assert_eq!(g.binder.selected(), Some(0));
        let fx = click_at(&mut g, center(portfolio::BUTTON_CLOSE), &a);
        assert!(fx.contains(&Effect::PlaySound(snd(portfolio::sfx::CLOSE))));
        assert!(!g.folder_open());
    }

    /// O inseto procura pelo nome (sem acento nem maiúscula).
    #[test]
    fn folder_search_finds_by_name() {
        let (_d, dir, a) = setup(2);
        let pal = a.palettes(Program::Artist).pal256.clone();
        let c = Canvas::new(CANVAS_W, CANVAS_H);
        crate::paint::store::save_mag_titled(&c, &pal, &dir.path().join("p.mag"), "Pássaro")
            .unwrap();
        let mut g = room(dir.path(), &a);
        let target = g
            .items()
            .iter()
            .position(|it| it.name == "Pássaro")
            .unwrap();
        click_at(&mut g, (FOLDER.0 + 20, FOLDER.1 + 20), &a);
        let fx = click_at(&mut g, center(portfolio::BUTTON_SEARCH), &a);
        assert!(fx.contains(&Effect::PlaySound(snd(portfolio::sfx::SEARCH))));
        assert!(g.binder.search_open());
        g.update(&[Input::Text("PASSA".into())], 0.1, &a);
        let r = g
            .binder
            .search_balloon()
            .unwrap()
            .option_rect(&a, 0)
            .unwrap();
        click_at(&mut g, (r.0 + 4, r.1 + 4), &a);
        assert!(!g.binder.search_open());
        assert_eq!(g.binder.selected(), Some(target));
        // nada achado: recusa e o balão fica
        click_at(&mut g, center(portfolio::BUTTON_SEARCH), &a);
        g.update(&[Input::Text("zzz".into())], 0.1, &a);
        let enter = Input::Key {
            key: crate::scene::Key::Enter,
            ctrl: false,
            alt: false,
        };
        let fx = g.update(&[enter], 0.1, &a);
        assert!(fx.contains(&Effect::PlaySound(snd(portfolio::sfx::REFUSE))));
        assert!(g.binder.search_open());
    }

    /// Só a pasta, aberta pela maleta de uma sala: por cima dela, clique
    /// duplo abre o quadro e fechar volta à sala.
    #[test]
    fn portfolio_over_a_room() {
        let (_d, dir, a) = setup(2);
        let mut g = GalleryRoom::portfolio(None, Some(dir.path().to_path_buf()), &a);
        assert!(g.overlay());
        assert!(g.folder_open());
        let (cx, cy) = center(portfolio::THUMB);
        // arrastar não tira a figura da pasta (não há parede aqui)
        g.update(&[down(cx, cy), mv(cx + 40, cy), up(cx + 40, cy)], 0.1, &a);
        assert!(g.folder_open());
        let fx = g.update(
            &[down(cx, cy), up(cx, cy), down(cx, cy), up(cx, cy)],
            0.1,
            &a,
        );
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Paint)));
        let mut g = GalleryRoom::portfolio(None, Some(dir.path().to_path_buf()), &a);
        let fx = click_at(&mut g, center(portfolio::BUTTON_CLOSE), &a);
        assert!(fx.contains(&Effect::Back));
        let mut g = GalleryRoom::portfolio(None, Some(dir.path().to_path_buf()), &a);
        let fx = click_at(&mut g, (5, 5), &a);
        assert!(fx.contains(&Effect::Back));
        // por cima da sala: só o fichário é desenhado
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        f.fill([1, 2, 3]);
        g.draw(&mut f, &a);
        assert_eq!(f.pixel(5, 470)[..3], [1, 2, 3]);
        // e sem fala de chegada
        let mut g = GalleryRoom::portfolio(None, Some(dir.path().to_path_buf()), &a);
        let fx = g.update(&[], 0.1, &a);
        assert!(!fx.contains(&Effect::PlaySound(snd(sfx::ARRIVAL))));
    }

    /// Quadro solto na impressora: a prévia com o McZee; Para volta.
    #[test]
    fn printer_opens_the_preview() {
        let (_d, dir, a) = setup(1);
        let mut g = room(dir.path(), &a);
        g.print_command = "/nao/existe/lp".into();
        let fx = drag_to(&mut g, 0, (PRINTER.0 + 30, PRINTER.1 + 30), &a);
        assert!(fx.contains(&Effect::PlaySound(snd(sfx::PRINTER))));
        let p = g.print.as_ref().unwrap();
        assert_eq!(p.stage(), super::super::gallery_print::Stage::Ready);
        let r = p
            .balloon()
            .button_rect(&a, crate::text_balloon::Button::Stop)
            .unwrap();
        // a sala não responde por baixo da prévia
        let fx = g.update(&[down(560, 295), up(560, 295)], 0.1, &a);
        assert!(!fx.contains(&Effect::ChangeScene(SceneId::Building)));
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        g.draw(&mut f, &a);
        g.update(&[down(r.0 + 3, r.1 + 3), up(r.0 + 3, r.1 + 3)], 0.1, &a);
        assert!(g.print.is_none());
    }

    #[test]
    fn arrow_goes_to_lobby() {
        let (_d, dir, a) = setup(0);
        let mut g = room(dir.path(), &a);
        let fx = g.update(&[down(560, 295), up(560, 295)], 0.1, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Building)));
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        g.draw(&mut f, &a);
    }

    /// Bolsa aberta no chão da Galeria: livro conta a história, a varinha
    /// vira o cursor e, no buraco do rato (sem dados), volta para a bolsa.
    #[test]
    fn gallery_bag_book_and_wand() {
        let (_d, dir, a) = setup(0);
        let mut g = room(dir.path(), &a);
        assert!(g.bag().open);
        let fx = g.update(&[down(270, 380), up(270, 380)], 0.1, &a);
        assert!(fx.contains(&Effect::Push(SceneId::BookStory)));
        let fx = g.update(&[down(270, 450), up(270, 450)], 0.1, &a);
        assert!(g.bag().holding_wand);
        assert!(fx.contains(&Effect::SetCursor(objects::WAND_CURSOR)));
        let fx = g.update(&[down(470, 370), up(470, 370)], 0.1, &a);
        assert!(!g.bag().holding_wand);
        assert!(g.bag().open);
        assert!(fx.contains(&Effect::PlaySound(snd(objects::SND_WAND_HOLE))));
    }

    /// Com os dados reais: a surpresa do buraco do rato toca e, no fim, a
    /// bolsa volta a ficar aberta com a varinha.
    #[test]
    fn gallery_mouse_hole_surprise_with_real_data() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let mut g = room(dir.path(), &a);
        g.update(&[down(270, 450), up(270, 450)], 0.1, &a);
        g.update(&[down(470, 370), up(470, 370)], 0.1, &a);
        assert!(g.surprise_playing());
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        g.update(&[], 0.5, &a);
        g.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "gallery_hole");
        for _ in 0..80 {
            g.update(&[], 0.1, &a);
        }
        assert!(!g.surprise_playing());
        assert!(g.bag().open && !g.bag().holding_wand);
        assert!(matches!(g.balloon, Some((Ask::Welcome, _))));
    }
}
