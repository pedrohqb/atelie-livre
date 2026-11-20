//! Estilo dos documentos novos: Ctrl+clique no Aplicador da Palavra grava o
//! pincel atual como padrão "para todos os documentos novos" (balões
//! `0fa0[2]` e `0fa0[22]`). O lugar em que o original guarda isso não foi
//! localizado; o port grava num arquivo próprio na pasta do usuário,
//! [`FILE`], com linhas `chave=valor` (ver `docs/formats/arquivos-do-port.md`).
//!
//! O estilo vale para o `\r` (e, portanto, para o que se digita) e para as
//! propriedades do primeiro parágrafo do documento em branco.

use super::doc::{Align, CharProps, DEFAULT_FONT, Document, Effect, Indent, ParaProps, Size};
use formats::Rgb;
use std::path::Path;

/// Nome do arquivo na pasta do usuário.
pub const FILE: &str = "estilo-writer.txt";

/// Estilo dos documentos novos (a fonte pelo nome, porque o `ftc` é de cada
/// documento).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewStyle {
    pub font: String,
    pub chars: CharProps,
    pub para: ParaProps,
}

impl Default for NewStyle {
    fn default() -> NewStyle {
        NewStyle {
            font: DEFAULT_FONT.to_string(),
            chars: CharProps::default(),
            para: ParaProps::default(),
        }
    }
}

fn hex(c: Option<Rgb>) -> String {
    c.map_or_else(String::new, |c| {
        format!("{:02x}{:02x}{:02x}", c.r, c.g, c.b)
    })
}

fn parse_hex(s: &str) -> Option<Rgb> {
    let v = u32::from_str_radix(s.trim(), 16).ok()?;
    (s.trim().len() == 6).then_some(Rgb {
        r: (v >> 16) as u8,
        g: (v >> 8) as u8,
        b: v as u8,
    })
}

impl NewStyle {
    /// O estilo do documento `doc` (fonte e caracteres de `chars`, parágrafo
    /// `para`).
    pub fn of(doc: &Document, chars: CharProps, para: ParaProps) -> NewStyle {
        NewStyle {
            font: doc.fonts.name(chars.font).to_string(),
            chars: CharProps { font: 0, ..chars },
            para,
        }
    }

    pub fn is_default(&self) -> bool {
        *self == NewStyle::default()
    }

    /// Texto do arquivo.
    pub fn to_text(&self) -> String {
        let c = &self.chars;
        let p = &self.para;
        let b = |v: bool| u8::from(v);
        format!(
            "fonte={}\ntamanho={}\ncor={}\niluminador={}\nnegrito={}\ninclinado={}\n\
             sublinhado={}\nefeito={}\nalinhamento={}\nrecuo={}\nentre-linhas={}\n\
             depois={}\ncapitular={}\ndestacada={}\nnova-pagina={}\nmarcadores={}\n",
            self.font,
            c.size.pt(),
            hex(c.color),
            hex(c.highlight),
            b(c.bold),
            b(c.italic),
            b(c.underline),
            c.effect.map_or(0, Effect::code),
            p.align.code(),
            p.indent.code(),
            b(p.line_spacing),
            b(p.space_after),
            p.drop_cap.unwrap_or(0),
            b(p.raised_cap),
            b(p.new_page),
            b(p.bullets),
        )
    }

    /// Lê o texto do arquivo (chaves desconhecidas ou ruins são ignoradas).
    pub fn from_text(text: &str) -> NewStyle {
        let mut s = NewStyle::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            let n = v.parse::<u16>().ok();
            let on = n.is_some_and(|n| n != 0);
            let (c, p) = (&mut s.chars, &mut s.para);
            match k.trim() {
                "fonte" if !v.is_empty() => s.font = v.to_string(),
                "tamanho" => {
                    if let Some(n) = n {
                        c.size = Size::from_pt(n);
                    }
                }
                "cor" => c.color = parse_hex(v),
                "iluminador" => c.highlight = parse_hex(v),
                "negrito" => c.bold = on,
                "inclinado" => c.italic = on,
                "sublinhado" => c.underline = on,
                "efeito" => {
                    c.effect = n
                        .filter(|&n| (1..=255).contains(&n))
                        .map(|n| Effect::from_code(n as u8));
                }
                "alinhamento" => p.align = Align::from_code(n.unwrap_or(0) as u8),
                "recuo" => p.indent = Indent::from_code(n.unwrap_or(0) as u8),
                "entre-linhas" => p.line_spacing = on,
                "depois" => p.space_after = on,
                "capitular" => p.drop_cap = n.filter(|&n| n > 0).map(|n| n.min(255) as u8),
                "destacada" => p.raised_cap = on,
                "nova-pagina" => p.new_page = on,
                "marcadores" => p.bullets = on,
                _ => {}
            }
        }
        s
    }

    /// O estilo gravado na pasta do usuário (o padrão, sem arquivo).
    pub fn load(dir: Option<&Path>) -> NewStyle {
        dir.and_then(|d| std::fs::read_to_string(d.join(FILE)).ok())
            .map_or_else(NewStyle::default, |t| NewStyle::from_text(&t))
    }

    /// Grava na pasta do usuário.
    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join(FILE), self.to_text())
    }

    /// Aplica a um documento em branco: o `\r` e o parágrafo.
    pub fn apply_to(&self, doc: &mut Document) {
        if self.is_default() {
            return;
        }
        let ftc = doc.fonts.ensure(&self.font);
        let mark = CharProps {
            font: ftc,
            ..self.chars
        };
        for par in &mut doc.paras {
            par.mark = mark;
            par.props = self.para;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::CSCH;

    #[test]
    fn text_round_trip() {
        let s = NewStyle::default();
        assert_eq!(NewStyle::from_text(&s.to_text()), s);
        let s = NewStyle {
            font: "Impact".into(),
            chars: CharProps {
                size: Size::Large,
                color: Some(CSCH[5]),
                highlight: Some(CSCH[1]),
                bold: true,
                underline: true,
                effect: Some(Effect::Vupt),
                ..CharProps::default()
            },
            para: ParaProps {
                align: Align::Center,
                indent: Indent::Hanging,
                space_after: true,
                drop_cap: Some(3),
                ..ParaProps::default()
            },
        };
        assert_eq!(NewStyle::from_text(&s.to_text()), s);
        assert_eq!(
            NewStyle::from_text("lixo\ntamanho=x\n"),
            NewStyle::default()
        );
    }

    #[test]
    fn saved_style_applies_to_blank_documents() {
        let dir = std::env::temp_dir().join(format!("atelie-newstyle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(NewStyle::load(Some(&dir)).is_default());
        let mut d = Document::new();
        let chars = CharProps {
            size: Size::Giant,
            ..CharProps::default()
        };
        let ftc = d.fonts.ensure("Stencil");
        let s = NewStyle::of(&d, CharProps { font: ftc, ..chars }, ParaProps::default());
        s.save(&dir).unwrap();
        let mut blank = Document::new();
        NewStyle::load(Some(&dir)).apply_to(&mut blank);
        let mark = blank.paras[0].mark;
        assert_eq!(mark.size, Size::Giant);
        assert_eq!(blank.fonts.name(mark.font), "Stencil");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
