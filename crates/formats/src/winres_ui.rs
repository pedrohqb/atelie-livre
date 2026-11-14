//! Recursos de interface do Windows 3.x: `MENU`, `DIALOG` e `ACCELERATOR`
//! (ver `docs/formats/winres-ui.md`).
//!
//! Os dois programas trazem uma barra de menus escondida (Arquivo, Editar,
//! Ferramentas), atalhos de teclado e caixas de diálogo pequenas (Sobre,
//! Controle de Som, Opções de Impressão…). Os leitores aqui devolvem os
//! textos e a geometria em unidades de diálogo, sem desenhar nada. Os dados
//! dos recursos no NE vêm com preenchimento no fim, que é ignorado.

use crate::{ErrorKind, Format, Reader, Result, cp1252};

/// Flags de item de menu (`MF_*` do Windows 3.1).
pub mod mf {
    pub const GRAYED: u16 = 0x0001;
    pub const DISABLED: u16 = 0x0002;
    pub const CHECKED: u16 = 0x0008;
    pub const POPUP: u16 = 0x0010;
    pub const MENUBARBREAK: u16 = 0x0020;
    pub const MENUBREAK: u16 = 0x0040;
    pub const END: u16 = 0x0080;
}

/// Item de um menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItem {
    /// Submenu (os títulos da barra: "&Arquivo", "&Editar"…).
    Popup {
        text: String,
        flags: u16,
        items: Vec<MenuItem>,
    },
    /// Comando com id (`WM_COMMAND`).
    Command { id: u16, text: String, flags: u16 },
    /// Linha separadora (id 0, texto vazio).
    Separator,
}

impl MenuItem {
    /// Texto como gravado ("Salvar &Como...\tCtrl+S"); vazio no separador.
    pub fn text(&self) -> &str {
        match self {
            MenuItem::Popup { text, .. } | MenuItem::Command { text, .. } => text,
            MenuItem::Separator => "",
        }
    }

    /// Texto sem o atalho e sem o `&` do mnemônico ("Salvar Como...").
    pub fn label(&self) -> String {
        strip_mnemonic(self.text().split('\t').next().unwrap_or(""))
    }

    /// Atalho escrito depois do tab ("Ctrl+S"), se houver.
    pub fn shortcut(&self) -> Option<&str> {
        self.text().split_once('\t').map(|(_, s)| s)
    }

    pub fn flags(&self) -> u16 {
        match self {
            MenuItem::Popup { flags, .. } | MenuItem::Command { flags, .. } => *flags,
            MenuItem::Separator => 0,
        }
    }

    /// Item cinza (desabilitado) no estado inicial.
    pub fn is_grayed(&self) -> bool {
        self.flags() & mf::GRAYED != 0
    }
}

/// Menu: a barra com os submenus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    pub items: Vec<MenuItem>,
}

impl Menu {
    /// Comando pelo id, procurando em todos os submenus.
    pub fn find(&self, id: u16) -> Option<&MenuItem> {
        fn walk(items: &[MenuItem], id: u16) -> Option<&MenuItem> {
            items.iter().find_map(|it| match it {
                MenuItem::Command { id: i, .. } if *i == id => Some(it),
                MenuItem::Popup { items, .. } => walk(items, id),
                _ => None,
            })
        }
        walk(&self.items, id)
    }

    /// Todos os comandos, em ordem, com o submenu de cada um.
    pub fn commands(&self) -> Vec<(&str, &MenuItem)> {
        let mut out = Vec::new();
        for it in &self.items {
            if let MenuItem::Popup { text, items, .. } = it {
                for c in items {
                    if matches!(c, MenuItem::Command { .. }) {
                        out.push((text.as_str(), c));
                    }
                }
            }
        }
        out
    }
}

/// Tira o `&` que marca o mnemônico (`&&` vira `&`).
pub fn strip_mnemonic(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '&' {
            if it.peek() == Some(&'&') {
                it.next();
                out.push('&');
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn cstr(r: &mut Reader<'_>) -> Result<String> {
    let start = r.pos();
    let rest = r.bytes(r.remaining())?;
    let Some(n) = rest.iter().position(|&b| b == 0) else {
        r.seek(start)?;
        return Err(r.error(ErrorKind::Invalid("string without terminator".into())));
    };
    r.seek(start + n + 1)?;
    Ok(cp1252::decode(&rest[..n]))
}

/// Lê um `MENU` (formato Win16: `u16` versão, `u16` tamanho do cabeçalho,
/// itens com `MF_END` no último de cada nível).
pub fn parse_menu(data: &[u8]) -> Result<Menu> {
    let mut r = Reader::new(data, Format::WinRes);
    let version = r.u16_le()?;
    if version != 0 {
        return Err(r.error(ErrorKind::Unsupported(format!("menu version {version}"))));
    }
    let header = r.u16_le()? as usize;
    r.skip(header)?;
    let items = menu_items(&mut r, 0)?;
    Ok(Menu { items })
}

fn menu_items(r: &mut Reader<'_>, depth: u32) -> Result<Vec<MenuItem>> {
    if depth > 8 {
        return Err(r.error(ErrorKind::Invalid("menu nested too deep".into())));
    }
    let mut items = Vec::new();
    loop {
        let flags = r.u16_le()?;
        let item = if flags & mf::POPUP != 0 {
            let text = cstr(r)?;
            let sub = menu_items(r, depth + 1)?;
            MenuItem::Popup {
                text,
                flags,
                items: sub,
            }
        } else {
            let id = r.u16_le()?;
            let text = cstr(r)?;
            if id == 0 && text.is_empty() {
                MenuItem::Separator
            } else {
                MenuItem::Command { id, text, flags }
            }
        };
        items.push(item);
        if flags & mf::END != 0 {
            return Ok(items);
        }
    }
}

/// Flags de atalho (`ACCELERATOR` Win16).
pub mod accel_flags {
    /// `key` é uma tecla virtual (`VK_*`), não um caractere.
    pub const VIRTKEY: u8 = 0x01;
    pub const NOINVERT: u8 = 0x02;
    pub const SHIFT: u8 = 0x04;
    pub const CONTROL: u8 = 0x08;
    pub const ALT: u8 = 0x10;
    /// Última entrada da tabela.
    pub const END: u8 = 0x80;
}

/// Um atalho de teclado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accelerator {
    pub flags: u8,
    pub key: u16,
    pub command: u16,
}

impl Accelerator {
    /// Para atalhos em ASCII de controle (`^N` = 0x0e, sem `VIRTKEY`), a
    /// letra que se aperta com Ctrl ('N').
    pub fn ctrl_letter(&self) -> Option<char> {
        if self.flags & accel_flags::VIRTKEY == 0 && (1..=26).contains(&self.key) {
            char::from_u32(u32::from(self.key) + 0x40)
        } else {
            None
        }
    }
}

/// Lê uma tabela `ACCELERATOR` (registros de 5 bytes: `u8` flags, `u16`
/// tecla, `u16` comando; o último tem o bit 0x80).
pub fn parse_accelerators(data: &[u8]) -> Result<Vec<Accelerator>> {
    let mut r = Reader::new(data, Format::WinRes);
    let mut out = Vec::new();
    loop {
        let flags = r.u8()?;
        let key = r.u16_le()?;
        let command = r.u16_le()?;
        out.push(Accelerator {
            flags: flags & !accel_flags::END,
            key,
            command,
        });
        if flags & accel_flags::END != 0 {
            return Ok(out);
        }
    }
}

/// Nome ou número (menu, classe e texto de controle).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameOrId {
    Name(String),
    Id(u16),
}

impl NameOrId {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            NameOrId::Name(s) => Some(s),
            NameOrId::Id(_) => None,
        }
    }
}

fn name_or_id(r: &mut Reader<'_>) -> Result<NameOrId> {
    let start = r.pos();
    if r.u8()? == 0xff {
        return Ok(NameOrId::Id(r.u16_le()?));
    }
    r.seek(start)?;
    Ok(NameOrId::Name(cstr(r)?))
}

/// Classe de um controle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlClass {
    Button,
    Edit,
    Static,
    ListBox,
    ScrollBar,
    ComboBox,
    /// Outra classe predefinida (byte com o bit 0x80).
    Predefined(u8),
    /// Classe registrada pelo programa.
    Named(String),
}

/// Estilos de diálogo usados aqui (`DS_*`, `WS_*`).
pub mod ds {
    /// O modelo traz tamanho e nome da fonte.
    pub const SETFONT: u32 = 0x0000_0040;
    pub const CAPTION: u32 = 0x00c0_0000;
}

/// Estilo `SS_CENTER` / `BS_DEFPUSHBUTTON` (ambos 1 no byte baixo).
pub const SS_CENTER: u32 = 0x0001;
pub const BS_DEFPUSHBUTTON: u32 = 0x0001;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Control {
    /// Retângulo em unidades de diálogo.
    pub x: i16,
    pub y: i16,
    pub cx: i16,
    pub cy: i16,
    /// 0xffff nos textos fixos.
    pub id: u16,
    pub style: u32,
    pub class: ControlClass,
    pub text: NameOrId,
    /// Dados de criação (quase sempre vazios).
    pub extra: Vec<u8>,
}

impl Control {
    /// Texto do controle ("" quando é um número de ícone).
    pub fn text(&self) -> &str {
        self.text.as_str().unwrap_or("")
    }

    /// Botão padrão (`BS_DEFPUSHBUTTON`), o que o Enter aperta.
    pub fn is_default_button(&self) -> bool {
        self.class == ControlClass::Button && self.style & 0x0f == BS_DEFPUSHBUTTON
    }
}

/// Modelo de caixa de diálogo (`DLGTEMPLATE` Win16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    pub style: u32,
    pub x: i16,
    pub y: i16,
    pub cx: i16,
    pub cy: i16,
    pub menu: NameOrId,
    pub class: NameOrId,
    pub caption: String,
    /// Tamanho em pontos e nome da fonte, com `DS_SETFONT`.
    pub font: Option<(u16, String)>,
    pub controls: Vec<Control>,
}

impl Dialog {
    /// Controle pelo id.
    pub fn control(&self, id: u16) -> Option<&Control> {
        self.controls.iter().find(|c| c.id == id)
    }

    /// Textos fixos (`STATIC`), em ordem.
    pub fn static_texts(&self) -> Vec<&str> {
        self.controls
            .iter()
            .filter(|c| c.class == ControlClass::Static)
            .map(Control::text)
            .collect()
    }

    /// Botões, em ordem: (id, texto sem mnemônico).
    pub fn buttons(&self) -> Vec<(u16, String)> {
        self.controls
            .iter()
            .filter(|c| c.class == ControlClass::Button)
            .map(|c| (c.id, strip_mnemonic(c.text())))
            .collect()
    }
}

/// Lê um `DIALOG` Win16.
pub fn parse_dialog(data: &[u8]) -> Result<Dialog> {
    let mut r = Reader::new(data, Format::WinRes);
    let style = r.u32_le()?;
    let n = r.u8()?;
    let x = r.i16_le()?;
    let y = r.i16_le()?;
    let cx = r.i16_le()?;
    let cy = r.i16_le()?;
    let menu = name_or_id(&mut r)?;
    let class = name_or_id(&mut r)?;
    let caption = cstr(&mut r)?;
    let font = if style & ds::SETFONT != 0 {
        let pt = r.u16_le()?;
        Some((pt, cstr(&mut r)?))
    } else {
        None
    };
    let mut controls = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let x = r.i16_le()?;
        let y = r.i16_le()?;
        let cx = r.i16_le()?;
        let cy = r.i16_le()?;
        let id = r.u16_le()?;
        let cstyle = r.u32_le()?;
        let start = r.pos();
        let c = r.u8()?;
        let class = if c & 0x80 != 0 {
            match c {
                0x80 => ControlClass::Button,
                0x81 => ControlClass::Edit,
                0x82 => ControlClass::Static,
                0x83 => ControlClass::ListBox,
                0x84 => ControlClass::ScrollBar,
                0x85 => ControlClass::ComboBox,
                _ => ControlClass::Predefined(c),
            }
        } else {
            r.seek(start)?;
            ControlClass::Named(cstr(&mut r)?)
        };
        let text = name_or_id(&mut r)?;
        let len = r.u8()? as usize;
        let extra = r.bytes(len)?.to_vec();
        controls.push(Control {
            x,
            y,
            cx,
            cy,
            id,
            style: cstyle,
            class,
            text,
            extra,
        });
    }
    Ok(Dialog {
        style,
        x,
        y,
        cx,
        cy,
        menu,
        class,
        caption,
        font,
        controls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu_bytes() -> Vec<u8> {
        let mut v = vec![0, 0, 0, 0];
        // popup "&Arquivo"
        v.extend_from_slice(&mf::POPUP.to_le_bytes());
        v.extend_from_slice(b"&Arquivo\0");
        v.extend_from_slice(&mf::GRAYED.to_le_bytes());
        v.extend_from_slice(&0x1f41u16.to_le_bytes());
        v.extend_from_slice(b"&Novo\tCtrl+N\0");
        v.extend_from_slice(&[0, 0, 0, 0, 0]); // separador
        v.extend_from_slice(&mf::END.to_le_bytes());
        v.extend_from_slice(&0x1f5cu16.to_le_bytes());
        v.extend_from_slice(b"&Sobre\0");
        // último popup
        v.extend_from_slice(&(mf::POPUP | mf::END).to_le_bytes());
        v.extend_from_slice(b"&Ferramentas\0");
        v.extend_from_slice(&mf::END.to_le_bytes());
        v.extend_from_slice(&0x1f50u16.to_le_bytes());
        v.extend_from_slice(b"&Redefinir M\xe1quina\0");
        v.extend_from_slice(&[0, 0]); // preenchimento do NE
        v
    }

    #[test]
    fn reads_nested_menu() {
        let m = parse_menu(&menu_bytes()).unwrap();
        assert_eq!(m.items.len(), 2);
        let MenuItem::Popup { text, items, .. } = &m.items[0] else {
            panic!()
        };
        assert_eq!(text, "&Arquivo");
        assert_eq!(items.len(), 3);
        assert_eq!(items[1], MenuItem::Separator);
        let novo = m.find(0x1f41).unwrap();
        assert_eq!(novo.label(), "Novo");
        assert_eq!(novo.shortcut(), Some("Ctrl+N"));
        assert!(novo.is_grayed());
        assert_eq!(m.find(0x1f50).unwrap().label(), "Redefinir Máquina");
        let cmds = m.commands();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[2].0, "&Ferramentas");
    }

    #[test]
    fn menu_without_end_is_an_error() {
        let mut v = menu_bytes();
        v.truncate(20);
        assert!(parse_menu(&v).is_err());
    }

    #[test]
    fn mnemonics() {
        assert_eq!(strip_mnemonic("Opções glo&bais"), "Opções globais");
        assert_eq!(strip_mnemonic("A && B"), "A & B");
    }

    #[test]
    fn reads_accelerators() {
        let v = [
            0x00, 0x0e, 0x00, 0x41, 0x1f, //
            0x09, 0x2e, 0x00, 0x4c, 0x1f, // VIRTKEY|CONTROL, VK_DELETE
            0x80, 0x16, 0x00, 0x4b, 0x1f, 0x00,
        ];
        let a = parse_accelerators(&v).unwrap();
        assert_eq!(a.len(), 3);
        assert_eq!(a[0].ctrl_letter(), Some('N'));
        assert_eq!(a[1].ctrl_letter(), None);
        assert_eq!(a[1].flags, accel_flags::VIRTKEY | accel_flags::CONTROL);
        assert_eq!(a[2].command, 0x1f4b);
        assert_eq!(a[2].flags, 0);
        assert_eq!(a[2].ctrl_letter(), Some('V'));
    }

    #[test]
    fn reads_dialog_with_font_and_controls() {
        let mut v = Vec::new();
        v.extend_from_slice(&(0x90c8_0000u32 | ds::SETFONT).to_le_bytes());
        v.push(2);
        for n in [0i16, 0, 195, 47] {
            v.extend_from_slice(&n.to_le_bytes());
        }
        v.extend_from_slice(b"\0\0Controle de Som\0");
        v.extend_from_slice(&8u16.to_le_bytes());
        v.extend_from_slice(b"MS Sans Serif\0");
        // STATIC
        for n in [6i16, 10, 117, 29] {
            v.extend_from_slice(&n.to_le_bytes());
        }
        v.extend_from_slice(&0xffffu16.to_le_bytes());
        v.extend_from_slice(&0x5002_0000u32.to_le_bytes());
        v.push(0x82);
        v.extend_from_slice(b"Liga e desliga sons\0\0");
        // BUTTON padrão
        for n in [125i16, 26, 67, 14] {
            v.extend_from_slice(&n.to_le_bytes());
        }
        v.extend_from_slice(&0x0bb9u16.to_le_bytes());
        v.extend_from_slice(&0x5001_0001u32.to_le_bytes());
        v.push(0x80);
        v.extend_from_slice(b"&Desligar sons\0");
        v.extend_from_slice(&[2, 0xaa, 0xbb]);
        let d = parse_dialog(&v).unwrap();
        assert_eq!(d.caption, "Controle de Som");
        assert_eq!(d.font, Some((8, "MS Sans Serif".into())));
        assert_eq!(d.menu, NameOrId::Name(String::new()));
        assert_eq!((d.cx, d.cy), (195, 47));
        assert_eq!(d.static_texts(), ["Liga e desliga sons"]);
        assert_eq!(d.buttons(), [(0x0bb9, "Desligar sons".to_string())]);
        let b = d.control(0x0bb9).unwrap();
        assert!(b.is_default_button());
        assert_eq!(b.extra, [0xaa, 0xbb]);
    }

    #[test]
    fn dialog_ordinals_and_named_class() {
        let mut v = Vec::new();
        v.extend_from_slice(&0x8000_0000u32.to_le_bytes());
        v.push(1);
        v.extend_from_slice(&[0; 8]);
        v.extend_from_slice(&[0xff, 0x34, 0x12]); // menu por número
        v.extend_from_slice(b"\0Titulo\0");
        v.extend_from_slice(&[0; 8]);
        v.extend_from_slice(&7u16.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(b"MinhaClasse\0");
        v.extend_from_slice(&[0xff, 0x02, 0x00, 0]); // ícone 2
        let d = parse_dialog(&v).unwrap();
        assert_eq!(d.menu, NameOrId::Id(0x1234));
        assert!(d.font.is_none());
        let c = &d.controls[0];
        assert_eq!(c.class, ControlClass::Named("MinhaClasse".into()));
        assert_eq!(c.text, NameOrId::Id(2));
        assert_eq!(c.text(), "");
    }

    #[test]
    fn truncated_dialog_is_an_error() {
        assert!(parse_dialog(&[0x40, 0, 0, 0, 3, 0, 0]).is_err());
    }
}
