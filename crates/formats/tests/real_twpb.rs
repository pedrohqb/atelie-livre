mod support;

use formats::{chnk, twpb, wmf};

/// `(arquivo, etiqueta do chunk, TWPB, META)`.
type Found = (String, String, twpb::Twpb, Option<Vec<u8>>);

/// Todos os `TWPB` dos documentos de exemplo.
fn all() -> Option<Vec<Found>> {
    let dir = support::data_dir()?.join("SHARED/USERS/MAX");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("max")))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for p in files {
        let d = std::fs::read(&p).unwrap();
        let f = chnk::parse(&d).unwrap();
        for c in &f.chunks {
            let Some(t) = c.parts.iter().find(|p| &p.tag.0 == b"TWPB") else {
                continue;
            };
            let meta = c
                .parts
                .iter()
                .find(|p| &p.tag.0 == b"META")
                .map(|p| p.data.to_vec());
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            out.push((
                name,
                format!("{} {}", c.tag, c.id),
                twpb::read(t.data).unwrap(),
                meta,
            ));
        }
    }
    Some(out)
}

#[test]
fn sixty_four_outlines_are_coherent() {
    let Some(v) = all() else { return };
    assert_eq!(v.len(), 64);
    for (file, chunk, t, _) in &v {
        assert!(
            matches!(t.kind, 4 | 8 | 16),
            "{file} {chunk}: tipo {}",
            t.kind
        );
        for side in [&t.left, &t.right] {
            assert!(side.len() >= 2, "{file} {chunk}");
            for &(x, y) in side.iter() {
                assert!((0..=t.width as i16).contains(&x), "{file} {chunk}: x {x}");
                assert!((0..=t.height as i16).contains(&y), "{file} {chunk}: y {y}");
            }
        }
        assert_eq!(twpb::read(&twpb::write(t)).unwrap(), *t);
    }
}

#[test]
fn outline_from_the_picture_is_within_two_pixels() {
    let Some(v) = all() else { return };
    let (mut rows, mut close, mut pics) = (0usize, 0usize, 0usize);
    for (file, chunk, t, meta) in &v {
        let Some(meta) = meta else { continue };
        let img = wmf::meta_dib(meta).unwrap();
        let (w, h) = (t.width as u32, t.height as u32);
        // a figura pode estar redimensionada na página: amostra a tinta (não branco) no tamanho do TWPB
        let mask: Vec<bool> = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let sx = (x * img.width / w).min(img.width - 1);
                let sy = (y * img.height / h).min(img.height - 1);
                let p = &img.rgba[((sy * img.width + sx) * 4) as usize..][..4];
                p[3] > 0 && !(p[0] == 255 && p[1] == 255 && p[2] == 255)
            })
            .collect();
        let calc = twpb::from_mask(&mask, w, h);
        pics += 1;
        let (mut r, mut c) = (0, 0);
        for y in 0..h {
            let y = y as f32 + 0.5;
            if let (Some(a), Some(b)) = (t.margins(y), calc.margins(y)) {
                r += 2;
                c += ((a.0 - b.0).abs() <= 2.0) as usize + ((a.1 - b.1).abs() <= 2.0) as usize;
            }
        }
        // e cada figura fica na mesma ordem de pontos que a gravada
        let (nt, nc) = (
            t.left.len() + t.right.len(),
            calc.left.len() + calc.right.len(),
        );
        assert!(
            nc * 2 >= nt && nc <= nt * 2,
            "{file} {chunk}: {nc} pontos, gravados {nt}"
        );
        assert!(c * 100 >= r * 70, "{file} {chunk}: {c}/{r}");
        rows += r;
        close += c;
    }
    assert_eq!(pics, 58);
    assert!(close * 100 >= rows * 95, "{close}/{rows}");
}
