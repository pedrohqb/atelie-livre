//! Corretor, sinônimos e anagramas sobre os dicionários originais
//! (`ATELIE_DATA`) e sobre os livres do sistema (Hunspell pt_BR + MyThes).

use engine::writer::dict::{Dictionary, Source, Variant};
use std::path::{Path, PathBuf};

fn original() -> Option<Dictionary> {
    let Some(d) = std::env::var_os("ATELIE_DATA") else {
        eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
        return None;
    };
    Some(Dictionary::from_original(&PathBuf::from(d), Variant::Brasil).unwrap())
}

fn free() -> Option<Dictionary> {
    let aff = Path::new("/usr/share/hunspell/pt_BR.aff");
    if !aff.exists() {
        eprintln!("aviso: hunspell pt_BR ausente; teste do dicionário livre pulado");
        return None;
    }
    let thes = Path::new("/usr/share/mythes/th_pt_BR.dat");
    let thes = thes.exists().then_some(thes);
    Some(Dictionary::from_free(aff, &aff.with_extension("dic"), thes, Variant::Brasil).unwrap())
}

/// O mesmo comportamento mínimo vale para as duas fontes.
fn common(d: &Dictionary) {
    for w in [
        "casa",
        "Casa",
        "CASA",
        "meninos",
        "escrevíamos",
        "Brasil",
        "ação",
        "ônibus",
    ] {
        assert!(d.contains(w), "{w} ({:?})", d.source());
    }
    for w in ["gatu", "acção", "zzqk", ""] {
        assert!(!d.contains(w), "{w} ({:?})", d.source());
    }
    let g = d.suggestions("gatu", 10);
    assert!(g.contains(&"gato".to_string()), "{g:?}");
    assert!(d.words().any(|w| w == "abacaxi"));
    if d.has_thesaurus() {
        let s = d.synonyms("casa");
        assert!(s.contains(&"lar".to_string()), "{s:?}");
        assert!(!s.contains(&"casa".to_string()));
    }
    let a = d.anagrams("ramo");
    for w in ["amor", "mora", "ramo"] {
        assert!(a.contains(&w.to_string()), "{w} em {a:?}");
    }
    assert!(!a.contains(&"Roma".to_string()));
    assert!(d.anagrams("a-b").is_empty());
}

#[test]
fn original_dictionary() {
    let Some(d) = original() else { return };
    assert_eq!(d.source(), Source::Original);
    common(&d);
    assert_eq!(d.words().count(), 211_462);
    assert_eq!(d.suggestions("gatu", 3), ["gata", "gato", "tatu"]);
    assert!(!d.contains("brasil") && d.suggestions("brasil", 3)[0] == "Brasil");
    assert!(!d.contains("pexe") && d.suggestions("pexe", 10).contains(&"peixe".to_string()));
    assert_eq!(d.synonyms("casa")[0], "botoeira");
    // observado no original: ramo → amor, mora, oram, ramo
    assert_eq!(d.anagrams("ramo"), ["amor", "mora", "oram", "ramo"]);
    let pt = Dictionary::from_original(
        &PathBuf::from(std::env::var_os("ATELIE_DATA").unwrap()),
        Variant::Portugal,
    )
    .unwrap();
    assert!(pt.contains("acção") && pt.contains("ideia") && !pt.contains("ônibus"));
}

#[test]
fn free_dictionary() {
    let Some(d) = free() else { return };
    assert_eq!(d.source(), Source::Free);
    common(&d);
    // flexões e plurais irregulares que faltam no SPELL.DCT
    for w in ["irmãos", "pães", "alemães", "abafá-lo"] {
        assert!(d.contains(w), "{w}");
    }
    assert!(d.words().count() > 300_000);
}

#[test]
fn load_prefers_original() {
    let d = Dictionary::load(
        std::env::var_os("ATELIE_DATA").as_deref().map(Path::new),
        Variant::Brasil,
    );
    if let Some(d) = d
        && std::env::var_os("ATELIE_DATA").is_some()
    {
        assert_eq!(d.source(), Source::Original);
    }
}
