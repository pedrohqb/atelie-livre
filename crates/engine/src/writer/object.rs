//! Objetos flutuantes do Escritório: adesivos, desenhos do Cavalete de lápis
//! de cera, palavras com forma (Splots) e as molduras de página dos cartões.
//! Cada um tem página, posição e tamanho em pontos da página, modo de
//! "Flutua-texto" (como o texto o contorna), som e borda.
//!
//! Corresponde 1:1 à moldura `FRAM` do `.MAX` (`formats::max::Frame`, ver
//! `docs/formats/max.md`): os campos têm os mesmos nomes e unidades
//! (`page`, `x`, `y`, `width`, `height`, `orig_width`, `orig_height`,
//! `object`, `sound`, `wrap`). O conteúdo (figura, Splot, som, borda) não
//! fica aqui: o objeto só o cita por [`ObjRef`], como o arquivo faz.

/// Referência a um chunk de objeto do documento ou das bibliotecas: etiqueta
/// de 4 letras e número (`PICT 2`, `SPLE 0`, `SND 803`, `BDR 0`). É o
/// argumento de 6 bytes dos sprms `0x35`/`0x37` (moldura) e `0x58`/`0x5a`
/// (documento).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjRef {
    pub tag: [u8; 4],
    pub id: u16,
}

impl ObjRef {
    pub const PICT: [u8; 4] = *b"PICT";
    pub const SPLE: [u8; 4] = *b"SPLE";
    pub const SND: [u8; 4] = *b"SND ";
    pub const BDR: [u8; 4] = *b"BDR ";

    /// Figura (adesivo, figura importada, desenho do cavalete).
    pub fn pict(id: u16) -> ObjRef {
        ObjRef {
            tag: Self::PICT,
            id,
        }
    }

    /// Palavra com forma (Splot).
    pub fn sple(id: u16) -> ObjRef {
        ObjRef {
            tag: Self::SPLE,
            id,
        }
    }

    /// Som (WAVE da biblioteca `MAX100.SND`, de `WRITER.SND` ou gravado).
    pub fn snd(id: u16) -> ObjRef {
        ObjRef { tag: Self::SND, id }
    }

    /// Borda ou fundo de página (`?MAX100.BOR`, `0MAG100.BOR`).
    pub fn bdr(id: u16) -> ObjRef {
        ObjRef { tag: Self::BDR, id }
    }
}

/// Que tipo de objeto é. No `.MAX` o tipo da moldura (sprm `0x28`) só
/// distingue figura (3), Splot (2) e moldura de página (4); adesivo e
/// desenho são ambos figuras `PICT` (o desenho do cavalete não tem a prévia
/// `PKBM`), por isso a leitura deve decidir entre os dois pela figura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ObjectKind {
    /// Adesivo de clip-art (ferramenta Seleciona-adesivos, Cachorrinho).
    #[default]
    Sticker,
    /// Desenho feito no Cavalete de lápis de cera.
    Drawing,
    /// Palavra com forma (FORMA, Cavalete de palavras com forma).
    Shaped,
    /// Moldura de página dos cartões (Oficina de Projetos).
    PageFrame,
}

impl ObjectKind {
    /// Código do sprm `0x28`.
    pub fn frame_code(self) -> u8 {
        match self {
            ObjectKind::Shaped => 2,
            ObjectKind::Sticker | ObjectKind::Drawing => 3,
            ObjectKind::PageFrame => 4,
        }
    }

    /// Tipo pelo código do sprm `0x28`; uma figura (3) volta como adesivo.
    pub fn from_frame_code(code: u8) -> Option<ObjectKind> {
        match code {
            2 => Some(ObjectKind::Shaped),
            3 => Some(ObjectKind::Sticker),
            4 => Some(ObjectKind::PageFrame),
            _ => None,
        }
    }

    /// Os que o Mover-modificar arrasta e o Dimensionar estica.
    pub fn is_movable(self) -> bool {
        !matches!(self, ObjectKind::PageFrame)
    }
}

/// Os 4 modos da lista do Flutua-texto (Transformadores, bitmaps `0c2b`,
/// `0c2d`, `0c2f`, `0c31`), na ordem da lista.
///
/// Código do sprm `0x29` (provisório, a confirmar no original — W26): o
/// índice na lista a partir de 0, com o modo normal sem sprm. Nos exemplos
/// só aparecem 3 (uma figura de jornal: "acima e abaixo") e 2 (as molduras de
/// página dos cartões).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Wrap {
    /// "As palavras flutuam em volta das bordas — é o normal": o texto
    /// segue o contorno da figura ([`Outline`]), dos dois lados.
    #[default]
    Contour,
    /// "Coloca o adesivo em todas as páginas": o objeto se repete na mesma
    /// posição em todas as páginas, com o texto em volta das bordas.
    AllPages,
    /// "As palavras flutuam, deixando um quadrado de espaço em branco": o
    /// texto contorna o retângulo do objeto.
    Box,
    /// "As palavras flutuam acima e abaixo": nenhuma palavra ao lado do objeto.
    TopBottom,
}

impl Wrap {
    pub const ALL: [Wrap; 4] = [Wrap::Contour, Wrap::AllPages, Wrap::Box, Wrap::TopBottom];

    /// Valor do sprm `0x29`; `None` = sem sprm (modo normal).
    pub fn code(self) -> Option<u8> {
        match self {
            Wrap::Contour => None,
            Wrap::AllPages => Some(1),
            Wrap::Box => Some(2),
            Wrap::TopBottom => Some(3),
        }
    }

    /// Modo pelo sprm `0x29`; ausente ou desconhecido = normal.
    pub fn from_code(code: Option<u8>) -> Wrap {
        match code {
            Some(1) => Wrap::AllPages,
            Some(2) => Wrap::Box,
            Some(3) => Wrap::TopBottom,
            _ => Wrap::Contour,
        }
    }
}

/// Contorno de um objeto para o texto contornar: a borda esquerda e a
/// direita da parte desenhada, como pontos `(x, y)` com `y` crescente de 0 à
/// altura, nas medidas originais do objeto. É a parte `TWPB` dos `PICT` do
/// `.MAX` (cabeçalho `u16 8, largura, altura, nº de pontos à esquerda, nº à
/// direita`, depois os pontos).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outline {
    pub width: u16,
    pub height: u16,
    pub left: Vec<(i16, i16)>,
    pub right: Vec<(i16, i16)>,
}

impl Outline {
    /// Contorno de uma figura `w × h` a partir da máscara (`true` = pixel
    /// desenhado): um ponto por linha com tinta, nas bordas mais à esquerda
    /// e mais à direita. Linhas sem tinta ficam de fora (o texto pode
    /// passar por elas).
    pub fn from_mask(w: u16, h: u16, opaque: impl Fn(u16, u16) -> bool) -> Outline {
        let mut left = Vec::new();
        let mut right = Vec::new();
        for y in 0..h {
            let mut xs = (0..w).filter(|&x| opaque(x, y));
            if let Some(first) = xs.next() {
                let last = xs.last().unwrap_or(first);
                left.push((first as i16, y as i16));
                right.push((last as i16 + 1, y as i16));
            }
        }
        Outline {
            width: w,
            height: h,
            left,
            right,
        }
    }

    /// Extensão horizontal `(esq, dir)` da parte desenhada entre as alturas
    /// `y0..y1` (em medidas originais), interpolando entre os pontos; `None`
    /// se não há desenho nessa faixa.
    pub fn span(&self, y0: i32, y1: i32) -> Option<(i32, i32)> {
        let l = edge(&self.left, y0, y1, i32::min)?;
        let r = edge(&self.right, y0, y1, i32::max)?;
        Some((l, r))
    }
}

/// O mais extremo (`pick`) dos `x` da borda dentro de `y0..y1`, incluindo os
/// valores interpolados nos limites da faixa.
fn edge(pts: &[(i16, i16)], y0: i32, y1: i32, pick: fn(i32, i32) -> i32) -> Option<i32> {
    let mut best: Option<i32> = None;
    let mut take = |x: i32| best = Some(best.map_or(x, |b| pick(b, x)));
    for (k, &(x, y)) in pts.iter().enumerate() {
        let (x, y) = (i32::from(x), i32::from(y));
        if (y0..y1).contains(&y) {
            take(x);
        }
        if let Some(&(nx, ny)) = pts.get(k + 1) {
            let (nx, ny) = (i32::from(nx), i32::from(ny));
            for t in [y0, y1 - 1] {
                if y < t && t < ny {
                    take(x + (nx - x) * (t - y) / (ny - y));
                }
            }
        }
    }
    best
}

/// Como esticar com os botões Dimensionar (Transformadores).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resize {
    /// "Dimensionar tudo": largura e altura na mesma proporção.
    All,
    /// "Dimensionar lateral": só a largura.
    Width,
    /// "Dimensionar vertical": só a altura.
    Height,
}

/// Menor lado de um objeto, em pontos.
pub const MIN_SIDE: u16 = 8;

/// Um objeto solto na página.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloatObject {
    pub kind: ObjectKind,
    /// O conteúdo: `PICT n` (adesivo, desenho), `SPLE n` (palavra com forma).
    pub object: ObjRef,
    /// Página (0 = primeira). Com [`Wrap::AllPages`] o objeto aparece em todas.
    pub page: u16,
    /// Canto superior esquerdo, em pontos a partir do canto da página.
    pub x: i16,
    pub y: i16,
    /// Tamanho atual, em pontos.
    pub width: u16,
    pub height: u16,
    /// Tamanho de quando foi colado (difere do atual depois de esticar).
    pub orig_width: u16,
    pub orig_height: u16,
    pub wrap: Wrap,
    /// Som preso ao objeto (Seleciona-sons); a Varinha Mágica o toca.
    pub sound: Option<ObjRef>,
    /// Borda em volta do objeto (Preparação da página, 2ª página). O sprm da
    /// moldura que a grava ainda não foi identificado (W26).
    pub border: Option<ObjRef>,
    /// Contorno para o modo normal; sem ele, vale o retângulo.
    pub outline: Option<Outline>,
}

impl FloatObject {
    /// Objeto no lugar e tamanho dados, com o tamanho original igual ao atual.
    pub fn new(
        kind: ObjectKind,
        object: ObjRef,
        page: u16,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    ) -> FloatObject {
        FloatObject {
            kind,
            object,
            page,
            x,
            y,
            width,
            height,
            orig_width: width,
            orig_height: height,
            wrap: Wrap::Contour,
            sound: None,
            border: None,
            outline: None,
        }
    }

    /// Objeto centrado no ponto `(cx, cy)` da página — como o adesivo colado
    /// com um clique.
    pub fn centered(
        kind: ObjectKind,
        object: ObjRef,
        page: u16,
        cx: i32,
        cy: i32,
        width: u16,
        height: u16,
    ) -> FloatObject {
        let x = clamp_i16(cx - i32::from(width) / 2);
        let y = clamp_i16(cy - i32::from(height) / 2);
        FloatObject::new(kind, object, page, x, y, width, height)
    }

    /// Retângulo `(x0, y0, x1, y1)` na página, com o fim exclusivo.
    pub fn rect(&self) -> (i32, i32, i32, i32) {
        let (x, y) = (i32::from(self.x), i32::from(self.y));
        (x, y, x + i32::from(self.width), y + i32::from(self.height))
    }

    /// O objeto aparece na página `page`.
    pub fn on_page(&self, page: usize) -> bool {
        self.wrap == Wrap::AllPages || usize::from(self.page) == page
    }

    /// O ponto `(x, y)` da página cai no retângulo do objeto.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        let (x0, y0, x1, y1) = self.rect();
        (x0..x1).contains(&x) && (y0..y1).contains(&y)
    }

    /// Arrasta (Mover-modificar), mantendo ao menos um canto de
    /// [`MIN_SIDE`] dentro da página `page_w × page_h`.
    pub fn move_by(&mut self, dx: i32, dy: i32, page_w: i32, page_h: i32) {
        let (w, h) = (i32::from(self.width), i32::from(self.height));
        let m = i32::from(MIN_SIDE);
        let x = (i32::from(self.x) + dx).clamp(m - w, (page_w - m).max(m - w));
        let y = (i32::from(self.y) + dy).clamp(m - h, (page_h - m).max(m - h));
        self.x = clamp_i16(x);
        self.y = clamp_i16(y);
    }

    /// Estica para `width × height` (Dimensionar), com o canto superior
    /// esquerdo fixo. Em [`Resize::All`] vale a maior das duas proporções,
    /// para que a figura não se deforme; nos outros só um lado muda.
    pub fn resize(&mut self, mode: Resize, width: i32, height: i32) {
        let clamp = |v: i32| v.clamp(i32::from(MIN_SIDE), i32::from(i16::MAX)) as u16;
        match mode {
            Resize::Width => self.width = clamp(width),
            Resize::Height => self.height = clamp(height),
            Resize::All => {
                let (w0, h0) = (f64::from(self.width.max(1)), f64::from(self.height.max(1)));
                let k = (f64::from(width) / w0).max(f64::from(height) / h0);
                let min_k = f64::from(MIN_SIDE) / w0.min(h0);
                let k = k.max(min_k);
                self.width = clamp((w0 * k).round() as i32);
                self.height = clamp((h0 * k).round() as i32);
            }
        }
    }

    /// Volta ao tamanho de quando foi colado.
    pub fn reset_size(&mut self) {
        self.width = self.orig_width;
        self.height = self.orig_height;
    }

    /// Extensão horizontal `(x0, x1)` na página que o texto deve evitar na
    /// faixa vertical `y0..y1` da página, conforme o modo do Flutua-texto, já
    /// com a folga `gap` em volta; `None` se o objeto não ocupa essa faixa.
    /// Em [`Wrap::TopBottom`] devolve a largura toda da página (`page_w`).
    pub fn exclusion(&self, y0: i32, y1: i32, gap: i32, page_w: i32) -> Option<(i32, i32)> {
        let (ox0, oy0, ox1, oy1) = self.rect();
        if y1 <= oy0 - gap || y0 >= oy1 + gap {
            return None;
        }
        match self.wrap {
            Wrap::TopBottom => Some((i32::MIN / 2, page_w.max(ox1) + gap)),
            Wrap::Box => Some((ox0 - gap, ox1 + gap)),
            Wrap::Contour | Wrap::AllPages => match &self.outline {
                Some(o) if o.width > 0 && o.height > 0 => {
                    // faixa em medidas originais do contorno
                    let (ow, oh) = (i32::from(o.width), i32::from(o.height));
                    let (w, h) = (i32::from(self.width).max(1), i32::from(self.height).max(1));
                    let ty0 = ((y0 - gap - oy0).max(0) * oh) / h;
                    let ty1 = ceil_div((y1 + gap - oy0).min(h) * oh, h);
                    let (l, r) = o.span(ty0, ty1.max(ty0 + 1))?;
                    Some((ox0 + l * w / ow - gap, ox0 + ceil_div(r * w, ow) + gap))
                }
                _ => Some((ox0 - gap, ox1 + gap)),
            },
        }
    }
}

/// Divisão arredondada para cima (`b > 0`).
fn ceil_div(a: i32, b: i32) -> i32 {
    a.div_euclid(b) + i32::from(a.rem_euclid(b) != 0)
}

fn clamp_i16(v: i32) -> i16 {
    v.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sticker() -> FloatObject {
        FloatObject::new(ObjectKind::Sticker, ObjRef::pict(2), 0, 254, 570, 149, 150)
    }

    #[test]
    fn frame_codes_round_trip() {
        for k in [
            ObjectKind::Sticker,
            ObjectKind::Shaped,
            ObjectKind::PageFrame,
        ] {
            assert_eq!(ObjectKind::from_frame_code(k.frame_code()), Some(k));
        }
        assert_eq!(ObjectKind::Drawing.frame_code(), 3);
        assert_eq!(ObjectKind::from_frame_code(9), None);
        for w in Wrap::ALL {
            assert_eq!(Wrap::from_code(w.code()), w);
        }
        assert_eq!(Wrap::from_code(None), Wrap::Contour);
        assert_eq!(Wrap::from_code(Some(77)), Wrap::Contour);
        assert_eq!(ObjRef::pict(2).tag, *b"PICT");
        assert_eq!(ObjRef::snd(803).tag, *b"SND ");
    }

    #[test]
    fn centered_move_and_contains() {
        let mut o =
            FloatObject::centered(ObjectKind::Sticker, ObjRef::pict(1), 0, 100, 100, 40, 20);
        assert_eq!(o.rect(), (80, 90, 120, 110));
        assert!(o.contains(80, 90) && !o.contains(120, 100));
        o.move_by(1000, 1000, 595, 792);
        // um canto de MIN_SIDE continua na página
        assert_eq!((o.x, o.y), (587, 784));
        o.move_by(-5000, -5000, 595, 792);
        assert_eq!((o.x, o.y), (8 - 40, 8 - 20));
        assert!(o.on_page(0) && !o.on_page(1));
        o.wrap = Wrap::AllPages;
        assert!(o.on_page(7));
    }

    #[test]
    fn resize_modes() {
        let mut o = sticker();
        o.resize(Resize::Width, 300, 0);
        assert_eq!((o.width, o.height), (300, 150));
        o.resize(Resize::Height, 0, 2);
        assert_eq!((o.width, o.height), (300, MIN_SIDE));
        o.reset_size();
        assert_eq!((o.width, o.height), (149, 150));
        o.resize(Resize::All, 298, 10);
        assert_eq!((o.width, o.height), (298, 300));
        o.resize(Resize::All, 1, 1);
        assert_eq!(o.width.min(o.height), MIN_SIDE);
        assert_eq!((o.orig_width, o.orig_height), (149, 150));
    }

    /// Um losango 10×10: largo no meio, estreito nas pontas.
    fn diamond() -> Outline {
        Outline::from_mask(10, 10, |x, y| {
            let (x, y) = (i32::from(x), i32::from(y));
            (x - 5).abs() + (y - 5).abs() <= 4
        })
    }

    #[test]
    fn outline_from_mask_and_span() {
        let o = diamond();
        assert_eq!(o.left.first(), Some(&(5, 1)));
        assert_eq!(o.right.first(), Some(&(6, 1)));
        assert_eq!(o.left.len(), 9);
        assert_eq!(o.span(0, 1), None);
        assert_eq!(o.span(1, 2), Some((5, 6)));
        assert_eq!(o.span(5, 6), Some((1, 10)));
        assert_eq!(o.span(0, 10), Some((1, 10)));
        // interpolação entre pontos distantes
        let sparse = Outline {
            width: 10,
            height: 10,
            left: vec![(8, 0), (0, 8)],
            right: vec![(9, 0), (10, 8)],
        };
        assert_eq!(sparse.span(4, 5), Some((4, 9)));
    }

    #[test]
    fn exclusion_by_wrap_mode() {
        let mut o = FloatObject::new(ObjectKind::Sticker, ObjRef::pict(0), 0, 100, 100, 100, 100);
        o.outline = Some(diamond());
        // fora da faixa vertical
        assert_eq!(o.exclusion(0, 50, 4, 595), None);
        assert_eq!(o.exclusion(205, 220, 4, 595), None);
        // contorno: perto da ponta de cima é estreito; no meio, largo
        // a 1ª linha do losango não tem tinta: nada a evitar
        assert_eq!(o.exclusion(100, 110, 0, 595), None);
        let top = o.exclusion(100, 120, 0, 595).unwrap();
        assert_eq!(top, (150, 160));
        let mid = o.exclusion(145, 155, 0, 595).unwrap();
        assert!(top.1 - top.0 < mid.1 - mid.0, "{top:?} {mid:?}");
        assert_eq!(mid, (110, 200));
        o.wrap = Wrap::Box;
        assert_eq!(o.exclusion(100, 110, 4, 595), Some((96, 204)));
        o.wrap = Wrap::TopBottom;
        let (l, r) = o.exclusion(100, 110, 4, 595).unwrap();
        assert!(l < 0 && r >= 595);
        o.wrap = Wrap::AllPages;
        o.outline = None;
        assert_eq!(o.exclusion(100, 110, 0, 595), Some((100, 200)));
    }
}
