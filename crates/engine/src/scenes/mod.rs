//! Cenas do Ateliê Livre.

pub mod attic;
pub mod browser;
pub mod chooser;
pub mod elevator;
pub mod entry;
pub mod gags;
pub mod gallery;
pub mod gallery_print;
pub mod gallery_room;
pub mod imaginopolis;
pub mod paint;
pub mod paint_sfx;
pub mod portfolio;
pub mod projects;
pub mod room;
pub mod room_objects;
pub mod room_talk;
pub mod rooms_data;
pub mod sounds;
pub mod splash;
pub mod sticker_scripts;
pub mod stickers;
pub mod story;
pub mod street;
pub mod studio_wall;
pub mod supertools;
pub mod transformers;
pub mod tricks_3d;
pub mod tricks_basic;
pub mod wizard;
pub mod writer;

pub use browser::Browser;
pub use chooser::Chooser;
pub use entry::Entry;
pub use gallery::Gallery;
pub use gallery_room::GalleryRoom;
pub use imaginopolis::Imaginopolis;
pub use paint::Paint;
pub use room::{Floor, Room};
pub use splash::Splash;
pub use story::Story;
pub use street::Street;
pub use writer::Writer;

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

    /// Grava o quadro em `$ATELIE_DUMP/NOME.ppm` (para conferir telas contra
    /// capturas do original); sem a variável, não faz nada.
    #[allow(dead_code)]
    pub fn dump(frame: &crate::Frame, name: &str) {
        let Some(dir) = std::env::var_os("ATELIE_DUMP") else {
            return;
        };
        let (w, h) = (frame.width(), frame.height());
        let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
        for p in frame.pixels().chunks(4) {
            out.extend_from_slice(&p[..3]);
        }
        let _ = std::fs::write(PathBuf::from(dir).join(format!("{name}.ppm")), out);
    }

    /// Assets mínimos: um diretório temporário só com um ARTIST.EXE vazio de recursos.
    pub fn empty_assets() -> (tempfile::TempDir, Assets) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ARTIST.EXE"), b"MZ").unwrap();
        let a = Assets::open(dir.path()).unwrap();
        (dir, a)
    }
}
