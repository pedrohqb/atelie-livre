//! Tocador dos roteiros de animação `ATAG` dos executáveis
//! (`docs/formats/atag.md`): bichos cruzando a tela, McZee saindo do buraco,
//! hidrante da rua etc.
//!
//! O roteiro traz as células (imagem, duração, som) e o caminho. No tipo 5 há
//! um deslocamento por célula: a célula `i` fica parada na posição acumulada
//! durante a sua duração e depois a posição salta `moves[i]`. No tipo 9 os
//! deslocamentos são deslizes interpolados em `tween` quadros; a célula
//! continua visível enquanto desliza. Uma célula de duração 0 só aparece
//! durante o deslize. Quando um movimento do tipo 5 traz `tween > 0`
//! (`ARTIST.EXE` 0x401a), o deslize vale também.
//!
//! As imagens dos roteiros são as versões de 1 bit; no `.PIC` da versão em
//! português a versão colorida é a imagem de id imediatamente anterior e a
//! máscara é `id + 1` ([`load_exe`] resolve isso por [`Assets::pic_prev_id`]).

use crate::assets::{Assets, ImageRef, Program, SoundRef};
use crate::{Frame, Image};
use formats::atag::{self, Script, Step};

/// Quadros por segundo dos deslizes interpolados (contagem de `tween`).
pub const TWEEN_FPS: f32 = 15.0;

/// Tolerância na soma dos tempos (evita que 0,3 + 0,2 fique aquém de 0,5).
const EPS: f32 = 1e-4;

/// Imagens de uma célula: a colorida e, se houver, a máscara (preto = opaco).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellImages {
    pub image: ImageRef,
    pub mask: Option<ImageRef>,
}

/// Em que fase do passo atual o tocador está.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// Célula parada; `t` é o tempo decorrido em segundos.
    Hold { t: f32 },
    /// Deslizando até a posição do passo seguinte; `t` em segundos.
    Slide { t: f32, total: f32 },
}

/// Tocador de um roteiro `ATAG`.
#[derive(Debug, Clone)]
pub struct Animator {
    script: Script,
    steps: Vec<Step>,
    images: Vec<CellImages>,
    program: Program,
    origin: (i32, i32),
    offset: (i32, i32),
    looping: bool,
    step: usize,
    /// Canto superior esquerdo do passo atual (sem o `offset`).
    pos: (i32, i32),
    phase: Phase,
    /// O som da célula atual ainda não foi emitido.
    pending_sound: bool,
    done: bool,
}

impl Animator {
    /// Cria o tocador; `images[i]` são as imagens da célula `i` (as que
    /// faltarem viram o quadriculado de imagem ausente). Os sons são
    /// recursos `SND` do `program`.
    pub fn new(script: Script, images: Vec<CellImages>, program: Program) -> Animator {
        let steps = script.playback();
        let origin = (script.origin.0 as i32, script.origin.1 as i32);
        let mut a = Animator {
            script,
            steps,
            images,
            program,
            origin,
            offset: (0, 0),
            looping: false,
            step: 0,
            pos: (0, 0),
            phase: Phase::Hold { t: 0.0 },
            pending_sound: true,
            done: false,
        };
        a.restart();
        a
    }

    /// Troca a posição inicial gravada no roteiro.
    pub fn with_origin(mut self, origin: (i32, i32)) -> Animator {
        self.origin = origin;
        self.restart();
        self
    }

    /// Deslocamento somado a todas as posições (para roteiros gravados em
    /// relação a um ponto, como o hidrante da rua).
    pub fn with_offset(mut self, offset: (i32, i32)) -> Animator {
        self.offset = offset;
        self
    }

    pub fn looping(mut self, looping: bool) -> Animator {
        self.looping = looping;
        self
    }

    pub fn script(&self) -> &Script {
        &self.script
    }

    pub fn is_finished(&self) -> bool {
        self.done
    }

    /// Volta ao primeiro passo.
    pub fn restart(&mut self) {
        self.step = 0;
        self.pos = self.step_pos(0);
        self.phase = Phase::Hold { t: 0.0 };
        self.pending_sound = true;
        self.done = self.steps.is_empty();
    }

    /// Posição do passo `k` (origem mais a posição acumulada do caminho).
    fn step_pos(&self, k: usize) -> (i32, i32) {
        match self.steps.get(k) {
            Some(s) => (self.origin.0 + s.x as i32, self.origin.1 + s.y as i32),
            None => self.origin,
        }
    }

    fn cell(&self, k: usize) -> Option<&atag::Cell> {
        self.steps
            .get(k)
            .and_then(|s| self.script.cells.get(s.cell as usize))
    }

    /// Duração da parada do passo `k`, em segundos.
    fn hold_secs(&self, k: usize) -> f32 {
        self.cell(k)
            .map(|c| c.duration_ms as f32 / 1000.0)
            .unwrap_or(0.0)
    }

    /// Deslize após o passo `k`: (segundos, dx, dy); `None` se é um salto.
    fn slide_after(&self, k: usize) -> Option<(f32, i32, i32)> {
        // nos roteiros com passos explícitos (.ANI) não há deslize
        if !self.script.steps.is_empty() {
            return None;
        }
        let m = self.script.moves.get(k)?;
        if m.tween == 0 {
            return None;
        }
        Some((m.tween as f32 / TWEEN_FPS, m.dx as i32, m.dy as i32))
    }

    /// Avança `dt` segundos; devolve os sons das células que começaram.
    pub fn update(&mut self, dt: f32) -> Vec<SoundRef> {
        let mut sounds = Vec::new();
        if self.done {
            return sounds;
        }
        let mut left = dt.max(0.0);
        // laço limitado: cada volta consome tempo ou avança um passo
        for _ in 0..(self.steps.len() * 2 + 4) {
            if self.pending_sound {
                self.pending_sound = false;
                if let Some(c) = self.cell(self.step)
                    && c.sound != 0
                {
                    sounds.push(SoundRef::Snd(self.program, c.sound));
                }
            }
            match self.phase {
                Phase::Hold { t } => {
                    let hold = self.hold_secs(self.step);
                    let t2 = t + left;
                    if t2 + EPS < hold {
                        self.phase = Phase::Hold { t: t2 };
                        return sounds;
                    }
                    left = (t2 - hold).max(0.0);
                    match self.slide_after(self.step) {
                        Some((total, _, _)) if total > 0.0 => {
                            self.phase = Phase::Slide { t: 0.0, total };
                        }
                        _ => {
                            if !self.advance() {
                                return sounds;
                            }
                        }
                    }
                }
                Phase::Slide { t, total } => {
                    let t2 = t + left;
                    if t2 + EPS < total {
                        self.phase = Phase::Slide { t: t2, total };
                        return sounds;
                    }
                    left = (t2 - total).max(0.0);
                    if !self.advance() {
                        return sounds;
                    }
                }
            }
        }
        sounds
    }

    /// Passa ao passo seguinte; `false` quando o roteiro acabou (sem laço).
    fn advance(&mut self) -> bool {
        self.step += 1;
        if self.step >= self.steps.len() {
            if self.looping {
                self.restart();
                return true;
            }
            self.step = self.steps.len().saturating_sub(1);
            self.done = true;
            return false;
        }
        self.pos = self.step_pos(self.step);
        self.phase = Phase::Hold { t: 0.0 };
        self.pending_sound = true;
        true
    }

    /// Índice da célula em exibição, ou `None` se nada está visível.
    pub fn current_cell(&self) -> Option<usize> {
        if self.done || self.steps.is_empty() {
            return None;
        }
        let step = self.steps.get(self.step)?;
        let cell = self.script.cells.get(step.cell as usize)?;
        // célula de duração 0 só aparece durante o deslize
        if cell.duration_ms == 0 && matches!(self.phase, Phase::Hold { .. }) {
            return None;
        }
        Some(step.cell as usize)
    }

    /// Canto superior esquerdo da célula em exibição (com o `offset`).
    pub fn position(&self) -> (i32, i32) {
        let (mut x, mut y) = self.pos;
        if let Phase::Slide { t, total } = self.phase
            && let Some((_, dx, dy)) = self.slide_after(self.step)
            && total > 0.0
        {
            let f = (t / total).clamp(0.0, 1.0);
            x += (dx as f32 * f).round() as i32;
            y += (dy as f32 * f).round() as i32;
        }
        (x + self.offset.0, y + self.offset.1)
    }

    pub fn draw(&self, frame: &mut Frame, assets: &Assets) {
        let Some(k) = self.current_cell() else {
            return;
        };
        let Some(ci) = self.images.get(k) else {
            return;
        };
        let (x, y) = self.position();
        let img = assets.image(&ci.image);
        match &ci.mask {
            Some(m) => blit_masked(frame, &img, &assets.image(m), x, y),
            None => frame.blit(&img, x, y),
        }
    }
}

/// Copia `img` onde a máscara é escura; tolera máscaras de tamanho ligeiramente
/// diferente (`ARTIST.PIC` 0x64dd é 1 px mais estreita que 0x64dc), usando só
/// a área comum.
pub fn blit_masked(f: &mut Frame, img: &Image, mask: &Image, x: i32, y: i32) {
    let w = img.width.min(mask.width);
    let h = img.height.min(mask.height);
    for dy in 0..h {
        for dx in 0..w {
            let mi = ((dy * mask.width + dx) * 4) as usize;
            let m = &mask.rgba[mi..mi + 3];
            if (m[0] as u32 + m[1] as u32 + m[2] as u32) < 384 {
                let i = ((dy * img.width + dx) * 4) as usize;
                let p = &img.rgba[i..i + 3];
                f.put(x + dx as i32, y + dy as i32, [p[0], p[1], p[2]]);
            }
        }
    }
}

/// Imagens de uma célula de roteiro de executável: a colorida é a imagem de id
/// anterior no `.PIC` (quando tem o mesmo tamanho) e a máscara é `id + 1`
/// (quando existe e tem o mesmo tamanho, a 1 px). Sem versão colorida, usa a
/// de 1 bit.
pub fn cell_images(assets: &Assets, program: Program, cell: &atag::Cell) -> CellImages {
    let bit = ImageRef::Pic(program, cell.image);
    let bit_img = assets.image(&bit);
    let image = assets
        .pic_prev_id(program, cell.image)
        .map(|id| ImageRef::Pic(program, id))
        .filter(|r| {
            let c = assets.image(r);
            (c.width, c.height) == (bit_img.width, bit_img.height)
        })
        .unwrap_or(bit);
    let mask_id = cell.image.wrapping_add(1);
    let mask = assets
        .pic_ids(program)
        .binary_search(&mask_id)
        .ok()
        .map(|_| ImageRef::Pic(program, mask_id))
        .filter(|r| {
            let m = assets.image(r);
            m.height.abs_diff(bit_img.height) <= 1 && m.width.abs_diff(bit_img.width) <= 1
        });
    CellImages { image, mask }
}

/// Carrega o recurso `ATAG` `id` do executável com as imagens do `.PIC`;
/// `None` se o recurso falta ou não decodifica.
pub fn load_exe(assets: &Assets, program: Program, id: u16) -> Option<Animator> {
    let data = assets.exe_resource(program, "ATAG", id)?;
    let script = atag::parse(&data).ok()?;
    let images = script
        .cells
        .iter()
        .map(|c| cell_images(assets, program, c))
        .collect();
    Some(Animator::new(script, images, program))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;
    use formats::atag::{Cell, Move, Source};

    fn cell(image: u16, ms: u32, sound: u16) -> Cell {
        Cell {
            image,
            mask: image + 3,
            offset: (0, 0),
            duration_ms: ms,
            sound,
        }
    }

    fn mv(dx: i16, dy: i16, tween: u8) -> Move {
        Move { dx, dy, tween }
    }

    fn script(kind: u16, cells: Vec<Cell>, moves: Vec<Move>) -> Script {
        Script {
            source: Source::Resource,
            flags: 0,
            cells,
            steps: Vec::new(),
            path_kind: kind,
            origin: (100, 50),
            moves,
        }
    }

    fn images(n: usize) -> Vec<CellImages> {
        (0..n)
            .map(|i| CellImages {
                image: ImageRef::Pic(Program::Artist, i as u16),
                mask: None,
            })
            .collect()
    }

    #[test]
    fn type5_holds_then_jumps_and_reports_sounds() {
        let s = script(
            5,
            vec![cell(1, 200, 0x20a), cell(2, 300, 0), cell(3, 100, 0x217)],
            vec![mv(-10, 5, 0), mv(-20, 0, 0), mv(0, 0, 0)],
        );
        let mut a = Animator::new(s, images(3), Program::Artist);
        assert_eq!(a.position(), (100, 50));
        assert_eq!(a.current_cell(), Some(0));
        let snd = a.update(0.0);
        assert_eq!(snd, vec![SoundRef::Snd(Program::Artist, 0x20a)]);
        assert!(a.update(0.1).is_empty());
        assert_eq!(a.current_cell(), Some(0));
        // 0,2 s: passa à célula 1 (sem som) já deslocada
        assert!(a.update(0.1).is_empty());
        assert_eq!(a.current_cell(), Some(1));
        assert_eq!(a.position(), (90, 55));
        // pula direto para a célula 2
        let snd = a.update(0.3);
        assert_eq!(snd, vec![SoundRef::Snd(Program::Artist, 0x217)]);
        assert_eq!(a.position(), (70, 55));
        assert!(!a.is_finished());
        a.update(0.1);
        assert!(a.is_finished());
        assert_eq!(a.current_cell(), None);
    }

    #[test]
    fn big_dt_crosses_several_cells_and_collects_sounds() {
        let s = script(
            5,
            vec![cell(1, 100, 1), cell(2, 100, 2), cell(3, 100, 3)],
            vec![mv(1, 0, 0), mv(1, 0, 0), mv(0, 0, 0)],
        );
        let mut a = Animator::new(s, images(3), Program::Artist);
        let snd = a.update(0.25);
        assert_eq!(
            snd,
            vec![
                SoundRef::Snd(Program::Artist, 1),
                SoundRef::Snd(Program::Artist, 2),
                SoundRef::Snd(Program::Artist, 3)
            ]
        );
        assert_eq!(a.current_cell(), Some(2));
        assert_eq!(a.position(), (102, 50));
    }

    #[test]
    fn type9_slides_and_zero_duration_cell_shows_only_while_sliding() {
        // como 0x3840: uma célula de duração 0 e um deslize de 20 quadros
        let s = script(9, vec![cell(1, 0, 0)], vec![mv(-300, -60, 15)]);
        let mut a = Animator::new(s, images(1), Program::Artist);
        a.update(0.0);
        assert_eq!(a.current_cell(), Some(0));
        a.update(0.5);
        assert_eq!(a.current_cell(), Some(0));
        assert_eq!(a.position(), (100 - 150, 50 - 30));
        a.update(0.5);
        assert!(a.is_finished());
        assert_eq!(a.current_cell(), None);
    }

    #[test]
    fn type9_with_durations_holds_then_slides() {
        // como 0x4010: células de 400 ms com deslizes de 4 quadros
        let s = script(
            9,
            vec![cell(1, 400, 0), cell(2, 400, 0)],
            vec![mv(-150, 0, 3), mv(0, 0, 0)],
        );
        let mut a = Animator::new(s, images(2), Program::Artist);
        a.update(0.3);
        assert_eq!(a.position(), (100, 50));
        // 0,5 s: 0,1 s dentro do deslize de 0,2 s
        a.update(0.2);
        assert_eq!(a.current_cell(), Some(0));
        assert_eq!(a.position(), (25, 50));
        a.update(0.1);
        assert_eq!(a.current_cell(), Some(1));
        assert_eq!(a.position(), (-50, 50));
    }

    #[test]
    fn looping_restarts_and_offset_origin_apply() {
        let s = script(5, vec![cell(1, 100, 7)], vec![mv(10, 0, 0)]);
        let mut a = Animator::new(s, images(1), Program::Artist)
            .looping(true)
            .with_origin((0, 0))
            .with_offset((5, 6));
        assert_eq!(a.update(0.0), vec![SoundRef::Snd(Program::Artist, 7)]);
        assert_eq!(a.position(), (5, 6));
        let snd = a.update(0.15);
        assert_eq!(snd, vec![SoundRef::Snd(Program::Artist, 7)]);
        assert!(!a.is_finished());
        assert_eq!(a.current_cell(), Some(0));
    }

    #[test]
    fn draw_without_images_does_not_panic_and_uses_placeholder() {
        let (_d, a) = empty_assets();
        let s = script(5, vec![cell(1, 100, 0)], vec![mv(0, 0, 0)]);
        let an = Animator::new(s, images(1), Program::Artist).with_origin((10, 10));
        let mut f = Frame::new(64, 64);
        f.fill([0, 0, 0]);
        an.draw(&mut f, &a);
        // quadriculado magenta do placeholder
        assert_eq!(f.pixel(10, 10), [255, 0, 255, 255]);
        // tocador sem células não quebra
        let empty = Animator::new(
            Script {
                source: Source::Resource,
                flags: 0,
                cells: Vec::new(),
                steps: Vec::new(),
                path_kind: 5,
                origin: (0, 0),
                moves: Vec::new(),
            },
            Vec::new(),
            Program::Artist,
        );
        assert!(empty.is_finished());
        empty.draw(&mut f, &a);
    }

    #[test]
    fn load_exe_without_resources_is_none() {
        let (_d, a) = empty_assets();
        assert!(load_exe(&a, Program::Artist, 0x67).is_none());
    }

    #[test]
    fn blit_masked_uses_common_area_of_mismatched_mask() {
        let img = Image {
            width: 2,
            height: 1,
            rgba: vec![1, 2, 3, 255, 4, 5, 6, 255],
        };
        let mask = Image {
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 255],
        };
        let mut f = Frame::new(2, 1);
        f.fill([9, 9, 9]);
        blit_masked(&mut f, &img, &mask, 0, 0);
        assert_eq!(f.pixel(0, 0), [1, 2, 3, 255]);
        assert_eq!(f.pixel(1, 0), [9, 9, 9, 255]);
    }
}
