//! Gravador do Seleciona-sons: barra de TEMPO, campo "Nome do Som >",
//! microfone (gravar/parar), TOCAR, NÃO e OK. A captura do microfone é da
//! plataforma (`Effect::RecordStart` / `RecordStop`, que grava um `.wav`);
//! aqui ficam o tempo, o nome e o arquivo: a gravação vai para
//! `<desenhos>/sons/.gravacao-<n>.wav` e o OK a renomeia para
//! `<desenhos>/sons/<nome>.wav` e põe o som na mão. Geometria medida no
//! original (varredura `paint-sounds`, `paint-sounds_orig_10_record.png`).

use super::finder::dim;
use super::{Hand, Rect, inside, pic, snd};
use crate::Frame;
use crate::assets::{Assets, SoundRef};
use crate::paths::safe_component;
use crate::scene::{Effect, Key};
use std::path::{Path, PathBuf};

/// Subpasta da pasta de desenhos com os sons gravados.
pub const SOUNDS_DIR: &str = "sons";
/// Prefixo das gravações ainda sem nome.
const TEMP_PREFIX: &str = ".gravacao-";
/// Tempo máximo de uma gravação, em segundos.
pub const MAX_SECONDS: f32 = 10.0;
const MAX_NAME: usize = 30;
const DEFAULT_NAME: &str = "Meu som";

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const ORANGE: [u8; 3] = [255, 140, 50];
const GRAY: [u8; 3] = [140, 140, 140];
const LILAC: [u8; 3] = [136, 136, 230];
const BLUE: [u8; 3] = [97, 65, 219];
const YELLOW: [u8; 3] = [255, 238, 76];
const RED: [u8; 3] = [250, 74, 114];

const FRAME: Rect = (201, 164, 237, 152);
const TIME_PANEL: Rect = (306, 174, 117, 57);
const TIME_BAR: Rect = (313, 186, 98, 26);
const TIME_LABEL: Rect = (309, 215, 64, 15);
const NAME_LABEL: Rect = (205, 241, 113, 24);
const NAME_FIELD: Rect = (321, 241, 113, 24);
const LEFT_PANEL: Rect = (205, 268, 140, 44);
const RIGHT_PANEL: Rect = (348, 268, 86, 44);
/// Botões (imagem colorida, máscara, posição, tamanho).
const MIC: (u16, u16, Rect) = (0x02d3, 0x02d5, (216, 272, 41, 36));
const PLAY: (u16, u16, Rect) = (0x02d7, 0x02d9, (272, 272, 67, 35));
const NO: (u16, u16, Rect) = (0x0271, 0x0273, (354, 276, 28, 28));
const OK: (u16, u16, Rect) = (0x026d, 0x026f, (397, 276, 26, 27));

/// Resposta a um clique ou tecla no Gravador.
#[derive(Debug, Default, PartialEq)]
pub struct Reply {
    pub fx: Vec<Effect>,
    pub close: bool,
    pub chosen: Option<Hand>,
}

pub struct Recorder {
    /// Pasta `sons` (sem pasta de desenhos, não há onde gravar).
    dir: Option<PathBuf>,
    recording: bool,
    elapsed: f32,
    /// Duração da última gravação (para a barra de TEMPO).
    length: f32,
    take: u32,
    /// Arquivo da última gravação, se houve.
    recorded: Option<PathBuf>,
    pub name: String,
}

impl Recorder {
    /// Abre o Gravador; limpa gravações sem nome de vezes anteriores.
    pub fn new(drawings: Option<&Path>) -> Recorder {
        let dir = drawings.map(|d| d.join(SOUNDS_DIR));
        if let Some(d) = &dir
            && let Ok(rd) = std::fs::read_dir(d)
        {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().starts_with(TEMP_PREFIX) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        Recorder {
            dir,
            recording: false,
            elapsed: 0.0,
            length: 0.0,
            take: 0,
            recorded: None,
            name: String::new(),
        }
    }

    pub fn recording(&self) -> bool {
        self.recording
    }

    pub fn recorded(&self) -> Option<&Path> {
        self.recorded.as_deref()
    }

    fn temp_path(&self) -> Option<PathBuf> {
        let d = self.dir.as_ref()?;
        Some(d.join(format!(
            "{TEMP_PREFIX}{}-{}.wav",
            std::process::id(),
            self.take
        )))
    }

    /// Efeito que para a gravação em andamento.
    pub fn stop_effect(&self) -> Option<Effect> {
        if !self.recording {
            return None;
        }
        self.recorded.clone().map(Effect::RecordStop)
    }

    fn stop(&mut self) -> Option<Effect> {
        let e = self.stop_effect();
        if self.recording {
            self.recording = false;
            self.length = self.elapsed;
        }
        e
    }

    fn start(&mut self) -> Vec<Effect> {
        let Some(d) = self.dir.clone() else {
            return vec![Effect::Log(
                "gravador: sem pasta de desenhos, nada a gravar".into(),
            )];
        };
        if let Err(e) = std::fs::create_dir_all(&d) {
            return vec![Effect::Log(format!("gravador: {}: {e}", d.display()))];
        }
        if let Some(old) = self.recorded.take() {
            let _ = std::fs::remove_file(old);
        }
        self.take += 1;
        self.recorded = self.temp_path();
        self.recording = true;
        self.elapsed = 0.0;
        self.length = 0.0;
        vec![Effect::RecordStart]
    }

    /// Tempo passando: a gravação para sozinha em `MAX_SECONDS`.
    pub fn tick(&mut self, dt: f32) -> Option<Effect> {
        if !self.recording || !dt.is_finite() {
            return None;
        }
        self.elapsed += dt.max(0.0);
        if self.elapsed >= MAX_SECONDS {
            self.elapsed = MAX_SECONDS;
            return self.stop();
        }
        None
    }

    /// Texto digitado no campo do nome.
    pub fn type_text(&mut self, t: &str) {
        for c in t.chars() {
            if self.name.chars().count() >= MAX_NAME {
                break;
            }
            if !c.is_control() && !matches!(c, '/' | '\\') {
                self.name.push(c);
            }
        }
    }

    /// Tecla: Backspace apaga, Enter é o OK, Esc é o NÃO.
    pub fn key(&mut self, key: Key, assets: &Assets) -> Reply {
        match key {
            Key::Backspace => {
                self.name.pop();
                Reply::default()
            }
            Key::Enter => self.ok(assets),
            Key::Escape => self.no(),
            _ => Reply::default(),
        }
    }

    fn no(&mut self) -> Reply {
        let mut r = Reply {
            close: true,
            ..Reply::default()
        };
        r.fx.extend(self.stop());
        // a gravação ainda pode ser escrita depois do RecordStop: o arquivo
        // some na próxima abertura do Gravador
        if let Some(p) = self.recorded.take() {
            let _ = std::fs::remove_file(p);
        }
        r.fx.push(snd(super::SND_CLOSE));
        r
    }

    fn ok(&mut self, assets: &Assets) -> Reply {
        let mut r = Reply::default();
        if self.recording {
            // primeiro para; o arquivo só existe depois do RecordStop
            r.fx.extend(self.stop());
            return r;
        }
        r.close = true;
        let (Some(temp), Some(dir)) = (self.recorded.take(), self.dir.clone()) else {
            return r;
        };
        if !temp.is_file() {
            r.fx.push(Effect::Log("gravador: nada foi gravado".into()));
            return r;
        }
        let name = match self.name.trim() {
            "" => DEFAULT_NAME.to_string(),
            n => n.to_string(),
        };
        let path = dir.join(format!("{}.wav", safe_component(&name)));
        match std::fs::rename(&temp, &path) {
            Ok(()) => {
                let sound = SoundRef::File(path);
                assets.forget_sound(&sound);
                r.chosen = Some(Hand::Sound { name, sound });
            }
            Err(e) => {
                r.fx.push(Effect::Log(format!("gravador: {}: {e}", path.display())))
            }
        }
        r
    }

    pub fn click(&mut self, x: i32, y: i32, assets: &Assets) -> Reply {
        let mut r = Reply::default();
        if inside(MIC.2, x, y) {
            if self.recording {
                r.fx.extend(self.stop());
            } else {
                r.fx.extend(self.start());
            }
        } else if inside(PLAY.2, x, y) {
            if self.recording {
                r.fx.extend(self.stop());
            } else if let Some(p) = &self.recorded {
                let sound = SoundRef::File(p.clone());
                assets.forget_sound(&sound);
                r.fx.push(Effect::PlaySound(sound));
            }
        } else if inside(NO.2, x, y) {
            return self.no();
        } else if inside(OK.2, x, y) {
            return self.ok(assets);
        }
        r
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let font = a.font(8);
        let text = |f: &mut Frame, x: i32, y: i32, s: &str| match &font {
            Some(fo) => {
                f.text_font(fo, x, y, s, BLACK);
            }
            None => f.text(x, y, s, BLACK),
        };
        let boxed = |f: &mut Frame, r: Rect, c: [u8; 3]| {
            f.rect(r.0, r.1, r.2, r.3, BLACK);
            f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, c);
        };
        dim(f);
        let fr = FRAME;
        boxed(f, fr, ORANGE);
        boxed(f, (fr.0 + 3, fr.1 + 3, fr.2 - 6, fr.3 - 6), GRAY);
        // TEMPO: painel azul com rebites e a barra que enche
        boxed(f, TIME_PANEL, LILAC);
        for (dx, dy) in [(8, 4), (46, 4), (84, 4), (108, 4), (108, 46)] {
            f.rect(TIME_PANEL.0 + dx, TIME_PANEL.1 + dy, 4, 4, BLACK);
        }
        boxed(f, TIME_BAR, YELLOW);
        let shown = if self.recording {
            self.elapsed
        } else {
            self.length
        };
        let w = ((TIME_BAR.2 - 2) as f32 * (shown / MAX_SECONDS).clamp(0.0, 1.0)) as i32;
        f.rect(
            TIME_BAR.0 + 1,
            TIME_BAR.1 + 1,
            w,
            TIME_BAR.3 - 2,
            if self.recording { RED } else { BLUE },
        );
        boxed(f, TIME_LABEL, LILAC);
        text(f, TIME_LABEL.0 + 4, TIME_LABEL.1 + 1, "TEMPO");
        boxed(f, NAME_LABEL, GRAY);
        text(f, NAME_LABEL.0 + 6, NAME_LABEL.1 + 6, "Nome do Som >");
        boxed(f, NAME_FIELD, WHITE);
        let caret = match &font {
            Some(fo) => {
                let name =
                    super::finder::clip_text(&self.name, |s| fo.width(s) as i32, NAME_FIELD.2 - 10);
                f.text_font(fo, NAME_FIELD.0 + 4, NAME_FIELD.1 + 6, &name, BLACK)
            }
            None => {
                f.text(NAME_FIELD.0 + 4, NAME_FIELD.1 + 8, &self.name, BLACK);
                8 * self.name.chars().count() as i32
            }
        };
        f.rect(
            NAME_FIELD.0 + 4 + caret,
            NAME_FIELD.1 + 4,
            1,
            NAME_FIELD.3 - 8,
            BLACK,
        );
        boxed(f, LEFT_PANEL, GRAY);
        boxed(f, RIGHT_PANEL, GRAY);
        for (img, mask, r) in [MIC, PLAY, NO, OK] {
            f.blit_masked(&a.image(&pic(img)), &a.image(&pic(mask)), r.0, r.1);
        }
        if self.recording {
            // gravando: moldura vermelha no microfone
            f.frame_rect(MIC.2.0 - 2, MIC.2.1 - 2, MIC.2.2 + 4, MIC.2.3 + 4, 2, RED);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(r: Rect) -> (i32, i32) {
        (r.0 + r.2 / 2, r.1 + r.3 / 2)
    }

    #[test]
    fn record_stop_play_name_and_ok() {
        let (_a, assets) = crate::scenes::testutil::empty_assets();
        let d = tempfile::tempdir().unwrap();
        // gravação esquecida de outra vez some ao abrir
        std::fs::create_dir_all(d.path().join(SOUNDS_DIR)).unwrap();
        let stale = d.path().join(SOUNDS_DIR).join(".gravacao-1-1.wav");
        std::fs::write(&stale, b"x").unwrap();
        let mut r = Recorder::new(Some(d.path()));
        assert!(!stale.exists());
        let (mx, my) = center(MIC.2);
        assert_eq!(r.click(mx, my, &assets).fx, vec![Effect::RecordStart]);
        assert!(r.recording());
        let temp = r.recorded().unwrap().to_path_buf();
        assert!(temp.starts_with(d.path().join(SOUNDS_DIR)));
        // o tempo para a gravação sozinho
        assert_eq!(r.tick(4.0), None);
        assert_eq!(r.tick(7.0), Some(Effect::RecordStop(temp.clone())));
        assert!(!r.recording());
        // a plataforma grava o arquivo
        std::fs::write(&temp, b"RIFF").unwrap();
        let (px, py) = center(PLAY.2);
        assert_eq!(
            r.click(px, py, &assets).fx,
            vec![Effect::PlaySound(SoundRef::File(temp.clone()))]
        );
        r.type_text("Galo/doido");
        r.key(Key::Backspace, &assets);
        assert_eq!(r.name, "Galodoid");
        let (ox, oy) = center(OK.2);
        let out = r.click(ox, oy, &assets);
        assert!(out.close);
        let path = d.path().join(SOUNDS_DIR).join("Galodoid.wav");
        assert_eq!(
            out.chosen,
            Some(Hand::Sound {
                name: "Galodoid".into(),
                sound: SoundRef::File(path.clone())
            })
        );
        assert!(path.is_file() && !temp.exists());
    }

    #[test]
    fn ok_while_recording_stops_first_and_no_discards() {
        let (_a, assets) = crate::scenes::testutil::empty_assets();
        let d = tempfile::tempdir().unwrap();
        let mut r = Recorder::new(Some(d.path()));
        let (mx, my) = center(MIC.2);
        r.click(mx, my, &assets);
        let temp = r.recorded().unwrap().to_path_buf();
        let (ox, oy) = center(OK.2);
        let out = r.click(ox, oy, &assets);
        assert_eq!(out.fx, vec![Effect::RecordStop(temp.clone())]);
        assert!(!out.close);
        // sem arquivo (sem microfone): OK fecha sem som na mão
        let out = r.click(ox, oy, &assets);
        assert!(out.close && out.chosen.is_none());
        // NÃO fecha e apaga
        let mut r = Recorder::new(Some(d.path()));
        r.click(mx, my, &assets);
        let temp = r.recorded().unwrap().to_path_buf();
        std::fs::write(&temp, b"x").unwrap();
        let (nx, ny) = center(NO.2);
        let out = r.click(nx, ny, &assets);
        assert!(out.close);
        assert!(out.fx.contains(&Effect::RecordStop(temp.clone())));
        assert!(!temp.exists());
    }

    #[test]
    fn without_drawings_dir_nothing_is_recorded() {
        let (_a, assets) = crate::scenes::testutil::empty_assets();
        let mut r = Recorder::new(None);
        let (mx, my) = center(MIC.2);
        let out = r.click(mx, my, &assets);
        assert!(matches!(out.fx[..], [Effect::Log(_)]));
        assert!(!r.recording());
    }
}
