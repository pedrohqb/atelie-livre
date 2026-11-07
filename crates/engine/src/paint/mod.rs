//! Núcleo da pintura: quadro, tinta, formas, traço, preenchimento, histórico e arquivos.

pub mod brush;
pub mod bucket_fx;
pub mod canvas;
pub mod geom;
pub mod objects;
pub mod print;
pub mod shape;
pub mod shaped;
pub mod stamp;
pub mod store;
pub mod text;
pub mod ttf;

pub use canvas::{
    BLACK, CANVAS_H, CANVAS_W, COLORS, Canvas, History, Ink, ORIGIN, WHITE, line_points,
};
pub use objects::{Object, Objects, Source};
pub use shape::{Cell, Shape, page_shapes};
pub use shaped::{Shadow, ShapedWords, WordShape};
pub use text::{
    Align, BalloonShape, WordsLayout, WordsStyle, balloon_pixels, text_pixels, text_size,
    words_layout,
};
