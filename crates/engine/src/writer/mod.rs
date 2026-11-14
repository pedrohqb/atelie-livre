//! Núcleo do Escritório do Creative Writer.
//!
//! Modelo rico (biblioteca, ainda não ligada à cena; ver
//! `docs/telas/creative-writer-modelo.md`):
//!
//! - [`doc`]: o documento — parágrafos ([`doc::ParaProps`]), caracteres
//!   ([`doc::CharProps`]), figuras na linha ([`doc::Item::Rebus`]),
//!   propriedades da folha ([`doc::DopProps`]) e tabela de fontes, com os
//!   mesmos nomes e semânticas do formato `.MAX`;
//! - [`object`]: objetos flutuantes (adesivo, desenho, palavra com forma)
//!   com posição, tamanho, Flutua-texto, som e borda;
//! - [`fonts`]: as 6 famílias do Seleciona-letras resolvidas para as
//!   originais instaladas ou substitutas livres, com as métricas de linha
//!   medidas no original; implementa [`layout::Measure`] e desenha trechos;
//! - [`layout`]: diagramação em folhas, margens e colunas, contornando os
//!   objetos, com quebra de página e número de página, sobre uma medição
//!   de texto abstrata ([`layout::Measure`]);
//! - [`selection`]: seleção, movimentos com Shift e área de transferência
//!   interna;
//! - [`undo`]: a pilha de 20 ações do Ovo.
//!
//! O modelo provisório que a cena `scenes/writer` usa hoje fica em
//! [`legacy`] e é reexportado aqui com os nomes antigos até a integração.

pub mod dict;
pub mod doc;
pub mod fonts;
pub mod ideas;
pub mod layout;
pub mod legacy;
pub mod object;
pub mod selection;
pub mod store;
pub mod undo;

// Nomes antigos, usados pela cena atual (`crate::writer::Document` etc.).
pub use legacy::doc::{Align, Cursor, Document, History, Item, Paragraph, Size, Sticker, Style};
pub use legacy::layout::{FontSet, Layout, Line};
