//! PICT e MacPaint gerados por outro programa (ver `tests/data/README.md`).

use formats::{macpaint, pict};

fn data(name: &str) -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn rgb(p: &pict::Picture, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * p.image.width + x) * 4) as usize;
    [p.image.rgba[i], p.image.rgba[i + 1], p.image.rgba[i + 2]]
}

#[test]
fn imagemagick_direct_pixels() {
    let p = pict::read(&data("im-direct-70x6.pict")).unwrap();
    assert_eq!(p.version, 2);
    assert_eq!((p.image.width, p.image.height), (70, 6));
    assert_eq!(p.bitmaps, 1);
    assert!(p.skipped.is_empty());
    for y in 0..6 {
        for x in 0..70 {
            let want = if (20..40).contains(&x) {
                [255, 255, 255]
            } else {
                [
                    ((x * 3) % 256) as u8,
                    (y * 40) as u8,
                    (((x + y) * 2) % 256) as u8,
                ]
            };
            assert_eq!(rgb(&p, x, y), want, "({x}, {y})");
        }
    }
}

#[test]
fn imagemagick_indexed_pixels() {
    const PAL: [[u8; 3]; 6] = [
        [255, 255, 255],
        [0, 0, 0],
        [255, 0, 0],
        [0, 128, 0],
        [0, 0, 255],
        [250, 200, 10],
    ];
    let p = pict::read(&data("im-indexed-300x4.pict")).unwrap();
    assert_eq!((p.image.width, p.image.height), (300, 4));
    for y in 0..4 {
        for x in 0..300 {
            let i = if (100..200).contains(&x) {
                2
            } else {
                ((x / 7 + y) % 6) as usize
            };
            assert_eq!(rgb(&p, x, y), PAL[i], "({x}, {y})");
        }
    }
}

#[test]
fn imagemagick_small_pictures() {
    for name in ["im-argb-7x5.pict", "im-indexed-7x5.pict"] {
        let p = pict::read(&data(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((p.image.width, p.image.height), (7, 5), "{name}");
        for y in 0..5 {
            for x in 0..7 {
                let want = match (x, y) {
                    (1, 1) => [255, 0, 0],
                    (5, 3) => [0, 0, 255],
                    _ => [255, 255, 255],
                };
                assert_eq!(rgb(&p, x, y), want, "{name} ({x}, {y})");
            }
        }
    }
}

#[test]
fn macpaint_page() {
    let d = data("check.mac");
    assert!(!pict::is_pict(&d));
    assert!(macpaint::is_macpaint(&d));
    let m = macpaint::read(&d).unwrap();
    assert_eq!(m.version, 2);
    for (k, p) in m.patterns.iter().enumerate() {
        for (j, &b) in p.iter().enumerate() {
            assert_eq!(b, ((k * 7 + j * 13) & 0xff) as u8);
        }
    }
    for y in 0..macpaint::HEIGHT {
        for x in 0..macpaint::WIDTH {
            let want = if x < 64 && y < 64 {
                (x / 8 + y / 8) % 2 == 1
            } else {
                x == y || y == 719
            };
            assert_eq!(m.pixel(x, y), want, "({x}, {y})");
        }
    }
    assert_eq!(m.ink_bounds(), Some((0, 0, 576, 720)));
}
