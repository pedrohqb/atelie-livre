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

/// Desenha o ícone da figura `what_id` em `(x, y)`; sem entrada no mapa, a
/// figura de mesmo id do `ARTIST.PIC`.
pub fn draw(f: &mut crate::Frame, a: &crate::assets::Assets, what_id: u16, x: i32, y: i32) {
    use crate::assets::{ImageRef, Program};
    let pic = |id| ImageRef::Pic(Program::Artist, id);
    match icon(what_id) {
        Some(Icon::Masked(p, m)) if a.has_image(&pic(p)) && a.has_image(&pic(m)) => {
            crate::anim::blit_masked(f, &a.image(&pic(p)), &a.image(&pic(m)), x, y);
        }
        Some(Icon::Auto(p)) => crate::text_balloon::blit_pic(f, a, p, x, y, true),
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
}
