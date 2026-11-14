//! Recursos do Splot (palavras com forma) lidos do executável: as formas
//! `SCRD`, os menus `TMEN` (estilos e sombras), as cores `CSCH` e os padrões
//! `PATS` — iguais no `ARTIST.EXE` e no `WRITER.EXE` (`docs/formats/splot.md`),
//! para o Fine Artist e o Creative Writer usarem o mesmo cavalete.
//!
//! Sem os dados originais, os textos e as cores (só números e rótulos)
//! vêm das tabelas abaixo; as formas ficam vazias (tudo sai em retângulo)
//! e os padrões se reduzem ao cheio.

use super::shaped::WordShape;
use crate::assets::{Assets, Program};
use formats::palette::Rgb;
use formats::scrd::{self, Scrd};
use formats::splot_res::{self, MenuItem, style};

/// Itens do `TMEN 1` (botão AA), na ordem do original.
pub const STYLE_LABELS: [&str; 9] = [
    "Normal",
    "Negrito",
    "Inclinado",
    "Contorno",
    "MAIÚSCULAS",
    "Vertical",
    "Preencher",
    "Em cima/embaixo",
    "Esquerda/direita",
];

/// As 16 cores do `CSCH 1`.
pub const COLORS: [(u8, u8, u8); 16] = [
    (0xff, 0xff, 0xff),
    (0xff, 0xee, 0x4c),
    (0xff, 0x8c, 0x32),
    (0xb9, 0x65, 0x24),
    (0xfa, 0x4a, 0x72),
    (0xfa, 0x00, 0x39),
    (0x88, 0x88, 0xe6),
    (0x61, 0x41, 0xdb),
    (0xde, 0x38, 0xc7),
    (0x93, 0x2c, 0xa3),
    (0x5c, 0xcb, 0x53),
    (0x47, 0x8c, 0x50),
    (0xbe, 0xbe, 0xbe),
    (0x8c, 0x8c, 0x8c),
    (0x64, 0x64, 0x64),
    (0x00, 0x00, 0x00),
];

/// Os 24 conjuntos do `CSCH 1` (índices em [`COLORS`]), na ordem da lista.
pub const COLOR_SETS: [&[u8]; 24] = [
    &[15],
    &[0],
    &[15, 0],
    &[1],
    &[2],
    &[3],
    &[4],
    &[5],
    &[6],
    &[7],
    &[8],
    &[9],
    &[10],
    &[11],
    &[12],
    &[13],
    &[14],
    &[5, 11],
    &[9, 11],
    &[4, 6],
    &[10, 4, 11],
    &[5, 0, 7],
    &[9, 8, 4],
    &[2, 15, 5],
];

/// O que o cavalete de palavras com forma usa do executável.
#[derive(Debug, Clone)]
pub struct SplotRes {
    /// `SCRD` 1 a 14 (posição = id − 1); vazio sem os dados.
    pub shapes: Vec<Scrd>,
    /// Itens do botão AA (`TMEN 1`), com o estilo em que são escritos.
    pub styles: Vec<MenuItem>,
    /// Itens do botão de sombras (`TMEN 2`).
    pub shadows: Vec<String>,
    /// Cores (`CSCH`) e conjuntos da lista de cores, como cores.
    pub color_sets: Vec<Vec<Rgb>>,
    /// Padrões da lista: o cheio e os do `PATS`, com **bit 1 = branco**
    /// (a convenção de [`super::shaped::Inks::pattern`]).
    pub patterns: Vec<[u8; 8]>,
}

impl Default for SplotRes {
    fn default() -> Self {
        SplotRes::from_resources(|_, _| None)
    }
}

impl SplotRes {
    /// Lê do `ARTIST.EXE` e, se faltar, do `WRITER.EXE`.
    pub fn load(a: &Assets) -> SplotRes {
        SplotRes::from_resources(|kind, id| {
            a.exe_resource(Program::Artist, kind, id)
                .or_else(|| a.exe_resource(Program::Writer, kind, id))
        })
    }

    /// Monta a partir de uma função que dá os bytes de um recurso (tipo, id).
    pub fn from_resources(get: impl Fn(&str, u16) -> Option<Vec<u8>>) -> SplotRes {
        let shapes: Vec<Scrd> = (1..=14)
            .map_while(|id| get("SCRD", id).and_then(|d| scrd::read(&d).ok()))
            .collect();
        let shapes = if shapes.len() == 14 {
            shapes
        } else {
            Vec::new()
        };
        let styles = get("TMEN", 1)
            .and_then(|d| splot_res::read_tmen(&d).ok())
            .filter(|t| t.items.len() == STYLE_LABELS.len())
            .map(|t| t.items)
            .unwrap_or_else(|| {
                STYLE_LABELS
                    .iter()
                    .enumerate()
                    .map(|(k, l)| MenuItem {
                        label: l.to_string(),
                        style: [0, style::BOLD, style::ITALIC, style::OUTLINE]
                            .get(k)
                            .copied()
                            .unwrap_or(0),
                    })
                    .collect()
            });
        let shadows = get("TMEN", 2)
            .and_then(|d| splot_res::read_tmen(&d).ok())
            .filter(|t| t.items.len() == super::shaped::Shadow::ALL.len())
            .map(|t| t.items.into_iter().map(|i| i.label).collect())
            .unwrap_or_else(|| {
                super::shaped::Shadow::ALL
                    .iter()
                    .map(|s| s.label().to_string())
                    .collect()
            });
        let color_sets = match get("CSCH", 1).and_then(|d| splot_res::read_csch(&d).ok()) {
            Some(c) => c
                .sets
                .iter()
                .map(|s| s.iter().map(|&i| c.colors[i as usize]).collect())
                .collect(),
            None => COLOR_SETS
                .iter()
                .map(|s| {
                    s.iter()
                        .map(|&i| {
                            let (r, g, b) = COLORS[i as usize];
                            Rgb { r, g, b }
                        })
                        .collect()
                })
                .collect(),
        };
        let mut patterns = vec![[0u8; 8]];
        if let Some(p) = get("PATS", 1).and_then(|d| splot_res::read_pats(&d).ok()) {
            patterns.extend(p.into_iter().map(|p| p.map(|row| !row)));
        }
        SplotRes {
            shapes,
            styles,
            shadows,
            color_sets,
            patterns,
        }
    }

    /// Curvas de uma forma (`None` no retângulo ou sem os dados).
    pub fn shape(&self, s: WordShape) -> Option<&Scrd> {
        s.scrd_id().and_then(|id| self.shapes.get(id as usize - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_data_uses_the_tables() {
        let r = SplotRes::default();
        assert!(r.shapes.is_empty());
        assert_eq!(r.styles.len(), 9);
        assert_eq!(r.styles[1].style, style::BOLD);
        assert_eq!(r.shadows[0], "Sem sombra");
        assert_eq!(r.color_sets.len(), 24);
        assert_eq!(r.color_sets[0], vec![Rgb { r: 0, g: 0, b: 0 }]);
        assert_eq!(r.color_sets[23].len(), 3);
        assert_eq!(r.patterns, vec![[0; 8]]);
        assert_eq!(r.shape(WordShape::Heart), None);
    }

    #[test]
    fn pats_are_inverted_after_the_solid_one() {
        let mut d = vec![1, 0];
        d.extend([0xf0; 8]);
        let r = SplotRes::from_resources(|k, _| (k == "PATS").then(|| d.clone()));
        assert_eq!(r.patterns, vec![[0; 8], [0x0f; 8]]);
    }
}
