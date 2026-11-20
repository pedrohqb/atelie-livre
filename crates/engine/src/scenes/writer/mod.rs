//! Escritório do Creative Writer: a página com o editor de texto, a barra de
//! ferramentas, a faixa de opções de cada ferramenta, a barra de baixo e o
//! McZee (`WRITER.PIC`, `WRITER.SND`, balões `WHAT` do `WRITER.EXE`).
//!
//! Organização (contrato em `docs/telas/creative-writer.md`):
//! - [`editor`]: documento, página, digitação, desfazer e gravação;
//! - [`tools`]: as 10 ferramentas da barra e o trait [`ToolStrip`];
//! - um arquivo por ferramenta (`word_strip`, `stickers`, `sounds`,
//!   `page_strip`, `speller`, `egg`, `removers`, `transformers`, `viewer`,
//!   `print`), mais a porta (`exit`);
//! - [`chrome`]: moldura, barra de ferramentas e barra de baixo;
//! - [`mczee`]: balões automáticos e menu do McZee (a Bolsa Mágica do
//!   editor); [`notebook`]: o Caderno de Idéias aberto por ela;
//!   [`control_center`]: a Central de Controle (Ctrl+Shift+A);
//!   [`cursors`]: cursores.

mod chrome;
pub mod control_center;
pub mod cursors;
pub mod editor;
mod egg;
mod exit;
pub mod mczee;
pub mod notebook;
mod page_strip;
pub mod print;
mod removers;
pub mod shaped;
mod sounds;
mod speller;
mod stickers;
pub mod tools;
mod transformers;
mod viewer;
mod word_strip;

use crate::Frame;
use crate::assets::{Assets, CursorRef, Program};
use crate::scene::{
    CwScene, DEFAULT_CURSOR, Effect, Input, Key, KidCommon, MouseButton, Scene, SceneId,
};
use crate::scenes::paint::exit_dialog::{Choice, ExitDialog, Reason};
use crate::scenes::paint::rename::{After as RenameAfter, Rename};
use crate::writer::doc::{Document, Item};
use crate::writer::ideas::machine::Piece;
use crate::writer::ideas::{session, story};
use crate::writer::object::ObjectKind;
use crate::writer::prefs;
use crate::writer::selection::Clip;
use control_center::{Action as CenterAction, ControlCenter};
use cursors::Pointer;
pub use editor::{Editor, LIBRARIES, ORIGIN, PAGE, PAGE_GAP, PagePoint};
use mczee::{McZee, Request};
use notebook::NotebookOverlay;
use std::path::{Path, PathBuf};
use tools::HelpRef;
pub use tools::{Cx, ToolId, ToolStrip};
use tools::{SND_CHOOSE, SND_TOOL_AGAIN, WHAT_GENERAL, WHAT_REMOVERS, inside, snd};
pub use word_strip::List;

/// Uma instância de cada ferramenta (na ordem da barra).
#[derive(Default)]
pub struct Tools {
    pub word: word_strip::WordStrip,
    pub stickers: stickers::StickersStrip,
    pub sounds: sounds::SoundsStrip,
    pub page: page_strip::PageStrip,
    pub speller: speller::SpellerStrip,
    pub egg: egg::EggStrip,
    pub removers: removers::RemoversStrip,
    pub transformers: transformers::TransformersStrip,
    pub viewer: viewer::ViewerStrip,
    pub print: print::PrintStrip,
}

impl Tools {
    pub fn get(&self, id: ToolId) -> &dyn ToolStrip {
        match id {
            ToolId::Word => &self.word,
            ToolId::Stickers => &self.stickers,
            ToolId::Sounds => &self.sounds,
            ToolId::Page => &self.page,
            ToolId::Speller => &self.speller,
            ToolId::Egg => &self.egg,
            ToolId::Removers => &self.removers,
            ToolId::Transformers => &self.transformers,
            ToolId::Viewer => &self.viewer,
            ToolId::Print => &self.print,
        }
    }

    pub fn get_mut(&mut self, id: ToolId) -> &mut dyn ToolStrip {
        match id {
            ToolId::Word => &mut self.word,
            ToolId::Stickers => &mut self.stickers,
            ToolId::Sounds => &mut self.sounds,
            ToolId::Page => &mut self.page,
            ToolId::Speller => &mut self.speller,
            ToolId::Egg => &mut self.egg,
            ToolId::Removers => &mut self.removers,
            ToolId::Transformers => &mut self.transformers,
            ToolId::Viewer => &mut self.viewer,
            ToolId::Print => &mut self.print,
        }
    }
}

/// Setas ▲▼◀▶ da barra de baixo: rolam a vista (sons `02bc`..`02bf`).
const SND_ARROWS: [u16; 4] = [0x02bc, 0x02bd, 0x02be, 0x02bf];
/// Quanto cada clique numa seta rola (provisório: não medido).
pub const ARROW_STEP: i32 = PAGE.3 / 2;
/// "Andando por aí e olhando as páginas", na primeira seta usada.
const WHAT_PAGES: usize = 11;
/// "Salvar": o título vira campo de nome (`0faa[14]`, som `0232`).
const WHAT_SAVE: usize = 14;
const SND_RENAME: u16 = 0x0232;
/// Pasta abrindo (a maleta da Bolsa Mágica, Ctrl+O).
const SND_PORTFOLIO: u16 = 0x02f1;
/// Porta aberta: `02d6` + `02fe` e o "Epa!" (`0203[0]`).
const SND_EPA: [u16; 2] = [0x02d6, 0x02fe];
/// Ctrl+X/C/V: sons e balões dos Re-movedores (`0fa3[8]`, `[12]`, `[14]`).
const SND_CUT: u16 = 0x022f;
const SND_COPY: u16 = 0x0288;
const SND_PASTE: u16 = 0x028d;
/// Ctrl+Z: desfazer `020e`, refazer `0217`.
const SND_UNDO: u16 = 0x020e;
const SND_REDO: u16 = 0x0217;
/// "Epa!" do Escritório: `0203[0]` (salvar, jogar fora, continuar) e
/// `0202[3]` (documento com nome: as duas versões, só a nova, ainda não).
const EPA: (u16, usize) = (0x0203, 0);
const VERSIONS: (u16, usize) = (0x0202, 3);
const EPA_FALLBACK: (&str, [&str; 3]) = (
    "O que você quer fazer agora?",
    [
        "Salvar este trabalho",
        "Jogar fora — reciclar",
        "Ops!  Ainda não terminei!",
    ],
);
const VERSIONS_FALLBACK: (&str, [&str; 3]) = (
    "O que você quer salvar? Vou salvar “@” agora. Você quer:",
    [
        "Salvar a versão antiga e a nova?",
        "Salvar só a nova?",
        "Ops!  Ainda não quero salvar!",
    ],
);
/// Título do documento na barra de baixo (campo de nome ao salvar).
pub const TITLE_LABEL: (i32, i32, i32, i32) = (195, 461, 250, 14);

/// Acrescenta texto à lista de pedaços (juntando com o texto anterior).
fn push_text(pieces: &mut Vec<Piece>, t: &str) {
    match pieces.last_mut() {
        Some(Piece::Text(s)) => s.push_str(t),
        _ => pieces.push(Piece::Text(t.to_string())),
    }
}

/// O que fazer depois de salvar ou jogar fora.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum After {
    /// Continuar no Escritório (SALVAR, Ctrl+S).
    Stay,
    /// Ir à sala Escritório (porta Saída).
    Leave,
    /// Começar um documento em branco (NOVO).
    New,
    /// Sair do programa (Ctrl+Q).
    Quit,
    /// Abrir a pasta (maleta da Bolsa Mágica, Ctrl+O): o documento escolhido
    /// substitui este.
    Open,
}

/// Diálogo "Epa!" aberto.
pub enum Dialog {
    /// `0203[0]`: salvar, jogar fora ou continuar.
    Epa(ExitDialog, After),
    /// `0202[3]`: as duas versões, só a nova, ou ainda não; com o título.
    Versions(ExitDialog, After, String),
}

impl Dialog {
    pub fn exit(&self) -> &ExitDialog {
        match self {
            Dialog::Epa(d, _) | Dialog::Versions(d, _, _) => d,
        }
    }
}

pub struct Writer {
    common: KidCommon,
    ed: Editor,
    tools: Tools,
    tool: ToolId,
    mczee: McZee,
    pointer: Pointer,
    mouse: (i32, i32),
    ctrl: bool,
    shift: bool,
    /// Já entrou (som e balão de boas-vindas).
    started: bool,
    /// Alguma seta da barra de baixo já foi usada (balão só na primeira).
    arrows_used: bool,
    /// Nome completo do usuário (autor gravado no `.MAX`).
    author: String,
    /// Porta aberta (o "Epa!" da Saída).
    door_open: bool,
    dialog: Option<Dialog>,
    /// Campo de nome na barra de baixo e o que fazer depois.
    rename: Option<(Rename, After)>,
    /// Apelido do usuário (sessão da Oficina de Idéias: área de transferência).
    user: String,
    /// O que o editor pôs na área de transferência da sessão por último.
    last_copy: Option<Vec<Piece>>,
    /// Andar na barra de baixo.
    floor: &'static str,
    /// Para onde a porta SAÍDA leva (a sala Escritório; a Oficina de Idéias
    /// nas histórias começadas lá).
    exit_to: SceneId,
    /// Balão de boas-vindas.
    welcome: HelpRef,
    /// Pasta `MSKIDS` dos dados originais (os exemplos do Max e o
    /// `~MAXUSER.INF`, `writer::prefs`).
    data: PathBuf,
    /// Caderno de Idéias aberto (Bolsa Mágica).
    notebook: Option<NotebookOverlay>,
    /// Central de Controle aberta (Ctrl+Shift+A).
    center: Option<ControlCenter>,
    /// Cavalete de palavras com forma (FORMA).
    shaped: shaped::ShapedHost,
}

/// Pasta dos usuários e apelido a partir da pasta do usuário
/// (`<usuários>/<apelido>`).
fn split_user_dir(dir: &Path) -> Option<(&Path, String)> {
    let user = dir.file_name()?.to_string_lossy().to_string();
    Some((dir.parent()?, user))
}

/// O documento que o Escritório abre ao entrar: o último do usuário
/// (`FILR 0` do `~MAXUSER.INF`, o do quadro da sala; para um usuário novo,
/// o exemplo do Max). Um usuário sem `~MAXUSER.INF` que já tem documentos
/// na pasta (gravados antes de o port guardar as preferências) abre o mais
/// recente deles.
pub fn last_document(dir: &Path, data: &Path) -> Option<PathBuf> {
    let (users, user) = split_user_dir(dir)?;
    if !prefs::path(users, &user).exists()
        && let Some(f) = crate::writer::store::latest(dir)
    {
        return Some(f);
    }
    prefs::last_document(Some(users), &user, data).map(|(_, f)| f)
}

impl Writer {
    /// O Escritório com o último documento da pasta do usuário `dir` (ou um
    /// em branco).
    pub fn new(dir: Option<PathBuf>, assets: &Assets) -> Writer {
        let mut ed = Editor::blank(dir.clone(), assets);
        let opened = dir
            .as_deref()
            .and_then(|d| last_document(d, assets.data_dir()))
            .is_some_and(|f| ed.open(&f).is_ok());
        if !opened {
            ed = Editor::new(dir, assets);
        }
        Writer::with_editor(ed, assets)
    }

    /// O Escritório com um documento em branco (prancheta da sala, NOVO,
    /// "Ir direto para o Escritório").
    pub fn new_blank(dir: Option<PathBuf>, assets: &Assets) -> Writer {
        Writer::with_editor(Editor::blank(dir, assets), assets)
    }

    /// O Escritório com o documento `path`.
    pub fn open(dir: Option<PathBuf>, path: &Path, assets: &Assets) -> Result<Writer, String> {
        let mut ed = Editor::blank(dir, assets);
        ed.open(path)?;
        Ok(Writer::with_editor(ed, assets))
    }

    /// O Escritório com a história começada na Oficina de Idéias
    /// (`CwScene::IdeaStory`): pega o conteúdo deixado na sessão do usuário
    /// (`ideas::session::take_story`); sem ele, um documento em branco.
    pub fn idea_story(dir: Option<PathBuf>, user: &str, assets: &Assets) -> Writer {
        let mut ed = Editor::blank(dir, assets);
        let story = session::take_story(user);
        let mut w = match story {
            Some(s) => {
                let (help, back) = (s.source.help(), s.source.back());
                ed.open_story(s);
                let mut w = Writer::with_editor(ed, assets);
                w.floor = story::FLOOR_LABEL;
                w.exit_to = back;
                w.welcome = help;
                w
            }
            None => Writer::with_editor(ed, assets),
        };
        w.user = user.to_string();
        w
    }

    /// Apelido do usuário (a área de transferência da Oficina de Idéias).
    pub fn with_user(mut self, user: &str) -> Writer {
        self.user = user.to_string();
        self
    }

    fn with_editor(ed: Editor, assets: &Assets) -> Writer {
        Writer {
            common: KidCommon::new(None),
            ed,
            tools: Tools::default(),
            tool: ToolId::Word,
            mczee: McZee::default(),
            pointer: Pointer::default(),
            mouse: (0, 0),
            ctrl: false,
            shift: false,
            started: false,
            arrows_used: false,
            author: String::new(),
            door_open: false,
            dialog: None,
            rename: None,
            user: String::new(),
            last_copy: None,
            floor: chrome::FLOOR_OFFICE,
            exit_to: SceneId::Studio.in_program(Program::Writer),
            welcome: mczee::WELCOME,
            data: assets.data_dir().to_path_buf(),
            notebook: None,
            center: None,
            shaped: shaped::ShapedHost::default(),
        }
    }

    /// Torna `file` o último documento do usuário (`writer::prefs`); com
    /// `replaced`, os lugares da parede que apontavam para o arquivo antigo
    /// passam para o novo. Devolve o erro, se houver, para o registro.
    fn remember(&self, file: &Path, title: &str, replaced: Option<&Path>) -> Option<String> {
        let (users, user) = split_user_dir(self.ed.dir()?)?;
        prefs::remember_saved(users, &user, &self.data, file, title, replaced)
            .err()
            .map(|e| format!("preferências: {e}"))
    }

    pub fn notebook(&self) -> Option<&NotebookOverlay> {
        self.notebook.as_ref()
    }

    pub fn control_center(&self) -> Option<&ControlCenter> {
        self.center.as_ref()
    }

    /// Andar da barra de baixo.
    pub fn floor(&self) -> &str {
        self.floor
    }

    /// Para onde a porta SAÍDA leva.
    pub fn exit_to(&self) -> SceneId {
        self.exit_to
    }

    /// Copiado no editor: vai também para a área de transferência da sessão
    /// (a do Caderno de Idéias), sem estilos.
    fn share_clip(&mut self) {
        let Some(Clip::Text(f)) = &self.ed.clipboard.clip else {
            return;
        };
        let mut pieces: Vec<Piece> = Vec::new();
        for (k, p) in f.paras.iter().enumerate() {
            if k > 0 {
                push_text(&mut pieces, "\n");
            }
            for it in &p.items {
                match *it {
                    Item::Char(c, _) => push_text(&mut pieces, &c.to_string()),
                    Item::Rebus(c) => pieces.push(Piece::Rebus(c)),
                }
            }
        }
        session::set_clip(&self.user, session::Clip::Text(pieces.clone()));
        self.last_copy = Some(pieces);
    }

    /// Colar: a área de transferência da sessão, se o que está nela veio do
    /// Caderno de Idéias (texto com figuras-rebus ou uma figura); senão, a do
    /// editor (com os estilos).
    fn paste(&mut self) -> bool {
        match session::clip(&self.user) {
            Some(session::Clip::Text(p)) if Some(&p) != self.last_copy.as_ref() => {
                let props = self.ed.typing_props();
                self.ed.edit(|d, s| {
                    let mut at = d.delete(s.range());
                    for piece in &p {
                        match piece {
                            Piece::Text(t) => {
                                for (k, line) in t.split('\n').enumerate() {
                                    if k > 0 {
                                        at = d.split(at);
                                    }
                                    at = d.insert_text(at, line, props);
                                }
                            }
                            Piece::Rebus(c) if *c > 0 => at = d.insert_item(at, Item::Rebus(*c)),
                            Piece::Rebus(_) => {}
                        }
                    }
                    *s = crate::writer::selection::Selection::at(at);
                });
                true
            }
            Some(session::Clip::Picture(img, title)) => {
                let c = self.ed.layout().caret(self.ed.sel.caret);
                self.ed.add_picture(&img, &title, c.page, c.x, c.top);
                true
            }
            _ => self.ed.paste(),
        }
    }

    /// Nome do autor gravado nos documentos (o do cadastro).
    pub fn with_author(mut self, author: &str) -> Writer {
        self.author = author.to_string();
        self
    }

    pub fn doc(&self) -> &Document {
        self.ed.doc()
    }

    pub fn editor(&self) -> &Editor {
        &self.ed
    }

    pub fn editor_mut(&mut self) -> &mut Editor {
        &mut self.ed
    }

    pub fn tool(&self) -> ToolId {
        self.tool
    }

    /// Abre a máquina de impressão do McZee para o documento (W18; os
    /// assistentes da Oficina de Projetos a chamam no fim): escolhe a
    /// ferramenta Imprimir e abre a máquina, com as escolhas ou, com
    /// `quick`, direto ao "pronto". `kind` = o projeto (jornal, cartão,
    /// faixa); `None` = o gravado no documento. Ver [`print::print_document`].
    pub fn print_document(
        &mut self,
        kind: Option<crate::writer::assistant::Kind>,
        quick: bool,
        assets: &Assets,
    ) -> Vec<Effect> {
        let mut fx = Vec::new();
        if self.tool != ToolId::Print {
            self.select(ToolId::Print, &mut fx, assets);
        }
        let (strip, ed) = (&mut self.tools.print, &self.ed);
        let mut cx = Cx::new(&mut fx, assets);
        strip.open(ed, &mut cx, quick, kind);
        self.mczee.show((0, usize::MAX), false, assets);
        fx
    }

    /// A máquina de impressão está aberta.
    pub fn printing(&self) -> bool {
        self.tools.print.is_open()
    }

    pub fn tools(&self) -> &Tools {
        &self.tools
    }

    pub fn tools_mut(&mut self) -> &mut Tools {
        &mut self.tools
    }

    /// Folha no meio da vista.
    pub fn page(&self) -> usize {
        self.ed.page()
    }

    pub fn mczee(&self) -> &McZee {
        &self.mczee
    }

    pub fn dialog(&self) -> Option<&Dialog> {
        self.dialog.as_ref()
    }

    pub fn renaming(&self) -> Option<&Rename> {
        self.rename.as_ref().map(|(r, _)| r)
    }

    pub fn door_open(&self) -> bool {
        self.door_open
    }

    /// Título na barra de baixo.
    pub fn title(&self) -> &str {
        self.ed.title().unwrap_or(chrome::DEFAULT_TITLE)
    }

    /// Cursor do mouse atual.
    pub fn cursor(&self) -> CursorRef {
        self.pointer.current()
    }

    /// Roda um gancho da ferramenta `id` e abre o balão que ela pedir.
    fn with_tool<R>(
        &mut self,
        id: ToolId,
        fx: &mut Vec<Effect>,
        a: &Assets,
        f: impl FnOnce(&mut dyn ToolStrip, &mut Editor, &mut Cx) -> R,
    ) -> R {
        let mut cx = Cx::new(fx, a);
        cx.ctrl = self.ctrl;
        cx.shift = self.shift;
        let r = f(self.tools.get_mut(id), &mut self.ed, &mut cx);
        let shaped = cx.take_shaped();
        let clip = cx.take_clip();
        if let Some((h, forced)) = cx.take_help() {
            self.mczee.show(h, forced, a);
        }
        if let Some(editing) = shaped {
            self.open_shaped(editing, fx, a);
        }
        if let Some(c) = clip {
            self.clip_request(c, fx, a);
        }
        r
    }

    /// Cavalete de palavras com forma aberto.
    pub fn shaped(&self) -> &shaped::ShapedHost {
        &self.shaped
    }

    /// Abre o cavalete: palavra nova com o nome do usuário, ou o objeto `k`.
    fn open_shaped(&mut self, editing: Option<usize>, fx: &mut Vec<Effect>, a: &Assets) {
        let name = if self.user.is_empty() {
            self.author.split_whitespace().next().unwrap_or("")
        } else {
            &self.user
        }
        .to_string();
        if let Some(h) = self.shaped.open(&self.ed, editing, &name, fx, a) {
            self.mczee.show(h, false, a);
        }
    }

    /// Entrada com o cavalete aberto (os balões do McZee vêm antes).
    fn shaped_input(&mut self, i: &Input, fx: &mut Vec<Effect>, a: &Assets) {
        if let Input::Click { x, y, .. } = *i
            && self.mczee.front_click(x, y, fx, a).is_some()
        {
            return;
        }
        if let Some(h) = self.shaped.input(i, &mut self.ed, fx, a) {
            self.mczee.show(h, false, a);
        }
    }

    /// Escolhe a ferramenta `id` na barra: som, aba, balão do McZee.
    fn select(&mut self, id: ToolId, fx: &mut Vec<Effect>, a: &Assets) {
        let again = id == self.tool;
        let def = id.def();
        fx.push(snd(if again { SND_TOOL_AGAIN } else { def.sound }));
        if !again {
            let old = self.tool;
            self.with_tool(old, fx, a, |t, ed, cx| t.deactivate(ed, cx));
            self.tool = id;
        }
        self.ed.undo.close();
        self.mczee.set_wand(false);
        self.mczee.show(def.help, false, a);
        self.with_tool(id, fx, a, |t, ed, cx| t.activate(ed, cx, again));
    }

    /// Pedido do menu do McZee.
    fn request(&mut self, r: Request, fx: &mut Vec<Effect>, a: &Assets) {
        match r {
            Request::New => self.ask_or(After::New, fx, a),
            Request::Save => self.start_rename(After::Stay, fx, a),
            Request::Open => self.ask_or(After::Open, fx, a),
            Request::Notebook => self.open_notebook(fx, a),
        }
    }

    // ---------------------------------------------------------------
    // Salvar, jogar fora, sair (W21)

    /// Com mudanças, pergunta no "Epa!" o que fazer antes de `after`; sem
    /// mudanças, faz já.
    fn ask_or(&mut self, after: After, fx: &mut Vec<Effect>, a: &Assets) {
        if self.ed.changed() {
            self.open_epa(after, fx, a);
        } else {
            self.finish(after, fx);
        }
    }

    fn open_epa(&mut self, after: After, fx: &mut Vec<Effect>, a: &Assets) {
        fx.extend(SND_EPA.map(snd));
        self.door_open = after == After::Leave;
        self.mczee.close();
        let reason = if matches!(after, After::New | After::Open) {
            Reason::New
        } else {
            Reason::Exit
        };
        let d = ExitDialog::writer(reason, EPA, "", EPA_FALLBACK, a);
        self.dialog = Some(Dialog::Epa(d, after));
    }

    fn open_versions(&mut self, title: String, after: After, a: &Assets) {
        let d = ExitDialog::writer(Reason::Exit, VERSIONS, &title, VERSIONS_FALLBACK, a);
        self.dialog = Some(Dialog::Versions(d, after, title));
    }

    /// Clique com um "Epa!" aberto: só as bolinhas respondem.
    fn dialog_click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        let Some(d) = self.dialog.take() else { return };
        let Some(c) = d.exit().hit(a, x, y) else {
            self.dialog = Some(d);
            return;
        };
        fx.push(snd(SND_CHOOSE));
        match (d, c) {
            (Dialog::Epa(_, after), Choice::Save) => match self.ed.title() {
                Some(t) if self.ed.file().is_some() => self.open_versions(t.to_string(), after, a),
                _ => self.start_rename(after, fx, a),
            },
            (Dialog::Epa(_, after), Choice::Discard) => {
                self.ed.discard();
                self.finish(after, fx);
            }
            (Dialog::Versions(_, after, title), Choice::Save | Choice::Discard) => {
                let keep_old = c == Choice::Save;
                self.save(&title, keep_old, fx);
                self.finish(after, fx);
            }
            (_, Choice::Stay) => self.door_open = false,
        }
    }

    /// SALVAR, Ctrl+S, "Salvar este trabalho": o título vira campo de nome.
    fn start_rename(&mut self, after: After, fx: &mut Vec<Effect>, a: &Assets) {
        fx.push(snd(SND_RENAME));
        let r = Rename::new(self.title(), RenameAfter::Stay);
        self.rename = Some((r, after));
        self.mczee.show((WHAT_GENERAL, WHAT_SAVE), false, a);
    }

    /// Return no campo de nome: salva com o nome e segue.
    fn finish_rename(&mut self, fx: &mut Vec<Effect>, a: &Assets) {
        let Some((r, after)) = self.rename.take() else {
            return;
        };
        self.mczee.close();
        let t = r.text.trim();
        let title = if t.is_empty() {
            chrome::DEFAULT_TITLE.to_string()
        } else {
            t.to_string()
        };
        if self.ed.file().is_some() {
            // já gravado antes: as duas versões ou só a nova?
            self.open_versions(title, after, a);
        } else {
            self.save(&title, false, fx);
            self.finish(after, fx);
        }
    }

    fn save(&mut self, title: &str, keep_old: bool, fx: &mut Vec<Effect>) {
        let author = self.author.clone();
        let old = self.ed.file().map(Path::to_path_buf);
        match self.ed.save_as(title, &author, keep_old) {
            Ok(p) => {
                fx.push(Effect::Log(format!(
                    "documento \"{title}\" guardado em {}",
                    p.display()
                )));
                let replaced = old.as_deref().filter(|o| !keep_old && *o != p);
                if let Some(e) = self.remember(&p, title, replaced) {
                    fx.push(Effect::Log(e));
                }
            }
            Err(e) => fx.push(Effect::Log(e)),
        }
    }

    fn finish(&mut self, after: After, fx: &mut Vec<Effect>) {
        match after {
            After::Stay => {}
            After::Leave => fx.push(Effect::ChangeScene(self.exit_to)),
            After::New => self.ed.new_document(),
            After::Quit => fx.push(Effect::Quit),
            After::Open => {
                fx.push(snd(SND_PORTFOLIO));
                fx.push(Effect::Push(SceneId::Cw(CwScene::Portfolio)));
            }
        }
    }

    // ---------------------------------------------------------------
    // Caderno de Idéias (W20) e Central de Controle (W23)

    /// Caderno de Idéias da Bolsa Mágica: por cima da página, com o balão
    /// `0faa[2]` do McZee.
    fn open_notebook(&mut self, fx: &mut Vec<Effect>, a: &Assets) {
        let users = self
            .ed
            .dir()
            .and_then(split_user_dir)
            .map(|(u, _)| u.to_path_buf());
        let user = if self.user.is_empty() {
            self.ed
                .dir()
                .and_then(split_user_dir)
                .map(|(_, u)| u)
                .unwrap_or_default()
        } else {
            self.user.clone()
        };
        fx.push(snd(notebook::SND_OPEN));
        self.notebook = Some(NotebookOverlay::open(users.as_deref(), &user, a));
        self.mczee
            .show((WHAT_GENERAL, notebook::WHAT_NOTEBOOK), false, a);
    }

    /// Entrada com o Caderno aberto: os balões do McZee primeiro, depois o
    /// caderno.
    fn notebook_input(&mut self, i: &Input, fx: &mut Vec<Effect>, a: &Assets) {
        if let Input::Click {
            button: MouseButton::Left,
            x,
            y,
        } = *i
        {
            if self.mczee.covers(a, x, y) {
                self.mczee.front_click(x, y, fx, a);
                return;
            }
            // clique fora do caderno com o balão do McZee aberto: só o fecha
            if self.mczee.help().is_some() && !self.notebook_contains(x, y) {
                self.mczee.close();
                return;
            }
        }
        let Some(nb) = self.notebook.as_mut() else {
            return;
        };
        let (out, help) = nb.handle(i, fx, a);
        if let Some(h) = help {
            self.mczee.show(h, false, a);
        }
        if out.is_some() {
            self.notebook = None;
            self.mczee.close();
        }
    }

    fn notebook_contains(&self, x: i32, y: i32) -> bool {
        self.notebook.as_ref().is_some_and(|n| {
            let (ox, oy) = n.view().origin();
            let (w, h) = crate::scenes::writer_ideas::notebook::SIZE;
            inside((ox, oy, w, h), x, y)
        })
    }

    /// Central de Controle (Ctrl+Shift+A).
    fn open_center(&mut self, a: &Assets) {
        self.mczee.close();
        self.center = Some(ControlCenter::new(a));
    }

    fn center_input(&mut self, i: &Input, fx: &mut Vec<Effect>, a: &Assets) {
        let Some(c) = self.center.as_mut() else {
            return;
        };
        let Some(act) = c.handle(i, a) else { return };
        match act {
            CenterAction::Close => self.center = None,
            CenterAction::SaveAs => {
                self.center = None;
                self.start_rename(After::Stay, fx, a);
            }
            CenterAction::SaveText => {
                self.center = None;
                fx.push(Effect::Log(match self.ed.save_text() {
                    Ok(p) => format!("texto guardado em {}", p.display()),
                    Err(e) => format!("Salvar Como Texto: {e}"),
                }));
            }
            CenterAction::Sound(on) => {
                fx.push(Effect::SetVolume(if on { 100 } else { 0 }));
            }
            CenterAction::ResetIdeas => {
                let dir = self
                    .ed
                    .dir()
                    .and_then(split_user_dir)
                    .map(|(u, n)| crate::writer::ideas::store::ideas_dir(u, &n));
                if let Some(d) = dir {
                    let empty: [Vec<String>; crate::writer::ideas::machine::WINDOWS] =
                        Default::default();
                    if let Err(e) = crate::writer::ideas::store::save_machine_entries(&d, &empty) {
                        fx.push(Effect::Log(format!("Máquina de Idéias: {e}")));
                    }
                }
                fx.push(Effect::Log(
                    "Máquina de Idéias: entradas do usuário apagadas".into(),
                ));
            }
            CenterAction::PrintOptions { global } => fx.push(Effect::Log(format!(
                "Opções de Impressão ({}): o port imprime pelo lp, sem a caixa Impressora",
                if global { "globais" } else { "locais" }
            ))),
        }
    }

    // ---------------------------------------------------------------
    // Teclado e mouse

    /// Atalhos com Ctrl (`office.md` §6). `true` = tratado.
    fn shortcut(&mut self, key: Key, fx: &mut Vec<Effect>, a: &Assets) -> bool {
        if matches!(key, Key::X | Key::C) && self.ed.encrypted() {
            // Agente Secreto: "Você não pode recortar nem copiar…"
            fx.push(snd(transformers::SND_EPA));
            self.mczee.show(transformers::ENCRYPTED_CUT_COPY, true, a);
            return true;
        }
        match key {
            Key::X => {
                if self.ed.cut() {
                    self.share_clip();
                    fx.push(snd(SND_CUT));
                    self.mczee.show((WHAT_REMOVERS, 8), false, a);
                }
            }
            Key::C => {
                if self.ed.copy() {
                    self.share_clip();
                    fx.push(snd(SND_COPY));
                    self.mczee.show((WHAT_REMOVERS, 12), false, a);
                }
            }
            Key::V => {
                if self.paste() {
                    fx.push(snd(SND_PASTE));
                    self.mczee.show((WHAT_REMOVERS, 14), false, a);
                }
            }
            Key::Z => {
                let undo = !self.ed.undo.can_redo() || self.ed.undo.can_undo();
                if self.ed.toggle_undo() {
                    fx.push(snd(if undo { SND_UNDO } else { SND_REDO }));
                }
            }
            Key::A if self.shift => self.open_center(a),
            Key::A => {
                let d = self.ed.doc().clone();
                self.ed.sel.select_all(&d);
            }
            Key::S => self.start_rename(After::Stay, fx, a),
            Key::Q => self.ask_or(After::Quit, fx, a),
            Key::O => self.ask_or(After::Open, fx, a),
            _ => return false,
        }
        true
    }

    fn key(&mut self, key: Key, ctrl: bool, fx: &mut Vec<Effect>, a: &Assets) {
        if let Some((r, _)) = self.rename.as_mut() {
            if r.key(key) {
                self.finish_rename(fx, a);
            }
            return;
        }
        if self.dialog.is_some() {
            return;
        }
        let t = self.tool;
        if self.with_tool(t, fx, a, |t, ed, cx| t.key(ed, cx, key, ctrl)) {
            return;
        }
        if ctrl && self.shortcut(key, fx, a) {
            return;
        }
        let shift = self.shift;
        self.ed.key(key, ctrl, shift);
    }

    fn bottom_arrow(&mut self, k: usize, fx: &mut Vec<Effect>, a: &Assets) {
        fx.push(snd(SND_ARROWS[k]));
        if !std::mem::replace(&mut self.arrows_used, true) {
            self.mczee.show((WHAT_GENERAL, WHAT_PAGES), false, a);
        }
        let (dx, dy) = [(0, -1), (0, 1), (-1, 0), (1, 0)][k];
        self.ed.scroll_by(dx * ARROW_STEP, dy * ARROW_STEP);
    }

    fn click(&mut self, x: i32, y: i32, fx: &mut Vec<Effect>, a: &Assets) {
        if self.dialog.is_some() {
            self.dialog_click(x, y, fx, a);
            return;
        }
        if self.rename.is_some() {
            if inside(TITLE_LABEL, x, y) {
                return;
            }
            // clicar fora do campo de nome conclui
            self.finish_rename(fx, a);
            return;
        }
        if let Some(req) = self.mczee.front_click(x, y, fx, a) {
            if let Some(r) = req {
                self.request(r, fx, a);
            }
            return;
        }
        if self.mczee.wand() && inside(PAGE, x, y) {
            // W11: tocar o som da palavra ou do adesivo
            fx.push(Effect::Log("Varinha Mágica: nenhum som aqui".into()));
            return;
        }
        let tool = self.tool;
        if self.tools.get(tool).modal() {
            self.with_tool(tool, fx, a, |t, ed, cx| t.press(ed, cx, x, y));
            return;
        }
        if let Some(id) = ToolId::at(x, y) {
            self.select(id, fx, a);
            return;
        }
        if mczee::niche_hit(x, y) {
            self.mczee.niche_click(fx, a);
            return;
        }
        if inside(exit::DOOR_RECT, x, y) {
            self.open_epa(After::Leave, fx, a);
            return;
        }
        if inside(TITLE_LABEL, x, y) {
            self.start_rename(After::Stay, fx, a);
            return;
        }
        for (k, p) in [
            chrome::ARROW_UP,
            chrome::ARROW_DOWN,
            chrome::ARROW_LEFT,
            chrome::ARROW_RIGHT,
        ]
        .into_iter()
        .enumerate()
        {
            if inside(chrome::arrow_rect(p), x, y) {
                self.bottom_arrow(k, fx, a);
                return;
            }
        }
        if self.with_tool(tool, fx, a, |t, ed, cx| t.press(ed, cx, x, y)) {
            return;
        }
        // clique numa palavra com forma reabre o cavalete
        if let Some(k) = self.ed.object_at(x, y)
            && self.ed.doc().objects[k].kind == ObjectKind::Shaped
        {
            self.open_shaped(Some(k), fx, a);
            return;
        }
        let shift = self.shift;
        self.ed.press_page(x, y, shift);
    }

    /// Cursor que o ponto `(x, y)` pede.
    fn wanted_cursor(&self, a: &Assets) -> CursorRef {
        let (x, y) = self.mouse;
        let tool = self.tools.get(self.tool);
        if self.dialog.is_some()
            || self.center.is_some()
            || self.notebook.is_some()
            || self.shaped.is_open()
        {
            DEFAULT_CURSOR
        } else if self.mczee.wand() {
            cursors::writer(cursors::WAND)
        } else if self.mczee.covers(a, x, y) {
            DEFAULT_CURSOR
        } else if let Some(c) = tool.cursor(&self.ed, x, y) {
            c
        } else if inside(PAGE, x, y) && !tool.modal() {
            cursors::writer(cursors::TEXT)
        } else {
            DEFAULT_CURSOR
        }
    }
}

impl Scene for Writer {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        // Ctrl+Q pergunta antes de sair (o "Epa!"), em vez de sair direto
        fx.retain(|e| *e != Effect::Quit);
        if !std::mem::replace(&mut self.started, true) {
            fx.push(snd(mczee::SND_ENTER));
            self.mczee.show(self.welcome, false, assets);
        }
        for i in input {
            if let Input::Shift(on) = *i {
                self.shift = on;
            }
            if let Input::Ctrl(on) = *i {
                self.ctrl = on;
            }
            if self.center.is_some() {
                self.center_input(i, &mut fx, assets);
                continue;
            }
            if self.notebook.is_some() {
                if let Input::MouseMove { x, y } = *i {
                    self.mouse = (x, y);
                }
                self.notebook_input(i, &mut fx, assets);
                continue;
            }
            if self.shaped.is_open() && !matches!(i, Input::Shift(_) | Input::Ctrl(_)) {
                if let Input::MouseMove { x, y } | Input::Click { x, y, .. } = *i {
                    self.mouse = (x, y);
                }
                self.shaped_input(i, &mut fx, assets);
                continue;
            }
            match *i {
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.mouse = (x, y);
                    self.click(x, y, &mut fx, assets);
                }
                Input::MouseMove { x, y } => {
                    self.mouse = (x, y);
                    if self.ed.dragging() {
                        self.ed.drag_page(x, y);
                    }
                    let t = self.tool;
                    self.with_tool(t, &mut fx, assets, |t, ed, cx| t.motion(ed, cx, x, y));
                }
                Input::MouseUp {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.mouse = (x, y);
                    self.ed.release_page();
                    let t = self.tool;
                    self.with_tool(t, &mut fx, assets, |t, ed, cx| t.release(ed, cx, x, y));
                }
                Input::Key {
                    key,
                    ctrl,
                    alt: false,
                } => self.key(key, ctrl, &mut fx, assets),
                Input::Text(ref t) => {
                    let tool = self.tool;
                    if let Some((r, _)) = self.rename.as_mut() {
                        r.type_text(t);
                    } else if !self.with_tool(tool, &mut fx, assets, |w, ed, cx| w.text(ed, cx, t))
                        && self.dialog.is_none()
                        && !self.ctrl
                    {
                        self.ed.type_text(t);
                    }
                }
                Input::Shift(on) => self.shift = on,
                Input::Ctrl(on) => self.ctrl = on,
                _ => {}
            }
        }
        self.ed.tick(dt);
        if let Some(n) = self.notebook.as_mut()
            && dt.is_finite()
        {
            n.tick(dt.max(0.0));
        }
        if dt.is_finite() {
            self.shaped.tick(dt.max(0.0));
        }
        let t = self.tool;
        if dt.is_finite() {
            self.tools.get_mut(t).tick(&mut self.ed, dt.max(0.0));
            self.with_tool(t, &mut fx, assets, |tl, ed, cx| {
                tl.update(ed, cx, dt.max(0.0))
            });
        }
        self.ed.caret_visible = !self.tools.get(t).modal()
            && self.dialog.is_none()
            && self.rename.is_none()
            && self.notebook.is_none()
            && !self.shaped.is_open();
        let want = self.wanted_cursor(assets);
        if let Some(e) = self.pointer.set(want) {
            fx.push(e);
        }
        fx
    }

    fn draw(&self, f: &mut Frame, a: &Assets) {
        let tool = self.tools.get(self.tool);
        chrome::draw_frame(f);
        chrome::draw_toolbar(f, a, self.tool);
        self.mczee.draw_niche(f, a);
        exit::draw(f, a, self.door_open);
        tool.draw_strip(&self.ed, f, a);
        self.ed.draw_page(f, a);
        chrome::draw_bottom(f, a, &self.ed, self.floor, self.title());
        if let Some((r, _)) = &self.rename {
            let (x, y, w, h) = TITLE_LABEL;
            f.rect(x, y, w, h, [255, 255, 255]);
            r.draw_in(f, a.font(8).as_deref(), TITLE_LABEL);
        }
        tool.draw_front(&self.ed, f, a);
        if let Some(n) = &self.notebook {
            n.draw(f, a);
        }
        self.shaped.draw(f, a);
        self.mczee.draw(f, a);
        if let Some(d) = &self.dialog {
            d.exit().draw(f, a);
        }
        if let Some(c) = &self.center {
            c.draw(f, a);
        }
    }

    fn leave(&mut self) -> Vec<Effect> {
        // saiu sem escolher no "Epa!" (janela fechada): guarda num KIDSnnnn.MAX
        match self.ed.save_auto() {
            Some(Ok(p)) => {
                let title = self.ed.title().unwrap_or_default().to_string();
                let mut fx = vec![Effect::Log(format!(
                    "documento guardado em {}",
                    p.display()
                ))];
                fx.extend(self.remember(&p, &title, None).map(Effect::Log));
                fx
            }
            Some(Err(e)) => vec![Effect::Log(e)],
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Program;
    use crate::scene::Key;
    use crate::scenes::testutil::{assets, empty_assets};
    use crate::ui::center;
    use crate::writer::doc::{Align, CSCH, Pos, Size};
    use crate::writer::ideas::machine::Piece;
    use crate::writer::object::ObjectKind;
    use crate::{SCREEN_H, SCREEN_W};
    use word_strip::WordStrip;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn up(x: i32, y: i32) -> Input {
        Input::MouseUp {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            ctrl: false,
            alt: false,
        }
    }

    fn ctrl(key: Key) -> Input {
        Input::Key {
            key,
            ctrl: true,
            alt: false,
        }
    }

    fn text(s: &str) -> Input {
        Input::Text(s.into())
    }

    /// Clique no meio da ferramenta `id` da barra.
    fn tool(id: ToolId) -> Input {
        let (x, y) = center(id.def().hit);
        click(x, y)
    }

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(crate::assets::SoundRef::Snd(Program::Writer, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    fn logs(fx: &[Effect]) -> String {
        fx.iter()
            .filter_map(|e| match e {
                Effect::Log(s) => Some(s.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn para(w: &Writer, k: usize) -> String {
        w.doc().paras[k].text()
    }

    /// Pixels não brancos dentro da página.
    fn inked(w: &Writer, a: &Assets) -> usize {
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, a);
        let mut n = 0;
        for y in PAGE.1..PAGE.1 + PAGE.3 {
            for x in PAGE.0..PAGE.0 + PAGE.2 {
                if f.pixel(x as u32, y as u32) != [255, 255, 255, 255] {
                    n += 1;
                }
            }
        }
        n
    }

    /// Bolinha `k` do "Epa!" aberto.
    fn bullet(w: &Writer, a: &Assets, k: usize) -> Input {
        let (x, y) = w.dialog().expect("Epa! aberto").exit().bullet(a, k);
        click(x, y)
    }

    #[test]
    fn typing_shows_text_on_the_page() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        // cursor apagado (meio período) para contar só o texto
        w.update(&[], 0.6, &a);
        assert_eq!(inked(&w, &a), 0);
        w.update(&[text("Oi")], 0.0, &a);
        w.update(&[], 0.6, &a);
        assert_eq!(para(&w, 0), "Oi");
        let ink = inked(&w, &a);
        assert!(ink > 20, "{ink}");
        // o cursor aceso é uma linha de 1 px da altura da linha (15)
        w.update(&[], 0.5, &a);
        assert_eq!(inked(&w, &a), ink + 15);
    }

    #[test]
    fn text_starts_where_the_original_starts() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[text("H")], 0.6, &a);
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        let ink: Vec<(i32, i32)> = (PAGE.1..PAGE.1 + PAGE.3)
            .flat_map(|y| (PAGE.0..PAGE.0 + PAGE.2).map(move |x| (x, y)))
            .filter(|&(x, y)| f.pixel(x as u32, y as u32)[0] < 128)
            .collect();
        let x0 = ink.iter().map(|p| p.0).min().unwrap();
        let y1 = ink.iter().map(|p| p.1).max().unwrap();
        // 'H' de 12 pontos: base na linha 179 + 12 (ascendente)
        assert!((93..=95).contains(&x0), "{x0}");
        assert_eq!(y1, 179 + 12 - 1);
    }

    #[test]
    fn entering_plays_the_welcome_sound_once() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        assert_eq!(sounds(&w.update(&[], 0.0, &a)), vec![mczee::SND_ENTER]);
        assert!(sounds(&w.update(&[], 0.0, &a)).is_empty());
    }

    #[test]
    fn egg_strip_undoes_and_redoes_with_the_egg_queue() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[text("a"), key(Key::Enter), text("b")], 0.0, &a);
        assert_eq!(w.doc().paras.len(), 2);
        // escolher o ovo não desfaz: só mostra a faixa
        w.update(&[tool(ToolId::Egg)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Egg);
        assert_eq!(para(&w, 1), "b");
        assert_eq!(w.editor().undo.eggs().len(), 3);
        let fx = w.update(&[click(60, 75)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![egg::SND_UNDO]);
        assert_eq!(para(&w, 1), "");
        w.update(&[click(60, 75)], 0.0, &a);
        assert_eq!(w.doc().paras.len(), 1);
        assert_eq!(w.editor().undo.undo_len(), 1);
        assert_eq!(w.editor().undo.redo_len(), 2);
        let fx = w.update(&[click(460, 75)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![egg::SND_REDO]);
        assert_eq!(w.doc().paras.len(), 2);
        // Ctrl+Z alterna: desfaz, e depois de um desfazer refaz
        w.update(&[ctrl(Key::Z)], 0.0, &a);
        assert_eq!(w.doc().paras.len(), 1);
        w.update(&[ctrl(Key::Z)], 0.0, &a);
        assert_eq!(w.doc().paras.len(), 2);
    }

    #[test]
    fn egg_keeps_twenty_actions() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        for _ in 0..25 {
            w.update(&[key(Key::Enter)], 0.0, &a);
        }
        assert_eq!(w.editor().undo.eggs().len(), 20);
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.update(&[tool(ToolId::Egg)], 0.0, &a);
        w.draw(&mut f, &a);
    }

    #[test]
    fn size_list_changes_the_typing_style() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[click(100, 70)], 0.0, &a);
        assert_eq!(w.tools().word.list(), Some(List::Sizes));
        // primeiro item: Gigante
        let r = WordStrip::list_item_rect(List::Sizes, 0);
        w.update(&[click(r.0 + 5, r.1 + 5), text("a")], 0.0, &a);
        assert_eq!(w.tools().word.list(), None);
        assert_eq!(w.doc().paras[0].items[0].props().unwrap().size, Size::Giant);
        // clique fora fecha a lista sem escolher
        w.update(&[click(100, 70), click(300, 300)], 0.0, &a);
        assert_eq!(w.tools().word.list(), None);
        // Esc também fecha
        w.update(&[click(100, 70), key(Key::Escape)], 0.0, &a);
        assert_eq!(w.tools().word.list(), None);
    }

    #[test]
    fn open_list_takes_clicks_before_the_toolbar() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[click(100, 70)], 0.0, &a);
        // clicar numa ferramenta com a lista aberta só fecha a lista
        w.update(&[tool(ToolId::Stickers)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Word);
        assert_eq!(w.tools().word.list(), None);
    }

    #[test]
    fn lists_apply_to_the_selection() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[text("abc"), ctrl(Key::A)], 0.0, &a);
        w.update(&[click(170, 70)], 0.0, &a);
        let r = WordStrip::list_item_rect(List::Colors, 5);
        w.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        let p = *w.doc().paras[0].items[2].props().unwrap();
        assert_eq!(p.color, Some(CSCH[5]));
        w.update(&[click(230, 70)], 0.0, &a);
        let r = WordStrip::list_item_rect(List::Styles, 1);
        w.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert!(w.doc().paras[0].items[0].props().unwrap().bold);
        w.update(&[click(300, 70)], 0.0, &a);
        let r = WordStrip::list_item_rect(List::Align, 2);
        w.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert_eq!(w.doc().paras[0].props.align, Align::Center);
        // letras: a Impact entra na tabela de fontes do documento
        w.update(&[click(40, 70)], 0.0, &a);
        let r = WordStrip::list_item_rect(List::Fonts, 4);
        w.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        let ftc = w.doc().fonts.ftc_of("Impact").unwrap();
        assert_eq!(w.doc().paras[0].items[1].props().unwrap().font, ftc);
        // quatro ações no Ovo
        assert_eq!(w.editor().undo.undo_len(), 5);
    }

    #[test]
    fn sticker_without_data_uses_placeholder_object() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[click(80, 30)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Stickers);
        assert_eq!(w.editor().rolls.len(), 0);
        // sem rolos: cliques na faixa e nas setas não fazem nada
        w.update(&[click(40, 70), click(515, 60), click(515, 90)], 0.0, &a);
        assert!(!w.tools().stickers.film_open());
        w.update(&[], 0.6, &a);
        w.ed.insert_sticker(5, 2);
        let o = &w.doc().objects[0];
        assert_eq!((o.kind, o.width, o.height), (ObjectKind::Sticker, 32, 32));
        assert_eq!((o.x, o.y), (72, 72));
        assert!(inked(&w, &a) > 60);
    }

    #[test]
    fn removers_explosion_clears_with_undo() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[text("abc"), click(330, 30)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Removers);
        let (x, y) = center(removers::EXPLOSION.rect());
        assert_eq!((x, y), (457, 77));
        // a explosão é escolhida; o clique na página apaga
        let fx = w.update(&[click(x, y), click(300, 300)], 0.0, &a);
        assert!(w.doc().is_empty());
        assert!(sounds(&fx).contains(&0x025f));
        w.update(&[ctrl(Key::Z)], 0.0, &a);
        assert_eq!(para(&w, 0), "abc");
    }

    #[test]
    fn cursor_keys_click_and_selection() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[text("ab"), key(Key::Left), text("x")], 0.0, &a);
        assert_eq!(para(&w, 0), "axb");
        w.update(
            &[key(Key::Home), text("1"), key(Key::End), text("2")],
            0.0,
            &a,
        );
        assert_eq!(para(&w, 0), "1axb2");
        w.update(&[key(Key::Enter), text("c"), key(Key::Up)], 0.0, &a);
        assert_eq!(w.editor().sel.caret.para, 0);
        w.update(&[key(Key::Down)], 0.0, &a);
        assert_eq!(w.editor().sel.caret.para, 1);
        // clique no começo da primeira linha
        w.update(&[click(94, 182), up(94, 182)], 0.0, &a);
        assert_eq!(w.editor().sel.caret, Pos::new(0, 0));
        w.update(&[key(Key::Backspace)], 0.0, &a);
        assert_eq!(para(&w, 0), "1axb2");
        // Shift+→ seleciona; digitar troca a seleção
        w.update(
            &[
                Input::Shift(true),
                key(Key::Right),
                key(Key::Right),
                Input::Shift(false),
                text("Z"),
            ],
            0.0,
            &a,
        );
        assert_eq!(para(&w, 0), "Zxb2");
        // arrastar seleciona
        let (x0, y0) = w.editor().to_screen(0, 72, 75);
        w.update(
            &[
                click(x0, y0),
                Input::MouseMove { x: x0 + 200, y: y0 },
                up(x0 + 200, y0),
            ],
            0.0,
            &a,
        );
        let r = w.editor().sel.range();
        assert_eq!((r.start, r.end), (Pos::new(0, 0), Pos::new(0, 4)));
        // Delete apaga a seleção
        w.update(&[key(Key::Delete)], 0.0, &a);
        assert_eq!(para(&w, 0), "");
    }

    #[test]
    fn clipboard_shortcuts() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[text("ola"), ctrl(Key::A)], 0.0, &a);
        let fx = w.update(&[ctrl(Key::C)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_COPY]);
        w.update(&[key(Key::End), ctrl(Key::V)], 0.0, &a);
        assert_eq!(para(&w, 0), "olaola");
        w.update(
            &[
                Input::Shift(true),
                key(Key::Home),
                Input::Shift(false),
                ctrl(Key::X),
            ],
            0.0,
            &a,
        );
        assert_eq!(para(&w, 0), "");
        let fx = w.update(&[ctrl(Key::V)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_PASTE]);
        assert_eq!(para(&w, 0), "olaola");
        // Ctrl+letra não vira texto
        w.update(&[Input::Ctrl(true), text("v"), Input::Ctrl(false)], 0.0, &a);
        assert_eq!(para(&w, 0), "olaola");
    }

    #[test]
    fn bottom_arrows_scroll_the_view() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        let mut inputs = Vec::new();
        // 43 linhas de 15 por folha
        for _ in 0..100 {
            inputs.push(text("a"));
            inputs.push(key(Key::Enter));
        }
        w.update(&inputs, 0.0, &a);
        assert_eq!(w.editor().layout().page_count(), 3);
        // o cursor ficou na última folha e a vista o acompanhou
        assert_eq!(w.page(), 2);
        w.update(&[key(Key::Home), key(Key::PageUp)], 0.0, &a);
        w.ed.show_page(0);
        assert_eq!(w.page(), 0);
        let (dx, dy) = center(chrome::arrow_rect(chrome::ARROW_DOWN));
        let (ux, uy) = center(chrome::arrow_rect(chrome::ARROW_UP));
        let fx = w.update(&[click(dx, dy)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_ARROWS[1]]);
        assert_eq!(w.editor().scroll.1, ARROW_STEP);
        w.update(&[click(ux, uy), click(ux, uy)], 0.0, &a);
        assert_eq!(w.editor().scroll.1, 0);
        // a primeira seta abre "Andando por aí…" (só com a ajuda e os dados)
        for _ in 0..10 {
            w.update(&[click(dx, dy)], 0.0, &a);
        }
        assert_eq!(w.page(), 2);
    }

    #[test]
    fn exit_door_discard_goes_to_the_office_room() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut w = Writer::new_blank(Some(dir.path().to_path_buf()), &a);
        w.update(&[text("oi")], 0.0, &a);
        let fx = w.update(&[click(600, 60)], 0.0, &a);
        assert!(w.door_open());
        assert_eq!(sounds(&fx), SND_EPA.to_vec());
        assert!(matches!(w.dialog(), Some(Dialog::Epa(_, After::Leave))));
        // o "Epa!" é modal: clicar fora não faz nada
        w.update(&[click(100, 30)], 0.0, &a);
        assert_eq!(w.tool(), ToolId::Word);
        // "Ops! Ainda não terminei!" fecha e a porta fecha
        let b = bullet(&w, &a, 2);
        w.update(&[b, click(600, 60)], 0.0, &a);
        assert!(w.door_open());
        // "Jogar fora": vai à sala Escritório e nada é gravado
        let b = bullet(&w, &a, 1);
        let fx = w.update(&[b], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Cw(
            crate::scene::CwScene::Building(crate::scenes::room::Floor::Studio)
        ))));
        assert!(w.leave().is_empty());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn exit_door_save_asks_the_name_then_versions() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut w = Writer::new_blank(Some(dir.path().to_path_buf()), &a).with_author("Ana Teste");
        w.update(&[text("O gato")], 0.0, &a);
        w.update(&[click(600, 60)], 0.0, &a);
        let b = bullet(&w, &a, 0);
        let fx = w.update(&[b], 0.0, &a);
        // sem nome: o título vira campo selecionado
        assert!(sounds(&fx).contains(&SND_RENAME));
        assert_eq!(w.renaming().unwrap().text, "Documento Novo");
        let fx = w.update(&[text("Gato Teste"), key(Key::Enter)], 0.0, &a);
        assert!(logs(&fx).contains("GATO_TES.MAX"), "{}", logs(&fx));
        assert!(fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))));
        let p = dir.path().join("GATO_TES.MAX");
        let (doc, extra) = crate::writer::store::load(&p).unwrap();
        assert_eq!(doc.paras[0].text(), "O gato");
        assert_eq!(extra.add.title, "Gato Teste");
        assert_eq!(extra.add.author, "Ana Teste");
        // com a miniatura da primeira folha, como o original
        let m = formats::max::read(&std::fs::read(&p).unwrap()).unwrap();
        let t = m.thumbnail_bitmap().unwrap();
        assert_eq!((t.width, t.height, t.depth), (108, 144, 4));
        assert!(t.pixels.iter().any(|&c| c != 0));
        // aberto de novo (o último da pasta), com o nome na barra de baixo
        let mut w = Writer::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(w.title(), "Gato Teste");
        w.update(&[key(Key::End), text("!")], 0.0, &a);
        w.update(&[click(600, 60)], 0.0, &a);
        let b = bullet(&w, &a, 0);
        w.update(&[b], 0.0, &a);
        // com nome: "as duas versões ou só a nova?"
        assert!(
            matches!(w.dialog(), Some(Dialog::Versions(_, After::Leave, t)) if t == "Gato Teste")
        );
        let b = bullet(&w, &a, 0);
        w.update(&[b], 0.0, &a);
        let names = |d: &Path| {
            let mut v: Vec<String> = std::fs::read_dir(d)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|n| !n.starts_with('~'))
                .collect();
            v.sort();
            v
        };
        assert_eq!(names(dir.path()), vec!["GATO_T11.MAX", "GATO_TES.MAX"]);
        // "só a nova", como o original: grava com um nome novo e apaga o
        // arquivo do documento
        let mut w = Writer::open(Some(dir.path().to_path_buf()), &p, &a).unwrap();
        w.update(&[text("x"), ctrl(Key::S)], 0.0, &a);
        w.update(&[key(Key::Enter)], 0.0, &a);
        let b = bullet(&w, &a, 1);
        w.update(&[b], 0.0, &a);
        assert!(!w.editor().changed());
        assert_eq!(names(dir.path()), vec!["GATO_T11.MAX", "GATO_T12.MAX"]);
        let p12 = dir.path().join("GATO_T12.MAX");
        assert_eq!(w.editor().file(), Some(p12.as_path()));
        let (doc, _) = crate::writer::store::load(&p12).unwrap();
        assert_eq!(doc.paras[0].text(), "xO gato");
        // e ele é o último documento (o que o Escritório abre ao entrar)
        let again = Writer::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(again.editor().file(), Some(p12.as_path()));
    }

    #[test]
    fn leave_without_choosing_saves_a_kids_file() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut w = Writer::new_blank(Some(dir.path().to_path_buf()), &a);
        assert!(w.leave().is_empty());
        w.update(&[text("Olá"), key(Key::Enter), text("mundo")], 0.0, &a);
        w.ed.insert_sticker(1, 0);
        assert!(logs(&w.leave()).contains("KIDS0001.MAX"));
        let again = Writer::new(Some(dir.path().to_path_buf()), &a);
        assert_eq!(again.doc().paras.len(), 2);
        assert_eq!(again.doc().paras[1].text(), "mundo");
        assert_eq!(again.doc().objects.len(), 1);
        // sem pasta: só o registro
        let mut w = Writer::new_blank(None, &a);
        w.update(&[text("x")], 0.0, &a);
        assert!(logs(&w.leave()).contains("não guardado"));
    }

    #[test]
    fn ctrl_q_and_new_ask_only_with_changes() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        assert!(w.update(&[ctrl(Key::Q)], 0.0, &a).contains(&Effect::Quit));
        w.update(&[text("a")], 0.0, &a);
        let fx = w.update(&[ctrl(Key::Q)], 0.0, &a);
        assert!(!fx.contains(&Effect::Quit));
        assert!(matches!(w.dialog(), Some(Dialog::Epa(_, After::Quit))));
        assert!(!w.door_open());
        let b = bullet(&w, &a, 1);
        assert!(w.update(&[b], 0.0, &a).contains(&Effect::Quit));
        // NOVO do menu do McZee com mudanças: o "Epa!", depois em branco
        let mut w = Writer::new_blank(None, &a);
        w.update(&[text("a"), click(556, 80)], 0.0, &a);
        w.update(&[click(556, 165)], 0.0, &a);
        assert!(matches!(w.dialog(), Some(Dialog::Epa(_, After::New))));
        let b = bullet(&w, &a, 1);
        w.update(&[b], 0.0, &a);
        assert!(w.doc().is_empty());
    }

    #[test]
    fn title_click_renames_and_saves_in_place() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut w = Writer::new_blank(Some(dir.path().to_path_buf()), &a);
        w.update(&[text("a")], 0.0, &a);
        let fx = w.update(&[click(300, 467)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_RENAME]);
        assert!(w.renaming().unwrap().selected);
        // clicar fora conclui
        w.update(&[text("Meu"), click(300, 300)], 0.0, &a);
        assert!(w.renaming().is_none());
        assert_eq!(w.title(), "Meu");
        assert!(dir.path().join("MEU.MAX").exists());
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.update(&[click(300, 467)], 0.0, &a);
        w.draw(&mut f, &a);
    }

    #[test]
    fn idea_story_sentence_opens_with_floor_label_and_returns_to_the_workshop() {
        use crate::writer::ideas::story::IdeaStory;
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let story = IdeaStory::from_sentence(&[
            Piece::Text("O gato ".into()),
            Piece::Rebus(0x25),
            Piece::Text(" voou.".into()),
        ]);
        session::set_story("bia", story);
        let mut w = Writer::idea_story(Some(dir.path().to_path_buf()), "bia", &a);
        // a história é pega uma vez só
        assert!(session::take_story("bia").is_none());
        assert_eq!(para(&w, 0), "O gato  voou.");
        assert!(matches!(w.doc().paras[0].items[7], Item::Rebus(0x25)));
        assert_eq!(w.editor().sel.caret, w.doc().end());
        assert_eq!(w.floor(), story::FLOOR_LABEL);
        assert!(w.editor().changed());
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        // salvar grava FLOR 4 e WIZA "plot"; a porta volta à Oficina
        w.update(&[click(600, 60)], 0.0, &a);
        let b = bullet(&w, &a, 0);
        w.update(&[b], 0.0, &a);
        let fx = w.update(&[text("Gato"), key(Key::Enter)], 0.0, &a);
        assert!(
            fx.contains(&Effect::ChangeScene(SceneId::Cw(
                crate::scene::CwScene::Building(crate::scenes::room::Floor::Tricks)
            ))),
            "{fx:?}"
        );
        let m = formats::max::read(&std::fs::read(dir.path().join("GATO.MAX")).unwrap()).unwrap();
        assert_eq!(m.add.floor, story::FLOOR);
        assert_eq!(m.add.project.to_bytes(), *b"plot");
        // sem história na sessão: em branco, no Escritório
        let w = Writer::idea_story(None, "bia", &a);
        assert!(w.doc().is_empty());
        assert_eq!(w.floor(), chrome::FLOOR_OFFICE);
    }

    #[test]
    fn idea_story_picture_becomes_a_drawing_saved_as_meta() {
        use crate::writer::ideas::story::IdeaStory;
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let img = crate::Image {
            width: 20,
            height: 10,
            rgba: [200u8, 0, 0, 255].repeat(200),
        };
        session::set_story("ana", IdeaStory::from_picture("Foguete", img));
        let mut w = Writer::idea_story(Some(dir.path().to_path_buf()), "ana", &a);
        let o = w.doc().objects[0].clone();
        assert_eq!((o.kind, o.width, o.height), (ObjectKind::Drawing, 20, 10));
        assert!(o.outline.is_some());
        assert!(w.editor().picture(&o).is_some());
        w.update(&[], 0.6, &a);
        assert!(inked(&w, &a) >= 200);
        let p = w.editor_mut().save_as("Foguete", "", false).unwrap();
        let (doc, extra) = crate::writer::store::load(&p).unwrap();
        assert_eq!(doc.objects.len(), 1);
        assert_eq!(extra.add.project.to_bytes(), *b"wpic");
        let c = extra.object(doc.objects[0].object).unwrap();
        assert_eq!(c.name.as_deref(), Some("Foguete"));
    }

    #[test]
    fn paste_uses_the_notebook_clipboard_until_the_editor_copies() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a).with_user("caio");
        w.update(&[], 0.0, &a);
        session::set_clip(
            "caio",
            session::Clip::Text(vec![Piece::Text("Era ".into()), Piece::Rebus(3)]),
        );
        let fx = w.update(&[ctrl(Key::V)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_PASTE]);
        assert_eq!(para(&w, 0), "Era ");
        assert!(matches!(w.doc().paras[0].items[4], Item::Rebus(3)));
        // copiado no editor: o Colar usa o do editor (com os estilos) e a
        // sessão fica com o mesmo texto
        w.update(&[text(" fim"), ctrl(Key::A), ctrl(Key::C)], 0.0, &a);
        assert!(matches!(
            session::clip("caio"),
            Some(session::Clip::Text(_))
        ));
        w.update(&[key(Key::End), ctrl(Key::V)], 0.0, &a);
        assert_eq!(w.doc().paras[0].items.len(), 2 * 9);
        // uma figura do Caderno vira objeto solto
        let img = crate::Image {
            width: 4,
            height: 4,
            rgba: [0u8, 0, 0, 255].repeat(16),
        };
        session::set_clip("caio", session::Clip::Picture(img, "Bola".into()));
        w.update(&[ctrl(Key::V)], 0.0, &a);
        assert_eq!(w.doc().objects.len(), 1);
    }

    #[test]
    fn every_tool_is_selectable_with_its_sound_and_strip_buttons_log() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[], 0.0, &a);
        // a Palavra já está ativa: começa pela segunda e termina nela
        for id in ToolId::ALL.into_iter().cycle().skip(1).take(10) {
            let fx = w.update(&[tool(id)], 0.0, &a);
            assert_eq!(w.tool(), id);
            assert!(sounds(&fx).contains(&id.def().sound), "{id:?}");
        }
        // de novo na mesma: som de reabrir o balão
        let fx = w.update(&[tool(ToolId::Word)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_TOOL_AGAIN]);
        // botão de faixa ainda sem função: som de opção e registro
        w.update(&[tool(ToolId::Sounds)], 0.0, &a);
        let fx = w.update(&[click(470, 70)], 0.0, &a);
        assert!(logs(&fx).contains("Gravador"));
        assert_eq!(sounds(&fx), vec![tools::SND_OPTION]);
        // o interruptor troca a página da faixa da Preparação da página
        w.update(&[tool(ToolId::Page)], 0.0, &a);
        let fx = w.update(&[click(515, 70), click(150, 75)], 0.0, &a);
        assert!(logs(&fx).contains("Borda"));
    }

    #[test]
    fn mczee_niche_opens_the_menu_and_dismiss_turns_help_off() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[], 0.0, &a);
        let fx = w.update(&[click(556, 80)], 0.0, &a);
        assert!(w.mczee().menu_open());
        assert_eq!(sounds(&fx), vec![0x02ce]);
        // cara do McZee: "vou embora?"; a primeira bolinha desliga a ajuda
        w.update(&[click(556, 120)], 0.0, &a);
        assert!(!w.mczee().menu_open());
        assert_eq!(w.mczee().talk(), Some(mczee::Talk::Dismiss));
        assert!(w.mczee().help_on());
        // Caderno de Idéias: por cima da página; NÃO fecha
        w.update(&[click(300, 300), click(556, 80)], 0.0, &a);
        let fx = w.update(&[click(556, 300)], 0.0, &a);
        assert_eq!(sounds(&fx), vec![SND_CHOOSE, notebook::SND_OPEN]);
        assert!(w.notebook().is_some());
        let (ox, oy) = w.notebook().unwrap().view().origin();
        w.update(&[click(ox + 210, oy + 300)], 0.0, &a);
        assert!(w.notebook().is_none());
        // o livro verde: a história do Writer por cima
        w.update(&[click(556, 80)], 0.0, &a);
        let fx = w.update(&[click(556, 430)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::Cw(CwScene::BookStory))));
        // Varinha: cursor de varinha até escolher uma ferramenta
        w.update(&[click(556, 80), click(556, 345)], 0.0, &a);
        assert!(w.mczee().wand());
        assert_eq!(w.cursor(), cursors::writer(cursors::WAND));
        w.update(&[tool(ToolId::Word)], 0.0, &a);
        assert!(!w.mczee().wand());
        // SALVAR do menu: o campo de nome
        w.update(&[click(556, 80), click(556, 255)], 0.0, &a);
        assert!(w.renaming().is_some());
    }

    /// Máscara da Bolsa Mágica: uma piada do `0201` num balão sem
    /// rabicho, centrado em (320,240), com o som `0226`; um clique fecha.
    #[test]
    fn mask_tells_a_joke_in_the_middle_without_tail() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[], 0.0, &a);
        w.update(&[click(556, 80)], 0.0, &a);
        let fx = w.update(&[click(556, 389)], 0.0, &a);
        let s = sounds(&fx);
        assert_eq!(s[0], SND_CHOOSE);
        assert!(mczee::SND_JOKE.contains(&s[1]), "{s:?}");
        assert_eq!(w.mczee().talk(), Some(mczee::Talk::Joke));
        let b = w.mczee().talk_balloon().unwrap();
        assert!(b.tail.is_none());
        let h = b.height(&a);
        assert!((b.x + b.box_width() / 2 - 320).abs() <= 1, "{}", b.x);
        assert!((b.y + h / 2 - 240).abs() <= 1, "{}", b.y);
        w.update(&[click(30, 300)], 0.0, &a);
        assert_eq!(w.mczee().talk(), None);
    }

    /// Maleta da Bolsa Mágica e Ctrl+O: a pasta por cima (som `02f1`); com
    /// mudanças, o "Epa!" antes.
    #[test]
    fn suitcase_and_ctrl_o_open_the_portfolio() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        w.update(&[click(556, 80)], 0.0, &a);
        let fx = w.update(&[click(556, 213)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::Cw(CwScene::Portfolio))));
        assert!(sounds(&fx).contains(&SND_PORTFOLIO));
        let fx = w.update(&[ctrl(Key::O)], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::Cw(CwScene::Portfolio))));
        w.update(&[text("a")], 0.0, &a);
        let fx = w.update(&[ctrl(Key::O)], 0.0, &a);
        assert!(!fx.contains(&Effect::Push(SceneId::Cw(CwScene::Portfolio))));
        assert!(matches!(w.dialog(), Some(Dialog::Epa(_, After::Open))));
        assert!(!w.door_open());
        // jogar fora: a pasta abre
        let b = bullet(&w, &a, 1);
        let fx = w.update(&[b], 0.0, &a);
        assert!(fx.contains(&Effect::Push(SceneId::Cw(CwScene::Portfolio))));
    }

    /// Ctrl+Shift+A: a Central de Controle por cima de tudo; Salvar Como
    /// Texto grava o `.TXT`, Controle de Som desliga, Esc volta.
    #[test]
    fn ctrl_shift_a_opens_the_control_center() {
        let (_d, a) = empty_assets();
        let dir = tempfile::tempdir().unwrap();
        let mut w = Writer::new_blank(Some(dir.path().join("ana")), &a);
        std::fs::create_dir_all(dir.path().join("ana")).unwrap();
        w.update(&[text("Oi")], 0.0, &a);
        w.update(
            &[Input::Shift(true), ctrl(Key::A), Input::Shift(false)],
            0.0,
            &a,
        );
        let c = w.control_center().expect("Central aberta");
        // tudo vai para a Central: digitar não muda o documento
        let (tx, ty) = center(c.title_rects(&a)[0]);
        w.update(&[text("zzz")], 0.0, &a);
        assert_eq!(para(&w, 0), "Oi");
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        assert_eq!(f.pixel(320, 20), [0, 0, 0, 255]);
        // Arquivo › Salvar Como Texto...
        w.update(&[click(tx, ty)], 0.0, &a);
        let c = w.control_center().unwrap();
        let (_, items) = c.list_rects(&a, 0);
        let k = c.columns()[0]
            .items
            .iter()
            .position(|e| e.id == control_center::cmd::SAVE_TEXT)
            .unwrap();
        let (x, y) = center(items[k]);
        let fx = w.update(&[click(x, y)], 0.0, &a);
        assert!(logs(&fx).contains("TEXTO.TXT"), "{}", logs(&fx));
        assert!(w.control_center().is_none());
        let t = std::fs::read_to_string(dir.path().join("ana").join("TEXTO.TXT")).unwrap();
        assert!(t.starts_with("Oi"));
        // Ferramentas › Controle de Som › Desligar sons
        w.update(
            &[Input::Shift(true), ctrl(Key::A), Input::Shift(false)],
            0.0,
            &a,
        );
        let c = w.control_center().unwrap();
        let (tx, ty) = center(c.title_rects(&a)[2]);
        w.update(&[click(tx, ty)], 0.0, &a);
        let (_, items) = w.control_center().unwrap().list_rects(&a, 2);
        let (x, y) = center(items[0]);
        w.update(&[click(x, y)], 0.0, &a);
        let d = w.control_center().unwrap().dialog().unwrap().clone();
        let off = d
            .buttons
            .iter()
            .find(|b| b.1 == control_center::dlg::SOUND_OFF)
            .unwrap();
        let (x, y) = center(off.0);
        let fx = w.update(&[click(x, y)], 0.0, &a);
        assert!(fx.contains(&Effect::SetVolume(0)));
        // Esc volta ao Creative Writer
        w.update(&[key(Key::Escape)], 0.0, &a);
        assert!(w.control_center().is_none());
        // Salvar Como: o campo de nome
        w.update(
            &[Input::Shift(true), ctrl(Key::A), Input::Shift(false)],
            0.0,
            &a,
        );
        let c = w.control_center().unwrap();
        let (tx, ty) = center(c.title_rects(&a)[0]);
        w.update(&[click(tx, ty)], 0.0, &a);
        let (_, items) = w.control_center().unwrap().list_rects(&a, 0);
        let (x, y) = center(items[2]);
        w.update(&[click(x, y)], 0.0, &a);
        assert!(w.renaming().is_some());
    }

    /// Caderno de Idéias do editor: a câmera copia uma sentença da aba
    /// Máquina (balão `0faa[19]`), OK grava, e o Ctrl+V a cola na página.
    #[test]
    fn notebook_copies_a_sentence_that_the_editor_pastes() {
        use crate::scenes::writer_ideas::notebook::ToolMode;
        use crate::writer::ideas::store::{Notebook, ideas_dir};
        let (_d, a) = empty_assets();
        let users = tempfile::tempdir().unwrap();
        let user = format!("cad{}", std::process::id());
        let mut nb = Notebook::default();
        nb.sentences
            .push(vec![Piece::Text("Um gato no telhado".into())]);
        nb.save(&ideas_dir(users.path(), &user)).unwrap();
        session::set_tab(&user, session::Tab::Machine);
        let mut w = Writer::new_blank(Some(users.path().join(&user)), &a).with_user(&user);
        w.update(&[click(556, 80), click(556, 300)], 0.0, &a);
        let (ox, oy) = w.notebook().expect("caderno").view().origin();
        assert_eq!(w.notebook().unwrap().view().sentences().len(), 1);
        // câmera, depois a sentença
        w.update(&[click(ox + 69, oy + 302)], 0.0, &a);
        assert_eq!(w.notebook().unwrap().view().mode(), Some(ToolMode::Copy));
        w.update(&[click(ox + 80, oy + 20)], 0.0, &a);
        assert!(matches!(
            session::clip(&user),
            Some(session::Clip::Text(p)) if p == vec![Piece::Text("Um gato no telhado".into())]
        ));
        // OK fecha (e grava); o texto digitado volta ao editor
        w.update(&[click(ox + 243, oy + 302)], 0.0, &a);
        assert!(w.notebook().is_none());
        w.update(&[ctrl(Key::V)], 0.0, &a);
        assert_eq!(para(&w, 0), "Um gato no telhado");
    }

    #[test]
    fn cursor_is_text_over_the_page_and_default_elsewhere() {
        let (_d, a) = empty_assets();
        let mut w = Writer::new_blank(None, &a);
        let fx = w.update(&[Input::MouseMove { x: 300, y: 300 }], 0.0, &a);
        let text_cursor = cursors::writer(cursors::TEXT);
        assert!(fx.contains(&Effect::SetCursor(text_cursor)));
        // sem repetir o mesmo cursor
        let fx = w.update(&[Input::MouseMove { x: 310, y: 300 }], 0.0, &a);
        assert!(!fx.iter().any(|e| matches!(e, Effect::SetCursor(_))));
        let fx = w.update(&[Input::MouseMove { x: 300, y: 30 }], 0.0, &a);
        assert!(fx.contains(&Effect::SetCursor(DEFAULT_CURSOR)));
        // lista aberta: a página não mostra o cursor de texto
        w.update(
            &[click(100, 70), Input::MouseMove { x: 300, y: 300 }],
            0.0,
            &a,
        );
        assert_eq!(w.cursor(), DEFAULT_CURSOR);
    }

    #[test]
    fn real_assets_balloons_follow_tools_and_options() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new_blank(None, &a);
        w.update(&[], 0.0, &a);
        // boas-vindas: "Oi! Aqui é o McZee!"
        assert_eq!(w.mczee().help(), Some(mczee::WELCOME));
        // os cursores usados existem no WRITER.EXE
        for c in [cursors::TEXT, cursors::WAND, cursors::BRUSH, cursors::CUT] {
            assert!(a.cursor(&cursors::writer(c)).is_some(), "{c:x}");
        }
        for id in ToolId::ALL {
            w.update(&[tool(id)], 0.0, &a);
            assert_eq!(w.mczee().help(), Some(id.def().help), "{id:?}");
        }
        // opção da faixa: Dimensiona-palavras = 0fa0[4]
        w.update(&[tool(ToolId::Word), click(100, 70)], 0.0, &a);
        assert_eq!(w.mczee().help(), Some((tools::WHAT_WORD, 4)));
        w.update(&[key(Key::Escape)], 0.0, &a);
        // "dê o fora": a primeira bolinha desliga a ajuda
        w.update(&[click(300, 300), click(556, 80), click(556, 120)], 0.0, &a);
        assert_eq!(w.mczee().talk(), Some(mczee::Talk::Dismiss));
        let (x, y) = center(w.mczee().talk_option(&a, 0).unwrap());
        w.update(&[click(x, y)], 0.0, &a);
        assert!(!w.mczee().help_on());
        // sem o McZee, escolher ferramenta não abre balão
        w.update(&[tool(ToolId::Viewer)], 0.0, &a);
        assert_eq!(w.mczee().help(), None);
        // o nicho vazio pergunta se ele deve voltar; "Sim, por favor!"
        w.update(&[click(556, 80)], 0.0, &a);
        assert_eq!(w.mczee().talk(), Some(mczee::Talk::Return));
        let (x, y) = center(w.mczee().talk_option(&a, 0).unwrap());
        w.update(&[click(x, y), tool(ToolId::Print)], 0.0, &a);
        assert!(w.mczee().help_on());
        assert_eq!(w.mczee().help(), Some(ToolId::Print.def().help));
    }

    #[test]
    fn real_assets_idea_story_opens_with_its_balloon() {
        use crate::writer::ideas::story::{IdeaSource, IdeaStory};
        let Some(a) = assets() else { return };
        session::set_story("rui", IdeaStory::from_sentence(&[Piece::Text("Oi".into())]));
        let mut w = Writer::idea_story(None, "rui", &a);
        w.update(&[], 0.0, &a);
        assert_eq!(w.mczee().help(), Some(IdeaSource::Machine.help()));
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "writer_idea_story");
    }

    /// Com os dados reais: um usuário novo entra no Escritório com o último
    /// documento das preferências, o exemplo do Max ("O Ponto de Fuga"),
    /// aberto como cópia (salvar grava na pasta dele); o último gravado por
    /// `prefs::remember_last` é o que ele abre da próxima vez.
    #[test]
    fn real_assets_writer_new_opens_the_last_document() {
        let Some(a) = assets() else { return };
        let users = tempfile::tempdir().unwrap();
        let dir = users.path().join("ana");
        std::fs::create_dir_all(&dir).unwrap();
        let w = Writer::new(Some(dir.clone()), &a);
        assert_eq!(w.title(), "O Ponto de Fuga");
        assert!(w.editor().file().is_none());
        let samp = crate::paths::find_ci(a.data_dir(), "SHARED/USERS/MAX/SAMP9.MAX").unwrap();
        // a pasta e a Biblioteca o gravam como último antes do OpenDocument
        let w = Writer::open(Some(dir.clone()), &samp, &a).unwrap();
        let t = w.title().to_string();
        prefs::remember_last(users.path(), "ana", a.data_dir(), &samp, &t).unwrap();
        let (d, f) = prefs::last_document(Some(users.path()), "ana", a.data_dir()).unwrap();
        assert_eq!(
            (d.file.as_str(), d.title.as_str()),
            ("SAMP9.MAX", t.as_str())
        );
        assert_eq!(f, samp);
        assert_eq!(Writer::new(Some(dir), &a).title(), t);
    }

    /// Com os dados reais: o Caderno do editor abre com o balão do McZee
    /// `0faa[2]` ("Caderno de Idéias") e a moldura pontilhada.
    #[test]
    fn real_assets_notebook_opens_with_the_mczee_balloon() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new_blank(None, &a).with_user("ana");
        w.update(&[], 0.0, &a);
        w.update(&[click(556, 80), click(556, 300)], 0.0, &a);
        assert!(w.notebook().is_some());
        assert_eq!(
            w.mczee().help(),
            Some((WHAT_GENERAL, notebook::WHAT_NOTEBOOK))
        );
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "cw_notebook_editor");
        // pontilhado cinza 100 na barra de ferramentas
        assert_eq!(f.pixel(10, 30), [100, 100, 100, 255]);
    }

    #[test]
    fn real_assets_epa_uses_the_writer_records() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new_blank(None, &a);
        w.update(&[text("a"), click(600, 60)], 0.0, &a);
        let d = w.dialog().unwrap().exit();
        // a caixa do 0203[0] medida em office_134_saida: x 211..428
        let (x, _, wd, _) = d.rect(&a);
        assert!(
            (x - 211).abs() <= 2 && (wd - 218).abs() <= 2,
            "{:?}",
            d.rect(&a)
        );
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "writer_epa");
    }

    #[test]
    fn real_assets_chrome_draws_icons_and_stickers() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new_blank(None, &a);
        assert!(!w.editor().rolls.is_empty());
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        // ícone da Palavra, em cima da aba clara
        assert_ne!(f.pixel(30, 30), [190, 190, 190, 255]);
        // adesivo real: objeto solto do tamanho do desenho, com contorno
        w.update(&[click(80, 30), click(40, 70)], 0.0, &a);
        assert!(w.tools().stickers.film_open());
        let c = w.tools().stickers.film().cell(0);
        w.update(&[click(c.0 + 5, c.1 + 5)], 0.0, &a);
        assert!(!w.tools().stickers.film_open());
        let o = w.doc().objects[0].clone();
        assert!(o.width != 32 || o.height != 32);
        assert!(o.outline.is_some());
        assert!(w.editor().extra.object(o.object).is_some());
        w.update(&[], 0.6, &a);
        assert!(inked(&w, &a) > 0);
        // o texto corre em volta do adesivo
        w.update(&[text("palavra ")], 0.0, &a);
        let l = w.editor().layout();
        assert!(
            l.lines[0].boxes[0].x >= i32::from(o.x) + i32::from(o.width)
                || l.lines[0].top >= i32::from(o.y) + i32::from(o.height)
        );
    }

    /// Com `ATELIE_DUMP`, grava a tela com a faixa de cada ferramenta e o
    /// menu do McZee (para comparar com `cw/shots/office_*.png`).
    #[test]
    fn real_assets_dump_every_strip() {
        let Some(a) = assets() else { return };
        let mut w = Writer::new_blank(None, &a);
        w.update(&[], 0.0, &a);
        for id in ToolId::ALL {
            w.update(&[tool(id)], 0.0, &a);
            let mut f = Frame::new(SCREEN_W, SCREEN_H);
            w.draw(&mut f, &a);
            crate::scenes::testutil::dump(&f, &format!("writer_{:?}", id).to_lowercase());
        }
        w.update(&[click(300, 300), click(556, 80)], 0.0, &a);
        assert!(w.mczee().menu_open());
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        w.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "writer_menu");
    }
}
