//! Balões das salas do prédio (bloco `WHAT 0x100d` de `ARTIST.EXE`): as
//! dicas que as opções do balão de chegada mostram (no original elas só
//! apontam o objeto, não navegam), os balões de escolha dos objetos da
//! Oficina e dos Truques ("criar novo" / "trabalhar com um que você já
//! criou" / "Ignorar isto"), o menu do McZee e da Maggie ("Você quer me ver
//! menos?") e o "EPA!" da luminária e do "Tchau". Posições e sons medidos no
//! original (ver `docs/telas/salas-baloes.md`).

use crate::assets::{Assets, Program, SoundRef};
use crate::balloon::Balloon;
use crate::scene::SceneId;
use crate::scenes::entry::Entry;
use crate::scenes::entry_spec::QuitSpec;
use crate::scenes::projects::ProjectKind;
use crate::scenes::room_spec::{RoomKit, Tip};
use crate::scenes::rooms_data::ARTIST_KIT;
use crate::scenes::wizard::TrickKind;
use crate::text_balloon::TextBalloon;
use formats::what::Item;

/// Bloco `WHAT` com os balões das salas.
pub const ROOMS_WHAT: u16 = 0x100d;

/// Registros do bloco usados aqui.
pub mod rec {
    /// Dicas das opções do balão de chegada do saguão (Ateliê, Projetos,
    /// Truques, Galeria), apontando o quadro de andares e a seta.
    pub const LOBBY_STUDIO: usize = 2;
    pub const LOBBY_PROJECTS: usize = 3;
    pub const LOBBY_TRICKS: usize = 4;
    pub const LOBBY_GALLERY: usize = 5;
    /// "Se você quiser, pode digitar outro nome!" (plaqueta da parede).
    pub const NAME: usize = 18;
    /// Cavalete: "Para criar um quadro novo • Dê um clique duplo…".
    pub const STUDIO_NEW: usize = 27;
    /// Quadro da parede: "Esta é a última coisa que você salvou…".
    pub const STUDIO_LAST: usize = 28;
    /// "Você quer me ver menos?" (ajuda ligada).
    pub const HELP_OFF: usize = 29;
    /// "Que bom que você me chamou de novo." (ajuda desligada).
    pub const HELP_ON: usize = 30;
    pub const COMIC: usize = 35;
    pub const STICKER: usize = 36;
    pub const PIGTURE: usize = 37;
    /// "Quer ignorar este balão…? • Na próxima vez, segure Ctrl…".
    pub const SKIP_PROJECT: usize = 38;
    pub const BASIC: usize = 44;
    pub const SKIP_BASIC: usize = 45;
    pub const PERSP: usize = 49;
    pub const SKIP_PERSP: usize = 50;
    /// Glossário "tridimensional".
    pub const GLOSS_3D: usize = 51;
    /// Maleta: "A pasta é o lugar em que são guardados…".
    pub const STUDIO_PORTFOLIO: usize = 66;
    /// "Outra coisa?" do Ateliê (mastro, com "Tecla de atalho" e "Descendo?").
    pub const STUDIO_POLE: usize = 67;
    /// "Quer subir DOIS andares?".
    pub const HOTKEY_UP: usize = 68;
    /// "Quer descer pelo mastro de bombeiro?".
    pub const POLE_DOWN: usize = 69;
    /// "Outra coisa?" dos Projetos.
    pub const PROJECTS_POLE: usize = 70;
    /// "Quero fazer outra coisa" dos Truques.
    pub const TRICKS_POLE: usize = 71;
    /// "Quer descer DOIS andares?".
    pub const HOTKEY_DOWN: usize = 72;
}

/// Sons dos balões das salas (ids do `ARTIST.SND`).
pub mod snd {
    /// Clique num objeto que responde com balão (cavalete, vaca, estátua…).
    pub const OBJECT: u16 = 0x21c;
    /// Abre o balão de escolha do objeto.
    pub const PICK: u16 = 0x2f4;
    /// Opção escolhida (bolinha) ou balão fechado por ela.
    pub const CHOICE: u16 = 0x254;
    /// Dica "Ignorar isto" e "Descendo?".
    pub const TIP: u16 = 0x2dd;
    /// Glossário e "Tecla de atalho".
    pub const GLOSSARY: u16 = 0x2e0;
    /// Fala do McZee ao ser clicado.
    pub const MCZEE: u16 = 0x252;
    /// Fala da Maggie ao ser clicada.
    pub const MAGGIE: u16 = 0x226;
    /// Luminária ou "Tchau": abre o "EPA!".
    pub const QUIT: [u16; 2] = [0x2f7, 0x2fe];
    /// Clique fora do "EPA!" (modal).
    pub const REFUSE: u16 = 0x217;
    /// Moldura dos Truques troca de quadro.
    pub const PAINTING: u16 = 0x2d2;
    /// Buraco do rato sem a varinha.
    pub const MOUSE_HOLE: u16 = 0x20a;
    /// Pasta (maleta) aberta pelo balão de escolha.
    pub const PORTFOLIO: u16 = 0x2f1;
    /// Fala do McZee ao usar o quadro de andares do saguão (e ao chegar ao
    /// Ateliê pelo elevador).
    pub const STUDIO_ARRIVAL: u16 = 0x304;
    /// Seta "Galeria" do saguão.
    pub const GALLERY_ARROW: u16 = 0x2fb;
}

pub fn sound(id: u16) -> SoundRef {
    SoundRef::Snd(Program::Artist, id)
}

/// Hotkeys "Tecla de atalho" (figura `21C`) viram ligações como as dicas da
/// lâmpada (`21B`): o clique abre o registro que elas apontam.
pub const PIC_HOTKEY: u16 = 0x21c;

/// Balão `k` do bloco, na posição dada; sem os dados (ou com outro número
/// de opções), o reserva.
pub fn load(assets: &Assets, k: usize, pos: (i32, i32), user: &str, fallback: Balloon) -> Balloon {
    load_in(&ARTIST_KIT, assets, k, pos, user, fallback)
}

/// Como [`load`], com o bloco `WHAT` do kit (Fine Artist ou Creative Writer).
pub fn load_in(
    kit: &RoomKit,
    assets: &Assets,
    k: usize,
    pos: (i32, i32),
    user: &str,
    fallback: Balloon,
) -> Balloon {
    let mut records = assets.dialogs(kit.program, kit.what);
    if let (Some(r), Some((hotkey, tip))) = (records.get_mut(k), kit.hotkey_pic) {
        for i in &mut r.items {
            if let Item::Picture { id, .. } = i
                && *id == hotkey
            {
                *id = tip;
            }
        }
    }
    match Balloon::from_block(&records, k, pos.0, pos.1, user) {
        Some(b)
            if !b.text.is_empty()
                && b.options.len() == fallback.options.len()
                && b.links.len() >= fallback.links.len() =>
        {
            b
        }
        _ => Balloon {
            x: pos.0,
            y: pos.1,
            ..fallback
        },
    }
}

/// Balão reserva (sem posição: quem chama a dá).
pub fn fallback(w: i32, text: &str, options: &[&str], links: &[(usize, &str)]) -> Balloon {
    Balloon {
        x: 0,
        y: 0,
        w,
        text: text.into(),
        options: options.iter().map(|o| o.to_string()).collect(),
        links: links.iter().map(|(k, t)| (*k, t.to_string())).collect(),
        figures: Vec::new(),
        ..Default::default()
    }
}

/// Dicas (sem opções) do `WHAT 100d`: posição medida no original e texto
/// reserva.
pub const ARTIST_TIPS: &[Tip] = &[
    Tip {
        record: rec::LOBBY_STUDIO,
        pos: (34, 111),
        w: 167,
        text: "Clique no andar do Ateliê de Pintura para começar a desenhar por conta própria.\nVocê pode desenhar qualquer coisa aqui, desde um simples rabisco até a Monalisa!",
        links: &[],
    },
    Tip {
        record: rec::LOBBY_PROJECTS,
        pos: (30, 65),
        w: 172,
        text: "Clique na Oficina de Projetos para trabalhar com a Maggie.\nEla está esperando para ajudar você a fazer quadrinhos, botões e adesivos, ou para montar um Desenho Animado!",
        links: &[],
    },
    Tip {
        record: rec::LOBBY_TRICKS,
        pos: (25, 45),
        w: 162,
        text: "Inseguro quanto ao seu talento de desenhista? Não esquenta! Clique aqui para ir até o andar dos Truques de Desenho.\nA Maggie pode te mostrar alguns dos truques que ela aprendeu.",
        links: &[],
    },
    Tip {
        record: rec::LOBBY_GALLERY,
        pos: (95, 182),
        w: 162,
        text: "Clique na seta da Galeria para ver todas as obras que você pintou.\nAli você também pode copiar, jogar fora ou levar os quadros para outros lugares.",
        links: &[],
    },
    Tip {
        record: rec::STUDIO_NEW,
        pos: (130, 233),
        w: 204,
        text: "Para criar um quadro novo\n• Dê um clique duplo na tela em branco.",
        links: &[],
    },
    Tip {
        record: rec::STUDIO_LAST,
        pos: (62, 110),
        w: 166,
        text: "Esta é a última coisa que você salvou no Ateliê de Pintura.\nPara trabalhar neste quadro outra vez\n• Dê um clique duplo nele.",
        links: &[],
    },
    Tip {
        record: rec::STUDIO_PORTFOLIO,
        pos: (266, 178),
        w: 175,
        text: "A pasta é o lugar em que são guardados todos os seus quadros (e os dos outros também).\nPara abrir a pasta\n• Clique nela.",
        links: &[],
    },
    Tip {
        record: rec::STUDIO_POLE,
        pos: (340, 82),
        w: 200,
        text: "Você quer que a Maggie te ajude a fazer uma história em quadrinhos, um botão ou adesivo, ou um Desenho Animado?\n• Clique no mastro de bombeiro para subir até a Oficina de Projetos!\nQuer aprender os Truques de Desenho da Maggie?\n• Use o mastro de bombeiro para subir DOIS andares até o andar dos Truques de Desenho.",
        links: &[
            (rec::HOTKEY_UP, "Tecla de atalho"),
            (rec::POLE_DOWN, "Descendo?"),
        ],
    },
    Tip {
        record: rec::HOTKEY_UP,
        pos: (195, 204),
        w: 240,
        text: "Quer subir DOIS andares?\nSegure Ctrl enquanto clica no mastro de bombeiro.",
        links: &[],
    },
    Tip {
        record: rec::POLE_DOWN,
        pos: (195, 204),
        w: 249,
        text: "Quer descer pelo mastro de bombeiro?\n• Clique na abertura do mastro de bombeiro do andar em que você está.",
        links: &[],
    },
    Tip {
        record: rec::PROJECTS_POLE,
        pos: (376, 92),
        w: 164,
        text: "Precisa de inspiração para desenhar?\n• Clique no mastro de bombeiro para subir até o andar dos Truques de Desenho.\nQuer pintar por conta própria?\n• Clique na abertura do mastro de bombeiro para descer até o Ateliê de Pintura.",
        links: &[],
    },
    Tip {
        record: rec::TRICKS_POLE,
        pos: (432, 62),
        w: 165,
        text: "Quer a ajuda da Maggie para fazer uma história em quadrinhos, um botão ou adesivo, ou um desenho animado?\n• Clique na abertura do mastro de bombeiro para descer até a Oficina de Projetos.\nQuer pintar por conta própria?\n• Use a abertura do mastro de bombeiro para descer dois andares até o Ateliê de Pintura",
        links: &[(rec::HOTKEY_DOWN, "Tecla de atalho")],
    },
    Tip {
        record: rec::HOTKEY_DOWN,
        pos: (195, 204),
        w: 251,
        text: "Quer descer DOIS andares?\nSegure a tecla Ctrl enquanto clica na abertura do mastro de bombeiro do andar em que você está.",
        links: &[],
    },
    Tip {
        record: rec::SKIP_PROJECT,
        pos: (203, 190),
        w: 233,
        text: "Quer ignorar este balão e começar um novo projeto imediatamente?\n• Na próxima vez, segure Ctrl quando clicar no projeto que você quer começar.",
        links: &[],
    },
    Tip {
        record: rec::SKIP_BASIC,
        pos: (203, 190),
        w: 232,
        text: "Você quer começar os truques básicos sem nenhuma mensagem?\n• Na próxima vez, segure Ctrl quando clicar em Básico.",
        links: &[],
    },
    Tip {
        record: rec::SKIP_PERSP,
        pos: (157, 210),
        w: 325,
        text: "Quer começar a lição tridimensional sem nenhuma mensagem?\n• Da próxima vez, segure Ctrl quando clicar em 3D.",
        links: &[],
    },
    Tip {
        record: rec::GLOSS_3D,
        pos: (238, 192),
        w: 162,
        text: "tridimensional\nUma figura é tridimensional ou 3-D quando mostra profundidade. As imagens no quadro parecem saltar da página.",
        links: &[],
    },
];

/// Dica (sem opções) do registro `k`, do Fine Artist.
pub fn tip(assets: &Assets, k: usize, user: &str) -> Balloon {
    let kit = &ARTIST_KIT;
    tip_in(
        kit,
        kit.tips.iter().find(|t| t.record == k),
        assets,
        k,
        user,
    )
}

/// Dica do registro `k` do bloco do kit; `found` é a posição e o texto
/// reserva (`RoomSpec::tip`), sem eles um balão genérico.
pub fn tip_in(
    kit: &RoomKit,
    found: Option<&Tip>,
    assets: &Assets,
    k: usize,
    user: &str,
) -> Balloon {
    let (pos, w, text, links) = match found {
        Some(t) => (t.pos, t.w, t.text, t.links),
        None => ((200, 200), 200, "", &[][..]),
    };
    load_in(kit, assets, k, pos, user, fallback(w, text, &[], links))
}

/// Som da dica aberta por uma ligação (lâmpada ou tecla de atalho).
pub fn link_sound(k: usize) -> u16 {
    ARTIST_KIT.link_sound(k)
}

/// Objeto que abre um balão de escolha ("criar novo" / "trabalhar com um
/// que você já criou" / "Ignorar isto"): os da Oficina de Projetos e dos
/// Truques do Fine Artist e os da Oficina de Projetos e de Ideias do
/// Creative Writer (dados em `writer_floors`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// Vaca (QUADRINHOS).
    Comics,
    /// Cobra (ADESIVO).
    Sticker,
    /// Porco (DESENHO ANIMADO).
    Flipbook,
    /// Estátua do porco (Básico).
    Basic,
    /// O Pensador (3D).
    ThreeD,
    /// Writer: alce JORNAL.
    Newspaper,
    /// Writer: burro CARTÃO.
    Card,
    /// Writer: rinoceronte FAIXA.
    Banner,
    /// Writer: Máquina de Ideias.
    IdeaMachine,
    /// Writer: Quadro de Figuras.
    PictureBoard,
}

/// Linha "Ignorar isto" do balão de escolha (depois das duas opções).
pub const IGNORE: usize = 2;

/// O que fazer ao começar um trabalho novo do objeto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    Scene(SceneId),
    /// "Botão ou adesivo?": no port são dois projetos.
    AskButton,
}

/// Dados do balão de escolha de um objeto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickSpec {
    /// Registro do bloco `WHAT` do kit da sala.
    pub record: usize,
    /// Onde o balão aparece (apontando o objeto).
    pub pos: (i32, i32),
    /// Largura, texto e opções reserva.
    pub w: i32,
    pub text: &'static str,
    pub options: [&'static str; 2],
    /// Dica do "Ignorar isto" ("segure Ctrl…").
    pub skip: usize,
    /// O que a primeira opção (ou Ctrl+clique) começa.
    pub start: Start,
}

impl Pick {
    /// Dados do objeto (os do Writer vêm de `writer_floors::pick`).
    pub fn spec(self) -> PickSpec {
        let p = |record, pos, w, text, options, skip, start| PickSpec {
            record,
            pos,
            w,
            text,
            options,
            skip,
            start,
        };
        match self {
            Pick::Comics => p(
                rec::COMIC,
                (8, 116),
                204,
                "Oi! Você quer:",
                [
                    "Criar uma história em quadrinhos?",
                    "Trabalhar com uma história em quadrinhos que você já criou?",
                ],
                rec::SKIP_PROJECT,
                Start::Scene(SceneId::Project(ProjectKind::Comics)),
            ),
            Pick::Sticker => p(
                rec::STICKER,
                (183, 239),
                175,
                "Você quer:",
                [
                    "Começar um novo botão ou adesivo?",
                    "Trabalhar com um botão ou adesivo que você já criou?",
                ],
                rec::SKIP_PROJECT,
                Start::AskButton,
            ),
            Pick::Flipbook => p(
                rec::PIGTURE,
                (232, 82),
                175,
                "Oi! Você quer:",
                [
                    "Fazer um novo desenho animado?",
                    "Trabalhar com um desenho animado que você já criou?",
                ],
                rec::SKIP_PROJECT,
                Start::Scene(SceneId::Project(ProjectKind::Flipbook)),
            ),
            Pick::Basic => p(
                rec::BASIC,
                (140, 193),
                179,
                "Tudo bem! Você quer:",
                [
                    "Começar uma coisa nova?",
                    "Trabalhar com um truque que você já começou?",
                ],
                rec::SKIP_BASIC,
                Start::Scene(SceneId::Tricks(TrickKind::Basic)),
            ),
            Pick::ThreeD => p(
                rec::PERSP,
                (323, 215),
                198,
                "O truque tridimensional! Você quer:",
                [
                    "Começar do começo",
                    "Trabalhar com um truque que você já começou",
                ],
                rec::SKIP_PERSP,
                Start::Scene(SceneId::Tricks(TrickKind::ThreeD)),
            ),
            Pick::Newspaper
            | Pick::Card
            | Pick::Banner
            | Pick::IdeaMachine
            | Pick::PictureBoard => super::writer_floors::pick(self),
        }
    }

    pub fn record(self) -> usize {
        self.spec().record
    }

    pub fn start(self) -> Start {
        self.spec().start
    }

    /// Balão de escolha do objeto, apontando para ele (bloco do Fine Artist).
    pub fn balloon(self, assets: &Assets, user: &str) -> Balloon {
        self.balloon_in(&ARTIST_KIT, assets, user)
    }

    /// Balão de escolha com o bloco `WHAT` do kit da sala.
    pub fn balloon_in(self, kit: &RoomKit, assets: &Assets, user: &str) -> Balloon {
        let s = self.spec();
        let mut b = load_in(
            kit,
            assets,
            s.record,
            s.pos,
            user,
            fallback(s.w, s.text, &s.options, &[(s.skip, "Ignorar isto")]),
        );
        // no original "Ignorar isto" vem depois das opções: vira a última
        // linha (opção `IGNORE`), que a sala trata como a dica
        b.options
            .extend(b.links.drain(..).map(|l| l.1.trim().to_string()));
        b
    }

    /// Dica do "Ignorar isto" ("segure Ctrl…").
    pub fn skip_tip(self) -> usize {
        self.spec().skip
    }
}

/// Personagem do andar: McZee (saguão e Ateliê/Escritório), Maggie
/// (Projetos e Truques do Fine Artist) ou Max (Projetos e Ideias do Writer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Helper {
    McZee,
    Maggie,
    Max,
}

impl Helper {
    /// Fala ao ser clicado (sons do Fine Artist; as salas usam a do
    /// `CharacterSpec`, que vale para os dois programas).
    pub fn sound(self) -> u16 {
        match self {
            Helper::McZee => snd::MCZEE,
            Helper::Maggie => snd::MAGGIE,
            Helper::Max => super::writer_rooms::snd::MAX,
        }
    }

    /// "Você quer me ver menos?" (ajuda ligada) ou "Que bom que você me
    /// chamou de novo." (ajuda desligada).
    /// `pos` depende do andar (medido no original).
    pub fn balloon(self, assets: &Assets, user: &str, help_off: bool, pos: (i32, i32)) -> Balloon {
        self.balloon_in(&ARTIST_KIT, assets, user, help_off, pos)
    }

    /// Como [`Helper::balloon`], com os registros do kit da sala.
    pub fn balloon_in(
        self,
        kit: &RoomKit,
        assets: &Assets,
        user: &str,
        help_off: bool,
        pos: (i32, i32),
    ) -> Balloon {
        if help_off {
            load_in(
                kit,
                assets,
                kit.help_on,
                pos,
                user,
                fallback(
                    203,
                    "Que bom que você me chamou de novo. O que você quer que eu faça?",
                    &[
                        "Ajudar você mais um pouco?",
                        "Não, eu não preciso de mais ajuda.",
                    ],
                    &[],
                ),
            )
        } else {
            load_in(
                kit,
                assets,
                kit.help_off,
                pos,
                user,
                fallback(
                    162,
                    "Você quer me ver menos?",
                    &[
                        "Sim, por favor.",
                        "Diga de novo o que eu posso fazer aqui.",
                        "Deixa para lá.",
                    ],
                    &[],
                ),
            )
        }
    }
}

/// "EPA! O que você quer fazer?" (Sair do Fine Artist / Continuar
/// trabalhando), o mesmo da Entrada, sobre a sala.
pub fn quit_balloon(assets: &Assets, pos: (i32, i32)) -> TextBalloon {
    let mut b = Entry::quit_balloon(assets);
    b.x = pos.0;
    b.y = pos.1;
    b
}

/// Aviso "EPA!" do Rex sem opções (registro `k` do bloco `block` do
/// programa `p`) em `pos`; sem os dados, o texto reserva. Fecha com
/// qualquer clique.
pub fn alert_balloon_in(
    p: Program,
    assets: &Assets,
    (block, k): (u16, usize),
    pos: (i32, i32),
    text: &str,
) -> TextBalloon {
    match assets.dialogs(p, block).get(k) {
        Some(r) if !r.plain().trim().is_empty() => {
            TextBalloon::from_record(r, pos.0, pos.1, "", Vec::new())
        }
        _ => TextBalloon::new(
            pos.0,
            pos.1,
            235,
            vec![
                crate::text_balloon::Part::Figure(crate::text_balloon::PIC_ALERT),
                crate::text_balloon::Part::Text(text.into()),
            ],
            Vec::new(),
        ),
    }
}

/// Como [`quit_balloon`], com o "EPA!" do programa (`kit.quit`).
pub fn quit_balloon_in(q: &QuitSpec, assets: &Assets, pos: (i32, i32)) -> TextBalloon {
    let mut b = Entry::quit_balloon_in(q, assets);
    b.x = pos.0;
    b.y = pos.1;
    b
}

/// Ponta do rabicho dos balões do McZee nas salas: ele fica à esquerda,
/// embaixo do balão, e o rabicho sai do topo, 20 px depois do canto, até 12
/// px acima (medido no saguão, no Ateliê e na Galeria). A Maggie (Projetos,
/// Truques) e o elevador não têm rabicho no original.
pub fn mczee_tail(helper: Option<Helper>, elevator: bool, b: &Balloon) -> Option<(i32, i32)> {
    (helper == Some(Helper::McZee) && !elevator).then_some((b.x + 20, b.y - 12))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::assets;

    /// Com os dados reais: os balões vêm do `WHAT 100d` com as opções e as
    /// ligações esperadas ("Ignorar isto", "Tecla de atalho", "Descendo?").
    #[test]
    fn balloons_from_real_data() {
        let Some(a) = assets() else { return };
        for p in [
            Pick::Comics,
            Pick::Sticker,
            Pick::Flipbook,
            Pick::Basic,
            Pick::ThreeD,
        ] {
            let b = p.balloon(&a, "ana");
            assert_eq!(b.options.len(), 3, "{p:?}");
            assert!(b.links.is_empty(), "{p:?}");
            assert_eq!(b.options[IGNORE], "Ignorar isto", "{p:?}");
            assert!(!b.text.contains("Ignorar"), "{p:?}");
        }
        let b = tip(&a, rec::STUDIO_POLE, "ana");
        let targets: Vec<usize> = b.links.iter().map(|l| l.0).collect();
        assert_eq!(targets, [rec::HOTKEY_UP, rec::POLE_DOWN]);
        assert!(!b.text.contains("Tecla de atalho"));
        let b = tip(&a, rec::TRICKS_POLE, "ana");
        assert_eq!(
            b.links.iter().map(|l| l.0).collect::<Vec<_>>(),
            [rec::HOTKEY_DOWN]
        );
        let b = tip(&a, rec::HOTKEY_DOWN, "ana");
        assert!(b.text.contains("DOIS"), "{}", b.text);
        let b = tip(&a, rec::STUDIO_NEW, "ana");
        assert!(b.text.contains("clique duplo"), "{}", b.text);
        for off in [false, true] {
            let b = Helper::Maggie.balloon(&a, "ana", off, (109, 132));
            assert_eq!(b.options.len(), if off { 2 } else { 3 });
        }
        let q = quit_balloon(&a, (214, 200));
        assert_eq!(q.options().len(), 2);
        assert_eq!((q.x, q.y), (214, 200));
    }
}
