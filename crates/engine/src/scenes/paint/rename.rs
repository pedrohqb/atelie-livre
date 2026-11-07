//! Nome do quadro na barra de baixo: clicar no rótulo (ou SALVAR no menu do
//! McZee, ou "Salvar este trabalho" no "Epa!") vira o rótulo num campo de
//! texto com o nome selecionado; Return guarda (medido no original, ver
//! `docs/telas/pintura-menu-mczee.md`).

use crate::Frame;
use crate::scene::Key;
use formats::fnt::Font;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
/// Fundo da seleção (azul do original).
const SELECTION: [u8; 3] = [48, 150, 250];
/// Rótulo do quadro na barra de baixo (área clicável).
pub const LABEL: (i32, i32, i32, i32) = (152, 461, 335, 15);
/// Nome de um quadro ainda sem nome.
pub const DEFAULT_NAME: &str = "Quadro Novo";
/// Tamanho máximo do nome, em caracteres.
const MAX_LEN: usize = 40;

/// O que fazer depois de guardar com o nome digitado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum After {
    /// Só guardar e seguir pintando.
    Stay,
    /// Guardar e sair da tela (porta).
    Exit,
    /// Guardar e começar um quadro em branco (NOVO).
    New,
}

/// Campo de nome em edição.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rename {
    pub text: String,
    /// O texto inteiro está selecionado: digitar substitui tudo.
    pub selected: bool,
    pub after: After,
}

impl Rename {
    pub fn new(current: &str, after: After) -> Rename {
        Rename {
            text: current.to_string(),
            selected: true,
            after,
        }
    }

    /// Texto digitado: substitui a seleção ou acrescenta no fim.
    pub fn type_text(&mut self, t: &str) {
        if self.selected {
            self.text.clear();
            self.selected = false;
        }
        for c in t.chars() {
            if self.text.chars().count() >= MAX_LEN {
                break;
            }
            if !c.is_control() && !matches!(c, '/' | '\\') {
                self.text.push(c);
            }
        }
    }

    /// Tecla: `true` quando Return/Enter conclui a edição. Esc não cancela
    /// (como no original).
    pub fn key(&mut self, key: Key) -> bool {
        match key {
            Key::Enter => return true,
            Key::Backspace | Key::Delete => {
                if self.selected {
                    self.text.clear();
                    self.selected = false;
                } else {
                    self.text.pop();
                }
            }
            _ => {}
        }
        false
    }

    /// Nome final: o digitado, ou o padrão se ficou vazio.
    pub fn name(&self) -> String {
        let t = self.text.trim();
        if t.is_empty() {
            DEFAULT_NAME.to_string()
        } else {
            t.to_string()
        }
    }

    /// Campo na barra de baixo: texto à esquerda, seleção azul com texto
    /// branco ou cursor no fim.
    pub fn draw(&self, f: &mut Frame, font: Option<&Font>) {
        let (x, y) = (LABEL.0 + 6, LABEL.1 + 1);
        let w = match font {
            Some(fo) => fo.width(&self.text) as i32,
            None => 8 * self.text.chars().count() as i32,
        };
        if self.selected {
            f.rect(x - 2, LABEL.1, w + 4, 14, SELECTION);
        }
        let color = if self.selected { WHITE } else { BLACK };
        match font {
            Some(fo) => {
                f.text_font(fo, x, y, &self.text, color);
            }
            None => f.text(x, y + 2, &self.text, color),
        }
        if !self.selected {
            f.rect(x + w + 1, LABEL.1 + 1, 1, 12, BLACK);
        }
    }
}

/// Nome do arquivo para um nome de quadro (ver `store::name_stem`).
pub fn file_stem(name: &str) -> Option<String> {
    crate::paint::store::name_stem(name)
}

/// O ponto está no rótulo do quadro?
pub fn label_hit(x: i32, y: i32) -> bool {
    x >= LABEL.0 && y >= LABEL.1 && x < LABEL.0 + LABEL.2 && y < LABEL.1 + LABEL.3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_replaces_selection_and_enter_finishes() {
        let mut r = Rename::new(DEFAULT_NAME, After::Stay);
        assert!(r.selected);
        r.type_text("Mon");
        assert_eq!(r.text, "Mon");
        assert!(!r.selected);
        r.type_text("tanha");
        assert!(!r.key(Key::Escape));
        assert!(r.key(Key::Enter));
        assert_eq!(r.name(), "Montanha");
    }

    #[test]
    fn backspace_on_selection_clears_and_empty_name_is_default() {
        let mut r = Rename::new("Casa", After::Exit);
        r.key(Key::Backspace);
        assert_eq!(r.text, "");
        assert_eq!(r.name(), DEFAULT_NAME);
        r.type_text("ab/c\n");
        assert_eq!(r.text, "abc");
        r.key(Key::Backspace);
        assert_eq!(r.text, "ab");
    }

    #[test]
    fn file_stems_are_safe() {
        assert_eq!(file_stem(DEFAULT_NAME), None);
        assert_eq!(file_stem("  "), None);
        assert_eq!(file_stem("..."), None);
        assert_eq!(file_stem("Minha casa"), Some("Minha casa".into()));
        assert_eq!(file_stem("a/b:c"), Some("a_b_c".into()));
        assert_eq!(file_stem("Ação!"), Some("Ação!".into()));
    }

    #[test]
    fn draws_without_font() {
        let mut f = Frame::new(640, 480);
        let r = Rename::new("Oi", After::Stay);
        r.draw(&mut f, None);
        assert_eq!(
            f.pixel(LABEL.0 as u32 + 5, LABEL.1 as u32 + 3)[..3],
            SELECTION
        );
        assert!(label_hit(300, 468));
        assert!(!label_hit(300, 450));
    }
}
