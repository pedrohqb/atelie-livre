//! Botão ou adesivo: forma recortada no quadro e folha de impressão.

/// Estado do botão/adesivo (ainda sem comportamento).
#[derive(Debug, Clone)]
pub struct Button {
    /// `true` = adesivo retangular; `false` = botão redondo.
    pub sticker: bool,
}

impl Button {
    pub fn new(sticker: bool) -> Button {
        Button { sticker }
    }
}
