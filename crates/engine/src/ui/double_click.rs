//! Clique duplo: o segundo clique no mesmo alvo (e, se pedido, perto do
//! primeiro) em menos de [`DOUBLE_CLICK_S`] segundos.

/// Intervalo máximo entre os dois cliques (s).
pub const DOUBLE_CLICK_S: f32 = 0.4;

/// Lembra o último clique: alvo, instante e posição.
#[derive(Debug, Clone)]
pub struct DoubleClick<K> {
    last: Option<(K, f32, (i32, i32))>,
    /// Distância máxima (px, em x e em y) entre os dois cliques; `None`
    /// não confere a posição.
    slack: Option<i32>,
}

impl<K> Default for DoubleClick<K> {
    fn default() -> Self {
        DoubleClick {
            last: None,
            slack: None,
        }
    }
}

impl<K: Copy + PartialEq> DoubleClick<K> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Também exige que o segundo clique caia a até `px` pixels do primeiro.
    pub fn with_slack(px: i32) -> Self {
        DoubleClick {
            last: None,
            slack: Some(px),
        }
    }

    fn close(&self, t: f32, p: (i32, i32), now: f32, pos: (i32, i32)) -> bool {
        now - t < DOUBLE_CLICK_S
            && self
                .slack
                .is_none_or(|s| (pos.0 - p.0).abs() <= s && (pos.1 - p.1).abs() <= s)
    }

    /// Clique em `key` no instante `now`: `true` se completa um clique duplo
    /// (e então o esquece); senão passa a ser o primeiro clique.
    pub fn click(&mut self, key: K, now: f32, pos: (i32, i32)) -> bool {
        if let Some((k, t, p)) = self.last
            && k == key
            && self.close(t, p, now, pos)
        {
            self.last = None;
            return true;
        }
        self.last = Some((key, now, pos));
        false
    }

    /// Se o clique em `pos` completa um duplo com o anterior (qualquer alvo),
    /// devolve o alvo do primeiro e o esquece.
    pub fn take(&mut self, now: f32, pos: (i32, i32)) -> Option<K> {
        let (k, t, p) = self.last?;
        if !self.close(t, p, now, pos) {
            return None;
        }
        self.last = None;
        Some(k)
    }

    /// Guarda o primeiro clique sem testar.
    pub fn arm(&mut self, key: K, now: f32, pos: (i32, i32)) {
        self.last = Some((key, now, pos));
    }

    /// Esquece o clique pendente.
    pub fn reset(&mut self) {
        self.last = None;
    }

    /// O alvo do clique pendente.
    pub fn pending(&self) -> Option<K> {
        self.last.map(|l| l.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_key_inside_the_interval_is_double() {
        let mut d = DoubleClick::new();
        assert!(!d.click(3, 1.0, (0, 0)));
        assert!(d.click(3, 1.3, (50, 50)));
        // esquecido depois do duplo: o terceiro é um primeiro clique
        assert!(!d.click(3, 1.4, (0, 0)));
    }

    #[test]
    fn other_key_or_late_click_starts_over() {
        let mut d = DoubleClick::new();
        d.click(1, 0.0, (0, 0));
        assert!(!d.click(2, 0.1, (0, 0)));
        assert!(!d.click(2, 0.1 + DOUBLE_CLICK_S, (0, 0)));
        assert!(d.click(2, 0.2 + DOUBLE_CLICK_S, (0, 0)));
    }

    #[test]
    fn slack_limits_the_distance() {
        let mut d = DoubleClick::with_slack(6);
        d.arm('e', 0.0, (100, 100));
        assert_eq!(d.take(0.1, (110, 100)), None);
        assert_eq!(d.take(0.1, (106, 94)), Some('e'));
        assert_eq!(d.take(0.2, (106, 94)), None);
        d.arm('e', 0.0, (100, 100));
        assert_eq!(d.pending(), Some('e'));
        d.reset();
        assert_eq!(d.pending(), None);
    }
}
