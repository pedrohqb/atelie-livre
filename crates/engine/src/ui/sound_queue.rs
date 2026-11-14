//! Sons em sequência: o primeiro toca já, cada um dos outros quando o
//! anterior acaba (pela duração do PCM). Usado onde o original encadeia
//! vinhetas (por ex. "Tchau" + "EPA!" na Galeria, passos da Entrada).

use crate::assets::{Assets, SoundRef};
use crate::scene::Effect;
use formats::wave::Pcm;
use std::collections::VecDeque;

/// Duração presumida de um som que não abre.
const UNKNOWN_SECS: f32 = 0.5;

/// Duração do som em segundos.
pub fn pcm_seconds(p: &Pcm) -> f32 {
    p.samples.len() as f32 / (p.sample_rate.max(1) as f32 * p.channels.max(1) as f32)
}

#[derive(Debug, Default)]
pub struct SoundQueue {
    queue: VecDeque<SoundRef>,
    wait: f32,
}

impl SoundQueue {
    pub fn new() -> SoundQueue {
        SoundQueue::default()
    }

    /// Troca a sequência pendente por esta (uma ação nova interrompe a
    /// anterior); o primeiro som sai no próximo [`SoundQueue::pump`].
    pub fn play(&mut self, sounds: impl IntoIterator<Item = SoundRef>) {
        self.queue.clear();
        self.queue.extend(sounds);
        self.wait = 0.0;
    }

    /// Passa o tempo; põe em `fx` o próximo som quando o anterior acabou.
    pub fn pump(&mut self, dt: f32, fx: &mut Vec<Effect>, assets: &Assets) {
        self.wait -= dt;
        if self.wait > 0.0 {
            return;
        }
        if let Some(r) = self.queue.pop_front() {
            self.wait = assets.sound(&r).map_or(UNKNOWN_SECS, |p| pcm_seconds(&p));
            fx.push(Effect::PlaySound(r));
        } else {
            self.wait = 0.0;
        }
    }

    /// Nada pendente.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Program;

    fn snd(id: u16) -> SoundRef {
        SoundRef::Snd(Program::Artist, id)
    }

    #[test]
    fn plays_one_after_the_other() {
        let (_d, a) = crate::scenes::testutil::empty_assets();
        let mut q = SoundQueue::new();
        q.play([snd(1), snd(2)]);
        let mut fx = Vec::new();
        q.pump(0.0, &mut fx, &a);
        assert_eq!(fx, vec![Effect::PlaySound(snd(1))]);
        // sem os dados, cada som dura meio segundo
        q.pump(0.3, &mut fx, &a);
        assert_eq!(fx.len(), 1);
        q.pump(0.3, &mut fx, &a);
        assert_eq!(fx.last(), Some(&Effect::PlaySound(snd(2))));
        assert!(q.is_empty());
    }

    #[test]
    fn a_new_sequence_replaces_the_pending_one() {
        let (_d, a) = crate::scenes::testutil::empty_assets();
        let mut q = SoundQueue::new();
        q.play([snd(1), snd(2)]);
        let mut fx = Vec::new();
        q.pump(0.0, &mut fx, &a);
        q.play([snd(3)]);
        q.pump(0.0, &mut fx, &a);
        q.pump(1.0, &mut fx, &a);
        assert_eq!(
            fx,
            vec![Effect::PlaySound(snd(1)), Effect::PlaySound(snd(3))]
        );
    }

    #[test]
    fn duration_counts_channels() {
        let p = Pcm {
            samples: vec![0; 22050 * 2],
            sample_rate: 22050,
            channels: 2,
            loop_points: None,
        };
        assert!((pcm_seconds(&p) - 1.0).abs() < 1e-6);
    }
}
