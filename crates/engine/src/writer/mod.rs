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
//! - [`borders`]: borda e fundo de página (rolos BORDA e FUNDO).
//! - [`rebus_apply`]: Coloca-figuras (Maçã): figuras depois das palavras.
//! - [`cipher`]: Agente Secreto 009 (texto embaralhado na tela, senha).
//! - [`splot`]: palavras com forma (`SPLE`): desenho, montagem e escolhas.

pub mod assistant;
pub mod assistant_banner;
pub mod assistant_card;
pub mod assistant_news;
pub mod borders;
pub mod cipher;
pub mod convert;
pub mod dict;
pub mod doc;
pub mod fold_figure;
pub mod fonts;
pub mod ideas;
pub mod layout;
pub mod object;
pub mod prefs;
pub mod rebus_apply;
pub mod selection;
pub mod splot;
pub mod store;
pub mod undo;
