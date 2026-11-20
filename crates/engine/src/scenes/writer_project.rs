//! Oficina de Projetos do Creative Writer em andamento (tarefas P6, P7,
//! P12, P17–P20 de `cw/report/projects.md`; contrato em
//! `docs/telas/creative-writer-projetos.md`): o editor do Escritório
//! ([`Writer`]) em modo projeto, com o Max (`7e13`/`7e9f`) no lugar da
//! porta Saída e o assistente do jornal, do cartão ou da faixa
//! ([`Assistant`]) guiando balão a balão.
//!
//! A cena:
//!
//! - manda ao assistente os cliques nos balões e no Max antes de tudo; o
//!   resto vai ao editor;
//! - traduz as ações do editor em [`Event`]s: ferramenta e botão de faixa
//!   clicados pelo lugar do clique ([`HITS`]), clique na página, e o que as
//!   ferramentas mudaram no documento (colunas, Alto/Largo, etiquetas,
//!   borda, palavra com forma, adesivo, 5ª folha do cartão) comparando o
//!   antes e o depois ([`project::events`]); assim funciona com qualquer
//!   interface das ferramentas;
//! - executa os comandos: pasta de amostras do Max filtrada por `WIZA`
//!   (P19, o fichário da pasta com miniatura e a seta do Título), páginas da
//!   faixa com a palavra esticada (P12), nome na barra de baixo e gravação
//!   do `.MAX` com `WIZA`/`FLOR 3`/`ADD` e a borda copiada (P17/P18),
//!   jogar fora, projeto novo, volta ao andar;
//! - mantém o [`Context`] em dia (documento vazio, Palavra ativa, faixa
//!   aberta, páginas) e aplica as restrições do projeto com o aviso do Rex
//!   (`0805[17]`, `[32]`; P20).
//!
//! O botão do McZee da faixa do Imprimir abre a máquina de impressão do
//! Escritório (`scenes::writer::print`, com o projeto gravado no
//! documento); quando ela fecha, o assistente recebe `Printed`. As visões
//! (`SetView`) são as do Visor de páginas (`Editor::set_view`).

use super::portfolio::{Action as BinderAction, Binder};
use super::writer::tools::{SWITCH_RECT, inside};
use super::writer::view::Arrange;
use super::writer::{PAGE, ToolId, Writer};
use super::writer_portfolio::{DocItem, WRITER_BINDER, list_documents, project_area};
use crate::Frame;
use crate::assets::{Assets, Program, SoundRef};
use crate::scene::{CwScene, Effect, Input, Key, MouseButton, Scene, SceneId, click_time};
use crate::scenes::room::Floor;
use crate::text_balloon::TextBalloon;
use crate::ui::Rect;
use crate::writer::assistant::{
    self as max, Assistant, Button, Command, Context, Event, Kind, Out, Tool,
};
use crate::writer::project::{self, Snapshot};
use std::path::PathBuf;

/// A porta Saída inteira (onde o Max fica): clicar nela é clicar no Max.
pub const DOOR_RECT: Rect = (581, 16, 55, 88);
/// Voltar ao andar dos Projetos (`projects.md` §4).
pub const SND_LEAVE: [u16; 2] = [0x02d7, 0x02c0];
/// Aviso do Rex.
const SND_REX: u16 = 0x02fe;
/// Cinza do pontilhado por baixo do cavalete e da máquina de impressão.
const DIM: [u8; 3] = [100, 100, 100];

/// Ferramenta do assistente com o mesmo nome da do editor.
pub fn tool_of(id: ToolId) -> Tool {
    match id {
        ToolId::Word => Tool::Word,
        ToolId::Stickers => Tool::Stickers,
        ToolId::Sounds => Tool::Sounds,
        ToolId::Page => Tool::Page,
        ToolId::Speller => Tool::Speller,
        ToolId::Egg => Tool::Egg,
        ToolId::Removers => Tool::Removers,
        ToolId::Transformers => Tool::Transformers,
        ToolId::Viewer => Tool::Viewer,
        ToolId::Print => Tool::Print,
    }
}

/// Um botão de faixa que os roteiros mandam clicar: a ferramenta, a página
/// da faixa (`Some(true)` = a de depois da alavanca; `None` = qualquer) e
/// o retângulo, medidos nas faixas do Escritório
/// (`office-ferramentas.md`, os `StripButton` de cada ferramenta).
pub struct Hit {
    pub tool: ToolId,
    pub more: Option<bool>,
    pub rect: Rect,
    pub button: Button,
}

const fn hit(tool: ToolId, more: Option<bool>, rect: Rect, button: Button) -> Hit {
    Hit {
        tool,
        more,
        rect,
        button,
    }
}

pub const HITS: &[Hit] = &[
    hit(ToolId::Word, Some(false), (443, 57, 46, 41), Button::Shape),
    hit(
        ToolId::Page,
        Some(false),
        (188, 61, 49, 34),
        Button::Columns,
    ),
    hit(
        ToolId::Page,
        Some(false),
        (267, 61, 49, 34),
        Button::Orientation,
    ),
    hit(
        ToolId::Page,
        Some(false),
        (425, 61, 49, 34),
        Button::PageNumber,
    ),
    // os três rolos BORDA, de (104,57) a (412,99)
    hit(
        ToolId::Page,
        Some(true),
        (104, 57, 308, 42),
        Button::BorderRoll,
    ),
    hit(
        ToolId::Viewer,
        Some(true),
        (55, 58, 66, 39),
        Button::ViewNormal,
    ),
    // as sete casas dos rolos de adesivos, de (30,57), 52 px cada
    hit(
        ToolId::Stickers,
        None,
        (30, 57, 364, 42),
        Button::StickerRoll,
    ),
    hit(ToolId::Stickers, None, (459, 57, 43, 41), Button::Crayon),
    hit(ToolId::Print, None, (337, 59, 70, 37), Button::PrintMcZee),
];

/// Ferramentas com alavanca de duas páginas de faixa.
const LEVERS: [ToolId; 2] = [ToolId::Page, ToolId::Viewer];

/// A pasta de amostras do Max aberta pelo assistente (P19).
struct SampleFolder {
    binder: Binder,
    items: Vec<DocItem>,
}

pub struct ProjectScene {
    w: Writer,
    max: Assistant,
    kind: Kind,
    started: bool,
    /// Pasta dos usuários e o apelido (amostras e pasta).
    users: Option<PathBuf>,
    user: String,
    animal: Option<usize>,
    folder: Option<SampleFolder>,
    /// Aviso do Rex das restrições do projeto.
    notice: Option<TextBalloon>,
    /// Esperando o nome (depois de `AskName`).
    naming: bool,
    /// Página da faixa em que está cada ferramenta com alavanca.
    more: [bool; 2],
    /// A máquina de impressão estava aberta no quadro anterior.
    printing: bool,
    /// O documento como o assistente o viu por último (depois dos próprios
    /// comandos): a diferença para ele são as ações das ferramentas.
    last: Snapshot,
    clock: f32,
}

impl ProjectScene {
    /// Projeto novo `kind` com o assistente, na pasta `dir` do usuário
    /// (`<users>/<user>`).
    pub fn new(
        kind: Kind,
        users: Option<PathBuf>,
        user: &str,
        animal: Option<usize>,
        author: &str,
        assets: &Assets,
    ) -> ProjectScene {
        let dir = users.as_ref().map(|u| u.join(user));
        let mut w = Writer::new_blank(dir, assets)
            .with_author(author)
            .with_user(user)
            .for_project(kind.doing());
        let (doc, extra) = project::blank(kind);
        w.editor_mut().new_from(doc, extra);
        let last = {
            let ed = w.editor();
            Snapshot::of(ed.doc(), ed.layout().pages.len())
        };
        ProjectScene {
            w,
            max: Assistant::new(kind, Context::default()),
            kind,
            started: false,
            users,
            user: user.to_string(),
            animal,
            folder: None,
            notice: None,
            naming: false,
            more: [false; 2],
            printing: false,
            last,
            clock: 0.0,
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn writer(&self) -> &Writer {
        &self.w
    }

    pub fn writer_mut(&mut self) -> &mut Writer {
        &mut self.w
    }

    pub fn assistant(&self) -> &Assistant {
        &self.max
    }

    /// A pasta de amostras está aberta (com os títulos dela).
    pub fn samples(&self) -> Option<Vec<&str>> {
        self.folder
            .as_ref()
            .map(|f| f.items.iter().map(|i| i.title.as_str()).collect())
    }

    pub fn notice(&self) -> Option<&TextBalloon> {
        self.notice.as_ref()
    }

    /// Uma tela do Escritório está por cima do editor (cavalete de palavras
    /// com forma, máquina de impressão, Caderno de Idéias, Central de
    /// Controle): os cliques são dela.
    fn overlay(&self) -> bool {
        let w = &self.w;
        w.shaped().is_open()
            || w.printing()
            || w.notebook().is_some()
            || w.control_center().is_some()
    }

    fn lever(&self, id: ToolId) -> Option<usize> {
        LEVERS.iter().position(|&t| t == id)
    }

    /// O contexto do assistente a partir do editor.
    fn sync(&mut self) {
        let ed = self.w.editor();
        let pages = ed.layout().pages.len();
        let tool = self.w.tool();
        self.max.ctx = Context {
            blank: project::is_blank(ed.doc()),
            word_active: tool == ToolId::Word,
            open_strip: Some(tool_of(tool)),
            pages: pages.min(usize::from(u8::MAX)) as u8,
            printing: self.w.printing(),
        };
    }

    fn snapshot(&self) -> Snapshot {
        let ed = self.w.editor();
        let tools = self.w.tools();
        Snapshot::of(ed.doc(), ed.layout().pages.len())
            .with_hand(tools.page.border_armed(), tools.stickers.held().is_some())
    }

    /// Manda um evento ao assistente e executa o que ele pedir.
    pub fn send(&mut self, ev: Event, fx: &mut Vec<Effect>, a: &Assets) {
        self.sync();
        let outs = self.max.on_event(ev);
        self.run(outs, fx, a);
    }

    fn run(&mut self, outs: Vec<Out>, fx: &mut Vec<Effect>, a: &Assets) {
        for o in outs {
            match o {
                Out::Entered(_) => {
                    fx.push(Effect::Log(format!("assistente: passo {}", self.max.at())));
                }
                Out::Sound(id) => fx.push(sound(id)),
                Out::Do(c) => {
                    self.command(c, fx, a);
                    self.last = self.snapshot();
                }
            }
        }
    }

    fn command(&mut self, c: Command, fx: &mut Vec<Effect>, a: &Assets) {
        match c {
            Command::OpenSamples => self.open_samples(a),
            Command::SetView(v) => {
                // as visões do Visor de páginas (página 2 da faixa)
                let ed = self.w.editor_mut();
                let mut view = ed.view;
                view.overview = false;
                view.arrange = match v {
                    max::View::Normal => Arrange::Normal,
                    max::View::TwoPages => Arrange::Two,
                    max::View::AllPages => Arrange::All,
                    max::View::SideBySide => Arrange::Side,
                };
                ed.set_view(view);
            }
            Command::SetPages(n) => {
                let ed = self.w.editor_mut();
                ed.record();
                project::stretch_banner(ed.doc_mut(), n);
                fx.push(Effect::Log(format!("faixa com {n} páginas")));
            }
            Command::AskName => {
                // o original grava a borda inteira no documento (P18)
                let libs: Vec<(&str, &[u8])> = project::BORDER_LIBRARIES
                    .iter()
                    .filter_map(|&n| Some((n, a.library_bytes(n)?)))
                    .collect();
                let ed = self.w.editor_mut();
                let doc = ed.doc().clone();
                project::copy_border(&doc, &mut ed.extra, &libs);
                project::mark(&mut ed.extra, self.kind);
                self.w.ask_name(fx);
                self.naming = true;
            }
            Command::Discard => self.w.editor_mut().discard(),
            Command::NewDocument => {
                let (doc, extra) = project::blank(self.kind);
                self.w.editor_mut().new_from(doc, extra);
            }
            Command::Leave => {
                fx.extend(SND_LEAVE.map(sound));
                fx.push(Effect::ChangeScene(SceneId::Cw(CwScene::Building(
                    Floor::Projects,
                ))));
            }
        }
    }

    /// `OpenSamples`: a pasta em `MAX › Jornais|Cartões|Faixas`, só com as
    /// amostras do projeto (`WIZA`).
    fn open_samples(&mut self, a: &Assets) {
        let area = project_area(self.kind);
        let items: Vec<DocItem> = list_documents(self.users.as_deref(), &self.user, a)
            .into_iter()
            .filter(|i| i.sample && i.group == area)
            .collect();
        let mut binder = Binder::new("C:\\".to_string()).with_spec(&WRITER_BINDER);
        binder.set_user(&self.user, self.animal, &items);
        // os balões são do assistente, não o `0bb8[53]` da pasta
        binder.open(false, a);
        self.folder = Some(SampleFolder { binder, items });
    }

    /// Já na pasta: a amostra abre como cópia (salvar grava na pasta do
    /// usuário) e o assistente segue.
    fn choose_sample(&mut self, k: usize, fx: &mut Vec<Effect>, a: &Assets) {
        let Some(f) = self.folder.take() else { return };
        let Some(it) = f.items.get(k) else { return };
        match self.w.editor_mut().open(&it.path) {
            Ok(()) => {
                self.last = self.snapshot();
                fx.push(Effect::Log(format!("amostra \"{}\" aberta", it.title)));
                self.send(Event::SampleChosen, fx, a);
            }
            Err(e) => fx.push(Effect::Log(format!("amostra: {e}"))),
        }
    }

    /// Clique na pasta de amostras.
    fn folder_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        let Some(f) = self.folder.as_mut() else {
            return;
        };
        match f.binder.click(x, y, self.clock, &f.items, fx, a) {
            Some(BinderAction::Open(k)) => self.choose_sample(k, fx, a),
            Some(BinderAction::Close) => {
                // fechar sem escolher: o passo anterior ("começar com uma
                // amostra?")
                self.folder = None;
                self.send(Event::Back, fx, a);
            }
            None => {}
        }
    }

    /// Eventos de um clique que vai ao editor: ferramenta, botão de faixa
    /// ou página. Atualiza a página da faixa das ferramentas com alavanca.
    fn classify(&mut self, x: i32, y: i32, a: &Assets) -> Vec<Event> {
        let w = &self.w;
        if self.overlay()
            || w.dialog().is_some()
            || w.renaming().is_some()
            || w.mczee().covers(a, x, y)
        {
            return Vec::new();
        }
        let tool = w.tool();
        if w.tools().get(tool).modal() {
            return Vec::new();
        }
        if let Some(id) = ToolId::at(x, y) {
            return vec![Event::ToolClicked(tool_of(id))];
        }
        // a Palavra diz em que página da faixa está; a Página e o Visor, a
        // cena acompanha pelas alavancas
        let word = (tool == ToolId::Word).then(|| w.tools().word.more());
        if inside(SWITCH_RECT, x, y) && (word.is_some() || self.lever(tool).is_some()) {
            if let Some(k) = self.lever(tool) {
                self.more[k] = !self.more[k];
            }
            return vec![Event::ButtonClicked(Button::Lever)];
        }
        let more = word.or_else(|| self.lever(tool).map(|k| self.more[k]));
        if let Some(h) = HITS.iter().find(|h| {
            h.tool == tool && (h.more.is_none() || h.more == more) && inside(h.rect, x, y)
        }) {
            return vec![Event::ButtonClicked(h.button)];
        }
        if inside(PAGE, x, y) {
            return vec![Event::PageClicked];
        }
        Vec::new()
    }

    /// Clique: aviso do Rex, balões e Max, pasta; senão o editor (e os
    /// eventos que ele representa).
    fn click(
        &mut self,
        x: i32,
        y: i32,
        pending: &mut Vec<Event>,
        fx: &mut Vec<Effect>,
        a: &Assets,
    ) -> bool {
        if self.notice.take().is_some() {
            return true;
        }
        // a máquina de impressão cobre a tela: os balões do Max ficam
        // escondidos e os cliques são dela
        let ev = (!self.w.printing())
            .then(|| self.max.click(a, x, y))
            .flatten()
            .or_else(|| inside(DOOR_RECT, x, y).then_some(Event::MaxClicked))
            // com o cavalete ou a máquina por cima, o Max fica apagado
            .filter(|&e| !(e == Event::MaxClicked && self.overlay()));
        if let Some(ev) = ev {
            self.send(ev, fx, a);
            return true;
        }
        if self.folder.is_some() {
            self.folder_click(x, y, fx, a);
            return true;
        }
        pending.extend(self.classify(x, y, a));
        false
    }

    /// Teclas do campo de páginas da faixa.
    fn field_key(&mut self, i: &Input, fx: &mut Vec<Effect>, a: &Assets) -> bool {
        if self.max.field().is_none() {
            return false;
        }
        match i {
            Input::Text(t) => {
                for c in t.chars() {
                    self.send(Event::Typed(c), fx, a);
                }
            }
            Input::Key {
                key: Key::Backspace | Key::Delete,
                ..
            } => self.send(Event::Backspace, fx, a),
            Input::Key {
                key: Key::Enter, ..
            } => self.send(Event::Next, fx, a),
            _ => return false,
        }
        true
    }

    /// Restrições do projeto: desfaz e, se for o caso, mostra o aviso do
    /// Rex.
    fn restrict(&mut self, before: &Snapshot, fx: &mut Vec<Effect>, a: &Assets) {
        let mut doc = self.w.editor().doc().clone();
        let Some(r) = project::restrict(self.kind, before, &mut doc) else {
            return;
        };
        *self.w.editor_mut().doc_mut() = doc;
        let project::Refused::Alert(block, rec) = r else {
            return;
        };
        fx.push(sound(SND_REX));
        let records = a.dialogs(Program::Writer, block);
        if let Some(r) = records.get(rec) {
            let mut b = TextBalloon::from_record(r, 0, 0, "", Vec::new()).as_alert();
            let (w, h) = (b.box_width(), b.height(a));
            (b.x, b.y) = ((640 - w) / 2, ((480 - h) / 2).max(110));
            self.notice = Some(b);
        }
    }
}

fn sound(id: u16) -> Effect {
    Effect::PlaySound(SoundRef::Snd(Program::Writer, id))
}

impl Scene for ProjectScene {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.clock += click_time(dt);
        if !std::mem::replace(&mut self.started, true) {
            self.sync();
            let outs = self.max.begin();
            self.run(outs, &mut fx, assets);
        }
        let mut pass = Vec::new();
        let mut pending = Vec::new();
        for i in input {
            match *i {
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    if !self.click(x, y, &mut pending, &mut fx, assets) {
                        pass.push(i.clone());
                    }
                }
                Input::Key { .. } | Input::Text(_) => {
                    if self.field_key(i, &mut fx, assets) {
                        continue;
                    }
                    if let Some(f) = self.folder.as_mut() {
                        f.binder.key(i, &f.items, &mut fx, assets);
                    } else {
                        pass.push(i.clone());
                    }
                }
                _ => pass.push(i.clone()),
            }
        }
        fx.extend(self.w.update(&pass, dt, assets));
        let before = self.last.clone();
        self.restrict(&before, &mut fx, assets);
        let after = self.snapshot();
        if self.kind == Kind::Banner && after.shaped > before.shaped {
            // a palavra da faixa vai de margem a margem através das folhas
            let ed = self.w.editor_mut();
            let n = ed
                .doc()
                .min_pages
                .clamp(1, u16::from(max::BANNER_MAX_PAGES)) as u8;
            project::stretch_banner(ed.doc_mut(), n);
        }
        // o que mudou no documento vem antes dos cliques: a borda aplicada
        // com um clique na página é "escolhida" e depois "aplicada"
        let clicks = std::mem::replace(&mut pending, project::events(self.kind, &before, &after));
        pending.extend(clicks);
        self.last = self.snapshot();
        if self.naming && self.w.renaming().is_none() && self.w.dialog().is_none() {
            self.naming = false;
            let ed = self.w.editor();
            if ed.file().is_some() && !ed.changed() {
                pending.push(Event::Saved);
            }
        }
        // a máquina de impressão fechou: "Tudo pronto!"
        let printing = self.w.printing();
        if std::mem::replace(&mut self.printing, printing) && !printing {
            pending.push(Event::Printed);
        }
        for ev in pending {
            self.send(ev, &mut fx, assets);
        }
        self.max.tick(dt);
        fx
    }

    fn draw(&self, f: &mut Frame, a: &Assets) {
        self.w.draw(f, a);
        let (x, y, w, h) = DOOR_RECT;
        f.rect(x, y, w, h, [0, 0, 0]);
        self.max.draw_max(f, a);
        if self.overlay() {
            crate::ui::checker(f, DOOR_RECT, DIM);
        }
        if let Some(folder) = &self.folder {
            folder.binder.draw(f, a, &folder.items, &[]);
        }
        if let Some(n) = &self.notice {
            n.draw(f, a);
        }
        if !self.w.printing() {
            self.max.draw(f, a);
        }
    }

    fn leave(&mut self) -> Vec<Effect> {
        self.w.leave()
    }
}

#[cfg(test)]
mod tests;
