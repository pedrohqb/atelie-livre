//! Arquivos do Escritório na pasta do usuário (como o original:
//! `C:\MSKIDS\USERS\<APELIDO>\<NOME8>.MAX`; ver `office.md` §7 e
//! `office-arquivos.md` §1 da pesquisa).
//!
//! - O nome do arquivo sai do título: maiúsculas, sem acentos, `_` no lugar
//!   de espaços e pontuação, até 8 caracteres ("Gato Teste" →
//!   `GATO_TES.MAX`). Se já existir outro arquivo com esse nome, os dois
//!   últimos viram um número a partir de 11 (`GATO_T11.MAX`), como o
//!   original fez ao salvar de novo.
//! - Documento sem título gravado sem ser pedido (ao sair sem escolher no
//!   "Epa!") vai para `KIDSnnnn.MAX`, como os do original depois de uma queda.
//! - "Salvar como texto": texto puro em UTF-8, um parágrafo por linha.
//!
//! A conversão documento ↔ arquivo é a de [`super::convert`].

use super::convert::{MaxExtra, from_max, to_max};
use super::doc::{Document, Pos, Range};
use formats::max;
use std::path::{Path, PathBuf};

/// Extensão dos documentos do Creative Writer.
pub const EXT: &str = "MAX";
/// Tamanho máximo do nome (DOS 8.3).
pub const STEM_LEN: usize = 8;
/// Primeiro número posto no fim de um nome repetido (`GATO_T11`).
const FIRST_COPY: u32 = 11;

/// Letra do nome do arquivo: maiúscula sem acento, número, ou `_`.
fn stem_char(c: char) -> Option<char> {
    let base = match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ç' | 'Ç' => 'C',
        'ñ' | 'Ñ' => 'N',
        c if c.is_ascii_alphanumeric() => c.to_ascii_uppercase(),
        // "!" no começo = escondido: não vai para o nome do arquivo
        '!' => return None,
        _ => '_',
    };
    Some(base)
}

/// Nome do arquivo (sem extensão) para o título: `None` se não sobra nada.
pub fn file_stem(title: &str) -> Option<String> {
    let s: String = title
        .trim()
        .chars()
        .filter_map(stem_char)
        .take(STEM_LEN)
        .collect();
    let s = s.trim_end_matches('_').to_string();
    (!s.is_empty() && s.chars().any(|c| c != '_')).then_some(s)
}

fn with_ext(dir: &Path, stem: &str) -> PathBuf {
    dir.join(format!("{stem}.{EXT}"))
}

/// Já existe um arquivo com esse nome (sem diferença de maiúsculas)?
fn taken(dir: &Path, stem: &str) -> bool {
    let want = format!("{stem}.{EXT}").to_ascii_lowercase();
    std::fs::read_dir(dir).is_ok_and(|rd| {
        rd.flatten()
            .any(|e| e.file_name().to_string_lossy().to_ascii_lowercase() == want)
    })
}

/// Caminho livre para o título: `GATO_TES.MAX`, senão `GATO_T11.MAX`,
/// `GATO_T12.MAX`… `keep` é um arquivo que pode ser reaproveitado (o do
/// próprio documento).
pub fn path_for(dir: &Path, title: &str, keep: Option<&Path>) -> PathBuf {
    let stem = file_stem(title).unwrap_or_else(|| "DOC".into());
    let ok = |s: &str| {
        let p = with_ext(dir, s);
        keep.is_some_and(|k| same_file(k, &p)) || !taken(dir, s)
    };
    if ok(&stem) {
        return with_ext(dir, &stem);
    }
    for n in FIRST_COPY.. {
        let digits = n.to_string();
        let head: String = stem.chars().take(STEM_LEN - digits.len()).collect();
        let s = format!("{head}{digits}");
        if ok(&s) {
            return with_ext(dir, &s);
        }
    }
    unreachable!("os números não acabam")
}

fn same_file(a: &Path, b: &Path) -> bool {
    a.file_name().zip(b.file_name()).is_some_and(|(x, y)| {
        x.to_string_lossy()
            .eq_ignore_ascii_case(&y.to_string_lossy())
    }) && a.parent() == b.parent()
}

/// `KIDSnnnn.MAX` livre (documento sem título gravado ao sair).
pub fn kids_path(dir: &Path) -> PathBuf {
    (1..10000)
        .map(|n| format!("KIDS{n:04}"))
        .find(|s| !taken(dir, s))
        .map_or_else(|| with_ext(dir, "KIDS9999"), |s| with_ext(dir, &s))
}

/// Documentos `.MAX` da pasta (sem os internos, que começam com `~`), do
/// mais recente para o mais antigo.
pub fn list(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<(std::time::SystemTime, PathBuf)> = rd
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_ascii_lowercase();
            n.ends_with(".max") && !n.starts_with('~')
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    v.into_iter().map(|(_, p)| p).collect()
}

/// O documento gravado por último.
pub fn latest(dir: &Path) -> Option<PathBuf> {
    list(dir).into_iter().next()
}

/// Abre um `.MAX`.
pub fn load(path: &Path) -> Result<(Document, MaxExtra), String> {
    let data =
        std::fs::read(path).map_err(|e| format!("não consegui ler {}: {e}", path.display()))?;
    let m = max::read(&data).map_err(|e| format!("{} não é um documento: {e}", path.display()))?;
    Ok(from_max(&m))
}

/// Grava o documento num `.MAX`: `pages` = começos das folhas da
/// diagramação, `sel` = seleção guardada.
pub fn save(
    path: &Path,
    doc: &Document,
    extra: &MaxExtra,
    pages: &[Pos],
    sel: Option<Range>,
) -> Result<(), String> {
    let fail = |e: std::io::Error, p: &Path| format!("não consegui gravar {}: {e}", p.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| fail(e, dir))?;
    }
    let bytes = max::write(&to_max(doc, extra, pages, sel));
    // grava ao lado e troca, para não deixar um arquivo pela metade
    let tmp = path.with_extension("MA~");
    std::fs::write(&tmp, bytes).map_err(|e| fail(e, &tmp))?;
    std::fs::rename(&tmp, path).map_err(|e| fail(e, path))
}

/// "Salvar como texto": um parágrafo por linha, sem as figuras.
pub fn to_text(doc: &Document) -> String {
    let mut s = doc.text();
    s.push('\n');
    s
}

/// Grava o texto puro ao lado do documento (`<NOME>.TXT`).
pub fn save_text(path: &Path, doc: &Document) -> Result<(), String> {
    std::fs::write(path, to_text(doc))
        .map_err(|e| format!("não consegui gravar {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::CharProps;

    #[test]
    fn stems_like_the_original() {
        assert_eq!(file_stem("Gato Teste").as_deref(), Some("GATO_TES"));
        assert_eq!(file_stem("A vida é um osso").as_deref(), Some("A_VIDA_E"));
        assert_eq!(file_stem("Ação").as_deref(), Some("ACAO"));
        assert_eq!(file_stem("!Segredo").as_deref(), Some("SEGREDO"));
        assert_eq!(file_stem("oi!").as_deref(), Some("OI"));
        assert_eq!(file_stem("  "), None);
        assert_eq!(file_stem("..."), None);
        assert_eq!(file_stem("x/y:z").as_deref(), Some("X_Y_Z"));
    }

    #[test]
    fn repeated_names_get_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let p = path_for(d, "Gato Teste", None);
        assert_eq!(p, d.join("GATO_TES.MAX"));
        std::fs::write(&p, b"x").unwrap();
        assert_eq!(path_for(d, "Gato Teste", None), d.join("GATO_T11.MAX"));
        // o próprio arquivo do documento pode ser reaproveitado
        assert_eq!(path_for(d, "gato teste", Some(&p)), p);
        std::fs::write(d.join("gato_t11.max"), b"x").unwrap();
        assert_eq!(path_for(d, "Gato Teste", None), d.join("GATO_T12.MAX"));
        assert_eq!(kids_path(d), d.join("KIDS0001.MAX"));
        std::fs::write(d.join("KIDS0001.MAX"), b"x").unwrap();
        assert_eq!(kids_path(d), d.join("KIDS0002.MAX"));
    }

    #[test]
    fn save_load_and_list() {
        let dir = tempfile::tempdir().unwrap();
        let mut doc = Document::from_text("Olá\nmundo", CharProps::default());
        doc.paras[1].props.space_after = true;
        let mut extra = MaxExtra::new();
        extra.add.title = "Olá".into();
        let p = dir.path().join("sub").join("OLA.MAX");
        save(&p, &doc, &extra, &[Pos::default()], None).unwrap();
        let (back, e) = load(&p).unwrap();
        assert_eq!(back, doc);
        assert_eq!(e.add.title, "Olá");
        assert_eq!(list(&dir.path().join("sub")), vec![p.clone()]);
        assert_eq!(latest(&dir.path().join("sub")), Some(p));
        assert!(load(&dir.path().join("nada.MAX")).is_err());
        std::fs::write(dir.path().join("lixo.MAX"), b"nada").unwrap();
        assert!(load(&dir.path().join("lixo.MAX")).is_err());
        // os internos (~NOTESID.MAX) não aparecem
        std::fs::write(dir.path().join("~NOTESID.MAX"), b"x").unwrap();
        assert_eq!(list(dir.path()).len(), 1);
    }

    #[test]
    fn plain_text_export() {
        let doc = Document::from_text("Olá\n\nfim", CharProps::default());
        assert_eq!(to_text(&doc), "Olá\n\nfim\n");
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("OLA.TXT");
        save_text(&p, &doc).unwrap();
        assert_eq!(std::fs::read_to_string(p).unwrap(), "Olá\n\nfim\n");
    }
}
