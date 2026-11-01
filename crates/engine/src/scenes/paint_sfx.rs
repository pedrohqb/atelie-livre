//! Sons da tela de pintura, medidos no original (ver `docs/telas/sons-das-acoes.md`).
//!
//! Só tabelas e funções puras: `paint.rs` chama daqui nos ganchos das ações.

use super::supertools::Op as SuperOp;
use crate::assets::{Program, SoundRef};
use crate::scene::Effect;

/// Lupa da barra de baixo (sem função ainda; só o som).
pub const MAGNIFIER_ICON: (i32, i32, i32, i32) = (522, 460, 24, 20);

/// Botão de opção da faixa.
pub const OPTION: u16 = 0x021c;
/// Seta de página da faixa.
pub const PAGE_ARROW: u16 = 0x021d;
/// Opção do aspirador (todas).
pub const VACUUM_OPTION: u16 = 0x0252;
/// Opção da Palavra (no original abre o Cavalete de palavras).
pub const WORDS_OPTION: u16 = 0x0224;
/// Clique no quadro com a Palavra (começa o texto).
pub const WORDS_START: u16 = 0x0209;
/// Palavra pronta no quadro (o OK do Cavalete).
pub const WORDS_DONE: u16 = 0x02ee;
pub const STICKER_ROLL: u16 = 0x0259;
pub const STICKER_PICK: u16 = 0x022d;
pub const STICKER_PLACE: u16 = 0x0228;
pub const SOUND_CATEGORY: u16 = 0x0233;
pub const MCZEE: u16 = 0x02cf;
pub const MAP: u16 = 0x02d3;
pub const MAGNIFIER: u16 = 0x0309;
/// Traçar guias com as Super-ferramentas (laço).
pub const GUIDE_STROKE: u16 = 0x022c;

/// Som de cada ferramenta da barra, na ordem de `TOOLS`.
pub const TOOLS: [u16; 10] = [
    0x0213, // pincel
    0x0214, // lata
    0x020a, // adesivos
    0x020b, // sons
    0x0209, // palavra
    0x020e, // ovo
    0x0215, // aspirador
    0x0210, // transformadores
    0x0216, // super-ferramentas
    0x0212, // imprimir
];

const EFFECTS: [u16; 12] = [
    0x0266, 0x0267, 0x0268, 0x0269, 0x0301, 0x026b, 0x026c, 0x026d, 0x02c6, 0x0255, 0x026e, 0x024f,
];

/// Piscada do ícone clicado (versão de 1 bit), em segundos.
const FLASH: f32 = 0.1;
/// Ovo inteiro (cor, máscara), no lugar do pintinho logo depois do clique.
const EGG_WHOLE: (u16, u16) = (0x1041, 0x1043);
const EGG_TIME: f32 = 0.33;
/// Torradeira soltando fumaça enquanto "imprime".
const PRINT_ACTIVE: (u16, u16) = (0x106d, 0x106f);
const PRINT_TIME: f32 = 1.0;
/// Índices em `TOOLS`.
const EGG: usize = 5;
const PRINT: usize = 9;

/// Estado dos sons e animações da tela de pintura.
#[derive(Debug, Default)]
pub struct State {
    /// Há um som em laço tocando (parar ao soltar o botão).
    pub looping: bool,
    /// Ícone da barra animado depois do clique: índice em `TOOLS` e tempo decorrido.
    anim: Option<(usize, f32)>,
}

impl State {
    /// Clique no ícone `i` da barra: começa a animação dele.
    pub fn tool_clicked(&mut self, i: usize) {
        self.anim = Some((i, 0.0));
    }

    pub fn update(&mut self, dt: f32) {
        if let Some((i, t)) = self.anim.as_mut() {
            *t += if dt.is_finite() { dt.max(0.0) } else { 0.0 };
            let end = match *i {
                EGG => EGG_TIME,
                PRINT => PRINT_TIME,
                _ => FLASH,
            };
            if *t >= end {
                self.anim = None;
            }
        }
    }

    /// Imagem (cor, máscara) a desenhar no lugar do ícone `i`, cuja máscara
    /// normal é `mask`, enquanto ele anima; o fundo fica aceso.
    ///
    /// O original pisca a versão de 1 bit do ícone (a imagem antes da máscara)
    /// por um quadro e passa à imagem ativa; o ovo mostra o ovo inteiro antes
    /// de voltar ao pintinho e a impressora solta fumaça (ver
    /// `docs/telas/sons-das-acoes.md`).
    pub fn icon(&self, i: usize, mask: u16) -> Option<(u16, u16)> {
        let (j, t) = self.anim?;
        if j != i {
            return None;
        }
        Some(match i {
            _ if t < FLASH => (mask.wrapping_sub(1), mask),
            EGG => EGG_WHOLE,
            PRINT => PRINT_ACTIVE,
            _ => return None,
        })
    }

    /// Começa o som em laço do traço (ou do aspirador, das guias).
    pub fn start_loop(&mut self, id: u16) -> Effect {
        self.looping = true;
        Effect::PlayLoop(snd(id))
    }
}

/// Som de um clique na faixa dos adesivos: colocou, pegou do filme, seta, abriu um rolo.
/// `before` e `after` são (segurando um adesivo, filme aberto).
pub fn sticker(
    placed: bool,
    before: (bool, bool),
    after: (bool, bool),
    arrow: bool,
) -> Option<u16> {
    if placed {
        Some(STICKER_PLACE)
    } else if after.0 && !before.0 {
        Some(STICKER_PICK)
    } else if arrow {
        Some(PAGE_ARROW)
    } else if after.1 && !before.1 {
        Some(STICKER_ROLL)
    } else {
        None
    }
}

/// As Super-ferramentas traçam no quadro (som em laço) com esta opção?
pub fn supertool_traces(op: SuperOp) -> bool {
    !matches!(op, SuperOp::Approach | SuperOp::Recede | SuperOp::Eraser)
}

pub fn snd(id: u16) -> SoundRef {
    SoundRef::Snd(Program::Artist, id)
}

pub fn play(id: u16) -> Effect {
    Effect::PlaySound(snd(id))
}

/// Som do traço da forma `k` da página `page` (tocado em laço enquanto arrasta).
pub fn stroke(page: u8, k: usize) -> u16 {
    match page {
        2 => 0x0263,
        3 => EFFECTS[k.min(EFFECTS.len() - 1)],
        5 => match k {
            2 | 3 | 5 => 0x022b,
            6..=10 => 0x022c,
            _ => 0x022a,
        },
        6 => 0x0250,
        _ => 0x021e,
    }
}

/// Som do despejo da lata com a opção `k`.
pub fn bucket(k: usize) -> u16 {
    match k {
        0 => 0x0225,
        1 | 2 => 0x0227,
        3 => 0x0258,
        _ => 0x0223,
    }
}

/// Som do uso do aspirador na opção `k`: id e se repete enquanto arrasta.
pub fn vacuum(k: usize) -> Option<(u16, bool)> {
    match k {
        0 | 5 => Some((0x0229, true)),
        2 => Some((0x0228, false)),
        3 | 7 => Some((0x025f, false)),
        4 => Some((0x025e, true)),
        _ => None,
    }
}

/// Som da opção `k` da faixa dos Transformadores.
pub fn transformer_option(k: usize) -> u16 {
    if k == 0 { 0x021f } else { OPTION }
}

/// Som da casa `k` da faixa das Super-ferramentas.
pub fn supertool_option(k: usize) -> u16 {
    match k {
        0 => 0x0221,
        1 | 2 => 0x0233,
        _ => OPTION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stroke_sounds_by_page() {
        assert_eq!(stroke(1, 5), 0x021e);
        assert_eq!(stroke(2, 0), 0x0263);
        assert_eq!(stroke(3, 4), 0x0301);
        assert_eq!(stroke(3, 11), 0x024f);
        assert_eq!(stroke(4, 11), 0x021e);
        assert_eq!(stroke(5, 0), 0x022a);
        assert_eq!(stroke(5, 3), 0x022b);
        assert_eq!(stroke(5, 8), 0x022c);
        assert_eq!(stroke(5, 11), 0x022a);
        assert_eq!(stroke(6, 2), 0x0250);
    }

    #[test]
    fn icon_animation_flashes_then_shows_egg() {
        let mut s = State::default();
        s.tool_clicked(1);
        assert_eq!(s.icon(1, 0x1005), Some((0x1004, 0x1005)));
        assert_eq!(s.icon(2, 0x1023), None);
        s.update(0.2);
        assert_eq!(s.icon(1, 0x1005), None);
        s.tool_clicked(EGG);
        s.update(0.15);
        assert_eq!(s.icon(EGG, 0x103f), Some(EGG_WHOLE));
        s.update(0.3);
        assert_eq!(s.icon(EGG, 0x103f), None);
        s.update(f32::NAN);
    }

    #[test]
    fn bucket_and_vacuum() {
        assert_eq!(bucket(0), 0x0225);
        assert_eq!(bucket(9), 0x0223);
        assert_eq!(vacuum(0), Some((0x0229, true)));
        assert_eq!(vacuum(6), None);
    }
}
