//! `atelie-tools extract`: extrai tudo de uma pasta instalada.

use crate::detect::{FileKind, detect};
use crate::html::{self, Section};
use crate::paths::{find_ci, relative, safe_component, walk_files};
use crate::report::{Failure, FileSummary, Report};
use formats::{Rgb, chnk, glob, macres, ne, palette, pkbm, wave, winres};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

/// Arquivos esperados numa instalação, na ordem de processamento.
const EXPECTED: [&str; 6] = [
    "ARTIST.EXE",
    "WRITER.EXE",
    "ARTIST/ARTIST.PIC",
    "ARTIST/ARTIST.SND",
    "WRITER/WRITER.PIC",
    "WRITER/WRITER.SND",
];

struct Palettes {
    pal16: Vec<Rgb>,
    pal256: Vec<Rgb>,
}

impl Palettes {
    /// Lê as `LOGPALETTE` 4 e 8 de um executável; sem ele, uma rampa de cinzas.
    fn from_exe(path: Option<&PathBuf>) -> Palettes {
        let read = |id: u16| -> Option<Vec<Rgb>> {
            let data = std::fs::read(path?).ok()?;
            let f = ne::parse(&data).ok()?;
            palette::parse_logpalette(f.find("LOGPALETTE", id)?.data).ok()
        };
        let gray = |n: usize| -> Vec<Rgb> {
            (0..n)
                .map(|i| {
                    let v = (255 - i * 255 / (n - 1)) as u8;
                    Rgb { r: v, g: v, b: v }
                })
                .collect()
        };
        Palettes {
            pal16: read(4).unwrap_or_else(|| gray(16)),
            pal256: read(8).unwrap_or_else(|| gray(256)),
        }
    }
}

struct Ctx<'a> {
    input: &'a Path,
    output: &'a Path,
    report: Report,
    sections: Vec<Section>,
}

/// Contadores de um arquivo de origem.
struct Tally<'a> {
    path: &'a str,
    ok: usize,
    failed: usize,
}

impl Ctx<'_> {
    fn fail(&mut self, t: &mut Tally, item: String, error: String) {
        t.failed += 1;
        self.report.failures.push(Failure {
            path: t.path.to_string(),
            item,
            error,
        });
    }

    fn rel_out(&self, p: &Path) -> String {
        relative(self.output, p)
    }
}

pub fn run(input: &Path, output: &Path) -> io::Result<Report> {
    std::fs::create_dir_all(output)?;
    let out_canon = output.canonicalize()?;
    let mut ctx = Ctx {
        input,
        output,
        report: Report::default(),
        sections: Vec::new(),
    };
    let found: Vec<Option<PathBuf>> = EXPECTED.iter().map(|rel| find_ci(input, rel)).collect();
    let artist_pal = Palettes::from_exe(found[0].as_ref());
    let writer_pal = Palettes::from_exe(found[1].as_ref());
    let other_pal = if found[0].is_some() || found[1].is_none() {
        &artist_pal
    } else {
        &writer_pal
    };

    for (i, rel) in EXPECTED.iter().enumerate() {
        let Some(path) = &found[i] else {
            ctx.report.missing.push(rel.to_string());
            continue;
        };
        let (prefix, pal) = if rel.starts_with("ARTIST") {
            ("ARTIST", &artist_pal)
        } else {
            ("WRITER", &writer_pal)
        };
        process_file(&mut ctx, path, Some(prefix), pal);
    }
    // Todo o resto da árvore é identificado pela assinatura (SHARED/, USERS/, ...),
    // exceto os arquivos já tratados acima e a própria pasta de saída.
    for path in walk_files(input) {
        let inside_output = path
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .is_some_and(|p| p.starts_with(&out_canon));
        if inside_output || found.iter().flatten().any(|f| f == &path) {
            continue;
        }
        process_file(&mut ctx, &path, None, other_pal);
    }

    std::fs::write(output.join("index.html"), html::render(&ctx.sections))?;
    let json = serde_json::to_string_pretty(&ctx.report).expect("report serializes");
    std::fs::write(output.join("report.json"), json + "\n")?;
    Ok(ctx.report)
}

/// Processa um arquivo; erros de leitura e de gravação viram falhas no relatório.
/// `prefix` é `ARTIST`/`WRITER` para os arquivos principais e `None` para os demais.
fn process_file(ctx: &mut Ctx, path: &Path, prefix: Option<&str>, pal: &Palettes) {
    let rel = relative(ctx.input, path);
    let mut t = Tally {
        path: &rel,
        ok: 0,
        failed: 0,
    };
    let kind_name = match std::fs::read(path) {
        Err(e) => {
            ctx.fail(&mut t, "file".into(), format!("leitura: {e}"));
            "unreadable"
        }
        Ok(data) => match process_data(ctx, &mut t, &data, prefix, pal) {
            Ok(None) => return,
            Ok(Some(kind)) => kind,
            Err(e) => {
                ctx.fail(&mut t, "file".into(), format!("gravação: {e}"));
                "error"
            }
        },
    };
    let summary = FileSummary {
        path: rel.clone(),
        kind: kind_name,
        items_ok: t.ok,
        items_failed: t.failed,
    };
    ctx.report.processed.push(summary);
}

/// Devolve o tipo do arquivo, ou `None` para um arquivo não reconhecido fora dos esperados.
fn process_data(
    ctx: &mut Ctx,
    t: &mut Tally,
    data: &[u8],
    prefix: Option<&str>,
    pal: &Palettes,
) -> io::Result<Option<&'static str>> {
    let rel = t.path.to_string();
    let kind_name = match detect(data) {
        FileKind::Unknown if prefix.is_none() => return Ok(None),
        FileKind::Unknown => {
            ctx.fail(t, "file".into(), "formato não reconhecido".into());
            "unknown"
        }
        FileKind::Ne => {
            let sounds = prefix.map(|p| ctx.output.join(p).join("sounds"));
            extract_ne(ctx, t, data, sounds.as_deref())?;
            "ne"
        }
        FileKind::Glob => {
            let dir = match prefix {
                Some(p) => ctx.output.join(p).join("bitmaps"),
                None => ctx.output.join("documents").join(&rel),
            };
            extract_glob(ctx, t, data, &dir, pal)?;
            "glob"
        }
        FileKind::Chnk => {
            let base = ctx.output.join("documents").join(&rel);
            let mut section = Section {
                title: rel.clone(),
                ..Default::default()
            };
            match chnk::parse(data) {
                Ok(f) => {
                    let tree = extract_chnk(ctx, t, &mut section, &f, data.len(), &base, pal, 0)?;
                    std::fs::create_dir_all(base.parent().expect("has parent"))?;
                    let json = serde_json::to_string_pretty(&tree).expect("json");
                    std::fs::write(with_suffix(&base, ".json"), json + "\n")?;
                }
                Err(e) => ctx.fail(t, "file".into(), e.to_string()),
            }
            ctx.sections.push(section);
            "chnk"
        }
        FileKind::RiffWave => {
            let out = with_suffix(&ctx.output.join("documents").join(&rel), ".wav");
            match wave::decode(data) {
                Ok(pcm) => {
                    write_file(&out, &wave::encode_pcm16(&pcm))?;
                    t.ok += 1;
                    let href = ctx.rel_out(&out);
                    ctx.sections.push(Section {
                        title: rel.clone(),
                        sounds: vec![(href, rel.clone())],
                        ..Default::default()
                    });
                }
                Err(e) => ctx.fail(t, "file".into(), e.to_string()),
            }
            "wave"
        }
    };
    Ok(Some(kind_name))
}

fn extract_ne(ctx: &mut Ctx, t: &mut Tally, data: &[u8], sounds: Option<&Path>) -> io::Result<()> {
    let f = match ne::parse(data) {
        Ok(f) => f,
        Err(e) => {
            ctx.fail(t, "file".into(), e.to_string());
            return Ok(());
        }
    };
    let res_root = ctx.output.join("resources").join(t.path);
    let mut section = Section {
        title: format!("{} — recursos", t.path),
        ..Default::default()
    };
    let mut snd_section = Section {
        title: format!("{} — sons", t.path),
        ..Default::default()
    };
    for r in &f.resources {
        let label = r.kind.label();
        let dir = res_root.join(safe_component(&label));
        let stem = match &r.name {
            Some(n) => safe_component(n),
            None => format!("{:04x}", r.id),
        };
        let item = format!("{label} {stem}");
        write_file(&dir.join(format!("{stem}.bin")), r.data)?;
        write_file(
            &dir.join(format!("{stem}.hex.txt")),
            hexdump(r.data).as_bytes(),
        )?;
        let decoded: Result<(), String> = (|| {
            match label.as_str() {
                "STR#" => {
                    let lines = macres::parse_str_list(r.data).map_err(|e| e.to_string())?;
                    write_file(
                        &dir.join(format!("{stem}.txt")),
                        (lines.join("\n") + "\n").as_bytes(),
                    )
                    .map_err(|e| e.to_string())?;
                }
                "STR" => {
                    let s = macres::parse_cstring(r.data).map_err(|e| e.to_string())?;
                    write_file(&dir.join(format!("{stem}.txt")), (s + "\n").as_bytes())
                        .map_err(|e| e.to_string())?;
                }
                "STRING" => {
                    let items =
                        winres::parse_string_table(r.id, r.data).map_err(|e| e.to_string())?;
                    let mut txt = String::new();
                    for (id, s) in items {
                        writeln!(txt, "{id}\t{s}").unwrap();
                    }
                    write_file(&dir.join(format!("{stem}.txt")), txt.as_bytes())
                        .map_err(|e| e.to_string())?;
                }
                "ICON" | "BITMAP" => {
                    let img = if label == "ICON" {
                        winres::decode_icon(r.data)
                    } else {
                        winres::decode_bitmap(r.data)
                    }
                    .map_err(|e| e.to_string())?;
                    let out = dir.join(format!("{stem}.png"));
                    write_png(&out, img.width, img.height, &img.rgba).map_err(|e| e.to_string())?;
                    section
                        .images
                        .push((relative(ctx.output, &out), item.clone()));
                }
                "CURSOR" => {
                    let (img, (x, y)) = winres::decode_cursor(r.data).map_err(|e| e.to_string())?;
                    let out = dir.join(format!("{stem}-hs{x}x{y}.png"));
                    write_png(&out, img.width, img.height, &img.rgba).map_err(|e| e.to_string())?;
                    section
                        .images
                        .push((relative(ctx.output, &out), item.clone()));
                }
                "SND" => {
                    if let Some(sdir) = sounds {
                        let pcm = wave::decode(r.data).map_err(|e| e.to_string())?;
                        let out = sdir.join(format!("{stem}.wav"));
                        write_file(&out, &wave::encode_pcm16(&pcm)).map_err(|e| e.to_string())?;
                        snd_section.sounds.push((
                            relative(ctx.output, &out),
                            format!("{stem} — {} Hz", pcm.sample_rate),
                        ));
                    }
                }
                _ => {}
            }
            Ok(())
        })();
        match decoded {
            Ok(()) => t.ok += 1,
            Err(e) => ctx.fail(t, item, e),
        }
    }
    ctx.sections.push(section);
    ctx.sections.push(snd_section);
    Ok(())
}

fn extract_glob(
    ctx: &mut Ctx,
    t: &mut Tally,
    data: &[u8],
    dir: &Path,
    pal: &Palettes,
) -> io::Result<()> {
    let entries = match glob::parse(data) {
        Ok(e) => e,
        Err(e) => {
            ctx.fail(t, "file".into(), e.to_string());
            return Ok(());
        }
    };
    let mut section = Section {
        title: format!("{} — imagens", t.path),
        ..Default::default()
    };
    for e in entries {
        let out = dir.join(format!("{:04x}.png", e.id));
        match pkbm::decode(e.data) {
            Ok(b) => {
                write_bitmap(&out, &b, pal)?;
                section.images.push((
                    relative(ctx.output, &out),
                    caption(&format!("{:04x}", e.id), &b),
                ));
                t.ok += 1;
            }
            Err(err) => ctx.fail(t, format!("image {:04x}", e.id), err.to_string()),
        }
    }
    ctx.sections.push(section);
    Ok(())
}

/// Profundidade máxima de CHNK aninhados (os dados reais têm 1 nível).
const MAX_NESTING: usize = 8;

/// Paleta de um arquivo CHNK: a `CTBL` do próprio arquivo substitui a de 16 ou a de
/// 256 cores do executável, conforme o número de cores dela.
fn chnk_palettes(f: &chnk::ChunkFile<'_>, base: &Palettes) -> Palettes {
    let ctbl = f
        .chunks
        .iter()
        .flat_map(|c| &c.parts)
        .find(|p| p.tag.0 == *b"CTBL")
        .and_then(|p| palette::parse_ctbl(p.data).ok());
    match ctbl {
        Some(c) if c.len() == 16 => Palettes {
            pal16: c,
            pal256: base.pal256.clone(),
        },
        Some(c) if c.len() == 256 => Palettes {
            pal16: base.pal16.clone(),
            pal256: c,
        },
        _ => Palettes {
            pal16: base.pal16.clone(),
            pal256: base.pal256.clone(),
        },
    }
}

/// Grava as partes de um CHNK (recursivamente) e devolve a árvore para o JSON.
#[allow(clippy::too_many_arguments)]
fn extract_chnk(
    ctx: &mut Ctx,
    t: &mut Tally,
    section: &mut Section,
    f: &chnk::ChunkFile<'_>,
    file_len: usize,
    base: &Path,
    pal: &Palettes,
    depth: usize,
) -> io::Result<Value> {
    let pal = &chnk_palettes(f, pal);
    let mut chunks = Vec::new();
    for c in &f.chunks {
        let mut parts = Vec::new();
        for (k, p) in c.parts.iter().enumerate() {
            let stem = format!(
                "{}-{}-{}-{}",
                safe_component(&c.tag.to_string()),
                c.id,
                k,
                safe_component(&p.tag.to_string())
            );
            let item = format!("{} {} part {k} {}", c.tag, c.id, p.tag);
            write_file(&base.join(format!("{stem}.bin")), p.data)?;
            let mut part = json!({ "tag": p.tag.to_string(), "offset": p.offset, "size": p.data.len(), "file": format!("{stem}.bin") });
            let label = c
                .name
                .clone()
                .unwrap_or_else(|| format!("{} {}", c.tag, c.id));
            if chnk::is_chnk(p.data) {
                if depth + 1 >= MAX_NESTING || p.data.len() >= file_len {
                    ctx.fail(t, item, "CHNK aninhado não é menor que o arquivo que o contém, ou aninhamento profundo demais".into());
                } else {
                    match chnk::parse(p.data) {
                        Ok(inner) => {
                            let dir = base.join(&stem);
                            part["nested"] = extract_chnk(
                                ctx,
                                t,
                                section,
                                &inner,
                                p.data.len(),
                                &dir,
                                pal,
                                depth + 1,
                            )?;
                            t.ok += 1;
                        }
                        Err(e) => ctx.fail(t, item, e.to_string()),
                    }
                }
            } else if pkbm::is_pkbm(p.data) {
                match pkbm::decode(p.data) {
                    Ok(b) => {
                        let out = base.join(format!("{stem}.png"));
                        write_bitmap(&out, &b, pal)?;
                        section
                            .images
                            .push((relative(ctx.output, &out), caption(&label, &b)));
                        part["image"] = json!(format!("{stem}.png"));
                        t.ok += 1;
                    }
                    Err(e) => ctx.fail(t, item, e.to_string()),
                }
            } else if wave::is_riff_wave(p.data) {
                match wave::decode(p.data) {
                    Ok(pcm) => {
                        let out = base.join(format!("{stem}.wav"));
                        write_file(&out, &wave::encode_pcm16(&pcm))?;
                        section.sounds.push((relative(ctx.output, &out), label));
                        part["sound"] = json!(format!("{stem}.wav"));
                        t.ok += 1;
                    }
                    Err(e) => ctx.fail(t, item, e.to_string()),
                }
            } else {
                t.ok += 1;
            }
            parts.push(part);
        }
        chunks.push(json!({ "tag": c.tag.to_string(), "id": c.id, "flags": c.flags, "name": c.name, "parts": parts }));
    }
    let order = match f.byte_order {
        chnk::ByteOrder::Little => "little",
        chnk::ByteOrder::Big => "big",
    };
    Ok(json!({
        "creator": f.creator.to_string(),
        "version": [f.version.0, f.version.1],
        "byte_order": order,
        "chunks": chunks,
    }))
}

fn caption(label: &str, b: &pkbm::Bitmap) -> String {
    format!("{label} · {}×{} · {} bit", b.width, b.height, b.depth)
}

fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}

fn write_bitmap(path: &Path, b: &pkbm::Bitmap, pal: &Palettes) -> io::Result<()> {
    let colors = b
        .palette
        .clone()
        .unwrap_or_else(|| pkbm::default_palette_for(b.depth, &pal.pal16, &pal.pal256));
    let mut rgba = Vec::with_capacity(b.pixels.len() * 4);
    for &i in &b.pixels {
        let c = colors
            .get(i as usize)
            .copied()
            .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
        rgba.extend_from_slice(&[c.r, c.g, c.b, 255]);
    }
    write_png(path, b.width as u32, b.height as u32, &rgba)
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Fast);
    let mut w = enc.write_header().map_err(io::Error::other)?;
    w.write_image_data(rgba).map_err(io::Error::other)?;
    w.finish().map_err(io::Error::other)
}

/// Hexdump com 16 bytes por linha e coluna ASCII.
fn hexdump(data: &[u8]) -> String {
    let mut s = String::new();
    for (i, row) in data.chunks(16).enumerate() {
        write!(s, "{:08x}  ", i * 16).unwrap();
        for k in 0..16 {
            match row.get(k) {
                Some(b) => write!(s, "{b:02x} ").unwrap(),
                None => s.push_str("   "),
            }
        }
        s.push(' ');
        s.extend(row.iter().map(|&b| {
            if (0x20..0x7f).contains(&b) {
                b as char
            } else {
                '.'
            }
        }));
        s.push('\n');
    }
    s
}
