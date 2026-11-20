//! Efeitos especiais animados das palavras (lista "Efeitos especiais" da
//! Palavra; sprm de caractere `0x0b`), medidos quadro a quadro no original
//! (`office-complemento.md` §2).
//!
//! Todos os efeitos do documento andam **juntos**, num ciclo de 2,7 s com 9
//! estados: S0 (1,1 s) com tudo normal e S1..S8 (0,2 s cada) com os 8 passos
//! da animação. Cada trecho seguido com o mesmo efeito numa linha forma um
//! retângulo (x do primeiro ao último caractere, y = a caixa da linha); os
//! efeitos não mudam a diagramação. Aqui fica só a lógica (passo do relógio
//! e o que cada efeito pinta em cada passo); o desenho na folha está em
//! `scenes/writer/effects_draw.rs`.

/// Duração do ciclo inteiro.
pub const CYCLE_MS: u64 = 2700;
/// Duração de S0 (tudo normal).
pub const REST_MS: u64 = 1100;
/// Duração de cada passo S1..S8.
pub const STEP_MS: u64 = 200;
/// Número de passos animados.
pub const STEPS: u8 = 8;

/// Estado do relógio em `ms` desde o começo: 0 = normal, 1..=8 = passos.
pub fn phase(ms: u64) -> u8 {
    let t = ms % CYCLE_MS;
    if t < REST_MS {
        0
    } else {
        (1 + (t - REST_MS) / STEP_MS).min(u64::from(STEPS)) as u8
    }
}

/// Número do ciclo (para sortear as faíscas do Brilho a cada passo).
pub fn cycle(ms: u64) -> u64 {
    ms / CYCLE_MS
}

/// Formiguinhas: o pixel `(x, y)` (coordenadas da tela 640×480) da
/// moldura está aceso no passo `step`. Traços de 4 px com 4 de intervalo
/// que andam 1 px por passo (para baixo nos lados verticais, para a direita
/// nos horizontais).
pub fn ants_on(x: i32, y: i32, step: u8) -> bool {
    step > 0 && (x + y - (i32::from(step) - 1)).rem_euclid(8) >= 4
}

/// Treme-treme: deslocamento horizontal da linha de pixels `y` (da tela)
/// no passo `step`: nos passos ímpares, y ímpar vai +1 e y par −1; nos
/// pares, nada.
pub fn shake_dx(y: i32, step: u8) -> i32 {
    if step % 2 == 1 {
        if y.rem_euclid(2) == 1 { 1 } else { -1 }
    } else {
        0
    }
}

/// Vapt: linhas de pixels apagadas do alto da caixa da linha (altura `h`)
/// no passo `step` (`2k`; no último passo, todas).
pub fn vapt_rows(step: u8, h: i32) -> i32 {
    match step {
        0 => 0,
        s if s >= STEPS => h.max(0),
        s => (2 * i32::from(s)).min(h.max(0)),
    }
}

/// Vupt: colunas apagadas da esquerda do retângulo de largura `w` no passo
/// `step` (`⌈w·k/8⌉`).
pub fn vupt_cols(step: u8, w: i32) -> i32 {
    let w = w.max(0);
    let k = i32::from(step.min(STEPS));
    (w * k + i32::from(STEPS) - 1) / i32::from(STEPS)
}

/// Forma de uma faísca do Brilho (3×3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparkShape {
    /// `#.#/.#./#.#`
    X,
    /// `.#./#.#/.#.`
    Plus,
    /// `###/#.#/###`
    Ring,
    /// `#.#/.../#.#`
    Corners,
    /// `#../.#./..#`
    Back,
    /// `..#/.#./#..`
    Slash,
    /// só o meio
    Dot,
}

impl SparkShape {
    pub const ALL: [SparkShape; 7] = [
        SparkShape::X,
        SparkShape::Plus,
        SparkShape::Ring,
        SparkShape::Corners,
        SparkShape::Back,
        SparkShape::Slash,
        SparkShape::Dot,
    ];

    /// Linhas da forma, de cima para baixo (bit 2 = coluna da esquerda).
    pub fn rows(self) -> [u8; 3] {
        match self {
            SparkShape::X => [0b101, 0b010, 0b101],
            SparkShape::Plus => [0b010, 0b101, 0b010],
            SparkShape::Ring => [0b111, 0b101, 0b111],
            SparkShape::Corners => [0b101, 0b000, 0b101],
            SparkShape::Back => [0b100, 0b010, 0b001],
            SparkShape::Slash => [0b001, 0b010, 0b100],
            SparkShape::Dot => [0b000, 0b010, 0b000],
        }
    }

    /// Pixels acesos `(dx, dy)` dentro do quadrado 3×3.
    pub fn pixels(self) -> impl Iterator<Item = (i32, i32)> {
        let rows = self.rows();
        (0..3).flat_map(move |dy| {
            (0..3)
                .filter(move |dx| rows[dy as usize] & (0b100 >> dx) != 0)
                .map(move |dx| (dx, dy))
        })
    }
}

/// Cores puras das faíscas: magenta, ciano, verde, vermelho, azul, amarelo.
pub const SPARK_COLORS: [[u8; 3]; 6] = [
    [255, 0, 255],
    [0, 255, 255],
    [0, 255, 0],
    [255, 0, 0],
    [0, 0, 255],
    [255, 255, 0],
];

/// Uma faísca do Brilho: canto do quadrado 3×3 relativo ao retângulo do
/// trecho, forma e cor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spark {
    pub x: i32,
    pub y: i32,
    pub shape: SparkShape,
    pub color: [u8; 3],
}

/// Gerador simples (splitmix64), para o sorteio ser o mesmo enquanto o
/// passo não muda.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
}

/// Semente das faíscas de um trecho num passo: muda a cada passo e de
/// trecho para trecho.
pub fn spark_seed(cycle: u64, step: u8, run: u64) -> u64 {
    cycle
        .wrapping_mul(0x1000_0000_01b3)
        .wrapping_add(u64::from(step) << 32)
        .wrapping_add(run.wrapping_mul(0x9e37))
}

/// Brilho: 2 a 4 faíscas sorteadas dentro do retângulo `w × h` (inclusive
/// sobre as letras).
pub fn sparks(seed: u64, w: i32, h: i32) -> Vec<Spark> {
    let mut r = Rng(seed);
    let n = 2 + r.below(3) as usize;
    let (fw, fh) = ((w - 2).max(1) as u64, (h - 2).max(1) as u64);
    (0..n)
        .map(|_| Spark {
            x: r.below(fw) as i32,
            y: r.below(fh) as i32,
            shape: SparkShape::ALL[r.below(SparkShape::ALL.len() as u64) as usize],
            color: SPARK_COLORS[r.below(SPARK_COLORS.len() as u64) as usize],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_has_a_rest_and_eight_steps() {
        assert_eq!(phase(0), 0);
        assert_eq!(phase(1099), 0);
        assert_eq!(phase(1100), 1);
        assert_eq!(phase(1299), 1);
        assert_eq!(phase(1300), 2);
        assert_eq!(phase(2500), 8);
        assert_eq!(phase(2699), 8);
        assert_eq!(phase(2700), 0);
        assert_eq!(phase(2700 + 1100), 1);
        assert_eq!(cycle(2699), 0);
        assert_eq!(cycle(2700), 1);
        // 33 quadros normais e 6 por passo a 30 qps
        let frames: Vec<u8> = (0..81).map(|f| phase(f * 100 / 3)).collect();
        assert_eq!(frames.iter().filter(|&&p| p == 0).count(), 33);
        for s in 1..=8 {
            assert_eq!(frames.iter().filter(|&&p| p == s).count(), 6, "{s}");
        }
    }

    #[test]
    fn ants_are_dashes_of_four_that_walk_one_pixel_per_step() {
        assert!(!ants_on(0, 0, 0));
        let row: Vec<bool> = (0..8).map(|x| ants_on(x, 0, 1)).collect();
        assert_eq!(row, [false, false, false, false, true, true, true, true]);
        // passo seguinte: o mesmo desenho 1 px à direita (e 1 px abaixo)
        for x in 0..16 {
            assert_eq!(ants_on(x + 1, 0, 2), ants_on(x, 0, 1));
            assert_eq!(ants_on(0, x + 1, 2), ants_on(0, x, 1));
        }
        // 8 passos = um período
        for x in 0..16 {
            assert_eq!(ants_on(x + 8, 3, 1), ants_on(x, 3, 1));
        }
    }

    #[test]
    fn shake_combs_the_rows_on_odd_steps() {
        assert_eq!((shake_dx(1, 1), shake_dx(2, 1)), (1, -1));
        assert_eq!((shake_dx(1, 3), shake_dx(-1, 7)), (1, 1));
        assert_eq!((shake_dx(1, 2), shake_dx(2, 8), shake_dx(1, 0)), (0, 0, 0));
    }

    #[test]
    fn vapt_and_vupt_wipe_until_everything_is_gone() {
        assert_eq!(vapt_rows(0, 15), 0);
        assert_eq!(vapt_rows(1, 15), 2);
        assert_eq!(vapt_rows(7, 15), 14);
        assert_eq!(vapt_rows(8, 15), 15);
        assert_eq!(vapt_rows(8, 51), 51);
        assert_eq!(vapt_rows(5, 6), 6);
        assert_eq!(vupt_cols(0, 52), 0);
        assert_eq!(vupt_cols(1, 52), 7);
        assert_eq!(vupt_cols(4, 52), 26);
        assert_eq!(vupt_cols(8, 52), 52);
    }

    #[test]
    fn sparks_are_two_to_four_inside_the_rectangle() {
        let mut seen = [false; 3];
        for seed in 0..200 {
            let s = sparks(spark_seed(seed, 3, 1), 40, 15);
            assert!((2..=4).contains(&s.len()));
            seen[s.len() - 2] = true;
            for k in &s {
                assert!(k.x >= 0 && k.x + 3 <= 40 && k.y >= 0 && k.y + 3 <= 15);
                assert!(SPARK_COLORS.contains(&k.color));
            }
        }
        assert_eq!(seen, [true; 3]);
        // o mesmo passo sorteia o mesmo; outro passo, outro sorteio
        assert_eq!(
            sparks(spark_seed(1, 2, 0), 40, 15),
            sparks(spark_seed(1, 2, 0), 40, 15)
        );
        assert_ne!(
            sparks(spark_seed(1, 2, 0), 40, 15),
            sparks(spark_seed(1, 3, 0), 40, 15)
        );
        assert_eq!(SparkShape::Ring.pixels().count(), 8);
        assert_eq!(SparkShape::Dot.pixels().collect::<Vec<_>>(), [(1, 1)]);
        assert_eq!(
            SparkShape::Slash.pixels().collect::<Vec<_>>(),
            [(2, 0), (1, 1), (0, 2)]
        );
    }
}
