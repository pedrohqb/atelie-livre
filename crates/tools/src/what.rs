//! `atelie-tools what`: imprime os balões (`WHAT`) de um executável.

use formats::what::{Item, Record};
use std::path::Path;

pub fn run(exe: &Path, block: Option<&str>) -> Result<(), String> {
    let data = std::fs::read(exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    let ne = formats::ne::parse(&data).map_err(|e| e.to_string())?;
    let wanted = match block {
        Some(b) => {
            Some(u16::from_str_radix(b.trim_start_matches("0x"), 16).map_err(|e| e.to_string())?)
        }
        None => None,
    };
    let mut ids: Vec<u16> = ne
        .of_kind("WHAT")
        .filter(|r| r.name.is_none())
        .map(|r| r.id)
        .collect();
    ids.sort_unstable();
    for id in ids {
        if wanted.is_some_and(|w| w != id) {
            continue;
        }
        let Some(r) = ne.find("WHAT", id) else {
            continue;
        };
        let recs = formats::what::parse(r.data).map_err(|e| e.to_string())?;
        println!("== WHAT {id:04x}: {} balões", recs.len());
        for (k, rec) in recs.iter().enumerate() {
            print_record(k, rec);
        }
    }
    Ok(())
}

fn print_record(k: usize, r: &Record) {
    println!(
        "--- [{k}] \"{}\" {}×{} estilo {}",
        r.title, r.width, r.height, r.style
    );
    let mut line = String::new();
    for i in &r.items {
        match i {
            Item::Text { text, bold, .. } => {
                if *bold {
                    line.push('*');
                    line.push_str(text);
                    line.push('*');
                } else {
                    line.push_str(text);
                }
            }
            Item::Newline => {
                println!("    {line}");
                line.clear();
            }
            Item::Picture { id, param } => {
                line.push_str(&format!("[{id:x}:{param:x}]"));
            }
        }
    }
    if !line.trim().is_empty() {
        println!("    {line}");
    }
}
