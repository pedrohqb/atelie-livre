//! Especificação das salas do prédio, por programa: o que muda entre o
//! saguão do Fine Artist e o do Creative Writer (fundo, elementos, áreas
//! clicáveis, balões, personagem, Bolsa Mágica, animações…) é **dado**; a
//! mecânica (`room.rs`) é uma só. Cada sala é um [`RoomSpec`]; o que é comum
//! a todas as salas de um programa (bloco `WHAT`, sons, figuras da bolsa,
//! cursor da varinha, balão de saída) é um [`RoomKit`].
//!
//! - Fine Artist: `rooms_data` (salas e `ARTIST_KIT`).
//! - Creative Writer: `writer_rooms` (`WRITER_KIT` e o despacho por andar)
//!   e um arquivo por sala (`writer_lobby`, `writer_office_room`,
//!   `writer_floors`, `writer_attic`).
//!
//! O contrato está em `docs/arquitetura.md` ("Creative Writer: specs por
//! programa").

use super::entry_spec::QuitSpec;
use super::room::Floor;
use super::room_objects::{BagLayout, BagPics, Surprises};
use super::room_talk::{Helper, Pick};
use super::rooms_data::{self, Element};
use crate::assets::{Assets, CursorRef, Program, SoundRef};
use crate::ui::Rect;
use std::path::{Path, PathBuf};

/// O que um clique numa área faz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Fecha o balão.
    Close,
    /// Andar do quadro do saguão (nome ou figura): fala do quadro
    /// (`RoomSounds::board`) e troca de andar (o sótão inclusive).
    Floor(Floor),
    /// Galeria (Fine Artist) ou Biblioteca (Creative Writer), com o som dado.
    Gallery(u16),
    /// Luminária ou "Tchau": o "EPA!" (Sair / Continuar).
    Quit,
    /// Pergunta "Botão ou adesivo?" num segundo balão (Fine Artist).
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
    /// Cavalete do Ateliê / prancheta do Escritório: dica; clique duplo
    /// abre um trabalho novo.
    Easel,
    /// Quadro da parede (último trabalho): dica; clique duplo o abre.
    Wall,
    /// Plaqueta do quadro da parede: troca o nome do último trabalho.
    WallLabel,
    /// Objeto com balão de escolha (bichos dos Projetos, estátuas, máquina).
    Pick(Pick),
    /// Moldura dos Truques: troca a obra-prima.
    Painting,
    /// Dica do bloco `WHAT` (opções do balão de chegada).
    Tip(usize),
    /// Só um som (mostrador do elevador no saguão do Writer); fecha o
    /// balão aberto.
    Sound(u16),
}

impl Action {
    /// Objetos que respondem com balão: clicar num deles com outro balão
    /// aberto troca direto para o balão dele.
    pub fn talks(self) -> bool {
        matches!(
            self,
            Action::Easel | Action::Wall | Action::Pick(_) | Action::Painting | Action::Quit
        )
    }
}

/// Área clicável; numa sala vale a primeira que contém o ponto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotspot {
    pub rect: Rect,
    pub action: Action,
}

/// Atalho para as tabelas: `hot((x, y, w, h), Action::Quit)`.
pub const fn hot(rect: Rect, action: Action) -> Hotspot {
    Hotspot { rect, action }
}

/// Texto reserva de um balão (sem os dados originais, ou se o registro do
/// `WHAT` não tiver as opções esperadas) e a posição em que ele aparece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fallback {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub text: &'static str,
    pub options: &'static [&'static str],
}

/// Palavra do glossário dentro de uma opção do balão de chegada (a
/// "tridimensional" dos Truques): o clique nela abre o registro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glossary {
    pub option: usize,
    pub word: &'static str,
    pub record: usize,
}

/// Balão de chegada à sala.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrival {
    /// Registro do bloco `WHAT` do kit.
    pub record: usize,
    pub fallback: Fallback,
    /// Ação de cada opção (a última vale também para as que sobrarem).
    pub actions: &'static [Action],
    pub glossary: Option<Glossary>,
}

/// Personagem parado à esquerda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterSpec {
    /// Fantasias ou poses (imagem colorida; 1 bit e máscara são as duas
    /// seguintes no `.PIC`), uma sorteada a cada chegada.
    pub choices: &'static [u16],
    pub pos: (i32, i32),
    pub helper: Helper,
    /// Som do clique no personagem.
    pub sound: u16,
    /// Onde abre o menu "Você quer me ver menos?".
    pub menu: (i32, i32),
    /// Poses com posição própria (o Max do Writer: em pé `7e0f` e
    /// encostado `7e03`); quando há alguma, valem no lugar de `choices` e
    /// `pos`.
    pub poses: &'static [(u16, (i32, i32))],
}

/// Dica (balão sem opções) de um registro: posição, largura, texto reserva
/// e ligações ("Tecla de atalho", "Descendo?").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tip {
    pub record: usize,
    pub pos: (i32, i32),
    pub w: i32,
    pub text: &'static str,
    pub links: &'static [(usize, &'static str)],
}

/// Moldura que troca de figura a cada clique (obras-primas dos Truques).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Painting {
    /// Imagem colorida e máscara.
    pub choices: &'static [(u16, u16)],
    pub pos: (i32, i32),
}

/// Quadro da parede com o último trabalho e o cavalete/prancheta com a
/// plaqueta "Quadro Novo" (Ateliê do Fine Artist).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Studio {
    /// Dicas do clique simples no cavalete e no quadro da parede.
    pub easel_tip: usize,
    pub wall_tip: usize,
    /// Canto do último trabalho reduzido (1/`wall_scale`) no quadro.
    pub wall_pic: (i32, i32),
    pub wall_scale: u32,
    /// Centro (x) e topo do texto das plaquetas da parede e do cavalete.
    pub wall_label: (i32, i32),
    pub easel_label: (i32, i32),
    /// Plaqueta da parede (campo de troca de nome).
    pub label_rect: Rect,
    /// Onde abre o balão "Se você quiser, pode digitar outro nome!".
    pub name_balloon: (i32, i32),
    /// Texto da plaqueta do cavalete/prancheta ("Quadro Novo",
    /// "Documento Novo").
    pub new_label: &'static str,
    /// O quadro da parede mostra um documento do programa (Writer), e não o
    /// `ultimo.png` da pasta de desenhos.
    pub doc: Option<&'static DocWall>,
}

/// Onde a sala procura o último trabalho do usuário: pasta dos usuários do
/// port, apelido e pasta `MSKIDS` dos dados originais (exemplos).
#[derive(Debug, Clone, Copy)]
pub struct WorkPlace<'a> {
    pub users: Option<&'a Path>,
    pub user: &'a str,
    pub data: &'a Path,
}

/// Último trabalho mostrado no quadro da parede, quando ele vem de um
/// documento (Writer): miniatura, título e se é do próprio usuário.
#[derive(Debug, Clone, PartialEq)]
pub struct LastWork {
    pub image: Option<crate::Image>,
    pub title: String,
    /// É do usuário (pode trocar o título); senão é de outro (o exemplo do
    /// Max): a dica e a plaqueta mudam.
    pub own: bool,
    pub path: PathBuf,
}

/// Lê o último trabalho do usuário.
pub type LastWorkLoader = fn(&WorkPlace<'_>, &Assets) -> Option<LastWork>;
/// Troca o título do último trabalho (só os do usuário).
pub type LastWorkRename = fn(&WorkPlace<'_>, &LastWork, &str) -> Result<(), String>;

/// Quadro da parede com um documento (escritório do Writer).
#[derive(Debug, Clone, Copy)]
pub struct DocWall {
    pub load: LastWorkLoader,
    pub rename: LastWorkRename,
    /// Caixa da página na moldura (x, y, w, h): a miniatura é reduzida para
    /// caber na altura `h`, centrada em x e encostada no topo.
    pub thumb_box: Rect,
    /// Dica do quadro quando o documento é de outra pessoa ("Esse
    /// documento é do Max").
    pub other_tip: usize,
    /// "EPA! Você não pode mudar esse título": bloco e registro do `WHAT`,
    /// posição e texto reserva.
    pub not_yours: (u16, usize, (i32, i32), &'static str),
}

// funções comparadas pelo endereço (os specs são `const`, uma cópia só)
impl PartialEq for DocWall {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::fn_addr_eq(self.load, o.load)
            && std::ptr::fn_addr_eq(self.rename, o.rename)
            && (self.thumb_box, self.other_tip, self.not_yours)
                == (o.thumb_box, o.other_tip, o.not_yours)
    }
}
impl Eq for DocWall {}

/// Luminária "Tchau" desenhada pela sala: a figura de sempre e, com o
/// "EPA!" aberto, a da fumaça (Writer; o Fine Artist não tem a figura).
/// Imagem colorida e posição; a máscara é a do `.PIC`, duas imagens adiante.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lamp {
    pub normal: (u16, (i32, i32)),
    pub smoke: (u16, (i32, i32)),
}

/// Objeto enfeitado pela Varinha Mágica (bichos da parede dos Projetos do
/// Writer): área, roteiros `ATAG` (um sorteado) e o ponto que se soma às
/// origens gravadas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WandProp {
    pub rect: Rect,
    pub scripts: &'static [u16],
    pub anchor: (i32, i32),
}

/// Uma sala do prédio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomSpec {
    /// O que é comum às salas do programa (e de que `.PIC`/`.SND`/`WHAT`
    /// são os ids desta sala).
    pub kit: &'static RoomKit,
    /// Fundo (centralizado; 0 = tela preta).
    pub background: u16,
    /// Figuras fixas por cima do fundo.
    pub elements: &'static [Element],
    /// Áreas clicáveis, a primeira que contém o ponto vale.
    pub hotspots: &'static [Hotspot],
    pub arrival: Option<Arrival>,
    pub character: Option<CharacterSpec>,
    /// Canto do "EPA!" da luminária.
    pub quit_pos: (i32, i32),
    /// Onde ficam a bolsa e o que sai dela.
    pub bag: BagLayout,
    /// A bolsa já está aberta ao chegar.
    pub bag_open: bool,
    /// Surpresas da Varinha Mágica no mastro e no buraco do rato.
    pub surprises: Surprises,
    /// Roteiros `ATAG` das animações ambientes.
    pub gags: &'static [u16],
    /// Dicas desta sala (procuradas antes das do kit).
    pub tips: &'static [Tip],
    pub painting: Option<Painting>,
    pub studio: Option<Studio>,
    /// Som ao entrar no elevador por esta sala (portas ou skate).
    pub elevator_sound: u16,
    /// Som ao chegar à sala (por qualquer caminho).
    pub arrival_sound: Option<u16>,
    /// Som ao chegar à sala pelo elevador.
    pub elevator_arrival_sound: Option<u16>,
    /// Luminária com a figura da fumaça (senão ela é um dos `elements`).
    pub lamp: Option<Lamp>,
    /// Onde abrem os balões dos objetos (cavalete/prancheta, quadro da
    /// parede): no Writer saem da boca do McZee; `None` = na posição da dica.
    pub object_balloon: Option<(i32, i32)>,
    /// Objetos que a Varinha Mágica enfeita.
    pub wand_props: &'static [WandProp],
}

impl RoomSpec {
    /// Ação da opção `k` do balão de chegada.
    pub fn arrival_action(&self, k: usize) -> Action {
        self.arrival
            .and_then(|a| a.actions.get(k).or(a.actions.last()).copied())
            .unwrap_or(Action::Close)
    }

    /// Dica do registro `k`: a da sala, senão a do kit.
    pub fn tip(&self, k: usize) -> Option<&'static Tip> {
        self.tips
            .iter()
            .chain(self.kit.tips)
            .find(|t| t.record == k)
    }
}

/// Sons das salas (ids do `.SND` do programa do kit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomSounds {
    /// Clique num objeto que responde com balão.
    pub object: u16,
    /// Abre o balão de escolha do objeto.
    pub pick: u16,
    /// Opção escolhida (bolinha).
    pub choice: u16,
    /// Dicas "Ignorar isto" e "Descendo?".
    pub tip: u16,
    /// Glossário e "Tecla de atalho".
    pub glossary: u16,
    /// Luminária ou "Tchau": abre o "EPA!".
    pub quit: [u16; 2],
    /// Clique fora de um balão modal.
    pub refuse: u16,
    /// Moldura dos Truques.
    pub painting: u16,
    /// Buraco do rato sem a varinha.
    pub mouse_hole: u16,
    /// Pasta aberta pelo balão de escolha.
    pub portfolio: u16,
    /// Quadro de andares do saguão.
    pub board: u16,
    /// Clique no McZee do elevador.
    pub mczee: u16,
    pub bag_open: u16,
    pub bag_close: u16,
    pub suitcase: u16,
    pub book: [u16; 2],
    pub wand_take: u16,
    pub wand_hole: u16,
    pub pole_up: u16,
    pub pole_down: u16,
    pub skate: u16,
    /// Chegada do elevador (depois das portas ou do skate).
    pub elevator_arrive: u16,
}

/// O que é comum às salas de um programa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomKit {
    pub program: Program,
    /// Bloco `WHAT` com os balões das salas.
    pub what: u16,
    pub snd: RoomSounds,
    /// Dicas comuns às salas (atalhos, mastro…).
    pub tips: &'static [Tip],
    /// Registros cujas ligações tocam o som do glossário (os outros tocam
    /// o da dica).
    pub glossary_links: &'static [usize],
    /// Figura de "Tecla de atalho" nos registros e a da lâmpada, que vira
    /// ligação (as duas abrem o registro apontado).
    pub hotkey_pic: Option<(u16, u16)>,
    /// "Você quer me ver menos?" e "Que bom que você me chamou de novo".
    pub help_off: usize,
    pub help_on: usize,
    /// Balão da Varinha Mágica nos andares (sem opções), se houver.
    pub wand: Option<Tip>,
    /// "EPA! O que você quer fazer?" (Sair / Continuar).
    pub quit: &'static QuitSpec,
    pub bag: &'static BagPics,
    pub wand_cursor: CursorRef,
    /// Onde abre o menu do McZee dentro do elevador.
    pub elevator_helper: (i32, i32),
    /// Estado guardado por usuário (bolsa aberta, ajuda desligada), se o
    /// programa o guarda.
    pub persist: Option<Persist>,
    /// Ctrl+O abre a pasta em qualquer sala.
    pub ctrl_o: bool,
}

/// Estado das salas que vale entre salas e entre sessões (Writer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RoomState {
    pub bag_open: bool,
    pub help_off: bool,
}

/// Leitura e gravação do [`RoomState`] na pasta dos usuários.
#[derive(Debug, Clone, Copy)]
pub struct Persist {
    pub load: fn(&Path, &str) -> RoomState,
    pub save: fn(&Path, &str, RoomState),
}

impl PartialEq for Persist {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::fn_addr_eq(self.load, o.load) && std::ptr::fn_addr_eq(self.save, o.save)
    }
}
impl Eq for Persist {}

impl RoomKit {
    pub fn sound(&self, id: u16) -> SoundRef {
        SoundRef::Snd(self.program, id)
    }

    /// Som da dica aberta por uma ligação (lâmpada ou tecla de atalho).
    pub fn link_sound(&self, k: usize) -> u16 {
        if self.glossary_links.contains(&k) {
            self.snd.glossary
        } else {
            self.snd.tip
        }
    }
}

/// As salas de um programa, por andar.
pub type Rooms = fn(Floor) -> &'static RoomSpec;

/// Salas do prédio do programa `p`.
pub fn rooms(p: Program) -> Rooms {
    match p {
        Program::Artist => rooms_data::room,
        Program::Writer => super::writer_rooms::room,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_floor_has_a_room_in_both_programs() {
        for p in [Program::Artist, Program::Writer] {
            for f in [
                Floor::Lobby,
                Floor::Studio,
                Floor::Projects,
                Floor::Tricks,
                Floor::Attic,
            ] {
                let r = rooms(p)(f);
                assert!(r.hotspots.iter().all(|h| h.rect.2 > 0 && h.rect.3 > 0));
                if let Some(a) = r.arrival {
                    assert!(!a.actions.is_empty(), "{p:?} {f:?}");
                }
            }
        }
    }

    #[test]
    fn arrival_action_repeats_the_last_one() {
        let r = rooms(Program::Artist)(Floor::Tricks);
        assert_eq!(r.arrival_action(9), r.arrival_action(2));
        assert_eq!(
            rooms(Program::Artist)(Floor::Attic).arrival_action(0),
            Action::Close
        );
    }
}
