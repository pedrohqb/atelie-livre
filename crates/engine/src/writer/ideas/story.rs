//! Ponte da Oficina de Idéias para o editor (I10): o conteúdo do documento
//! novo que "Escrever uma história com essa sentença/figura" abre
//! (`ideas.md` §6), no modelo rico [`crate::writer::doc`].
//!
//! - **Máquina**: o 1º parágrafo é a sentença, na fonte padrão (Lucida
//!   Sans), sem recuo, com cada janela que mostrava figura como um
//!   caractere-figura ([`Item::Rebus`]); o cursor fica no fim.
//! - **Quadro**: a figura vira um adesivo flutuante (`PICT 0`, com a imagem
//!   em [`IdeaStory::pictures`]) em tamanho natural, centrado na largura da
//!   folha e encostado na margem de cima; o texto fica vazio.
//!
//! O documento salvo leva `FLOR` = 4 e `WIZA` = `"plot"` ou `"wpic"`
//! ([`IdeaSource::wiza`]), que a pasta usa para filtrar "Trabalhar com uma
//! história que você já começou" (I12); a barra de status do editor mostra
//! "Oficina de Idéias" e o McZee o balão `0faa[13]`/`[16]`.
//!
//! **O que falta ligar** (Escritório integrado, ramo `cw-office-core`): a
//! cena do editor, montada para `SceneId::Cw(CwScene::IdeaStory(_))`, deve
//! pegar o conteúdo com [`super::session::take_story`] e abrir o
//! documento com o cursor em [`IdeaStory::cursor`], as figuras de
//! [`IdeaStory::pictures`] como os `PICT` do documento, a barra de status
//! com [`FLOOR_LABEL`], o balão [`IdeaSource::help`] e, ao salvar, gravar
//! [`FLOOR`] e [`IdeaSource::wiza`]; a porta SAÍDA volta à Oficina
//! ([`IdeaSource::back`]). Até lá `run.rs` abre o editor atual sem o
//! conteúdo.

use super::machine::Piece;
use crate::Image;
use crate::scene::{CwScene, SceneId};
use crate::scenes::room::Floor;
use crate::writer::doc::{CharProps, Document, Item, Pos};
use crate::writer::object::{FloatObject, ObjRef, ObjectKind};

/// Nome do andar na barra de status do editor (`STR#0808[10]`).
pub const FLOOR_LABEL: &str = "Oficina de Idéias";
/// `FLOR` dos documentos começados na Oficina.
pub const FLOOR: u16 = 4;

/// De onde veio a história.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdeaSource {
    /// Máquina de Idéias (`WIZA` `"plot"`).
    Machine,
    /// Quadro de Figuras (`WIZA` `"wpic"`).
    Board,
}

impl IdeaSource {
    /// Etiqueta `WIZA` (e primeiros 4 bytes do `ADD`) do documento salvo.
    pub fn wiza(self) -> [u8; 4] {
        match self {
            IdeaSource::Machine => *b"plot",
            IdeaSource::Board => *b"wpic",
        }
    }

    pub fn from_wiza(tag: &[u8]) -> Option<IdeaSource> {
        match tag {
            b"plot" => Some(IdeaSource::Machine),
            b"wpic" => Some(IdeaSource::Board),
            _ => None,
        }
    }

    /// Tipo do documento na pasta ("Conteúdo:", `STR#03ac[9..10]`).
    pub fn kind_label(self) -> &'static str {
        match self {
            IdeaSource::Machine => "Máquina de Idéias",
            IdeaSource::Board => "Quadro de Figuras",
        }
    }

    /// Balão do McZee ao abrir o editor: `0faa[13]` (Máquina) ou `[16]`.
    pub fn help(self) -> (u16, usize) {
        match self {
            IdeaSource::Machine => (0x0faa, 13),
            IdeaSource::Board => (0x0faa, 16),
        }
    }

    /// Para onde a porta SAÍDA do editor volta: a Oficina de Idéias.
    pub fn back(self) -> SceneId {
        SceneId::Cw(CwScene::Building(Floor::Tricks))
    }
}

/// Figura que vai junto com o documento (o `PICT` citado pelo objeto).
#[derive(Debug, Clone)]
pub struct StoryPicture {
    pub object: ObjRef,
    pub title: String,
    pub image: Image,
}

/// Documento novo começado na Oficina.
#[derive(Debug, Clone)]
pub struct IdeaStory {
    pub source: IdeaSource,
    pub doc: Document,
    /// Onde o cursor fica (fim da sentença; começo do texto no Quadro).
    pub cursor: Pos,
    pub pictures: Vec<StoryPicture>,
}

impl IdeaStory {
    /// "Escrever uma história com essa sentença".
    pub fn from_sentence(pieces: &[Piece]) -> IdeaStory {
        let props = CharProps::default();
        let mut doc = Document::new();
        let para = &mut doc.paras[0];
        for p in pieces {
            match p {
                Piece::Text(t) => para.items.extend(
                    t.chars()
                        .filter(|c| !c.is_control())
                        .map(|c| Item::Char(c, props)),
                ),
                Piece::Rebus(c) if *c > 0 => para.items.push(Item::Rebus(*c)),
                Piece::Rebus(_) => {}
            }
        }
        let cursor = doc.end();
        IdeaStory {
            source: IdeaSource::Machine,
            doc,
            cursor,
            pictures: Vec::new(),
        }
    }

    /// "Escrever uma história com essa figura": a figura em tamanho natural
    /// (1 pt = 1 pixel), centrada na largura da folha, na margem de cima.
    pub fn from_picture(title: &str, image: Image) -> IdeaStory {
        let mut doc = Document::new();
        let object = ObjRef::pict(0);
        let page_w = i32::from(doc.dop.page_width);
        let (w, h) = (
            image.width.clamp(1, u16::MAX as u32) as u16,
            image.height.clamp(1, u16::MAX as u32) as u16,
        );
        let x = ((page_w - i32::from(w)) / 2).max(0) as i16;
        let y = doc.dop.margin_top as i16;
        doc.objects
            .push(FloatObject::new(ObjectKind::Sticker, object, 0, x, y, w, h));
        IdeaStory {
            source: IdeaSource::Board,
            doc,
            cursor: Pos::default(),
            pictures: vec![StoryPicture {
                object,
                title: title.to_string(),
                image,
            }],
        }
    }

    /// Tela do editor com este conteúdo.
    pub fn scene(&self) -> SceneId {
        SceneId::Cw(CwScene::IdeaStory(self.source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentence_becomes_the_first_paragraph_with_rebus_items() {
        let s = IdeaStory::from_sentence(&[
            Piece::Rebus(0x25),
            Piece::Text(" riu com desdém ".into()),
            Piece::Rebus(0x1a),
        ]);
        assert_eq!(s.source, IdeaSource::Machine);
        assert_eq!(s.doc.paras.len(), 1);
        let items = &s.doc.paras[0].items;
        assert_eq!(items.first(), Some(&Item::Rebus(0x25)));
        assert_eq!(items.last(), Some(&Item::Rebus(0x1a)));
        assert_eq!(s.doc.text(), " riu com desdém ");
        assert_eq!(s.cursor, s.doc.end());
        assert!(items.iter().filter_map(Item::props).all(|p| p.is_default()));
        assert_eq!(
            s.scene(),
            SceneId::Cw(CwScene::IdeaStory(IdeaSource::Machine))
        );
    }

    #[test]
    fn picture_becomes_a_centered_sticker_at_the_top() {
        let img = Image {
            width: 363,
            height: 249,
            rgba: vec![255; 363 * 249 * 4],
        };
        let s = IdeaStory::from_picture("O castelo", img);
        assert_eq!(s.source, IdeaSource::Board);
        assert!(s.doc.text().is_empty());
        assert_eq!(s.doc.objects.len(), 1);
        let o = &s.doc.objects[0];
        assert_eq!((o.width, o.height), (363, 249));
        assert_eq!(o.x, (595 - 363) / 2);
        assert_eq!(o.y, 72);
        assert_eq!(o.object, s.pictures[0].object);
        assert_eq!(s.pictures[0].title, "O castelo");
    }

    #[test]
    fn wiza_tags_round_trip() {
        for s in [IdeaSource::Machine, IdeaSource::Board] {
            assert_eq!(IdeaSource::from_wiza(&s.wiza()), Some(s));
        }
        assert_eq!(IdeaSource::from_wiza(b"xxxx"), None);
        assert_eq!(IdeaSource::Machine.kind_label(), "Máquina de Idéias");
        assert_eq!(IdeaSource::Board.help(), (0x0faa, 16));
    }
}
