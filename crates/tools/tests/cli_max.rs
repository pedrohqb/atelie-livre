use formats::max::{self, CharProps, Document, Run};
use std::path::{Path, PathBuf};
use std::process::Command;

fn tools(args: &[&std::ffi::OsStr]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_atelie-tools"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn sample(dir: &Path) -> PathBuf {
    let mut d = Document::from_text("Feliz Dia\nsegunda linha");
    d.add.title = "Cartão".into();
    d.add.author = "Ana".into();
    d.char_runs = vec![
        Run {
            start: 0,
            end: 5,
            props: CharProps {
                bold: true,
                size: Some(48),
                ..CharProps::default()
            },
        },
        Run {
            start: 5,
            end: d.len(),
            props: CharProps::default(),
        },
    ];
    let p = dir.join("CARTAO.MAX");
    std::fs::write(&p, max::write(&d)).unwrap();
    p
}

#[test]
fn max2txt_prints_text_and_styles() {
    let dir = tempfile::tempdir().unwrap();
    let p = sample(dir.path());
    let (code, out, err) = tools(&["max2txt".as_ref(), p.as_os_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("título: Cartão"), "{out}");
    assert!(
        out.contains("== texto ==\nFeliz Dia\nsegunda linha\n"),
        "{out}"
    );
    assert!(out.contains("[0, 5) \"Feliz\" negrito, 48 pt"), "{out}");
    // só o texto, num arquivo
    let txt = dir.path().join("cartao.txt");
    let (code, _, err) = tools(&[
        "max2txt".as_ref(),
        p.as_os_str(),
        txt.as_os_str(),
        "--texto".as_ref(),
    ]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        std::fs::read_to_string(&txt).unwrap(),
        "Feliz Dia\nsegunda linha\n"
    );
}

#[test]
fn info_summarizes_ink_documents() {
    let dir = tempfile::tempdir().unwrap();
    let p = sample(dir.path());
    let (code, out, err) = tools(&["info".as_ref(), p.as_os_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("creator: INK "), "{out}");
    assert!(out.contains("INK (documento do Creative Writer)"), "{out}");
    assert!(out.contains("autor: Ana"), "{out}");
    assert!(out.contains("texto: 25 caracteres, 3 parágrafos"), "{out}");
}

#[test]
fn max2txt_rejects_other_files() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x.MAX");
    std::fs::write(&p, b"nada").unwrap();
    let (code, _, err) = tools(&["max2txt".as_ref(), p.as_os_str()]);
    assert_eq!(code, 1);
    assert!(err.contains("x.MAX"), "{err}");
}

#[test]
fn max2txt_on_sample_card() {
    let Some(d) = std::env::var_os("ATELIE_DATA") else {
        eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
        return;
    };
    let p = PathBuf::from(d).join("SHARED/USERS/MAX/SAMP1.MAX");
    let (code, out, err) = tools(&["max2txt".as_ref(), p.as_os_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("título: A vida é um osso duro de roer"),
        "{out}"
    );
    assert!(out.contains("projeto: cartão (card)"), "{out}");
    assert!(out.contains("moldura 1: moldura de página"), "{out}");
    assert!(
        out.contains("\"Namorados\" negrito, fonte 1 (Lucida Sans), 48 pt, cor #fa0039"),
        "{out}"
    );
}
