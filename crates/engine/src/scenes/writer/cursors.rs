//! Cursores do Escritório: os `CURSOR` 0001–0023 do `WRITER.EXE` (folha
//! em `cw/shots/formats_cursores_bitmaps.png`; hotspot no nome do PNG
//! extraído). Os nomes vêm do desenho de cada um e dos relatórios; os
//! marcados "provável" ainda não foram vistos em uso no original.
//!
//! O [`Pointer`] guarda o cursor atual e só emite `Effect::SetCursor` quando
//! ele muda; cada ferramenta escolhe o seu em
//! [`ToolStrip::cursor`](super::tools::ToolStrip::cursor).

use super::tools::cursor;
use crate::assets::CursorRef;
use crate::scene::{DEFAULT_CURSOR, Effect};

/// Seta.
pub const ARROW: u16 = 0x0001;
/// Barra de ponto de inserção com a mão (cursor de texto na página).
pub const TEXT: u16 = 0x0002;
/// Documento (provável: Visor de páginas / páginas).
pub const DOCUMENT: u16 = 0x0003;
/// Pincel das opções da Palavra (`office-palavra.md` §1).
pub const BRUSH: u16 = 0x0004;
/// Bomba e documento (provável: Jogar tudo fora).
pub const ALL_GONE: u16 = 0x0005;
/// Tesoura (Recortar).
pub const CUT: u16 = 0x0006;
/// Câmera (Copiar).
pub const COPY: u16 = 0x0007;
/// Cola (Colar).
pub const PASTE: u16 = 0x0008;
/// Nota musical (associar um som).
pub const SOUND: u16 = 0x0009;
/// Canto (provável: bordas e margens).
pub const CORNER: u16 = 0x000a;
/// Bolinha (provável: Flutua-texto).
pub const DOT: u16 = 0x000b;
/// Setas para cima e para baixo (Dimensionar vertical).
pub const SIZE_V: u16 = 0x000c;
/// Setas para os lados (Dimensionar lateral).
pub const SIZE_H: u16 = 0x000d;
/// Quatro setas (Dimensionar tudo).
pub const SIZE_ALL: u16 = 0x000e;
/// Varinha Mágica.
pub const WAND: u16 = 0x000f;
/// Estrela.
pub const STAR: u16 = 0x0010;
/// Despertador (espera).
pub const WAIT: u16 = 0x0011;
/// Mão apontando.
pub const POINT: u16 = 0x0015;
/// Mão aberta (pode pegar: Mover-modificar sobre um objeto).
pub const HAND_OPEN: u16 = 0x0016;
/// Mão pegando.
pub const HAND_GRAB: u16 = 0x0017;
/// Mão fechada (arrastando).
pub const HAND_FIST: u16 = 0x0018;
/// Documento com figura (provável: adesivo a colar).
pub const PICTURE: u16 = 0x0019;
/// Rolo de pintura (provável: Aplicador / fundo).
pub const ROLLER: u16 = 0x001d;
/// Dedo apontando.
pub const FINGER: u16 = 0x0023;

/// Cursor atual do mouse; emite `SetCursor` só quando muda.
pub struct Pointer {
    current: CursorRef,
}

impl Default for Pointer {
    fn default() -> Self {
        // `KidCommon` já pôs a mão padrão no primeiro quadro
        Pointer {
            current: DEFAULT_CURSOR,
        }
    }
}

impl Pointer {
    pub fn current(&self) -> CursorRef {
        self.current
    }

    /// Passa a usar `want`; devolve o efeito se mudou.
    pub fn set(&mut self, want: CursorRef) -> Option<Effect> {
        (want != self.current).then(|| {
            self.current = want;
            Effect::SetCursor(want)
        })
    }
}

/// Cursor do `WRITER.EXE` de id `id`.
pub fn writer(id: u16) -> CursorRef {
    cursor(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Program;

    #[test]
    fn pointer_only_emits_on_change() {
        let mut p = Pointer::default();
        assert_eq!(p.set(DEFAULT_CURSOR), None);
        let text = writer(TEXT);
        assert_eq!(text, CursorRef(Program::Writer, 2));
        assert_eq!(p.set(text), Some(Effect::SetCursor(text)));
        assert_eq!(p.set(text), None);
        assert_eq!(p.current(), text);
    }
}
