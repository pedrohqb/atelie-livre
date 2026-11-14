//! Texto editável do Caderno de Idéias (aba Lembretes) e dos campos de uma
//! linha da Oficina (POE da Máquina, título do Quadro): cursor, seleção por
//! arrasto (por caractere, `office-complemento.md` §6.2), recortar e copiar,
//! e a quebra em linhas para desenhar e achar o caractere sob o mouse.

/// Texto com cursor e seleção, em caracteres.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoteText {
    chars: Vec<char>,
    cursor: usize,
    /// Outra ponta da seleção (a seleção vai de `anchor` a `cursor`).
    anchor: Option<usize>,
}

impl NoteText {
    pub fn new(text: &str) -> NoteText {
        let chars: Vec<char> = text.chars().collect();
        NoteText {
            cursor: chars.len(),
            chars,
            anchor: None,
        }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, i: usize) {
        self.cursor = i.min(self.chars.len());
        self.anchor = None;
    }

    /// Arrasto do mouse: a seleção vai do ponto apertado até `i`.
    pub fn drag_to(&mut self, i: usize) {
        if self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        }
        self.cursor = i.min(self.chars.len());
    }

    /// Seleção `(início, fim)` com o fim exclusivo; `None` se vazia.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        (a != self.cursor).then(|| (a.min(self.cursor), a.max(self.cursor)))
    }

    fn delete_selection(&mut self) -> Option<String> {
        let (a, b) = self.selection()?;
        let out: String = self.chars.drain(a..b).collect();
        self.cursor = a;
        self.anchor = None;
        Some(out)
    }

    /// Texto digitado (substitui a seleção). Quebras de linha `\r` viram `\n`.
    pub fn insert(&mut self, s: &str) {
        self.delete_selection();
        for c in s.chars() {
            let c = if c == '\r' { '\n' } else { c };
            if c.is_control() && c != '\n' {
                continue;
            }
            self.chars.insert(self.cursor, c);
            self.cursor += 1;
        }
    }

    pub fn backspace(&mut self) {
        if self.delete_selection().is_none() && self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.delete_selection().is_none() && self.cursor < self.chars.len() {
            self.chars.remove(self.cursor);
        }
    }

    pub fn left(&mut self) {
        self.set_cursor(self.cursor.saturating_sub(1));
    }

    pub fn right(&mut self) {
        self.set_cursor(self.cursor + 1);
    }

    /// Tesoura: tira a seleção e a devolve.
    pub fn cut(&mut self) -> Option<String> {
        self.delete_selection()
    }

    /// Câmera: a seleção, sem mudar o texto.
    pub fn copied(&self) -> Option<String> {
        let (a, b) = self.selection()?;
        Some(self.chars[a..b].iter().collect())
    }

    /// Linhas `(início, fim)` para a largura `width`, quebrando nas palavras
    /// (uma palavra maior que a linha é cortada) e nos `\n`. O fim não
    /// inclui o espaço ou o `\n` da quebra.
    pub fn lines(&self, width: i32, measure: impl Fn(&str) -> i32) -> Vec<(usize, usize)> {
        wrap(&self.chars, width, measure)
    }

    /// Caractere mais perto do ponto `(x, y)`, com as linhas de `lines`
    /// desenhadas a partir de y = 0 com altura `line_h`.
    pub fn hit(
        &self,
        lines: &[(usize, usize)],
        line_h: i32,
        x: i32,
        y: i32,
        measure: impl Fn(&str) -> i32,
    ) -> usize {
        let Some(&(a, b)) = lines
            .get((y.max(0) / line_h.max(1)) as usize)
            .or(lines.last())
        else {
            return 0;
        };
        if y >= line_h * lines.len() as i32 {
            return b;
        }
        let mut best = a;
        let mut s = String::new();
        for i in a..b {
            s.push(self.chars[i]);
            let w = measure(&s);
            if x >= w - measure(&self.chars[i].to_string()) / 2 {
                best = i + 1;
            }
        }
        best
    }

    /// Linha e x (relativo ao começo da linha) do cursor.
    pub fn caret(&self, lines: &[(usize, usize)], measure: impl Fn(&str) -> i32) -> (usize, i32) {
        let k = lines
            .iter()
            .rposition(|&(a, _)| a <= self.cursor)
            .unwrap_or(0);
        let (a, b) = lines.get(k).copied().unwrap_or((0, 0));
        let s: String = self.chars[a..self.cursor.min(b).max(a)].iter().collect();
        (k, measure(&s))
    }

    /// Caracteres `a..b` como texto.
    pub fn slice(&self, a: usize, b: usize) -> String {
        self.chars[a.min(self.len())..b.min(self.len())]
            .iter()
            .collect()
    }
}

/// Quebra `chars` em linhas de no máximo `width` pixels.
pub fn wrap(chars: &[char], width: i32, measure: impl Fn(&str) -> i32) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    let n = chars.len();
    while start <= n {
        // fim do parágrafo
        let para_end = chars[start..]
            .iter()
            .position(|&c| c == '\n')
            .map_or(n, |p| start + p);
        let mut a = start;
        loop {
            let s: String = chars[a..para_end].iter().collect();
            if measure(&s) <= width || para_end - a <= 1 {
                out.push((a, para_end));
                break;
            }
            // maior prefixo que cabe, quebrando no último espaço
            let mut fit = a + 1;
            let mut last_space = None;
            for i in a + 1..=para_end {
                let s: String = chars[a..i].iter().collect();
                if measure(&s) > width {
                    break;
                }
                fit = i;
                if chars[i - 1] == ' ' {
                    last_space = Some(i - 1);
                }
            }
            match last_space {
                Some(sp) if sp > a => {
                    out.push((a, sp));
                    a = sp + 1;
                }
                _ => {
                    out.push((a, fit));
                    a = fit;
                }
            }
        }
        if para_end == n {
            break;
        }
        start = para_end + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(s: &str) -> i32 {
        6 * s.chars().count() as i32
    }

    #[test]
    fn typing_and_selection_cut_like_the_original() {
        let mut t = NoteText::new("Ideia: gato astronauta");
        // arrastar sobre "ato ast" e recortar
        t.set_cursor(8);
        t.drag_to(15);
        assert_eq!(t.copied().as_deref(), Some("ato ast"));
        assert_eq!(t.cut().as_deref(), Some("ato ast"));
        assert_eq!(t.text(), "Ideia: gronauta");
        assert_eq!(t.selection(), None);
        assert_eq!(t.cut(), None);
    }

    #[test]
    fn insert_replaces_the_selection_and_backspace_deletes() {
        let mut t = NoteText::new("abc");
        t.set_cursor(0);
        t.drag_to(2);
        t.insert("X\rY");
        assert_eq!(t.text(), "X\nYc");
        t.backspace();
        assert_eq!(t.text(), "X\nc");
        t.left();
        t.delete();
        assert_eq!(t.text(), "Xc");
    }

    #[test]
    fn wrap_breaks_on_spaces_and_newlines() {
        let chars: Vec<char> = "um dois tres\nquatro".chars().collect();
        // 8 caracteres por linha
        let l = wrap(&chars, 48, m);
        let s: Vec<String> = l
            .iter()
            .map(|&(a, b)| chars[a..b].iter().collect())
            .collect();
        assert_eq!(s, ["um dois", "tres", "quatro"]);
        assert_eq!(wrap(&[], 48, m), vec![(0, 0)]);
        // palavra comprida é cortada
        let long: Vec<char> = "abcdefghijkl".chars().collect();
        assert_eq!(wrap(&long, 30, m), vec![(0, 5), (5, 10), (10, 12)]);
    }

    #[test]
    fn hit_and_caret_find_characters() {
        let t = NoteText::new("um dois tres");
        let l = t.lines(48, m);
        assert_eq!(t.hit(&l, 10, 0, 0, m), 0);
        assert_eq!(t.hit(&l, 10, 13, 2, m), 2);
        assert_eq!(t.hit(&l, 10, 7, 12, m), 9);
        assert_eq!(t.hit(&l, 10, 500, 500, m), 12);
        assert_eq!(t.caret(&l, m), (1, 24));
    }
}
