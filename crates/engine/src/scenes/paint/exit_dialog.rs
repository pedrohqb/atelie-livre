//! Diálogo "Epa!" da tela de pintura: aparece ao abrir a porta de saída ou ao
//! pedir um quadro NOVO (`WHAT 0203`, registro "now what?"). É modal: só as
//! bolinhas respondem; o resto dá um beep (medido no original, ver
//! `docs/telas/pintura-menu-mczee.md`).

use crate::Frame;
use crate::assets::{Assets, Program};
use crate::balloon::{balloon_font, draw_box, draw_bullet, draw_line, line_height, wrap_opt};
use crate::text_balloon::blit_pic;
use formats::fnt::Font;

/// Bloco `WHAT` dos diálogos da pintura ("now what?" é o registro 0).
pub const WHAT_BLOCK: u16 = 0x0203;
/// Casinha do Rex + EPA! + cachorro, no alto do diálogo.
const PICTURE: u16 = 0x02a9;
const PICTURE_POS: (i32, i32) = (223, 174);
/// Caixa do diálogo, medida no original.
pub const BOX: (i32, i32, i32, i32) = (210, 170, 220, 140);
/// Onde começa o texto, abaixo da figura.
const TEXT_Y: i32 = 230;
const PAD: i32 = 8;
const BULLET: i32 = 14;
const OPTION_GAP: i32 = 6;

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
    text: String,
    options: Vec<String>,
}

impl ExitDialog {
    pub fn new(reason: Reason, assets: &Assets) -> ExitDialog {
        let records = assets.dialogs(Program::Artist, WHAT_BLOCK);
        let (text, options) = match records.first() {
            Some(r) if r.options().len() == 3 => {
                let b = crate::balloon::Balloon::from_record(r, 0, 0, "");
                (b.text, b.options)
            }
            _ => (
                "O que você quer fazer agora?".to_string(),
                vec![
                    "Salvar este trabalho".to_string(),
                    "Jogar fora — reciclar".to_string(),
                    "Ops!  Ainda não terminei!".to_string(),
                ],
            ),
        };
        ExitDialog {
            reason,
            text,
            options,
        }
    }

    fn layout(&self, font: Option<&Font>) -> (Vec<String>, Vec<Vec<String>>) {
        let inner = (BOX.2 - 2 * PAD) as u32;
        let lines = wrap_opt(font, &self.text, inner);
        let opts = self
            .options
            .iter()
            .map(|o| wrap_opt(font, o, inner - (BULLET + 4) as u32))
            .collect();
        (lines, opts)
    }

    /// Retângulo da opção `k` (bolinha e texto).
    fn option_rect(&self, font: Option<&Font>, k: usize) -> (i32, i32, i32, i32) {
        let (lines, opts) = self.layout(font);
        let lh = line_height(font);
        let mut y0 = TEXT_Y + lines.len() as i32 * lh + OPTION_GAP;
        for o in opts.iter().take(k) {
            y0 += o.len() as i32 * lh + OPTION_GAP;
        }
        let n = opts.get(k).map_or(1, |o| o.len() as i32);
        (BOX.0 + PAD, y0 - 2, BOX.2 - 2 * PAD, n * lh + 4)
    }

    /// Centro da bolinha da opção `k` (para testes e ferramentas).
    pub fn bullet(&self, a: &Assets, k: usize) -> (i32, i32) {
        let font = balloon_font(a);
        let r = self.option_rect(font.as_deref(), k);
        (r.0 + 5, r.1 + r.3 / 2)
    }

    /// Escolha sob `(x, y)`: só a bolinha conta (o texto não é botão).
    pub fn hit(&self, a: &Assets, x: i32, y: i32) -> Option<Choice> {
        let font = balloon_font(a);
        (0..self.options.len())
            .find(|&k| {
                let r = self.option_rect(font.as_deref(), k);
                x >= r.0 - 2 && y >= r.1 && x < r.0 + BULLET + 2 && y < r.1 + r.3
            })
            .map(|k| match k {
                0 => Choice::Save,
                1 => Choice::Discard,
                _ => Choice::Stay,
            })
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let (x, y, w, h) = BOX;
        draw_box(f, x, y, w, h);
        blit_pic(f, a, PICTURE, PICTURE_POS.0, PICTURE_POS.1, true);
        let font = balloon_font(a);
        let (lines, opts) = self.layout(font.as_deref());
        let lh = line_height(font.as_deref());
        let mut ty = TEXT_Y;
        for line in &lines {
            draw_line(f, font.as_deref(), x + PAD, ty, line);
            ty += lh;
        }
        for (k, opt) in opts.iter().enumerate() {
            let r = self.option_rect(font.as_deref(), k);
            draw_bullet(f, r.0, r.1 + 2 + (lh - 10) / 2);
            for (j, line) in opt.iter().enumerate() {
                draw_line(
                    f,
                    font.as_deref(),
                    r.0 + BULLET + 4,
                    r.1 + 2 + j as i32 * lh,
                    line,
                );
            }
        }
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
        assert_eq!(d.options.len(), 3);
        let font = balloon_font(&a);
        let r0 = d.option_rect(font.as_deref(), 0);
        let r2 = d.option_rect(font.as_deref(), 2);
        assert_eq!(d.hit(&a, r0.0 + 2, r0.1 + 2), Some(Choice::Save));
        assert_eq!(d.hit(&a, r2.0 + 2, r2.1 + 2), Some(Choice::Stay));
        assert_eq!(d.hit(&a, BOX.0 + 10, TEXT_Y), None);
        // o texto da opção não é botão
        assert_eq!(d.hit(&a, r0.0 + 60, r0.1 + 2), None);
        assert_eq!(d.hit(&a, 100, 100), None);
        let mut f = Frame::new(640, 480);
        d.draw(&mut f, &a);
    }
}
