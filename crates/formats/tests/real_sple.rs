mod support;

use formats::chnk;
use formats::sple::{self, Fill};

/// `(arquivo, texto, fonte, nº de glifos, forma, sombra, padrão, cores)`.
const SAMPLES: [(&str, &str, &str, usize, u8, u8, u8, u8); 6] = [
    ("SAMP7", "Feliz Aniversário!", "Impact", 34, 3, 6, 7, 0x12),
    ("SAMP13", "Tomara Que Você Melhore Logo!", "Lucida Handwriting", 50, 6, 1, 0, 0x14),
    ("SAMP14", "SUA MANCHETE AQUI", "Impact", 30, 7, 6, 0, 0x17),
    ("SAMP15", "SUA MANCHETE AQUI", "Impact", 30, 0, 1, 0, 0),
    ("SAMP16", "Sua Manchete Aqui", "Lucida Blackletter", 15, 0, 0, 0, 0),
    ("SAMP17", "Nosso Melhor Trabalho Em Grupo", "Impact", 26, 0, 0, 0, 0x12),
];

#[test]
fn sample_splots_read_and_write_back_byte_for_byte() {
    for (name, text, font, n, shape, shadow, pattern, colors) in SAMPLES {
        let Some(d) = support::read(&format!("SHARED/USERS/MAX/{name}.MAX")) else {
            return;
        };
        let f = chnk::parse(&d).unwrap();
        let c = f.chunks.iter().find(|c| c.tag.0 == *b"SPLE").unwrap();
        let part = c.parts.iter().find(|p| p.tag.0 == *b"SPLE").unwrap();
        let s = sple::read(part.data).unwrap();
        assert_eq!(formats::cp1252::decode(s.text()), text, "{name}");
        assert_eq!(s.font(), font.as_bytes(), "{name}");
        assert_eq!(s.glyphs.len(), n, "{name}");
        assert_eq!(s.letters.len(), text.chars().count(), "{name}");
        let ch = s.choices();
        assert_eq!(
            (ch.shape, ch.shadow, ch.pattern, ch.color_set),
            (shape, shadow, pattern, colors),
            "{name}"
        );
        // a caixa contém todos os glifos
        let [t, l, b, r] = s.bounds();
        for g in &s.glyphs {
            assert!(g.rect[0] >= t && g.rect[1] >= l && g.rect[2] <= b && g.rect[3] <= r);
        }
        assert_eq!(s.shadowed(), ch.shadow != 0, "{name}");
        assert_eq!(sple::write(&s), part.data, "{name}");
    }
}

#[test]
fn sample_glyph_kinds() {
    let Some(d) = support::read("SHARED/USERS/MAX/SAMP7.MAX") else {
        return;
    };
    let f = chnk::parse(&d).unwrap();
    let c = f.chunks.iter().find(|c| c.tag.0 == *b"SPLE").unwrap();
    let s = sple::read(c.parts[0].data).unwrap();
    // 17 sombras cheias e 17 letras com padrão, roxo e verde alternados
    assert!(s.glyphs[..17].iter().all(|g| g.fill == Fill::Solid));
    let Fill::Pattern { back, .. } = &s.glyphs[17].fill else {
        panic!("sem padrão");
    };
    assert_eq!(sple::rgb8(s.glyphs[17].color), [0x93, 0x2c, 0xa3]);
    assert_eq!(sple::rgb8(*back), [0x47, 0x8c, 0x50]);
    assert_eq!(s.shape_record().len(), 2430);
}
