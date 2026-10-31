//! Oficina de Projetos (andar 3): história em quadrinhos, botão ou adesivo e
//! desenho animado — modos da tela de pintura. Spec:
//! `docs/superpowers/specs/2026-10-31-projetos-design.md`.

pub mod button;
pub mod comics;
pub mod flipbook;

/// Qual projeto a tela de pintura está fazendo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Comics,
    Button,
    Sticker,
    Flipbook,
}

impl ProjectKind {
    /// Rótulo da barra inferior (textos das `STR#` do `ARTIST.EXE`).
    pub fn label(self) -> &'static str {
        match self {
            ProjectKind::Comics => "Fazendo uma História em Quadrinhos",
            ProjectKind::Button => "Fazendo um Botão",
            ProjectKind::Sticker => "Fazendo um Adesivo",
            ProjectKind::Flipbook => "Fazendo um Desenho Animado",
        }
    }

    /// Nome do quadro novo, no centro da barra inferior.
    pub fn new_name(self) -> &'static str {
        match self {
            ProjectKind::Comics => "Quadrinhos Novos",
            ProjectKind::Button => "Botão Novo",
            ProjectKind::Sticker => "Adesivo Novo",
            ProjectKind::Flipbook => "Desenho Animado Novo",
        }
    }

    /// Prefixo dos arquivos guardados (`quadrinhos-…png`, `animacao-…/`).
    pub fn file_stem(self) -> &'static str {
        match self {
            ProjectKind::Comics => "quadrinhos",
            ProjectKind::Button => "botao",
            ProjectKind::Sticker => "adesivo",
            ProjectKind::Flipbook => "animacao",
        }
    }
}

/// Estado do projeto dentro da tela de pintura.
#[derive(Debug, Clone)]
pub enum Project {
    Comics(comics::Comics),
    Button(button::Button),
    Flipbook(flipbook::Flipbook),
}

impl Project {
    pub fn new(kind: ProjectKind) -> Project {
        match kind {
            ProjectKind::Comics => Project::Comics(comics::Comics::default()),
            ProjectKind::Button => Project::Button(button::Button::new(false)),
            ProjectKind::Sticker => Project::Button(button::Button::new(true)),
            ProjectKind::Flipbook => Project::Flipbook(flipbook::Flipbook::default()),
        }
    }

    pub fn kind(&self) -> ProjectKind {
        match self {
            Project::Comics(_) => ProjectKind::Comics,
            Project::Button(b) if b.sticker => ProjectKind::Sticker,
            Project::Button(_) => ProjectKind::Button,
            Project::Flipbook(_) => ProjectKind::Flipbook,
        }
    }
}
