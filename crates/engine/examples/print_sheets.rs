//! Folhas de conferência da impressão do Creative Writer: abre um `.MAX` no
//! Escritório, grava a tela da máquina de impressão do McZee (passo das
//! boas-vindas) e as folhas que sairiam, em PNG, para comparar com as
//! capturas do original (no Wine a impressão não sai: compare com as
//! figuras do assistente e com a prévia da máquina).
//!
//! ```text
//! print_sheets DADOS ARQUIVO.MAX PASTA [PREFIXO] [--sem-guias] [--sem-fundo]
//!              [--texto=TEXTO] [--passo=boas-vindas|copias|pronto|rex]
//! ```
//!
//! Grava `PREFIXO_maquina.png` (640×480) e `PREFIXO_folha_K.png` (144 dpi).
//! Com `ARQUIVO.MAX` = `-`, um documento novo com o `--texto` digitado. O
//! `--passo` escolhe a tela: as boas-vindas (botão do McZee), as cópias
//! (Segue), o pronto do "imprimir depressa" ou o aviso do Rex depois de
//! Segue sem impressora.

use engine::Frame;
use engine::assets::Assets;
use engine::paint::print::{Rgb8, save_sheet};
use engine::scene::{Input, MouseButton, Scene};
use engine::scenes::writer::Writer;
use engine::scenes::writer::print::{SheetOptions, print_document};
use engine::text_balloon::Button;
use std::path::{Path, PathBuf};

fn click(x: i32, y: i32) -> Input {
    Input::Click {
        button: MouseButton::Left,
        x,
        y,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "uso: print_sheets DADOS ARQUIVO.MAX PASTA [PREFIXO] [--sem-guias] [--sem-fundo]"
        );
        std::process::exit(2);
    }
    let a = Assets::open(Path::new(&args[1])).expect("dados");
    let file = PathBuf::from(&args[2]);
    let out = PathBuf::from(&args[3]);
    let rest: Vec<&str> = args[4..].iter().map(String::as_str).collect();
    let prefix = rest
        .iter()
        .find(|s| !s.starts_with("--"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            file.file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        });
    let opts = SheetOptions {
        guides: !rest.contains(&"--sem-guias"),
        background: !rest.contains(&"--sem-fundo"),
        pages: None,
    };
    std::fs::create_dir_all(&out).expect("pasta");
    let opt = |name: &str| {
        rest.iter()
            .find_map(|s| s.strip_prefix(name).map(str::to_string))
    };
    let tmp = std::env::temp_dir().join("print_sheets_user");
    let mut w = if args[2] == "-" {
        let mut w = Writer::new_blank(Some(tmp), &a);
        w.update(&[], 0.0, &a);
        let text = opt("--texto=").unwrap_or_else(|| "Teste de impressao".into());
        w.update(&[Input::Text(text)], 0.0, &a);
        w
    } else {
        Writer::open(Some(tmp), &file, &a).expect("documento")
    };
    w.update(&[], 0.0, &a);
    w.tools_mut().print.command = Some("/nao/existe/lp".into());
    let doc = print_document(w.editor(), &a, None);
    println!(
        "{}: {:?}, {} página(s), papel {:?}, borda {}, fundo {}",
        file.display(),
        doc.kind,
        doc.page_count(),
        doc.paper,
        doc.art.border.is_some(),
        doc.art.background.is_some()
    );
    for (k, s) in doc.sheets(&opts).iter().enumerate() {
        let p = out.join(format!("{prefix}_folha_{}.png", k + 1));
        save_sheet(s, &p).expect("folha");
        println!("{}", p.display());
    }
    // a ferramenta Imprimir e o botão do McZee (ou o "imprimir depressa")
    let step = opt("--passo=").unwrap_or_else(|| "boas-vindas".into());
    let quick = matches!(step.as_str(), "pronto" | "rex");
    w.update(&[click(492, 26)], 0.0, &a);
    w.update(&[click(if quick { 170 } else { 370 }, 75)], 0.0, &a);
    let press = |w: &mut Writer, b: Button| {
        let m = w.tools().print.machine().expect("máquina");
        let (x, y) = m.button_center(&a, b).expect("botão");
        w.update(&[click(x, y)], 0.0, &a);
    };
    match step.as_str() {
        "copias" => press(&mut w, Button::Next),
        "rex" => {
            press(&mut w, Button::Next);
            let start = std::time::Instant::now();
            while w
                .tools()
                .print
                .machine()
                .is_some_and(|m| m.alert().is_none())
                && start.elapsed().as_secs() < 60
            {
                w.update(&[], 0.05, &a);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        _ => {}
    }
    let mut f = Frame::new(engine::SCREEN_W, engine::SCREEN_H);
    w.draw(&mut f, &a);
    let img = Rgb8::from_rgba(f.width(), f.height(), f.pixels());
    let p = out.join(format!("{prefix}_maquina.png"));
    save_sheet(&img, &p).expect("tela");
    println!("{}", p.display());
}
