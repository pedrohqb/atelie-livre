//! Mistura de sons: um fluxo de áudio da SDL3 por som em reprodução, mais um
//! som em laço (traços do pincel, aspirador) alimentado aos poucos.

use engine::Pcm;
use sdl3::AudioSubsystem;
use sdl3::audio::{AudioDevice, AudioFormat, AudioSpec, AudioStreamOwner};
use std::collections::VecDeque;
use std::rc::Rc;

const MAX_STREAMS: usize = 8;

/// Fração de segundo que o laço mantém na fila; pouco, para soltar o botão
/// e ouvir logo o fim do som.
const LOOP_AHEAD_DIV: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopState {
    /// Nada enfileirado ainda.
    Fresh,
    /// Começo enfileirado; repete o trecho enquanto ativo.
    Looping,
    /// Solto: falta enfileirar o fim.
    Released,
    Done,
}

/// Lógica do som em laço, sem SDL: diz que trecho de amostras pôr na fila.
///
/// Toca do começo até o fim do trecho de repetição, repete o trecho enquanto
/// ativo e, ao ser solto, toca o resto do som (ver `docs/formats/sons.md`).
pub struct Looper {
    pcm: Rc<Pcm>,
    state: LoopState,
    released: bool,
}

impl Looper {
    pub fn new(pcm: Rc<Pcm>) -> Looper {
        Looper {
            pcm,
            state: LoopState::Fresh,
            released: false,
        }
    }

    /// Trecho (`início..fim`) em amostras intercaladas.
    fn span(&self) -> Option<(usize, usize)> {
        let ch = self.pcm.channels.max(1) as usize;
        let (a, b) = self.pcm.loop_points?;
        let (a, b) = (a as usize * ch, b as usize * ch);
        (a < b && b <= self.pcm.samples.len()).then_some((a, b))
    }

    /// Próximo pedaço a enfileirar, dado quanto ainda está na fila (em quadros);
    /// `None` quando não há o que pôr agora.
    pub fn feed(&mut self, queued_frames: usize) -> Option<&[i16]> {
        let s = &self.pcm.samples;
        let Some((a, b)) = self.span() else {
            // sem trecho de repetição: toca o som inteiro uma vez
            return match self.state {
                LoopState::Done => None,
                _ => {
                    self.state = LoopState::Done;
                    Some(s)
                }
            };
        };
        match self.state {
            LoopState::Fresh if self.released => {
                self.state = LoopState::Done;
                Some(s)
            }
            LoopState::Fresh => {
                self.state = LoopState::Looping;
                Some(&s[..b])
            }
            LoopState::Looping => {
                let ahead = self.pcm.sample_rate as usize / LOOP_AHEAD_DIV;
                (queued_frames < ahead).then(|| &s[a..b])
            }
            LoopState::Released => {
                self.state = LoopState::Done;
                (b < s.len()).then(|| &s[b..])
            }
            LoopState::Done => None,
        }
    }

    /// Para de repetir: o próximo `feed` dá o fim do som.
    pub fn release(&mut self) {
        self.released = true;
        if self.state == LoopState::Looping {
            self.state = LoopState::Released;
        }
    }

    pub fn finished(&self) -> bool {
        self.state == LoopState::Done
    }
}

pub struct Mixer {
    audio: AudioSubsystem,
    device: AudioDevice,
    streams: VecDeque<AudioStreamOwner>,
    /// Som em laço e o fluxo que ele alimenta.
    looping: Option<(Looper, AudioStreamOwner)>,
    gain: f32,
}

impl Mixer {
    pub fn new(sdl: &sdl3::Sdl, volume: u8) -> Result<Mixer, String> {
        let audio = sdl.audio().map_err(|e| e.to_string())?;
        let device = audio
            .open_playback_device(&AudioSpec::new(None, None, None))
            .map_err(|e| e.to_string())?;
        Ok(Mixer {
            audio,
            device,
            streams: VecDeque::new(),
            looping: None,
            gain: volume as f32 / 100.0,
        })
    }

    fn stream(&self, pcm: &Pcm) -> Result<AudioStreamOwner, String> {
        let spec = AudioSpec::new(
            Some(pcm.sample_rate as i32),
            Some(pcm.channels as i32),
            Some(AudioFormat::S16LE),
        );
        let stream = self
            .audio
            .new_stream(Some(&spec), None)
            .map_err(|e| e.to_string())?;
        self.device
            .bind_stream(&stream)
            .map_err(|e| e.to_string())?;
        stream.set_gain(self.gain).map_err(|e| e.to_string())?;
        Ok(stream)
    }

    /// Guarda um fluxo de som avulso; com 8 em reprodução, o mais antigo é interrompido.
    fn keep(&mut self, stream: AudioStreamOwner) {
        self.streams
            .retain(|s| s.available_bytes().map(|n| n > 0).unwrap_or(false));
        while self.streams.len() >= MAX_STREAMS {
            self.streams.pop_front();
        }
        self.streams.push_back(stream);
    }

    /// Toca o som uma vez.
    pub fn play(&mut self, pcm: &Pcm) -> Result<(), String> {
        let stream = self.stream(pcm)?;
        stream
            .put_data_i16(&pcm.samples)
            .map_err(|e| e.to_string())?;
        self.keep(stream);
        Ok(())
    }

    /// Começa um som em laço; o laço anterior termina (toca o fim) por conta própria.
    pub fn play_loop(&mut self, pcm: Rc<Pcm>) -> Result<(), String> {
        self.stop_loop();
        let stream = self.stream(&pcm)?;
        self.looping = Some((Looper::new(pcm), stream));
        self.tick()
    }

    /// Solta o laço: termina a repetição em curso e toca o fim do som.
    pub fn stop_loop(&mut self) {
        if let Some((mut l, s)) = self.looping.take() {
            l.release();
            while let Some(chunk) = l.feed(usize::MAX) {
                if s.put_data_i16(chunk).is_err() {
                    break;
                }
            }
            self.keep(s);
        }
    }

    /// Mantém a fila do laço abastecida; chamar a cada quadro.
    pub fn tick(&mut self) -> Result<(), String> {
        let Some((l, s)) = self.looping.as_mut() else {
            return Ok(());
        };
        let ch = l.pcm.channels.max(1) as usize;
        let mut queued = s.queued_bytes().map_err(|e| e.to_string())?.max(0) as usize / (2 * ch);
        while let Some(chunk) = l.feed(queued) {
            queued += chunk.len() / ch;
            s.put_data_i16(chunk).map_err(|e| e.to_string())?;
        }
        if l.finished()
            && let Some((_, s)) = self.looping.take()
        {
            self.keep(s);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(samples: Vec<i16>, loop_points: Option<(u32, u32)>) -> Rc<Pcm> {
        Rc::new(Pcm {
            sample_rate: 80,
            channels: 1,
            samples,
            loop_points,
        })
    }

    /// Simula o tocador: a fila esvazia `drain` quadros por quadro de tela.
    fn run(l: &mut Looper, frames: usize, drain: usize, release_at: usize) -> Vec<i16> {
        let mut out = Vec::new();
        let mut queued = 0usize;
        for f in 0..frames {
            if f == release_at {
                l.release();
            }
            while let Some(c) = l.feed(queued) {
                queued += c.len();
                out.extend_from_slice(c);
            }
            queued = queued.saturating_sub(drain);
        }
        out
    }

    #[test]
    fn loop_plays_intro_repeats_then_tail() {
        // som 0..10, trecho 2..5; fila mínima = 80/8 = 10 quadros
        let p = pcm((0..10).collect(), Some((2, 5)));
        let mut l = Looper::new(p);
        let out = run(&mut l, 10, 6, 3);
        assert_eq!(&out[..5], &[0, 1, 2, 3, 4]);
        assert!(out[5..out.len() - 5].chunks(3).all(|c| c == [2, 3, 4]));
        assert_eq!(&out[out.len() - 5..], &[5, 6, 7, 8, 9]);
        assert!(l.finished());
    }

    #[test]
    fn loop_keeps_repeating_while_held() {
        let p = pcm((0..10).collect(), Some((0, 4)));
        let mut l = Looper::new(p);
        let out = run(&mut l, 50, 4, usize::MAX);
        assert!(out.len() >= 4 * 40, "{}", out.len());
        assert!(!l.finished());
        // fila nunca passa muito do mínimo
        assert!(l.feed(1000).is_none());
    }

    #[test]
    fn release_before_start_plays_whole_sound_once() {
        let p = pcm((0..10).collect(), Some((2, 5)));
        let mut l = Looper::new(p);
        l.release();
        assert_eq!(l.feed(0), Some(&(0..10).collect::<Vec<_>>()[..]));
        assert_eq!(l.feed(0), None);
        assert!(l.finished());
    }

    #[test]
    fn sound_without_loop_plays_once() {
        let p = pcm(vec![1, 2, 3], None);
        let mut l = Looper::new(p);
        assert_eq!(l.feed(0), Some(&[1, 2, 3][..]));
        assert_eq!(l.feed(0), None);
        l.release();
        assert_eq!(l.feed(0), None);
    }

    #[test]
    fn loop_to_the_end_has_no_tail() {
        let p = pcm((0..6).collect(), Some((3, 6)));
        let mut l = Looper::new(p);
        assert_eq!(l.feed(0), Some(&[0, 1, 2, 3, 4, 5][..]));
        l.release();
        assert_eq!(l.feed(0), None);
        assert!(l.finished());
    }

    #[test]
    fn stereo_loop_points_count_frames() {
        let p = Rc::new(Pcm {
            sample_rate: 80,
            channels: 2,
            samples: (0..8).collect(),
            loop_points: Some((1, 3)),
        });
        let mut l = Looper::new(p);
        assert_eq!(l.feed(0), Some(&[0, 1, 2, 3, 4, 5][..]));
        assert_eq!(l.feed(0), Some(&[2, 3, 4, 5][..]));
        l.release();
        assert_eq!(l.feed(0), Some(&[6, 7][..]));
    }
}
