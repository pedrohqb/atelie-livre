//! Dicionários do Creative Writer: `SPELL.DCT` (ortográfico, INSO
//! International CorrectSpell) e `THES.DCT` (sinônimos, Houghton Mifflin).
//!
//! Os dois guardam texto comprimido com uma **gramática de pares** (estilo
//! Re-Pair): um fluxo de códigos de N bits (do bit mais alto para o mais
//! baixo); cada código é um *terminal* (uma cadeia em claro) ou um *par* de
//! dois códigos, expandido recursivamente. A descrição completa do formato
//! está em `docs/formats/dct.md`.
//!
//! O conteúdo é licenciado: o port lê a cópia instalada do usuário e nunca
//! redistribui os dados.

use crate::cp1252;
use crate::error::{ErrorKind, Format, FormatError, Result};
use std::collections::HashMap;

fn err(offset: usize, msg: impl Into<String>) -> FormatError {
    FormatError {
        format: Format::Dct,
        offset,
        kind: ErrorKind::Invalid(msg.into()),
    }
}

fn eof(offset: usize, wanted: usize, available: usize) -> FormatError {
    FormatError {
        format: Format::Dct,
        offset,
        kind: ErrorKind::UnexpectedEof { wanted, available },
    }
}

fn slice(d: &[u8], off: usize, len: usize) -> Result<&[u8]> {
    d.get(off..off.checked_add(len).ok_or_else(|| err(off, "tamanho"))?)
        .ok_or_else(|| eof(off, len, d.len().saturating_sub(off)))
}

fn be16(d: &[u8], off: usize) -> Result<u16> {
    let b = slice(d, off, 2)?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}

fn be32(d: &[u8], off: usize) -> Result<u32> {
    let b = slice(d, off, 4)?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn le16(d: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([d[i], d[i + 1]])
}

// ---------------------------------------------------------------------------
// Gramática de pares
// ---------------------------------------------------------------------------

/// Tabelas de uma gramática de pares.
#[derive(Debug, Clone, Copy)]
enum Tables<'a> {
    /// `SPELL.DCT` e índice do `THES.DCT`: `tab` com 2 bytes por código e
    /// `len` com 1. Terminal `c`: cadeia em `tab[c]` (LE) com `len[c]` bytes.
    /// Par `c` (bytes `lo, hi` de `tab[c]`): esquerdo `hi << 4 | lo >> 4`,
    /// direito `(lo & 0xF) << 8 | len[c]`.
    Packed { tab: &'a [u8], len: &'a [u8] },
    /// Arquivo de entradas do `THES.DCT`: duas tabelas de 2 bytes (LE).
    /// Terminal: deslocamento `a[c]`, comprimento `b[c]`; par: `a[c]`, `b[c]`.
    Wide { a: &'a [u8], b: &'a [u8] },
}

#[derive(Debug, Clone, Copy)]
struct Grammar<'a> {
    bits: u32,
    /// Códigos `<= last_terminal` são terminais.
    last_terminal: u16,
    strings: &'a [u8],
    tables: Tables<'a>,
}

/// Limite de bytes por código expandido (protege contra dados corrompidos).
const MAX_EXPANSION: usize = 1 << 16;

impl Grammar<'_> {
    fn codes(&self) -> usize {
        1 << self.bits
    }

    fn validate(&self) -> Result<()> {
        if !(8..=16).contains(&self.bits) {
            return Err(err(
                0,
                format!("largura de código {} fora do esperado", self.bits),
            ));
        }
        let n = self.codes();
        let ok = match self.tables {
            Tables::Packed { tab, len } => tab.len() >= 2 * n && len.len() >= n,
            Tables::Wide { a, b } => a.len() >= 2 * n && b.len() >= 2 * n,
        };
        if !ok || usize::from(self.last_terminal) >= n {
            return Err(err(
                0,
                "tabelas da gramática menores que o número de códigos",
            ));
        }
        Ok(())
    }

    fn expand(&self, code: u16, out: &mut Vec<u8>) -> Result<()> {
        let start = out.len();
        let mut stack = vec![code];
        while let Some(c) = stack.pop() {
            let ci = usize::from(c);
            if ci >= self.codes() {
                return Err(err(0, format!("código {c} fora da tabela")));
            }
            let terminal = c <= self.last_terminal;
            match self.tables {
                Tables::Packed { tab, len } => {
                    if terminal {
                        let off = usize::from(le16(tab, 2 * ci));
                        out.extend_from_slice(slice(self.strings, off, usize::from(len[ci]))?);
                    } else {
                        let (lo, hi) = (u16::from(tab[2 * ci]), u16::from(tab[2 * ci + 1]));
                        stack.push(((lo & 0xF) << 8) | u16::from(len[ci]));
                        stack.push((hi << 4) | (lo >> 4));
                    }
                }
                Tables::Wide { a, b } => {
                    let (x, y) = (le16(a, 2 * ci), le16(b, 2 * ci));
                    if terminal {
                        out.extend_from_slice(slice(self.strings, x.into(), y.into())?);
                    } else {
                        stack.push(y);
                        stack.push(x);
                    }
                }
            }
            if out.len() - start > MAX_EXPANSION || stack.len() > MAX_EXPANSION {
                return Err(err(0, format!("expansão do código {code} não termina")));
            }
        }
        Ok(())
    }

    /// Expande o fluxo de códigos de `data`; com `stop_at_zero`, o código 0
    /// encerra o fluxo (fim de bloco).
    fn expand_stream(&self, data: &[u8], stop_at_zero: bool) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let bits = self.bits as usize;
        let total = data.len() * 8 / bits;
        let (mut acc, mut have, mut pos) = (0u32, 0usize, 0usize);
        for _ in 0..total {
            while have < bits {
                acc = (acc << 8) | u32::from(data[pos]);
                pos += 1;
                have += 8;
            }
            have -= bits;
            let code = ((acc >> have) & ((1 << bits) - 1)) as u16;
            acc &= (1 << have) - 1;
            if code == 0 && stop_at_zero {
                break;
            }
            self.expand(code, &mut out)?;
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// SPELL.DCT
// ---------------------------------------------------------------------------

/// Variante ortográfica de uma palavra (byte de atributo de cada registro).
pub mod variant {
    /// Grafia brasileira (`ação`, `ônibus`, `idéia`).
    pub const BRASIL: u8 = 0x40;
    /// Grafia europeia (`acção`, `facto`, `ideia`).
    pub const PORTUGAL: u8 = 0x80;
    /// As duas.
    pub const AMBAS: u8 = BRASIL | PORTUGAL;
}

/// Uma palavra do dicionário ortográfico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellWord {
    /// A palavra com a caixa registrada (`casa`, `Brasil`, `EUA`, `a.C.`).
    pub text: String,
    /// Classe da terminação (0 = registro sem classe; 1 = `-mos`; 2 = `-s`,
    /// `-r`, `-z`; 3 = vogal; 4 = nasal `-m`/`-ão`). Corresponde às regras de
    /// ênclise, mas o corretor do Writer não a usa para aceitar palavras.
    pub class: u8,
    /// Máscara de variantes ([`variant`]); a palavra vale se `variants & máscara != 0`.
    pub variants: u8,
}

/// Caixa de uma palavra registrada, que decide que grafias do usuário valem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    /// Toda em minúsculas: vale também Capitalizada e TODA EM MAIÚSCULAS.
    Lower,
    /// Inicial maiúscula (`Brasil`): vale assim ou em maiúsculas.
    Title,
    /// Outra (`EUA`, `a.C.`, `McZee`): vale exata ou em maiúsculas.
    Exact,
}

/// Caixa de `text`.
pub fn case_of(text: &str) -> Case {
    let lower = text.to_lowercase();
    if lower == text {
        return Case::Lower;
    }
    let mut cs = text.chars();
    let first: String = cs.next().into_iter().flat_map(char::to_lowercase).collect();
    if first + cs.as_str() == lower && cs.as_str().to_lowercase() == cs.as_str() {
        Case::Title
    } else {
        Case::Exact
    }
}

/// `w` com a primeira letra maiúscula.
pub fn capitalize(w: &str) -> String {
    let mut cs = w.chars();
    match cs.next() {
        Some(c) => c.to_uppercase().chain(cs).collect(),
        None => String::new(),
    }
}

/// `typed` é uma grafia aceitável da palavra registrada `stored`?
pub fn case_accepts(stored: &str, typed: &str) -> bool {
    if typed == stored {
        return true;
    }
    if typed.to_lowercase() != stored.to_lowercase() {
        return false;
    }
    typed == typed.to_uppercase() || case_of(stored) == Case::Lower && typed == capitalize(stored)
}

/// Parâmetros lidos do cabeçalho do `SPELL.DCT` (inteiros big-endian).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellHeader {
    /// Aviso de copyright (primeira linha).
    pub notice: String,
    /// Data AAMMDD.
    pub date: String,
    /// Idioma (`PORTUGUESE`).
    pub language: String,
    /// Código de idioma do motor (12 = português).
    pub language_code: u16,
    pub block_size: usize,
    pub blocks: usize,
    /// Largura dos códigos, em bits.
    pub code_bits: u32,
    /// Último código terminal.
    pub last_terminal: u16,
}

/// Dicionário ortográfico decodificado.
#[derive(Debug, Clone)]
pub struct SpellDct {
    pub header: SpellHeader,
    words: Vec<SpellWord>,
    /// Pares (de, para) de equivalências fonéticas usados nas sugestões.
    pub phonetic: Vec<(String, String)>,
    /// Palavra em minúsculas → índices em `words`.
    by_lower: HashMap<String, Vec<u32>>,
}

/// Regiões do `SPELL.DCT` depois dos blocos, na ordem em que o motor lê
/// (campos do cabeçalho em 0x1E8..0x20B).
const SPELL_REGIONS: [(usize, &str); 11] = [
    (0x1EC, "tabela auxiliar"),
    (0x206, "cadeias"),
    (0x208, "tab16"),
    (0x20A, "len"),
    (0x1EA, "índice de blocos"),
    (0x200, "n-gramas"),
    (0x1EE, "hash"),
    (0x1F0, "vazia"),
    (0x1FE, "fonética"),
    (0x202, "vazia"),
    (0x204, "classes"),
];

impl SpellDct {
    /// Lê um `SPELL.DCT`.
    pub fn parse(d: &[u8]) -> Result<Self> {
        if d.first() != Some(&b'C') || be16(d, 0x180)? != 6 {
            return Err(FormatError {
                format: Format::Dct,
                offset: 0,
                kind: ErrorKind::BadMagic {
                    expected: "SPELL.DCT (INSO, cabeçalho 6)",
                },
            });
        }
        let text = |o: usize, n: usize| -> Result<String> {
            let b = slice(d, o, n)?;
            let end = b.iter().position(|&c| c == 0).unwrap_or(n);
            Ok(cp1252::decode(&b[..end]))
        };
        let notice_end = d.iter().position(|&c| c == 0x1A || c == b'\r').unwrap_or(0);
        let block_size = usize::from(be16(d, 0x188)?);
        let skip = usize::from(be16(d, 0x18A)?);
        let blocks = usize::from(be16(d, 0x1F6)?);
        let bits = u32::from(be16(d, 0x20C)?);
        if block_size == 0 || blocks == 0 {
            return Err(err(0x188, "sem blocos"));
        }
        // regiões depois dos blocos, em sequência
        let mut off = block_size * (skip + blocks);
        let mut region = HashMap::new();
        for (field, name) in SPELL_REGIONS {
            let len = usize::from(be16(d, field)?);
            region.insert(name, (off, len));
            off += len;
        }
        if off != d.len() {
            return Err(err(
                off,
                format!("regiões somam {off} bytes, arquivo tem {}", d.len()),
            ));
        }
        let get = |name: &str| -> Result<&[u8]> {
            let (o, n) = region[name];
            slice(d, o, n)
        };
        let tab = get("tab16")?;
        let pairs = be16(tab, 0)?;
        let last_terminal = (1u32 << bits)
            .checked_sub(u32::from(pairs) + 1)
            .ok_or_else(|| err(region["tab16"].0, "número de pares inválido"))?;
        let g = Grammar {
            bits,
            last_terminal: last_terminal as u16,
            strings: get("cadeias")?,
            tables: Tables::Packed {
                tab,
                len: get("len")?,
            },
        };
        g.validate()?;
        let header = SpellHeader {
            notice: cp1252::decode(&d[..notice_end]),
            date: text(0x18C, 8)?,
            language: text(0x19F, 32)?,
            language_code: be16(d, 0x182)?,
            block_size,
            blocks,
            code_bits: bits,
            last_terminal: last_terminal as u16,
        };
        let mut words = Vec::new();
        for k in 0..blocks {
            let o = block_size * (skip + k);
            let stream = g
                .expand_stream(slice(d, o, block_size)?, true)
                .map_err(|e| FormatError { offset: o, ..e })?;
            parse_spell_records(&stream, &mut words).map_err(|e| FormatError { offset: o, ..e })?;
        }
        let phonetic = parse_phonetic(get("fonética")?);
        let mut by_lower: HashMap<String, Vec<u32>> = HashMap::with_capacity(words.len());
        for (i, w) in words.iter().enumerate() {
            by_lower
                .entry(w.text.to_lowercase())
                .or_default()
                .push(i as u32);
        }
        Ok(SpellDct {
            header,
            words,
            phonetic,
            by_lower,
        })
    }

    /// Todos os registros, na ordem do arquivo (alfabética).
    pub fn entries(&self) -> &[SpellWord] {
        &self.words
    }

    /// Palavras válidas para a máscara `variants` ([`variant`]), na ordem do arquivo.
    pub fn words(&self, variants: u8) -> impl Iterator<Item = &str> {
        self.words
            .iter()
            .filter(move |w| w.variants & variants != 0)
            .map(|w| w.text.as_str())
    }

    /// Registros cuja forma em minúsculas é `lower`, válidos para `variants`.
    fn lookup(&self, lower: &str, variants: u8) -> impl Iterator<Item = (u32, &SpellWord)> {
        self.by_lower
            .get(lower)
            .into_iter()
            .flatten()
            .map(|&i| (i, &self.words[i as usize]))
            .filter(move |(_, w)| w.variants & variants != 0)
    }

    /// `word` está escrita corretamente? Segue o corretor do Writer: a caixa
    /// registrada vale (`brasil` não, `Brasil` e `BRASIL` sim) e, se a forma
    /// com hífen não estiver no dicionário, cada parte é verificada (o Writer
    /// separa as palavras no hífen: `encontrei-o` passa, `vamo-nos` não).
    /// Todas as formas flexionadas são registros explícitos.
    pub fn contains(&self, word: &str, variants: u8) -> bool {
        if word.is_empty() {
            return false;
        }
        if self
            .lookup(&word.to_lowercase(), variants)
            .any(|(_, w)| case_accepts(&w.text, word))
        {
            return true;
        }
        word.contains('-')
            && word
                .split('-')
                .all(|p| p.is_empty() || self.contains(p, variants))
    }

    /// Sugestões para `word`, da mais provável para a menos: a mesma palavra
    /// com a caixa registrada; trocas da tabela fonética (`pexe` → `pese`);
    /// uma edição (tirar, pôr, trocar ou inverter uma letra); e, se houver
    /// poucas, palavras a duas edições. Empates seguem a ordem do dicionário.
    /// O ranking do original (modelo de n-gramas) não foi reproduzido; veja
    /// `docs/formats/dct.md`.
    pub fn suggestions(&self, word: &str, variants: u8, max: usize) -> Vec<String> {
        let lower = word.to_lowercase();
        let mut found: HashMap<u32, u32> = HashMap::new();
        let add = |cand: &str, cost: u32, found: &mut HashMap<u32, u32>| {
            for (i, _) in self.lookup(cand, variants) {
                let e = found.entry(i).or_insert(cost);
                *e = (*e).min(cost);
            }
        };
        add(&lower, 0, &mut found);
        let chars: Vec<char> = lower.chars().collect();
        // equivalências fonéticas
        for (from, to) in &self.phonetic {
            for (pos, _) in lower.match_indices(from.as_str()) {
                let cand = format!("{}{}{}", &lower[..pos], to, &lower[pos + from.len()..]);
                add(&cand, 1, &mut found);
            }
        }
        // uma edição
        for cand in edits1(&chars) {
            add(&cand, 2, &mut found);
        }
        if found.len() < 3 {
            let n = chars.len();
            for (i, w) in self.words.iter().enumerate() {
                if w.variants & variants == 0 || found.contains_key(&(i as u32)) {
                    continue;
                }
                let wl: Vec<char> = w.text.to_lowercase().chars().collect();
                if wl.len().abs_diff(n) > 2 {
                    continue;
                }
                if let Some(dist) = bounded_distance(&chars, &wl, 2) {
                    found.insert(i as u32, 2 + dist);
                }
            }
        }
        let mut ranked: Vec<(u32, u32)> = found.into_iter().map(|(i, c)| (c, i)).collect();
        ranked.sort_unstable();
        let mut out: Vec<String> = Vec::new();
        for (_, i) in ranked {
            let stored = &self.words[i as usize].text;
            let shown = match_case(stored, word);
            if shown != word && !out.contains(&shown) {
                out.push(shown);
            }
            if out.len() >= max {
                break;
            }
        }
        out
    }
}

/// Ajusta a caixa da sugestão `stored` à da palavra digitada.
fn match_case(stored: &str, typed: &str) -> String {
    if case_of(stored) != Case::Lower {
        return stored.to_string();
    }
    if typed.chars().count() > 1 && typed == typed.to_uppercase() && typed != typed.to_lowercase() {
        stored.to_uppercase()
    } else if case_of(typed) == Case::Title {
        capitalize(stored)
    } else {
        stored.to_string()
    }
}

/// Letras usadas para inserir e trocar nas sugestões.
const ALPHABET: &str = "aeosrindmutclpvgqbfhzjxkwyáàâãéêíóôõúüç-";

fn edits1(w: &[char]) -> Vec<String> {
    let s = |v: &[char]| v.iter().collect::<String>();
    let mut out = Vec::new();
    for i in 0..=w.len() {
        if i < w.len() {
            let mut v = w.to_vec();
            v.remove(i);
            out.push(s(&v));
            if i + 1 < w.len() {
                let mut v = w.to_vec();
                v.swap(i, i + 1);
                out.push(s(&v));
            }
        }
        for c in ALPHABET.chars() {
            let mut v = w.to_vec();
            v.insert(i, c);
            out.push(s(&v));
            if i < w.len() && w[i] != c {
                let mut v = w.to_vec();
                v[i] = c;
                out.push(s(&v));
            }
        }
    }
    out
}

/// Distância de edição (com transposição) entre `a` e `b`, se `<= max`.
fn bounded_distance(a: &[char], b: &[char], max: u32) -> Option<u32> {
    let (n, m) = (a.len(), b.len());
    let mut prev2 = vec![0u32; m + 1];
    let mut prev: Vec<u32> = (0..=m as u32).collect();
    for i in 1..=n {
        let mut cur = vec![i as u32; m + 1];
        let mut best = cur[0];
        for j in 1..=m {
            let cost = u32::from(a[i - 1] != b[j - 1]);
            let mut v = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(prev2[j - 2] + 1);
            }
            cur[j] = v;
            best = best.min(v);
        }
        if best > max {
            return None;
        }
        prev2 = std::mem::replace(&mut prev, cur);
    }
    (prev[m] <= max).then_some(prev[m])
}

/// Registros de um bloco expandido:
/// `[k | caixa<<6] [letras ≥ 0x04] (01 | 02 classe) [variantes]`.
/// `k` = letras a tirar do fim da palavra anterior; bit 7 = inicial
/// maiúscula; bit 6 = letras maiúsculas valem como estão (senão, a parte
/// herdada perde as maiúsculas). O bloco termina com um cabeçalho solto.
fn parse_spell_records(s: &[u8], out: &mut Vec<SpellWord>) -> Result<()> {
    let mut chars: Vec<char> = Vec::new();
    let mut upper: Vec<bool> = Vec::new();
    let mut mixed = false;
    let mut i = 0;
    while i < s.len() {
        let head = s[i];
        i += 1;
        let start = i;
        while i < s.len() && s[i] >= 4 {
            i += 1;
        }
        if i >= s.len() {
            break; // cabeçalho solto no fim do bloco
        }
        let cut = usize::from(head & 0x3F);
        let keep = chars
            .len()
            .checked_sub(cut)
            .ok_or_else(|| err(start, "prefixo maior que a palavra anterior"))?;
        chars.truncate(keep);
        upper.truncate(keep);
        if !mixed {
            upper.iter_mut().for_each(|u| *u = false);
        }
        if head & 0xC0 == 0 {
            mixed = false;
        }
        if head & 0x40 != 0 {
            mixed = true;
        }
        for c in cp1252::decode(&s[start..i]).chars() {
            let low: Vec<char> = c.to_lowercase().collect();
            if mixed && low.len() == 1 && low[0] != c {
                chars.push(low[0]);
                upper.push(true);
            } else {
                chars.push(c);
                upper.push(false);
            }
        }
        if let Some(u) = upper.first_mut() {
            *u = head & 0x80 != 0;
        }
        let term = s[i];
        i += 1;
        let class = match term {
            1 => 0,
            2 => {
                let c = *s.get(i).ok_or_else(|| eof(i, 1, 0))?;
                i += 1;
                c
            }
            _ => return Err(err(i, format!("terminador {term:#x}"))),
        };
        let variants = s.get(i).copied().unwrap_or(variant::AMBAS);
        i += 1;
        let text = chars
            .iter()
            .zip(&upper)
            .flat_map(|(&c, &u)| {
                let v: Vec<char> = if u {
                    c.to_uppercase().collect()
                } else {
                    vec![c]
                };
                v
            })
            .collect();
        out.push(SpellWord {
            text,
            class,
            variants,
        });
    }
    Ok(())
}

/// Tabela fonética: 8 bytes de cabeçalho, pares de registros de 4 bytes
/// (texto completado com espaços) e `FF` no fim.
fn parse_phonetic(d: &[u8]) -> Vec<(String, String)> {
    let body = d.get(8..).unwrap_or_default();
    body.chunks_exact(8)
        .take_while(|p| p[0] != 0xFF)
        .map(|p| {
            let f = |b: &[u8]| cp1252::decode(b).trim().to_string();
            (f(&p[..4]), f(&p[4..]))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// THES.DCT
// ---------------------------------------------------------------------------

/// Uma entrada do thesaurus: palavra-título, classe gramatical e sinônimos
/// (o primeiro sinônimo é a própria palavra-título).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThesEntry {
    pub headword: String,
    /// Código da classe gramatical (`<PS>`): 01 substantivo, 11 verbo,
    /// 21/25 adjetivo, 31 advérbio, 41 preposição, 44 conjunção…
    pub pos: u8,
    pub synonyms: Vec<String>,
}

/// Nome da classe gramatical de um código `<PS>` (pela dezena).
pub fn pos_name(pos: u8) -> &'static str {
    match pos / 10 {
        0 => "substantivo",
        1 => "verbo",
        2 => "adjetivo",
        3 => "advérbio",
        4 => match pos {
            41 => "preposição",
            44 => "conjunção",
            _ => "outra",
        },
        _ => "outra",
    }
}

/// Um registro do índice de formas do thesaurus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThesForm {
    pub form: String,
    /// Número da entrada (base 1), se a forma tiver entrada própria.
    pub entry: Option<u32>,
    /// Código `<PS>` (0 nas formas flexionadas).
    pub pos: u8,
    /// Lema (forma de dicionário), nas formas flexionadas.
    pub lemma: Option<String>,
}

/// Thesaurus decodificado.
#[derive(Debug, Clone)]
pub struct Thesaurus {
    entries: Vec<ThesEntry>,
    forms: Vec<ThesForm>,
    by_lower: HashMap<String, Vec<u32>>,
}

struct Section {
    header: usize,
    data: usize,
    end: usize,
    dir: HashMap<u16, (usize, u16)>,
}

impl Section {
    fn read(d: &[u8], header: usize, data: usize, end: usize) -> Result<Self> {
        let n = usize::from(be16(d, header + 0x50)?);
        let at = header + be32(d, header + 0x52)? as usize;
        let mut dir = HashMap::new();
        for k in 0..n {
            let e = at + 16 * k;
            let id = be16(d, e)?;
            let off = header + be32(d, e + 2)? as usize;
            let count = be16(d, e + 10)?;
            let len = be32(d, e + 12)? as usize;
            slice(d, off, len)?;
            dir.insert(id, (off, count));
            dir.entry(id | 0x8000).or_insert((len, 0));
        }
        Ok(Section {
            header,
            data,
            end,
            dir,
        })
    }

    fn table<'a>(&self, d: &'a [u8], id: u16) -> Result<&'a [u8]> {
        let (off, _) = *self
            .dir
            .get(&id)
            .ok_or_else(|| err(self.header, format!("seção {id} ausente")))?;
        let (len, _) = self.dir[&(id | 0x8000)];
        slice(d, off, len)
    }

    fn count(&self, id: u16) -> Result<u16> {
        self.dir
            .get(&id)
            .map(|x| x.1)
            .ok_or_else(|| err(self.header, format!("seção {id} ausente")))
    }
}

impl Thesaurus {
    /// Lê um `THES.DCT`.
    pub fn parse(d: &[u8]) -> Result<Self> {
        // diretório do arquivo: u16 em 0x316, u32 em 0x318 → registros de 12
        // bytes (tipo, tipo, cabeçalho, início dos dados)
        let n = usize::from(be16(d, 0x316)?);
        let at = be32(d, 0x318)? as usize;
        if !(1..=8).contains(&n) {
            return Err(FormatError {
                format: Format::Dct,
                offset: 0x316,
                kind: ErrorKind::BadMagic {
                    expected: "THES.DCT (diretório de seções)",
                },
            });
        }
        let mut secs = Vec::new();
        for k in 0..n {
            let e = at + 12 * k;
            secs.push((
                be16(d, e)?,
                be32(d, e + 4)? as usize,
                be32(d, e + 8)? as usize,
            ));
        }
        let end_of = |h: usize| {
            secs.iter()
                .map(|s| s.1)
                .filter(|&x| x > h)
                .min()
                .unwrap_or(d.len())
        };
        let find = |t: u16| {
            secs.iter()
                .find(|s| s.0 == t)
                .copied()
                .ok_or_else(|| err(at, format!("seção do tipo {t} ausente")))
        };
        let (_, eh, ed) = find(1)?;
        let entry_sec = Section::read(d, eh, ed, end_of(eh))?;
        let (_, ih, id) = find(2)?;
        let index_sec = Section::read(d, ih, id, d.len())?;
        let entries = parse_entries(d, &entry_sec)?;
        let forms = parse_forms(d, &index_sec)?;
        let mut by_lower: HashMap<String, Vec<u32>> = HashMap::new();
        for (i, f) in forms.iter().enumerate() {
            if let Some(e) = f.entry
                && (e == 0 || e as usize > entries.len())
            {
                return Err(err(ih, format!("forma {:?} aponta a entrada {e}", f.form)));
            }
            by_lower
                .entry(f.form.to_lowercase())
                .or_default()
                .push(i as u32);
        }
        Ok(Thesaurus {
            entries,
            forms,
            by_lower,
        })
    }

    /// Entradas na ordem do arquivo (a entrada `n` do índice é `entries()[n - 1]`).
    pub fn entries(&self) -> &[ThesEntry] {
        &self.entries
    }

    /// Registros do índice de formas.
    pub fn forms(&self) -> &[ThesForm] {
        &self.forms
    }

    fn direct(&self, lower: &str) -> Vec<&ThesForm> {
        self.by_lower
            .get(lower)
            .into_iter()
            .flatten()
            .map(|&i| &self.forms[i as usize])
            .collect()
    }

    /// Entradas de `word`: as da própria forma; se não houver, as do lema
    /// (`corri` → `correr`). Sem distinção de maiúsculas.
    pub fn lookup(&self, word: &str) -> Vec<&ThesEntry> {
        let forms = self.direct(&word.to_lowercase());
        let mut ids: Vec<u32> = forms.iter().filter_map(|f| f.entry).collect();
        if ids.is_empty() {
            for f in &forms {
                if let Some(l) = &f.lemma {
                    ids.extend(
                        self.direct(&l.to_lowercase())
                            .iter()
                            .filter_map(|f| f.entry),
                    );
                }
            }
        }
        let mut seen = Vec::new();
        ids.retain(|i| {
            let new = !seen.contains(i);
            seen.push(*i);
            new
        });
        ids.iter().map(|&i| &self.entries[i as usize - 1]).collect()
    }

    /// Sinônimos de `word`, na ordem das entradas, sem repetir e sem a
    /// própria palavra (`casa` → `botoeira`, `domicílio`, `edifício`…).
    pub fn synonyms(&self, word: &str) -> Vec<String> {
        let lower = word.to_lowercase();
        let mut out: Vec<String> = Vec::new();
        for e in self.lookup(word) {
            for s in &e.synonyms {
                if s.to_lowercase() != lower && !out.contains(s) {
                    out.push(s.clone());
                }
            }
        }
        out
    }
}

/// Arquivo de entradas: gramática "larga" de N bits (cabeçalho +0x76),
/// tabelas nas seções 20 e 30, cadeias na 60; o fluxo começa no início dos
/// dados e cada entrada é `FE FE FE FE <WD>título FF <PS>nn FF <SY>s1<D1>s2…</SY> FF`.
fn parse_entries(d: &[u8], sec: &Section) -> Result<Vec<ThesEntry>> {
    let a = sec.table(d, 20)?;
    let bits = u32::from(be16(d, sec.header + 0x76)?);
    if a.len() < 2 || !(8..=16).contains(&bits) {
        return Err(err(sec.header, "tabelas de entradas inválidas"));
    }
    let pairs = u32::from(le16(a, 0));
    let last = (1u32 << bits)
        .checked_sub(pairs + 1)
        .ok_or_else(|| err(sec.header, "pares"))?;
    let g = Grammar {
        bits,
        last_terminal: last as u16,
        strings: sec.table(d, 60)?,
        tables: Tables::Wide {
            a,
            b: sec.table(d, 30)?,
        },
    };
    g.validate()?;
    let stream = g.expand_stream(slice(d, sec.data, sec.end - sec.data)?, false)?;
    let mut out = Vec::new();
    let mut parts = Vec::new();
    let mut rest = &stream[..];
    while let Some(p) = rest.windows(4).position(|w| w == [0xFE; 4]) {
        parts.push(&rest[..p]);
        rest = &rest[p + 4..];
    }
    parts.push(rest);
    for part in parts.into_iter().filter(|p| !p.is_empty()) {
        let Some(rest) = part.strip_prefix(b"<WD>") else {
            break; // preenchimento depois da última entrada
        };
        let text = cp1252::decode(rest);
        let field = |tag: &str, end: &str| -> Option<String> {
            let a = text.find(tag)? + tag.len();
            let z = text[a..].find(end).map_or(text.len(), |z| a + z);
            Some(text[a..z].to_string())
        };
        let headword = text.split('\u{ff}').next().unwrap_or_default().to_string();
        let pos = field("<PS>", "\u{ff}")
            .and_then(|p| p.trim().parse().ok())
            .ok_or_else(|| err(sec.data, format!("entrada {headword:?} sem <PS>")))?;
        let sy = field("<SY>", "</SY>").unwrap_or_default();
        let synonyms = split_d_tags(&sy);
        out.push(ThesEntry {
            headword,
            pos,
            synonyms,
        });
    }
    Ok(out)
}

/// Separa a lista de sinônimos nas marcas `<Dn>`.
fn split_d_tags(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s;
    loop {
        let next = rest.find("<D").filter(|&p| {
            let b = rest.as_bytes();
            b.get(p + 2).is_some_and(u8::is_ascii_digit) && b.get(p + 3) == Some(&b'>')
        });
        match next {
            Some(p) => {
                out.push(rest[..p].to_string());
                rest = &rest[p + 4..];
            }
            None => {
                out.push(rest.to_string());
                break;
            }
        }
    }
    out.retain(|x| !x.is_empty());
    out
}

/// Índice de formas: blocos (tamanho no cabeçalho +0x8C, quantidade = itens
/// da seção 230) com a gramática "empacotada" (seções 200, 210, 220; N em
/// +0x94; último terminal = contagem da seção 200). Registro:
/// `[k|caixa<<6] letras 0A 00 80 [delta i24 BE] [ps] letras-do-lema [corte] 00`.
fn parse_forms(d: &[u8], sec: &Section) -> Result<Vec<ThesForm>> {
    let g = Grammar {
        bits: u32::from(be16(d, sec.header + 0x94)?),
        last_terminal: sec.count(200)?,
        strings: sec.table(d, 200)?,
        tables: Tables::Packed {
            tab: sec.table(d, 210)?,
            len: sec.table(d, 220)?,
        },
    };
    g.validate()?;
    let bsize = usize::from(be16(d, sec.header + 0x8C)?);
    let blocks = usize::from(sec.count(230)?);
    let mut out = Vec::new();
    for k in 0..blocks {
        let o = sec.data + bsize * k;
        let n = bsize.min(d.len().saturating_sub(o));
        let s = g.expand_stream(slice(d, o, n)?, true)?;
        parse_form_records(&s, &mut out);
    }
    Ok(out)
}

fn parse_form_records(s: &[u8], out: &mut Vec<ThesForm>) {
    let mut raw: Vec<char> = Vec::new();
    let mut acc: i32 = 0;
    let mut i = 0;
    let letters = |i: &mut usize| {
        let st = *i;
        while *i < s.len() && s[*i] >= 0x20 {
            *i += 1;
        }
        cp1252::decode(&s[st..*i])
    };
    while i < s.len() {
        let head = s[i];
        i += 1;
        let add = letters(&mut i);
        if i + 7 > s.len() || s[i] != 0x0A {
            break;
        }
        let v = u32::from_be_bytes([0, s[i + 3], s[i + 4], s[i + 5]]);
        let pos = s[i + 6];
        i += 7;
        let keep = raw.len().saturating_sub(usize::from(head & 0x3F));
        raw.truncate(keep);
        raw.extend(add.chars());
        let lem = letters(&mut i);
        if i + 2 > s.len() {
            break;
        }
        let lcut = usize::from(s[i]);
        i += 2;
        let raw_s: String = raw.iter().collect();
        let form = if head & 0x80 != 0 {
            capitalize(&raw_s)
        } else {
            raw_s.clone()
        };
        let lemma = (lcut > 0 || !lem.is_empty()).then(|| {
            let base: String = form
                .chars()
                .take(form.chars().count().saturating_sub(lcut))
                .collect();
            base + &lem
        });
        let entry = if v == 0x0F_FFFF {
            acc = 0;
            None
        } else {
            let delta = ((v << 8) as i32) >> 8;
            acc += delta;
            (acc > 0).then_some(acc as u32)
        };
        out.push(ThesForm {
            form,
            entry,
            pos: if entry.is_some() { pos } else { 0 },
            lemma,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caixa() {
        assert_eq!(case_of("casa"), Case::Lower);
        assert_eq!(case_of("Brasil"), Case::Title);
        assert_eq!(case_of("EUA"), Case::Exact);
        assert_eq!(case_of("a.C."), Case::Exact);
        assert!(case_accepts("casa", "casa"));
        assert!(case_accepts("casa", "Casa"));
        assert!(case_accepts("casa", "CASA"));
        assert!(!case_accepts("casa", "cAsa"));
        assert!(case_accepts("Brasil", "Brasil"));
        assert!(case_accepts("Brasil", "BRASIL"));
        assert!(!case_accepts("Brasil", "brasil"));
        assert!(case_accepts("EUA", "EUA"));
        assert!(!case_accepts("EUA", "Eua"));
        assert!(case_accepts("ação", "AÇÃO"));
    }

    #[test]
    fn registros_ortograficos() {
        // "abafado" (classe 3), "abafados" (k=0, +s, classe 2), "Brasil" (só BR)
        let mut s = Vec::new();
        s.extend_from_slice(b"\x00abafado\x02\x03\xc0");
        s.extend_from_slice(b"\x00s\x02\x02\xc0");
        s.extend_from_slice(b"\x88Brasil\x01\x40");
        s.push(0x02); // cabeçalho solto do fim do bloco
        let mut w = Vec::new();
        parse_spell_records(&s, &mut w).unwrap();
        let t: Vec<_> = w
            .iter()
            .map(|x| (x.text.as_str(), x.class, x.variants))
            .collect();
        assert_eq!(
            t,
            [
                ("abafado", 3, 0xC0),
                ("abafados", 2, 0xC0),
                ("Brasil", 0, 0x40)
            ]
        );
    }

    #[test]
    fn gramatica_empacotada() {
        // 4 bits: terminais 1 = "ab", 2 = "c"; par 15 = (1, 2)
        let strings = b"abc";
        let mut tab = vec![0u8; 32];
        let mut len = vec![0u8; 16];
        tab[2] = 0;
        len[1] = 2;
        tab[4] = 2;
        len[2] = 1;
        // par 15: esquerdo = hi<<4 | lo>>4 = 1 ; direito = (lo&0xF)<<8 | len = 2
        tab[30] = 0x10;
        tab[31] = 0x00;
        len[15] = 2;
        let g = Grammar {
            bits: 8,
            last_terminal: 2,
            strings,
            tables: Tables::Packed {
                tab: &tab,
                len: &len,
            },
        };
        // validate exige tabelas do tamanho de 2^bits; teste só o expand
        let mut out = Vec::new();
        g.expand(15, &mut out).unwrap();
        assert_eq!(out, b"abc");
    }

    #[test]
    fn marcas_d() {
        assert_eq!(split_d_tags("a<D1>b<D2>c"), ["a", "b", "c"]);
        assert_eq!(split_d_tags("a<Dx>b"), ["a<Dx>b"]);
    }

    #[test]
    fn distancia() {
        let c = |s: &str| s.chars().collect::<Vec<_>>();
        assert_eq!(bounded_distance(&c("gatu"), &c("gato"), 2), Some(1));
        assert_eq!(bounded_distance(&c("abc"), &c("bac"), 2), Some(1));
        assert_eq!(bounded_distance(&c("abc"), &c("xyz"), 2), None);
    }

    #[test]
    fn registros_do_indice() {
        let mut s = Vec::new();
        s.extend_from_slice(b"\x00casa\x0a\x00\x80\x00\x00\x05\x01\x00\x00");
        s.extend_from_slice(b"\x00s\x0a\x00\x80\x0f\xff\xff\x00\x01\x00");
        s.extend_from_slice(b"\x00\x0a\x00\x80\x00\x00\x02\x01\x00\x00");
        let mut f = Vec::new();
        parse_form_records(&s, &mut f);
        assert_eq!(f[0].entry, Some(5));
        assert_eq!(f[1].form, "casas");
        assert_eq!(f[1].lemma.as_deref(), Some("casa"));
        assert_eq!(f[1].entry, None);
        // depois de uma forma flexionada o acumulador recomeça do zero
        assert_eq!(f[2].form, "casas");
        assert_eq!(f[2].entry, Some(2));
    }
}
