//! Teste de jogo dos arquivos: tudo o que a Galeria e o Cachorrinho listam
//! precisa abrir.

mod common;

use common::real_assets;
use engine::assets::Program;

/// Os desenhos de exemplo da instalação (`SHARED/USERS/*/*.MAG`), que a
/// Galeria e o Cachorrinho ("Fora de Imaginópolis") oferecem, abrem todos.
#[test]
fn every_listed_sample_opens() {
    let Some(a) = real_assets() else { return };
    let pal = a.palettes(Program::Artist).pal256.clone();
    let samples = engine::scenes::gallery::list_samples(a.data_dir());
    assert!(!samples.is_empty());
    let broken: Vec<String> = samples
        .iter()
        .filter(|p| engine::paint::store::load_picture(p, &pal).is_none())
        .map(|p| p.display().to_string())
        .collect();
    assert!(
        broken.is_empty(),
        "{} de {} exemplos não abrem: {broken:?}",
        broken.len(),
        samples.len()
    );
}

