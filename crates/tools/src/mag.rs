//! `atelie-tools mag2png` e `png2mag`: conversão entre desenhos `.MAG` e PNG.

use formats::palette::{self, Rgb};
use formats::render::Palettes;
use formats::{mag, ne};
use std::collections::HashMap;
use std::io;
use std::path::Path;

/// Converte um `.MAG` em PNG RGB (os objetos soltos já estão na pintura).
pub fn mag2png(input: &Path, output: &Path) -> Result<(), String> {
    let data = std::fs::read(input).map_err(|e| format!("{}: {e}", input.display()))?;
    let pic = mag::read(&data).map_err(|e| format!("{}: {e}", input.display()))?;
    let black = Rgb { r: 0, g: 0, b: 0 };
    let mut rgb = Vec::with_capacity(pic.pixels.len() * 3);
    for &i in &pic.pixels {
        let c = pic.palette.get(i as usize).copied().unwrap_or(black);
        rgb.extend_from_slice(&[c.r, c.g, c.b]);
    }
    write_png(output, pic.width as u32, pic.height as u32, &rgb)
        .map_err(|e| format!("{}: {e}", output.display()))?;
    eprintln!(
        "{}: {}×{}, título \"{}\", autor \"{}\"",
        input.display(),
        pic.width,
        pic.height,
        pic.title,
        pic.author
    );
    Ok(())
}

/// Converte um PNG em `.MAG`. As cores são aproximadas à paleta de 256 cores
/// do `ARTIST.EXE` da pasta `data` (ou `ATELIE_DATA`); sem ela, o PNG precisa
/// ter até 256 cores, que viram a paleta do desenho.
pub fn png2mag(
    input: &Path,
    output: &Path,
    title: &str,
    author: &str,
    data: Option<&Path>,
) -> Result<(), String> {
    let (w, h, rgb) = read_png(input).map_err(|e| format!("{}: {e}", input.display()))?;
    if w > u16::MAX as u32 || h > u16::MAX as u32 {
        return Err(format!(
            "{}: imagem de {w}×{h} grande demais",
            input.display()
        ));
    }
    if (w, h) != (mag::PICTURE_W as u32, mag::PICTURE_H as u32) {
        eprintln!(
            "aviso: {} tem {w}×{h}; o Fine Artist grava desenhos de {}×{}",
            input.display(),
            mag::PICTURE_W,
            mag::PICTURE_H
        );
    }
    let pal = match data {
        Some(dir) => artist_palette(dir)?,
        None => own_palette(&rgb).ok_or_else(|| {
            format!(
                "{}: mais de 256 cores; indique a pasta MSKIDS com --data ou ATELIE_DATA",
                input.display()
            )
        })?,
    };
    let mut cache = HashMap::new();
    let pixels = rgb
        .chunks_exact(3)
        .map(|c| {
            *cache
                .entry((c[0], c[1], c[2]))
                .or_insert_with(|| palette::nearest(&pal, c[0], c[1], c[2]))
        })
        .collect();
    let pic = mag::Picture {
        title: title.into(),
        author: author.into(),
        palette: pal,
        width: w as u16,
        height: h as u16,
        pixels,
    };
    std::fs::write(output, mag::write(&pic)).map_err(|e| format!("{}: {e}", output.display()))
}

/// Paleta `LOGPALETTE` 8 do `ARTIST.EXE` da instalação.
fn artist_palette(dir: &Path) -> Result<Vec<Rgb>, String> {
    let exe = engine::paths::find_ci(dir, "ARTIST.EXE")
        .ok_or_else(|| format!("{}: ARTIST.EXE não encontrado", dir.display()))?;
    let data = std::fs::read(&exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    ne::parse(&data).map_err(|e| format!("{}: {e}", exe.display()))?;
    let pal = Palettes::from_exe(Some(&data)).pal256;
    if pal.len() != 256 {
        return Err(format!("{}: sem LOGPALETTE de 256 cores", exe.display()));
    }
    Ok(pal)
}

/// As cores do próprio PNG, na ordem em que aparecem, completadas com preto
/// até 256; `None` se houver mais de 256.
fn own_palette(rgb: &[u8]) -> Option<Vec<Rgb>> {
    let mut pal = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for c in rgb.chunks_exact(3) {
        if seen.insert((c[0], c[1], c[2])) {
            if pal.len() == 256 {
                return None;
            }
            pal.push(Rgb {
                r: c[0],
                g: c[1],
                b: c[2],
            });
        }
    }
    pal.resize(256, Rgb { r: 0, g: 0, b: 0 });
    Some(pal)
}

/// Lê um PNG qualquer como RGB de 8 bits.
fn read_png(path: &Path) -> io::Result<(u32, u32, Vec<u8>)> {
    let file = std::fs::File::open(path)?;
    let mut dec = png::Decoder::new(io::BufReader::new(file));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().map_err(io::Error::other)?;
    let (w, h) = (reader.info().width, reader.info().height);
    if w as u64 * h as u64 > 1 << 24 {
        return Err(io::Error::other(format!("imagem de {w}×{h} grande demais")));
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(io::Error::other)?;
    let step = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Indexed => {
            return Err(io::Error::other("PNG indexado não expandido"));
        }
    };
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for px in buf[..info.buffer_size()].chunks_exact(step) {
        if step <= 2 {
            rgb.extend_from_slice(&[px[0], px[0], px[0]]);
        } else {
            rgb.extend_from_slice(&px[..3]);
        }
    }
    Ok((info.width, info.height, rgb))
}

fn write_png(path: &Path, width: u32, height: u32, rgb: &[u8]) -> io::Result<()> {
    let file = io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, width, height);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(io::Error::other)?;
    w.write_image_data(rgb).map_err(io::Error::other)?;
    w.finish().map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_with(
        dir: &Path,
        name: &str,
        w: u32,
        h: u32,
        f: impl Fn(u32, u32) -> [u8; 3],
    ) -> std::path::PathBuf {
        let p = dir.join(name);
        let mut rgb = Vec::new();
        for y in 0..h {
            for x in 0..w {
                rgb.extend_from_slice(&f(x, y));
            }
        }
        write_png(&p, w, h, &rgb).unwrap();
        p
    }

    #[test]
    fn png_to_mag_and_back_without_installation() {
        let d = tempfile::tempdir().unwrap();
        let png = png_with(d.path(), "a.png", 594, 347, |x, y| {
            [(x % 7 * 30) as u8, (y % 5 * 50) as u8, 0]
        });
        let m = d.path().join("a.mag");
        png2mag(&png, &m, "Teste Ção", "Eu", None).unwrap();
        let pic = mag::read(&std::fs::read(&m).unwrap()).unwrap();
        assert_eq!(pic.title, "Teste Ção");
        assert_eq!(pic.author, "Eu");
        assert_eq!((pic.width, pic.height), (594, 347));
        let back = d.path().join("b.png");
        mag2png(&m, &back).unwrap();
        let (w, h, rgb) = read_png(&back).unwrap();
        assert_eq!((w, h), (594, 347));
        let (_, _, orig) = read_png(&png).unwrap();
        assert_eq!(rgb, orig);
    }

    #[test]
    fn png_with_too_many_colours_needs_installation() {
        let d = tempfile::tempdir().unwrap();
        let png = png_with(d.path(), "a.png", 64, 64, |x, y| {
            [x as u8 * 4, y as u8 * 4, 7]
        });
        let e = png2mag(&png, &d.path().join("a.mag"), "", "", None).unwrap_err();
        assert!(e.contains("256 cores"), "{e}");
        assert!(own_palette(&[1, 2, 3, 1, 2, 3]).unwrap().len() == 256);
    }

    #[test]
    fn errors_name_the_file() {
        let d = tempfile::tempdir().unwrap();
        let missing = d.path().join("nada.mag");
        assert!(
            mag2png(&missing, &d.path().join("x.png"))
                .unwrap_err()
                .contains("nada.mag")
        );
        std::fs::write(&missing, b"lixo").unwrap();
        assert!(
            mag2png(&missing, &d.path().join("x.png"))
                .unwrap_err()
                .contains("nada.mag")
        );
        assert!(png2mag(&missing, &d.path().join("x.mag"), "", "", None).is_err());
        assert!(artist_palette(d.path()).unwrap_err().contains("ARTIST.EXE"));
    }

    #[test]
    fn grayscale_png_is_accepted() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("g.png");
        let file = std::fs::File::create(&p).unwrap();
        let mut enc = png::Encoder::new(file, 2, 1);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[0, 200]).unwrap();
        w.finish().unwrap();
        assert_eq!(read_png(&p).unwrap().2, [0, 0, 0, 200, 200, 200]);
    }
}
