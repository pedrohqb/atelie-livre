//! Camada da frente da tela de pintura: menu do McZee, lente de aumento,
//! diálogo "Epa!", nome do quadro e balões de opções. Métodos de [`Paint`]
//! chamados de `click`, `update` e `draw` (ver
//! `docs/telas/pintura-menu-mczee.md`).

use super::*;

impl Paint {
    /// Ponto da tela → pixel do quadro (pela lente, se ligada).
    pub(super) fn canvas_pt(&self, x: i32, y: i32) -> (i32, i32) {
        match &self.zoom {
            Some(z) => z.to_canvas(x, y),
            None => (x - ORIGIN.0, y - ORIGIN.1),
        }
    }

    /// Quadro comum do Ateliê (não é projeto, lição nem figura de um desenho
    /// animado): onde valem o "Epa!", o nome e a faixa do Imprimir.
    pub(super) fn plain_studio(&self) -> bool {
        self.project.is_none() && self.trick.is_none() && self.edit_file.is_none()
    }

    /// Nome para gravar: o dado pelo usuário, ou nenhum (carimbo de data).
    pub(super) fn save_name(&self) -> Option<String> {
        (self.title != DEFAULT_NAME).then(|| self.title.clone())
    }

    /// Nome do quadro na barra de baixo.
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn menu_open(&self) -> bool {
        self.menu_open
    }

    pub fn zoomed(&self) -> bool {
        self.zoom.is_some()
    }

    pub fn exit_dialog_open(&self) -> bool {
        self.exit_dialog.is_some()
    }

    pub fn renaming(&self) -> Option<&str> {
        self.rename.as_ref().map(|r| r.text.as_str())
    }

    /// Cliques que vêm antes de tudo: o "Epa!" (modal), o nome em edição,
    /// os balões de opções, as setas da lente e o menu do McZee. `true`
    /// quando o clique foi consumido.
    pub(super) fn front_click(
        &mut self,
        x: i32,
        y: i32,
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) -> bool {
        if let Some(d) = &self.exit_dialog {
            match d.hit(assets, x, y) {
                Some(c) => {
                    let reason = d.reason;
                    self.exit_choice(reason, c, fx, assets);
                }
                None => fx.push(sfx::play(SND_BEEP)),
            }
            return true;
        }
        if self.rename.is_some() {
            // qualquer clique fora do campo cancela a edição
            if !rename::label_hit(x, y) {
                self.rename = None;
                self.help = None;
            }
            return true;
        }
        if let Some((b, kind)) = self.talk.take() {
            if b.contains(assets, x, y) {
                match b.hit(assets, x, y) {
                    Some(k) => self.talk_choice(kind, k, fx),
                    None => {
                        if kind == Talk::Joke {
                            return true;
                        }
                        self.talk = Some((b, kind));
                    }
                }
                return true;
            }
            if kind == Talk::Maggie {
                self.door = Door::Closed;
            }
            if kind == Talk::Joke {
                return true;
            }
            // clique fora fecha e segue
        }
        // o Cavalete e o Cachorrinho cobrem a lente do quadro
        if self.easel.is_some() || self.finder.is_some() {
            return false;
        }
        if let Some(z) = self.zoom.as_mut()
            && let Some(d) = zoom::arrow_at(x, y)
        {
            if z.scroll(d) {
                fx.push(sfx::play(SND_ZOOM_SCROLL[d as usize]));
            }
            return true;
        }
        if self.menu_open && menu::contains(x, y) {
            if let Some(item) = menu::hit(x, y) {
                self.menu_open = false;
                self.help = None;
                self.menu_item(item, fx, assets);
            }
            return true;
        }
        false
    }

    /// Abre um balão de opções do bloco `block`, registro `k`, em `pos`.
    pub(super) fn open_talk(
        &mut self,
        block: u16,
        k: usize,
        pos: (i32, i32),
        kind: Talk,
        assets: &Assets,
    ) {
        let records = self
            .dialogs
            .entry(block)
            .or_insert_with(|| Rc::new(assets.dialogs(Program::Artist, block)))
            .clone();
        let name = super::super::room::DEFAULT_USER;
        let fallback = Balloon {
            x: pos.0,
            y: pos.1,
            w: 170,
            text: String::new(),
            options: Vec::new(),
            links: Vec::new(),
            figures: Vec::new(),
        };
        let mut b = Balloon::from_block(&records, k, pos.0, pos.1, name).unwrap_or(fallback);
        if b.text.is_empty() && b.options.is_empty() {
            let (text, options) = match kind {
                Talk::Dismiss => (
                    "OK, vou embora se você quiser mesmo que eu vá.",
                    vec!["É, por favor dê o fora", "Não, preciso de mais ajuda!"],
                ),
                Talk::Return => (
                    "Oi! Aqui está o McZee para resolver todos os problemas!",
                    vec!["Sim, por favor!", "Não, obrigado!"],
                ),
                Talk::Maggie => (
                    "Oi! Você quer que eu te mostre alguns truques de desenho?",
                    vec!["É, me mostre alguns truques!", "Não, obrigado!"],
                ),
                Talk::Joke => ("Por que é que o elefante fica vesgo?", vec![]),
            };
            b.text = text.to_string();
            b.options = options.into_iter().map(str::to_string).collect();
        }
        self.help = None;
        self.talk = Some((b, kind));
    }

    /// Bolinha `k` de um balão de opções.
    fn talk_choice(&mut self, kind: Talk, k: usize, fx: &mut Vec<Effect>) {
        fx.push(sfx::play(if k == 0 {
            SND_BALLOON_YES
        } else {
            SND_BALLOON_NO
        }));
        match (kind, k) {
            (Talk::Dismiss, 0) => {
                self.help_on = false;
                self.help = None;
                self.menu_open = false;
            }
            (Talk::Return, 0) => self.help_on = true,
            (Talk::Maggie, 0) => fx.push(Effect::ChangeScene(SceneId::TricksFloor)),
            (Talk::Maggie, _) => {
                fx.extend(SND_EPA_CLOSE.map(sfx::play));
                self.door = Door::Closed;
            }
            _ => {}
        }
    }

    /// Item escolhido no menu do McZee.
    fn menu_item(&mut self, item: menu::Item, fx: &mut Vec<Effect>, assets: &Assets) {
        match item {
            menu::Item::Dismiss => {
                fx.push(sfx::play(SND_DISMISS_OPEN));
                self.open_talk(WHAT_MAP, WHAT_MCZEE_OFF, HELP_POS, Talk::Dismiss, assets);
            }
            menu::Item::New => self.open_exit_dialog(Reason::New, fx, assets),
            menu::Item::Open => {
                fx.push(sfx::play(SND_FOLDER));
                fx.extend(self.save_now());
                self.folder_pushed = true;
                fx.push(Effect::Push(SceneId::Portfolio));
            }
            menu::Item::Save => {
                fx.push(sfx::play(SND_SAVE_MENU));
                self.start_rename(After::Stay, assets);
            }
            menu::Item::Tricks => {
                fx.push(sfx::play(SND_MCZEE_SCREEN));
                self.door = Door::Maggie;
                self.open_talk(WHAT_MAGGIE.0, WHAT_MAGGIE.1, HELP_POS, Talk::Maggie, assets);
            }
            menu::Item::Wand => {
                fx.push(sfx::play(SND_WAND));
                self.wand = true;
                self.open_help_at(WHAT_MAP, WHAT_WAND, HELP_POS, assets);
            }
            menu::Item::Joke => {
                fx.push(sfx::play(SND_JOKE));
                let n = assets.dialogs(Program::Artist, WHAT_JOKES).len().max(1);
                let k = crate::rng::XorShift::from_clock().below(n);
                self.open_talk(WHAT_JOKES, k, (JOKE_POS.0, JOKE_POS.1), Talk::Joke, assets);
                if let Some((b, _)) = self.talk.as_mut() {
                    b.w = JOKE_POS.2;
                    b.options.clear();
                }
            }
            menu::Item::Story => {
                fx.push(sfx::play(SND_MCZEE_SCREEN));
                fx.push(Effect::Push(SceneId::BookStory));
            }
        }
    }

    /// Lente de aumento: liga centrada no último ponto tocado; desliga.
    pub(super) fn toggle_zoom(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        if self.zoom.take().is_some() {
            fx.push(sfx::play(SND_ZOOM_OFF));
            self.help = None;
            return;
        }
        fx.push(sfx::play(SND_ZOOM_ON));
        self.zoom = Some(Zoom::centered(self.last_canvas.0, self.last_canvas.1));
        if self.help_on {
            self.open_help_at(WHAT_MAP, WHAT_MAGNIFY, HELP_POS, assets);
        }
    }

    /// Abre o campo de nome na barra de baixo, com o nome selecionado.
    pub(super) fn start_rename(&mut self, after: After, assets: &Assets) {
        self.rename = Some(Rename::new(&self.title, after));
        self.menu_open = false;
        if self.help_on {
            self.open_help_at(WHAT_MAP, WHAT_RENAME, HELP_POS, assets);
        }
    }

    /// Return no campo de nome: guarda com o nome e faz o que estava
    /// pendente (sair pela porta ou começar um quadro novo).
    pub(super) fn finish_rename(&mut self, fx: &mut Vec<Effect>) {
        let Some(r) = self.rename.take() else {
            return;
        };
        self.help = None;
        self.title = r.name();
        fx.extend(self.save_now());
        match r.after {
            After::Stay => {}
            After::Exit => fx.push(Effect::ChangeScene(SceneId::Studio)),
            After::New => self.new_picture(false),
        }
    }

    /// Grava o quadro agora (`ultimo.png` e a cópia com o nome ou a data).
    pub(super) fn save_now(&mut self) -> Vec<Effect> {
        let Some(dir) = &self.drawings else {
            return vec![Effect::Log("sem pasta de desenhos: nada guardado".into())];
        };
        let saved = save_drawing_named(
            dir,
            &self.canvas,
            &self.objects,
            &self.pal,
            self.changed,
            now_secs(),
            self.save_name().as_deref(),
        );
        self.changed = false;
        vec![Effect::Log(match saved {
            Ok(()) => format!("desenho \"{}\" guardado em {}", self.title, dir.display()),
            Err(e) => e,
        })]
    }

    /// Quadro em branco: limpa a pintura e os objetos e volta ao nome
    /// padrão; `undoable` deixa o ovo trazer o anterior de volta.
    fn new_picture(&mut self, undoable: bool) {
        if undoable {
            self.history.record_all(&self.canvas, &self.objects);
        }
        self.canvas = Canvas::new(CANVAS_W, CANVAS_H);
        self.objects = Objects::default();
        self.title = DEFAULT_NAME.to_string();
        self.changed = false;
        self.zoom = None;
    }

    /// Abre a porta (ou não, no NOVO) e o diálogo "Epa!".
    pub(super) fn open_exit_dialog(
        &mut self,
        reason: Reason,
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) {
        fx.extend(SND_EPA.map(sfx::play));
        if reason == Reason::Exit {
            self.door = Door::Open;
        }
        self.menu_open = false;
        self.help = None;
        self.talk = None;
        self.exit_dialog = Some(ExitDialog::new(reason, assets));
    }

    /// Bolinha escolhida no "Epa!".
    fn exit_choice(&mut self, reason: Reason, c: Choice, fx: &mut Vec<Effect>, assets: &Assets) {
        self.exit_dialog = None;
        match c {
            Choice::Save => {
                fx.push(sfx::play(SND_BALLOON_YES));
                let after = match reason {
                    Reason::Exit => After::Exit,
                    Reason::New => After::New,
                };
                self.start_rename(after, assets);
            }
            Choice::Discard => {
                fx.push(sfx::play(SND_BALLOON_YES));
                match reason {
                    Reason::Exit => {
                        self.new_picture(false);
                        fx.push(Effect::ChangeScene(SceneId::Studio));
                    }
                    Reason::New => self.new_picture(true),
                }
            }
            Choice::Stay => {
                fx.extend(SND_EPA_CLOSE.map(sfx::play));
                self.door = Door::Closed;
            }
        }
    }
}
