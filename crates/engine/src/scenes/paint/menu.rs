//! Menu do McZee: clicar no McZee do nicho abre uma coluna branca de 8
//! botões à direita do quadro (medida no original, ver
//! `docs/telas/pintura-menu-mczee.md`). O Creative Writer tem o mesmo menu,
//! um pouco mais à esquerda, com o Caderno de Idéias no lugar da cartola
//! ([`WRITER`], `docs/telas/creative-writer.md`).

use crate::Frame;
use crate::assets::{Assets, Program};
use crate::text_balloon::blit_pic_in;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];

type Rect = (i32, i32, i32, i32);

/// Coluna do menu da pintura: cobre a borda laranja de cima e vai até a de baixo.
pub const COLUMN: Rect = ARTIST.column;
/// Altura de cada botão (os separadores ficam na última linha de cada um).
const CELL_H: i32 = 44;

/// Itens do menu, de cima para baixo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// Cara do McZee dormindo: "OK, vou embora se você quiser mesmo…".
    Dismiss,
    /// Estrela NOVO: começa um quadro novo (passa pelo "Epa!").
    New,
    /// Pasta vermelha (maleta no Writer): abre a pasta de trabalhos.
    Open,
    /// Cofrinho SALVAR: dá nome ao quadro e guarda.
    Save,
    /// Cartola: a Maggie oferece os truques de desenho (só no Fine Artist).
    Tricks,
    /// Caderno de Idéias (só no Creative Writer).
    Notebook,
    /// Varinha Mágica.
    Wand,
    /// Cara de sol: uma piada.
    Joke,
    /// Álbum (livro verde no Writer): a história do McZee.
    Story,
}

/// Um menu do McZee: onde fica a coluna e as figuras dos botões.
pub struct Menu {
    pub column: Rect,
    /// `.PIC` das figuras.
    pub program: Program,
    /// Botões de cima para baixo, com a figura de cada um.
    pub items: [(Item, u16); 8],
}

/// Menu da tela de pintura (`ARTIST.PIC`).
pub const ARTIST: Menu = Menu {
    column: (538, 105, 48, 351),
    program: Program::Artist,
    items: [
        (Item::Dismiss, 0x041b),
        (Item::New, 0x15e7),
        (Item::Open, 0x044f),
        (Item::Save, 0x15e3),
        (Item::Tricks, 0x15a3),
        (Item::Wand, 0x0457),
        (Item::Joke, 0x03f3),
        (Item::Story, 0x0453),
    ],
};

/// Menu do Escritório do Creative Writer (`WRITER.PIC`, captura
/// `office_115_mczee.png`): coluna x 533..580, figuras centradas.
pub const WRITER: Menu = Menu {
    column: (533, 103, 48, 352),
    program: Program::Writer,
    items: [
        (Item::Dismiss, 0x041b),
        (Item::New, 0x084d),
        (Item::Open, 0x044f),
        (Item::Save, 0x08af),
        (Item::Notebook, 0x0c01),
        (Item::Wand, 0x0457),
        (Item::Joke, 0x03f3),
        (Item::Story, 0x0453),
    ],
};

impl Menu {
    /// Retângulo do botão `k`.
    pub fn cell(&self, k: usize) -> Rect {
        let c = self.column;
        (c.0, c.1 + CELL_H * k as i32, c.2, CELL_H)
    }

    /// Item sob `(x, y)`, se o ponto cai num botão.
    pub fn hit(&self, x: i32, y: i32) -> Option<Item> {
        self.items.iter().enumerate().find_map(|(k, (item, _))| {
            let r = self.cell(k);
            (x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3).then_some(*item)
        })
    }

    /// O ponto está na coluna do menu (botão ou não)?
    pub fn contains(&self, x: i32, y: i32) -> bool {
        let c = self.column;
        x >= c.0 && y >= c.1 && x < c.0 + c.2 && y < c.1 + c.3
    }

    /// Desenha a coluna aberta: fundo branco com borda preta à esquerda, um
    /// separador preto entre os botões e cada figura centrada no seu botão.
    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let (x, y, w, h) = self.column;
        f.rect(x, y, w, h, WHITE);
        f.rect(x - 1, y, 1, h, BLACK);
        for (k, (_, id)) in self.items.iter().enumerate() {
            let (cx, cy, cw, ch) = self.cell(k);
            if k + 1 < self.items.len() {
                f.rect(cx, cy + ch - 1, cw, 1, BLACK);
            }
            let img = a.image(&crate::assets::ImageRef::Pic(self.program, *id));
            let ix = cx + (cw - img.width as i32) / 2;
            let iy = cy + (ch - 1 - img.height as i32) / 2;
            blit_pic_in(f, a, self.program, *id, ix, iy, true);
        }
    }
}

/// Retângulo do botão `k` do menu da pintura.
pub fn cell(k: usize) -> Rect {
    ARTIST.cell(k)
}

/// Item do menu da pintura sob `(x, y)`.
pub fn hit(x: i32, y: i32) -> Option<Item> {
    ARTIST.hit(x, y)
}

/// O ponto está na coluna do menu da pintura?
pub fn contains(x: i32, y: i32) -> bool {
    ARTIST.contains(x, y)
}

/// Desenha o menu da pintura.
pub fn draw(f: &mut Frame, a: &Assets) {
    ARTIST.draw(f, a);
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

    #[test]
    fn writer_menu_has_the_notebook_in_the_fifth_button() {
        // figuras localizadas na captura do original, centradas nas casas
        assert_eq!(WRITER.hit(556, 120), Some(Item::Dismiss));
        assert_eq!(WRITER.hit(556, 300), Some(Item::Notebook));
        assert_eq!(WRITER.hit(556, 430), Some(Item::Story));
        assert_eq!(WRITER.hit(586, 300), None);
        assert!(WRITER.contains(533, 454));
        assert!(!WRITER.items.iter().any(|(i, _)| *i == Item::Tricks));
        assert!(!ARTIST.items.iter().any(|(i, _)| *i == Item::Notebook));
        assert_eq!(WRITER.cell(4).1, 103 + 4 * 44);
    }
}
