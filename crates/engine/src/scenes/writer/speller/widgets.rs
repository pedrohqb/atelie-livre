//! Peças dos painéis da abelha, medidas no original (`office_77`..`93`):
//! moldura laranja, células com borda preta e sombra, campos de texto em
//! Pablo Graphos 16 e listas em Pablo 8 "negrito".

use crate::Frame;
use crate::assets::Assets;
use crate::scene::Key;
use crate::ui::Rect;

pub const BLACK: [u8; 3] = [0, 0, 0];
pub const WHITE: [u8; 3] = [255, 255, 255];
pub const GRAY: [u8; 3] = [140, 140, 140];
pub const LIGHT: [u8; 3] = [190, 190, 190];
pub const DARK: [u8; 3] = [100, 100, 100];
pub const ORANGE: [u8; 3] = [255, 140, 50];
pub const BROWN: [u8; 3] = [185, 101, 36];
/// Fundo do texto selecionado num campo.
pub const HILITE: [u8; 3] = [48, 150, 250];

/// Barra de cima e faixa escurecidas enquanto um painel está aberto: os
/// pixels com `x + y` par ficam cinza 100.
pub const DIM: Rect = (4, 4, 632, 98);

pub fn dim(f: &mut Frame, (x0, y0, w, h): Rect) {
    for y in y0..y0 + h {
        let mut x = x0 + (x0 + y) % 2;
        while x < x0 + w {
            f.put(x, y, DARK);
            x += 2;
        }
    }
}

/// Fundo do painel, que empurra a página `depth` pixels para baixo: as
/// duas linhas pretas de cima da página, o laranja com sombra à esquerda e
/// embaixo, e as duas linhas pretas de novo, `depth` abaixo.
pub fn draw_backdrop(f: &mut Frame, depth: i32) {
    let bottom = 105 + depth;
    f.rect(5, 106, 630, 2, BLACK);
    f.rect(5, 108, 630, bottom - 108 + 1, ORANGE);
    f.rect(5, 109, 1, bottom - 108, BROWN);
    f.rect(5, bottom, 630, 1, BROWN);
    f.rect(5, bottom + 1, 630, 2, BLACK);
}

/// Célula com borda preta de 1 pixel e sombra escura em cima e à direita.
pub fn draw_cell(f: &mut Frame, (x, y, w, h): Rect, fill: [u8; 3]) {
    f.rect(x - 1, y - 1, w + 2, 1, BROWN);
    f.rect(x + w, y - 1, 1, h + 2, BROWN);
    f.rect(x, y, w, h, BLACK);
    f.rect(x + 1, y + 1, w - 2, h - 2, fill);
}

/// Pablo Graphos 16 dos campos (ou a de 8 / a 8×8 de reserva).
pub fn field_text(f: &mut Frame, a: &Assets, x: i32, y: i32, s: &str, rgb: [u8; 3]) -> i32 {
    match a.font(16).or_else(|| a.font(8)) {
        Some(fo) => f.text_font(&fo, x, y, s, rgb),
        None => {
            f.text(x, y + 6, s, rgb);
            8 * s.chars().count() as i32
        }
    }
}

pub fn field_width(a: &Assets, s: &str) -> i32 {
    match a.font(16).or_else(|| a.font(8)) {
        Some(fo) => fo.width(s) as i32,
        None => 8 * s.chars().count() as i32,
    }
}

/// Texto das listas: Pablo 8 desenhada duas vezes (o "negrito" do original).
pub fn list_text(f: &mut Frame, a: &Assets, x: i32, y: i32, s: &str, rgb: [u8; 3]) {
    match a.font(8) {
        Some(fo) => {
            f.text_font(&fo, x, y, s, rgb);
            f.text_font(&fo, x + 1, y, s, rgb);
        }
        None => f.text(x, y + 2, s, rgb),
    }
}

/// Altura das letras dos campos (Pablo 16 = 21 pixels).
const FIELD_LINE: i32 = 21;

/// Campo de texto de uma linha. O texto pode estar todo selecionado (fundo
/// azul): digitar troca tudo, como no original ao abrir o painel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Field {
    pub text: String,
    /// Posição do cursor, em caracteres.
    pub caret: usize,
    pub all_selected: bool,
}

impl Field {
    pub fn with(text: &str) -> Field {
        Field {
            text: text.to_string(),
            caret: text.chars().count(),
            all_selected: !text.is_empty(),
        }
    }

    fn byte(&self, k: usize) -> usize {
        self.text
            .char_indices()
            .nth(k)
            .map_or(self.text.len(), |(b, _)| b)
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    fn take_selection(&mut self) -> bool {
        if std::mem::take(&mut self.all_selected) {
            self.text.clear();
            self.caret = 0;
            true
        } else {
            false
        }
    }

    pub fn type_text(&mut self, s: &str) {
        self.take_selection();
        let clean: String = s.chars().filter(|c| !c.is_control()).collect();
        let b = self.byte(self.caret);
        self.text.insert_str(b, &clean);
        self.caret += clean.chars().count();
    }

    /// Teclas de edição; `true` = mudou o texto.
    pub fn key(&mut self, key: Key) -> bool {
        match key {
            Key::Backspace | Key::Delete if self.take_selection() => true,
            Key::Backspace if self.caret > 0 => {
                let b = self.byte(self.caret - 1);
                self.text.remove(b);
                self.caret -= 1;
                true
            }
            Key::Delete if self.caret < self.len() => {
                let b = self.byte(self.caret);
                self.text.remove(b);
                true
            }
            Key::Left => {
                self.all_selected = false;
                self.caret = self.caret.saturating_sub(1);
                false
            }
            Key::Right => {
                self.all_selected = false;
                self.caret = (self.caret + 1).min(self.len());
                false
            }
            Key::Home => {
                self.all_selected = false;
                self.caret = 0;
                false
            }
            Key::End => {
                self.all_selected = false;
                self.caret = self.len();
                false
            }
            _ => false,
        }
    }

    /// Clique no campo em `x` (tela), sendo `x0` o começo do texto.
    pub fn click(&mut self, a: &Assets, x0: i32, x: i32) {
        self.all_selected = false;
        let chars: Vec<char> = self.text.chars().collect();
        let mut best = (i32::MAX, chars.len());
        for k in 0..=chars.len() {
            let w = field_width(a, &chars[..k].iter().collect::<String>());
            let d = (x0 + w - x).abs();
            if d < best.0 {
                best = (d, k);
            }
        }
        self.caret = best.1;
    }

    /// Desenha o texto em `r` (fundo já pintado); com `focus`, o cursor ou
    /// o fundo azul da seleção.
    pub fn draw(&self, f: &mut Frame, a: &Assets, r: Rect, focus: bool) {
        let (x, y) = (r.0 + 2, r.1);
        let shown = clip(a, &self.text, r.2 - 4);
        if focus && self.all_selected {
            let w = field_width(a, &shown);
            f.rect(x, y, w + 1, FIELD_LINE, HILITE);
            field_text(f, a, x, y, &shown, WHITE);
            return;
        }
        field_text(f, a, x, y, &shown, BLACK);
        if focus {
            let before: String = self.text.chars().take(self.caret).collect();
            let cx = (x + field_width(a, &before) + 1).min(r.0 + r.2 - 2);
            f.rect(cx, y, 1, FIELD_LINE, BLACK);
        }
    }
}

/// O texto que cabe em `max` pixels (o resto fica de fora).
fn clip(a: &Assets, s: &str, max: i32) -> String {
    let mut out = String::new();
    for c in s.chars() {
        out.push(c);
        if field_width(a, &out) > max {
            out.pop();
            break;
        }
    }
    out
}

/// Lista de escolhas com linhas de 20 pixels separadas por 1 pixel preto
/// (a MAIS ESCOLHAS do Corrigir e as suspensas do Trocar e do Embaralhar).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct List {
    pub items: Vec<String>,
    /// Primeira linha visível.
    pub top: usize,
    pub chosen: Option<usize>,
    /// Itens que não são escolhas ("(Sem sugestões)").
    pub inert: bool,
}

pub const ROW: i32 = 21;

impl List {
    pub fn new(items: Vec<String>) -> List {
        List {
            items,
            ..List::default()
        }
    }

    /// Linha `k` visível (`0..rows`) com o canto em `(x, y)` → item.
    pub fn item_at(
        &self,
        rows: usize,
        (x, y, w): (i32, i32, i32),
        px: i32,
        py: i32,
    ) -> Option<usize> {
        if self.inert || px < x || px >= x + w || py < y {
            return None;
        }
        let k = ((py - y) / ROW) as usize;
        (k < rows && (py - y) % ROW < ROW - 1)
            .then_some(self.top + k)
            .filter(|&i| i < self.items.len())
    }

    /// Rola `d` linhas, sem passar das pontas; `true` se mexeu.
    pub fn scroll(&mut self, rows: usize, d: i32) -> bool {
        let max = self.items.len().saturating_sub(rows);
        let top = (self.top as i32 + d).clamp(0, max as i32) as usize;
        std::mem::replace(&mut self.top, top) != top
    }

    /// Linhas a partir de `(x, y)` (a primeira linha branca), `w` de largura;
    /// a escolhida em preto com letras brancas, cobrindo as linhas pretas
    /// de cima e de baixo.
    pub fn draw(&self, f: &mut Frame, a: &Assets, rows: usize, (x, y, w): (i32, i32, i32)) {
        for k in 0..rows {
            let ry = y + k as i32 * ROW;
            f.rect(x, ry, w, ROW - 1, WHITE);
            f.rect(x, ry + ROW - 1, w, 1, BLACK);
            let i = self.top + k;
            let Some(s) = self.items.get(i) else { continue };
            let chosen = self.chosen == Some(i);
            if chosen {
                f.rect(x, ry - 1, w, ROW + 1, BLACK);
            }
            list_text(f, a, x + 3, ry + 3, s, if chosen { WHITE } else { BLACK });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campo_edita_e_troca_a_selecao() {
        let mut c = Field::with("pexe");
        assert!(c.all_selected);
        c.type_text("pe");
        assert_eq!((c.text.as_str(), c.caret), ("pe", 2));
        c.type_text("ixe");
        assert_eq!(c.text, "peixe");
        c.key(Key::Left);
        c.key(Key::Left);
        assert!(c.key(Key::Backspace));
        assert_eq!(c.text, "pexe");
        assert!(c.key(Key::Delete));
        assert_eq!(c.text, "pee");
        c.key(Key::Home);
        c.type_text("á");
        assert_eq!((c.text.as_str(), c.caret), ("ápee", 1));
        let mut c = Field::with("abc");
        assert!(c.key(Key::Backspace));
        assert_eq!(c.text, "");
        assert!(!c.key(Key::Backspace));
    }

    #[test]
    fn lista_rola_e_acha_a_linha() {
        let mut l = List::new((0..8).map(|k| k.to_string()).collect());
        let geo = (322, 128, 309);
        assert_eq!(l.item_at(5, geo, 330, 128), Some(0));
        assert_eq!(l.item_at(5, geo, 330, 148), None); // linha preta
        assert_eq!(l.item_at(5, geo, 330, 149), Some(1));
        assert_eq!(l.item_at(5, geo, 330, 128 + 5 * 21), None);
        assert!(l.scroll(5, 10));
        assert_eq!(l.top, 3);
        assert!(!l.scroll(5, 1));
        assert_eq!(l.item_at(5, geo, 330, 128), Some(3));
        assert!(l.scroll(5, -1));
        assert_eq!(l.top, 2);
        l.inert = true;
        assert_eq!(l.item_at(5, geo, 330, 128), None);
    }
}
