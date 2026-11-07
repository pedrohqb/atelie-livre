//! Quadro novo pelo cavalete do Ateliê ("Quadro Novo"): a tela abre em
//! branco sem apagar o último quadro guardado, que continua no quadro da
//! parede. No original a parede só muda quando o quadro é salvo; aqui o
//! quadro é guardado ao sair, mas um quadro em branco e sem mudanças não
//! substitui o último (sair de um quadro novo sem desenhar, ou jogá-lo fora,
//! deixa a parede como estava).

use super::Paint;
use super::rename::DEFAULT_NAME;
use crate::assets::Assets;
use crate::paint::objects::Objects;
use crate::paint::{CANVAS_H, CANVAS_W, Canvas, History, WHITE};
use std::path::PathBuf;

impl Paint {
    /// Tela de pintura com um quadro em branco, sem objetos nem desfazer, e
    /// o nome padrão; o último quadro fica guardado como estava.
    pub fn new_blank(drawings: Option<PathBuf>, assets: &Assets) -> Paint {
        let mut p = Paint::new(drawings, assets);
        p.canvas = Canvas::new(CANVAS_W, CANVAS_H);
        p.objects = Objects::default();
        p.history = History::default();
        p.title = DEFAULT_NAME.to_string();
        p.changed = false;
        p
    }

    /// Ao sair do Ateliê livre: um quadro em branco, sem objetos e sem
    /// mudanças não é guardado por cima do último que já existe.
    pub(super) fn keeps_last(&self) -> bool {
        self.drawings
            .as_ref()
            .is_some_and(|d| d.join(crate::paint::store::LAST_NAME).exists())
            && self.project.is_none()
            && self.trick.is_none()
            && self.edit_file.is_none()
            && !self.changed
            && self.objects.is_empty()
            && self.canvas.px.iter().all(|&v| v == WHITE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::store::{LAST_NAME, load_name, load_png, save_drawing_named};
    use crate::scene::Scene;
    use crate::scenes::testutil::empty_assets;

    #[test]
    fn blank_picture_does_not_replace_the_last_one() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let pal = a.palettes(crate::assets::Program::Artist).pal256.clone();
        let mut c = Canvas::new(CANVAS_W, CANVAS_H);
        c.px.fill(1);
        save_drawing_named(
            dir.path(),
            &c,
            &Objects::default(),
            &pal,
            true,
            0,
            Some("Fundo Lunar"),
        )
        .unwrap();
        let mut p = Paint::new_blank(Some(dir.path().to_path_buf()), &a);
        assert_eq!(p.title(), DEFAULT_NAME);
        assert!(p.canvas.px.iter().all(|&v| v == WHITE));
        p.leave();
        let last = load_png(&dir.path().join(LAST_NAME), &pal).unwrap();
        assert!(last.px.iter().all(|&v| v == 1), "o último continua");
        assert_eq!(load_name(dir.path()).as_deref(), Some("Fundo Lunar"));
        // a tela comum (o quadro da parede) abre o último, com o nome
        let p = Paint::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(p.title(), "Fundo Lunar");
        assert!(p.canvas.px.iter().all(|&v| v == 1));
    }
}
