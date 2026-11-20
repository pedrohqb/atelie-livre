//! "Mix de estilos" da Palavra (página 2 da faixa): os 7 estilos prontos,
//! com os valores exatos que o original grava no `.MAX` (medidos aplicando
//! cada um e lendo o documento, `office-complemento.md` §3).
//!
//! | Mix | caractere | parágrafo |
//! |---|---|---|
//! | Maluco | Brilho, 36 pt (Enorme), violeta `#932ca3` | — |
//! | Anotação | sublinhado, vermelho `#fa0039`, marca-texto amarelo `#ffee4c` | — |
//! | Título | negrito, 24 pt (Grande) | centralizado |
//! | Era uma vez | — | capitular de 3 linhas, espaço depois |
//! | Poesia | — | esquerda e direita, recuo de todas as linhas |
//! | Relatório | — | recuo da 1ª linha, espaço depois |
//! | Super-simples | tira tudo | tira tudo |
//!
//! Regras observadas: a parte de caractere vale só para as **palavras
//! inteiras tocadas** pelo pincel; a de parágrafo, para o parágrafo inteiro
//! tocado; o Super-simples limpa **todos os parágrafos tocados** (inclusive
//! as palavras não tocadas). O "Maluco" é fixo (não é sorteado), e não há
//! efeito especial "Maluco": ele grava o Brilho.

use super::doc::{
    Align, CSCH, CharProps, DROP_CAP_LINES, Document, Effect, Indent, ParaProps, ParaStyle, Range,
    Size,
};

/// Os 7 estilos prontos, na ordem da lista (`STR#` 0x807).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mix {
    Crazy,
    Note,
    Title,
    OnceUpon,
    Poetry,
    Report,
    SuperSimple,
}

/// Violeta do Maluco (`CSCH[9]`).
pub const CRAZY_COLOR: formats::Rgb = CSCH[9];
/// Vermelho da Anotação (`CSCH[5]`).
pub const NOTE_COLOR: formats::Rgb = CSCH[5];
/// Marca-texto amarelo da Anotação (`CSCH[1]`).
pub const NOTE_HIGHLIGHT: formats::Rgb = CSCH[1];

impl Mix {
    pub const ALL: [Mix; 7] = [
        Mix::Crazy,
        Mix::Note,
        Mix::Title,
        Mix::OnceUpon,
        Mix::Poetry,
        Mix::Report,
        Mix::SuperSimple,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Mix::Crazy => "Maluco",
            Mix::Note => "Anotação",
            Mix::Title => "Título",
            Mix::OnceUpon => "Era uma vez",
            Mix::Poetry => "Poesia",
            Mix::Report => "Relatório",
            Mix::SuperSimple => "Super-simples",
        }
    }

    /// Tem parte de caractere (vale para as palavras tocadas).
    pub fn has_char(self) -> bool {
        matches!(self, Mix::Crazy | Mix::Note | Mix::Title)
    }

    /// Tem parte de parágrafo (vale para o parágrafo inteiro tocado; basta
    /// clicar).
    pub fn has_para(self) -> bool {
        !matches!(self, Mix::Crazy | Mix::Note)
    }

    /// A parte de caractere aplicada a `p` (o resto de `p` fica).
    pub fn apply_char(self, p: &mut CharProps) {
        match self {
            Mix::Crazy => {
                p.effect = Some(Effect::Sparkle);
                p.size = Size::Huge;
                p.color = Some(CRAZY_COLOR);
            }
            Mix::Note => {
                p.underline = true;
                p.color = Some(NOTE_COLOR);
                p.highlight = Some(NOTE_HIGHLIGHT);
            }
            Mix::Title => {
                p.bold = true;
                p.size = Size::Large;
            }
            Mix::SuperSimple => *p = CharProps::default(),
            Mix::OnceUpon | Mix::Poetry | Mix::Report => {}
        }
    }

    /// A parte de parágrafo aplicada a `p`.
    pub fn apply_para(self, p: &mut ParaProps) {
        match self {
            Mix::Title => p.align = Align::Center,
            Mix::OnceUpon => {
                p.set_para_style(ParaStyle::DropCap);
                p.drop_cap = Some(DROP_CAP_LINES);
                p.space_after = true;
            }
            Mix::Poetry => {
                p.align = Align::Justify;
                p.indent = Indent::All;
            }
            Mix::Report => {
                p.indent = Indent::FirstLine;
                p.space_after = true;
            }
            Mix::SuperSimple => *p = ParaProps::default(),
            Mix::Crazy | Mix::Note => {}
        }
    }

    /// Aplica o mix: a parte de caractere em `words` (as palavras tocadas,
    /// `None` = nenhuma, como num clique sem arrasto) e a de parágrafo nos
    /// parágrafos de `paras`. O Super-simples limpa os parágrafos de
    /// `paras` inteiros (caracteres, `\r` e parágrafo).
    pub fn apply(self, doc: &mut Document, words: Option<Range>, paras: Range) {
        if self == Mix::SuperSimple {
            let (s, e) = (doc.clamp(paras.start), doc.clamp(paras.end));
            for par in &mut doc.paras[s.para..=e.para] {
                for it in &mut par.items {
                    if let super::doc::Item::Char(_, p) = it {
                        *p = CharProps::default();
                    }
                }
                par.mark = CharProps::default();
                par.props = ParaProps::default();
            }
            return;
        }
        if self.has_char()
            && let Some(r) = words
        {
            let (s, e) = (doc.clamp(r.start), doc.clamp(r.end));
            for k in s.para..=e.para {
                let par = &mut doc.paras[k];
                let a = if k == s.para { s.index } else { 0 };
                let b = if k == e.para { e.index } else { par.len() };
                for it in &mut par.items[a..b] {
                    if let super::doc::Item::Char(_, p) = it {
                        self.apply_char(p);
                    }
                }
                if k < e.para || (a == 0 && b == par.len()) {
                    self.apply_char(&mut par.mark);
                }
            }
        }
        if self.has_para() {
            let (s, e) = (doc.clamp(paras.start), doc.clamp(paras.end));
            for par in &mut doc.paras[s.para..=e.para] {
                self.apply_para(&mut par.props);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::Pos;
    use formats::Rgb;

    fn rgb(c: u32) -> Rgb {
        Rgb {
            r: (c >> 16) as u8,
            g: (c >> 8) as u8,
            b: c as u8,
        }
    }

    fn doc() -> Document {
        Document::from_text("Era uma vez\num gato azul", CharProps::default())
    }

    fn props(d: &Document, para: usize, k: usize) -> CharProps {
        *d.paras[para].items[k].props().unwrap()
    }

    #[test]
    fn crazy_is_sparkle_huge_and_violet() {
        let mut d = doc();
        let words = Range::new(Pos::new(0, 0), Pos::new(0, 3));
        Mix::Crazy.apply(&mut d, Some(words), words);
        let p = props(&d, 0, 0);
        assert_eq!(p.effect, Some(Effect::Sparkle));
        assert_eq!(p.effect.unwrap().code(), 5);
        assert_eq!(p.size.pt(), 36);
        assert_eq!(p.color, Some(rgb(0x932ca3)));
        // só as palavras tocadas
        assert!(props(&d, 0, 4).is_default());
        assert!(d.paras[0].props.is_default());
    }

    #[test]
    fn note_and_title() {
        let mut d = doc();
        let w = Range::new(Pos::new(1, 0), Pos::new(1, 2));
        Mix::Note.apply(&mut d, Some(w), w);
        let p = props(&d, 1, 0);
        assert!(p.underline);
        assert_eq!(p.color, Some(rgb(0xfa0039)));
        assert_eq!(p.highlight, Some(rgb(0xffee4c)));
        assert!(props(&d, 1, 3).is_default());
        let mut d = doc();
        Mix::Title.apply(&mut d, Some(w), w);
        let p = props(&d, 1, 0);
        assert!(p.bold);
        assert_eq!(p.size.pt(), 24);
        assert_eq!(d.paras[1].props.align, Align::Center);
        assert!(props(&d, 1, 5).is_default());
        // num clique (sem palavras) só o parágrafo
        let mut d = doc();
        Mix::Title.apply(&mut d, None, w);
        assert!(props(&d, 1, 0).is_default());
        assert_eq!(d.paras[1].props.align, Align::Center);
    }

    #[test]
    fn paragraph_mixes() {
        let at = Range::at(Pos::new(0, 1));
        let mut d = doc();
        Mix::OnceUpon.apply(&mut d, None, at);
        let p = d.paras[0].props;
        assert_eq!((p.drop_cap, p.space_after), (Some(3), true));
        assert!(d.paras[1].props.is_default());
        let mut d = doc();
        Mix::Poetry.apply(&mut d, None, at);
        let p = d.paras[0].props;
        assert_eq!((p.align, p.indent), (Align::Justify, Indent::All));
        let mut d = doc();
        Mix::Report.apply(&mut d, None, at);
        let p = d.paras[0].props;
        assert_eq!((p.indent, p.space_after), (Indent::FirstLine, true));
        assert!(!p.line_spacing);
        assert!(props(&d, 0, 0).is_default());
    }

    #[test]
    fn super_simple_clears_whole_touched_paragraphs() {
        let mut d = doc();
        let all = d.all();
        Mix::Crazy.apply(&mut d, Some(all), all);
        Mix::Poetry.apply(&mut d, None, all);
        // toca só a 1ª palavra do 1º parágrafo
        let w = Range::new(Pos::new(0, 0), Pos::new(0, 3));
        Mix::SuperSimple.apply(&mut d, Some(w), w);
        assert!(
            d.paras[0]
                .items
                .iter()
                .all(|i| i.props().unwrap().is_default())
        );
        assert!(d.paras[0].mark.is_default() && d.paras[0].props.is_default());
        // o 2º parágrafo não foi tocado
        assert!(!props(&d, 1, 0).is_default());
        assert_eq!(d.paras[1].props.indent, Indent::All);
    }

    #[test]
    fn list_order_and_names() {
        let names: Vec<_> = Mix::ALL.iter().map(|m| m.name()).collect();
        assert_eq!(
            names,
            [
                "Maluco",
                "Anotação",
                "Título",
                "Era uma vez",
                "Poesia",
                "Relatório",
                "Super-simples"
            ]
        );
    }
}
