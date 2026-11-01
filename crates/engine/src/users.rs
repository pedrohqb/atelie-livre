//! Usuários do Ateliê Livre: o cadastro feito na Entrada (nome, sobrenome,
//! apelido e bicho preferido), guardado em
//! `<pasta de usuários>/<apelido>/perfil.toml`. O TOML é simples o bastante
//! para ser lido e escrito à mão (só chaves `nome = "..."` e `bicho = N`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Nome do arquivo de perfil dentro da pasta do usuário.
pub const PROFILE_FILE: &str = "perfil.toml";
/// Tamanho máximo do apelido (caracteres), como no original.
pub const MAX_NICKNAME: usize = 8;
/// Quantos bichos há para escolher (figuras `0x45b`… de `ARTIST.PIC`).
pub const ANIMALS: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Profile {
    pub nome: String,
    pub sobrenome: String,
    pub apelido: String,
    /// Índice do bicho preferido (0 a `ANIMALS - 1`).
    pub bicho: usize,
}

/// Apelido aceitável: 1 a 8 letras ou números, sem espaços nem pontuação.
pub fn valid_nickname(s: &str) -> bool {
    let n = s.chars().count();
    (1..=MAX_NICKNAME).contains(&n) && s.chars().all(char::is_alphanumeric)
}

/// Sugestão de apelido a partir do nome: só as letras e números, até 8.
pub fn suggest_nickname(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .take(MAX_NICKNAME)
        .collect()
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn unquote(s: &str) -> Option<String> {
    let s = s.trim();
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

impl Profile {
    pub fn to_toml(&self) -> String {
        format!(
            "nome = {}\nsobrenome = {}\napelido = {}\nbicho = {}\n",
            quote(&self.nome),
            quote(&self.sobrenome),
            quote(&self.apelido),
            self.bicho
        )
    }

    /// Lê o perfil; chaves desconhecidas são ignoradas, linhas inválidas também.
    pub fn from_toml(text: &str) -> Profile {
        let mut p = Profile::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "nome" => p.nome = unquote(value).unwrap_or_default(),
                "sobrenome" => p.sobrenome = unquote(value).unwrap_or_default(),
                "apelido" => p.apelido = unquote(value).unwrap_or_default(),
                "bicho" => {
                    p.bicho = value
                        .trim()
                        .parse::<usize>()
                        .map(|b| b.min(ANIMALS - 1))
                        .unwrap_or(0)
                }
                _ => {}
            }
        }
        p
    }
}

/// Apelidos cadastrados (pastas com `perfil.toml`), em ordem alfabética.
pub fn list(dir: &Path) -> Vec<String> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join(PROFILE_FILE).is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

pub fn profile_path(dir: &Path, apelido: &str) -> PathBuf {
    dir.join(apelido).join(PROFILE_FILE)
}

pub fn load(dir: &Path, apelido: &str) -> Option<Profile> {
    let text = fs::read_to_string(profile_path(dir, apelido)).ok()?;
    let mut p = Profile::from_toml(&text);
    if p.apelido.is_empty() {
        p.apelido = apelido.to_string();
    }
    Some(p)
}

/// Grava o perfil em `<dir>/<apelido>/perfil.toml`, criando as pastas.
pub fn save(dir: &Path, p: &Profile) -> io::Result<()> {
    if !valid_nickname(&p.apelido) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "apelido inválido",
        ));
    }
    let folder = dir.join(&p.apelido);
    fs::create_dir_all(&folder)?;
    fs::write(folder.join(PROFILE_FILE), p.to_toml())
}

/// Apaga a pasta de um usuário (ao trocar o apelido no cadastro).
pub fn remove(dir: &Path, apelido: &str) -> io::Result<()> {
    if !valid_nickname(apelido) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "apelido inválido",
        ));
    }
    fs::remove_dir_all(dir.join(apelido))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nickname_rules() {
        assert!(valid_nickname("Ana"));
        assert!(valid_nickname("ana123"));
        assert!(valid_nickname("José"));
        assert!(valid_nickname("12345678"));
        assert!(!valid_nickname("123456789"));
        assert!(!valid_nickname(""));
        assert!(!valid_nickname("ana b"));
        assert!(!valid_nickname("ana.b"));
        assert_eq!(suggest_nickname("Ana Maria Silva"), "AnaMaria");
        assert_eq!(suggest_nickname("Zé!"), "Zé");
    }

    #[test]
    fn toml_round_trip() {
        let p = Profile {
            nome: "Ana \"Nina\"".into(),
            sobrenome: "Teste\\Silva".into(),
            apelido: "ana".into(),
            bicho: 7,
        };
        assert_eq!(Profile::from_toml(&p.to_toml()), p);
        let lixo = "# comentário\nnome = \"X\"\nbicho = 99\ncor = 3\nsem igual\n";
        let q = Profile::from_toml(lixo);
        assert_eq!(q.nome, "X");
        assert_eq!(q.bicho, ANIMALS - 1);
        assert_eq!(q.apelido, "");
    }

    #[test]
    fn save_list_load_remove() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list(dir.path()).is_empty());
        let p = Profile {
            nome: "Ana".into(),
            sobrenome: "Teste".into(),
            apelido: "ana".into(),
            bicho: 2,
        };
        save(dir.path(), &p).unwrap();
        let z = Profile {
            apelido: "Zeca".into(),
            ..Profile::default()
        };
        save(dir.path(), &z).unwrap();
        // pasta sem perfil não conta
        std::fs::create_dir_all(dir.path().join("lixo")).unwrap();
        assert_eq!(
            list(dir.path()),
            vec!["ana".to_string(), "Zeca".to_string()]
        );
        assert_eq!(load(dir.path(), "ana"), Some(p));
        assert_eq!(load(dir.path(), "ninguem"), None);
        remove(dir.path(), "ana").unwrap();
        assert_eq!(list(dir.path()), vec!["Zeca".to_string()]);
        assert!(
            save(
                dir.path(),
                &Profile {
                    apelido: "a b".into(),
                    ..Profile::default()
                }
            )
            .is_err()
        );
    }
}
