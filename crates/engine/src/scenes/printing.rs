//! Impressão comum à prévia da Galeria ([`super::gallery_print`]) e à
//! máquina do McZee na pintura ([`super::paint::print_machine`]): o trabalho
//! em andamento (folhas montadas numa thread, `lp`, o tempo mínimo do
//! "Eu ainda estou imprimindo…", cancelar) e os balões do McZee
//! (`WHAT 4c00`) e do Rex (`100f`). Constantes em
//! [`crate::paint::print::machine`]; ver `docs/telas/pintura-impressao.md`.

use crate::assets::{Assets, Program};
use crate::balloon::Balloon;
use crate::paint::print::machine::*;
use crate::text_balloon::{Button, PIC_ALERT, Part, TextBalloon};
use formats::what::Record;
use std::path::PathBuf;
use std::process::Child;
use std::sync::mpsc::{Receiver, TryRecvError};

/// Folhas gravadas pela thread de montagem, ou o erro.
pub type Rendered = Result<Vec<PathBuf>, String>;

/// Como anda o trabalho.
#[derive(Debug, PartialEq, Eq)]
pub enum Poll {
    /// A thread ainda monta as folhas (o tempo não conta).
    Rendering,
    /// As folhas ficaram prontas (ou não): é a vez de chamar o `lp`.
    Rendered(Rendered),
    /// Imprimindo, ou ainda dentro do tempo mínimo.
    Waiting,
    /// Acabou: `true` se o `lp` terminou bem.
    Finished(bool),
}

/// Impressão em andamento. Soltar o trabalho não mata o `lp` (quem quiser,
/// chama [`PrintJob::cancel`]).
#[derive(Default)]
pub struct PrintJob {
    render: Option<Receiver<Rendered>>,
    child: Option<Child>,
    t: f32,
    ok: Option<bool>,
}

impl PrintJob {
    /// Trabalho sem montagem: o `lp` vem em seguida com [`PrintJob::lp`].
    pub fn new() -> PrintJob {
        PrintJob::default()
    }

    /// Monta as folhas numa thread (para não travar a tela).
    pub fn render(work: impl FnOnce() -> Rendered + Send + 'static) -> PrintJob {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(work());
        });
        PrintJob {
            render: Some(rx),
            ..PrintJob::default()
        }
    }

    /// Chama o programa de impressão com as folhas; se ele nem começa, o
    /// trabalho fica como falho.
    pub fn lp(&mut self, command: &str, copies: u8, files: &[PathBuf]) -> std::io::Result<()> {
        match crate::paint::print::spawn_lp(command, copies, files) {
            Ok(c) => {
                self.child = Some(c);
                Ok(())
            }
            Err(e) => {
                self.ok = Some(false);
                Err(e)
            }
        }
    }

    /// Marca o trabalho como falho (sem pasta, erro ao gravar…).
    pub fn fail(&mut self) {
        self.ok = Some(false);
    }

    /// Mata o `lp`, se ainda roda.
    pub fn cancel(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    /// Segundos desde que as folhas ficaram prontas.
    pub fn elapsed(&self) -> f32 {
        self.t
    }

    /// O `lp` já terminou bem.
    pub fn succeeded(&self) -> bool {
        self.ok == Some(true)
    }

    /// Passa o tempo. O fim só é dado depois de [`MIN_PRINTING`] segundos.
    pub fn poll(&mut self, dt: f32) -> Poll {
        if let Some(rx) = &self.render {
            let got = match rx.try_recv() {
                Ok(r) => r,
                Err(TryRecvError::Empty) => return Poll::Rendering,
                Err(TryRecvError::Disconnected) => Err("a montagem das folhas parou".into()),
            };
            self.render = None;
            return Poll::Rendered(got);
        }
        if dt.is_finite() && dt > 0.0 {
            self.t += dt;
        }
        if self.ok.is_none()
            && let Some(c) = self.child.as_mut()
        {
            match c.try_wait() {
                Ok(Some(status)) => {
                    self.ok = Some(status.success());
                    self.child = None;
                }
                Ok(None) => {}
                Err(_) => self.ok = Some(false),
            }
        }
        match self.ok {
            Some(ok) if self.t >= MIN_PRINTING => Poll::Finished(ok),
            _ => Poll::Waiting,
        }
    }
}

/// Texto e opções de um registro, ou a reserva.
pub fn record_text(
    r: Option<&Record>,
    fallback: &str,
    fallback_opts: &[&str],
) -> (String, Vec<String>) {
    match r {
        Some(r) if !r.plain().trim().is_empty() => {
            let b = Balloon::from_record(r, 0, 0, "");
            (b.text, b.options)
        }
        _ => (
            fallback.to_string(),
            fallback_opts.iter().map(|s| s.to_string()).collect(),
        ),
    }
}

/// Balão do McZee (texto e opções do registro, ou a reserva), encostado no
/// canto de cima à direita (`BALLOON_RIGHT`, `BALLOON_TOP`).
pub fn mczee_balloon(
    r: Option<&Record>,
    fallback: &str,
    opts: &[&str],
    buttons: Vec<Button>,
) -> TextBalloon {
    let width = r.map_or(200, |r| r.width as i32).clamp(150, 260);
    let (text, options) = record_text(r, fallback, opts);
    let mut parts = vec![Part::Text(text)];
    if !options.is_empty() {
        parts.push(Part::Options(options));
    }
    let mut b = TextBalloon::new(0, BALLOON_TOP, width, parts, buttons);
    b.x = BALLOON_RIGHT - b.w;
    b
}

/// Os três balões do McZee comuns à Galeria e à máquina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// "OK. Já estou pronto para imprimir a figura!" (Pára / Segue).
    Ready,
    /// "Eu ainda estou imprimindo…" (Esc cancela).
    Printing,
    /// "Terminei de imprimir a figura." (Pára).
    Done,
}

pub fn step_balloon(assets: &Assets, step: Step) -> TextBalloon {
    let recs = assets.dialogs(Program::Artist, WHAT_MACHINE);
    let (k, fallback, buttons) = match step {
        Step::Ready => (
            REC_READY,
            "OK. Já estou pronto para imprimir a figura!\nQuando você estiver pronto para imprimir, clique em Segue.",
            vec![Button::Stop, Button::Next],
        ),
        Step::Printing => (
            REC_PRINTING,
            "Eu ainda estou imprimindo... um pouquinho de paciência!\nPara cancelar a impressão, aperte a tecla Esc.",
            vec![],
        ),
        Step::Done => (
            REC_DONE,
            "Terminei de imprimir a figura. Foi divertido!\nPara voltar ao seu trabalho, clique em Pára.",
            vec![Button::Stop],
        ),
    };
    mczee_balloon(recs.get(k), fallback, &[], buttons)
}

/// Aviso "EPA!" do Rex (`block`, `k`) em `ALERT_POS`, ou a reserva. Vem
/// com o fundo de balão (como na prévia da Galeria e no original); a
/// máquina da pintura o pinta de branco com [`TextBalloon::as_alert`].
pub fn alert_balloon(assets: &Assets, (block, k, fallback): (u16, usize, &str)) -> TextBalloon {
    let recs = assets.dialogs(Program::Artist, block);
    match recs.get(k) {
        Some(r) if !r.plain().trim().is_empty() => {
            TextBalloon::from_record(r, ALERT_POS.0, ALERT_POS.1, "", vec![])
        }
        _ => TextBalloon::new(
            ALERT_POS.0,
            ALERT_POS.1,
            220,
            vec![Part::Figure(PIC_ALERT), Part::Text(fallback.into())],
            vec![],
        ),
    }
}

/// "Sinto muito, por algum motivo eu não consigo imprimir isto."
pub const CANT_PRINT: (u16, usize, &str) = (
    WHAT_ERRORS,
    REC_CANT_PRINT,
    "Sinto muito, por algum motivo eu não consigo imprimir isto.\nVerifique a impressora e tente outra vez.",
);
/// "Você acabou de cancelar a impressão."
pub const CANCELED: (u16, usize, &str) = (
    WHAT_ERRORS,
    REC_CANCELED,
    "Você acabou de cancelar a impressão.",
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;

    fn settle(j: &mut PrintJob) -> Poll {
        for _ in 0..200 {
            match j.poll(0.05) {
                Poll::Waiting | Poll::Rendering => {
                    std::thread::sleep(std::time::Duration::from_millis(5))
                }
                p => return p,
            }
        }
        panic!("o trabalho não terminou");
    }

    #[test]
    fn finishes_only_after_the_minimum_time() {
        let mut j = PrintJob::new();
        j.lp("true", 1, &[PathBuf::from("/x.png")]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(j.poll(0.1), Poll::Waiting);
        assert_eq!(j.poll(MIN_PRINTING), Poll::Finished(true));
        assert!(j.succeeded());
    }

    #[test]
    fn missing_lp_fails() {
        let mut j = PrintJob::new();
        assert!(j.lp("/nao/existe/lp", 1, &[]).is_err());
        assert_eq!(j.poll(0.1), Poll::Waiting);
        assert_eq!(j.poll(MIN_PRINTING), Poll::Finished(false));
        let mut j = PrintJob::new();
        j.fail();
        assert_eq!(j.poll(MIN_PRINTING), Poll::Finished(false));
    }

    #[test]
    fn render_thread_hands_over_the_files_without_counting_time() {
        let mut j = PrintJob::render(|| Ok(vec![PathBuf::from("/a.png")]));
        assert_eq!(
            settle(&mut j),
            Poll::Rendered(Ok(vec![PathBuf::from("/a.png")]))
        );
        assert_eq!(j.elapsed(), 0.0);
        j.lp("true", 1, &[]).unwrap();
        assert_eq!(settle(&mut j), Poll::Finished(true));
        assert!(j.elapsed() >= MIN_PRINTING);
        let mut j = PrintJob::render(|| Err("sem papel".into()));
        assert_eq!(settle(&mut j), Poll::Rendered(Err("sem papel".into())));
    }

    #[test]
    fn balloons_have_their_buttons_and_places() {
        let (_d, a) = empty_assets();
        let b = step_balloon(&a, Step::Ready);
        assert_eq!(b.buttons, vec![Button::Stop, Button::Next]);
        assert_eq!((b.x + b.w, b.y), (BALLOON_RIGHT, BALLOON_TOP));
        assert!(step_balloon(&a, Step::Printing).buttons.is_empty());
        let r = alert_balloon(&a, CANT_PRINT);
        assert_eq!((r.x, r.y), ALERT_POS);
    }
}
