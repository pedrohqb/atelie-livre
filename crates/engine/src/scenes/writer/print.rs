//! Imprimir, a torradeira (`office-ferramentas.md` §9, tarefa W18): a faixa
//! tem "imprimir depressa" (impressora com papel, `83a`) e "fazer escolhas"
//! (impressora com o McZee, `83b`). Os dois abrem a máquina de impressão do
//! McZee por cima do Escritório (barras pontilhadas de cinza): o depressa
//! vai direto ao "OK. Já estou pronto para imprimir!"; o do McZee passa
//! pelas escolhas do `WHAT 4c00` do `WRITER.EXE` (boas-vindas, fundo,
//! páginas, cópias). A máquina é a [`PrintPreview`] com o
//! [`WRITER_PREVIEW`] da Biblioteca; as folhas vêm de
//! [`crate::writer::print`] a partir das páginas que o editor desenha.
//!
//! Para os assistentes da Oficina de Projetos: [`print_document`] monta o
//! que imprimir de um editor (com o projeto: jornal, cartão ou faixa) e
//! [`super::Writer::print_document`] abre a máquina; o fim do roteiro
//! ("Tudo pronto!") vem quando [`super::Writer::printing`] volta a `false`.
//! Ver `docs/telas/creative-writer-impressao.md`.

use super::editor::Editor;
use super::tools::{Cx, StripButton, ToolId, ToolStrip, WHAT_PRINT, strip_button};
use crate::Frame;
use crate::assets::Assets;
use crate::paint::print::Rgb8;
use crate::scene::{Input, Key, MouseButton};
use crate::scenes::gallery_print::PrintPreview;
use crate::scenes::writer_library::WRITER_PREVIEW;
use crate::writer::assistant::Kind;
use crate::writer::borders::PageGeom;
use crate::writer::print::{CARD_PAGES, DEFAULT_PAPER, page_art, paper_of, project_kind};
pub use crate::writer::print::{PrintDoc, SheetKind, SheetOptions};
use std::path::Path;

/// Abrir a máquina: som da opção e o da tela do McZee (`0233` + `02f6`).
pub const SND_OPEN: [u16; 2] = [0x0233, 0x02f6];
/// Pontilhado cinza por cima do Escritório enquanto a máquina está aberta.
const DIM: [u8; 3] = [100, 100, 100];

const QUICK: usize = 0;
const STRIP: [StripButton; 2] = [
    strip_button(
        0x02ef,
        Some(0x02f1),
        (133, 60, 79, 35),
        (WHAT_PRINT, 0),
        "Imprimir depressa",
    ),
    strip_button(
        0x02f3,
        Some(0x02f5),
        (337, 59, 70, 37),
        (WHAT_PRINT, 0),
        "Fazer escolhas",
    ),
];

/// O documento do editor pronto para imprimir: as páginas como o editor as
/// desenha, a borda e o fundo. `kind` = o projeto da Oficina de Projetos
/// (`None`: o gravado no documento, ou documento do Escritório).
pub fn print_document(ed: &Editor, a: &Assets, kind: Option<Kind>) -> PrintDoc {
    let doc = ed.doc();
    let kind = SheetKind::of(kind.or_else(|| project_kind(doc, &ed.extra)));
    // as folhas do `PGD` contam (faixa de N folhas, cartão de 4) mesmo
    // quando o texto não chega a elas
    let mut n = ed
        .layout()
        .page_count()
        .max(ed.extra.page_flags.len())
        .max(1);
    if kind == SheetKind::Card {
        n = n.max(CARD_PAGES);
    }
    let pages = (0..n)
        .map(|k| {
            let f = ed.render_page(a, k);
            Rgb8::from_rgba(f.width(), f.height(), f.pixels())
        })
        .collect();
    let d = &doc.dop;
    PrintDoc {
        kind,
        geom: PageGeom {
            width: f64::from(d.page_width),
            height: f64::from(d.page_height),
            margin_top: f64::from(d.margin_top),
            margin_left: f64::from(d.margin_left),
            margin_bottom: f64::from(d.margin_bottom),
            margin_right: f64::from(d.margin_right),
        },
        paper: paper_of(ed.extra.printer.as_deref()).unwrap_or(DEFAULT_PAPER),
        pages,
        art: page_art(doc, &ed.extra),
    }
}

/// Um `.MAX` pronto para imprimir (a impressora da Biblioteca).
pub fn document_from_file(path: &Path, a: &Assets) -> Result<PrintDoc, String> {
    let mut ed = Editor::blank(None, a);
    ed.open(path)?;
    Ok(print_document(&ed, a, None))
}

/// Pinta o pontilhado cinza da tela por baixo da máquina (as molduras
/// laranja e pretas ficam como estão).
fn dim(f: &mut Frame) {
    const KEEP: [[u8; 3]; 3] = [[255, 140, 50], [185, 101, 36], [0, 0, 0]];
    for y in 0..f.height() {
        for x in 0..f.width() {
            let p = f.pixel(x, y);
            if (x + y) % 2 == 0 && !KEEP.contains(&[p[0], p[1], p[2]]) {
                f.put(x as i32, y as i32, DIM);
            }
        }
    }
}

#[derive(Default)]
pub struct PrintStrip {
    machine: Option<PrintPreview>,
    /// Programa de impressão (`ATELIE_LP`, senão `lp`); os testes trocam.
    pub command: Option<String>,
}

impl PrintStrip {
    /// A máquina aberta.
    pub fn machine(&self) -> Option<&PrintPreview> {
        self.machine.as_ref()
    }

    pub fn is_open(&self) -> bool {
        self.machine.is_some()
    }

    /// Abre a máquina para o documento do editor (`quick` = imprimir
    /// depressa; `kind` = projeto, ver [`print_document`]).
    pub fn open(&mut self, ed: &Editor, cx: &mut Cx, quick: bool, kind: Option<Kind>) {
        for s in SND_OPEN {
            cx.sound(s);
        }
        cx.close_help();
        let doc = print_document(ed, cx.assets, kind);
        let command = self
            .command
            .clone()
            .or_else(|| std::env::var("ATELIE_LP").ok())
            .unwrap_or_else(|| "lp".into());
        cx.log(format!(
            "imprimir: {} página(s), {:?}{}",
            doc.page_count(),
            doc.kind,
            if quick { ", depressa" } else { "" }
        ));
        self.machine = Some(PrintPreview::machine(
            &WRITER_PREVIEW,
            doc,
            quick,
            ed.dir().map(Path::to_path_buf),
            &command,
            cx.assets,
        ));
    }

    /// Repassa uma entrada à máquina; fecha-a quando ela termina.
    fn input(&mut self, cx: &mut Cx, i: &Input) -> bool {
        let Some(m) = self.machine.as_mut() else {
            return false;
        };
        if m.input(i, cx.assets, cx.fx) {
            self.machine = None;
            cx.log("imprimir: a máquina fechou");
        }
        true
    }
}

impl ToolStrip for PrintStrip {
    fn id(&self) -> ToolId {
        ToolId::Print
    }

    fn modal(&self) -> bool {
        self.machine.is_some()
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if self.machine.is_some() {
            let click = Input::Click {
                button: MouseButton::Left,
                x,
                y,
            };
            return self.input(cx, &click);
        }
        match StripButton::find(&STRIP, x, y) {
            Some(k) => {
                self.open(ed, cx, k == QUICK, None);
                true
            }
            None => false,
        }
    }

    fn key(&mut self, _ed: &mut Editor, cx: &mut Cx, key: Key, ctrl: bool) -> bool {
        if self.machine.is_none() {
            return false;
        }
        let i = Input::Key {
            key,
            ctrl,
            alt: false,
        };
        self.input(cx, &i)
    }

    fn text(&mut self, _ed: &mut Editor, cx: &mut Cx, s: &str) -> bool {
        self.input(cx, &Input::Text(s.to_string()))
    }

    fn update(&mut self, _ed: &mut Editor, cx: &mut Cx, dt: f32) {
        if let Some(m) = self.machine.as_mut() {
            let fx = m.update(dt, cx.assets);
            cx.fx.extend(fx);
        }
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
    }

    fn draw_front(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        if let Some(m) = &self.machine {
            dim(f);
            m.draw(f, a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Writer;
    use super::*;
    use crate::scene::{Effect, Scene};
    use crate::scenes::gallery_print::Stage;
    use crate::scenes::testutil::{assets, dump, empty_assets};
    use crate::text_balloon::Button;
    use crate::writer::print::paper_of;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn key(k: Key) -> Input {
        Input::Key {
            key: k,
            ctrl: false,
            alt: false,
        }
    }

    fn stage(w: &Writer) -> Stage {
        w.tools().print.machine().expect("máquina aberta").stage()
    }

    fn press(w: &mut Writer, a: &Assets, b: Button) -> Vec<Effect> {
        let (x, y) = w
            .tools()
            .print
            .machine()
            .unwrap()
            .button_center(a, b)
            .unwrap();
        w.update(&[click(x, y)], 0.0, a)
    }

    fn settle(w: &mut Writer, a: &Assets) -> Vec<Effect> {
        let mut fx = Vec::new();
        let start = std::time::Instant::now();
        while w.printing() && stage(w) == Stage::Printing && start.elapsed().as_secs() < 120 {
            fx.extend(w.update(&[], 0.05, a));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        fx
    }

    fn writer(a: &Assets, dir: &Path) -> Writer {
        let mut w = Writer::new_blank(Some(dir.to_path_buf()), a);
        w.update(&[], 0.0, a);
        w.tools_mut().print.command = Some("true".into());
        w
    }

    #[test]
    fn mczee_button_walks_the_machine_and_typing_goes_to_it() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut w = writer(&a, dir.path());
        w.update(&[Input::Text("Oi".into())], 0.0, &a);
        w.update(&[click(492, 26)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Print);
        let fx = w.update(&[click(370, 75)], 0.0, &a);
        assert!(w.printing());
        for s in SND_OPEN {
            assert!(fx.contains(&super::super::tools::snd(s)), "{fx:?}");
        }
        assert_eq!(stage(&w), Stage::Welcome);
        // modal: clicar na barra de ferramentas não troca de ferramenta
        w.update(&[click(30, 30)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Print);
        press(&mut w, &a, Button::Next);
        // documento de uma página, sem fundo: direto às cópias
        assert_eq!(stage(&w), Stage::Copies);
        w.update(&[Input::Text("3".into())], 0.0, &a);
        assert_eq!(w.doc().text(), "Oi");
        w.update(&[key(Key::Enter)], 0.0, &a);
        assert_eq!(stage(&w), Stage::Ready);
        assert_eq!(w.tools().print.machine().unwrap().choices().copies, 3);
        let mut f = crate::Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        w.draw(&mut f, &a);
        // o Escritório fica pontilhado de cinza por trás
        let dimmed = (0..640u32)
            .flat_map(|x| (0..30u32).map(move |y| (x, y)))
            .filter(|&(x, y)| (x + y) % 2 == 0 && f.pixel(x, y)[..3] == DIM)
            .count();
        assert!(dimmed > 640 * 30 / 4, "{dimmed}");
        press(&mut w, &a, Button::Next);
        settle(&mut w, &a);
        assert_eq!(stage(&w), Stage::Done);
        assert_eq!(w.tools().print.machine().unwrap().files().len(), 1);
        press(&mut w, &a, Button::Stop);
        assert!(!w.printing());
        assert_eq!(w.tool(), ToolId::Print);
    }

    #[test]
    fn quick_button_and_project_api() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut w = writer(&a, dir.path());
        w.update(&[click(492, 26)], 0.0, &a);
        w.update(&[click(170, 75)], 0.0, &a);
        assert_eq!(stage(&w), Stage::Ready);
        // Esc fecha
        w.update(&[key(Key::Escape)], 0.0, &a);
        assert!(!w.printing());
        // pelos assistentes: escolhe o Imprimir e abre a máquina do cartão
        let mut w = writer(&a, dir.path());
        let fx = w.print_document(Some(Kind::Card), false, &a);
        assert!(!fx.is_empty());
        assert_eq!(w.tool(), ToolId::Print);
        let m = w.tools().print.machine().unwrap();
        assert_eq!(m.stage(), Stage::Welcome);
        assert_eq!(m.document().unwrap().kind, SheetKind::Card);
    }

    /// Com os dados reais: as folhas dos exemplos do Max (cartão, faixa,
    /// jornal, documento com borda). Com `ATELIE_PRINT_DUMP=pasta`, grava
    /// cada folha em PNG (`cw-print_<exemplo>_<k>.png`) e a tela da máquina.
    #[test]
    fn real_samples_compose_their_sheets() {
        let Some(a) = assets() else { return };
        let out = std::env::var_os("ATELIE_PRINT_DUMP").map(std::path::PathBuf::from);
        let samples = crate::scenes::writer_library::list_samples(a.data_dir());
        let find = |n: &str| {
            samples
                .iter()
                .find(|p| p.file_name().is_some_and(|f| f.eq_ignore_ascii_case(n)))
                .unwrap_or_else(|| panic!("{n}"))
                .clone()
        };
        for (name, kind, sheets) in [
            ("SAMP1.MAX", SheetKind::Card, 1),
            ("SAMP7.MAX", SheetKind::Banner, 0),
            ("SAMP14.MAX", SheetKind::Pages, 0),
            ("SAMP17.MAX", SheetKind::Banner, 0),
            ("SAMP18.MAX", SheetKind::Pages, 0),
        ] {
            let path = find(name);
            let doc = document_from_file(&path, &a).unwrap();
            assert_eq!(doc.kind, kind, "{name}");
            assert_eq!(doc.paper, (612, 792), "{name}");
            let opts = SheetOptions::default();
            let s = doc.sheets(&opts);
            let want = if sheets == 0 {
                doc.page_count()
            } else {
                sheets
            };
            assert_eq!(s.len(), want, "{name}");
            let ink = |img: &Rgb8| img.px.iter().filter(|p| **p != [255, 255, 255]).count();
            assert!(
                s.iter().all(|img| ink(img) > 100),
                "{name}: folha em branco"
            );
            if matches!(name, "SAMP14.MAX" | "SAMP18.MAX") {
                assert!(doc.art.border.is_some(), "{name}: borda");
            }
            if kind == SheetKind::Card {
                // a capa (em cima, à esquerda) e o miolo têm alguma coisa
                let bare = SheetOptions {
                    guides: false,
                    ..opts
                };
                let img = &doc.sheets(&bare)[0];
                let (hw, hh) = (img.w / 2, img.h / 2);
                for (qx, qy) in [(0, 0), (0, hh)] {
                    let n = (qy..qy + hh)
                        .flat_map(|y| (qx..qx + hw).map(move |x| (x, y)))
                        .filter(|&(x, y)| img.px[(y * img.w + x) as usize] != [255, 255, 255])
                        .count();
                    assert!(n > 0, "{name}: quarto ({qx}, {qy}) vazio");
                }
            }
            if let Some(dir) = &out {
                let stem = name.trim_end_matches(".MAX").to_lowercase();
                for (k, img) in s.iter().enumerate() {
                    let p = dir.join(format!("cw-print_{stem}_{}.png", k + 1));
                    crate::paint::print::save_sheet(img, &p).unwrap();
                }
            }
        }
        // a máquina com o jornal, como em `projects_19`
        let dir = tempfile::tempdir().unwrap();
        let mut w = Writer::open(Some(dir.path().to_path_buf()), &find("SAMP14.MAX"), &a).unwrap();
        w.update(&[], 0.0, &a);
        assert_eq!(
            paper_of(w.editor().extra.printer.as_deref()),
            Some((612, 792))
        );
        w.update(&[click(492, 26), click(370, 75)], 0.0, &a);
        let mut f = crate::Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        w.draw(&mut f, &a);
        dump(&f, "cw_print_machine_news");
        if let Some(dir) = &out {
            let img = Rgb8::from_rgba(f.width(), f.height(), f.pixels());
            crate::paint::print::save_sheet(&img, &dir.join("cw-print_machine_news.png")).unwrap();
        }
    }
}
