# Ateliê Livre — Camada de plataforma (2a): plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Binário `atelie-livre` que abre a pasta `MSKIDS`, mostra a abertura e a rua do Fine Artist ampliadas com cursor e som originais, e um navegador de recursos para medir posições.

**Architecture:** `formats` ganha a conversão PKBM→RGBA com a regra de paleta (compartilhada com o `atelie-tools`). Novo crate `engine` (sem SDL): `Assets`, `Frame` 640×480, `Input`/`Effect`/`Scene` e as cenas. Novo crate `app` (binário `atelie-livre`): SDL3, configuração e execução dos efeitos.

**Tech Stack:** Rust 1.97 (edition 2024), `sdl3` 0.20 (feature `build-from-source-static`), `font8x8` 0.3, `toml` + `serde`, `clap`.

**Spec:** `docs/superpowers/specs/2026-10-31-plataforma-design.md`

## Global Constraints

- Licença GPL-3.0-or-later; edition 2024; `rust-version = "1.85"`; `#![forbid(unsafe_code)]` em `engine` e `app`.
- Só o crate `app` depende da SDL; `engine` e `formats` não fazem E/S de janela/som (o `Assets` lê arquivos, isso é permitido).
- Quadro lógico 640×480; coordenadas lógicas em `i32`.
- Esc não fecha o programa nas cenas para crianças; Ctrl+Q fecha; Alt+Enter alterna tela cheia.
- Cursor padrão: `CURSOR` 0x0001 de `ARTIST.EXE` (mão apontando).
- Abertura: `ARTIST.PIC` 0x6ca7; rua: `ARTIST.PIC` 0x6d01; abertura → rua por clique ou 5 s.
- Recurso ausente nunca derruba o programa (placeholder quadriculado magenta 32×32, ou do tamanho pedido).
- Toda tarefa termina com `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` passando (com `ATELIE_DATA=/home/claude/fa_cw/MSKIDS`; os testes de dados reais pulam sem ela).
- Commits em português com as linhas de atribuição da sessão.

## Review Focus

1. Cliques nas faixas pretas (fora de 640×480) → ignorados. Teste na Tarefa 7 (`logical_point`).
2. Instalação só com o Fine Artist, ou nomes em minúsculas → `Assets` abre e o catálogo lista só o que existe. Teste na Tarefa 3.
3. `config.toml` ausente, malformado ou com chave desconhecida → valores padrão e aviso, sem travar. Teste na Tarefa 6.
4. Janela redimensionada / Alt+Enter → cursor recriado no novo fator inteiro (mínimo 1). Teste na Tarefa 7 (`integer_scale`).
5. Navegador com setas além do início/fim, ou fonte vazia → limita sem pânico. Teste na Tarefa 5.

---

### Task 1: Conversão PKBM→RGBA compartilhada em `formats`

**Files:**
- Create: `crates/formats/src/render.rs`
- Modify: `crates/formats/src/lib.rs`, `crates/tools/src/extract.rs` (usar `render` em vez de `Palettes`/`chnk_palettes`/`write_bitmap` locais)

**Interfaces:**
- Produces:
  - `pub struct Palettes { pub pal16: Vec<Rgb>, pub pal256: Vec<Rgb> }`
  - `Palettes::from_exe(exe: Option<&[u8]>) -> Palettes` — `LOGPALETTE` 4 e 8 do NE; ausentes → rampa de cinzas (16 e 256, branco→preto).
  - `Palettes::for_chnk(&self, f: &chnk::ChunkFile) -> Palettes` — `CTBL` de 16 substitui `pal16`; de 256 substitui `pal256`.
  - `pub fn bitmap_to_rgba(b: &pkbm::Bitmap, pal: &Palettes) -> winres::Image` — paleta embutida → senão `default_palette_for`; índice fora da paleta → preto; alfa 255.

- [ ] **Step 1: Failing tests** em `render.rs`: `from_exe_none_gives_gray_ramps` (16 e 256 entradas, `[0]` branco, último preto); `bitmap_uses_embedded_palette_first`; `bitmap_falls_back_to_depth_palette` (1 bit → branco/preto; 4 bits → `pal16`); `index_outside_palette_is_black`; `for_chnk_replaces_matching_size` (CTBL de 16 troca só `pal16`).
- [ ] **Step 2: Run** `cargo test -p formats render` → FAIL (módulo ausente)
- [ ] **Step 3: Implement** `render.rs`; reescrever `extract.rs` para usar `formats::render` (remover duplicatas).
- [ ] **Step 4: Run** gate global → PASS (os testes do `atelie-tools`, inclusive `painting_layers_use_the_document_palette`, continuam verdes)
- [ ] **Step 5: Commit** "Conversão PKBM→RGBA compartilhada"

---

### Task 2: Crate `engine` — caminhos e `Frame`

**Files:**
- Create: `crates/engine/Cargo.toml` (deps `formats`, `font8x8 = "0.3"`; dev-dep `tempfile`), `crates/engine/src/lib.rs`, `crates/engine/src/paths.rs` (movido de `tools`), `crates/engine/src/frame.rs`
- Modify: `crates/tools/Cargo.toml` (dep `engine`), `crates/tools/src/*` (usar `engine::paths`; apagar `tools/src/paths.rs`, levando os testes junto)

**Interfaces:**
- Produces:
  - `engine::paths::{find_ci, walk_files, relative, safe_component}` (mesmas assinaturas de hoje)
  - `pub use formats::winres::Image;` em `engine`
  - `pub struct Frame` com `new(w: u32, h: u32)`, `width()`, `height()`, `pixels(&self) -> &[u8]` (RGBA), `pixel(x: u32, y: u32) -> [u8; 4]`, `fill(rgb: [u8; 3])`, `blit(&mut self, img: &Image, x: i32, y: i32)` (pula alfa 0, recorta), `text(&mut self, x: i32, y: i32, s: &str, rgb: [u8; 3])` (8×8, `font8x8::BASIC_FONTS`/`LATIN_FONTS`, bit 0 = pixel mais à esquerda, caractere sem glifo → `?`, avanço de 8 px)
  - `pub const SCREEN_W: u32 = 640; pub const SCREEN_H: u32 = 480;`

- [ ] **Step 1: Failing tests** em `frame.rs`: `blit_clips_all_edges` (imagem 4×4 em (−2,−2), (638,478) e (700,0): só pixels dentro mudam, sem pânico); `blit_skips_transparent`; `fill_sets_every_pixel`; `text_draws_glyph` (`"A"` em (0,0): ao menos um pixel aceso na linha 1 e nenhum fora de 8×8); `text_handles_accents_and_unknown` (`"ã"` acende pixels; `"\u{4e2d}"` desenha o mesmo que `"?"`).
- [ ] **Step 2: Run** → FAIL
- [ ] **Step 3: Implement**; mover `paths.rs` e seus testes.
- [ ] **Step 4: Run** gate → PASS
- [ ] **Step 5: Commit** "Crate engine: caminhos e quadro"

---

### Task 3: `engine::Assets` e catálogo

**Files:**
- Create: `crates/engine/src/assets.rs`
- Test: unitários + `crates/engine/tests/real_assets.rs`

**Interfaces:**
- Consumes: `formats::{ne, glob, pkbm, chnk, wave, winres, render}`, `engine::paths::{find_ci, walk_files}`
- Produces:
  - `pub enum Program { Artist, Writer }`
  - `pub enum ImageRef { Pic(Program, u16), Library { file: String, index: usize }, Icon(Program, u16) }` (derive `Clone, Debug, PartialEq, Eq, Hash`)
  - `pub enum SoundRef { Snd(Program, u16), Library { file: String, index: usize } }` (idem)
  - `pub struct CursorRef(pub Program, pub u16);` (idem)
  - `pub enum ItemKind { Image(ImageRef), Sound(SoundRef), Cursor(CursorRef) }`, `pub struct Item { pub label: String, pub kind: ItemKind }`, `pub struct Source { pub name: String, pub items: Vec<Item> }`
  - `pub enum AssetsError { NotFound(PathBuf), NotInstallation(PathBuf) }` (Display em português)
  - `Assets::open(dir: &Path) -> Result<Assets, AssetsError>` — válido se achar `ARTIST.EXE` ou `WRITER.EXE` (sem distinguir maiúsculas); carrega na abertura os executáveis, `.PIC`, `.SND` e os `CHNK` de `SHARED/STUFF`.
  - `Assets::catalog(&self) -> &[Source]` — ordem: `ARTIST.PIC`, `WRITER.PIC`, cada biblioteca de `SHARED/STUFF` com imagens ou sons (ordem alfabética, nome = arquivo), `Ícones` (dos dois EXE), `Cursores` (dos dois EXE), `ARTIST.SND`, `WRITER.SND`; fontes de arquivos ausentes não aparecem; fontes sem itens não aparecem.
  - `Assets::image(&self, r: &ImageRef) -> Rc<Image>` — cache interno (`RefCell<HashMap>`); falha → `placeholder(32, 32)` e aviso em stderr (uma vez por recurso).
  - `Assets::sound(&self, r: &SoundRef) -> Option<Rc<wave::Pcm>>`
  - `Assets::cursor(&self, c: &CursorRef) -> Option<(Rc<Image>, (u16, u16))>`
  - `pub fn placeholder(w: u32, h: u32) -> Image` — quadriculado magenta/preto de 4 px.
  - Imagens de bibliotecas: na ordem de `chnk` (recursivo nos aninhados), rótulo `"<TAG> <id> <nome>"`; paleta: `Palettes::from_exe(ARTIST.EXE)` (ou WRITER) `.for_chnk(arquivo)`.

- [ ] **Step 1: Failing tests** — unitários: `open_missing_dir_is_not_found`; `open_dir_without_exe_is_not_installation`; `placeholder_is_checkered`. Reais (`real_assets.rs`, pulam sem `ATELIE_DATA`): `opens_real_data` (catálogo começa com `ARTIST.PIC` com 2468 itens); `splash_and_street_are_full_screen` (0x6ca7 e 0x6d01 com 640×480); `default_cursor_has_hotspot_inside` (`CursorRef(Artist, 1)`); `sound_0x209_decodes`; `missing_image_gives_placeholder` (`Pic(Artist, 0x0001)` inexistente → 32×32); `artist_only_install_lists_only_artist` (cópia temporária com `ARTIST.EXE` + `ARTIST/` em minúsculas → sem `WRITER.PIC`/`WRITER.SND` no catálogo).
- [ ] **Step 2: Run** → FAIL
- [ ] **Step 3: Implement** `assets.rs`
- [ ] **Step 4: Run** gate → PASS
- [ ] **Step 5: Commit** "Assets e catálogo de recursos"

---

### Task 4: Interface de cenas, abertura e rua

**Files:**
- Create: `crates/engine/src/scene.rs`, `crates/engine/src/scenes/mod.rs`, `crates/engine/src/scenes/splash.rs`, `crates/engine/src/scenes/street.rs`

**Interfaces:**
- Produces:
  - `pub enum MouseButton { Left, Right, Middle }`; `pub enum Key { Up, Down, Left, Right, PageUp, PageDown, Home, End, Enter, Escape, F12, Q, Other }`
  - `pub enum Input { MouseMove { x: i32, y: i32 }, Click { button: MouseButton, x: i32, y: i32 }, Key { key: Key, ctrl: bool, alt: bool } }`
  - `pub enum SceneId { Splash, Street, Browser }`
  - `pub enum Effect { PlaySound(SoundRef), SetCursor(CursorRef), ChangeScene(SceneId), Back, Log(String), Quit }`
  - `pub trait Scene { fn update(&mut self, input: &[Input], dt: f32, assets: &Assets) -> Vec<Effect>; fn draw(&self, frame: &mut Frame, assets: &Assets); }`
  - `pub fn kid_keys(input: &[Input]) -> Vec<Effect>` — Ctrl+Q → `Quit`; Esc → nada (usado pelas cenas para crianças)
  - `Splash::new(click: Option<SoundRef>)`, `Street::new(click: Option<SoundRef>)`; ambas emitem `SetCursor(CursorRef(Artist, 1))` no primeiro `update`, `PlaySound(click)` a cada clique esquerdo (se houver); `Splash` emite `ChangeScene(Street)` no clique ou quando o `dt` acumulado ≥ 5,0 s (uma vez só).
  - `pub const DEFAULT_CURSOR: CursorRef = CursorRef(Program::Artist, 1);`

- [ ] **Step 1: Failing tests**: `splash_click_goes_to_street`; `splash_timeout_goes_to_street_once` (5 updates de 1,0 s → um único `ChangeScene`; mais updates → nenhum); `first_update_sets_cursor`; `click_plays_configured_sound` (e nenhum `PlaySound` com `None`); `ctrl_q_quits_escape_does_not`; `splash_draws_title` (com `ATELIE_DATA`: pixel do quadro igual ao pixel (320,240) da imagem 0x6ca7).
- [ ] **Step 2: Run** → FAIL
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** gate → PASS
- [ ] **Step 5: Commit** "Cenas de abertura e rua"

---

### Task 5: Navegador de recursos

**Files:**
- Create: `crates/engine/src/scenes/browser.rs`

**Interfaces:**
- Consumes: `Assets::catalog/image/sound/cursor`, `Frame::{fill, blit, text}`, `Scene`, `Input`, `Effect`
- Produces: `Browser::new()`; `pub fn image_origin(img_w: u32, img_h: u32) -> (i32, i32)` (centraliza na área abaixo da faixa de informação de 16 px: x = (640−w)/2, y = 16 + (464−h)/2, mínimo (0, 16)).

Comportamento: ↑/↓ fonte (limita nas pontas), ←/→ item (limita), PageUp/PageDown ±10, Home/End; Enter num som → `PlaySound`; clique esquerdo dentro da imagem → `Log("<fonte> <rótulo> x=<x> y=<y>")` com coordenadas na imagem; Esc → `Back`; Ctrl+Q → `Quit`. Desenho: fundo quadriculado cinza; faixa superior preta com texto `"<fonte> <i+1>/<n> <rótulo> <w>x<h>"` (imagens) ou `"<fonte> <i+1>/<n> <rótulo> <taxa> Hz <seg> s"` (sons) e, se o mouse estiver sobre a imagem, `" @ x,y"`. Cursores exibidos como imagem com o hotspot no rótulo.

- [ ] **Step 1: Failing tests** (catálogo sintético via um `Assets` de teste? Não: usar `ATELIE_DATA`; com ele ausente, os testes pulam): `arrows_change_source_and_item`; `navigation_clamps_at_ends` (100× ← e ↑ a partir do início não entram em pânico e ficam em 0/0; 10 000× → para no último); `click_logs_image_coordinates` (abrir `ARTIST.PIC`, ir ao item de id 0x6d01, clicar em (320, 256) → `Log` com `x=320 y=240`); `enter_plays_sound` (fonte `ARTIST.SND`); `escape_goes_back`; e unitário sem dados: `image_origin_centers_below_bar` (640×480 → (0,16); 100×50 → (270, 223)).
- [ ] **Step 2: Run** → FAIL
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** gate → PASS
- [ ] **Step 5: Commit** "Navegador de recursos"

---

### Task 6: `app` — configuração

**Files:**
- Create: `crates/app/Cargo.toml` (`[[bin]] name = "atelie-livre"`; deps `engine`, `clap` derive, `serde` derive, `toml`), `crates/app/src/main.rs` (por ora só parse e impressão da configuração efetiva), `crates/app/src/config.rs`

**Interfaces:**
- Produces:
  - `pub enum Scale { Integer, Fit }`
  - `pub struct Config { pub data_dir: Option<PathBuf>, pub scale: Scale, pub fullscreen: bool, pub volume: u8, pub click_sound: Option<SoundRef>, pub dev: bool }` — padrões: `None`, `Integer`, `true`, `80`, `None`, `false`
  - `pub struct Args { data: Option<PathBuf>, window: bool, scale: Option<Scale>, browser: bool, frames: Option<u32> /* oculto */ }` (clap)
  - `pub fn config_path() -> PathBuf` — `$XDG_CONFIG_HOME/atelie-livre/config.toml` ou `~/.config/atelie-livre/config.toml`
  - `pub fn parse_config(text: &str) -> (Config, Vec<String>)` — avisos para TOML inválido (→ padrões) e chaves desconhecidas; `volume` limitado a 0–100; `click_sound = { program = "artist", id = 0x209 }`
  - `pub fn effective(cfg: Config, args: &Args) -> Config` — argumentos têm prioridade

- [ ] **Step 1: Failing tests**: `empty_file_gives_defaults`; `invalid_toml_gives_defaults_and_warning`; `unknown_key_warns_but_keeps_rest`; `volume_is_clamped`; `click_sound_parses`; `args_override_config` (`--window` → `fullscreen=false`, `--scale fit`, `--data`).
- [ ] **Step 2: Run** → FAIL
- [ ] **Step 3: Implement**
- [ ] **Step 4: Run** gate → PASS
- [ ] **Step 5: Commit** "Configuração do aplicativo"

---

### Task 7: `app` — plataforma SDL3

**Files:**
- Create: `crates/app/src/platform.rs` (funções puras testáveis), `crates/app/src/audio.rs`, `crates/app/src/run.rs`
- Modify: `crates/app/Cargo.toml` (dep `sdl3 = { version = "0.20", features = ["build-from-source-static"] }`), `crates/app/src/main.rs`
- Test: unitários em `platform.rs`; `crates/app/tests/smoke.rs`

**Interfaces:**
- Consumes: `Config`, `Args`, `Assets`, `Scene`, `Splash`, `Street`, `Browser`, `Frame`, `Effect`
- Produces (puras, em `platform.rs`):
  - `pub fn integer_scale(win_w: u32, win_h: u32) -> u32` — `max(1, min(win_w / 640, win_h / 480))`
  - `pub fn logical_point(x: f32, y: f32) -> Option<(i32, i32)>` — `None` fora de `[0,640)×[0,480)`
  - `pub fn scale_cursor(img: &Image, factor: u32) -> Image` — vizinho mais próximo
  - `pub fn key_from_sdl(...)` fica em `run.rs` (depende da SDL)

Comportamento (`run.rs`): SDL com vídeo e áudio (falha do áudio → mudo com aviso); janela "Ateliê Livre" 1280×960, tela cheia se `fullscreen`; canvas com vsync; `set_logical_size(640, 480, INTEGER_SCALE | LETTERBOX)` conforme `Scale` (`Fit` com escala linear na textura, `Integer` com vizinho mais próximo); textura streaming RGBA 640×480 atualizada com `Frame::pixels` a cada quadro; eventos convertidos com `get_converted_coords` e `logical_point`; Alt+Enter alterna tela cheia; F12 abre o navegador se `dev`; `--browser` começa nele; `Effect::Back` volta à cena anterior; cursor recriado quando `integer_scale` muda; sem pasta válida → `show_simple_message_box` + stderr + código 1 (com `SDL_VIDEODRIVER=dummy`, só stderr); `--frames N` encerra após N quadros com código 0.
`audio.rs`: `Mixer` com até 8 `AudioStream` (um por som, spec = taxa/canais do `Pcm`, S16), ligados ao dispositivo padrão; ao exceder 8, o mais antigo é destruído; volume aplicado por ganho no fluxo.

- [ ] **Step 1: Failing tests**: `integer_scale_minimum_one` (300×200 → 1; 1920×1080 → 2; 2560×1440 → 3); `logical_point_rejects_bars` ((−1, 10), (640, 10), (10, 480) → `None`; (639.9, 479.9) → `Some((639, 479))`); `scale_cursor_doubles` (2×2 → 4×4, pixel copiado). `smoke.rs` (pula sem `ATELIE_DATA`): roda o binário com `SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy --data $ATELIE_DATA --window --frames 120` → código 0; e com `--data /tmp/nao-existe` → código 1 e stderr não vazio.
- [ ] **Step 2: Run** → FAIL
- [ ] **Step 3: Implement** (a primeira compilação da SDL3 pode levar minutos; se faltarem cabeçalhos de Wayland/X11 nesta máquina, a SDL compila só com os drivers disponíveis — `dummy` basta para o teste de fumaça)
- [ ] **Step 4: Run** gate → PASS
- [ ] **Step 5: Commit** "Plataforma SDL3: janela, ampliação, cursor e som"

---

### Task 8: Documentação e verificação final

**Files:**
- Modify: `README.md` (seção "Rodar o Ateliê Livre": dependências de compilação por distro — CachyOS/Arch: `cmake`, `wayland`, `libxkbcommon`, `libx11`, `libxext`, `libxrandr`, `libxcursor`, `libxi`, `libpulse`/`pipewire`, `alsa-lib`; Fedora/Bluefin em distrobox: equivalentes `-devel`; comando `cargo run --release -p app -- --data …`; teclas: Alt+Enter, Ctrl+Q, F12; `config.toml` de exemplo)

- [ ] **Step 1:** rodar o teste de fumaça manualmente e registrar a saída
- [ ] **Step 2:** escrever a seção do README
- [ ] **Step 3:** gate global → PASS
- [ ] **Step 4: Commit and push** "Documentação do subprojeto 2a"
