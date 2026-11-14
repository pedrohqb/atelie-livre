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
//!
//! O que muda por programa é um [`PreviewSpec`]: [`ARTIST_PREVIEW`] (o
//! desenho pela metade numa folha deitada) e o `WRITER_PREVIEW` da
//! Biblioteca do Creative Writer (`writer_library`: a 1ª página numa folha
//! em pé, balões `4c00[8]`/`[9]`/`[11]` e avisos `0805[21]`/`4c02[2]` do
//! `WRITER.EXE`). A sequência é a mesma nos dois: no original, arrastar um
//! documento para a impressora da Biblioteca vai direto ao "OK. Já estou
//! pronto para imprimir!", sem as boas-vindas nem as cópias da máquina do
//! Escritório.

use super::printing::{self, PrintJob};
use crate::assets::{Assets, Program, SoundRef};
use crate::paint::Canvas;
use crate::paint::print::Rgb8;
use crate::scene::{Effect, Input, Key, MouseButton};
use crate::text_balloon::{Button, TextBalloon, blit_pic_in};
use crate::ui::blit_canvas;
use crate::{Frame, paint::store::save_png};
use formats::palette::Rgb;
use std::path::PathBuf;

use crate::paint::print::machine::*;
use crate::ui::Rect;

/// Faixa de progresso no canto inferior esquerdo e o bicho que anda nela.
const STRIP: Rect = (22, 430, 200, 26);
const WALKER: u16 = 0x25d;

/// Balão do McZee ou aviso do Rex: bloco, registro e texto reserva.
pub type Rec = (u16, usize, &'static str);

/// O que anda no canto de baixo enquanto imprime.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Walker {
    /// Faixa de progresso com um bicho por cima (Fine Artist).
    Strip(u16),
    /// Quadros que se alternam a cada `period` segundos, no ponto dado,
    /// andando `step` px por quadro (Writer: o dinossauro atrás do rato).
    Frames {
        frames: &'static [u16],
        at: (i32, i32),
        step: i32,
        period: f32,
    },
}

/// O que muda entre a prévia da Galeria e a da Biblioteca do Writer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewSpec {
    pub program: Program,
    /// Janela: moldura preta/laranja e área branca.
    pub window: Rect,
    /// Faixa cinza em cima (linhas de cima e de baixo), se houver.
    pub gray_bar: Option<(i32, i32)>,
    /// Moldura cinza em volta da folha (Writer), se houver.
    pub sheet_frame: Option<Rect>,
    /// Folha da prévia, com a borda preta de 1 px.
    pub page: Rect,
    /// McZee no alto à direita (figura, x, y; máscara duas adiante).
    pub mczee: (u16, i32, i32),
    /// Canto de cima à direita dos balões do McZee e posição dos avisos.
    pub balloon_right: i32,
    pub balloon_top: i32,
    pub alert_pos: (i32, i32),
    pub ready: Rec,
    pub printing: Rec,
    pub done: Rec,
    pub cant_print: Rec,
    pub canceled: Rec,
    pub walker: Walker,
    /// Sons: Pára, Segue e o "Epa!" do Rex.
    pub snd_stop: u16,
    pub snd_next: u16,
    pub snd_alert: u16,
}

/// A prévia da Galeria do Fine Artist (medidas de `gallery_o_dprint_3.png`).
pub const ARTIST_PREVIEW: PreviewSpec = PreviewSpec {
    program: Program::Artist,
    window: (16, 28, 610, 434),
    gray_bar: Some((33, 87)),
    sheet_frame: None,
    page: PAGE,
    mczee: MCZEE,
    balloon_right: BALLOON_RIGHT,
    balloon_top: BALLOON_TOP,
    alert_pos: ALERT_POS,
    ready: (
        WHAT_MACHINE,
        REC_READY,
        "OK. Já estou pronto para imprimir a figura!\nQuando você estiver pronto para imprimir, clique em Segue.",
    ),
    printing: (
        WHAT_MACHINE,
        REC_PRINTING,
        "Eu ainda estou imprimindo... um pouquinho de paciência!\nPara cancelar a impressão, aperte a tecla Esc.",
    ),
    done: (
        WHAT_MACHINE,
        REC_DONE,
        "Terminei de imprimir a figura. Foi divertido!\nPara voltar ao seu trabalho, clique em Pára.",
    ),
    cant_print: printing::CANT_PRINT,
    canceled: printing::CANCELED,
    walker: Walker::Strip(WALKER),
    snd_stop: SND_STOP,
    snd_next: SND_NEXT,
    snd_alert: SND_ALERT,
};

/// O que vai para a folha.
pub enum Sheet {
    /// Um desenho da pintura (índices da paleta), impresso inteiro e
    /// mostrado pela metade no meio da folha.
    Picture(Canvas, Vec<Rgb>),
    /// Uma folha já montada (a página de um documento do Writer), mostrada
    /// reduzida para caber na folha da prévia.
    Page(Rgb8),
}

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const ORANGE: [u8; 3] = [255, 140, 50];
const ORANGE_DARK: [u8; 3] = [185, 101, 36];
const GRAY: [u8; 3] = [140, 140, 140];

fn snd_in(p: Program, id: u16) -> Effect {
    Effect::PlaySound(SoundRef::Snd(p, id))
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
    spec: &'static PreviewSpec,
    sheet: Sheet,
    /// Relógio do bicho que anda enquanto imprime.
    clock: f32,
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
        PrintPreview::with_spec(
            &ARTIST_PREVIEW,
            Sheet::Picture(picture, pal),
            dir,
            command,
            assets,
        )
    }

    /// Prévia com os dados de `spec` (a da Biblioteca do Writer:
    /// `writer_library::WRITER_PREVIEW`) e a folha `sheet`.
    pub fn with_spec(
        spec: &'static PreviewSpec,
        sheet: Sheet,
        dir: Option<PathBuf>,
        command: &str,
        assets: &Assets,
    ) -> PrintPreview {
        let mut p = PrintPreview {
            spec,
            sheet,
            clock: 0.0,
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
        let s = self.spec;
        let step = |(block, k, fallback): Rec, buttons: Vec<Button>| {
            let recs = assets.dialogs(s.program, block);
            let mut b = printing::mczee_balloon(recs.get(k), fallback, &[], buttons);
            (b.x, b.y) = (s.balloon_right - b.w, s.balloon_top);
            b
        };
        self.balloon = match stage {
            Stage::Ready => step(s.ready, vec![Button::Stop, Button::Next]),
            Stage::Printing => step(s.printing, vec![]),
            Stage::Done => step(s.done, vec![Button::Stop]),
            Stage::Alert => printing::alert_balloon_in(
                assets,
                s.program,
                if canceled { s.canceled } else { s.cant_print },
                s.alert_pos,
            ),
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
        let saved = std::fs::create_dir_all(dir).and_then(|()| match &self.sheet {
            Sheet::Picture(c, pal) => save_png(c, pal, &path),
            Sheet::Page(s) => crate::paint::print::save_sheet(s, &path),
        });
        if let Err(e) = saved {
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
        if dt.is_finite() && dt > 0.0 {
            self.clock += dt;
        }
        let Some(job) = self.job.as_mut() else {
            return fx;
        };
        match job.poll(dt) {
            printing::Poll::Finished(true) => self.go(Stage::Done, assets, false),
            printing::Poll::Finished(false) => {
                fx.push(snd_in(self.spec.program, self.spec.snd_alert));
                self.go(Stage::Alert, assets, false);
            }
            _ => {}
        }
        fx
    }

    /// Trata uma entrada; `true` quando a prévia fecha (volta à Galeria).
    pub fn input(&mut self, i: &Input, assets: &Assets, fx: &mut Vec<Effect>) -> bool {
        let (p, s) = (self.spec.program, self.spec);
        let snd = |id| snd_in(p, id);
        match self.stage {
            Stage::Ready | Stage::Done => {
                if let Input::Key {
                    key: Key::Escape, ..
                } = i
                {
                    fx.push(snd(s.snd_stop));
                    return true;
                }
                match self.balloon.handle(assets, i) {
                    Some(Button::Stop) => {
                        fx.push(snd(s.snd_stop));
                        true
                    }
                    Some(Button::Next) if self.stage == Stage::Ready => {
                        fx.push(snd(s.snd_next));
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
                    fx.push(snd(s.snd_alert));
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

    fn draw_window(&self, f: &mut Frame) {
        let (x, y, w, h) = self.spec.window;
        f.rect(x, y, w, h, BLACK);
        f.rect(x + 1, y + 1, w - 2, h - 2, ORANGE);
        f.rect(x + 3, y + 3, w - 6, h - 6, ORANGE_DARK);
        f.rect(x + 4, y + 4, w - 8, h - 8, BLACK);
        if self.spec.program == Program::Writer {
            // no Writer as bordas dos lados têm o escuro por fora
            for sx in [x + 1, x + w - 4] {
                f.rect(sx, y + 4, 1, h - 8, ORANGE_DARK);
                f.rect(sx + 1, y + 4, 2, h - 8, ORANGE);
            }
        }
        f.rect(x + 5, y + 5, w - 10, h - 10, WHITE);
        if let Some((top, bottom)) = self.spec.gray_bar {
            f.rect(x + 5, top, w - 10, bottom - top, GRAY);
            f.rect(x + 5, bottom, w - 10, 1, BLACK);
            f.rect(x + 5, bottom + 1, w - 10, 2, ORANGE);
            f.rect(x + 5, bottom + 3, w - 10, 1, ORANGE_DARK);
            f.rect(x + 5, bottom + 4, w - 10, 1, BLACK);
        }
    }

    fn draw_page(&self, f: &mut Frame) {
        if let Some((x, y, w, h)) = self.spec.sheet_frame {
            f.rect(x, y, w, h, GRAY);
        }
        let (px, py, pw, ph) = self.spec.page;
        f.rect(px, py, pw, ph, BLACK);
        f.rect(px + 1, py + 1, pw - 2, ph - 2, WHITE);
        match &self.sheet {
            Sheet::Picture(c, pal) => {
                // o desenho pela metade, no meio da folha
                let half = crate::paint::drawings::shrink(c, 2);
                let (w, h) = (half.w as i32, half.h as i32);
                blit_canvas(f, &half, pal, px + (pw - w) / 2, py + (ph - h) / 2);
            }
            Sheet::Page(s) => draw_sheet_fit(f, s, (px + 1, py + 1, pw - 2, ph - 2)),
        }
    }

    fn draw_progress(&self, f: &mut Frame, a: &Assets) {
        let (t, ok) = self
            .job
            .as_ref()
            .map_or((0.0, false), |j| (j.elapsed(), j.succeeded()));
        match self.spec.walker {
            Walker::Strip(walker) => {
                let (x, y, w, h) = STRIP;
                f.rect(x, y, w, h, BLACK);
                f.rect(x + 1, y + 1, w - 2, h - 2, WHITE);
                let k = if ok {
                    1.0
                } else {
                    (t / MIN_PRINTING).min(1.0) * 0.9
                };
                let done = ((w - 4) as f32 * k) as i32;
                f.rect(x + 2, y + h - 6, done, 4, ORANGE);
                // o bicho anda sobre a faixa
                blit_pic_in(
                    f,
                    a,
                    self.spec.program,
                    walker,
                    x + 2 + done - 18,
                    y - 10,
                    true,
                );
            }
            Walker::Frames {
                frames,
                at,
                step,
                period,
            } => {
                if frames.is_empty() {
                    return;
                }
                let n = (self.clock / period.max(0.01)) as usize;
                let id = frames[n % frames.len()];
                let x = at.0 + (n as i32 * step) % 120;
                blit_pic_in(f, a, self.spec.program, id, x, at.1, true);
            }
        }
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        self.draw_window(f);
        self.draw_page(f);
        let (id, x, y) = self.spec.mczee;
        blit_pic_in(f, a, self.spec.program, id, x, y, true);
        if matches!(self.stage, Stage::Printing | Stage::Alert) {
            self.draw_progress(f, a);
        }
        self.balloon.draw(f, a);
    }
}

/// A folha reduzida para caber em `r` (o pixel mais escuro de cada bloco,
/// para o texto miúdo não sumir), centrada.
fn draw_sheet_fit(f: &mut Frame, s: &Rgb8, r: Rect) {
    if s.w == 0 || s.h == 0 || r.2 <= 0 || r.3 <= 0 {
        return;
    }
    let k = (r.2 as f32 / s.w as f32).min(r.3 as f32 / s.h as f32);
    let (w, h) = ((s.w as f32 * k) as i32, (s.h as f32 * k) as i32);
    let (x0, y0) = (r.0 + (r.2 - w) / 2, r.1 + (r.3 - h) / 2);
    for ty in 0..h {
        let sy0 = (ty as u32 * s.h / h as u32).min(s.h - 1);
        let sy1 = (((ty + 1) as u32 * s.h).div_ceil(h as u32)).clamp(sy0 + 1, s.h);
        for tx in 0..w {
            let sx0 = (tx as u32 * s.w / w as u32).min(s.w - 1);
            let sx1 = (((tx + 1) as u32 * s.w).div_ceil(w as u32)).clamp(sx0 + 1, s.w);
            let mut best = [255u8; 3];
            let mut lum = u32::MAX;
            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let p = s.px[(sy * s.w + sx) as usize];
                    let l = p[0] as u32 * 3 + p[1] as u32 * 6 + p[2] as u32;
                    if l < lum {
                        lum = l;
                        best = p;
                    }
                }
            }
            f.put(x0 + tx, y0 + ty, best);
        }
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

    fn snd(id: u16) -> Effect {
        snd_in(Program::Artist, id)
    }

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
