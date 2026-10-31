//! Laço principal com a SDL3: janela, ampliação, eventos e execução dos efeitos.

use crate::audio::Mixer;
use crate::config::{Args, Config, ProgramChoice, Scale, drawings_dir, texts_dir};
use crate::platform::{clamped_point, integer_scale, key_allowed, logical_point, scale_cursor};
use engine::assets::{Assets, CursorRef};
use engine::scene::{Effect, Input, Key, MouseButton, Scene, SceneId};
use engine::scenes::{Browser, Chooser, Floor, Gallery, Paint, Room, Splash, Street, Writer};
use engine::{Frame, SCREEN_H, SCREEN_W};
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::{Keycode, Mod};
use sdl3::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl3::mouse::{Cursor, MouseButton as SdlButton, SystemCursor};
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::ScaleMode;
use sdl3::surface::Surface;
use sdl3::sys::render::SDL_RendererLogicalPresentation;
use std::time::{Duration, Instant};

const FRAME_TIME: Duration = Duration::from_micros(16_667);

fn fail(msg: &str) -> i32 {
    eprintln!("atelie-livre: {msg}");
    let _ = show_simple_message_box(MessageBoxFlag::ERROR, "Ateliê Livre", msg, None);
    1
}

fn make_scene(id: SceneId, cfg: &Config, assets: &Assets) -> Box<dyn Scene> {
    match id {
        SceneId::Paint => Box::new(Paint::new(drawings_dir(), assets)),
        SceneId::Gallery => Box::new(Gallery::new(drawings_dir(), assets)),
        SceneId::Building => Box::new(Room::new(
            Floor::Lobby,
            cfg.click_sound.clone(),
            drawings_dir(),
            assets,
        )),
        SceneId::Studio => Box::new(Room::new(
            Floor::Studio,
            cfg.click_sound.clone(),
            drawings_dir(),
            assets,
        )),
        SceneId::Chooser => Box::new(Chooser::new()),
        SceneId::Splash => Box::new(Splash::new(cfg.click_sound.clone())),
        SceneId::Street => Box::new(Street::new(cfg.click_sound.clone())),
        SceneId::Browser => Box::new(Browser::new()),
        SceneId::WriterSplash => Box::new(Splash::writer(cfg.click_sound.clone())),
        SceneId::Writer => Box::new(Writer::new(texts_dir(), assets)),
    }
}

/// Cena inicial: navegador, abertura do Creative Writer ou do Fine Artist.
fn first_scene(browser: bool, program: ProgramChoice) -> SceneId {
    match (browser, program) {
        (true, _) => SceneId::Browser,
        (false, ProgramChoice::Writer) => SceneId::WriterSplash,
        (false, ProgramChoice::Artist) => SceneId::Splash,
        (false, ProgramChoice::Ask) => SceneId::Chooser,
    }
}

fn map_key(k: Keycode) -> Key {
    match k {
        Keycode::Up => Key::Up,
        Keycode::Down => Key::Down,
        Keycode::Left => Key::Left,
        Keycode::Right => Key::Right,
        Keycode::PageUp => Key::PageUp,
        Keycode::PageDown => Key::PageDown,
        Keycode::Home => Key::Home,
        Keycode::End => Key::End,
        Keycode::Return | Keycode::KpEnter => Key::Enter,
        Keycode::Escape => Key::Escape,
        Keycode::Backspace => Key::Backspace,
        Keycode::F12 => Key::F12,
        Keycode::Q => Key::Q,
        _ => Key::Other,
    }
}

fn map_button(b: SdlButton) -> Option<MouseButton> {
    match b {
        SdlButton::Left => Some(MouseButton::Left),
        SdlButton::Right => Some(MouseButton::Right),
        SdlButton::Middle => Some(MouseButton::Middle),
        _ => None,
    }
}

/// Efeitos de `leave()`: só os `Log` fazem sentido na saída.
fn print_logs(effects: Vec<Effect>) {
    for e in effects {
        if let Effect::Log(s) = e {
            println!("{s}");
        }
    }
}

/// Cursor original ampliado pelo fator; cursor do sistema se faltar.
fn make_cursor(assets: &Assets, c: CursorRef, factor: u32) -> Option<Cursor> {
    let Some((img, (hx, hy))) = assets.cursor(&c) else {
        return Cursor::from_system(SystemCursor::Arrow).ok();
    };
    let big = scale_cursor(&img, factor);
    let mut surface = Surface::new(big.width, big.height, PixelFormat::RGBA32).ok()?;
    let pitch = surface.pitch() as usize;
    let row = (big.width * 4) as usize;
    surface.with_lock_mut(|px: &mut [u8]| {
        for y in 0..big.height as usize {
            px[y * pitch..y * pitch + row].copy_from_slice(&big.rgba[y * row..(y + 1) * row]);
        }
    });
    Cursor::from_surface(
        &surface,
        hx as i32 * factor as i32,
        hy as i32 * factor as i32,
    )
    .ok()
}

pub fn run(cfg: Config, args: &Args) -> i32 {
    let Some(dir) = cfg.data_dir.clone() else {
        return fail("indique a pasta MSKIDS com --data ou em data_dir no config.toml");
    };
    let assets = match Assets::open(&dir) {
        Ok(a) => a,
        Err(e) => return fail(&e.to_string()),
    };
    let sdl = match sdl3::init() {
        Ok(s) => s,
        Err(e) => return fail(&format!("SDL: {e}")),
    };
    let video = match sdl.video() {
        Ok(v) => v,
        Err(e) => return fail(&format!("vídeo: {e}")),
    };
    let mut mixer = match Mixer::new(&sdl, cfg.volume) {
        Ok(m) => Some(m),
        Err(e) => {
            eprintln!("aviso: sem áudio ({e}); seguindo mudo");
            None
        }
    };
    sdl3::hint::set("SDL_RENDER_VSYNC", "1");
    let mut window = match video
        .window("Ateliê Livre", SCREEN_W * 2, SCREEN_H * 2)
        .position_centered()
        .resizable()
        .build()
    {
        Ok(w) => w,
        Err(e) => return fail(&format!("janela: {e}")),
    };
    let mut fullscreen = cfg.fullscreen;
    if fullscreen && let Err(e) = window.set_fullscreen(true) {
        eprintln!("aviso: tela cheia indisponível ({e})");
    }
    let mut canvas = window.into_canvas();
    let presentation = match cfg.scale {
        Scale::Integer => SDL_RendererLogicalPresentation::INTEGER_SCALE,
        Scale::Fit => SDL_RendererLogicalPresentation::LETTERBOX,
    };
    if let Err(e) = canvas.set_logical_size(SCREEN_W, SCREEN_H, presentation) {
        return fail(&format!("apresentação lógica: {e}"));
    }
    let creator = canvas.texture_creator();
    let mut texture =
        match creator.create_texture_streaming(PixelFormat::RGBA32, SCREEN_W, SCREEN_H) {
            Ok(t) => t,
            Err(e) => return fail(&format!("textura: {e}")),
        };
    texture.set_scale_mode(match cfg.scale {
        Scale::Integer => ScaleMode::Nearest,
        Scale::Fit => ScaleMode::Linear,
    });
    let mut pump = match sdl.event_pump() {
        Ok(p) => p,
        Err(e) => return fail(&format!("eventos: {e}")),
    };
    // texto digitado chega como `Event::TextInput` (já com acentos compostos)
    video.text_input().start(canvas.window());

    let mut stack: Vec<Box<dyn Scene>> = vec![make_scene(
        first_scene(args.browser, cfg.program),
        &cfg,
        &assets,
    )];
    let mut frame = Frame::new(SCREEN_W, SCREEN_H);
    let mut cursor_ref: Option<CursorRef> = None;
    let mut cursor: Option<Cursor> = None;
    let mut cursor_factor = 0;
    let mut frames: u32 = 0;
    let mut last = Instant::now();
    let mut running = true;

    while running {
        let start = Instant::now();
        let dt = start.duration_since(last).as_secs_f32();
        last = start;
        let mut inputs = Vec::new();
        for event in pump.poll_iter() {
            let event = event.get_converted_coords(&canvas).unwrap_or(event);
            match event {
                Event::Quit { .. } => running = false,
                Event::KeyDown {
                    keycode: Some(k),
                    keymod,
                    repeat,
                    ..
                } => {
                    let ctrl = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
                    let alt = keymod.intersects(Mod::LALTMOD | Mod::RALTMOD);
                    let key = map_key(k);
                    if !key_allowed(key, repeat) {
                        continue;
                    }
                    if alt && matches!(k, Keycode::Return | Keycode::KpEnter) {
                        fullscreen = !fullscreen;
                        let _ = canvas.window_mut().set_fullscreen(fullscreen);
                    } else if k == Keycode::F12 && cfg.dev && stack.len() == 1 {
                        stack.push(make_scene(SceneId::Browser, &cfg, &assets));
                    } else {
                        inputs.push(Input::Key { key, ctrl, alt });
                    }
                }
                Event::TextInput { text, .. } => {
                    if !text.is_empty() {
                        inputs.push(Input::Text(text));
                    }
                }
                Event::MouseMotion { x, y, .. } => {
                    let (x, y) = clamped_point(x, y);
                    inputs.push(Input::MouseMove { x, y });
                }
                Event::MouseButtonDown {
                    mouse_btn, x, y, ..
                } => {
                    if let (Some(button), Some((x, y))) =
                        (map_button(mouse_btn), logical_point(x, y))
                    {
                        inputs.push(Input::Click { button, x, y });
                    }
                }
                Event::MouseButtonUp {
                    mouse_btn, x, y, ..
                } => {
                    // soltar fora do quadro (ou da janela) também termina o traço
                    if let Some(button) = map_button(mouse_btn) {
                        let (x, y) = clamped_point(x, y);
                        inputs.push(Input::MouseUp { button, x, y });
                    }
                }
                Event::Window {
                    win_event: WindowEvent::PixelSizeChanged(..),
                    ..
                } => cursor_factor = 0,
                _ => {}
            }
        }

        let effects = stack.last_mut().expect("cena").update(&inputs, dt, &assets);
        for effect in effects {
            match effect {
                Effect::PlaySound(r) => {
                    if let (Some(m), Some(pcm)) = (mixer.as_mut(), assets.sound(&r))
                        && let Err(e) = m.play(&pcm)
                    {
                        eprintln!("aviso: som {r:?}: {e}");
                    }
                }
                Effect::SetCursor(c) => {
                    cursor_ref = Some(c);
                    cursor_factor = 0;
                }
                Effect::ChangeScene(id) => {
                    let top = stack.last_mut().expect("cena");
                    print_logs(top.leave());
                    *top = make_scene(id, &cfg, &assets);
                }
                Effect::Back => {
                    if stack.len() > 1 {
                        let mut top = stack.pop().expect("cena");
                        print_logs(top.leave());
                    }
                }
                Effect::Log(s) => println!("{s}"),
                Effect::Quit => running = false,
            }
        }

        // cursores coloridos da SDL3 são medidos em pontos, não em pixels (telas HiDPI)
        let (w, h) = canvas.window().size();
        let factor = integer_scale(w, h);
        if let Some(c) = cursor_ref
            && factor != cursor_factor
        {
            cursor = make_cursor(&assets, c, factor);
            if let Some(cur) = &cursor {
                cur.set();
            }
            cursor_factor = factor;
        }

        stack.last().expect("cena").draw(&mut frame, &assets);
        if let Err(e) = texture.update(None, frame.pixels(), (SCREEN_W * 4) as usize) {
            eprintln!("aviso: textura: {e}");
        }
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        let _ = canvas.copy(&texture, None, None);
        canvas.present();

        frames += 1;
        if args.frames.is_some_and(|n| frames >= n) {
            break;
        }
        if let Some(rest) = FRAME_TIME.checked_sub(start.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    while let Some(mut scene) = stack.pop() {
        print_logs(scene.leave());
    }
    drop(cursor);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_scene_by_program() {
        assert_eq!(first_scene(false, ProgramChoice::Artist), SceneId::Splash);
        assert_eq!(
            first_scene(false, ProgramChoice::Writer),
            SceneId::WriterSplash
        );
        assert_eq!(first_scene(true, ProgramChoice::Writer), SceneId::Browser);
        assert_eq!(first_scene(false, ProgramChoice::Ask), SceneId::Chooser);
    }
}
