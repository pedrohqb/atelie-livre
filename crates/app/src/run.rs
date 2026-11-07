//! Laço principal com a SDL3: janela, ampliação, eventos e execução dos efeitos.

use crate::audio::Mixer;
use crate::config::{Args, Config, ProgramChoice, Scale, drawings_dir, texts_dir, users_dir};
use crate::platform::{clamped_point, integer_scale, key_allowed, logical_point, scale_cursor};
use engine::assets::{Assets, CursorRef};
use engine::scene::{Effect, Input, Key, MouseButton, Scene, SceneId};
use engine::scenes::room::DEFAULT_USER;
use engine::scenes::{
    Browser, Chooser, Entry, Floor, GalleryRoom, Imaginopolis, Paint, Room, Splash, Story, Street,
    Writer,
};
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

/// Usuário escolhido na Entrada (ou dado na configuração), para o `@` dos balões.
#[derive(Debug, Clone, Default)]
struct UserContext {
    user: Option<String>,
}

impl UserContext {
    fn name(&self) -> &str {
        self.user.as_deref().unwrap_or(DEFAULT_USER)
    }
}

fn make_scene(id: SceneId, cfg: &Config, user: &UserContext, assets: &Assets) -> Box<dyn Scene> {
    // o nome da sessão é o texto inicial das palavras com forma
    engine::scenes::paint::set_user_name(user.name());
    let room = |floor| {
        Box::new(
            Room::new(floor, cfg.click_sound.clone(), drawings_dir(), assets)
                .with_user(user.name(), assets),
        )
    };
    match id {
        SceneId::Paint => Box::new(Paint::new(drawings_dir(), assets).with_user(user.name())),
        SceneId::PaintNew => {
            Box::new(Paint::new_blank(drawings_dir(), assets).with_user(user.name()))
        }
        SceneId::Gallery => Box::new(GalleryRoom::new(
            cfg.click_sound.clone(),
            drawings_dir(),
            assets,
        )),
        SceneId::Building => room(Floor::Lobby),
        SceneId::Studio => room(Floor::Studio),
        SceneId::TricksFloor => room(Floor::Tricks),
        SceneId::Projects => room(Floor::Projects),
        SceneId::Tricks(kind) => Box::new(Paint::new_tricks(kind, drawings_dir(), assets)),
        SceneId::Project(kind) => Box::new(Paint::new_project(kind, drawings_dir(), assets)),
        SceneId::FlipbookFigure => Box::new(Paint::new_flipbook_figure(drawings_dir(), assets)),
        SceneId::Chooser => Box::new(Chooser::new()),
        SceneId::Splash => Box::new(Splash::new(cfg.click_sound.clone())),
        SceneId::Entry => {
            let e = Entry::new(users_dir(), cfg.click_sound.clone(), assets);
            Box::new(match &cfg.user {
                Some(u) => e.with_user(u),
                None => e,
            })
        }
        SceneId::Story => Box::new(Story::new(cfg.click_sound.clone(), assets)),
        SceneId::BookStory => Box::new(Story::new(cfg.click_sound.clone(), assets).over_room()),
        SceneId::Portfolio => Box::new(GalleryRoom::portfolio(
            cfg.click_sound.clone(),
            drawings_dir(),
            assets,
        )),
        SceneId::Imaginopolis => Box::new(Imaginopolis::new(cfg.click_sound.clone())),
        SceneId::Street => Box::new(Street::new(cfg.click_sound.clone())),
        SceneId::Browser => Box::new(Browser::new()),
        SceneId::WriterSplash => Box::new(Splash::writer(cfg.click_sound.clone())),
        SceneId::Writer => Box::new(Writer::new(texts_dir(), assets)),
    }
}

/// Cena inicial: navegador, abertura do Creative Writer ou do Fine Artist.
/// Cena pedida por `--scene` (ferramentas de teste).
fn scene_by_name(name: &str) -> Option<SceneId> {
    use engine::scenes::projects::ProjectKind;
    use engine::scenes::wizard::TrickKind;
    Some(match name {
        "entry" => SceneId::Entry,
        "story" => SceneId::Story,
        "imaginopolis" => SceneId::Imaginopolis,
        "street" => SceneId::Street,
        "lobby" | "building" => SceneId::Building,
        "studio" => SceneId::Studio,
        "projects" => SceneId::Projects,
        "tricks" => SceneId::TricksFloor,
        "gallery" => SceneId::Gallery,
        "paint" => SceneId::Paint,
        "paint-new" => SceneId::PaintNew,
        "comics" => SceneId::Project(ProjectKind::Comics),
        "button" => SceneId::Project(ProjectKind::Button),
        "sticker" => SceneId::Project(ProjectKind::Sticker),
        "flipbook" => SceneId::Project(ProjectKind::Flipbook),
        "flipbook-figure" => SceneId::FlipbookFigure,
        "tricks-basic" => SceneId::Tricks(TrickKind::Basic),
        "tricks-3d" => SceneId::Tricks(TrickKind::ThreeD),
        _ => return None,
    })
}

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
        Keycode::Delete => Key::Delete,
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

    let mut user = UserContext {
        user: cfg.user.clone(),
    };
    let first = match args.scene.as_deref().map(scene_by_name) {
        Some(Some(id)) => id,
        Some(None) => return fail("cena desconhecida em --scene"),
        None => first_scene(args.browser, cfg.program),
    };
    let mut stack: Vec<Box<dyn Scene>> = vec![make_scene(first, &cfg, &user, &assets)];
    let mut frame = Frame::new(SCREEN_W, SCREEN_H);
    let mut cursor_ref: Option<CursorRef> = None;
    let mut cursor: Option<Cursor> = None;
    let mut cursor_factor = 0;
    let mut frames: u32 = 0;
    let mut last = Instant::now();
    let mut running = true;
    // ATELIE_TRACE=1 imprime cada efeito (som, troca de cena) — usado pelos testes de varredura
    let trace = std::env::var_os("ATELIE_TRACE").is_some();

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
                    if matches!(k, Keycode::LShift | Keycode::RShift) {
                        if !repeat {
                            inputs.push(Input::Shift(true));
                        }
                        continue;
                    }
                    if matches!(k, Keycode::LCtrl | Keycode::RCtrl) {
                        if !repeat {
                            inputs.push(Input::Ctrl(true));
                        }
                        continue;
                    }
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
                        stack.push(make_scene(SceneId::Browser, &cfg, &user, &assets));
                    } else {
                        inputs.push(Input::Key { key, ctrl, alt });
                    }
                }
                Event::KeyUp {
                    keycode: Some(Keycode::LShift | Keycode::RShift),
                    ..
                } => inputs.push(Input::Shift(false)),
                Event::KeyUp {
                    keycode: Some(Keycode::LCtrl | Keycode::RCtrl),
                    ..
                } => inputs.push(Input::Ctrl(false)),
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
        if let Some(m) = mixer.as_mut()
            && let Err(e) = m.tick()
        {
            eprintln!("aviso: som em laço: {e}");
        }
        for effect in effects {
            if trace {
                trace_effect(&effect);
            }
            match effect {
                Effect::PlaySound(r) => {
                    if let (Some(m), Some(pcm)) = (mixer.as_mut(), assets.sound(&r))
                        && let Err(e) = m.play(&pcm)
                    {
                        eprintln!("aviso: som {r:?}: {e}");
                    }
                }
                Effect::PlayLoop(r) => {
                    if let (Some(m), Some(pcm)) = (mixer.as_mut(), assets.sound(&r))
                        && let Err(e) = m.play_loop(pcm)
                    {
                        eprintln!("aviso: som {r:?}: {e}");
                    }
                }
                Effect::StopLoop => {
                    if let Some(m) = mixer.as_mut() {
                        m.stop_loop();
                    }
                }
                Effect::SetCursor(c) => {
                    cursor_ref = Some(c);
                    cursor_factor = 0;
                }
                Effect::ChangeScene(id) => {
                    // o laço é da cena que sai (traço interrompido)
                    if let Some(m) = mixer.as_mut() {
                        m.stop_loop();
                    }
                    // as cenas guardadas por baixo (Push) também saem
                    while stack.len() > 1 {
                        let mut s = stack.pop().expect("cena");
                        print_logs(s.leave());
                    }
                    let top = stack.last_mut().expect("cena");
                    print_logs(top.leave());
                    *top = make_scene(id, &cfg, &user, &assets);
                }
                Effect::Push(id) => stack.push(make_scene(id, &cfg, &user, &assets)),
                Effect::Back => {
                    if let Some(m) = mixer.as_mut() {
                        m.stop_loop();
                    }
                    if stack.len() > 1 {
                        let mut top = stack.pop().expect("cena");
                        print_logs(top.leave());
                    }
                }
                Effect::SetUser(u) => user.user = Some(u),
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

        let top = stack.last().expect("cena");
        if top.overlay() && stack.len() > 1 {
            stack[stack.len() - 2].draw(&mut frame, &assets);
        }
        top.draw(&mut frame, &assets);
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

/// Linha de rastreio por efeito, para ferramentas externas (varredura de cliques).
fn trace_effect(effect: &Effect) {
    match effect {
        Effect::PlaySound(r) => println!("TRACE som {r:?}"),
        Effect::PlayLoop(r) => println!("TRACE laço {r:?}"),
        Effect::StopLoop => println!("TRACE fim-laço"),
        Effect::ChangeScene(id) => println!("TRACE cena {id:?}"),
        Effect::Push(id) => println!("TRACE push {id:?}"),
        Effect::Back => println!("TRACE back"),
        Effect::Quit => println!("TRACE quit"),
        _ => {}
    }
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
