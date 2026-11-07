//! História do McZee: o álbum de fotos (`4201`, 495×314, com o McZee
//! espiando por trás, recortado pela máscara `4209`) sobre as nuvens
//! (`4200`), as 9 falas do `WHAT 4200` (os registros 2 e 3 repetem o 1) em 7
//! páginas duplas com duas fotos e as legendas escritas à mão, como no
//! original (`studio_o_book_6.png`, `studio_o_bookpg_*.png`). Segue (ou
//! Enter, →, um clique no balão ou na página direita do álbum) passa a
//! fala; Volta (a partir da segunda; ou ←, PageUp, ou um clique na página
//! esquerda) volta uma; Para sai para Imaginópolis, como o fim. Os botões
//! tocam os sons do original (Segue `2eb`, Volta `2f5`, Para `2ec`).
//! Aberta pelo livro verde de uma sala ([`Story::over_room`]), aparece por
//! cima da sala e Para (ou o fim) volta a ela.

use super::entry::BACKGROUND;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Input, Key, KidCommon, MouseButton, Scene, SceneId};
use crate::text_balloon::{Button, Part, TextBalloon, blit_pic, blit_pic_opaque};
use crate::{Frame, SCREEN_H, SCREEN_W};

/// Álbum aberto com o McZee espiando por trás (máscara `4209`).
pub const ALBUM: u16 = 0x4201;
/// Fundo de nuvens próprio da história (sem o quadro-negro da Entrada).
pub const STORY_BACKGROUND: u16 = 0x4200;
/// Posição do álbum na tela (medida em `studio_o_book_6.png`) e das fotos
/// (169×119; as de 167×117 vão um pixel para dentro) dentro dele.
pub const ALBUM_POS: (i32, i32) = (76, 114);
const LEFT_PHOTO: (i32, i32) = (37, 122);
const RIGHT_PHOTO: (i32, i32) = (276, 163);
const PHOTO: (u32, u32) = (169, 119);
/// Páginas do álbum (dentro dele): clicar na direita passa, na esquerda volta.
const LEFT_PAGE: (i32, i32, i32, i32) = (8, 106, 232, 200);
const RIGHT_PAGE: (i32, i32, i32, i32) = (246, 106, 230, 200);
const STORY_WHAT: u16 = 0x4200;
/// Canto inferior direito do balão (ele fica à esquerda da cabeça do McZee).
const BALLOON_CORNER: (i32, i32) = (416, 197);

/// Sons dos botões do balão: Segue, Volta e Para.
pub mod sfx {
    pub const NEXT: u16 = 0x2eb;
    pub const BACK: u16 = 0x2f5;
    pub const STOP: u16 = 0x2ec;
}

/// Uma página dupla: foto da esquerda, foto da direita e as figuras por
/// cima (legendas escritas à mão e, na primeira, a Maggie na rua), com a
/// posição dentro do álbum.
pub struct Page {
    pub left: u16,
    pub right: u16,
    pub figures: &'static [(u16, i32, i32)],
}

/// As 7 páginas duplas do original (`studio_o_bookpg_*.png`).
pub const PAGES: [Page; 7] = [
    Page {
        left: 0x420b,
        right: 0x4211,
        // Maggie (na rua da esquerda), "Maggie" e "Max"
        figures: &[(0x422b, 161, 152), (0x4219, 77, 260), (0x4223, 336, 122)],
    },
    Page {
        left: 0x423b,
        right: 0x4253,
        // "Nossa casa na árvore", "Os contadores de histórias"
        figures: &[(0x4241, 47, 262), (0x424b, 278, 117)],
    },
    Page {
        left: 0x4273,
        right: 0x4259,
        // "A chegada do McZee", "Nosso primeiro encontro"
        figures: &[(0x426b, 26, 257), (0x4261, 262, 114)],
    },
    Page {
        left: 0x4293,
        right: 0x4279,
        // "Imaginópolis", "O Museu?!"
        figures: &[(0x428b, 42, 253), (0x4281, 286, 115)],
    },
    Page {
        left: 0x42b3,
        right: 0x4299,
        // "Obras-primas" dos dois lados
        figures: &[(0x42a1, 44, 261), (0x42a1, 284, 119)],
    },
    Page {
        left: 0x42d3,
        right: 0x42b9,
        // "McZee explica", "mais Obras-primas"
        figures: &[(0x42cb, 32, 256), (0x42a1, 284, 119), (0x42c1, 312, 108)],
    },
    Page {
        left: 0x42f3,
        right: 0x42d9,
        // "Histórias do McZee", "A visita"
        figures: &[(0x42eb, 37, 259), (0x42e1, 277, 113)],
    },
];

/// As falas: registro do `WHAT 4200` e página dupla. Os registros 2 e 3 do
/// original repetem o 1 ("Same as #2") e não aparecem: são 9 falas.
pub const SPEECHES: [(usize, usize); 9] = [
    (0, 0),
    (1, 0),
    (4, 1),
    (5, 2),
    (6, 3),
    (7, 4),
    (8, 5),
    (9, 6),
    (10, 6),
];

/// Textos do original (por registro), usados quando o `ARTIST.EXE` não está instalado.
const FALLBACK: [&str; 11] = [
    "Oi, eu sou o McZee! Seja bem-vindo ao meu mundo! Você provavelmente quer saber alguma coisa sobre mim e por que motivo eu estou aqui. A minha história começa com um garoto e uma garota como você: uma artista plástica chamada Maggie e um escritor chamado Max.",
    "Eles se mudaram no mesmo dia para casas vizinhas e se tornaram amigos inseparáveis!",
    "Eles se mudaram no mesmo dia para casas vizinhas e se tornaram amigos inseparáveis!",
    "Eles se mudaram no mesmo dia para casas vizinhas e se tornaram amigos inseparáveis!",
    "Não demorou muito para brincarem juntos todo dia. Eles construíram uma casa na árvore e passaram muito tempo juntos ali, escrevendo e desenhando para contar suas aventuras.",
    "Um dia, quando eles estavam brincando na casa na árvore, eu entrei como um furacão. Eles não perceberam que o nosso encontro ia mudar suas vidas para sempre.",
    "Eu levei os dois para a minha casa em Imaginópolis — o Museu Mágico. Logo de cara, o Max e a Maggie perceberam que aquele era um lugar um pouco diferente.",
    "As obras do meu museu eram diferentes de tudo o que a Maggie já tinha visto. Todos os quadros e esculturas tinham o mesmo tema — eu!",
    "Era o meu museu. Eu sabia que os meus amigos Leonardo, Georgia O'Keefe, Picasso e Rembrandt não iam se incomodar com as minhas pequenas mudanças.",
    "— Você conheceu esses caras de verdade? — a Maggie perguntou.\n— Claro que sim! Quem você acha que inspirou todos eles? Eu sou a centelha criativa. Eu ajudo todo mundo a ter idéias — expliquei.",
    "Eu mostrei a eles todo o museu. Dei a eles muitas ferramentas e idéias para criar coisas chocantes! Venha você também para o meu mundo em Imaginópolis. Explore e veja quantas ferramentas diferentes você vai encontrar para brincar!",
];

pub struct Story {
    common: KidCommon,
    page: usize,
    balloon: TextBalloon,
    done: bool,
    /// Aberta por cima de uma sala: sem o fundo de nuvens; sai com `Back`.
    over_room: bool,
}

impl Story {
    pub fn new(click: Option<SoundRef>, assets: &Assets) -> Story {
        Story {
            common: KidCommon::new(click),
            page: 0,
            balloon: Self::balloon(0, assets),
            done: false,
            over_room: false,
        }
    }

    /// A história contada pelo livro verde de uma sala: desenhada por cima
    /// dela, e Para ou o fim voltam à sala.
    pub fn over_room(mut self) -> Story {
        self.over_room = true;
        self
    }

    fn leave_story(&mut self, fx: &mut Vec<Effect>) {
        self.done = true;
        fx.push(if self.over_room {
            Effect::Back
        } else {
            Effect::ChangeScene(SceneId::Imaginopolis)
        });
    }

    /// Fala atual (índice em `SPEECHES`).
    pub fn page(&self) -> usize {
        self.page
    }

    /// Página dupla do álbum aberta.
    pub fn spread(&self) -> usize {
        SPEECHES[self.page].1
    }

    fn balloon(speech: usize, assets: &Assets) -> TextBalloon {
        let mut buttons = Vec::new();
        if speech > 0 {
            buttons.push(Button::Back);
        }
        buttons.push(Button::Stop);
        buttons.push(Button::Next);
        let k = SPEECHES[speech].0;
        let record = assets
            .dialogs(Program::Artist, STORY_WHAT)
            .into_iter()
            .nth(k);
        let text = record
            .as_ref()
            .map(|r| r.plain().trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| FALLBACK[k].to_string());
        let w = record.map_or(275, |r| r.width as i32);
        let mut b = TextBalloon::new(0, 0, w, vec![Part::Text(text)], buttons);
        // à esquerda da cabeça do McZee, com a base logo acima do álbum
        b.x = BALLOON_CORNER.0 - b.w;
        b.y = (BALLOON_CORNER.1 - b.height(assets)).max(4);
        b
    }

    fn next(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        fx.push(sound(sfx::NEXT));
        if self.page + 1 < SPEECHES.len() {
            self.page += 1;
            self.balloon = Self::balloon(self.page, assets);
        } else {
            self.leave_story(fx);
        }
    }

    fn back(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        if self.page > 0 {
            fx.push(sound(sfx::BACK));
            self.page -= 1;
            self.balloon = Self::balloon(self.page, assets);
        }
    }

    /// Página do álbum sob `(x, y)`: `Some(true)` na direita, `Some(false)` na esquerda.
    fn album_page_at(x: i32, y: i32) -> Option<bool> {
        let (ax, ay) = ALBUM_POS;
        let (x, y) = (x - ax, y - ay);
        let inside =
            |r: (i32, i32, i32, i32)| x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3;
        if inside(RIGHT_PAGE) {
            Some(true)
        } else if inside(LEFT_PAGE) {
            Some(false)
        } else {
            None
        }
    }
}

impl Scene for Story {
    fn update(&mut self, input: &[Input], _dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = self.common.update(input);
        for i in input {
            if self.done {
                break;
            }
            match self.balloon.handle(assets, i) {
                Some(Button::Next) => self.next(&mut fx, assets),
                Some(Button::Back) => self.back(&mut fx, assets),
                Some(Button::Stop) => {
                    fx.push(sound(sfx::STOP));
                    self.leave_story(&mut fx);
                }
                None => match i {
                    Input::Click {
                        button: MouseButton::Left,
                        x,
                        y,
                    } => {
                        if self.balloon.contains(assets, *x, *y) {
                            self.next(&mut fx, assets);
                        } else {
                            match Self::album_page_at(*x, *y) {
                                Some(true) => self.next(&mut fx, assets),
                                Some(false) => self.back(&mut fx, assets),
                                None => {}
                            }
                        }
                    }
                    Input::Key {
                        key: Key::Right | Key::PageDown,
                        ..
                    } => self.next(&mut fx, assets),
                    Input::Key {
                        key: Key::Left | Key::PageUp | Key::Backspace,
                        ..
                    } => self.back(&mut fx, assets),
                    _ => {}
                },
            }
        }
        fx
    }

    fn overlay(&self) -> bool {
        self.over_room
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        if !self.over_room {
            frame.fill([136, 136, 230]);
            let id = if assets
                .pic_ids(Program::Artist)
                .binary_search(&STORY_BACKGROUND)
                .is_ok()
            {
                STORY_BACKGROUND
            } else {
                BACKGROUND
            };
            let bg = assets.image(&ImageRef::Pic(Program::Artist, id));
            frame.blit(
                &bg,
                (SCREEN_W as i32 - bg.width as i32) / 2,
                (SCREEN_H as i32 - bg.height as i32) / 2,
            );
        }
        let (ax, ay) = ALBUM_POS;
        blit_pic(frame, assets, ALBUM, ax, ay, true);
        let page = &PAGES[self.spread()];
        for (id, (px, py)) in [(page.left, LEFT_PHOTO), (page.right, RIGHT_PHOTO)] {
            let img = assets.image(&ImageRef::Pic(Program::Artist, id));
            // as fotos menores (167×117) ficam centradas no lugar das maiores
            let dx = (PHOTO.0.saturating_sub(img.width) / 2) as i32;
            let dy = (PHOTO.1.saturating_sub(img.height) / 2) as i32;
            blit_pic_opaque(frame, assets, id, ax + px + dx, ay + py + dy);
        }
        for &(id, x, y) in page.figures {
            blit_pic(frame, assets, id, ax + x, ay + y, true);
        }
        self.balloon.draw(frame, assets);
    }
}

fn sound(id: u16) -> Effect {
    Effect::PlaySound(SoundRef::Snd(Program::Artist, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};

    fn click(x: i32, y: i32) -> Input {
        Input::Click {
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

    #[test]
    fn pages_advance_and_end_in_imaginopolis() {
        let (_d, a) = empty_assets();
        let mut s = Story::new(None, &a);
        let mut f = Frame::new(640, 480);
        for k in 0..SPEECHES.len() - 1 {
            assert_eq!(s.page(), k);
            s.draw(&mut f, &a);
            // Para em todas as falas
            assert!(s.balloon.button_rect(&a, Button::Stop).is_some());
            let r = s.balloon.button_rect(&a, Button::Next).unwrap();
            let fx = s.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
            assert!(fx.contains(&sound(sfx::NEXT)));
            assert!(!fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))));
        }
        let fx = s.update(
            &[Input::Key {
                key: Key::Enter,
                ctrl: false,
                alt: false,
            }],
            0.0,
            &a,
        );
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Imaginopolis)));
        assert!(
            !s.update(&[click(20, 20)], 0.0, &a)
                .iter()
                .any(|e| matches!(e, Effect::ChangeScene(_)))
        );
    }

    /// 9 falas em 7 páginas duplas: as duas primeiras e as duas últimas
    /// dividem a página.
    #[test]
    fn nine_speeches_in_seven_spreads() {
        let spreads: Vec<usize> = SPEECHES.iter().map(|s| s.1).collect();
        assert_eq!(spreads, [0, 0, 1, 2, 3, 4, 5, 6, 6]);
        assert_eq!(PAGES.len(), 7);
        // os registros repetidos ("Same as #2") ficam de fora
        assert!(!SPEECHES.iter().any(|s| s.0 == 2 || s.0 == 3));
    }

    #[test]
    fn back_button_and_album_pages_turn_back_and_forth() {
        let (_d, a) = empty_assets();
        let mut s = Story::new(None, &a);
        // na primeira página não há Volta; ← não faz nada
        assert!(s.balloon.button_rect(&a, Button::Back).is_none());
        s.update(&[key(Key::Left)], 0.0, &a);
        assert_eq!(s.page(), 0);
        // clique na página direita do álbum passa; na esquerda volta
        let (ax, ay) = ALBUM_POS;
        s.update(&[click(ax + 350, ay + 200)], 0.0, &a);
        assert_eq!(s.page(), 1);
        s.update(&[click(ax + 350, ay + 200)], 0.0, &a);
        assert_eq!(s.page(), 2);
        s.update(&[click(ax + 100, ay + 200)], 0.0, &a);
        assert_eq!(s.page(), 1);
        // o McZee atrás do álbum não vira página
        s.update(&[click(ax + 420, ay + 20)], 0.0, &a);
        assert_eq!(s.page(), 1);
        // botão Volta e tecla ←
        let r = s.balloon.button_rect(&a, Button::Back).unwrap();
        let fx = s.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert_eq!(s.page(), 0);
        assert!(!fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))));
        s.update(&[key(Key::Right), key(Key::Right), key(Key::Left)], 0.0, &a);
        assert_eq!(s.page(), 1);
        s.update(&[key(Key::PageUp)], 0.0, &a);
        assert_eq!(s.page(), 0);
    }

    #[test]
    fn stop_leaves_early() {
        let (_d, a) = empty_assets();
        let mut s = Story::new(None, &a);
        let r = s.balloon.button_rect(&a, Button::Stop).unwrap();
        let fx = s.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert!(fx.contains(&Effect::ChangeScene(SceneId::Imaginopolis)));
    }

    #[test]
    fn book_story_returns_to_the_room() {
        let (_d, a) = empty_assets();
        let mut s = Story::new(None, &a).over_room();
        assert!(s.overlay());
        // por cima da sala: o fundo de nuvens não é desenhado
        let mut f = Frame::new(640, 480);
        f.fill([1, 2, 3]);
        s.draw(&mut f, &a);
        assert_eq!(f.pixel(2, 470)[..3], [1, 2, 3]);
        let r = s.balloon.button_rect(&a, Button::Stop).unwrap();
        let fx = s.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
        assert!(fx.contains(&Effect::Back));
        assert!(!fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))));
    }

    #[test]
    fn texts_come_from_what_with_real_data() {
        let Some(a) = assets() else { return };
        let mut s = Story::new(None, &a);
        assert!(
            matches!(&s.balloon.parts[0], Part::Text(t) if t.starts_with("Oi, eu sou o McZee!"))
        );
        // largura do registro, à esquerda da cabeça do McZee
        assert_eq!(s.balloon.w, 0x12f);
        assert_eq!(s.balloon.x + s.balloon.w, BALLOON_CORNER.0);
        let mut f = Frame::new(640, 480);
        f.fill([1, 2, 3]);
        s.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "story_1");
        // a foto da esquerda (rua cinza) ocupa a moldura
        let (ax, ay) = ALBUM_POS;
        let p = f.pixel(
            (ax + LEFT_PHOTO.0 + 80) as u32,
            (ay + LEFT_PHOTO.1 + 20) as u32,
        );
        assert_ne!(p[..3], [255, 255, 255]);
        // sem retângulo branco atrás do McZee: o canto de cima do álbum é fundo
        let s2 = Story::new(None, &a).over_room();
        let mut f = Frame::new(640, 480);
        f.fill([1, 2, 3]);
        s2.draw(&mut f, &a);
        assert_eq!(f.pixel((ax + 5) as u32, (ay + 5) as u32)[..3], [1, 2, 3]);
        s.update(&[click(s.balloon.x + 20, s.balloon.y + 10)], 0.0, &a);
        assert_eq!(s.page(), 1);
        assert!(matches!(&s.balloon.parts[0], Part::Text(t) if t.starts_with("Eles se mudaram")));
        // a terceira fala já é "Não demorou muito…", na segunda página dupla
        s.update(&[key(Key::Right)], 0.0, &a);
        assert_eq!(s.spread(), 1);
        assert!(matches!(&s.balloon.parts[0], Part::Text(t) if t.starts_with("Não demorou")));
    }
}
