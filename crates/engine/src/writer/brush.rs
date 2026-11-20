//! O pincel da Palavra (`office-palavra.md` §1): cada item escolhido numa
//! lista da faixa vira um pincel, que se arrasta sobre o texto.
//!
//! - a parte de **caractere** (letras, tamanho, cor, estilo, iluminador,
//!   efeito, parte de caractere dos Mix) vale para as **palavras inteiras**
//!   tocadas pelo arrasto; um clique sem arrasto não muda nada;
//! - a parte de **parágrafo** (Posicionador, estilos de parágrafo,
//!   espaçamento, recuos, parte de parágrafo dos Mix) vale para os
//!   parágrafos tocados, e para ela basta clicar;
//! - o Aplicador passa o pincel no documento inteiro (inclusive nos `\r`).

use super::doc::{CharChange, CharProps, Document, ParaChange, ParaProps, Range};
use super::mix::Mix;

/// Um pincel carregado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Brush {
    /// Seleciona-letras: o nome da fonte (entra na tabela do documento).
    Font(&'static str),
    /// Tamanho, cor, estilo, iluminador ou efeito.
    Char(CharChange),
    /// Alinhamento, estilo de parágrafo, espaçamento ou recuo.
    Para(ParaChange),
    /// Um dos 7 Mix de estilos.
    Mix(Mix),
}

impl Brush {
    /// Tem parte de caractere (precisa de arrasto).
    pub fn has_char(self) -> bool {
        match self {
            Brush::Font(_) | Brush::Char(_) => true,
            Brush::Para(_) => false,
            Brush::Mix(m) => m.has_char() || m == Mix::SuperSimple,
        }
    }

    /// Tem parte de parágrafo (basta clicar).
    pub fn has_para(self) -> bool {
        match self {
            Brush::Para(_) => true,
            Brush::Mix(m) => m.has_para(),
            _ => false,
        }
    }

    fn char_change(self, doc: &mut Document) -> Option<CharChange> {
        match self {
            Brush::Font(name) => Some(CharChange::Font(doc.fonts.ensure(name))),
            Brush::Char(c) => Some(c),
            _ => None,
        }
    }

    /// Uma passada do pincel sobre `touched` (do ponto em que o botão foi
    /// apertado até onde está agora); `dragged` = o mouse já se moveu.
    pub fn stroke(self, doc: &mut Document, touched: Range, dragged: bool) {
        let words = dragged.then(|| doc.expand_to_words(touched));
        match self {
            Brush::Mix(m) => m.apply(doc, words, touched),
            Brush::Para(p) => doc.apply_para(touched, p),
            Brush::Font(_) | Brush::Char(_) => {
                if let Some(w) = words.filter(|w| !w.is_empty())
                    && let Some(c) = self.char_change(doc)
                {
                    doc.apply_char(w, c);
                }
            }
        }
    }

    /// Aplicador: o documento inteiro.
    pub fn apply_all(self, doc: &mut Document) {
        let all = doc.all();
        match self {
            Brush::Mix(m) => m.apply(doc, Some(all), all),
            Brush::Para(p) => doc.apply_para(all, p),
            Brush::Font(_) | Brush::Char(_) => {
                if let Some(c) = self.char_change(doc) {
                    doc.apply_char(all, c);
                }
            }
        }
    }

    /// O pincel aplicado a um estilo solto (Ctrl+Aplicador: o padrão dos
    /// documentos novos). A fonte vem à parte, pelo nome.
    pub fn apply_style(self, font: &mut String, chars: &mut CharProps, para: &mut ParaProps) {
        match self {
            Brush::Font(name) => *font = name.to_string(),
            Brush::Char(c) => c.apply(chars),
            Brush::Para(p) => p.apply(para),
            Brush::Mix(m) => {
                if m == Mix::SuperSimple {
                    *font = super::doc::DEFAULT_FONT.to_string();
                }
                m.apply_char(chars);
                m.apply_para(para);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::{Align, CSCH, Pos, Size};

    fn doc() -> Document {
        Document::from_text("Era uma vez um gato\nazul", CharProps::default())
    }

    fn size(d: &Document, para: usize, k: usize) -> Size {
        d.paras[para].items[k].props().unwrap().size
    }

    #[test]
    fn char_brush_needs_a_drag_and_takes_whole_words() {
        let b = Brush::Char(CharChange::Size(Size::Large));
        let mut d = doc();
        // um clique não basta
        b.stroke(&mut d, Range::at(Pos::new(0, 5)), false);
        assert_eq!(d, doc());
        // arrastar de "uma" (meio) até "vez" (meio) muda as duas inteiras
        b.stroke(&mut d, Range::new(Pos::new(0, 5), Pos::new(0, 9)), true);
        let sizes: Vec<_> = (0..d.paras[0].len()).map(|k| size(&d, 0, k)).collect();
        assert_eq!(sizes[..4], [Size::Minimal; 4]);
        assert_eq!(sizes[4..11], [Size::Large; 7]);
        assert_eq!(sizes[11..], [Size::Minimal; 8]);
        assert!(d.paras[0].props.is_default());
    }

    #[test]
    fn para_brush_works_with_a_click() {
        let b = Brush::Para(ParaChange::Align(Align::Center));
        let mut d = doc();
        b.stroke(&mut d, Range::at(Pos::new(1, 2)), false);
        assert_eq!(d.paras[1].props.align, Align::Center);
        assert_eq!(d.paras[0].props.align, Align::Left);
        assert!(
            d.paras[1]
                .items
                .iter()
                .all(|i| i.props().unwrap().is_default())
        );
        // arrastando por dois parágrafos, os dois
        b.stroke(&mut d, Range::new(Pos::new(0, 2), Pos::new(1, 0)), true);
        assert_eq!(d.paras[0].props.align, Align::Center);
    }

    #[test]
    fn font_brush_adds_the_font_to_the_document() {
        let mut d = doc();
        Brush::Font("Impact").stroke(&mut d, Range::new(Pos::new(1, 0), Pos::new(1, 1)), true);
        let ftc = d.fonts.ftc_of("Impact").unwrap();
        assert!(
            d.paras[1]
                .items
                .iter()
                .all(|i| i.props().unwrap().font == ftc)
        );
        assert_eq!(d.paras[0].items[0].props().unwrap().font, 0);
    }

    #[test]
    fn mix_on_click_changes_only_the_paragraph() {
        let mut d = doc();
        Brush::Mix(Mix::Title).stroke(&mut d, Range::at(Pos::new(0, 1)), false);
        assert_eq!(d.paras[0].props.align, Align::Center);
        assert!(!d.paras[0].items[0].props().unwrap().bold);
        Brush::Mix(Mix::Title).stroke(&mut d, Range::new(Pos::new(0, 1), Pos::new(0, 2)), true);
        assert!(d.paras[0].items[0].props().unwrap().bold);
        assert!(!d.paras[0].items[4].props().unwrap().bold);
    }

    #[test]
    fn applicator_changes_everything_including_the_marks() {
        let mut d = doc();
        Brush::Char(CharChange::Color(Some(CSCH[10]))).apply_all(&mut d);
        for par in &d.paras {
            assert!(
                par.items
                    .iter()
                    .all(|i| i.props().unwrap().color == Some(CSCH[10]))
            );
            assert_eq!(par.mark.color, Some(CSCH[10]));
        }
        Brush::Mix(Mix::SuperSimple).apply_all(&mut d);
        assert_eq!(d.paras[1].mark, CharProps::default());
    }

    #[test]
    fn style_for_new_documents() {
        let (mut f, mut c, mut p) = (
            String::from("Lucida Sans"),
            CharProps::default(),
            ParaProps::default(),
        );
        Brush::Font("Stencil").apply_style(&mut f, &mut c, &mut p);
        Brush::Char(CharChange::Size(Size::Huge)).apply_style(&mut f, &mut c, &mut p);
        Brush::Mix(Mix::Report).apply_style(&mut f, &mut c, &mut p);
        assert_eq!(
            (f.as_str(), c.size, p.space_after),
            ("Stencil", Size::Huge, true)
        );
        Brush::Mix(Mix::SuperSimple).apply_style(&mut f, &mut c, &mut p);
        assert_eq!(f, "Lucida Sans");
        assert!(c.is_default() && p.is_default());
    }
}
