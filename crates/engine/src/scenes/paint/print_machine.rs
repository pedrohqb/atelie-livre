//! Máquina de impressão do McZee (faixa do Imprimir, `WHAT 4c00`): a tela
//! com a prévia da folha à esquerda, o botão FUNDO na faixa de cima e o McZee
//! no alto à direita, com balões e botões Volta / Pára / Segue.
//!
//! - "fazer escolhas": boas-vindas → fundo (escolher, sem fundo, deixar como
//!   está) → esticar (normal, por igual, ocupar a página, pôster) → cores ou
//!   branco e preto → cópias (1 a 10) → imprime.
//! - "imprimir depressa": a prévia e "OK. Já estou pronto para imprimir a
//!   figura!" com Pára / Segue.
//!
//! Imprimindo, o balão "Eu ainda estou imprimindo…" fica até o `lp` terminar
//! (Esc cancela); se o `lp` falhar ou não existir, aparece o "Epa!" do Rex
//! (`WHAT 100f[19]`). As folhas são montadas por [`crate::paint::print`].
//! Ver `docs/telas/pintura-impressao.md`.

use super::*;
use crate::paint::print::{self as pr, Composer, Options, Rgb8, Stretch};
use crate::text_balloon::{
    Button, PIC_BALL, PIC_BALL_CHECKED, Part, TextBalloon, blit_pic, blit_pic_opaque,
};
use formats::what::Record;

const WHAT_MACHINE: u16 = 0x4c00;
const REC_WELCOME: usize = 0;
const REC_BACKGROUND: usize = 1;
const REC_TOO_MANY: usize = 3;
const REC_PICK_BACKGROUND: usize = 4;
const REC_STRETCH: usize = 6;
const REC_POSTER_BACKGROUND: usize = 7;
const REC_COLOR: usize = 8;
const REC_COPIES: usize = 10;
const REC_READY: usize = 11;
const REC_PRINTING: usize = 12;
const REC_DONE: usize = 14;
/// Avisos do Rex no bloco `WHAT_ERRORS` (100f).
const REC_CANT_PRINT: usize = 19;
const REC_CANCELED: usize = 20;

/// McZee no alto à direita (máscara duas figuras adiante, `4c01`).
const MCZEE: (u16, i32, i32) = (0x4403, 540, 2);
const MCZEE_RECT: (i32, i32, i32, i32) = (545, 2, 90, 158);
/// Botão FUNDO (ilha) na faixa cinza.
const FUNDO: (u16, i32, i32) = (0x10df, 35, 37);
const FUNDO_RECT: (i32, i32, i32, i32) = (35, 37, 120, 42);
/// Folha da prévia (com a borda preta de 1 px).
const PAGE: (i32, i32, i32, i32) = (26, 98, 385, 297);
/// Canto superior direito dos balões do McZee e posição dos avisos do Rex.
const BALLOON_RIGHT: i32 = 590;
const BALLOON_TOP: i32 = 63;
const ALERT_POS: (i32, i32) = (210, 167);
/// Miniaturas dos fundos na faixa, à direita do FUNDO, e as setas.
const THUMB_X0: i32 = 176;
const THUMB: (i32, i32) = (60, 44);
const THUMB_STEP: i32 = 66;
const THUMBS: usize = 5;
const THUMB_LEFT: (i32, i32, i32, i32) = (162, 44, 12, 30);
const THUMB_RIGHT: (i32, i32, i32, i32) = (176 + 5 * 66, 44, 12, 30);
/// Biblioteca dos fundos de impressão (`IBGF`).
const BACKGROUNDS_LIBRARY: &str = "0MAG100.BOR";
/// Tempo mínimo do balão "Eu ainda estou imprimindo…".
const MIN_PRINTING: f32 = 1.5;
/// Sons: Segue que imprime e o "Epa!" do Rex.
const SND_PRINT_GO: u16 = 0x02eb;
const SND_ERROR: u16 = 0x02fe;

const GRAY: [u8; 3] = [140, 140, 140];
const GREEN_RED: [u8; 3] = [250, 0, 57];

/// Passo da máquina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Welcome,
    Background,
    PickBackground,
    Stretch,
    Color,
    Copies,
    /// "OK. Já estou pronto para imprimir a figura!" (imprimir depressa).
    Ready,
    Printing,
    Done,
}

/// Aviso do Rex por cima da máquina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alert {
    TooManyCopies,
    PosterBackground,
    /// "Sinto muito, por algum motivo eu não consigo imprimir isto." — fecha a máquina.
    CantPrint,
    /// "Você acabou de cancelar a impressão." — fecha a máquina.
    Canceled,
}

/// Fundo de impressão: miniatura e a figura inteira.
pub struct PrintBackground {
    pub name: String,
    pub thumb: Rc<crate::Image>,
    pub full: Rgb8,
}

/// Folhas gravadas pela thread de montagem, ou o erro.
type Rendered = Result<Vec<PathBuf>, String>;

/// Impressão em andamento.
struct Job {
    /// Montagem e gravação das folhas, numa thread (não trava a tela).
    render: Option<std::sync::mpsc::Receiver<Rendered>>,
    child: Option<std::process::Child>,
    t: f32,
    /// Resultado do `lp`, quando terminou.
    ok: Option<bool>,
}

pub struct PrintMachine {
    pub quick: bool,
    pub stage: Stage,
    pub options: Options,
    pub background: Option<usize>,
    backgrounds: Option<Vec<PrintBackground>>,
    thumb_first: usize,
    balloon: TextBalloon,
    alert: Option<(TextBalloon, Alert)>,
    picture: Rgb8,
    preview: Rgb8,
    job: Option<Job>,
    /// Programa de impressão (`ATELIE_LP`, senão `lp`).
    pub lp: String,
    /// Arquivos gravados na última impressão.
    pub files: Vec<PathBuf>,
}

/// Texto e opções de um registro, ou a reserva.
fn record_text(
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

/// Lê os fundos de impressão (`IBGF` de `0MAG100.BOR`): a miniatura `PKBM`
/// e a figura do `PICT` apontado (metarquivo com um DIB).
pub fn load_backgrounds(assets: &Assets) -> Vec<PrintBackground> {
    let Some(data) = assets.library_bytes(BACKGROUNDS_LIBRARY) else {
        return Vec::new();
    };
    let Ok(f) = formats::chnk::parse(data) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut images = 0;
    for c in &f.chunks {
        let thumb_index = c
            .parts
            .iter()
            .position(|p| formats::pkbm::is_pkbm(p.data))
            .map(|_| images);
        images += c
            .parts
            .iter()
            .filter(|p| formats::pkbm::is_pkbm(p.data))
            .count();
        if &c.tag.0 != b"IBGF" {
            continue;
        }
        let (Some(index), Some(ibgf)) = (thumb_index, c.parts.iter().find(|p| &p.tag.0 == b"IBGF"))
        else {
            continue;
        };
        if ibgf.data.len() < 6 || &ibgf.data[..4] != b"PICT" {
            continue;
        }
        let id = u16::from_le_bytes([ibgf.data[4], ibgf.data[5]]);
        let full = f
            .chunks
            .iter()
            .find(|p| &p.tag.0 == b"PICT" && p.id == id)
            .and_then(|p| p.parts.iter().find(|q| &q.tag.0 == b"META"))
            .and_then(|m| formats::wmf::meta_dib(m.data).ok());
        let Some(full) = full else { continue };
        out.push(PrintBackground {
            name: c.name.clone().unwrap_or_default(),
            thumb: assets.image(&ImageRef::Library {
                file: BACKGROUNDS_LIBRARY.to_string(),
                index,
            }),
            full: Rgb8::from_rgba(full.width, full.height, &full.rgba),
        });
    }
    out
}

impl PrintMachine {
    pub fn new(quick: bool, picture: Rgb8, assets: &Assets) -> PrintMachine {
        let stage = if quick { Stage::Ready } else { Stage::Welcome };
        let mut m = PrintMachine {
            quick,
            stage,
            options: Options::default(),
            background: None,
            backgrounds: None,
            thumb_first: 0,
            balloon: TextBalloon::new(0, 0, 120, Vec::new(), Vec::new()),
            alert: None,
            preview: Rgb8::new(1, 1),
            picture,
            job: None,
            lp: std::env::var("ATELIE_LP").unwrap_or_else(|_| "lp".into()),
            files: Vec::new(),
        };
        m.refresh_preview();
        m.go(stage, assets);
        m
    }

    pub fn alert(&self) -> Option<Alert> {
        self.alert.as_ref().map(|(_, a)| *a)
    }

    /// Texto da caixa de cópias (no passo das cópias).
    pub fn copies_text(&self) -> Option<&str> {
        (self.stage == Stage::Copies)
            .then(|| self.balloon.fields.first().map(String::as_str))
            .flatten()
    }

    fn background_image(&self) -> Option<&Rgb8> {
        let k = self.background?;
        self.backgrounds.as_ref()?.get(k).map(|b| &b.full)
    }

    fn refresh_preview(&mut self) {
        let c = Composer::new(&self.picture, self.background_image(), self.options);
        self.preview = c.preview((PAGE.2 - 2) as u32, (PAGE.3 - 2) as u32);
    }

    fn records(assets: &Assets, block: u16) -> Rc<Vec<Record>> {
        Rc::new(assets.dialogs(Program::Artist, block))
    }

    /// Vai ao passo `stage` e monta o balão dele.
    fn go(&mut self, stage: Stage, assets: &Assets) {
        self.stage = stage;
        let recs = Self::records(assets, WHAT_MACHINE);
        let rec = |k: usize| recs.get(k);
        use Button::{Back, Next, Stop};
        let (k, fallback, opts, buttons): (usize, &str, &[&str], Vec<Button>) = match stage {
            Stage::Welcome => (
                REC_WELCOME,
                "Oi! Seja bem-vindo à minha máquina de impressão.\nPara continuar, clique em Segue.",
                &[],
                vec![Stop, Next],
            ),
            Stage::Background => (
                REC_BACKGROUND,
                "Você quer colocar um fundo no desenho antes?",
                &[
                    "É, quero escolher um fundo",
                    "Sem fundo",
                    "Deixar como está",
                ],
                vec![Back, Stop],
            ),
            Stage::PickBackground => (
                REC_PICK_BACKGROUND,
                "Para ver os fundos, clique aqui. Quando você achar um que quer usar, clique nele.",
                &[],
                vec![Back, Stop, Next],
            ),
            Stage::Stretch => (
                REC_STRETCH,
                "Eu posso esticar a figura de várias formas antes de imprimir.",
                &[
                    "Deixar a figura no tamanho normal",
                    "Esticar a figura igualmente para caber no papel.",
                    "Esticar a figura para ocupar todo o espaço da página",
                    "Fazer um pôster",
                ],
                vec![Back, Stop, Next],
            ),
            Stage::Color => (
                REC_COLOR,
                "Como você quer que eu imprima a figura?",
                &[
                    "Em cores ou usando tons de cinza.",
                    "Em branco e preto sem tons de cinza",
                ],
                vec![Back, Stop, Next],
            ),
            Stage::Copies => (
                REC_COPIES,
                "Quantas cópias do documento você quer imprimir?",
                &[],
                vec![Back, Stop, Next],
            ),
            Stage::Ready => (
                REC_READY,
                "OK. Já estou pronto para imprimir a figura!\nQuando você estiver pronto para imprimir, clique em Segue.",
                &[],
                vec![Stop, Next],
            ),
            Stage::Printing => (
                REC_PRINTING,
                "Eu ainda estou imprimindo... um pouquinho de paciência!\nPara cancelar a impressão, aperte a tecla Esc.",
                &[],
                vec![],
            ),
            Stage::Done => (
                REC_DONE,
                "Terminei de imprimir a figura. Foi divertido!\nPara voltar ao seu trabalho, clique em Pára.",
                &[],
                vec![Stop],
            ),
        };
        let r = rec(k);
        let width = r.map_or(200, |r| r.width as i32).clamp(150, 260);
        let mut b = if stage == Stage::Copies
            && let Some(r) = r
        {
            TextBalloon::from_record(r, 0, BALLOON_TOP, "", buttons)
        } else if stage == Stage::Copies {
            TextBalloon::new(
                0,
                BALLOON_TOP,
                width,
                vec![
                    Part::Text(fallback.into()),
                    Part::Field {
                        field: 0,
                        width: 40,
                    },
                ],
                buttons,
            )
        } else {
            let (text, options) = record_text(r, fallback, opts);
            let mut parts = vec![Part::Text(text)];
            if !options.is_empty() {
                parts.push(Part::Options(options));
            }
            TextBalloon::new(0, BALLOON_TOP, width, parts, buttons)
        };
        b.x = BALLOON_RIGHT - b.w;
        if stage == Stage::Copies {
            if b.fields.is_empty() {
                b.fields.push(String::new());
            }
            b.fields[0] = self.options.copies.to_string();
            b.select_all = true;
        }
        self.balloon = b;
    }

    fn open_alert(&mut self, alert: Alert, assets: &Assets) {
        let (block, k, fallback) = match alert {
            Alert::TooManyCopies => (
                WHAT_MACHINE,
                REC_TOO_MANY,
                "Ops! Não consigo imprimir tantas cópias. Digite um número entre 1 e 10 na caixa.",
            ),
            Alert::PosterBackground => (
                WHAT_MACHINE,
                REC_POSTER_BACKGROUND,
                "Sinto muito! Eu não consigo imprimir um pôster com fundo! Eu vou apagar o fundo.",
            ),
            Alert::CantPrint => (
                WHAT_ERRORS,
                REC_CANT_PRINT,
                "Sinto muito, por algum motivo eu não consigo imprimir isto.\nVerifique a impressora e tente outra vez.",
            ),
            Alert::Canceled => (
                WHAT_ERRORS,
                REC_CANCELED,
                "Você acabou de cancelar a impressão.",
            ),
        };
        let recs = Self::records(assets, block);
        let b = match recs.get(k) {
            Some(r) if !r.plain().trim().is_empty() => {
                TextBalloon::from_record(r, ALERT_POS.0, ALERT_POS.1, "", vec![])
            }
            _ => TextBalloon::new(
                ALERT_POS.0,
                ALERT_POS.1,
                220,
                vec![
                    Part::Figure(crate::text_balloon::PIC_ALERT),
                    Part::Text(fallback.into()),
                ],
                vec![],
            ),
        };
        self.alert = Some((b.as_alert(), alert));
    }

    fn set_stretch(&mut self, s: Stretch, assets: &Assets) {
        self.options.stretch = s;
        if s == Stretch::Poster && self.background.is_some() {
            self.background = None;
            self.open_alert(Alert::PosterBackground, assets);
        }
        self.refresh_preview();
    }

    fn set_background(&mut self, k: Option<usize>, assets: &Assets) {
        self.background = k;
        if k.is_some() && self.options.stretch == Stretch::Poster {
            self.background = None;
            self.open_alert(Alert::PosterBackground, assets);
        }
        self.refresh_preview();
    }

    fn ensure_backgrounds(&mut self, assets: &Assets) {
        if self.backgrounds.is_none() {
            self.backgrounds = Some(load_backgrounds(assets));
        }
    }

    pub fn background_count(&self) -> usize {
        self.backgrounds.as_ref().map_or(0, Vec::len)
    }

    fn thumb_rect(&self, k: usize) -> Option<(i32, i32, i32, i32)> {
        let i = k.checked_sub(self.thumb_first)?;
        (i < THUMBS && k < self.background_count()).then_some((
            THUMB_X0 + i as i32 * THUMB_STEP,
            FUNDO_RECT.1,
            THUMB.0,
            THUMB.1,
        ))
    }

    /// Centro da miniatura `k` (para testes).
    pub fn thumb_center(&self, k: usize) -> Option<(i32, i32)> {
        self.thumb_rect(k).map(|r| (r.0 + r.2 / 2, r.1 + r.3 / 2))
    }

    /// Centro do botão `b` no balão (para testes).
    pub fn button_center(&self, a: &Assets, b: Button) -> Option<(i32, i32)> {
        self.balloon
            .button_rect(a, b)
            .map(|r| (r.0 + r.2 / 2, r.1 + r.3 / 2))
    }

    /// Ponto dentro da opção `k` do balão (para testes).
    pub fn option_point(&self, a: &Assets, k: usize) -> Option<(i32, i32)> {
        self.balloon
            .option_rect(a, k)
            .map(|r| (r.0 + 30, r.1 + r.3 / 2))
    }
}

/// O que a tela de pintura deve fazer depois de uma entrada na máquina.
enum Out {
    Stay,
    Close,
    Print,
}

impl PrintMachine {
    fn next(&mut self, fx: &mut Vec<Effect>, assets: &Assets) -> Out {
        match self.stage {
            Stage::Welcome => self.go(Stage::Background, assets),
            Stage::Background => {}
            Stage::PickBackground => self.go(Stage::Stretch, assets),
            Stage::Stretch => self.go(Stage::Color, assets),
            Stage::Color => self.go(Stage::Copies, assets),
            Stage::Copies => {
                let text = self.balloon.fields.first().cloned().unwrap_or_default();
                match pr::parse_copies(&text) {
                    Some(n) => {
                        self.options.copies = n;
                        fx.push(sfx::play(SND_PRINT_GO));
                        return Out::Print;
                    }
                    None => {
                        self.balloon.select_all = true;
                        self.open_alert(Alert::TooManyCopies, assets);
                    }
                }
            }
            Stage::Ready => {
                fx.push(sfx::play(SND_PRINT_GO));
                return Out::Print;
            }
            Stage::Printing => {}
            Stage::Done => return Out::Close,
        }
        Out::Stay
    }

    fn back(&mut self, assets: &Assets) {
        let to = match self.stage {
            Stage::Background => Stage::Welcome,
            Stage::PickBackground | Stage::Stretch => Stage::Background,
            Stage::Color => Stage::Stretch,
            Stage::Copies => Stage::Color,
            s => s,
        };
        if to != self.stage {
            self.go(to, assets);
        }
    }

    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) -> Out {
        if let Some((_, alert)) = self.alert.take() {
            // qualquer clique fecha o aviso
            return match alert {
                Alert::CantPrint | Alert::Canceled => Out::Close,
                _ => Out::Stay,
            };
        }
        if self.stage == Stage::Printing {
            fx.push(sfx::play(SND_BEEP));
            return Out::Stay;
        }
        if self.stage == Stage::Copies {
            let input = Input::Click {
                button: MouseButton::Left,
                x,
                y,
            };
            if let Some(b) = self.balloon.handle(a, &input) {
                return self.button(b, fx, a);
            }
            if self.balloon.contains(a, x, y) {
                return Out::Stay;
            }
        } else if let Some(b) = self.balloon.hit_button(a, x, y) {
            return self.button(b, fx, a);
        } else if let Some(k) = self.balloon.hit_option(a, x, y) {
            self.option(k, a);
            return Out::Stay;
        }
        let editing = !matches!(self.stage, Stage::Ready | Stage::Done);
        if editing && inside(FUNDO_RECT, x, y) {
            self.ensure_backgrounds(a);
            self.go(Stage::PickBackground, a);
            return Out::Stay;
        }
        if self.stage == Stage::PickBackground {
            if inside(MCZEE_RECT, x, y) {
                self.go(Stage::Stretch, a);
                return Out::Stay;
            }
            if inside(THUMB_LEFT, x, y) {
                self.thumb_first = self.thumb_first.saturating_sub(THUMBS);
                return Out::Stay;
            }
            if inside(THUMB_RIGHT, x, y) {
                if self.thumb_first + THUMBS < self.background_count() {
                    self.thumb_first += THUMBS;
                }
                return Out::Stay;
            }
            if let Some(k) = (0..self.background_count())
                .find(|&k| self.thumb_rect(k).is_some_and(|r| inside(r, x, y)))
            {
                self.set_background(Some(k), a);
                return Out::Stay;
            }
        }
        if !self.balloon.contains(a, x, y) {
            fx.push(sfx::play(SND_BEEP));
        }
        Out::Stay
    }

    fn button(&mut self, b: Button, fx: &mut Vec<Effect>, a: &Assets) -> Out {
        match b {
            Button::Stop => Out::Close,
            Button::Back => {
                self.back(a);
                Out::Stay
            }
            Button::Next => self.next(fx, a),
        }
    }

    fn option(&mut self, k: usize, a: &Assets) {
        match self.stage {
            Stage::Background => match k {
                0 => {
                    self.ensure_backgrounds(a);
                    self.go(Stage::PickBackground, a);
                }
                1 => {
                    self.set_background(None, a);
                    self.go(Stage::Stretch, a);
                }
                _ => self.go(Stage::Stretch, a),
            },
            Stage::Stretch => {
                if let Some(&s) = Stretch::ALL.get(k) {
                    self.set_stretch(s, a);
                }
            }
            Stage::Color => {
                self.options.mono = k == 1;
                self.refresh_preview();
            }
            _ => {}
        }
    }

    /// Opção marcada (bolinha verde) no passo atual.
    fn checked(&self) -> Option<usize> {
        match self.stage {
            Stage::Stretch => Stretch::ALL.iter().position(|&s| s == self.options.stretch),
            Stage::Color => Some(usize::from(self.options.mono)),
            _ => None,
        }
    }

    /// Monta as folhas e grava os PNG em `dir` numa thread; o `lp` é chamado
    /// quando elas ficam prontas (em `tick`).
    fn start_print(&mut self, dir: Option<&Path>, fx: &mut Vec<Effect>, assets: &Assets) {
        self.files.clear();
        let Some(dir) = dir else {
            fx.push(Effect::Log("imprimir: sem pasta de desenhos".into()));
            return self.fail(fx, assets);
        };
        let dir = dir.to_path_buf();
        let picture = self.picture.clone();
        let background = self.background_image().cloned();
        let options = self.options;
        let stamp = crate::paint::store::timestamp(now_secs());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(render_files(
                &dir,
                &picture,
                background.as_ref(),
                options,
                &stamp,
            ));
        });
        self.job = Some(Job {
            render: Some(rx),
            child: None,
            t: 0.0,
            ok: None,
        });
        self.go(Stage::Printing, assets);
    }

    /// Folhas prontas: chama o `lp`.
    fn spawn_lp(&mut self, files: Vec<PathBuf>, fx: &mut Vec<Effect>, assets: &Assets) {
        self.files = files;
        let Some(first) = self.files.first().cloned() else {
            return self.fail(fx, assets);
        };
        let child = std::process::Command::new(&self.lp)
            .args(pr::lp_args(self.options.copies, &self.files))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        match child {
            Ok(c) => {
                fx.push(Effect::Log(format!(
                    "imprimindo {} folha(s), {} cópia(s): {}",
                    self.files.len(),
                    self.options.copies,
                    first.display()
                )));
                if let Some(job) = self.job.as_mut() {
                    job.child = Some(c);
                }
            }
            Err(_) => {
                fx.push(Effect::Log(format!(
                    "sem impressora ({}): o desenho ficou em {}",
                    self.lp,
                    first.display()
                )));
                self.fail(fx, assets);
            }
        }
    }

    fn fail(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        self.job = None;
        // como no original, o "Epa!" aparece por cima do "Eu ainda estou imprimindo…"
        self.go(Stage::Printing, assets);
        fx.push(sfx::play(SND_ERROR));
        self.open_alert(Alert::CantPrint, assets);
    }

    fn cancel(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        if let Some(mut job) = self.job.take()
            && let Some(c) = job.child.as_mut()
        {
            let _ = c.kill();
            let _ = c.wait();
        }
        fx.push(Effect::Log("impressão cancelada".into()));
        self.open_alert(Alert::Canceled, assets);
    }

    fn tick(&mut self, dt: f32, fx: &mut Vec<Effect>, assets: &Assets) {
        let Some(job) = self.job.as_mut() else { return };
        if let Some(rx) = &job.render {
            use std::sync::mpsc::TryRecvError;
            let got = match rx.try_recv() {
                Ok(r) => r,
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => Err("a montagem das folhas parou".into()),
            };
            job.render = None;
            match got {
                Ok(files) => self.spawn_lp(files, fx, assets),
                Err(e) => {
                    fx.push(Effect::Log(format!("imprimir: {e}")));
                    self.fail(fx, assets);
                }
            }
            return;
        }
        let Some(job) = self.job.as_mut() else { return };
        if dt.is_finite() && dt > 0.0 {
            job.t += dt;
        }
        if job.ok.is_none()
            && let Some(c) = job.child.as_mut()
        {
            match c.try_wait() {
                Ok(Some(status)) => job.ok = Some(status.success()),
                Ok(None) => {}
                Err(_) => job.ok = Some(false),
            }
        }
        if job.t < MIN_PRINTING {
            return;
        }
        match job.ok {
            Some(true) => {
                self.job = None;
                self.go(Stage::Done, assets);
            }
            Some(false) => {
                fx.push(Effect::Log(format!("{}: a impressão falhou", self.lp)));
                self.fail(fx, assets);
            }
            None => {}
        }
    }

    fn draw(&self, f: &mut Frame, a: &Assets) {
        dim(f);
        // moldura e faixa cinza de cima, área branca embaixo
        f.rect(18, 30, 572, 428, ORANGE);
        f.rect(20, 32, 568, 424, BLACK);
        f.rect(21, 33, 566, 56, GRAY);
        f.rect(21, 89, 566, 2, BROWN);
        f.rect(21, 92, 566, 363, WHITE);
        if a.has_image(&pic(FUNDO.0)) {
            blit_pic(f, a, FUNDO.0, FUNDO.1, FUNDO.2, true);
        } else {
            let (x, y, w, h) = FUNDO_RECT;
            f.rect(x, y, w, h, BLACK);
            f.rect(x + 1, y + 1, w - 2, h - 2, WHITE);
            f.text(x + 40, y + 16, "FUNDO", BLACK);
        }
        if self.stage == Stage::PickBackground {
            self.draw_thumbs(f);
        }
        // folha da prévia (no pôster, as quatro folhas)
        let (px, py, pw, ph) = PAGE;
        f.rect(px, py, pw, ph, BLACK);
        let p = &self.preview;
        for y in 0..p.h {
            for x in 0..p.w {
                f.put(
                    px + 1 + x as i32,
                    py + 1 + y as i32,
                    p.px[(y * p.w + x) as usize],
                );
            }
        }
        // contorno da figura na folha (só na tela; o fundo traz a moldura dele)
        let (tw, th) = pr::total_size(self.options.stretch);
        let (sx, sy) = (p.w as f32 / tw as f32, p.h as f32 / th as f32);
        let r = pr::placement(self.options.stretch, self.picture.w, self.picture.h);
        if self.background.is_none() {
            f.frame_rect(
                px + 1 + (r.0 * sx) as i32,
                py + 1 + (r.1 * sy) as i32,
                (r.2 * sx).round() as i32,
                (r.3 * sy).round() as i32,
                1,
                BLACK,
            );
        }
        if self.options.stretch == Stretch::Poster {
            f.rect(px + pw / 2 - 1, py, 2, ph, BLACK);
            f.rect(px, py + ph / 2 - 1, pw, 2, BLACK);
        }
        blit_pic(f, a, MCZEE.0, MCZEE.1, MCZEE.2, true);
        self.balloon.draw(f, a);
        crate::balloon::draw_tail(
            f,
            BALLOON_RIGHT - 26,
            BALLOON_TOP,
            (BALLOON_RIGHT - 8, BALLOON_TOP - 20),
        );
        if let Some(k) = self.checked() {
            for i in 0..self.balloon.options().len() {
                let Some(r) = self.balloon.option_rect(a, i) else {
                    continue;
                };
                let ball = if i == k { PIC_BALL_CHECKED } else { PIC_BALL };
                let (bx, by) = (r.0 - 2, r.1 + (r.3.min(18) - 14) / 2);
                if a.has_image(&pic(ball)) {
                    blit_pic_opaque(f, a, ball, bx, by);
                } else if i == k {
                    f.rect(r.0 + 3, r.1 + 6, 4, 4, WHITE);
                }
            }
        }
        if let Some((b, _)) = &self.alert {
            b.draw(f, a);
        }
    }

    fn draw_thumbs(&self, f: &mut Frame) {
        let Some(bgs) = &self.backgrounds else { return };
        for (k, bg) in bgs.iter().enumerate() {
            let Some(r) = self.thumb_rect(k) else {
                continue;
            };
            f.rect(r.0, r.1, r.2, r.3, BLACK);
            f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, WHITE);
            blit_fit(f, &bg.thumb, (r.0 + 2, r.1 + 2, r.2 - 4, r.3 - 4));
            if self.background == Some(k) {
                f.frame_rect(r.0 - 2, r.1 - 2, r.2 + 4, r.3 + 4, 2, GREEN_RED);
            }
        }
        if self.thumb_first > 0 {
            arrow(f, THUMB_LEFT, true);
        }
        if self.thumb_first + THUMBS < bgs.len() {
            arrow(f, THUMB_RIGHT, false);
        }
    }
}

/// Escurece a tela de pintura por trás da máquina (tons de cinza claros).
fn dim(f: &mut Frame) {
    for y in 0..f.height() {
        for x in 0..f.width() {
            let p = f.pixel(x, y);
            let l = ((p[0] as u32 * 3 + p[1] as u32 * 6 + p[2] as u32) / 10) as u8;
            let g = (l / 2).saturating_add(70);
            f.put(x as i32, y as i32, [g, g, g]);
        }
    }
}

/// Seta preta para a esquerda ou a direita, centrada em `r`.
fn arrow(f: &mut Frame, r: (i32, i32, i32, i32), left: bool) {
    let cy = r.1 + r.3 / 2;
    for i in 0..r.2 {
        let half = if left { i } else { r.2 - 1 - i } / 2;
        f.rect(r.0 + i, cy - half, 1, 2 * half + 1, BLACK);
    }
}

/// Imagem reduzida (vizinho mais próximo) para caber em `r`, mantendo a proporção.
fn blit_fit(f: &mut Frame, img: &crate::Image, r: (i32, i32, i32, i32)) {
    if img.width == 0 || img.height == 0 {
        return;
    }
    let s = (r.2 as f32 / img.width as f32).min(r.3 as f32 / img.height as f32);
    let (w, h) = (
        (img.width as f32 * s) as i32,
        (img.height as f32 * s) as i32,
    );
    let (ox, oy) = (r.0 + (r.2 - w) / 2, r.1 + (r.3 - h) / 2);
    for y in 0..h {
        for x in 0..w {
            let sx = ((x as f32 / s) as u32).min(img.width - 1);
            let sy = ((y as f32 / s) as u32).min(img.height - 1);
            let k = ((sy * img.width + sx) * 4) as usize;
            let p = &img.rgba[k..k + 4];
            if p[3] != 0 {
                f.put(ox + x, oy + y, [p[0], p[1], p[2]]);
            }
        }
    }
}

/// Monta as folhas e grava `impressao-<carimbo>[-k].png` em `dir`.
fn render_files(
    dir: &Path,
    picture: &Rgb8,
    background: Option<&Rgb8>,
    options: Options,
    stamp: &str,
) -> Rendered {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("não consegui criar {}: {e}", dir.display()))?;
    let sheets = Composer::new(picture, background, options).sheets();
    let names = pr::sheet_names(stamp, sheets.len());
    let mut files = Vec::new();
    for (s, n) in sheets.iter().zip(&names) {
        let path = dir.join(n);
        pr::save_sheet(s, &path)
            .map_err(|e| format!("não consegui gravar {}: {e}", path.display()))?;
        files.push(path);
    }
    Ok(files)
}

impl Paint {
    pub fn print_machine(&self) -> Option<&PrintMachine> {
        self.print_machine.as_ref()
    }

    pub fn print_machine_mut(&mut self) -> Option<&mut PrintMachine> {
        self.print_machine.as_mut()
    }

    /// Abre a máquina: `quick` = "imprimir depressa".
    pub(super) fn open_print_machine(
        &mut self,
        quick: bool,
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) {
        let flat = self.composed();
        let mut pic = Rgb8::new(flat.w, flat.h);
        for (k, &i) in flat.px.iter().enumerate() {
            pic.px[k] = self.rgb(i);
        }
        self.help = None;
        self.menu_open = false;
        fx.push(sfx::play(SND_MCZEE_SCREEN));
        self.print_machine = Some(PrintMachine::new(quick, pic, assets));
    }

    /// Entrada com a máquina aberta: ela é modal. `true` se consumiu.
    pub(super) fn print_machine_input(
        &mut self,
        input: &Input,
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) -> bool {
        let Some(m) = self.print_machine.as_mut() else {
            return false;
        };
        let out = match input {
            Input::Click { x, y, .. } => m.click(*x, *y, fx, assets),
            Input::Key {
                key: Key::Escape, ..
            } => {
                if m.alert.is_some() {
                    m.click(-1, -1, fx, assets)
                } else if m.stage == Stage::Printing {
                    m.cancel(fx, assets);
                    Out::Stay
                } else {
                    Out::Close
                }
            }
            Input::Key {
                key: Key::Enter, ..
            } if m.alert.is_some() => m.click(-1, -1, fx, assets),
            Input::Key {
                key: Key::Enter, ..
            } if m.stage != Stage::Copies && m.balloon.buttons.contains(&Button::Next) => {
                m.next(fx, assets)
            }
            Input::Key { .. } | Input::Text(_) if m.stage == Stage::Copies && m.alert.is_none() => {
                match m.balloon.handle(assets, input) {
                    Some(b) => m.button(b, fx, assets),
                    None => Out::Stay,
                }
            }
            _ => Out::Stay,
        };
        match out {
            Out::Stay => {}
            Out::Close => self.print_machine = None,
            Out::Print => {
                let dir = self.drawings.clone();
                if let Some(m) = self.print_machine.as_mut() {
                    m.start_print(dir.as_deref(), fx, assets);
                }
            }
        }
        true
    }

    pub(super) fn print_machine_update(&mut self, dt: f32, fx: &mut Vec<Effect>, assets: &Assets) {
        if let Some(m) = self.print_machine.as_mut() {
            m.tick(dt, fx, assets);
        }
    }

    pub(super) fn draw_print_machine(&self, f: &mut Frame, a: &Assets) {
        if let Some(m) = &self.print_machine {
            m.draw(f, a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn key(k: Key) -> Input {
        Input::Key {
            key: k,
            ctrl: false,
            alt: false,
        }
    }

    /// Tela de pintura com a ferramenta Imprimir escolhida.
    fn with_print_strip(a: &Assets, dir: &Path) -> Paint {
        let mut p = Paint::new(Some(dir.to_path_buf()), a);
        p.update(&[click(470, 25)], 0.0, a);
        assert!(p.print_strip);
        p
    }

    fn press(p: &mut Paint, a: &Assets, b: Button) -> Vec<Effect> {
        let (x, y) = p.print_machine().unwrap().button_center(a, b).unwrap();
        p.update(&[click(x, y)], 0.0, a)
    }

    fn choose(p: &mut Paint, a: &Assets, k: usize) {
        let (x, y) = p.print_machine().unwrap().option_point(a, k).unwrap();
        p.update(&[click(x, y)], 0.0, a);
    }

    /// Deixa a impressão andar (thread das folhas e `lp`) até um aviso ou o fim.
    fn settle(p: &mut Paint, a: &Assets) {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < 60 {
            let m = p.print_machine().unwrap();
            if m.alert().is_some() || m.stage == Stage::Done {
                return;
            }
            p.update(&[], 0.05, a);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("a impressão não terminou");
    }

    fn stage(p: &Paint) -> Stage {
        p.print_machine().unwrap().stage
    }

    #[test]
    fn choices_walk_the_four_steps_and_print_with_them() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = with_print_strip(&a, dir.path());
        let fx = p.update(&[click(370, 80)], 0.0, &a);
        assert!(fx.contains(&sfx::play(SND_PRINT_MACHINE)));
        assert!(fx.contains(&sfx::play(SND_MCZEE_SCREEN)));
        assert_eq!(stage(&p), Stage::Welcome);
        // modal: clique no quadro só dá beep
        let fx = p.update(&[click(200, 300)], 0.0, &a);
        assert!(fx.contains(&sfx::play(SND_BEEP)));
        press(&mut p, &a, Button::Next);
        assert_eq!(stage(&p), Stage::Background);
        // "Deixar como está" segue direto
        choose(&mut p, &a, 2);
        assert_eq!(stage(&p), Stage::Stretch);
        choose(&mut p, &a, 3);
        assert_eq!(p.print_machine().unwrap().options.stretch, Stretch::Poster);
        press(&mut p, &a, Button::Back);
        assert_eq!(stage(&p), Stage::Background);
        choose(&mut p, &a, 1);
        press(&mut p, &a, Button::Next);
        assert_eq!(stage(&p), Stage::Color);
        choose(&mut p, &a, 1);
        assert!(p.print_machine().unwrap().options.mono);
        press(&mut p, &a, Button::Next);
        assert_eq!(stage(&p), Stage::Copies);
        assert_eq!(p.print_machine().unwrap().copies_text(), Some("1"));
        // 12 cópias: aviso do Rex, continua nas cópias
        p.update(&[Input::Text("12".into())], 0.0, &a);
        press(&mut p, &a, Button::Next);
        assert_eq!(
            p.print_machine().unwrap().alert(),
            Some(Alert::TooManyCopies)
        );
        p.update(&[click(5, 5)], 0.0, &a);
        assert_eq!(p.print_machine().unwrap().alert(), None);
        assert_eq!(stage(&p), Stage::Copies);
        p.update(&[Input::Text("3".into())], 0.0, &a);
        // sem lp: grava as quatro folhas e mostra o "Epa!"
        p.print_machine_mut().unwrap().lp = "/nao/existe/lp".into();
        let fx = p.update(&[key(Key::Enter)], 0.0, &a);
        assert!(fx.contains(&sfx::play(SND_PRINT_GO)));
        assert_eq!(stage(&p), Stage::Printing);
        settle(&mut p, &a);
        let m = p.print_machine().unwrap();
        assert_eq!(m.options.copies, 3);
        assert_eq!(m.alert(), Some(Alert::CantPrint));
        assert_eq!(m.files.len(), 4);
        assert!(m.files.iter().all(|f| f.exists()));
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        // o "Epa!" fecha a máquina; a ferramenta Imprimir continua escolhida
        p.update(&[click(300, 250)], 0.0, &a);
        assert!(p.print_machine().is_none());
        assert!(p.print_strip);
    }

    #[test]
    fn quick_shows_ready_then_prints_until_lp_ends() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = with_print_strip(&a, dir.path());
        p.update(&[click(170, 80)], 0.0, &a);
        assert_eq!(stage(&p), Stage::Ready);
        // Pára volta à pintura sem imprimir
        press(&mut p, &a, Button::Stop);
        assert!(p.print_machine().is_none());
        assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
        p.update(&[click(170, 80)], 0.0, &a);
        p.print_machine_mut().unwrap().lp = "true".into();
        press(&mut p, &a, Button::Next);
        assert_eq!(stage(&p), Stage::Printing);
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        settle(&mut p, &a);
        assert_eq!(stage(&p), Stage::Done);
        assert_eq!(p.print_machine().unwrap().files.len(), 1);
        press(&mut p, &a, Button::Stop);
        assert!(p.print_machine().is_none());

        // Esc cancela durante a impressão
        p.update(&[click(170, 80)], 0.0, &a);
        p.print_machine_mut().unwrap().lp = "sleep".into();
        // `sleep -n 1 -o fit-to-page …` falha logo; o importante é o Esc antes do tempo mínimo
        press(&mut p, &a, Button::Next);
        p.update(&[key(Key::Escape)], 0.0, &a);
        assert_eq!(p.print_machine().unwrap().alert(), Some(Alert::Canceled));
        p.update(&[key(Key::Enter)], 0.0, &a);
        assert!(p.print_machine().is_none());
    }

    #[test]
    fn real_backgrounds_load_and_poster_drops_background() {
        let Some(a) = assets() else { return };
        let bgs = load_backgrounds(&a);
        assert!(bgs.len() >= 8, "{}", bgs.len());
        assert!(bgs.iter().any(|b| b.name.contains("Island")));
        assert!(bgs.iter().all(|b| b.full.w > 100 && b.thumb.width > 10));
        let dir = tempfile::tempdir().unwrap();
        let mut p = with_print_strip(&a, dir.path());
        p.update(&[click(370, 80)], 0.0, &a);
        press(&mut p, &a, Button::Next);
        choose(&mut p, &a, 0);
        assert_eq!(stage(&p), Stage::PickBackground);
        let (x, y) = p.print_machine().unwrap().thumb_center(0).unwrap();
        p.update(&[click(x, y)], 0.0, &a);
        assert_eq!(p.print_machine().unwrap().background, Some(0));
        let mut f = Frame::new(640, 480);
        p.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "print_machine_background");
        // o McZee leva ao passo de esticar; pôster apaga o fundo com aviso
        p.update(&[click(590, 120)], 0.0, &a);
        assert_eq!(stage(&p), Stage::Stretch);
        choose(&mut p, &a, 3);
        let m = p.print_machine().unwrap();
        assert_eq!(m.alert(), Some(Alert::PosterBackground));
        assert_eq!(m.background, None);
    }
}
