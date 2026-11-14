//! Barra de ferramentas do Escritório: as 10 ferramentas com as figuras
//! (normal e ativa, com máscara) e o que cada uma faz.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Text,
    Stickers,
    Scissors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Select(Tool),
    Undo,
    Unavailable(&'static str),
}

pub(super) struct ToolDef {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) normal: (u16, u16),
    pub(super) active: (u16, u16),
    pub(super) action: Action,
}

pub(super) const TOOLS: [ToolDef; 10] = [
    ToolDef {
        x: 17,
        y: 12,
        w: 39,
        normal: (0x0251, 0x0253),
        active: (0x0252, 0x0253),
        action: Action::Select(Tool::Text),
    },
    ToolDef {
        x: 65,
        y: 12,
        w: 32,
        normal: (0x01ff, 0x0201),
        active: (0x0200, 0x0201),
        action: Action::Select(Tool::Stickers),
    },
    ToolDef {
        x: 110,
        y: 10,
        w: 34,
        normal: (0x0205, 0x0207),
        active: (0x0206, 0x0207),
        action: Action::Unavailable("sons"),
    },
    ToolDef {
        x: 159,
        y: 16,
        w: 46,
        normal: (0x0800, 0x0801),
        active: (0x0800, 0x0801),
        action: Action::Unavailable("página"),
    },
    ToolDef {
        x: 214,
        y: 11,
        w: 43,
        normal: (0x091f, 0x0921),
        active: (0x0920, 0x0921),
        action: Action::Unavailable("inseto"),
    },
    ToolDef {
        x: 266,
        y: 6,
        w: 44,
        normal: (0x0212, 0x0213),
        active: (0x0212, 0x0213),
        action: Action::Undo,
    },
    ToolDef {
        x: 319,
        y: 9,
        w: 44,
        normal: (0x08e8, 0x08e9),
        active: (0x08e8, 0x08e9),
        action: Action::Select(Tool::Scissors),
    },
    ToolDef {
        x: 372,
        y: 9,
        w: 36,
        normal: (0x0219, 0x0223),
        active: (0x0222, 0x0223),
        action: Action::Unavailable("mão"),
    },
    ToolDef {
        x: 417,
        y: 12,
        w: 46,
        normal: (0x0804, 0x0805),
        active: (0x0804, 0x0805),
        action: Action::Unavailable("livro"),
    },
    ToolDef {
        x: 472,
        y: 11,
        w: 42,
        normal: (0x0226, 0x0227),
        active: (0x0226, 0x0227),
        action: Action::Unavailable("imprimir"),
    },
];
