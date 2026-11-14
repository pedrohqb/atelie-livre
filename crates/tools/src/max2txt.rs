//! `atelie-tools max2txt` e o resumo de `info` para documentos do Creative
//! Writer (`.MAX`): texto, estilos e objetos, para conferência.

use formats::Rgb;
use formats::max::{
    self, Align, CharProps, Document, Effect, FrameKind, Indent, ObjRef, ParaProps, Project,
};
use std::fmt::Write;
use std::path::Path;

fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

fn obj(r: ObjRef) -> String {
    format!("{} {}", r.tag.to_string().trim_end(), r.id)
}

fn effect_name(e: Effect) -> String {
    match e {
        Effect::Blink => "Pisca-pisca".into(),
        Effect::Ants => "Formiguinhas".into(),
        Effect::Vapt => "Vapt".into(),
        Effect::Vupt => "Vupt".into(),
        Effect::Glow => "Brilho".into(),
        Effect::Shake => "Treme-treme".into(),
        Effect::Crazy => "Maluco".into(),
        Effect::Other(v) => format!("efeito {v}"),
    }
}

fn project_name(p: Project) -> String {
    match p {
        Project::None => "nenhum".into(),
        Project::Card => "cartão (card)".into(),
        Project::Banner => "faixa (banr)".into(),
        Project::Newspaper => "jornal (news)".into(),
        Project::Other(b) => format!("{:?}", String::from_utf8_lossy(&b)),
    }
}

/// Propriedades de caractere em palavras; vazio = padrão.
pub fn describe_char(p: &CharProps, doc: &Document) -> String {
    let mut v: Vec<String> = Vec::new();
    if p.anchor {
        v.push("âncora".into());
    }
    for (on, name) in [
        (p.bold, "negrito"),
        (p.italic, "inclinado"),
        (p.underline, "sublinhado"),
    ] {
        if on {
            v.push(name.into());
        }
    }
    if let Some(f) = p.font {
        v.push(format!("fonte {f} ({})", doc.font_name(f)));
    }
    if let Some(s) = p.size {
        v.push(format!("{s} pt"));
    }
    if let Some(c) = p.text_color() {
        v.push(format!("cor {}", hex(c)));
    }
    if let Some(c) = p.highlight_color() {
        v.push(format!("marca-texto {}", hex(c)));
    }
    if let Some(e) = p.effect {
        v.push(effect_name(e));
    }
    for s in &p.extra {
        v.push(format!("sprm 0x{:02x}={}", s.code, hexbytes(&s.arg)));
    }
    v.join(", ")
}

/// Propriedades de parágrafo em palavras; vazio = padrão.
pub fn describe_para(p: &ParaProps) -> String {
    let mut v: Vec<String> = Vec::new();
    match p.align {
        Align::Left => {}
        Align::Right => v.push("direita".into()),
        Align::Center => v.push("centro".into()),
        Align::Justify => v.push("justificado".into()),
    }
    match p.indent {
        Indent::None => {}
        Indent::FirstLine => v.push("recuo na 1ª linha".into()),
        Indent::Hanging => v.push("recuo menos na 1ª linha".into()),
        Indent::All => v.push("recuo em todas".into()),
    }
    for (on, name) in [
        (p.line_spacing, "entrelinha extra"),
        (p.space_after, "espaço depois"),
        (p.raised_cap, "letra destacada"),
        (p.new_page, "nova página"),
        (p.bullets, "marcadores"),
    ] {
        if on {
            v.push(name.into());
        }
    }
    if let Some(n) = p.drop_cap {
        v.push(format!("capitular ({n})"));
    }
    for s in &p.extra {
        v.push(format!("sprm 0x{:02x}={}", s.code, hexbytes(&s.arg)));
    }
    v.join(", ")
}

fn hexbytes(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Trecho do texto para exibir numa linha: `\r` como `¶`, cortado.
fn snippet(doc: &Document, start: u32, end: u32) -> String {
    let s: String = doc.text_range(start, end).replace('\r', "¶");
    if s.chars().count() > 50 {
        let cut: String = s.chars().take(47).collect();
        format!("{cut:?}…")
    } else {
        format!("{s:?}")
    }
}

/// Cabeçalho: título, autor, projeto, andar, senha, página, fontes, folhas.
pub fn summary(doc: &Document) -> String {
    let mut o = String::new();
    let a = &doc.add;
    let d = &doc.dop;
    writeln!(o, "título: {}", a.title).unwrap();
    writeln!(o, "autor: {}", a.author).unwrap();
    writeln!(o, "projeto: {}", project_name(a.project)).unwrap();
    writeln!(o, "andar: {}", a.floor).unwrap();
    if a.encrypted || !a.password.is_empty() {
        writeln!(
            o,
            "agente secreto: {} (senha {:?})",
            if a.encrypted { "embaralhado" } else { "não" },
            a.password
        )
        .unwrap();
    }
    let mut page = format!(
        "página: {} × {} pt, margens {}/{}/{}/{}, {} coluna(s)",
        d.page_width,
        d.page_height,
        d.margin_top,
        d.margin_left,
        d.margin_bottom,
        d.margin_right,
        d.columns
    );
    if let Some(b) = d.border {
        page += &format!(", borda {}", obj(b));
    }
    if let Some(s) = d.page_number_style {
        page += &format!(", nº de página (estilo {s})");
    }
    if d.newspaper {
        page += ", jornal";
    }
    if d.banner {
        page += ", faixa";
    }
    writeln!(o, "{page}").unwrap();
    let fonts: Vec<String> = doc
        .fonts
        .iter()
        .map(|f| format!("{} {} ({})", f.ftc, f.name, f.family))
        .collect();
    writeln!(o, "fontes: {}", fonts.join(", ")).unwrap();
    writeln!(
        o,
        "texto: {} caracteres, {} parágrafos, {} trechos de caractere, {} folha(s)",
        doc.len(),
        doc.para_runs.len(),
        doc.char_runs.len(),
        doc.pages.len()
    )
    .unwrap();
    writeln!(
        o,
        "objetos: {} moldura(s), {} som(ns) em palavras, {} chunk(s) de objeto",
        doc.frames.len(),
        doc.word_sounds.len(),
        doc.objects.len()
    )
    .unwrap();
    o
}

/// Relatório completo: resumo, texto, parágrafos, caracteres, folhas e objetos.
pub fn report(doc: &Document) -> String {
    let mut o = summary(doc);
    writeln!(o, "\n== texto ==\n{}", doc.plain_text()).unwrap();
    writeln!(o, "\n== parágrafos ==").unwrap();
    for r in &doc.para_runs {
        writeln!(
            o,
            "[{}, {}) {} {}",
            r.start,
            r.end,
            snippet(doc, r.start, r.end),
            describe_para(&r.props)
        )
        .unwrap();
    }
    writeln!(o, "\n== caracteres ==").unwrap();
    for r in &doc.char_runs {
        writeln!(
            o,
            "[{}, {}) {} {}",
            r.start,
            r.end,
            snippet(doc, r.start, r.end),
            describe_char(&r.props, doc)
        )
        .unwrap();
    }
    writeln!(o, "\n== folhas ==").unwrap();
    for (k, p) in doc.pages.iter().enumerate() {
        writeln!(o, "{}: [{}, {}) flags {}", k + 1, p.start, p.end, p.flags).unwrap();
    }
    if !doc.frames.is_empty() || !doc.word_sounds.is_empty() || !doc.objects.is_empty() {
        writeln!(o, "\n== objetos ==").unwrap();
    }
    for (k, f) in doc.frames.iter().enumerate() {
        let kind = match f.kind {
            FrameKind::Splot => "Splot".to_string(),
            FrameKind::Picture => "figura".to_string(),
            FrameKind::PageFrame => "moldura de página".to_string(),
            FrameKind::Other(v) => format!("tipo {v}"),
        };
        let mut line = format!(
            "moldura {}: {kind} em ({}, {}) {} × {}, página {}",
            k + 1,
            f.x,
            f.y,
            f.width,
            f.height,
            f.page
        );
        if let Some(r) = f.object {
            line += &format!(", {}", obj(r));
        }
        if let Some(r) = f.sound {
            line += &format!(", som {}", obj(r));
        }
        if let Some(w) = f.wrap {
            line += &format!(", contorno {w}");
        }
        writeln!(o, "{line}").unwrap();
    }
    for w in &doc.word_sounds {
        writeln!(
            o,
            "som em palavras: [{}, {}) {} {}",
            w.start,
            w.end,
            obj(w.sound),
            snippet(doc, w.start, w.end)
        )
        .unwrap();
    }
    for c in &doc.objects {
        let parts: Vec<String> = c
            .parts
            .iter()
            .map(|p| format!("{} {} B", p.tag.to_string().trim_end(), p.data.len()))
            .collect();
        let name = c
            .name
            .as_deref()
            .map(|n| format!(" {n:?}"))
            .unwrap_or_default();
        writeln!(
            o,
            "chunk {} {}{name}: {}",
            c.tag.to_string().trim_end(),
            c.id,
            parts.join(" + ")
        )
        .unwrap();
    }
    o
}

/// `max2txt`: lê `input` e escreve o relatório (ou só o texto) em `output`
/// ou na saída padrão.
pub fn run(input: &Path, output: Option<&Path>, text_only: bool) -> Result<(), String> {
    let data = std::fs::read(input).map_err(|e| format!("{}: {e}", input.display()))?;
    let doc = max::read(&data).map_err(|e| format!("{}: {e}", input.display()))?;
    let mut out = if text_only {
        doc.plain_text()
    } else {
        report(&doc)
    };
    if text_only && !out.ends_with('\n') {
        out.push('\n');
    }
    match output {
        Some(p) => std::fs::write(p, out).map_err(|e| format!("{}: {e}", p.display())),
        None => {
            print!("{out}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formats::max::{Frame, Run};

    fn doc() -> Document {
        let mut d = Document::from_text("Olá\nmundo");
        d.add.title = "Teste".into();
        d.add.encrypted = true;
        d.add.password = "abc".into();
        let mut p = CharProps {
            bold: true,
            size: Some(24),
            font: Some(7),
            effect: Some(Effect::Glow),
            ..CharProps::default()
        };
        p.set_text_color(Some(Rgb {
            r: 0xfa,
            g: 0,
            b: 0x39,
        }));
        d.char_runs = vec![
            Run {
                start: 0,
                end: 3,
                props: p,
            },
            Run {
                start: 3,
                end: d.len(),
                props: CharProps::default(),
            },
        ];
        d.para_runs[0].props.align = Align::Center;
        d.para_runs[0].props.bullets = true;
        d.frames.push(Frame::new(
            FrameKind::Picture,
            ObjRef::new(b"PICT", 2),
            10,
            20,
            30,
            40,
        ));
        d
    }

    #[test]
    fn describes_properties_in_portuguese() {
        let d = doc();
        assert_eq!(
            describe_char(&d.char_runs[0].props, &d),
            "negrito, fonte 7 (Lucida Sans), 24 pt, cor #fa0039, Brilho"
        );
        assert_eq!(describe_char(&CharProps::default(), &d), "");
        assert_eq!(describe_para(&d.para_runs[0].props), "centro, marcadores");
    }

    #[test]
    fn report_lists_everything() {
        let r = report(&doc());
        for want in [
            "título: Teste",
            "agente secreto: embaralhado (senha \"abc\")",
            "página: 595 × 792 pt, margens 72/72/72/72, 1 coluna(s)",
            "fontes: 0 Lucida Sans (2)",
            "== texto ==\nOlá\nmundo\n",
            "[0, 4) \"Olá¶\" centro, marcadores",
            "[0, 3) \"Olá\" negrito",
            "1: [0, 11) flags 0",
            "moldura 1: figura em (10, 20) 30 × 40, página 0, PICT 2",
        ] {
            assert!(r.contains(want), "{want:?} em\n{r}");
        }
    }
}
