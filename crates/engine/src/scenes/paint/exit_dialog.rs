//! Diálogo "Epa!" da tela de pintura: aparece ao abrir a porta de saída ou ao
//! pedir um quadro NOVO (`WHAT 0203`, registro "now what?"). É modal: só as
//! bolinhas respondem; o resto dá um beep (medido no original, ver
//! `docs/telas/pintura-menu-mczee.md`).

use crate::Frame;
use crate::assets::{Assets, Program};
use crate::text_balloon::{PIC_ALERT, Part, TextBalloon};

/// Bloco `WHAT` dos diálogos da pintura ("now what?" é o registro 0).
pub const WHAT_BLOCK: u16 = 0x0203;
/// Canto e largura do aviso, medidos no original (a caixa tem 1 px a mais).
pub const BOX: (i32, i32, i32) = (208, 169, 223);
/// Vaga da bolinha das opções: só ela é botão.
const BULLET_SLOT: i32 = crate::rich::OPTION_W;

/// O que o usuário escolheu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// "Salvar este trabalho": dá nome ao quadro e guarda.
    Save,
    /// "Jogar fora — reciclar": descarta o quadro.
    Discard,
    /// "Ops! Ainda não terminei!": fecha o diálogo e fica.
    Stay,
}

/// Por que o diálogo abriu: o que fazer depois de guardar ou descartar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Porta de saída: depois, sai da tela.
    Exit,
    /// NOVO do menu do McZee: depois, começa um quadro em branco.
    New,
}

pub struct ExitDialog {
    pub reason: Reason,
    /// O aviso do Rex, desenhado como os outros balões.
    balloon: TextBalloon,
}

impl ExitDialog {
    pub fn new(reason: Reason, assets: &Assets) -> ExitDialog {
        let records = assets.dialogs(Program::Artist, WHAT_BLOCK);
        let balloon = match records.first() {
            Some(r) if r.options().len() == 3 => {
                TextBalloon::from_record(r, BOX.0, BOX.1, "", Vec::new())
            }
            _ => TextBalloon::new(
                BOX.0,
                BOX.1,
                BOX.2,
                vec![
                    Part::Figure(PIC_ALERT),
                    Part::Text("O que você quer fazer agora?".into()),
                    Part::Options(vec![
                        "Salvar este trabalho".to_string(),
                        "Jogar fora — reciclar".to_string(),
                        "Ops!  Ainda não terminei!".to_string(),
                    ]),
                ],
                Vec::new(),
            ),
        }
        .as_alert();
        ExitDialog { reason, balloon }
    }

    /// Retângulo da opção `k` (linha inteira).
    fn option_rect(&self, a: &Assets, k: usize) -> (i32, i32, i32, i32) {
        self.balloon.option_rect(a, k).unwrap_or((0, 0, 0, 0))
    }

    /// Centro da bolinha da opção `k` (para testes e ferramentas).
    pub fn bullet(&self, a: &Assets, k: usize) -> (i32, i32) {
        let r = self.option_rect(a, k);
        (r.0 + BULLET_SLOT / 2, r.1 + r.3 / 2)
    }

    /// Escolha sob `(x, y)`: só a bolinha conta (o texto não é botão).
    pub fn hit(&self, a: &Assets, x: i32, y: i32) -> Option<Choice> {
        (0..self.balloon.options().len())
            .find(|&k| {
                let r = self.option_rect(a, k);
                x >= r.0 - 2 && y >= r.1 && x < r.0 + BULLET_SLOT + 2 && y < r.1 + r.3
            })
            .map(|k| match k {
                0 => Choice::Save,
                1 => Choice::Discard,
                _ => Choice::Stay,
            })
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        self.balloon.draw(f, a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    #[test]
    fn bullets_in_order_and_text_is_not_a_button() {
        let (_d, a) = empty_assets();
        let d = ExitDialog::new(Reason::Exit, &a);
        assert_eq!(d.balloon.options().len(), 3);
        let r0 = d.option_rect(&a, 0);
        let r2 = d.option_rect(&a, 2);
        assert_eq!(d.hit(&a, r0.0 + 2, r0.1 + 4), Some(Choice::Save));
        assert_eq!(d.hit(&a, r2.0 + 2, r2.1 + 4), Some(Choice::Stay));
        assert_eq!(d.hit(&a, BOX.0 + 10, BOX.1 + 10), None);
        // o texto da opção não é botão
        assert_eq!(d.hit(&a, r0.0 + 60, r0.1 + 4), None);
        assert_eq!(d.hit(&a, 100, 100), None);
        let mut f = Frame::new(640, 480);
        d.draw(&mut f, &a);
    }
}
