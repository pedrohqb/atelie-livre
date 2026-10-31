//! Núcleo da pintura: quadro, tinta, formas, traço, preenchimento, histórico e arquivos.

pub mod canvas;
pub mod geom;
pub mod shape;
pub mod stamp;
pub mod store;
pub mod text;

pub use canvas::{
    BLACK, CANVAS_H, CANVAS_W, COLORS, Canvas, History, Ink, ORIGIN, WHITE, line_points,
};
pub use shape::{Cell, Shape, effect_shape, page_shapes};
pub use text::{balloon_pixels, text_pixels, text_size};
