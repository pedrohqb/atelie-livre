//! Interface entre as cenas e a plataforma: entradas, efeitos e a trait `Scene`.

use crate::Frame;
use crate::assets::{Assets, CursorRef, Program, SoundRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Escape,
    Backspace,
    F12,
    Q,
    Other,
}

/// Entrada já em coordenadas lógicas (640×480).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    MouseMove {
        x: i32,
        y: i32,
    },
    /// Botão apertado.
    Click {
        button: MouseButton,
        x: i32,
        y: i32,
    },
    /// Botão solto.
    MouseUp {
        button: MouseButton,
        x: i32,
        y: i32,
    },
    Key {
        key: Key,
        ctrl: bool,
        alt: bool,
    },
    /// Texto digitado (UTF-8), já composto pelo sistema (acentos, teclado morto).
    Text(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneId {
    Splash,
    Street,
    /// Prédio: saguão e andares.
    Building,
    /// Ateliê de Pintura (andar 2), ao sair da pintura.
    Studio,
    Paint,
    /// Galeria de desenhos guardados.
    Gallery,
    Browser,
}

/// Pedidos de uma cena à plataforma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    PlaySound(SoundRef),
    SetCursor(CursorRef),
    ChangeScene(SceneId),
    /// Volta à cena anterior.
    Back,
    Log(String),
    Quit,
}

pub trait Scene {
    fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect>;
    fn draw(&self, frame: &mut Frame, assets: &Assets);
    /// Chamado antes de a cena ser trocada ou descartada e ao fechar o programa.
    fn leave(&mut self) -> Vec<Effect> {
        Vec::new()
    }
}

/// Mão apontando (`CURSOR` 0x0001 de `ARTIST.EXE`).
pub const DEFAULT_CURSOR: CursorRef = CursorRef(Program::Artist, 1);

/// Teclas das cenas para crianças: Ctrl+Q sai; Esc não faz nada.
pub fn kid_keys(input: &[Input]) -> Vec<Effect> {
    input
        .iter()
        .filter_map(|i| match i {
            Input::Key {
                key: Key::Q,
                ctrl: true,
                ..
            } => Some(Effect::Quit),
            _ => None,
        })
        .collect()
}

/// Comportamento comum às cenas para crianças: cursor padrão, som de clique e teclas.
#[derive(Debug, Clone, Default)]
pub struct KidCommon {
    click: Option<SoundRef>,
    cursor_set: bool,
}

impl KidCommon {
    pub fn new(click: Option<SoundRef>) -> KidCommon {
        KidCommon {
            click,
            cursor_set: false,
        }
    }

    pub fn update(&mut self, input: &[Input]) -> Vec<Effect> {
        let mut fx = Vec::new();
        if !self.cursor_set {
            self.cursor_set = true;
            fx.push(Effect::SetCursor(DEFAULT_CURSOR));
        }
        if let Some(snd) = &self.click {
            for i in input {
                if let Input::Click {
                    button: MouseButton::Left,
                    ..
                } = i
                {
                    fx.push(Effect::PlaySound(snd.clone()));
                }
            }
        }
        fx.extend(kid_keys(input));
        fx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_q_quits_escape_does_not() {
        let quit = kid_keys(&[Input::Key {
            key: Key::Q,
            ctrl: true,
            alt: false,
        }]);
        assert_eq!(quit, [Effect::Quit]);
        assert!(
            kid_keys(&[Input::Key {
                key: Key::Escape,
                ctrl: false,
                alt: false
            }])
            .is_empty()
        );
        assert!(
            kid_keys(&[Input::Key {
                key: Key::Q,
                ctrl: false,
                alt: false
            }])
            .is_empty()
        );
    }
}
