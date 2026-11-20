//! Central de Controle do Escritório (W23, Ctrl+Shift+A; `office.md` §6,
//! capturas `office_121`..`124`). No original a tela do jogo some (fica
//! preta) e aparece uma janela do Windows com a barra de menus escondida do
//! `WRITER.EXE` (`MENU 0x3e8`): **Arquivo** (Salvar Como…, Salvar Como
//! Texto…, Opções de Impressão…, Voltar ao Creative Writer, Sobre o Creative
//! Writer…), **Editar** (tudo cinza ali) e **Ferramentas** (Controle de
//! Som…, Decodificar Documento…, Redefinir Máquina de Idéias…). Os comandos
//! com "…" abrem as caixas de diálogo do executável (`DIALOG 0x7d0`,
//! `0xbb8`, `0x1388`, `0x1770`, `0x1b58`).
//!
//! O port desenha a mesma coisa sobre o quadro de 640×480: fundo preto, a
//! janela branca com a barra de menus (geometria medida em
//! `office_122_menu_arquivo`, menos o deslocamento 192,144 da janela do
//! Wine), as listas e as caixas de diálogo no estilo do Windows 3.1 com a
//! geometria do `DIALOG` (1 unidade = 6/4 px na horizontal e 13/8 px na
//! vertical, a MS Sans Serif 8). Os textos vêm de `formats::winres_ui`;
//! sem o executável, os embutidos abaixo.
//!
//! A Central só decide; quem executa ([`Action`]) é o `Writer`.

use crate::Frame;
use crate::assets::{Assets, Program};
use crate::scene::{Input, Key, MouseButton};
use crate::ui::{Pen, Rect, inside};
use formats::winres_ui::{self, ControlClass, MenuItem};

/// Comandos do `MENU 0x3e8` (ids do `WM_COMMAND`).
pub mod cmd {
    pub const NEW: u16 = 0x1f41;
    pub const OPEN: u16 = 0x1f42;
    pub const SAVE_AS: u16 = 0x1f43;
    pub const SAVE_TEXT: u16 = 0x1f44;
    pub const PRINT_OPTIONS: u16 = 0x1f45;
    /// "Voltar ao Creative Writer" e "Sair" (o mesmo comando).
    pub const BACK: u16 = 0x1f46;
    pub const ABOUT: u16 = 0x1f5c;
    pub const SOUND: u16 = 0x1f4d;
    pub const DECODE: u16 = 0x1f4f;
    pub const RESET_IDEAS: u16 = 0x1f50;
}

/// Caixas de diálogo (`DIALOG`) e os botões que importam.
pub mod dlg {
    pub const ABOUT: u16 = 0x07d0;
    pub const SOUND: u16 = 0x0bb8;
    pub const SOUND_ON: u16 = 0x0bba;
    pub const SOUND_OFF: u16 = 0x0bb9;
    pub const DECODE: u16 = 0x1388;
    pub const RESET: u16 = 0x1770;
    pub const RESET_ERASE: u16 = 3;
    pub const PRINT: u16 = 0x1b58;
    pub const PRINT_LOCAL: u16 = 0x1b5a;
    pub const PRINT_GLOBAL: u16 = 0x1b59;
}

/// Habilitados na Central (os outros aparecem cinza, como no original:
/// Novo, Abrir, Sair e todo o Editar).
const ENABLED: [u16; 8] = [
    cmd::SAVE_AS,
    cmd::SAVE_TEXT,
    cmd::PRINT_OPTIONS,
    cmd::BACK,
    cmd::ABOUT,
    cmd::SOUND,
    cmd::DECODE,
    cmd::RESET_IDEAS,
];

/// Janela branca com a barra de menus.
pub const WINDOW: Rect = (4, 227, 632, 50);
/// Linha dos títulos da barra (o destaque azul tem esta altura).
const BAR_Y: i32 = 228;
const BAR_H: i32 = 19;
/// Primeiro título e o espaço entre o texto e a borda do destaque.
const BAR_X: i32 = 5;
const BAR_PAD: i32 = 5;
/// Lista aberta: começa logo abaixo da barra; itens de 17 px, separadores
/// de 9 px; texto a 19 px da borda, atalho em x+151.
const LIST_Y: i32 = 246;
const ITEM_H: i32 = 17;
const SEP_H: i32 = 9;
const TEXT_DX: i32 = 19;
const SHORTCUT_DX: i32 = 151;
const LIST_MIN_W: i32 = 198;

const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];
const BLUE: [u8; 3] = [48, 150, 250];
const BORDER: [u8; 3] = [166, 166, 166];
const GRAY_TEXT: [u8; 3] = [128, 128, 128];
/// Caixas de diálogo do Windows 3.1.
const FACE: [u8; 3] = [192, 192, 192];
const SHADOW: [u8; 3] = [128, 128, 128];
const CAPTION: [u8; 3] = [0, 0, 128];
const CAPTION_H: i32 = 18;

/// O que o `Writer` deve fazer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Voltar ao Creative Writer (fecha a Central).
    Close,
    /// Salvar Como… (fecha a Central e abre o campo de nome).
    SaveAs,
    /// Salvar Como Texto… (`Editor::save_text`).
    SaveText,
    /// Controle de Som: ligar ou desligar os sons.
    Sound(bool),
    /// Redefinir a Máquina de Idéias: apagar as entradas do usuário.
    ResetIdeas,
    /// Opções de Impressão (locais ou globais): o port imprime pelo `lp`,
    /// sem a caixa Impressora.
    PrintOptions { global: bool },
}

/// Item de uma lista.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    pub shortcut: Option<String>,
    /// 0 = separador.
    pub id: u16,
    pub enabled: bool,
}

/// Um título da barra e a sua lista.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub title: String,
    pub items: Vec<Entry>,
}

/// Uma caixa de diálogo já em pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogBox {
    pub id: u16,
    pub caption: String,
    pub rect: Rect,
    /// Textos fixos: retângulo, texto, centralizado.
    pub texts: Vec<(Rect, String, bool)>,
    /// Botões: retângulo, id, texto, padrão (Enter).
    pub buttons: Vec<(Rect, u16, String, bool)>,
}

impl DialogBox {
    /// O botão sob o ponto.
    pub fn button_at(&self, x: i32, y: i32) -> Option<u16> {
        self.buttons
            .iter()
            .find(|(r, ..)| inside(*r, x, y))
            .map(|(_, id, ..)| *id)
    }

    /// O botão padrão (o primeiro, se nenhum for marcado).
    pub fn default_button(&self) -> Option<u16> {
        self.buttons
            .iter()
            .find(|b| b.3)
            .or(self.buttons.first())
            .map(|b| b.1)
    }

    pub fn texts(&self) -> Vec<&str> {
        self.texts.iter().map(|t| t.1.as_str()).collect()
    }
}

/// Unidades de diálogo → pixels (MS Sans Serif 8: 6×13).
fn du(x: i16, y: i16) -> (i32, i32) {
    (i32::from(x) * 6 / 4, i32::from(y) * 13 / 8)
}

/// Menu embutido (sem o executável), na ordem do `MENU 0x3e8`.
fn fallback_menu() -> Vec<Column> {
    let e = |label: &str, shortcut: Option<&str>, id: u16| Entry {
        label: label.to_string(),
        shortcut: shortcut.map(str::to_string),
        id,
        enabled: false,
    };
    let sep = || e("", None, 0);
    vec![
        Column {
            title: "Arquivo".into(),
            items: vec![
                e("Novo", Some("Ctrl+N"), cmd::NEW),
                e("Abrir...", Some("Ctrl+O"), cmd::OPEN),
                e("Salvar Como...", Some("Ctrl+S"), cmd::SAVE_AS),
                e("Salvar Como Texto...", None, cmd::SAVE_TEXT),
                sep(),
                e("Opções de Impressão...", None, cmd::PRINT_OPTIONS),
                sep(),
                e("Voltar ao Creative Writer", None, cmd::BACK),
                e("Sair", Some("Ctrl+Q"), cmd::BACK),
                sep(),
                e("Sobre o Creative Writer...", None, cmd::ABOUT),
            ],
        },
        Column {
            title: "Editar".into(),
            items: vec![
                e("Desfazer", Some("Ctrl+Z"), 0x1f47),
                sep(),
                e("Recortar", Some("Ctrl+X"), 0x1f49),
                e("Copiar", Some("Ctrl+C"), 0x1f4a),
                e("Colar", Some("Ctrl+V"), 0x1f4b),
                e("Limpar", None, 0x1f4c),
            ],
        },
        Column {
            title: "Ferramentas".into(),
            items: vec![
                e("Controle de Som...", None, cmd::SOUND),
                e("Decodificar Documento...", None, cmd::DECODE),
                e("Redefinir Máquina de Idéias...", None, cmd::RESET_IDEAS),
            ],
        },
    ]
}

/// As colunas do menu (do executável, ou as embutidas), com os itens
/// habilitados como na Central do original.
pub fn load_menu(a: &Assets) -> Vec<Column> {
    let parsed = a
        .exe_resource(Program::Writer, "MENU", 0x3e8)
        .and_then(|d| winres_ui::parse_menu(&d).ok())
        .map(|m| {
            m.items
                .iter()
                .filter_map(|it| match it {
                    MenuItem::Popup { items, .. } => Some(Column {
                        title: it.label(),
                        items: items
                            .iter()
                            .map(|c| match c {
                                MenuItem::Command { id, .. } => Entry {
                                    label: c.label(),
                                    shortcut: c.shortcut().map(str::to_string),
                                    id: *id,
                                    enabled: false,
                                },
                                _ => Entry {
                                    label: String::new(),
                                    shortcut: None,
                                    id: 0,
                                    enabled: false,
                                },
                            })
                            .collect(),
                    }),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .filter(|c| c.len() == 3);
    let mut cols = parsed.unwrap_or_else(fallback_menu);
    for c in &mut cols {
        let mut back_seen = false;
        for e in &mut c.items {
            // "Voltar ao Creative Writer" e "Sair" têm o mesmo comando: só o
            // primeiro (Voltar) fica habilitado
            let first_back = e.id == cmd::BACK && !std::mem::replace(&mut back_seen, true);
            e.enabled = ENABLED.contains(&e.id) && (e.id != cmd::BACK || first_back);
        }
    }
    cols
}

/// Caixa embutida: título, linhas de texto e botões (id, rótulo).
type FallbackDialog = (&'static str, Vec<&'static str>, Vec<(u16, &'static str)>);

/// Textos embutidos das caixas (sem o executável).
fn fallback_dialog(id: u16) -> Option<FallbackDialog> {
    Some(match id {
        dlg::ABOUT => (
            "Creative Writer",
            vec![
                "Versão 1.0",
                "20 de outubro de 1995",
                "® 1993-1995  Microsoft Corporation. Direitos reservados.",
                "Olhe a barra de menu para ver comandos úteis.",
            ],
            vec![(1, "OK")],
        ),
        dlg::SOUND => (
            "Controle de Som",
            vec!["Liga e desliga sons enquanto você usa ferramentas e se move de lá para cá."],
            vec![
                (dlg::SOUND_ON, "Ligar sons"),
                (dlg::SOUND_OFF, "Desligar sons"),
            ],
        ),
        dlg::DECODE => (
            "Decodificar o Documento",
            vec![
                "O McZee vai decodificar um documento se você esquecer a senha!",
                "1. Abra o documento.",
                "2. Escolha o Agente Secreto",
                "3. Digite:",
                "Detetive McZee para salvar o dia",
                "4. Segure Control+Shift e clique em OK",
            ],
            vec![(1, "Valeu, McZee")],
        ),
        dlg::RESET => (
            "Redefinir a Máquina de Idéias",
            vec![
                "Redefinir as entradas originais da Máquina de Idéias.",
                "Remove todas as entradas que você colocou.",
                "Cuidado! Você não vai poder recuperar as entradas!",
            ],
            vec![(2, "Deixar como está"), (dlg::RESET_ERASE, "Apagar")],
        ),
        dlg::PRINT => (
            "Opções de Impressão",
            vec![
                "Opções de controle para impressão.",
                "Você vai ver a caixa Impressora.",
                "Escolha as opções e clique em OK.",
                "Definir opções só para este computador:",
                "Definir opções para todos os usuários:",
                "Este comando não imprime o documento. Ele define opções para imprimir no futuro.",
            ],
            vec![
                (dlg::PRINT_LOCAL, "Opções locais"),
                (dlg::PRINT_GLOBAL, "Opções globais"),
                (2, "Deixar como está"),
            ],
        ),
        _ => return None,
    })
}

/// Centraliza a caixa de tamanho `(w, h)` na tela.
fn centered(w: i32, h: i32) -> (i32, i32) {
    ((640 - w) / 2, (480 - h) / 2)
}

/// A caixa de diálogo `id` em pixels: a geometria do `DIALOG` do
/// executável, ou uma montagem simples dos textos embutidos.
pub fn load_dialog(a: &Assets, id: u16) -> Option<DialogBox> {
    if let Some(d) = a
        .exe_resource(Program::Writer, "DIALOG", id)
        .and_then(|b| winres_ui::parse_dialog(&b).ok())
    {
        let (w, h) = du(d.cx, d.cy);
        let h = h + CAPTION_H;
        let (x0, y0) = centered(w, h);
        let at = |c: &winres_ui::Control| {
            let (x, y) = du(c.x, c.y);
            let (cw, ch) = du(c.cx, c.cy);
            (x0 + x, y0 + CAPTION_H + y, cw, ch)
        };
        let texts = d
            .controls
            .iter()
            .filter(|c| c.class == ControlClass::Static && !c.text().is_empty())
            .map(|c| (at(c), c.text().to_string(), c.style & 0x0f == 1))
            .collect();
        let buttons = d
            .controls
            .iter()
            .filter(|c| c.class == ControlClass::Button)
            .map(|c| {
                let label = winres_ui::strip_mnemonic(c.text());
                (at(c), c.id, label, c.is_default_button())
            })
            .collect();
        return Some(DialogBox {
            id,
            caption: d.caption.clone(),
            rect: (x0, y0, w, h),
            texts,
            buttons,
        });
    }
    let (caption, texts, buttons) = fallback_dialog(id)?;
    let w = 330;
    let line = 26;
    let h = CAPTION_H + 12 + line * texts.len() as i32 + 34;
    let (x0, y0) = centered(w, h);
    let texts = texts
        .iter()
        .enumerate()
        .map(|(k, t)| {
            (
                (x0 + 10, y0 + CAPTION_H + 8 + line * k as i32, w - 20, line),
                t.to_string(),
                false,
            )
        })
        .collect();
    let bw = 96;
    let gap = 8;
    let n = buttons.len() as i32;
    let bx = x0 + (w - n * bw - (n - 1) * gap) / 2;
    let buttons = buttons
        .iter()
        .enumerate()
        .map(|(k, (id, t))| {
            (
                (bx + k as i32 * (bw + gap), y0 + h - 30, bw, 22),
                *id,
                t.to_string(),
                k == 0,
            )
        })
        .collect();
    Some(DialogBox {
        id,
        caption: caption.to_string(),
        rect: (x0, y0, w, h),
        texts,
        buttons,
    })
}

pub struct ControlCenter {
    cols: Vec<Column>,
    /// Lista aberta (índice da coluna).
    open: Option<usize>,
    dialog: Option<DialogBox>,
    mouse: (i32, i32),
}

impl ControlCenter {
    pub fn new(a: &Assets) -> ControlCenter {
        ControlCenter {
            cols: load_menu(a),
            open: None,
            dialog: None,
            mouse: (0, 0),
        }
    }

    pub fn columns(&self) -> &[Column] {
        &self.cols
    }

    pub fn open_column(&self) -> Option<usize> {
        self.open
    }

    pub fn dialog(&self) -> Option<&DialogBox> {
        self.dialog.as_ref()
    }

    /// Títulos da barra: retângulo do destaque de cada um.
    pub fn title_rects(&self, a: &Assets) -> Vec<Rect> {
        let pen = Pen::new(a);
        let mut x = BAR_X;
        self.cols
            .iter()
            .map(|c| {
                let w = pen.width(&c.title) + 2 * BAR_PAD;
                let r = (x, BAR_Y, w, BAR_H);
                x += w;
                r
            })
            .collect()
    }

    /// Lista da coluna `k`: retângulo e os retângulos dos itens.
    pub fn list_rects(&self, a: &Assets, k: usize) -> (Rect, Vec<Rect>) {
        let pen = Pen::new(a);
        let col = &self.cols[k];
        let x = self.title_rects(a)[k].0 - 1;
        let w = col
            .items
            .iter()
            .map(|e| {
                TEXT_DX
                    + pen.width(&e.label)
                    + e.shortcut.as_ref().map_or(0, |s| 24 + pen.width(s))
                    + 16
            })
            .max()
            .unwrap_or(0)
            .max(LIST_MIN_W)
            .max(if e_has_shortcut(col) {
                SHORTCUT_DX + 50
            } else {
                0
            });
        let mut y = LIST_Y + 3;
        let items = col
            .items
            .iter()
            .map(|e| {
                let h = if e.id == 0 { SEP_H } else { ITEM_H };
                let r = (x + 3, y, w - 6, h);
                y += h;
                r
            })
            .collect();
        ((x, LIST_Y, w, y + 3 - LIST_Y), items)
    }

    /// Trata uma entrada; `Some(ação)` para o `Writer`.
    pub fn handle(&mut self, i: &Input, a: &Assets) -> Option<Action> {
        match *i {
            Input::MouseMove { x, y } => {
                self.mouse = (x, y);
                // com uma lista aberta, passar sobre outro título a troca
                if self.open.is_some()
                    && self.dialog.is_none()
                    && let Some(k) = self.title_at(a, x, y)
                {
                    self.open = Some(k);
                }
                None
            }
            Input::Click {
                button: MouseButton::Left,
                x,
                y,
            } => {
                self.mouse = (x, y);
                self.click(x, y, a)
            }
            Input::Key { key, .. } => self.key(key),
            _ => None,
        }
    }

    fn key(&mut self, key: Key) -> Option<Action> {
        match key {
            Key::Escape if self.dialog.is_some() => {
                self.dialog = None;
                None
            }
            Key::Escape if self.open.is_some() => {
                self.open = None;
                None
            }
            Key::Escape => Some(Action::Close),
            Key::Enter => {
                let id = self.dialog.as_ref()?.default_button()?;
                self.press_button(id)
            }
            _ => None,
        }
    }

    fn title_at(&self, a: &Assets, x: i32, y: i32) -> Option<usize> {
        self.title_rects(a).iter().position(|r| inside(*r, x, y))
    }

    fn click(&mut self, x: i32, y: i32, a: &Assets) -> Option<Action> {
        if let Some(d) = &self.dialog {
            let id = d.button_at(x, y)?;
            return self.press_button(id);
        }
        if let Some(k) = self.title_at(a, x, y) {
            self.open = if self.open == Some(k) { None } else { Some(k) };
            return None;
        }
        if let Some(k) = self.open {
            let (list, items) = self.list_rects(a, k);
            self.open = None;
            if !inside(list, x, y) {
                return None;
            }
            let e = items
                .iter()
                .position(|r| inside(*r, x, y))
                .map(|j| self.cols[k].items[j].clone())?;
            if !e.enabled || e.id == 0 {
                return None;
            }
            return self.command(e.id, a);
        }
        // fora da janela, sem lista aberta: volta ao Creative Writer
        (!inside(WINDOW, x, y)).then_some(Action::Close)
    }

    /// Comando do menu.
    fn command(&mut self, id: u16, a: &Assets) -> Option<Action> {
        let dialog = match id {
            cmd::SAVE_AS => return Some(Action::SaveAs),
            cmd::SAVE_TEXT => return Some(Action::SaveText),
            cmd::BACK => return Some(Action::Close),
            cmd::PRINT_OPTIONS => dlg::PRINT,
            cmd::ABOUT => dlg::ABOUT,
            cmd::SOUND => dlg::SOUND,
            cmd::DECODE => dlg::DECODE,
            cmd::RESET_IDEAS => dlg::RESET,
            _ => return None,
        };
        self.dialog = load_dialog(a, dialog);
        None
    }

    /// Botão de uma caixa de diálogo: fecha a caixa e diz o que fazer.
    fn press_button(&mut self, id: u16) -> Option<Action> {
        let d = self.dialog.take()?;
        match (d.id, id) {
            (dlg::SOUND, dlg::SOUND_ON) => Some(Action::Sound(true)),
            (dlg::SOUND, dlg::SOUND_OFF) => Some(Action::Sound(false)),
            (dlg::RESET, dlg::RESET_ERASE) => Some(Action::ResetIdeas),
            (dlg::PRINT, dlg::PRINT_LOCAL) => Some(Action::PrintOptions { global: false }),
            (dlg::PRINT, dlg::PRINT_GLOBAL) => Some(Action::PrintOptions { global: true }),
            _ => None,
        }
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let pen = Pen::new(a);
        f.rect(0, 0, 640, 480, BLACK);
        let (x, y, w, h) = WINDOW;
        f.rect(x, y, w, h, WHITE);
        for (k, (c, r)) in self.cols.iter().zip(self.title_rects(a)).enumerate() {
            let (fg, bg) = if self.open == Some(k) {
                (WHITE, BLUE)
            } else {
                (BLACK, WHITE)
            };
            f.rect(r.0, r.1, r.2, r.3, bg);
            pen.text(f, r.0 + BAR_PAD, r.1 + 3, &c.title, fg);
        }
        if let Some(k) = self.open {
            self.draw_list(f, a, k, &pen);
        }
        if let Some(d) = &self.dialog {
            draw_dialog(f, d, &pen);
        }
    }

    fn draw_list(&self, f: &mut Frame, a: &Assets, k: usize, pen: &Pen) {
        let ((x, y, w, h), items) = self.list_rects(a, k);
        f.rect(x, y, w, h, WHITE);
        f.frame_rect(x, y, w, h, 1, BORDER);
        // sombra preta à direita e embaixo, como na captura
        f.rect(x + w, y + 2, 1, h, BLACK);
        f.rect(x + 2, y + h, w, 1, BLACK);
        for (e, r) in self.cols[k].items.iter().zip(items) {
            if e.id == 0 {
                f.rect(r.0 + 1, r.1 + r.3 / 2, r.2 - 2, 1, BORDER);
                continue;
            }
            let hot = e.enabled && inside(r, self.mouse.0, self.mouse.1);
            let fg = if !e.enabled {
                GRAY_TEXT
            } else if hot {
                WHITE
            } else {
                BLACK
            };
            if hot {
                f.rect(r.0, r.1, r.2, r.3, BLUE);
            }
            pen.text(f, x + TEXT_DX, r.1 + 2, &e.label, fg);
            if let Some(s) = &e.shortcut {
                pen.text(f, x + SHORTCUT_DX, r.1 + 2, s, fg);
            }
        }
    }
}

fn e_has_shortcut(c: &Column) -> bool {
    c.items.iter().any(|e| e.shortcut.is_some())
}

/// Botão em relevo do Windows 3.1 (o padrão com a borda mais grossa).
fn draw_button(f: &mut Frame, (x, y, w, h): Rect, label: &str, default: bool, pen: &Pen) {
    f.rect(x, y, w, h, FACE);
    f.frame_rect(x, y, w, h, if default { 2 } else { 1 }, BLACK);
    let t = if default { 2 } else { 1 };
    f.rect(x + t, y + t, w - 2 * t - 1, 1, WHITE);
    f.rect(x + t, y + t, 1, h - 2 * t - 1, WHITE);
    f.rect(x + t, y + h - t - 1, w - 2 * t, 1, SHADOW);
    f.rect(x + w - t - 1, y + t, 1, h - 2 * t, SHADOW);
    pen.centered(f, x + w / 2, y + (h - 12) / 2, label, BLACK);
}

/// Caixa de diálogo: moldura, barra de título azul, textos e botões.
fn draw_dialog(f: &mut Frame, d: &DialogBox, pen: &Pen) {
    let (x, y, w, h) = d.rect;
    f.rect(x, y, w, h, FACE);
    f.frame_rect(x, y, w, h, 1, BLACK);
    f.rect(x + 1, y + 1, w - 2, CAPTION_H - 1, CAPTION);
    f.rect(x, y + CAPTION_H, w, 1, BLACK);
    pen.centered(f, x + w / 2, y + 3, &d.caption, WHITE);
    for (r, t, center) in &d.texts {
        let lines = crate::balloon::wrap_opt(pen.font(), t, r.2.max(8) as u32);
        for (k, l) in lines.iter().enumerate() {
            let ly = r.1 + 13 * k as i32;
            if *center {
                pen.centered(f, r.0 + r.2 / 2, ly, l, BLACK);
            } else {
                pen.text(f, r.0, ly, l, BLACK);
            }
        }
    }
    for (r, _, label, default) in &d.buttons {
        draw_button(f, *r, label, *default, pen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};
    use crate::ui::center;

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
            button: MouseButton::Left,
            x,
            y,
        }
    }

    /// Escolhe o item `label` da coluna `k`.
    fn choose(c: &mut ControlCenter, a: &Assets, k: usize, label: &str) -> Option<Action> {
        let (tx, ty) = center(c.title_rects(a)[k]);
        c.handle(&click(tx, ty), a);
        assert_eq!(c.open_column(), Some(k));
        let (_, items) = c.list_rects(a, k);
        let j = c.columns()[k]
            .items
            .iter()
            .position(|e| e.label == label)
            .unwrap_or_else(|| panic!("{label}"));
        let (x, y) = center(items[j]);
        c.handle(&click(x, y), a)
    }

    fn press(c: &mut ControlCenter, a: &Assets, label: &str) -> Option<Action> {
        let d = c.dialog().expect("caixa aberta");
        let (r, ..) = d
            .buttons
            .iter()
            .find(|b| b.2 == label)
            .unwrap_or_else(|| panic!("{label}: {:?}", d.buttons));
        let (x, y) = center(*r);
        c.handle(&click(x, y), a)
    }

    #[test]
    fn menu_has_three_columns_with_original_enabled_items() {
        let (_d, a) = empty_assets();
        let c = ControlCenter::new(&a);
        let titles: Vec<&str> = c.columns().iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles, ["Arquivo", "Editar", "Ferramentas"]);
        let enabled: Vec<&str> = c.columns()[0]
            .items
            .iter()
            .filter(|e| e.enabled)
            .map(|e| e.label.as_str())
            .collect();
        assert_eq!(
            enabled,
            [
                "Salvar Como...",
                "Salvar Como Texto...",
                "Opções de Impressão...",
                "Voltar ao Creative Writer",
                "Sobre o Creative Writer...",
            ]
        );
        assert!(c.columns()[1].items.iter().all(|e| !e.enabled));
        assert!(c.columns()[2].items.iter().all(|e| e.enabled));
    }

    #[test]
    fn commands_and_dialogs_give_actions() {
        let (_d, a) = empty_assets();
        let mut c = ControlCenter::new(&a);
        assert_eq!(
            choose(&mut c, &a, 0, "Salvar Como..."),
            Some(Action::SaveAs)
        );
        assert_eq!(
            choose(&mut c, &a, 0, "Salvar Como Texto..."),
            Some(Action::SaveText)
        );
        assert_eq!(
            choose(&mut c, &a, 0, "Voltar ao Creative Writer"),
            Some(Action::Close)
        );
        // Sair fica cinza: nada
        assert_eq!(choose(&mut c, &a, 0, "Sair"), None);
        // Controle de Som
        assert_eq!(choose(&mut c, &a, 2, "Controle de Som..."), None);
        assert_eq!(c.dialog().unwrap().id, dlg::SOUND);
        assert_eq!(
            press(&mut c, &a, "Desligar sons"),
            Some(Action::Sound(false))
        );
        assert!(c.dialog().is_none());
        choose(&mut c, &a, 2, "Controle de Som...");
        assert_eq!(press(&mut c, &a, "Ligar sons"), Some(Action::Sound(true)));
        // Redefinir: Deixar como está não faz nada; Apagar apaga
        choose(&mut c, &a, 2, "Redefinir Máquina de Idéias...");
        assert_eq!(press(&mut c, &a, "Deixar como está"), None);
        choose(&mut c, &a, 2, "Redefinir Máquina de Idéias...");
        assert_eq!(press(&mut c, &a, "Apagar"), Some(Action::ResetIdeas));
        // Opções de Impressão
        choose(&mut c, &a, 0, "Opções de Impressão...");
        assert_eq!(
            press(&mut c, &a, "Opções globais"),
            Some(Action::PrintOptions { global: true })
        );
        // Sobre e Decodificar só mostram textos
        choose(&mut c, &a, 0, "Sobre o Creative Writer...");
        assert!(c.dialog().unwrap().texts().contains(&"Versão 1.0"));
        assert_eq!(press(&mut c, &a, "OK"), None);
        choose(&mut c, &a, 2, "Decodificar Documento...");
        assert!(
            c.dialog()
                .unwrap()
                .texts()
                .contains(&"Detetive McZee para salvar o dia")
        );
        // Enter aperta o botão padrão
        let enter = Input::Key {
            key: Key::Enter,
            ctrl: false,
            alt: false,
        };
        assert_eq!(c.handle(&enter, &a), None);
        assert!(c.dialog().is_none());
        // Esc e clique fora da janela voltam
        let esc = Input::Key {
            key: Key::Escape,
            ctrl: false,
            alt: false,
        };
        assert_eq!(c.handle(&esc, &a), Some(Action::Close));
        assert_eq!(c.handle(&click(320, 20), &a), Some(Action::Close));
        let mut f = Frame::new(640, 480);
        c.draw(&mut f, &a);
        assert_eq!(f.pixel(320, 20), [0, 0, 0, 255]);
        assert_eq!(f.pixel(320, 260), [255, 255, 255, 255]);
    }

    /// Com o executável: os textos do `MENU 0x3e8` e dos `DIALOG`, com a
    /// geometria em pixels dentro da tela.
    #[test]
    fn real_assets_menu_and_dialogs_from_the_exe() {
        let Some(a) = assets() else { return };
        let mut c = ControlCenter::new(&a);
        let arquivo: Vec<&str> = c.columns()[0]
            .items
            .iter()
            .map(|e| e.label.as_str())
            .collect();
        assert!(arquivo.contains(&"Salvar Como Texto..."), "{arquivo:?}");
        assert_eq!(c.columns()[0].items[2].shortcut.as_deref(), Some("Ctrl+S"));
        for (k, label) in [
            (0, "Sobre o Creative Writer..."),
            (0, "Opções de Impressão..."),
            (2, "Controle de Som..."),
            (2, "Decodificar Documento..."),
            (2, "Redefinir Máquina de Idéias..."),
        ] {
            choose(&mut c, &a, k, label);
            let d = c.dialog().expect(label).clone();
            let (x, y, w, h) = d.rect;
            assert!(x >= 0 && y >= 0 && x + w <= 640 && y + h <= 480, "{d:?}");
            for (r, ..) in &d.buttons {
                assert!(inside(d.rect, r.0, r.1), "{label}: {r:?}");
            }
            let mut f = Frame::new(640, 480);
            c.draw(&mut f, &a);
            c.handle(
                &Input::Key {
                    key: Key::Escape,
                    ctrl: false,
                    alt: false,
                },
                &a,
            );
        }
        choose(&mut c, &a, 2, "Controle de Som...");
        assert_eq!(c.dialog().unwrap().caption, "Controle de Som");
        let mut f = Frame::new(640, 480);
        c.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "cw_control_center_sound");
        assert_eq!(
            press(&mut c, &a, "Desligar sons"),
            Some(Action::Sound(false))
        );
    }
}
