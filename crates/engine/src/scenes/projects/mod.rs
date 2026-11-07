//! Oficina de Projetos (andar 3): história em quadrinhos, botão ou adesivo e
//! desenho animado — modos da tela de pintura. Spec:
//! `docs/superpowers/specs/2026-10-31-projetos-design.md`.

pub mod button;
pub mod comics;
pub mod flipbook;

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::paint::store::{compose_unclipped, save_objects, save_png, timestamp};
use crate::paint::{Canvas, History, Objects};
use formats::palette::Rgb;
use std::path::Path;

/// Faixa de opções, sem o nicho do McZee (que vira o botão do projeto).
pub const STRIP: (i32, i32, i32, i32) = (5, 56, 527, 48);
/// Nicho escuro à direita da faixa: mostra o estado do projeto e abre o painel.
pub const NICHE: (i32, i32, i32, i32) = (533, 56, 47, 48);

pub(crate) const BLACK: [u8; 3] = [0, 0, 0];
pub(crate) const WHITE: [u8; 3] = [255, 255, 255];
pub(crate) const RED: [u8; 3] = [250, 0, 57];
pub(crate) const GRAY: [u8; 3] = [140, 140, 140];
pub(crate) const DARK: [u8; 3] = [100, 100, 100];

/// Limpa o nicho (fundo escuro) antes de mostrar o estado do projeto nele.
pub(crate) fn clear_niche(f: &mut Frame) {
    f.rect(NICHE.0, NICHE.1, NICHE.2, NICHE.3, DARK);
}

pub(crate) fn inside(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

pub(crate) fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Artist, id)
}

/// Cartão branco com borda preta e rótulo centrado (Pablo 8 pt, ou a fonte reserva).
pub(crate) fn card(f: &mut Frame, a: &Assets, r: (i32, i32, i32, i32), text: &str, chosen: bool) {
    f.rect(r.0, r.1, r.2, r.3, WHITE);
    f.frame_rect(r.0, r.1, r.2, r.3, 1, BLACK);
    if chosen {
        f.frame_rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, 1, RED);
    }
    label(f, a, r, text);
}

/// Rótulo centrado em `r` (Pablo 8 pt, ou a fonte reserva), sem fundo.
pub(crate) fn label(f: &mut Frame, a: &Assets, r: (i32, i32, i32, i32), label: &str) {
    if label.is_empty() {
        return;
    }
    match a.font(8) {
        Some(font) => {
            let w = font.width(label) as i32;
            let h = font.height as i32;
            f.text_font(
                &font,
                r.0 + (r.2 - w) / 2,
                r.1 + (r.3 - h) / 2,
                label,
                BLACK,
            );
        }
        None => {
            let w = 8 * label.chars().count() as i32;
            f.text(r.0 + (r.2 - w) / 2, r.1 + (r.3 - 8) / 2, label, BLACK);
        }
    }
}

/// Qual projeto a tela de pintura está fazendo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Comics,
    Button,
    Sticker,
    Flipbook,
}

impl ProjectKind {
    /// Rótulo curto da caixa esquerda da barra inferior (no lugar de "Ateliê").
    pub fn label(self) -> &'static str {
        match self {
            ProjectKind::Comics => "Quadrinhos",
            ProjectKind::Button => "Botão",
            ProjectKind::Sticker => "Adesivo",
            ProjectKind::Flipbook => "Animação",
        }
    }

    /// Texto da caixa central da barra inferior (títulos das `STR#` do `ARTIST.EXE`).
    pub fn new_name(self) -> &'static str {
        match self {
            ProjectKind::Comics => "Fazendo uma História em Quadrinhos",
            ProjectKind::Button => "Fazendo um Botão",
            ProjectKind::Sticker => "Fazendo um Adesivo",
            ProjectKind::Flipbook => "Fazendo um Desenho Animado",
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

    /// Nome do último trabalho guardado deste tipo (`ultimo-quadrinhos.png`).
    pub fn last_name(self) -> String {
        format!("ultimo-{}.png", self.file_stem())
    }
}

/// Grava o quadro de um projeto de página única: `ultimo-<tipo>.png` sempre e,
/// se mudou, `<tipo>-AAAAMMDD-HHMMSS.png`. Os objetos vão achatados no PNG e,
/// ao lado, em `*.objetos.json` + `*.fundo.png` (como em `save_drawing_objects`).
pub fn save_picture(
    dir: &Path,
    kind: ProjectKind,
    c: &Canvas,
    objs: &Objects,
    pal: &[Rgb],
    changed: bool,
    now: u64,
) -> Result<(), String> {
    save_picture_meta(dir, kind, c, objs, pal, &[], changed, None, now)
}

/// Como [`save_picture`], com os metadados do projeto ao lado
/// (`<stem>.projeto.txt`) e, se `name` vier, o nome escolhido no lugar da
/// data (`adesivo-<nome>.png`).
#[allow(clippy::too_many_arguments)]
pub fn save_picture_meta(
    dir: &Path,
    kind: ProjectKind,
    c: &Canvas,
    objs: &Objects,
    pal: &[Rgb],
    meta: &[(String, String)],
    changed: bool,
    name: Option<&str>,
    now: u64,
) -> Result<(), String> {
    let fail = |e: std::io::Error, p: &Path| format!("não consegui gravar {}: {e}", p.display());
    std::fs::create_dir_all(dir).map_err(|e| fail(e, dir))?;
    let flat = compose_unclipped(c, objs);
    let last_stem = format!("ultimo-{}", kind.file_stem());
    let last = dir.join(kind.last_name());
    save_png(&flat, pal, &last).map_err(|e| fail(e, &last))?;
    save_objects(dir, &last_stem, c, objs, &flat, pal).map_err(|e| fail(e, dir))?;
    save_meta(dir, &last_stem, meta).map_err(|e| fail(e, dir))?;
    if changed || name.is_some() {
        let suffix = match name {
            Some(n) => file_safe(n),
            None => timestamp(now),
        };
        let stem = format!("{}-{}", kind.file_stem(), suffix);
        let named = dir.join(format!("{stem}.png"));
        save_png(&flat, pal, &named).map_err(|e| fail(e, &named))?;
        save_objects(dir, &stem, c, objs, &flat, pal).map_err(|e| fail(e, dir))?;
        save_meta(dir, &stem, meta).map_err(|e| fail(e, dir))?;
    }
    Ok(())
}

/// Nome de arquivo a partir de um nome digitado: letras, dígitos e hífens,
/// sem acentos nem espaços (como `timestamp`); vazio vira "sem-nome".
pub fn file_safe(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let c = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' | 'É' | 'Ê' | 'È' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
            _ => '-',
        };
        if c == '-' && out.ends_with('-') {
            continue;
        }
        out.push(c);
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "sem-nome".into()
    } else {
        out
    }
}

/// Nome do arquivo de metadados de um trabalho (`ultimo-adesivo.projeto.txt`):
/// linhas `chave=valor` (forma do adesivo, número de quadrinhos, título...).
pub fn meta_name(stem: &str) -> String {
    format!("{stem}.projeto.txt")
}

/// Grava os metadados `(chave, valor)`; sem pares, apaga o arquivo.
pub fn save_meta(dir: &Path, stem: &str, meta: &[(String, String)]) -> std::io::Result<()> {
    let path = dir.join(meta_name(stem));
    if meta.is_empty() {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    let mut text = String::new();
    for (k, v) in meta {
        let v: String = v.chars().filter(|c| !c.is_control()).collect();
        text.push_str(&format!("{k}={v}\n"));
    }
    std::fs::write(path, text)
}

/// Lê os metadados; arquivo ausente ou ilegível → lista vazia.
pub fn load_meta(dir: &Path, stem: &str) -> Vec<(String, String)> {
    std::fs::read_to_string(dir.join(meta_name(stem)))
        .map(|t| {
            t.lines()
                .filter_map(|l| l.split_once('='))
                .map(|(k, v)| (k.trim().to_string(), v.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Estado do projeto dentro da tela de pintura.
#[derive(Debug, Clone)]
pub enum Project {
    Comics(comics::Comics),
    Button(button::Button),
    Flipbook(Box<flipbook::Flipbook>),
}

impl Project {
    pub fn new(kind: ProjectKind) -> Project {
        match kind {
            ProjectKind::Comics => Project::Comics(comics::Comics::default()),
            ProjectKind::Button => Project::Button(button::Button::new(false)),
            ProjectKind::Sticker => Project::Button(button::Button::new(true)),
            ProjectKind::Flipbook => Project::Flipbook(Box::default()),
        }
    }

    pub fn kind(&self) -> ProjectKind {
        match self {
            Project::Comics(_) => ProjectKind::Comics,
            Project::Button(b) if b.sticker() => ProjectKind::Sticker,
            Project::Button(_) => ProjectKind::Button,
            Project::Flipbook(_) => ProjectKind::Flipbook,
        }
    }

    /// Metadados a gravar ao lado do trabalho (forma do adesivo, quadrinhos...).
    pub fn meta(&self) -> Vec<(String, String)> {
        match self {
            Project::Comics(k) => k.meta(),
            Project::Button(b) => vec![("forma".into(), b.kind().file_name().into())],
            Project::Flipbook(_) => Vec::new(),
        }
    }

    /// Aplica os metadados lidos de `*.projeto.txt` (antes de `prepare`).
    pub fn apply_meta(&mut self, meta: &[(String, String)]) {
        for (k, v) in meta {
            match self {
                Project::Button(b) if k == "forma" => {
                    if let Some(kind) = button::Kind::from_file_name(v) {
                        b.kind = kind;
                    }
                }
                Project::Comics(c) => c.apply_meta(k, v),
                _ => {}
            }
        }
    }

    /// Prepara o quadro recém-aberto (máscara de recorte do botão/adesivo).
    pub fn prepare(&self, c: &mut Canvas) {
        if let Project::Button(b) = self {
            b.prepare(c);
        }
    }

    /// O painel do projeto está aberto na faixa de opções?
    pub fn asking(&self) -> bool {
        match self {
            Project::Comics(c) => c.asking,
            Project::Button(b) => b.asking,
            Project::Flipbook(_) => false,
        }
    }

    /// Ajusta o recorte do quadro para um movimento que começa em `(cx, cy)`
    /// (coordenadas do quadro).
    pub fn clip_at(&self, c: &mut Canvas, cx: i32, cy: i32) {
        if let Project::Comics(k) = self {
            k.clip_at(c, cx, cy);
        }
    }

    /// Clique na tela: nicho do projeto e, com o painel aberto, a faixa de opções.
    /// `Some(mudou)` se o projeto tratou o clique; `None` deixa seguir para a pintura.
    pub fn click(&mut self, x: i32, y: i32, c: &mut Canvas, h: &mut History) -> Option<bool> {
        match self {
            Project::Comics(k) => k.click(x, y),
            Project::Button(b) => b.click(x, y, c, h),
            Project::Flipbook(_) => None,
        }
    }

    /// Painel do projeto na faixa de opções (quando `asking()`).
    pub fn draw_strip(&self, f: &mut Frame, a: &Assets) {
        match self {
            Project::Comics(k) => k.draw_strip(f, a),
            Project::Button(b) => b.draw_strip(f, a),
            Project::Flipbook(_) => {}
        }
    }

    /// Molduras ou forma por cima do quadro, mais o nicho do projeto.
    pub fn draw_overlay(&self, f: &mut Frame, a: &Assets) {
        match self {
            Project::Comics(k) => k.draw_overlay(f, a),
            Project::Button(b) => b.draw_overlay(f, a),
            Project::Flipbook(_) => {}
        }
    }

    /// Página a imprimir: o quadro com as molduras (quadrinhos) ou a folha de
    /// cópias (botão/adesivo).
    pub fn print_page(&self, c: &Canvas, pal: &[Rgb]) -> Canvas {
        match self {
            Project::Comics(k) => k.compose(c),
            Project::Button(b) => b.print_sheet(c, pal),
            Project::Flipbook(_) => c.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{CANVAS_H, CANVAS_W};

    fn pal() -> Vec<Rgb> {
        (0..256u32)
            .map(|i| Rgb {
                r: (255 - i) as u8,
                g: (i * 7 % 256) as u8,
                b: (i * 13 % 256) as u8,
            })
            .collect()
    }

    #[test]
    fn save_picture_names() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().join("desenhos");
        let c = Canvas::new(CANVAS_W, CANVAS_H);
        let o = Objects::default();
        save_picture(&d, ProjectKind::Comics, &c, &o, &pal(), false, 0).unwrap();
        assert!(d.join("ultimo-quadrinhos.png").exists());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1);
        save_picture(&d, ProjectKind::Button, &c, &o, &pal(), true, 1_793_404_800).unwrap();
        assert!(d.join("ultimo-botao.png").exists());
        assert!(d.join("botao-20261031-000000.png").exists());
        save_picture(&d, ProjectKind::Sticker, &c, &o, &pal(), true, 0).unwrap();
        assert!(d.join("adesivo-19700101-000000.png").exists());
        assert_eq!(ProjectKind::Sticker.last_name(), "ultimo-adesivo.png");
    }

    #[test]
    fn save_picture_unwritable_is_err() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("arquivo");
        std::fs::write(&file, b"x").unwrap();
        let c = Canvas::new(2, 2);
        let e = save_picture(
            &file.join("sub"),
            ProjectKind::Comics,
            &c,
            &Objects::default(),
            &pal(),
            true,
            0,
        )
        .unwrap_err();
        assert!(e.contains("não consegui gravar"));
    }

    #[test]
    fn meta_roundtrip_and_file_safe_names() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let meta = vec![
            ("forma".to_string(), "oval-alto".to_string()),
            ("titulo".to_string(), "Olá, mundo\n!".to_string()),
        ];
        save_meta(d, "ultimo-adesivo", &meta).unwrap();
        let back = load_meta(d, "ultimo-adesivo");
        assert_eq!(back[0], meta[0]);
        assert_eq!(back[1], ("titulo".to_string(), "Olá, mundo!".to_string()));
        save_meta(d, "ultimo-adesivo", &[]).unwrap();
        assert!(load_meta(d, "ultimo-adesivo").is_empty());
        assert_eq!(file_safe("Meu Botão Ótimo!"), "meu-botao-otimo");
        assert_eq!(file_safe("   "), "sem-nome");
        let mut p = Project::new(ProjectKind::Sticker);
        p.apply_meta(&[("forma".into(), "oval-largo".into())]);
        assert!(matches!(&p, Project::Button(b) if b.kind() == button::Kind::WideOval));
        assert_eq!(
            p.meta(),
            vec![("forma".to_string(), "oval-largo".to_string())]
        );
        // gravar com nome usa o nome no arquivo
        let c = Canvas::new(CANVAS_W, CANVAS_H);
        save_picture_meta(
            d,
            ProjectKind::Sticker,
            &c,
            &Objects::default(),
            &pal(),
            &p.meta(),
            false,
            Some("Dina"),
            0,
        )
        .unwrap();
        assert!(d.join("adesivo-dina.png").exists());
        assert_eq!(load_meta(d, "adesivo-dina")[0].1, "oval-largo");
        assert_eq!(load_meta(d, "ultimo-adesivo")[0].1, "oval-largo");
    }

    #[test]
    fn kind_roundtrip() {
        for k in [
            ProjectKind::Comics,
            ProjectKind::Button,
            ProjectKind::Sticker,
            ProjectKind::Flipbook,
        ] {
            assert_eq!(Project::new(k).kind(), k);
        }
    }
}
