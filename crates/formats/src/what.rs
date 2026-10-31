//! Recurso `WHAT` dos executáveis: os balões de ajuda e dos assistentes
//! (McZee, Maggie, Max e Rex) com texto formatado, opções e ligações.
//!
//! Cada recurso é um bloco de registros. Cabeçalho: `u16` número de
//! registros, `u16` 1, dois `u32` zero e `n + 1` deslocamentos `u32` a partir
//! do início do bloco. Cada registro começa com `08 '1' 09 <título> 09` e
//! segue com uma sequência de códigos de controle e texto Windows-1252:
//!
//! | código | argumento                  | significado                          |
//! |--------|----------------------------|--------------------------------------|
//! | `00`   | hex até espaço             | largura do balão                     |
//! | `01`   | hex até espaço             | altura do balão                      |
//! | `02`   | hex até espaço             | estilo (2 nos balões dos personagens)|
//! | `03`   | hex até espaço             | cor do texto (`F` preto, `9` roxo)   |
//! | `04`   | `1`/`0` até espaço         | assinatura (texto à direita) liga/desliga |
//! | `05`   | 8 hex até espaço           | figura: id << 16 \| parâmetro         |
//! | `06`   | número até espaço          | recuo (0, 1 ou 2)                    |
//! | `07`   | `1`/`0` até espaço         | nome de tecla liga/desliga           |
//! | `0b`   | —                          | negrito liga/desliga                 |
//! | `0c`   | —                          | separador de nomes de tecla equivalentes |
//! | `0d`   | —                          | quebra de linha                      |
//! | `0e`   | —                          | marca em volta das figuras (ignorada)|
//! | `0f`   | 1 byte cru                 | espaçamento da figura (ignorado)     |
//!
//! Figuras com significado próprio: `0x22c` opção de resposta (parâmetro =
//! índice), `0x22e` opção com figura, `0x21b` dica (parâmetro = registro de
//! destino, com o bit 15 às vezes ligado), `0x21d` dica do Max, `0x222`
//! "Precisa de ajuda?", `0x223` pasta, `0x230` Rex, `0xffff` palavra do
//! glossário (parâmetro = registro com a definição); ids `0x1000`… são os
//! ícones das ferramentas.

use crate::cp1252;
use crate::error::{ErrorKind, Format, FormatError, Result};

/// Figura de opção de resposta.
pub const PIC_OPTION: u16 = 0x22c;
/// Opção de resposta com figura ao lado.
pub const PIC_OPTION_PICTURE: u16 = 0x22e;
/// Ligação para uma dica (lâmpada).
pub const PIC_TIP: u16 = 0x21b;
/// Ligação para uma dica do Max.
pub const PIC_TIP_MAX: u16 = 0x21d;
/// "Precisa de ajuda?".
pub const PIC_HELP: u16 = 0x222;
/// Palavra do glossário.
pub const PIC_GLOSSARY: u16 = 0xffff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// Trecho de texto com a formatação em vigor.
    Text {
        text: String,
        bold: bool,
        /// Cor (índice hex do recurso; `0xf` = preto).
        color: u8,
        indent: u8,
        /// Dentro de um nome de tecla (`Delete`, `Enter`).
        key: bool,
        /// Assinatura ("Maggie", "Max"), alinhada à direita no original.
        signature: bool,
    },
    Newline,
    /// Figura embutida: id do `.PIC` e parâmetro (índice da opção, registro).
    Picture {
        id: u16,
        param: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Título interno (em inglês), para achar o registro.
    pub title: String,
    pub width: u16,
    pub height: u16,
    pub style: u16,
    pub items: Vec<Item>,
}

impl Record {
    /// Texto corrido, sem formatação (figuras viram espaço).
    pub fn plain(&self) -> String {
        let mut s = String::new();
        for i in &self.items {
            match i {
                Item::Text { text, .. } => s.push_str(text),
                Item::Newline => s.push('\n'),
                Item::Picture { .. } => s.push(' '),
            }
        }
        s
    }

    /// Opções de resposta, na ordem: (índice do original, texto).
    pub fn options(&self) -> Vec<(u16, String)> {
        let mut out = Vec::new();
        let mut current: Option<(u16, String)> = None;
        for i in &self.items {
            match i {
                Item::Picture { id, param } if *id == PIC_OPTION || *id == PIC_OPTION_PICTURE => {
                    if let Some(c) = current.take() {
                        out.push(c);
                    }
                    current = Some((*param, String::new()));
                }
                Item::Text { text, .. } => {
                    if let Some(c) = current.as_mut() {
                        c.1.push_str(text);
                    }
                }
                Item::Newline => {
                    if let Some(c) = current.take() {
                        out.push(c);
                    }
                }
                Item::Picture { .. } => {}
            }
        }
        if let Some(c) = current.take() {
            out.push(c);
        }
        out.into_iter()
            .map(|(k, t)| (k, t.trim().to_string()))
            .filter(|(_, t)| !t.is_empty())
            .collect()
    }

    /// Ligações para outros registros (dicas e glossário): (registro, texto);
    /// as do glossário vêm sem texto — a palavra é o título do destino.
    pub fn links(&self) -> Vec<(u16, String)> {
        let mut out = Vec::new();
        let mut current: Option<(u16, String)> = None;
        for i in &self.items {
            match i {
                Item::Picture { id, param } if *id == PIC_GLOSSARY => {
                    // a palavra do glossário é o título do registro de destino
                    if let Some(c) = current.take() {
                        out.push(c);
                    }
                    out.push((*param, String::new()));
                }
                Item::Picture { id, param } if matches!(*id, PIC_TIP | PIC_TIP_MAX) => {
                    if let Some(c) = current.take() {
                        out.push(c);
                    }
                    current = Some((*param & 0x7fff, String::new()));
                }
                Item::Text { text, .. } => {
                    if let Some(c) = current.as_mut() {
                        c.1.push_str(text);
                    }
                }
                Item::Newline => {
                    if let Some(c) = current.take() {
                        out.push(c);
                    }
                }
                Item::Picture { .. } => {}
            }
        }
        if let Some(c) = current.take() {
            out.push(c);
        }
        out.into_iter()
            .map(|(k, t)| (k, t.trim().to_string()))
            .collect()
    }
}

fn err(offset: usize, msg: &str) -> FormatError {
    FormatError {
        format: Format::What,
        offset,
        kind: ErrorKind::Invalid(msg.to_string()),
    }
}

fn u16_at(d: &[u8], o: usize) -> Result<u16> {
    d.get(o..o + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| err(o, "fim inesperado"))
}

fn u32_at(d: &[u8], o: usize) -> Result<u32> {
    d.get(o..o + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| err(o, "fim inesperado"))
}

/// Lê um bloco `WHAT` inteiro.
pub fn parse(data: &[u8]) -> Result<Vec<Record>> {
    let n = u16_at(data, 0)? as usize;
    let mut offsets = Vec::with_capacity(n + 1);
    for k in 0..=n {
        offsets.push(u32_at(data, 12 + 4 * k)? as usize);
    }
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let (a, b) = (offsets[k], offsets[k + 1]);
        if a > b || b > data.len() {
            return Err(err(12 + 4 * k, "deslocamento fora do bloco"));
        }
        out.push(parse_record(&data[a..b], a)?);
    }
    Ok(out)
}

/// Lê o argumento textual de um código: bytes até o espaço (consumido).
fn arg<'a>(rec: &'a [u8], i: &mut usize) -> &'a [u8] {
    let start = *i;
    while *i < rec.len() && rec[*i] != b' ' {
        *i += 1;
    }
    let a = &rec[start..*i];
    if *i < rec.len() {
        *i += 1;
    }
    a
}

fn hex(a: &[u8]) -> u32 {
    a.iter().fold(0u32, |v, &c| {
        let d = (c as char).to_digit(16).unwrap_or(0);
        v.wrapping_mul(16).wrapping_add(d)
    })
}

fn parse_record(rec: &[u8], base: usize) -> Result<Record> {
    // 08 '1' 09 título 09
    if rec.len() < 4 || rec[0] != 0x08 {
        return Err(err(base, "registro sem cabeçalho"));
    }
    let mut i = 1;
    let _kind = arg_until(rec, &mut i, 0x09);
    let title = cp1252::decode(arg_until(rec, &mut i, 0x09));
    let mut r = Record {
        title,
        width: 0,
        height: 0,
        style: 0,
        items: Vec::new(),
    };
    let mut bold = false;
    let mut color = 0xf;
    let mut indent = 0;
    let mut key = false;
    let mut signature = false;
    let mut text = Vec::new();
    let flush = |text: &mut Vec<u8>, items: &mut Vec<Item>, bold, color, indent, key, signature| {
        if !text.is_empty() {
            items.push(Item::Text {
                text: cp1252::decode(text),
                bold,
                color,
                indent,
                key,
                signature,
            });
            text.clear();
        }
    };
    while i < rec.len() {
        let c = rec[i];
        i += 1;
        match c {
            0x00..=0x07 => {
                flush(&mut text, &mut r.items, bold, color, indent, key, signature);
                let a = arg(rec, &mut i);
                let v = hex(a);
                match c {
                    0x00 => r.width = v as u16,
                    0x01 => r.height = v as u16,
                    0x02 => r.style = v as u16,
                    0x03 => color = (v & 0xff) as u8,
                    0x04 => signature = v != 0,
                    0x05 => r.items.push(Item::Picture {
                        id: (v >> 16) as u16,
                        param: (v & 0xffff) as u16,
                    }),
                    0x06 => indent = (v & 0xff) as u8,
                    _ => key = v != 0,
                }
            }
            0x0b => {
                flush(&mut text, &mut r.items, bold, color, indent, key, signature);
                bold = !bold;
            }
            0x0c => {
                // separa nomes de tecla equivalentes (`Delete` / `Backspace`)
                flush(&mut text, &mut r.items, bold, color, indent, key, signature);
                if key {
                    r.items.push(Item::Text {
                        text: "/".into(),
                        bold,
                        color,
                        indent,
                        key: false,
                        signature,
                    });
                }
            }
            0x0d => {
                flush(&mut text, &mut r.items, bold, color, indent, key, signature);
                r.items.push(Item::Newline);
            }
            0x0e | 0x08 | 0x09 | 0x0a => {}
            0x0f => {
                i += 1;
            }
            0x10..=0x1f => {}
            _ => text.push(c),
        }
    }
    flush(&mut text, &mut r.items, bold, color, indent, key, signature);
    Ok(r)
}

fn arg_until<'a>(rec: &'a [u8], i: &mut usize, sep: u8) -> &'a [u8] {
    let start = *i;
    while *i < rec.len() && rec[*i] != sep {
        *i += 1;
    }
    let a = &rec[start..*i];
    if *i < rec.len() {
        *i += 1;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(records: &[&[u8]]) -> Vec<u8> {
        let n = records.len();
        let mut d = Vec::new();
        d.extend_from_slice(&(n as u16).to_le_bytes());
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&[0; 8]);
        let mut off = 12 + 4 * (n + 1);
        for r in records {
            d.extend_from_slice(&(off as u32).to_le_bytes());
            off += r.len();
        }
        d.extend_from_slice(&(off as u32).to_le_bytes());
        for r in records {
            d.extend_from_slice(r);
        }
        d
    }

    #[test]
    fn parses_header_text_and_options() {
        let rec = b"\x081\tButton or sticker\t\x00D0 \x0146 \x022 \x03F O que voc\xea quer?\r\x0b\x062 \x0e\x0522C0000 \x0f\x03\x0e Vamos fazer um bot\xe3o!\x0b\r\x0b\x0e\x0522C0001 \x0f\x03\x0e Eu quero um adesivo!\x0b\x060 \r";
        let recs = parse(&block(&[rec])).unwrap();
        assert_eq!(recs.len(), 1);
        let r = &recs[0];
        assert_eq!(r.title, "Button or sticker");
        assert_eq!((r.width, r.height, r.style), (0xd0, 0x46, 2));
        assert_eq!(
            r.items[0],
            Item::Text {
                text: "O que você quer?".into(),
                bold: false,
                color: 0xf,
                indent: 0,
                key: false,
                signature: false
            }
        );
        assert_eq!(
            r.options(),
            vec![
                (0, "Vamos fazer um botão!".to_string()),
                (1, "Eu quero um adesivo!".to_string())
            ]
        );
        assert!(r.plain().starts_with("O que você quer?\n"));
    }

    #[test]
    fn parses_links_keys_and_signature() {
        let rec = b"\x081\tTip\t\x00C8 \x0184 \x022 \x03F \x062 \x0e\x0521B0000 \x0f\x02\x0e  \x0bQuer?\x0b\rAperte \x071 Delete \x0cBackspace\x0c\x070 .\r\x0e\x0521B800A \x0f\x01\x0e  Tirando\r\x041 Maggie";
        let r = &parse(&block(&[rec])).unwrap()[0];
        assert_eq!(
            r.links(),
            vec![(0, "Quer?".to_string()), (10, "Tirando".to_string())]
        );
        let keys: Vec<_> = r
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Text {
                    text, key: true, ..
                } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(keys, vec!["Delete ", "Backspace"]);
        assert!(matches!(
            r.items.last(),
            Some(Item::Text {
                text,
                signature: true,
                ..
            }) if text == "Maggie"
        ));
    }

    #[test]
    fn rejects_bad_offsets() {
        let mut d = block(&[b"\x081\tx\t\x0010 "]);
        d[12] = 0xff;
        assert!(parse(&d).is_err());
    }
}
