mod support;

use formats::{atag, chnk, ne, pkbm};

/// Todos os recursos `ATAG` dos dois executáveis são lidos sem erro e
/// obedecem às invariantes observadas.
#[test]
fn every_exe_atag_parses() {
    for (file, expected) in [("ARTIST.EXE", 162), ("WRITER.EXE", 155)] {
        let Some(d) = support::read(file) else { return };
        let exe = ne::parse(&d).unwrap();
        let mut n = 0;
        for res in exe.of_kind("ATAG") {
            let s =
                atag::parse(res.data).unwrap_or_else(|e| panic!("{file} ATAG {:#x}: {e}", res.id));
            assert_eq!(s.source, atag::Source::Resource);
            assert!(!s.cells.is_empty(), "{file} ATAG {:#x}", res.id);
            assert!(s.steps.is_empty());
            assert!(
                s.path_kind == 5 || s.path_kind == 9,
                "{file} ATAG {:#x}",
                res.id
            );
            for c in &s.cells {
                assert_eq!(c.mask, c.image.wrapping_add(3));
                assert_eq!(c.offset, (0, 0));
            }
            // tipo 5: um deslocamento por célula (WRITER 0x934 tem um a mais)
            if s.path_kind == 5 {
                assert!(s.moves.len() >= s.cells.len(), "{file} ATAG {:#x}", res.id);
            }
            n += 1;
        }
        assert_eq!(n, expected, "{file}");
    }
}

/// O pássaro do título (`ARTIST.EXE`, ATAG 0x325): 48 quadros de 100 ms.
#[test]
fn artist_title_bird() {
    let Some(d) = support::read("ARTIST.EXE") else {
        return;
    };
    let exe = ne::parse(&d).unwrap();
    let res = exe.find("ATAG", 0x325).unwrap();
    let s = atag::parse(res.data).unwrap();
    assert_eq!(s.cells.len(), 48);
    assert_eq!(s.cells[0].image, 0x64b0);
    assert_eq!(s.cells[0].duration_ms, 100);
    assert_eq!(s.cells[3].image, 0x64a8);
    assert_eq!(s.cells[3].sound, 0x217);
    assert_eq!(s.origin, (628, 194));
    assert_eq!(
        s.moves[0],
        atag::Move {
            dx: -28,
            dy: -15,
            tween: 0
        }
    );
    let p = s.playback();
    assert_eq!((p[1].x, p[1].y), (-28, -15));
    assert_eq!((p[2].x, p[2].y), (-43, -17));
}

/// Pablo entrando correndo (ATAG 0x3840): um movimento interpolado.
#[test]
fn artist_tweened_move() {
    let Some(d) = support::read("ARTIST.EXE") else {
        return;
    };
    let exe = ne::parse(&d).unwrap();
    let s = atag::parse(exe.find("ATAG", 0x3840).unwrap().data).unwrap();
    assert_eq!(s.flags, 0x10);
    assert_eq!(s.path_kind, 9);
    assert_eq!(s.cells.len(), 1);
    assert_eq!(
        s.moves,
        vec![atag::Move {
            dx: -487,
            dy: -52,
            tween: 20
        }]
    );
}

/// Os 16 adesivos animados de `A-MAG100.ANI`: a cobra (chunk 0) reproduz a
/// sequência gravada do programa original.
#[test]
fn animated_stickers() {
    let Some(d) = support::read("SHARED/STUFF/A-MAG100.ANI") else {
        return;
    };
    let lib = chnk::parse(&d).unwrap();
    let stickers: Vec<_> = lib.chunks.iter().filter(|c| &c.tag.0 == b"ATAG").collect();
    assert_eq!(stickers.len(), 16);
    let mut snake = None;
    for chunk in &stickers {
        let inner = chnk::parse(chunk.parts[0].data).unwrap();
        let part = inner
            .chunks
            .iter()
            .find(|c| &c.tag.0 == b"ATAG")
            .map(|c| c.parts[0].data)
            .unwrap();
        let s = atag::parse(part).unwrap_or_else(|e| panic!("ATAG {}: {e}", chunk.id));
        assert_eq!(s.source, atag::Source::Chunk);
        assert_eq!(s.path_kind, 5);
        assert_eq!(s.steps.len(), s.moves.len());
        // cada passo: posição acumulada, caixa = posição + tamanho da imagem
        let (mut x, mut y) = (0i16, 0i16);
        for (st, mv) in s.steps.iter().zip(&s.moves) {
            assert_eq!((st.x, st.y), (x, y));
            assert_eq!(st.delta, (mv.dx, mv.dy));
            let cell = s.cells[st.cell as usize];
            assert_eq!(cell.mask, cell.image + 1);
            let img = inner
                .chunks
                .iter()
                .find(|c| &c.tag.0 == b"PKBM" && c.id == cell.image)
                .map(|c| pkbm::decode(c.parts[0].data).unwrap())
                .unwrap();
            assert_eq!(st.right - st.x, img.width as i16);
            assert_eq!(st.bottom - st.y, img.height as i16);
            x += mv.dx;
            y += mv.dy;
        }
        if chunk.id == 0 {
            snake = Some(s);
        }
    }
    let s = snake.unwrap();
    assert_eq!(s.cells.len(), 4);
    assert!(s.cells.iter().all(|c| c.duration_ms == 300));
    assert_eq!(s.steps.len(), 28);
    assert_eq!(s.origin, (-215, 253));
    // gravado: 753b em (-109,-33), 7543 em (-53,-41), 754b em (-41,-35),
    // quadro 0 (216×77, centrado no clique: (-108,-38)), 753b em (18,-33)...
    let base = (-108, -38);
    let got: Vec<_> = s.steps[1..8]
        .iter()
        .map(|st| (st.cell, base.0 + st.x, base.1 + st.y))
        .collect();
    assert_eq!(
        got,
        vec![
            (1, -109, -33),
            (2, -53, -41),
            (3, -41, -35),
            (0, 18, -38),
            (1, 18, -33),
            (2, 73, -41),
            (3, 86, -35)
        ]
    );
}
