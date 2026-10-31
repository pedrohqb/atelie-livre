use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_atelie-livre"))
        .args(args)
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .env(
            "XDG_CONFIG_HOME",
            std::env::temp_dir().join("atelie-smoke-cfg"),
        )
        .output()
        .unwrap()
}

#[test]
fn runs_120_frames_headless() {
    let Some(data) = std::env::var_os("ATELIE_DATA") else {
        eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
        return;
    };
    let data = data.to_string_lossy().into_owned();
    let out = run(&["--data", &data, "--window", "--frames", "120"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run(&["--data", &data, "--window", "--browser", "--frames", "30"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn missing_data_dir_fails_with_message() {
    let out = run(&[
        "--data",
        "/tmp/nao-existe-atelie",
        "--window",
        "--frames",
        "5",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(!out.stderr.is_empty());
}
