use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn data_dir() -> Option<PathBuf> {
    match std::env::var_os("ATELIE_DATA") {
        Some(d) => Some(PathBuf::from(d)),
        None => {
            eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
            None
        }
    }
}

fn extract(input: &Path, output: &Path) -> i32 {
    let out = Command::new(env!("CARGO_BIN_EXE_atelie-tools"))
        .arg("extract")
        .arg(input)
        .arg(output)
        .output()
        .unwrap();
    out.status.code().unwrap_or(-1)
}

fn report(output: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(output.join("report.json")).unwrap()).unwrap()
}

fn count(dir: &Path, ext: &str) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter(|e| {
                e.as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|x| x == ext)
            })
            .count()
        })
        .unwrap_or(0)
}

/// Copia `src` para `dst`, opcionalmente com todos os nomes em minúsculas.
fn copy_tree(src: &Path, dst: &Path, lower: bool) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let mut name = e.file_name().to_string_lossy().into_owned();
        if lower {
            name = name.to_lowercase();
        }
        let to = dst.join(name);
        if e.path().is_dir() {
            copy_tree(&e.path(), &to, lower);
        } else {
            std::fs::copy(e.path(), to).unwrap();
        }
    }
}

fn check_full(out: &Path) {
    assert_eq!(count(&out.join("ARTIST/bitmaps"), "png"), 2468);
    assert_eq!(count(&out.join("WRITER/bitmaps"), "png"), 2311);
    assert_eq!(count(&out.join("ARTIST/sounds"), "wav"), 131);
    assert_eq!(count(&out.join("WRITER/sounds"), "wav"), 122);
    let r = report(out);
    assert_eq!(
        r["failures"].as_array().unwrap().len(),
        0,
        "{:#}",
        r["failures"]
    );
    assert_eq!(r["missing"].as_array().unwrap().len(), 0);
    assert!(out.join("index.html").exists());
}

#[test]
fn full_extraction_and_second_run() {
    let Some(data) = data_dir() else { return };
    let out = tempfile::tempdir().unwrap();
    assert_eq!(extract(&data, out.path()), 0);
    check_full(out.path());
    // amostras de cada tipo de saída
    let exe = out.path().join("resources/ARTIST.EXE");
    assert!(exe.join("WHAT/0201.bin").exists());
    assert!(exe.join("WHAT/0201.hex.txt").exists());
    assert!(
        std::fs::read_to_string(exe.join("STR#/0206.txt"))
            .unwrap()
            .starts_with("Pablo Graphos")
    );
    assert!(
        exe.join("ICON")
            .read_dir()
            .unwrap()
            .any(|e| e.unwrap().path().extension().unwrap() == "png")
    );
    let docs = out.path().join("documents/SHARED/STUFF/MAG100.ART");
    assert!(
        out.path()
            .join("documents/SHARED/STUFF/MAG100.ART.json")
            .exists()
    );
    assert_eq!(count(&docs, "png"), 128);
    assert_eq!(
        count(&out.path().join("documents/SHARED/STUFF/MAG100.SND"), "wav"),
        50
    );
    let first = std::fs::read(out.path().join("report.json")).unwrap();
    assert_eq!(extract(&data, out.path()), 0);
    assert_eq!(
        std::fs::read(out.path().join("report.json")).unwrap(),
        first
    );
}

#[test]
fn lowercase_copy_works() {
    let Some(data) = data_dir() else { return };
    let src = tempfile::tempdir().unwrap();
    copy_tree(&data, src.path(), true);
    let out = tempfile::tempdir().unwrap();
    assert_eq!(extract(src.path(), out.path()), 0);
    check_full(out.path());
}

#[test]
fn partial_install_reports_missing() {
    let Some(data) = data_dir() else { return };
    let src = tempfile::tempdir().unwrap();
    std::fs::copy(data.join("ARTIST.EXE"), src.path().join("ARTIST.EXE")).unwrap();
    copy_tree(&data.join("ARTIST"), &src.path().join("ARTIST"), false);
    let out = tempfile::tempdir().unwrap();
    assert_eq!(extract(src.path(), out.path()), 0);
    let missing: Vec<String> = report(out.path())["missing"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        missing,
        ["WRITER.EXE", "WRITER/WRITER.PIC", "WRITER/WRITER.SND"]
    );
    assert_eq!(count(&out.path().join("ARTIST/bitmaps"), "png"), 2468);
}

#[test]
fn user_document_with_odd_name_detected() {
    // documentos de usuários novos ficam em USERS/, irmã de SHARED/
    let Some(data) = data_dir() else { return };
    let src = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(src.path().join("USERS/JOAO")).unwrap();
    std::fs::create_dir_all(src.path().join("SHARED/USERS/X")).unwrap();
    let sample = data.join("SHARED/USERS/MAGGIE/SAMP1.MAG");
    std::fs::copy(&sample, src.path().join("USERS/JOAO/MEU DESENHO.MAG")).unwrap();
    std::fs::copy(&sample, src.path().join("SHARED/USERS/X/meu desenho")).unwrap();
    let out = tempfile::tempdir().unwrap();
    assert_eq!(extract(src.path(), out.path()), 0);
    let r = report(out.path());
    for path in ["USERS/JOAO/MEU DESENHO.MAG", "SHARED/USERS/X/meu desenho"] {
        let doc = r["processed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["path"] == path)
            .unwrap();
        assert_eq!(doc["kind"], "chnk");
        assert!(doc["items_ok"].as_u64().unwrap() > 0);
    }
}

#[test]
fn painting_layers_use_the_document_palette() {
    use formats::{chnk, palette, pkbm};
    let Some(data) = data_dir() else { return };
    let src = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(src.path().join("SHARED")).unwrap();
    std::fs::copy(data.join("ARTIST.EXE"), src.path().join("ARTIST.EXE")).unwrap();
    std::fs::copy(
        data.join("SHARED/USERS/MAGGIE/SAMP2.MAG"),
        src.path().join("SHARED/SAMP2.MAG"),
    )
    .unwrap();
    let out = tempfile::tempdir().unwrap();
    assert_eq!(extract(src.path(), out.path()), 0);
    let doc = std::fs::read(src.path().join("SHARED/SAMP2.MAG")).unwrap();
    let f = chnk::parse(&doc).unwrap();
    let ctbl_part = f.chunks.iter().find(|c| c.tag.0 == *b"CTBL").unwrap().parts[0].data;
    let ctbl = palette::parse_ctbl(ctbl_part).unwrap();
    let layer = f
        .chunks
        .iter()
        .find(|c| c.tag.0 == *b"PKBM" && c.id == 0)
        .unwrap();
    let bitmap = pkbm::decode(layer.parts[0].data).unwrap();
    assert_eq!(bitmap.depth, 8);
    let png_path = out
        .path()
        .join("documents/SHARED/SAMP2.MAG/PKBM-0-0-PKBM.png");
    let mut reader = png::Decoder::new(std::fs::File::open(png_path).unwrap())
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut buf).unwrap();
    for (i, &idx) in bitmap.pixels.iter().enumerate() {
        let c = ctbl[idx as usize];
        assert_eq!(&buf[i * 4..i * 4 + 3], &[c.r, c.g, c.b], "pixel {i}");
    }
}

#[test]
fn unreadable_file_is_reported_and_extraction_continues() {
    let Some(data) = data_dir() else { return };
    let src = tempfile::tempdir().unwrap();
    std::fs::copy(data.join("ARTIST.EXE"), src.path().join("ARTIST.EXE")).unwrap();
    std::fs::create_dir_all(src.path().join("SHARED")).unwrap();
    std::os::unix::fs::symlink("/nao/existe", src.path().join("SHARED/quebrado")).unwrap();
    let out = tempfile::tempdir().unwrap();
    assert_eq!(extract(src.path(), out.path()), 1);
    let r = report(out.path());
    assert!(
        r["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == "SHARED/quebrado")
    );
    assert!(
        out.path()
            .join("resources/ARTIST.EXE/WHAT/0201.bin")
            .exists()
    );
}

/// CHNK de 56 bytes cuja única parte (aninhada) é o próprio arquivo.
fn self_nested_chnk() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"CHNKTEST");
    v.extend_from_slice(&[4, 0, 4, 0, 1, 0, 0, 3]);
    v.extend_from_slice(&24u32.to_le_bytes());
    v.extend_from_slice(&56u32.to_le_bytes());
    v.extend_from_slice(&[0x00, 0x02, 1, 0, 0xff, 0xff, 0xff, 0xff]);
    v.extend_from_slice(&[24, 0]);
    v.extend_from_slice(b"ATAG");
    v.extend_from_slice(&[0, 0, 1, 0, 4, 0]);
    v.extend_from_slice(b"ATAG");
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&56u32.to_le_bytes());
    assert_eq!(v.len(), 56);
    v
}

fn walk_depth(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| 1 + walk_depth(&e.path()))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

#[test]
fn self_nested_chnk_is_reported_not_recursed() {
    let src = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(src.path().join("SHARED")).unwrap();
    std::fs::write(src.path().join("SHARED/laco.chk"), self_nested_chnk()).unwrap();
    let out = tempfile::tempdir().unwrap();
    assert_eq!(extract(src.path(), out.path()), 1);
    let r = report(out.path());
    assert!(
        r["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == "SHARED/laco.chk")
    );
    let depth = walk_depth(&out.path().join("documents"));
    assert!(depth < 10, "profundidade {depth}");
}

#[test]
fn output_inside_input_is_not_reprocessed() {
    let Some(data) = data_dir() else { return };
    let src = tempfile::tempdir().unwrap();
    std::fs::copy(data.join("ARTIST.EXE"), src.path().join("ARTIST.EXE")).unwrap();
    std::fs::create_dir_all(src.path().join("ARTIST")).unwrap();
    std::fs::copy(
        data.join("ARTIST/ARTIST.SND"),
        src.path().join("ARTIST/ARTIST.SND"),
    )
    .unwrap();
    let out = src.path().join("saida");
    assert_eq!(extract(src.path(), &out), 0);
    let first = std::fs::read(out.join("report.json")).unwrap();
    assert_eq!(extract(src.path(), &out), 0);
    assert_eq!(std::fs::read(out.join("report.json")).unwrap(), first);
}

#[test]
fn corrupt_file_is_reported_and_extraction_continues() {
    let Some(data) = data_dir() else { return };
    let src = tempfile::tempdir().unwrap();
    std::fs::copy(data.join("ARTIST.EXE"), src.path().join("ARTIST.EXE")).unwrap();
    std::fs::create_dir_all(src.path().join("SHARED")).unwrap();
    let mut doc = std::fs::read(data.join("SHARED/USERS/MAGGIE/SAMP1.MAG")).unwrap();
    doc.truncate(4000);
    std::fs::write(src.path().join("SHARED/quebrado.mag"), doc).unwrap();
    let out = tempfile::tempdir().unwrap();
    assert_eq!(extract(src.path(), out.path()), 1);
    let r = report(out.path());
    assert!(
        r["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["path"] == "SHARED/quebrado.mag")
    );
    assert!(
        out.path()
            .join("resources/ARTIST.EXE/WHAT/0201.bin")
            .exists()
    );
}

#[test]
fn empty_dir_exits_2() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    assert_eq!(extract(src.path(), out.path()), 2);
}
