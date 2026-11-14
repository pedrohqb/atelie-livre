//! Recurso `SCRD` dos executáveis (`ARTIST.EXE` e `WRITER.EXE`, ids 1 a 14,
//! dados idênticos nos dois): as formas do Splot, o "Cavalete de palavras
//! com forma". Ver `docs/formats/scrd.md`.
//!
//! ```text
//! u16  tipo        0 (três curvas), 1 (metade direita), 2 (metade esquerda), 3 (duas curvas)
//! u16  1 nas de tipo 3, 0 nas demais
//! u16  n           total de pontos
//! u16  1
//! u16  largura     da caixa de referência (largura do texto, em unidades da forma)
//! u16  altura      altura de uma linha de texto, em unidades da forma
//! u16  c0          pontos de cada curva
//! u16  c1          (0, c0 ou 2·c0; não usado pela leitura)
//! u16  c2          início da curva de cima (0)
//! u16  c3          início da curva do meio (igual a c2 ou c4 quando não há meio)
//! u16  c4          início da curva de baixo
//! n × { i16 x, i16 y }   centrados em (0, 0), y para baixo
//! ```
//!
//! O texto é distribuído **pelo índice** dos pontos: a letra na fração `u`
//! da linha fica entre os pontos `u·(c0−1)` das curvas de cima e de baixo
//! (conferido no original: a forma 4, com pontos mais juntos à esquerda,
//! encolhe o começo da palavra e aumenta o fim).

use crate::{ErrorKind, Format, Reader, Result};

/// Ponto de uma curva, nas unidades da forma.
pub type Point = (i16, i16);

/// Uma forma do Splot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scrd {
    /// 0 = três curvas, 1/2 = metade direita/esquerda, 3 = duas curvas.
    pub kind: u16,
    /// Segundo `u16` (1 nas formas de tipo 3).
    pub flag: u16,
    /// Largura da caixa de referência: a largura do texto em unidades da forma.
    pub width: u16,
    /// Altura de uma linha de texto em unidades da forma.
    pub height: u16,
    /// Os cinco campos depois da altura (`c0..c4`).
    pub fields: [u16; 5],
    pub points: Vec<Point>,
}

impl Scrd {
    /// Pontos de cada curva.
    pub fn curve_len(&self) -> usize {
        self.fields[0] as usize
    }

    fn curve(&self, start: u16) -> &[Point] {
        let s = start as usize;
        &self.points[s..s + self.curve_len()]
    }

    /// Curva de cima (topo das letras).
    pub fn top(&self) -> &[Point] {
        self.curve(self.fields[2])
    }

    /// Curva do meio, só quando é uma curva própria (formas de três curvas e a seta).
    pub fn middle(&self) -> Option<&[Point]> {
        let [_, _, top, mid, bottom] = self.fields;
        (mid != top && mid != bottom).then(|| self.curve(mid))
    }

    /// Curva de baixo (pé das letras).
    pub fn bottom(&self) -> &[Point] {
        self.curve(self.fields[4])
    }

    /// Caixa `(x0, y0, x1, y1)` de todos os pontos (inclusiva).
    pub fn bounds(&self) -> (i16, i16, i16, i16) {
        let mut b = (i16::MAX, i16::MAX, i16::MIN, i16::MIN);
        for &(x, y) in &self.points {
            b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
        if self.points.is_empty() {
            (0, 0, 0, 0)
        } else {
            b
        }
    }
}

/// Lê um `SCRD`. Erro se as curvas saem da lista de pontos.
pub fn read(data: &[u8]) -> Result<Scrd> {
    let mut r = Reader::new(data, Format::WinRes);
    let kind = r.u16_le()?;
    let flag = r.u16_le()?;
    let n = r.u16_le()? as usize;
    r.skip(2)?;
    let width = r.u16_le()?;
    let height = r.u16_le()?;
    let mut fields = [0u16; 5];
    for f in &mut fields {
        *f = r.u16_le()?;
    }
    let mut points = Vec::with_capacity(n);
    for _ in 0..n {
        points.push((r.i16_le()?, r.i16_le()?));
    }
    let c0 = fields[0] as usize;
    if c0 < 2 {
        return Err(r.error(ErrorKind::Invalid(format!("SCRD com curva de {c0} pontos"))));
    }
    for start in [fields[2], fields[3], fields[4]] {
        if start as usize + c0 > n {
            return Err(r.error(ErrorKind::Invalid(format!(
                "curva em {start} + {c0} passa de {n} pontos"
            ))));
        }
    }
    if width == 0 || height == 0 {
        return Err(r.error(ErrorKind::Invalid("SCRD com caixa vazia".into())));
    }
    Ok(Scrd {
        kind,
        flag,
        width,
        height,
        fields,
        points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Retângulo de duas curvas como o `SCRD 7`, com 3 pontos por curva.
    pub(crate) fn sample(fields: [u16; 5], pts: &[Point]) -> Vec<u8> {
        let mut v = Vec::new();
        for x in [3u16, 1, pts.len() as u16, 1, 8192, 1365] {
            v.extend(x.to_le_bytes());
        }
        for x in fields {
            v.extend(x.to_le_bytes());
        }
        for &(x, y) in pts {
            v.extend(x.to_le_bytes());
            v.extend(y.to_le_bytes());
        }
        v
    }

    #[test]
    fn reads_two_curves_without_middle() {
        let pts = [
            (-4096, 0),
            (0, -4096),
            (4096, 0),
            (-4096, 0),
            (0, 0),
            (4096, 0),
        ];
        let s = read(&sample([3, 3, 0, 3, 3], &pts)).unwrap();
        assert_eq!((s.kind, s.flag, s.width, s.height), (3, 1, 8192, 1365));
        assert_eq!(s.top(), &pts[..3]);
        assert_eq!(s.bottom(), &pts[3..]);
        assert_eq!(s.middle(), None);
        assert_eq!(s.bounds(), (-4096, -4096, 4096, 0));
    }

    #[test]
    fn middle_only_when_distinct() {
        let pts = [(0, -1), (1, -1), (0, 1), (1, 1), (0, 0), (1, 0)];
        // como a seta: baixo em 2, meio em 4
        let s = read(&sample([2, 4, 0, 4, 2], &pts)).unwrap();
        assert_eq!(s.middle(), Some(&pts[4..]));
        assert_eq!(s.bottom(), &pts[2..4]);
        // como o arco (SCRD 1): meio = cima
        let s = read(&sample([2, 0, 0, 0, 2], &pts[..4])).unwrap();
        assert_eq!(s.middle(), None);
    }

    #[test]
    fn rejects_curves_past_the_points() {
        let pts = [(0, 0), (1, 0), (0, 1)];
        assert!(read(&sample([2, 2, 0, 2, 2], &pts)).is_err());
        assert!(read(&sample([1, 1, 0, 1, 1], &pts)).is_err());
        assert!(read(&[3, 0, 1]).is_err());
    }
}
