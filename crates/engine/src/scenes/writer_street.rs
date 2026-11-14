//! Imaginópolis e rua do Creative Writer (tarefa T3;
//! `cw/report/building.md` §6–7): Imaginópolis `6801` com o carrinho
//! `ATAG 0861` e `302`; rua `6803` com a fachada (bola `080c`), a porta
//! giratória `0811`, o hidrante `0816`, a lata `081b`, as zonas que levam a
//! Imaginópolis e os bichos `0800`–`0805` (`writer_gags::STREET_GAGS`).
//!
//! Padrão: extraia das cenas um `ImaginopolisSpec`/`StreetSpec` com os
//! dados do Fine Artist (`ARTIST_*`, no próprio `imaginopolis.rs`/
//! `street.rs`), ponha aqui os `WRITER_*` e faça `Imaginopolis::for_program`
//! e `Street::for_program` escolherem pelo programa (quem chama já passa o
//! programa). As saídas já passam por `SceneId::in_program`.
