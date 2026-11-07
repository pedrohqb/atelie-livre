//! Camada de objetos sobre o quadro: adesivos e palavras colocados não viram
//! pixels — ficam por cima da pintura, em ordem (z), com escala, inversões,
//! giros de 90°, correntes (objetos grudados) e som preso. O pincel e a lata
//! pintam o fundo, por baixo deles. Spec: `docs/superpowers/specs/2026-10-31-objetos-design.md`.

use super::canvas::{BLACK, Canvas, Ink, ORIGIN, WHITE};
use super::shaped::{Shadow, ShapedWords, WordShape};
use super::stamp::Stamp;
use super::text::{Align, BalloonShape, WordsStyle, text_pixels, words_layout};
use crate::assets::SoundRef;
use formats::fnt::Font;
use std::rc::Rc;

/// Menor lado de um objeto, em pixels.
pub const MIN_SIZE: i32 = 4;

/// Corrente entre dois objetos: centro de um, centro do outro e id do que guarda a ligação.
pub type Chain = ((i32, i32), (i32, i32), u32);

/// De onde o objeto veio, para gravar e reabrir editável.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Adesivo de um rolo: (rolo, casa); rolo 0 = bichos.
    Roll { roll: usize, item: usize },
    /// Adesivo do usuário: arquivo PNG na pasta de desenhos.
    Png(String),
    /// Palavra: texto, estilo (balão, posição, largura, fonte) e índice da cor.
    Words {
        text: String,
        style: WordsStyle,
        color: u8,
    },
    /// Palavra com forma (Cavalete de palavras com forma).
    Shaped(Box<ShapedWords>),
    /// Dipitidu (ponteiro de balão) do rolo da Palavra: casa no rolo.
    Tail(usize),
}

#[derive(Debug, Clone)]
pub struct Object {
    pub id: u32,
    /// Canto superior esquerdo, coordenadas do quadro.
    pub x: i32,
    pub y: i32,
    /// Tamanho mostrado (a imagem é esticada pelo vizinho mais próximo).
    pub w: i32,
    pub h: i32,
    pub flip_h: bool,
    pub flip_v: bool,
    /// Giros de 90° no sentido horário (0..4).
    pub turns: u8,
    /// Objeto grudado por uma corrente.
    pub link: Option<u32>,
    pub sound: Option<SoundRef>,
    /// Imagem base, sem escala nem giro.
    pub image: Rc<Stamp>,
    pub source: Source,
}

impl PartialEq for Object {
    fn eq(&self, o: &Object) -> bool {
        self.id == o.id
            && (self.x, self.y, self.w, self.h) == (o.x, o.y, o.w, o.h)
            && (self.flip_h, self.flip_v, self.turns) == (o.flip_h, o.flip_v, o.turns)
            && self.link == o.link
            && self.sound == o.sound
            && self.source == o.source
            && Rc::ptr_eq(&self.image, &o.image)
    }
}

impl Object {
    pub fn new(id: u32, image: Rc<Stamp>, x: i32, y: i32, source: Source) -> Object {
        let (w, h) = (image.w as i32, image.h as i32);
        Object {
            id,
            x,
            y,
            w,
            h,
            flip_h: false,
            flip_v: false,
            turns: 0,
            link: None,
            sound: None,
            image,
            source,
        }
    }

    pub fn rect(&self) -> (i32, i32, i32, i32) {
        (self.x, self.y, self.w, self.h)
    }

    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    /// Tamanho da imagem depois dos giros (largura e altura trocam nos ímpares).
    fn turned_size(&self) -> (i32, i32) {
        let (w, h) = (self.image.w as i32, self.image.h as i32);
        if self.turns % 2 == 1 { (h, w) } else { (w, h) }
    }

    /// Índice da paleta no pixel `(ox, oy)` relativo ao canto do objeto; `None` = transparente.
    pub fn pixel(&self, ox: i32, oy: i32) -> Option<u8> {
        if ox < 0 || oy < 0 || ox >= self.w || oy >= self.h || self.w <= 0 || self.h <= 0 {
            return None;
        }
        let (bw, bh) = self.turned_size();
        let mut sx = (ox as i64 * bw as i64 / self.w as i64) as i32;
        let mut sy = (oy as i64 * bh as i64 / self.h as i64) as i32;
        if self.flip_h {
            sx = bw - 1 - sx;
        }
        if self.flip_v {
            sy = bh - 1 - sy;
        }
        let (iw, ih) = (self.image.w as i32, self.image.h as i32);
        let (ix, iy) = match self.turns % 4 {
            0 => (sx, sy),
            1 => (sy, ih - 1 - sx),
            2 => (iw - 1 - sx, ih - 1 - sy),
            _ => (iw - 1 - sy, sx),
        };
        if ix < 0 || iy < 0 || ix >= iw || iy >= ih {
            return None;
        }
        self.image.px[(iy * iw + ix) as usize]
    }

    /// Despeja tinta dentro do objeto, no ponto `(ox, oy)` relativo ao canto:
    /// a área da mesma cor ligada ao ponto muda para a tinta. A imagem passa
    /// a ser a que se vê (com escala, giros e inversões aplicados), sem eles.
    /// Devolve `false` se o ponto é transparente ou nada mudou.
    pub fn fill_at(&mut self, ox: i32, oy: i32, ink: &Ink) -> bool {
        let Some(target) = self.pixel(ox, oy) else {
            return false;
        };
        let (w, h) = (self.w.max(1), self.h.max(1));
        let mut px: Vec<Option<u8>> = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| self.pixel(x, y))
            .collect();
        let mut changed = false;
        let mut seen = vec![false; px.len()];
        let mut stack = vec![(ox, oy)];
        seen[(oy * w + ox) as usize] = true;
        while let Some((x, y)) = stack.pop() {
            let i = (y * w + x) as usize;
            let c = ink.index_at(self.x + x + ORIGIN.0, self.y + y + ORIGIN.1);
            if px[i] != Some(c) {
                changed = true;
            }
            px[i] = Some(c);
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let j = (ny * w + nx) as usize;
                if !seen[j] && self.pixel(nx, ny) == Some(target) {
                    seen[j] = true;
                    stack.push((nx, ny));
                }
            }
        }
        if changed {
            self.image = Rc::new(Stamp {
                w: w as u32,
                h: h as u32,
                px,
            });
            self.flip_h = false;
            self.flip_v = false;
            self.turns = 0;
        }
        changed
    }

    /// Gira 90° (horário se `right`), mantendo o centro.
    pub fn turn(&mut self, right: bool) {
        let (cx, cy) = self.center();
        self.turns = if right {
            (self.turns + 1) % 4
        } else {
            (self.turns + 3) % 4
        };
        std::mem::swap(&mut self.w, &mut self.h);
        self.x = cx - self.w / 2;
        self.y = cy - self.h / 2;
    }

    /// Muda o tamanho mantendo o centro; nunca menor que `MIN_SIZE`.
    pub fn resize_centered(&mut self, w: i32, h: i32) {
        let (cx, cy) = self.center();
        self.w = w.max(MIN_SIZE);
        self.h = h.max(MIN_SIZE);
        self.x = cx - self.w / 2;
        self.y = cy - self.h / 2;
    }

    /// Pixels do contorno (fora do objeto, vizinhos de um pixel opaco), para o piscar.
    pub fn outline(&self) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        for oy in -1..=self.h {
            for ox in -1..=self.w {
                if self.pixel(ox, oy).is_some() {
                    continue;
                }
                let near = [(ox - 1, oy), (ox + 1, oy), (ox, oy - 1), (ox, oy + 1)]
                    .iter()
                    .any(|&(nx, ny)| self.pixel(nx, ny).is_some());
                if near {
                    out.push((self.x + ox, self.y + oy));
                }
            }
        }
        out
    }
}

/// Imagem de uma palavra (texto e, se for o caso, balão de fala), como objeto.
pub fn words_stamp(font: Option<&Font>, text: &str, style: &WordsStyle, color: u8) -> Stamp {
    let l = words_layout(font, text, style);
    let (w, h) = (l.grid.w.max(1), l.grid.h.max(1));
    let mut px: Vec<Option<u8>> = l
        .grid
        .px
        .iter()
        .map(|p| p.map(|edge| if edge { BLACK } else { WHITE }))
        .collect();
    px.resize((w * h) as usize, None);
    let mut put = |x: i32, y: i32, v: u8| {
        if x >= 0 && y >= 0 && x < w && y < h {
            px[(y * w + x) as usize] = Some(v);
        }
    };
    for (k, (line, dx)) in l.lines.iter().enumerate() {
        let (x0, y0) = (l.grid.pad_x + dx, l.grid.pad_y + k as i32 * l.line_h);
        text_pixels(font, line, |dx, dy| put(x0 + dx, y0 + dy, color));
    }
    Stamp {
        w: w as u32,
        h: h as u32,
        px,
    }
}

/// Lista ordenada de objetos (o último fica por cima).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Objects {
    pub items: Vec<Object>,
    next_id: u32,
}

impl Objects {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Coloca um objeto por cima de todos; devolve o id.
    pub fn place(&mut self, image: Rc<Stamp>, x: i32, y: i32, source: Source) -> u32 {
        self.next_id += 1;
        let id = self.next_id;
        self.items.push(Object::new(id, image, x, y, source));
        id
    }

    /// Acrescenta um objeto já montado (reabertura), preservando o id.
    pub fn push(&mut self, o: Object) {
        self.next_id = self.next_id.max(o.id);
        self.items.push(o);
    }

    pub fn get(&self, id: u32) -> Option<&Object> {
        self.items.iter().find(|o| o.id == id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Object> {
        self.items.iter_mut().find(|o| o.id == id)
    }

    fn index(&self, id: u32) -> Option<usize> {
        self.items.iter().position(|o| o.id == id)
    }

    /// Objeto de cima cujo pixel opaco está em `(x, y)` (coordenadas do quadro).
    pub fn hit(&self, x: i32, y: i32) -> Option<u32> {
        self.items
            .iter()
            .rev()
            .find(|o| o.pixel(x - o.x, y - o.y).is_some())
            .map(|o| o.id)
    }

    pub fn remove(&mut self, id: u32) -> Option<Object> {
        let i = self.index(id)?;
        let o = self.items.remove(i);
        for other in &mut self.items {
            if other.link == Some(id) {
                other.link = None;
            }
        }
        Some(o)
    }

    /// Desce um nível; falso se já estava embaixo.
    pub fn push_back(&mut self, id: u32) -> bool {
        match self.index(id) {
            Some(i) if i > 0 => {
                self.items.swap(i, i - 1);
                true
            }
            _ => false,
        }
    }

    /// Sobe um nível; falso se já estava em cima.
    pub fn bring_front(&mut self, id: u32) -> bool {
        match self.index(id) {
            Some(i) if i + 1 < self.items.len() => {
                self.items.swap(i, i + 1);
                true
            }
            _ => false,
        }
    }

    /// Ids do objeto e de todos os grudados a ele (direta ou indiretamente).
    pub fn group(&self, id: u32) -> Vec<u32> {
        let mut out = vec![id];
        let mut k = 0;
        while k < out.len() {
            let cur = out[k];
            k += 1;
            let partner = self.get(cur).and_then(|o| o.link);
            let mut adj: Vec<u32> = self
                .items
                .iter()
                .filter(|o| o.link == Some(cur))
                .map(|o| o.id)
                .collect();
            adj.extend(partner);
            for a in adj {
                if !out.contains(&a) {
                    out.push(a);
                }
            }
        }
        out
    }

    /// Move o objeto e os grudados a ele.
    pub fn move_by(&mut self, id: u32, dx: i32, dy: i32) {
        for g in self.group(id) {
            if let Some(o) = self.get_mut(g) {
                o.x += dx;
                o.y += dy;
            }
        }
    }

    /// Gruda `a` em `b` (corrente). Falso se é o mesmo objeto ou já estão grudados.
    pub fn link(&mut self, a: u32, b: u32) -> bool {
        if a == b || self.group(a).contains(&b) {
            return false;
        }
        match self.get_mut(a) {
            Some(o) if o.link.is_none() => {
                o.link = Some(b);
                true
            }
            Some(_) => match self.get_mut(b) {
                Some(p) if p.link.is_none() => {
                    p.link = Some(a);
                    true
                }
                _ => false,
            },
            None => false,
        }
    }

    /// Correntes: pares (centro de um, centro do outro, id do que guarda a ligação).
    pub fn chains(&self) -> Vec<Chain> {
        self.items
            .iter()
            .filter_map(|o| {
                let p = self.get(o.link?)?;
                Some((o.center(), p.center(), o.id))
            })
            .collect()
    }

    /// Corrente a até 4 px de `(x, y)`: id do objeto que guarda a ligação.
    pub fn chain_hit(&self, x: i32, y: i32) -> Option<u32> {
        self.chains()
            .into_iter()
            .find(|(a, b, _)| segment_distance(*a, *b, (x, y)) <= 4.0)
            .map(|(_, _, id)| id)
    }

    pub fn unlink(&mut self, id: u32) -> bool {
        match self.get_mut(id) {
            Some(o) if o.link.is_some() => {
                o.link = None;
                true
            }
            _ => false,
        }
    }

    /// Composição achatada: os objetos pintados sobre uma cópia do quadro,
    /// respeitando o recorte dele.
    pub fn compose(&self, c: &Canvas) -> Canvas {
        let mut out = c.clone();
        self.draw_onto(&mut out);
        out
    }

    /// Pinta os objetos no quadro (respeitando o recorte).
    pub fn draw_onto(&self, c: &mut Canvas) {
        for o in &self.items {
            for oy in 0..o.h {
                for ox in 0..o.w {
                    if let Some(v) = o.pixel(ox, oy) {
                        c.set(o.x + ox, o.y + oy, v);
                    }
                }
            }
        }
    }
}

/// Distância do ponto `p` ao segmento `ab`.
pub fn segment_distance(a: (i32, i32), b: (i32, i32), p: (i32, i32)) -> f32 {
    let (ax, ay) = (a.0 as f32, a.1 as f32);
    let (bx, by) = (b.0 as f32, b.1 as f32);
    let (px, py) = (p.0 as f32, p.1 as f32);
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
    };
    let (qx, qy) = (ax + t * dx, ay + t * dy);
    ((px - qx).powi(2) + (py - qy).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2×1: esquerda cor 5, direita transparente.
    fn stamp() -> Rc<Stamp> {
        Rc::new(Stamp {
            w: 2,
            h: 1,
            px: vec![Some(5), None],
        })
    }

    fn pixels(o: &Object) -> Vec<Option<u8>> {
        (0..o.h)
            .flat_map(|y| (0..o.w).map(move |x| (x, y)))
            .map(|(x, y)| o.pixel(x, y))
            .collect()
    }

    #[test]
    fn hit_finds_top_opaque_pixel_only() {
        let mut objs = Objects::default();
        let a = objs.place(stamp(), 10, 10, Source::Roll { roll: 1, item: 0 });
        let b = objs.place(stamp(), 10, 10, Source::Roll { roll: 1, item: 1 });
        assert_eq!(objs.hit(10, 10), Some(b));
        assert_eq!(objs.hit(11, 10), None);
        assert_eq!(objs.hit(9, 10), None);
        objs.push_back(b);
        assert_eq!(objs.hit(10, 10), Some(a));
        assert!(!objs.push_back(b));
        assert!(objs.bring_front(b));
        assert!(!objs.bring_front(b));
    }

    #[test]
    fn scale_flip_and_turn() {
        let mut o = Object::new(1, stamp(), 0, 0, Source::Png("x".into()));
        o.w = 4;
        assert_eq!(pixels(&o), vec![Some(5), Some(5), None, None]);
        o.flip_h = true;
        assert_eq!(pixels(&o), vec![None, None, Some(5), Some(5)]);
        o.flip_h = false;
        o.w = 2;
        o.turn(true);
        assert_eq!((o.w, o.h), (1, 2));
        // horário: a coluna da esquerda vira a linha de cima
        assert_eq!(pixels(&o), vec![Some(5), None]);
        o.flip_v = true;
        assert_eq!(pixels(&o), vec![None, Some(5)]);
        o.flip_v = false;
        for _ in 0..3 {
            o.turn(true);
        }
        assert_eq!(o.turns, 0);
        assert_eq!(pixels(&o), vec![Some(5), None]);
        o.turn(false);
        assert_eq!(o.turns, 3);
        assert_eq!(pixels(&o), vec![None, Some(5)]);
    }

    #[test]
    fn turn_keeps_center_and_resize_has_minimum() {
        let mut o = Object::new(1, stamp(), 10, 10, Source::Png("x".into()));
        o.w = 20;
        o.h = 10;
        let c = o.center();
        o.turn(true);
        assert_eq!(o.center(), c);
        o.resize_centered(1, 1);
        assert_eq!((o.w, o.h), (MIN_SIZE, MIN_SIZE));
    }

    #[test]
    fn outline_surrounds_opaque_pixels() {
        let o = Object::new(1, stamp(), 5, 5, Source::Png("x".into()));
        let mut out = o.outline();
        out.sort();
        assert_eq!(out, vec![(4, 5), (5, 4), (5, 6), (6, 5)]);
    }

    #[test]
    fn links_move_together_and_chain_hit_unlinks() {
        let mut objs = Objects::default();
        let a = objs.place(stamp(), 0, 0, Source::Roll { roll: 1, item: 0 });
        let b = objs.place(stamp(), 20, 0, Source::Roll { roll: 1, item: 1 });
        let c = objs.place(stamp(), 40, 0, Source::Roll { roll: 1, item: 2 });
        assert!(!objs.link(a, a));
        assert!(objs.link(a, b));
        assert!(!objs.link(b, a));
        assert!(objs.link(c, b));
        assert_eq!(objs.group(c).len(), 3);
        objs.move_by(a, 3, 4);
        assert_eq!(objs.get(b).unwrap().rect(), (23, 4, 2, 1));
        assert_eq!(objs.get(c).unwrap().rect(), (43, 4, 2, 1));
        assert_eq!(objs.chains().len(), 2);
        let hit = objs.chain_hit(13, 4).unwrap();
        assert_eq!(hit, a);
        assert!(objs.unlink(hit));
        assert_eq!(objs.group(a), vec![a]);
        objs.remove(b);
        assert_eq!(objs.get(c).unwrap().link, None);
    }

    #[test]
    fn compose_respects_clip_and_leaves_canvas() {
        let mut c = Canvas::new(4, 4);
        c.clip = Some((0, 0, 1, 1));
        let mut objs = Objects::default();
        objs.place(stamp(), 0, 0, Source::Roll { roll: 1, item: 0 });
        objs.place(stamp(), 2, 2, Source::Roll { roll: 1, item: 0 });
        let out = objs.compose(&c);
        assert_eq!(out.get(0, 0), Some(5));
        assert_eq!(out.get(2, 2), Some(WHITE));
        assert_eq!(c.get(0, 0), Some(WHITE));
    }

    #[test]
    fn words_stamp_sizes() {
        let s = words_stamp(None, "ab", &WordsStyle::plain(), 7);
        assert_eq!((s.w, s.h), (16, 8));
        assert!(s.px.contains(&Some(7)));
        let b = words_stamp(None, "ab", &WordsStyle::balloon(), 7);
        assert!(b.w > 16 && b.h > 8);
        assert!(b.px.contains(&Some(BLACK)));
        assert!(b.px.contains(&Some(WHITE)));
        // largura quebra em duas linhas, centradas
        let style = WordsStyle {
            shape: BalloonShape::Oval,
            align: Align::Center,
            width: 40,
            small: false,
        };
        let o = words_stamp(None, "abc de", &style, 7);
        let one = words_stamp(None, "abc de", &WordsStyle::plain(), 7);
        assert!(o.h > one.h + 8);
        assert!(o.w < one.w + 12);
    }

    #[test]
    fn json_reads_old_balao_flag_and_new_style_fields() {
        let text = "{\"quadro\": \"1\",\n\"objetos\": [\n{\"id\": 1, \"x\": 0, \"y\": 0, \"w\": 5, \"h\": 5, \"inv_h\": false, \"inv_v\": false, \"giros\": 0, \"texto\": \"a\", \"balao\": true, \"cor\": 3},\n{\"id\": 2, \"x\": 0, \"y\": 0, \"w\": 5, \"h\": 5, \"inv_h\": false, \"inv_v\": false, \"giros\": 0, \"texto\": \"b\", \"balao\": false, \"cor\": 3, \"forma\": \"oval\", \"alinha\": \"direita\", \"largura\": 50, \"fonte\": \"pequena\"}\n]}";
        let (_, recs) = parse_json(text).unwrap();
        assert!(
            matches!(&recs[0].source, Source::Words { style, .. } if *style == WordsStyle::balloon())
        );
        assert!(matches!(
            &recs[1].source,
            Source::Words { style, .. }
                if style.shape == BalloonShape::Oval && style.align == Align::Right && style.width == 50 && style.small
        ));
        // o texto solto não grava campos extras
        let objs = {
            let mut o = Objects::default();
            o.place(
                stamp(),
                0,
                0,
                Source::Words {
                    text: "x".into(),
                    style: WordsStyle::plain(),
                    color: 1,
                },
            );
            o
        };
        let j = to_json(&objs.records(), 0);
        assert!(j.contains("\"balao\": false") && !j.contains("forma"));
    }

    #[test]
    fn segment_distance_cases() {
        assert_eq!(segment_distance((0, 0), (10, 0), (5, 3)), 3.0);
        assert_eq!(segment_distance((0, 0), (10, 0), (15, 0)), 5.0);
        assert_eq!(segment_distance((2, 2), (2, 2), (2, 6)), 4.0);
    }
}

/// Objeto como vai ao arquivo `*.objetos.json` (sem a imagem).
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub flip_h: bool,
    pub flip_v: bool,
    pub turns: u8,
    pub link: Option<u32>,
    pub sound: Option<SoundRef>,
    pub source: Source,
}

impl Objects {
    pub fn records(&self) -> Vec<Record> {
        self.items
            .iter()
            .map(|o| Record {
                id: o.id,
                x: o.x,
                y: o.y,
                w: o.w,
                h: o.h,
                flip_h: o.flip_h,
                flip_v: o.flip_v,
                turns: o.turns,
                link: o.link,
                sound: o.sound.clone(),
                source: o.source.clone(),
            })
            .collect()
    }

    /// Remonta os objetos; `image` dá a imagem de cada origem (origem
    /// desconhecida → o objeto é pulado).
    pub fn from_records(recs: &[Record], image: impl Fn(&Source) -> Option<Stamp>) -> Objects {
        let mut out = Objects::default();
        for r in recs {
            let Some(img) = image(&r.source) else {
                continue;
            };
            out.push(Object {
                id: r.id,
                x: r.x,
                y: r.y,
                w: r.w.max(1),
                h: r.h.max(1),
                flip_h: r.flip_h,
                flip_v: r.flip_v,
                turns: r.turns % 4,
                link: r.link,
                sound: r.sound.clone(),
                image: Rc::new(img),
                source: r.source.clone(),
            });
        }
        // correntes para objetos que não vieram são apagadas
        let ids: Vec<u32> = out.items.iter().map(|o| o.id).collect();
        for o in &mut out.items {
            if o.link.is_some_and(|l| !ids.contains(&l) || l == o.id) {
                o.link = None;
            }
        }
        out
    }
}

/// Impressão digital do quadro achatado (FNV-1a das cores), gravada no JSON
/// para saber se o `ultimo.png` ainda é a composição destes objetos.
pub fn fingerprint(c: &Canvas, pal: &[formats::palette::Rgb]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for &i in &c.px {
        let p = pal
            .get(i as usize)
            .copied()
            .unwrap_or(formats::palette::Rgb { r: 0, g: 0, b: 0 });
        mix(p.r);
        mix(p.g);
        mix(p.b);
    }
    h
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// JSON (uma linha por objeto) dos registros com a impressão digital do quadro.
pub fn to_json(recs: &[Record], fingerprint: u64) -> String {
    let mut out = format!("{{\"quadro\": \"{fingerprint:016x}\",\n\"objetos\": [\n");
    for (i, r) in recs.iter().enumerate() {
        let mut line = format!(
            "  {{\"id\": {}, \"x\": {}, \"y\": {}, \"w\": {}, \"h\": {}, \"inv_h\": {}, \"inv_v\": {}, \"giros\": {}",
            r.id, r.x, r.y, r.w, r.h, r.flip_h, r.flip_v, r.turns
        );
        if let Some(l) = r.link {
            line.push_str(&format!(", \"corrente\": {l}"));
        }
        match &r.sound {
            Some(SoundRef::Library { file, index }) => {
                line.push_str(&format!(
                    ", \"som\": {}, \"som_indice\": {index}",
                    json_str(file)
                ));
            }
            Some(SoundRef::Snd(_, id)) => {
                line.push_str(&format!(", \"som\": \"\", \"som_indice\": {id}"));
            }
            None => {}
        }
        match &r.source {
            Source::Roll { roll, item } => {
                line.push_str(&format!(", \"rolo\": {roll}, \"casa\": {item}"));
            }
            Source::Png(p) => line.push_str(&format!(", \"png\": {}", json_str(p))),
            Source::Words { text, style, color } => {
                line.push_str(&format!(
                    ", \"texto\": {}, \"balao\": {}, \"cor\": {color}",
                    json_str(text),
                    style.has_balloon()
                ));
                // só o que foge do texto solto / balão "Oi!"
                if !matches!(style.shape, BalloonShape::None | BalloonShape::RectTail) {
                    line.push_str(&format!(", \"forma\": \"{}\"", style.shape.name()));
                }
                if style.align != Align::Left {
                    line.push_str(&format!(", \"alinha\": \"{}\"", style.align.name()));
                }
                if style.width > 0 {
                    line.push_str(&format!(", \"largura\": {}", style.width));
                }
                if style.small {
                    line.push_str(", \"fonte\": \"pequena\"");
                }
            }
            Source::Shaped(w) => {
                let colors: Vec<String> = w.colors.iter().map(|c| c.to_string()).collect();
                line.push_str(&format!(
                    ", \"palavra_forma\": {}, \"tipo\": {}, \"molde\": \"{}\", \"estilos\": {}, \"cores\": \"{}\", \"padrao\": {}, \"sombra\": \"{}\", \"tamanho\": {}, \"giro\": {}",
                    json_str(&w.text),
                    json_str(&w.font),
                    w.shape.name(),
                    json_str(&w.styles()),
                    colors.join(","),
                    w.pattern,
                    w.shadow.name(),
                    w.size,
                    w.angle
                ));
            }
            Source::Tail(k) => line.push_str(&format!(", \"dipitidu\": {k}")),
        }
        line.push('}');
        if i + 1 < recs.len() {
            line.push(',');
        }
        line.push('\n');
        out.push_str(&line);
    }
    out.push_str("]}\n");
    out
}

/// Valor do campo `key` numa linha `{...}`: texto (sem aspas, com escapes
/// desfeitos) ou o literal até a vírgula/chave.
fn field(line: &str, key: &str) -> Option<String> {
    let k = format!("\"{key}\": ");
    let i = line.find(&k)? + k.len();
    let rest = &line[i..];
    if let Some(r) = rest.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = r.chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => return Some(out),
                '\\' => match chars.next()? {
                    'n' => out.push('\n'),
                    o => out.push(o),
                },
                c => out.push(c),
            }
        }
        None
    } else {
        Some(rest[..rest.find([',', '}'])?].trim().to_string())
    }
}

/// Lê o JSON de `to_json`: (impressão digital, registros). Linhas inválidas são puladas.
pub fn parse_json(text: &str) -> Option<(u64, Vec<Record>)> {
    let first = text.lines().next()?;
    let fp = u64::from_str_radix(&field(first, "quadro")?, 16).ok()?;
    let mut recs = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with('{') || line.contains("\"quadro\"") {
            continue;
        }
        let num = |k: &str| field(line, k).and_then(|v| v.parse::<i64>().ok());
        let flag = |k: &str| field(line, k).is_some_and(|v| v == "true");
        let (Some(id), Some(x), Some(y), Some(w), Some(h)) =
            (num("id"), num("x"), num("y"), num("w"), num("h"))
        else {
            continue;
        };
        let source = if let (Some(roll), Some(item)) = (num("rolo"), num("casa")) {
            Source::Roll {
                roll: roll.max(0) as usize,
                item: item.max(0) as usize,
            }
        } else if let Some(p) = field(line, "png") {
            Source::Png(p)
        } else if let Some(text) = field(line, "palavra_forma") {
            let mut w = ShapedWords::new(&text, 1);
            w.font = field(line, "tipo").unwrap_or_default();
            w.shape = field(line, "molde")
                .and_then(|m| WordShape::from_name(&m))
                .unwrap_or_default();
            w.set_styles(&field(line, "estilos").unwrap_or_default());
            let colors: Vec<u8> = field(line, "cores")
                .unwrap_or_default()
                .split(',')
                .filter_map(|c| c.trim().parse().ok())
                .collect();
            if !colors.is_empty() {
                w.colors = colors;
            }
            w.pattern = num("padrao").unwrap_or(0).clamp(0, 255) as u8;
            w.shadow = field(line, "sombra")
                .and_then(|m| Shadow::from_name(&m))
                .unwrap_or_default();
            w.size = num("tamanho").unwrap_or(w.size as i64).clamp(4, 400) as u32;
            w.angle = num("giro").unwrap_or(0).rem_euclid(360) as i32;
            Source::Shaped(Box::new(w))
        } else if let Some(k) = num("dipitidu") {
            Source::Tail(k.clamp(0, 1000) as usize)
        } else if let Some(text) = field(line, "texto") {
            let shape = match field(line, "forma").and_then(|f| BalloonShape::from_name(&f)) {
                Some(s) => s,
                None if flag("balao") => BalloonShape::RectTail,
                None => BalloonShape::None,
            };
            Source::Words {
                text,
                style: WordsStyle {
                    shape,
                    align: field(line, "alinha")
                        .and_then(|a| Align::from_name(&a))
                        .unwrap_or_default(),
                    width: num("largura").unwrap_or(0).clamp(0, 10_000) as u32,
                    small: field(line, "fonte").is_some_and(|f| f == "pequena"),
                },
                color: num("cor").unwrap_or(1).clamp(0, 255) as u8,
            }
        } else {
            continue;
        };
        let sound = match (field(line, "som"), num("som_indice")) {
            (Some(file), Some(index)) if !file.is_empty() => Some(SoundRef::Library {
                file,
                index: index.max(0) as usize,
            }),
            _ => None,
        };
        recs.push(Record {
            id: id.clamp(0, u32::MAX as i64) as u32,
            x: x as i32,
            y: y as i32,
            w: w.clamp(1, 10_000) as i32,
            h: h.clamp(1, 10_000) as i32,
            flip_h: flag("inv_h"),
            flip_v: flag("inv_v"),
            turns: (num("giros").unwrap_or(0).rem_euclid(4)) as u8,
            link: num("corrente").map(|l| l.clamp(0, u32::MAX as i64) as u32),
            sound,
            source,
        });
    }
    Some((fp, recs))
}

#[cfg(test)]
mod store_tests {
    use super::*;

    fn stamp() -> Stamp {
        Stamp {
            w: 1,
            h: 1,
            px: vec![Some(3)],
        }
    }

    #[test]
    fn json_roundtrip_with_escapes_and_links() {
        let mut objs = Objects::default();
        let a = objs.place(Rc::new(stamp()), 1, 2, Source::Roll { roll: 0, item: 5 });
        let b = objs.place(
            Rc::new(stamp()),
            3,
            4,
            Source::Words {
                text: "Oi \"você\"\\ aí".into(),
                style: WordsStyle {
                    shape: BalloonShape::Thought,
                    align: Align::Center,
                    width: 120,
                    small: true,
                },
                color: 9,
            },
        );
        objs.place(Rc::new(stamp()), 5, 6, Source::Png("meu.png".into()));
        objs.link(a, b);
        {
            let o = objs.get_mut(b).unwrap();
            o.flip_h = true;
            o.turns = 3;
            o.w = 40;
            o.sound = Some(SoundRef::Library {
                file: "MAG100.SND".into(),
                index: 7,
            });
        }
        let recs = objs.records();
        let text = to_json(&recs, 0xdead_beef);
        let (fp, back) = parse_json(&text).unwrap();
        assert_eq!(fp, 0xdead_beef);
        assert_eq!(back, recs);
        let again = Objects::from_records(&back, |_| Some(stamp()));
        assert_eq!(again.records(), recs);
        // uma origem desconhecida some, e a corrente que apontava para ela também
        let partial = Objects::from_records(&back, |s| match s {
            Source::Words { .. } => None,
            _ => Some(stamp()),
        });
        assert_eq!(partial.len(), 2);
        assert_eq!(partial.get(a).unwrap().link, None);
        // ids novos continuam depois dos lidos
        let mut again = again;
        let n = again.place(Rc::new(stamp()), 0, 0, Source::Png("x".into()));
        assert!(n > b);
    }

    #[test]
    fn json_roundtrip_of_shaped_words_and_tails() {
        let mut objs = Objects::default();
        let mut w = ShapedWords::new("Oi, \"Ana\"", 5);
        w.font = "DejaVu Sans".into();
        w.shape = WordShape::Circle;
        w.set_styles("negrito,contorno,vertical");
        w.colors = vec![5, 9, 12];
        w.pattern = 4;
        w.shadow = Shadow::Converge;
        w.size = 80;
        w.angle = 345;
        objs.place(Rc::new(stamp()), 1, 2, Source::Shaped(Box::new(w)));
        objs.place(Rc::new(stamp()), 3, 4, Source::Tail(2));
        let recs = objs.records();
        let (_, back) = parse_json(&to_json(&recs, 1)).unwrap();
        assert_eq!(back, recs);
        // campos que faltam ficam no padrão
        let text = "{\"quadro\": \"1\",\n\"objetos\": [\n{\"id\": 1, \"x\": 0, \"y\": 0, \"w\": 5, \"h\": 5, \"palavra_forma\": \"x\"}\n]}";
        let (_, recs) = parse_json(text).unwrap();
        let Source::Shaped(w) = &recs[0].source else {
            panic!("não é palavra com forma");
        };
        assert_eq!((w.text.as_str(), w.font.as_str()), ("x", ""));
        assert_eq!(w.shape, WordShape::Plain);
        assert_eq!(w.colors, vec![1]);
    }

    #[test]
    fn parse_tolerates_garbage() {
        assert_eq!(parse_json(""), None);
        assert_eq!(parse_json("{\"quadro\": \"zz\"}"), None);
        let (fp, recs) =
            parse_json("{\"quadro\": \"10\",\n\"objetos\": [\n{\"id\": 1}\nlixo\n]}").unwrap();
        assert_eq!(fp, 16);
        assert!(recs.is_empty());
    }

    #[test]
    fn fingerprint_changes_with_pixels() {
        let pal: Vec<formats::palette::Rgb> = (0..4)
            .map(|i| formats::palette::Rgb {
                r: i * 60,
                g: 0,
                b: 0,
            })
            .collect();
        let mut c = Canvas::new(3, 3);
        let a = fingerprint(&c, &pal);
        c.px[4] = 2;
        assert_ne!(a, fingerprint(&c, &pal));
    }
}
