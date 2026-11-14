//! Dicionários do Escritório: corretor (abelha **Corrigir**), sinônimos
//! (**Trocar** → "Palavras parecidas") e anagramas (**Embaralhar**).
//!
//! Duas fontes com a mesma API:
//!
//! - **original** — `WRITER/SPELL.DCT` e `WRITER/THES.DCT` da cópia instalada
//!   do usuário, decodificados por [`formats::dct`];
//! - **livre** — Hunspell pt-BR (VERO, LGPLv3/MPL) pelo `spellbook` e o
//!   tesauro pt-BR do LibreOffice (MyThes, MPL-2.0), lidos dos arquivos do
//!   sistema (`/usr/share/hunspell`, `/usr/share/mythes`) ou da pasta
//!   `ATELIE_DICT`.
//!
//! [`Dictionary::load`] prefere o original e cai no livre. Veja
//! `docs/formats/dct.md`.

use formats::dct::{self, SpellDct, Thesaurus};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Grafia aceita pelo corretor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    /// Brasileira (o Writer usa esta quando o Windows está em "ptb"/"Brazil").
    #[default]
    Brasil,
    /// Europeia.
    Portugal,
}

impl Variant {
    fn mask(self) -> u8 {
        match self {
            Variant::Brasil => dct::variant::BRASIL,
            Variant::Portugal => dct::variant::PORTUGAL,
        }
    }
}

/// De onde vieram os dados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Original,
    Free,
}

/// Maior palavra para a qual os anagramas do dicionário livre são buscados
/// por permutação.
const MAX_ANAGRAM_LEN: usize = 8;

enum Backend {
    Original {
        spell: Box<SpellDct>,
        thes: Option<Box<Thesaurus>>,
    },
    Free {
        hun: Box<spellbook::Dictionary>,
        stems: Vec<String>,
        thes: Option<MyThes>,
    },
}

/// Corretor, sinônimos e anagramas.
pub struct Dictionary {
    backend: Backend,
    variant: Variant,
    anagrams: OnceLock<HashMap<String, Vec<String>>>,
}

impl std::fmt::Debug for Dictionary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dictionary")
            .field("source", &self.source())
            .field("variant", &self.variant)
            .finish()
    }
}

fn read(p: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn read_text(p: &Path) -> Result<String, String> {
    let b = read(p)?;
    let s = String::from_utf8(b).map_err(|e| format!("{}: {e}", p.display()))?;
    Ok(s.trim_start_matches('\u{feff}').to_string())
}

impl Dictionary {
    /// Dicionários originais de uma pasta `MSKIDS` (o de sinônimos é opcional).
    pub fn from_original(mskids: &Path, variant: Variant) -> Result<Self, String> {
        let dir = mskids.join("WRITER");
        let spell = SpellDct::parse(&read(&dir.join("SPELL.DCT"))?).map_err(|e| e.to_string())?;
        let thes = match read(&dir.join("THES.DCT")) {
            Ok(d) => Some(Box::new(Thesaurus::parse(&d).map_err(|e| e.to_string())?)),
            Err(_) => None,
        };
        Ok(Self::new(
            Backend::Original {
                spell: Box::new(spell),
                thes,
            },
            variant,
        ))
    }

    /// Dicionário Hunspell (`.aff` + `.dic`) e, opcionalmente, tesauro MyThes (`.dat` com `.idx`).
    pub fn from_free(
        aff: &Path,
        dic: &Path,
        thes: Option<&Path>,
        variant: Variant,
    ) -> Result<Self, String> {
        let aff_s = read_text(aff)?;
        let dic_s = read_text(dic)?;
        let hun = spellbook::Dictionary::new(&aff_s, &dic_s)
            .map_err(|e| format!("{}: {e}", dic.display()))?;
        let stems = dic_s
            .lines()
            .skip(1)
            .filter_map(|l| {
                let w = l.split(['/', '\t']).next()?.trim();
                (!w.is_empty()).then(|| w.to_string())
            })
            .collect();
        let thes = match thes {
            Some(p) => Some(MyThes::open(p)?),
            None => None,
        };
        Ok(Self::new(
            Backend::Free {
                hun: Box::new(hun),
                stems,
                thes,
            },
            variant,
        ))
    }

    /// Dicionários livres do sistema: pasta `ATELIE_DICT` (se definida),
    /// depois `/usr/share/hunspell` + `/usr/share/mythes`. Procura
    /// `pt_BR` (ou `pt_PT` para [`Variant::Portugal`], com `pt_BR` de reserva).
    pub fn free_system(variant: Variant) -> Option<Self> {
        let mut dirs: Vec<(PathBuf, PathBuf)> = Vec::new();
        if let Some(d) = std::env::var_os("ATELIE_DICT") {
            let d = PathBuf::from(d);
            dirs.push((d.clone(), d));
        }
        dirs.push(("/usr/share/hunspell".into(), "/usr/share/mythes".into()));
        let langs: &[&str] = match variant {
            Variant::Brasil => &["pt_BR"],
            Variant::Portugal => &["pt_PT", "pt_BR"],
        };
        for (hdir, tdir) in &dirs {
            for lang in langs {
                let aff = hdir.join(format!("{lang}.aff"));
                let dic = hdir.join(format!("{lang}.dic"));
                if !aff.exists() || !dic.exists() {
                    continue;
                }
                let thes = [format!("th_{lang}.dat"), format!("th_{lang}_v2.dat")]
                    .into_iter()
                    .map(|n| tdir.join(n))
                    .find(|p| p.exists() && p.with_extension("idx").exists());
                if let Ok(d) = Self::from_free(&aff, &dic, thes.as_deref(), variant) {
                    return Some(d);
                }
            }
        }
        None
    }

    /// O original, se houver `mskids/WRITER/SPELL.DCT` legível; senão o livre do sistema.
    pub fn load(mskids: Option<&Path>, variant: Variant) -> Option<Self> {
        mskids
            .and_then(|m| Self::from_original(m, variant).ok())
            .or_else(|| Self::free_system(variant))
    }

    fn new(backend: Backend, variant: Variant) -> Self {
        Dictionary {
            backend,
            variant,
            anagrams: OnceLock::new(),
        }
    }

    pub fn source(&self) -> Source {
        match self.backend {
            Backend::Original { .. } => Source::Original,
            Backend::Free { .. } => Source::Free,
        }
    }

    pub fn variant(&self) -> Variant {
        self.variant
    }

    /// Há dicionário de sinônimos?
    pub fn has_thesaurus(&self) -> bool {
        match &self.backend {
            Backend::Original { thes, .. } => thes.is_some(),
            Backend::Free { thes, .. } => thes.is_some(),
        }
    }

    /// Lista de palavras: no original, todas as formas da variante (cada
    /// flexão é um registro); no livre, os radicais do `.dic` (as flexões
    /// saem das regras de afixos e não são listadas).
    pub fn words(&self) -> Box<dyn Iterator<Item = &str> + '_> {
        match &self.backend {
            Backend::Original { spell, .. } => Box::new(spell.words(self.variant.mask())),
            Backend::Free { stems, .. } => Box::new(stems.iter().map(String::as_str)),
        }
    }

    /// `word` (uma palavra, como sai de [`tokens`]) está correta?
    pub fn contains(&self, word: &str) -> bool {
        match &self.backend {
            Backend::Original { spell, .. } => spell.contains(word, self.variant.mask()),
            Backend::Free { hun, .. } => {
                !word.is_empty()
                    && (hun.check(word)
                        || word.contains('-')
                            && word.split('-').all(|p| p.is_empty() || hun.check(p)))
            }
        }
    }

    /// Até `max` sugestões para uma palavra desconhecida, da melhor para a pior.
    pub fn suggestions(&self, word: &str, max: usize) -> Vec<String> {
        match &self.backend {
            Backend::Original { spell, .. } => spell.suggestions(word, self.variant.mask(), max),
            Backend::Free { hun, .. } => {
                let mut out = Vec::new();
                hun.suggest(word, &mut out);
                out.retain(|s| s != word);
                out.truncate(max);
                out
            }
        }
    }

    /// Sinônimos ("Palavras parecidas"), sem a própria palavra; vazio se não houver.
    pub fn synonyms(&self, word: &str) -> Vec<String> {
        match &self.backend {
            Backend::Original { thes, .. } => {
                thes.as_ref().map(|t| t.synonyms(word)).unwrap_or_default()
            }
            Backend::Free { thes, .. } => {
                thes.as_ref().map(|t| t.synonyms(word)).unwrap_or_default()
            }
        }
    }

    /// Anagramas ("Palavras embaralhadas"): palavras com as mesmas letras,
    /// em ordem alfabética, incluindo a própria se existir (`ramo` → amor,
    /// mora, oram, ramo). Só palavras em minúsculas, sem hífen nem ponto. No
    /// livre, por permutação (até 8 letras) verificada pelo Hunspell.
    pub fn anagrams(&self, word: &str) -> Vec<String> {
        let lower = word.to_lowercase();
        if lower.chars().count() < 2 || !lower.chars().all(char::is_alphabetic) {
            return Vec::new();
        }
        let mut out: Vec<String> = match &self.backend {
            Backend::Original { spell, thes } => {
                // o Writer tira os anagramas do índice de formas do thesaurus
                // (`ramo` → amor, mora, oram, ramo; `armo` e `roam`, que só
                // estão no ortográfico, não aparecem); sem ele, do ortográfico
                let index = self.anagrams.get_or_init(|| {
                    let mut m: HashMap<String, Vec<String>> = HashMap::new();
                    let mut add = |w: &str| {
                        if dct::case_of(w) == dct::Case::Lower && w.chars().all(char::is_alphabetic)
                        {
                            m.entry(anagram_key(w)).or_default().push(w.to_string());
                        }
                    };
                    match thes {
                        Some(t) => t.forms().iter().for_each(|f| add(&f.form)),
                        None => spell.words(self.variant.mask()).for_each(&mut add),
                    }
                    m
                });
                index.get(&anagram_key(&lower)).cloned().unwrap_or_default()
            }
            Backend::Free { hun, .. } => {
                let mut chars: Vec<char> = lower.chars().collect();
                if chars.len() > MAX_ANAGRAM_LEN {
                    return Vec::new();
                }
                chars.sort_unstable();
                let mut out = Vec::new();
                loop {
                    let w: String = chars.iter().collect();
                    if hun.check(&w) {
                        out.push(w);
                    }
                    if !next_permutation(&mut chars) {
                        break;
                    }
                }
                out
            }
        };
        out.sort();
        out.dedup();
        out
    }
}

fn anagram_key(w: &str) -> String {
    let mut c: Vec<char> = w.chars().collect();
    c.sort_unstable();
    c.into_iter().collect()
}

fn next_permutation(v: &mut [char]) -> bool {
    let Some(i) = (1..v.len()).rev().find(|&i| v[i - 1] < v[i]) else {
        return false;
    };
    let j = (i..v.len()).rev().find(|&j| v[j] > v[i - 1]).unwrap_or(i);
    v.swap(i - 1, j);
    v[i..].reverse();
    true
}

/// Palavras de um texto como o corretor do Writer as separa: sequências de
/// letras (o hífen e a pontuação separam; `vamo-nos` são duas palavras).
pub fn tokens(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        match (c.is_alphabetic(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push(s..i);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(s..text.len());
    }
    out
}

/// Tesauro MyThes (`.dat` + `.idx`, formato do LibreOffice): lê do disco sob demanda.
struct MyThes {
    dat: PathBuf,
    index: HashMap<String, u64>,
}

impl MyThes {
    fn open(dat: &Path) -> Result<Self, String> {
        let idx = read_text(&dat.with_extension("idx"))?;
        let mut index = HashMap::new();
        for line in idx.lines().skip(2) {
            if let Some((w, off)) = line.rsplit_once('|')
                && let Ok(off) = off.trim().parse()
            {
                index.insert(w.to_string(), off);
            }
        }
        if index.is_empty() {
            return Err(format!("{}: índice vazio", dat.display()));
        }
        Ok(MyThes {
            dat: dat.to_path_buf(),
            index,
        })
    }

    fn meanings(&self, word: &str) -> Vec<Vec<String>> {
        let Some(&off) = self.index.get(word) else {
            return Vec::new();
        };
        let read = || -> std::io::Result<Vec<Vec<String>>> {
            let mut f = BufReader::new(File::open(&self.dat)?);
            f.seek(SeekFrom::Start(off))?;
            let mut line = String::new();
            f.read_line(&mut line)?;
            let n: usize = line
                .trim_end()
                .rsplit('|')
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
            let mut out = Vec::new();
            for _ in 0..n {
                line.clear();
                if f.read_line(&mut line)? == 0 {
                    break;
                }
                out.push(
                    line.trim_end()
                        .split('|')
                        .skip(1)
                        .map(str::to_string)
                        .collect(),
                );
            }
            Ok(out)
        };
        read().unwrap_or_default()
    }

    fn synonyms(&self, word: &str) -> Vec<String> {
        let mut m = self.meanings(word);
        if m.is_empty() {
            m = self.meanings(&word.to_lowercase());
        }
        let lower = word.to_lowercase();
        let mut out: Vec<String> = Vec::new();
        for s in m.into_iter().flatten() {
            if s.to_lowercase() != lower && !out.contains(&s) {
                out.push(s);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separa_como_o_writer() {
        let t = "O gato, vamo-nos já! a.C.";
        let w: Vec<&str> = tokens(t).into_iter().map(|r| &t[r]).collect();
        assert_eq!(w, ["O", "gato", "vamo", "nos", "já", "a", "C"]);
    }

    #[test]
    fn permutacoes() {
        let mut v: Vec<char> = "aab".chars().collect();
        let mut all = vec![v.iter().collect::<String>()];
        while next_permutation(&mut v) {
            all.push(v.iter().collect());
        }
        assert_eq!(all, ["aab", "aba", "baa"]);
    }

    #[test]
    fn mythes_minimo() {
        let dir = tempfile::tempdir().unwrap();
        let dat = dir.path().join("th_t.dat");
        let body = "UTF-8\ncasa|2\n(Sinônimo)lar|lar|casa|morada\n(Sinônimo)botoeira|botoeira\n";
        std::fs::write(&dat, body).unwrap();
        let off = body.find("casa|").unwrap();
        std::fs::write(
            dir.path().join("th_t.idx"),
            format!("UTF-8\n1\ncasa|{off}\n"),
        )
        .unwrap();
        let t = MyThes::open(&dat).unwrap();
        assert_eq!(t.synonyms("casa"), ["lar", "morada", "botoeira"]);
        assert_eq!(t.synonyms("Casa"), ["lar", "morada", "botoeira"]);
        assert!(t.synonyms("xyz").is_empty());
    }
}
