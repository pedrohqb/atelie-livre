//! Gerador pseudoaleatório pequeno e determinístico (xorshift32), para os
//! sprays da pintura e o sorteio das animações ambientes.

#[derive(Debug, Clone)]
pub struct XorShift(u32);

impl XorShift {
    pub fn new(seed: u32) -> XorShift {
        XorShift(seed.max(1))
    }

    /// Semente tirada do relógio.
    pub fn from_clock() -> XorShift {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() ^ (d.as_secs() as u32))
            .unwrap_or(1);
        XorShift::new(nanos)
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Inteiro em `-r..=r`.
    pub fn offset(&mut self, r: i32) -> i32 {
        (self.next_u32() % (2 * r as u32 + 1)) as i32 - r
    }

    /// Índice em `0..n` (0 se `n == 0`).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u32() as usize) % n
        }
    }

    /// Número real em `lo..=hi`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() % 10_000) as f32 / 10_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_stay_in_bounds() {
        let mut r = XorShift::new(7);
        for _ in 0..1000 {
            assert!((-3..=3).contains(&r.offset(3)));
            assert!(r.below(5) < 5);
            let v = r.range(1.0, 2.0);
            assert!((1.0..=2.0).contains(&v));
        }
        assert_eq!(r.below(0), 0);
        assert_ne!(XorShift::new(0).next_u32(), 0);
    }
}
