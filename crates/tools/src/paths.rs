//! Caminhos: busca sem distinguir maiúsculas, varredura ordenada e nomes seguros.

use std::path::{Path, PathBuf};

/// Resolve `rel` (componentes separados por `/`) sob `root` sem distinguir maiúsculas.
pub fn find_ci(root: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = root.to_path_buf();
    for part in rel.split('/') {
        let exact = cur.join(part);
        if exact.exists() {
            cur = exact;
            continue;
        }
        let wanted = part.to_lowercase();
        let found = std::fs::read_dir(&cur)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().to_lowercase() == wanted)
            })
            .min()?;
        cur = found;
    }
    Some(cur)
}

/// Todos os arquivos sob `dir`, recursivamente, em ordem.
pub fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return out;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            out.extend(walk_files(&p));
        } else {
            out.push(p);
        }
    }
    out
}

/// Caminho relativo com `/`, para relatórios e nomes de saída.
pub fn relative(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Componente de nome de arquivo sem separadores, espaços ou controles.
pub fn safe_component(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c.is_whitespace() || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    if out == "." || out == ".." {
        "_".repeat(out.len())
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_ci_ignores_case() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Artist")).unwrap();
        std::fs::write(dir.path().join("Artist/artist.pic"), b"x").unwrap();
        let found = find_ci(dir.path(), "ARTIST/ARTIST.PIC").unwrap();
        assert_eq!(found, dir.path().join("Artist/artist.pic"));
        assert!(find_ci(dir.path(), "WRITER/WRITER.PIC").is_none());
        assert!(find_ci(dir.path(), "ARTIST").is_some());
    }

    #[test]
    fn walk_is_sorted_and_recursive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("b/c")).unwrap();
        std::fs::write(dir.path().join("b/c/z"), b"").unwrap();
        std::fs::write(dir.path().join("b/a"), b"").unwrap();
        let files = walk_files(&dir.path().join("b"));
        assert_eq!(files, [dir.path().join("b/a"), dir.path().join("b/c/z")]);
    }

    #[test]
    fn safe_component_replaces_separators() {
        assert_eq!(safe_component("STR#"), "STR#");
        assert_eq!(safe_component("SND "), "SND_");
        assert_eq!(safe_component("a/b\\c"), "a_b_c");
        assert_eq!(safe_component(".."), "__");
    }
}
