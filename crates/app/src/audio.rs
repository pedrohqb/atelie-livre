//! Mistura de sons: um fluxo de áudio da SDL3 por som em reprodução, mais um
//! som em laço (traços do pincel, aspirador) alimentado aos poucos. Também
//! grava do microfone (Gravador do Seleciona-sons) num `.wav`.

use engine::Pcm;
use engine::scene::Effect;
use sdl3::AudioSubsystem;
use sdl3::audio::{AudioDevice, AudioFormat, AudioSpec, AudioStreamOwner};
use std::collections::VecDeque;
use std::io::Read;
use std::rc::Rc;

const MAX_STREAMS: usize = 8;
/// Taxa da gravação do microfone (a dos sons do Fine Artist).
const RECORD_RATE: u32 = 22_050;
/// Limite de amostras de uma gravação: os 10 s da barra de TEMPO do Gravador
/// (o tempo da cena pode atrasar em relação ao relógio do áudio).
const RECORD_MAX: usize =
    (RECORD_RATE as f32 * engine::scenes::sounds::recorder::MAX_SECONDS) as usize;

/// Gravação do microfone em andamento.
pub struct Recording {
    // o fluxo sai antes do dispositivo
    stream: AudioStreamOwner,
    device: AudioDevice,
    samples: Vec<i16>,
}

impl Recording {
    /// Taxa e canais do dispositivo de captura (antes da conversão).
    #[cfg(test)]
    pub fn device_format(&self) -> (i32, usize) {
        let spec = self.device.format().map(|(s, _)| s).ok();
        (
            spec.as_ref().and_then(|s| s.freq).unwrap_or(0),
            spec.as_ref().and_then(|s| s.channels).unwrap_or(0).max(0) as usize,
        )
    }

    /// Recolhe o que o microfone mandou; chamar a cada quadro. O que passa
    /// do limite é lido e jogado fora (não acumula na fila da SDL).
    pub fn tick(&mut self) {
        let mut buf = [0u8; 8192];
        loop {
            // o fluxo entrega S16LE mono (formato pedido em `start_recording`)
            let n = match self.stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let room = RECORD_MAX - self.samples.len();
            self.samples.extend(
                buf[..n & !1]
                    .chunks_exact(2)
                    .take(room)
                    .map(|c| i16::from_le_bytes([c[0], c[1]])),
            );
        }
    }

    /// Para e grava o `.wav` (PCM de 16 bits, mono, 22 050 Hz).
    pub fn finish(mut self, path: &std::path::Path) -> std::io::Result<()> {
        // para de captar e esvazia o conversor (as últimas amostras
        // ficam retidas no reamostrador até o flush)
        self.device.pause();
        self.tick();
        let _ = self.stream.flush();
        self.tick();
        let pcm = Pcm {
            sample_rate: RECORD_RATE,
            channels: 1,
            samples: std::mem::take(&mut self.samples),
            loop_points: None,
        };
        std::fs::write(path, engine::encode_pcm16(&pcm))
    }
}

/// Microfone do Gravador: trata `RecordStart`, `RecordStop` e `RecordCancel`.
#[derive(Default)]
pub struct Mic {
    rec: Option<Recording>,
}

impl Mic {
    /// Aplica um efeito de gravação; falso se `e` não é de gravação.
    pub fn handle(&mut self, e: &Effect, mixer: Option<&Mixer>) -> bool {
        match e {
            Effect::RecordStart => {
                self.rec = None;
                match mixer.map(|m| m.start_recording()) {
                    Some(Ok(r)) => self.rec = Some(r),
                    Some(Err(e)) => eprintln!("aviso: microfone: {e}"),
                    None => eprintln!("aviso: sem áudio, nada a gravar"),
                }
            }
            Effect::RecordStop(path) => {
                if let Some(r) = self.rec.take()
                    && let Err(e) = r.finish(path)
                {
                    eprintln!("aviso: gravação {}: {e}", path.display());
                }
            }
            Effect::RecordCancel => self.cancel(),
            _ => return false,
        }
        true
    }

    /// Recolhe o que o microfone mandou; chamar a cada quadro.
    pub fn tick(&mut self) {
        if let Some(r) = self.rec.as_mut() {
            r.tick();
        }
    }

    /// Descarta a gravação em andamento.
    pub fn cancel(&mut self) {
        self.rec = None;
    }

    #[cfg(test)]
    pub fn recording(&self) -> bool {
        self.rec.is_some()
    }
}

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

    /// Volume (0–100) dos sons seguintes e dos que estão tocando.
    pub fn set_volume(&mut self, volume: u8) {
        self.gain = volume.min(100) as f32 / 100.0;
        for s in self
            .streams
            .iter()
            .chain(self.looping.iter().map(|(_, s)| s))
        {
            let _ = s.set_gain(self.gain);
        }
    }

    /// Taxa e canais do dispositivo de saída.
    #[cfg(test)]
    pub fn playback_format(&self) -> (i32, usize) {
        let spec = self.device.format().map(|(s, _)| s).ok();
        (
            spec.as_ref().and_then(|s| s.freq).unwrap_or(0),
            spec.as_ref().and_then(|s| s.channels).unwrap_or(0).max(0) as usize,
        )
    }

    /// Começa a gravar do microfone padrão (mono, 22 050 Hz, 16 bits).
    pub fn start_recording(&self) -> Result<Recording, String> {
        let spec = AudioSpec::new(Some(RECORD_RATE as i32), Some(1), Some(AudioFormat::S16LE));
        let device = self
            .audio
            .open_recording_device(&spec)
            .map_err(|e| e.to_string())?;
        let stream = self
            .audio
            .new_stream(None, Some(&spec))
            .map_err(|e| e.to_string())?;
        device.bind_stream(&stream).map_err(|e| e.to_string())?;
        stream.resume().map_err(|e| e.to_string())?;
        Ok(Recording {
            stream,
            device,
            samples: Vec::new(),
        })
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
