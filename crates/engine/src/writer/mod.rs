//! Núcleo do Escritório do Creative Writer: o documento (parágrafos, trechos
//! com estilo e adesivos), a diagramação em linhas e páginas, e os arquivos.
//! Nada aqui depende dos recursos originais: a fonte é opcional (sem ela,
//! vale a 8×8 reserva), e os adesivos são só referências (rolo e número).

pub mod doc;
pub mod layout;
pub mod store;

pub use doc::{Align, Cursor, Document, History, Item, Paragraph, Size, Sticker, Style};
pub use layout::{FontSet, Layout, Line};
