//! Pasta de trabalhos como no original: o fichário verde aberto pela maleta
//! vermelha (na Galeria e nas salas). Página esquerda: "Esta Pasta" com a
//! pasta de desenhos, "Conteúdo" com a árvore (a criança com o seu bicho ›
//! Ateliê de Pintura › quadros; os exemplos da Maggie embaixo) e os botões
//! inseto (procurar pelo nome), X (fechar) e JÁ (abrir na pintura). Página
//! direita: a miniatura do quadro escolhido, "Título" e "Artista", com setas
//! que passam ao quadro anterior/seguinte e ao artista anterior/seguinte.
//! Ao abrir, o balão "Para ver mais escolhas, clique aqui…" (`100d[54]`).
//! Posições medidas em `studio_o_suitcase_3.png` e `studio_o_pf_item_3.png`.
//! Não há unidades de disco: as setas de "Esta Pasta" só fazem o som.

use super::gallery_room::Item;
use crate::assets::{Assets, ImageRef, Program};
use crate::balloon::{Balloon, balloon_font};
use crate::scene::{Effect, Input, Key, MouseButton};
use crate::text_balloon::{PIC_ANIMAL0, Part, TextBalloon, blit_pic};
use crate::{Frame, Image};
use formats::palette::Rgb;

type Rect = (i32, i32, i32, i32);

/// Fichário inteiro (borda preta) e o miolo verde.
pub const BINDER: Rect = (115, 111, 406, 257);
const GREEN_AREA: Rect = (125, 116, 390, 247);
/// Lombada com as argolas.
const SPINE: Rect = (312, 116, 18, 247);
/// Caixas brancas da página esquerda.
const FOLDER_BOX: Rect = (142, 138, 155, 21);
const LIST_BOX: Rect = (142, 176, 155, 139);
const ROW0: i32 = 179;
const ROW_H: i32 = 17;
const ROWS: usize = 8;
/// Setas (14×16) de "Esta Pasta", "Conteúdo", "Título" e "Artista".
const FOLDER_ARROWS: (i32, i32, i32) = (143, 282, 121);
const LIST_ARROWS: (i32, i32, i32) = (143, 282, 159);
const TITLE_ARROWS: (i32, i32, i32) = (346, 482, 279);
const ARTIST_ARROWS: (i32, i32, i32) = (346, 482, 318);
/// Rótulos (centro x, topo y).
const LABEL_FOLDER: (i32, i32) = (219, 122);
const LABEL_LIST: (i32, i32) = (219, 161);
const LABEL_TITLE: (i32, i32) = (421, 280);
const LABEL_ARTIST: (i32, i32) = (421, 319);
/// Botões: inseto (procurar), X (fechar) e JÁ (abrir).
const BUG: (u16, i32, i32) = (0x25d, 158, 325);
const CLOSE: (u16, i32, i32) = (0x3bb, 208, 328);
const GO: (u16, i32, i32) = (0x3bf, 250, 328);
const BUG_RECT: Rect = (158, 325, 37, 32);
const CLOSE_RECT: Rect = (208, 328, 28, 28);
const GO_RECT: Rect = (250, 328, 26, 27);
/// Página direita: miniatura (como a `THMB 0` do `.MAG`, 144×84) e caixas.
pub const THUMB: Rect = (345, 155, 144, 84);
const TITLE_BOX: Rect = (346, 296, 152, 21);
const ARTIST_BOX: Rect = (346, 335, 152, 21);
/// Setas verdes: direita (`3af`, máscara `3b1`), cima e baixo; a esquerda é
/// a direita espelhada.
const ARROW_RIGHT: (u16, u16) = (0x3af, 0x3b1);
/// Ícone do Ateliê de Pintura na árvore (o "2" do andar).
const STUDIO_ICON: u16 = 0x3c3;
/// Balões: "Para ver mais escolhas…" e "Digite o nome da figura…".
const WHAT_ROOMS: u16 = 0x100d;
const HELP: usize = 54;
const HELP_POS: (i32, i32) = (310, 118);
const SEARCH_WHAT: (u16, usize) = (0x213, 0);
const SEARCH_POS: (i32, i32) = (238, 188);
/// Índice do original da opção "Ache".
const OPT_FIND: u16 = 1;

/// Sons do fichário.
pub mod sfx {
    /// Tique ao escolher um item.
    pub const SELECT: u16 = 0x230;
    /// Setas.
    pub const ARROW: u16 = 0x2bc;
    /// X: fecha (fala).
    pub const CLOSE: u16 = 0x2ed;
    /// JÁ: abre na pintura.
    pub const GO: u16 = 0x239;
    /// Inseto: "Digite o nome da figura…".
    pub const SEARCH: u16 = 0x226;
    /// Opção escolhida; recusa (nada achado).
    pub const CHOICE: u16 = 0x254;
    pub const REFUSE: u16 = 0x217;
}

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const GREEN: [u8; 3] = [71, 140, 80];
const GRAY: [u8; 3] = [190, 190, 190];
const DOUBLE_CLICK: f32 = 0.4;

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && y >= r.1 && x < r.0 + r.2 && y < r.1 + r.3
}

fn sound(id: u16) -> Effect {
    Effect::PlaySound(super::room_objects::snd(id))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    /// Dono dos quadros abaixo, com o bicho (nos exemplos, sem bicho).
    Owner(String, Option<usize>),
    /// "Ateliê de Pintura".
    Studio,
    /// Um quadro (índice em `items`).
    Item(usize),
}

/// O que o fichário pede à sala.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Close,
    Open(usize),
}

pub struct Binder {
    rows: Vec<Row>,
    /// Primeira linha visível.
    scroll: usize,
    selected: Option<usize>,
    /// Nome da pasta em "Esta Pasta".
    folder: String,
    user: String,
    animal: Option<usize>,
    help: Option<Balloon>,
    search: Option<(TextBalloon, Vec<u16>)>,
    last_click: Option<(usize, f32)>,
}

/// Compara nomes sem maiúsculas nem acentos.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

impl Binder {
    pub fn new(folder: String) -> Binder {
        Binder {
            rows: Vec::new(),
            scroll: 0,
            selected: None,
            folder,
            user: super::room::DEFAULT_USER.to_string(),
            animal: None,
            help: None,
            search: None,
            last_click: None,
        }
    }

    pub fn set_user(&mut self, user: &str, animal: Option<usize>, items: &[Item]) {
        self.user = user.to_string();
        self.animal = animal;
        self.rebuild(items);
    }

    /// Refaz a árvore depois que a lista de quadros mudou.
    pub fn rebuild(&mut self, items: &[Item]) {
        let mut rows = vec![Row::Owner(self.user.clone(), self.animal), Row::Studio];
        rows.extend(
            (0..items.len())
                .filter(|&i| !items[i].sample)
                .map(Row::Item),
        );
        if items.iter().any(|it| it.sample) {
            rows.push(Row::Owner("Maggie".into(), None));
            rows.push(Row::Studio);
            rows.extend((0..items.len()).filter(|&i| items[i].sample).map(Row::Item));
        }
        self.rows = rows;
        if self.selected.is_some_and(|i| i >= items.len()) {
            self.selected = None;
        }
        if self.selected.is_none() {
            self.selected = self.first_item();
        }
        self.scroll = self.scroll.min(self.max_scroll());
    }

    fn first_item(&self) -> Option<usize> {
        self.rows.iter().find_map(|r| match r {
            Row::Item(i) => Some(*i),
            _ => None,
        })
    }

    fn max_scroll(&self) -> usize {
        self.rows.len().saturating_sub(ROWS)
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn search_open(&self) -> bool {
        self.search.is_some()
    }

    /// Abre o fichário com o balão de ajuda.
    pub fn open(&mut self, help: bool, assets: &Assets) {
        self.search = None;
        self.help = if help {
            let records = assets.dialogs(Program::Artist, WHAT_ROOMS);
            Balloon::from_block(&records, HELP, HELP_POS.0, HELP_POS.1, &self.user)
        } else {
            None
        };
    }

    fn row_rect(k: usize) -> Rect {
        (
            LIST_BOX.0 + 2,
            ROW0 + ROW_H * k as i32,
            LIST_BOX.2 - 4,
            ROW_H,
        )
    }

    fn arrow_rects(a: (i32, i32, i32)) -> (Rect, Rect) {
        ((a.0, a.2, 14, 16), (a.1, a.2, 14, 16))
    }

    fn row_of_item(&self, i: usize) -> Option<usize> {
        self.rows.iter().position(|r| *r == Row::Item(i))
    }

    /// Escolhe o quadro e rola a lista até ele.
    fn select(&mut self, i: usize) {
        self.selected = Some(i);
        if let Some(r) = self.row_of_item(i) {
            if r < self.scroll {
                self.scroll = r;
            } else if r >= self.scroll + ROWS {
                self.scroll = (r + 1 - ROWS).min(self.max_scroll());
            }
        }
    }

    /// Quadro anterior/seguinte na árvore.
    fn step_item(&mut self, forward: bool) {
        let items: Vec<usize> = self
            .rows
            .iter()
            .filter_map(|r| match r {
                Row::Item(i) => Some(*i),
                _ => None,
            })
            .collect();
        if items.is_empty() {
            return;
        }
        let pos = self
            .selected
            .and_then(|s| items.iter().position(|&i| i == s))
            .unwrap_or(0);
        let n = items.len();
        let next = if forward {
            (pos + 1) % n
        } else {
            (pos + n - 1) % n
        };
        self.select(items[next]);
    }

    /// Primeiro quadro do artista anterior/seguinte.
    fn step_owner(&mut self, forward: bool) {
        let groups: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r, Row::Owner(..)))
            .map(|(k, _)| k)
            .collect();
        let cur = self.selected.and_then(|i| self.row_of_item(i)).unwrap_or(0);
        let g = groups.iter().rposition(|&k| k <= cur).unwrap_or(0);
        let n = groups.len();
        let next = groups[if forward {
            (g + 1) % n
        } else {
            (g + n - 1) % n
        }];
        if let Some(i) = self.rows[next..].iter().find_map(|r| match r {
            Row::Item(i) => Some(*i),
            _ => None,
        }) {
            self.select(i);
        }
    }

    fn open_search(&mut self, assets: &Assets) {
        let (b, ids) = match assets
            .dialogs(Program::Artist, SEARCH_WHAT.0)
            .get(SEARCH_WHAT.1)
        {
            Some(r) => (
                TextBalloon::from_record(r, SEARCH_POS.0, SEARCH_POS.1, "", Vec::new()),
                r.options().into_iter().map(|o| o.0).collect(),
            ),
            None => (
                TextBalloon::new(
                    SEARCH_POS.0,
                    SEARCH_POS.1,
                    162,
                    vec![
                        Part::Text("Digite o nome da figura que você quer que eu ache:".into()),
                        Part::Field {
                            field: 0,
                            width: 144,
                        },
                        Part::Options(vec!["Ache".into(), "Não, esqueça".into()]),
                    ],
                    Vec::new(),
                ),
                vec![OPT_FIND, 0],
            ),
        };
        self.search = Some((b, ids));
    }

    /// Procura o primeiro quadro (depois do escolhido) com o texto no nome.
    fn find(&mut self, text: &str, items: &[Item]) -> bool {
        let t = fold(text.trim());
        if t.is_empty() {
            return false;
        }
        let order: Vec<usize> = self
            .rows
            .iter()
            .filter_map(|r| match r {
                Row::Item(i) => Some(*i),
                _ => None,
            })
            .collect();
        let start = self
            .selected
            .and_then(|s| order.iter().position(|&i| i == s))
            .map_or(0, |p| p + 1);
        let n = order.len();
        for k in 0..n {
            let i = order[(start + k) % n];
            if fold(&items[i].name).contains(&t) {
                self.select(i);
                return true;
            }
        }
        false
    }

    /// Teclas e texto para o campo da busca (os outros não usam teclado).
    pub fn key(&mut self, i: &Input, items: &[Item], fx: &mut Vec<Effect>, assets: &Assets) {
        let Some((b, _)) = &mut self.search else {
            if let Input::Key {
                key: k @ (Key::Up | Key::Down),
                ..
            } = i
            {
                self.step_item(*k == Key::Down);
                fx.push(sound(sfx::SELECT));
            }
            return;
        };
        b.handle(assets, i);
        if let Input::Key {
            key: Key::Enter, ..
        } = i
        {
            let text = b.fields.first().cloned().unwrap_or_default();
            self.answer_search(Some(OPT_FIND), &text, items, fx);
        } else if let Input::Key {
            key: Key::Escape, ..
        } = i
        {
            self.search = None;
        }
    }

    fn answer_search(&mut self, id: Option<u16>, text: &str, items: &[Item], fx: &mut Vec<Effect>) {
        if id != Some(OPT_FIND) {
            fx.push(sound(sfx::CHOICE));
            self.search = None;
            return;
        }
        if self.find(text, items) {
            fx.push(sound(sfx::CHOICE));
            self.search = None;
        } else {
            fx.push(sound(sfx::REFUSE));
            if let Some((b, _)) = &mut self.search {
                b.select_all = true;
            }
        }
    }

    /// Um clique no fichário; `None` se não pede nada à sala. `clock` serve
    /// para o clique duplo numa linha.
    pub fn click(
        &mut self,
        x: i32,
        y: i32,
        clock: f32,
        items: &[Item],
        fx: &mut Vec<Effect>,
        assets: &Assets,
    ) -> Option<Action> {
        // o balão de ajuda fecha com qualquer clique, que segue adiante
        self.help = None;
        if let Some((b, ids)) = &mut self.search {
            let input = Input::Click {
                button: MouseButton::Left,
                x,
                y,
            };
            if let Some(k) = b.hit_option(assets, x, y) {
                let id = ids.get(k).copied();
                let text = b.fields.first().cloned().unwrap_or_default();
                self.answer_search(id, &text, items, fx);
            } else if b.contains(assets, x, y) {
                b.handle(assets, &input);
            } else {
                self.search = None;
            }
            return None;
        }
        if inside(CLOSE_RECT, x, y) {
            fx.push(sound(sfx::CLOSE));
            return Some(Action::Close);
        }
        if inside(GO_RECT, x, y) {
            return self.selected.map(|i| {
                fx.push(sound(sfx::GO));
                Action::Open(i)
            });
        }
        if inside(BUG_RECT, x, y) {
            fx.push(sound(sfx::SEARCH));
            self.open_search(assets);
            return None;
        }
        let arrows = [FOLDER_ARROWS, LIST_ARROWS, TITLE_ARROWS, ARTIST_ARROWS];
        for (n, a) in arrows.into_iter().enumerate() {
            let (l, r) = Self::arrow_rects(a);
            let forward = if inside(l, x, y) {
                false
            } else if inside(r, x, y) {
                true
            } else {
                continue;
            };
            fx.push(sound(sfx::ARROW));
            match n {
                // uma só pasta: as setas das unidades só fazem o som
                0 => {}
                1 => {
                    self.scroll = if forward {
                        (self.scroll + ROWS - 1).min(self.max_scroll())
                    } else {
                        self.scroll.saturating_sub(ROWS - 1)
                    }
                }
                2 => self.step_item(forward),
                _ => self.step_owner(forward),
            }
            return None;
        }
        for k in 0..ROWS {
            if !inside(Self::row_rect(k), x, y) {
                continue;
            }
            if let Some(Row::Item(i)) = self.rows.get(self.scroll + k).cloned() {
                fx.push(sound(sfx::SELECT));
                if let Some((j, t)) = self.last_click
                    && j == i
                    && clock - t < DOUBLE_CLICK
                {
                    self.last_click = None;
                    return Some(Action::Open(i));
                }
                self.last_click = Some((i, clock));
                self.selected = Some(i);
            }
            return None;
        }
        None
    }

    fn label(f: &mut Frame, a: &Assets, cx: i32, y: i32, s: &str) {
        match balloon_font(a) {
            Some(font) => {
                let w = font.width(s) as i32;
                f.text_font(&font, cx - w / 2, y, s, BLACK);
            }
            None => f.text(cx - 4 * s.chars().count() as i32, y, s, BLACK),
        }
    }

    /// Texto cortado com "…" para caber em `w`.
    fn fit(a: &Assets, s: &str, w: i32) -> String {
        let Some(font) = balloon_font(a) else {
            return s.to_string();
        };
        if font.width(s) as i32 <= w {
            return s.to_string();
        }
        let mut chars: Vec<char> = s.chars().collect();
        while !chars.is_empty() {
            chars.pop();
            let t: String = chars.iter().collect::<String>() + "…";
            if font.width(&t) as i32 <= w {
                return t;
            }
        }
        String::new()
    }

    fn text(f: &mut Frame, a: &Assets, x: i32, y: i32, s: &str, rgb: [u8; 3]) {
        match balloon_font(a) {
            Some(font) => {
                f.text_font(&font, x, y, s, rgb);
            }
            None => f.text(x, y + 2, s, rgb),
        }
    }

    fn white_box(f: &mut Frame, r: Rect) {
        f.rect(r.0, r.1, r.2, r.3, BLACK);
        f.rect(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2, WHITE);
    }

    fn mirrored(f: &mut Frame, img: &Image, mask: Option<&Image>, x: i32, y: i32) {
        for dy in 0..img.height {
            for dx in 0..img.width {
                let i = ((dy * img.width + dx) * 4) as usize;
                let p = &img.rgba[i..i + 4];
                let opaque = match mask {
                    Some(m) if m.width == img.width && m.height == img.height => m.rgba[i] < 128,
                    _ => p[3] != 0 && p[..3] != WHITE,
                };
                if opaque {
                    let tx = x + (img.width - 1 - dx) as i32;
                    f.put(tx, y + dy as i32, [p[0], p[1], p[2]]);
                }
            }
        }
    }

    fn arrows(f: &mut Frame, a: &Assets, r: (i32, i32, i32)) {
        let ids = a.pic_ids(Program::Artist);
        if ids.binary_search(&ARROW_RIGHT.0).is_ok() {
            let img = a.image(&ImageRef::Pic(Program::Artist, ARROW_RIGHT.0));
            let mask = ids
                .binary_search(&ARROW_RIGHT.1)
                .is_ok()
                .then(|| a.image(&ImageRef::Pic(Program::Artist, ARROW_RIGHT.1)));
            Self::mirrored(f, &img, mask.as_deref(), r.0, r.2);
            blit_pic(f, a, ARROW_RIGHT.0, r.1, r.2, true);
        } else {
            f.text(r.0 + 3, r.2 + 4, "<", BLACK);
            f.text(r.1 + 3, r.2 + 4, ">", BLACK);
        }
    }

    fn draw_thumb(f: &mut Frame, it: &Item, pal: &[Rgb]) {
        let (x, y, w, h) = THUMB;
        f.rect(x - 1, y - 1, w + 2, h + 2, BLACK);
        let t = it.thumb();
        for ty in 0..h {
            for tx in 0..w {
                let sx = (tx as u32 * t.w / w as u32).min(t.w - 1);
                let sy = (ty as u32 * t.h / h as u32).min(t.h - 1);
                let p = pal
                    .get(t.px[(sy * t.w + sx) as usize] as usize)
                    .copied()
                    .unwrap_or(Rgb { r: 0, g: 0, b: 0 });
                f.put(x + tx, y + ty, [p.r, p.g, p.b]);
            }
        }
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets, items: &[Item], pal: &[Rgb]) {
        let (x, y, w, h) = BINDER;
        f.rect(x, y, w, h, BLACK);
        f.rect(x + 2, y + 5, 2, h - 10, GRAY);
        let (gx, gy, gw, gh) = GREEN_AREA;
        f.rect(gx, gy, gw, gh, GREEN);
        // lombada com as argolas
        let (sx, sy, sw, sh) = SPINE;
        f.rect(sx, sy, sw, sh, BLACK);
        let mut ry = sy + 7;
        while ry < sy + sh - 4 {
            f.rect(sx + 3, ry, 9, 2, WHITE);
            ry += 8;
        }
        // página esquerda
        Self::label(f, a, LABEL_FOLDER.0, LABEL_FOLDER.1, "Esta Pasta");
        Self::arrows(f, a, FOLDER_ARROWS);
        Self::white_box(f, FOLDER_BOX);
        let folder = Self::fit(a, &self.folder, FOLDER_BOX.2 - 8);
        Self::label(
            f,
            a,
            FOLDER_BOX.0 + FOLDER_BOX.2 / 2,
            FOLDER_BOX.1 + 3,
            &folder,
        );
        Self::label(f, a, LABEL_LIST.0, LABEL_LIST.1, "Conteúdo");
        Self::arrows(f, a, LIST_ARROWS);
        Self::white_box(f, LIST_BOX);
        for k in 0..ROWS {
            let Some(row) = self.rows.get(self.scroll + k) else {
                break;
            };
            let (rx, ry, rw, rh) = Self::row_rect(k);
            match row {
                Row::Owner(name, animal) => {
                    if let Some(b) = animal {
                        blit_pic(f, a, PIC_ANIMAL0 + 2 * *b as u16, rx + 4, ry + 3, false);
                    }
                    Self::text(f, a, rx + 26, ry + 1, name, BLACK);
                }
                Row::Studio => {
                    blit_pic(f, a, STUDIO_ICON, rx + 16, ry + 3, true);
                    Self::text(f, a, rx + 30, ry + 1, "Ateliê de Pintura", BLACK);
                }
                Row::Item(i) => {
                    let name = Self::fit(a, &items[*i].name, rw - 34);
                    if self.selected == Some(*i) {
                        f.rect(rx, ry, rw, rh, BLACK);
                        Self::text(f, a, rx + 30, ry + 1, &name, WHITE);
                    } else {
                        Self::text(f, a, rx + 30, ry + 1, &name, BLACK);
                    }
                }
            }
        }
        blit_pic(f, a, BUG.0, BUG.1, BUG.2, true);
        blit_pic(f, a, CLOSE.0, CLOSE.1, CLOSE.2, true);
        blit_pic(f, a, GO.0, GO.1, GO.2, true);
        // página direita
        let it = self.selected.and_then(|i| items.get(i));
        if let Some(it) = it {
            Self::draw_thumb(f, it, pal);
        }
        Self::label(f, a, LABEL_TITLE.0, LABEL_TITLE.1, "Título");
        Self::arrows(f, a, TITLE_ARROWS);
        Self::white_box(f, TITLE_BOX);
        Self::label(f, a, LABEL_ARTIST.0, LABEL_ARTIST.1, "Artista");
        Self::arrows(f, a, ARTIST_ARROWS);
        Self::white_box(f, ARTIST_BOX);
        if let Some(it) = it {
            let title = Self::fit(a, &it.name, TITLE_BOX.2 - 8);
            Self::label(f, a, TITLE_BOX.0 + TITLE_BOX.2 / 2, TITLE_BOX.1 + 3, &title);
            let artist = if it.sample {
                if it.author.is_empty() {
                    "Maggie".to_string()
                } else {
                    it.author.clone()
                }
            } else {
                self.user.clone()
            };
            Self::label(
                f,
                a,
                ARTIST_BOX.0 + ARTIST_BOX.2 / 2,
                ARTIST_BOX.1 + 3,
                &artist,
            );
        }
        if let Some(b) = &self.help {
            b.draw(f, a);
        }
        if let Some((b, _)) = &self.search {
            b.draw(f, a);
        }
    }

    /// Rótulo do item numa linha visível (para os testes).
    #[cfg(test)]
    pub fn row_point(&self, i: usize) -> Option<(i32, i32)> {
        let r = self.row_of_item(i)?;
        let k = r.checked_sub(self.scroll).filter(|&k| k < ROWS)?;
        let rr = Self::row_rect(k);
        Some((rr.0 + 40, rr.1 + 5))
    }

    #[cfg(test)]
    pub fn search_balloon(&self) -> Option<&TextBalloon> {
        self.search.as_ref().map(|(b, _)| b)
    }
}

pub const BUTTON_CLOSE: Rect = CLOSE_RECT;
pub const BUTTON_GO: Rect = GO_RECT;
pub const BUTTON_SEARCH: Rect = BUG_RECT;
pub const TITLE_NEXT: Rect = (TITLE_ARROWS.1, TITLE_ARROWS.2, 14, 16);
pub const ARTIST_NEXT: Rect = (ARTIST_ARROWS.1, ARTIST_ARROWS.2, 14, 16);
