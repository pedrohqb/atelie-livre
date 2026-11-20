//! Caderno de Idéias no Escritório (W20, `ideas.md` §5): sai da Bolsa
//! Mágica do editor (o 5º botão do menu do McZee, `0c01`) e é o mesmo
//! [`NotebookView`] da Oficina de Idéias, aberto por cima da página com a
//! moldura do Escritório pontilhada de cinza (`ideas_89_caderno_editor`).
//!
//! - Abrir: sons `022d` (item do menu) e `0209`; balão do McZee `0faa[2]`
//!   ("Caderno de Idéias… Para mover o caderno…"); abre na última aba usada.
//! - Tesoura e câmera: os balões do McZee `0faa[4]`/`[3]` (Lembretes) e
//!   `[20]`/`[19]` (Máquina e Figuras); o que é recortado ou copiado vai para
//!   a área de transferência da sessão (`ideas::session::clip`), que o Ctrl+V
//!   e o Colar dos Re-movedores do editor usam.
//! - OK grava as três abas no formato do port (`<usuários>/<apelido>/ideias/
//!   caderno/`, `writer::ideas::store::Notebook`; o original grava
//!   `~NOTESID.MAX`); NÃO descarta.

use super::tools::{HelpRef, WHAT_GENERAL};
use crate::Frame;
use crate::assets::Assets;
use crate::scene::{Effect, Input};
use crate::scenes::writer_ideas::Owner;
use crate::scenes::writer_ideas::notebook::{NotebookView, Outcome};
use crate::ui::{Rect, checker};
use std::path::Path;

/// Balão do McZee ao abrir (`0faa[2]`).
pub const WHAT_NOTEBOOK: usize = 2;
/// Abrir o caderno (o mesmo som da aba Lembretes na Oficina).
pub const SND_OPEN: u16 = 0x0209;
/// Partes da moldura que ficam pontilhadas de cinza 100 com o caderno
/// aberto (medidas em `ideas_89_caderno_editor`): barra de ferramentas e
/// faixa, barra de baixo.
pub const DIMMED: [Rect; 2] = [(4, 4, 632, 100), (4, 459, 632, 17)];
const DIM: [u8; 3] = [100, 100, 100];

pub struct NotebookOverlay {
    view: NotebookView,
}

impl NotebookOverlay {
    /// Abre o caderno do usuário `user` (pasta dos usuários `users`).
    pub fn open(users: Option<&Path>, user: &str, a: &Assets) -> NotebookOverlay {
        let owner = Owner::new(users, user);
        NotebookOverlay {
            view: NotebookView::open(&owner, a).in_editor(),
        }
    }

    pub fn view(&self) -> &NotebookView {
        &self.view
    }

    /// Repassa uma entrada ao caderno. Devolve como ele fechou (se fechou) e
    /// o balão do McZee que ele pediu (se pediu).
    pub fn handle(
        &mut self,
        i: &Input,
        fx: &mut Vec<Effect>,
        a: &Assets,
    ) -> (Option<Outcome>, Option<HelpRef>) {
        let out = self.view.handle(i, fx, a);
        let help = self.view.take_editor_help().map(|k| (WHAT_GENERAL, k));
        (out, help)
    }

    pub fn tick(&mut self, dt: f32) {
        self.view.tick(dt);
    }

    /// Moldura pontilhada e o caderno (a página fica como está).
    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        for r in DIMMED {
            checker(f, r, DIM);
        }
        self.view.draw(f, a);
    }
}
