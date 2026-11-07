//! Menu do McZee na tela de pintura: clicar no McZee do nicho abre uma coluna
//! branca de 8 botões à direita do quadro (medida no original, ver
//! `docs/telas/pintura-menu-mczee.md`).

use crate::Frame;
use crate::assets::Assets;
use crate::text_balloon::blit_pic;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];

/// Coluna do menu: cobre a borda laranja de cima e vai até a de baixo.
pub const COLUMN: (i32, i32, i32, i32) = (538, 105, 48, 351);
/// Altura de cada botão (os separadores ficam na última linha de cada um).
const CELL_H: i32 = 44;

/// Itens do menu, de cima para baixo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// Cara do McZee dormindo: "OK, vou embora se você quiser mesmo…".
    Dismiss,
    /// Estrela NOVO: começa um quadro novo (passa pelo "Epa!").
    New,
    /// Pasta vermelha: abre a pasta de trabalhos.
    Open,
    /// Cofrinho SALVAR: dá nome ao quadro e guarda.
    Save,
    /// Cartola: a Maggie oferece os truques de desenho.
    Tricks,
    /// Varinha Mágica.
    Wand,
    /// Cara de sol: uma piada.
    Joke,
    /// Álbum: a história do McZee.
    Story,
}

/// Figuras dos botões (`ARTIST.PIC`), na ordem de [`Item`].
const ITEMS: [(Item, u16); 8] = [
    (Item::Dismiss, 0x041b),
    (Item::New, 0x15e7),
    (Item::Open, 0x044f),
    (Item::Save, 0x15e3),
    (Item::Tricks, 0x15a3),
    (Item::Wand, 0x0457),
    (Item::Joke, 0x03f3),
    (Item::Story, 0x0453),
];

/// Retângulo do botão `k`.
pub fn cell(k: usize) -> (i32, i32, i32, i32) {
    (COLUMN.0, COLUMN.1 + CELL_H * k as i32, COLUMN.2, CELL_H)
}

/// Item sob `(x, y)`, se o ponto cai num botão.
pub fn hit(x: i32, y: i32) -> Option<Item> {
    ITEMS.iter().enumerate().find_map(|(k, (item, _))| {
        let r = cell(k);
        (x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3).then_some(*item)
    })
}

/// O ponto está na coluna do menu (botão ou não)?
pub fn contains(x: i32, y: i32) -> bool {
    x >= COLUMN.0 && y >= COLUMN.1 && x < COLUMN.0 + COLUMN.2 && y < COLUMN.1 + COLUMN.3
}

/// Desenha a coluna aberta: fundo branco com borda preta à esquerda, um
/// separador preto entre os botões e cada figura centrada no seu botão.
pub fn draw(f: &mut Frame, a: &Assets) {
    let (x, y, w, h) = COLUMN;
    f.rect(x, y, w, h, WHITE);
    f.rect(x - 1, y, 1, h, BLACK);
    for (k, (_, id)) in ITEMS.iter().enumerate() {
        let (cx, cy, cw, ch) = cell(k);
        if k + 1 < ITEMS.len() {
            f.rect(cx, cy + ch - 1, cw, 1, BLACK);
        }
        let img = a.image(&crate::assets::ImageRef::Pic(
            crate::assets::Program::Artist,
            *id,
        ));
        let ix = cx + (cw - img.width as i32) / 2;
        let iy = cy + (ch - 1 - img.height as i32) / 2;
        blit_pic(f, a, *id, ix, iy, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_top_to_bottom() {
        assert_eq!(hit(560, 110), Some(Item::Dismiss));
        assert_eq!(hit(560, 160), Some(Item::New));
        assert_eq!(hit(560, 210), Some(Item::Open));
        assert_eq!(hit(560, 250), Some(Item::Save));
        assert_eq!(hit(560, 300), Some(Item::Tricks));
        assert_eq!(hit(560, 340), Some(Item::Wand));
        assert_eq!(hit(560, 380), Some(Item::Joke));
        assert_eq!(hit(560, 440), Some(Item::Story));
        assert_eq!(hit(530, 300), None);
        assert_eq!(hit(560, 100), None);
        assert!(contains(540, 455));
        assert!(!contains(590, 300));
    }
}
