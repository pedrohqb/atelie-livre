//! Coloca-figuras (a Maçã dos Transformadores) sobre o documento: põe a
//! figura-rébus logo depois de cada palavra conhecida e tira todas
//! (Ctrl+clique). Comportamento medido no original em
//! `docs/formats/maca.md` §5:
//!
//! - percorre **o documento inteiro**, com ou sem seleção;
//! - palavra = sequência máxima de letras (inclusive as acentuadas) e `-`,
//!   comparada em minúsculas; "pique-nique" acha, "maç" não;
//! - a figura entra entre a palavra e a pontuação (`Bicicleta[🚲], …`);
//! - palavra que já tem figura logo depois não recebe outra.
//!
//! A tabela palavra → figura é a do `WRITER.PIC` ([`formats::maca`]); aqui
//! ela entra como uma função, para testar sem os dados.

use super::doc::{Document, Item, Pos};
use super::selection::Selection;

/// Resultado de um clique na Maçã.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placed {
    /// Pôs `n` figuras (n ≥ 1).
    Some(usize),
    /// Todas as palavras conhecidas já tinham figura: "Epa!" `0805[4]`.
    AllMatched,
    /// Nenhuma palavra conhecida no documento: `0805[3]`.
    NoMatches,
}

fn word_char(it: &Item) -> bool {
    matches!(it, Item::Char(c, _) if c.is_alphabetic() || *c == '-')
}

/// Palavras do parágrafo `items`: `(início, fim, texto)` com o fim logo
/// depois da última letra (os hífens das pontas ficam de fora).
fn words(items: &[Item]) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    let mut k = 0;
    while k < items.len() {
        if !word_char(&items[k]) {
            k += 1;
            continue;
        }
        let s = k;
        while k < items.len() && word_char(&items[k]) {
            k += 1;
        }
        let text: Vec<char> = items[s..k].iter().filter_map(Item::char).collect();
        let lead = text.iter().take_while(|&&c| c == '-').count();
        let trail = text.iter().rev().take_while(|&&c| c == '-').count();
        if lead < text.len() {
            let word: String = text[lead..text.len() - trail].iter().collect();
            out.push((s + lead, k - trail, word));
        }
    }
    out
}

/// Desloca `p` uma posição se um item entrou antes dele no parágrafo.
fn shift_after(p: &mut Pos, at: Pos) {
    if p.para == at.para && p.index >= at.index {
        p.index += 1;
    }
}

/// Recua `p` se o item em `at` saiu.
fn shift_removed(p: &mut Pos, at: Pos) {
    if p.para == at.para && p.index > at.index {
        p.index -= 1;
    }
}

/// Põe a figura `lookup(palavra)` depois de cada palavra conhecida que ainda
/// não tem uma, em todo o documento; a seleção acompanha o texto.
pub fn place(doc: &mut Document, sel: &mut Selection, lookup: impl Fn(&str) -> Option<u8>) -> Placed {
    let mut placed = 0;
    let mut known = 0;
    for pi in 0..doc.paras.len() {
        // de trás para a frente: os índices das palavras seguintes não mudam
        for (_, end, word) in words(&doc.paras[pi].items).into_iter().rev() {
            let Some(n) = lookup(&word).filter(|&n| n != 0) else {
                continue;
            };
            known += 1;
            if matches!(doc.paras[pi].items.get(end), Some(Item::Rebus(_))) {
                continue;
            }
            let at = Pos::new(pi, end);
            doc.insert_item(at, Item::Rebus(n));
            shift_after(&mut sel.anchor, at);
            shift_after(&mut sel.caret, at);
            placed += 1;
        }
    }
    sel.goal_x = None;
    match (placed, known) {
        (0, 0) => Placed::NoMatches,
        (0, _) => Placed::AllMatched,
        (n, _) => Placed::Some(n),
    }
}

/// Tira todas as figuras-rébus do documento (Ctrl+clique na Maçã); devolve
/// quantas.
pub fn remove(doc: &mut Document, sel: &mut Selection) -> usize {
    let mut n = 0;
    for pi in 0..doc.paras.len() {
        let mut k = doc.paras[pi].items.len();
        while k > 0 {
            k -= 1;
            if matches!(doc.paras[pi].items[k], Item::Rebus(_)) {
                doc.paras[pi].items.remove(k);
                let at = Pos::new(pi, k);
                shift_removed(&mut sel.anchor, at);
                shift_removed(&mut sel.caret, at);
                n += 1;
            }
        }
    }
    sel.goal_x = None;
    n
}

/// O documento tem alguma figura-rébus.
pub fn has_rebus(doc: &Document) -> bool {
    doc.paras
        .iter()
        .any(|p| p.items.iter().any(|i| matches!(i, Item::Rebus(_))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::CharProps;

    /// A tabela de teste: as palavras de `w26_01_typed`.
    fn table(w: &str) -> Option<u8> {
        match w.to_lowercase().as_str() {
            "gato" | "gatos" => Some(0x25),
            "aeronave" => Some(1),
            "sono" | "cama" => Some(0x0c),
            "bicicleta" => Some(0x0f),
            "parque" => Some(0xeb),
            "pique-nique" => Some(0x99),
            "tic-tac" => Some(0x02),
            "café" => Some(0xa9),
            _ => None,
        }
    }

    /// O texto com as figuras como `[xx]`.
    fn show(d: &Document) -> String {
        d.paras
            .iter()
            .map(|p| {
                p.items
                    .iter()
                    .map(|i| match i {
                        Item::Char(c, _) => c.to_string(),
                        Item::Rebus(n) => format!("[{n:02x}]"),
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    fn doc(t: &str) -> Document {
        Document::from_text(t, CharProps::default())
    }

    #[test]
    fn words_are_letters_and_hyphens() {
        let d = doc("Bicicleta, pique-nique. -gato- maç 12abc");
        let w: Vec<String> = words(&d.paras[0].items)
            .into_iter()
            .map(|(_, _, w)| w)
            .collect();
        assert_eq!(w, ["Bicicleta", "pique-nique", "gato", "maç", "abc"]);
        let (s, e, _) = words(&d.paras[0].items)[2].clone();
        assert_eq!((s, e), (25, 29));
    }

    #[test]
    fn places_after_known_words_before_punctuation() {
        let mut d = doc(
            "gato Gato GATO aeronave sono gatos Bicicleta, parque. cama bzzz\n\
             pique-nique tic-tac abelha maç café papai",
        );
        let mut sel = Selection::at(d.end());
        assert_eq!(place(&mut d, &mut sel, table), Placed::Some(12));
        assert_eq!(
            show(&d),
            "gato[25] Gato[25] GATO[25] aeronave[01] sono[0c] gatos[25] \
             Bicicleta[0f], parque[eb]. cama[0c] bzzz/\
             pique-nique[99] tic-tac[02] abelha maç café[a9] papai"
        );
        // o cursor continua no fim
        assert_eq!(sel.caret, d.end());
        // de novo: nada novo
        assert_eq!(place(&mut d, &mut sel, table), Placed::AllMatched);
        // tirar tudo volta ao texto
        assert_eq!(remove(&mut d, &mut sel), 12);
        assert!(!has_rebus(&d));
        assert!(show(&d).starts_with("gato Gato GATO aeronave"));
        assert_eq!(sel.caret, d.end());
    }

    #[test]
    fn no_known_words_and_partial_placement() {
        let mut d = doc("abelha papai");
        let mut sel = Selection::default();
        assert_eq!(place(&mut d, &mut sel, table), Placed::NoMatches);
        let mut d = doc("gato e cama");
        let mut sel = Selection::new(Pos::new(0, 5), Pos::new(0, 11));
        d.insert_item(Pos::new(0, 4), Item::Rebus(0x25));
        sel.anchor.index += 1;
        sel.caret.index += 1;
        // só "cama" é novidade; a seleção "e cama" cresce com a figura
        assert_eq!(place(&mut d, &mut sel, table), Placed::Some(1));
        assert_eq!(show(&d), "gato[25] e cama[0c]");
        assert_eq!((sel.anchor.index, sel.caret.index), (6, 13));
        assert_eq!(remove(&mut d, &mut sel), 2);
        assert_eq!((sel.anchor.index, sel.caret.index), (5, 11));
    }

    #[test]
    fn zero_from_the_table_is_ignored() {
        let mut d = doc("gato");
        let mut sel = Selection::default();
        assert_eq!(place(&mut d, &mut sel, |_| Some(0)), Placed::NoMatches);
        assert_eq!(show(&d), "gato");
    }
}
