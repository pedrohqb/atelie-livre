//! Microfone virtual: o driver de áudio `disk` da SDL3 lê a "captura" de um
//! arquivo cru (`SDL_AUDIO_DISK_INPUT_FILE`) e escreve o que tocaria em outro
//! (`SDL_AUDIO_DISK_OUTPUT_FILE`). Injeta um sinal conhecido e confere, por
//! código, o `.wav` gravado. Ver `docs/telas/fine-artist.md` (Gravador).

use crate::audio::{Mic, Mixer};
use engine::assets::{Assets, Program, SoundRef};
use engine::scene::Effect;
use engine::scenes::sounds::Hand;
use engine::scenes::sounds::recorder::{Recorder, Reply, SOUNDS_DIR};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const RATE: f64 = 22_050.0;

/// Senoide em `rate` Hz, mono, amplitude `amp` (fração do máximo).
fn sine(freq: f64, rate: f64, secs: f64, amp: f64) -> Vec<i16> {
    (0..(rate * secs) as usize)
        .map(|i| {
            let t = i as f64 / rate;
            (amp * 32767.0 * (2.0 * std::f64::consts::PI * freq * t).sin()) as i16
        })
        .collect()
}

/// Cru S16 nativo, `channels` canais iguais.
fn raw(samples: &[i16], channels: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(samples.len() * 2 * channels);
    for s in samples {
        for _ in 0..channels {
            v.extend_from_slice(&s.to_ne_bytes());
        }
    }
    v
}

/// Potência relativa da frequência `f` (Goertzel), normalizada pela energia.
fn tone_share(x: &[i16], rate: f64, f: f64) -> f64 {
    let w = 2.0 * std::f64::consts::PI * f / rate;
    let c = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &v in x {
        let s0 = v as f64 + c * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let p = s1 * s1 + s2 * s2 - c * s1 * s2;
    let e: f64 = x.iter().map(|&v| (v as f64).powi(2)).sum();
    if e == 0.0 {
        0.0
    } else {
        2.0 * p / (e * x.len() as f64)
    }
}

/// Frequência pelos cruzamentos de zero.
fn zero_cross_freq(x: &[i16], rate: f64) -> f64 {
    let n = x.windows(2).filter(|w| (w[0] < 0) != (w[1] < 0)).count();
    n as f64 / 2.0 / (x.len() as f64 / rate)
}

fn rms(x: &[i16]) -> f64 {
    (x.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt()
}

/// Maior correlação normalizada entre `x` e `r` com atraso 0..`max_lag`.
fn best_corr(x: &[i16], r: &[i16], max_lag: usize) -> (f64, usize) {
    let n = x.len().min(r.len()).saturating_sub(max_lag);
    let mut best = (0.0, 0);
    for lag in 0..max_lag {
        let a = &x[lag..lag + n];
        let b = &r[..n];
        let dot: f64 = a.iter().zip(b).map(|(&p, &q)| p as f64 * q as f64).sum();
        let na: f64 = a.iter().map(|&p| (p as f64).powi(2)).sum::<f64>().sqrt();
        let nb: f64 = b.iter().map(|&q| (q as f64).powi(2)).sum::<f64>().sqrt();
        let c = if na * nb == 0.0 { 0.0 } else { dot / (na * nb) };
        if c > best.0 {
            best = (c, lag);
        }
    }
    best
}

/// Lê um `.wav` PCM de 16 bits conferindo o cabeçalho de 44 bytes.
fn read_wav(path: &Path) -> (u32, u16, Vec<i16>) {
    let d = std::fs::read(path).unwrap();
    assert!(d.len() >= 44, "{} bytes", d.len());
    let u32_at = |i: usize| u32::from_le_bytes(d[i..i + 4].try_into().unwrap());
    let u16_at = |i: usize| u16::from_le_bytes(d[i..i + 2].try_into().unwrap());
    assert_eq!(&d[0..4], b"RIFF");
    assert_eq!(u32_at(4) as usize, d.len() - 8, "tamanho RIFF");
    assert_eq!(&d[8..16], b"WAVEfmt ");
    assert_eq!(u32_at(16), 16);
    assert_eq!(u16_at(20), 1, "PCM");
    let (ch, rate, bits) = (u16_at(22), u32_at(24), u16_at(34));
    assert_eq!(bits, 16);
    assert_eq!(u32_at(28), rate * ch as u32 * 2, "bytes por segundo");
    assert_eq!(u16_at(32), ch * 2, "alinhamento");
    assert_eq!(&d[36..40], b"data");
    assert_eq!(u32_at(40) as usize, d.len() - 44, "tamanho dos dados");
    let s = d[44..]
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    (rate, ch, s)
}

/// Centro dos botões do Gravador (`scenes/sounds/recorder.rs`).
const MIC: (i32, i32) = (236, 290);
const PLAY: (i32, i32) = (305, 289);
const NO: (i32, i32) = (368, 290);
const OK: (i32, i32) = (410, 289);

struct Rig {
    mixer: Mixer,
    mic: Mic,
    assets: Assets,
    input: PathBuf,
    output: PathBuf,
    /// Formato do microfone virtual (taxa, canais).
    dev: (i32, usize),
}

impl Rig {
    fn apply(&mut self, fx: Vec<Effect>) {
        for e in fx {
            if self.mic.handle(&e, Some(&self.mixer)) {
                continue;
            }
            // o MSKIDS de mentira não tem `.SND`: só os arquivos tocam
            if let Effect::PlaySound(r @ SoundRef::File(_)) = e {
                let pcm = self.assets.sound(&r).expect("gravação decodificável");
                self.mixer.play(&pcm).unwrap();
            }
        }
    }

    /// Põe `samples` (mono, na taxa do dispositivo) no microfone virtual;
    /// vale para o próximo `RecordStart`.
    fn inject(&self, samples: &[i16]) {
        std::fs::write(&self.input, raw(samples, self.dev.1)).unwrap();
    }

    /// Passa `real` de tempo, como o laço de quadros: recolhe o microfone e
    /// dá ao Gravador `dt` vezes `game` (0 = cena travada).
    fn pump(&mut self, rec: &mut Recorder, real: Duration, game: f32) {
        let t0 = Instant::now();
        let mut last = t0;
        while t0.elapsed() < real {
            std::thread::sleep(Duration::from_millis(16));
            let dt = last.elapsed().as_secs_f32() * game;
            last = Instant::now();
            self.mic.tick();
            let fx = rec.tick(dt).into_iter().collect();
            self.apply(fx);
        }
    }
}

fn click(rig: &mut Rig, rec: &mut Recorder, (x, y): (i32, i32)) -> Reply {
    let mut r = rec.click(x, y, &rig.assets);
    rig.apply(std::mem::take(&mut r.fx));
    r
}

fn temp_files(sons: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(sons)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with('.'))
                .collect()
        })
        .unwrap_or_default()
}

/// Reamostragem linear (para injetar um som do jogo na taxa do dispositivo).
fn resample(x: &[i16], from: f64, to: f64) -> Vec<i16> {
    let n = (x.len() as f64 * to / from) as usize;
    (0..n)
        .map(|i| {
            let p = i as f64 * from / to;
            let (k, f) = (p as usize, p.fract());
            let a = x[k.min(x.len() - 1)] as f64;
            let b = x[(k + 1).min(x.len() - 1)] as f64;
            (a + (b - a) * f) as i16
        })
        .collect()
}

/// Um único teste: a SDL só pode ser iniciada numa thread por processo.
#[test]
fn virtual_microphone_records_the_injected_signal() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("mic-in.raw");
    let output = tmp.path().join("alto-falante.raw");
    std::fs::write(&input, b"").unwrap();
    sdl3::hint::set("SDL_AUDIO_DRIVER", "disk");
    sdl3::hint::set("SDL_AUDIO_DISK_INPUT_FILE", input.to_str().unwrap());
    sdl3::hint::set("SDL_AUDIO_DISK_OUTPUT_FILE", output.to_str().unwrap());
    sdl3::hint::set("SDL_AUDIO_DISK_TIMESCALE", "1");
    let sdl = sdl3::init().unwrap();
    let mixer = Mixer::new(&sdl, 100).unwrap();
    // o dispositivo de captura abre no formato dele (a SDL reamostra)
    let dev = mixer.start_recording().unwrap().device_format();
    eprintln!("microfone virtual: {} Hz, {} canal(is)", dev.0, dev.1);
    let data = tmp.path().join("MSKIDS");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(data.join("ARTIST.EXE"), b"MZ").unwrap();
    let assets = Assets::open(&data).unwrap();
    let drawings = tmp.path().join("desenhos");
    let sons = drawings.join(SOUNDS_DIR);
    let mut rig = Rig {
        mixer,
        mic: Mic::default(),
        assets,
        input,
        output,
        dev,
    };
    let dev_rate = dev.0 as f64;

    // 1. senoide de 440 Hz pelo Gravador: microfone, 1,5 s, microfone
    rig.inject(&sine(440.0, dev_rate, 3.0, 0.5));
    let mut rec = Recorder::new(Some(&drawings));
    click(&mut rig, &mut rec, MIC);
    assert!(rig.mic.recording(), "o microfone virtual abriu");
    rig.pump(&mut rec, Duration::from_millis(1500), 1.0);
    click(&mut rig, &mut rec, MIC);
    assert!(!rig.mic.recording());
    let temp = rec.recorded().unwrap().to_path_buf();
    let (rate, ch, s) = read_wav(&temp);
    assert_eq!((rate, ch), (RATE as u32, 1));
    let secs = s.len() as f64 / RATE;
    eprintln!("gravado: {secs:.3} s, rms {:.0}", rms(&s));
    assert!((1.3..1.7).contains(&secs), "{secs} s");
    let body = &s[s.len() / 10..s.len() * 9 / 10];
    let f = zero_cross_freq(body, RATE);
    assert!((f - 440.0).abs() < 3.0, "{f} Hz");
    assert!(tone_share(body, RATE, 440.0) > 0.9);
    assert!(tone_share(body, RATE, 880.0) < 0.01);
    // amplitude 0,5 → rms ≈ 0,5·32767/√2
    let r = rms(body);
    assert!((r - 0.5 * 32767.0 / 2f64.sqrt()).abs() < 600.0, "rms {r}");
    let (c, lag) = best_corr(body, &sine(440.0, RATE, 1.0, 0.5), 60);
    assert!(c > 0.99, "correlação {c} (atraso {lag})");
    // termina em som (sem silêncio nem lixo no fim)
    let tail = &s[s.len() - 441..];
    assert!(rms(tail) > 0.9 * r, "fim {:.0} vs {r:.0}", rms(tail));

    // 2. TOCAR: o mixer toca a gravação (o "alto-falante" é um arquivo)
    let before = std::fs::metadata(&rig.output).unwrap().len() as usize;
    let out = click(&mut rig, &mut rec, PLAY);
    assert!(out.fx.is_empty() && !out.close);
    rig.pump(&mut rec, Duration::from_millis(1800), 1.0);
    let played = std::fs::read(&rig.output).unwrap();
    let (out_rate, out_ch) = rig.mixer.playback_format();
    let played: Vec<i16> = played[before & !3..]
        .chunks_exact(2 * out_ch)
        .map(|c| i16::from_ne_bytes([c[0], c[1]]))
        .collect();
    let loud: Vec<i16> = played.iter().copied().filter(|v| v.abs() > 2000).collect();
    let heard = loud.len() as f64 / out_rate as f64;
    eprintln!("tocado: {heard:.2} s de som em {out_rate} Hz");
    assert!(heard > 1.0, "{heard} s");
    let start = played.iter().position(|v| v.abs() > 2000).unwrap();
    let seg = &played[start..start + out_rate as usize];
    let f = zero_cross_freq(seg, out_rate as f64);
    assert!((f - 440.0).abs() < 3.0, "tocado a {f} Hz");

    // 3. nome e OK: o som vai para a mão com o arquivo renomeado
    rec.type_text("Galo doido");
    let out = click(&mut rig, &mut rec, OK);
    assert!(out.close);
    let file = sons.join("Galo_doido.wav");
    assert_eq!(
        out.chosen,
        Some(Hand::Sound {
            name: "Galo doido".into(),
            sound: SoundRef::File(file.clone()),
        })
    );
    assert!(!temp.exists() && temp_files(&sons).is_empty());
    let pcm = rig.assets.sound(&SoundRef::File(file.clone())).unwrap();
    assert_eq!((pcm.sample_rate, pcm.channels), (22_050, 1));
    assert_eq!(pcm.samples, s, "o que o jogo lê é o que foi gravado");

    // 4. NÃO no meio da gravação: nada fica para trás
    let mut rec = Recorder::new(Some(&drawings));
    click(&mut rig, &mut rec, MIC);
    rig.pump(&mut rec, Duration::from_millis(300), 1.0);
    let out = click(&mut rig, &mut rec, NO);
    assert!(out.close && !rig.mic.recording());
    rig.pump(&mut rec, Duration::from_millis(100), 1.0);
    assert_eq!(temp_files(&sons), Vec::<PathBuf>::new());

    // 5. gravação longa (microfone 10× mais rápido): 440 Hz até 8 s, depois
    // 880 Hz. Com a cena travada (dt 0) a gravação passa de 10 s de áudio e
    // o arquivo fica nos 10 s da barra de TEMPO, sem buracos.
    sdl3::hint::set("SDL_AUDIO_DISK_TIMESCALE", "0.1");
    let mut long = sine(440.0, dev_rate, 8.0, 0.5);
    long.extend(sine(880.0, dev_rate, 8.0, 0.5));
    rig.inject(&long);
    let mut rec = Recorder::new(Some(&drawings));
    click(&mut rig, &mut rec, MIC);
    rig.pump(&mut rec, Duration::from_millis(2500), 0.0);
    assert!(rec.recording());
    click(&mut rig, &mut rec, MIC);
    let (_, _, s) = read_wav(rec.recorded().unwrap());
    assert_eq!(s.len(), 10 * 22_050, "teto de 10 s");
    let at = |t: f64| &s[(t * RATE) as usize..((t + 0.2) * RATE) as usize];
    for t in [0.5, 4.0, 7.5] {
        assert!((zero_cross_freq(at(t), RATE) - 440.0).abs() < 5.0, "{t} s");
    }
    for t in [8.3, 9.7] {
        assert!((zero_cross_freq(at(t), RATE) - 880.0).abs() < 5.0, "{t} s");
    }
    // a troca de frequência cai em 8 s: nenhuma amostra perdida no caminho
    let switch = (0..s.len() - 441)
        .step_by(110)
        .find(|&i| zero_cross_freq(&s[i..i + 441], RATE) > 660.0)
        .unwrap() as f64
        / RATE;
    assert!((switch - 8.0).abs() < 0.03, "troca em {switch} s");
    // sem travar, o próprio Gravador para nos 10 s
    let mut rec = Recorder::new(Some(&drawings));
    click(&mut rig, &mut rec, MIC);
    let t0 = Instant::now();
    while rec.recording() && t0.elapsed() < Duration::from_secs(5) {
        rig.pump(&mut rec, Duration::from_millis(50), 10.0);
    }
    assert!(!rec.recording() && !rig.mic.recording());
    let (_, _, s) = read_wav(rec.recorded().unwrap());
    assert!(s.len() > 22_050 * 5, "{} amostras", s.len());
    sdl3::hint::set("SDL_AUDIO_DISK_TIMESCALE", "1");

    // 6. um som do próprio jogo (balão do Gravador, 0224) no microfone
    let Some(real) = std::env::var_os("ATELIE_DATA") else {
        eprintln!("aviso: ATELIE_DATA não definida; som do jogo pulado");
        return;
    };
    let game = Assets::open(Path::new(&real)).unwrap();
    let pcm = game.sound(&SoundRef::Snd(Program::Artist, 0x0224)).unwrap();
    assert_eq!(pcm.channels, 1);
    let secs = pcm.samples.len() as f64 / pcm.sample_rate as f64;
    eprintln!("som 0224: {} Hz, {secs:.2} s", pcm.sample_rate);
    rig.inject(&resample(&pcm.samples, pcm.sample_rate as f64, dev_rate));
    let mut rec = Recorder::new(Some(&drawings));
    click(&mut rig, &mut rec, MIC);
    rig.pump(&mut rec, Duration::from_secs_f64(secs + 0.4), 1.0);
    click(&mut rig, &mut rec, MIC);
    let (_, _, s) = read_wav(rec.recorded().unwrap());
    let reference = resample(&pcm.samples, pcm.sample_rate as f64, RATE);
    let (c, lag) = best_corr(&s, &reference, 400);
    eprintln!("som 0224 gravado: correlação {c:.4}, atraso {lag}");
    assert!(c > 0.95, "correlação {c}");
}
