//! Os 21 documentos de exemplo do Max (`SHARED/USERS/MAX/SAMP*.MAX`) no
//! Escritório: abrem, voltam iguais pela conversão documento ↔ `.MAX`, são
//! diagramados e desenhados. Com `ATELIE_DUMP`, grava a tela de cada um
//! (`writer_sampN.ppm`) e as folhas inteiras (`writer_sampN_pK.ppm`) para
//! comparar com o original. Pulado sem `ATELIE_DATA`.

use engine::assets::Assets;
use engine::scene::Scene;
use engine::scenes::Writer;
use engine::writer::convert::{from_max, to_max};
use engine::writer::doc::Item;
use engine::writer::store;
use engine::{Frame, SCREEN_H, SCREEN_W};
use formats::max;
use std::path::{Path, PathBuf};

fn data_dir() -> Option<PathBuf> {
    match std::env::var_os("ATELIE_DATA") {
        Some(d) => Some(PathBuf::from(d)),
        None => {
            eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
            None
        }
    }
}

fn samples(data: &Path) -> Vec<PathBuf> {
    let dir = data.join("SHARED").join("USERS").join("MAX");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("max")))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn dump(f: &Frame, name: &str) {
    let Some(dir) = std::env::var_os("ATELIE_DUMP") else {
        return;
    };
    let (w, h) = (f.width(), f.height());
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    for p in f.pixels().chunks(4) {
        out.extend_from_slice(&p[..3]);
    }
    std::fs::write(PathBuf::from(dir).join(format!("{name}.ppm")), out).unwrap();
}

#[test]
fn samples_open_roundtrip_and_draw() {
    let Some(data) = data_dir() else { return };
    let assets = Assets::open(&data).unwrap();
    let files = samples(&data);
    assert_eq!(files.len(), 21);
    let mut pictures = (0, 0);
    for path in &files {
        let name = path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase();
        let bytes = std::fs::read(path).unwrap();
        let m = max::read(&bytes).unwrap();
        let (doc, extra) = from_max(&m);
        // o texto é o mesmo (sem as âncoras das figuras do Maçã)
        assert_eq!(doc.text(), m.plain_text(), "{name}");
        let rebus = doc
            .paras
            .iter()
            .flat_map(|p| &p.items)
            .filter(|i| matches!(i, Item::Rebus(_)))
            .count();
        let anchors = (0..m.len())
            .filter(|&cp| m.char_props_at(cp).anchor)
            .count();
        assert_eq!(rebus, anchors, "{name}");
        // e volta igual pelo arquivo
        let w = Writer::open(None, path, &assets).unwrap();
        let ed = w.editor();
        let l = ed.layout();
        let pages = l.page_starts(ed.doc());
        let again = max::read(&max::write(&to_max(&doc, &extra, &pages, None))).unwrap();
        let (doc2, extra2) = from_max(&again);
        assert_eq!(doc2, doc, "{name}");
        assert_eq!(extra2.objects, extra.objects, "{name}");
        assert_eq!(extra2.frames, extra.frames, "{name}");
        assert_eq!(again.add, m.add, "{name}");
        assert_eq!(again.dop, m.dop, "{name}");
        assert_eq!(again.word_sounds, m.word_sounds, "{name}");
        // os objetos com figura são desenhados
        for o in &doc.objects {
            pictures.0 += 1;
            if ed.picture(o).is_some() {
                pictures.1 += 1;
            }
        }
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &assets);
        dump(&f, &format!("writer_{name}"));
        for p in 0..l.page_count() {
            dump(
                &ed.render_page(&assets, p),
                &format!("writer_{name}_p{}", p + 1),
            );
        }
        eprintln!(
            "{name}: {} parágrafos, {} folhas (arquivo {}), {} objetos, {} molduras de fora",
            doc.paras.len(),
            l.page_count(),
            m.pages.len(),
            doc.objects.len(),
            extra.frames.len()
        );
    }
    eprintln!("figuras desenhadas: {} de {}", pictures.1, pictures.0);
    assert!(pictures.1 * 10 >= pictures.0 * 9, "{pictures:?}");
}

#[test]
fn saved_by_the_port_reopens_the_same() {
    let Some(data) = data_dir() else { return };
    let assets = Assets::open(&data).unwrap();
    let dir = tempfile::tempdir().unwrap();
    for path in samples(&data).iter().take(5) {
        let mut w = Writer::open(Some(dir.path().to_path_buf()), path, &assets).unwrap();
        // aberto de fora da pasta do usuário: é uma cópia
        assert!(w.editor().file().is_none());
        let title = w.editor().title().unwrap_or("Exemplo").to_string();
        let saved = w.editor_mut().save_as(&title, "", false).unwrap();
        assert!(saved.starts_with(dir.path()));
        let (a, _) = store::load(path).unwrap();
        let (b, _) = store::load(&saved).unwrap();
        assert_eq!(a, b, "{}", path.display());
        // o original não foi tocado
        assert_eq!(
            std::fs::read(path).unwrap().len() as u64,
            std::fs::metadata(path).unwrap().len()
        );
        assert!(w.leave().is_empty());
    }
}

#[test]
#[ignore]
fn probe_objects() {
    let Some(data) = data_dir() else { return };
    let assets = Assets::open(&data).unwrap();
    let name = std::env::var("SAMP").unwrap_or("SAMP18".into());
    let path = if name.contains('/') {
        PathBuf::from(&name)
    } else {
        data.join("SHARED/USERS/MAX").join(format!("{name}.MAX"))
    };
    let w = Writer::open(None, &path, &assets).unwrap();
    let mut f = Frame::new(SCREEN_W, SCREEN_H);
    w.draw(&mut f, &assets);
    dump(&f, "writer_probe");
    let ed = w.editor();
    for o in &ed.doc().objects {
        let c = ed.extra.object(o.object);
        let parts: Vec<String> = c
            .map(|c| {
                c.parts
                    .iter()
                    .map(|p| format!("{}:{}", String::from_utf8_lossy(&p.tag.0), p.data.len()))
                    .collect()
            })
            .unwrap_or_default();
        eprintln!(
            "{:?} {:?} p{} ({},{}) {}x{} orig {}x{} wrap {:?} outline {:?} pic {} parts {:?}",
            o.kind,
            o.object,
            o.page,
            o.x,
            o.y,
            o.width,
            o.height,
            o.orig_width,
            o.orig_height,
            o.wrap,
            o.outline.as_ref().map(|t| (
                t.width,
                t.height,
                t.left.len(),
                t.right.len(),
                t.left.first(),
                t.right.first()
            )),
            ed.picture(o).is_some(),
            parts
        );
    }
    for f in &ed.extra.frames {
        eprintln!("fora: {f:?}");
    }
    let l = ed.layout();
    for line in l.lines_on(0).iter().filter(|l| l.top > 560) {
        let xs: Vec<(i32, i32)> = line.boxes.iter().map(|b| (b.x, b.x + b.w)).collect();
        let gaps: Vec<i32> = xs
            .windows(2)
            .filter(|w| w[1].0 - w[0].1 > 10)
            .map(|w| w[0].1)
            .collect();
        eprintln!(
            "linha y {} h {} x {}..{} gaps {:?} col {}..{}",
            line.top,
            line.height,
            xs.first().map_or(0, |x| x.0),
            xs.last().map_or(0, |x| x.1),
            gaps,
            line.left,
            line.right
        );
    }
    for o in &ed.doc().objects {
        eprintln!(
            "excl {:?}",
            (600..720)
                .step_by(15)
                .map(|y| o.exclusion(y, y + 15, 6, 595))
                .collect::<Vec<_>>()
        );
    }
}

/// Grava em `$CW_EXPORT` documentos feitos no port, para abrir no original.
#[test]
#[ignore]
fn export_for_original() {
    use engine::scene::Key;
    use engine::writer::doc::{CSCH, CharChange, ParaChange, Size, Spacing, TextStyle};
    let Some(data) = data_dir() else { return };
    let Some(out) = std::env::var_os("CW_EXPORT").map(PathBuf::from) else {
        return;
    };
    let assets = Assets::open(&data).unwrap();
    let mut w = Writer::new_blank(Some(out.clone()), &assets);
    let ed = w.editor_mut();
    ed.type_text("Feito no port");
    ed.sel.select_all(&ed.doc().clone());
    ed.apply_char(CharChange::Size(Size::Large));
    ed.apply_char(CharChange::Color(Some(CSCH[5])));
    ed.key(Key::End, true, false);
    ed.key(Key::Enter, false, false);
    ed.apply_char(CharChange::Size(Size::Minimal));
    ed.apply_char(CharChange::Color(None));
    ed.type_text("O gato e a bicicleta foram passear no parque. ");
    ed.type_text("Texto em negrito, ");
    ed.apply_char(CharChange::Style(TextStyle::Bold));
    ed.type_text("negrito");
    ed.apply_char(CharChange::Style(TextStyle::Plain));
    ed.type_text(" e normal de novo.");
    ed.apply_para(ParaChange::Spacing(Spacing::ExtraLines));
    ed.key(Key::Enter, false, false);
    ed.apply_para(ParaChange::Spacing(Spacing::Normal));
    ed.type_text("Um adesivo aqui ao lado.");
    let roll = ed.rolls[0].id;
    ed.add_sticker(roll, 0, 0, 300, 300);
    let p = ed.save_as("Port Teste", "Ana Teste", false).unwrap();
    eprintln!("{}", p.display());
    let path = data.join("SHARED/USERS/MAX/SAMP16.MAX");
    let mut w = Writer::open(Some(out.clone()), &path, &assets).unwrap();
    let p = w
        .editor_mut()
        .save_as("Jornal Port", "Ana Teste", false)
        .unwrap();
    eprintln!("{}", p.display());
}

/// Sons em palavras (`SNDT`): cada elemento marca a palavra que começa no
/// seu cp; abrir e regravar devolve a mesma tabela (com os 4 bytes sem uso).
#[test]
fn word_sounds_land_on_their_words() {
    let Some(data) = data_dir() else { return };
    let dir = data.join("SHARED").join("USERS").join("MAX");
    for (file, want) in [("SAMP6.MAX", 2), ("SAMP14.MAX", 0)] {
        let Ok(bytes) = std::fs::read(dir.join(file)) else {
            continue;
        };
        let m = max::read(&bytes).unwrap();
        let (doc, extra) = from_max(&m);
        let words = doc.word_sounds();
        let text: Vec<String> = words
            .iter()
            .map(|(r, _, s)| format!("{:?}={}", doc.text_range(*r), s.id))
            .collect();
        eprintln!("{file}: {:?} -> {text:?}", m.word_sounds);
        if want > 0 {
            assert_eq!(words.len(), want, "{file}");
        }
        assert_eq!(words.len(), m.word_sounds.len(), "{file}");
        let back = to_max(&doc, &extra, &[], None);
        assert_eq!(back.word_sounds, m.word_sounds, "{file}");
    }
}
