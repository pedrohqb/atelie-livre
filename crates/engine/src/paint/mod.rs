//! Núcleo da pintura: quadro, tinta, formas, traço, preenchimento, histórico e arquivos.

pub mod canvas;
pub mod shape;

pub use canvas::{BLACK, CANVAS_H, CANVAS_W, Canvas, History, Ink, ORIGIN, WHITE, line_points};
pub use shape::{Cell, Shape, page_shapes};
