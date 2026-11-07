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

use crate::assets::{Assets, Program, SoundRef};
use crate::paint::Canvas;
use crate::scene::{Effect, Input, Key, MouseButton};
use crate::text_balloon::{Button, Part, TextBalloon, blit_pic};
use crate::{Frame, paint::store::save_png};
use formats::palette::Rgb;
use std::path::PathBuf;
use std::process::Child;

type Rect = (i32, i32, i32, i32);

use crate::paint::print::machine::*;

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
    child: Option<Child>,
    /// O `lp` terminou: `Some(true)` com sucesso.
    result: Option<bool>,
    t: f32,
}

fn record_balloon(
    assets: &Assets,
    block: u16,
    k: usize,
    fallback: &str,
    buttons: Vec<Button>,
) -> TextBalloon {
    match assets.dialogs(Program::Artist, block).get(k) {
        Some(r) => TextBalloon::from_record(r, 0, 0, "", buttons),
        None => TextBalloon::new(0, 0, 200, vec![Part::Text(fallback.into())], buttons),
    }
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
            child: None,
            result: None,
            t: 0.0,
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
        let mut b = match stage {
            Stage::Ready => record_balloon(
                assets,
                WHAT_MACHINE,
                REC_READY,
                "OK. Já estou pronto para imprimir a figura! Quando você estiver pronto para imprimir, clique em Segue.",
                vec![Button::Stop, Button::Next],
            ),
            Stage::Printing => record_balloon(
                assets,
                WHAT_MACHINE,
                REC_PRINTING,
                "Eu ainda estou imprimindo... um pouquinho de paciência! Para cancelar a impressão, aperte a tecla Esc.",
                Vec::new(),
            ),
            Stage::Done => record_balloon(
                assets,
                WHAT_MACHINE,
                REC_DONE,
                "Terminei de imprimir a figura. Foi divertido!",
                vec![Button::Stop],
            ),
            Stage::Alert => {
                let (k, text) = if canceled {
                    (REC_CANCELED, "Você acabou de cancelar a impressão.")
                } else {
                    (
                        REC_CANT_PRINT,
                        "Sinto muito, por algum motivo eu não consigo imprimir isto. Verifique a impressora e tente outra vez.",
                    )
                };
                let mut b = record_balloon(assets, WHAT_ERRORS, k, text, Vec::new());
                b.x = ALERT_POS.0;
                b.y = ALERT_POS.1;
                self.balloon = b;
                return;
            }
        };
        b.x = BALLOON_RIGHT - b.w;
        b.y = BALLOON_TOP;
        self.balloon = b;
    }

    /// Grava a folha e chama o programa de impressão.
    fn start(&mut self, fx: &mut Vec<Effect>) {
        self.t = 0.0;
        self.result = None;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let Some(dir) = &self.dir else {
            fx.push(Effect::Log("imprimir: sem pasta de desenhos".into()));
            self.result = Some(false);
            return;
        };
        let path =
            dir.join(crate::paint::store::timestamp_name(now).replace("desenho-", "impressao-"));
        if let Err(e) =
            std::fs::create_dir_all(dir).and_then(|()| save_png(&self.picture, &self.pal, &path))
        {
            fx.push(Effect::Log(format!("imprimir: {e}")));
            self.result = Some(false);
            return;
        }
        match crate::paint::print::spawn_lp(&self.command, 1, std::slice::from_ref(&path)) {
            Ok(c) => {
                fx.push(Effect::Log(format!("imprimindo {}", path.display())));
                self.child = Some(c);
            }
            Err(_) => {
                fx.push(Effect::Log(format!(
                    "sem impressora ({}): o desenho ficou em {}",
                    self.command,
                    path.display()
                )));
                self.result = Some(false);
            }
        }
    }

    /// Passa o tempo; devolve os efeitos (sons do aviso).
    pub fn update(&mut self, dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = Vec::new();
        if self.stage != Stage::Printing {
            return fx;
        }
        self.t += dt;
        if let Some(c) = self.child.as_mut()
            && let Ok(Some(status)) = c.try_wait()
        {
            self.result = Some(status.success());
            self.child = None;
        }
        if self.t >= MIN_PRINTING
            && let Some(ok) = self.result
        {
            if ok {
                self.go(Stage::Done, assets, false);
            } else {
                fx.push(snd(SND_ALERT));
                self.go(Stage::Alert, assets, false);
            }
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
                    if let Some(mut c) = self.child.take() {
                        let _ = c.kill();
                        let _ = c.wait();
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
        let c = &self.picture;
        let (w, h) = (c.w as i32 / 2, c.h as i32 / 2);
        let (ox, oy) = (px + (pw - w) / 2, py + (ph - h) / 2);
        for y in 0..h {
            for x in 0..w {
                let i = c.px[((2 * y) as u32 * c.w + (2 * x) as u32) as usize] as usize;
                let p = self.pal.get(i).copied().unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                f.put(ox + x, oy + y, [p.r, p.g, p.b]);
            }
        }
    }

    fn draw_progress(&self, f: &mut Frame, a: &Assets) {
        let (x, y, w, h) = STRIP;
        f.rect(x, y, w, h, BLACK);
        f.rect(x + 1, y + 1, w - 2, h - 2, WHITE);
        let k = if self.result == Some(true) {
            1.0
        } else {
            (self.t / MIN_PRINTING).min(1.0) * 0.9
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
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
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
