//! Prévia de impressão da Galeria: o quadro arrastado para a impressora abre
//! uma janela (moldura laranja, faixa cinza e a folha com o desenho pela
//! metade) com o McZee no alto à direita e o balão "OK. Já estou pronto para
//! imprimir a figura!" (`WHAT 4c00[11]`) com Para / Segue. Segue grava a
//! folha e chama o `lp`; enquanto ele trabalha (no mínimo `MIN_PRINTING`
//! segundos) aparece "Eu ainda estou imprimindo…" (`4c00[12]`) e um bichinho
//! anda na faixa de progresso; Esc cancela (`100f[20]`). No fim, "Terminei de
//! imprimir a figura" (`4c00[14]`) com Para; se o `lp` falhar ou não existir,
//! o "EPA!" do Rex (`100f[19]`), que fecha com qualquer clique ou tecla.
//! Medido em `gallery_o_dprint_3.png` e `gallery_o_print_segue_3.png`.

use super::printing::{self, PrintJob};
use crate::assets::{Assets, Program, SoundRef};
use crate::paint::Canvas;
use crate::scene::{Effect, Input, Key, MouseButton};
use crate::text_balloon::{Button, TextBalloon, blit_pic};
use crate::ui::blit_canvas;
use crate::{Frame, paint::store::save_png};
use formats::palette::Rgb;
use std::path::PathBuf;

use crate::paint::print::machine::*;
use crate::ui::Rect;

/// Janela: moldura preta/laranja, faixa cinza em cima, área branca.
const WINDOW: Rect = (16, 28, 610, 434);
const GRAY_BAR: (i32, i32) = (33, 87);
/// Faixa de progresso no canto inferior esquerdo e o bicho que anda nela.
const STRIP: Rect = (22, 430, 200, 26);
const WALKER: u16 = 0x25d;

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const ORANGE: [u8; 3] = [255, 140, 50];
const ORANGE_DARK: [u8; 3] = [185, 101, 36];
const GRAY: [u8; 3] = [140, 140, 140];

fn snd(id: u16) -> Effect {
    Effect::PlaySound(SoundRef::Snd(Program::Artist, id))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// "OK. Já estou pronto…" com Para / Segue.
    Ready,
    /// "Eu ainda estou imprimindo…".
    Printing,
    /// "Terminei de imprimir" com Para.
    Done,
    /// Aviso do Rex (não imprimiu ou cancelado): qualquer clique fecha.
    Alert,
}

pub struct PrintPreview {
    picture: Canvas,
    pal: Vec<Rgb>,
    /// Pasta onde a folha é gravada (`impressao-….png`) antes do `lp`.
    dir: Option<PathBuf>,
    /// Programa de impressão (`lp`; os testes trocam).
    command: String,
    stage: Stage,
    balloon: TextBalloon,
    /// Impressão em andamento (desde Segue).
    job: Option<PrintJob>,
}

impl PrintPreview {
    pub fn new(
        picture: Canvas,
        pal: Vec<Rgb>,
        dir: Option<PathBuf>,
        command: &str,
        assets: &Assets,
    ) -> PrintPreview {
        let mut p = PrintPreview {
            picture,
            pal,
            dir,
            command: command.to_string(),
            stage: Stage::Ready,
            balloon: TextBalloon::new(0, 0, 120, Vec::new(), Vec::new()),
            job: None,
        };
        p.go(Stage::Ready, assets, false);
        p
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn balloon(&self) -> &TextBalloon {
        &self.balloon
    }

    fn go(&mut self, stage: Stage, assets: &Assets, canceled: bool) {
        self.stage = stage;
        self.balloon = match stage {
            Stage::Ready => printing::step_balloon(assets, printing::Step::Ready),
            Stage::Printing => printing::step_balloon(assets, printing::Step::Printing),
            Stage::Done => printing::step_balloon(assets, printing::Step::Done),
            Stage::Alert if canceled => printing::alert_balloon(assets, printing::CANCELED),
            Stage::Alert => printing::alert_balloon(assets, printing::CANT_PRINT),
        };
    }

    /// Grava a folha e chama o programa de impressão.
    fn start(&mut self, fx: &mut Vec<Effect>) {
        let job = self.job.insert(PrintJob::new());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let Some(dir) = &self.dir else {
            fx.push(Effect::Log("imprimir: sem pasta de desenhos".into()));
            job.fail();
            return;
        };
        let path =
            dir.join(crate::paint::store::timestamp_name(now).replace("desenho-", "impressao-"));
        if let Err(e) =
            std::fs::create_dir_all(dir).and_then(|()| save_png(&self.picture, &self.pal, &path))
        {
            fx.push(Effect::Log(format!("imprimir: {e}")));
            job.fail();
            return;
        }
        match job.lp(&self.command, 1, std::slice::from_ref(&path)) {
            Ok(()) => fx.push(Effect::Log(format!("imprimindo {}", path.display()))),
            Err(_) => fx.push(Effect::Log(format!(
                "sem impressora ({}): o desenho ficou em {}",
                self.command,
                path.display()
            ))),
        }
    }

    /// Passa o tempo; devolve os efeitos (sons do aviso).
    pub fn update(&mut self, dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = Vec::new();
        if self.stage != Stage::Printing {
            return fx;
        }
        let Some(job) = self.job.as_mut() else {
            return fx;
        };
        match job.poll(dt) {
            printing::Poll::Finished(true) => self.go(Stage::Done, assets, false),
            printing::Poll::Finished(false) => {
                fx.push(snd(SND_ALERT));
                self.go(Stage::Alert, assets, false);
            }
            _ => {}
        }
        fx
    }

    /// Trata uma entrada; `true` quando a prévia fecha (volta à Galeria).
    pub fn input(&mut self, i: &Input, assets: &Assets, fx: &mut Vec<Effect>) -> bool {
        match self.stage {
            Stage::Ready | Stage::Done => {
                if let Input::Key {
                    key: Key::Escape, ..
                } = i
                {
                    fx.push(snd(SND_STOP));
                    return true;
                }
                match self.balloon.handle(assets, i) {
                    Some(Button::Stop) => {
                        fx.push(snd(SND_STOP));
                        true
                    }
                    Some(Button::Next) if self.stage == Stage::Ready => {
                        fx.push(snd(SND_NEXT));
                        self.go(Stage::Printing, assets, false);
                        self.start(fx);
                        false
                    }
                    _ => false,
                }
            }
            Stage::Printing => {
                if let Input::Key {
                    key: Key::Escape, ..
                } = i
                {
                    if let Some(job) = self.job.as_mut() {
                        job.cancel();
                    }
                    fx.push(snd(SND_ALERT));
                    self.go(Stage::Alert, assets, true);
                }
                false
            }
            Stage::Alert => matches!(
                i,
                Input::Click {
                    button: MouseButton::Left,
                    ..
                } | Input::Key { .. }
            ),
        }
    }

    fn draw_window(f: &mut Frame) {
        let (x, y, w, h) = WINDOW;
        f.rect(x, y, w, h, BLACK);
        f.rect(x + 1, y + 1, w - 2, h - 2, ORANGE);
        f.rect(x + 3, y + 3, w - 6, h - 6, ORANGE_DARK);
        f.rect(x + 4, y + 4, w - 8, h - 8, BLACK);
        f.rect(x + 5, y + 5, w - 10, h - 10, WHITE);
        f.rect(x + 5, GRAY_BAR.0, w - 10, GRAY_BAR.1 - GRAY_BAR.0, GRAY);
        f.rect(x + 5, GRAY_BAR.1, w - 10, 1, BLACK);
        f.rect(x + 5, GRAY_BAR.1 + 1, w - 10, 2, ORANGE);
        f.rect(x + 5, GRAY_BAR.1 + 3, w - 10, 1, ORANGE_DARK);
        f.rect(x + 5, GRAY_BAR.1 + 4, w - 10, 1, BLACK);
    }

    fn draw_page(&self, f: &mut Frame) {
        let (px, py, pw, ph) = PAGE;
        f.rect(px, py, pw, ph, BLACK);
        f.rect(px + 1, py + 1, pw - 2, ph - 2, WHITE);
        // o desenho pela metade, no meio da folha
        let half = crate::paint::drawings::shrink(&self.picture, 2);
        let (w, h) = (half.w as i32, half.h as i32);
        blit_canvas(f, &half, &self.pal, px + (pw - w) / 2, py + (ph - h) / 2);
    }

    fn draw_progress(&self, f: &mut Frame, a: &Assets) {
        let (x, y, w, h) = STRIP;
        f.rect(x, y, w, h, BLACK);
        f.rect(x + 1, y + 1, w - 2, h - 2, WHITE);
        let (t, ok) = self
            .job
            .as_ref()
            .map_or((0.0, false), |j| (j.elapsed(), j.succeeded()));
        let k = if ok {
            1.0
        } else {
            (t / MIN_PRINTING).min(1.0) * 0.9
        };
        let done = ((w - 4) as f32 * k) as i32;
        f.rect(x + 2, y + h - 6, done, 4, ORANGE);
        // o bicho anda sobre a faixa
        blit_pic(f, a, WALKER, x + 2 + done - 18, y - 10, true);
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        Self::draw_window(f);
        self.draw_page(f);
        blit_pic(f, a, MCZEE.0, MCZEE.1, MCZEE.2, true);
        if matches!(self.stage, Stage::Printing | Stage::Alert) {
            self.draw_progress(f, a);
        }
        self.balloon.draw(f, a);
    }
}

impl Drop for PrintPreview {
    fn drop(&mut self) {
        if let Some(job) = self.job.as_mut() {
            job.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{CANVAS_H, CANVAS_W};
    use crate::scenes::testutil::empty_assets;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn preview(cmd: &str, dir: &std::path::Path, a: &Assets) -> PrintPreview {
        let pal = a.palettes(Program::Artist).pal256.clone();
        PrintPreview::new(
            Canvas::new(CANVAS_W, CANVAS_H),
            pal,
            Some(dir.to_path_buf()),
            cmd,
            a,
        )
    }

    fn press(p: &mut PrintPreview, b: Button, a: &Assets, fx: &mut Vec<Effect>) -> bool {
        let r = p.balloon().button_rect(a, b).unwrap();
        p.input(&click(r.0 + 3, r.1 + 3), a, fx)
    }

    #[test]
    fn stop_closes_without_printing() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = preview("true", dir.path(), &a);
        let mut fx = Vec::new();
        assert!(press(&mut p, Button::Stop, &a, &mut fx));
        assert!(fx.contains(&snd(SND_STOP)));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        p.draw(&mut f, &a);
    }

    #[test]
    fn next_prints_then_done() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = preview("true", dir.path(), &a);
        let mut fx = Vec::new();
        assert!(!press(&mut p, Button::Next, &a, &mut fx));
        assert!(fx.contains(&snd(SND_NEXT)));
        assert_eq!(p.stage(), Stage::Printing);
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        p.draw(&mut f, &a);
        // a folha foi gravada
        let names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().any(|n| n.starts_with("impressao-")),
            "{names:?}"
        );
        for _ in 0..100 {
            p.update(0.05, &a);
            if p.stage() != Stage::Printing {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(p.stage(), Stage::Done);
        let mut fx = Vec::new();
        assert!(press(&mut p, Button::Stop, &a, &mut fx));
    }

    #[test]
    fn missing_printer_shows_rex_and_any_click_closes() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = preview("/nao/existe/lp", dir.path(), &a);
        let mut fx = Vec::new();
        press(&mut p, Button::Next, &a, &mut fx);
        // ainda imprimindo durante o tempo mínimo
        assert!(p.update(0.5, &a).is_empty());
        assert_eq!(p.stage(), Stage::Printing);
        let fx = p.update(MIN_PRINTING, &a);
        assert!(fx.contains(&snd(SND_ALERT)));
        assert_eq!(p.stage(), Stage::Alert);
        assert!(p.input(&click(5, 5), &a, &mut Vec::new()));
    }

    #[test]
    fn escape_cancels_printing() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = preview("sleep", dir.path(), &a);
        let mut fx = Vec::new();
        press(&mut p, Button::Next, &a, &mut fx);
        let esc = Input::Key {
            key: Key::Escape,
            ctrl: false,
            alt: false,
        };
        let mut fx = Vec::new();
        assert!(!p.input(&esc, &a, &mut fx));
        assert_eq!(p.stage(), Stage::Alert);
        assert!(fx.contains(&snd(SND_ALERT)));
        assert!(p.input(&esc, &a, &mut fx));
    }
}
