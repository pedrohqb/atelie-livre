//! Montagem das folhas impressas pela máquina de impressão do McZee: a figura
//! numa folha carta deitada (11 × 8,5 polegadas), no tamanho normal (72 pixels
//! por polegada, como no original), esticada por igual, esticada até ocupar a
//! folha ou em pôster de 2 × 2 folhas; com um fundo por trás (o branco da
//! figura deixa o fundo aparecer), em cores ou em branco e preto.
//!
//! As coordenadas internas são pontos (1/72 de polegada) sobre a área de
//! todas as folhas juntas. Ver `docs/telas/pintura-impressao.md`.

use std::ffi::OsString;
use std::path::PathBuf;

/// Folha carta deitada, em pontos.
pub const PAGE_W: u32 = 792;
pub const PAGE_H: u32 = 612;
/// Margem que a impressora não alcança (¼ de polegada).
pub const MARGIN: u32 = 18;
/// Pixels por ponto nos PNG impressos (144 por polegada).
pub const SCALE: u32 = 2;
/// Mais cópias que isso, o McZee não imprime (`WHAT 4c00[3]`).
pub const MAX_COPIES: u8 = 10;

const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];

/// Como esticar a figura (`WHAT 4c00[6]`, na ordem das opções).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stretch {
    /// Tamanho normal: um pixel do quadro por ponto.
    #[default]
    Normal,
    /// Esticar por igual até caber no papel.
    Fit,
    /// Esticar até ocupar todo o espaço da página (sem manter a proporção).
    Fill,
    /// Pôster: a figura esticada por igual sobre 2 × 2 folhas.
    Poster,
}

impl Stretch {
    pub const ALL: [Stretch; 4] = [
        Stretch::Normal,
        Stretch::Fit,
        Stretch::Fill,
        Stretch::Poster,
    ];

    /// Folhas na horizontal e na vertical.
    pub fn sheets(self) -> (u32, u32) {
        match self {
            Stretch::Poster => (2, 2),
            _ => (1, 1),
        }
    }
}

/// Escolhas feitas na máquina de impressão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub stretch: Stretch,
    /// Branco e preto sem tons de cinza (`WHAT 4c00[8]`, segunda opção).
    pub mono: bool,
    pub copies: u8,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            stretch: Stretch::Normal,
            mono: false,
            copies: 1,
        }
    }
}

/// Imagem RGB simples (figura composta, fundo ou folha pronta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgb8 {
    pub w: u32,
    pub h: u32,
    pub px: Vec<[u8; 3]>,
}

impl Rgb8 {
    pub fn new(w: u32, h: u32) -> Rgb8 {
        Rgb8 {
            w,
            h,
            px: vec![WHITE; (w * h) as usize],
        }
    }

    /// De RGBA de 8 bits (o alfa zero vira branco).
    pub fn from_rgba(w: u32, h: u32, rgba: &[u8]) -> Rgb8 {
        let px = rgba
            .chunks_exact(4)
            .map(|p| if p[3] == 0 { WHITE } else { [p[0], p[1], p[2]] })
            .collect();
        Rgb8 { w, h, px }
    }

    fn at(&self, x: u32, y: u32) -> [u8; 3] {
        self.px[(y * self.w + x) as usize]
    }

    /// Bytes RGB em sequência (para gravar em PNG).
    pub fn bytes(&self) -> Vec<u8> {
        self.px.iter().flatten().copied().collect()
    }
}

/// Retângulo `(x, y, w, h)` em pontos.
pub type Rect = (f32, f32, f32, f32);

/// Tamanho total (em pontos) da área de todas as folhas.
pub fn total_size(stretch: Stretch) -> (u32, u32) {
    let (c, r) = stretch.sheets();
    (c * PAGE_W, r * PAGE_H)
}

/// Área imprimível: tudo menos a margem de fora.
pub fn printable(stretch: Stretch) -> Rect {
    let (w, h) = total_size(stretch);
    let m = MARGIN as f32;
    (m, m, w as f32 - 2.0 * m, h as f32 - 2.0 * m)
}

/// Onde vai a figura de `w`×`h` pixels, em pontos, centrada.
pub fn placement(stretch: Stretch, w: u32, h: u32) -> Rect {
    let (px, py, pw, ph) = printable(stretch);
    let (w, h) = (w.max(1) as f32, h.max(1) as f32);
    let (dw, dh) = match stretch {
        Stretch::Normal => (w.min(pw), h.min(ph)),
        Stretch::Fill => (pw, ph),
        Stretch::Fit | Stretch::Poster => {
            let s = (pw / w).min(ph / h);
            (w * s, h * s)
        }
    };
    (px + (pw - dw) / 2.0, py + (ph - dh) / 2.0, dw, dh)
}

/// Branco e preto sem tons de cinza: limiar no brilho.
pub fn to_mono(c: [u8; 3]) -> [u8; 3] {
    let lum = 299 * c[0] as u32 + 587 * c[1] as u32 + 114 * c[2] as u32;
    if lum < 128_000 { BLACK } else { WHITE }
}

/// Montador da página: figura, fundo e escolhas.
pub struct Composer<'a> {
    pub picture: &'a Rgb8,
    pub background: Option<&'a Rgb8>,
    pub options: Options,
    place: Rect,
    area: Rect,
}

impl<'a> Composer<'a> {
    pub fn new(picture: &'a Rgb8, background: Option<&'a Rgb8>, options: Options) -> Self {
        Composer {
            picture,
            background,
            options,
            place: placement(options.stretch, picture.w, picture.h),
            area: printable(options.stretch),
        }
    }

    /// Cor no ponto `(x, y)` da área de todas as folhas.
    pub fn color_at(&self, x: f32, y: f32) -> [u8; 3] {
        let mut c = WHITE;
        if let Some(bg) = self.background
            && let Some(p) = sample(bg, self.area, x, y)
        {
            c = p;
        }
        if let Some(p) = sample(self.picture, self.place, x, y) {
            // com fundo, o branco da figura é transparente
            if self.background.is_none() || p != WHITE {
                c = p;
            }
        }
        if self.options.mono { to_mono(c) } else { c }
    }

    /// As folhas prontas para imprimir, `SCALE` pixels por ponto, da
    /// esquerda para a direita e de cima para baixo.
    pub fn sheets(&self) -> Vec<Rgb8> {
        let (cols, rows) = self.options.stretch.sheets();
        let (w, h) = (PAGE_W * SCALE, PAGE_H * SCALE);
        let mut out = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let mut s = Rgb8::new(w, h);
                for y in 0..h {
                    let py = (r * PAGE_H) as f32 + (y as f32 + 0.5) / SCALE as f32;
                    for x in 0..w {
                        let px = (c * PAGE_W) as f32 + (x as f32 + 0.5) / SCALE as f32;
                        s.px[(y * w + x) as usize] = self.color_at(px, py);
                    }
                }
                out.push(s);
            }
        }
        out
    }

    /// Prévia de todas as folhas reduzida a `w`×`h` pixels.
    pub fn preview(&self, w: u32, h: u32) -> Rgb8 {
        let (tw, th) = total_size(self.options.stretch);
        let mut s = Rgb8::new(w, h);
        for y in 0..h {
            let py = (y as f32 + 0.5) * th as f32 / h as f32;
            for x in 0..w {
                let px = (x as f32 + 0.5) * tw as f32 / w as f32;
                s.px[(y * w + x) as usize] = self.color_at(px, py);
            }
        }
        s
    }
}

/// Pixel da imagem esticada para `r` no ponto `(x, y)`, se dentro dela.
fn sample(img: &Rgb8, r: Rect, x: f32, y: f32) -> Option<[u8; 3]> {
    if img.w == 0 || img.h == 0 || x < r.0 || y < r.1 || x >= r.0 + r.2 || y >= r.1 + r.3 {
        return None;
    }
    let sx = (((x - r.0) / r.2) * img.w as f32) as u32;
    let sy = (((y - r.1) / r.3) * img.h as f32) as u32;
    Some(img.at(sx.min(img.w - 1), sy.min(img.h - 1)))
}

/// Número de cópias digitado na caixa: de 1 a 10, senão `None`.
pub fn parse_copies(s: &str) -> Option<u8> {
    let n: u32 = s.trim().parse().ok()?;
    (1..=MAX_COPIES as u32).contains(&n).then_some(n as u8)
}

/// Nomes dos arquivos das folhas: `impressao-<carimbo>.png`, ou com `-1`,
/// `-2`… quando há mais de uma folha (pôster).
pub fn sheet_names(stamp: &str, n: usize) -> Vec<String> {
    if n <= 1 {
        return vec![format!("impressao-{stamp}.png")];
    }
    (1..=n)
        .map(|k| format!("impressao-{stamp}-{k}.png"))
        .collect()
}

/// Argumentos do `lp`: número de cópias, folha esticada para caber no papel
/// (os PNG já têm o tamanho da folha) e os arquivos.
pub fn lp_args(copies: u8, files: &[PathBuf]) -> Vec<OsString> {
    let mut v: Vec<OsString> = vec![
        "-n".into(),
        copies.max(1).to_string().into(),
        "-o".into(),
        "fit-to-page".into(),
    ];
    v.extend(files.iter().map(|f| f.as_os_str().to_owned()));
    v
}

/// Grava a folha como PNG RGB.
pub fn save_sheet(s: &Rgb8, path: &std::path::Path) -> std::io::Result<()> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, s.w, s.h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(std::io::Error::other)?;
    w.write_image_data(&s.bytes())
        .map_err(std::io::Error::other)?;
    w.finish().map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture() -> Rgb8 {
        // 594×347 branco com um ponto vermelho no meio
        let mut p = Rgb8::new(594, 347);
        p.px[(173 * 594 + 297) as usize] = [255, 0, 0];
        p
    }

    #[test]
    fn normal_size_is_centered_at_72_dpi() {
        let r = placement(Stretch::Normal, 594, 347);
        assert_eq!(r, (99.0, 132.5, 594.0, 347.0));
        // a prévia do original: folha de 384 px, figura de 288 × 168
        let s = 384.0 / PAGE_W as f32;
        assert_eq!((r.2 * s).round(), 288.0);
        assert_eq!((r.3 * s).round(), 168.0);
    }

    #[test]
    fn fit_keeps_ratio_and_fill_takes_the_page() {
        let (_, _, pw, ph) = printable(Stretch::Fit);
        let f = placement(Stretch::Fit, 594, 347);
        assert_eq!(f.2, pw);
        assert!((f.2 / f.3 - 594.0 / 347.0).abs() < 0.01);
        assert!(f.3 < ph);
        assert_eq!(placement(Stretch::Fill, 594, 347), (18.0, 18.0, pw, ph));
        // pôster: 2 × 2 folhas, por igual
        let p = placement(Stretch::Poster, 594, 347);
        assert_eq!(total_size(Stretch::Poster), (1584, 1224));
        assert_eq!(p.2, 1584.0 - 36.0);
        assert!((p.2 / p.3 - 594.0 / 347.0).abs() < 0.01);
    }

    #[test]
    fn sheets_poster_mono_and_background() {
        let pic = picture();
        let one = Composer::new(&pic, None, Options::default()).sheets();
        assert_eq!(one.len(), 1);
        assert_eq!((one[0].w, one[0].h), (PAGE_W * SCALE, PAGE_H * SCALE));
        // o ponto vermelho aparece no meio da folha
        let mid = one[0].at(
            ((99.0 + 297.5) * SCALE as f32) as u32,
            ((132.5 + 173.5) * SCALE as f32) as u32,
        );
        assert_eq!(mid, [255, 0, 0]);

        let poster = Options {
            stretch: Stretch::Poster,
            ..Options::default()
        };
        let four = Composer::new(&pic, None, poster).sheets();
        assert_eq!(four.len(), 4);

        let mono = Options {
            mono: true,
            ..Options::default()
        };
        let c = Composer::new(&pic, None, mono);
        let p = placement(Stretch::Normal, 594, 347);
        assert_eq!(c.color_at(p.0 + 297.5, p.1 + 173.5), BLACK);
        assert_eq!(to_mono([200, 200, 200]), WHITE);

        // fundo azul: aparece em volta e no branco da figura, não no ponto
        let mut bg = Rgb8::new(4, 4);
        bg.px.fill([0, 0, 255]);
        let c = Composer::new(&pic, Some(&bg), Options::default());
        assert_eq!(c.color_at(40.0, 40.0), [0, 0, 255]);
        assert_eq!(c.color_at(p.0 + 10.0, p.1 + 10.0), [0, 0, 255]);
        assert_eq!(c.color_at(p.0 + 297.5, p.1 + 173.5), [255, 0, 0]);
        // fora da margem, branco
        assert_eq!(c.color_at(5.0, 5.0), WHITE);
        let prev = c.preview(385, 297);
        assert_eq!((prev.w, prev.h), (385, 297));
        assert_eq!(prev.at(0, 0), WHITE);
        assert_eq!(prev.at(30, 30), [0, 0, 255]);
    }

    #[test]
    fn copies_names_and_lp() {
        assert_eq!(parse_copies(" 3 "), Some(3));
        assert_eq!(parse_copies("10"), Some(10));
        assert_eq!(parse_copies("11"), None);
        assert_eq!(parse_copies("0"), None);
        assert_eq!(parse_copies("dois"), None);
        assert_eq!(
            sheet_names("20261106-120000", 1),
            ["impressao-20261106-120000.png"]
        );
        assert_eq!(sheet_names("x", 4)[3], "impressao-x-4.png");
        let args = lp_args(3, &[PathBuf::from("/a.png"), PathBuf::from("/b.png")]);
        let args: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["-n", "3", "-o", "fit-to-page", "/a.png", "/b.png"]);
    }

    #[test]
    fn rgba_and_png() {
        let i = Rgb8::from_rgba(2, 1, &[1, 2, 3, 255, 9, 9, 9, 0]);
        assert_eq!(i.px, [[1, 2, 3], WHITE]);
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("f.png");
        save_sheet(&i, &path).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
    }
}
