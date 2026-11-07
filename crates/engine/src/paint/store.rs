//! Desenhos em PNG e `.MAG`: gravar o quadro e abrir o último.

use super::canvas::{CANVAS_H, CANVAS_W, Canvas};
use super::objects::{Objects, Record, fingerprint, parse_json, to_json};
use formats::mag;
use formats::palette::Rgb;
use std::io;
use std::path::Path;

pub const LAST_NAME: &str = "ultimo.png";
/// Nome que o usuário deu ao último quadro, ao lado de `ultimo.png`.
pub const NAME_FILE: &str = "ultimo.nome.txt";
/// Título dos desenhos gravados em `.MAG`.
pub const MAG_TITLE: &str = "Quadro Novo";

/// Grava o quadro como PNG RGB, convertendo os índices pela paleta.
pub fn save_png(c: &Canvas, pal: &[Rgb], path: &Path) -> io::Result<()> {
    let mut rgb = Vec::with_capacity(c.px.len() * 3);
    for &i in &c.px {
        let p = pal
            .get(i as usize)
            .copied()
            .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
        rgb.extend_from_slice(&[p.r, p.g, p.b]);
    }
    // grava num temporário e renomeia: uma queda no meio não estraga o arquivo anterior
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    {
        let file = io::BufWriter::new(std::fs::File::create(&tmp)?);
        let mut enc = png::Encoder::new(file, c.w, c.h);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(io::Error::other)?;
        w.write_image_data(&rgb).map_err(io::Error::other)?;
        w.finish().map_err(io::Error::other)?;
    }
    std::fs::rename(&tmp, path)
}

pub use formats::palette::nearest;

/// Abre um desenho de 594×347 (RGB ou RGBA de 8 bits); outro tamanho ou erro → `None`.
pub fn load_png(path: &Path, pal: &[Rgb]) -> Option<Canvas> {
    let file = std::fs::File::open(path).ok()?;
    let mut dec = png::Decoder::new(io::BufReader::new(file));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().ok()?;
    // conferir o tamanho antes de alocar: um cabeçalho forjado pediria gigabytes
    let (w, h) = (reader.info().width, reader.info().height);
    if (w, h) != (CANVAS_W, CANVAS_H) {
        return None;
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if (info.width, info.height) != (CANVAS_W, CANVAS_H) {
        return None;
    }
    let step = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return None,
    };
    let mut cache = std::collections::HashMap::new();
    let mut c = Canvas::new(info.width, info.height);
    for (k, px) in buf[..info.buffer_size()].chunks_exact(step).enumerate() {
        let key = (px[0], px[1], px[2]);
        c.px[k] = *cache
            .entry(key)
            .or_insert_with(|| nearest(pal, key.0, key.1, key.2));
    }
    Some(c)
}

/// Grava um adesivo do usuário como PNG RGBA (transparente fora da forma),
/// para `load_stamp_png` reabrir; troca o arquivo de uma vez.
pub fn save_stamp_png(st: &super::stamp::Stamp, pal: &[Rgb], path: &Path) -> io::Result<()> {
    let mut rgba = Vec::with_capacity((st.w * st.h * 4) as usize);
    for p in &st.px {
        match p {
            Some(i) => {
                let c = pal
                    .get(*i as usize)
                    .copied()
                    .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                rgba.extend_from_slice(&[c.r, c.g, c.b, 255]);
            }
            None => rgba.extend_from_slice(&[255, 255, 255, 0]),
        }
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    {
        let file = std::fs::File::create(&tmp)?;
        let mut enc = png::Encoder::new(io::BufWriter::new(file), st.w, st.h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(io::Error::other)?;
        w.write_image_data(&rgba).map_err(io::Error::other)?;
    }
    std::fs::rename(&tmp, path)
}

/// Abre um PNG de qualquer tamanho (até 594×347) como carimbo: alfa 0 e o
/// branco puro ficam transparentes; as cores vão para a paleta.
pub fn load_stamp_png(path: &Path, pal: &[Rgb]) -> Option<super::stamp::Stamp> {
    let file = std::fs::File::open(path).ok()?;
    let mut dec = png::Decoder::new(io::BufReader::new(file));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().ok()?;
    let (w, h) = (reader.info().width, reader.info().height);
    if w == 0 || h == 0 || w > CANVAS_W || h > CANVAS_H {
        return None;
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let step = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return None,
    };
    let mut cache = std::collections::HashMap::new();
    let px = buf[..info.buffer_size()]
        .chunks_exact(step)
        .map(|p| {
            // com canal alfa só ele diz o que é transparente; sem ele, o branco
            if (step == 4 && p[3] == 0) || (step == 3 && (p[0], p[1], p[2]) == (255, 255, 255)) {
                return None;
            }
            let key = (p[0], p[1], p[2]);
            Some(
                *cache
                    .entry(key)
                    .or_insert_with(|| nearest(pal, key.0, key.1, key.2)),
            )
        })
        .collect();
    Some(super::stamp::Stamp {
        w: info.width,
        h: info.height,
        px,
    })
}

/// Abre um desenho `.MAG` do Fine Artist de 594×347; as cores da paleta do
/// documento são aproximadas à do programa. Outro tamanho ou erro → `None`.
pub fn load_mag(path: &Path, pal: &[Rgb]) -> Option<Canvas> {
    let data = std::fs::read(path).ok()?;
    let pic = mag::read(&data).ok()?;
    if (pic.width as u32, pic.height as u32) != (CANVAS_W, CANVAS_H) {
        return None;
    }
    let mut c = Canvas::new(CANVAS_W, CANVAS_H);
    if pic.palette == pal {
        c.px.copy_from_slice(&pic.pixels);
        return Some(c);
    }
    let mut map = [None; 256];
    for (dst, &i) in c.px.iter_mut().zip(&pic.pixels) {
        *dst = *map[i as usize].get_or_insert_with(|| {
            let p = pic.palette[i as usize];
            nearest(pal, p.r, p.g, p.b)
        });
    }
    Some(c)
}

/// Abre um desenho pela extensão: `.png` ou `.mag` (sem distinguir maiúsculas).
pub fn load_picture(path: &Path, pal: &[Rgb]) -> Option<Canvas> {
    let ext = path.extension()?.to_str()?;
    if ext.eq_ignore_ascii_case("png") {
        load_png(path, pal)
    } else if ext.eq_ignore_ascii_case("mag") {
        load_mag(path, pal)
    } else {
        None
    }
}

/// Grava o quadro como `.MAG` do Fine Artist (título "Quadro Novo", sem autor).
pub fn save_mag(c: &Canvas, pal: &[Rgb], path: &Path) -> io::Result<()> {
    save_mag_titled(c, pal, path, MAG_TITLE)
}

/// Como `save_mag`, com o título dado pelo usuário.
pub fn save_mag_titled(c: &Canvas, pal: &[Rgb], path: &Path, title: &str) -> io::Result<()> {
    let mut palette = pal.iter().take(256).copied().collect::<Vec<_>>();
    palette.resize(256, Rgb { r: 0, g: 0, b: 0 });
    let pic = mag::Picture {
        title: title.into(),
        author: String::new(),
        palette,
        width: c.w as u16,
        height: c.h as u16,
        pixels: c.px.clone(),
    };
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, mag::write(&pic))?;
    std::fs::rename(&tmp, path)
}

/// Nome do arquivo de objetos de um desenho (`ultimo.objetos.json`).
pub fn objects_name(stem: &str) -> String {
    format!("{stem}.objetos.json")
}

/// Nome do fundo (quadro sem os objetos) de um desenho (`ultimo.fundo.png`).
pub fn background_name(stem: &str) -> String {
    format!("{stem}.fundo.png")
}

/// Grava, ao lado de `<stem>.png` (já achatado), os objetos em
/// `<stem>.objetos.json` e o fundo em `<stem>.fundo.png`; sem objetos, apaga
/// os dois.
pub fn save_objects(
    dir: &Path,
    stem: &str,
    background: &Canvas,
    objs: &Objects,
    composed: &Canvas,
    pal: &[Rgb],
) -> io::Result<()> {
    let json = dir.join(objects_name(stem));
    let bg = dir.join(background_name(stem));
    if objs.is_empty() {
        let _ = std::fs::remove_file(&json);
        let _ = std::fs::remove_file(&bg);
        return Ok(());
    }
    save_png(background, pal, &bg)?;
    let text = to_json(&objs.records(), fingerprint(composed, pal));
    let mut tmp = json.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &json)
}

/// Abre `<stem>.png`; se `<stem>.objetos.json` existe, bate com o PNG
/// (impressão digital) e `<stem>.fundo.png` abre, devolve o fundo e os
/// objetos a remontar; senão o PNG achatado e nenhum objeto.
pub fn load_drawing(dir: &Path, stem: &str, pal: &[Rgb]) -> Option<(Canvas, Vec<Record>)> {
    let flat = load_png(&dir.join(format!("{stem}.png")), pal)?;
    let editable = std::fs::read_to_string(dir.join(objects_name(stem)))
        .ok()
        .and_then(|t| parse_json(&t))
        .filter(|(fp, recs)| !recs.is_empty() && *fp == fingerprint(&flat, pal))
        .and_then(|(_, recs)| {
            let bg = load_png(&dir.join(background_name(stem)), pal)?;
            Some((bg, recs))
        });
    Some(editable.unwrap_or((flat, Vec::new())))
}

/// `desenho-AAAAMMDD-HHMMSS.png` em UTC.
pub fn timestamp_name(unix_secs: u64) -> String {
    format!("desenho-{}.png", timestamp(unix_secs))
}

/// `desenho-AAAAMMDD-HHMMSS` em UTC.
pub fn timestamp_stem(unix_secs: u64) -> String {
    format!("desenho-{}", timestamp(unix_secs))
}

/// `AAAAMMDD-HHMMSS` em UTC, para os nomes dos arquivos guardados.
pub fn timestamp(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let rem = unix_secs % 86_400;
    // civil_from_days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Grava `ultimo.png` e, se o quadro mudou, também uma cópia com data e hora
/// em PNG e em `.MAG` (o formato do Fine Artist).
pub fn save_drawing(
    dir: &Path,
    c: &Canvas,
    pal: &[Rgb],
    changed: bool,
    now: u64,
) -> Result<(), String> {
    save_drawing_objects(dir, c, &Objects::default(), pal, changed, now)
}

/// Como `save_drawing`, com os objetos: os PNG e o `.MAG` recebem a
/// composição achatada; `ultimo.objetos.json` e `ultimo.fundo.png` (e as
/// cópias com data) guardam os objetos e o fundo para reabrir editável.
pub fn save_drawing_objects(
    dir: &Path,
    c: &Canvas,
    objs: &Objects,
    pal: &[Rgb],
    changed: bool,
    now: u64,
) -> Result<(), String> {
    save_drawing_named(dir, c, objs, pal, changed, now, None)
}

/// Como `save_drawing_objects`, com o nome que o usuário deu ao quadro:
/// a cópia (sempre gravada, mesmo sem mudança) fica em `<nome>.png` e
/// `<nome>.mag` (com esse título) em vez do carimbo de data, e o nome vai
/// para `ultimo.nome.txt`, para o rótulo voltar ao reabrir.
pub fn save_drawing_named(
    dir: &Path,
    c: &Canvas,
    objs: &Objects,
    pal: &[Rgb],
    changed: bool,
    now: u64,
    name: Option<&str>,
) -> Result<(), String> {
    let fail = |e: io::Error, p: &Path| format!("não consegui gravar {}: {e}", p.display());
    std::fs::create_dir_all(dir).map_err(|e| fail(e, dir))?;
    let flat = compose_unclipped(c, objs);
    let last = dir.join(LAST_NAME);
    save_png(&flat, pal, &last).map_err(|e| fail(e, &last))?;
    save_objects(dir, "ultimo", c, objs, &flat, pal).map_err(|e| fail(e, dir))?;
    let name_file = dir.join(NAME_FILE);
    match name {
        Some(n) => std::fs::write(&name_file, n).map_err(|e| fail(e, &name_file))?,
        None => {
            let _ = std::fs::remove_file(&name_file);
        }
    }
    if changed || name.is_some() {
        let (stem, title) = match name {
            Some(n) => (name_stem(n).unwrap_or_else(|| timestamp_stem(now)), n),
            None => (timestamp_stem(now), MAG_TITLE),
        };
        let named = dir.join(format!("{stem}.png"));
        save_png(&flat, pal, &named).map_err(|e| fail(e, &named))?;
        let named = dir.join(format!("{stem}.mag"));
        save_mag_titled(&flat, pal, &named, title).map_err(|e| fail(e, &named))?;
        save_objects(dir, &stem, c, objs, &flat, pal).map_err(|e| fail(e, dir))?;
    }
    Ok(())
}

/// Nome do arquivo para um nome de quadro: só o que é seguro num caminho
/// (sem barras, pontos no início nem caracteres de controle); `None` para o
/// nome padrão ("Quadro Novo"), que fica com o carimbo de data.
pub fn name_stem(name: &str) -> Option<String> {
    let t = name.trim();
    if t.is_empty() || t == MAG_TITLE {
        return None;
    }
    let s: String = t
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let s = s.trim_start_matches('.').to_string();
    (!s.is_empty()).then_some(s)
}

/// Nome dado ao último quadro (`ultimo.nome.txt`), se houver.
pub fn load_name(dir: &Path) -> Option<String> {
    let s = std::fs::read_to_string(dir.join(NAME_FILE)).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// Composição achatada para gravar: o recorte retangular (quadrinho em
/// edição) não vale, só a máscara de recorte (forma do botão/adesivo).
pub fn compose_unclipped(c: &Canvas, objs: &Objects) -> Canvas {
    let mut out = c.clone();
    out.clip = None;
    objs.draw_onto(&mut out);
    out.clip = c.clip;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pal() -> Vec<Rgb> {
        (0..256u32)
            .map(|i| Rgb {
                r: (255 - i) as u8,
                g: (i * 7 % 256) as u8,
                b: (i * 13 % 256) as u8,
            })
            .collect()
    }

    #[test]
    fn png_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.png");
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        for (k, v) in c.px.iter_mut().enumerate() {
            *v = (k % 256) as u8;
        }
        save_png(&c, &pal(), &p).unwrap();
        assert_eq!(load_png(&p, &pal()), Some(c));
    }

    #[test]
    fn load_wrong_size_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.png");
        save_png(&Canvas::new(10, 10), &pal(), &p).unwrap();
        assert_eq!(load_png(&p, &pal()), None);
    }

    #[test]
    fn load_garbage_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.png");
        std::fs::write(&p, b"\x89PNG lixo").unwrap();
        assert_eq!(load_png(&p, &pal()), None);
        assert_eq!(load_png(&dir.path().join("nao-existe.png"), &pal()), None);
    }

    #[test]
    fn load_huge_header_is_none_without_allocating() {
        // PNG de 60000×60000 com IDAT minúsculo: deve ser recusado antes de alocar a saída
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.png");
        let file = std::fs::File::create(&p).unwrap();
        let mut enc = png::Encoder::new(file, 60_000, 60_000);
        enc.set_color(png::ColorType::Rgba);
        let mut w = enc.write_header().unwrap();
        // só o cabeçalho e um IDAT qualquer; a escrita incompleta é proposital
        let _ = w.write_chunk(
            png::chunk::IDAT,
            &[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
        );
        drop(w);
        assert_eq!(load_png(&p, &pal()), None);
    }

    #[test]
    fn load_stamp_png_makes_white_transparent() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.png");
        let mut c = Canvas::new(2, 1);
        c.px[1] = 7;
        // índice 0 da paleta de teste não é branco puro: usar uma paleta com branco no 0
        let mut pal = pal();
        pal[0] = Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        save_png(&c, &pal, &p).unwrap();
        let s = load_stamp_png(&p, &pal).unwrap();
        assert_eq!((s.w, s.h), (2, 1));
        assert_eq!(s.px, vec![None, Some(7)]);
        assert_eq!(load_stamp_png(&dir.path().join("nao.png"), &pal), None);
    }

    #[test]
    fn save_leaves_no_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.png");
        save_png(&Canvas::new(3, 3), &pal(), &p).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn timestamp_name_known_date() {
        assert_eq!(timestamp_name(1_793_404_800), "desenho-20261031-000000.png");
        assert_eq!(timestamp_name(0), "desenho-19700101-000000.png");
        assert_eq!(
            timestamp_name(951_782_400 + 3_661),
            "desenho-20000229-010101.png"
        );
    }

    #[test]
    fn save_drawing_writes_both_when_changed() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().join("x/desenhos");
        let c = Canvas::new(CANVAS_W, CANVAS_H);
        save_drawing(&d, &c, &pal(), false, 0).unwrap();
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1);
        save_drawing(&d, &c, &pal(), true, 1_793_404_800).unwrap();
        assert!(d.join(LAST_NAME).exists());
        assert!(d.join("desenho-20261031-000000.png").exists());
        let m = d.join("desenho-20261031-000000.mag");
        assert!(m.exists());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 3);
        let pic = formats::mag::read(&std::fs::read(&m).unwrap()).unwrap();
        assert_eq!(pic.title, MAG_TITLE);
        assert_eq!(pic.author, "");
        assert_eq!(pic.palette, pal());
        assert_eq!(pic.pixels, c.px);
    }

    #[test]
    fn mag_roundtrip_and_load_picture() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        for (k, v) in c.px.iter_mut().enumerate() {
            *v = (k % 253) as u8;
        }
        let m = dir.path().join("a.MAG");
        save_mag(&c, &pal(), &m).unwrap();
        assert_eq!(load_mag(&m, &pal()), Some(c.clone()));
        assert_eq!(load_picture(&m, &pal()), Some(c.clone()));
        let p = dir.path().join("a.png");
        save_png(&c, &pal(), &p).unwrap();
        assert_eq!(load_picture(&p, &pal()), Some(c.clone()));
        assert_eq!(load_picture(&dir.path().join("a.txt"), &pal()), None);
        assert_eq!(load_picture(&dir.path().join("a"), &pal()), None);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn load_mag_with_other_palette_uses_nearest_colour() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(3);
        // paleta invertida: o índice 3 da gravação tem a cor do índice 252 da nossa
        let inv: Vec<Rgb> = pal().into_iter().rev().collect();
        let m = dir.path().join("b.mag");
        save_mag(&c, &inv, &m).unwrap();
        let got = load_mag(&m, &pal()).unwrap();
        assert_eq!(got.px[0], 252);
    }

    #[test]
    fn load_mag_wrong_size_or_garbage_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let m = dir.path().join("c.mag");
        save_mag(&Canvas::new(10, 10), &pal(), &m).unwrap();
        assert_eq!(load_mag(&m, &pal()), None);
        std::fs::write(&m, b"CHNK lixo").unwrap();
        assert_eq!(load_mag(&m, &pal()), None);
        assert_eq!(load_mag(&dir.path().join("nao.mag"), &pal()), None);
    }

    #[test]
    fn save_with_objects_writes_json_and_background_and_reopens() {
        use super::super::objects::Source;
        use super::super::stamp::Stamp;
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().to_path_buf();
        let c = Canvas::new(CANVAS_W, CANVAS_H);
        let mut objs = Objects::default();
        let img = std::rc::Rc::new(Stamp {
            w: 2,
            h: 2,
            px: vec![Some(3); 4],
        });
        objs.place(img, 10, 20, Source::Roll { roll: 2, item: 1 });
        save_drawing_objects(&d, &c, &objs, &pal(), true, 1_793_404_800).unwrap();
        let mut names: Vec<String> = std::fs::read_dir(&d)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "desenho-20261031-000000.fundo.png",
                "desenho-20261031-000000.mag",
                "desenho-20261031-000000.objetos.json",
                "desenho-20261031-000000.png",
                "ultimo.fundo.png",
                "ultimo.objetos.json",
                "ultimo.png",
            ]
        );
        // o PNG está achatado, o fundo não
        assert_eq!(
            load_png(&d.join(LAST_NAME), &pal()).unwrap().get(10, 20),
            Some(3)
        );
        let (bg, recs) = load_drawing(&d, "ultimo", &pal()).unwrap();
        assert_eq!(bg.get(10, 20), Some(0));
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].source, Source::Roll { roll: 2, item: 1 });
        // outro ultimo.png (quadro novo) invalida o JSON: abre achatado
        save_png(&c, &pal(), &d.join(LAST_NAME)).unwrap();
        let (bg, recs) = load_drawing(&d, "ultimo", &pal()).unwrap();
        assert_eq!(bg.get(10, 20), Some(0));
        assert!(recs.is_empty());
        // gravar sem objetos apaga o JSON e o fundo
        save_drawing_objects(&d, &c, &Objects::default(), &pal(), false, 0).unwrap();
        assert!(!d.join("ultimo.objetos.json").exists());
        assert!(!d.join("ultimo.fundo.png").exists());
    }

    #[test]
    fn save_drawing_unwritable_dir_is_err() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arquivo");
        std::fs::write(&file, b"x").unwrap();
        let c = Canvas::new(2, 2);
        let e = save_drawing(&file.join("sub"), &c, &pal(), true, 0).unwrap_err();
        assert!(e.contains("não consegui gravar"));
    }
}
