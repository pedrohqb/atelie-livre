//! Seleciona-sons com os dados reais (pulado sem `ATELIE_DATA`).

use engine::assets::{Assets, ImageRef, Program};
use engine::scenes::sounds::{Sounds, banner_image, load_categories};
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
fn eight_categories_from_both_libraries_in_the_original_order() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let cats = load_categories(&a);
    let names: Vec<&str> = cats.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Grave",
            "Agudo",
            "Dinossauro",
            "AnimalSelvagem",
            "AnimalDeFazenda",
            "Ação",
            "Maluco",
            "Absurdo"
        ]
    );
    // Grave: Sol2 … Sol3, de MAX100.SND, como na lista do original
    let grave: Vec<&str> = cats[0].sounds.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(grave.first(), Some(&"Sol2"));
    assert_eq!(grave.last(), Some(&"Sol3"));
    assert_eq!(grave.len(), 13);
    assert_eq!(cats[5].sounds.len(), 23);
    // todos os sons decodificam
    for c in &cats {
        for (n, s) in &c.sounds {
            assert!(a.sound(s).is_some(), "{} / {n}", c.name);
        }
    }
    let s = Sounds::new(&a);
    assert_eq!(s.pages(), 3);
}

#[test]
fn theater_banner_is_painted_from_its_outline() {
    let Some(d) = data_dir() else { return };
    let a = Assets::open(&d).unwrap();
    let pic = |id| a.image(&ImageRef::Pic(Program::Artist, id));
    let img = banner_image(&pic(0x024e), &pic(0x024f)).unwrap();
    let px = |x: u32, y: u32| {
        let i = 4 * (y * img.width + x) as usize;
        [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2]]
    };
    // céu lilás à esquerda, placa amarela e cortina verde (medidos no original)
    assert_eq!(px(6, 1), [136, 136, 230]);
    assert_eq!(px(30, 25), [255, 238, 76]);
    assert_eq!(px(3, 3), [92, 203, 83]);
}
