//! Diagramação do documento rico em folhas.
//!
//! Tudo em pontos da folha (na Visão normal do original, 1 pt ≈ 1 pixel).
//! A folha e as margens vêm do [`DopProps`](super::doc::DopProps); a área do
//! texto se divide em 1 a 4 colunas; o texto corre coluna a coluna e folha a
//! folha, uma linha nunca fica entre duas colunas e "Começa uma nova página"
//! força uma folha nova. Cada linha é dividida em **segmentos** pelos
//! objetos da folha (o texto passa dos dois lados de um adesivo), conforme o
//! Flutua-texto de cada um ([`Wrap`](super::object::Wrap)): contorno da
//! figura, retângulo, só acima e abaixo, ou em todas as folhas.
//!
//! A largura das letras vem de uma [`Measure`] abstrata: a cena liga as
//! fontes de verdade; os testes usam [`SimpleMeasure`].
//!
//! Medidas ainda não conferidas no original (W26) ficam nas constantes
//! [`INDENT`], [`BULLET_INDENT`], [`WRAP_GAP`], [`MIN_SEGMENT`],
//! [`DROP_GAP`] e [`RAISED_SCALE`]. Os espaçamentos foram medidos
//! (`office-complemento.md` §3.1): "entre as linhas" (sprm `0x1c`) dobra a
//! entrelinha e "depois do parágrafo" (`0x1b`) soma meia linha (a última).

use super::doc::{Align, CharProps, Document, Indent, Item, Paragraph, Pos, Range};

/// Recuo dos "Recuos", em pontos.
pub const INDENT: i32 = 36;
/// Recuo do texto dos marcadores; a bolinha fica no começo dele.
pub const BULLET_INDENT: i32 = 18;
/// O marcador.
pub const BULLET: char = '•';
/// Folga entre o texto e um objeto.
pub const WRAP_GAP: i32 = 6;
/// Segmento mais estreito em que ainda se escreve.
pub const MIN_SEGMENT: i32 = 24;
/// Espaço entre a capitular e o texto.
pub const DROP_GAP: i32 = 4;
/// Ampliação da letra "Destacada".
pub const RAISED_SCALE: u16 = 3;
/// Largura do realce do `\r` selecionado.
pub const MARK_W: i32 = 4;

/// Fonte de uma medição: nome (da tabela do documento), tamanho e estilo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontKey<'a> {
    pub name: &'a str,
    pub size_pt: u16,
    pub bold: bool,
    pub italic: bool,
}

impl<'a> FontKey<'a> {
    pub fn of(doc: &'a Document, p: &CharProps) -> FontKey<'a> {
        FontKey {
            name: doc.fonts.name(p.font),
            size_pt: p.size.pt(),
            bold: p.bold,
            italic: p.italic,
        }
    }

    fn scaled(self, k: u16) -> FontKey<'a> {
        FontKey {
            size_pt: self.size_pt.saturating_mul(k),
            ..self
        }
    }
}

/// Medição de texto, em pontos.
pub trait Measure {
    /// Largura de avanço do caractere.
    fn advance(&self, font: &FontKey, c: char) -> i32;
    /// `(ascendente, descendente)`: a linha tem a soma de altura e a base
    /// fica a `ascendente` do topo.
    fn metrics(&self, font: &FontKey) -> (i32, i32);
    /// Altura das maiúsculas (do alto do 'H' à base). Padrão: 3/4 do
    /// ascendente.
    fn cap_height(&self, font: &FontKey) -> i32 {
        (self.metrics(font).0 * 3 + 2) / 4
    }
    /// `(largura, altura)` da figura do Maçã `id` numa linha com a fonte
    /// `font` (a figura fica em pé na base). Padrão: um quadrado da altura
    /// da linha.
    fn rebus_size(&self, _id: u8, font: &FontKey) -> (i32, i32) {
        let (a, d) = self.metrics(font);
        (a + d, a + d)
    }
}

/// Medição simples, sem fontes: avanço de 3/5 do tamanho (+1 no negrito),
/// ascendente de 4/5 e descendente de 1/4. Para testes e para quando
/// nenhuma fonte foi carregada.
#[derive(Debug, Clone, Copy, Default)]
pub struct SimpleMeasure;

impl Measure for SimpleMeasure {
    fn advance(&self, f: &FontKey, _c: char) -> i32 {
        ((i32::from(f.size_pt) * 3 + 4) / 5).max(1) + i32::from(f.bold)
    }

    fn metrics(&self, f: &FontKey) -> (i32, i32) {
        let s = i32::from(f.size_pt);
        ((s * 4 + 4) / 5, (s + 3) / 4)
    }
}

/// Caixa de um item numa linha, em coordenadas da folha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemBox {
    /// Índice do item no parágrafo.
    pub index: usize,
    pub x: i32,
    pub w: i32,
    /// Base (y) do item; a da capitular é a da sua última linha.
    pub baseline: i32,
    pub ascent: i32,
    pub descent: i32,
    /// Tamanho de desenho, em pontos (a capitular e a destacada ampliam).
    pub size_pt: u16,
    pub space: bool,
}

/// Uma linha diagramada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub para: usize,
    /// Itens `start..end` do parágrafo.
    pub start: usize,
    pub end: usize,
    pub page: usize,
    pub column: usize,
    /// Topo e altura da linha (y da folha).
    pub top: i32,
    pub height: i32,
    pub baseline: i32,
    /// Lugar do cursor numa linha sem itens.
    pub x0: i32,
    /// A coluna `left..right` da linha.
    pub left: i32,
    pub right: i32,
    /// Em ordem de índice.
    pub boxes: Vec<ItemBox>,
    /// Posição x da bolinha dos marcadores (primeira linha).
    pub bullet: Option<i32>,
    pub last_in_para: bool,
}

/// Número de uma folha: texto e caixa `(x0, y0, x1, y1)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageNumberBox {
    pub text: String,
    pub rect: (i32, i32, i32, i32),
}

/// Uma folha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Linhas `lines.start..lines.end` de [`Layout::lines`].
    pub lines: std::ops::Range<usize>,
    /// Índices de [`Document::objects`] desenhados nesta folha, de baixo
    /// para cima.
    pub objects: Vec<usize>,
    pub number: Option<PageNumberBox>,
}

/// Posição do cursor na folha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caret {
    pub page: usize,
    pub x: i32,
    pub top: i32,
    pub height: i32,
}

/// Retângulo de seleção (vídeo inverso) `(x0, y0, x1, y1)` numa folha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelRect {
    pub page: usize,
    pub rect: (i32, i32, i32, i32),
}

/// O documento diagramado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub page_w: i32,
    pub page_h: i32,
    /// Todas as linhas, em ordem de leitura.
    pub lines: Vec<Line>,
    /// Pelo menos uma.
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Metric {
    w: i32,
    asc: i32,
    desc: i32,
    size: u16,
    space: bool,
}

/// Capitular do parágrafo em diagramação. A letra é ampliada até as
/// maiúsculas irem do alto das maiúsculas da 1ª linha à base da última
/// (`lines`) — ~32×40 px na fonte padrão (`office-complemento.md` §3, Mix
/// "Era uma vez").
#[derive(Debug, Clone, Copy)]
struct DropCap {
    m: Metric,
    /// Altura e ascendente de uma linha normal do parágrafo.
    line_h: i32,
    line_asc: i32,
    lines: i32,
    /// Onde ficou (folha, coluna, topo), depois da primeira linha.
    placed: Option<(usize, usize, i32)>,
}

impl DropCap {
    fn extra(&self, line_no: usize, page: usize, col: usize, y: i32) -> i32 {
        let on = match self.placed {
            None => line_no == 0,
            Some((p, c, top)) => p == page && c == col && y < top + self.lines * self.line_h,
        };
        if on { self.m.w + DROP_GAP } else { 0 }
    }
}

/// Linha candidata, antes do alinhamento.
struct Cand {
    end: usize,
    boxes: Vec<ItemBox>,
    asc: i32,
    desc: i32,
    /// `(x0, x1, primeira caixa, fim)` de cada segmento usado.
    runs: Vec<(i32, i32, usize, usize)>,
    /// Começo do primeiro segmento livre, antes do recuo (onde fica o
    /// marcador).
    start: i32,
}

impl Cand {
    fn height(&self) -> i32 {
        self.asc + self.desc
    }
}

/// Área ao lado de uma capitular já posta: as linhas que começam nela (até
/// as de parágrafos seguintes) começam depois da letra.
#[derive(Debug, Clone, Copy)]
struct DropZone {
    page: usize,
    col: usize,
    /// Onde o texto começa (depois da letra e da folga).
    x: i32,
    bottom: i32,
}

impl DropZone {
    fn left(&self, page: usize, col: usize, y: i32, left: i32) -> i32 {
        if page == self.page && col == self.col && y < self.bottom {
            left.max(self.x)
        } else {
            left
        }
    }
}

struct Builder<'a> {
    doc: &'a Document,
    m: &'a dyn Measure,
    cols: Vec<(i32, i32)>,
    top: i32,
    bottom: i32,
    page_w: i32,
    lines: Vec<Line>,
    page: usize,
    col: usize,
    y: i32,
    col_has_lines: bool,
    page_has_lines: bool,
    /// Capitular de um parágrafo anterior que ainda não acabou.
    zone: Option<DropZone>,
}

impl Builder<'_> {
    fn measure(&self, par: &Paragraph, para: usize, k: usize, scale: u16) -> Metric {
        match par.items[k] {
            Item::Char(c, p) => {
                let f = FontKey::of(self.doc, &p).scaled(scale);
                let (asc, desc) = self.m.metrics(&f);
                let space = c.is_whitespace();
                Metric {
                    w: self.m.advance(&f, c),
                    asc,
                    desc,
                    size: f.size_pt,
                    space,
                }
            }
            Item::Rebus(id) => {
                let p = self.doc.props_at(Pos::new(para, k));
                let f = FontKey::of(self.doc, &p);
                let (w, h) = self.m.rebus_size(id, &f);
                Metric {
                    w,
                    asc: h,
                    desc: 0,
                    size: f.size_pt,
                    space: false,
                }
            }
        }
    }

    /// A 1ª letra no tamanho da capitular de `lines` linhas de altura
    /// `line_h`: o tamanho cujas maiúsculas mais se aproximam da altura das
    /// linhas de cima mais as maiúsculas da última.
    fn drop_cap_metric(&self, par: &Paragraph, para: usize, lines: i32, line_h: i32) -> Metric {
        let Item::Char(_, p) = par.items[0] else {
            return self.measure(par, para, 0, lines as u16);
        };
        let f = FontKey::of(self.doc, &p);
        let want = (lines - 1) * line_h + self.m.cap_height(&f);
        let pt = (f.size_pt..=255)
            .min_by_key(|&pt| {
                let k = FontKey { size_pt: pt, ..f };
                (self.m.cap_height(&k) - want).abs()
            })
            .unwrap_or(f.size_pt);
        let key = FontKey { size_pt: pt, ..f };
        let (asc, desc) = self.m.metrics(&key);
        let c = par.items[0].char().unwrap_or(' ');
        Metric {
            w: self.m.advance(&key, c),
            asc,
            desc,
            size: pt,
            space: false,
        }
    }

    fn mark_metric(&self, p: &CharProps) -> Metric {
        let f = FontKey::of(self.doc, p);
        let (asc, desc) = self.m.metrics(&f);
        Metric {
            asc,
            desc,
            size: f.size_pt,
            ..Metric::default()
        }
    }

    fn next_column(&mut self) {
        if self.col + 1 < self.cols.len() {
            self.col += 1;
        } else {
            self.page += 1;
            self.col = 0;
            self.page_has_lines = false;
        }
        self.y = self.top;
        self.col_has_lines = false;
    }

    fn new_page(&mut self) {
        self.page += 1;
        self.col = 0;
        self.y = self.top;
        self.col_has_lines = false;
        self.page_has_lines = false;
    }

    /// Partes livres de `left..right` na faixa `y0..y1` da folha atual.
    fn segments(&self, left: i32, right: i32, y0: i32, y1: i32, ignore: bool) -> Vec<(i32, i32)> {
        let mut segs = vec![(left, right)];
        if !ignore {
            let far = 4 * self.page_w.max(1);
            let obstacles = self.doc.dop.obstacles.iter().filter_map(|o| {
                let (ox0, oy0) = (i32::from(o.x), i32::from(o.y));
                let (ox1, oy1) = (ox0 + i32::from(o.width), oy0 + i32::from(o.height));
                if y1 <= oy0 - WRAP_GAP || y0 >= oy1 + WRAP_GAP {
                    return None;
                }
                // 0x29: 1 = texto só à esquerda, 2 = só à direita, 3 = nenhum
                Some(match o.wrap {
                    Some(1) => (ox0 - WRAP_GAP, far),
                    Some(2) => (-far, ox1 + WRAP_GAP),
                    _ => (-far, far),
                })
            });
            let exclusions: Vec<(i32, i32)> = self
                .doc
                .objects
                .iter()
                .filter(|o| o.on_page(self.page))
                .filter_map(|o| o.exclusion(y0, y1, WRAP_GAP, self.page_w))
                .chain(obstacles)
                .collect();
            for (a, b) in exclusions {
                segs = segs
                    .into_iter()
                    .flat_map(|(s, e)| {
                        [(s, e.min(a)), (s.max(b), e)]
                            .into_iter()
                            .filter(|(x0, x1)| x1 > x0)
                    })
                    .collect();
            }
        }
        segs.retain(|(a, b)| b - a >= MIN_SEGMENT.min(right - left));
        segs
    }

    /// Próximo y a tentar quando a faixa `y..y+h` não tem lugar.
    fn skip_y(&self, y: i32, h: i32) -> i32 {
        let h = h.max(1);
        let below = self
            .doc
            .objects
            .iter()
            .filter(|o| o.on_page(self.page))
            .map(|o| o.rect().3 + WRAP_GAP)
            .chain(
                self.doc
                    .dop
                    .obstacles
                    .iter()
                    .map(|o| i32::from(o.y) + i32::from(o.height) + WRAP_GAP),
            )
            .filter(|&b| b > y)
            .min();
        below.map_or(y + h, |b| b.min(y + h))
    }

    /// Enche uma linha na faixa `y..y+h` a partir do item `k`; `Err(y)` =
    /// não coube nada, tentar de novo em `y`. O recuo `indent` (recuos e
    /// marcadores) conta a partir do começo do primeiro segmento livre: ao
    /// lado de um objeto, da borda dele (SAMP16), e não da margem `left`.
    #[allow(clippy::too_many_arguments)]
    fn fill(
        &self,
        ms: &[Metric],
        mark: Metric,
        k0: usize,
        (left, indent): (i32, i32),
        right: i32,
        y: i32,
        h: i32,
        ignore: bool,
    ) -> Result<Cand, i32> {
        let mut segs = self.segments(left, right, y, y + h, ignore);
        let Some(&(start, first_end)) = segs.first() else {
            return Err(self.skip_y(y, h));
        };
        segs[0].0 = (start + indent).min(first_end - 1);
        let left = (left + indent).min(right - 1);
        let n = ms.len();
        let mut k = k0;
        let mut boxes: Vec<ItemBox> = Vec::new();
        let mut runs = Vec::new();
        let push = |boxes: &mut Vec<ItemBox>, i: usize, x: i32| {
            let m = ms[i];
            boxes.push(ItemBox {
                index: i,
                x,
                w: m.w,
                baseline: 0,
                ascent: m.asc,
                descent: m.desc,
                size_pt: m.size,
                space: m.space,
            });
            x + m.w
        };
        for &(sx0, sx1) in &segs {
            let b0 = boxes.len();
            let mut x = sx0;
            while k < n {
                let we = (k..n).find(|&i| ms[i].space).unwrap_or(n);
                let ws = (we..n).find(|&i| !ms[i].space).unwrap_or(n);
                let ww: i32 = ms[k..we].iter().map(|m| m.w).sum();
                if x + ww > sx1 {
                    break;
                }
                for i in k..ws {
                    x = push(&mut boxes, i, x);
                }
                k = ws;
            }
            if boxes.len() > b0 {
                runs.push((sx0, sx1, b0, boxes.len()));
            }
            if k >= n {
                break;
            }
        }
        if boxes.is_empty() && k < n {
            let full = segs.len() == 1 && segs[0] == (left, right);
            if !(full || ignore) {
                return Err(self.skip_y(y, h));
            }
            // palavra maior que a coluna: quebra no meio
            let (sx0, sx1) = segs[0];
            let mut x = sx0;
            while k < n && (boxes.is_empty() || x + ms[k].w <= sx1) {
                x = push(&mut boxes, k, x);
                k += 1;
            }
            runs.push((sx0, sx1, 0, boxes.len()));
        }
        if boxes.is_empty() {
            // linha vazia (parágrafo vazio)
            runs.push((segs[0].0, segs[0].1, 0, 0));
        }
        let asc = boxes.iter().map(|b| b.ascent).max().unwrap_or(mark.asc);
        let desc = boxes.iter().map(|b| b.descent).max().unwrap_or(mark.desc);
        Ok(Cand {
            end: k,
            boxes,
            asc,
            desc,
            runs,
            start,
        })
    }

    fn lay_paragraph(&mut self, pi: usize) {
        let doc = self.doc;
        let par = &doc.paras[pi];
        let props = par.props;
        if props.new_page && self.page_has_lines {
            self.new_page();
        }
        let n = par.items.len();
        let first_word = par.items.first().is_some_and(|i| !i.is_space());
        let mut ms: Vec<Metric> = (0..n).map(|k| self.measure(par, pi, k, 1)).collect();
        let mark = self.mark_metric(&par.mark);
        let mut drop = None;
        let mut k = 0;
        if let Some(lines) = props.drop_cap.filter(|&l| l > 0 && first_word) {
            let lines = i32::from(lines);
            let normal = ms.get(1).copied().unwrap_or(ms[0]);
            let big = self.drop_cap_metric(par, pi, lines, normal.asc + normal.desc);
            drop = Some(DropCap {
                m: big,
                line_h: normal.asc + normal.desc,
                line_asc: normal.asc,
                lines,
                placed: None,
            });
            k = 1;
        } else if props.raised_cap && first_word {
            ms[0] = self.measure(par, pi, 0, RAISED_SCALE);
        }
        let mut line_no = 0;
        loop {
            let mut extra = if props.bullets { BULLET_INDENT } else { 0 };
            extra += match props.indent {
                Indent::FirstLine if line_no == 0 => INDENT,
                Indent::Hanging if line_no > 0 => INDENT,
                Indent::All => INDENT,
                _ => 0,
            };
            k = self.place_line(pi, &ms, mark, k, line_no, extra, &mut drop);
            line_no += 1;
            if k >= n {
                break;
            }
        }
        if props.space_after
            && let Some(l) = self.lines.last().filter(|l| l.para == pi)
        {
            // meia linha (a última do parágrafo): +7 na fonte padrão de 15
            self.y += l.height / 2;
        }
    }

    /// Põe uma linha a partir do item `k` (procurando lugar na coluna, na
    /// seguinte ou na folha seguinte); devolve onde a próxima começa.
    #[allow(clippy::too_many_arguments)]
    fn place_line(
        &mut self,
        pi: usize,
        ms: &[Metric],
        mark: Metric,
        k: usize,
        line_no: usize,
        extra: i32,
        drop: &mut Option<DropCap>,
    ) -> usize {
        let doc = self.doc;
        let par = &doc.paras[pi];
        let n = ms.len();
        let mut stuck = 0;
        loop {
            let (cl, cr) = self.cols[self.col];
            let dc = drop.map_or(0, |d| d.extra(line_no, self.page, self.col, self.y));
            // recuo SAMP16: `(começo, recuo)`, o recuo contado a partir do
            // começo do segmento livre (ao lado de um objeto, da borda dele)
            let mut left = (cl + dc, extra);
            if drop.is_none()
                && let Some(z) = self.zone
            {
                // ao lado da capitular de um parágrafo anterior o texto
                // começa depois da letra, ou no recuo próprio se for maior
                let pos = z.left(self.page, self.col, self.y, cl + dc + extra);
                left.0 = pos - extra;
            }
            // "Recuar todas as linhas" recua também do lado direito
            let right = if par.props.indent == Indent::All {
                (cr - INDENT).max(cl + 1)
            } else {
                cr
            };
            let left = (left.0.min(right - 1), left.1);
            let ignore = stuck > 2 * self.cols.len();
            let h_est = match drop {
                Some(d) if line_no == 0 => d.line_h,
                _ => ms.get(k).map_or(mark.asc + mark.desc, |m| m.asc + m.desc),
            };
            let mut cand = self.fill(ms, mark, k, left, right, self.y, h_est, ignore);
            if let Ok(c) = &cand
                && c.height() > h_est
            {
                cand = self.fill(ms, mark, k, left, right, self.y, c.height(), ignore);
            }
            let c = match cand {
                Err(next_y) => {
                    self.y = next_y;
                    if self.y >= self.bottom {
                        if !self.col_has_lines {
                            stuck += 1;
                        }
                        self.next_column();
                    }
                    continue;
                }
                Ok(c) => c,
            };
            let h = c.height();
            if self.y + h > self.bottom && (self.col_has_lines || self.y > self.top) && !ignore {
                if !self.col_has_lines {
                    stuck += 1;
                }
                self.next_column();
                continue;
            }
            self.commit(pi, par, c, k, line_no, extra, (cl, cr), drop);
            return self.lines.last().map_or(n, |l| l.end);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn commit(
        &mut self,
        pi: usize,
        par: &Paragraph,
        mut c: Cand,
        k: usize,
        line_no: usize,
        extra: i32,
        (cl, cr): (i32, i32),
        drop: &mut Option<DropCap>,
    ) {
        let n = par.items.len();
        let last = c.end >= n;
        let props = par.props;
        // alinhamento, segmento a segmento
        for &(sx0, sx1, b0, b1) in &c.runs {
            let run = &mut c.boxes[b0..b1];
            let ink = run.iter().rposition(|b| !b.space);
            let used = ink.map_or(sx0, |i| run[i].x + run[i].w);
            let free = (sx1 - used).max(0);
            match props.align {
                Align::Left => {}
                Align::Right | Align::Center => {
                    let dx = if props.align == Align::Right {
                        free
                    } else {
                        free / 2
                    };
                    run.iter_mut().for_each(|b| b.x += dx);
                }
                Align::Justify if !last => {
                    let ink = ink.unwrap_or(0);
                    let gaps = run[..ink].iter().filter(|b| b.space).count() as i32;
                    if gaps > 0 {
                        let (each, mut rest) = (free / gaps, free % gaps);
                        let mut dx = 0;
                        for b in run.iter_mut() {
                            b.x += dx;
                            if b.space && dx < free {
                                let add = each + i32::from(rest > 0);
                                rest -= 1;
                                b.w += add;
                                dx += add;
                            }
                        }
                    }
                }
                Align::Justify => {}
            }
        }
        let (sx0, sx1, _, _) = c.runs[0];
        let x0 = match props.align {
            Align::Center => (sx0 + sx1) / 2,
            Align::Right => sx1,
            _ => sx0,
        };
        let top = self.y;
        let baseline = top + c.asc;
        for b in &mut c.boxes {
            b.baseline = baseline;
        }
        let mut start = k;
        if line_no == 0
            && let Some(d) = drop
        {
            let x = cl + extra;
            c.boxes.insert(
                0,
                ItemBox {
                    index: 0,
                    x,
                    w: d.m.w,
                    baseline: top + (d.lines - 1) * d.line_h + d.line_asc,
                    ascent: d.m.asc,
                    descent: d.m.desc,
                    size_pt: d.m.size,
                    space: false,
                },
            );
            d.placed = Some((self.page, self.col, top));
            start = 0;
        }
        let bullet = (props.bullets && line_no == 0).then_some(c.start + extra - BULLET_INDENT);
        let h = c.height();
        self.lines.push(Line {
            para: pi,
            start,
            end: c.end,
            page: self.page,
            column: self.col,
            top,
            height: h,
            baseline,
            x0,
            left: cl,
            right: cr,
            boxes: c.boxes,
            bullet,
            last_in_para: last,
        });
        // "entre as linhas": entrelinha dupla
        self.y += h + if props.line_spacing { h } else { 0 };
        if last && let Some(d) = drop {
            // os parágrafos seguintes continuam ao lado da capitular
            self.zone = d.placed.map(|(page, col, t)| DropZone {
                page,
                col,
                x: cl + extra + d.m.w + DROP_GAP,
                bottom: t + d.lines * d.line_h,
            });
        }
        self.col_has_lines = true;
        self.page_has_lines = true;
    }
}

impl Layout {
    /// Diagrama o documento.
    pub fn build(doc: &Document, m: &dyn Measure) -> Layout {
        let dop = &doc.dop;
        let (_, top, _, bottom) = dop.text_area();
        let page_w = i32::from(dop.page_width);
        let page_h = i32::from(dop.page_height);
        let mut b = Builder {
            doc,
            m,
            cols: dop.column_spans(),
            top,
            bottom,
            page_w,
            lines: Vec::new(),
            page: 0,
            col: 0,
            y: top,
            col_has_lines: false,
            page_has_lines: false,
            zone: None,
        };
        for pi in 0..doc.paras.len() {
            b.lay_paragraph(pi);
        }
        let lines = b.lines;
        let text_pages = lines.last().map_or(1, |l| l.page + 1);
        let npages = text_pages
            .max(doc.object_pages())
            .max(usize::from(doc.min_pages))
            .max(1);
        let mut pages = Vec::with_capacity(npages);
        let mut li = 0;
        for p in 0..npages {
            let s = li;
            while li < lines.len() && lines[li].page == p {
                li += 1;
            }
            let objects = (0..doc.objects.len())
                .filter(|&k| doc.object_on_page(k, p))
                .collect();
            let number = dop.page_number.text(p).map(|text| {
                let [t, l, bt, r] = dop.page_number_box().map(i32::from);
                PageNumberBox {
                    text,
                    rect: (l, t, r, bt),
                }
            });
            pages.push(Page {
                lines: s..li,
                objects,
                number,
            });
        }
        Layout {
            page_w,
            page_h,
            lines,
            pages,
        }
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// As linhas da folha.
    pub fn lines_on(&self, page: usize) -> &[Line] {
        self.pages
            .get(page)
            .map_or(&[][..], |p| &self.lines[p.lines.clone()])
    }

    /// Índice da linha em que fica a posição. No fim de uma linha que
    /// continua na seguinte, vale a seguinte.
    pub fn line_of(&self, pos: Pos) -> usize {
        let mut last = None;
        for (k, l) in self.lines.iter().enumerate() {
            if l.para < pos.para {
                continue;
            }
            if l.para > pos.para {
                break;
            }
            last = Some(k);
            if pos.index < l.end || (pos.index == l.end && l.last_in_para) {
                return k;
            }
        }
        last.unwrap_or_else(|| self.lines.len().saturating_sub(1))
    }

    /// Onde desenhar o cursor.
    pub fn caret(&self, pos: Pos) -> Caret {
        let Some(l) = self.lines.get(self.line_of(pos)) else {
            return Caret {
                page: 0,
                x: 0,
                top: 0,
                height: 0,
            };
        };
        let x = match l.boxes.iter().find(|b| b.index == pos.index) {
            Some(b) => b.x,
            None => l
                .boxes
                .iter()
                .filter(|b| b.index < pos.index)
                .max_by_key(|b| b.index)
                .map_or(l.x0, |b| b.x + b.w),
        };
        Caret {
            page: l.page,
            x,
            top: l.top,
            height: l.height,
        }
    }

    /// Posição mais perto de `x` na linha `li`.
    pub fn index_in_line(&self, li: usize, x: i32) -> Pos {
        let l = &self.lines[li];
        let mut best = (l.start, i32::MAX);
        for b in &l.boxes {
            for (bx, i) in [(b.x, b.index), (b.x + b.w, b.index + 1)] {
                let d = (x - bx).abs();
                if d < best.1 {
                    best = (i, d);
                }
            }
        }
        let mut i = best.0.clamp(l.start, l.end);
        if i == l.end && !l.last_in_para && l.end > l.start {
            i -= 1;
        }
        Pos::new(l.para, i)
    }

    /// Posição do clique no ponto `(x, y)` da folha `page`.
    pub fn pos_at(&self, page: usize, x: i32, y: i32) -> Pos {
        let range = match self.pages.get(page) {
            Some(p) if !p.lines.is_empty() => p.lines.clone(),
            _ => {
                // folha sem texto: o fim do texto das folhas anteriores
                let li = self.lines.iter().rposition(|l| l.page <= page).unwrap_or(0);
                return match self.lines.get(li) {
                    Some(l) => Pos::new(l.para, l.end),
                    None => Pos::default(),
                };
            }
        };
        let dist = |l: &Line| {
            let hx = if x < l.left {
                l.left - x
            } else if x >= l.right {
                x - l.right + 1
            } else {
                0
            };
            let vy = if y < l.top {
                l.top - y
            } else if y >= l.top + l.height {
                y - (l.top + l.height) + 1
            } else {
                0
            };
            (hx, vy)
        };
        let li = range
            .min_by_key(|&k| dist(&self.lines[k]))
            .expect("folha com linhas");
        self.index_in_line(li, x)
    }

    /// A posição na linha de cima (`up`) ou de baixo, perto de `x`.
    pub fn vertical(&self, pos: Pos, up: bool, x: i32) -> Option<Pos> {
        let li = self.line_of(pos);
        let target = if up { li.checked_sub(1)? } else { li + 1 };
        (target < self.lines.len()).then(|| self.index_in_line(target, x))
    }

    /// Home: começo da linha.
    pub fn line_home(&self, pos: Pos) -> Pos {
        let l = &self.lines[self.line_of(pos)];
        Pos::new(l.para, l.start)
    }

    /// End: fim da linha (antes do espaço em que ela quebra).
    pub fn line_end(&self, pos: Pos) -> Pos {
        let li = self.line_of(pos);
        let l = &self.lines[li];
        let mut i = l.end;
        if !l.last_in_para && l.end > l.start {
            i -= 1;
        }
        Pos::new(l.para, i)
    }

    /// Retângulos do intervalo, para pintar em vídeo inverso.
    pub fn selection_rects(&self, r: Range) -> Vec<SelRect> {
        let mut out: Vec<SelRect> = Vec::new();
        if r.is_empty() {
            return out;
        }
        for l in &self.lines {
            if l.para < r.start.para || l.para > r.end.para {
                continue;
            }
            let (y0, y1) = (l.top, l.top + l.height);
            let mut cur: Option<(i32, i32)> = None;
            let mut flush = |cur: &mut Option<(i32, i32)>| {
                if let Some((a, b)) = cur.take() {
                    out.push(SelRect {
                        page: l.page,
                        rect: (a, y0, b, y1),
                    });
                }
            };
            for b in &l.boxes {
                if r.contains(Pos::new(l.para, b.index)) {
                    cur = match cur {
                        Some((a, e)) if e == b.x => Some((a, b.x + b.w)),
                        other => {
                            let mut o = other;
                            flush(&mut o);
                            Some((b.x, b.x + b.w))
                        }
                    };
                } else {
                    flush(&mut cur);
                }
            }
            if l.last_in_para && r.contains(Pos::new(l.para, l.end)) {
                let x = l.boxes.last().map_or(l.x0, |b| b.x + b.w);
                cur = match cur {
                    Some((a, e)) if e == x => Some((a, x + MARK_W)),
                    other => {
                        let mut o = other;
                        flush(&mut o);
                        Some((x, x + MARK_W))
                    }
                };
            }
            flush(&mut cur);
        }
        out
    }

    /// Posição em que começa cada folha (para a tabela `PGD ` do `.MAX`).
    /// Uma folha sem texto começa onde começa a seguinte (ou no fim).
    pub fn page_starts(&self, doc: &Document) -> Vec<Pos> {
        let mut out = vec![doc.end(); self.pages.len()];
        let mut next = doc.end();
        for (p, page) in self.pages.iter().enumerate().rev() {
            if let Some(l) = self
                .lines
                .get(page.lines.start)
                .filter(|_| !page.lines.is_empty())
            {
                next = Pos::new(l.para, l.start);
            }
            out[p] = next;
        }
        if let Some(first) = out.first_mut() {
            *first = Pos::default();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::doc::{
        CharChange, CharProps, Margin, Orientation, PageNumber, ParaChange, ParaStyle, Size,
        Spacing,
    };
    use crate::writer::object::{FloatObject, ObjRef, ObjectKind, Outline, Wrap};

    const M: SimpleMeasure = SimpleMeasure;

    fn doc(text: &str) -> Document {
        Document::from_text(text, CharProps::default())
    }

    fn lay(d: &Document) -> Layout {
        Layout::build(d, &M)
    }

    fn line_texts(l: &Layout, d: &Document) -> Vec<String> {
        l.lines
            .iter()
            .map(|ln| {
                d.paras[ln.para].items[ln.start..ln.end]
                    .iter()
                    .filter_map(Item::char)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    /// Texto com `n` palavras "abcd" (cada uma 4×8 = 32 pt + espaço de 8).
    fn words(n: usize) -> String {
        vec!["abcd"; n].join(" ")
    }

    #[test]
    fn empty_document_has_one_line_at_the_margin() {
        let d = Document::new();
        let l = lay(&d);
        assert_eq!(l.page_count(), 1);
        assert_eq!(l.lines.len(), 1);
        let ln = &l.lines[0];
        // margens de 72 pt (2,5 cm): texto em (72, 72), como (93, 180) na tela
        assert_eq!((ln.top, ln.x0, ln.height), (72, 72, 13));
        let c = l.caret(d.start());
        assert_eq!((c.page, c.x, c.top, c.height), (0, 72, 72, 13));
        assert_eq!(l.pages[0].number, None);
        assert_eq!((l.page_w, l.page_h), (595, 792));
    }

    #[test]
    fn wraps_at_the_right_margin() {
        // largura útil 451: 11 palavras de 32 + 10 espaços de 8 = 432
        let d = doc(&words(12));
        let l = lay(&d);
        assert_eq!(l.lines.len(), 2);
        assert_eq!(line_texts(&l, &d), [words(11), words(1)]);
        let first = &l.lines[0];
        assert_eq!(first.boxes.last().unwrap().x, 72 + 11 * 40 - 8);
        assert!(!first.last_in_para && l.lines[1].last_in_para);
        assert_eq!(l.lines[1].top, 72 + 13);
        assert_eq!(first.baseline, 72 + 10);
    }

    #[test]
    fn long_word_is_split() {
        let d = doc(&"x".repeat(60));
        let l = lay(&d);
        // 451 / 8 = 56 letras na primeira linha
        assert_eq!(l.lines[0].end, 56);
        assert_eq!(l.lines.len(), 2);
    }

    #[test]
    fn alignment_and_justify() {
        let mut d = doc(&format!("{}\nab", words(12)));
        let l = lay(&d);
        assert_eq!(l.lines[2].boxes[0].x, 72);
        d.apply_para(d.all(), ParaChange::Align(Align::Right));
        let l = lay(&d);
        let last = &l.lines[2];
        assert_eq!(last.boxes[1].x + last.boxes[1].w, 523);
        d.apply_para(d.all(), ParaChange::Align(Align::Center));
        let l = lay(&d);
        assert_eq!(l.lines[2].boxes[0].x, 72 + (451 - 16) / 2);
        assert_eq!(
            lay(&Document::from_text("", CharProps::default())).lines[0].x0,
            72
        );
        d.apply_para(d.all(), ParaChange::Align(Align::Justify));
        let l = lay(&d);
        let first = &l.lines[0];
        let ink = first.boxes.iter().rev().find(|b| !b.space).unwrap();
        assert_eq!(ink.x + ink.w, 523, "linha justificada vai até a margem");
        // a última linha do parágrafo não se justifica
        assert_eq!(l.lines[1].boxes[0].x, 72);
    }

    #[test]
    fn line_heights_follow_the_biggest_letter() {
        let mut d = doc("ab cd");
        d.apply_char(
            Range::new(Pos::new(0, 3), Pos::new(0, 4)),
            CharChange::Size(Size::Giant),
        );
        let l = lay(&d);
        let ln = &l.lines[0];
        // 48 pt: ascendente 39, descendente 12
        assert_eq!((ln.height, ln.baseline - ln.top), (51, 39));
        assert!(ln.boxes.iter().all(|b| b.baseline == ln.baseline));
        assert_eq!(ln.boxes[3].size_pt, 48);
        // parágrafo vazio com o \r grande
        let mut d = Document::new();
        d.apply_char(Range::at(d.start()), CharChange::Size(Size::Large));
        assert_eq!(lay(&d).lines[0].height, 20 + 6);
    }

    #[test]
    fn indents_and_bullets() {
        let text = words(24);
        let mut d = doc(&text);
        for (ind, first, second) in [
            (Indent::None, 72, 72),
            (Indent::FirstLine, 72 + INDENT, 72),
            (Indent::Hanging, 72, 72 + INDENT),
            (Indent::All, 72 + INDENT, 72 + INDENT),
        ] {
            d.apply_para(d.all(), ParaChange::Indent(ind));
            let l = lay(&d);
            assert_eq!(
                (l.lines[0].boxes[0].x, l.lines[1].boxes[0].x),
                (first, second),
                "{ind:?}"
            );
        }
        // "Recuar todas as linhas" recua dos dois lados (Poesia: x 130..509)
        d.apply_para(d.all(), ParaChange::Indent(Indent::All));
        let l = lay(&d);
        for ln in &l.lines {
            let ink = ln.boxes.iter().rev().find(|b| !b.space).unwrap();
            assert!(ink.x + ink.w <= 523 - INDENT, "{}", ink.x + ink.w);
        }
        d.apply_para(d.all(), ParaChange::Indent(Indent::None));
        d.apply_para(d.all(), ParaChange::Style(ParaStyle::Bullets));
        let l = lay(&d);
        assert_eq!(l.lines[0].bullet, Some(72));
        assert_eq!(l.lines[1].bullet, None);
        assert_eq!(l.lines[0].boxes[0].x, 72 + BULLET_INDENT);
        assert_eq!(l.lines[1].boxes[0].x, 72 + BULLET_INDENT);
    }

    #[test]
    fn spacing_options() {
        let mut d = doc("a\nb");
        let base = lay(&d).lines[1].top;
        assert_eq!(base, 72 + 13);
        d.apply_para(
            Range::at(Pos::new(0, 0)),
            ParaChange::Spacing(Spacing::ExtraLines),
        );
        assert_eq!(lay(&d).lines[1].top, 72 + 13 + 13);
        d.apply_para(
            Range::at(Pos::new(0, 0)),
            ParaChange::Spacing(Spacing::ExtraAfter),
        );
        assert_eq!(lay(&d).lines[1].top, 72 + 13 + 6);
        d.apply_para(
            Range::at(Pos::new(1, 0)),
            ParaChange::Spacing(Spacing::NewPage),
        );
        let l = lay(&d);
        assert_eq!(l.page_count(), 2);
        assert_eq!((l.lines[1].page, l.lines[1].top), (1, 72));
        // nova página no primeiro parágrafo não cria folha em branco
        let mut d = doc("a");
        d.apply_para(d.all(), ParaChange::Spacing(Spacing::NewPage));
        assert_eq!(lay(&d).page_count(), 1);
    }

    #[test]
    fn page_breaks_never_split_a_line() {
        // 648 pt de altura útil / 13 = 49 linhas por folha
        let text = vec!["x"; 120].join("\n");
        let d = doc(&text);
        let l = lay(&d);
        assert_eq!(l.page_count(), 3);
        assert_eq!(l.lines_on(0).len(), 49);
        assert_eq!(l.lines_on(1)[0].top, 72);
        assert!(l.lines.iter().all(|ln| ln.top + ln.height <= 720));
        assert_eq!(
            l.page_starts(&d),
            vec![Pos::new(0, 0), Pos::new(49, 0), Pos::new(98, 0)]
        );
        let c = l.caret(Pos::new(50, 1));
        assert_eq!((c.page, c.top), (1, 72 + 13));
    }

    #[test]
    fn columns_and_landscape() {
        let mut d = doc(&vec!["x"; 60].join("\n"));
        d.dop.set_columns(2);
        let l = lay(&d);
        assert_eq!(l.page_count(), 1);
        let second = &l.lines[49];
        assert_eq!((second.column, second.top), (1, 72));
        let cols = d.dop.column_spans();
        assert_eq!(second.boxes[0].x, cols[1].0);
        assert_eq!(cols[1].0 - cols[0].1, 12);
        d.dop.set_orientation(Orientation::Wide);
        d.dop.set_margin(Margin::Cm38);
        d.dop.set_columns(3);
        let l = lay(&d);
        assert_eq!((l.page_w, l.page_h), (792, 595));
        // 595 - 216 = 379 de altura útil: 29 linhas por coluna, 87 por folha
        assert_eq!(l.lines_on(0).len(), 60);
        assert_eq!(l.lines[29].column, 1);
        assert_eq!(l.lines[58].column, 2);
    }

    #[test]
    fn page_numbers() {
        let mut d = doc(&vec!["x"; 60].join("\n"));
        d.dop.set_page_number(PageNumber::Label);
        let l = lay(&d);
        let n = l.pages[1].number.as_ref().unwrap();
        assert_eq!(n.text, "Pág. 2");
        assert_eq!(n.rect, (265, 777, 329, 792));
        d.dop.set_page_number(PageNumber::Plain);
        d.dop.page_number_rect = Some([10, 20, 30, 40]);
        let l = lay(&d);
        let n = l.pages[0].number.as_ref().unwrap();
        assert_eq!((n.text.as_str(), n.rect), ("1", (20, 10, 40, 30)));
    }

    fn sticker(x: i16, y: i16, w: u16, h: u16) -> FloatObject {
        FloatObject::new(ObjectKind::Sticker, ObjRef::pict(2), 0, x, y, w, h)
    }

    #[test]
    fn text_flows_on_both_sides_of_an_object() {
        let mut d = doc(&words(40));
        d.add_object(sticker(250, 72, 100, 30));
        let l = lay(&d);
        let first = &l.lines[0];
        // a linha pula 244..356 (objeto + folga de 6)
        assert!(
            first
                .boxes
                .iter()
                .all(|b| b.space || b.x + b.w <= 244 || b.x >= 356)
        );
        assert!(
            first.boxes.iter().any(|b| b.x >= 356),
            "texto à direita do adesivo"
        );
        // abaixo do objeto (y >= 108), linha inteira de novo
        let below = l.lines.iter().find(|ln| ln.top >= 108).unwrap();
        assert!(
            below
                .boxes
                .iter()
                .any(|b| !b.space && b.x > 244 && b.x < 356)
        );
    }

    #[test]
    fn wrap_modes() {
        let mut d = doc(&words(60));
        let mut o = sticker(200, 80, 100, 40);
        o.outline = Some(Outline {
            width: 100,
            height: 40,
            left: vec![(40, 0), (40, 39)],
            right: vec![(60, 0), (60, 39)],
        });
        d.add_object(o);
        let gap_of = |l: &Layout| {
            // maior espaço sem letras na 2ª linha (que cruza o objeto)
            let ln = &l.lines[1];
            let mut xs: Vec<(i32, i32)> = ln
                .boxes
                .iter()
                .filter(|b| !b.space)
                .map(|b| (b.x, b.x + b.w))
                .collect();
            xs.sort_unstable();
            xs.windows(2).map(|w| w[1].0 - w[0].1).max().unwrap_or(0)
        };
        // contorno: só a parte desenhada (40..60) + folga
        let contour = gap_of(&lay(&d));
        assert!((32..=60).contains(&contour), "{contour}");
        d.objects[0].wrap = Wrap::Box;
        let boxed = gap_of(&lay(&d));
        assert!((112..=140).contains(&boxed), "{boxed}");
        d.objects[0].wrap = Wrap::TopBottom;
        let l = lay(&d);
        assert!(
            l.lines
                .iter()
                .all(|ln| ln.top + ln.height <= 74 || ln.top >= 126)
        );
        // a 1ª linha já cruzaria a folga (74..126): o texto começa abaixo
        assert_eq!(l.lines[0].top, 126);
        d.objects[0].wrap = Wrap::AllPages;
        d.objects[0].page = 3;
        let l = lay(&d);
        assert_eq!(l.page_count(), 4);
        assert!(l.pages.iter().all(|p| p.objects == vec![0]));
        let g = gap_of(&l);
        assert!((32..=60).contains(&g), "{g}");
    }

    /// SAMP16 (Jornal Mágico): a manchete fica numa palavra com forma à
    /// esquerda e os parágrafos têm recuo da 1ª linha. O recuo conta a
    /// partir de onde o texto começa ao lado do objeto, não da margem
    /// (senão o objeto "engole" o recuo).
    #[test]
    fn indent_counts_from_the_object_edge() {
        let mut d = doc(&words(300));
        let mut o = sticker(60, 60, 70, 300);
        o.wrap = Wrap::Box;
        d.add_object(o);
        let edge = 130 + WRAP_GAP;
        d.apply_para(d.all(), ParaChange::Indent(Indent::FirstLine));
        let l = lay(&d);
        assert_eq!(l.lines[0].boxes[0].x, edge + INDENT);
        assert_eq!(l.lines[1].boxes[0].x, edge);
        d.apply_para(d.all(), ParaChange::Indent(Indent::Hanging));
        let l = lay(&d);
        assert_eq!(l.lines[0].boxes[0].x, edge);
        assert_eq!(l.lines[1].boxes[0].x, edge + INDENT);
        d.apply_para(d.all(), ParaChange::Indent(Indent::None));
        d.apply_para(d.all(), ParaChange::Style(ParaStyle::Bullets));
        let l = lay(&d);
        assert_eq!(l.lines[0].bullet, Some(edge));
        assert_eq!(l.lines[0].boxes[0].x, edge + BULLET_INDENT);
        // abaixo do objeto volta a valer a margem
        let below = l.lines.iter().find(|ln| ln.top >= 366).unwrap();
        assert_eq!(below.boxes[0].x, 72 + BULLET_INDENT);
    }

    #[test]
    fn objects_on_later_pages_add_pages() {
        let mut d = doc("x");
        let mut o = sticker(10, 10, 20, 20);
        o.page = 2;
        d.add_object(o);
        let l = lay(&d);
        assert_eq!(l.page_count(), 3);
        assert_eq!(l.pages[2].objects, vec![0]);
        assert!(l.pages[0].objects.is_empty());
        assert_eq!(l.page_starts(&d), vec![Pos::default(), d.end(), d.end()]);
        assert_eq!(l.pos_at(2, 100, 100), Pos::new(0, 1));
    }

    #[test]
    fn object_covering_the_whole_area_does_not_hang() {
        let mut d = doc(&words(30));
        let mut o = sticker(0, 0, 595, 792);
        o.wrap = Wrap::AllPages;
        d.add_object(o);
        let l = lay(&d);
        assert!(l.lines.len() >= 3);
        assert_eq!(l.lines.last().unwrap().end, d.paras[0].len());
    }

    #[test]
    fn drop_and_raised_caps() {
        let mut d = doc(&words(40));
        d.apply_para(d.all(), ParaChange::Style(ParaStyle::DropCap));
        let l = lay(&d);
        let first = &l.lines[0];
        let cap = first.boxes[0];
        assert_eq!((cap.index, cap.x), (0, 72));
        // a maiúscula vai do alto das maiúsculas da 1ª linha à base da 3ª
        // (office-complemento.md §3: ~32×40 px na fonte padrão)
        let key = |pt| FontKey {
            name: "",
            size_pt: pt,
            bold: false,
            italic: false,
        };
        let want = 2 * 13 + M.cap_height(&key(12));
        assert!(
            (M.cap_height(&key(cap.size_pt)) - want).abs() <= 1,
            "{}",
            cap.size_pt
        );
        assert!(cap.size_pt > 36);
        // base da capitular = base da 3ª linha
        assert_eq!(cap.baseline, l.lines[2].baseline);
        let indent = 72 + cap.w + DROP_GAP;
        for k in 0..3 {
            let b = l.lines[k].boxes.iter().find(|b| b.index > 0).unwrap();
            assert_eq!(b.x, indent, "linha {k}");
        }
        assert_eq!(l.lines[3].boxes[0].x, 72);
        assert_eq!(l.lines[0].start, 0);
        assert_eq!(l.caret(Pos::new(0, 0)).x, 72);
        assert_eq!(l.caret(Pos::new(0, 1)).x, indent);
        // parágrafo curto: o seguinte continua recuado ao lado da
        // capitular enquanto ela não acaba (Mix "Era uma vez")
        let mut d = doc(&format!("abc\n{}\n{}", words(2), words(30)));
        d.apply_para(
            Range::at(Pos::new(0, 0)),
            ParaChange::Style(ParaStyle::DropCap),
        );
        let l = lay(&d);
        let cap = l.lines[0].boxes[0];
        let beside = 72 + cap.w + DROP_GAP;
        assert_eq!(l.lines[1].top, 72 + 13);
        assert_eq!(l.lines[1].boxes[0].x, beside);
        assert_eq!(l.lines[2].boxes[0].x, beside);
        // a 4ª linha já está abaixo da capitular
        assert_eq!(l.lines[3].top, 72 + 3 * 13);
        assert_eq!(l.lines[3].boxes[0].x, 72);
        // um recuo próprio maior que a capitular vale (Poesia ao lado)
        d.apply_para(Range::at(Pos::new(1, 0)), ParaChange::Indent(Indent::All));
        let l = lay(&d);
        assert_eq!(l.lines[1].boxes[0].x, beside.max(72 + INDENT));
        // destacada: a 1ª letra cresce e a linha também
        let mut d = doc("abc");
        d.apply_para(d.all(), ParaChange::Style(ParaStyle::RaisedCap));
        let l = lay(&d);
        assert_eq!(l.lines[0].boxes[0].size_pt, 36);
        assert_eq!(l.lines[0].boxes[1].size_pt, 12);
        assert_eq!(l.lines[0].height, 29 + 9);
    }

    #[test]
    fn rebus_uses_measure_size() {
        let mut d = doc("gato");
        d.insert_item(d.end(), Item::Rebus(0x49));
        let l = lay(&d);
        let b = l.lines[0].boxes[4];
        assert_eq!((b.w, b.ascent, b.descent), (13, 13, 0));
        assert_eq!(l.lines[0].height, 16);
    }

    #[test]
    fn caret_hit_testing_and_navigation() {
        let d = doc(&format!("{}\nab", words(12)));
        let l = lay(&d);
        // fim da 1ª linha = começo da 2ª
        let wrap = Pos::new(0, l.lines[0].end);
        assert_eq!(l.line_of(wrap), 1);
        assert_eq!(l.caret(wrap).x, 72);
        assert_eq!(l.caret(Pos::new(0, 1)).x, 80);
        assert_eq!(l.caret(Pos::new(1, 2)).x, 88);
        assert_eq!(l.pos_at(0, 83, 75), Pos::new(0, 1));
        assert_eq!(l.pos_at(0, 85, 75), Pos::new(0, 2));
        // clique à direita do fim de uma linha que quebra: antes do espaço
        assert_eq!(l.pos_at(0, 580, 75), Pos::new(0, l.lines[0].end - 1));
        // clique abaixo do texto: última linha
        assert_eq!(l.pos_at(0, 500, 700), Pos::new(1, 2));
        assert_eq!(l.pos_at(0, 0, 0), Pos::new(0, 0));
        assert_eq!(
            l.vertical(Pos::new(0, 1), false, 80),
            Some(Pos::new(0, l.lines[0].end + 1))
        );
        assert_eq!(l.vertical(Pos::new(0, 1), true, 80), None);
        assert_eq!(
            l.vertical(Pos::new(1, 0), true, 80),
            Some(Pos::new(0, l.lines[1].start + 1))
        );
        assert_eq!(l.line_home(Pos::new(0, 5)), Pos::new(0, 0));
        assert_eq!(l.line_end(Pos::new(0, 5)), Pos::new(0, l.lines[0].end - 1));
        assert_eq!(l.line_end(Pos::new(1, 0)), Pos::new(1, 2));
    }

    #[test]
    fn clicks_in_columns() {
        let mut d = doc(&vec!["x"; 60].join("\n"));
        d.dop.set_columns(2);
        let l = lay(&d);
        let cols = d.dop.column_spans();
        // clique na 2ª coluna abaixo do fim dela: última linha da 2ª coluna
        assert_eq!(l.pos_at(0, cols[1].0 + 6, 700), Pos::new(59, 1));
        assert_eq!(l.pos_at(0, cols[0].0 + 6, 700), Pos::new(48, 1));
        assert_eq!(l.pos_at(0, cols[1].0 + 1, 75), Pos::new(49, 0));
    }

    #[test]
    fn selection_rectangles() {
        let d = doc("ab cd\nef");
        let l = lay(&d);
        let r = l.selection_rects(Range::new(Pos::new(0, 1), Pos::new(1, 1)));
        assert_eq!(
            r,
            vec![
                SelRect {
                    page: 0,
                    rect: (80, 72, 112 + MARK_W, 85)
                },
                SelRect {
                    page: 0,
                    rect: (72, 85, 80, 98)
                },
            ]
        );
        assert!(l.selection_rects(Range::at(Pos::new(0, 1))).is_empty());
        // com objeto no meio, os pedaços da linha viram retângulos separados
        let mut d = doc(&words(20));
        d.add_object(sticker(250, 72, 100, 30));
        let l = lay(&d);
        let r = l.selection_rects(Range::new(Pos::new(0, 0), Pos::new(0, l.lines[0].end)));
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn background_obstacles_push_the_text_on_every_page() {
        // Scroll (FUNDO_SC.MAX): rolo de cima sem texto ao lado, tira
        // esquerda com texto só à direita, tira direita com texto só à
        // esquerda (office-complemento.md §5.3)
        let mut d = Document::from_text(&"abc de ".repeat(1500), CharProps::default());
        let ob = |x, y, w, h, wrap| {
            let mut f = formats::max::Frame::new(
                formats::max::FrameKind::Other(6),
                formats::max::ObjRef::new(b"PICT", 0),
                x,
                y,
                w,
                h,
            );
            f.page = u16::MAX;
            f.wrap = Some(wrap);
            f
        };
        d.dop.obstacles = vec![
            ob(73, 10, 443, 121, 3),
            ob(73, 677, 443, 107, 3),
            ob(71, 127, 12, 555, 2),
            ob(501, 125, 12, 555, 1),
        ];
        let l = Layout::build(&d, &M);
        assert!(l.lines[0].top >= 131, "{}", l.lines[0].top);
        assert!(l.page_count() >= 2);
        for line in &l.lines {
            for b in &line.boxes {
                assert!(b.x >= 83 && b.x + b.w <= 501, "{} {}", b.x, b.w);
                assert!(line.top + line.height <= 677, "{}", line.top);
            }
        }
        // a folha 2 também começa abaixo do rolo de cima
        let second = l.lines.iter().find(|line| line.page == 1).unwrap();
        assert!(second.top >= 131);
    }
}
