//! `atelie-tools info`: inventário de um arquivo.

use crate::detect::{FileKind, detect};
use formats::{FormatError, chnk, glob, ne, pkbm, wave};
use std::collections::BTreeMap;
use std::fmt::Write;

/// Devolve o relatório em texto, ou a mensagem de erro.
pub fn report(data: &[u8]) -> Result<String, String> {
    let mut out = String::new();
    let fmt_err = |e: FormatError| e.to_string();
    match detect(data) {
        FileKind::Ne => {
            let f = ne::parse(data).map_err(fmt_err)?;
            writeln!(out, "NE").unwrap();
            writeln!(out, "modules: {}", f.modules.join(", ")).unwrap();
            let mut order: Vec<String> = Vec::new();
            let mut totals: BTreeMap<String, (usize, usize)> = BTreeMap::new();
            for r in &f.resources {
                let label = r.kind.label();
                if !totals.contains_key(&label) {
                    order.push(label.clone());
                }
                let t = totals.entry(label).or_default();
                t.0 += 1;
                t.1 += r.data.len();
            }
            writeln!(out, "type\tcount\tbytes").unwrap();
            for label in order {
                let (n, bytes) = totals[&label];
                writeln!(out, "{label}\t{n}\t{bytes}").unwrap();
            }
        }
        FileKind::Glob => {
            let entries = glob::parse(data).map_err(fmt_err)?;
            writeln!(out, "GLOB").unwrap();
            writeln!(out, "images: {}", entries.len()).unwrap();
            let mut depths: BTreeMap<u8, usize> = BTreeMap::new();
            for e in &entries {
                let b = pkbm::decode(e.data).map_err(|err| format!("image {}: {err}", e.id))?;
                *depths.entry(b.depth).or_default() += 1;
            }
            for (d, n) in depths {
                writeln!(out, "depth {d}\t{n}").unwrap();
            }
        }
        FileKind::Chnk => {
            let f = chnk::parse(data).map_err(fmt_err)?;
            let order = match f.byte_order {
                chnk::ByteOrder::Little => "little-endian",
                chnk::ByteOrder::Big => "big-endian",
            };
            writeln!(out, "CHNK").unwrap();
            writeln!(out, "creator: {}", f.creator).unwrap();
            writeln!(out, "version: {}/{} {order}", f.version.0, f.version.1).unwrap();
            writeln!(out, "chunks: {}", f.chunks.len()).unwrap();
            writeln!(out, "chunk\tid\tflags\tpart\toffset\tbytes\tname").unwrap();
            for c in &f.chunks {
                for p in &c.parts {
                    writeln!(
                        out,
                        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        c.tag,
                        c.id,
                        c.flags,
                        p.tag,
                        p.offset,
                        p.data.len(),
                        c.name.as_deref().unwrap_or("")
                    )
                    .unwrap();
                }
            }
        }
        FileKind::RiffWave => {
            let p = wave::decode(data).map_err(fmt_err)?;
            writeln!(out, "WAVE").unwrap();
            let frames = p.samples.len() / p.channels.max(1) as usize;
            writeln!(
                out,
                "{} Hz, {} channel(s), {frames} frames",
                p.sample_rate, p.channels
            )
            .unwrap();
        }
        FileKind::Unknown => return Err("formato não reconhecido".into()),
    }
    Ok(out)
}
