mod support;

use formats::dct::{SpellDct, Thesaurus, variant};

fn spell() -> Option<SpellDct> {
    let d = support::read("WRITER/SPELL.DCT")?;
    Some(SpellDct::parse(&d).unwrap())
}

#[test]
fn spell_header_and_counts() {
    let Some(s) = spell() else { return };
    let h = &s.header;
    assert_eq!(h.language, "PORTUGUESE");
    assert_eq!(h.date, "950721");
    assert_eq!(
        (h.block_size, h.blocks, h.code_bits, h.last_terminal),
        (128, 881, 10, 1023)
    );
    assert_eq!(s.entries().len(), 215_667);
    assert_eq!(s.words(variant::AMBAS).count(), 215_667);
    assert_eq!(s.words(variant::BRASIL).count(), 211_462);
    assert_eq!(s.words(variant::PORTUGAL).count(), 209_220);
    assert_eq!(s.entries().first().unwrap().text, "a");
    assert_eq!(s.phonetic.len(), 169);
    assert_eq!(s.phonetic[0], ("al".into(), "au".into()));
    assert_eq!(s.phonetic[168], ("ü".into(), "u".into()));
}

#[test]
fn spell_words_and_flexions() {
    let Some(s) = spell() else { return };
    let br = variant::BRASIL;
    for w in [
        "casa",
        "casas",
        "menino",
        "meninos",
        "escrever",
        "escrevendo",
        "escreveu",
        "escrevíamos",
        "abafadiço",
        "abafadíssimo",
        "você",
        "xícara",
        "canções",
        "mãos",
        "irmãs",
        "Brasil",
        "BRASIL",
        "São",
        "Paulo",
        "EUA",
        "TV",
        "a.C.",
        "ação",
        "ônibus",
        "idéia",
        "encontrei-o",
        "abafam-no",
        "pão-de-ló",
        "Casa",
        "CASA",
        "irmos",
    ] {
        assert!(s.contains(w, br), "{w}");
    }
    for w in [
        "brasil", "acção", "facto", "ideia", "vamo-nos", "faz-lo", "gatu", "pexe", "cAsa", "eua",
        "",
    ] {
        assert!(!s.contains(w, br), "{w}");
    }
    let pt = variant::PORTUGAL;
    assert!(s.contains("acção", pt) && s.contains("ideia", pt) && !s.contains("ônibus", pt));
    let e = s.entries().iter().find(|w| w.text == "abafamos").unwrap();
    assert_eq!((e.class, e.variants), (1, variant::AMBAS));
}

#[test]
fn spell_suggestions_like_the_original() {
    let Some(s) = spell() else { return };
    let br = variant::BRASIL;
    // observado no original: gatu → gata, gato, tatu
    assert_eq!(s.suggestions("gatu", br, 3), ["gata", "gato", "tatu"]);
    // pexe → pese (troca fonética x→s) primeiro; peixe, peie, pede, pele também
    let p = s.suggestions("pexe", br, 12);
    assert_eq!(p[0], "pese");
    for w in ["peixe", "peie", "pede", "pele"] {
        assert!(p.contains(&w.to_string()), "{w} em {p:?}");
    }
    // brasil → Brasil (caixa) primeiro
    assert_eq!(s.suggestions("brasil", br, 3)[0], "Brasil");
    // aco → aço (c→ç) entre as sugestões; abaf → aba, abafa, abafe, abafo, abas
    assert!(s.suggestions("aco", br, 8).contains(&"aço".to_string()));
    let a = s.suggestions("abaf", br, 8);
    for w in ["aba", "abafa", "abafe", "abafo", "abas"] {
        assert!(a.contains(&w.to_string()), "{w} em {a:?}");
    }
    assert_eq!(s.suggestions("Gatu", br, 1), ["Gata"]);
}

#[test]
fn thesaurus_entries_and_synonyms() {
    let Some(d) = support::read("WRITER/THES.DCT") else {
        return;
    };
    let t = Thesaurus::parse(&d).unwrap();
    assert_eq!(t.entries().len(), 3983);
    assert_eq!(t.forms().len(), 114_581);
    let first = &t.entries()[0];
    assert_eq!((first.headword.as_str(), first.pos), ("a", 41));
    assert_eq!(first.synonyms, ["a", "para"]);
    assert_eq!(t.entries().last().unwrap().headword, "zumbido");
    // o original mostra "botoeira" primeiro para "casa"
    let casa = t.synonyms("casa");
    assert_eq!(casa[0], "botoeira");
    assert!(casa.contains(&"domicílio".to_string()) && casa.contains(&"lar".to_string()));
    assert!(!casa.contains(&"casa".to_string()));
    // forma flexionada → entradas do lema
    assert_eq!(t.lookup("corri")[0].headword, "correr");
    assert!(t.synonyms("bonito").contains(&"lindo".to_string()));
    assert_eq!(t.lookup("a despeito de")[0].headword, "apesar de");
    assert_eq!(t.lookup("A Virgem")[0].headword, "Madona");
    assert!(t.synonyms("xyzzy").is_empty());
    // toda forma com entrada aponta uma entrada que a contém (ou contém o lema)
    let mut bad = 0;
    for f in t.forms() {
        if let Some(e) = f.entry {
            let syn = &t.entries()[e as usize - 1].synonyms;
            let hit = |w: &str| syn.iter().any(|s| s.to_lowercase() == w.to_lowercase());
            if !hit(&f.form) && !f.lemma.as_deref().is_some_and(hit) {
                bad += 1;
            }
        }
    }
    assert_eq!(bad, 0);
}
