//! Rolos de adesivos de clip-art (bibliotecas `MAXMAG10.ART`, `MAX100.ART` e
//! `MAG100.ART`). Cada rolo é um chunk `CARO` (lista de `PICT` + ícone); os
//! números dos `CARO` formam uma sequência única entre as três bibliotecas
//! (0 a 27), que é a ordem dos rolos no Fine Artist, depois do rolo de bichos.

use crate::Image;
use crate::assets::{Assets, Program};
use formats::render::bitmap_to_rgba;
use formats::{chnk, pkbm, wmf};
use std::collections::HashMap;
use std::rc::Rc;

pub const LIBRARIES: [&str; 3] = ["MAXMAG10.ART", "MAX100.ART", "MAG100.ART"];

/// Um adesivo de clip-art: miniatura (para a tira) e o metarquivo com o desenho.
pub struct ClipItem {
    pub preview: Option<Rc<Image>>,
    meta: Vec<u8>,
}

impl ClipItem {
    /// Desenho em tamanho real (bitmap de 1 bit), decodificado na hora.
    pub fn art(&self) -> Option<Image> {
        wmf::meta_dib(&self.meta).ok()
    }
}

pub struct ClipRoll {
    /// Número do `CARO`.
    pub id: u16,
    pub name: String,
    pub icon: Option<Rc<Image>>,
    pub items: Vec<ClipItem>,
}

fn tag_is(t: &formats::tag::Tag, s: &[u8; 4]) -> bool {
    &t.0 == s
}

/// Lista de um `CARO`: `u16` contagem, etiqueta `PICT`, `u16` ids (little-endian).
/// Lista de um `CARO`/`CSRO`: `u16` contagem, etiqueta, `u16` ids (little-endian).
pub fn parse_list(d: &[u8]) -> Vec<u16> {
    let Some(n) = d
        .get(0..2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
    else {
        return Vec::new();
    };
    d.get(6..)
        .unwrap_or_default()
        .chunks_exact(2)
        .take(n)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect()
}

/// Rolos das três bibliotecas, em ordem de número; bibliotecas ausentes são puladas.
pub fn load_rolls(assets: &Assets) -> Vec<ClipRoll> {
    let mut rolls = Vec::new();
    for name in LIBRARIES {
        let Some(data) = assets.library_bytes(name) else {
            continue;
        };
        let Ok(f) = chnk::parse(data) else { continue };
        let pal = assets.palettes(Program::Artist).for_chnk(&f);
        let image = |d: &[u8]| {
            pkbm::decode(d)
                .ok()
                .map(|b| Rc::new(bitmap_to_rgba(&b, &pal)))
        };
        let mut picts: HashMap<u16, (Option<Rc<Image>>, Vec<u8>)> = HashMap::new();
        for c in f.chunks.iter().filter(|c| tag_is(&c.tag, b"PICT")) {
            let meta = c.parts.iter().find(|p| tag_is(&p.tag, b"META"));
            let prev = c.parts.iter().find(|p| tag_is(&p.tag, b"PKBM"));
            if let Some(m) = meta {
                picts.insert(c.id, (prev.and_then(|p| image(p.data)), m.data.to_vec()));
            }
        }
        for c in f.chunks.iter().filter(|c| tag_is(&c.tag, b"CARO")) {
            let Some(list) = c.parts.iter().find(|p| tag_is(&p.tag, b"CARO")) else {
                continue;
            };
            let icon = c
                .parts
                .iter()
                .find(|p| tag_is(&p.tag, b"PKBM"))
                .and_then(|p| image(p.data));
            let items = parse_list(list.data)
                .into_iter()
                .filter_map(|id| picts.get(&id))
                .map(|(preview, meta)| ClipItem {
                    preview: preview.clone(),
                    meta: meta.clone(),
                })
                .collect::<Vec<_>>();
            if !items.is_empty() {
                rolls.push(ClipRoll {
                    id: c.id,
                    name: c.name.clone().unwrap_or_default(),
                    icon,
                    items,
                });
            }
        }
    }
    rolls.sort_by_key(|r| r.id);
    rolls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caro_list_layout() {
        let d = [3, 0, b'P', b'I', b'C', b'T', 5, 0, 6, 0, 7, 0];
        assert_eq!(parse_list(&d), vec![5, 6, 7]);
        // contagem maior que os dados: só o que existe
        let d = [9, 0, b'P', b'I', b'C', b'T', 1, 0];
        assert_eq!(parse_list(&d), vec![1]);
        assert!(parse_list(&[1]).is_empty());
    }

    #[test]
    fn no_libraries_no_rolls() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        assert!(load_rolls(&a).is_empty());
    }
}
