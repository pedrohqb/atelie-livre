//! Close-ups da Oficina de Idéias do Creative Writer (`cw/report/ideas.md`
//! §3–5, `office-complemento.md` §6–7): a Máquina de Idéias
//! ([`machine::IdeaMachine`], `CwScene::IdeaMachine`), o Quadro de Figuras
//! ([`board::PictureBoard`], `CwScene::PictureBoard`) e o Caderno de Idéias
//! ([`notebook::NotebookView`]), uma sobreposição aberta pelos dois e,
//! mais adiante, pela Bolsa Mágica do editor. A lógica sem tela fica em
//! `crate::writer::ideas`; o Cavalete de lápis de cera em
//! `scenes::writer_easel`.
//!
//! A sala do 4º andar (Oficina de Idéias, `writer_floors::IDEAS`) é de
//! outro ramo (`cw-rooms`): os objetos dela (`Pick::IdeaMachine`,
//! `Pick::PictureBoard`) abrem estas cenas.

pub mod board;
pub mod buttons;
pub mod machine;
pub mod notebook;

pub use board::PictureBoard;
pub use machine::IdeaMachine;

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::balloon::{Balloon, draw_box};
use crate::scene::{CwScene, Effect, SceneId};
use crate::scenes::room::Floor;
use crate::scenes::room_talk::{fallback, load_in};
use crate::scenes::writer_gags::{MAX_SITTING, MAX_SITTING_POS};
use crate::scenes::writer_rooms::WRITER_KIT;
use crate::text_balloon::{Part, TextBalloon};
use crate::ui::Rect;
use std::path::{Path, PathBuf};

/// Fundo dos close-ups: a parede da sala (`6813`).
pub const BACKGROUND: u16 = 0x6813;
/// Max sentado no chão: figura, 1 bit e máscara.
pub const MAX_ART: (u16, u16) = (MAX_SITTING, MAX_SITTING + 2);
/// Área do Max sentado (clique: o balão de chegada outra vez).
pub const MAX_RECT: Rect = (0, 336, 150, 144);

/// Sons do `WRITER.SND` (`ideas.md` §10).
pub mod snd {
    /// Clique na Máquina ou no Quadro da sala.
    pub const OBJECT: u16 = 0x021c;
    /// Entrada num close-up.
    pub const CLOSE_UP: u16 = 0x02f6;
    /// A máquina, a alavanca, a aba "Máquina" do Caderno.
    pub const MACHINE: u16 = 0x021d;
    /// Botão redondo da janela; OUTRA.
    pub const ROUND: u16 = 0x02dc;
    pub const POE: u16 = 0x023b;
    /// Alerta do Rex (latido + "EPA!").
    pub const REX: u16 = 0x025f;
    pub const EPA: u16 = 0x02fe;
    /// FEITO; OK do cavalete e do Caderno.
    pub const DONE: u16 = 0x0239;
    /// Abrir o Caderno; aba Lembretes.
    pub const NOTEBOOK: u16 = 0x0209;
    /// Aba Figuras; abrir o cavalete.
    pub const PICTURES: u16 = 0x0267;
    pub const EASEL_OPEN: u16 = 0x0224;
    /// NÃO do cavalete e do Caderno.
    pub const NO: u16 = 0x02ed;
    /// Entrada no Quadro; câmera (copiar) do Caderno.
    pub const BOARD: u16 = 0x0288;
    /// Tesoura (recortar) do Caderno.
    pub const CUT: u16 = 0x0217;
    /// Abrir a lista de figuras; escolher um item.
    pub const LIST: u16 = 0x02bc;
    pub const ITEM: u16 = 0x022d;
    /// Abrir o editor.
    pub const EDITOR: u16 = 0x02fc;
    /// Volta à sala.
    pub const ROOM: u16 = 0x02c0;
    /// Opção de balão; clique num balão fora das opções.
    pub const CHOICE: u16 = 0x02ff;
    pub const BALLOON: u16 = 0x02f4;
}

pub fn wpic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Writer, id)
}

pub fn play(id: u16) -> Effect {
    Effect::PlaySound(SoundRef::Snd(Program::Writer, id))
}

/// A sala da Oficina de Idéias (volta dos close-ups).
pub fn room() -> SceneId {
    SceneId::Cw(CwScene::Building(Floor::Tricks))
}

/// Usuário e pasta das ideias (`<usuarios>/<apelido>/ideias`).
#[derive(Debug, Clone, Default)]
pub struct Owner {
    pub user: String,
    pub dir: Option<PathBuf>,
}

impl Owner {
    /// `users`: a pasta dos usuários (`None` = nada é lido nem gravado).
    pub fn new(users: Option<&Path>, user: &str) -> Owner {
        Owner {
            user: user.to_string(),
            dir: users.map(|u| crate::writer::ideas::store::ideas_dir(u, user)),
        }
    }
}

/// Parede e Max sentado, comuns aos close-ups.
pub fn draw_backdrop(f: &mut Frame, a: &Assets) {
    f.blit(&a.image(&wpic(BACKGROUND)), 0, 0);
    let (x, y) = MAX_SITTING_POS;
    f.blit_masked(&a.image(&wpic(MAX_ART.0)), &a.image(&wpic(MAX_ART.1)), x, y);
}

/// Balão do `WHAT 0bb8` (o Max falando) em `pos`, com o reserva.
pub fn max_balloon(
    a: &Assets,
    k: usize,
    pos: (i32, i32),
    user: &str,
    w: i32,
    text: &str,
    options: &[&str],
) -> Balloon {
    let fb = fallback(w, text, options, &[]);
    let mut b = load_in(&WRITER_KIT, a, k, pos, user, fb).in_program(Program::Writer);
    (b.x, b.y) = pos;
    b
}

/// Balão amarelo de um registro do `WHAT 0805` sem a arte do Rex ("O que
/// você quer fazer em seguida?", `[40]`/`[41]`).
pub fn plain_balloon(
    a: &Assets,
    k: usize,
    pos: (i32, i32),
    w: i32,
    text: &str,
    options: &[&str],
) -> Balloon {
    let records = a.dialogs(Program::Writer, 0x0805);
    match Balloon::from_block(&records, k, pos.0, pos.1, "") {
        Some(b) if b.options.len() == options.len() && !b.text.is_empty() => {
            b.in_program(Program::Writer)
        }
        _ => {
            let mut b = fallback(w, text, options, &[]);
            (b.x, b.y) = pos;
            b
        }
    }
}

/// Aviso "EPA!" do Rex (`WHAT 0805[k]`), com fundo branco.
pub fn rex_alert(
    a: &Assets,
    k: usize,
    pos: (i32, i32),
    w: i32,
    text: &str,
    options: &[&str],
) -> TextBalloon {
    let mut parts = vec![
        Part::Figure(crate::text_balloon::pic_for(0x230).unwrap_or(0)),
        Part::Text(text.to_string()),
    ];
    if !options.is_empty() {
        parts.push(Part::Options(
            options.iter().map(|s| s.to_string()).collect(),
        ));
    }
    let fb = TextBalloon::new(pos.0, pos.1, w, parts, vec![]).as_alert();
    match a.dialogs(Program::Writer, 0x0805).get(k) {
        Some(r) => {
            let b = TextBalloon::from_record(r, pos.0, pos.1, "", vec![]).as_alert();
            if b.options().len() == options.len() {
                b
            } else {
                fb
            }
        }
        None => fb,
    }
}

/// Balão de instruções ("Usando a Máquina de Idéias — clique para
/// continuar", `0bb8[47]`/`[52]`): título com a gema e a figura explicativa
/// (`WHAT` `[860:0]` = `0c3f`, `[85f:0]` = `0c3b` do `WRITER.PIC`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instructions {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub title: &'static str,
    pub pic: u16,
}

impl Instructions {
    pub fn rect(&self) -> Rect {
        (self.x, self.y, self.w, self.h)
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        draw_box(f, self.x, self.y, self.w, self.h);
        // gema roxa das dicas
        let (gx, gy) = (self.x + 38, self.y + 8);
        for dy in 0..9 {
            let half: i32 = if dy < 3 { 3 + dy } else { 8 - dy };
            for dx in -half..=half {
                let c = if dy == 0 || dx.abs() == half {
                    [0, 0, 0]
                } else {
                    [153, 51, 204]
                };
                f.put(gx + dx, gy + dy, c);
            }
        }
        let pen = crate::ui::Pen::new(a);
        let tw = self.w - 70;
        let lines = crate::balloon::wrap_opt(pen.font(), self.title, tw as u32);
        for (k, l) in lines.iter().enumerate() {
            let y = self.y + 6 + 15 * k as i32;
            pen.centered(f, self.x + self.w / 2 + 10, y, l, [0, 0, 0]);
            pen.centered(f, self.x + self.w / 2 + 11, y, l, [0, 0, 0]);
        }
        let img = a.image(&wpic(self.pic));
        let px = self.x + (self.w - img.width as i32) / 2;
        let py = self.y + self.h - img.height as i32 - 6;
        f.rect(
            px - 1,
            py - 1,
            img.width as i32 + 2,
            img.height as i32 + 2,
            [0, 0, 0],
        );
        f.blit(&img, px, py);
    }
}

/// Moldura de seleção por cima de um retângulo (o "apertado" desenhado
/// quando falta a figura e a moldura laranja do Caderno).
pub fn outline(f: &mut Frame, r: Rect, t: i32, rgb: [u8; 3]) {
    f.frame_rect(r.0, r.1, r.2, r.3, t, rgb);
}
