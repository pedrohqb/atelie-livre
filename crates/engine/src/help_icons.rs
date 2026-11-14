//! Ícones embutidos no texto dos balões de ajuda: as figuras `1000`–`10ff`
//! do `WHAT` são uma numeração própria do `ARTIST.EXE`, não ids do
//! `ARTIST.PIC`. Pelos registros e pelas capturas do original: `1000`–`1005`
//! e `1036`–`1039` são os ícones da barra de ferramentas (pincel, lata,
//! super-ferramentas, palavra, transformadores, imprimir; aspirador,
//! adesivos, sons, ovo), `1049`–`1052` os botões da faixa do Aspirador e
//! `1071`/`1072` os dois botões da faixa do Imprimir. Ids sem entrada aqui
//! continuam lidos direto do `ARTIST.PIC`.

/// Como desenhar o ícone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Figura inteira (branco opaco).
    Opaque(u16),
    /// Figura com a máscara indicada.
    Masked(u16, u16),
    /// Figura com a máscara duas imagens adiante, se houver (`blit_pic`).
    Auto(u16),
}

impl Icon {
    pub fn pic(self) -> u16 {
        match self {
            Icon::Opaque(p) | Icon::Masked(p, _) | Icon::Auto(p) => p,
        }
    }
}

/// Ícone do `ARTIST.PIC` para a figura `what_id` de um balão.
pub fn icon(what_id: u16) -> Option<Icon> {
    use Icon::*;
    Some(match what_id {
        // barra de ferramentas: (normal, máscara), como em `scenes::paint::TOOLS`
        0x1000 => Masked(0x0495, 0x1001),
        0x1001 => Masked(0x1003, 0x1005),
        0x1002 => Masked(0x1007, 0x1009),
        0x1003 => Masked(0x1031, 0x103b),
        0x1004 => Masked(0x1045, 0x104b),
        0x1005 => Masked(0x104d, 0x104f),
        0x1036 => Masked(0x100b, 0x100d),
        0x1037 => Masked(0x101b, 0x1023),
        0x1038 => Masked(0x1029, 0x102b),
        0x1039 => Masked(0x103d, 0x103f),
        // faixa do Aspirador, na ordem dos botões
        0x1049 => Opaque(0x1557),
        0x104a => Opaque(0x1559),
        0x104b => Opaque(0x155b),
        0x104c => Opaque(0x155c),
        0x104d => Opaque(0x155f),
        0x1050 => Opaque(0x1561),
        0x1051 => Opaque(0x1567),
        0x1052 => Opaque(0x1569),
        // faixa do Imprimir: depressa e fazer escolhas
        0x1071 => Auto(0x02e7),
        0x1072 => Auto(0x02f3),
        _ => return None,
    })
}

/// Ícone do `WRITER.PIC` para a figura `what_id` de um balão do Creative
/// Writer. Lá as figuras embutidas são uma numeração própria do `WRITER.EXE`
/// (`200` Palavra, `211` Seleciona-letras, `802` Dimensiona-palavras,
/// `83d` seta ▲…), identificada pelo texto ao lado de cada uma nos blocos
/// `0fa0`–`0faa` (ver `docs/telas/creative-writer.md`). Sem entrada, a
/// figura não é desenhada (o id do `WHAT` não é um id do `WRITER.PIC`).
pub fn writer_icon(what_id: u16) -> Option<Icon> {
    use Icon::*;
    Some(match what_id {
        // barra de ferramentas (figura inativa e máscara)
        0x200 => Masked(0x020d, 0x020f),
        0x225 => Masked(0x01ff, 0x0201),
        0x226 => Masked(0x0205, 0x0207),
        0x800 => Masked(0x0495, 0x0801),
        0x841 => Masked(0x091f, 0x0921),
        0x227 => Masked(0x0211, 0x0213),
        0x853 | 0x812 => Masked(0x08e7, 0x08e9),
        0x22f => Masked(0x0219, 0x0223),
        0x801 => Masked(0x0803, 0x0805),
        0x202 => Masked(0x0225, 0x0227),
        // faixa da Palavra
        0x211 => Masked(0x0275, 0x0277),
        0x802 => Opaque(0x0811),
        0x213 => Opaque(0x027b),
        0x212 => Opaque(0x0279),
        0x842 => Opaque(0x0bc3),
        0x805 => Masked(0x080b, 0x080d),
        0x806 => Masked(0x0831, 0x0833),
        0x20e => Masked(0x0269, 0x026b),
        0x22b => Masked(0x0229, 0x022b),
        0x807 => Opaque(0x0863),
        0x808 => Opaque(0x080f),
        0x809 => Opaque(0x087d),
        0x803 => Opaque(0x0815),
        0x844 => Opaque(0x08d5),
        0x804 => Opaque(0x0817),
        // Re-movedores
        0x818 => Masked(0x0839, 0x083b),
        0x819 => Masked(0x083d, 0x083f),
        0x81a => Masked(0x0841, 0x0843),
        0x81c => Masked(0x0849, 0x084b),
        0x81e => Masked(0x0851, 0x0853),
        0x81b => Masked(0x0845, 0x0847),
        // Preparação da página
        0x80b => Masked(0x0821, 0x0823),
        0x80c => Masked(0x0953, 0x0955),
        0x811 => Opaque(0x086b),
        0x814 => Opaque(0x08c3),
        0x810 => Opaque(0x0869),
        0x813 => Opaque(0x0875),
        // Transformadores
        0x843 => Masked(0x0923, 0x0929),
        0x82e => Masked(0x0857, 0x0859),
        0x82f => Masked(0x085b, 0x085d),
        0x830 => Masked(0x085f, 0x0861),
        0x82c => Masked(0x0807, 0x0809),
        0x82b => Opaque(0x0865),
        0x82d => Masked(0x08c7, 0x08cb),
        // Visor de páginas
        0x836 => Opaque(0x08a9),
        0x837 => Opaque(0x08ad),
        0x838 => Masked(0x0891, 0x0893),
        0x839 => Masked(0x0895, 0x0897),
        0x832 => Masked(0x0899, 0x089b),
        0x833 => Opaque(0x089d),
        0x834 => Opaque(0x08a1),
        0x835 => Opaque(0x08a5),
        // Adesivos: rolo, setas, Cachorrinho
        0x80a => Masked(0x0249, 0x024b),
        0x851 => Masked(0x03b3, 0x03b5),
        0x20c => Masked(0x025d, 0x0263),
        // Imprimir: depressa e fazer escolhas
        0x83a => Masked(0x02ef, 0x02f1),
        0x83b => Masked(0x02f3, 0x02f5),
        // barra de baixo: setas e "todas as páginas"
        0x83d => Masked(0x02db, 0x02dd),
        0x83e => Masked(0x02df, 0x02e1),
        0x83f => Masked(0x0819, 0x081b),
        0x840 => Masked(0x0871, 0x0873),
        0x85a => Masked(0x0c37, 0x0c39),
        // menu do McZee: pasta, McZee (Bolsa Mágica), Varinha, Caderno
        0x223 => Masked(0x044f, 0x0451),
        0x229 => Masked(0x0235, 0x0237),
        0x228 => Masked(0x0457, 0x0459),
        0x846 => Masked(0x0c01, 0x0c03),
        _ => return None,
    })
}

/// Ícone da figura `what_id` dos balões do programa `p`.
pub fn icon_for(p: crate::assets::Program, what_id: u16) -> Option<Icon> {
    match p {
        crate::assets::Program::Writer => writer_icon(what_id),
        _ => icon(what_id),
    }
}

/// A figura `what_id` de um balão do programa `p` é um ícone embutido no
/// texto (e não uma ligação, opção ou caixa de texto)?
pub fn embedded(p: crate::assets::Program, what_id: u16) -> bool {
    match p {
        crate::assets::Program::Writer => writer_icon(what_id).is_some(),
        _ => (0x1000..0x2000).contains(&what_id),
    }
}

/// Desenha o ícone da figura `what_id` em `(x, y)`; sem entrada no mapa, a
/// figura de mesmo id do `ARTIST.PIC`.
pub fn draw(f: &mut crate::Frame, a: &crate::assets::Assets, what_id: u16, x: i32, y: i32) {
    draw_for(f, a, crate::assets::Program::Artist, what_id, x, y);
}

/// Como [`draw`], para os balões do programa `p` (no Writer, sem entrada no
/// mapa não desenha nada).
pub fn draw_for(
    f: &mut crate::Frame,
    a: &crate::assets::Assets,
    p: crate::assets::Program,
    what_id: u16,
    x: i32,
    y: i32,
) {
    use crate::assets::{ImageRef, Program};
    let pic = |id| ImageRef::Pic(p, id);
    let mapped = icon_for(p, what_id);
    if mapped.is_none() && p == Program::Writer {
        return;
    }
    match mapped {
        Some(Icon::Masked(p, m)) if a.has_image(&pic(p)) && a.has_image(&pic(m)) => {
            crate::anim::blit_masked(f, &a.image(&pic(p)), &a.image(&pic(m)), x, y);
        }
        Some(Icon::Auto(id)) => crate::text_balloon::blit_pic_in(f, a, p, id, x, y, true),
        Some(i) => {
            if a.has_image(&pic(i.pic())) {
                f.blit(&a.image(&pic(i.pic())), x, y);
            }
        }
        None => {
            if a.has_image(&pic(what_id)) {
                f.blit(&a.image(&pic(what_id)), x, y);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_icons_map_to_pic_ids() {
        assert_eq!(icon(0x1039), Some(Icon::Masked(0x103d, 0x103f)));
        assert_eq!(icon(0x1051).map(Icon::pic), Some(0x1567));
        assert_eq!(icon(0x1072).map(Icon::pic), Some(0x02f3));
        assert_eq!(icon(0x10ab), None);
    }

    #[test]
    fn writer_icons_use_the_writer_numbering() {
        use crate::assets::Program;
        assert_eq!(writer_icon(0x200), Some(Icon::Masked(0x020d, 0x020f)));
        assert_eq!(writer_icon(0x805).map(Icon::pic), Some(0x080b));
        assert!(embedded(Program::Writer, 0x83d));
        assert!(!embedded(Program::Writer, 0x21b));
        assert!(!embedded(Program::Writer, 0x1000));
        assert!(embedded(Program::Artist, 0x1000));
        assert_eq!(icon_for(Program::Artist, 0x1039), icon(0x1039));
        // máscaras sempre duas figuras adiante (cor, preto e branco, máscara)
        for id in 0x200..0x900 {
            if let Some(Icon::Masked(p, m)) = writer_icon(id) {
                assert!(m >= p + 2, "{id:x}");
            }
        }
    }
}
