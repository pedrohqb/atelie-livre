//! Carga do dicionário em segundo plano.
//!
//! Decodificar o `SPELL.DCT` e o `THES.DCT` (e montar o índice de anagramas)
//! leva alguns segundos num executável de depuração: o Escritório pede a
//! carga ao entrar ([`preload`]) e a abelha só consulta ([`get`]). A carga é
//! uma por pasta de dados e vale para o processo inteiro.

use crate::writer::dict::{Dictionary, Variant};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Resultado de uma carga: `None` enquanto carrega; `Some(None)` se não
/// houver dicionário nenhum (nem o original nem o livre do sistema).
type Slot = Arc<OnceLock<Option<Arc<Dictionary>>>>;

fn slots() -> &'static Mutex<HashMap<PathBuf, Slot>> {
    static SLOTS: OnceLock<Mutex<HashMap<PathBuf, Slot>>> = OnceLock::new();
    SLOTS.get_or_init(Default::default)
}

/// Chave da carga: a pasta, se tiver o `SPELL.DCT`; senão uma só para o
/// dicionário livre do sistema (que não depende da pasta).
fn key(data: &Path) -> PathBuf {
    if data.join("WRITER").join("SPELL.DCT").is_file() {
        data.to_path_buf()
    } else {
        PathBuf::new()
    }
}

/// Começa a carregar o dicionário da pasta `MSKIDS` `data` (o original, ou o
/// livre do sistema na falta dele), se ainda não começou.
pub fn preload(data: &Path) {
    let k = key(data);
    let mut map = slots().lock().unwrap_or_else(|e| e.into_inner());
    if map.contains_key(&k) {
        return;
    }
    let slot: Slot = Arc::default();
    map.insert(k, slot.clone());
    let dir = data.to_path_buf();
    let spawned = std::thread::Builder::new()
        .name("dicionario".into())
        .spawn({
            let slot = slot.clone();
            move || {
                let _ = slot.set(load(&dir));
            }
        });
    if spawned.is_err() {
        // sem threads: carrega aqui mesmo
        let _ = slot.set(load(data));
    }
}

fn load(dir: &Path) -> Option<Arc<Dictionary>> {
    let d = Dictionary::load(Some(dir), Variant::Brasil)?;
    // monta o índice de anagramas agora, e não no primeiro clique
    let _ = d.anagrams("ramo");
    Some(Arc::new(d))
}

/// Estado da carga para a pasta `data`.
#[derive(Debug, Clone)]
pub enum State {
    /// [`preload`] não foi chamado.
    NotStarted,
    Loading,
    Ready(Arc<Dictionary>),
    /// Não há dicionário.
    Missing,
}

pub fn get(data: &Path) -> State {
    let map = slots().lock().unwrap_or_else(|e| e.into_inner());
    match map.get(&key(data)).map(|s| s.get()) {
        None => State::NotStarted,
        Some(None) => State::Loading,
        Some(Some(Some(d))) => State::Ready(d.clone()),
        Some(Some(None)) => State::Missing,
    }
}

/// Espera a carga terminar (testes).
#[cfg(test)]
pub fn wait(data: &Path) -> State {
    loop {
        match get(data) {
            State::Loading => std::thread::sleep(std::time::Duration::from_millis(20)),
            s => return s,
        }
    }
}
