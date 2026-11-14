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
//!
//! O que muda por programa é um [`GallerySpec`]: [`ARTIST_GALLERY`] (a
//! Galeria do Fine Artist) e o `WRITER_LIBRARY` (a Biblioteca do Creative
//! Writer, em `writer_library`: quatro molduras com as miniaturas dos
//! documentos, interruptor e gaveta de unidades, avisos do `0805`). O que os
//! trabalhos são (desenhos ou documentos: listar, abrir, imprimir, jogar
//! fora, gravar a parede) é um [`Works`] dentro do spec.

use super::gags::{self, Character};
use super::gallery_print::PrintPreview;
use super::portfolio::{self, Action, Binder};
use super::room_objects::{self as objects, BagLayout, BagPart, MagicBag, Surprises, Target};
use super::room_spec::RoomKit;
use super::room_talk::{self, Helper};
use super::rooms_data;
use crate::Frame;
use crate::anim::Animator;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::Balloon;
use crate::paint::Canvas;
use crate::paint::drawings;
use crate::paint::store::{MAG_TITLE, NAME_FILE, load_picture};
use crate::rng::XorShift;
use crate::scene::{DEFAULT_CURSOR, Effect, Input, KidCommon, MouseButton, Scene, SceneId};
use crate::text_balloon::{PIC_ALERT, Part, TextBalloon};
use crate::ui::{DoubleClick, SoundQueue, blit_canvas, clip_text};
use crate::ui::{Rect, inside};
use formats::palette::Rgb;
use std::path::{Path, PathBuf};

/// Molduras da parede (`6c79`, máscara `6c7d`, 200×140).
pub const FRAMES: [(i32, i32); 3] = [(95, 50), (323, 50), (211, 195)];
const FRAME_IMG: (u16, u16) = (0x6c79, 0x6c7d);
/// Área do desenho dentro da moldura (1/4 do quadro) e a plaqueta do nome.
const PICTURE_OFF: (i32, i32) = (24, 25);
const PLATE: Rect = (32, 117, 140, 12);

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

/// Arquivo com os nomes dos quadros das três molduras.
pub const WALL_FILE: &str = "galeria.txt";

const BLACK: [u8; 3] = [0, 0, 0];
const WALL_A: [u8; 3] = [136, 136, 230];
const WALL_B: [u8; 3] = [71, 140, 80];
const FLOOR_A: [u8; 3] = [100, 100, 100];
const BASEBOARD: (i32, i32) = (375, 395);

/// Figura fixa da sala: (cor, máscara, x, y); máscara 0 = sem máscara.
pub type Element = (u16, u16, i32, i32);

/// Como o trabalho aparece dentro da moldura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picture {
    /// A miniatura inteira, com o canto neste ponto da moldura (Fine
    /// Artist: o quadro a 1/4).
    Offset((i32, i32)),
    /// A área (relativa à moldura) é pintada com `bg` e a miniatura, já
    /// reduzida para caber nela, fica no meio (Writer: a página em cinza).
    /// Com `edges`, a página ganha um traço preto de cada lado (por cima da
    /// primeira coluna e logo depois da última), como no original.
    Fit {
        area: Rect,
        bg: [u8; 3],
        edges: bool,
    },
}

/// Registros do bloco `WHAT` da sala (`kit.what`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GalleryRecords {
    /// Boas-vindas e "Outra coisa...".
    pub welcome: usize,
    pub welcome2: usize,
    /// Dica de cada uma das quatro primeiras opções das boas-vindas (a
    /// quinta abre `welcome2`).
    pub welcome_tips: [usize; 4],
    /// Opções de `welcome2`, na ordem mostrada: `Some(dica)` ou `None`
    /// (volta às boas-vindas); a última vale para as que sobrarem.
    pub welcome2_actions: &'static [Option<usize>],
    /// Dicas da impressora e da lixeira (clique nelas).
    pub print: usize,
    pub trash: usize,
    /// "Se você quiser trabalhar neste…" (clique simples na moldura).
    pub work_on: usize,
    /// Balão da varinha tirada da bolsa.
    pub wand: usize,
    /// Dica do interruptor (`0bb8[25]` "Suporte de pastas"), se houver.
    pub switch: Option<usize>,
}

/// Avisos "EPA!" e o bloco deles (Fine Artist: o das salas; Writer: `0805`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GalleryAlerts {
    pub block: u16,
    /// Trabalho da criança na lixeira: da pasta (2 opções) / da parede (3).
    pub recycle: usize,
    pub recycle_wall: usize,
    /// Exemplo na lixeira: da parede (troca por outro) / da pasta.
    pub not_yours_wall: usize,
    pub not_yours: usize,
    /// Plaqueta de um exemplo ("Você não pode mudar esse título"), se houver.
    pub plate: Option<usize>,
    /// Unidade onde o documento já está ("Já está lá!").
    pub already_there: Option<usize>,
    /// Não deu para copiar para a unidade.
    pub cant_copy: Option<usize>,
}

/// Textos reserva (sem os dados originais).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GalleryTexts {
    pub welcome: (&'static str, &'static [&'static str]),
    pub welcome2: (&'static str, &'static [&'static str]),
    pub work_on: (&'static str, &'static [&'static str]),
    pub not_yours: &'static str,
    pub recycle_wall: (&'static str, &'static [&'static str]),
    pub recycle: (&'static str, &'static [&'static str]),
    pub plate: &'static str,
    pub already_there: &'static str,
    pub cant_copy: &'static str,
    /// Linha do registro (`Effect::Log`) ao tentar jogar um exemplo fora.
    pub sample_log: &'static str,
}

/// Sons (ids do `.SND` do programa).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GallerySounds {
    /// Fala de chegada (a Biblioteca do Writer não tem).
    pub arrival: Option<u16>,
    pub pick: u16,
    pub choice: u16,
    pub printer: u16,
    pub preview: u16,
    /// Clique na lixeira e trabalho solto nela.
    pub trash: u16,
    pub trash_drop: &'static [u16],
    pub alert: u16,
    pub refuse: u16,
    /// "Tchau" (luminária no Writer), antes do "EPA!".
    pub quit: &'static [u16],
    pub mczee: u16,
    pub lobby: u16,
    pub mouse_hole: u16,
    /// Interruptor da gaveta de unidades.
    pub switch: u16,
}

/// Posições dos balões.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GalleryPositions {
    /// Boas-vindas, "Outra coisa..." e o menu do McZee.
    pub welcome: (i32, i32),
    /// Dicas sem posição própria.
    pub tip: (i32, i32),
    /// Dica de cada registro, quando ela tem lugar certo (opções do Writer).
    pub tips: &'static [(usize, (i32, i32))],
    /// Dicas do clique na impressora, na lixeira e no interruptor.
    pub printer_tip: (i32, i32),
    pub trash_tip: (i32, i32),
    pub switch_tip: (i32, i32),
    /// "Se você quiser trabalhar…": distância à direita da moldura e altura
    /// em relação ao topo dela (à esquerda, se não couber)…
    pub work_on: (i32, i32),
    /// …ou, se não vazio, o lugar do balão para cada moldura.
    pub work_on_at: &'static [(i32, i32)],
    pub alert: (i32, i32),
    pub quit: (i32, i32),
    /// Aviso da plaqueta.
    pub plate_alert: (i32, i32),
}

/// Interruptor e gaveta de unidades (só no Writer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawerSpec {
    /// Interruptor desligado e ligado (cor, máscara, x, y).
    pub switch_off: Element,
    pub switch_on: Element,
    /// Área do clique no interruptor (vale nos dois estados).
    pub switch_area: Rect,
    /// A gaveta aberta.
    pub drawer: Element,
    /// Fileiras das unidades: canto da primeira, largura, altura, passo, quantas.
    pub rows: (i32, i32, i32, i32, i32, usize),
    /// Margens das fileiras ocupadas: pontas vermelhas e o meio cinza.
    pub caps: (i32, i32),
}

/// Uma unidade da gaveta: nome e pasta para onde se copia (`None` = só de
/// leitura, como os exemplos).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drive {
    pub label: String,
    pub dir: Option<PathBuf>,
}

/// O que se pede ao `Works` ao pôr um trabalho na impressora.
pub type PrintFn =
    fn(&Item, &[Rgb], Option<PathBuf>, &str, &Assets) -> Result<PrintPreview, String>;

/// Os trabalhos mostrados na sala e o que se faz com eles.
pub struct Works {
    /// Trabalhos da criança (mais recentes primeiro) e os exemplos, com as
    /// miniaturas prontas para a moldura.
    pub load: fn(Option<&Path>, &Assets, &[Rgb]) -> Vec<Item>,
    /// O que vai em cada uma das `n` molduras (índices em `items`).
    pub load_wall: fn(Option<&Path>, &Assets, &[Item], usize) -> Vec<Option<usize>>,
    /// Grava as molduras.
    pub save_wall: fn(Option<&Path>, &Assets, &[Option<&Item>]) -> Result<(), String>,
    /// Abre o trabalho (efeitos da troca de tela).
    pub open: fn(&Item, Option<&Path>, &[Rgb], Program) -> Vec<Effect>,
    /// Monta a prévia de impressão.
    pub print: PrintFn,
    /// Apaga o arquivo e os que andam com ele.
    pub remove: fn(&Item) -> std::io::Result<()>,
    /// Unidades da gaveta (vazio sem gaveta).
    pub drives: fn(Option<&Path>, &Assets) -> Vec<Drive>,
}

/// Galeria (Fine Artist) ou Biblioteca (Creative Writer).
pub struct GallerySpec {
    /// O que é comum às salas do programa: programa, bloco `WHAT`, menu do
    /// McZee, "EPA!" de saída, figuras da bolsa, cursor da varinha e sons
    /// da bolsa.
    pub kit: &'static RoomKit,
    /// Fundo inteiro; `None` = papel de parede de corações desenhado.
    pub background: Option<u16>,
    pub elements: &'static [Element],
    pub frames: &'static [(i32, i32)],
    /// Figura de cada moldura (cor, máscara), se as molduras não vierem no
    /// fundo ou nos elementos.
    pub frame_img: Option<(u16, u16)>,
    pub frame_size: (i32, i32),
    pub picture: Picture,
    /// Plaqueta do nome, relativa à moldura.
    pub plate: Rect,
    pub quit: Rect,
    pub printer: Rect,
    pub trash: Rect,
    pub lobby: Rect,
    pub mouse_hole: Rect,
    pub drawer: Option<DrawerSpec>,
    pub rec: GalleryRecords,
    pub alerts: GalleryAlerts,
    /// Opções de `welcome2` tiradas (partes do texto): as de disquete do FA.
    pub welcome2_drop: &'static [&'static str],
    pub text: GalleryTexts,
    pub snd: GallerySounds,
    pub pos: GalleryPositions,
    /// Fantasias do McZee e onde ele fica.
    pub character: (&'static [u16], (i32, i32)),
    pub bag: BagLayout,
    pub surprises: Surprises,
    pub works: &'static Works,
}

/// Os desenhos do Fine Artist.
pub const DRAWINGS: Works = Works {
    load: load_drawings,
    load_wall: |dir, _, items, _| load_wall(dir, items).to_vec(),
    save_wall: save_wall_file,
    open: open_drawing,
    print: print_drawing,
    remove: remove_drawing,
    drives: |_, _| Vec::new(),
};

/// A Galeria do Fine Artist.
pub const ARTIST_GALLERY: GallerySpec = GallerySpec {
    kit: &rooms_data::ARTIST_KIT,
    background: None,
    elements: &ELEMENTS,
    frames: &FRAMES,
    frame_img: Some(FRAME_IMG),
    frame_size: (200, 140),
    picture: Picture::Offset(PICTURE_OFF),
    plate: PLATE,
    quit: TCHAU,
    printer: PRINTER,
    trash: TRASH,
    lobby: LOBBY_ARROW,
    mouse_hole: MOUSE_HOLE,
    drawer: None,
    rec: GalleryRecords {
        welcome: WELCOME,
        welcome2: WELCOME2,
        welcome_tips: [TIP_REORGANIZE, TIP_OPEN, TIP_TRASH, TIP_PORTFOLIO],
        welcome2_actions: &[Some(TIP_PRINT), None],
        print: TIP_PRINT,
        trash: TIP_TRASH,
        work_on: WORK_ON,
        wand: WAND_LIBRARY,
        switch: None,
    },
    alerts: GalleryAlerts {
        block: WHAT,
        recycle: RECYCLE,
        recycle_wall: RECYCLE_WALL,
        not_yours_wall: NOT_YOURS_WALL,
        not_yours: NOT_YOURS,
        plate: None,
        already_there: None,
        cant_copy: None,
    },
    // as opções de disquete do original não existem aqui
    welcome2_drop: &["disquete", "unidades"],
    text: GalleryTexts {
        welcome: (
            "Seja bem-vindo à Galeria! O que você quer fazer?",
            &[
                "Reorganizar a parede",
                "Abrir um quadro",
                "Jogar fora alguns quadros",
                "Ver outros quadros",
                "Outra coisa...",
            ],
        ),
        welcome2: (
            "O que você quer fazer?",
            &["Imprimir um desenho", "Voltar para outras escolhas..."],
        ),
        work_on: (
            "Se você quiser trabalhar neste quadro, eu vou levar você para o Ateliê de Pintura na Bolsa Mágica.",
            &[
                "Estou pronto para ir!",
                "Ops! Não quero trabalhar com isto!",
            ],
        ),
        not_yours: "Não, obrigado... Não posso reciclar isso — ele não é seu.",
        recycle_wall: (
            "Tem certeza de que você quer que o Rex jogue isso fora?",
            &[
                "Jogue no lixo! Não quero ver isso nunca mais!",
                "Tire da parede, posso precisar mais tarde",
                "Ops! Esquece, pode deixar na parede",
            ],
        ),
        recycle: (
            "O Rex está se preparando para jogar isto fora... Você tem certeza?",
            &[
                "Jogue no lixo! Não quero ver isso nunca mais!",
                "Tira a mão! Posso precisar mais tarde",
            ],
        ),
        plate: "",
        already_there: "",
        cant_copy: "",
        sample_log: "os exemplos do Fine Artist não podem ir para a lixeira",
    },
    snd: GallerySounds {
        arrival: Some(sfx::ARRIVAL),
        pick: sfx::PICK,
        choice: sfx::CHOICE,
        printer: sfx::PRINTER,
        preview: sfx::PREVIEW,
        trash: sfx::TRASH,
        trash_drop: &[sfx::TRASH, sfx::ALERT],
        alert: sfx::ALERT,
        refuse: sfx::REFUSE,
        quit: &[sfx::BYE, sfx::ALERT],
        mczee: sfx::MCZEE,
        lobby: sfx::LOBBY,
        mouse_hole: sfx::MOUSE_HOLE,
        switch: 0,
    },
    pos: GalleryPositions {
        welcome: WELCOME_POS,
        tip: (150, 170),
        tips: &[],
        printer_tip: (PRINTER.0 - 190, PRINTER.1 - 40),
        trash_tip: (TRASH.0 - 180, TRASH.1 - 120),
        switch_tip: (150, 170),
        work_on: (23, 2),
        work_on_at: &[],
        alert: ALERT_POS,
        quit: QUIT_POS,
        plate_alert: ALERT_POS,
    },
    character: (gags::MCZEE_COSTUMES, gags::MCZEE_LOBBY),
    bag: objects::GALLERY_BAG,
    surprises: objects::GALLERY_SURPRISES,
    works: &DRAWINGS,
};

/// Um trabalho: arquivo, miniatura e nome para a plaqueta.
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
    /// Trabalho com a miniatura já do tamanho de mostrar (índices da paleta
    /// da sala).
    pub fn new(
        path: PathBuf,
        name: String,
        title: Option<String>,
        author: String,
        sample: bool,
        thumb: Canvas,
    ) -> Item {
        Item {
            path,
            name,
            title,
            author,
            sample,
            thumb,
        }
    }

    /// Miniatura 1/4 do quadro.
    pub fn thumb(&self) -> &Canvas {
        &self.thumb
    }
}

/// Título e autor do `.MAG` do quadro (o próprio arquivo ou o gêmeo do `.png`).
fn mag_info(path: &Path) -> (Option<String>, String) {
    let Some((t, author)) = drawings::mag_meta(path) else {
        return (None, String::new());
    };
    let title = (!t.is_empty() && t != MAG_TITLE).then_some(t);
    (title, author)
}

/// Nome para mostrar: o título do `.MAG`, ou o nome do arquivo sem o carimbo.
fn display_name(path: &Path) -> String {
    if let (Some(t), _) = mag_info(path) {
        return t;
    }
    stamped_name(path)
}

/// O nome do arquivo sem o carimbo: `desenho-AAAAMMDD-HHMMSS` → "Desenho
/// 31/10 14:05" (também `texto-…` no Writer).
pub fn stamped_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
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

/// Os desenhos da criança (mais recentes primeiro) e os exemplos da Maggie,
/// com a miniatura 1/4.
fn load_drawings(dir: Option<&Path>, assets: &Assets, pal: &[Rgb]) -> Vec<Item> {
    let mut items = Vec::new();
    let mut add = |paths: Vec<PathBuf>, sample: bool| {
        for path in paths {
            if let Some(c) = load_picture(&path, pal) {
                let (title, author) = mag_info(&path);
                items.push(Item {
                    name: display_name(&path),
                    title,
                    author,
                    // o pixel mais escuro de cada bloco: as linhas finas não somem
                    thumb: drawings::shrink_darkest(&c, 4, pal),
                    path,
                    sample,
                });
            }
        }
    };
    add(dir.map(drawings::list_drawings).unwrap_or_default(), false);
    add(drawings::list_samples(assets.data_dir()), true);
    items
}

/// `galeria.txt`: um caminho por moldura.
fn save_wall_file(dir: Option<&Path>, _: &Assets, wall: &[Option<&Item>]) -> Result<(), String> {
    let Some(dir) = dir else { return Ok(()) };
    let text: String = wall
        .iter()
        .map(|w| {
            w.map(|it| it.path.to_string_lossy().to_string())
                .unwrap_or_default()
                + "\n"
        })
        .collect();
    std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::write(dir.join(WALL_FILE), text))
        .map_err(|e| e.to_string())
}

/// Abre o quadro na pintura: ele vira o último desenho, com o seu título
/// no rótulo (ou "Quadro Novo", se não tiver um).
fn open_drawing(it: &Item, dir: Option<&Path>, pal: &[Rgb], p: Program) -> Vec<Effect> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    let Some(c) = load_picture(&it.path, pal) else {
        return vec![Effect::Log(format!(
            "galeria: não consegui abrir {}",
            it.path.display()
        ))];
    };
    if let Err(e) = crate::paint::store::open_as_last(dir, &c, &it.path, pal) {
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
    fx.push(Effect::ChangeScene(SceneId::Paint.in_program(p)));
    fx
}

/// A prévia do quadro (o desenho inteiro na folha).
fn print_drawing(
    it: &Item,
    pal: &[Rgb],
    dir: Option<PathBuf>,
    command: &str,
    assets: &Assets,
) -> Result<PrintPreview, String> {
    let c = load_picture(&it.path, pal)
        .ok_or_else(|| format!("imprimir: não consegui abrir {}", it.path.display()))?;
    Ok(PrintPreview::new(c, pal.to_vec(), dir, command, assets))
}

/// Apaga o desenho, o `.mag` gêmeo do `.png` e os objetos, o fundo e os
/// metadados de projeto gravados ao lado.
fn remove_drawing(it: &Item) -> std::io::Result<()> {
    std::fs::remove_file(&it.path)?;
    let twin = it.path.with_extension("mag");
    if twin != it.path {
        let _ = std::fs::remove_file(twin);
    }
    if let Some(stem) = it
        .path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
    {
        let [objects, background] = crate::paint::store::companions(&stem);
        for name in [objects, background, super::projects::meta_name(&stem)] {
            let _ = std::fs::remove_file(it.path.with_file_name(name));
        }
    }
    Ok(())
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
    /// Aviso sem opções (plaqueta de um exemplo, unidades).
    Notice,
}

struct Alert {
    kind: Modal,
    balloon: TextBalloon,
    /// Índice do original de cada opção, na ordem em que aparecem.
    ids: Vec<u16>,
}

pub struct GalleryRoom {
    spec: &'static GallerySpec,
    common: KidCommon,
    dir: Option<PathBuf>,
    pal: Vec<Rgb>,
    items: Vec<Item>,
    /// Índice em `items` do trabalho de cada moldura.
    wall: Vec<Option<usize>>,
    folder_open: bool,
    /// O fichário verde (a pasta de trabalhos).
    binder: Binder,
    drag: Option<Drag>,
    clicks: DoubleClick<Source>,
    /// Relógio do clique duplo (s): cada quadro conta no máximo
    /// [`crate::scene::CLICK_FRAME_MAX`].
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
    sounds: SoundQueue,
    /// Programa: as saídas levam às telas dele (`SceneId::in_program`).
    program: Program,
    /// Gaveta de unidades aberta (interruptor ligado) e as unidades.
    drawer_open: bool,
    drives: Vec<Drive>,
}

impl GalleryRoom {
    pub fn new(click: Option<SoundRef>, dir: Option<PathBuf>, assets: &Assets) -> GalleryRoom {
        GalleryRoom::with_spec(&ARTIST_GALLERY, click, dir, assets)
    }

    /// Sala com os dados de `spec`.
    pub fn with_spec(
        spec: &'static GallerySpec,
        click: Option<SoundRef>,
        dir: Option<PathBuf>,
        assets: &Assets,
    ) -> GalleryRoom {
        let program = spec.kit.program;
        let pal = assets.palettes(program).pal256.clone();
        let items = (spec.works.load)(dir.as_deref(), assets, &pal);
        let wall = (spec.works.load_wall)(dir.as_deref(), assets, &items, spec.frames.len());
        let folder = dir
            .as_ref()
            .and_then(|d| d.file_name())
            .map_or("(sem pasta)".to_string(), |n| {
                format!("{}/", n.to_string_lossy())
            });
        let mut binder = Binder::new(folder);
        binder.rebuild(&items);
        let mut rng = XorShift::from_clock();
        let (costumes, pos) = spec.character;
        let character = Character::pick_in(program, costumes, pos, &mut rng);
        let drives = (spec.works.drives)(dir.as_deref(), assets);
        let mut g = GalleryRoom {
            spec,
            common: KidCommon::new(click),
            dir,
            pal,
            items,
            wall,
            folder_open: false,
            binder,
            drag: None,
            clicks: DoubleClick::new(),
            clock: 0.0,
            balloon: None,
            alert: None,
            print: None,
            print_command: PRINT_COMMAND.to_string(),
            character,
            help_off: false,
            mouse: (0, 0),
            overlay: false,
            bag: MagicBag::with_pics(spec.kit.bag, spec.bag, true),
            surprise: None,
            rng,
            wand_cursor: false,
            arrived: false,
            sounds: SoundQueue::new(),
            program,
            drawer_open: false,
            drives,
        };
        g.ask(Ask::Welcome, spec.rec.welcome, assets);
        g
    }

    fn snd(&self, id: u16) -> SoundRef {
        SoundRef::Snd(self.spec.kit.program, id)
    }

    /// Toca sons em sequência: o primeiro já, os outros quando o anterior
    /// acabar. Uma ação nova interrompe a sequência pendente.
    fn play(&mut self, ids: &[u16]) {
        let p = self.spec.kit.program;
        self.sounds.play(ids.iter().map(|&id| SoundRef::Snd(p, id)));
    }

    fn pump_sounds(&mut self, dt: f32, fx: &mut Vec<Effect>, assets: &Assets) {
        self.sounds.pump(dt, fx, assets);
    }

    /// Aviso "EPA!" do bloco `block`, registro `k`, em `pos`; o reserva vale
    /// sem o executável.
    fn alert(&mut self, kind: Modal, block: u16, k: usize, pos: (i32, i32), assets: &Assets) {
        let t = &self.spec.text;
        if kind == Modal::Quit {
            // o mesmo "EPA!" da Entrada e das salas; aqui com o fundo de
            // balão, como os outros avisos da Galeria
            let mut balloon = room_talk::quit_balloon_in(self.spec.kit.quit, assets, pos);
            balloon.alert = false;
            self.balloon = None;
            self.alert = Some(Alert {
                kind,
                balloon,
                // no original, "Sair" é a opção 1 e "Continuar" a 0
                ids: vec![OPT_QUIT, 0],
            });
            return;
        }
        let (fallback, opts): (&str, &[&str]) = match kind {
            Modal::Quit => unreachable!("tratado acima"),
            Modal::NotYours(_) => (t.not_yours, &[]),
            Modal::Notice => (
                match k {
                    _ if Some(k) == self.spec.alerts.plate => t.plate,
                    _ if Some(k) == self.spec.alerts.already_there => t.already_there,
                    _ => t.cant_copy,
                },
                &[],
            ),
            Modal::Trash(_, Some(_)) => t.recycle_wall,
            Modal::Trash(_, None) => t.recycle,
        };
        let fallback_ids: Vec<u16> = match kind {
            Modal::Trash(_, Some(_)) => vec![OPT_TRASH, OPT_KEEP, 2],
            Modal::Trash(_, None) => vec![OPT_TRASH, OPT_KEEP],
            Modal::Quit | Modal::NotYours(_) | Modal::Notice => Vec::new(),
        };
        let (balloon, ids) = match assets.dialogs(self.spec.kit.program, block).get(k) {
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

    /// Galeria do programa `p`: a do Fine Artist ou a Biblioteca do Writer
    /// (`writer_library::WRITER_LIBRARY`).
    pub fn for_program(
        p: Program,
        click: Option<SoundRef>,
        dir: Option<PathBuf>,
        assets: &Assets,
    ) -> GalleryRoom {
        let spec = match p {
            Program::Artist => &ARTIST_GALLERY,
            Program::Writer => &super::writer_library::WRITER_LIBRARY,
        };
        GalleryRoom::with_spec(spec, click, dir, assets)
    }

    /// Pasta de trabalhos do programa `p` (`writer_portfolio`, T13).
    pub fn portfolio_for_program(
        p: Program,
        click: Option<SoundRef>,
        dir: Option<PathBuf>,
        assets: &Assets,
    ) -> GalleryRoom {
        let mut g = GalleryRoom::portfolio(click, dir, assets);
        g.program = p;
        g
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
        let s = self.spec.kit.snd;
        match part {
            BagPart::Bag => {
                let open = self.bag.holding_wand || !self.bag.open;
                self.bag.holding_wand = false;
                self.bag.open = open;
                fx.push(Effect::PlaySound(self.snd(if open {
                    s.bag_open
                } else {
                    s.bag_close
                })));
            }
            BagPart::Suitcase => {
                fx.push(Effect::PlaySound(self.snd(s.suitcase)));
                self.open_folder(assets);
            }
            BagPart::Book => {
                fx.extend(s.book.map(|id| Effect::PlaySound(self.snd(id))));
                fx.push(Effect::Push(SceneId::BookStory.in_program(self.program)));
            }
            BagPart::Wand => {
                self.bag.holding_wand = true;
                self.bag.open = false;
                fx.push(Effect::PlaySound(self.snd(s.wand_take)));
                self.ask(Ask::Tip, self.spec.rec.wand, assets);
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
                self.spec.kit.wand_cursor
            } else {
                DEFAULT_CURSOR
            }));
        }
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Índice em `items` do trabalho de cada moldura.
    pub fn wall(&self) -> &[Option<usize>] {
        &self.wall
    }

    pub fn folder_open(&self) -> bool {
        self.folder_open
    }

    /// A gaveta de unidades está aberta.
    pub fn drawer_open(&self) -> bool {
        self.drawer_open
    }

    fn frame_rect(&self, k: usize) -> Rect {
        let (x, y) = self.spec.frames[k];
        (x, y, self.spec.frame_size.0, self.spec.frame_size.1)
    }

    fn frame_at(&self, x: i32, y: i32) -> Option<usize> {
        (0..self.spec.frames.len())
            .rev()
            .find(|&k| inside(self.frame_rect(k), x, y))
    }

    fn ask(&mut self, kind: Ask, k: usize, assets: &Assets) {
        let t = &self.spec.text;
        let balloon = |(text, opts): (&str, &[&str]), w: i32| Balloon {
            x: 0,
            y: 0,
            w,
            text: text.into(),
            options: opts.iter().map(|o| o.to_string()).collect(),
            links: Vec::new(),
            figures: Vec::new(),
        };
        let fallback = match kind {
            Ask::Welcome => balloon(t.welcome, 239),
            Ask::Welcome2 => balloon(t.welcome2, 206),
            Ask::WorkOn(_) => balloon(t.work_on, 225),
            Ask::HelpOff | Ask::HelpOn => {
                // o mesmo menu do McZee das salas
                let off = kind == Ask::HelpOn;
                let b = Helper::McZee.balloon_in(
                    self.spec.kit,
                    assets,
                    super::room::DEFAULT_USER,
                    off,
                    self.spec.pos.welcome,
                );
                self.balloon = Some((kind, b));
                return;
            }
            Ask::Tip => balloon(("", &[]), 200),
        };
        let records = assets.dialogs(self.spec.kit.program, self.spec.kit.what);
        let mut b =
            Balloon::from_block(&records, k, 0, 0, super::room::DEFAULT_USER).unwrap_or(fallback);
        if kind == Ask::Welcome2 {
            let drop = self.spec.welcome2_drop;
            b.options.retain(|o| !drop.iter().any(|d| o.contains(d)));
        }
        let pos = &self.spec.pos;
        (b.x, b.y) = match kind {
            Ask::Welcome | Ask::Welcome2 | Ask::HelpOff | Ask::HelpOn => pos.welcome,
            Ask::WorkOn(_) => (0, 0),
            Ask::Tip => pos
                .tips
                .iter()
                .find(|(r, _)| *r == k)
                .map_or(pos.tip, |&(_, p)| p),
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
        self.ask(Ask::WorkOn(i), self.spec.rec.work_on, assets);
        let (fx, fy, fw, _) = self.frame_rect(k);
        let (gap, dy) = self.spec.pos.work_on;
        let fixed = self.spec.pos.work_on_at.get(k).copied();
        if let Some((_, b)) = &mut self.balloon {
            if let Some(p) = fixed {
                (b.x, b.y) = p;
                return;
            }
            let right = fx + fw + gap;
            let left = fx - b.w - gap;
            b.x = if right + b.w <= 636 {
                right
            } else if left >= 4 {
                left
            } else {
                636 - b.w
            };
            b.y = fy + dy;
        }
    }

    /// Arquivo do trabalho da moldura `k`.
    pub fn wall_path(&self, k: usize) -> Option<&Path> {
        self.wall[k].map(|i| self.items[i].path.as_path())
    }

    fn save_wall(&self, assets: &Assets) -> Option<Effect> {
        let wall: Vec<Option<&Item>> = self
            .wall
            .iter()
            .map(|w| w.and_then(|i| self.items.get(i)))
            .collect();
        (self.spec.works.save_wall)(self.dir.as_deref(), assets, &wall)
            .err()
            .map(|e| Effect::Log(format!("galeria: {e}")))
    }

    /// Abre o trabalho no programa (pintura ou área de escrita).
    fn open(&self, i: usize) -> Vec<Effect> {
        let Some(it) = self.items.get(i) else {
            return Vec::new();
        };
        (self.spec.works.open)(it, self.dir.as_deref(), &self.pal, self.program)
    }

    /// Trabalho solto na impressora: a prévia com o McZee.
    fn print(&mut self, i: usize, assets: &Assets) -> Option<Effect> {
        let it = self.items.get(i)?;
        let preview =
            (self.spec.works.print)(it, &self.pal, self.dir.clone(), &self.print_command, assets);
        match preview {
            Ok(p) => {
                self.balloon = None;
                self.play(&[self.spec.snd.printer, self.spec.snd.preview]);
                self.print = Some(p);
                None
            }
            Err(e) => Some(Effect::Log(e)),
        }
    }

    /// Outro trabalho para a moldura que ficou vazia: um exemplo que não
    /// está na parede (sorteado), ou qualquer trabalho fora dela; nunca
    /// `except`.
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

    /// Põe outro trabalho na moldura `k` e grava a parede.
    fn refill(&mut self, k: usize, assets: &Assets) -> Option<Effect> {
        let old = self.wall[k].take();
        self.wall[k] = self.replacement(old);
        self.save_wall(assets)
    }

    /// Joga fora o trabalho: só os da criança (os exemplos ficam).
    fn recycle(&mut self, i: usize, assets: &Assets) -> Vec<Effect> {
        let Some(it) = self.items.get(i) else {
            return Vec::new();
        };
        if it.sample {
            return vec![Effect::Log(self.spec.text.sample_log.into())];
        }
        let mut fx = Vec::new();
        if let Err(e) = (self.spec.works.remove)(it) {
            fx.push(Effect::Log(format!("lixeira: {e}")));
            return fx;
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
        fx.extend(self.save_wall(assets));
        fx
    }

    /// Trabalho solto na lixeira: os exemplos são recusados pelo Rex; os da
    /// criança pedem confirmação.
    fn drop_in_trash(&mut self, i: usize, from: Source, assets: &Assets) {
        self.play(self.spec.snd.trash_drop);
        let slot = match from {
            Source::Wall(k) => Some(k),
            Source::Folder(_) => None,
        };
        let (a, pos) = (self.spec.alerts, self.spec.pos.alert);
        if self.items[i].sample {
            let k = if slot.is_some() {
                a.not_yours_wall
            } else {
                a.not_yours
            };
            self.alert(Modal::NotYours(slot), a.block, k, pos, assets);
        } else {
            let k = if slot.is_some() {
                a.recycle_wall
            } else {
                a.recycle
            };
            self.alert(Modal::Trash(i, slot), a.block, k, pos, assets);
        }
    }

    /// Aviso sem opções do bloco dos avisos (registro `k`), depois dos sons
    /// `before` e da vinheta.
    fn notice(&mut self, k: Option<usize>, pos: (i32, i32), before: &[u16], assets: &Assets) {
        let Some(k) = k else { return };
        let mut ids = before.to_vec();
        ids.push(self.spec.snd.alert);
        self.play(&ids);
        self.alert(Modal::Notice, self.spec.alerts.block, k, pos, assets);
    }

    /// Unidade da gaveta sob (x, y).
    fn drive_at(&self, x: i32, y: i32) -> Option<usize> {
        let d = self.spec.drawer.as_ref()?;
        if !self.drawer_open {
            return None;
        }
        let (rx, ry, rw, rh, pitch, n) = d.rows;
        (0..n.min(self.drives.len())).find(|&k| inside((rx, ry + k as i32 * pitch, rw, rh), x, y))
    }

    /// Documento solto numa unidade: copiado para a pasta dela (se já está
    /// lá, "Já está lá!"; numa unidade só de leitura, "não posso copiar").
    fn copy_to_drive(&mut self, i: usize, k: usize, assets: &Assets) -> Vec<Effect> {
        let pos = self.spec.pos.alert;
        let (Some(it), Some(drive)) = (self.items.get(i), self.drives.get(k)) else {
            return Vec::new();
        };
        let Some(target) = drive.dir.clone() else {
            let rec = if it.sample {
                self.spec.alerts.already_there
            } else {
                self.spec.alerts.cant_copy
            };
            self.notice(rec, pos, &[], assets);
            return Vec::new();
        };
        if it.path.parent() == Some(target.as_path()) {
            self.notice(self.spec.alerts.already_there, pos, &[], assets);
            return Vec::new();
        }
        let Some(name) = it.path.file_name() else {
            return Vec::new();
        };
        let mut dest = target.join(name);
        let mut n = 2;
        while dest.exists() {
            let stem = it.path.file_stem().unwrap_or_default().to_string_lossy();
            let ext = it
                .path
                .extension()
                .map(|e| e.to_string_lossy().into_owned());
            dest = target.join(match ext {
                Some(e) => format!("{stem}-{n}.{e}"),
                None => format!("{stem}-{n}"),
            });
            n += 1;
        }
        match std::fs::create_dir_all(&target).and_then(|()| std::fs::copy(&it.path, &dest)) {
            Ok(_) => {
                self.play(&[self.spec.snd.pick]);
                self.reload(assets);
                vec![Effect::Log(format!("copiado para {}", dest.display()))]
            }
            Err(e) => {
                self.notice(self.spec.alerts.cant_copy, pos, &[], assets);
                vec![Effect::Log(format!("copiar: {e}"))]
            }
        }
    }

    /// Relê os trabalhos (depois de uma cópia), mantendo as molduras.
    fn reload(&mut self, assets: &Assets) {
        let paths: Vec<Option<PathBuf>> = self
            .wall
            .iter()
            .map(|w| w.map(|i| self.items[i].path.clone()))
            .collect();
        self.items = (self.spec.works.load)(self.dir.as_deref(), assets, &self.pal);
        self.wall = paths
            .into_iter()
            .map(|p| p.and_then(|p| self.items.iter().position(|it| it.path == p)))
            .collect();
        self.binder.rebuild(&self.items);
    }

    /// Clique com um aviso "EPA!" aberto.
    fn alert_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(a) = &self.alert else { return };
        let kind = a.kind;
        if a.ids.is_empty() {
            // sem opções: qualquer clique fecha
            self.alert = None;
            if let Modal::NotYours(Some(k)) = kind {
                fx.extend(self.refill(k, assets));
            }
            return;
        }
        let Some(pos) = a.balloon.hit_option(assets, x, y) else {
            self.play(&[self.spec.snd.refuse]);
            return;
        };
        let id = a.ids.get(pos).copied().unwrap_or(pos as u16);
        self.alert = None;
        self.play(&[self.spec.snd.choice]);
        match kind {
            Modal::Quit => {
                // no original, "Sair" é a opção 1 e "Continuar" a 0
                if id == OPT_QUIT {
                    fx.push(Effect::Quit);
                }
            }
            Modal::NotYours(_) | Modal::Notice => {}
            Modal::Trash(i, slot) => match id {
                OPT_TRASH => {
                    fx.extend(self.recycle(i, assets));
                    if let Some(k) = slot {
                        fx.extend(self.refill(k, assets));
                    }
                }
                OPT_KEEP => {
                    // "Tire da parede": o arquivo fica na pasta
                    if let Some(k) = slot {
                        fx.extend(self.refill(k, assets));
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
        if self.drive_at(x, y).is_some() {
            return None;
        }
        self.frame_at(x, y).map(Source::Wall)
    }

    fn item_of(&self, s: Source) -> Option<usize> {
        match s {
            Source::Wall(k) => self.wall[k],
            Source::Folder(i) => Some(i),
        }
    }

    /// A plaqueta da moldura `k` (área de clique).
    fn on_plate(&self, k: usize, x: i32, y: i32) -> bool {
        let (fx, fy, _, _) = self.frame_rect(k);
        let (px, py, pw, ph) = self.spec.plate;
        inside((fx + px, fy + py, pw, ph), x, y)
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
            // ligação de uma dica ("Não consegue achar um documento?")
            if let Some(r) = b.link_hit(assets, x, y) {
                self.play(&[self.spec.kit.snd.tip]);
                self.ask(Ask::Tip, r, assets);
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
            if let Some(i) = self.item_of(src) {
                if let Source::Wall(k) = src {
                    // plaqueta de um exemplo: "Você não pode mudar esse título"
                    if self.spec.alerts.plate.is_some()
                        && self.items[i].sample
                        && self.on_plate(k, x, y)
                    {
                        let rec = self.spec.alerts.plate;
                        let pick = self.spec.snd.pick;
                        self.notice(rec, self.spec.pos.plate_alert, &[pick], assets);
                        return;
                    }
                    self.play(&[self.spec.snd.pick]);
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
        let spec = self.spec;
        if self.drive_at(x, y).is_some() {
            // a unidade abre a pasta (o port só tem as pastas da gaveta)
            fx.push(Effect::PlaySound(self.snd(spec.kit.snd.suitcase)));
            self.open_folder(assets);
        } else if let Some(d) = &spec.drawer
            && inside(d.switch_area, x, y)
        {
            self.drawer_open = !self.drawer_open;
            self.play(&[spec.snd.switch]);
            if self.drawer_open
                && let Some(r) = spec.rec.switch
            {
                self.tip(r, spec.pos.switch_tip, assets);
            }
        } else if let Some(part) = self.bag.hit(x, y) {
            self.use_bag(part, fx, assets);
        } else if inside(spec.mouse_hole, x, y) {
            if !self.bag.holding_wand {
                self.play(&[spec.snd.mouse_hole]);
                return;
            }
            // a varinha no buraco do rato: uma surpresa, e ela volta à bolsa
            self.bag.holding_wand = false;
            fx.push(Effect::PlaySound(self.snd(spec.kit.snd.wand_hole)));
            match spec
                .surprises
                .start_in(spec.kit.program, Target::Hole, &mut self.rng, assets)
            {
                Some(mut a) => {
                    fx.extend(a.update(0.0).into_iter().map(Effect::PlaySound));
                    self.surprise = Some(a);
                }
                None => self.bag.open = true,
            }
        } else if inside(spec.lobby, x, y) {
            fx.push(Effect::PlaySound(self.snd(spec.snd.lobby)));
            fx.push(Effect::ChangeScene(
                SceneId::Building.in_program(self.program),
            ));
        } else if inside(spec.quit, x, y) {
            self.play(spec.snd.quit);
            self.alert(Modal::Quit, 0, 0, spec.pos.quit, assets);
        } else if inside(spec.printer, x, y) {
            self.play(&[spec.snd.printer]);
            self.tip(spec.rec.print, spec.pos.printer_tip, assets);
        } else if inside(spec.trash, x, y) {
            self.play(&[spec.snd.trash]);
            self.tip(spec.rec.trash, spec.pos.trash_tip, assets);
        } else if let Some(c) = &self.character
            && x < 120
            && y > c.y
        {
            self.play(&[spec.snd.mczee]);
            if self.help_off {
                self.ask(Ask::HelpOn, spec.kit.help_on, assets);
            } else {
                self.ask(Ask::HelpOff, spec.kit.help_off, assets);
            }
        }
    }

    fn release(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(d) = self.drag.take() else { return };
        let Some(i) = self.item_of(d.from) else {
            return;
        };
        if !d.moved {
            // clique: duplo abre o trabalho; simples, na parede, pergunta
            if self.clicks.click(d.from, self.clock, (x, y)) {
                self.balloon = None;
                fx.extend(self.open(i));
                return;
            }
            if let Source::Wall(k) = d.from {
                self.ask_work_on(i, k, assets);
            }
            return;
        }
        if inside(self.spec.printer, x, y) {
            fx.extend(self.print(i, assets));
            return;
        }
        if inside(self.spec.trash, x, y) {
            self.drop_in_trash(i, d.from, assets);
            return;
        }
        if let Some(k) = self.drive_at(x, y) {
            fx.extend(self.copy_to_drive(i, k, assets));
            return;
        }
        if let Some(k) = self.frame_at(x, y) {
            match d.from {
                Source::Wall(j) => self.wall.swap(j, k),
                Source::Folder(_) => self.wall[k] = Some(i),
            }
            fx.extend(self.save_wall(assets));
        }
    }

    fn answer(&mut self, kind: Ask, k: usize, fx: &mut Vec<Effect>, assets: &Assets) {
        if kind != Ask::Tip {
            self.play(&[self.spec.snd.choice]);
        }
        let rec = self.spec.rec;
        match (kind, k) {
            (Ask::Welcome, 0..=3) => self.ask(Ask::Tip, rec.welcome_tips[k], assets),
            (Ask::Welcome, _) => self.ask(Ask::Welcome2, rec.welcome2, assets),
            (Ask::Welcome2, _) => {
                let a = rec.welcome2_actions;
                match a.get(k).or(a.last()).copied().flatten() {
                    Some(tip) => self.ask(Ask::Tip, tip, assets),
                    None => self.ask(Ask::Welcome, rec.welcome, assets),
                }
            }
            (Ask::WorkOn(i), 0) => fx.extend(self.open(i)),
            (Ask::HelpOff, 0) => self.help_off = true,
            (Ask::HelpOff, 1) => self.ask(Ask::Welcome, rec.welcome, assets),
            (Ask::HelpOn, 0) => {
                self.help_off = false;
                self.ask(Ask::Welcome, rec.welcome, assets);
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
        blit_canvas(f, t, &self.pal, x, y);
    }

    fn draw_text_centered(f: &mut Frame, a: &Assets, r: Rect, s: &str) {
        match a.font(8) {
            Some(font) => {
                let s = clip_text(s, |t| font.width(t) as i32, r.2);
                let w = font.width(&s) as i32;
                f.text_font(&font, r.0 + (r.2 - w) / 2, r.1, &s, BLACK);
            }
            None => f.text(r.0 + 2, r.1 + 2, s, BLACK),
        }
    }

    fn draw_element(&self, f: &mut Frame, a: &Assets, (img, mask, x, y): Element) {
        let p = self.spec.kit.program;
        let image = a.image(&ImageRef::Pic(p, img));
        if mask == 0 {
            f.blit(&image, x, y);
        } else {
            f.blit_masked(&image, &a.image(&ImageRef::Pic(p, mask)), x, y);
        }
    }

    /// Interruptor e, ligado, a gaveta com as unidades.
    fn draw_drawer(&self, f: &mut Frame, a: &Assets) {
        let Some(d) = &self.spec.drawer else { return };
        if !self.drawer_open {
            self.draw_element(f, a, d.switch_off);
            return;
        }
        self.draw_element(f, a, d.switch_on);
        self.draw_element(f, a, d.drawer);
        let (rx, ry, rw, rh, pitch, n) = d.rows;
        let (cap_l, cap_r) = d.caps;
        const RED: [u8; 3] = [250, 0, 57];
        const LIGHT: [u8; 3] = [190, 190, 190];
        const SHADE: [u8; 3] = [140, 140, 140];
        for k in 0..n {
            let y = ry + k as i32 * pitch;
            f.rect(rx, y, rw, rh, BLACK);
            let Some(drive) = self.drives.get(k) else {
                continue;
            };
            f.rect(rx + 1, y + 1, cap_l - 2, rh - 2, RED);
            f.rect(rx + rw - cap_r + 1, y + 1, cap_r - 2, rh - 2, RED);
            let (gx, gw) = (rx + cap_l, rw - cap_l - cap_r);
            f.rect(gx, y, gw, rh - 1, LIGHT);
            f.rect(gx, y + rh - 1, gw, 1, SHADE);
            Self::draw_text_centered(f, a, (gx, y + 1, gw, rh - 2), &drive.label);
        }
    }

    fn draw_frames(&self, f: &mut Frame, a: &Assets) {
        let p = self.spec.kit.program;
        let frame_img = self.spec.frame_img.map(|(img, mask)| {
            (
                a.image(&ImageRef::Pic(p, img)),
                a.image(&ImageRef::Pic(p, mask)),
            )
        });
        let plate = self.spec.plate;
        for (k, &(x, y)) in self.spec.frames.iter().enumerate() {
            if let Some((img, mask)) = &frame_img {
                f.blit_masked(img, mask, x, y);
            }
            let dragging = matches!(self.drag, Some(d) if d.moved && d.from == Source::Wall(k));
            if let Picture::Fit { area, bg, .. } = self.spec.picture {
                f.rect(x + area.0, y + area.1, area.2, area.3, bg);
            }
            if let Some(i) = self.wall[k]
                && !dragging
            {
                let it = &self.items[i];
                let (w, h) = (it.thumb.w as i32, it.thumb.h as i32);
                match self.spec.picture {
                    Picture::Offset((dx, dy)) => self.draw_thumb(f, &it.thumb, x + dx, y + dy),
                    Picture::Fit { area, edges, .. } => {
                        let pw = if edges { w + 1 } else { w };
                        let (tx, ty) = (
                            x + area.0 + (area.2 - pw) / 2,
                            y + area.1 + (area.3 - h) / 2,
                        );
                        self.draw_thumb(f, &it.thumb, tx, ty);
                        if edges {
                            f.rect(tx, ty, 1, h, BLACK);
                            f.rect(tx + w, ty, 1, h, BLACK);
                        }
                    }
                }
                Self::draw_text_centered(
                    f,
                    a,
                    (x + plate.0, y + plate.1, plate.2, plate.3),
                    &it.name,
                );
            }
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
    fill_wall(&mut wall, items);
    // sem repetição quando o arquivo e o preenchimento coincidem
    if wall[0].is_some() && wall[0] == wall[1] {
        wall[1] = None;
    }
    wall
}

/// Preenche as molduras vazias com os trabalhos na ordem da lista (os da
/// criança, mais recentes primeiro, e depois os exemplos), sem repetir.
pub fn fill_wall(wall: &mut [Option<usize>], items: &[Item]) {
    let mut next = 0;
    for k in 0..wall.len() {
        if wall[k].is_none() {
            while next < items.len() && wall_contains(&[wall[k]], next) {
                next += 1;
            }
            if next < items.len() {
                wall[k] = Some(next);
                next += 1;
            }
        }
    }
}

fn wall_contains(w: &[Option<usize>], i: usize) -> bool {
    w.contains(&Some(i))
}

impl Scene for GalleryRoom {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        self.clock += crate::scene::click_time(dt);
        let mut fx = self.common.update(input);
        if !self.arrived {
            self.arrived = true;
            if !self.overlay
                && let Some(s) = self.spec.snd.arrival
            {
                self.play(&[s]);
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
                self.ask(Ask::Welcome, self.spec.rec.welcome, assets);
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
        match self.spec.background {
            Some(bg) => f.blit(&a.image(&ImageRef::Pic(self.spec.kit.program, bg)), 0, 0),
            None => Self::draw_background(f),
        }
        // as molduras do Writer vêm na figura da parede (o primeiro
        // elemento); o desenho delas vai antes dos outros elementos
        let (first, rest) = match self.spec.frame_img {
            Some(_) => (&[][..], self.spec.elements),
            None => self.spec.elements.split_at(self.spec.elements.len().min(1)),
        };
        for &e in first {
            self.draw_element(f, a, e);
        }
        if self.spec.frame_img.is_none() {
            self.draw_frames(f, a);
        }
        for &e in rest {
            self.draw_element(f, a, e);
        }
        if self.spec.frame_img.is_some() {
            self.draw_frames(f, a);
        }
        self.draw_drawer(f, a);
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
    use crate::paint::store::{LAST_NAME, save_drawing};
    use crate::paint::{CANVAS_H, CANVAS_W};
    use crate::scenes::room_objects::snd;
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

    // ---- Biblioteca do Writer (`WRITER_LIBRARY`) ----

    use crate::scenes::writer_library::{self as lib, WRITER_LIBRARY};

    fn wsnd(id: u16) -> Effect {
        Effect::PlaySound(SoundRef::Snd(Program::Writer, id))
    }

    /// `n` textos do Escritório, do mais novo (0) ao mais velho.
    fn lib_setup(n: u64) -> (tempfile::TempDir, tempfile::TempDir, Assets) {
        let (d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        for k in 0..n {
            let p = dir
                .path()
                .join(format!("texto-2026110{}-100000.txt", k + 1));
            std::fs::write(&p, format!("Texto {k}\n")).unwrap();
            let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000 - 60 * k);
            std::fs::File::options()
                .write(true)
                .open(&p)
                .unwrap()
                .set_modified(t)
                .unwrap();
        }
        (d, dir, a)
    }

    fn library(dir: &Path, a: &Assets) -> GalleryRoom {
        let mut g = GalleryRoom::with_spec(&WRITER_LIBRARY, None, Some(dir.to_path_buf()), a);
        g.balloon = None;
        g
    }

    fn lib_center(k: usize) -> (i32, i32) {
        (lib::FRAMES[k].0 + 50, lib::FRAMES[k].1 + 50)
    }

    fn lib_drag(g: &mut GalleryRoom, k: usize, to: (i32, i32), a: &Assets) -> Vec<Effect> {
        let (x0, y0) = lib_center(k);
        g.update(&[down(x0, y0), mv(to.0, to.1), up(to.0, to.1)], 0.1, a)
    }

    #[test]
    fn writer_library_four_frames_and_page_thumbs() {
        let (_d, dir, a) = lib_setup(5);
        let g = library(dir.path(), &a);
        assert_eq!(g.items().len(), 5);
        assert_eq!(g.wall(), [Some(0), Some(1), Some(2), Some(3)]);
        assert_eq!(g.items()[0].name, "Texto 01/11 10:00");
        // a página (108×144) reduzida para o quadrado de 100×100
        let t = g.items()[0].thumb();
        assert_eq!((t.w, t.h), (75, 100));
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        g.draw(&mut f, &a);
        // o cinza em volta da página, dentro da moldura
        let (x, y) = lib::FRAMES[0];
        assert_eq!(
            &f.pixel((x + 3) as u32, (y + 50) as u32)[..3],
            &[140, 140, 140]
        );
    }

    /// A parede vai para os `FILR` 1–4 do `~MAXUSER.INF`, mantendo o lugar
    /// 0 (Escritório) e as fontes gravadas antes.
    #[test]
    fn writer_library_wall_persists_in_maxuser_inf() {
        let (_d, dir, a) = lib_setup(5);
        let mut p = formats::maxprefs::UserPrefs::new();
        p.wall.push(formats::maxprefs::WallDoc {
            slot: 0,
            kind: "kwMAX".into(),
            title: "Último".into(),
            dirs: Vec::new(),
            file: "texto-20261104-100000.txt".into(),
        });
        p.fonts.push("Arial".into());
        lib::write_prefs(dir.path(), &p).unwrap();
        // sem FILR 1–4 gravados: preenchida na ordem
        let mut g = library(dir.path(), &a);
        assert_eq!(g.wall(), [Some(0), Some(1), Some(2), Some(3)]);
        lib_drag(&mut g, 0, lib_center(2), &a);
        assert_eq!(g.wall(), [Some(2), Some(1), Some(0), Some(3)]);
        let back = lib::read_prefs(dir.path()).unwrap();
        assert_eq!(back.wall_slot(0).unwrap().title, "Último");
        assert_eq!(back.fonts, ["Arial"]);
        assert_eq!(back.wall_slot(1).unwrap().file, "texto-20261103-100000.txt");
        assert!(back.wall_slot(3).unwrap().dirs.is_empty());
        let g2 = library(dir.path(), &a);
        assert_eq!(g2.wall(), [Some(2), Some(1), Some(0), Some(3)]);
    }

    #[test]
    fn writer_library_double_click_opens_in_the_office() {
        let (_d, dir, a) = lib_setup(2);
        let mut g = library(dir.path(), &a);
        let (x, y) = lib_center(1);
        let first = g.update(&[down(x, y), up(x, y)], 0.05, &a);
        assert!(first.contains(&wsnd(lib::sfx::PICK)));
        // clique simples: "Se você quiser trabalhar neste documento…" ao lado
        let (kind, b) = g.balloon.as_ref().unwrap();
        assert_eq!(*kind, Ask::WorkOn(1));
        // a do canto de cima à direita fica à esquerda dela (medido no original)
        assert_eq!((b.x, b.y), (48, 29));
        let fx = g.update(&[down(x, y), up(x, y)], 0.1, &a);
        let path = g.items()[1].path.clone();
        assert!(fx.contains(&Effect::OpenDocument(path)), "{fx:?}");
    }

    #[test]
    fn writer_library_printer_opens_the_writer_preview() {
        let (_d, dir, a) = lib_setup(1);
        let mut g = library(dir.path(), &a);
        let fx = lib_drag(&mut g, 0, (590, 140), &a);
        let p = g.print.as_ref().expect("prévia");
        assert_eq!(p.stage(), super::super::gallery_print::Stage::Ready);
        assert!(fx.contains(&wsnd(lib::sfx::PRINTER)), "{fx:?}");
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        g.draw(&mut f, &a);
        // a folha em pé, na moldura cinza (`WRITER_PREVIEW`)
        assert_eq!(&f.pixel(70, 200)[..3], &[140, 140, 140]);
        assert_eq!(&f.pixel(76, 200)[..3], &[0, 0, 0]);
    }

    #[test]
    fn writer_library_switch_drawer_and_copy() {
        let (_d, dir, a) = lib_setup(2);
        let mut g = library(dir.path(), &a);
        assert!(!g.drawer_open());
        let fx = g.update(&[down(510, 170), up(510, 170)], 0.1, &a);
        assert!(g.drawer_open());
        assert!(fx.contains(&wsnd(lib::sfx::SWITCH)));
        let rows = lib::DRAWER.rows;
        let row0 = (rows.0 + 40, rows.1 + 5);
        // o documento já está na pasta de textos: "Já está lá!"
        lib_drag(&mut g, 0, row0, &a);
        assert_eq!(g.alert.as_ref().unwrap().kind, Modal::Notice);
        g.update(&[down(5, 470), up(5, 470)], 0.1, &a);
        assert!(g.alert.is_none());
        // um exemplo copiado para a pasta de textos vira documento da criança
        let other = tempfile::tempdir().unwrap();
        let sample = other.path().join("texto-20250101-000000.txt");
        std::fs::write(&sample, "Do Max").unwrap();
        g.items[1].path = sample;
        g.items[1].sample = true;
        let fx = lib_drag(&mut g, 1, row0, &a);
        assert!(g.alert.is_none(), "{fx:?}");
        assert!(dir.path().join("texto-20250101-000000.txt").exists());
        assert_eq!(g.items().len(), 3);
        // a segunda unidade (exemplos) é só de leitura
        let row1 = (rows.0 + 40, rows.1 + rows.4 + 5);
        lib_drag(&mut g, 0, row1, &a);
        assert_eq!(g.alert.as_ref().unwrap().kind, Modal::Notice);
        g.update(&[down(5, 470), up(5, 470)], 0.1, &a);
        // desligar fecha a gaveta
        g.update(&[down(510, 200), up(510, 200)], 0.1, &a);
        assert!(!g.drawer_open());
    }

    #[test]
    fn writer_library_trash_asks_and_deletes() {
        let (_d, dir, a) = lib_setup(5);
        let mut g = library(dir.path(), &a);
        let path = g.wall_path(0).unwrap().to_path_buf();
        let fx = lib_drag(&mut g, 0, (540, 400), &a);
        // no original, só o "EPA!" do Rex (sem o som da lixeira)
        assert!(fx.contains(&wsnd(lib::sfx::ALERT)), "{fx:?}");
        let al = g.alert.as_ref().unwrap();
        assert_eq!(al.kind, Modal::Trash(0, Some(0)));
        assert_eq!(al.balloon.options().len(), 3);
        assert_eq!((al.balloon.x, al.balloon.y), (192, 156));
        pick_option(&mut g, 0, &a);
        assert!(!path.exists());
        assert_eq!(g.items().len(), 4);
        // a moldura recebe o que sobrou fora da parede
        assert_eq!(g.wall(), [Some(3), Some(0), Some(1), Some(2)]);
    }

    /// Plaqueta de um exemplo: "Você não pode mudar esse título" (`0805[30]`).
    #[test]
    fn writer_library_sample_plate_refuses() {
        let (_d, dir, a) = lib_setup(1);
        let mut g = library(dir.path(), &a);
        g.items[0].sample = true;
        let (x, y) = lib::FRAMES[0];
        let fx = g.update(&[down(x + 50, y + 113), up(x + 50, y + 113)], 0.1, &a);
        assert!(fx.contains(&wsnd(lib::sfx::PICK)));
        assert_eq!(g.alert.as_ref().unwrap().kind, Modal::Notice);
        assert!(g.drag.is_none());
    }

    #[test]
    fn writer_library_lamp_and_welcome_choices() {
        let (_d, dir, a) = lib_setup(0);
        let mut g = library(dir.path(), &a);
        let fx = g.update(&[down(520, 60), up(520, 60)], 0.1, &a);
        assert!(fx.contains(&wsnd(lib::sfx::LAMP)));
        assert_eq!(g.alert.as_ref().unwrap().kind, Modal::Quit);
        assert_eq!(
            (
                g.alert.as_ref().unwrap().balloon.x,
                g.alert.as_ref().unwrap().balloon.y
            ),
            (208, 179)
        );
        let fx = pick_option(&mut g, 1, &a);
        assert!(!fx.contains(&Effect::Quit));
        // "Outra coisa..." → quatro opções; "Examinar outras unidades" → [13]
        let mut fx = Vec::new();
        g.answer(Ask::Welcome, 4, &mut fx, &a);
        let (kind, b) = g.balloon.as_ref().unwrap();
        assert_eq!((*kind, b.options.len()), (Ask::Welcome2, 4));
        g.answer(Ask::Welcome2, 2, &mut fx, &a);
        let (kind, b) = g.balloon.as_ref().unwrap();
        assert_eq!((*kind, (b.x, b.y)), (Ask::Tip, (288, 111)));
        g.answer(Ask::Welcome2, 3, &mut fx, &a);
        assert_eq!(g.balloon.as_ref().unwrap().0, Ask::Welcome);
    }

    /// Com os dados reais: a parede de um usuário novo tem os exemplos do
    /// Max; grava as telas em `$ATELIE_DUMP` para comparar com o original.
    #[test]
    fn writer_library_with_real_data() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let mut g = GalleryRoom::with_spec(&WRITER_LIBRARY, None, Some(dir.path().into()), &a);
        let names: Vec<String> = g
            .wall()
            .iter()
            .map(|w| {
                let p = &g.items()[w.unwrap()].path;
                p.file_name().unwrap().to_string_lossy().to_uppercase()
            })
            .collect();
        assert_eq!(names, lib::NEW_USER_WALL);
        assert_eq!(
            g.items()[g.wall()[0].unwrap()].name,
            "Feliz Dia dos Professores!"
        );
        let shot = |g: &GalleryRoom, name: &str| {
            let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
            g.draw(&mut f, &a);
            crate::scenes::testutil::dump(&f, name);
        };
        // o McZee de fraque, como na captura `building_81_lib_base`
        g.character = Character::pick_in(Program::Writer, &[0x7deb], (-18, 129), &mut g.rng);
        shot(&g, "library_welcome");
        g.balloon = None;
        shot(&g, "library_base");
        g.update(&[down(510, 170), up(510, 170)], 0.1, &a);
        shot(&g, "library_switch");
        g.update(&[down(5, 470), up(5, 470)], 0.1, &a);
        g.update(&[down(510, 200), up(510, 200)], 0.1, &a);
        let (x, y) = lib_center(0);
        g.update(&[down(x, y), up(x, y)], 0.1, &a);
        shot(&g, "library_frame");
        g.balloon = None;
        g.update(&[down(590, 140), up(590, 140)], 0.1, &a);
        shot(&g, "library_printer");
        g.balloon = None;
        lib_drag(&mut g, 0, (590, 140), &a);
        shot(&g, "library_print");
        g.print = None;
        lib_drag(&mut g, 0, (540, 400), &a);
        shot(&g, "library_trash");
        g.update(&[down(5, 470), up(5, 470)], 0.1, &a);
        g.update(&[down(520, 60), up(520, 60)], 0.1, &a);
        shot(&g, "library_quit");
    }
}
