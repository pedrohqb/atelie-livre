//! Os desenhos guardados, como as telas os listam: a pasta de desenhos da
//! criança, os exemplos da instalação, as miniaturas e o título e o autor
//! gravados no `.MAG`. Usado pela Galeria, pelo fichário verde, pelo
//! Cachorrinho dos adesivos e pela pasta do desenho animado.

use super::Canvas;
use super::store::LAST_NAME;
use crate::paths::find_ci;
use formats::palette::Rgb;
use std::path::{Path, PathBuf};

fn has_ext(p: &Path, ext: &str) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

/// Prefixo dos adesivos do usuário na pasta de desenhos (ver
/// `scenes::stickers::USER_PREFIX`).
pub const USER_STICKER_PREFIX: &str = "meu-adesivo-";

/// Arquivos de trabalho que não aparecem nas listas: os "últimos" reabertos
/// pela pintura e pelos projetos (`ultimo*.png`), as páginas de impressão e
/// os adesivos do usuário (que ficam no rolo "Meus adesivos").
fn is_listed(name: &std::ffi::OsStr) -> bool {
    let n = name.to_string_lossy();
    n != LAST_NAME
        && !n.starts_with("ultimo")
        && !n.starts_with("impressao-")
        && !n.starts_with(USER_STICKER_PREFIX)
}

/// Desenhos da pasta (`.png` e `.mag`), mais recentes primeiro (`ultimo.png`
/// fora, pois repete o desenho mais recente; `*.fundo.png` fora, pois é o
/// fundo sem os objetos). Um `.mag` fica de fora quando há um `.png` de mesmo
/// nome: são o mesmo desenho gravado duas vezes.
pub fn list_drawings(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let all: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    let is_background = |p: &Path| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".fundo.png"))
    };
    let mut files: Vec<PathBuf> = all
        .iter()
        .filter(|p| {
            (has_ext(p, "png") && p.file_name().is_some_and(is_listed) && !is_background(p))
                || (has_ext(p, "mag") && !all.contains(&p.with_extension("png")))
        })
        .cloned()
        .collect();
    files.sort();
    files.reverse();
    files
}

/// Desenhos de exemplo da instalação: `SHARED/USERS/*/*.MAG`, em ordem.
pub fn list_samples(data_dir: &Path) -> Vec<PathBuf> {
    let Some(users) = find_ci(data_dir, "SHARED/USERS") else {
        return Vec::new();
    };
    let Ok(rd) = std::fs::read_dir(&users) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let mut files = Vec::new();
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        let mut here: Vec<PathBuf> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| has_ext(p, "mag"))
            .collect();
        here.sort();
        files.extend(here);
    }
    files
}

/// Título e autor gravados no `.MAG` do desenho (o próprio arquivo ou o
/// `.mag` gêmeo de um `.png`), sem espaços nas pontas.
pub fn mag_meta(path: &Path) -> Option<(String, String)> {
    let mag = if has_ext(path, "mag") {
        path.to_path_buf()
    } else {
        path.with_extension("mag")
    };
    let m = std::fs::read(&mag)
        .ok()
        .and_then(|d| formats::mag::read(&d).ok())?;
    Some((m.title.trim().to_string(), m.author.trim().to_string()))
}

/// Miniatura 1/`n`: um pixel de cada bloco `n`×`n` (o do canto).
pub fn shrink(c: &Canvas, n: u32) -> Canvas {
    let (w, h) = (c.w / n, c.h / n);
    let mut t = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            t.px[(y * w + x) as usize] = c.px[((y * n) * c.w + x * n) as usize];
        }
    }
    t
}

/// Miniatura 1/`n` em que de cada bloco fica o pixel mais escuro, para as
/// linhas finas não sumirem (o original reduz com o `StretchBlt` do
/// Windows, que preserva o preto).
pub fn shrink_darkest(c: &Canvas, n: u32, pal: &[Rgb]) -> Canvas {
    let (w, h) = (c.w / n, c.h / n);
    let luma = |i: u8| {
        pal.get(i as usize)
            .map_or(0, |p| 3 * p.r as u32 + 6 * p.g as u32 + p.b as u32)
    };
    let mut t = Canvas::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let mut best = c.px[((y * n) * c.w + x * n) as usize];
            for dy in 0..n {
                for dx in 0..n {
                    let (sx, sy) = (x * n + dx, y * n + dy);
                    if sx < c.w && sy < c.h {
                        let v = c.px[(sy * c.w + sx) as usize];
                        if luma(v) < luma(best) {
                            best = v;
                        }
                    }
                }
            }
            t.px[(y * w + x) as usize] = best;
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> Canvas {
        // 8×4: coluna 1 preta (índice 0) sobre branco (índice 1)
        let mut c = Canvas::new(8, 4);
        c.px = vec![1; 32];
        for y in 0..4 {
            c.px[y * 8 + 1] = 0;
        }
        c
    }

    #[test]
    fn shrink_takes_the_corner_and_darkest_keeps_thin_lines() {
        let pal = vec![
            Rgb { r: 0, g: 0, b: 0 },
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
        ];
        let t = shrink(&canvas(), 4);
        assert_eq!((t.w, t.h), (2, 1));
        assert_eq!(t.px, vec![1, 1]);
        let t = shrink_darkest(&canvas(), 4, &pal);
        assert_eq!(t.px, vec![0, 1]);
        assert_eq!(shrink(&canvas(), 2).px, vec![1, 1, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn listing_skips_work_files_and_twins() {
        let d = tempfile::tempdir().unwrap();
        for n in [
            "desenho-1.png",
            "desenho-1.mag",
            "velho.MAG",
            "ultimo.png",
            "ultimo-livro.png",
            "impressao-1.png",
            "desenho-2.fundo.png",
            "nota.txt",
        ] {
            std::fs::write(d.path().join(n), b"x").unwrap();
        }
        let names: Vec<String> = list_drawings(d.path())
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["velho.MAG", "desenho-1.png"]);
        assert!(mag_meta(&d.path().join("nota.txt")).is_none());
    }
}
