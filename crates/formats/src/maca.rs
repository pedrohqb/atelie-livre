//! Tabela palavra → figura da Maçã (Coloca-figuras, Transformadores do
//! Creative Writer), guardada no `WRITER.PIC` entre o fim do índice `GLOB`
//! e a primeira imagem — uma região que o índice não descreve.
//!
//! Formato em `docs/formats/maca.md` (little-endian; `R` = fim do índice =
//! `18 + 6 × entradas`):
//!
//! ```text
//! R+0       u16  N                    baldes = registros (1866 no PT-BR)
//! R+2       N × {u16 off, u16 cnt}    baldes da tabela de hash
//! R+2+4N    N × {u16 fig, u16 str}    registros, na ordem dos baldes
//! R+2+8N    N × palavra\0             latin1, minúsculas (str conta de R)
//! ```
//!
//! `fig` é par e `n = fig / 2` (1..255) é a figura: o mesmo byte que o
//! `TEXT` do `.MAX` guarda como caractere-rébus. O balde de uma palavra é
//! `hash(palavra) mod N`, com `h = rol16(h, 5) + byte` para cada byte. O
//! primeiro balde não vazio grava `off = 0`; por isso os registros são
//! percorridos pelas contagens, não pelos deslocamentos.

use crate::{ErrorKind, Format, FormatError, Reader, Result};

const MAGIC: &[u8; 12] = b"GLOBGLOBGLOB";

/// Hash de uma palavra (bytes latin1): `h = rol16(h, 5) + c`.
pub fn hash(word: &[u8]) -> u16 {
    word.iter()
        .fold(0u16, |h, &c| h.rotate_left(5).wrapping_add(u16::from(c)))
}

/// Id no `WRITER.PIC` da figura colorida (4 bits, ≈30×28) de `n`: `2n − 1`,
/// salvo `n = 1` (o avião), que é a figura 0 (o id 1 não existe).
pub fn color_id(n: u8) -> Option<u16> {
    match n {
        0 => None,
        1 => Some(0),
        _ => Some(2 * u16::from(n) - 1),
    }
}

/// Id da versão em 1 bit (preto e branco, não é máscara) de `n`: `2n`.
pub fn mono_id(n: u8) -> Option<u16> {
    (n != 0).then(|| 2 * u16::from(n))
}

/// A tabela inteira, como está no arquivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Maca {
    /// Registros `(palavra, figura)` de cada balde, na ordem do arquivo.
    buckets: Vec<Vec<(String, u8)>>,
}

fn invalid(r: &Reader, msg: String) -> FormatError {
    r.error(ErrorKind::Invalid(msg))
}

impl Maca {
    /// Lê a tabela de um `WRITER.PIC` inteiro. Erro se não for um `GLOB` ou
    /// se não houver tabela entre o índice e a primeira imagem (o
    /// `ARTIST.PIC` não tem).
    pub fn read(pic: &[u8]) -> Result<Maca> {
        let mut r = Reader::new(pic, Format::Glob);
        if r.bytes(12).ok() != Some(MAGIC.as_slice()) {
            return Err(Reader::new(pic, Format::Glob).error(ErrorKind::BadMagic {
                expected: "GLOBGLOBGLOB",
            }));
        }
        r.skip(4)?;
        let entries = usize::from(r.u16_le()?);
        let mut first_image = usize::MAX;
        for _ in 0..entries {
            r.skip(2)?;
            first_image = first_image.min(r.u32_le()? as usize);
        }
        let base = r.pos();
        if first_image <= base + 2 || first_image > pic.len() {
            return Err(invalid(&r, "sem tabela da Maçã".into()));
        }
        let region = &pic[base..first_image];
        let mut r = Reader::new(region, Format::Glob);
        let n = usize::from(r.u16_le()?);
        let mut counts = Vec::with_capacity(n);
        for _ in 0..n {
            r.skip(2)?;
            counts.push(usize::from(r.u16_le()?));
        }
        if counts.iter().sum::<usize>() != n {
            return Err(invalid(&r, "soma das contagens dos baldes ≠ N".into()));
        }
        let word = |off: usize| -> Option<String> {
            let s = region.get(off..)?;
            let end = s.iter().position(|&b| b == 0)?;
            Some(s[..end].iter().map(|&b| char::from(b)).collect())
        };
        let mut buckets = Vec::with_capacity(n);
        for (b, &cnt) in counts.iter().enumerate() {
            let mut items = Vec::with_capacity(cnt);
            for _ in 0..cnt {
                let fig = r.u16_le()?;
                let off = usize::from(r.u16_le()?);
                if fig % 2 != 0 || fig == 0 || fig > 510 {
                    return Err(invalid(&r, format!("figura {fig} inválida")));
                }
                let w = word(off).ok_or_else(|| invalid(&r, format!("palavra em 0x{off:x}")))?;
                let bytes: Vec<u8> = w.chars().map(|c| c as u8).collect();
                if usize::from(hash(&bytes)) % n != b {
                    return Err(invalid(&r, format!("{w:?} fora do seu balde")));
                }
                items.push((w, (fig / 2) as u8));
            }
            buckets.push(items);
        }
        Ok(Maca { buckets })
    }

    /// Número de registros (= de baldes).
    pub fn len(&self) -> usize {
        self.buckets.iter().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Todas as figuras da palavra, na ordem do arquivo. A comparação é em
    /// minúsculas; palavra com caractere fora do latin1 não tem figura.
    pub fn lookup_all(&self, word: &str) -> Vec<u8> {
        let lower = word.to_lowercase();
        let Some(bytes) = lower
            .chars()
            .map(|c| u8::try_from(u32::from(c)).ok())
            .collect::<Option<Vec<u8>>>()
        else {
            return Vec::new();
        };
        if self.buckets.is_empty() || bytes.is_empty() {
            return Vec::new();
        }
        let b = usize::from(hash(&bytes)) % self.buckets.len();
        self.buckets[b]
            .iter()
            .filter(|(w, _)| *w == lower)
            .map(|&(_, n)| n)
            .collect()
    }

    /// A figura que o Coloca-figuras usa: a primeira do balde.
    pub fn lookup(&self, word: &str) -> Option<u8> {
        self.lookup_all(word).first().copied()
    }

    /// `(balde, palavra, figura)` de todos os registros.
    pub fn entries(&self) -> impl Iterator<Item = (usize, &str, u8)> {
        self.buckets
            .iter()
            .enumerate()
            .flat_map(|(b, v)| v.iter().map(move |(w, n)| (b, w.as_str(), *n)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Um `.PIC` com um índice de `images` entradas e a tabela das
    /// `words` no meio, como o `WRITER.PIC`.
    fn build(words: &[(&str, u8)], images: u16) -> Vec<u8> {
        let n = words.len();
        let mut buckets: Vec<Vec<(&str, u8)>> = vec![Vec::new(); n];
        for &(w, f) in words {
            let bytes: Vec<u8> = w.chars().map(|c| c as u8).collect();
            buckets[usize::from(hash(&bytes)) % n].push((w, f));
        }
        let mut region = (n as u16).to_le_bytes().to_vec();
        let recs = 2 + 4 * n;
        let strs = recs + 4 * n;
        let mut k = 0;
        for b in &buckets {
            region.extend_from_slice(&((recs + 4 * k) as u16).to_le_bytes());
            region.extend_from_slice(&(b.len() as u16).to_le_bytes());
            k += b.len();
        }
        let mut text = Vec::new();
        for b in &buckets {
            for &(w, f) in b {
                region.extend_from_slice(&(u16::from(f) * 2).to_le_bytes());
                region.extend_from_slice(&((strs + text.len()) as u16).to_le_bytes());
                text.extend(w.chars().map(|c| c as u8));
                text.push(0);
            }
        }
        region.extend_from_slice(&text);
        region.extend_from_slice(&[0; 11]);
        let mut v = MAGIC.to_vec();
        v.extend_from_slice(&[1, 0, 1, 0]);
        v.extend_from_slice(&images.to_le_bytes());
        let first = 18 + 6 * usize::from(images) + region.len();
        for id in 0..images {
            v.extend_from_slice(&id.to_le_bytes());
            v.extend_from_slice(&((first + 4 * usize::from(id)) as u32).to_le_bytes());
        }
        v.extend_from_slice(&region);
        v.extend(std::iter::repeat_n(0xaa, 4 * usize::from(images)));
        v
    }

    #[test]
    fn hash_rotates_five_bits() {
        assert_eq!(hash(b""), 0);
        assert_eq!(hash(b"a"), 0x61);
        assert_eq!(hash(b"ab"), (0x61 << 5) + 0x62);
        // rol16 traz de volta os bits altos
        assert_eq!(hash(&[0xff; 4]), {
            let mut h = 0u16;
            for _ in 0..4 {
                h = ((h << 5) | (h >> 11)).wrapping_add(0xff);
            }
            h
        });
    }

    #[test]
    fn picture_ids() {
        assert_eq!(color_id(0), None);
        assert_eq!(color_id(1), Some(0));
        assert_eq!(color_id(0x25), Some(0x49));
        assert_eq!(mono_id(0x25), Some(0x4a));
        assert_eq!(mono_id(1), Some(2));
    }

    #[test]
    fn reads_and_looks_up_a_built_table() {
        let words = [
            ("gato", 0x25),
            ("gatos", 0x25),
            ("gato", 0xc6),
            ("pique-nique", 0x99),
            ("café", 0x40),
            ("aeronave", 1),
        ];
        let pic = build(&words, 3);
        let m = Maca::read(&pic).unwrap();
        assert_eq!(m.len(), 6);
        assert_eq!(m.lookup("gato"), Some(0x25));
        assert_eq!(m.lookup_all("gato"), vec![0x25, 0xc6]);
        assert_eq!(m.lookup("GATO"), Some(0x25));
        assert_eq!(m.lookup("Café"), Some(0x40));
        assert_eq!(m.lookup("pique-nique"), Some(0x99));
        assert_eq!(m.lookup("aeronave"), Some(1));
        assert_eq!(m.lookup("abelha"), None);
        assert_eq!(m.lookup("maç"), None);
        assert_eq!(m.lookup("日本"), None);
        assert_eq!(m.lookup(""), None);
        assert_eq!(m.entries().count(), 6);
    }

    #[test]
    fn rejects_files_without_a_table() {
        assert!(Maca::read(b"nada").is_err());
        // índice colado na primeira imagem: sem tabela (como o ARTIST.PIC)
        let mut v = MAGIC.to_vec();
        v.extend_from_slice(&[1, 0, 1, 0, 1, 0, 0, 0]);
        v.extend_from_slice(&24u32.to_le_bytes());
        v.extend_from_slice(&[0; 8]);
        assert!(Maca::read(&v).is_err());
        // palavra fora do seu balde
        let mut pic = build(&[("gato", 0x25), ("cama", 0x30)], 1);
        let at = pic.windows(4).position(|w| w == b"gato").unwrap();
        pic[at] = (b'a'..=b'z')
            .find(|&c| hash(&[c, b'a', b't', b'o']) % 2 != hash(b"gato") % 2)
            .unwrap();
        assert!(Maca::read(&pic).is_err());
        // figura ímpar
        let mut pic = build(&[("gato", 0x25)], 1);
        let at = 18 + 6 + 2 + 4;
        assert_eq!(pic[at], 0x4a);
        pic[at] = 0x4b;
        assert!(Maca::read(&pic).is_err());
    }
}
