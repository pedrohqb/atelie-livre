#![allow(dead_code)]

use std::path::PathBuf;

/// Pasta `MSKIDS` indicada por `ATELIE_DATA`; `None` (com aviso) se ausente.
pub fn data_dir() -> Option<PathBuf> {
    match std::env::var_os("ATELIE_DATA") {
        Some(p) => Some(PathBuf::from(p)),
        None => {
            eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
            None
        }
    }
}

/// Lê `rel` dentro da pasta de dados.
pub fn read(rel: &str) -> Option<Vec<u8>> {
    let p = data_dir()?.join(rel);
    Some(std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
}
