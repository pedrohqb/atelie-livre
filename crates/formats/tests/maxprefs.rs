//! `~MAXUSER.INF` reais (gravados pelo original no Wine). Não há nenhum nos
//! dados do produto; indique arquivos em `ATELIE_USER_INF` (caminhos
//! separados por `:`) para rodar. Sem a variável o teste é pulado.

use formats::{chnk, maxprefs};

#[test]
fn real_user_inf_files() {
    let Some(list) = std::env::var_os("ATELIE_USER_INF") else {
        eprintln!("aviso: ATELIE_USER_INF não definida; teste com dados reais pulado");
        return;
    };
    for path in std::env::split_paths(&list) {
        let data = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let p = maxprefs::read(&data).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            p.wall.len(),
            maxprefs::WALL_SLOTS as usize,
            "{}",
            path.display()
        );
        for d in &p.wall {
            assert!(d.kind.starts_with("kw"), "{d:?}");
            assert!(!d.title.is_empty() && d.file.contains('.'), "{d:?}");
        }
        assert!(!p.fonts.is_empty());
        // gravar e ler de novo não perde nada
        let again = maxprefs::read(&maxprefs::write(&p)).unwrap();
        assert_eq!(again, p, "{}", path.display());
        // com índice: cada chunk regravado é igual byte a byte ao original
        if chnk::is_chnk(&data) {
            let orig = chnk::parse(&data).unwrap();
            let ours_bytes = maxprefs::write(&p);
            let ours = chnk::parse(&ours_bytes).unwrap();
            assert_eq!(ours.chunks.len(), orig.chunks.len());
            for c in &orig.chunks {
                let o = ours
                    .chunks
                    .iter()
                    .find(|x| x.tag == c.tag && x.id == c.id)
                    .unwrap_or_else(|| panic!("{} {}", c.tag, c.id));
                assert_eq!(o.parts[0].data, c.parts[0].data, "{} {}", c.tag, c.id);
            }
        }
        eprintln!(
            "{}: {} documentos, {} fontes ({} distintas), {} tabelas STTB",
            path.display(),
            p.wall.len(),
            p.fonts.len(),
            p.unique_fonts().len(),
            p.string_tables.len()
        );
    }
}
