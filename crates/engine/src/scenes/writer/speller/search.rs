//! Busca, troca e varredura ortográfica sobre o documento (abelha:
//! PULAR PARA, CORRIGIR, TROCAR e Embaralhar).
//!
//! As comparações não diferenciam maiúsculas de minúsculas, mas diferenciam
//! acentos (`e` não acha `é`). Sem Ctrl, só palavras inteiras: `porto` acha
//! `porto` e `Porto`, não `aeroporto`; com Ctrl, qualquer trecho
//! (`0fa4[3]`, `[13]`). As figuras do Maçã separam palavras e nunca casam.
//! Um trecho procurado não atravessa parágrafos.

use crate::writer::dict::tokens;
use crate::writer::doc::{Document, Pos, Range};
use formats::dct::{Case, capitalize, case_of};

/// Letras do parágrafo, item a item (`None` nas figuras), já em minúsculas.
fn folded(doc: &Document, para: usize) -> Vec<Option<char>> {
    doc.paras[para]
        .items
        .iter()
        .map(|it| it.char().map(fold))
        .collect()
}

/// Minúscula de um caractere sem mudar a contagem (`İ` fica como está).
fn fold(c: char) -> char {
    let mut l = c.to_lowercase();
    match (l.next(), l.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

fn is_letter(c: Option<char>) -> bool {
    c.is_some_and(char::is_alphabetic)
}

/// Ocorrências de `needle` no parágrafo, como intervalos de itens.
fn matches_in(hay: &[Option<char>], needle: &[char], partial: bool) -> Vec<(usize, usize)> {
    let n = needle.len();
    if n == 0 || hay.len() < n {
        return Vec::new();
    }
    (0..=hay.len() - n)
        .filter(|&i| {
            hay[i..i + n]
                .iter()
                .zip(needle)
                .all(|(h, c)| *h == Some(*c))
        })
        .filter(|&i| {
            partial
                || (!is_letter(i.checked_sub(1).and_then(|j| hay[j]))
                    && !is_letter(hay.get(i + n).copied().flatten()))
        })
        .map(|i| (i, i + n))
        .collect()
}

fn needle_of(text: &str) -> Vec<char> {
    text.chars().map(fold).collect()
}

/// Todas as ocorrências de `text` no documento, em ordem.
pub fn find_all(doc: &Document, text: &str, partial: bool) -> Vec<Range> {
    let needle = needle_of(text);
    (0..doc.paras.len())
        .flat_map(|p| {
            matches_in(&folded(doc, p), &needle, partial)
                .into_iter()
                .map(move |(s, e)| Range::new(Pos::new(p, s), Pos::new(p, e)))
        })
        .collect()
}

/// Primeira ocorrência que começa em `from` ou depois.
pub fn find_next(doc: &Document, from: Pos, text: &str, partial: bool) -> Option<Range> {
    let needle = needle_of(text);
    (from.para.min(doc.paras.len())..doc.paras.len()).find_map(|p| {
        matches_in(&folded(doc, p), &needle, partial)
            .into_iter()
            .find(|&(s, _)| p > from.para || s >= from.index)
            .map(|(s, e)| Range::new(Pos::new(p, s), Pos::new(p, e)))
    })
}

/// O intervalo `r` é exatamente uma ocorrência de `text`?
pub fn is_match(doc: &Document, r: Range, text: &str, partial: bool) -> bool {
    r.start.para == r.end.para
        && !r.is_empty()
        && find_next(doc, r.start, text, partial).is_some_and(|m| m == r)
}

/// Texto puro do intervalo (as figuras somem).
pub fn text_of(doc: &Document, r: Range) -> String {
    doc.text_range(r)
}

/// Palavras do parágrafo como o corretor as separa, em itens.
pub fn words_in(doc: &Document, para: usize) -> Vec<(usize, usize, String)> {
    // as figuras viram espaço: 1 caractere por item
    let text: String = doc.paras[para]
        .items
        .iter()
        .map(|it| it.char().unwrap_or(' '))
        .collect();
    // posições em bytes → em caracteres (itens)
    let starts: Vec<usize> = text.char_indices().map(|(b, _)| b).collect();
    let item = |b: usize| starts.partition_point(|&s| s < b);
    tokens(&text)
        .into_iter()
        .map(|r| (item(r.start), item(r.end), text[r].to_string()))
        .collect()
}

/// Próxima palavra que `known` não aceita, a partir de `from` (inclusive) e
/// antes de `until` (exclusive; `None` = até o fim do documento).
pub fn next_unknown(
    doc: &Document,
    from: Pos,
    until: Option<Pos>,
    known: impl Fn(&str) -> bool,
) -> Option<(Range, String)> {
    for p in from.para..doc.paras.len() {
        for (s, e, w) in words_in(doc, p) {
            let start = Pos::new(p, s);
            if start < from {
                continue;
            }
            if until.is_some_and(|u| start >= u) {
                return None;
            }
            if !known(&w) {
                return Some((Range::new(start, Pos::new(p, e)), w));
            }
        }
    }
    None
}

/// Aplica à troca a caixa do trecho achado: `Gato` → `Peixe`, `GATO` →
/// `PEIXE`; minúsculas ou mistas ficam como foram digitadas.
pub fn match_case(replacement: &str, found: &str) -> String {
    let letters = found.chars().filter(|c| c.is_alphabetic()).count();
    if letters > 1 && found == found.to_uppercase() && found != found.to_lowercase() {
        return replacement.to_uppercase();
    }
    if case_of(found) == Case::Title && case_of(replacement) == Case::Lower {
        return capitalize(replacement);
    }
    replacement.to_string()
}

/// Troca o intervalo (de um parágrafo) por `text`, com o estilo do primeiro
/// caractere trocado. Devolve o intervalo do texto novo.
pub fn replace(doc: &mut Document, r: Range, text: &str) -> Range {
    let props = doc.props_at(Pos::new(r.start.para, r.start.index + 1));
    let start = doc.delete(r);
    let end = doc.insert_text(start, text, props);
    Range::new(start, end)
}

/// Troca todas as ocorrências; devolve quantas.
pub fn replace_all(doc: &mut Document, find: &str, by: &str, partial: bool) -> usize {
    let all = find_all(doc, find, partial);
    // de trás para a frente: os intervalos anteriores não mudam
    for r in all.iter().rev() {
        let found = text_of(doc, *r);
        replace(doc, *r, &match_case(by, &found));
    }
    all.len()
}

/// Palavra sob o ponto de inserção (ou a seleção, se houver e for de um
/// parágrafo só): o texto com que o TROCAR ISTO começa.
pub fn word_at_caret(doc: &Document, sel: Range) -> String {
    if !sel.is_empty() {
        if sel.start.para == sel.end.para {
            return text_of(doc, sel).trim().to_string();
        }
        return String::new();
    }
    let p = sel.start;
    words_in(doc, p.para)
        .into_iter()
        .find(|(s, e, _)| *s <= p.index && p.index <= *e)
        .map(|(_, _, w)| w)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::{CharProps, Item};

    fn doc(t: &str) -> Document {
        Document::from_text(t, CharProps::default())
    }

    fn r(p: usize, s: usize, e: usize) -> Range {
        Range::new(Pos::new(p, s), Pos::new(p, e))
    }

    #[test]
    fn palavra_inteira_ou_parte() {
        let d = doc("O porto e o aeroporto.\nPORTO Porto");
        assert_eq!(
            find_all(&d, "porto", false),
            [r(0, 2, 7), r(1, 0, 5), r(1, 6, 11)]
        );
        assert_eq!(find_all(&d, "porto", true).len(), 4);
        assert_eq!(
            find_next(&d, Pos::new(0, 3), "porto", false),
            Some(r(1, 0, 5))
        );
        assert_eq!(
            find_next(&d, Pos::new(0, 3), "porto", true),
            Some(r(0, 16, 21))
        );
        assert_eq!(find_next(&d, Pos::new(1, 7), "porto", false), None);
        // acentos contam; trechos com espaço valem
        assert!(find_all(&doc("café cafe"), "cafe", false) == [r(0, 5, 9)]);
        assert_eq!(find_all(&d, "o porto", false), [r(0, 0, 7)]);
        assert!(find_all(&d, "", false).is_empty());
    }

    #[test]
    fn varre_palavras_desconhecidas() {
        let d = doc("O gatu comeu\no pexe na caza.");
        let known = |w: &str| ["O", "o", "comeu", "na"].contains(&w);
        let (rg, w) = next_unknown(&d, Pos::default(), None, known).unwrap();
        assert_eq!((rg, w.as_str()), (r(0, 2, 6), "gatu"));
        let (rg, w) = next_unknown(&d, rg.end, None, known).unwrap();
        assert_eq!((rg, w.as_str()), (r(1, 2, 6), "pexe"));
        assert_eq!(
            next_unknown(&d, Pos::new(1, 3), None, known).unwrap().1,
            "caza"
        );
        assert!(next_unknown(&d, Pos::default(), Some(Pos::new(0, 2)), known).is_none());
    }

    #[test]
    fn troca_com_a_caixa_e_o_estilo() {
        assert_eq!(match_case("peixe", "Gato"), "Peixe");
        assert_eq!(match_case("peixe", "GATO"), "PEIXE");
        assert_eq!(match_case("peixe", "gato"), "peixe");
        assert_eq!(match_case("McZee", "Gato"), "McZee");
        let mut d = doc("O gato e o Gato.");
        assert_eq!(replace_all(&mut d, "gato", "peixe", false), 2);
        assert_eq!(d.text(), "O peixe e o Peixe.");
        let mut d = doc("abc");
        let props = CharProps {
            bold: true,
            ..CharProps::default()
        };
        d.paras[0].items[1] = Item::Char('b', props);
        let n = replace(&mut d, r(0, 1, 2), "XY");
        assert_eq!(n, r(0, 1, 3));
        assert_eq!(d.text(), "aXYc");
        assert_eq!(d.paras[0].items[2].props(), Some(&props));
    }

    #[test]
    fn palavra_do_cursor() {
        let d = doc("O gato comeu");
        assert_eq!(word_at_caret(&d, Range::at(Pos::new(0, 4))), "gato");
        assert_eq!(word_at_caret(&d, Range::at(Pos::new(0, 6))), "gato");
        assert_eq!(word_at_caret(&d, r(0, 7, 12)), "comeu");
        assert_eq!(word_at_caret(&d, Range::at(Pos::new(0, 1))), "O");
    }
}
