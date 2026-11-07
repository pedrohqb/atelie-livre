//! Rolos de adesivos de clip-art (bibliotecas `MAXMAG10.ART`, `MAX100.ART`,
//! `MAG100.ART` e `C-MAG100.ART`). Cada rolo é um chunk `CARO` (lista de
//! `PICT` + ícone); os números dos `CARO` formam uma sequência única entre as
//! bibliotecas (0 a 27, e 28 = cenários de `C-MAG100.ART`: Caverna, Fazenda,
//! Selva…), que é a ordem dos rolos no Fine Artist, depois do rolo de bichos.

use crate::Image;
use crate::assets::{Assets, Program};
use formats::render::bitmap_to_rgba;
use formats::{chnk, pkbm, wmf};
use std::collections::HashMap;
use std::rc::Rc;

pub const LIBRARIES: [&str; 4] = ["MAXMAG10.ART", "MAX100.ART", "MAG100.ART", "C-MAG100.ART"];

/// Desenho de um adesivo: metarquivo da biblioteca ou imagem já montada.
pub enum ClipArt {
    Meta(Vec<u8>),
    Image(Rc<Image>),
}

/// Um adesivo de clip-art: miniatura (para a tira) e o desenho.
pub struct ClipItem {
    pub preview: Option<Rc<Image>>,
    art: ClipArt,
}

impl ClipItem {
    /// Desenho em tamanho real (bitmap de 1 bit), decodificado na hora.
    pub fn art(&self) -> Option<Image> {
        match &self.art {
            ClipArt::Meta(m) => wmf::meta_dib(m).ok(),
            ClipArt::Image(i) => Some((**i).clone()),
        }
    }
}

/// Biblioteca dos balões (`B-MAG100.CHK`): silhuetas pequenas de balões e
/// de rabinhos ("dipitidus"); os rabinhos são os `PICT` 16 a 19 e o ícone do
/// rolo é o 20.
pub const BALLOONS_LIBRARY: &str = "B-MAG100.CHK";
const TAIL_PICTS: [u16; 4] = [16, 17, 18, 19];
const TAIL_ICON: u16 = 20;
/// Ampliação dos rabinhos como adesivos.
const TAIL_SCALE: u32 = 2;
/// Número do rolo de rabinhos (depois dos 29 de clip-art, 0 a 28).
pub const TAILS_ROLL_ID: u16 = 29;

/// Silhueta (preto = dentro) ampliada `scale` vezes, como desenho de traço:
/// contorno preto de 1 px e interior branco; o resto branco (transparente
/// ao virar adesivo).
pub fn outlined(img: &Image, scale: u32) -> Image {
    let (w, h) = (img.width * scale, img.height * scale);
    let inside = |x: i32, y: i32| {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            return false;
        }
        let k = (((y as u32 / scale) * img.width + x as u32 / scale) * 4) as usize;
        img.rgba.get(k).is_some_and(|&v| v < 128)
    };
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if !inside(x, y) {
                continue;
            }
            let edge = (-1..=1).any(|dy| (-1..=1).any(|dx| !inside(x + dx, y + dy)));
            if edge {
                let k = ((y as u32 * w + x as u32) * 4) as usize;
                rgba[k] = 0;
                rgba[k + 1] = 0;
                rgba[k + 2] = 0;
            }
        }
    }
    Image {
        width: w,
        height: h,
        rgba,
    }
}

/// Rolo de rabinhos de balão ("dipitidus") da história em quadrinhos; `None`
/// sem a biblioteca.
pub fn load_tails(assets: &Assets) -> Option<ClipRoll> {
    let data = assets.library_bytes(BALLOONS_LIBRARY)?;
    let f = chnk::parse(data).ok()?;
    let meta_of = |id: u16| {
        f.chunks
            .iter()
            .find(|c| tag_is(&c.tag, b"PICT") && c.id == id)
            .and_then(|c| c.parts.iter().find(|p| tag_is(&p.tag, b"META")))
            .and_then(|p| wmf::meta_dib(p.data).ok())
    };
    let items: Vec<ClipItem> = TAIL_PICTS
        .iter()
        .filter_map(|&id| {
            let raw = meta_of(id)?;
            Some(ClipItem {
                preview: Some(Rc::new(outlined(&raw, 1))),
                art: ClipArt::Image(Rc::new(outlined(&raw, TAIL_SCALE))),
            })
        })
        .collect();
    if items.is_empty() {
        return None;
    }
    Some(ClipRoll {
        id: TAILS_ROLL_ID,
        library: BALLOONS_LIBRARY,
        name: "Dipitidus".into(),
        icon: meta_of(TAIL_ICON).map(Rc::new),
        items,
    })
}

pub struct ClipRoll {
    /// Número do `CARO`.
    pub id: u16,
    /// Biblioteca de origem (`MAXMAG10.ART`, `MAX100.ART` ou `MAG100.ART`).
    pub library: &'static str,
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
                    art: ClipArt::Meta(meta.clone()),
                })
                .collect::<Vec<_>>();
            if !items.is_empty() {
                rolls.push(ClipRoll {
                    id: c.id,
                    library: name,
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
    fn outlined_keeps_edge_and_whitens_inside() {
        // quadrado preto 4×4 numa imagem 6×6
        let mut rgba = vec![255u8; 6 * 6 * 4];
        for y in 1..5 {
            for x in 1..5 {
                let k = (y * 6 + x) * 4;
                rgba[k] = 0;
                rgba[k + 1] = 0;
                rgba[k + 2] = 0;
            }
        }
        let img = Image {
            width: 6,
            height: 6,
            rgba,
        };
        let o = outlined(&img, 2);
        assert_eq!((o.width, o.height), (12, 12));
        let at = |x: usize, y: usize| o.rgba[(y * 12 + x) * 4];
        assert_eq!(at(0, 0), 255);
        assert_eq!(at(2, 2), 0);
        assert_eq!(at(2, 5), 0);
        assert_eq!(at(3, 5), 255);
        assert_eq!(at(5, 5), 255);
        assert_eq!(at(9, 9), 0);
    }

    #[test]
    fn no_libraries_no_rolls() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        assert!(load_rolls(&a).is_empty());
        assert!(load_tails(&a).is_none());
    }
}
