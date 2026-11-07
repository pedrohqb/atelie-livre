//! Fluxo de fim guiado dos projetos (botão, adesivo e história em quadrinhos),
//! com os balões do original (`WHAT` 3c00 e 3e00 do `ARTIST.EXE`):
//!
//! - Imprimir: "Você quer imprimir o botão?" → imprime → "D+. Você terminou!
//!   O que você quer fazer agora?" (continuar, começar um novo, voltar ao
//!   andar dos Projetos).
//! - Sair (ou começar um novo) com mudanças: "Você quer salvar ...?" → "Digite
//!   um nome descritivo" → grava `<tipo>-<nome>.png`; "Não, jogar fora" →
//!   "Você quer imprimir antes que vá para o lixo?" → o trabalho não é
//!   guardado (e o último desse tipo é esquecido).
//!
//! O McZee imprime quantas cópias cabem na página (3c00 [21]), por isso não
//! há escolha do número de cópias.

use super::ProjectKind;
use crate::assets::{Assets, Program};
use crate::balloon::Balloon;
use crate::scene::{Input, Key, MouseButton};
use crate::text_balloon::{Button, Part, TextBalloon};
use crate::{Frame, SCREEN_W};

const WHAT_BUTTON: u16 = 0x3c00;
const WHAT_COMICS: u16 = 0x3e00;
/// Canto do balão (o mesmo da ajuda), puxado para a esquerda se não couber.
const POS: (i32, i32) = (445, 115);

/// Passo do fluxo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// "Você quer imprimir ...?"
    PrintAsk,
    /// "D+. Você terminou! O que você quer fazer agora?"
    Done,
    /// "Você quer salvar ...?"
    SaveAsk,
    /// "Digite um nome descritivo e aperte Enter."
    NameAsk,
    /// "Você quer imprimir ... antes que vá para o lixo?"
    PrintBeforeTrash,
}

/// O que fazer quando o fluxo de salvar termina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum After {
    /// Começar um novo trabalho do mesmo tipo.
    New,
    /// Voltar ao andar dos Projetos.
    Leave,
}

/// O que a tela de pintura deve fazer depois de um clique no balão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nada ainda (o balão mudou ou continua).
    Continue,
    /// Fechar o balão sem mais nada.
    Close,
    /// Imprimir; depois o balão "D+. Você terminou!".
    Print,
    /// Gravar com o nome digitado e seguir para `After`.
    SaveAs(String, After),
    /// Imprimir, jogar fora e seguir para `After`.
    PrintAndDiscard(After),
    /// Jogar fora e seguir para `After`.
    Discard(After),
}

/// Balão de opções ou de nome em andamento.
pub struct Dialog {
    pub step: Step,
    pub after: After,
    kind: ProjectKind,
    balloon: Balloon,
    /// Caixa de texto do nome (só em `NameAsk`).
    name: Option<TextBalloon>,
}

/// Registro do `WHAT` e texto reserva de cada passo, por tipo de projeto.
fn script(kind: ProjectKind, step: Step) -> (u16, usize, &'static str, &'static [&'static str]) {
    // registros (botão, adesivo, quadrinhos) de cada passo
    let (button, sticker, comics) = match step {
        Step::PrintAsk => (15, 30, 154),
        Step::Done => (22, 31, 162),
        Step::SaveAsk => (12, 27, 151),
        Step::NameAsk => (13, 28, 153),
        Step::PrintBeforeTrash => (14, 29, 152),
    };
    let (block, record) = match kind {
        ProjectKind::Button => (WHAT_BUTTON, button),
        ProjectKind::Sticker => (WHAT_BUTTON, sticker),
        _ => (WHAT_COMICS, comics),
    };
    let (text, options): (&'static str, &'static [&'static str]) = match step {
        Step::PrintAsk => (
            match kind {
                ProjectKind::Button => "Você quer imprimir o botão?",
                ProjectKind::Sticker => "Você quer imprimir o adesivo?",
                _ => "Você quer imprimir esta história em quadrinhos?",
            },
            &["É, imprimir", "Não imprimir, não"],
        ),
        Step::Done => (
            if kind == ProjectKind::Comics {
                "Chocante! Você acabou. O que você quer fazer agora?"
            } else {
                "D+. Você terminou!\nO que você quer fazer agora?"
            },
            match kind {
                ProjectKind::Button => &[
                    "Trabalhar com este botão por conta própria",
                    "Começar um novo botão ou adesivo",
                    "Voltar ao andar dos Projetos",
                ],
                ProjectKind::Sticker => &[
                    "Trabalhar com este adesivo por conta própria",
                    "Começar um novo botão ou adesivo",
                    "Voltar ao andar dos Projetos",
                ],
                _ => &[
                    "Trabalhar na história em quadrinhos por conta própria",
                    "Começar outra história em quadrinhos",
                    "Voltar para o andar dos Projetos",
                ],
            },
        ),
        Step::SaveAsk => (
            match kind {
                ProjectKind::Button => {
                    "Você quer salvar o seu botão — para poder abri-lo mais tarde e trabalhar com ele?"
                }
                ProjectKind::Sticker => {
                    "Você quer salvar o seu adesivo — para poder abri-lo mais tarde e trabalhar com ele?"
                }
                _ => {
                    "Você quer salvar a sua história em quadrinhos — para poder abrir mais tarde e trabalhar com ela?"
                }
            },
            &["É, salvar", "Não, jogar fora"],
        ),
        Step::NameAsk => ("Digite um nome descritivo e aperte Enter.", &[]),
        Step::PrintBeforeTrash => (
            if kind == ProjectKind::Comics {
                "Você quer imprimir a história em quadrinhos antes de jogar no lixo?"
            } else if kind == ProjectKind::Button {
                "Você quer imprimir o botão antes que ele vá para o lixo?"
            } else {
                "Você quer imprimir o adesivo antes que ele vá para o lixo?"
            },
            &["É, imprimir", "Não, jogar fora"],
        ),
    };
    (block, record, text, options)
}

impl Dialog {
    /// Abre o fluxo no passo `step`.
    pub fn open(kind: ProjectKind, step: Step, after: After, assets: &Assets) -> Dialog {
        let (block, record, text, options) = script(kind, step);
        let records = assets.dialogs(Program::Artist, block);
        let mut balloon = Balloon::from_block(&records, record, POS.0, POS.1, "")
            .filter(|b| !b.text.is_empty())
            .unwrap_or_else(|| Balloon {
                x: POS.0,
                y: POS.1,
                w: 230,
                text: text.into(),
                options: options.iter().map(|o| o.to_string()).collect(),
            });
        if balloon.options.is_empty() && !options.is_empty() {
            balloon.options = options.iter().map(|o| o.to_string()).collect();
        }
        balloon.x = balloon.x.min(SCREEN_W as i32 - 8 - balloon.w);
        let name = (step == Step::NameAsk).then(|| {
            let mut b = TextBalloon::new(
                balloon.x,
                balloon.y,
                balloon.w,
                vec![
                    Part::Text(balloon.text.clone()),
                    Part::Field {
                        field: 0,
                        width: 160,
                    },
                ],
                vec![Button::Stop, Button::Next],
            );
            b.fields = vec![String::new()];
            b
        });
        Dialog {
            step,
            after,
            kind,
            balloon,
            name,
        }
    }

    fn goto(&mut self, step: Step, assets: &Assets) {
        *self = Dialog::open(self.kind, step, self.after, assets);
    }

    pub fn contains(&self, a: &Assets, x: i32, y: i32) -> bool {
        match &self.name {
            Some(b) => b.contains(a, x, y),
            None => self.balloon.contains(a, x, y),
        }
    }

    /// Retângulo da opção `k` (para testes e para quem quiser clicar nela).
    pub fn option_rect(&self, a: &Assets, k: usize) -> (i32, i32, i32, i32) {
        self.balloon.option_rect(a, k)
    }

    /// Trata uma entrada. Um clique fora do balão cancela (`Close`).
    pub fn handle(&mut self, input: &Input, assets: &Assets) -> Outcome {
        if let Some(b) = self.name.as_mut() {
            if let Input::Click { x, y, .. } = input
                && !b.contains(assets, *x, *y)
            {
                return Outcome::Close;
            }
            return match b.handle(assets, input) {
                Some(Button::Next) => {
                    let name = b.fields.first().cloned().unwrap_or_default();
                    let name = name.trim().to_string();
                    if name.is_empty() {
                        Outcome::Continue
                    } else {
                        Outcome::SaveAs(name, self.after)
                    }
                }
                Some(Button::Stop) | Some(Button::Back) => Outcome::Close,
                None => Outcome::Continue,
            };
        }
        let Input::Click {
            button: MouseButton::Left,
            x,
            y,
        } = input
        else {
            if matches!(
                input,
                Input::Key {
                    key: Key::Escape,
                    ..
                }
            ) {
                return Outcome::Close;
            }
            return Outcome::Continue;
        };
        if !self.balloon.contains(assets, *x, *y) {
            return Outcome::Close;
        }
        let Some(k) = self.balloon.hit(assets, *x, *y) else {
            return Outcome::Continue;
        };
        match (self.step, k) {
            (Step::PrintAsk, 0) => Outcome::Print,
            (Step::PrintAsk, _) => Outcome::Close,
            (Step::Done, 0) => Outcome::Close,
            (Step::Done, 1) => {
                self.after = After::New;
                self.goto(Step::SaveAsk, assets);
                Outcome::Continue
            }
            (Step::Done, _) => {
                self.after = After::Leave;
                self.goto(Step::SaveAsk, assets);
                Outcome::Continue
            }
            (Step::SaveAsk, 0) => {
                self.goto(Step::NameAsk, assets);
                Outcome::Continue
            }
            (Step::SaveAsk, _) => {
                self.goto(Step::PrintBeforeTrash, assets);
                Outcome::Continue
            }
            (Step::PrintBeforeTrash, 0) => Outcome::PrintAndDiscard(self.after),
            (Step::PrintBeforeTrash, _) => Outcome::Discard(self.after),
            (Step::NameAsk, _) => Outcome::Continue,
        }
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        match &self.name {
            Some(b) => b.draw(f, a),
            None => self.balloon.draw(f, a),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    fn click_option(d: &mut Dialog, a: &Assets, k: usize) -> Outcome {
        let r = d.option_rect(a, k);
        d.handle(
            &Input::Click {
                button: MouseButton::Left,
                x: r.0 + 2,
                y: r.1 + 2,
            },
            a,
        )
    }

    #[test]
    fn print_flow_goes_to_done_then_save_and_name() {
        let (_d, a) = empty_assets();
        let mut d = Dialog::open(ProjectKind::Button, Step::PrintAsk, After::Leave, &a);
        assert!(d.balloon.x + d.balloon.w <= SCREEN_W as i32);
        assert_eq!(click_option(&mut d, &a, 1), Outcome::Close);
        assert_eq!(click_option(&mut d, &a, 0), Outcome::Print);
        let mut d = Dialog::open(ProjectKind::Sticker, Step::Done, After::Leave, &a);
        assert_eq!(d.balloon.options.len(), 3);
        assert_eq!(click_option(&mut d, &a, 0), Outcome::Close);
        assert_eq!(click_option(&mut d, &a, 1), Outcome::Continue);
        assert_eq!((d.step, d.after), (Step::SaveAsk, After::New));
        assert_eq!(click_option(&mut d, &a, 0), Outcome::Continue);
        assert_eq!(d.step, Step::NameAsk);
        // nome vazio não grava; com nome, grava e segue
        assert_eq!(
            d.handle(
                &Input::Key {
                    key: Key::Enter,
                    ctrl: false,
                    alt: false
                },
                &a
            ),
            Outcome::Continue
        );
        assert_eq!(
            d.handle(&Input::Text("Meu botão".into()), &a),
            Outcome::Continue
        );
        assert_eq!(
            d.handle(
                &Input::Key {
                    key: Key::Enter,
                    ctrl: false,
                    alt: false
                },
                &a
            ),
            Outcome::SaveAs("Meu botão".into(), After::New)
        );
    }

    #[test]
    fn trash_flow_and_clicks_outside() {
        let (_d, a) = empty_assets();
        let mut d = Dialog::open(ProjectKind::Comics, Step::SaveAsk, After::Leave, &a);
        assert_eq!(click_option(&mut d, &a, 1), Outcome::Continue);
        assert_eq!(d.step, Step::PrintBeforeTrash);
        assert_eq!(
            click_option(&mut d, &a, 0),
            Outcome::PrintAndDiscard(After::Leave)
        );
        assert_eq!(click_option(&mut d, &a, 1), Outcome::Discard(After::Leave));
        // fora do balão: cancela; dentro, fora das opções: nada
        assert_eq!(
            d.handle(
                &Input::Click {
                    button: MouseButton::Left,
                    x: 10,
                    y: 470
                },
                &a
            ),
            Outcome::Close
        );
        assert_eq!(
            d.handle(
                &Input::Click {
                    button: MouseButton::Left,
                    x: d.balloon.x + 3,
                    y: d.balloon.y + 3
                },
                &a
            ),
            Outcome::Continue
        );
        assert!(d.contains(&a, d.balloon.x + 3, d.balloon.y + 3));
    }
}
