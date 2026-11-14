//! Barra de ferramentas do Escritório e o contrato de cada ferramenta.
//!
//! As 10 ferramentas do original ficam em [`ToolId`], com as figuras
//! (inativa e ativa), o som e o balão do McZee de cada uma em [`TOOLS`]. Cada
//! ferramenta mora num arquivo próprio desta pasta e implementa
//! [`ToolStrip`]: desenha a sua faixa de opções, trata os cliques, o arrasto
//! e o teclado e diz qual cursor usar. O [`Writer`](super::Writer) só escolhe
//! a ferramenta e repassa os eventos (ver `docs/telas/creative-writer.md`).
//!
//! Figuras do `WRITER.PIC`: cada figura colorida vem seguida (na ordem do
//! arquivo) da versão em preto e branco (para vídeo de 16 cores) e, quando a
//! figura não é retangular, da máscara. Por isso a máscara fica **duas**
//! figuras adiante (`020d` cor, `020e` preto e branco, `020f` máscara), e
//! os botões retangulares não têm máscara.

use super::editor::Editor;
use crate::Frame;
use crate::assets::{Assets, CursorRef, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Key};

pub use crate::ui::{Rect, inside};

/// Figura do `WRITER.PIC`.
pub fn pic(id: u16) -> ImageRef {
    ImageRef::Pic(Program::Writer, id)
}

/// Som do `WRITER.SND`.
pub fn snd(id: u16) -> Effect {
    Effect::PlaySound(SoundRef::Snd(Program::Writer, id))
}

/// Cursor do `WRITER.EXE` (ver [`super::cursors`]).
pub fn cursor(id: u16) -> CursorRef {
    CursorRef(Program::Writer, id)
}

/// Figura posta num lugar fixo da tela, com a máscara (se tiver).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pic {
    pub id: u16,
    pub mask: Option<u16>,
    pub x: i32,
    pub y: i32,
}

impl Pic {
    pub const fn masked(id: u16, mask: u16, x: i32, y: i32) -> Pic {
        Pic {
            id,
            mask: Some(mask),
            x,
            y,
        }
    }

    pub const fn opaque(id: u16, x: i32, y: i32) -> Pic {
        Pic {
            id,
            mask: None,
            x,
            y,
        }
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        self.draw_at(f, a, self.x, self.y);
    }

    pub fn draw_at(&self, f: &mut Frame, a: &Assets, x: i32, y: i32) {
        let img = a.image(&pic(self.id));
        match self.mask {
            Some(m) => f.blit_masked(&img, &a.image(&pic(m)), x, y),
            None => f.blit(&img, x, y),
        }
    }
}

/// Bloco e registro de um balão `WHAT` do `WRITER.EXE`.
pub type HelpRef = (u16, usize);

/// Blocos `WHAT` do Escritório.
pub const WHAT_WORD: u16 = 0x0fa0;
pub const WHAT_STICKERS: u16 = 0x0fa1;
pub const WHAT_SOUNDS: u16 = 0x0fa2;
pub const WHAT_REMOVERS: u16 = 0x0fa3;
pub const WHAT_SPELLER: u16 = 0x0fa4;
pub const WHAT_PAGE: u16 = 0x0fa5;
pub const WHAT_VIEWER: u16 = 0x0fa6;
pub const WHAT_PRINT: u16 = 0x0fa7;
pub const WHAT_TRANSFORMERS: u16 = 0x0fa8;
pub const WHAT_EGG: u16 = 0x0fa9;
/// Geral: ligar/desligar o McZee, Caderno, Varinha, mapa, nome, páginas, pasta.
pub const WHAT_GENERAL: u16 = 0x0faa;

/// As 10 ferramentas, na ordem da barra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    /// Palavra (letras, tamanhos, cores, estilos, parágrafo…).
    Word,
    /// Seleciona-adesivos.
    Stickers,
    /// Seleciona-sons.
    Sounds,
    /// Preparação da página (fundo, colunas, margens, nº de página, bordas).
    Page,
    /// Corretor e troca-palavras (abelha).
    Speller,
    /// Ovo desfazer.
    Egg,
    /// Re-movedores (cola e tesoura).
    Removers,
    /// Transformadores (mão com tinta).
    Transformers,
    /// Visor de páginas (livro com lupa).
    Viewer,
    /// Imprimir (torradeira).
    Print,
}

impl ToolId {
    pub const ALL: [ToolId; 10] = [
        ToolId::Word,
        ToolId::Stickers,
        ToolId::Sounds,
        ToolId::Page,
        ToolId::Speller,
        ToolId::Egg,
        ToolId::Removers,
        ToolId::Transformers,
        ToolId::Viewer,
        ToolId::Print,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn def(self) -> &'static ToolDef {
        &TOOLS[self.index()]
    }

    /// Ferramenta sob `(x, y)` na barra de cima.
    pub fn at(x: i32, y: i32) -> Option<ToolId> {
        ToolId::ALL.into_iter().find(|t| inside(t.def().hit, x, y))
    }
}

/// Uma ferramenta da barra: onde clicar, figuras, som e balão.
pub struct ToolDef {
    pub id: ToolId,
    /// Nome do balão do original.
    pub name: &'static str,
    pub hit: Rect,
    pub inactive: Pic,
    pub active: Pic,
    /// Som ao escolher (`0209` Palavra … `0212` Imprimir).
    pub sound: u16,
    /// Balão do McZee ao escolher.
    pub help: HelpRef,
}

/// Faixa de cima: a aba da ferramenta ativa vai de y 5 a 55.
const BAR_Y: i32 = 5;
const BAR_H: i32 = 50;

const fn hit(x0: i32, x1: i32) -> Rect {
    (x0, BAR_Y, x1 - x0 + 1, BAR_H)
}

/// Tabela medida no original (`office.md` §3).
pub const TOOLS: [ToolDef; 10] = [
    ToolDef {
        id: ToolId::Word,
        name: "Ferramenta Palavra",
        hit: hit(12, 59),
        inactive: Pic::masked(0x020d, 0x020f, 17, 12),
        active: Pic::masked(0x0251, 0x0253, 17, 12),
        sound: 0x0209,
        help: (WHAT_WORD, 1),
    },
    ToolDef {
        id: ToolId::Stickers,
        name: "Seleciona-adesivos",
        hit: hit(60, 103),
        inactive: Pic::masked(0x01ff, 0x0201, 65, 12),
        active: Pic::opaque(0x0203, 65, 12),
        sound: 0x020a,
        help: (WHAT_STICKERS, 0),
    },
    ToolDef {
        id: ToolId::Sounds,
        name: "Seleciona-sons",
        hit: hit(104, 154),
        inactive: Pic::masked(0x0205, 0x0207, 110, 10),
        active: Pic::masked(0x0209, 0x020b, 106, 9),
        sound: 0x020b,
        help: (WHAT_SOUNDS, 0),
    },
    ToolDef {
        id: ToolId::Page,
        name: "Preparação da página",
        hit: hit(155, 208),
        inactive: Pic::masked(0x0495, 0x0801, 159, 16),
        active: Pic::masked(0x0907, 0x0909, 159, 9),
        sound: 0x020c,
        help: (WHAT_PAGE, 0),
    },
    ToolDef {
        id: ToolId::Speller,
        name: "Corretor e troca-palavras",
        hit: hit(209, 260),
        inactive: Pic::masked(0x091f, 0x0921, 214, 11),
        active: Pic::masked(0x0937, 0x0939, 214, 21),
        sound: 0x020d,
        help: (WHAT_SPELLER, 0),
    },
    ToolDef {
        id: ToolId::Egg,
        name: "Ovo desfazer",
        hit: hit(261, 314),
        inactive: Pic::masked(0x0211, 0x0213, 266, 6),
        active: Pic::masked(0x0215, 0x0217, 272, 6),
        sound: 0x020e,
        help: (WHAT_EGG, 0),
    },
    ToolDef {
        id: ToolId::Removers,
        name: "Re-movedores",
        hit: hit(315, 366),
        // a figura ativa não aparece no WRITER.PIC: no original a aba
        // ativa mostra a mesma cola e tesoura
        inactive: Pic::masked(0x08e7, 0x08e9, 319, 9),
        active: Pic::masked(0x08e7, 0x08e9, 319, 9),
        sound: 0x020f,
        help: (WHAT_REMOVERS, 0),
    },
    ToolDef {
        id: ToolId::Transformers,
        name: "Transformadores",
        hit: hit(367, 412),
        inactive: Pic::masked(0x0219, 0x0223, 372, 9),
        active: Pic::masked(0x02ad, 0x02af, 372, 9),
        sound: 0x0210,
        help: (WHAT_TRANSFORMERS, 0),
    },
    ToolDef {
        id: ToolId::Viewer,
        name: "Visor de páginas",
        hit: hit(413, 466),
        inactive: Pic::masked(0x0803, 0x0805, 417, 12),
        active: Pic::masked(0x090b, 0x090d, 417, 8),
        sound: 0x0211,
        help: (WHAT_VIEWER, 0),
    },
    ToolDef {
        id: ToolId::Print,
        name: "Imprimir",
        hit: hit(467, 518),
        inactive: Pic::masked(0x0225, 0x0227, 472, 11),
        active: Pic::masked(0x02bb, 0x02bd, 472, 11),
        sound: 0x0212,
        help: (WHAT_PRINT, 0),
    },
];

/// Clicar na ferramenta que já está ativa: reabre o balão (som `02fc`).
pub const SND_TOOL_AGAIN: u16 = 0x02fc;
/// Botão de opção da faixa, rolo, lista ou painel (`0233`).
pub const SND_OPTION: u16 = 0x0233;
/// Item escolhido numa lista ou no menu do McZee (`022d`).
pub const SND_CHOOSE: u16 = 0x022d;
/// Interruptor "Mais escolhas" e setas dos rolos (`021d`).
pub const SND_SWITCH: u16 = 0x021d;

/// Botão de uma faixa de opções: figura, retângulo de clique e balão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StripButton {
    pub pic: Pic,
    /// Largura e altura do clique (a da figura).
    pub w: i32,
    pub h: i32,
    pub help: Option<HelpRef>,
    /// Nome para o registro ("ainda não disponível").
    pub name: &'static str,
}

impl StripButton {
    pub fn rect(&self) -> Rect {
        (self.pic.x, self.pic.y, self.w, self.h)
    }

    /// Botão sob `(x, y)` numa lista de botões.
    pub fn find(buttons: &[StripButton], x: i32, y: i32) -> Option<usize> {
        buttons.iter().position(|b| inside(b.rect(), x, y))
    }

    pub fn draw_all(buttons: &[StripButton], f: &mut Frame, a: &Assets) {
        for b in buttons {
            b.pic.draw(f, a);
        }
    }
}

/// Clique num botão de faixa que ainda não faz nada: som da opção, balão
/// do McZee e registro. É o comportamento das faixas ainda não implementadas.
pub fn unavailable(b: &StripButton, cx: &mut Cx) {
    cx.sound(SND_OPTION);
    if let Some(h) = b.help {
        cx.help(h);
    }
    cx.log(format!("{}: ainda não disponível", b.name));
}

/// Clique numa faixa que ainda só é desenhada: o botão sob o clique dá
/// [`unavailable`]. `true` se acertou um botão.
pub fn press_static(buttons: &[StripButton], cx: &mut Cx, x: i32, y: i32) -> bool {
    match StripButton::find(buttons, x, y) {
        Some(k) => {
            unavailable(&buttons[k], cx);
            true
        }
        None => false,
    }
}

/// Interruptor "Mais escolhas" (página 1 ⇄ 2 da faixa), à direita.
pub const SWITCH: Pic = Pic::masked(0x0229, 0x022b, 508, 59);
pub const SWITCH_RECT: Rect = (508, 59, 20, 30);

/// Botão de faixa com balão `(block, help)`.
pub const fn strip_button(
    id: u16,
    mask: Option<u16>,
    (x, y, w, h): Rect,
    help: HelpRef,
    name: &'static str,
) -> StripButton {
    StripButton {
        pic: Pic { id, mask, x, y },
        w,
        h,
        help: Some(help),
        name,
    }
}

/// Saída de um evento tratado por uma ferramenta: efeitos (sons, troca de
/// cena, cursor…) e o pedido de balão do McZee, que o Writer só mostra se o
/// McZee estiver ligado.
pub struct Cx<'a> {
    pub fx: &'a mut Vec<Effect>,
    pub assets: &'a Assets,
    /// Ctrl apertado (Ctrl+clique muda o sentido de vários botões).
    pub ctrl: bool,
    pub shift: bool,
    help: Option<HelpRef>,
    help_forced: bool,
}

impl<'a> Cx<'a> {
    pub fn new(fx: &'a mut Vec<Effect>, assets: &'a Assets) -> Cx<'a> {
        Cx {
            fx,
            assets,
            ctrl: false,
            shift: false,
            help: None,
            help_forced: false,
        }
    }

    pub fn sound(&mut self, id: u16) {
        self.fx.push(snd(id));
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        self.fx.push(Effect::Log(msg.into()));
    }

    /// Pede o balão do McZee (aparece se a ajuda estiver ligada).
    pub fn help(&mut self, h: HelpRef) {
        self.help = Some(h);
    }

    /// Pede o balão mesmo com a ajuda desligada (os Re-movedores ajudam nas
    /// três primeiras vezes de qualquer jeito).
    pub fn help_even_off(&mut self, h: HelpRef) {
        self.help = Some(h);
        self.help_forced = true;
    }

    /// Fecha o balão do McZee que estiver aberto.
    pub fn close_help(&mut self) {
        self.help = Some((0, usize::MAX));
    }

    /// Balão pedido (e se vale com a ajuda desligada); esvazia o pedido.
    pub(super) fn take_help(&mut self) -> Option<(HelpRef, bool)> {
        let forced = std::mem::take(&mut self.help_forced);
        self.help.take().map(|h| (h, forced))
    }
}

/// Contrato de uma ferramenta do Escritório. Todos os ganchos têm uma
/// versão vazia: uma ferramenta nova começa só com `id` e `draw_strip`.
///
/// Ordem dos cliques (botão apertado): balões e menu do McZee; ferramenta
/// em modo exclusivo ([`ToolStrip::modal`]); barra de ferramentas; nicho do
/// McZee; porta; barra de baixo; [`ToolStrip::press`]; e, se ninguém tratou,
/// clique na página põe o ponto de inserção.
pub trait ToolStrip {
    fn id(&self) -> ToolId;

    /// A ferramenta foi escolhida na barra; `again` = já era a ativa. O som,
    /// a aba e o balão da ferramenta já foram tratados pelo Writer.
    fn activate(&mut self, _ed: &mut Editor, _cx: &mut Cx, _again: bool) {}

    /// Outra ferramenta foi escolhida.
    fn deactivate(&mut self, _ed: &mut Editor, _cx: &mut Cx) {}

    /// Lista, painel ou cavalete aberto: recebe todos os cliques antes da
    /// barra de ferramentas.
    fn modal(&self) -> bool {
        false
    }

    /// Botão apertado em `(x, y)`; `true` = tratado.
    fn press(&mut self, _ed: &mut Editor, _cx: &mut Cx, _x: i32, _y: i32) -> bool {
        false
    }

    /// Mouse movido (com ou sem botão apertado).
    fn motion(&mut self, _ed: &mut Editor, _cx: &mut Cx, _x: i32, _y: i32) {}

    /// Botão solto.
    fn release(&mut self, _ed: &mut Editor, _cx: &mut Cx, _x: i32, _y: i32) {}

    /// Tecla; `true` = tratada (senão vai para o editor de texto).
    fn key(&mut self, _ed: &mut Editor, _cx: &mut Cx, _key: Key, _ctrl: bool) -> bool {
        false
    }

    /// Passagem do tempo (animações).
    fn tick(&mut self, _ed: &mut Editor, _dt: f32) {}

    /// Faixa de opções (y 56..104), por cima do fundo cinza.
    fn draw_strip(&self, _ed: &Editor, _f: &mut Frame, _a: &Assets) {}

    /// Por cima da página e do texto (tiras de filme, listas, painéis).
    fn draw_front(&self, _ed: &Editor, _f: &mut Frame, _a: &Assets) {}

    /// Cursor do mouse em `(x, y)`; `None` = o padrão do Writer.
    fn cursor(&self, _ed: &Editor, _x: i32, _y: i32) -> Option<CursorRef> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_are_in_bar_order_and_hit_tests_do_not_overlap() {
        for (k, t) in TOOLS.iter().enumerate() {
            assert_eq!(t.id.index(), k);
            assert_eq!(ToolId::ALL[k], t.id);
            assert_eq!(t.sound, 0x0209 + k as u16);
            let (x, y, w, h) = t.hit;
            assert_eq!(ToolId::at(x, y), Some(t.id));
            assert_eq!(ToolId::at(x + w - 1, y + h - 1), Some(t.id));
            // a figura fica dentro do retângulo de clique
            assert!(t.inactive.x >= x && t.inactive.x < x + w);
        }
        assert_eq!(ToolId::at(30, 30), Some(ToolId::Word));
        assert_eq!(ToolId::at(280, 30), Some(ToolId::Egg));
        assert_eq!(ToolId::at(330, 30), Some(ToolId::Removers));
        assert_eq!(ToolId::at(125, 30), Some(ToolId::Sounds));
        assert_eq!(ToolId::at(530, 30), None);
        assert_eq!(ToolId::at(30, 70), None);
    }

    #[test]
    fn masks_are_two_pictures_ahead() {
        // cor, preto e branco, máscara: a máscara nunca é a figura seguinte
        for t in &TOOLS {
            for p in [t.inactive, t.active] {
                if let Some(m) = p.mask {
                    assert!(m >= p.id + 2, "{:04x}/{:04x}", p.id, m);
                }
            }
        }
        assert_eq!(TOOLS[0].inactive.id, 0x020d);
        assert_eq!(TOOLS[0].active.id, 0x0251);
    }

    #[test]
    fn help_requests_are_taken_once() {
        let a = crate::scenes::testutil::empty_assets().1;
        let mut fx = Vec::new();
        let mut cx = Cx::new(&mut fx, &a);
        cx.help((WHAT_WORD, 3));
        assert_eq!(cx.take_help(), Some(((WHAT_WORD, 3), false)));
        assert_eq!(cx.take_help(), None);
        cx.help_even_off((WHAT_REMOVERS, 0));
        assert_eq!(cx.take_help(), Some(((WHAT_REMOVERS, 0), true)));
    }
}
