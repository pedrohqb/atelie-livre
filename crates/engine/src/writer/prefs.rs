//! Preferências do Creative Writer por usuário (tarefa T14): o
//! `~MAXUSER.INF` do original (`formats::maxprefs`, `docs/formats/maxprefs.md`),
//! gravado na pasta do usuário do port, `<usuários>/<apelido>/~MAXUSER.INF`.
//!
//! - `FILR 0` é o **último documento**, o do quadro da parede do Escritório;
//! - `FILR 1..4` são as **quatro molduras da Biblioteca**, na ordem
//!   (159,9), (296,9), (296,143), (159,143);
//! - as tabelas `STTB`/`SUDG` (entradas da Máquina de Idéias) e o cache de
//!   fontes `FNTL` são guardados como vieram.
//!
//! Para um usuário sem o arquivo valem os exemplos do Max, como no original
//! (`SHARED/USERS/MAX`): "O Ponto de Fuga" no lugar 0 e, nas molduras, os
//! que o original mostra a um usuário novo (`building_81_lib_base.png`).
//!
//! Onde ficam os documentos: os caminhos do `FILR` são pastas a partir de
//! `MSKIDS` (`SHARED\USERS\MAX`, `USERS\ANA`). Um documento de um usuário do
//! port (`…\USERS\<apelido>`, com a pasta em `<usuários>/<apelido>`) é
//! procurado lá; os outros, na pasta `MSKIDS` dos dados originais. Os
//! documentos que o port grava para um usuário ficam na pasta dele
//! (`<usuários>/<apelido>/*.MAX`), registrados como `USERS\<APELIDO>`.
//!
//! O estado das salas que o original também guarda (bolsa aberta, ajuda
//! desligada; os bytes não foram localizados no original) vai num arquivo
//! do port, `<usuários>/<apelido>/predio-writer.txt` (linhas `chave=valor`).

use crate::paths::find_ci;
use crate::scenes::room_spec::RoomState;
use formats::maxprefs::{self, KIND_MAX, UserPrefs, WallDoc};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Nome do arquivo de preferências na pasta do usuário.
pub const FILE: &str = "~MAXUSER.INF";
/// Estado das salas (bolsa, ajuda) na pasta do usuário.
pub const STATE_FILE: &str = "predio-writer.txt";
/// Lugar do último documento (quadro do Escritório).
pub const LAST_SLOT: u16 = 0;
/// Lugares das molduras da Biblioteca.
pub const LIBRARY_SLOTS: [u16; 4] = [1, 2, 3, 4];
/// Pastas dos exemplos do Max, a partir de `MSKIDS`.
pub const SAMPLES_DIR: [&str; 3] = ["SHARED", "USERS", "MAX"];
/// Exemplos da parede de um usuário novo, por lugar (0 = último documento).
pub const DEFAULT_WALL: [&str; 5] = [
    "SAMP18.MAX",
    "SAMP9.MAX",
    "SAMP16.MAX",
    "SAMP5.MAX",
    "SAMP6.MAX",
];

/// Caminho do `~MAXUSER.INF` do usuário.
pub fn path(users: &Path, user: &str) -> PathBuf {
    users.join(user).join(FILE)
}

/// Título de um `.MAX` (o do `TITL`/`ADD `), se ele abrir.
pub fn read_title(path: &Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    formats::max::read(&data).ok().map(|d| d.add.title)
}

/// Registro da parede para um exemplo do Max.
fn sample(slot: u16, file: &str, data: &Path) -> WallDoc {
    let rel = format!("{}/{file}", SAMPLES_DIR.join("/"));
    let title = find_ci(data, &rel)
        .and_then(|p| read_title(&p))
        .unwrap_or_default();
    WallDoc {
        slot,
        kind: KIND_MAX.to_string(),
        title,
        dirs: SAMPLES_DIR.iter().map(|s| s.to_string()).collect(),
        file: file.to_string(),
    }
}

/// Preferências de um usuário novo: a parede com os exemplos do Max (os
/// títulos são lidos dos exemplos em `data`, a pasta `MSKIDS`).
pub fn defaults(data: &Path) -> UserPrefs {
    let mut p = UserPrefs::new();
    p.wall = DEFAULT_WALL
        .iter()
        .enumerate()
        .map(|(slot, f)| sample(slot as u16, f, data))
        .collect();
    p
}

/// Preferências do usuário: as gravadas (os lugares que faltarem vêm dos
/// exemplos) ou, sem arquivo legível, as de um usuário novo.
pub fn load(users: Option<&Path>, user: &str, data: &Path) -> UserPrefs {
    let read = users
        .and_then(|u| fs::read(path(u, user)).ok())
        .and_then(|b| maxprefs::read(&b).ok());
    let Some(mut p) = read else {
        return defaults(data);
    };
    for (slot, f) in DEFAULT_WALL.iter().enumerate() {
        let slot = slot as u16;
        if p.wall_slot(slot).is_none() {
            p.wall.push(sample(slot, f, data));
        }
    }
    p.wall.sort_by_key(|d| d.slot);
    p
}

/// Grava as preferências (num `.tmp` renomeado), criando a pasta do usuário.
pub fn save(users: &Path, user: &str, p: &UserPrefs) -> io::Result<()> {
    let dir = users.join(user);
    fs::create_dir_all(&dir)?;
    let tmp = dir.join(format!("{FILE}.tmp"));
    fs::write(&tmp, maxprefs::write(p))?;
    fs::rename(&tmp, path(users, user))
}

/// Pasta do usuário do port a que o registro se refere (`…\USERS\<apelido>`
/// com `<usuários>/<apelido>` existente, sem diferença de maiúsculas).
fn user_folder(doc: &WallDoc, users: Option<&Path>) -> Option<PathBuf> {
    let users = users?;
    let n = doc.dirs.len();
    if n < 2 || !doc.dirs[n - 2].eq_ignore_ascii_case("USERS") {
        return None;
    }
    let name = &doc.dirs[n - 1];
    fs::read_dir(users)
        .ok()?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
        .filter(|p| p.is_dir())
}

/// Arquivo do documento do registro: na pasta do usuário do port, se for de
/// um, senão na pasta `MSKIDS` (`data`); `None` se não existir.
pub fn resolve(doc: &WallDoc, users: Option<&Path>, data: &Path) -> Option<PathBuf> {
    if let Some(dir) = user_folder(doc, users) {
        return find_ci(&dir, &doc.file);
    }
    find_ci(data, &doc.rel_path())
}

/// O documento é do usuário (está na pasta dele): ele pode trocar o
/// título; senão é de outro (o exemplo do Max).
pub fn is_own(doc: &WallDoc, user: &str) -> bool {
    doc.dirs
        .last()
        .is_some_and(|d| d.eq_ignore_ascii_case(user))
}

/// Registro da parede para o arquivo `file`: documentos da pasta de um
/// usuário do port viram `USERS\<APELIDO>`; os de dentro de `data`, o
/// caminho a partir dela. `None` para um arquivo fora dos dois.
pub fn wall_doc(file: &Path, title: &str, users: Option<&Path>, data: &Path) -> Option<WallDoc> {
    let name = file.file_name()?.to_string_lossy().to_string();
    let parent = file.parent()?;
    let dirs: Vec<String> = match users.and_then(|u| parent.strip_prefix(u).ok()) {
        Some(rel) if rel.components().count() == 1 => {
            vec!["USERS".into(), rel.to_string_lossy().to_uppercase()]
        }
        _ => parent
            .strip_prefix(data)
            .ok()?
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect(),
    };
    Some(WallDoc {
        slot: 0,
        kind: KIND_MAX.to_string(),
        title: title.to_string(),
        dirs,
        file: name,
    })
}

/// Põe `doc` no lugar `slot` (trocando o que houver).
pub fn set_slot(p: &mut UserPrefs, slot: u16, mut doc: WallDoc) {
    doc.slot = slot;
    p.wall.retain(|d| d.slot != slot);
    p.wall.push(doc);
    p.wall.sort_by_key(|d| d.slot);
}

/// Último documento do usuário (registro e arquivo), se o arquivo existir.
pub fn last_document(users: Option<&Path>, user: &str, data: &Path) -> Option<(WallDoc, PathBuf)> {
    let p = load(users, user, data);
    let d = p.wall_slot(LAST_SLOT)?.clone();
    let f = resolve(&d, users, data)?;
    Some((d, f))
}

/// As quatro molduras da Biblioteca (registro e arquivo, se existir).
pub fn library(users: Option<&Path>, user: &str, data: &Path) -> Vec<(WallDoc, Option<PathBuf>)> {
    let p = load(users, user, data);
    LIBRARY_SLOTS
        .iter()
        .filter_map(|&s| p.wall_slot(s).cloned())
        .map(|d| {
            let f = resolve(&d, users, data);
            (d, f)
        })
        .collect()
}

/// Torna `file` o último documento do usuário e grava (o original faz isso
/// ao abrir ou salvar um documento).
pub fn remember_last(
    users: &Path,
    user: &str,
    data: &Path,
    file: &Path,
    title: &str,
) -> io::Result<()> {
    let mut p = load(Some(users), user, data);
    let doc = wall_doc(file, title, Some(users), data).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} fora das pastas conhecidas", file.display()),
        )
    })?;
    set_slot(&mut p, LAST_SLOT, doc);
    save(users, user, &p)
}

/// Troca o título de um documento do usuário: o `TITL`/`ADD ` do `.MAX` e o
/// título nos lugares da parede que apontam para ele.
pub fn retitle(users: &Path, user: &str, file: &Path, title: &str) -> Result<(), String> {
    let fail = |e: String| format!("não consegui trocar o título de {}: {e}", file.display());
    let bytes = fs::read(file).map_err(|e| fail(e.to_string()))?;
    let mut doc = formats::max::read(&bytes).map_err(|e| fail(e.to_string()))?;
    doc.add.title = title.to_string();
    fs::write(file, formats::max::write(&doc)).map_err(|e| fail(e.to_string()))?;
    // só o arquivo gravado: sem ele não há lugar da parede a acertar
    let Some(mut p) = fs::read(path(users, user))
        .ok()
        .and_then(|b| maxprefs::read(&b).ok())
    else {
        return Ok(());
    };
    let name = file.file_name().map(|n| n.to_string_lossy().to_string());
    let mut changed = false;
    for d in &mut p.wall {
        if Some(&d.file) == name.as_ref() && is_own(d, user) {
            d.title = title.to_string();
            changed = true;
        }
    }
    if changed {
        save(users, user, &p).map_err(|e| fail(e.to_string()))?;
    }
    Ok(())
}

/// Estado das salas do usuário (bolsa fechada e ajuda ligada se não houver).
pub fn load_state(users: &Path, user: &str) -> RoomState {
    let text = fs::read_to_string(users.join(user).join(STATE_FILE)).unwrap_or_default();
    let mut st = RoomState::default();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let on = v.trim() == "1";
        match k.trim() {
            "bolsa_aberta" => st.bag_open = on,
            "ajuda_desligada" => st.help_off = on,
            _ => {}
        }
    }
    st
}

/// Grava o estado das salas (falhas são ignoradas: é só conforto).
pub fn save_state(users: &Path, user: &str, st: RoomState) {
    let dir = users.join(user);
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let text = format!(
        "bolsa_aberta={}\najuda_desligada={}\n",
        st.bag_open as u8, st.help_off as u8
    );
    let _ = fs::write(dir.join(STATE_FILE), text);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "atelie-prefs-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn new_user_gets_the_samples_and_saves_them() {
        let users = tmp("users");
        let data = tmp("data");
        let p = load(Some(&users), "ana", &data);
        assert_eq!(p.wall.len(), 5);
        let last = p.wall_slot(LAST_SLOT).unwrap();
        assert_eq!(last.file, "SAMP18.MAX");
        assert_eq!(last.rel_path(), "SHARED/USERS/MAX/SAMP18.MAX");
        assert!(!is_own(last, "ana"));
        assert!(is_own(last, "max"));
        save(&users, "ana", &p).unwrap();
        assert!(path(&users, "ana").is_file());
        assert_eq!(load(Some(&users), "ana", &data), p);
    }

    #[test]
    fn user_documents_resolve_to_the_port_folder() {
        let users = tmp("users2");
        let data = tmp("data2");
        fs::create_dir_all(users.join("ana")).unwrap();
        let f = users.join("ana").join("CARTA.MAX");
        fs::write(&f, b"x").unwrap();
        let d = wall_doc(&f, "Carta", Some(&users), &data).unwrap();
        assert_eq!(d.dirs, ["USERS", "ANA"]);
        assert!(is_own(&d, "ana"));
        assert_eq!(resolve(&d, Some(&users), &data), Some(f.clone()));
        // um documento do MSKIDS
        fs::create_dir_all(data.join("SHARED/USERS/MAX")).unwrap();
        let s = data.join("SHARED/USERS/MAX/SAMP1.MAX");
        fs::write(&s, b"x").unwrap();
        let d = wall_doc(&s, "A vida", Some(&users), &data).unwrap();
        assert_eq!(d.dirs, ["SHARED", "USERS", "MAX"]);
        assert_eq!(resolve(&d, Some(&users), &data), Some(s));
        // fora dos dois
        assert!(wall_doc(Path::new("/nada/X.MAX"), "", Some(&users), &data).is_none());
    }

    #[test]
    fn remember_last_keeps_the_library() {
        let users = tmp("users3");
        let data = tmp("data3");
        fs::create_dir_all(users.join("ana")).unwrap();
        let f = users.join("ana").join("CARTA.MAX");
        fs::write(&f, b"x").unwrap();
        remember_last(&users, "ana", &data, &f, "Carta").unwrap();
        let p = load(Some(&users), "ana", &data);
        assert_eq!(p.wall_slot(0).unwrap().title, "Carta");
        assert_eq!(p.wall_slot(1).unwrap().file, "SAMP9.MAX");
        let (d, file) = last_document(Some(&users), "ana", &data).unwrap();
        assert_eq!((d.file.as_str(), file), ("CARTA.MAX", f));
    }

    #[test]
    fn room_state_round_trip() {
        let users = tmp("users4");
        assert_eq!(load_state(&users, "ana"), RoomState::default());
        let st = RoomState {
            bag_open: true,
            help_off: true,
        };
        save_state(&users, "ana", st);
        assert_eq!(load_state(&users, "ana"), st);
    }

    /// Com os dados reais: os exemplos da parede existem e têm título.
    #[test]
    fn samples_with_real_data() {
        let Some(dir) = std::env::var_os("ATELIE_DATA").map(PathBuf::from) else {
            return;
        };
        if !dir.is_dir() {
            return;
        }
        let p = defaults(&dir);
        assert_eq!(p.wall_slot(0).unwrap().title, "O Ponto de Fuga");
        assert_eq!(p.wall_slot(2).unwrap().title, "Jornal Mágico");
        for d in &p.wall {
            assert!(resolve(d, None, &dir).is_some(), "{d:?}");
        }
        let lib = library(None, "ana", &dir);
        assert_eq!(lib.len(), 4);
    }
}
