//! Corretor e troca-palavras, a abelha (`office-ferramentas.md` §4): PULAR
//! PARA, CORRIGIR, TROCAR e o Embaralhador. Cada um abre um **painel** no
//! lugar da faixa, que escurece a barra de cima e empurra a página para baixo
//! (50, 152 e 95 pixels). Os botões da faixa e dos painéis agem ao soltar
//! sobre eles.
//!
//! - **Pular para**: campo "AJUDE A ACHAR" e o coelho ACHAR (palavra
//!   inteira; com Ctrl, parte de palavra); fim do documento `0805[8]`, nada
//!   achado `0805[0]`.
//! - **Corrigir**: percorre o texto a partir do cursor (e volta ao começo
//!   até o ponto de partida); palavra desconhecida em vídeo inverso, a
//!   primeira sugestão no campo e todas na lista MAIS ESCOLHAS; MUDAR troca,
//!   PULAR segue; no fim, `0805[2]` "Já acabei de corrigir".
//! - **Trocar** / **Embaralhar**: TROCAR ISTO (a palavra do cursor) e POR
//!   ISTO; PALAVRAS PARECIDAS (sinônimos) ou PALAVRAS EMBARALHADAS
//!   (anagramas) numa lista suspensa; UMA, OUTRA e TODAS (só no Trocar).
//!
//! O dicionário ([`crate::writer::dict`]) carrega em segundo plano ao entrar
//! no Escritório ([`preload`]). Ver `docs/telas/creative-writer-abelha.md`.

mod dictload;
mod search;
mod widgets;

use super::cursors;
use super::editor::{Editor, PAGE, PAGE_GAP};
use super::tools::{
    Cx, Pic, Rect, SND_CHOOSE, SND_OPTION, SND_SWITCH, StripButton, ToolId, ToolStrip,
    WHAT_SPELLER, inside, strip_button,
};
use crate::Frame;
use crate::assets::{Assets, CursorRef};
use crate::scene::Key;
use crate::text_balloon::TextBalloon;
use crate::writer::dict::Dictionary;
use crate::writer::doc::{Pos, Range};
use crate::writer::selection::Selection;
use std::path::PathBuf;
use std::sync::Arc;
use widgets::{BLACK, Field, GRAY, LIGHT, List, ROW, WHITE};

pub use dictload::State as DictState;

/// Começa a carregar o dicionário da pasta de dados (o Escritório chama ao
/// entrar). Nos testes unitários não: cada um usa uma pasta temporária e a
/// abelha carrega quando for aberta.
pub fn preload(a: &Assets) {
    if !cfg!(test) {
        dictload::preload(a.data_dir());
    }
}

const STRIP: [StripButton; 4] = [
    strip_button(
        0x08cd,
        Some(0x08cf),
        (59, 59, 94, 38),
        (WHAT_SPELLER, 1),
        "Pular para",
    ),
    strip_button(
        0x082d,
        Some(0x082f),
        (204, 59, 89, 37),
        (WHAT_SPELLER, 4),
        "Corrigir",
    ),
    strip_button(
        0x0835,
        Some(0x0837),
        (344, 57, 54, 42),
        (WHAT_SPELLER, 5),
        "Trocar",
    ),
    strip_button(
        0x08b6,
        Some(0x08bb),
        (449, 56, 29, 43),
        (WHAT_SPELLER, 9),
        "Embaralhador",
    ),
];

/// Abrir o Trocar / o Embaralhar (`0296` / `0297`).
const SND_REPLACE: u16 = 0x0296;
const SND_SCRAMBLE: u16 = 0x0297;
/// FEITO (`02ee`).
const SND_DONE: u16 = 0x02ee;
/// MUDAR, UMA e TODAS: trocar no texto (`021c`).
const SND_CHANGE: u16 = 0x021c;
/// Abrir a lista de sinônimos ou de anagramas (`021f`).
const SND_LIST: u16 = 0x021f;

/// Avisos do Rex (`WHAT 0805`).
const ALERT_NOT_FOUND: usize = 0;
const ALERT_SPELL_DONE: usize = 2;
const ALERT_SEARCH_DONE: usize = 8;
/// Larguras e textos de reserva dos avisos.
const ALERTS: [(usize, i32, &str); 3] = [
    (
        ALERT_NOT_FOUND,
        223,
        "Não consegui achar a palavra que você digitou.",
    ),
    (ALERT_SPELL_DONE, 210, "Já acabei de corrigir."),
    (
        ALERT_SEARCH_DONE,
        225,
        "Já procurei no seu documento inteiro.",
    ),
];
/// "(Sem sugestões)" (`STR# 0808`).
const NO_SUGGESTIONS: &str = "(Sem sugestões)";
/// Sugestões pedidas ao dicionário.
const MAX_SUGGESTIONS: usize = 12;

/// Os quatro painéis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Find,
    Spell,
    Replace,
    Scramble,
}

impl Kind {
    const ALL: [Kind; 4] = [Kind::Find, Kind::Spell, Kind::Replace, Kind::Scramble];

    /// Quanto o painel empurra a página para baixo.
    pub fn depth(self) -> i32 {
        match self {
            Kind::Find => 50,
            Kind::Spell => 152,
            Kind::Replace | Kind::Scramble => 95,
        }
    }
}

/// Botões dos painéis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Btn {
    Find,
    Done,
    Change,
    Skip,
    Similar,
    One,
    Other,
    All,
    Up,
    Down,
}

/// Botão: figura, máscara e onde (medidos em `office_77`..`92`).
struct PanelButton {
    btn: Btn,
    pic: Pic,
    w: i32,
    h: i32,
}

const fn pb(btn: Btn, id: u16, mask: u16, x: i32, y: i32, w: i32, h: i32) -> PanelButton {
    PanelButton {
        btn,
        pic: Pic::masked(id, mask, x, y),
        w,
        h,
    }
}

const FIND_BUTTONS: [PanelButton; 2] = [
    pb(Btn::Find, 0x0be5, 0x0be7, 487, 115, 70, 34),
    pb(Btn::Done, 0x0c55, 0x0c57, 556, 119, 76, 27),
];
const SPELL_BUTTONS: [PanelButton; 5] = [
    pb(Btn::Change, 0x0829, 0x082b, 25, 212, 82, 33),
    pb(Btn::Skip, 0x0c51, 0x0c53, 124, 212, 83, 33),
    pb(Btn::Done, 0x0c55, 0x0c57, 224, 215, 76, 27),
    pb(Btn::Up, 0x02db, 0x02dd, 416, 234, 12, 12),
    pb(Btn::Down, 0x02df, 0x02e1, 524, 234, 12, 12),
];
const REPLACE_BUTTONS: [PanelButton; 5] = [
    pb(Btn::Similar, 0x0bfd, 0x0bff, 370, 122, 214, 20),
    pb(Btn::One, 0x0bf1, 0x0bf7, 332, 161, 38, 32),
    pb(Btn::Other, 0x0be9, 0x0bef, 379, 163, 85, 28),
    pb(Btn::All, 0x0bf9, 0x0bfb, 473, 160, 63, 34),
    pb(Btn::Done, 0x0c55, 0x0c57, 545, 164, 76, 27),
];
const SCRAMBLE_BUTTONS: [PanelButton; 4] = [
    pb(Btn::Similar, 0x0c4d, 0x0c4f, 350, 122, 253, 20),
    pb(Btn::One, 0x0bf1, 0x0bf7, 350, 161, 38, 32),
    pb(Btn::Other, 0x0be9, 0x0bef, 415, 163, 85, 28),
    pb(Btn::Done, 0x0c55, 0x0c57, 527, 164, 76, 27),
];

/// Faixas-título (barras azuis) em `(x, y)`.
const FIND_TITLE: (u16, i32, i32) = (0x094f, 9, 112);
const SPELL_TITLES: [(u16, i32, i32); 3] = [(0x0943, 8, 112), (0x0947, 8, 157), (0x094b, 321, 112)];
const REPLACE_TITLES: [(u16, i32, i32); 2] = [(0x093b, 8, 112), (0x093f, 8, 157)];

/// Campos (área branca, ou cinza-claro no "Não conheço").
const FIND_FIELD: Rect = (9, 128, 473, 24);
const TOP_FIELD: Rect = (9, 128, 308, 24);
const BOTTOM_FIELD: Rect = (9, 173, 308, 24);

/// Lista MAIS ESCOLHAS do Corrigir: 5 linhas a partir de (322,128).
const SPELL_ROWS: usize = 5;
const SPELL_LIST: (i32, i32, i32) = (322, 128, 309);
/// Listas suspensas: 92 de largura sob o botão, a partir de y 142.
const DROP_Y: i32 = 142;
const DROP_W: i32 = 92;
const DROP_ROWS: usize = 8;

impl Kind {
    fn buttons(self) -> &'static [PanelButton] {
        match self {
            Kind::Find => &FIND_BUTTONS,
            Kind::Spell => &SPELL_BUTTONS,
            Kind::Replace => &REPLACE_BUTTONS,
            Kind::Scramble => &SCRAMBLE_BUTTONS,
        }
    }

    /// Células (borda preta) e o fundo de cada uma.
    fn cells(self) -> &'static [(Rect, [u8; 3])] {
        match self {
            Kind::Find => &[((8, 111, 475, 42), WHITE), ((486, 111, 146, 42), GRAY)],
            Kind::Spell => &[
                ((8, 111, 310, 42), LIGHT),
                ((8, 156, 310, 42), WHITE),
                ((8, 201, 310, 54), GRAY),
                ((321, 111, 311, 144), GRAY),
            ],
            Kind::Replace | Kind::Scramble => &[
                ((8, 111, 310, 42), WHITE),
                ((8, 156, 310, 42), WHITE),
                ((321, 111, 311, 42), GRAY),
                ((321, 156, 311, 42), GRAY),
            ],
        }
    }

    fn titles(self) -> &'static [(u16, i32, i32)] {
        match self {
            Kind::Find => std::slice::from_ref(&FIND_TITLE),
            Kind::Spell => &SPELL_TITLES,
            Kind::Replace | Kind::Scramble => &REPLACE_TITLES,
        }
    }

    /// Campos editáveis, na ordem de `Panel::fields`.
    fn fields(self) -> &'static [Rect] {
        match self {
            Kind::Find => std::slice::from_ref(&FIND_FIELD),
            Kind::Spell => &[TOP_FIELD, BOTTOM_FIELD],
            Kind::Replace | Kind::Scramble => &[TOP_FIELD, BOTTOM_FIELD],
        }
    }

    /// O campo `k` aceita digitação? (o "Não conheço" é só leitura)
    fn editable(self, k: usize) -> bool {
        !(self == Kind::Spell && k == 0)
    }

    fn sound(self) -> u16 {
        match self {
            Kind::Find | Kind::Spell => SND_OPTION,
            Kind::Replace => SND_REPLACE,
            Kind::Scramble => SND_SCRAMBLE,
        }
    }

    /// Balão ao abrir; ao clicar no POR ISTO; depois de escolher a troca.
    fn helps(self) -> [usize; 3] {
        match self {
            Kind::Find => [1, 2, 2],
            Kind::Spell => [4, 4, 4],
            Kind::Replace => [5, 6, 8],
            Kind::Scramble => [9, 10, 12],
        }
    }
}

/// Correção em andamento.
#[derive(Debug, Clone, Default)]
struct Spell {
    /// Onde a varredura começou (ela volta ao começo do documento e para aí).
    origin: Pos,
    wrapped: bool,
    /// Palavra desconhecida mostrada agora.
    current: Option<Range>,
    /// Esperando o dicionário carregar para procurar a partir daqui.
    waiting: Option<Pos>,
}

/// Um painel aberto.
struct Panel {
    kind: Kind,
    fields: Vec<Field>,
    focus: usize,
    /// MAIS ESCOLHAS (Corrigir) ou a lista suspensa (Trocar, Embaralhar).
    list: List,
    dropdown: bool,
    spell: Spell,
    /// A próxima busca recomeça do começo (depois do `0805[8]`).
    from_start: bool,
    /// Balões já mostrados (POR ISTO, troca escolhida).
    helped: [bool; 2],
}

impl Panel {
    fn new(kind: Kind) -> Panel {
        Panel {
            kind,
            fields: vec![Field::default(); kind.fields().len()],
            focus: if kind == Kind::Spell { 1 } else { 0 },
            list: List::default(),
            dropdown: false,
            spell: Spell::default(),
            from_start: false,
            helped: [false; 2],
        }
    }

    fn button_at(&self, x: i32, y: i32) -> Option<Btn> {
        self.kind
            .buttons()
            .iter()
            .find(|b| inside((b.pic.x, b.pic.y, b.w, b.h), x, y))
            .map(|b| b.btn)
    }

    fn dropdown_geo(&self) -> (i32, i32, i32) {
        let x = self.kind.buttons()[0].pic.x;
        (x + 1, DROP_Y + 1, DROP_W - 2)
    }

    fn dropdown_rect(&self) -> Rect {
        let rows = self.list.items.len().clamp(1, DROP_ROWS) as i32;
        (self.kind.buttons()[0].pic.x, DROP_Y, DROP_W, rows * ROW + 1)
    }
}

/// Aviso do Rex aberto (clique fecha): o registro do `WHAT 0805`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Alert {
    record: usize,
    /// Fechar o painel junto (fim da correção).
    close_panel: bool,
}

impl Alert {
    /// O balão, no meio da vista abaixo do painel.
    fn balloon(&self, a: &Assets, depth: i32) -> TextBalloon {
        let (_, w, text) = ALERTS
            .into_iter()
            .find(|(n, ..)| *n == self.record)
            .unwrap_or(ALERTS[0]);
        let (x, y) = (PAGE.0 + (PAGE.2 - w) / 2, PAGE.1 + depth + 40);
        crate::scenes::writer_ideas::rex_alert(a, self.record, (x, y), w, text, &[])
    }
}

#[derive(Default)]
pub struct SpellerStrip {
    panel: Option<Panel>,
    alert: Option<Alert>,
    /// Botão apertado, que age se o mouse for solto sobre ele.
    pressed: Option<Pressed>,
    /// Rolagem da vista antes de abrir o painel.
    saved_scroll: Option<(i32, i32)>,
    /// Rolagem vertical da vista com o painel aberto (pode ser negativa:
    /// o alto da folha desce para baixo do painel).
    view: i32,
    /// Dicionário fixo (testes); senão o carregado em segundo plano.
    dict: Option<Arc<Dictionary>>,
    /// Pasta de dados (para o [`ToolStrip::tick`] consultar a carga).
    data: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pressed {
    Strip(usize),
    Panel(Btn),
}

impl SpellerStrip {
    /// Abelha com um dicionário já pronto (testes, ferramentas).
    pub fn with_dictionary(d: Arc<Dictionary>) -> SpellerStrip {
        SpellerStrip {
            dict: Some(d),
            ..SpellerStrip::default()
        }
    }

    /// Painel aberto.
    pub fn kind(&self) -> Option<Kind> {
        self.panel.as_ref().map(|p| p.kind)
    }

    /// Texto do campo `k` do painel aberto.
    pub fn field(&self, k: usize) -> Option<&str> {
        self.panel
            .as_ref()
            .and_then(|p| p.fields.get(k))
            .map(|f| f.text.as_str())
    }

    /// Itens da lista do painel (MAIS ESCOLHAS ou a suspensa).
    pub fn list(&self) -> Vec<String> {
        self.panel
            .as_ref()
            .map(|p| p.list.items.clone())
            .unwrap_or_default()
    }

    /// Registro do `WHAT 0805` do aviso do Rex aberto.
    pub fn alert(&self) -> Option<usize> {
        self.alert.map(|a| a.record)
    }

    /// O dicionário, se já carregou. Sem carga começada (Escritório aberto
    /// sem passar por [`preload`]), começa agora.
    fn dictionary(&mut self, a: Option<&Assets>) -> DictState {
        if let Some(d) = &self.dict {
            return DictState::Ready(d.clone());
        }
        if let Some(a) = a {
            self.data = Some(a.data_dir().to_path_buf());
        }
        let Some(data) = self.data.clone() else {
            return DictState::NotStarted;
        };
        match dictload::get(&data) {
            DictState::NotStarted => {
                dictload::preload(&data);
                DictState::Loading
            }
            s => s,
        }
    }

    // -----------------------------------------------------------------
    // Vista

    fn depth(&self) -> i32 {
        self.panel.as_ref().map_or(0, |p| p.kind.depth())
    }

    /// Repõe a rolagem do painel (as edições do `Editor` limitam a rolagem
    /// a valores positivos) e rola o mínimo para a seleção aparecer abaixo
    /// do painel.
    fn reveal(&mut self, ed: &mut Editor) {
        let depth = self.depth();
        let l = ed.layout();
        let r = ed.sel.range();
        let (a, b) = (l.caret(r.start), l.caret(r.end));
        let top = PAGE.1 + depth + 16;
        let bottom = PAGE.1 + PAGE.3 - 16;
        ed.scroll.1 = self.view;
        let (_, ya) = ed.to_screen(a.page, a.x, a.top);
        let (_, yb) = ed.to_screen(b.page, b.x, b.top + b.height);
        if ya < top {
            self.view -= top - ya;
        } else if yb > bottom {
            self.view += (yb - bottom).min(ya - top);
        }
        let pages = l.page_count() as i32;
        let step = i32::from(ed.doc().dop.page_height) + PAGE_GAP;
        let max = (pages * step - PAGE_GAP - PAGE.3).max(0);
        self.view = self.view.clamp(-depth, max.max(-depth));
        ed.scroll.1 = self.view;
    }

    fn open(&mut self, kind: Kind, ed: &mut Editor, cx: &mut Cx) {
        cx.sound(kind.sound());
        cx.help((WHAT_SPELLER, kind.helps()[0]));
        let mut p = Panel::new(kind);
        match kind {
            Kind::Find => {}
            Kind::Spell => {
                p.spell.origin = ed.sel.range().start;
            }
            Kind::Replace | Kind::Scramble => {
                let sel = ed.sel.range();
                let word = search::word_at_caret(ed.doc(), sel);
                if sel.is_empty()
                    && let Some((s, e, _)) = search::words_in(ed.doc(), sel.start.para)
                        .into_iter()
                        .find(|(s, e, _)| *s <= sel.start.index && sel.start.index <= *e)
                {
                    // a palavra do cursor fica selecionada no texto
                    let para = sel.start.para;
                    ed.sel = Selection::new(Pos::new(para, s), Pos::new(para, e));
                }
                p.fields[0] = Field::with(&word);
            }
        }
        if self.saved_scroll.is_none() {
            self.saved_scroll = Some(ed.scroll);
        }
        let (_, sy) = self.saved_scroll.unwrap_or_default();
        self.view = sy - kind.depth();
        self.panel = Some(p);
        if kind == Kind::Spell {
            let from = ed.sel.range().start;
            self.spell_from(ed, cx, from);
        }
        self.reveal(ed);
    }

    fn close(&mut self, ed: &mut Editor) {
        self.panel = None;
        self.alert = None;
        self.pressed = None;
        if let Some((sx, sy)) = self.saved_scroll.take() {
            // a vista volta a começar logo abaixo da faixa
            let dy = self.view - (sy - self.depth());
            ed.scroll = (sx.max(ed.scroll.0.min(sx)), sy);
            ed.scroll_by(0, dy);
        }
        self.view = 0;
    }

    fn show_alert(&mut self, record: usize, close_panel: bool) {
        self.alert = Some(Alert {
            record,
            close_panel,
        });
    }

    // -----------------------------------------------------------------
    // Pular para, Trocar, Embaralhar

    /// Acha a próxima ocorrência do campo 0 depois da seleção e a seleciona.
    /// `false` se não achou (o aviso já foi aberto).
    fn find_next(&mut self, ed: &mut Editor, cx: &mut Cx, after_selection: bool) -> bool {
        let Some(p) = self.panel.as_mut() else {
            return false;
        };
        let text = p.fields[0].text.trim().to_string();
        if text.is_empty() {
            return false;
        }
        let partial = cx.ctrl;
        let sel = ed.sel.range();
        let from = if std::mem::take(&mut p.from_start) {
            ed.doc().start()
        } else if after_selection {
            sel.end
        } else {
            sel.start
        };
        match search::find_next(ed.doc(), from, &text, partial) {
            Some(r) => {
                ed.sel = Selection::new(r.start, r.end);
                self.reveal(ed);
                true
            }
            None => {
                let any = !search::find_all(ed.doc(), &text, partial).is_empty();
                if any {
                    p.from_start = true;
                }
                cx.close_help();
                self.show_alert(
                    if any {
                        ALERT_SEARCH_DONE
                    } else {
                        ALERT_NOT_FOUND
                    },
                    false,
                );
                false
            }
        }
    }

    /// UMA: troca a seleção se ela for o que se procura e acha a próxima.
    fn replace_one(&mut self, ed: &mut Editor, cx: &mut Cx) {
        let Some(p) = self.panel.as_ref() else { return };
        let (find, by) = (
            p.fields[0].text.trim().to_string(),
            p.fields[1].text.clone(),
        );
        if find.is_empty() {
            return;
        }
        let partial = cx.ctrl;
        let sel = ed.sel.range();
        if search::is_match(ed.doc(), sel, &find, partial) {
            cx.sound(SND_CHANGE);
            let found = search::text_of(ed.doc(), sel);
            let by = search::match_case(&by, &found);
            ed.edit(|doc, s| {
                let r = search::replace(doc, sel, &by);
                *s = Selection::at(r.end);
            });
            self.reveal(ed);
            self.find_quiet(ed, partial);
        } else {
            cx.sound(SND_OPTION);
            self.find_next(ed, cx, false);
        }
    }

    /// Depois de trocar: seleciona a próxima ocorrência, sem aviso se não
    /// houver mais.
    fn find_quiet(&mut self, ed: &mut Editor, partial: bool) {
        let Some(p) = self.panel.as_ref() else { return };
        let find = p.fields[0].text.trim().to_string();
        if let Some(r) = search::find_next(ed.doc(), ed.sel.range().end, &find, partial) {
            ed.sel = Selection::new(r.start, r.end);
            self.reveal(ed);
        }
    }

    fn replace_all(&mut self, ed: &mut Editor, cx: &mut Cx) {
        let Some(p) = self.panel.as_ref() else { return };
        let (find, by) = (
            p.fields[0].text.trim().to_string(),
            p.fields[1].text.clone(),
        );
        if find.is_empty() {
            return;
        }
        let partial = cx.ctrl;
        if search::find_all(ed.doc(), &find, partial).is_empty() {
            cx.close_help();
            self.show_alert(ALERT_NOT_FOUND, false);
            return;
        }
        cx.sound(SND_CHANGE);
        let n = ed.edit(|doc, s| {
            let n = search::replace_all(doc, &find, &by, partial);
            s.clamp(doc);
            *s = Selection::at(s.caret);
            n
        });
        self.reveal(ed);
        cx.log(format!("Trocar: {n} trocas de \"{find}\" por \"{by}\""));
    }

    /// PALAVRAS PARECIDAS / EMBARALHADAS: abre a lista suspensa.
    fn open_dropdown(&mut self, cx: &mut Cx) {
        let d = match self.dictionary(Some(cx.assets)) {
            DictState::Ready(d) => Some(d),
            _ => None,
        };
        let Some(p) = self.panel.as_mut() else { return };
        let word = p.fields[0].text.trim().to_string();
        if word.is_empty() {
            return;
        }
        let mut items = match (&d, p.kind) {
            (Some(d), Kind::Replace) => d.synonyms(&word),
            (Some(d), _) => d.anagrams(&word),
            (None, _) => Vec::new(),
        };
        if d.is_none() {
            cx.log("Corretor: o dicionário ainda está carregando");
        }
        if items.is_empty() {
            // o original mostra a própria palavra quando não há nenhuma
            items.push(word);
        }
        cx.sound(SND_LIST);
        p.list = List::new(items);
        p.dropdown = true;
    }

    fn choose_dropdown(&mut self, cx: &mut Cx, k: usize) {
        let Some(p) = self.panel.as_mut() else { return };
        let Some(w) = p.list.items.get(k).cloned() else {
            return;
        };
        cx.sound(SND_CHOOSE);
        p.fields[1] = Field::with(&w);
        p.fields[1].all_selected = false;
        p.dropdown = false;
        if !std::mem::replace(&mut p.helped[1], true) {
            cx.help((WHAT_SPELLER, p.kind.helps()[2]));
        }
    }

    // -----------------------------------------------------------------
    // Corrigir

    /// Procura a próxima palavra desconhecida a partir de `from` (ou
    /// espera o dicionário carregar).
    fn spell_from(&mut self, ed: &mut Editor, cx: &mut Cx, from: Pos) {
        match self.dictionary(Some(cx.assets)) {
            DictState::Ready(d) => {
                if !self.spell_with(ed, &d, from) {
                    cx.close_help();
                }
            }
            DictState::Missing => {
                cx.log("Corretor: nenhum dicionário (nem SPELL.DCT nem pt_BR do sistema)");
                cx.close_help();
                self.show_alert(ALERT_SPELL_DONE, true);
            }
            DictState::Loading | DictState::NotStarted => {
                if let Some(p) = self.panel.as_mut() {
                    p.spell.waiting = Some(from);
                }
            }
        }
    }

    /// A varredura com o dicionário pronto; `false` = chegou ao fim (aviso
    /// "Já acabei de corrigir" aberto).
    fn spell_with(&mut self, ed: &mut Editor, dict: &Dictionary, from: Pos) -> bool {
        let Some(p) = self.panel.as_mut() else {
            return false;
        };
        p.spell.waiting = None;
        let known = |w: &str| dict.contains(w);
        let mut found = if p.spell.wrapped {
            search::next_unknown(ed.doc(), from, Some(p.spell.origin), known)
        } else {
            search::next_unknown(ed.doc(), from, None, known)
        };
        if found.is_none() && !p.spell.wrapped {
            p.spell.wrapped = true;
            found = search::next_unknown(ed.doc(), ed.doc().start(), Some(p.spell.origin), known);
        }
        let Some((r, word)) = found else {
            p.spell.current = None;
            p.fields.iter_mut().for_each(|f| *f = Field::default());
            p.list = List::default();
            self.show_alert(ALERT_SPELL_DONE, true);
            return false;
        };
        let sugg = dict.suggestions(&word, MAX_SUGGESTIONS);
        p.spell.current = Some(r);
        p.fields[0] = Field::with(&word);
        p.fields[0].all_selected = false;
        match sugg.first() {
            Some(s) => {
                p.fields[1] = Field::with(s);
                p.list = List::new(sugg.clone());
                p.list.chosen = Some(0);
            }
            None => {
                p.fields[1] = Field::with(&word);
                p.list = List::new(vec![NO_SUGGESTIONS.to_string()]);
                p.list.inert = true;
            }
        }
        p.focus = 1;
        ed.sel = Selection::new(r.start, r.end);
        self.reveal(ed);
        true
    }

    /// MUDAR: troca a palavra pelo campo "O que você acha desta?".
    fn spell_change(&mut self, ed: &mut Editor, cx: &mut Cx) {
        let Some(p) = self.panel.as_mut() else { return };
        let Some(r) = p.spell.current else { return };
        let by = p.fields[1].text.clone();
        cx.sound(SND_CHANGE);
        let new = ed.edit(|doc, s| {
            let n = search::replace(doc, r, &by);
            *s = Selection::at(n.end);
            n
        });
        // o ponto de partida anda junto com o texto do mesmo parágrafo
        let o = &mut p.spell.origin;
        if o.para == r.start.para && o.index >= r.end.index {
            o.index = o.index + new.end.index - r.end.index;
        }
        self.spell_from(ed, cx, new.end);
    }

    fn spell_skip(&mut self, ed: &mut Editor, cx: &mut Cx) {
        let Some(r) = self.panel.as_ref().and_then(|p| p.spell.current) else {
            return;
        };
        cx.sound(SND_OPTION);
        self.spell_from(ed, cx, r.end);
    }

    // -----------------------------------------------------------------
    // Eventos

    fn panel_button(&mut self, b: Btn, ed: &mut Editor, cx: &mut Cx) {
        let Some(kind) = self.kind() else { return };
        if cx.ctrl && matches!(b, Btn::Find | Btn::One | Btn::All) {
            cx.help((WHAT_SPELLER, if b == Btn::Find { 3 } else { 13 }));
        }
        match b {
            Btn::Done => {
                cx.sound(SND_DONE);
                cx.close_help();
                self.close(ed);
            }
            Btn::Find => {
                cx.sound(SND_OPTION);
                self.find_next(ed, cx, true);
            }
            Btn::Change => self.spell_change(ed, cx),
            Btn::Skip => self.spell_skip(ed, cx),
            Btn::Up | Btn::Down => {
                let d = if b == Btn::Up { -1 } else { 1 };
                if let Some(p) = self.panel.as_mut()
                    && p.list.scroll(SPELL_ROWS, d)
                {
                    cx.sound(SND_SWITCH);
                }
            }
            Btn::Similar => self.open_dropdown(cx),
            Btn::One => self.replace_one(ed, cx),
            Btn::Other => {
                cx.sound(SND_OPTION);
                self.find_next(ed, cx, true);
            }
            Btn::All if kind == Kind::Replace => self.replace_all(ed, cx),
            Btn::All => {}
        }
    }

    /// Clique no painel fora dos botões (campos, listas).
    fn panel_click(&mut self, cx: &mut Cx, x: i32, y: i32) {
        let Some(p) = self.panel.as_mut() else { return };
        let kind = p.kind;
        if kind == Kind::Spell
            && let Some(k) = p.list.item_at(SPELL_ROWS, SPELL_LIST, x, y)
        {
            cx.sound(SND_CHOOSE);
            p.list.chosen = Some(k);
            p.fields[1] = Field::with(&p.list.items[k]);
            p.focus = 1;
            return;
        }
        for (k, r) in kind.fields().iter().enumerate() {
            if inside(*r, x, y) && kind.editable(k) {
                let was = p.focus;
                p.focus = k;
                if was == k {
                    p.fields[k].click(cx.assets, r.0 + 2, x);
                }
                if k == 1 && kind != Kind::Spell && !std::mem::replace(&mut p.helped[0], true) {
                    cx.help((WHAT_SPELLER, kind.helps()[1]));
                }
                return;
            }
        }
    }

    fn draw_panel(&self, p: &Panel, f: &mut Frame, a: &Assets) {
        let kind = p.kind;
        widgets::draw_backdrop(f, kind.depth());
        for (r, fill) in kind.cells() {
            widgets::draw_cell(f, *r, *fill);
        }
        for (id, x, y) in kind.titles() {
            Pic::opaque(*id, *x, *y).draw(f, a);
        }
        for b in kind.buttons() {
            b.pic.draw(f, a);
        }
        for (k, r) in kind.fields().iter().enumerate() {
            let fld = &p.fields[k];
            if kind.editable(k) {
                fld.draw(f, a, *r, p.focus == k && self.alert.is_none());
            } else {
                // "Não conheço esta palavra": centrada, só leitura
                let w = widgets::field_width(a, &fld.text);
                widgets::field_text(f, a, r.0 + (r.2 - w) / 2, r.1, &fld.text, BLACK);
            }
        }
        if kind == Kind::Spell {
            p.list.draw(f, a, SPELL_ROWS, SPELL_LIST);
        }
        if p.dropdown {
            let (x, y, w, h) = p.dropdown_rect();
            f.rect(x, y, w, h, BLACK);
            let rows = p.list.items.len().clamp(1, DROP_ROWS);
            p.list.draw(f, a, rows, p.dropdown_geo());
        }
    }
}

impl ToolStrip for SpellerStrip {
    fn id(&self) -> ToolId {
        ToolId::Speller
    }

    fn deactivate(&mut self, ed: &mut Editor, _cx: &mut Cx) {
        self.close(ed);
    }

    fn modal(&self) -> bool {
        self.panel.is_some() || self.alert.is_some()
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if let Some(al) = self.alert.take() {
            // clique em qualquer lugar fecha o aviso
            if al.close_panel {
                self.close(ed);
            }
            return true;
        }
        let Some(p) = self.panel.as_mut() else {
            return match StripButton::find(&STRIP, x, y) {
                Some(k) => {
                    self.pressed = Some(Pressed::Strip(k));
                    true
                }
                None => false,
            };
        };
        if p.dropdown {
            let geo = p.dropdown_geo();
            let rows = p.list.items.len().clamp(1, DROP_ROWS);
            match p.list.item_at(rows, geo, x, y) {
                Some(k) => self.choose_dropdown(cx, k),
                // clicar fora fecha a lista (o original cai no Wine com Esc)
                None => p.dropdown = false,
            }
            return true;
        }
        match p.button_at(x, y) {
            Some(b) => self.pressed = Some(Pressed::Panel(b)),
            None => self.panel_click(cx, x, y),
        }
        true
    }

    fn release(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) {
        match self.pressed.take() {
            Some(Pressed::Strip(k)) if StripButton::find(&STRIP, x, y) == Some(k) => {
                self.open(Kind::ALL[k], ed, cx);
            }
            Some(Pressed::Panel(b))
                if self.panel.as_ref().and_then(|p| p.button_at(x, y)) == Some(b) =>
            {
                self.panel_button(b, ed, cx);
            }
            _ => {}
        }
    }

    fn key(&mut self, ed: &mut Editor, cx: &mut Cx, key: Key, ctrl: bool) -> bool {
        if self.alert.is_some() {
            if matches!(key, Key::Enter | Key::Escape) {
                let al = self.alert.take();
                if al.is_some_and(|a| a.close_panel) {
                    self.close(ed);
                }
            }
            return true;
        }
        let Some(p) = self.panel.as_mut() else {
            return false;
        };
        if p.dropdown {
            if key == Key::Escape {
                p.dropdown = false;
            }
            return true;
        }
        let kind = p.kind;
        match key {
            Key::Escape => {
                cx.sound(SND_DONE);
                self.close(ed);
            }
            Key::Enter => match kind {
                Kind::Find => {
                    cx.ctrl = ctrl;
                    cx.sound(SND_OPTION);
                    self.find_next(ed, cx, true);
                }
                Kind::Spell => self.spell_change(ed, cx),
                Kind::Replace | Kind::Scramble => {
                    if p.focus == 0 {
                        p.focus = 1;
                    } else {
                        self.replace_one(ed, cx);
                    }
                }
            },
            _ if !ctrl => {
                let k = p.focus;
                if kind.editable(k) && p.fields[k].key(key) && kind == Kind::Spell {
                    p.list.chosen = None;
                }
            }
            _ => {}
        }
        // com o painel aberto nenhuma tecla chega ao documento
        true
    }

    fn text(&mut self, _ed: &mut Editor, cx: &mut Cx, s: &str) -> bool {
        if self.alert.is_some() {
            return true;
        }
        let Some(p) = self.panel.as_mut() else {
            return false;
        };
        if p.dropdown || !p.kind.editable(p.focus) {
            return true;
        }
        let k = p.focus;
        let first = p.fields[k].text.is_empty();
        p.fields[k].type_text(s);
        match p.kind {
            Kind::Find if first => cx.help((WHAT_SPELLER, 2)),
            Kind::Spell => p.list.chosen = None,
            _ => {}
        }
        true
    }

    fn tick(&mut self, ed: &mut Editor, _dt: f32) {
        // Corrigir aberto antes de o dicionário terminar de carregar
        let Some(from) = self.panel.as_ref().and_then(|p| p.spell.waiting) else {
            return;
        };
        match self.dictionary(None) {
            DictState::Ready(d) => {
                self.spell_with(ed, &d, from);
            }
            DictState::Missing => self.show_alert(ALERT_SPELL_DONE, true),
            DictState::Loading | DictState::NotStarted => {}
        }
    }

    fn draw_strip(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        StripButton::draw_all(&STRIP, f, a);
    }

    fn draw_front(&self, _ed: &Editor, f: &mut Frame, a: &Assets) {
        if let Some(p) = &self.panel {
            widgets::dim(f, widgets::DIM);
            self.draw_panel(p, f, a);
        }
        if let Some(al) = &self.alert {
            al.balloon(a, self.depth()).draw(f, a);
        }
    }

    fn cursor(&self, _ed: &Editor, x: i32, y: i32) -> Option<CursorRef> {
        let p = self.panel.as_ref()?;
        if self.alert.is_none()
            && !p.dropdown
            && p.kind
                .fields()
                .iter()
                .enumerate()
                .any(|(k, r)| p.kind.editable(k) && inside(*r, x, y))
        {
            return Some(super::tools::cursor(cursors::TEXT));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Effect, Input, MouseButton, Scene};
    use crate::scenes::testutil::empty_assets;
    use crate::ui::center;
    use crate::writer::dict::Variant;

    /// Dicionário Hunspell mínimo (sem tesauro).
    fn tiny_dict() -> Arc<Dictionary> {
        let dir = tempfile::tempdir().unwrap();
        let aff = dir.path().join("t.aff");
        let dic = dir.path().join("t.dic");
        std::fs::write(&aff, "SET UTF-8\nTRY aeoistrnmcdlupgvbfhzjqxç\n").unwrap();
        let words = [
            "o", "e", "comeu", "gato", "gata", "peixe", "na", "casa", "ramo", "amor", "mora",
        ];
        std::fs::write(&dic, format!("{}\n{}\n", words.len(), words.join("\n"))).unwrap();
        Arc::new(Dictionary::from_free(&aff, &dic, None, Variant::Brasil).unwrap())
    }

    struct T {
        _dir: tempfile::TempDir,
        a: Assets,
        ed: Editor,
        bee: SpellerStrip,
    }

    impl T {
        fn new(text: &str) -> T {
            let (dir, a) = empty_assets();
            let mut ed = Editor::blank(None, &a);
            ed.type_text(text);
            T {
                _dir: dir,
                a,
                ed,
                bee: SpellerStrip::with_dictionary(tiny_dict()),
            }
        }

        fn at(&mut self, para: usize, index: usize) {
            self.ed.sel = Selection::at(Pos::new(para, index));
        }

        /// Aperta e solta em `(x, y)`; devolve os sons.
        fn click_ctrl(&mut self, (x, y): (i32, i32), ctrl: bool) -> Vec<u16> {
            let mut fx = Vec::new();
            let mut cx = Cx::new(&mut fx, &self.a);
            cx.ctrl = ctrl;
            self.bee.press(&mut self.ed, &mut cx, x, y);
            self.bee.release(&mut self.ed, &mut cx, x, y);
            sounds(&fx)
        }

        fn click(&mut self, p: (i32, i32)) -> Vec<u16> {
            self.click_ctrl(p, false)
        }

        fn strip(&mut self, k: usize) -> Vec<u16> {
            self.click(center(STRIP[k].rect()))
        }

        fn button(&mut self, b: Btn) -> Vec<u16> {
            self.button_ctrl(b, false)
        }

        fn button_ctrl(&mut self, b: Btn, ctrl: bool) -> Vec<u16> {
            let kind = self.bee.kind().expect("painel aberto");
            let pb = kind.buttons().iter().find(|p| p.btn == b).unwrap();
            self.click_ctrl((pb.pic.x + pb.w / 2, pb.pic.y + pb.h / 2), ctrl)
        }

        fn type_text(&mut self, s: &str) {
            let mut fx = Vec::new();
            let mut cx = Cx::new(&mut fx, &self.a);
            assert!(self.bee.text(&mut self.ed, &mut cx, s));
        }

        fn selected(&self) -> String {
            self.ed.doc().text_range(self.ed.sel.range())
        }

        fn text(&self) -> String {
            self.ed.doc().text()
        }
    }

    fn sounds(fx: &[Effect]) -> Vec<u16> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::PlaySound(crate::assets::SoundRef::Snd(_, id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn pular_para_acha_palavras_inteiras_e_avisa_no_fim() {
        let mut t = T::new("O gato e o aeroporto. Gato porto.");
        t.at(0, 0);
        assert_eq!(t.strip(0), [SND_OPTION]);
        assert_eq!(t.bee.kind(), Some(Kind::Find));
        assert!(t.bee.modal());
        // a página desce 50 pixels
        assert_eq!(t.ed.scroll.1, -50);
        t.type_text("gato");
        assert_eq!(t.bee.field(0), Some("gato"));
        t.button(Btn::Find);
        assert_eq!(t.ed.sel.range(), Range::new(Pos::new(0, 2), Pos::new(0, 6)));
        t.button(Btn::Find);
        assert_eq!(t.selected(), "Gato");
        assert_eq!(t.bee.alert(), None);
        t.button(Btn::Find);
        assert_eq!(t.bee.alert(), Some(ALERT_SEARCH_DONE));
        // clique fecha o aviso; a próxima busca recomeça do começo
        t.click((300, 300));
        assert_eq!(t.bee.alert(), None);
        t.button(Btn::Find);
        assert_eq!(t.ed.sel.range().start, Pos::new(0, 2));
        // só parte de palavra com Ctrl
        let mut t = T::new("O aeroporto.");
        t.at(0, 0);
        t.strip(0);
        t.type_text("porto");
        t.button(Btn::Find);
        assert_eq!(t.bee.alert(), Some(ALERT_NOT_FOUND));
        t.click((300, 300));
        t.button_ctrl(Btn::Find, true);
        assert_eq!(t.selected(), "porto");
        // FEITO fecha e a vista volta
        assert_eq!(t.button(Btn::Done), [SND_DONE]);
        assert_eq!(t.bee.kind(), None);
        assert_eq!(t.ed.scroll, (0, 0));
    }

    #[test]
    fn corrigir_percorre_sugere_e_muda() {
        let mut t = T::new("O gatu comeu o pexe.");
        t.at(0, 10);
        t.strip(1);
        assert_eq!(t.bee.kind(), Some(Kind::Spell));
        assert_eq!(t.ed.scroll.1, -152);
        // a partir do cursor: "pexe" primeiro, depois volta ao começo
        assert_eq!(t.bee.field(0), Some("pexe"));
        assert_eq!(t.selected(), "pexe");
        assert!(t.bee.list().contains(&"peixe".to_string()));
        assert_eq!(t.bee.field(1), Some(t.bee.list()[0].as_str()));
        let k = t.bee.list().iter().position(|w| w == "peixe").unwrap();
        // clicar na lista põe a escolha no campo
        assert_eq!(
            t.click((400, SPELL_LIST.1 + ROW * k as i32 + 5)),
            [SND_CHOOSE]
        );
        assert_eq!(t.bee.field(1), Some("peixe"));
        assert_eq!(t.button(Btn::Change), [SND_CHANGE]);
        assert_eq!(t.text(), "O gatu comeu o peixe.");
        assert_eq!(t.bee.field(0), Some("gatu"));
        // digitar a correção à mão
        t.type_text("gato");
        t.button(Btn::Change);
        assert_eq!(t.text(), "O gato comeu o peixe.");
        assert_eq!(t.bee.alert(), Some(ALERT_SPELL_DONE));
        // o clique no aviso fecha o painel; MUDAR desfaz-se pelo Ovo
        t.click((300, 300));
        assert_eq!(t.bee.kind(), None);
        assert_eq!(t.ed.scroll, (0, 0));
        assert!(t.ed.undo_step());
        assert_eq!(t.text(), "O gatu comeu o peixe.");
    }

    #[test]
    fn corrigir_pula_e_sem_sugestoes() {
        let mut t = T::new("xqzwk gatu");
        t.at(0, 0);
        t.strip(1);
        assert_eq!(t.bee.field(0), Some("xqzwk"));
        assert_eq!(t.bee.list(), [NO_SUGGESTIONS]);
        assert_eq!(t.bee.field(1), Some("xqzwk"));
        t.button(Btn::Skip);
        assert_eq!(t.bee.field(0), Some("gatu"));
        t.button(Btn::Skip);
        assert_eq!(t.bee.alert(), Some(ALERT_SPELL_DONE));
        assert_eq!(t.text(), "xqzwk gatu");
    }

    #[test]
    fn trocar_uma_outra_todas() {
        let mut t = T::new("O gato e o gato. Gato!");
        t.at(0, 4);
        assert_eq!(t.strip(2), [SND_REPLACE]);
        assert_eq!(t.ed.scroll.1, -95);
        // começa com a palavra do cursor, selecionada no texto
        assert_eq!(t.bee.field(0), Some("gato"));
        assert_eq!(t.ed.sel.range(), Range::new(Pos::new(0, 2), Pos::new(0, 6)));
        t.click(center(BOTTOM_FIELD));
        t.type_text("peixe");
        assert_eq!(t.button(Btn::One), [SND_CHANGE]);
        assert_eq!(t.text(), "O peixe e o gato. Gato!");
        assert_eq!(
            t.ed.sel.range(),
            Range::new(Pos::new(0, 12), Pos::new(0, 16))
        );
        // OUTRA pula esta
        t.button(Btn::Other);
        assert_eq!(t.selected(), "Gato");
        t.button(Btn::One);
        assert_eq!(t.text(), "O peixe e o gato. Peixe!");
        // TODAS, de uma vez só (uma ação do Ovo)
        t.button(Btn::All);
        assert_eq!(t.text(), "O peixe e o peixe. Peixe!");
        assert!(t.ed.undo_step());
        assert_eq!(t.text(), "O peixe e o gato. Peixe!");
    }

    #[test]
    fn embaralhar_lista_anagramas() {
        let mut t = T::new("ramo");
        t.at(0, 2);
        assert_eq!(t.strip(3), [SND_SCRAMBLE]);
        assert_eq!(t.bee.kind(), Some(Kind::Scramble));
        assert_eq!(t.button(Btn::Similar), [SND_LIST]);
        assert_eq!(t.bee.list(), ["amor", "mora", "ramo"]);
        // escolher a 1ª linha da lista suspensa
        let (x, y, _) = t.bee.panel.as_ref().unwrap().dropdown_geo();
        assert_eq!(t.click((x + 10, y + 5)), [SND_CHOOSE]);
        assert_eq!(t.bee.field(1), Some("amor"));
        t.button(Btn::One);
        assert_eq!(t.text(), "amor");
        // sem sinônimos (sem tesauro): a lista mostra a própria palavra
        let mut t = T::new("casa");
        t.strip(2);
        t.button(Btn::Similar);
        assert_eq!(t.bee.list(), ["casa"]);
        // clicar fora da lista suspensa só a fecha
        t.click((600, 300));
        assert!(!t.bee.panel.as_ref().unwrap().dropdown);
    }

    #[test]
    fn painel_desenha_e_escurece_a_barra() {
        let mut t = T::new("O gato.");
        t.strip(2);
        let mut f = Frame::new(640, 480);
        f.fill([190, 190, 190]);
        t.bee.draw_front(&t.ed, &mut f, &t.a);
        // pontilhado: x + y par fica cinza 100
        assert_eq!(f.pixel(10, 60)[..3], [100, 100, 100]);
        assert_eq!(f.pixel(11, 60)[..3], [190, 190, 190]);
        // moldura: preto em 106..107, laranja, células, sombra e a linha
        // preta de baixo empurrada 95 pixels
        assert_eq!(f.pixel(320, 106)[..3], BLACK);
        assert_eq!(f.pixel(320, 109)[..3], widgets::ORANGE);
        assert_eq!(f.pixel(318, 120)[..3], widgets::BROWN);
        assert_eq!(f.pixel(600, 140)[..3], GRAY);
        assert_eq!(f.pixel(320, 200)[..3], widgets::BROWN);
        assert_eq!(f.pixel(320, 201)[..3], BLACK);
    }

    #[test]
    fn com_o_painel_aberto_o_texto_vai_para_o_campo() {
        let (_d, a) = empty_assets();
        let mut w = super::super::Writer::new_blank(None, &a);
        let (x, y) = center(ToolId::Speller.def().hit);
        let (bx, by) = center(STRIP[0].rect());
        let input = [
            Input::Click {
                button: MouseButton::Left,
                x,
                y,
            },
            Input::Click {
                button: MouseButton::Left,
                x: bx,
                y: by,
            },
            Input::MouseUp {
                button: MouseButton::Left,
                x: bx,
                y: by,
            },
            Input::Text("gato".into()),
            Input::Key {
                key: Key::A,
                ctrl: false,
                alt: false,
            },
        ];
        w.update(&input, 0.0, &a);
        assert_eq!(w.tools().speller.kind(), Some(Kind::Find));
        assert_eq!(w.tools().speller.field(0), Some("gato"));
        assert!(w.doc().is_empty());
    }
}
