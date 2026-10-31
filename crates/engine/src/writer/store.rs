//! Arquivos do Escritório: texto puro, parágrafos separados por uma linha
//! vazia, adesivos como `[adesivo:ROLO:N]`. Os estilos ainda não são
//! guardados (o formato `.MAX` original fica para depois). `ultimo.txt` é
//! reaberto na próxima vez; cada texto modificado ganha também uma cópia
//! `texto-AAAAMMDD-HHMMSS.txt`.

use super::doc::{Document, Item, Paragraph, Sticker, Style};
use crate::paint::store::timestamp;
use std::io;
use std::path::Path;

pub const LAST_NAME: &str = "ultimo.txt";

/// Texto do documento (parágrafos separados por linha vazia, adesivos entre colchetes).
pub fn to_text(doc: &Document) -> String {
    let paras: Vec<String> = doc
        .paras
        .iter()
        .map(|p| {
            p.items
                .iter()
                .map(|i| match i {
                    Item::Char(c, _) => c.to_string(),
                    Item::Sticker(s) => format!("[adesivo:{}:{}]", s.roll, s.index),
                })
                .collect()
        })
        .collect();
    let mut s = paras.join("\n\n");
    s.push('\n');
    s
}

/// `[adesivo:ROLO:N]` no começo de `s`: o adesivo e o resto da string.
fn parse_sticker(s: &str) -> Option<(Sticker, &str)> {
    let body = s.strip_prefix("[adesivo:")?;
    let end = body.find(']')?;
    let mut parts = body[..end].split(':');
    let roll = parts.next()?.parse().ok()?;
    let index = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((Sticker::placeholder(roll, index), &body[end + 1..]))
}

/// Documento a partir do texto; o cursor fica no fim e o estilo é o padrão.
pub fn from_text(text: &str) -> Document {
    let text = text.strip_suffix('\n').unwrap_or(text);
    let style = Style::default();
    let mut paras = Vec::new();
    for chunk in text.split("\n\n") {
        let mut p = Paragraph::default();
        let mut rest = chunk;
        while !rest.is_empty() {
            if let Some((s, after)) = parse_sticker(rest) {
                p.items.push(Item::Sticker(s));
                rest = after;
                continue;
            }
            let mut chars = rest.chars();
            let c = chars.next().expect("resto não vazio");
            rest = chars.as_str();
            if !c.is_control() {
                p.items.push(Item::Char(c, style));
            }
        }
        paras.push(p);
    }
    let mut doc = Document {
        paras,
        cursor: Default::default(),
        style,
    };
    doc.clamp_cursor();
    doc.cursor.para = doc.paras.len() - 1;
    doc.end();
    doc
}

pub fn load(path: &Path) -> Option<Document> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(from_text(&text))
}

/// Grava `ultimo.txt` e, se o texto mudou, também uma cópia com data e hora.
pub fn save(dir: &Path, doc: &Document, changed: bool, now: u64) -> Result<(), String> {
    let fail = |e: io::Error, p: &Path| format!("não consegui gravar {}: {e}", p.display());
    std::fs::create_dir_all(dir).map_err(|e| fail(e, dir))?;
    let text = to_text(doc);
    let last = dir.join(LAST_NAME);
    std::fs::write(&last, &text).map_err(|e| fail(e, &last))?;
    if changed {
        let named = dir.join(format!("texto-{}.txt", timestamp(now)));
        std::fs::write(&named, &text).map_err(|e| fail(e, &named))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_roundtrip_with_stickers_and_empty_paragraphs() {
        let mut d = Document::new();
        d.insert_text("Olá, mundo");
        d.insert_sticker(Sticker::placeholder(7, 3));
        d.newline();
        d.newline();
        d.insert_text("fim");
        let t = to_text(&d);
        assert_eq!(t, "Olá, mundo[adesivo:7:3]\n\n\n\nfim\n");
        let back = from_text(&t);
        assert_eq!(back.paras, d.paras);
        assert_eq!(back.cursor.para, 2);
        assert_eq!(back.cursor.index, 3);
    }

    #[test]
    fn empty_and_malformed_text() {
        let d = from_text("");
        assert_eq!(d, Document::new());
        let d = from_text("[adesivo:x:1]a\n");
        assert_eq!(d.paras.len(), 1);
        assert_eq!(d.para().text(), "[adesivo:x:1]a");
        let d = from_text("[adesivo:1:2");
        assert_eq!(d.para().text(), "[adesivo:1:2");
        let d = from_text("a\tb\r\n");
        assert_eq!(d.para().text(), "ab");
    }

    #[test]
    fn save_writes_last_and_dated_copy() {
        let dir = tempfile::tempdir().unwrap();
        let mut d = Document::new();
        d.insert_text("oi");
        save(dir.path(), &d, false, 0).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        save(dir.path(), &d, true, 1_700_000_000).unwrap();
        let names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 2);
        assert!(names.iter().any(|n| n == "ultimo.txt"));
        assert!(names.iter().any(|n| n == "texto-20231114-221320.txt"));
        assert_eq!(
            load(&dir.path().join(LAST_NAME)).unwrap().para().text(),
            "oi"
        );
        assert!(load(&dir.path().join("nada.txt")).is_none());
    }
}
