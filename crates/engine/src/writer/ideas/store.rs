//! Gravação das ideias do usuário no formato do port (ver
//! `docs/formats/arquivos-do-port.md`). Tudo fica numa pasta por usuário,
//! normalmente `<usuarios>/<apelido>/ideias/` ([`ideas_dir`]):
//!
//! ```text
//! maquina.txt              entradas próprias da Máquina: "1=texto" (janela 1, 2 ou 3)
//! figuras.txt, figuras/    figuras próprias do Quadro, das mais novas para as mais velhas
//! caderno/maquina.txt      Caderno, aba Máquina: uma sentença por linha, rebus = [rebus:N]
//! caderno/lembretes.txt    Caderno, aba Lembretes: texto livre
//! caderno/figuras.txt, caderno/figuras/   Caderno, aba Figuras: uma página por figura
//! ```
//!
//! As listas de figuras têm uma linha `NNN.png=Título` por figura; as
//! imagens são PNG RGBA. Os leitores toleram arquivos ausentes e linhas
//! inválidas (são ignoradas). Cada gravação passa por um `.tmp` renomeado.

use super::machine::{Piece, WINDOWS};
use crate::Image;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const MACHINE_FILE: &str = "maquina.txt";
pub const PICTURES_LIST: &str = "figuras.txt";
pub const PICTURES_DIR: &str = "figuras";
pub const NOTEBOOK_DIR: &str = "caderno";
pub const NOTES_FILE: &str = "lembretes.txt";

/// Pasta das ideias de um usuário.
pub fn ideas_dir(users_dir: &Path, apelido: &str) -> PathBuf {
    users_dir.join(apelido).join("ideias")
}

/// Uma linha só, sem caracteres de controle e sem espaços nas pontas.
fn one_line(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

// --- Máquina de Idéias ----------------------------------------------------

/// Entradas próprias de cada janela (vazias se o arquivo não existe).
pub fn load_machine_entries(dir: &Path) -> [Vec<String>; WINDOWS] {
    let mut out: [Vec<String>; WINDOWS] = Default::default();
    let Ok(text) = fs::read_to_string(dir.join(MACHINE_FILE)) else {
        return out;
    };
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let Ok(w) = k.trim().parse::<usize>() else {
            continue;
        };
        let v = one_line(v);
        if (1..=WINDOWS).contains(&w) && !v.is_empty() {
            out[w - 1].push(v);
        }
    }
    out
}

pub fn save_machine_entries(dir: &Path, entries: &[Vec<String>; WINDOWS]) -> io::Result<()> {
    let mut s = String::from("# Máquina de Idéias: entradas próprias (janela=texto)\n");
    for (w, list) in entries.iter().enumerate() {
        for e in list {
            let e = one_line(e);
            if !e.is_empty() {
                s.push_str(&format!("{}={e}\n", w + 1));
            }
        }
    }
    write_atomic(&dir.join(MACHINE_FILE), s.as_bytes())
}

// --- sentenças com rebus ----------------------------------------------------

/// Sentença numa linha, com as figuras como `[rebus:N]` (N decimal).
pub fn sentence_to_line(pieces: &[Piece]) -> String {
    pieces
        .iter()
        .map(|p| match p {
            Piece::Text(t) => one_line_keep_spaces(t),
            Piece::Rebus(c) => format!("[rebus:{c}]"),
        })
        .collect()
}

fn one_line_keep_spaces(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// `[rebus:N]` (N em 1..=255) no começo de `s`: o caractere e o resto.
fn parse_rebus(s: &str) -> Option<(u8, &str)> {
    let body = s.strip_prefix("[rebus:")?;
    let end = body.find(']')?;
    let c = body[..end].parse::<u8>().ok().filter(|&c| c > 0)?;
    Some((c, &body[end + 1..]))
}

/// Inverso de [`sentence_to_line`]; `[rebus:...]` inválido fica como texto.
pub fn line_to_sentence(line: &str) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    let mut text = String::new();
    let mut rest = line;
    while !rest.is_empty() {
        if let Some((c, after)) = parse_rebus(rest) {
            if !text.is_empty() {
                out.push(Piece::Text(std::mem::take(&mut text)));
            }
            out.push(Piece::Rebus(c));
            rest = after;
            continue;
        }
        let mut chars = rest.chars();
        text.push(chars.next().expect("resto não vazio"));
        rest = chars.as_str();
    }
    if !text.is_empty() {
        out.push(Piece::Text(text));
    }
    out
}

// --- listas de figuras --------------------------------------------------------

/// Figura com título; `file` é o PNG já gravado (`None` = nova ou mudada).
#[derive(Debug, Clone)]
pub struct TitledPicture {
    pub title: String,
    pub image: Image,
    file: Option<String>,
}

impl TitledPicture {
    pub fn new(title: &str, image: Image) -> TitledPicture {
        TitledPicture {
            title: one_line(title),
            image,
            file: None,
        }
    }

    /// Troca a imagem (figura modificada no cavalete).
    pub fn set_image(&mut self, image: Image) {
        self.image = image;
        self.file = None;
    }
}

/// Nome de PNG aceito na lista: só dígitos + `.png` (nada de caminhos).
fn valid_png_name(s: &str) -> bool {
    s.strip_suffix(".png")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Figuras de `dir/figuras.txt` + `dir/figuras/`, na ordem da lista; PNG
/// ausente ou ilegível é pulado.
pub fn load_pictures(dir: &Path) -> Vec<TitledPicture> {
    let Ok(text) = fs::read_to_string(dir.join(PICTURES_LIST)) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let (file, title) = line.split_once('=')?;
            let file = file.trim();
            if !valid_png_name(file) {
                return None;
            }
            let image = read_png(&dir.join(PICTURES_DIR).join(file))?;
            Some(TitledPicture {
                title: one_line(title),
                image,
                file: Some(file.to_string()),
            })
        })
        .collect()
}

/// Grava a lista: PNGs novos com o próximo número livre, a lista e, por
/// fim, apaga da pasta os PNGs que saíram da lista.
pub fn save_pictures(dir: &Path, pics: &mut [TitledPicture]) -> io::Result<()> {
    let folder = dir.join(PICTURES_DIR);
    fs::create_dir_all(&folder)?;
    let number = |f: &str| f.trim_end_matches(".png").parse::<u64>().ok();
    let mut next = pics
        .iter()
        .filter_map(|p| p.file.as_deref().and_then(number))
        .chain(
            fs::read_dir(&folder)?
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|f| valid_png_name(f))
                .filter_map(|f| number(&f)),
        )
        .max()
        .map_or(1, |n| n + 1);
    for p in pics.iter_mut() {
        if p.file.as_ref().is_some_and(|f| folder.join(f).is_file()) {
            continue;
        }
        let name = format!("{next:03}.png");
        next += 1;
        write_png(&folder.join(&name), &p.image)?;
        p.file = Some(name);
    }
    let mut list = String::new();
    for p in pics.iter() {
        list.push_str(&format!(
            "{}={}\n",
            p.file.as_deref().unwrap_or_default(),
            one_line(&p.title)
        ));
    }
    write_atomic(&dir.join(PICTURES_LIST), list.as_bytes())?;
    for e in fs::read_dir(&folder)?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if valid_png_name(&name) && !pics.iter().any(|p| p.file.as_deref() == Some(&name)) {
            fs::remove_file(e.path())?;
        }
    }
    Ok(())
}

/// Figuras próprias do Quadro de Figuras (as mais novas primeiro).
pub fn load_board_pictures(dir: &Path) -> Vec<TitledPicture> {
    load_pictures(dir)
}

pub fn save_board_pictures(dir: &Path, pics: &mut [TitledPicture]) -> io::Result<()> {
    save_pictures(dir, pics)
}

// --- Caderno de Idéias ----------------------------------------------------------

/// As três abas do Caderno de Idéias.
#[derive(Debug, Clone, Default)]
pub struct Notebook {
    /// Aba "Máquina de Idéias": sentenças guardadas (marcador "•" na tela).
    pub sentences: Vec<Vec<Piece>>,
    /// Aba "Lembretes": texto livre.
    pub notes: String,
    /// Aba "Figuras": uma página por figura.
    pub pictures: Vec<TitledPicture>,
}

impl Notebook {
    pub fn load(dir: &Path) -> Notebook {
        let nb = dir.join(NOTEBOOK_DIR);
        let sentences = fs::read_to_string(nb.join(MACHINE_FILE))
            .map(|t| {
                t.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(line_to_sentence)
                    .collect()
            })
            .unwrap_or_default();
        Notebook {
            sentences,
            notes: fs::read_to_string(nb.join(NOTES_FILE)).unwrap_or_default(),
            pictures: load_pictures(&nb),
        }
    }

    /// Grava as três abas (o OK do Caderno).
    pub fn save(&mut self, dir: &Path) -> io::Result<()> {
        let nb = dir.join(NOTEBOOK_DIR);
        let mut s = String::new();
        for p in &self.sentences {
            let line = sentence_to_line(p);
            if !line.trim().is_empty() {
                s.push_str(&line);
                s.push('\n');
            }
        }
        write_atomic(&nb.join(MACHINE_FILE), s.as_bytes())?;
        write_atomic(&nb.join(NOTES_FILE), self.notes.as_bytes())?;
        save_pictures(&nb, &mut self.pictures)
    }
}

// --- PNG ----------------------------------------------------------------------

/// Grava uma imagem RGBA como PNG (via `.tmp`).
pub fn write_png(path: &Path, img: &Image) -> io::Result<()> {
    let mut buf = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut buf, img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(io::Error::other)?;
        w.write_image_data(&img.rgba).map_err(io::Error::other)?;
    }
    write_atomic(path, &buf)
}

/// Lê um PNG como RGBA (cinza, paleta e RGB são convertidos).
pub fn read_png(path: &Path) -> Option<Image> {
    let file = fs::File::open(path).ok()?;
    let mut dec = png::Decoder::new(io::BufReader::new(file));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => px
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some(Image {
        width: info.width,
        height: info.height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pic(w: u32, h: u32, v: u8) -> Image {
        Image {
            width: w,
            height: h,
            rgba: [v, 255 - v, 7, 255].repeat((w * h) as usize),
        }
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn ideas_dir_is_per_user() {
        assert_eq!(
            ideas_dir(Path::new("/u"), "ana"),
            PathBuf::from("/u/ana/ideias")
        );
    }

    #[test]
    fn machine_entries_round_trip_and_tolerance() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(
            load_machine_entries(d.path()),
            <[Vec<String>; 3]>::default()
        );
        let e = [
            vec!["O pirata Zeca,".to_string(), "A = B,".to_string()],
            vec![],
            vec!["na lua.\n".to_string(), "  ".to_string()],
        ];
        save_machine_entries(d.path(), &e).unwrap();
        let text = fs::read_to_string(d.path().join(MACHINE_FILE)).unwrap();
        assert!(
            text.contains("1=O pirata Zeca,\n1=A = B,\n3=na lua.\n"),
            "{text}"
        );
        let back = load_machine_entries(d.path());
        assert_eq!(back[0], ["O pirata Zeca,", "A = B,"]);
        assert!(back[1].is_empty());
        assert_eq!(back[2], ["na lua."]);
        fs::write(d.path().join(MACHINE_FILE), "lixo\n4=x\n0=y\n2=dan\nz=w\n").unwrap();
        assert_eq!(load_machine_entries(d.path())[1], ["dan"]);
    }

    #[test]
    fn sentence_lines_round_trip() {
        let s = vec![
            Piece::Rebus(37),
            Piece::Text(" riu com desdém ".into()),
            Piece::Rebus(26),
        ];
        let line = sentence_to_line(&s);
        assert_eq!(line, "[rebus:37] riu com desdém [rebus:26]");
        assert_eq!(line_to_sentence(&line), s);
        assert_eq!(
            line_to_sentence("a [rebus:0] [rebus:300] [rebus:x"),
            [Piece::Text("a [rebus:0] [rebus:300] [rebus:x".into())]
        );
        assert_eq!(sentence_to_line(&[Piece::Text("a\nb".into())]), "a b");
    }

    #[test]
    fn pictures_round_trip_keep_files_and_prune() {
        let d = tempfile::tempdir().unwrap();
        let mut pics = vec![
            TitledPicture::new("Meu gato", pic(3, 2, 10)),
            TitledPicture::new("Casa\n=nova", pic(2, 2, 200)),
        ];
        save_board_pictures(d.path(), &mut pics).unwrap();
        assert_eq!(names(&d.path().join(PICTURES_DIR)), ["001.png", "002.png"]);
        let mut back = load_board_pictures(d.path());
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].title, "Meu gato");
        assert_eq!(back[1].title, "Casa =nova");
        assert_eq!(back[0].image.rgba, pics[0].image.rgba);
        assert_eq!((back[1].image.width, back[1].image.height), (2, 2));

        // nova na frente, a primeira modificada, a segunda jogada fora
        back.remove(1);
        back[0].set_image(pic(1, 1, 50));
        back.insert(0, TitledPicture::new("Nova", pic(4, 1, 99)));
        save_board_pictures(d.path(), &mut back).unwrap();
        assert_eq!(names(&d.path().join(PICTURES_DIR)), ["003.png", "004.png"]);
        let again = load_board_pictures(d.path());
        let titles: Vec<&str> = again.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(titles, ["Nova", "Meu gato"]);
        assert_eq!(again[1].image.width, 1);
    }

    #[test]
    fn picture_list_ignores_bad_lines_and_paths() {
        let d = tempfile::tempdir().unwrap();
        let mut pics = vec![TitledPicture::new("ok", pic(1, 1, 1))];
        save_pictures(d.path(), &mut pics).unwrap();
        fs::write(d.path().join("segredo.png"), b"x").unwrap();
        fs::write(
            d.path().join(PICTURES_LIST),
            "001.png=ok\n../segredo.png=mal\n009.png=sumiu\nsem igual\n",
        )
        .unwrap();
        let back = load_pictures(d.path());
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].title, "ok");
    }

    #[test]
    fn notebook_round_trip() {
        let d = tempfile::tempdir().unwrap();
        let empty = Notebook::load(d.path());
        assert!(empty.sentences.is_empty() && empty.notes.is_empty() && empty.pictures.is_empty());
        let mut nb = Notebook {
            sentences: vec![
                vec![Piece::Text(
                    "O pirata Zeca, foi caminhar sobre uma dobra do tempo.".into(),
                )],
                vec![Piece::Rebus(0x25), Piece::Text(" dormiu".into())],
            ],
            notes: "Lembrar:\ncomprar tinta.\n".into(),
            pictures: vec![TitledPicture::new("Cara de cão", pic(5, 4, 3))],
        };
        nb.save(d.path()).unwrap();
        let back = Notebook::load(d.path());
        assert_eq!(back.sentences, nb.sentences);
        assert_eq!(back.notes, nb.notes);
        assert_eq!(back.pictures.len(), 1);
        assert_eq!(back.pictures[0].title, "Cara de cão");
        assert!(d.path().join("caderno/figuras/001.png").is_file());
    }

    #[test]
    fn png_read_handles_rgb() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("rgb.png");
        {
            let f = fs::File::create(&p).unwrap();
            let mut enc = png::Encoder::new(f, 2, 1);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header()
                .unwrap()
                .write_image_data(&[1, 2, 3, 4, 5, 6])
                .unwrap();
        }
        let img = read_png(&p).unwrap();
        assert_eq!(img.rgba, [1, 2, 3, 255, 4, 5, 6, 255]);
        assert!(read_png(&d.path().join("nada.png")).is_none());
    }
}
