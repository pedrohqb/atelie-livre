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
    Delete,
    F12,
    /// Ctrl+O abre a pasta nas salas do Creative Writer.
    O,
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
    /// A tecla Shift foi apertada (`true`) ou solta (`false`).
    Shift(bool),
    /// A tecla Ctrl foi apertada (`true`) ou solta (`false`): nas salas,
    /// Ctrl+clique pula o balão do objeto e o mastro anda dois andares.
    Ctrl(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneId {
    /// Escolha do programa.
    Chooser,
    Splash,
    /// Entrada: lista de usuários e cadastro.
    Entry,
    /// História do McZee (álbum de fotos).
    Story,
    /// Imaginópolis: a cidade vista de fora; a porta do museu leva à rua.
    Imaginopolis,
    Street,
    /// Prédio: saguão e andares.
    Building,
    /// Ateliê de Pintura (andar 2), ao sair da pintura.
    Studio,
    /// Andar dos Truques de Desenho (pelo mapa do prédio).
    TricksFloor,
    Paint,
    /// Tela de pintura com um quadro em branco ("Quadro Novo"), pelo clique
    /// duplo no cavalete do Ateliê; o último quadro só muda se este for
    /// guardado.
    PaintNew,
    /// Andar dos Projetos, ao sair de um projeto.
    Projects,
    /// Uma lição dos Truques de Desenho: começa na tela de apresentação
    /// (`TricksBoard`).
    Tricks(crate::scenes::wizard::TrickKind),
    /// A lição na tela de pintura (o "ateliê de treinamento"), a partir do
    /// passo dado do roteiro.
    TricksStudio(crate::scenes::wizard::TrickKind, usize),
    /// Um projeto da Oficina na tela de pintura.
    Project(crate::scenes::projects::ProjectKind),
    /// Uma figura do desenho animado aberta na tela de pintura pelo clique
    /// duplo (por cima do desenho animado, com `Push`); a porta de saída
    /// grava a figura e volta (`Back`).
    FlipbookFigure,
    /// Galeria de desenhos guardados.
    Gallery,
    /// Pasta de trabalhos aberta pela maleta vermelha de uma sala (por cima
    /// da sala; fechar volta a ela).
    Portfolio,
    /// História do McZee aberta pelo livro verde de uma sala (por cima da
    /// sala; Para volta a ela).
    BookStory,
    Browser,
    /// Abertura do Creative Writer.
    WriterSplash,
    /// Escritório do Creative Writer (editor de texto).
    Writer,
    /// Telas do Creative Writer que reaproveitam uma cena do Fine Artist: a
    /// mesma cena, montada com os dados (specs) do Writer. As cenas
    /// compartilhadas não escrevem estes ids à mão: emitem o id do Fine
    /// Artist passado por [`SceneId::in_program`].
    Cw(CwScene),
}

/// Telas do Creative Writer equivalentes às do Fine Artist (ver
/// `docs/arquitetura.md`, "Creative Writer: specs por programa").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CwScene {
    /// Entrada: lista de usuários e cadastro (`Entry` com o `WRITER_ENTRY`).
    Entry,
    /// História do McZee (Biblioteca Maluca).
    Story,
    /// História aberta pelo livro verde da Bolsa Mágica, por cima da sala.
    BookStory,
    Imaginopolis,
    Street,
    /// Prédio no andar dado: saguão (`Floor::Lobby`), Escritório (andar 2,
    /// `Floor::Studio`), Oficina de Projetos (3, `Floor::Projects`), Oficina
    /// de Ideias (4, `Floor::Tricks`) ou o sótão.
    Building(crate::scenes::room::Floor),
    /// Biblioteca do saguão (a Galeria do Writer).
    Library,
    /// Pasta de documentos aberta pela maleta (por cima da sala).
    Portfolio,
    /// Área de escrita com um "Documento Novo" (opção 4 da Entrada,
    /// clique duplo na prancheta).
    NewDocument,
}

impl SceneId {
    /// A tela equivalente no programa `p`. Uma cena usada pelos dois
    /// programas emite `SceneId::Street.in_program(self.program)`: no Fine
    /// Artist é a própria rua; no Writer, `Cw(CwScene::Street)`. Telas sem
    /// equivalente ficam como estão.
    pub fn in_program(self, p: Program) -> SceneId {
        use crate::scenes::room::Floor;
        match p {
            Program::Writer => match self {
                SceneId::Splash => SceneId::WriterSplash,
                SceneId::Entry => SceneId::Cw(CwScene::Entry),
                SceneId::Story => SceneId::Cw(CwScene::Story),
                SceneId::BookStory => SceneId::Cw(CwScene::BookStory),
                SceneId::Imaginopolis => SceneId::Cw(CwScene::Imaginopolis),
                SceneId::Street => SceneId::Cw(CwScene::Street),
                SceneId::Building => SceneId::Cw(CwScene::Building(Floor::Lobby)),
                SceneId::Studio => SceneId::Cw(CwScene::Building(Floor::Studio)),
                SceneId::Projects => SceneId::Cw(CwScene::Building(Floor::Projects)),
                SceneId::TricksFloor => SceneId::Cw(CwScene::Building(Floor::Tricks)),
                SceneId::Gallery => SceneId::Cw(CwScene::Library),
                SceneId::Portfolio => SceneId::Cw(CwScene::Portfolio),
                SceneId::Paint => SceneId::Writer,
                SceneId::PaintNew => SceneId::Cw(CwScene::NewDocument),
                other => other,
            },
            Program::Artist => match self {
                SceneId::WriterSplash => SceneId::Splash,
                SceneId::Writer => SceneId::Paint,
                SceneId::Cw(c) => match c {
                    CwScene::Entry => SceneId::Entry,
                    CwScene::Story => SceneId::Story,
                    CwScene::BookStory => SceneId::BookStory,
                    CwScene::Imaginopolis => SceneId::Imaginopolis,
                    CwScene::Street => SceneId::Street,
                    CwScene::Building(Floor::Studio) => SceneId::Studio,
                    CwScene::Building(Floor::Projects) => SceneId::Projects,
                    CwScene::Building(Floor::Tricks) => SceneId::TricksFloor,
                    CwScene::Building(_) => SceneId::Building,
                    CwScene::Library => SceneId::Gallery,
                    CwScene::Portfolio => SceneId::Portfolio,
                    CwScene::NewDocument => SceneId::PaintNew,
                },
                other => other,
            },
        }
    }

    /// De que programa é a tela (a escolha do programa e o navegador contam
    /// como do Fine Artist).
    pub fn program(self) -> Program {
        match self {
            SceneId::WriterSplash | SceneId::Writer | SceneId::Cw(_) => Program::Writer,
            _ => Program::Artist,
        }
    }
}

/// Pedidos de uma cena à plataforma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    PlaySound(SoundRef),
    /// Toca o som com repetição (bloco `loop`) até `StopLoop`; um laço por vez.
    PlayLoop(SoundRef),
    /// Para a repetição: o som em laço toca o fim e termina.
    StopLoop,
    SetCursor(CursorRef),
    ChangeScene(SceneId),
    /// Abre a cena por cima da atual, que fica guardada como está (sem
    /// atualizar); `Back` volta a ela. Um `ChangeScene` na cena de cima
    /// descarta também as de baixo.
    Push(SceneId),
    /// Volta à cena anterior.
    Back,
    /// Volume dos sons (0–100), do controle de volume do Seleciona-sons.
    SetVolume(u8),
    /// Começa a gravar do microfone (Gravador do Seleciona-sons).
    RecordStart,
    /// Para a gravação e grava o que foi captado neste `.wav` (nada, se não
    /// havia gravação ou microfone).
    RecordStop(std::path::PathBuf),
    /// Para a gravação e descarta o que foi captado (NÃO do Gravador, troca
    /// de ferramenta no meio da gravação).
    RecordCancel,
    /// Usuário escolhido na Entrada (apelido), para o `@` dos balões.
    SetUser(String),
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
    /// A cena é desenhada por cima da de baixo (aberta com `Effect::Push`),
    /// que aparece por trás dela.
    fn overlay(&self) -> bool {
        false
    }
}

/// Quanto um quadro conta, no máximo, no relógio do clique duplo.
pub const CLICK_FRAME_MAX: f32 = 0.1;

/// Passo do relógio do clique duplo: o `dt` do quadro, limitado a
/// [`CLICK_FRAME_MAX`]. Num quadro lento (máquina lenta, uma tela pesada
/// desenhando) os dois cliques chegam em quadros seguidos com um `dt` grande
/// entre eles, embora a criança tenha clicado depressa.
pub fn click_time(dt: f32) -> f32 {
    dt.clamp(0.0, CLICK_FRAME_MAX)
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
    use crate::scenes::room::Floor;

    #[test]
    fn scenes_map_between_programs() {
        let shared = [
            SceneId::Splash,
            SceneId::Entry,
            SceneId::Story,
            SceneId::BookStory,
            SceneId::Imaginopolis,
            SceneId::Street,
            SceneId::Building,
            SceneId::Studio,
            SceneId::Projects,
            SceneId::TricksFloor,
            SceneId::Gallery,
            SceneId::Portfolio,
            SceneId::Paint,
            SceneId::PaintNew,
        ];
        for id in shared {
            assert_eq!(id.in_program(Program::Artist), id);
            let w = id.in_program(Program::Writer);
            assert_eq!(w.program(), Program::Writer, "{id:?}");
            assert_eq!(w.in_program(Program::Artist), id, "{id:?}");
            assert_eq!(w.in_program(Program::Writer), w);
        }
        assert_eq!(
            SceneId::Studio.in_program(Program::Writer),
            SceneId::Cw(CwScene::Building(Floor::Studio))
        );
        assert_eq!(
            SceneId::PaintNew.in_program(Program::Writer),
            SceneId::Cw(CwScene::NewDocument)
        );
        // telas só do Fine Artist ficam como estão
        assert_eq!(
            SceneId::FlipbookFigure.in_program(Program::Writer),
            SceneId::FlipbookFigure
        );
        assert_eq!(SceneId::Browser.program(), Program::Artist);
    }

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
