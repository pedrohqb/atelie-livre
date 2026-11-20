//! Prévia e máquina de impressão do McZee, comum à Galeria do Fine Artist,
//! à Biblioteca do Creative Writer e à ferramenta Imprimir do Escritório.
//!
//! **Galeria** (o quadro arrastado para a impressora): uma janela (moldura
//! laranja, faixa cinza e a folha com o desenho pela metade) com o McZee no
//! alto à direita e o balão "OK. Já estou pronto para imprimir a figura!"
//! (`WHAT 4c00[11]`) com Para / Segue. Segue grava a folha e chama o `lp`;
//! enquanto ele trabalha (no mínimo `MIN_PRINTING` segundos) aparece "Eu
//! ainda estou imprimindo…" (`4c00[12]`) e um bichinho anda na faixa de
//! progresso; Esc cancela (`100f[20]`). No fim, "Terminei de imprimir a
//! figura" (`4c00[14]`) com Para; se o `lp` falhar ou não existir, o "EPA!"
//! do Rex (`100f[19]`) por cima do balão, que fecha com qualquer clique ou
//! tecla. Medido em `gallery_o_dprint_3.png` e `gallery_o_print_segue_3.png`.
//!
//! O que muda por programa é um [`PreviewSpec`]: [`ARTIST_PREVIEW`] (o
//! desenho pela metade numa folha deitada) e o `WRITER_PREVIEW` da
//! Biblioteca e do Escritório do Creative Writer (`writer_library`: a folha
//! em pé numa moldura cinza, balões `4c00`/`4c02` e avisos `0805[20]`/`[21]`
//! do `WRITER.EXE`, os bichos andando no canto). No original, arrastar um
//! documento para a impressora da Biblioteca vai direto ao "OK. Já estou
//! pronto para imprimir!", como o "imprimir depressa" do Escritório.
//!
//! **Máquina do Escritório** ([`PrintPreview::machine`], tarefa P16 de
//! `cw/report/projects-impressao.md`): com o [`MachineSpec`] do spec, antes
//! do "pronto" vêm as escolhas: boas-vindas `4c00[0]` → fundo `[1]` (só se o
//! documento tiver fundo) → páginas De/Até `[6]` (só documentos de várias
//! páginas) → cópias `[7]` (1 a 10) → pronto `[8]` com Volta / Pára / Segue.
//! Páginas ou cópias inválidas dão o aviso do Rex `4c02[0]`/`[1]`, que volta
//! ao mesmo passo. As folhas do documento ([`Sheet::Document`]) são montadas
//! numa thread por [`crate::writer::print`]. Ver
//! `docs/telas/creative-writer-impressao.md`.

use super::printing::{self, Poll, PrintJob};
use crate::assets::{Assets, Program, SoundRef};
use crate::paint::Canvas;
use crate::paint::print::{self as pr, Rgb8};
use crate::scene::{Effect, Input, Key, MouseButton};
use crate::text_balloon::{Button, TextBalloon, blit_pic_in};
use crate::ui::blit_canvas;
use crate::writer::print::{PrintDoc, SheetKind, SheetOptions};
use crate::{Frame, paint::store::save_png};
use formats::palette::Rgb;
use formats::what::{Item, PIC_OPTION_PICTURE, Record};
use std::path::{Path, PathBuf};

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

/// Os passos de escolha da máquina do Escritório (registros do programa).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MachineSpec {
    /// "Oi! Seja bem-vindo à minha máquina de impressão…" (Pára / Segue).
    pub welcome: Rec,
    /// "…você colocou um fundo. Você quer imprimir o fundo…?" (rádios).
    pub background: Rec,
    /// "Imprimir todas as páginas? / só algumas?" com De / Até.
    pub pages: Rec,
    /// "Quantas cópias…" com a caixa (`@` = o máximo).
    pub copies: Rec,
    /// Avisos do Rex: páginas e cópias inválidas.
    pub bad_pages: Rec,
    pub bad_copies: Rec,
}

/// O que muda entre a prévia da Galeria e a do Writer.
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
    /// Rabicho do balão até a boca do McZee: deslocamentos da base e da
    /// ponta a partir do canto de cima à direita.
    pub tail: Option<(i32, (i32, i32))>,
    pub alert_pos: (i32, i32),
    pub ready: Rec,
    pub printing: Rec,
    pub done: Rec,
    pub cant_print: Rec,
    /// "Não consigo achar a sua impressora" (sem programa de impressão); sem
    /// ele, vale o `cant_print`.
    pub no_printer: Option<Rec>,
    pub canceled: Rec,
    /// Passos de escolha da máquina do Escritório.
    pub machine: Option<MachineSpec>,
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
    tail: None,
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
    no_printer: None,
    canceled: printing::CANCELED,
    machine: None,
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
    /// Um documento do Writer: as folhas são montadas na hora de imprimir,
    /// com as escolhas da máquina.
    Document(Box<PrintDoc>),
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
    /// Boas-vindas da máquina (Pára / Segue).
    Welcome,
    /// Imprimir o fundo ou não.
    Background,
    /// Todas as páginas ou de… até….
    Pages,
    /// Quantas cópias.
    Copies,
    /// "OK. Já estou pronto…" com Para / Segue (e Volta, depois das escolhas).
    Ready,
    /// "Eu ainda estou imprimindo…".
    Printing,
    /// "Terminei de imprimir" com Para.
    Done,
    /// Aviso do Rex (não imprimiu ou cancelado) por cima do balão: qualquer
    /// clique fecha.
    Alert,
}

/// Escolhas feitas na máquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choices {
    pub sheet: SheetOptions,
    pub copies: u8,
    /// Caixas De / Até do passo das páginas.
    pub from: String,
    pub to: String,
}

/// A prévia já reduzida e onde ela vai.
struct Shown {
    /// Moldura cinza (ou coluna, na faixa) e a folha com a borda preta;
    /// `None` = sem folha branca (faixa: a tira fica direto na coluna).
    frame: Option<Rect>,
    sheet: Option<Rect>,
    image: Rgb8,
    at: (i32, i32),
}

pub struct PrintPreview {
    spec: &'static PreviewSpec,
    sheet: Sheet,
    /// Relógio do bicho que anda enquanto imprime.
    clock: f32,
    /// Pasta onde as folhas são gravadas (`impressao-….png`) antes do `lp`.
    dir: Option<PathBuf>,
    /// Programa de impressão (`lp`; os testes trocam).
    command: String,
    stage: Stage,
    balloon: TextBalloon,
    /// Aviso do Rex por cima do balão (o `Stage::Alert` ou um que volta ao
    /// passo, como cópias demais).
    alert: Option<TextBalloon>,
    /// O aviso aberto só avisa: fechá-lo volta ao passo.
    notice: bool,
    /// Passos de escolha antes do "pronto" (vazio = direto ao pronto).
    steps: Vec<Stage>,
    choices: Choices,
    shown: Option<Shown>,
    /// O programa de impressão nem começou (sem impressora).
    no_printer: bool,
    /// Impressão em andamento (desde Segue).
    job: Option<PrintJob>,
    /// Folhas gravadas na última impressão.
    files: Vec<PathBuf>,
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
    /// `writer_library::WRITER_PREVIEW`) e a folha `sheet`: direto ao
    /// "pronto".
    pub fn with_spec(
        spec: &'static PreviewSpec,
        sheet: Sheet,
        dir: Option<PathBuf>,
        command: &str,
        assets: &Assets,
    ) -> PrintPreview {
        PrintPreview::build(spec, sheet, Vec::new(), dir, command, assets)
    }

    /// A máquina de impressão do Escritório para `doc`: com `quick`
    /// ("imprimir depressa") vai direto ao "pronto"; senão passa pelas
    /// escolhas do [`MachineSpec`] do `spec`.
    pub fn machine(
        spec: &'static PreviewSpec,
        doc: PrintDoc,
        quick: bool,
        dir: Option<PathBuf>,
        command: &str,
        assets: &Assets,
    ) -> PrintPreview {
        let mut steps = Vec::new();
        if !quick && spec.machine.is_some() {
            steps.push(Stage::Welcome);
            if doc.has_background() {
                steps.push(Stage::Background);
            }
            if doc.kind == SheetKind::Pages && doc.page_count() > 1 {
                steps.push(Stage::Pages);
            }
            steps.push(Stage::Copies);
        }
        let sheet = Sheet::Document(Box::new(doc));
        PrintPreview::build(spec, sheet, steps, dir, command, assets)
    }

    fn build(
        spec: &'static PreviewSpec,
        sheet: Sheet,
        steps: Vec<Stage>,
        dir: Option<PathBuf>,
        command: &str,
        assets: &Assets,
    ) -> PrintPreview {
        let pages = match &sheet {
            Sheet::Document(d) => d.page_count().max(1),
            Sheet::Picture(..) => 1,
        };
        let first = steps.first().copied().unwrap_or(Stage::Ready);
        let mut p = PrintPreview {
            spec,
            sheet,
            clock: 0.0,
            dir,
            command: command.to_string(),
            stage: first,
            balloon: TextBalloon::new(0, 0, 120, Vec::new(), Vec::new()),
            alert: None,
            notice: false,
            steps,
            choices: Choices {
                sheet: SheetOptions::default(),
                copies: 1,
                from: "1".into(),
                to: pages.to_string(),
            },
            shown: None,
            no_printer: false,
            job: None,
            files: Vec::new(),
        };
        p.refresh_preview();
        p.go(first, assets);
        p
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn balloon(&self) -> &TextBalloon {
        &self.balloon
    }

    /// Aviso do Rex aberto.
    pub fn alert(&self) -> Option<&TextBalloon> {
        self.alert.as_ref()
    }

    pub fn choices(&self) -> &Choices {
        &self.choices
    }

    /// Folhas gravadas na última impressão.
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// O documento que vai ser impresso (prévia do Writer).
    pub fn document(&self) -> Option<&PrintDoc> {
        match &self.sheet {
            Sheet::Document(d) => Some(d),
            Sheet::Picture(..) => None,
        }
    }

    /// Centro do botão `b` do balão (para testes e roteiros).
    pub fn button_center(&self, a: &Assets, b: Button) -> Option<(i32, i32)> {
        self.balloon
            .button_rect(a, b)
            .map(|r| (r.0 + r.2 / 2, r.1 + r.3 / 2))
    }

    /// Centro do rádio `k` (na ordem da tela) do balão.
    pub fn radio_center(&self, a: &Assets, k: usize) -> Option<(i32, i32)> {
        self.balloon
            .cell_rect(a, k)
            .map(|r| (r.0 + r.2 / 2, r.1 + r.3 / 2))
    }

    /// Centro da caixa de texto `k` do balão.
    pub fn field_center(&self, a: &Assets, k: usize) -> Option<(i32, i32)> {
        self.balloon
            .field_rect(a, k)
            .map(|r| (r.0 + r.2 / 2, r.1 + r.3 / 2))
    }

    fn rec(&self, assets: &Assets, (block, k, fallback): Rec) -> Record {
        let recs = assets.dialogs(self.spec.program, block);
        match recs.get(k) {
            Some(r) if !r.plain().trim().is_empty() => r.clone(),
            _ => fallback_record(fallback),
        }
    }

    /// Balão do passo `stage` encostado no canto de cima à direita.
    fn step_balloon(&self, assets: &Assets, rec: Rec, buttons: Vec<Button>) -> TextBalloon {
        let r = self.rec(assets, rec);
        let max = pr::MAX_COPIES.to_string();
        let mut b = TextBalloon::from_record(&r, 0, self.spec.balloon_top, &max, buttons);
        b.x = self.spec.balloon_right - b.w;
        b
    }

    fn go(&mut self, stage: Stage, assets: &Assets) {
        if stage == Stage::Alert {
            return self.open_alert(self.spec.cant_print, false, assets);
        }
        self.stage = stage;
        let m = self.spec.machine;
        let choosing = !self.steps.is_empty();
        let back = |b: Vec<Button>| {
            let mut v = vec![Button::Back];
            v.extend(b);
            v
        };
        use Button::{Next, Stop};
        let (rec, buttons) = match (stage, m) {
            (Stage::Welcome, Some(m)) => (m.welcome, vec![Stop, Next]),
            (Stage::Background, Some(m)) => (m.background, back(vec![Stop, Next])),
            (Stage::Pages, Some(m)) => (m.pages, back(vec![Stop, Next])),
            (Stage::Copies, Some(m)) => (m.copies, back(vec![Stop, Next])),
            (Stage::Ready, _) if choosing => (self.spec.ready, back(vec![Stop, Next])),
            (Stage::Printing, _) => (self.spec.printing, vec![]),
            (Stage::Done, _) => (self.spec.done, vec![Stop]),
            _ => (self.spec.ready, vec![Stop, Next]),
        };
        let mut b = self.step_balloon(assets, rec, buttons);
        match stage {
            Stage::Background => {
                b.selected = radio_cell(&b, self.choices.sheet.background);
            }
            Stage::Pages => {
                b.selected = radio_cell(&b, self.choices.sheet.pages.is_none());
                set_fields(&mut b, &[&self.choices.from, &self.choices.to]);
                b.focus = 0;
            }
            Stage::Copies => {
                set_fields(&mut b, &[&self.choices.copies.to_string()]);
                b.select_all = true;
            }
            _ => {}
        }
        self.balloon = b;
    }

    /// Aviso do Rex; `notice` = só avisa e volta ao passo.
    fn open_alert(&mut self, rec: Rec, notice: bool, assets: &Assets) {
        let b = printing::alert_balloon_in(assets, self.spec.program, rec, self.spec.alert_pos);
        self.alert = Some(b);
        self.notice = notice;
        if !notice {
            self.stage = Stage::Alert;
        }
    }

    /// A prévia mostrada: a folha reduzida (a capa, no cartão; a tira toda,
    /// na faixa), com as escolhas atuais.
    fn refresh_preview(&mut self) {
        let Sheet::Document(doc) = &self.sheet else {
            return;
        };
        self.shown = Some(document_preview(self.spec, doc, &self.choices.sheet));
    }

    /// Escolhas do balão (rádios e caixas) para o estado.
    fn read_balloon(&mut self) {
        let b = &self.balloon;
        match self.stage {
            Stage::Background => {
                let on = radio_value(b, b.selected);
                if on != self.choices.sheet.background {
                    self.choices.sheet.background = on;
                    self.refresh_preview();
                }
            }
            Stage::Pages => {
                if let [from, to, ..] = b.fields.as_slice() {
                    self.choices.from = from.clone();
                    self.choices.to = to.clone();
                }
                let all = radio_value(b, b.selected);
                self.choices.sheet.pages = if all {
                    None
                } else {
                    Some(parse_range(&self.choices.from, &self.choices.to).unwrap_or((0, 0)))
                };
            }
            _ => {}
        }
    }

    fn step_index(&self) -> Option<usize> {
        self.steps.iter().position(|&s| s == self.stage)
    }

    fn back(&mut self, assets: &Assets) {
        let to = match self.stage {
            Stage::Ready => self.steps.last().copied(),
            _ => self
                .step_index()
                .and_then(|k| k.checked_sub(1))
                .map(|k| self.steps[k]),
        };
        if let Some(s) = to {
            self.go(s, assets);
        }
    }

    /// Segue: confere o passo e vai ao seguinte. `true` = imprimir.
    fn next(&mut self, fx: &mut Vec<Effect>, assets: &Assets) -> bool {
        let s = self.spec;
        match self.stage {
            Stage::Pages => {
                let pages = self.document().map_or(1, PrintDoc::page_count);
                if let Some((from, to)) = self.choices.sheet.pages
                    && !(from >= 1 && from <= to && to <= pages)
                {
                    fx.push(snd_in(s.program, s.snd_alert));
                    let m = s.machine.expect("passo da máquina");
                    self.open_alert(m.bad_pages, true, assets);
                    return false;
                }
            }
            Stage::Copies => {
                let text = self.balloon.fields.first().cloned().unwrap_or_default();
                match pr::parse_copies(&text) {
                    Some(n) => self.choices.copies = n,
                    None => {
                        fx.push(snd_in(s.program, s.snd_alert));
                        self.balloon.select_all = true;
                        let m = s.machine.expect("passo da máquina");
                        self.open_alert(m.bad_copies, true, assets);
                        return false;
                    }
                }
            }
            Stage::Ready => return true,
            _ => {}
        }
        let next = match self.step_index() {
            Some(k) => self.steps.get(k + 1).copied().unwrap_or(Stage::Ready),
            None => Stage::Ready,
        };
        fx.push(snd_in(s.program, s.snd_next));
        self.go(next, assets);
        false
    }

    /// Grava as folhas e chama o programa de impressão.
    fn start(&mut self, fx: &mut Vec<Effect>) {
        self.files.clear();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let Some(dir) = self.dir.clone() else {
            fx.push(Effect::Log("imprimir: sem pasta de desenhos".into()));
            self.job.insert(PrintJob::new()).fail();
            return;
        };
        let stamp = crate::paint::store::timestamp(now);
        match &self.sheet {
            Sheet::Picture(c, pal) => {
                let job = self.job.insert(PrintJob::new());
                let path = dir.join(format!("impressao-{stamp}.png"));
                let saved = std::fs::create_dir_all(&dir).and_then(|()| save_png(c, pal, &path));
                if let Err(e) = saved {
                    fx.push(Effect::Log(format!("imprimir: {e}")));
                    job.fail();
                    return;
                }
                self.files = vec![path];
                self.spawn_lp(fx);
            }
            Sheet::Document(doc) => {
                let doc = (**doc).clone();
                let opts = self.choices.sheet;
                self.job = Some(PrintJob::render(move || {
                    save_sheets(&dir, &doc.sheets(&opts), &stamp)
                }));
            }
        }
    }

    /// Folhas prontas: chama o `lp` com as cópias.
    fn spawn_lp(&mut self, fx: &mut Vec<Effect>) {
        let Some(job) = self.job.as_mut() else { return };
        let Some(first) = self.files.first().cloned() else {
            job.fail();
            return;
        };
        match job.lp(&self.command, self.choices.copies, &self.files) {
            Ok(()) => fx.push(Effect::Log(format!(
                "imprimindo {} folha(s), {} cópia(s): {}",
                self.files.len(),
                self.choices.copies,
                first.display()
            ))),
            Err(_) => {
                self.no_printer = true;
                fx.push(Effect::Log(format!(
                    "sem impressora ({}): as folhas ficaram em {}",
                    self.command,
                    first.display()
                )));
            }
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
            Poll::Rendered(Ok(files)) => {
                self.files = files;
                self.spawn_lp(&mut fx);
            }
            Poll::Rendered(Err(e)) => {
                fx.push(Effect::Log(format!("imprimir: {e}")));
                job.fail();
            }
            Poll::Finished(true) => self.go(Stage::Done, assets),
            Poll::Finished(false) => {
                fx.push(snd_in(self.spec.program, self.spec.snd_alert));
                let rec = match self.spec.no_printer {
                    Some(r) if self.no_printer => r,
                    _ => self.spec.cant_print,
                };
                self.open_alert(rec, false, assets);
            }
            Poll::Rendering | Poll::Waiting => {}
        }
        fx
    }

    /// Trata uma entrada; `true` quando a prévia fecha (volta à tela).
    pub fn input(&mut self, i: &Input, assets: &Assets, fx: &mut Vec<Effect>) -> bool {
        let (p, s) = (self.spec.program, self.spec);
        let snd = |id| snd_in(p, id);
        let any = matches!(
            i,
            Input::Click {
                button: MouseButton::Left,
                ..
            } | Input::Key { .. }
        );
        if self.alert.is_some() && self.notice {
            // o aviso só avisa: qualquer clique ou tecla volta ao passo
            if any {
                self.alert = None;
                self.notice = false;
            }
            return false;
        }
        match self.stage {
            Stage::Printing => {
                if let Input::Key {
                    key: Key::Escape, ..
                } = i
                {
                    if let Some(job) = self.job.as_mut() {
                        job.cancel();
                    }
                    self.job = None;
                    fx.push(snd(s.snd_alert));
                    self.open_alert(s.canceled, false, assets);
                }
                false
            }
            Stage::Alert => any,
            _ => {
                if let Input::Key {
                    key: Key::Escape, ..
                } = i
                {
                    fx.push(snd(s.snd_stop));
                    return true;
                }
                let pressed = self.balloon.handle(assets, i);
                if let (Stage::Pages, Input::Text(_)) = (self.stage, i) {
                    // digitar nas caixas escolhe "só algumas páginas"
                    self.balloon.selected = radio_cell(&self.balloon, false);
                }
                self.read_balloon();
                match pressed {
                    Some(Button::Stop) => {
                        fx.push(snd(s.snd_stop));
                        true
                    }
                    Some(Button::Back) => {
                        self.back(assets);
                        false
                    }
                    Some(Button::Next) if self.stage != Stage::Done => {
                        if self.next(fx, assets) {
                            fx.push(snd(s.snd_next));
                            self.go(Stage::Printing, assets);
                            self.start(fx);
                        }
                        false
                    }
                    _ => false,
                }
            }
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
        if let Some(sh) = &self.shown {
            if let Some((x, y, w, h)) = sh.frame {
                f.rect(x, y, w, h, GRAY);
            }
            if let Some((x, y, w, h)) = sh.sheet {
                f.rect(x, y, w, h, BLACK);
                f.rect(x + 1, y + 1, w - 2, h - 2, WHITE);
            }
            blit_rgb(f, &sh.image, sh.at.0, sh.at.1);
            return;
        }
        if let Some((x, y, w, h)) = self.spec.sheet_frame {
            f.rect(x, y, w, h, GRAY);
        }
        let (px, py, pw, ph) = self.spec.page;
        f.rect(px, py, pw, ph, BLACK);
        f.rect(px + 1, py + 1, pw - 2, ph - 2, WHITE);
        if let Sheet::Picture(c, pal) = &self.sheet {
            // o desenho pela metade, no meio da folha
            let half = crate::paint::drawings::shrink(c, 2);
            let (w, h) = (half.w as i32, half.h as i32);
            blit_canvas(f, &half, pal, px + (pw - w) / 2, py + (ph - h) / 2);
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
        if let Some((base, tip)) = self.spec.tail {
            let (r, t) = (self.spec.balloon_right, self.spec.balloon_top);
            crate::balloon::draw_tail_box_in(
                f,
                self.spec.program,
                (r + base - 40, t, 80, 40),
                (r + tip.0, t + tip.1),
            );
        }
        if let Some(b) = &self.alert {
            b.draw(f, a);
        }
    }
}

/// Grava as folhas como `impressao-<carimbo>[-k].png` em `dir`.
fn save_sheets(dir: &Path, sheets: &[Rgb8], stamp: &str) -> printing::Rendered {
    if sheets.is_empty() {
        return Err("nenhuma folha para imprimir".into());
    }
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("não consegui criar {}: {e}", dir.display()))?;
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

/// "De" e "Até" digitados: números de página, de 1 em diante.
pub fn parse_range(from: &str, to: &str) -> Option<(usize, usize)> {
    Some((from.trim().parse().ok()?, to.trim().parse().ok()?))
}

/// Índices `[22e:N]` dos rádios do balão, na ordem da tela.
fn radio_ids(b: &TextBalloon) -> Vec<u16> {
    b.parts
        .iter()
        .flat_map(|p| match p {
            crate::text_balloon::Part::Rich { items, .. } => items
                .iter()
                .filter_map(|i| match i {
                    Item::Picture { id, param } if *id == PIC_OPTION_PICTURE => Some(*param),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// Rádio da tela com o valor `on` (`[22e:1]` = sim, `[22e:0]` = não).
fn radio_cell(b: &TextBalloon, on: bool) -> usize {
    radio_ids(b)
        .iter()
        .position(|&k| (k == 1) == on)
        .unwrap_or(0)
}

/// Valor do rádio marcado.
fn radio_value(b: &TextBalloon, cell: usize) -> bool {
    radio_ids(b).get(cell).is_none_or(|&k| k == 1)
}

fn set_fields(b: &mut TextBalloon, values: &[&str]) {
    for (k, v) in values.iter().enumerate() {
        if k < b.fields.len() {
            b.fields[k] = v.to_string();
        } else {
            b.fields.push(v.to_string());
        }
    }
}

/// Registro reserva (sem os dados originais): linhas de texto, com
/// `(N) texto` para um rádio `[22e:N]` e `[]` para uma caixa de texto.
fn fallback_record(text: &str) -> Record {
    let mut items = Vec::new();
    let txt = |t: &str| Item::Text {
        text: t.to_string(),
        bold: false,
        color: 0xf,
        indent: 0,
        key: false,
        signature: false,
    };
    for line in text.split('\n') {
        if let Some(rest) = line.strip_prefix('(')
            && let Some((n, t)) = rest.split_once(") ")
            && let Ok(n) = n.parse::<u16>()
        {
            items.push(Item::Picture {
                id: PIC_OPTION_PICTURE,
                param: n,
            });
            items.push(txt(&format!(" {t}")));
        } else if let Some((label, _)) = line.split_once("[]") {
            items.push(txt(label));
            items.push(Item::Picture { id: 0, param: 28 });
        } else {
            items.push(txt(line));
        }
        items.push(Item::Newline);
    }
    Record {
        title: String::new(),
        width: 200,
        height: 0,
        style: 2,
        items,
    }
}

/// A prévia de um documento: no documento e no jornal, a primeira folha
/// que sai (em pé ou deitada); no cartão, só a capa no tamanho de uma
/// página; na faixa, as páginas emendadas numa tira estreita no meio de uma
/// coluna cinza (o papel contínuo).
fn document_preview(spec: &PreviewSpec, doc: &PrintDoc, opts: &SheetOptions) -> Shown {
    let frame = spec.sheet_frame.unwrap_or(spec.page);
    let full = match doc.kind {
        SheetKind::Banner => {
            let column = (frame.0, frame.1, 70, frame.3);
            let n = doc.page_count().max(1);
            let pages: Vec<Rgb8> = (0..n)
                .map(|k| doc.page_image(k, 1.0, opts.background))
                .collect();
            let (pw, ph) = (pages[0].w, pages[0].h);
            let mut strip = Rgb8::new(pw * n as u32, ph);
            for (k, p) in pages.iter().enumerate() {
                for y in 0..p.h.min(ph) {
                    for x in 0..p.w.min(pw) {
                        strip.px[(y * strip.w + k as u32 * pw + x) as usize] =
                            p.px[(y * p.w + x) as usize];
                    }
                }
            }
            let w = (column.2 - 16).max(4) as u32;
            let mut img = fit_darkest(&strip, w, column.3 as u32);
            // contorno e emendas das páginas
            outline(&mut img);
            for k in 1..n {
                let x = (k as u32 * img.w / n as u32).min(img.w - 1);
                for y in 0..img.h {
                    img.px[(y * img.w + x) as usize] = BLACK;
                }
            }
            let at = (
                column.0 + (column.2 - img.w as i32) / 2,
                column.1 + (column.3 - img.h as i32) / 2,
            );
            return Shown {
                frame: Some(column),
                sheet: None,
                image: img,
                at,
            };
        }
        SheetKind::Card => doc.page_image(0, 1.0, opts.background),
        SheetKind::Pages => match doc.plan(opts).first() {
            Some(p) => doc.render_sheet(p, opts, 1.0),
            None => Rgb8::new(doc.geom.width as u32, doc.geom.height as u32),
        },
    };
    // folha deitada: a moldura e a folha deitam também
    let wide = full.w > full.h;
    let frame = if wide {
        (frame.0, frame.1, frame.3, frame.2)
    } else {
        frame
    };
    let inner = (frame.0 + 10, frame.1 + 8, frame.2 - 20, frame.3 - 16);
    let k = ((inner.2 - 2) as f32 / full.w as f32).min((inner.3 - 2) as f32 / full.h as f32);
    let (w, h) = (
        ((full.w as f32 * k) as u32).max(1),
        ((full.h as f32 * k) as u32).max(1),
    );
    let image = fit_darkest(&full, w, h);
    let sheet = (
        inner.0 + (inner.2 - w as i32 - 2) / 2,
        inner.1 + (inner.3 - h as i32 - 2) / 2,
        w as i32 + 2,
        h as i32 + 2,
    );
    Shown {
        frame: spec.sheet_frame.map(|_| frame),
        sheet: Some(sheet),
        image,
        at: (sheet.0 + 1, sheet.1 + 1),
    }
}

/// Contorno preto de 1 px em volta da imagem.
fn outline(img: &mut Rgb8) {
    let (w, h) = (img.w, img.h);
    for x in 0..w {
        img.px[x as usize] = BLACK;
        img.px[((h - 1) * w + x) as usize] = BLACK;
    }
    for y in 0..h {
        img.px[(y * w) as usize] = BLACK;
        img.px[(y * w + w - 1) as usize] = BLACK;
    }
}

fn blit_rgb(f: &mut Frame, img: &Rgb8, x0: i32, y0: i32) {
    for y in 0..img.h {
        for x in 0..img.w {
            f.put(
                x0 + x as i32,
                y0 + y as i32,
                img.px[(y * img.w + x) as usize],
            );
        }
    }
}

/// A imagem reduzida para caber em `w`×`h` (proporção mantida): o pixel
/// mais escuro de cada bloco, para o texto miúdo não sumir.
pub fn fit_darkest(s: &Rgb8, w: u32, h: u32) -> Rgb8 {
    if s.w == 0 || s.h == 0 || w == 0 || h == 0 {
        return Rgb8::new(w.max(1), h.max(1));
    }
    let k = (w as f32 / s.w as f32).min(h as f32 / s.h as f32);
    let (w, h) = (
        ((s.w as f32 * k) as u32).max(1),
        ((s.h as f32 * k) as u32).max(1),
    );
    let mut out = Rgb8::new(w, h);
    for ty in 0..h {
        let sy0 = (ty * s.h / h).min(s.h - 1);
        let sy1 = ((ty + 1) * s.h).div_ceil(h).clamp(sy0 + 1, s.h);
        for tx in 0..w {
            let sx0 = (tx * s.w / w).min(s.w - 1);
            let sx1 = ((tx + 1) * s.w).div_ceil(w).clamp(sx0 + 1, s.w);
            let mut best = WHITE;
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
            out.px[(ty * w + tx) as usize] = best;
        }
    }
    out
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
    use crate::writer::borders::{PageArt, PageGeom};

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

    fn key(k: Key) -> Input {
        Input::Key {
            key: k,
            ctrl: false,
            alt: false,
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

    fn settle(p: &mut PrintPreview, a: &Assets) -> Vec<Effect> {
        let mut fx = Vec::new();
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < 120 {
            fx.extend(p.update(0.05, a));
            if p.stage() != Stage::Printing {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        fx
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
        settle(&mut p, &a);
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
        assert!(p.alert().is_some());
        assert!(p.input(&click(5, 5), &a, &mut Vec::new()));
    }

    #[test]
    fn escape_cancels_printing() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = preview("sleep", dir.path(), &a);
        let mut fx = Vec::new();
        press(&mut p, Button::Next, &a, &mut fx);
        let esc = key(Key::Escape);
        let mut fx = Vec::new();
        assert!(!p.input(&esc, &a, &mut fx));
        assert_eq!(p.stage(), Stage::Alert);
        assert!(fx.contains(&snd(SND_ALERT)));
        assert!(p.input(&esc, &a, &mut fx));
    }

    // ------------------------------------------------------------------
    // Máquina do Escritório

    const TEST_MACHINE: PreviewSpec = PreviewSpec {
        program: Program::Writer,
        sheet_frame: Some((66, 62, 210, 270)),
        page: (76, 70, 190, 254),
        tail: Some((-26, (-16, -12))),
        no_printer: Some((0x0805, 20, "Não consigo achar a sua impressora.")),
        machine: Some(MachineSpec {
            welcome: (
                0x4c00,
                0,
                "Oi! Seja bem-vindo à minha máquina de impressão.",
            ),
            background: (
                0x4c00,
                1,
                "Você quer imprimir o fundo?\n(1) É, imprimir o fundo.\n(0) Não, vamos ignorar.",
            ),
            pages: (
                0x4c00,
                6,
                "O que você quer que eu faça?\n(1) Todas\n(0) Algumas\nDe: []\nAté: []",
            ),
            copies: (0x4c00, 7, "Quantas cópias?\n[]"),
            bad_pages: (0x4c02, 0, "Eu não consigo achar as páginas!"),
            bad_copies: (0x4c02, 1, "Ops! Não consigo imprimir tantas cópias."),
        }),
        ..ARTIST_PREVIEW
    };

    fn wsnd(id: u16) -> Effect {
        snd_in(Program::Writer, id)
    }

    fn doc(kind: SheetKind, n: usize, background: bool) -> PrintDoc {
        let mut art = PageArt::default();
        if background {
            art.background = Some(crate::writer::borders::PageBackground {
                background: formats::bor::Background::new(
                    1,
                    "x",
                    formats::bor::Ibgf::parse(&[0; 22]).unwrap(),
                    Vec::new(),
                    None,
                ),
                pictures: Vec::new(),
            });
        }
        PrintDoc {
            kind,
            geom: PageGeom::new(595.0, 792.0, 72.0),
            paper: crate::writer::print::DEFAULT_PAPER,
            pages: (0..n).map(|_| Rgb8::new(595, 792)).collect(),
            art,
        }
    }

    fn machine(d: PrintDoc, quick: bool, cmd: &str, dir: &Path, a: &Assets) -> PrintPreview {
        PrintPreview::machine(&TEST_MACHINE, d, quick, Some(dir.to_path_buf()), cmd, a)
    }

    fn type_in(p: &mut PrintPreview, a: &Assets, field: usize, text: &str) {
        let (x, y) = p.field_center(a, field).unwrap();
        let mut fx = Vec::new();
        p.input(&click(x, y), a, &mut fx);
        p.input(&key(Key::Backspace), a, &mut fx);
        p.input(&key(Key::Backspace), a, &mut fx);
        p.input(&Input::Text(text.into()), a, &mut fx);
    }

    #[test]
    fn machine_walks_background_pages_and_copies() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = machine(
            doc(SheetKind::Pages, 3, true),
            false,
            "true",
            dir.path(),
            &a,
        );
        assert_eq!(p.stage(), Stage::Welcome);
        let mut fx = Vec::new();
        press(&mut p, Button::Next, &a, &mut fx);
        assert!(fx.contains(&wsnd(SND_NEXT)));
        assert_eq!(p.stage(), Stage::Background);
        assert!(p.choices().sheet.background);
        // "Não, vamos ignorar": o segundo rádio
        let (x, y) = p.radio_center(&a, 1).unwrap();
        p.input(&click(x, y), &a, &mut fx);
        assert!(!p.choices().sheet.background);
        press(&mut p, Button::Next, &a, &mut fx);
        assert_eq!(p.stage(), Stage::Pages);
        assert_eq!(p.balloon().fields, ["1", "3"]);
        // de 3 até 2: o Rex reclama e o passo continua
        type_in(&mut p, &a, 0, "3");
        type_in(&mut p, &a, 1, "2");
        assert_eq!(p.choices().sheet.pages, Some((3, 2)));
        let mut fx = Vec::new();
        press(&mut p, Button::Next, &a, &mut fx);
        assert!(fx.contains(&wsnd(SND_ALERT)));
        assert!(p.alert().is_some());
        assert_eq!(p.stage(), Stage::Pages);
        p.input(&click(1, 1), &a, &mut fx);
        assert!(p.alert().is_none());
        type_in(&mut p, &a, 0, "2");
        type_in(&mut p, &a, 1, "3");
        press(&mut p, Button::Next, &a, &mut fx);
        assert_eq!(p.stage(), Stage::Copies);
        // Volta e Segue mantêm as escolhas
        press(&mut p, Button::Back, &a, &mut fx);
        assert_eq!(p.stage(), Stage::Pages);
        assert_eq!(p.choices().sheet.pages, Some((2, 3)));
        press(&mut p, Button::Next, &a, &mut fx);
        assert_eq!(p.balloon().fields, ["1"]);
        p.input(&Input::Text("12".into()), &a, &mut fx);
        press(&mut p, Button::Next, &a, &mut fx);
        assert!(p.alert().is_some());
        assert_eq!(p.stage(), Stage::Copies);
        p.input(&key(Key::Enter), &a, &mut fx);
        p.input(&Input::Text("2".into()), &a, &mut fx);
        // Enter é Segue
        p.input(&key(Key::Enter), &a, &mut fx);
        assert_eq!(p.stage(), Stage::Ready);
        assert_eq!(p.choices().copies, 2);
        assert!(p.balloon().buttons.contains(&Button::Back));
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        p.draw(&mut f, &a);
        press(&mut p, Button::Next, &a, &mut fx);
        assert_eq!(p.stage(), Stage::Printing);
        settle(&mut p, &a);
        assert_eq!(p.stage(), Stage::Done);
        // páginas 2 e 3: duas folhas
        assert_eq!(p.files().len(), 2);
        assert!(p.files().iter().all(|f| f.exists()));
        assert!(press(&mut p, Button::Stop, &a, &mut fx));
    }

    #[test]
    fn machine_skips_steps_the_document_does_not_need() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        // cartão sem fundo: boas-vindas → cópias
        let mut p = machine(
            doc(SheetKind::Card, 4, false),
            false,
            "true",
            dir.path(),
            &a,
        );
        let mut fx = Vec::new();
        press(&mut p, Button::Next, &a, &mut fx);
        assert_eq!(p.stage(), Stage::Copies);
        press(&mut p, Button::Back, &a, &mut fx);
        assert_eq!(p.stage(), Stage::Welcome);
        // imprimir depressa: direto ao pronto, sem Volta
        let p = machine(doc(SheetKind::Pages, 5, true), true, "true", dir.path(), &a);
        assert_eq!(p.stage(), Stage::Ready);
        assert!(!p.balloon().buttons.contains(&Button::Back));
    }

    #[test]
    fn machine_without_printer_says_it_cannot_find_it() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut p = machine(
            doc(SheetKind::Banner, 3, false),
            true,
            "/nao/existe/lp",
            dir.path(),
            &a,
        );
        let mut fx = Vec::new();
        press(&mut p, Button::Next, &a, &mut fx);
        let fx = settle(&mut p, &a);
        assert!(fx.contains(&wsnd(SND_ALERT)));
        assert_eq!(p.stage(), Stage::Alert);
        let text = p.alert().unwrap().plain_text();
        assert!(text.contains("achar a sua impressora"), "{text}");
        // as três folhas da faixa ficaram gravadas
        assert_eq!(p.files().len(), 3);
        let mut f = Frame::new(crate::SCREEN_W, crate::SCREEN_H);
        p.draw(&mut f, &a);
    }

    #[test]
    fn previews_by_kind() {
        let spec = &TEST_MACHINE;
        let opts = SheetOptions::default();
        let s = document_preview(spec, &doc(SheetKind::Pages, 1, false), &opts);
        // folha em pé dentro da moldura cinza (como a da Biblioteca)
        assert_eq!(s.frame, Some((66, 62, 210, 270)));
        let sh = s.sheet.unwrap();
        assert!(sh.2 <= 190 && sh.3 <= 254 && sh.3 > sh.2, "{sh:?}");
        let mut wide = doc(SheetKind::Pages, 1, false);
        wide.geom = PageGeom::new(792.0, 595.0, 72.0);
        wide.pages = vec![Rgb8::new(792, 595)];
        let s = document_preview(spec, &wide, &opts);
        let sh = s.sheet.unwrap();
        assert!(sh.2 > sh.3, "{sh:?}");
        // faixa: tira na coluna cinza de 70 px, sem folha branca
        let mut banner = doc(SheetKind::Banner, 3, false);
        banner.geom = PageGeom::new(577.0, 595.0, 72.0);
        banner.pages = vec![Rgb8::new(577, 595); 3];
        let s = document_preview(spec, &banner, &opts);
        assert_eq!(s.frame, Some((66, 62, 70, 270)));
        assert!(s.sheet.is_none());
        assert_eq!(s.image.w, 54);
        assert!(s.image.h < 30);
    }

    #[test]
    fn fallback_records_have_radios_and_fields() {
        let r = fallback_record("Pergunta\n(1) Sim\n(0) Não\nDe: []\nAté: []");
        let b = TextBalloon::from_record(&r, 0, 0, "", vec![]);
        assert_eq!(radio_ids(&b), [1, 0]);
        assert_eq!(b.fields.len(), 2);
        assert_eq!(radio_cell(&b, false), 1);
        assert!(radio_value(&b, 0));
        assert_eq!(parse_range(" 2", "5 "), Some((2, 5)));
        assert_eq!(parse_range("x", "5"), None);
    }

    #[test]
    fn fit_darkest_keeps_thin_lines() {
        let mut s = Rgb8::new(100, 100);
        for x in 0..100 {
            s.px[(51 * 100 + x) as usize] = BLACK;
        }
        let f = fit_darkest(&s, 10, 10);
        assert_eq!((f.w, f.h), (10, 10));
        assert!((0..10).any(|y| f.px[(y * 10) as usize] == BLACK));
    }
}
