mod support;

use formats::{ne, scrd};

/// (id, tipo, nº de pontos, campos c0..c4), de `docs/formats/scrd.md`.
const TABLE: [(u16, u16, usize, [u16; 5]); 14] = [
    (1, 3, 322, [161, 0, 0, 0, 161]),
    (2, 3, 60, [30, 30, 0, 30, 30]),
    (3, 0, 603, [201, 201, 0, 201, 402]),
    (4, 2, 303, [101, 101, 0, 101, 202]),
    (5, 1, 303, [101, 101, 0, 101, 202]),
    (6, 3, 12, [6, 6, 0, 6, 6]),
    (7, 3, 6, [3, 3, 0, 3, 3]),
    (8, 3, 1503, [501, 1002, 0, 1002, 501]),
    (9, 0, 387, [129, 129, 0, 129, 258]),
    (10, 0, 579, [193, 193, 0, 193, 386]),
    (11, 3, 78, [39, 0, 0, 39, 39]),
    (12, 3, 1088, [544, 0, 0, 544, 544]),
    (13, 3, 100, [50, 0, 0, 50, 50]),
    (14, 3, 628, [314, 0, 0, 314, 314]),
];

fn shapes(file: &str) -> Option<Vec<scrd::Scrd>> {
    let d = support::read(file)?;
    let f = ne::parse(&d).unwrap();
    let mut v: Vec<_> = f
        .of_kind("SCRD")
        .map(|r| (r.id, scrd::read(r.data).unwrap()))
        .collect();
    v.sort_by_key(|(id, _)| *id);
    assert_eq!(
        v.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        (1..=14).collect::<Vec<_>>()
    );
    Some(v.into_iter().map(|(_, s)| s).collect())
}

#[test]
fn fourteen_shapes_match_the_table_and_both_programs() {
    let (Some(artist), Some(writer)) = (shapes("ARTIST.EXE"), shapes("WRITER.EXE")) else {
        return;
    };
    assert_eq!(artist, writer);
    for (s, (id, kind, n, fields)) in artist.iter().zip(TABLE) {
        assert_eq!(
            (s.kind, s.points.len(), s.fields),
            (kind, n, fields),
            "SCRD {id}"
        );
        assert!(
            s.top().len() >= 3 && s.bottom().len() == s.top().len(),
            "SCRD {id}"
        );
    }
    // arco embaixo: cima reta, baixo de 4000 nas pontas a 0 no meio
    let arch = &artist[0];
    assert!(arch.top().iter().all(|p| p.1 == -2000));
    assert_eq!(arch.bottom()[80], (0, 0));
    assert_eq!((arch.width, arch.height), (8000, 2000));
    // só a seta e as de três curvas têm curva do meio própria
    let with_middle: Vec<usize> = (0..14)
        .filter(|&k| artist[k].middle().is_some())
        .map(|k| k + 1)
        .collect();
    assert_eq!(with_middle, [3, 4, 5, 8, 9, 10]);
    // arco-íris: três arcos concêntricos de raio 4000, 3000, 2000
    let rainbow = &artist[8];
    assert_eq!(rainbow.bounds(), (-3999, -4000, 4000, 0));
    assert_eq!(rainbow.middle().unwrap()[0], (-2999, 0));
}
