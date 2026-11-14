//! Figuras embutidas no texto dos balões (código `05` do `WHAT`): o id da
//! figura (`0x2xx` e `0x1000`–`0x10b2`) não é um id do `ARTIST.PIC`, mas o
//! índice de uma tabela do `ARTIST.EXE` (segmento de dados, uma entrada de 3
//! palavras `a, b, c` por figura: `0x200 + k` e `0x1000 + k`).
//!
//! Regra, conferida nas capturas do original (pincel, ovo, aspirador, gema,
//! chave, bichos, cabeçalho "EPA!", bolinha das opções):
//!
//! - a imagem é a do maior id do `ARTIST.PIC` que não passa de `a − 1` (o
//!   `.PIC` guarda grupos colorida/branca/máscara e o `a` é o da branca; o
//!   pincel, `a = 0x1000`, cai na `0x495`);
//! - se `b ≠ 0` (grupo), a máscara é `a + 1` quando tem o mesmo tamanho;
//!   sem ela, o branco é transparente; com `b = 0` (figura solta, como os
//!   botões do Aspirador e as formas), a figura é opaca;
//! - `a = 0` remete à tabela dos botões da barra (16 bytes por entrada,
//!   índice `b`), cuja primeira palavra é o `a` (Aspirador, Seleciona-adesivos,
//!   Seleciona-sons, Ovo, Bolsa Mágica, lente, bolinhas das opções).
//!
//! O mapa abaixo traz todas as figuras usadas nos 34 blocos `WHAT` do
//! `ARTIST.EXE` (157 ids); fora dele, a figura de mesmo id do `.PIC`.

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};

/// Como desenhar o ícone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Figura inteira (branco opaco).
    Opaque(u16),
    /// Figura com a máscara indicada (preto = opaco).
    Masked(u16, u16),
    /// Figura com o branco transparente.
    Keyed(u16),
}

impl Icon {
    pub fn pic(self) -> u16 {
        match self {
            Icon::Opaque(p) | Icon::Masked(p, _) | Icon::Keyed(p) => p,
        }
    }
}

/// Ícone do `ARTIST.PIC` para a figura `what_id` de um balão.
pub fn icon(what_id: u16) -> Option<Icon> {
    use Icon::*;
    Some(match what_id {
        0x0201 => Masked(0x02d3, 0x02d5),
        0x0209 => Masked(0x0251, 0x0257),
        0x020a => Masked(0x0259, 0x025b),
        0x020c => Masked(0x025d, 0x0263),
        0x020d => Masked(0x0265, 0x0267),
        0x021b => Masked(0x03f7, 0x03f9),
        0x021c => Masked(0x03fb, 0x03fd),
        0x021d => Masked(0x03ff, 0x0403),
        0x021e => Masked(0x0405, 0x0407),
        0x0222 => Masked(0x041b, 0x044d),
        0x0223 => Masked(0x044f, 0x0451),
        0x0227 => Masked(0x103d, 0x103f),
        0x0228 => Masked(0x0457, 0x0459),
        0x0229 => Masked(0x1051, 0x1053),
        0x022c => Masked(0x03d3, 0x03e9),
        0x022d => Masked(0x03f3, 0x03f5),
        0x022e => Masked(0x0409, 0x040b),
        0x0230 => Masked(0x02a9, 0x02d1),
        0x0237 => Keyed(0x045b),
        0x0238 => Keyed(0x045d),
        0x0239 => Keyed(0x045f),
        0x023a => Keyed(0x0461),
        0x023b => Keyed(0x0463),
        0x023c => Keyed(0x0465),
        0x023d => Keyed(0x0467),
        0x023e => Keyed(0x0469),
        0x023f => Keyed(0x046b),
        0x0240 => Keyed(0x046d),
        0x0241 => Keyed(0x046f),
        0x0242 => Keyed(0x0471),
        0x0247 => Masked(0x03b3, 0x03b5),
        0x024a => Masked(0x03bf, 0x03c1),
        0x1000 => Masked(0x0495, 0x1001),
        0x1001 => Masked(0x1003, 0x1005),
        0x1002 => Masked(0x1007, 0x1009),
        0x1003 => Masked(0x1031, 0x103b),
        0x1004 => Masked(0x1045, 0x104b),
        0x1005 => Masked(0x104d, 0x104f),
        0x1006 => Masked(0x123f, 0x13fd),
        0x1007 => Keyed(0x13ff),
        0x100d => Masked(0x141f, 0x1421),
        0x100e => Opaque(0x1477),
        0x100f => Opaque(0x1479),
        0x1010 => Opaque(0x147d),
        0x1011 => Opaque(0x1481),
        0x1012 => Opaque(0x1485),
        0x1013 => Opaque(0x1489),
        0x1014 => Opaque(0x148d),
        0x1015 => Opaque(0x149b),
        0x1016 => Opaque(0x149d),
        0x1017 => Opaque(0x14a1),
        0x1018 => Opaque(0x14a5),
        0x101b => Opaque(0x14b1),
        0x101c => Opaque(0x14b5),
        0x101d => Opaque(0x14b9),
        0x101e => Opaque(0x14bd),
        0x101f => Opaque(0x14c1),
        0x1020 => Opaque(0x14c5),
        0x1021 => Opaque(0x14c9),
        0x1022 => Opaque(0x14cd),
        0x1023 => Opaque(0x14d1),
        0x1036 => Masked(0x100b, 0x100d),
        0x1037 => Masked(0x101b, 0x1023),
        0x1038 => Masked(0x1029, 0x102b),
        0x1039 => Masked(0x103d, 0x103f),
        0x103a => Masked(0x1051, 0x1053),
        0x103c => Masked(0x1403, 0x1405),
        0x1045 => Masked(0x14f3, 0x1519),
        0x1046 => Masked(0x151b, 0x151d),
        0x1047 => Masked(0x02db, 0x02dd),
        0x1048 => Masked(0x02df, 0x02e1),
        0x1049 => Opaque(0x1557),
        0x104a => Opaque(0x1559),
        0x104b => Opaque(0x155b),
        0x104c => Opaque(0x155c),
        0x104d => Opaque(0x155f),
        0x1050 => Opaque(0x1561),
        0x1051 => Opaque(0x1567),
        0x1052 => Opaque(0x1569),
        0x1053 => Opaque(0x156b),
        0x1054 => Opaque(0x1591),
        0x1055 => Opaque(0x1593),
        0x1056 => Opaque(0x1595),
        0x1057 => Opaque(0x1597),
        0x105f => Opaque(0x17b5),
        0x1060 => Opaque(0x17cf),
        0x1061 => Opaque(0x17d1),
        0x1062 => Opaque(0x17d3),
        0x1063 => Opaque(0x17d5),
        0x1064 => Opaque(0x17d7),
        0x1065 => Opaque(0x17d9),
        0x1066 => Opaque(0x17db),
        0x1067 => Opaque(0x17dd),
        0x1068 => Opaque(0x17df),
        0x1069 => Opaque(0x17e1),
        0x106a => Opaque(0x17e3),
        0x1070 => Masked(0x182d, 0x1839),
        0x1071 => Masked(0x02e7, 0x02f1),
        0x1072 => Masked(0x02f3, 0x02f5),
        0x1073 => Masked(0x10d7, 0x10d9),
        0x1076 => Masked(0x1491, 0x1495),
        0x1077 => Opaque(0x14d5),
        0x1078 => Opaque(0x14d7),
        0x1079 => Opaque(0x14d9),
        0x107a => Opaque(0x1599),
        0x107b => Opaque(0x159b),
        0x107c => Opaque(0x159d),
        0x107d => Opaque(0x159f),
        0x107e => Opaque(0x15a1),
        0x1080 => Opaque(0x17e5),
        0x1081 => Opaque(0x17e7),
        0x1082 => Opaque(0x17e9),
        0x1083 => Opaque(0x17eb),
        0x1084 => Opaque(0x17ed),
        0x1085 => Opaque(0x17ef),
        0x1086 => Opaque(0x17f1),
        0x1087 => Opaque(0x17f3),
        0x1088 => Opaque(0x17f5),
        0x1089 => Opaque(0x17f7),
        0x108a => Opaque(0x17f9),
        0x108b => Opaque(0x17fb),
        0x108c => Opaque(0x17fd),
        0x108d => Opaque(0x17fe),
        0x108e => Opaque(0x1800),
        0x108f => Opaque(0x1802),
        0x1090 => Opaque(0x1804),
        0x1091 => Opaque(0x1806),
        0x1092 => Opaque(0x1808),
        0x1093 => Opaque(0x180a),
        0x1094 => Opaque(0x180c),
        0x1095 => Opaque(0x180e),
        0x1096 => Opaque(0x1810),
        0x1097 => Opaque(0x1812),
        0x109b => Masked(0x107f, 0x1081),
        0x109c => Masked(0x1071, 0x107d),
        0x109d => Opaque(0x1087),
        0x109e => Opaque(0x108b),
        0x109f => Opaque(0x108f),
        0x10a0 => Opaque(0x1093),
        0x10a1 => Opaque(0x1097),
        0x10a2 => Masked(0x1083, 0x1085),
        0x10a3 => Opaque(0x1814),
        0x10a4 => Opaque(0x1817),
        0x10a5 => Opaque(0x1819),
        0x10a6 => Opaque(0x181b),
        0x10a7 => Opaque(0x181d),
        0x10a8 => Opaque(0x181f),
        0x10a9 => Opaque(0x1821),
        0x10aa => Opaque(0x1823),
        0x10ab => Opaque(0x1825),
        0x10ac => Opaque(0x1827),
        0x10ad => Opaque(0x1829),
        0x10ae => Opaque(0x182b),
        0x10af => Masked(0x10e3, 0x10e5),
        0x10b0 => Masked(0x10af, 0x10cd),
        0x10b1 => Masked(0x10cf, 0x10d1),
        0x10b2 => Masked(0x3a13, 0x3a15),
        _ => return None,
    })
}

/// Ícone do `WRITER.PIC` para a figura `what_id` de um balão do Creative
/// Writer. O `WRITER.EXE` tem a mesma tabela `0x200 + k` do `ARTIST.EXE`
/// (as ligações, opções e bichos — ver [`shared`] — têm o mesmo id e a mesma
/// imagem nos dois `.PIC`) e uma própria, `0x800 + k`, com a mesma regra; as
/// de `a = 0` apontam para a tabela dos botões do Writer (`200` Palavra,
/// `225` Seleciona-adesivos, `83d` seta ▲…) e foram identificadas pelo texto
/// ao lado de cada uma nos blocos `0fa0`–`0faa` (ver
/// `docs/telas/creative-writer.md`). Sem entrada (nem em [`shared`]), a
/// figura não é desenhada.
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
        0x853 => Masked(0x08e7, 0x08e9),
        // Re-movedores como ficam quando escolhidos
        0x812 => Masked(0x08eb, 0x08ed),
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
        // Preparação da página (rolo de bordas)
        0x80b => Masked(0x0821, 0x0823),
        0x80c => Masked(0x0825, 0x0827),
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
        // Corretor: lápis do Iluminador
        0x822 => Masked(0x0295, 0x0297),
        // Visor de páginas (e o bichinho da régua)
        0x836 => Opaque(0x08a9),
        0x837 => Opaque(0x08ad),
        0x838 => Masked(0x0891, 0x0893),
        0x839 => Masked(0x0895, 0x0897),
        0x832 => Masked(0x0899, 0x089b),
        0x833 => Opaque(0x089d),
        0x834 => Opaque(0x08a1),
        0x835 => Opaque(0x08a5),
        0x85b => Masked(0x092b, 0x092d),
        // Adesivos e Sons: Cavalete, setas dos rolos, Cachorrinho, volume,
        // Gravador
        0x80a => Masked(0x0881, 0x0883),
        0x851 => Masked(0x08d9, 0x08dd),
        0x20c => Masked(0x025d, 0x0263),
        0x201 => Masked(0x02d3, 0x02d5),
        0x209 => Masked(0x0255, 0x0257),
        0x20a => Masked(0x0259, 0x025b),
        0x20d => Masked(0x0265, 0x0267),
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
        // cabeçalho "EPA!" do Rex (no ARTIST.PIC é a 0x2a9)
        0x230 => Masked(0x02cf, 0x02d1),
        // mão do Caderno de Idéias e figuras grandes das dicas (Máquina de
        // Idéias, Quadro de Figuras, pasta)
        0x85e => Keyed(0x0c49),
        0x85f => Masked(0x0c3b, 0x0c3d),
        0x860 => Masked(0x0c3f, 0x0c41),
        0x86e => Masked(0x0c61, 0x0c63),
        _ => return None,
    })
}

/// Figuras comuns aos dois programas: ligações (gema, chave, Max, Maggie,
/// "Precisa de ajuda?"), opções e bichos do cadastro — mesma entrada nas
/// duas tabelas e mesma imagem nos dois `.PIC`.
fn shared(what_id: u16) -> bool {
    matches!(what_id, 0x21b..=0x21e | 0x222 | 0x22c..=0x22e | 0x237..=0x242)
}

/// Ícone da figura `what_id` dos balões do programa `p`.
pub fn icon_for(p: Program, what_id: u16) -> Option<Icon> {
    match p {
        Program::Writer => {
            writer_icon(what_id).or_else(|| if shared(what_id) { icon(what_id) } else { None })
        }
        _ => icon(what_id),
    }
}

fn pic(p: Program, id: u16) -> ImageRef {
    ImageRef::Pic(p, id)
}

/// A figura que se desenha para `what_id`: no Fine Artist, sem entrada no
/// mapa, a de mesmo id do `.PIC`; no Writer, nenhuma.
fn shown(p: Program, what_id: u16) -> Option<Icon> {
    match (icon_for(p, what_id), p) {
        (Some(i), _) => Some(i),
        (None, Program::Writer) => None,
        (None, _) => Some(Icon::Opaque(what_id)),
    }
}

/// Tamanho da figura `what_id` dos balões do programa `p` (`None` se o
/// `.PIC` não a tem ou, no Writer, se ela não tem entrada no mapa).
pub fn size(a: &Assets, p: Program, what_id: u16) -> Option<(i32, i32)> {
    let id = shown(p, what_id)?.pic();
    a.has_image(&pic(p, id)).then(|| {
        let img = a.image(&pic(p, id));
        (img.width as i32, img.height as i32)
    })
}

/// Cinza que apaga os ícones de ferramenta (cor `F` da paleta de 16).
const DIM: [u8; 3] = [100, 100, 100];

/// Figuras que o original desenha "apagadas" (um pixel sim, outro não — os
/// de x + y par, contados do canto da figura — vira cinza dentro dela).
/// No Fine Artist, as que não têm som de clique na tabela (terceira palavra
/// `ffff`), como os ícones das ferramentas e os bichos não escolhidos;
/// gemas, chaves, bolinhas e o "EPA!" ficam inteiros (a bolinha verde já é
/// pontilhada na figura). No Writer, conferido nas capturas do Escritório,
/// todas saem apagadas — até as setas da barra de baixo e os botões do
/// Imprimir, que têm som —, menos as ligações, as opções e o "EPA!".
pub fn dimmed(p: Program, what_id: u16) -> bool {
    match (p, what_id) {
        (Program::Writer, 0x230) => false,
        (Program::Writer, id) if shared(id) => dimmed(Program::Artist, id),
        (Program::Writer, id) => writer_icon(id).is_some(),
        (_, 0x1047 | 0x1048) => false,
        (_, 0x1000..=0x10ff) => true,
        (_, 0x201 | 0x223 | 0x228 | 0x229 | 0x22d | 0x237..=0x242 | 0x247) => true,
        _ => false,
    }
}

/// Desenha o ícone da figura `what_id` dos balões do programa `p` em
/// `(x, y)` (sem entrada no mapa, veja [`size`]). Apagado se [`dimmed`].
pub fn draw(f: &mut Frame, a: &Assets, p: Program, what_id: u16, x: i32, y: i32) {
    draw_with(f, a, p, what_id, x, y, dimmed(p, what_id));
}

/// Como [`draw`], escolhendo se a figura sai apagada (a escolhida de uma
/// grade não sai).
pub fn draw_with(f: &mut Frame, a: &Assets, p: Program, what_id: u16, x: i32, y: i32, dim: bool) {
    let Some(ic) = shown(p, what_id) else {
        return;
    };
    draw_plain(f, a, p, ic, x, y);
    if !dim || !a.has_image(&pic(p, ic.pic())) {
        return;
    }
    // a parte opaca da figura: a máscara, o que não é branco ou tudo
    let img = a.image(&pic(p, ic.pic()));
    let mask = match ic {
        Icon::Masked(_, m) if a.has_image(&pic(p, m)) => Some(a.image(&pic(p, m))),
        _ => None,
    };
    for dy in 0..img.height as i32 {
        for dx in 0..img.width as i32 {
            if (dx + dy) % 2 != 0 {
                continue;
            }
            let i = ((dy as u32 * img.width + dx as u32) * 4) as usize;
            let opaque = match (&ic, &mask) {
                (_, Some(m)) => m.rgba.get(i..i + 3) == Some(&[0, 0, 0][..]),
                (Icon::Opaque(_), None) => img.rgba[i + 3] != 0,
                _ => img.rgba[i + 3] != 0 && img.rgba[i..i + 3] != [255, 255, 255],
            };
            if opaque {
                f.put(x + dx, y + dy, DIM);
            }
        }
    }
}

fn draw_plain(f: &mut Frame, a: &Assets, p: Program, ic: Icon, x: i32, y: i32) {
    match ic {
        Icon::Masked(c, m) if a.has_image(&pic(p, c)) && a.has_image(&pic(p, m)) => {
            crate::anim::blit_masked(f, &a.image(&pic(p, c)), &a.image(&pic(p, m)), x, y);
        }
        Icon::Masked(c, _) | Icon::Keyed(c) => {
            crate::text_balloon::blit_pic_in(f, a, p, c, x, y, false)
        }
        Icon::Opaque(c) => {
            if a.has_image(&pic(p, c)) {
                f.blit(&a.image(&pic(p, c)), x, y);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_icons_map_to_pic_ids() {
        // barra de ferramentas: colorida e máscara
        assert_eq!(icon(0x1000), Some(Icon::Masked(0x0495, 0x1001)));
        assert_eq!(icon(0x1039), Some(Icon::Masked(0x103d, 0x103f)));
        assert_eq!(icon(0x1036), Some(Icon::Masked(0x100b, 0x100d)));
        // botões do Aspirador, formas e pincéis: opacos
        assert_eq!(icon(0x1051), Some(Icon::Opaque(0x1567)));
        assert_eq!(icon(0x1080), Some(Icon::Opaque(0x17e5)));
        // ligações: gema, chave, Max, Maggie; opções e "EPA!"
        assert_eq!(icon(0x21b), Some(Icon::Masked(0x03f7, 0x03f9)));
        assert_eq!(icon(0x21c), Some(Icon::Masked(0x03fb, 0x03fd)));
        assert_eq!(icon(0x21e).map(Icon::pic), Some(0x0405));
        assert_eq!(icon(0x22c).map(Icon::pic), Some(0x03d3));
        assert_eq!(icon(0x22e).map(Icon::pic), Some(0x0409));
        assert_eq!(icon(0x230).map(Icon::pic), Some(0x02a9));
        // bichos do cadastro
        assert_eq!(icon(0x237), Some(Icon::Keyed(0x045b)));
        assert_eq!(icon(0x242), Some(Icon::Keyed(0x0471)));
        assert_eq!(icon(0x1072).map(Icon::pic), Some(0x02f3));
        assert_eq!(icon(0x10ff), None);
    }

    #[test]
    fn every_figure_used_in_the_what_blocks_is_mapped() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut missing = std::collections::BTreeSet::new();
        for block in a.exe_resource_ids(Program::Artist, "WHAT") {
            for r in a.dialogs(Program::Artist, block) {
                for i in &r.items {
                    if let formats::what::Item::Picture { id, .. } = i
                        && (0x200..0x10ff).contains(id)
                        && *id != 0x203
                        && icon(*id).is_none()
                    {
                        missing.insert(*id);
                    }
                }
            }
        }
        assert!(missing.is_empty(), "sem mapa: {missing:x?}");
        // e as figuras existem no .PIC
        for id in [0x1000, 0x1070, 0x10b2, 0x21b, 0x230, 0x24a] {
            assert!(size(&a, Program::Artist, id).is_some(), "{id:x}");
        }
    }

    #[test]
    fn writer_icons_follow_the_writer_table() {
        // conferidos na tabela 0x800 + k do WRITER.EXE: Cavalete, rolo de
        // bordas, Re-movedores escolhido, setas dos rolos
        assert_eq!(writer_icon(0x80a), Some(Icon::Masked(0x0881, 0x0883)));
        assert_eq!(writer_icon(0x80c), Some(Icon::Masked(0x0825, 0x0827)));
        assert_eq!(writer_icon(0x812), Some(Icon::Masked(0x08eb, 0x08ed)));
        assert_eq!(writer_icon(0x851), Some(Icon::Masked(0x08d9, 0x08dd)));
        assert_eq!(writer_icon(0x200), Some(Icon::Masked(0x020d, 0x020f)));
        assert_eq!(writer_icon(0x805).map(Icon::pic), Some(0x080b));
        // "EPA!" e volume: mesma entrada, outra imagem no WRITER.PIC
        assert_eq!(writer_icon(0x230), Some(Icon::Masked(0x02cf, 0x02d1)));
        assert_eq!(writer_icon(0x209), Some(Icon::Masked(0x0255, 0x0257)));
        // máscaras sempre adiante da figura (cor, preto e branco, máscara)
        for id in 0x200..0x900 {
            if let Some(Icon::Masked(p, m)) = writer_icon(id) {
                assert!(m >= p + 2, "{id:x}");
            }
        }
    }

    #[test]
    fn writer_shares_links_and_options_but_not_artist_tools() {
        for id in [0x21b, 0x21c, 0x21e, 0x222, 0x22c, 0x22e, 0x237] {
            assert_eq!(icon_for(Program::Writer, id), icon(id), "{id:x}");
        }
        assert_eq!(icon_for(Program::Writer, 0x1000), None);
        assert_eq!(icon_for(Program::Writer, 0x227), writer_icon(0x227));
        assert_eq!(icon_for(Program::Artist, 0x1039), icon(0x1039));
        assert_eq!(icon_for(Program::Artist, 0x230), icon(0x230));
    }

    #[test]
    fn writer_dims_everything_but_links_options_and_epa() {
        let w = Program::Writer;
        for id in [0x200, 0x225, 0x20d, 0x80a, 0x83a, 0x83d, 0x85a, 0x223] {
            assert!(dimmed(w, id), "{id:x}");
        }
        for id in [0x21b, 0x21c, 0x21e, 0x22c, 0x22e, 0x230, 0x1000] {
            assert!(!dimmed(w, id), "{id:x}");
        }
        // o Fine Artist continua como era
        assert!(dimmed(Program::Artist, 0x1000));
        assert!(!dimmed(Program::Artist, 0x1047));
        assert!(!dimmed(Program::Artist, 0x21b));
        assert!(dimmed(Program::Artist, 0x223));
    }

    #[test]
    fn every_figure_used_in_the_writer_what_blocks_is_mapped() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        let mut missing = std::collections::BTreeSet::new();
        for block in a.exe_resource_ids(Program::Writer, "WHAT") {
            for r in a.dialogs(Program::Writer, block) {
                for i in &r.items {
                    if let formats::what::Item::Picture { id, .. } = i
                        && *id != formats::what::PIC_GLOSSARY
                        && *id != crate::rich::PIC_FIELD
                        && size(&a, Program::Writer, *id).is_none()
                    {
                        missing.insert(*id);
                    }
                }
            }
        }
        // 826: botão "TROCAR" do Corretor, na tabela de botões do Writer
        // (ainda não lida)
        assert_eq!(missing, [0x826].into(), "sem mapa: {missing:x?}");
    }
}
