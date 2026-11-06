//! História do McZee: o álbum de fotos (`4201`, 495×314, com o McZee
//! espiando por trás) sobre as nuvens da Entrada, uma foto e uma legenda por
//! página, e o balão correspondente do `WHAT 4200`. Segue (ou Enter, →,
//! um clique no balão ou na página direita do álbum, com o rótulo "mais"
//! `42c1`) passa a página; Volta (ou ←, PageUp, ou um clique na página
//! esquerda) volta uma; Para sai para Imaginópolis, como o fim.
//! Aberta pelo livro verde de uma sala ([`Story::over_room`]), aparece por
//! cima da sala e Para (ou o fim) volta a ela.

use super::entry::BACKGROUND;
use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::scene::{Effect, Input, Key, KidCommon, MouseButton, Scene, SceneId};
use crate::text_balloon::{Button, Part, TextBalloon, blit_pic_opaque};
use crate::{Frame, SCREEN_H, SCREEN_W};

pub const ALBUM: u16 = 0x4201;
/// Posição do álbum na tela e das fotos (169×119) dentro dele.
pub const ALBUM_POS: (i32, i32) = (72, 160);
const LEFT_PHOTO: (i32, i32) = (37, 122);
const RIGHT_FRAME: (i32, i32, i32, i32) = (277, 164, 167, 117);
/// Páginas do álbum (dentro dele): clicar na direita passa, na esquerda volta.
const LEFT_PAGE: (i32, i32, i32, i32) = (8, 92, 232, 216);
const RIGHT_PAGE: (i32, i32, i32, i32) = (246, 92, 224, 216);
/// Rótulo "mais" (`42c1`, 50×18) no canto da página direita, enquanto há
/// página seguinte; `42ab` é a versão vazia.
pub const MORE: u16 = 0x42c1;
const MORE_POS: (i32, i32) = (408, 286);
const STORY_WHAT: u16 = 0x4200;
/// Posição e largura do balão (mais largo que no original para caber sobre o álbum).
const BALLOON: (i32, i32, i32) = (16, 12, 460);

/// Páginas: foto da esquerda e figura da direita (foto ou legenda), por balão.
pub const PAGES: [(u16, u16); 11] = [
    (0x420b, 0x424b),
    (0x4210, 0x4219),
    (0x4211, 0x4223),
    (0x4218, 0x4219),
    (0x423b, 0x4241),
    (0x4273, 0x426b),
    (0x4293, 0x428b),
    (0x4299, 0x42a1),
    (0x42b3, 0x4281),
    (0x42b9, 0x42cb),
    (0x42f3, 0x42e1),
];

/// Textos do original, usados quando o `ARTIST.EXE` não está instalado.
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

    pub fn page(&self) -> usize {
        self.page
    }

    fn balloon(page: usize, assets: &Assets) -> TextBalloon {
        let mut buttons = Vec::new();
        if page > 0 {
            buttons.push(Button::Back);
        }
        if page + 1 < PAGES.len() {
            buttons.push(Button::Stop);
        }
        buttons.push(Button::Next);
        let text = assets
            .dialogs(Program::Artist, STORY_WHAT)
            .get(page)
            .map(|r| r.plain().trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| FALLBACK[page].to_string());
        TextBalloon::new(
            BALLOON.0,
            BALLOON.1,
            BALLOON.2,
            vec![Part::Text(text)],
            buttons,
        )
    }

    fn next(&mut self, fx: &mut Vec<Effect>, assets: &Assets) {
        if self.page + 1 < PAGES.len() {
            self.page += 1;
            self.balloon = Self::balloon(self.page, assets);
        } else {
            self.leave_story(fx);
        }
    }

    fn back(&mut self, assets: &Assets) {
        if self.page > 0 {
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
                Some(Button::Back) => self.back(assets),
                Some(Button::Stop) => self.leave_story(&mut fx),
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
                                Some(false) => self.back(assets),
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
                    } => self.back(assets),
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
            let bg = assets.image(&ImageRef::Pic(Program::Artist, BACKGROUND));
            frame.blit(
                &bg,
                (SCREEN_W as i32 - bg.width as i32) / 2,
                (SCREEN_H as i32 - bg.height as i32) / 2,
            );
        }
        let (ax, ay) = ALBUM_POS;
        blit_pic_opaque(frame, assets, ALBUM, ax, ay);
        let (left, right) = PAGES[self.page];
        blit_pic_opaque(frame, assets, left, ax + LEFT_PHOTO.0, ay + LEFT_PHOTO.1);
        let img = assets.image(&ImageRef::Pic(Program::Artist, right));
        // foto ou legenda centrada na moldura da direita
        let (fx, fy, fw, fh) = RIGHT_FRAME;
        let x = ax + fx + (fw - img.width as i32) / 2;
        let y = ay + fy + (fh - img.height as i32) / 2;
        blit_pic_opaque(frame, assets, right, x, y);
        if self.page + 1 < PAGES.len() {
            blit_pic_opaque(frame, assets, MORE, ax + MORE_POS.0, ay + MORE_POS.1);
        }
        self.balloon.draw(frame, assets);
    }
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
        for k in 0..PAGES.len() - 1 {
            assert_eq!(s.page(), k);
            s.draw(&mut f, &a);
            let r = s.balloon.button_rect(&a, Button::Next).unwrap();
            let fx = s.update(&[click(r.0 + 2, r.1 + 2)], 0.0, &a);
            assert!(!fx.iter().any(|e| matches!(e, Effect::ChangeScene(_))));
        }
        assert!(s.balloon.button_rect(&a, Button::Stop).is_none());
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
        s.update(&[click(ax + 420, ay + 30)], 0.0, &a);
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
        let mut f = Frame::new(640, 480);
        s.draw(&mut f, &a);
        // a foto da esquerda (rua cinza) ocupa a moldura
        let (ax, ay) = ALBUM_POS;
        let p = f.pixel(
            (ax + LEFT_PHOTO.0 + 80) as u32,
            (ay + LEFT_PHOTO.1 + 60) as u32,
        );
        assert_ne!(p[..3], [255, 255, 255]);
        s.update(&[click(100, 30)], 0.0, &a);
        assert_eq!(s.page(), 1);
        assert!(matches!(&s.balloon.parts[0], Part::Text(t) if t.starts_with("Eles se mudaram")));
    }
}
