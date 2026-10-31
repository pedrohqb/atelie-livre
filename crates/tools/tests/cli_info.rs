use std::path::PathBuf;
use std::process::Command;

fn data(rel: &str) -> Option<PathBuf> {
    match std::env::var_os("ATELIE_DATA") {
        Some(d) => Some(PathBuf::from(d).join(rel)),
        None => {
            eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
            None
        }
    }
}

fn info(path: &std::path::Path) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_atelie-tools"))
        .arg("info")
        .arg(path)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn info_on_executable() {
    let Some(p) = data("ARTIST.EXE") else { return };
    let (code, out, _) = info(&p);
    assert_eq!(code, 0);
    assert!(out.contains("WHAT\t34\t"), "{out}");
    assert!(out.contains("KERNEL"));
}

#[test]
fn info_on_pic() {
    let Some(p) = data("ARTIST/ARTIST.PIC") else {
        return;
    };
    let (code, out, _) = info(&p);
    assert_eq!(code, 0);
    assert!(out.contains("2468"), "{out}");
}

#[test]
fn info_on_chnk() {
    let Some(p) = data("SHARED/STUFF/MAG100.SND") else {
        return;
    };
    let (code, out, _) = info(&p);
    assert_eq!(code, 0);
    assert!(out.contains("ICCC"), "{out}");
    assert!(out.contains("Braquissauro"), "{out}");
}

#[test]
fn info_on_unknown_file_fails() {
    let dir = std::env::temp_dir().join(format!("atelie-info-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("texto.txt");
    std::fs::write(&p, "isto não é um arquivo dos programas").unwrap();
    let (code, _, err) = info(&p);
    assert_eq!(code, 1);
    assert!(!err.is_empty());
    let (code, _, _) = info(&dir.join("nao-existe"));
    assert_eq!(code, 1);
    std::fs::remove_dir_all(&dir).unwrap();
}
