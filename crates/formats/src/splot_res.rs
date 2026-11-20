//! Recursos do Splot (o "Cavalete de palavras com forma") nos executáveis,
//! idênticos no `ARTIST.EXE` e no `WRITER.EXE`: menus de texto (`TMEN`),
//! tabela de cores (`CSCH`), padrões (`PATS`) e, em forma bruta, sombras
//! (`SSHD`), geometria do cavalete (`SMNM`) e `TDRL`. As formas (`SCRD`)
//! estão em [`crate::scrd`]. Ver `docs/formats/splot.md`.

use crate::palette::Rgb;
use crate::{ErrorKind, Format, Reader, Result, cp1252};

/// Estilo de letra de um item de menu (bits do Mac).
pub mod style {
    pub const BOLD: u8 = 1;
    pub const ITALIC: u8 = 2;
    pub const UNDERLINE: u8 = 4;
    pub const OUTLINE: u8 = 8;
}

/// Item de um `TMEN`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub label: String,
    /// Estilo em que o item é escrito ([`style`]; `Negrito` = negrito).
    pub style: u8,
}

/// Menu de texto (`TMEN`): 1 = estilos do Splot, 2 = sombras; no
/// `ARTIST.EXE` também o 4 (Normal, Negrito, Sublinhado).
///
/// ```text
/// u16  n
/// u32  0
/// n × u8          estilo de cada item
/// n × { u8 tamanho, texto Windows-1252, u8 0 }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tmen {
    pub items: Vec<MenuItem>,
}

impl Tmen {
    pub fn labels(&self) -> Vec<&str> {
        self.items.iter().map(|i| i.label.as_str()).collect()
    }
}

pub fn read_tmen(data: &[u8]) -> Result<Tmen> {
    let mut r = Reader::new(data, Format::WinRes);
    let n = r.u16_le()? as usize;
    r.skip(4)?;
    let styles = r.bytes(n)?.to_vec();
    let mut items = Vec::with_capacity(n);
    for style in styles {
        let len = r.u8()? as usize;
        let label = cp1252::decode(r.bytes(len)?);
        if r.u8()? != 0 {
            return Err(r.error(ErrorKind::Invalid("TMEN: texto sem 0 no fim".into())));
        }
        items.push(MenuItem { label, style });
    }
    Ok(Tmen { items })
}

/// Tabela de cores do Splot (`CSCH` 1): as 16 cores das letras e os
/// conjuntos da lista de cores (uma cor ou duas/três alternadas por letra).
///
/// ```text
/// u16  16          cores
/// u16  24          conjuntos
/// u16  3           máximo de cores num conjunto
/// 16 × { u16 r, u16 g, u16 b }   como no Mac: o byte alto é a cor
/// 24 × { u8 n, n × u8 índice }   conjuntos, na ordem da lista
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Csch {
    pub colors: Vec<Rgb>,
    /// Índices em `colors`, na ordem da lista de cores do cavalete.
    pub sets: Vec<Vec<u8>>,
    pub max_per_set: u16,
}

pub fn read_csch(data: &[u8]) -> Result<Csch> {
    let mut r = Reader::new(data, Format::WinRes);
    let nc = r.u16_le()? as usize;
    let ns = r.u16_le()? as usize;
    let max_per_set = r.u16_le()?;
    let mut colors = Vec::with_capacity(nc);
    for _ in 0..nc {
        let mut c = [0u8; 3];
        for v in &mut c {
            *v = (r.u16_le()? >> 8) as u8;
        }
        colors.push(Rgb {
            r: c[0],
            g: c[1],
            b: c[2],
        });
    }
    let mut sets = Vec::with_capacity(ns);
    for _ in 0..ns {
        let k = r.u8()? as usize;
        let set = r.bytes(k)?.to_vec();
        if k == 0 || set.iter().any(|&i| i as usize >= nc) {
            return Err(r.error(ErrorKind::Invalid(format!("CSCH: conjunto {set:?}"))));
        }
        sets.push(set);
    }
    Ok(Csch {
        colors,
        sets,
        max_per_set,
    })
}

/// Padrões do Splot (`PATS` 1): `u16 n` + n × 8 bytes, uma linha de 8
/// pixels por byte, bit alto à esquerda, **bit 1 = tinta** (como no Mac;
/// o `PAT#` lido por [`crate::macres::parse_pattern_list`] é outro formato).
/// Na lista do cavalete vêm depois de um quadrado cheio.
pub fn read_pats(data: &[u8]) -> Result<Vec<[u8; 8]>> {
    let mut r = Reader::new(data, Format::WinRes);
    let n = r.u16_le()? as usize;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        let b = r.bytes(8)?;
        v.push(std::array::from_fn(|i| b[i]));
    }
    Ok(v)
}

/// Unidade dos números de ponto fixo do `SSHD` (1200 = 1,0).
pub const SSHD_UNIT: i16 = 1200;

/// Sombras (`SSHD` 1), em forma bruta: `u16` número de sombras (7, as do
/// `TMEN 2` depois de "Sem sombra"), `u16 1` e registros de tamanho
/// variável com a cor da sombra (3 × `u16` do Mac), deslocamentos e matrizes
/// 2×2 em ponto fixo ([`SSHD_UNIT`]). "Girar em círculo" são 17 cópias
/// giradas de 20° em 20° (20° … 340°; ver `docs/formats/splot.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sshd {
    pub count: u16,
    pub flag: u16,
    /// Todos os bytes do recurso (inclusive os 4 do cabeçalho).
    pub raw: Vec<u8>,
}

impl Sshd {
    /// Matrizes de giro `[a, b, c, d]` (÷ [`SSHD_UNIT`]) das cópias de
    /// "Girar em círculo" de 40° a 340°: os registros de 60 bytes que
    /// começam por `b0 04 ff ff 03 00 06 00 03 00`, com a matriz em +44 (a
    /// de 20° fica no registro anterior, de outro formato).
    pub fn spin_matrices(&self) -> Vec<[i16; 4]> {
        const SIG: [u8; 10] = [0xb0, 0x04, 0xff, 0xff, 0x03, 0x00, 0x06, 0x00, 0x03, 0x00];
        let b = &self.raw;
        let i16_at = |o: usize| i16::from_le_bytes([b[o], b[o + 1]]);
        (0..b.len().saturating_sub(52))
            .filter(|&o| b[o..o + 10] == SIG)
            .map(|o| std::array::from_fn(|k| i16_at(o + 44 + 2 * k)))
            // o último registro com essa assinatura já é da sombra seguinte
            .filter(|m: &[i16; 4]| {
                let n = (m[0] as f32).hypot(m[2] as f32);
                (n - SSHD_UNIT as f32).abs() < 12.0
            })
            .collect()
    }
}

pub fn read_sshd(data: &[u8]) -> Result<Sshd> {
    let mut r = Reader::new(data, Format::WinRes);
    let count = r.u16_le()?;
    let flag = r.u16_le()?;
    Ok(Sshd {
        count,
        flag,
        raw: data.to_vec(),
    })
}

/// Geometria do cavalete (`SMNM` 1 e 2): 20 `u16`. O primeiro par é o
/// tamanho da moldura (463 × 335 no `SMNM 1`, o do Fine Artist); o resto
/// não foi fechado (botões, prévia).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Smnm {
    pub values: [u16; 20],
}

impl Smnm {
    pub fn frame_size(&self) -> (u16, u16) {
        (self.values[0], self.values[1])
    }
}

pub fn read_smnm(data: &[u8]) -> Result<Smnm> {
    let mut r = Reader::new(data, Format::WinRes);
    let mut values = [0u16; 20];
    for v in &mut values {
        *v = r.u16_le()?;
    }
    Ok(Smnm { values })
}

/// `TDRL` 1: `u16 n` + n registros de 12 bytes (`ff ff`, `u16`, 4 bytes,
/// `04 00`, `u8`, `ff`); não interpretado.
pub fn read_tdrl(data: &[u8]) -> Result<Vec<[u8; 12]>> {
    let mut r = Reader::new(data, Format::WinRes);
    let n = r.u16_le()? as usize;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        let b = r.bytes(12)?;
        v.push(std::array::from_fn(|i| b[i]));
    }
    Ok(v)
}

/// Um "preferido" do cavalete (botões ＋ e − no Creative Writer): um
/// registro do `TDRL`, com as escolhas na ordem dos bytes `+0x40..+0x47` do
/// cabeçalho do `SPLE` (`docs/formats/sple.md`).
///
/// ```text
/// ff ff  i16 giro  u8 forma  u8 sombra  u8 padrão  u8 cores
/// u8 estilo das letras  u8 fonte (4, ou 0 com estilo 0)  u8 estilos  ff
/// ```
pub fn favorite(rec: &[u8; 12]) -> crate::sple::Choices {
    crate::sple::Choices {
        angle: i16::from_le_bytes([rec[2], rec[3]]),
        shape: rec[4],
        shadow: rec[5],
        pattern: rec[6],
        color_set: rec[7],
        letter_style: rec[8],
        style: rec[10],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tdrl_record_is_a_favorite() {
        let f = favorite(&[0xff, 0xff, 0x54, 0x01, 1, 5, 0, 10, 4, 0, 2, 0xff]);
        assert_eq!(f.angle, 340);
        assert_eq!((f.shape, f.shadow, f.pattern, f.color_set), (1, 5, 0, 10));
        assert_eq!((f.letter_style, f.style), (4, 2));
    }

    #[test]
    fn tmen_items_and_styles() {
        let mut d = vec![3, 0, 0, 0, 0, 0, 0, 1, 4];
        for s in ["Normal", "Negrito", "Sublinhado"] {
            d.push(s.len() as u8);
            d.extend(s.as_bytes());
            d.push(0);
        }
        d.extend([0; 8]);
        let t = read_tmen(&d).unwrap();
        assert_eq!(t.labels(), ["Normal", "Negrito", "Sublinhado"]);
        assert_eq!(t.items[1].style, style::BOLD);
        assert_eq!(t.items[2].style, style::UNDERLINE);
        // texto sem o 0 final
        let mut bad = d.clone();
        bad[9 + 7] = b'x';
        assert!(read_tmen(&bad).is_err());
        assert!(read_tmen(&d[..12]).is_err());
    }

    #[test]
    fn csch_colors_and_sets() {
        let mut d = vec![2, 0, 3, 0, 3, 0];
        d.extend([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        d.extend([0x39, 0xfa, 0, 0, 0x39, 0x39]);
        d.extend([1, 1, 2, 1, 0, 1, 0]);
        let c = read_csch(&d).unwrap();
        assert_eq!(
            c.colors[1],
            Rgb {
                r: 0xfa,
                g: 0,
                b: 0x39
            }
        );
        assert_eq!(c.sets, vec![vec![1], vec![1, 0], vec![0]]);
        // índice fora da tabela
        let mut bad = d.clone();
        *bad.last_mut().unwrap() = 7;
        assert!(read_csch(&bad).is_err());
    }

    #[test]
    fn pats_smnm_tdrl_sshd() {
        let mut d = vec![1, 0];
        d.extend([0xf7, 0xe3, 0xc1, 0x8d, 0xad, 0x8d, 0xff, 0xaa]);
        assert_eq!(read_pats(&d).unwrap()[0][7], 0xaa);
        assert!(read_pats(&d[..9]).is_err());
        let mut m = Vec::new();
        for v in 0..20u16 {
            m.extend((v + 1).to_le_bytes());
        }
        assert_eq!(read_smnm(&m).unwrap().frame_size(), (1, 2));
        let mut t = vec![1, 0];
        t.extend([0xff; 12]);
        assert_eq!(read_tdrl(&t).unwrap(), vec![[0xff; 12]]);
        let mut s = vec![7, 0, 1, 0];
        let mut rec = vec![0u8; 60];
        rec[..10].copy_from_slice(&[0xb0, 0x04, 0xff, 0xff, 0x03, 0x00, 0x06, 0x00, 0x03, 0x00]);
        rec[44..52].copy_from_slice(&[0x97, 0x03, 0xfd, 0xfc, 0x03, 0x03, 0x97, 0x03]);
        s.extend(&rec);
        let sh = read_sshd(&s).unwrap();
        assert_eq!((sh.count, sh.flag), (7, 1));
        assert_eq!(sh.spin_matrices(), vec![[919, -771, 771, 919]]);
    }
}
