mod support;

use formats::palette::Rgb;
use formats::{ne, splot_res};

fn res(file: &str, kind: &str, id: u16) -> Option<Vec<u8>> {
    let d = support::read(file)?;
    let f = ne::parse(&d).unwrap();
    Some(
        f.find(kind, id)
            .unwrap_or_else(|| panic!("{file} {kind} {id}"))
            .data
            .to_vec(),
    )
}

#[test]
fn tmen_styles_and_shadows() {
    let Some(d) = res("WRITER.EXE", "TMEN", 1) else {
        return;
    };
    let t = splot_res::read_tmen(&d).unwrap();
    assert_eq!(t.items.len(), 9);
    assert_eq!(
        t.labels(),
        [
            "Normal",
            "Negrito",
            "Inclinado",
            "Contorno",
            "MAIÚSCULAS",
            "Vertical",
            "Preencher",
            "Em cima/embaixo",
            "Esquerda/direita"
        ]
    );
    let styles: Vec<u8> = t.items.iter().map(|i| i.style).collect();
    assert_eq!(styles, [0, 1, 2, 8, 0, 0, 0, 0, 0]);
    let s = splot_res::read_tmen(&res("WRITER.EXE", "TMEN", 2).unwrap()).unwrap();
    assert_eq!(s.items.len(), 8);
    assert_eq!(s.labels()[5], "Girar em círculo");
    assert_eq!(s.labels()[7], "Efeito tridimensional");
    // iguais no Fine Artist, que tem ainda o TMEN 4
    for id in [1, 2] {
        let a = splot_res::read_tmen(&res("ARTIST.EXE", "TMEN", id).unwrap()).unwrap();
        let w = splot_res::read_tmen(&res("WRITER.EXE", "TMEN", id).unwrap()).unwrap();
        assert_eq!(a, w);
    }
    let four = splot_res::read_tmen(&res("ARTIST.EXE", "TMEN", 4).unwrap()).unwrap();
    assert_eq!(four.labels(), ["Normal", "Negrito", "Sublinhado"]);
}

#[test]
fn csch_sixteen_colors_and_sets() {
    let Some(d) = res("WRITER.EXE", "CSCH", 1) else {
        return;
    };
    let c = splot_res::read_csch(&d).unwrap();
    assert_eq!(c.colors.len(), 16);
    assert_eq!(
        c.colors[0],
        Rgb {
            r: 0xff,
            g: 0xff,
            b: 0xff
        }
    );
    assert_eq!(
        c.colors[5],
        Rgb {
            r: 0xfa,
            g: 0x00,
            b: 0x39
        }
    );
    assert_eq!(c.colors[15], Rgb { r: 0, g: 0, b: 0 });
    assert_eq!(c.sets.len(), 24);
    assert_eq!(c.max_per_set, 3);
    assert_eq!(&c.sets[..3], &[vec![15], vec![0], vec![15, 0]]);
    assert_eq!(c.sets[17], vec![5, 11]);
    assert_eq!(c.sets[23], vec![2, 15, 5]);
    assert_eq!(
        c,
        splot_res::read_csch(&res("ARTIST.EXE", "CSCH", 1).unwrap()).unwrap()
    );
}

#[test]
fn pats_sshd_smnm_tdrl() {
    let Some(d) = res("WRITER.EXE", "PATS", 1) else {
        return;
    };
    let p = splot_res::read_pats(&d).unwrap();
    assert_eq!(p.len(), 13);
    // listras diagonais (8º padrão)
    assert_eq!(p[7], [0xf0, 0x78, 0x3c, 0x1e, 0x0f, 0x87, 0xc3, 0xe1]);
    assert_eq!(
        p,
        splot_res::read_pats(&res("ARTIST.EXE", "PATS", 1).unwrap()).unwrap()
    );

    let s = splot_res::read_sshd(&res("WRITER.EXE", "SSHD", 1).unwrap()).unwrap();
    assert_eq!((s.count, s.flag, s.raw.len()), (7, 1, 1536));
    let m = s.spin_matrices();
    assert_eq!(m.len(), 16);
    for (k, a) in m.iter().enumerate() {
        let deg = (a[2] as f32)
            .atan2(a[0] as f32)
            .to_degrees()
            .rem_euclid(360.0);
        assert!((deg - 40.0 - 20.0 * k as f32).abs() < 0.5, "{k}: {deg}");
    }

    let m1 = splot_res::read_smnm(&res("ARTIST.EXE", "SMNM", 1).unwrap()).unwrap();
    assert_eq!(m1.frame_size(), (463, 335));
    let m2 = splot_res::read_smnm(&res("WRITER.EXE", "SMNM", 2).unwrap()).unwrap();
    assert_eq!(m2.frame_size(), (418, 245));

    let t = splot_res::read_tdrl(&res("WRITER.EXE", "TDRL", 1).unwrap()).unwrap();
    assert_eq!(t.len(), 13);
    assert!(t.iter().all(|r| r[11] == 0xff));
    assert_eq!(
        t,
        splot_res::read_tdrl(&res("ARTIST.EXE", "TDRL", 1).unwrap()).unwrap()
    );
}
