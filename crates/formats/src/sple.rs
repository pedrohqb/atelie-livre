//! Chunk `SPLE` do documento `.MAX` do Creative Writer: uma palavra com
//! forma ("Splot") pronta, com as escolhas do cavalete e o desenho já
//! montado em polígonos. Ver `docs/formats/sple.md`.
//!
//! ```text
//! 0x000  cabeçalho de 0x48 bytes (contagens, caixa, escolhas do cavalete)
//! 0x048  Pascal: nome da fonte (campo de 257 bytes)
//! 0x149  Pascal: texto (campo de 257 bytes)
//! 0x24a  registro da fonte, registro da forma (um `SCRD` sem os dois
//!        primeiros campos), sombra e cores
//! ....   uma letra por caractere: u16 2, u8 caractere, u8 8 | estilo,
//!        separadas por u16 0
//! +0x1c  glifos: polígonos coloridos, desenhados em ordem
//! ```
//!
//! O desenho do original é feito só com os glifos (as sombras são glifos
//! cinza antes das letras): o leitor guarda tudo antes das letras como
//! bytes crus, para gravar de volta igual, e dá acesso aos campos conhecidos.

use crate::{ErrorKind, Format, Reader, Result};

/// Tamanho do cabeçalho.
pub const HEADER_LEN: usize = 0x48;
/// Campo Pascal do nome da fonte.
pub const FONT_AT: usize = 0x48;
/// Campo Pascal do texto.
pub const TEXT_AT: usize = 0x149;
/// Registros da fonte, da forma, da sombra e das cores.
pub const RECORDS_AT: usize = 0x24a;
/// Lado da caixa das formas do `SCRD` (e do retângulo).
pub const SHAPE_UNITS: i16 = 8000;

/// Cor como no Mac: 3 × `u16`, o byte alto é a cor.
pub type Rgb48 = [u16; 3];

/// Cor de 8 bits por canal a partir de [`Rgb48`].
pub fn rgb8(c: Rgb48) -> [u8; 3] {
    [(c[0] >> 8) as u8, (c[1] >> 8) as u8, (c[2] >> 8) as u8]
}

/// [`Rgb48`] a partir de 8 bits por canal (`ab` → `abab`).
pub fn rgb48(c: [u8; 3]) -> Rgb48 {
    c.map(|v| u16::from(v) * 0x101)
}

/// Como um glifo é pintado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fill {
    /// Tipo 2: polígonos cheios (par-ímpar).
    Solid,
    /// Tipo 3: cheios e com o contorno na mesma cor (o normal das letras).
    Framed,
    /// Tipo 5: padrão 8×8 do `PATS` (bit 1 = cor da frente) sobre `back`.
    Pattern {
        back: Rgb48,
        /// `0xffff` nos exemplos.
        flag: u16,
        pattern: [u8; 8],
    },
}

/// Um glifo: uma letra (ou uma cópia de sombra dela) em polígonos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph {
    /// `u16` do começo: o byte baixo é o tipo (2, 3, 5); o alto, 0x06 ou
    /// 0x16 (com padrão).
    pub tag: u16,
    /// Caixa (cima, esquerda, baixo, direita), como um `Rect` do Mac.
    pub rect: [i16; 4],
    pub color: Rgb48,
    pub fill: Fill,
    /// Contornos fechados de pontos `(x, y)` (o último liga ao primeiro).
    pub contours: Vec<Vec<(i16, i16)>>,
}

impl Glyph {
    /// Glifo cheio com os contornos dados; a caixa é calculada.
    pub fn solid(color: Rgb48, contours: Vec<Vec<(i16, i16)>>) -> Glyph {
        let mut rect = [i16::MAX, i16::MAX, i16::MIN, i16::MIN];
        for &(x, y) in contours.iter().flatten() {
            rect[0] = rect[0].min(y);
            rect[1] = rect[1].min(x);
            rect[2] = rect[2].max(y);
            rect[3] = rect[3].max(x);
        }
        if rect[0] > rect[2] {
            rect = [0; 4];
        }
        Glyph {
            tag: 0x0602,
            rect,
            color,
            fill: Fill::Solid,
            contours,
        }
    }

    pub fn points(&self) -> usize {
        self.contours.iter().map(Vec::len).sum()
    }
}

/// Escolhas do cavalete gravadas no cabeçalho (`+0x40..+0x47`), na mesma
/// ordem dos registros do `TDRL` (os "preferidos").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Choices {
    /// Giro, em graus (vertical = 270).
    pub angle: i16,
    /// Forma: 0 = retângulo, 1..14 = `SCRD`.
    pub shape: u8,
    /// Sombra: índice no `TMEN 2` (0 = sem sombra).
    pub shadow: u8,
    /// Padrão: 0 = cheio, 1..13 = `PATS`.
    pub pattern: u8,
    /// Conjunto da lista de cores (`CSCH`).
    pub color_set: u8,
    /// Estilo das letras: 1 negrito, 2 inclinado, 4 contorno fino na cor
    /// (glifos de tipo 3).
    pub letter_style: u8,
    /// Estilos da forma: ver [`style`].
    pub style: u8,
}

/// Bits de [`Choices::style`] (botão AA, `TMEN 1`).
pub mod style {
    pub const VERTICAL: u8 = 0x01;
    pub const FILL: u8 = 0x02;
    pub const CAPS: u8 = 0x04;
    pub const FLIP_H: u8 = 0x20;
    pub const FLIP_V: u8 = 0x40;
}

/// Bits de [`Choices::letter_style`].
pub mod letter {
    pub const BOLD: u8 = 0x01;
    pub const ITALIC: u8 = 0x02;
    pub const FRAMED: u8 = 0x04;
}

/// Um Splot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sple {
    /// Tudo antes das letras (cabeçalho, nomes, fonte, forma, sombra e
    /// cores), como no arquivo.
    pub prefix: Vec<u8>,
    /// `(caractere, 8 | estilo)` de cada letra.
    pub letters: Vec<(u8, u8)>,
    pub glyphs: Vec<Glyph>,
}

fn u16_at(d: &[u8], at: usize) -> u16 {
    d.get(at..at + 2)
        .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
}

fn pascal(d: &[u8], at: usize) -> &[u8] {
    let n = d.get(at).copied().unwrap_or(0) as usize;
    d.get(at + 1..at + 1 + n).unwrap_or(&[])
}

impl Sple {
    fn header_u16(&self, at: usize) -> u16 {
        u16_at(&self.prefix, at)
    }

    /// Nome da fonte (Windows-1252).
    pub fn font(&self) -> &[u8] {
        pascal(&self.prefix, FONT_AT)
    }

    /// Texto (Windows-1252), como digitado.
    pub fn text(&self) -> &[u8] {
        pascal(&self.prefix, TEXT_AT)
    }

    /// Caixa do desenho (cima, esquerda, baixo, direita), nas unidades dos
    /// glifos.
    pub fn bounds(&self) -> [i16; 4] {
        [0x10, 0x12, 0x14, 0x16].map(|o| self.header_u16(o) as i16)
    }

    /// Número de cores do conjunto escolhido.
    pub fn color_count(&self) -> u16 {
        self.header_u16(0x0a)
    }

    /// Há sombra (cópias cinza antes das letras).
    pub fn shadowed(&self) -> bool {
        self.header_u16(0x08) != 0
    }

    pub fn choices(&self) -> Choices {
        let h = &self.prefix;
        let b = |o: usize| h.get(o).copied().unwrap_or(0);
        Choices {
            angle: self.header_u16(0x40) as i16,
            shape: b(0x42),
            shadow: b(0x43),
            pattern: b(0x44),
            color_set: b(0x45),
            letter_style: b(0x46),
            style: b(0x47),
        }
    }

    /// Registro da forma (o `SCRD` sem `tipo` e o campo seguinte).
    pub fn shape_record(&self) -> &[u8] {
        let font_len = self.header_u16(0x04) as usize;
        let shape_len = self.header_u16(0x06) as usize;
        let at = RECORDS_AT + font_len;
        self.prefix.get(at..at + shape_len).unwrap_or(&[])
    }
}

/// Lê o conteúdo da parte `SPLE` do chunk `SPLE`.
pub fn read(data: &[u8]) -> Result<Sple> {
    let mut r = Reader::new(data, Format::Max);
    if data.len() < RECORDS_AT {
        return Err(r.error(ErrorKind::Invalid("SPLE: curto demais".into())));
    }
    let n_letters = u16_at(data, 0x0c) as usize;
    let n_glyphs = u16_at(data, 0x0e) as usize;
    let glyphs_at = u16_at(data, 0x1c) as usize;
    let letters_len = (6 * n_letters).saturating_sub(2);
    let Some(letters_at) = glyphs_at.checked_sub(letters_len) else {
        return Err(r.error(ErrorKind::Invalid("SPLE: letras antes do cabeçalho".into())));
    };
    if letters_at < RECORDS_AT || glyphs_at > data.len() {
        return Err(r.error(ErrorKind::Invalid("SPLE: posição dos glifos".into())));
    }
    r.seek(letters_at)?;
    let mut letters = Vec::with_capacity(n_letters);
    for k in 0..n_letters {
        if k > 0 {
            r.u16_le()?;
        }
        if r.u16_le()? != 2 {
            return Err(r.error(ErrorKind::Invalid("SPLE: letra sem o 2".into())));
        }
        letters.push((r.u8()?, r.u8()?));
    }
    let mut glyphs = Vec::with_capacity(n_glyphs);
    for _ in 0..n_glyphs {
        glyphs.push(read_glyph(&mut r)?);
    }
    Ok(Sple {
        prefix: data[..letters_at].to_vec(),
        letters,
        glyphs,
    })
}

fn read_glyph(r: &mut Reader) -> Result<Glyph> {
    let tag = r.u16_le()?;
    let total = r.u16_le()? as usize;
    let n = r.u16_le()? as usize;
    let mut rect = [0i16; 4];
    for v in &mut rect {
        *v = r.i16_le()?;
    }
    let color = [r.u16_le()?, r.u16_le()?, r.u16_le()?];
    let fill = match tag & 0xff {
        2 => Fill::Solid,
        3 => Fill::Framed,
        5 => {
            let back = [r.u16_le()?, r.u16_le()?, r.u16_le()?];
            let flag = r.u16_le()?;
            let mut pattern = [0u8; 8];
            pattern.copy_from_slice(r.bytes(8)?);
            Fill::Pattern {
                back,
                flag,
                pattern,
            }
        }
        k => {
            return Err(r.error(ErrorKind::Unsupported(format!("SPLE: glifo de tipo {k}"))));
        }
    };
    let mut counts = Vec::with_capacity(n);
    for _ in 0..n {
        counts.push(r.u16_le()? as usize);
    }
    if counts.iter().sum::<usize>() != total {
        return Err(r.error(ErrorKind::Invalid("SPLE: soma dos contornos".into())));
    }
    let mut contours = Vec::with_capacity(n);
    for c in counts {
        let mut pts = Vec::with_capacity(c);
        for _ in 0..c {
            pts.push((r.i16_le()?, r.i16_le()?));
        }
        contours.push(pts);
    }
    Ok(Glyph {
        tag,
        rect,
        color,
        fill,
        contours,
    })
}

/// Grava a parte `SPLE` (o inverso de [`read`]). As contagens e a posição
/// dos glifos no cabeçalho são acertadas pelo que há em `s`.
pub fn write(s: &Sple) -> Vec<u8> {
    let mut v = s.prefix.clone();
    if v.len() < HEADER_LEN {
        v.resize(HEADER_LEN, 0);
    }
    let letters_len = (6 * s.letters.len()).saturating_sub(2);
    let glyphs_at = v.len() + letters_len;
    v[0x0c..0x0e].copy_from_slice(&(s.letters.len() as u16).to_le_bytes());
    v[0x0e..0x10].copy_from_slice(&(s.glyphs.len() as u16).to_le_bytes());
    v[0x1c..0x1e].copy_from_slice(&(glyphs_at as u16).to_le_bytes());
    for (k, &(c, st)) in s.letters.iter().enumerate() {
        if k > 0 {
            v.extend(0u16.to_le_bytes());
        }
        v.extend(2u16.to_le_bytes());
        v.push(c);
        v.push(st);
    }
    for g in &s.glyphs {
        write_glyph(&mut v, g);
    }
    v
}

fn write_glyph(v: &mut Vec<u8>, g: &Glyph) {
    let mut put = |x: u16, v: &mut Vec<u8>| v.extend(x.to_le_bytes());
    put(g.tag, v);
    put(g.points() as u16, v);
    put(g.contours.len() as u16, v);
    for x in g.rect {
        put(x as u16, v);
    }
    for c in g.color {
        put(c, v);
    }
    if let Fill::Pattern {
        back,
        flag,
        pattern,
    } = &g.fill
    {
        for c in back {
            put(*c, v);
        }
        put(*flag, v);
        v.extend(pattern);
    }
    for c in &g.contours {
        put(c.len() as u16, v);
    }
    for &(x, y) in g.contours.iter().flatten() {
        put(x as u16, v);
        put(y as u16, v);
    }
}

/// O que é preciso para montar um Splot novo (o que o OK do cavalete grava).
#[derive(Debug, Clone)]
pub struct NewSple<'a> {
    pub font: &'a [u8],
    pub text: &'a [u8],
    pub choices: Choices,
    /// Registro da forma ([`shape_record`] ou o de um `SCRD`).
    pub shape: Vec<u8>,
    /// `tipo` do `SCRD` da forma (0 no retângulo).
    pub shape_kind: u8,
    /// Cores do conjunto escolhido.
    pub colors: &'a [Rgb48],
    pub bounds: [i16; 4],
    pub glyphs: Vec<Glyph>,
}

/// Registro da forma do retângulo (sem `SCRD`), como o original grava.
pub fn rect_shape_record() -> Vec<u8> {
    let mut v = Vec::new();
    for x in [4u16, 1, 8000, 8000, 2, 0, 2, 0, 0] {
        v.extend(x.to_le_bytes());
    }
    for (x, y) in [(0i16, 0i16), (8000, 0), (0, -8000), (8000, -8000)] {
        v.extend(x.to_le_bytes());
        v.extend(y.to_le_bytes());
    }
    v
}

/// Registro da forma a partir do recurso `SCRD` inteiro: o original tira
/// os dois primeiros `u16` (`tipo` e o seguinte).
pub fn shape_record(scrd: &[u8]) -> Vec<u8> {
    scrd.get(4..).unwrap_or(&[]).to_vec()
}

/// Bloco da sombra como o original grava a "Sombra normal" (cinza
/// #8c8c8c); o port usa o mesmo para todas as sombras (o desenho vem dos
/// glifos).
const SHADOW_BLOCK: [u16; 30] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 12, 0x8c8c, 0x8c8c, 0x8c8c, 0, 0, 0, 0, 0,
    0, 0, 0,
];

/// Monta um Splot novo. As partes que o original usa só para reabrir o
/// cavalete seguem os exemplos sem sombra (`docs/formats/sple.md`).
pub fn build(n: &NewSple) -> Sple {
    let mut v = vec![0u8; RECORDS_AT];
    let put = |v: &mut Vec<u8>, at: usize, x: u16| v[at..at + 2].copy_from_slice(&x.to_le_bytes());
    let font = &n.font[..n.font.len().min(255)];
    let text = &n.text[..n.text.len().min(255)];
    // registro da fonte: u16 tamanho, Pascal, 0, completado até par
    let mut font_rec = Vec::new();
    font_rec.extend([0, 0, font.len() as u8]);
    font_rec.extend(font);
    font_rec.push(0);
    if font_rec.len() % 2 == 1 {
        font_rec.push(0);
    }
    let fl = font_rec.len() as u16;
    font_rec[..2].copy_from_slice(&fl.to_le_bytes());
    put(&mut v, 0x00, 1);
    put(&mut v, 0x02, (-13i16) as u16);
    put(&mut v, 0x04, fl);
    put(&mut v, 0x06, n.shape.len() as u16);
    put(&mut v, 0x08, u16::from(n.choices.shadow != 0));
    put(&mut v, 0x0a, n.colors.len() as u16);
    for (k, b) in n.bounds.iter().enumerate() {
        put(&mut v, 0x10 + 2 * k, *b as u16);
    }
    put(&mut v, 0x18, 0x210);
    v[0x20] = n.choices.style & 0x1f;
    v[0x21] = n.shape_kind;
    v[0x38..0x40].copy_from_slice(&[0, 0, 1, 0, 0xfc, 0xff, 0, 0]);
    let c = n.choices;
    put(&mut v, 0x40, c.angle as u16);
    v[0x42..0x48].copy_from_slice(&[
        c.shape,
        c.shadow,
        c.pattern,
        c.color_set,
        c.letter_style,
        c.style,
    ]);
    v[FONT_AT] = font.len() as u8;
    v[FONT_AT + 1..FONT_AT + 1 + font.len()].copy_from_slice(font);
    v[TEXT_AT] = text.len() as u8;
    v[TEXT_AT + 1..TEXT_AT + 1 + text.len()].copy_from_slice(text);
    v.extend(&font_rec);
    v.extend(&n.shape);
    if n.choices.shadow != 0 {
        for x in SHADOW_BLOCK {
            v.extend(x.to_le_bytes());
        }
    }
    // cada cor: frente, fundo (a seguinte; branco se só há uma), u16 0 e 8
    // bytes de padrão; depois um u16 0
    let k = n.colors.len();
    for (i, fg) in n.colors.iter().enumerate() {
        let bg = if k > 1 {
            n.colors[(i + 1) % k]
        } else {
            [0xffff; 3]
        };
        for x in fg.iter().chain(&bg) {
            v.extend(x.to_le_bytes());
        }
        v.extend([0u8; 10]);
    }
    v.extend([0, 0]);
    let style = 8 | (c.letter_style & 7);
    Sple {
        prefix: v,
        letters: text.iter().map(|&ch| (ch, style)).collect(),
        glyphs: n.glyphs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Sple {
        build(&NewSple {
            font: b"Impact",
            text: b"Ana",
            choices: Choices {
                shape: 0,
                shadow: 1,
                color_set: 7,
                letter_style: 4,
                ..Choices::default()
            },
            shape: rect_shape_record(),
            shape_kind: 0,
            colors: &[[0xfafa, 0, 0x3939]],
            bounds: [0, 0, 100, 300],
            glyphs: vec![Glyph::solid(
                [0, 0, 0],
                vec![vec![(0, 0), (10, 0), (10, 10), (0, 10)], vec![(2, 2), (4, 2), (3, 4)]],
            )],
        })
    }

    #[test]
    fn built_splot_reads_back() {
        let s = sample();
        let bytes = write(&s);
        let r = read(&bytes).unwrap();
        assert_eq!(r.text(), b"Ana");
        assert_eq!(r.font(), b"Impact");
        assert_eq!(r.letters, vec![(b'A', 12), (b'n', 12), (b'a', 12)]);
        assert_eq!(r.glyphs, s.glyphs);
        assert_eq!(r.glyphs[0].rect, [0, 0, 10, 10]);
        assert_eq!(r.bounds(), [0, 0, 100, 300]);
        assert!(r.shadowed());
        assert_eq!(r.color_count(), 1);
        assert_eq!(r.choices().color_set, 7);
        assert_eq!(r.shape_record(), &rect_shape_record()[..]);
        assert_eq!(write(&r), bytes);
        // registro da fonte "Impact": 10 bytes, como nos exemplos
        assert_eq!(&bytes[RECORDS_AT..RECORDS_AT + 4], &[10, 0, 6, b'I']);
    }

    #[test]
    fn pattern_glyph_round_trips() {
        let mut s = sample();
        s.glyphs[0].tag = 0x1605;
        s.glyphs[0].fill = Fill::Pattern {
            back: [1, 2, 3],
            flag: 0xffff,
            pattern: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let r = read(&write(&s)).unwrap();
        assert_eq!(r.glyphs, s.glyphs);
    }

    #[test]
    fn bad_glyph_kind_is_an_error() {
        let mut s = sample();
        s.glyphs[0].tag = 0x0609;
        assert!(read(&write(&s)).is_err());
    }

    #[test]
    fn colors_convert_like_the_mac() {
        assert_eq!(rgb8([0xfafa, 0x4a4a, 0x0000]), [0xfa, 0x4a, 0]);
        assert_eq!(rgb48([0x8c, 0, 0xff]), [0x8c8c, 0, 0xffff]);
    }
}
