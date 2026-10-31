//! Mistura de sons: um fluxo de áudio da SDL3 por som em reprodução.

use engine::Pcm;
use sdl3::AudioSubsystem;
use sdl3::audio::{AudioDevice, AudioFormat, AudioSpec, AudioStreamOwner};
use std::collections::VecDeque;

const MAX_STREAMS: usize = 8;

pub struct Mixer {
    audio: AudioSubsystem,
    device: AudioDevice,
    streams: VecDeque<AudioStreamOwner>,
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
            gain: volume as f32 / 100.0,
        })
    }

    /// Toca o som; com 8 em reprodução, o mais antigo é interrompido.
    pub fn play(&mut self, pcm: &Pcm) -> Result<(), String> {
        self.streams
            .retain(|s| s.available_bytes().map(|n| n > 0).unwrap_or(false));
        while self.streams.len() >= MAX_STREAMS {
            self.streams.pop_front();
        }
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
        stream
            .put_data_i16(&pcm.samples)
            .map_err(|e| e.to_string())?;
        self.streams.push_back(stream);
        Ok(())
    }
}
