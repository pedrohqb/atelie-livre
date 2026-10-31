//! Cenas do Ateliê Livre.

pub mod browser;
pub mod gallery;
pub mod paint;
pub mod room;
pub mod rooms_data;
pub mod sounds;
pub mod splash;
pub mod sticker_scripts;
pub mod stickers;
pub mod street;

pub use browser::Browser;
pub use gallery::Gallery;
pub use paint::Paint;
pub use room::{Floor, Room};
pub use splash::Splash;
pub use street::Street;

#[cfg(test)]
pub(crate) mod testutil {
    use crate::assets::Assets;
    use std::path::PathBuf;

    /// Assets reais, se `ATELIE_DATA` estiver definida.
    pub fn assets() -> Option<Assets> {
        let dir = std::env::var_os("ATELIE_DATA").map(PathBuf::from);
        match dir {
            Some(d) => Some(Assets::open(&d).expect("ATELIE_DATA válida")),
            None => {
                eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
                None
            }
        }
    }

    /// Assets mínimos: um diretório temporário só com um ARTIST.EXE vazio de recursos.
    pub fn empty_assets() -> (tempfile::TempDir, Assets) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        (dir, a)
    }
}
