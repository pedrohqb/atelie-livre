//! Núcleo comum dos assistentes passo a passo: a lição da Maggie nos
//! Truques do Fine Artist (`scenes::wizard`) e os projetos do Max no
//! Creative Writer (`writer::assistant`).
//!
//! Os dois têm o mesmo esqueleto, e ele mora aqui, sem tela e sem dados:
//!
//! - o passo atual e o histórico para o botão **Volta** ([`Guide::forward`],
//!   [`Guide::back`], [`Guide::restart`]);
//! - os balões de saída do bloco `WHAT 3803` ([`Ask`]): abertos pelo **Pára**
//!   ou por um clique no personagem fora de hora, respondidos com
//!   [`Guide::answer`] segundo uma tabela de [`Answer`]s por opção;
//! - "Deixar você trabalhar por conta própria um pouco" ([`Answer::Pause`]):
//!   o assistente some até o próximo clique no personagem ([`Guide::resume`]).
//!
//! O que cada assistente espera em cada passo (ferramentas, painel da
//! Maggie, campos, rádios) fica no módulo dele.

/// Bloco dos balões de saída dos assistentes (`WHAT 3803`, nos dois
/// executáveis).
pub const QUIT_BLOCK: u16 = 0x3803;

/// Qual balão de saída está aberto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// Aberto pelo botão Pára.
    Stop,
    /// Aberto por um clique no personagem (Maggie/McZee, Max) num passo que
    /// não o espera.
    Quit,
}

/// O que uma resposta a um balão de saída faz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Fecha o balão e continua no mesmo passo ("Ignorar isso").
    Close,
    /// Esconde o assistente até o próximo clique no personagem ("Deixar
    /// você trabalhar por conta própria um pouco").
    Pause,
    /// Sai da tela de volta ao andar ("Parar de trabalhar aqui — voltar…").
    Leave,
    /// Encerra o assistente e fica na tela ("Parar de trabalhar aqui" do
    /// Pára da lição do Fine Artist).
    Finish,
}

/// Respostas de um balão de saída, na ordem dos índices das opções do
/// original.
pub type Answers = &'static [Answer];

/// `3803[0]` "O que você quer que eu faça agora?": trabalhar por conta
/// própria / parar e voltar ao andar / ignorar. É o balão do Pára no
/// Writer e o do clique fora de hora nos dois programas.
pub const QUIT_ANSWERS: Answers = &[Answer::Pause, Answer::Leave, Answer::Close];
/// `3803[4]` do Fine Artist (Pára da lição): parar de trabalhar aqui /
/// ignorar.
pub const STOP_ANSWERS: Answers = &[Answer::Finish, Answer::Close];

/// Passo atual, histórico e balões de saída de um assistente.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Guide {
    current: usize,
    history: Vec<usize>,
    asking: Option<Ask>,
    paused: bool,
}

impl Guide {
    /// Começa no passo `start`, sem histórico.
    pub fn new(start: usize) -> Guide {
        Guide {
            current: start,
            ..Guide::default()
        }
    }

    pub fn current(&self) -> usize {
        self.current
    }

    /// Há para onde o Volta voltar.
    pub fn has_back(&self) -> bool {
        !self.history.is_empty()
    }

    /// Vai ao passo `s`, lembrando o atual para o Volta.
    pub fn forward(&mut self, s: usize) {
        self.history.push(self.current);
        self.current = s;
    }

    /// Troca o passo atual sem mexer no histórico (passo que se pula, ou o
    /// início de uma lição a partir de um ponto).
    pub fn replace(&mut self, s: usize) {
        self.current = s;
    }

    /// Recomeça no passo `s` e esquece o histórico ("Começar um novo…").
    pub fn restart(&mut self, s: usize) {
        self.history.clear();
        self.current = s;
    }

    /// Volta ao passo anterior, se houver.
    pub fn back(&mut self) -> Option<usize> {
        let s = self.history.pop()?;
        self.current = s;
        Some(s)
    }

    /// Balão de saída aberto.
    pub fn asking(&self) -> Option<Ask> {
        self.asking
    }

    /// Trabalhando por conta própria: sem balão até clicar no personagem.
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Nem pausado nem com balão de saída: o passo atual está à vista.
    pub fn active(&self) -> bool {
        !self.paused && self.asking.is_none()
    }

    /// Abre um balão de saída.
    pub fn ask(&mut self, a: Ask) {
        self.asking = Some(a);
    }

    /// Responde ao balão de saída aberto: `Some(k)` é a opção `k` do
    /// original, `None` um clique fora (ou no personagem), que vale
    /// "ignorar". Uma opção fora da tabela também fecha. Sem balão aberto,
    /// não faz nada ([`Answer::Close`]).
    pub fn answer(&mut self, option: Option<usize>, answers: Answers) -> Answer {
        if self.asking.take().is_none() {
            return Answer::Close;
        }
        let a = option
            .and_then(|k| answers.get(k).copied())
            .unwrap_or(Answer::Close);
        if a == Answer::Pause {
            self.paused = true;
        }
        a
    }

    /// Clique no personagem durante a pausa: o assistente volta ao passo
    /// em que estava. Devolve se estava pausado.
    pub fn resume(&mut self) -> bool {
        std::mem::take(&mut self.paused)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_back_replace_and_restart() {
        let mut g = Guide::new(0);
        assert!(!g.has_back());
        assert_eq!(g.back(), None);
        g.forward(3);
        g.forward(5);
        g.replace(6);
        assert_eq!(g.current(), 6);
        assert_eq!(g.back(), Some(3));
        assert_eq!(g.back(), Some(0));
        assert_eq!(g.back(), None);
        assert_eq!(g.current(), 0);
        g.forward(2);
        g.restart(0);
        assert!(!g.has_back());
        assert_eq!(g.current(), 0);
    }

    #[test]
    fn quit_answers_pause_leave_and_close() {
        let mut g = Guide::new(4);
        assert_eq!(g.answer(Some(1), QUIT_ANSWERS), Answer::Close);
        g.ask(Ask::Quit);
        assert!(!g.active());
        assert_eq!(g.answer(Some(2), QUIT_ANSWERS), Answer::Close);
        assert!(g.active());
        g.ask(Ask::Stop);
        assert_eq!(g.answer(None, QUIT_ANSWERS), Answer::Close);
        g.ask(Ask::Stop);
        assert_eq!(g.answer(Some(9), QUIT_ANSWERS), Answer::Close);
        g.ask(Ask::Quit);
        assert_eq!(g.answer(Some(1), QUIT_ANSWERS), Answer::Leave);
        g.ask(Ask::Quit);
        assert_eq!(g.answer(Some(0), QUIT_ANSWERS), Answer::Pause);
        assert!(g.paused() && !g.active());
        assert_eq!(g.current(), 4);
        assert!(g.resume());
        assert!(!g.resume());
        assert!(g.active());
    }

    #[test]
    fn stop_answers_of_the_fine_artist_lesson() {
        let mut g = Guide::new(0);
        g.ask(Ask::Stop);
        assert_eq!(g.asking(), Some(Ask::Stop));
        assert_eq!(g.answer(Some(0), STOP_ANSWERS), Answer::Finish);
        assert_eq!(g.asking(), None);
        assert!(!g.paused());
    }
}
