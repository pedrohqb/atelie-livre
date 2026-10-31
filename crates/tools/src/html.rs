//! Folha de contato (`index.html`) com as imagens e os sons extraídos.

use std::fmt::Write;

#[derive(Default)]
pub struct Section {
    pub title: String,
    /// (caminho relativo, legenda)
    pub images: Vec<(String, String)>,
    pub sounds: Vec<(String, String)>,
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Codifica um caminho relativo para uso em `src`/`href`.
fn href(path: &str) -> String {
    let mut out = String::new();
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            out.push(b as char);
        } else {
            write!(out, "%{b:02X}").unwrap();
        }
    }
    out
}

pub fn render(sections: &[Section]) -> String {
    let mut h = String::from(
        "<!DOCTYPE html>\n<html lang=\"pt-BR\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>Ateliê Livre — extração</title>\n<style>\n\
         body{font-family:sans-serif;margin:1.5rem;background:#f4f4f4;color:#222}\n\
         section{margin-bottom:2rem}\n\
         .grid{display:flex;flex-wrap:wrap;gap:8px}\n\
         figure{margin:0;padding:6px;background:#fff;border:1px solid #ccc;text-align:center}\n\
         figure img{image-rendering:pixelated;max-width:300px;background:repeating-conic-gradient(#ddd 0 25%,#fff 0 50%) 0 0/12px 12px}\n\
         figcaption{font-size:11px;color:#555}\n\
         .snd{display:flex;align-items:center;gap:8px;font-size:12px}\n\
         </style>\n</head>\n<body>\n<h1>Ateliê Livre — extração</h1>\n",
    );
    for s in sections {
        if s.images.is_empty() && s.sounds.is_empty() {
            continue;
        }
        writeln!(h, "<section>\n<h2>{}</h2>", escape(&s.title)).unwrap();
        if !s.images.is_empty() {
            h.push_str("<div class=\"grid\">\n");
            for (path, caption) in &s.images {
                writeln!(
                    h,
                    "<figure><img src=\"{}\" alt=\"\" loading=\"lazy\"><figcaption>{}</figcaption></figure>",
                    href(path),
                    escape(caption)
                )
                .unwrap();
            }
            h.push_str("</div>\n");
        }
        for (path, caption) in &s.sounds {
            writeln!(
                h,
                "<div class=\"snd\"><audio controls preload=\"none\" src=\"{}\"></audio>{}</div>",
                href(path),
                escape(caption)
            )
            .unwrap();
        }
        h.push_str("</section>\n");
    }
    h.push_str("</body>\n</html>\n");
    h
}
