//! Máquina de Idéias (o "Splot Machine"): três janelas, cada uma mostra um
//! texto sorteado das tabelas, uma figura-rebus ou uma entrada do usuário;
//! juntas formam uma sentença maluca para começar uma história.
//!
//! Regra de sorteio (rotina `seg68:0x0cc4` do `WRITER.EXE`): com `U`
//! entradas do usuário, `F` figuras e as tabelas `A`, `B`... da janela,
//! sorteia-se `r` em `0..T + F + U`, onde `T = max(|A|, |B|, ...)`;
//! `r < U` é a entrada `r` do usuário, `r < U + F` é a figura `r − U` e o
//! resto é texto, com uma frase de cada tabela sorteada de forma uniforme e
//! independente e posta no formato da janela. Com Ctrl só as entradas do
//! usuário entram no sorteio; com Shift só as figuras (se não houver
//! nenhuma, vale o sorteio completo).

use crate::rng::XorShift;
use formats::workshop::{self, Workshop};

/// Número de janelas da Máquina.
pub const WINDOWS: usize = 3;

/// Duração do giro da alavanca: o original sorteia de novo as três janelas
/// a cada quadro durante 1000 ms (`timeGetTime() + 0x3e8`).
pub const SPIN_MS: u32 = 1000;

/// Regra de uma janela: as tabelas de frases, o formato e as figuras.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRule {
    pub tables: Vec<Vec<String>>,
    /// Formato com um `%s` por tabela (`"    %s %s"` na janela 1).
    pub format: String,
    /// Caracteres das figuras-rebus.
    pub figures: Vec<u8>,
}

impl WindowRule {
    /// Peso do texto no sorteio: o tamanho da maior tabela.
    pub fn text_weight(&self) -> usize {
        self.tables.iter().map(Vec::len).max().unwrap_or(0)
    }

    /// Formato preenchido com as frases dadas, na ordem dos `%s`.
    fn fill(&self, phrases: &[&str]) -> String {
        let mut out = String::new();
        let mut rest = self.format.as_str();
        let mut it = phrases.iter();
        while let Some(at) = rest.find("%s") {
            out.push_str(&rest[..at]);
            out.push_str(it.next().copied().unwrap_or(""));
            rest = &rest[at + 2..];
        }
        out.push_str(rest);
        out
    }
}

/// Gramática completa: uma regra por janela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grammar {
    pub windows: [WindowRule; WINDOWS],
}

impl Grammar {
    /// Gramática do `WORKSHOP.CHK` (tabela ausente fica vazia).
    pub fn from_workshop(w: &Workshop<'_>) -> Result<Grammar, String> {
        if w.grammar.windows.len() < WINDOWS {
            return Err(format!(
                "SMDG com {} janelas (esperadas {WINDOWS})",
                w.grammar.windows.len()
            ));
        }
        let rule = |r: &workshop::WindowRule| WindowRule {
            tables: r
                .tables
                .iter()
                .map(|id| {
                    w.table(*id)
                        .map(|t| t.texts().map(str::to_string).collect())
                        .unwrap_or_default()
                })
                .collect(),
            format: r.format.clone(),
            figures: r.figures.iter().map(|f| f.code).collect(),
        };
        let g = &w.grammar.windows;
        Ok(Grammar {
            windows: [rule(&g[0]), rule(&g[1]), rule(&g[2])],
        })
    }

    /// Lê `SHARED/STUFF/WORKSHOP.CHK` da pasta `MSKIDS`.
    pub fn load(data_dir: &std::path::Path) -> Result<Grammar, String> {
        let path = crate::paths::find_ci(data_dir, "SHARED/STUFF/WORKSHOP.CHK")
            .ok_or("WORKSHOP.CHK não encontrado")?;
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let w = Workshop::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        Grammar::from_workshop(&w)
    }
}

/// Que itens entram num sorteio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Tudo: textos, figuras e entradas do usuário.
    #[default]
    Any,
    /// Só as entradas do usuário (Ctrl + clique).
    UserOnly,
    /// Só as figuras (Shift + clique).
    FiguresOnly,
}

impl Mode {
    /// Modo pelas teclas seguradas no clique (Ctrl vence Shift, como no original).
    pub fn from_keys(ctrl: bool, shift: bool) -> Mode {
        if ctrl {
            Mode::UserOnly
        } else if shift {
            Mode::FiguresOnly
        } else {
            Mode::Any
        }
    }
}

/// O que uma janela mostra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Slot {
    /// Texto das tabelas, já no formato da janela (pode ter espaços na frente).
    Text(String),
    /// Figura-rebus (caractere).
    Rebus(u8),
    /// Entrada do usuário (índice na lista da janela).
    User(usize),
}

/// Pedaço da sentença final.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Rebus(u8),
}

/// Por que a TIRA não jogou nada fora.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveError {
    /// A janela mostra uma coisa do Max (balão `0805[9]`).
    MaxEntry,
}

/// Estado da Máquina: a gramática, as entradas do usuário e o que cada
/// janela mostra. A sentença sorteada vale para a sessão inteira (voltar à
/// Máquina mostra a última).
#[derive(Debug, Clone)]
pub struct Machine {
    grammar: Grammar,
    user: [Vec<String>; WINDOWS],
    slots: [Slot; WINDOWS],
}

impl Machine {
    /// Máquina com as entradas do usuário e as três janelas já sorteadas.
    pub fn new(grammar: Grammar, user: [Vec<String>; WINDOWS], rng: &mut XorShift) -> Machine {
        let mut m = Machine {
            grammar,
            user,
            slots: [Slot::Rebus(0), Slot::Rebus(0), Slot::Rebus(0)],
        };
        m.spin(rng, Mode::Any);
        m
    }

    pub fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    pub fn slot(&self, w: usize) -> &Slot {
        &self.slots[w]
    }

    pub fn user_entries(&self, w: usize) -> &[String] {
        &self.user[w]
    }

    /// Entradas do usuário das três janelas (para gravar).
    pub fn all_user_entries(&self) -> &[Vec<String>; WINDOWS] {
        &self.user
    }

    /// Sorteia um item da janela `w` sem mudar o estado.
    pub fn pick(&self, w: usize, rng: &mut XorShift, mode: Mode) -> Slot {
        let rule = &self.grammar.windows[w];
        let u = self.user[w].len();
        let f = rule.figures.len();
        let (count, offset) = match mode {
            Mode::UserOnly if u > 0 => (u, 0),
            Mode::FiguresOnly if f > 0 => (f, u),
            _ => (rule.text_weight() + f + u, 0),
        };
        if count == 0 {
            return Slot::Text(String::new());
        }
        let r = rng.below(count) + offset;
        if r < u {
            Slot::User(r)
        } else if r < u + f {
            Slot::Rebus(rule.figures[r - u])
        } else {
            let phrases: Vec<&str> = rule
                .tables
                .iter()
                .map(|t| t.get(rng.below(t.len())).map_or("", String::as_str))
                .collect();
            Slot::Text(rule.fill(&phrases))
        }
    }

    /// Botão redondo da janela `w`: outra entrada só naquela janela.
    pub fn next(&mut self, w: usize, rng: &mut XorShift, mode: Mode) {
        self.slots[w] = self.pick(w, rng, mode);
    }

    /// Alavanca: as três janelas de uma vez (um passo do giro; o resultado
    /// final é o do último passo).
    pub fn spin(&mut self, rng: &mut XorShift, mode: Mode) {
        for w in 0..WINDOWS {
            self.next(w, rng, mode);
        }
    }

    /// POE: guarda uma entrada nova na janela `w` e a mostra. Texto vazio
    /// (só espaços) não entra; quebras de linha viram espaços.
    pub fn add_user(&mut self, w: usize, text: &str) -> bool {
        let text: String = text
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        let text = text.trim();
        if text.is_empty() {
            return false;
        }
        self.user[w].push(text.to_string());
        self.slots[w] = Slot::User(self.user[w].len() - 1);
        true
    }

    /// TIRA: joga fora a entrada do usuário mostrada na janela `w` e sorteia
    /// outra coisa para a janela. Coisas do Max não saem.
    pub fn remove_user(&mut self, w: usize, rng: &mut XorShift) -> Result<String, RemoveError> {
        let Slot::User(i) = self.slots[w] else {
            return Err(RemoveError::MaxEntry);
        };
        let removed = self.user[w].remove(i);
        self.next(w, rng, Mode::Any);
        Ok(removed)
    }

    /// O que a janela `w` mostra, como pedaço (texto sem os espaços das pontas).
    pub fn piece(&self, w: usize) -> Piece {
        match &self.slots[w] {
            Slot::Text(t) => Piece::Text(t.trim().to_string()),
            Slot::Rebus(c) => Piece::Rebus(*c),
            Slot::User(i) => Piece::Text(self.user[w].get(*i).cloned().unwrap_or_default()),
        }
    }

    /// Texto da janela `w` como aparece nela (com os espaços do formato);
    /// `None` se a janela mostra uma figura.
    pub fn window_text(&self, w: usize) -> Option<&str> {
        match &self.slots[w] {
            Slot::Text(t) => Some(t),
            Slot::Rebus(_) => None,
            Slot::User(i) => self.user[w].get(*i).map(String::as_str),
        }
    }

    /// A sentença: janela 1 + " " + janela 2 + " " + janela 3, com os
    /// textos vizinhos juntados.
    pub fn sentence(&self) -> Vec<Piece> {
        join((0..WINDOWS).map(|w| self.piece(w)))
    }
}

/// Junta os pedaços com um espaço entre eles e funde textos vizinhos.
pub fn join(pieces: impl IntoIterator<Item = Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    let push_text = |out: &mut Vec<Piece>, s: &str| match out.last_mut() {
        Some(Piece::Text(t)) => t.push_str(s),
        _ => out.push(Piece::Text(s.to_string())),
    };
    for (i, p) in pieces.into_iter().enumerate() {
        if i > 0 {
            push_text(&mut out, " ");
        }
        match p {
            Piece::Text(t) => push_text(&mut out, &t),
            r @ Piece::Rebus(_) => out.push(r),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn grammar() -> Grammar {
        Grammar {
            windows: [
                WindowRule {
                    tables: vec![s(&["O pirata", "A bruxa"]), s(&["de chapéu,"])],
                    format: "    %s %s".into(),
                    figures: vec![0x25],
                },
                WindowRule {
                    tables: vec![s(&["foi caminhar", "riu", "dormiu"])],
                    format: "%s".into(),
                    figures: vec![],
                },
                WindowRule {
                    tables: vec![s(&["sobre"]), s(&["a lua.", "o mar."])],
                    format: "%s %s".into(),
                    figures: vec![0x1a, 0x08],
                },
            ],
        }
    }

    fn machine(seed: u32) -> (Machine, XorShift) {
        let mut rng = XorShift::new(seed);
        let m = Machine::new(grammar(), Default::default(), &mut rng);
        (m, rng)
    }

    #[test]
    fn text_weight_is_the_largest_table() {
        let g = grammar();
        assert_eq!(g.windows[0].text_weight(), 2);
        assert_eq!(g.windows[1].text_weight(), 3);
        assert_eq!(g.windows[2].text_weight(), 2);
    }

    #[test]
    fn fill_uses_the_format() {
        let g = grammar();
        assert_eq!(
            g.windows[0].fill(&["O pirata", "de chapéu,"]),
            "    O pirata de chapéu,"
        );
        assert_eq!(g.windows[2].fill(&["sobre"]), "sobre ");
    }

    #[test]
    fn picks_follow_the_original_proportions() {
        // janela 3: T = 2, F = 2, U = 1 → figura 2/5, usuário 1/5, texto 2/5
        let (mut m, mut rng) = machine(99);
        m.add_user(2, "na escola.");
        let n = 20_000;
        let (mut fig, mut user, mut text) = (0, 0, 0);
        for _ in 0..n {
            match m.pick(2, &mut rng, Mode::Any) {
                Slot::Rebus(c) => {
                    assert!(c == 0x1a || c == 0x08);
                    fig += 1
                }
                Slot::User(0) => user += 1,
                Slot::Text(t) => {
                    assert!(t == "sobre a lua." || t == "sobre o mar.", "{t}");
                    text += 1
                }
                other => panic!("{other:?}"),
            }
        }
        let near = |k: usize, p: f64| ((k as f64 / n as f64) - p).abs() < 0.02;
        assert!(
            near(fig, 0.4) && near(user, 0.2) && near(text, 0.4),
            "{fig} {user} {text}"
        );
    }

    #[test]
    fn window_without_figures_never_shows_one() {
        let (m, mut rng) = machine(5);
        for _ in 0..500 {
            assert!(matches!(m.pick(1, &mut rng, Mode::Any), Slot::Text(_)));
        }
    }

    #[test]
    fn modes_restrict_the_draw() {
        let (mut m, mut rng) = machine(3);
        // sem entradas do usuário, Ctrl cai no sorteio completo
        let any = (0..200).any(|_| !matches!(m.pick(0, &mut rng, Mode::UserOnly), Slot::User(_)));
        assert!(any);
        m.add_user(0, "Zeca,");
        m.add_user(0, "Lia,");
        for _ in 0..200 {
            assert!(matches!(
                m.pick(0, &mut rng, Mode::UserOnly),
                Slot::User(0 | 1)
            ));
            assert_eq!(m.pick(0, &mut rng, Mode::FiguresOnly), Slot::Rebus(0x25));
        }
        // janela sem figuras: Shift cai no sorteio completo
        assert!(matches!(
            m.pick(1, &mut rng, Mode::FiguresOnly),
            Slot::Text(_)
        ));
        assert_eq!(Mode::from_keys(true, true), Mode::UserOnly);
        assert_eq!(Mode::from_keys(false, true), Mode::FiguresOnly);
        assert_eq!(Mode::from_keys(false, false), Mode::Any);
    }

    #[test]
    fn same_seed_same_sentence() {
        let (a, _) = machine(1234);
        let (b, _) = machine(1234);
        assert_eq!(a.sentence(), b.sentence());
    }

    #[test]
    fn spin_changes_all_windows_and_next_only_one() {
        let (mut m, mut rng) = machine(77);
        let before = [m.slot(0).clone(), m.slot(1).clone(), m.slot(2).clone()];
        for _ in 0..20 {
            m.next(1, &mut rng, Mode::Any);
        }
        assert_eq!(m.slot(0), &before[0]);
        assert_eq!(m.slot(2), &before[2]);
        let mut changed = [false; 3];
        for _ in 0..50 {
            m.spin(&mut rng, Mode::Any);
            for (w, c) in changed.iter_mut().enumerate() {
                *c |= m.slot(w) != &before[w];
            }
        }
        assert_eq!(changed, [true; 3]);
    }

    #[test]
    fn sentence_trims_format_spaces_and_keeps_rebus() {
        let mut m = Machine {
            grammar: grammar(),
            user: Default::default(),
            slots: [
                Slot::Text("    O pirata de chapéu,".into()),
                Slot::Text("riu".into()),
                Slot::Rebus(0x1a),
            ],
        };
        assert_eq!(m.window_text(0), Some("    O pirata de chapéu,"));
        assert_eq!(m.window_text(2), None);
        assert_eq!(
            m.sentence(),
            [
                Piece::Text("O pirata de chapéu, riu ".into()),
                Piece::Rebus(0x1a)
            ]
        );
        m.slots[0] = Slot::Rebus(0x25);
        assert_eq!(
            m.sentence(),
            [
                Piece::Rebus(0x25),
                Piece::Text(" riu ".into()),
                Piece::Rebus(0x1a)
            ]
        );
    }

    #[test]
    fn user_entries_add_show_and_remove() {
        let (mut m, mut rng) = machine(8);
        assert!(!m.add_user(1, "   "));
        assert!(m.add_user(1, " dançou\n"));
        assert_eq!(m.slot(1), &Slot::User(0));
        assert_eq!(m.window_text(1), Some("dançou"));
        assert_eq!(m.piece(1), Piece::Text("dançou".into()));
        assert_eq!(m.remove_user(1, &mut rng), Ok("dançou".into()));
        assert!(m.user_entries(1).is_empty());
        assert!(matches!(m.slot(1), Slot::Text(_)));
        assert_eq!(m.remove_user(1, &mut rng), Err(RemoveError::MaxEntry));
    }

    #[test]
    fn empty_rule_gives_empty_text() {
        let mut g = grammar();
        g.windows[1].tables.clear();
        let mut rng = XorShift::new(1);
        let m = Machine::new(g, Default::default(), &mut rng);
        assert_eq!(m.slot(1), &Slot::Text(String::new()));
    }
}
