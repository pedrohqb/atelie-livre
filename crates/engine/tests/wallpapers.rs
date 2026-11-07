//! Papéis de parede da lata de tinta (`W-MAG100.CHK`), com os dados reais.

use engine::assets::{Assets, Program};
use engine::paint::bucket_fx::{TILE, load_wallpapers};
use std::path::PathBuf;

fn data_dir() -> Option<PathBuf> {
    match std::env::var_os("ATELIE_DATA") {
        Some(d) => Some(PathBuf::from(d)),
        None => {
            eprintln!("aviso: ATELIE_DATA não definida; teste com dados reais pulado");
            None
        }
    }
}

#[test]
fn wallpapers_load_in_roll_order() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let pal = a.palettes(Program::Artist).pal256.clone();
    let wps = load_wallpapers(&a, &pal);
    // 13 que se mexem (APAT) e 22 parados
    assert_eq!(wps.len(), 35);
    let names: Vec<&str> = wps.iter().take(4).map(|w| w.name.as_str()).collect();
    assert_eq!(
        names,
        ["trees & telephone poles", "clouds", "snowy worms", "eyes"]
    );
    assert_eq!(wps.iter().filter(|w| w.animated()).count(), 13);
    assert_eq!(wps[10].frames.len(), 4, "balões");
    assert_eq!(wps[12].frames.len(), 3, "urso dançando");
    for w in &wps {
        assert!(
            w.frames.iter().all(|f| f.len() == TILE * TILE),
            "{}",
            w.name
        );
    }
    // árvore: céu lilás (136,136,230) = índice 7, como na captura do original
    let tree = &wps[0].frames[0];
    assert!(tree.contains(&7) && tree.contains(&11) && tree.contains(&12));
    // segundo quadro da árvore: o primeiro deslocado 16 px para a direita
    let (f0, f1) = (&wps[0].frames[0], &wps[0].frames[1]);
    let shifted = (0..TILE * TILE).all(|i| {
        let (x, y) = (i % TILE, i / TILE);
        f1[y * TILE + (x + 16) % TILE] == f0[i]
    });
    assert!(shifted);
}
