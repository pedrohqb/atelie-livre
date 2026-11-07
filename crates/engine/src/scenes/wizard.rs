//! Assistente passo a passo da Maggie (Truques de Desenho): um roteiro de
//! passos, cada um com o balão do `WHAT` do original, a figura da "Área de
//! Desenho da Maggie" e a condição para avançar. Spec:
//! `docs/superpowers/specs/2026-10-31-truques-design.md`; comportamento
//! detalhado em `docs/telas/truques.md`.
//!
//! O `Wizard` não desenha nem recebe cliques sozinho: a tela que o hospeda
//! (a pintura) traduz os eventos em [`WizardEvent`] e desenha o que
//! [`Wizard::balloon`] e [`Wizard::draw_panel`] devolvem.

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
/// Bloco das saídas dos assistentes (`WHAT 3803`).
pub const QUIT_BLOCK: u16 = 0x3803;
/// `3803[0]`: "O que você quer que eu faça agora?" (Maggie fora de hora).
pub const QUIT_WIZARD: usize = 0;
/// `3803[4]`: "O que você quer fazer?" (botão Pára).
pub const QUIT_TRICKS: usize = 4;
/// Tamanho máximo do nome digitado para gravar a figura.
const NAME_MAX: usize = 40;

/// Alvo que o passo manda clicar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Botão de ferramenta da barra (índice em `TOOLS` da pintura).
    Tool(usize),
    /// Botão `k` da faixa de opções da ferramenta `tool` (índice em `TOOLS`),
    /// na página `page` (`None` para faixas sem páginas). Só conta com a
    /// faixa certa aberta; com outra, a tela destaca a ferramenta (ou a seta
    /// de página) no lugar do botão.
    Option {
        tool: usize,
        page: Option<u8>,
        k: usize,
    },
    /// Interruptor mágico da faixa das Super-ferramentas.
    Switch,
    /// Ver a página `page` da faixa da ferramenta (índice em `TOOLS`): "clique
    /// na seta até ver…"; avança assim que a página aparece.
    Page(usize, u8),
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
    /// Achar as formas (elipses inscritas nos retângulos, relativos ao
    /// painel) uma a uma; clicar na Maggie (McZee) mostra onde elas estão.
    FindShapes(Vec<Rect>),
    /// Clique na Maggie (painel ou McZee).
    Maggie,
    /// Pintar alguma coisa; Segue fica disponível desde o início.
    PaintThenNext,
    /// Escolha no balão: (opção, destino).
    Choice(Vec<(usize, Goto)>),
    /// As opções do balão são botões de rádio que trocam a figura do
    /// painel (uma por opção, na ordem da tela); Segue continua.
    Pictures(Vec<u16>),
    /// Digitar um nome e apertar Enter: a tela grava a figura com esse nome.
    TypeName,
}

/// Para onde um passo leva.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goto {
    Step(usize),
    /// Sai do assistente e deixa pintar por conta própria.
    FreePaint,
    /// Sai para uma cena.
    Scene(SceneId),
    /// Imprime a figura e vai ao passo.
    PrintThen(usize),
    /// Joga a figura fora (a tela não a grava ao sair) e vai ao passo.
    DiscardThen(usize),
    /// Abre a pasta de trabalhos para reabrir uma figura.
    Folder,
}

/// Animação do painel da Maggie ao sair de um passo (o clique no alvo):
/// uma figura do `.PIC` (com a máscara `sprite + 8`, se houver) que
/// desliza de `from` a `to` em `secs` — ou, com `wipe`, aparece de cima
/// para baixo em `to` — e fica até o passo seguinte acabar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelAnim {
    pub sprite: u16,
    pub from: (i32, i32),
    pub to: (i32, i32),
    pub wipe: bool,
    pub secs: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// Registro do balão: (bloco `WHAT`, índice).
    pub record: (u16, usize),
    /// Figura mostrada na Área de Desenho da Maggie.
    pub maggie: Option<u16>,
    /// Figura pintada no quadro do usuário ao entrar no passo ("já comecei para você").
    pub your_area: Option<u16>,
    pub advance: Advance,
    /// Animação do painel ao avançar por este passo.
    pub anim: Option<PanelAnim>,
    /// Ícones do `.PIC` do port no lugar das figuras do registro ("clique
    /// até ver [ícone]"), cujos ids do `WHAT` não existem no port.
    pub icons: &'static [u16],
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
    /// Imprimir a figura (e, se o passo mudou, `current()` já é o novo).
    Print,
    /// Gravar a figura com o nome digitado.
    Save(String),
    /// A figura vai para o lixo: não gravar ao sair.
    Discard,
    /// Abrir a pasta de trabalhos.
    OpenFolder,
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
    /// Clique num botão da faixa: ferramenta aberta, página dela e botão.
    OptionButton {
        tool: usize,
        page: u8,
        k: usize,
    },
    /// Clique no Interruptor mágico (faixa das Super-ferramentas aberta).
    Switch,
    /// Depois de um clique: a faixa aberta é da ferramenta e mostra a página.
    PageShown(usize, u8),
    /// Clique no painel da Maggie, em coordenadas relativas ao painel.
    Panel(i32, i32),
    /// Clique no McZee.
    McZee,
    /// Um traço foi pintado.
    Painted,
    /// Tecla digitada (nome da figura).
    Typed(char),
    Backspace,
    Enter,
}

#[derive(Debug, Clone)]
pub struct Wizard {
    steps: Vec<Step>,
    current: usize,
    /// Formas já achadas num passo `FindShapes`.
    found: Vec<usize>,
    /// A Maggie mostrou as formas (`FindShapes`).
    hint: bool,
    /// Já pintou num passo `PaintThenNext` (só informativo).
    painted: bool,
    /// Balão de saída (`3803[4]`) aberto por Pára.
    stopping: bool,
    /// Balão `3803[0]` aberto por um clique na Maggie fora de hora.
    quitting: bool,
    /// "Deixar você trabalhar por conta própria um pouco": balão e painel
    /// escondidos até o próximo clique no McZee.
    paused: bool,
    /// Botão de rádio escolhido num passo `Pictures`.
    picture: Option<usize>,
    /// Nome digitado num passo `TypeName`.
    typed: String,
    /// Animação do painel em curso e o tempo decorrido.
    anim: Option<(PanelAnim, f32)>,
    /// Passos visitados, para o Volta.
    history: Vec<usize>,
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

/// `(x, y)` dentro da elipse inscrita em `r`.
fn in_ellipse(r: Rect, x: i32, y: i32) -> bool {
    let (rx, ry) = (r.2 as f32 / 2.0, r.3 as f32 / 2.0);
    if rx <= 0.0 || ry <= 0.0 {
        return false;
    }
    let (dx, dy) = (x as f32 - (r.0 as f32 + rx), y as f32 - (r.1 as f32 + ry));
    (dx / rx).powi(2) + (dy / ry).powi(2) <= 1.0
}

impl Wizard {
    pub fn new(steps: Vec<Step>) -> Wizard {
        Wizard {
            steps,
            current: 0,
            found: Vec::new(),
            hint: false,
            painted: false,
            stopping: false,
            quitting: false,
            paused: false,
            picture: None,
            typed: String::new(),
            anim: None,
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

    /// O balão `3803[0]` está aberto.
    pub fn quitting(&self) -> bool {
        self.quitting
    }

    /// Trabalhando por conta própria: sem balão nem painel até clicar no McZee.
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Nome digitado até agora (passo `TypeName`).
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// Formas já achadas e se a Maggie as mostrou (passo `FindShapes`).
    pub fn shapes_found(&self) -> (&[usize], bool) {
        (&self.found, self.hint)
    }

    /// O passo atual avança por Segue (mostra o botão).
    pub fn has_next(&self) -> bool {
        matches!(
            self.step().advance,
            Advance::Next | Advance::PaintThenNext | Advance::Pictures(_)
        )
    }

    pub fn has_back(&self) -> bool {
        !self.history.is_empty()
    }

    /// O passo atual espera um nome digitado.
    pub fn typing(&self) -> bool {
        !self.paused && !self.stopping && !self.quitting && self.step().advance == Advance::TypeName
    }

    /// Alvo a destacar, se o passo pede um clique.
    pub fn target(&self) -> Option<Target> {
        if self.paused || self.stopping || self.quitting {
            return None;
        }
        match self.step().advance {
            Advance::Click(t) => Some(t),
            Advance::FindShapes(_) => Some(Target::Panel),
            _ => None,
        }
    }

    /// Balão do passo (ou o de saída), com os textos do `WHAT` quando há
    /// dados; nenhum enquanto o usuário trabalha por conta própria.
    pub fn balloon(&self, assets: &Assets, fallback: &str) -> Option<Balloon> {
        if self.paused {
            return None;
        }
        let (block, k) = if self.stopping {
            (QUIT_BLOCK, QUIT_TRICKS)
        } else if self.quitting {
            (QUIT_BLOCK, QUIT_WIZARD)
        } else {
            self.step().record
        };
        let dialogs = assets.dialogs(Program::Artist, block);
        let mut b = match Balloon::from_block(&dialogs, k, BALLOON_POS.0, BALLOON_POS.1, "artista")
        {
            Some(b) => b,
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
                } else if self.quitting {
                    vec![
                        "Deixar você trabalhar por conta própria um pouco.".into(),
                        "Parar de trabalhar aqui".into(),
                        "Ignorar isso — vou ficar aqui!".into(),
                    ]
                } else {
                    Vec::new()
                },
                links: Vec::new(),
                figures: Vec::new(),
            },
        };
        if !self.stopping && !self.quitting && !self.step().icons.is_empty() {
            b.figures = self.step().icons.to_vec();
        }
        if self.typing() {
            b.text.push_str("\n\n> ");
            b.text.push_str(&self.typed);
            b.text.push('_');
        }
        Some(b)
    }

    /// Figura do painel da Maggie no passo atual.
    pub fn maggie_panel(&self) -> Option<u16> {
        if self.stopping || self.quitting || self.paused {
            return None;
        }
        if let (Advance::Pictures(pics), Some(k)) = (&self.step().advance, self.picture)
            && let Some(id) = pics.get(k)
        {
            return Some(*id);
        }
        self.step().maggie
    }

    /// Avança o relógio das animações do painel.
    pub fn tick(&mut self, dt: f32) {
        if let Some((_, t)) = self.anim.as_mut()
            && dt.is_finite()
        {
            *t += dt.max(0.0);
        }
    }

    fn enter(&mut self, s: usize) {
        self.current = s;
        self.found.clear();
        self.hint = false;
        self.painted = false;
        self.picture = None;
        self.typed.clear();
    }

    fn goto(&mut self, g: Goto) -> WizardOutcome {
        match g {
            Goto::Step(s) if s < self.steps.len() => {
                self.anim = self.steps[self.current].anim.map(|a| (a, 0.0));
                self.history.push(self.current);
                self.enter(s);
                WizardOutcome::Entered(s)
            }
            Goto::Step(_) => WizardOutcome::Stay,
            Goto::FreePaint => WizardOutcome::FreePaint,
            Goto::Scene(id) => WizardOutcome::Leave(id),
            Goto::PrintThen(s) => {
                self.goto(Goto::Step(s));
                WizardOutcome::Print
            }
            Goto::DiscardThen(s) => {
                self.goto(Goto::Step(s));
                WizardOutcome::Discard
            }
            Goto::Folder => WizardOutcome::OpenFolder,
        }
    }

    fn next(&mut self) -> WizardOutcome {
        self.goto(Goto::Step(self.current + 1))
    }

    /// O passo atual não espera a Maggie: clicar nela (ou no McZee) abre
    /// o balão `3803[0]`.
    fn maggie_out_of_turn(&self) -> bool {
        !matches!(
            self.step().advance,
            Advance::Maggie
                | Advance::FindShapes(_)
                | Advance::Click(Target::Panel | Target::PanelArea(_))
        )
    }

    pub fn on_event(&mut self, ev: WizardEvent) -> WizardOutcome {
        if self.paused {
            if ev == WizardEvent::McZee {
                self.paused = false;
                return WizardOutcome::Entered(self.current);
            }
            return WizardOutcome::Stay;
        }
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
        if self.quitting {
            return match ev {
                // "Deixar você trabalhar por conta própria um pouco"
                WizardEvent::Option(0) => {
                    self.quitting = false;
                    self.paused = true;
                    WizardOutcome::Stay
                }
                // "Parar de trabalhar aqui": volta ao andar
                WizardEvent::Option(1) => {
                    self.quitting = false;
                    WizardOutcome::Leave(SceneId::TricksFloor)
                }
                WizardEvent::Option(_) | WizardEvent::Stop | WizardEvent::McZee => {
                    self.quitting = false;
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
                        self.anim = None;
                        self.enter(s);
                        WizardOutcome::Entered(s)
                    }
                    None => WizardOutcome::Stay,
                };
            }
            WizardEvent::McZee | WizardEvent::Panel(..) if self.maggie_out_of_turn() => {
                self.quitting = true;
                return WizardOutcome::Stay;
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
            (Advance::Pictures(pics), WizardEvent::Option(k)) if k < pics.len() => {
                self.picture = Some(k);
                WizardOutcome::Stay
            }
            (Advance::Pictures(_), WizardEvent::Next) => self.next(),
            (Advance::Click(Target::Tool(t)), WizardEvent::Tool(k)) if t == k => self.next(),
            (
                Advance::Click(Target::Option { tool, page, k }),
                WizardEvent::OptionButton {
                    tool: t,
                    page: p,
                    k: b,
                },
            ) if tool == t && page.is_none_or(|pg| pg == p) && k == b => self.next(),
            (Advance::Click(Target::Switch), WizardEvent::Switch) => self.next(),
            (Advance::Click(Target::Page(t, p)), WizardEvent::PageShown(tool, page))
                if t == tool && p == page =>
            {
                self.next()
            }
            (Advance::Click(Target::Panel), WizardEvent::Panel(..)) => self.next(),
            (Advance::Click(Target::PanelArea(r)), WizardEvent::Panel(x, y)) if inside(r, x, y) => {
                self.next()
            }
            (Advance::FindShapes(shapes), WizardEvent::Panel(x, y)) => {
                let hit = shapes
                    .iter()
                    .position(|&r| in_ellipse(r, x, y))
                    .filter(|k| !self.found.contains(k));
                match hit {
                    Some(k) => {
                        self.found.push(k);
                        if self.found.len() >= shapes.len() {
                            self.next()
                        } else {
                            WizardOutcome::Stay
                        }
                    }
                    None => WizardOutcome::Stay,
                }
            }
            // "Se você ficar encalacrado, clique em mim (Maggie)": mostra as formas
            (Advance::FindShapes(_), WizardEvent::McZee) => {
                self.hint = true;
                WizardOutcome::Stay
            }
            (Advance::Maggie, WizardEvent::McZee | WizardEvent::Panel(..)) => self.next(),
            (Advance::Choice(opts), WizardEvent::Option(k)) => {
                match opts.iter().find(|(o, _)| *o == k) {
                    Some((_, g)) => self.goto(*g),
                    None => WizardOutcome::Stay,
                }
            }
            (Advance::TypeName, WizardEvent::Typed(c)) => {
                if self.typed.chars().count() < NAME_MAX && !c.is_control() {
                    self.typed.push(c);
                }
                WizardOutcome::Stay
            }
            (Advance::TypeName, WizardEvent::Backspace) => {
                self.typed.pop();
                WizardOutcome::Stay
            }
            (Advance::TypeName, WizardEvent::Enter) => {
                let name = self.typed.trim().to_string();
                if name.is_empty() {
                    return WizardOutcome::Stay;
                }
                self.next();
                WizardOutcome::Save(name)
            }
            _ => WizardOutcome::Stay,
        }
    }

    /// Desenha o painel da Maggie (moldura, figura, animação e as formas
    /// reveladas), se o passo tem figura. `blink` alterna o destaque.
    pub fn draw_panel(&self, f: &mut Frame, a: &Assets, blink: bool) {
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
        if let Some((an, t)) = &self.anim {
            let k = if an.secs > 0.0 {
                (t / an.secs).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let sprite = ImageRef::Pic(Program::Artist, an.sprite);
            if a.has_image(&sprite) {
                let img = a.image(&sprite);
                let mask = ImageRef::Pic(Program::Artist, an.sprite + 8);
                let (sx, sy) = if an.wipe {
                    (ox + an.to.0, oy + an.to.1)
                } else {
                    (
                        ox + an.from.0 + ((an.to.0 - an.from.0) as f32 * k) as i32,
                        oy + an.from.1 + ((an.to.1 - an.from.1) as f32 * k) as i32,
                    )
                };
                let rows = if an.wipe {
                    (img.height as f32 * k) as u32
                } else {
                    img.height
                };
                let part = crate::Image {
                    width: img.width,
                    height: rows,
                    rgba: img.rgba[..(rows * img.width * 4) as usize].to_vec(),
                };
                if a.has_image(&mask) {
                    let m = a.image(&mask);
                    let mpart = crate::Image {
                        width: m.width,
                        height: rows.min(m.height),
                        rgba: m.rgba[..(rows.min(m.height) * m.width * 4) as usize].to_vec(),
                    };
                    f.blit_masked(&part, &mpart, sx, sy);
                } else {
                    f.blit(&part, sx, sy);
                }
            }
        }
        if let Advance::FindShapes(shapes) = &self.step().advance {
            for (k, r) in shapes.iter().enumerate() {
                let done = self.found.contains(&k);
                if !done && !self.hint {
                    continue;
                }
                let color = if done {
                    [0, 160, 0]
                } else if blink {
                    [255, 238, 76]
                } else {
                    [0, 0, 0]
                };
                draw_ellipse(f, (x + r.0, y + r.1, r.2, r.3), color);
            }
        }
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

/// Contorno (2 px) da elipse inscrita em `r`.
fn draw_ellipse(f: &mut Frame, r: Rect, color: [u8; 3]) {
    let (rx, ry) = (r.2 as f32 / 2.0, r.3 as f32 / 2.0);
    let (cx, cy) = (r.0 as f32 + rx, r.1 as f32 + ry);
    let n = ((rx + ry) * 4.0) as usize;
    for i in 0..n {
        let t = i as f32 / n as f32 * std::f32::consts::TAU;
        let (px, py) = (cx + rx * t.cos(), cy + ry * t.sin());
        f.rect(px as i32, py as i32, 2, 2, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(record: usize, maggie: Option<u16>, advance: Advance) -> Step {
        Step {
            record: (0x3800, record),
            maggie,
            your_area: None,
            advance,
            anim: None,
            icons: &[],
        }
    }

    fn steps() -> Vec<Step> {
        vec![
            s(
                0,
                None,
                Advance::Choice(vec![
                    (0, Goto::Step(1)),
                    (1, Goto::FreePaint),
                    (2, Goto::Scene(SceneId::Building)),
                ]),
            ),
            s(3, None, Advance::Next),
            s(
                6,
                Some(0x3827),
                Advance::Click(Target::PanelArea((50, 50, 200, 200))),
            ),
            s(
                9,
                Some(0x3838),
                Advance::FindShapes(vec![(0, 0, 40, 40), (100, 100, 40, 40), (200, 200, 40, 40)]),
            ),
            s(14, None, Advance::Click(Target::Tool(8))),
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
        // as formas: fora de todas e repetida não contam
        assert_eq!(w.on_event(WizardEvent::Panel(60, 60)), WizardOutcome::Stay);
        assert_eq!(w.on_event(WizardEvent::Panel(20, 20)), WizardOutcome::Stay);
        assert_eq!(w.on_event(WizardEvent::Panel(20, 20)), WizardOutcome::Stay);
        assert_eq!(w.shapes_found().0, &[0]);
        assert_eq!(
            w.on_event(WizardEvent::Panel(120, 120)),
            WizardOutcome::Stay
        );
        assert_eq!(
            w.on_event(WizardEvent::Panel(220, 220)),
            WizardOutcome::Entered(4)
        );
        assert_eq!(w.target(), Some(Target::Tool(8)));
        assert_eq!(w.on_event(WizardEvent::Tool(2)), WizardOutcome::Stay);
        // último passo: a ferramenta certa não tem para onde ir
        assert_eq!(w.on_event(WizardEvent::Tool(8)), WizardOutcome::Stay);
    }

    #[test]
    fn maggie_reveals_the_shapes() {
        let mut w = Wizard::new(steps());
        w.on_event(WizardEvent::Option(0));
        w.on_event(WizardEvent::Next);
        w.on_event(WizardEvent::Panel(100, 100));
        assert!(!w.shapes_found().1);
        assert_eq!(w.on_event(WizardEvent::McZee), WizardOutcome::Stay);
        assert!(w.shapes_found().1);
        assert!(!w.quitting());
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
    fn maggie_out_of_turn_opens_quit_wizard_and_pauses() {
        let mut w = Wizard::new(steps());
        w.on_event(WizardEvent::Option(0));
        // passo 1 (Segue): o McZee abre `3803[0]`
        assert_eq!(w.on_event(WizardEvent::McZee), WizardOutcome::Stay);
        assert!(w.quitting());
        assert!(w.target().is_none());
        // "Ignorar isso" fecha
        assert_eq!(w.on_event(WizardEvent::Option(2)), WizardOutcome::Stay);
        assert!(!w.quitting());
        // "Deixar você trabalhar por conta própria": pausa até o McZee
        w.on_event(WizardEvent::McZee);
        assert_eq!(w.on_event(WizardEvent::Option(0)), WizardOutcome::Stay);
        assert!(w.paused());
        assert!(!w.has_next() || w.target().is_none());
        assert_eq!(w.on_event(WizardEvent::Next), WizardOutcome::Stay);
        assert_eq!(w.on_event(WizardEvent::McZee), WizardOutcome::Entered(1));
        assert!(!w.paused());
        // "Parar de trabalhar aqui" volta ao andar
        w.on_event(WizardEvent::McZee);
        assert_eq!(
            w.on_event(WizardEvent::Option(1)),
            WizardOutcome::Leave(SceneId::TricksFloor)
        );
    }

    #[test]
    fn option_button_checks_tool_and_page() {
        let mut w = Wizard::new(vec![
            s(
                66,
                None,
                Advance::Click(Target::Option {
                    tool: 0,
                    page: Some(5),
                    k: 0,
                }),
            ),
            s(
                52,
                None,
                Advance::Click(Target::Option {
                    tool: 8,
                    page: None,
                    k: 5,
                }),
            ),
            s(55, None, Advance::Next),
        ]);
        let ev = |tool, page, k| WizardEvent::OptionButton { tool, page, k };
        assert_eq!(w.on_event(ev(0, 1, 0)), WizardOutcome::Stay);
        assert_eq!(w.on_event(ev(2, 5, 0)), WizardOutcome::Stay);
        assert_eq!(w.on_event(ev(0, 5, 0)), WizardOutcome::Entered(1));
        assert_eq!(w.on_event(ev(2, 1, 5)), WizardOutcome::Stay);
        assert_eq!(w.on_event(ev(8, 3, 5)), WizardOutcome::Entered(2));
    }

    #[test]
    fn pictures_switch_the_panel_and_next_continues() {
        let mut w = Wizard::new(vec![
            s(4, Some(0x4008), Advance::Pictures(vec![0x3e0b, 0x4008])),
            s(6, None, Advance::Next),
        ]);
        assert!(w.has_next());
        assert_eq!(w.maggie_panel(), Some(0x4008));
        assert_eq!(w.on_event(WizardEvent::Option(0)), WizardOutcome::Stay);
        assert_eq!(w.maggie_panel(), Some(0x3e0b));
        assert_eq!(w.on_event(WizardEvent::Option(1)), WizardOutcome::Stay);
        assert_eq!(w.maggie_panel(), Some(0x4008));
        assert_eq!(w.on_event(WizardEvent::Option(7)), WizardOutcome::Stay);
        assert_eq!(w.on_event(WizardEvent::Next), WizardOutcome::Entered(1));
    }

    #[test]
    fn type_name_saves_and_choices_print_discard_and_open_folder() {
        let mut w = Wizard::new(vec![
            s(
                24,
                None,
                Advance::Choice(vec![(0, Goto::Step(1)), (1, Goto::DiscardThen(2))]),
            ),
            s(25, None, Advance::TypeName),
            s(
                27,
                None,
                Advance::Choice(vec![(0, Goto::PrintThen(3)), (1, Goto::Step(3))]),
            ),
            s(29, None, Advance::Choice(vec![(0, Goto::Folder)])),
        ]);
        w.on_event(WizardEvent::Option(0));
        assert!(w.typing());
        assert_eq!(w.on_event(WizardEvent::Enter), WizardOutcome::Stay);
        for c in "meu porco".chars() {
            w.on_event(WizardEvent::Typed(c));
        }
        w.on_event(WizardEvent::Backspace);
        assert_eq!(w.typed(), "meu porc");
        assert_eq!(
            w.on_event(WizardEvent::Enter),
            WizardOutcome::Save("meu porc".into())
        );
        assert_eq!(w.current(), 2);
        assert_eq!(w.on_event(WizardEvent::Option(0)), WizardOutcome::Print);
        assert_eq!(w.current(), 3);
        assert_eq!(
            w.on_event(WizardEvent::Option(0)),
            WizardOutcome::OpenFolder
        );
        let mut w2 = Wizard::new(steps_discard());
        assert_eq!(w2.on_event(WizardEvent::Option(1)), WizardOutcome::Discard);
        assert_eq!(w2.current(), 2);
    }

    fn steps_discard() -> Vec<Step> {
        vec![
            s(
                24,
                None,
                Advance::Choice(vec![(0, Goto::Step(1)), (1, Goto::DiscardThen(2))]),
            ),
            s(25, None, Advance::TypeName),
            s(29, None, Advance::Next),
        ]
    }

    #[test]
    fn anim_starts_on_the_click_and_stops_on_the_next_step() {
        let an = PanelAnim {
            sprite: 0x4069,
            from: (27, 218),
            to: (0, 160),
            wipe: false,
            secs: 1.0,
        };
        let mut w = Wizard::new(vec![
            Step {
                anim: Some(an),
                ..s(29, Some(0x4083), Advance::Click(Target::Panel))
            },
            s(36, Some(0x4083), Advance::Click(Target::Panel)),
            s(42, Some(0x4083), Advance::Next),
        ]);
        assert!(w.anim.is_none());
        w.on_event(WizardEvent::Panel(10, 10));
        assert_eq!(w.anim.map(|a| a.0), Some(an));
        w.tick(0.5);
        assert_eq!(w.anim.map(|a| a.1), Some(0.5));
        w.on_event(WizardEvent::Panel(10, 10));
        assert!(w.anim.is_none());
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
