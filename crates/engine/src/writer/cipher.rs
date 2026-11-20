//! Agente Secreto 009 (Transformadores): o texto "embaralhado" na tela.
//!
//! O original **não cifra o arquivo** (`docs/formats/max.md`, "Agente
//! secreto"): grava `ADD[5] = 1` e a senha (cada letra +5) e só a exibição
//! muda. A troca de letras é fixa — medida no original com várias senhas, ela
//! não depende da senha: cada caractere de `0-9A-Za-z` (62, nessa ordem)
//! anda 31 casas, de modo que embaralhar duas vezes devolve o texto
//! (involução). Pontuação, espaços, acentuadas e as figuras ficam no lugar
//! (as figuras viram quadrados pretos). Exemplo de `office-ferramentas.md`
//! §7: "O gato e a bicicleta foram passear no parque." →
//! "t B5OJ 9 5 6D7D7G9O5 AJM5H K5NN95M IJ K5MLP9." (na captura o espaço
//! depois do "t" quase some; a pesquisa o transcreveu junto).
//!
//! A senha só decide quem desembaralha: a mesma senha, ou a senha-mestra
//! [`MASTER`] com Ctrl+Shift apertados no OK (`DIALOG 1388`).

use formats::max::Add;

/// Os 62 caracteres trocados, na ordem do original.
const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Quantas casas cada caractere anda (metade do alfabeto: involução).
const SHIFT: usize = 31;

/// Senha-mestra (com Control+Shift + OK).
pub const MASTER: &str = "Detetive McZee para salvar o dia";

/// Maior senha que cabe no `ADD ` (32 bytes).
pub const MAX_LEN: usize = 32;

/// O caractere como aparece no documento embaralhado.
pub fn scramble_char(c: char) -> char {
    match ALPHABET.iter().position(|&a| u32::from(a) == u32::from(c)) {
        Some(k) => char::from(ALPHABET[(k + SHIFT) % ALPHABET.len()]),
        None => c,
    }
}

/// O texto como aparece no documento embaralhado.
pub fn scramble(s: &str) -> String {
    s.chars().map(scramble_char).collect()
}

/// O que o OK do painel SENHA fez.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Embaralhou (o documento estava normal).
    Scrambled,
    /// Desembaralhou com a senha certa ou a senha-mestra.
    Unscrambled,
    /// Senha vazia: `0805[43]` (ao embaralhar) ou `[37]` (ao desembaralhar).
    Empty,
    /// Senha errada: `0805[38]`.
    Wrong,
}

/// OK no painel SENHA com `typed` digitado; `master_keys` = Ctrl e Shift
/// apertados. Muda `add` (`encrypted` e `password`) e diz o que houve.
pub fn ok(add: &mut Add, typed: &str, master_keys: bool) -> Outcome {
    // o ADD guarda a senha completada com espaços: os do fim não contam
    let typed: String = typed.chars().take(MAX_LEN).collect();
    let typed = typed.trim_end();
    if typed.is_empty() {
        return Outcome::Empty;
    }
    if !add.encrypted {
        add.encrypted = true;
        add.password = typed.to_string();
        return Outcome::Scrambled;
    }
    if typed == add.password.trim_end() || (master_keys && typed == MASTER) {
        add.encrypted = false;
        add.password.clear();
        Outcome::Unscrambled
    } else {
        Outcome::Wrong
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_original_sample() {
        assert_eq!(
            scramble("O gato e a bicicleta foram passear no parque."),
            "t B5OJ 9 5 6D7D7G9O5 AJM5H K5NN95M IJ K5MLP9."
        );
    }

    #[test]
    fn is_an_involution_that_keeps_everything_else() {
        let all: String = (' '..='\u{ff}').collect();
        assert_eq!(scramble(&scramble(&all)), all);
        assert_eq!(scramble("ção, é! ?\r"), "çãJ, é! ?\r");
        assert_eq!(scramble("09AZaz"), "Vef45U");
        // nenhuma letra fica no lugar
        for &c in ALPHABET {
            assert_ne!(scramble_char(char::from(c)), char::from(c));
        }
        assert_eq!(MASTER.len(), MAX_LEN);
    }

    #[test]
    fn password_round_trip() {
        let mut add = Add::default();
        assert_eq!(ok(&mut add, "", false), Outcome::Empty);
        assert_eq!(ok(&mut add, "   ", false), Outcome::Empty);
        assert!(!add.encrypted);
        assert_eq!(ok(&mut add, "abc", false), Outcome::Scrambled);
        assert!(add.encrypted);
        assert_eq!(add.password, "abc");
        // gravado com +5 e lido de volta
        let back = Add::from_bytes(&add.to_bytes());
        assert!(back.encrypted);
        assert_eq!(back.password, "abc");
        assert_eq!(ok(&mut add, "", false), Outcome::Empty);
        assert_eq!(ok(&mut add, "abd", false), Outcome::Wrong);
        assert_eq!(ok(&mut add, "ABC", false), Outcome::Wrong);
        // a senha-mestra só vale com Ctrl+Shift
        assert_eq!(ok(&mut add, MASTER, false), Outcome::Wrong);
        assert_eq!(ok(&mut add, MASTER, true), Outcome::Unscrambled);
        assert!(!add.encrypted);
        assert!(add.password.is_empty());
        let back = Add::from_bytes(&add.to_bytes());
        assert!(!back.encrypted);
        assert_eq!(back.password, "");
        // a própria senha
        ok(&mut add, "segredo", false);
        assert_eq!(ok(&mut add, "segredo", false), Outcome::Unscrambled);
    }
}
