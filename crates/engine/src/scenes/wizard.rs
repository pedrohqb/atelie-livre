//! Assistente passo a passo da Maggie (Truques de Desenho): um roteiro de
//! passos, cada um com o balão do `WHAT` do original, a figura da "Área de
//! Desenho da Maggie" e a condição para avançar. Spec:
//! `docs/superpowers/specs/2026-10-31-truques-design.md`.
//!
//! O `Wizard` não desenha nem recebe cliques sozinho: a tela que o hospeda
//! (a pintura) traduz os eventos em [`WizardEvent`] e desenha o que
//! [`Wizard::balloon`] e [`Wizard::maggie_panel`] devolvem.

use crate::Frame;
use crate::assets::{Assets, ImageRef, Program};
use crate::balloon::Balloon;
use crate::scene::SceneId;

/// Qual lição dos Truques de Desenho.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrickKind {
    /// Truques básicos (o porco Chiquinho).
    Basic,
    /// Truques de 3-D (a mesa do café da manhã).
    ThreeD,
}

impl TrickKind {
    /// Texto da barra inferior (`STR# 3800`: "Estamos vendo os Truques Básicos").
    pub fn label(self) -> &'static str {
        match self {
            TrickKind::Basic => "Estamos vendo os Truques Básicos",
            TrickKind::ThreeD => "Estamos vendo os Truques de 3-D",
        }
    }
}

/// Retângulo (x, y, w, h) em coordenadas da tela.
pub type Rect = (i32, i32, i32, i32);

/// Painel "Área de Desenho da Maggie": 313×272 no canto inferior direito do quadro.
pub const MAGGIE_PANEL: Rect = (300, 180, 313, 272);
/// Posição padrão do balão do passo.
pub const BALLOON_POS: (i32, i32) = (30, 115);
/// Botões Volta / Pára / Segue (31×14) do `ARTIST.PIC`.
pub const BTN_BACK: (u16, u16) = (0x040f, 0x0410);
pub const BTN_STOP: (u16, u16) = (0x0417, 0x0418);
pub const BTN_NEXT: (u16, u16) = (0x0413, 0x0414);

/// Alvo que o passo manda clicar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Botão de ferramenta da barra (índice em `TOOLS` da pintura).
    Tool(usize),
    /// Botão `k` da faixa de opções.
    Option(usize),
    /// Qualquer ponto dentro do painel da Maggie.
    Panel,
    /// Um retângulo do painel da Maggie, relativo ao painel.
    PanelArea(Rect),
}

/// Como o passo avança.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Advance {
    /// Botão Segue.
    Next,
    /// Clique no alvo.
    Click(Target),
    /// `n` cliques no painel (as "3 formas circulares"), depois Maggie.
    ClicksThenMaggie(u32),
    /// Clique na Maggie (painel ou McZee).
    Maggie,
    /// Pintar alguma coisa; Segue fica disponível desde o início.
    PaintThenNext,
    /// Escolha no balão: (opção, destino).
    Choice(Vec<(usize, Goto)>),
}

/// Para onde um passo leva.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goto {
    Step(usize),
    /// Sai do assistente e deixa pintar por conta própria.
    FreePaint,
    /// Sai para uma cena.
    Scene(SceneId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Registro do balão: (bloco `WHAT`, índice).
    pub record: (u16, usize),
    /// Figura mostrada na Área de Desenho da Maggie.
    pub maggie: Option<u16>,
    /// Figura pintada no quadro do usuário ao entrar no passo ("já comecei para você").
    pub your_area: Option<u16>,
    pub advance: Advance,
}

/// O que o assistente pede à tela ao processar um evento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardOutcome {
    /// Nada muda.
    Stay,
    /// Entrou num passo novo (a tela redesenha o balão/painel; pinta `your_area` se houver).
    Entered(usize),
    FreePaint,
    Leave(SceneId),
}

/// Evento traduzido pela tela hospedeira.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardEvent {
    Next,
    Back,
    Stop,
    /// Clique numa opção do balão.
    Option(usize),
    /// Clique numa ferramenta da barra (índice em `TOOLS`).
    Tool(usize),
    /// Clique num botão da faixa.
    OptionButton(usize),
    /// Clique no painel da Maggie, em coordenadas relativas ao painel.
    Panel(i32, i32),
    /// Clique no McZee.
    McZee,
    /// Um traço foi pintado.
    Painted,
}

#[derive(Debug, Clone)]
pub struct Wizard {
    steps: Vec<Step>,
    current: usize,
    /// Cliques já dados num passo `ClicksThenMaggie`.
    clicks: u32,
    /// Já pintou num passo `PaintThenNext` (só informativo).
    painted: bool,
    /// Balão de saída (`3803[4]`) aberto por Pára.
    stopping: bool,
    /// Passos visitados, para o Volta.
    history: Vec<usize>,
}

impl Wizard {
    pub fn new(steps: Vec<Step>) -> Wizard {
        Wizard {
            steps,
            current: 0,
            clicks: 0,
            painted: false,
            stopping: false,
            history: Vec::new(),
        }
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn step(&self) -> &Step {
        &self.steps[self.current]
    }

    pub fn stopping(&self) -> bool {
        self.stopping
    }

    /// O passo atual avança por Segue (mostra o botão).
    pub fn has_next(&self) -> bool {
        matches!(self.step().advance, Advance::Next | Advance::PaintThenNext)
    }

    pub fn has_back(&self) -> bool {
        !self.history.is_empty()
    }

    /// Alvo a destacar, se o passo pede um clique.
    pub fn target(&self) -> Option<Target> {
        match self.step().advance {
            Advance::Click(t) => Some(t),
            Advance::ClicksThenMaggie(_) => Some(Target::Panel),
            _ => None,
        }
    }

    /// Balão do passo (ou o de saída), com os textos do `WHAT` quando há dados.
    pub fn balloon(&self, assets: &Assets, fallback: &str) -> Balloon {
        let (block, k) = if self.stopping {
            (0x3803, 4)
        } else {
            self.step().record
        };
        let dialogs = assets.dialogs(Program::Artist, block);
        match dialogs.get(k) {
            Some(r) => Balloon::from_record(r, BALLOON_POS.0, BALLOON_POS.1, "artista"),
            None => Balloon {
                x: BALLOON_POS.0,
                y: BALLOON_POS.1,
                w: 240,
                text: fallback.to_string(),
                options: if self.stopping {
                    vec![
                        "Parar de trabalhar aqui".into(),
                        "Ignore isso, vou ficar!".into(),
                    ]
                } else {
                    Vec::new()
                },
            },
        }
    }

    /// Figura do painel da Maggie no passo atual.
    pub fn maggie_panel(&self) -> Option<u16> {
        if self.stopping {
            return None;
        }
        self.step().maggie
    }

    fn goto(&mut self, g: Goto) -> WizardOutcome {
        match g {
            Goto::Step(s) if s < self.steps.len() => {
                self.history.push(self.current);
                self.current = s;
                self.clicks = 0;
                self.painted = false;
                WizardOutcome::Entered(s)
            }
            Goto::Step(_) => WizardOutcome::Stay,
            Goto::FreePaint => WizardOutcome::FreePaint,
            Goto::Scene(id) => WizardOutcome::Leave(id),
        }
    }

    fn next(&mut self) -> WizardOutcome {
        self.goto(Goto::Step(self.current + 1))
    }

    pub fn on_event(&mut self, ev: WizardEvent) -> WizardOutcome {
        if self.stopping {
            return match ev {
                // `3803[4]`: a primeira opção na tela é "parar" (índice 1 do original)
                WizardEvent::Option(0) => {
                    self.stopping = false;
                    WizardOutcome::FreePaint
                }
                WizardEvent::Option(_) | WizardEvent::Stop | WizardEvent::McZee => {
                    self.stopping = false;
                    WizardOutcome::Stay
                }
                _ => WizardOutcome::Stay,
            };
        }
        match ev {
            WizardEvent::Stop => {
                self.stopping = true;
                return WizardOutcome::Stay;
            }
            WizardEvent::Back => {
                return match self.history.pop() {
                    Some(s) => {
                        self.current = s;
                        self.clicks = 0;
                        self.painted = false;
                        WizardOutcome::Entered(s)
                    }
                    None => WizardOutcome::Stay,
                };
            }
            _ => {}
        }
        let advance = self.step().advance.clone();
        match (advance, ev) {
            (Advance::Next, WizardEvent::Next) => self.next(),
            (Advance::PaintThenNext, WizardEvent::Painted) => {
                self.painted = true;
                WizardOutcome::Stay
            }
            (Advance::PaintThenNext, WizardEvent::Next) => self.next(),
            (Advance::Click(Target::Tool(t)), WizardEvent::Tool(k)) if t == k => self.next(),
            (Advance::Click(Target::Option(o)), WizardEvent::OptionButton(k)) if o == k => {
                self.next()
            }
            (Advance::Click(Target::Panel), WizardEvent::Panel(..)) => self.next(),
            (Advance::Click(Target::PanelArea(r)), WizardEvent::Panel(x, y))
                if x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3 =>
            {
                self.next()
            }
            (Advance::ClicksThenMaggie(n), WizardEvent::Panel(..)) => {
                self.clicks += 1;
                if self.clicks >= n {
                    self.next()
                } else {
                    WizardOutcome::Stay
                }
            }
            (Advance::ClicksThenMaggie(n), WizardEvent::McZee) if self.clicks >= n => self.next(),
            (Advance::Maggie, WizardEvent::McZee | WizardEvent::Panel(..)) => self.next(),
            (Advance::Choice(opts), WizardEvent::Option(k)) => {
                match opts.iter().find(|(o, _)| *o == k) {
                    Some((_, g)) => self.goto(*g),
                    None => WizardOutcome::Stay,
                }
            }
            _ => WizardOutcome::Stay,
        }
    }

    /// Desenha o painel da Maggie (moldura e figura), se o passo tem figura.
    pub fn draw_panel(&self, f: &mut Frame, a: &Assets) {
        let Some(id) = self.maggie_panel() else {
            return;
        };
        let (x, y, w, h) = MAGGIE_PANEL;
        f.rect(x - 2, y - 2, w + 4, h + 4, [0, 0, 0]);
        f.rect(x, y, w, h, [255, 255, 255]);
        let img = a.image(&ImageRef::Pic(Program::Artist, id));
        let ox = x + (w - img.width as i32) / 2;
        let oy = y + (h - img.height as i32) / 2;
        f.blit(&img, ox, oy);
    }

    /// Clique em (x, y) da tela traduzido para o painel, se cair nele.
    pub fn panel_hit(&self, x: i32, y: i32) -> Option<WizardEvent> {
        let (px, py, w, h) = MAGGIE_PANEL;
        if self.maggie_panel().is_some() && x >= px && y >= py && x < px + w && y < py + h {
            Some(WizardEvent::Panel(x - px, y - py))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps() -> Vec<Step> {
        vec![
            Step {
                record: (0x3800, 0),
                maggie: None,
                your_area: None,
                advance: Advance::Choice(vec![
                    (0, Goto::Step(1)),
                    (1, Goto::FreePaint),
                    (2, Goto::Scene(SceneId::Building)),
                ]),
            },
            Step {
                record: (0x3800, 3),
                maggie: None,
                your_area: None,
                advance: Advance::Next,
            },
            Step {
                record: (0x3800, 6),
                maggie: Some(0x3827),
                your_area: None,
                advance: Advance::Click(Target::PanelArea((50, 50, 200, 200))),
            },
            Step {
                record: (0x3800, 9),
                maggie: Some(0x3838),
                your_area: None,
                advance: Advance::ClicksThenMaggie(3),
            },
            Step {
                record: (0x3800, 14),
                maggie: None,
                your_area: None,
                advance: Advance::Click(Target::Tool(8)),
            },
        ]
    }

    #[test]
    fn choice_next_and_targets_advance_in_order() {
        let mut w = Wizard::new(steps());
        assert_eq!(w.on_event(WizardEvent::Next), WizardOutcome::Stay);
        assert_eq!(
            w.on_event(WizardEvent::Option(0)),
            WizardOutcome::Entered(1)
        );
        assert!(w.has_next());
        assert_eq!(w.on_event(WizardEvent::Next), WizardOutcome::Entered(2));
        assert_eq!(w.maggie_panel(), Some(0x3827));
        // fora da área: não avança; dentro: avança
        assert_eq!(w.on_event(WizardEvent::Panel(5, 5)), WizardOutcome::Stay);
        assert_eq!(
            w.on_event(WizardEvent::Panel(100, 100)),
            WizardOutcome::Entered(3)
        );
        for _ in 0..2 {
            assert_eq!(w.on_event(WizardEvent::Panel(10, 10)), WizardOutcome::Stay);
        }
        assert_eq!(
            w.on_event(WizardEvent::Panel(10, 10)),
            WizardOutcome::Entered(4)
        );
        assert_eq!(w.target(), Some(Target::Tool(8)));
        assert_eq!(w.on_event(WizardEvent::Tool(2)), WizardOutcome::Stay);
        // último passo: a ferramenta certa não tem para onde ir
        assert_eq!(w.on_event(WizardEvent::Tool(8)), WizardOutcome::Stay);
    }

    #[test]
    fn choice_can_leave_and_back_returns() {
        let mut w = Wizard::new(steps());
        assert_eq!(
            w.on_event(WizardEvent::Option(2)),
            WizardOutcome::Leave(SceneId::Building)
        );
        assert_eq!(w.on_event(WizardEvent::Option(1)), WizardOutcome::FreePaint);
        w.on_event(WizardEvent::Option(0));
        assert!(w.has_back());
        assert_eq!(w.on_event(WizardEvent::Back), WizardOutcome::Entered(0));
        assert!(!w.has_back());
    }

    #[test]
    fn stop_opens_exit_balloon() {
        let mut w = Wizard::new(steps());
        w.on_event(WizardEvent::Stop);
        assert!(w.stopping());
        assert!(w.maggie_panel().is_none());
        assert_eq!(w.on_event(WizardEvent::Option(1)), WizardOutcome::Stay);
        assert!(!w.stopping());
        w.on_event(WizardEvent::Stop);
        assert_eq!(w.on_event(WizardEvent::Option(0)), WizardOutcome::FreePaint);
    }

    #[test]
    fn panel_hit_only_with_picture() {
        let mut w = Wizard::new(steps());
        assert!(w.panel_hit(400, 300).is_none());
        w.on_event(WizardEvent::Option(0));
        w.on_event(WizardEvent::Next);
        assert_eq!(w.panel_hit(400, 300), Some(WizardEvent::Panel(100, 120)));
        assert!(w.panel_hit(10, 10).is_none());
    }
}
