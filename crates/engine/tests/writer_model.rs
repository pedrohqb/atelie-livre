//! Confere as constantes do modelo rico do Escritório (`engine::writer`)
//! com os recursos do `WRITER.EXE` original. Pulado sem `ATELIE_DATA`.

use engine::writer::doc::{CSCH, DEFAULT_FONT, FONTS, PageNumber};
use formats::{macres, ne};
use std::path::PathBuf;

fn writer_exe() -> Option<Vec<u8>> {
    let Some(d) = std::env::var_os("ATELIE_DATA").map(PathBuf::from) else {
        eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
        return None;
    };
    Some(std::fs::read(d.join("WRITER.EXE")).expect("WRITER.EXE"))
}

fn strs(exe: &ne::NeFile, id: u16) -> Vec<String> {
    let r = exe
        .find("STR#", id)
        .unwrap_or_else(|| panic!("STR# {id:#x}"));
    macres::parse_str_list(r.data).unwrap()
}

#[test]
fn csch_colors_are_the_writer_table() {
    let Some(data) = writer_exe() else { return };
    let exe = ne::parse(&data).unwrap();
    let r = exe
        .find("CSCH", 0x80)
        .or_else(|| exe.of_kind("CSCH").next())
        .expect("CSCH");
    let d = r.data;
    let n = u16::from_le_bytes([d[0], d[1]]) as usize;
    assert_eq!(n, 16);
    for (k, c) in CSCH.iter().enumerate() {
        let at = 6 + k * 6;
        // estilo Mac: o byte alto de cada componente é a cor
        let (r, g, b) = (d[at + 1], d[at + 3], d[at + 5]);
        assert_eq!((r, g, b), (c.r, c.g, c.b), "cor {k}");
    }
}

#[test]
fn font_names_and_page_label_come_from_the_exe() {
    let Some(data) = writer_exe() else { return };
    let exe = ne::parse(&data).unwrap();
    let fonts = strs(&exe, 0x206);
    for (k, (name, _)) in FONTS.iter().enumerate() {
        assert_eq!(fonts[k], *name, "fonte {k}");
    }
    let doc = strs(&exe, 0x800);
    assert_eq!(doc[1], DEFAULT_FONT);
    assert_eq!(
        doc[2].replace("%d", "1"),
        PageNumber::Label.text(0).unwrap()
    );
}
