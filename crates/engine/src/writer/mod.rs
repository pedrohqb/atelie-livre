//! Núcleo do Escritório do Creative Writer.
//!
//! Modelo do documento, usado pela cena `scenes/writer` (ver
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
//! - [`undo`]: a pilha de 20 ações do Ovo;
//! - [`convert`]: documento ↔ `.MAX` (`formats::max`), com o que o modelo
//!   não guarda em [`convert::MaxExtra`];
//! - [`store`]: os `.MAX` na pasta do usuário (nome a partir do título) e
//!   "Salvar como texto".

pub mod convert;
pub mod dict;
pub mod doc;
pub mod fonts;
pub mod ideas;
pub mod layout;
pub mod object;
pub mod selection;
pub mod store;
pub mod undo;
