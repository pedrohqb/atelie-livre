use std::path::{Path, PathBuf};
use std::process::Command;

fn data() -> Option<PathBuf> {
    match std::env::var_os("ATELIE_DATA") {
        Some(d) => Some(PathBuf::from(d)),
        None => {
            eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
            None
        }
    }
}

fn run(args: &[&str], dir: &Path) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_atelie-tools"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn sample_to_png_and_back_with_artist_palette() {
    let Some(data) = data() else { return };
    let dir = tempfile::tempdir().unwrap();
    let samp = data.join("SHARED/USERS/MAGGIE/SAMP6.MAG");
    let (code, err) = run(&["mag2png", samp.to_str().unwrap(), "s6.png"], dir.path());
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("Fundo com Praia"), "{err}");
    let (code, err) = run(
        &[
            "png2mag",
            "s6.png",
            "s6.mag",
            "--title",
            "Praia",
            "--author",
            "Lisa",
            "--data",
            data.to_str().unwrap(),
        ],
        dir.path(),
    );
    assert_eq!(code, 0, "{err}");
    let m = std::fs::read(dir.path().join("s6.mag")).unwrap();
    assert_eq!(m.len() % 512, 0);
    let pic = formats::mag::read(&m).unwrap();
    assert_eq!(pic.title, "Praia");
    assert_eq!(pic.author, "Lisa");
    assert_eq!((pic.width, pic.height), (594, 347));
    // a paleta é a do ARTIST.EXE, e a imagem continua parecida com a original
    let exe = std::fs::read(data.join("ARTIST.EXE")).unwrap();
    let pal = formats::render::Palettes::from_exe(Some(&exe)).pal256;
    assert_eq!(pic.palette, pal);
    let orig = formats::mag::read(&std::fs::read(&samp).unwrap()).unwrap();
    let mut far = 0;
    for (a, b) in orig.pixels.iter().zip(&pic.pixels) {
        let (ca, cb) = (orig.palette[*a as usize], pic.palette[*b as usize]);
        let d = (ca.r as i32 - cb.r as i32).abs()
            + (ca.g as i32 - cb.g as i32).abs()
            + (ca.b as i32 - cb.b as i32).abs();
        if d > 60 {
            far += 1;
        }
    }
    assert!(
        far < orig.pixels.len() / 50,
        "{far} pixels muito diferentes"
    );
}

#[test]
fn bad_input_fails_with_message() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("x.mag"), b"nada").unwrap();
    let (code, err) = run(&["mag2png", "x.mag", "x.png"], dir.path());
    assert_eq!(code, 1);
    assert!(err.contains("x.mag"), "{err}");
    let (code, _) = run(&["png2mag", "nao-existe.png", "y.mag"], dir.path());
    assert_eq!(code, 1);
}
