//! Navegador de recursos (ferramenta de desenvolvimento): mostra imagens, cursores
//! e sons do catálogo e registra coordenadas para remontar as telas.

use crate::assets::{Assets, ItemKind};
use crate::scene::{Effect, Input, Key, MouseButton, Scene};
use crate::{Frame, Image, SCREEN_H, SCREEN_W};
use std::rc::Rc;

const BAR_H: i32 = 16;

/// Onde desenhar a imagem: centralizada abaixo da faixa de informação, ou em (0, 0)
/// se ela não couber nessa área (a faixa então se sobrepõe à imagem).
pub fn image_origin(img_w: u32, img_h: u32) -> (i32, i32) {
    let area_h = SCREEN_H as i32 - BAR_H;
    let x = ((SCREEN_W as i32 - img_w as i32) / 2).max(0);
    let y = if img_h as i32 <= area_h {
        BAR_H + (area_h - img_h as i32) / 2
    } else {
        ((SCREEN_H as i32 - img_h as i32) / 2).max(0)
    };
    (x, y)
}

#[derive(Debug, Default)]
pub struct Browser {
    source: usize,
    item: usize,
    mouse: Option<(i32, i32)>,
}

enum Shown {
    Image(Rc<Image>, Option<(u16, u16)>),
    Sound(Option<(u32, f32)>),
    Nothing,
}

impl Browser {
    pub fn new() -> Browser {
        Browser::default()
    }

    /// (fonte, item) atuais.
    pub fn position(&self) -> (usize, usize) {
        (self.source, self.item)
    }

    pub fn select(&mut self, source: usize, item: usize) {
        self.source = source;
        self.item = item;
    }

    fn shown(&self, assets: &Assets) -> (String, Shown) {
        let Some(src) = assets.catalog().get(self.source) else {
            return (String::new(), Shown::Nothing);
        };
        let Some(item) = src.items.get(self.item) else {
            return (String::new(), Shown::Nothing);
        };
        let shown = match &item.kind {
            ItemKind::Image(r) => Shown::Image(assets.image(r), None),
            ItemKind::Cursor(c) => match assets.cursor(c) {
                Some((img, hs)) => Shown::Image(img, Some(hs)),
                None => Shown::Nothing,
            },
            ItemKind::Sound(r) => Shown::Sound(assets.sound(r).map(|p| {
                let frames = p.samples.len() / p.channels.max(1) as usize;
                (p.sample_rate, frames as f32 / p.sample_rate.max(1) as f32)
            })),
        };
        (format!("{} {}", src.name, item.label), shown)
    }

    /// Coordenadas do ponto (x, y) da tela dentro da imagem exibida, se estiver sobre ela.
    fn in_image(&self, assets: &Assets, x: i32, y: i32) -> Option<(i32, i32)> {
        let (_, Shown::Image(img, _)) = self.shown(assets) else {
            return None;
        };
        let (ox, oy) = image_origin(img.width, img.height);
        let (ix, iy) = (x - ox, y - oy);
        (ix >= 0 && iy >= 0 && ix < img.width as i32 && iy < img.height as i32).then_some((ix, iy))
    }

    pub fn info_line(&self, assets: &Assets) -> String {
        let Some(src) = assets.catalog().get(self.source) else {
            return "catálogo vazio".into();
        };
        let Some(item) = src.items.get(self.item) else {
            return format!("{} vazia", src.name);
        };
        let mut line = format!(
            "{} {}/{} {}",
            src.name,
            self.item + 1,
            src.items.len(),
            item.label
        );
        match self.shown(assets).1 {
            Shown::Image(img, hs) => {
                line += &format!(" {}x{}", img.width, img.height);
                if let Some((x, y)) = hs {
                    line += &format!(" hotspot {x},{y}");
                }
            }
            Shown::Sound(Some((rate, secs))) => line += &format!(" {rate} Hz {secs:.1} s [Enter]"),
            Shown::Sound(None) => line += " (som inválido)",
            Shown::Nothing => {}
        }
        if let Some((mx, my)) = self.mouse
            && let Some((x, y)) = self.in_image(assets, mx, my)
        {
            line += &format!(" @ {x},{y}");
        }
        line
    }

    fn step(&mut self, assets: &Assets, delta: isize) {
        let n = assets
            .catalog()
            .get(self.source)
            .map_or(0, |s| s.items.len());
        if n == 0 {
            self.item = 0;
            return;
        }
        self.item = (self.item as isize + delta).clamp(0, n as isize - 1) as usize;
    }

    fn change_source(&mut self, assets: &Assets, delta: isize) {
        let n = assets.catalog().len();
        if n > 0 {
            self.source = (self.source as isize + delta).clamp(0, n as isize - 1) as usize;
        }
        self.item = 0;
    }
}

impl Scene for Browser {
    fn update(&mut self, input: &[Input], _dt: f32, assets: &Assets) -> Vec<Effect> {
        let mut fx = Vec::new();
        for i in input {
            match *i {
                Input::Key {
                    key: Key::Q,
                    ctrl: true,
                    ..
                } => fx.push(Effect::Quit),
                Input::Key { key, .. } => match key {
                    Key::Escape => fx.push(Effect::Back),
                    Key::Up => self.change_source(assets, -1),
                    Key::Down => self.change_source(assets, 1),
                    Key::Left => self.step(assets, -1),
                    Key::Right => self.step(assets, 1),
                    Key::PageUp => self.step(assets, -10),
                    Key::PageDown => self.step(assets, 10),
                    Key::Home => self.step(assets, isize::MIN / 2),
                    Key::End => self.step(assets, isize::MAX / 2),
                    Key::Enter => {
                        let kind = assets
                            .catalog()
                            .get(self.source)
                            .and_then(|s| s.items.get(self.item))
                            .map(|i| &i.kind);
                        if let Some(ItemKind::Sound(r)) = kind {
                            fx.push(Effect::PlaySound(r.clone()));
                        }
                    }
                    _ => {}
                },
                Input::MouseMove { x, y } => self.mouse = Some((x, y)),
                Input::Click {
                    button: MouseButton::Left,
                    x,
                    y,
                } => {
                    self.mouse = Some((x, y));
                    if let Some((ix, iy)) = self.in_image(assets, x, y) {
                        let (name, _) = self.shown(assets);
                        fx.push(Effect::Log(format!("{name} x={ix} y={iy}")));
                    }
                }
                Input::Click { .. } | Input::MouseUp { .. } => {}
            }
        }
        fx
    }

    fn draw(&self, frame: &mut Frame, assets: &Assets) {
        // fundo quadriculado cinza
        let checker = Image {
            width: SCREEN_W,
            height: SCREEN_H,
            rgba: (0..SCREEN_W * SCREEN_H)
                .flat_map(|i| {
                    let (x, y) = (i % SCREEN_W, i / SCREEN_W);
                    let v = if ((x / 8) + (y / 8)) % 2 == 0 {
                        0x99
                    } else {
                        0x77
                    };
                    [v, v, v, 255]
                })
                .collect(),
        };
        frame.blit(&checker, 0, 0);
        if let (_, Shown::Image(img, _)) = self.shown(assets) {
            let (x, y) = image_origin(img.width, img.height);
            frame.blit(&img, x, y);
        }
        // a faixa fica embaixo quando o mouse está no topo da tela
        let bar_y = match self.mouse {
            Some((_, my)) if my < BAR_H => SCREEN_H as i32 - BAR_H,
            _ => 0,
        };
        let bar = Image {
            width: SCREEN_W,
            height: BAR_H as u32,
            rgba: [0, 0, 0, 255].repeat((SCREEN_W * BAR_H as u32) as usize),
        };
        frame.blit(&bar, 0, bar_y);
        frame.text(4, bar_y + 4, &self.info_line(assets), [255, 255, 255]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::{assets, empty_assets};

    fn key(k: Key) -> Input {
        Input::Key {
            key: k,
            ctrl: false,
            alt: false,
        }
    }

    fn goto(b: &mut Browser, a: &Assets, source: &str, label: &str) {
        let s = a.catalog().iter().position(|s| s.name == source).unwrap();
        let i = a.catalog()[s]
            .items
            .iter()
            .position(|i| i.label == label)
            .unwrap();
        b.select(s, i);
    }

    #[test]
    fn image_origin_centers_or_fills() {
        assert_eq!(image_origin(640, 480), (0, 0));
        assert_eq!(image_origin(100, 50), (270, 223));
        assert_eq!(image_origin(700, 600), (0, 0));
        assert_eq!(image_origin(641, 558), (0, 0));
    }

    #[test]
    fn empty_catalog_does_not_panic() {
        let (_d, a) = empty_assets();
        let mut b = Browser::new();
        for k in [
            Key::Up,
            Key::Down,
            Key::Left,
            Key::Right,
            Key::PageDown,
            Key::End,
            Key::Enter,
        ] {
            b.update(&[key(k)], 0.0, &a);
        }
        b.update(
            &[Input::Click {
                button: MouseButton::Left,
                x: 5,
                y: 5,
            }],
            0.0,
            &a,
        );
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        b.draw(&mut f, &a);
    }

    #[test]
    fn arrows_change_source_and_item() {
        let Some(a) = assets() else { return };
        let mut b = Browser::new();
        assert_eq!(b.position(), (0, 0));
        b.update(&[key(Key::Right), key(Key::Right)], 0.0, &a);
        assert_eq!(b.position(), (0, 2));
        b.update(&[key(Key::PageDown)], 0.0, &a);
        assert_eq!(b.position(), (0, 12));
        b.update(&[key(Key::Down)], 0.0, &a);
        assert_eq!(b.position(), (1, 0));
        b.update(&[key(Key::End)], 0.0, &a);
        assert_eq!(b.position(), (1, a.catalog()[1].items.len() - 1));
        b.update(&[key(Key::Home)], 0.0, &a);
        assert_eq!(b.position(), (1, 0));
    }

    #[test]
    fn navigation_clamps_at_ends() {
        let Some(a) = assets() else { return };
        let mut b = Browser::new();
        let back: Vec<Input> = (0..100)
            .flat_map(|_| [key(Key::Left), key(Key::Up), key(Key::PageUp)])
            .collect();
        b.update(&back, 0.0, &a);
        assert_eq!(b.position(), (0, 0));
        let fwd: Vec<Input> = (0..10_000).map(|_| key(Key::Right)).collect();
        b.update(&fwd, 0.0, &a);
        assert_eq!(b.position(), (0, a.catalog()[0].items.len() - 1));
        let down: Vec<Input> = (0..100).map(|_| key(Key::Down)).collect();
        b.update(&down, 0.0, &a);
        assert_eq!(b.position().0, a.catalog().len() - 1);
    }

    #[test]
    fn click_logs_image_coordinates() {
        let Some(a) = assets() else { return };
        let mut b = Browser::new();
        goto(&mut b, &a, "ARTIST.PIC", "6d01");
        let fx = b.update(
            &[Input::Click {
                button: MouseButton::Left,
                x: 320,
                y: 256,
            }],
            0.0,
            &a,
        );
        assert_eq!(fx, [Effect::Log("ARTIST.PIC 6d01 x=320 y=256".into())]);
        goto(&mut b, &a, "ARTIST.PIC", "024e");
        let img = a.image(&crate::assets::ImageRef::Pic(
            crate::assets::Program::Artist,
            0x24e,
        ));
        let (ox, oy) = image_origin(img.width, img.height);
        let fx = b.update(
            &[Input::Click {
                button: MouseButton::Left,
                x: ox + 1,
                y: oy + 2,
            }],
            0.0,
            &a,
        );
        assert_eq!(fx, [Effect::Log("ARTIST.PIC 024e x=1 y=2".into())]);
        assert!(
            b.update(
                &[Input::Click {
                    button: MouseButton::Left,
                    x: 0,
                    y: 470
                }],
                0.0,
                &a
            )
            .is_empty()
        );
    }

    #[test]
    fn enter_plays_sound() {
        let Some(a) = assets() else { return };
        let mut b = Browser::new();
        goto(&mut b, &a, "ARTIST.SND", "0209");
        let fx = b.update(&[key(Key::Enter)], 0.0, &a);
        assert_eq!(
            fx,
            [Effect::PlaySound(crate::assets::SoundRef::Snd(
                crate::assets::Program::Artist,
                0x209
            ))]
        );
    }

    #[test]
    fn escape_goes_back_and_ctrl_q_quits() {
        let (_d, a) = empty_assets();
        let mut b = Browser::new();
        assert_eq!(b.update(&[key(Key::Escape)], 0.0, &a), [Effect::Back]);
        let q = Input::Key {
            key: Key::Q,
            ctrl: true,
            alt: false,
        };
        assert_eq!(b.update(&[q], 0.0, &a), [Effect::Quit]);
    }

    #[test]
    fn draw_shows_info_bar_text() {
        let Some(a) = assets() else { return };
        let mut b = Browser::new();
        let mut f = Frame::new(SCREEN_W, SCREEN_H);
        b.update(&[Input::MouseMove { x: 300, y: 300 }], 0.0, &a);
        b.draw(&mut f, &a);
        let bar_lit = (0..SCREEN_W).any(|x| (0..16).any(|y| f.pixel(x, y)[..3] == [255, 255, 255]));
        assert!(bar_lit);
        assert!(b.info_line(&a).starts_with("ARTIST.PIC 1/2468 "));
    }
}
