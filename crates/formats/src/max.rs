//! Documentos do Creative Writer (`.MAX`; ver `docs/formats/max.md`).
//!
//! Um `.MAX` é um arquivo `CHNK` little-endian de criador `INK ` com um
//! modelo de processador de texto da família do Word: **texto corrido**
//! (`TEXT`, Windows-1252, `\r` fecha parágrafo), **trechos de caractere e de
//! parágrafo** em páginas FKP (`FDPC`/`FDPP`, indexadas por `BTEC`/`BTEP`),
//! tabela de fontes (`FONT`), folhas (`PGD `), propriedades do documento
//! (`DOP `), **molduras** de objetos soltos (`FRAM`), sons em palavras
//! (`SNDT`) e o descritor `ADD ` (título, autor, projeto, andar, senha).
//!
//! [`read`] devolve um [`Document`] com tudo isso tipado; [`write`] grava
//! um documento que o original abre, na ordem de chunks e com o alinhamento
//! do original. `read(&write(&d)) == d` para todo documento consistente
//! (trechos cobrindo o texto sem buracos, ao menos uma folha), e os
//! documentos lidos dos originais são regravados byte a byte iguais, salvo
//! o enchimento.
//!
//! # Posições
//!
//! Todas as posições de texto (`cp`) contam **caracteres** de
//! [`Document::text`], do 0, e cada caractere corresponde a um byte do
//! arquivo: o texto é decodificado do Windows-1252 sem perdas (os cinco
//! bytes sem caractere, 0x81, 0x8d, 0x8f, 0x90 e 0x9d, viram U+0081 etc.).
//! Na gravação, caracteres fora do Windows-1252 viram `?`.
//!
//! # Objetos
//!
//! Figuras (`PICT`), Splots (`SPLE`), bordas (`BDR `) e sons (`SND `) são
//! cópias dos chunks das bibliotecas e ficam em [`Document::objects`] como
//! vieram; as molduras ([`Frame`]), o `DOP` ([`DopProps::border`]) e os sons
//! em palavras ([`WordSound`]) os citam por [`ObjRef`].

mod fkp;
mod font;
mod plc;
mod sprm;
mod write;

pub use font::Font;
pub use sprm::{
    Align, CharProps, DopProps, Effect, Frame, FrameKind, Indent, MacColor, ParaProps, Sprm,
};
pub use write::write;

use crate::chnk::{self, OwnedChunk};
use crate::{ErrorKind, Format, FormatError, Result, Rgb, Tag, cp1252, pkbm};

/// Criador dos documentos do Creative Writer.
pub const CREATOR: Tag = Tag(*b"INK ");
/// Offset do texto nos arquivos gravados (os fcs das páginas FKP contam daí).
pub const TEXT_FC: u32 = 0x200;
/// Os 7 tamanhos do Writer, em pontos: Gigante, Enorme, Grande, Média,
/// Pequena, Mínima (padrão, sem sprm) e Micro.
pub const SIZES_PT: [u16; 7] = [48, 36, 24, 18, 14, 12, 9];
/// Tamanho do texto sem o sprm `0x0f`.
pub const DEFAULT_SIZE_PT: u16 = 12;
/// Fonte de `ftc` 0 e de qualquer `ftc` ausente da tabela.
pub const DEFAULT_FONT: &str = "Lucida Sans";
/// Andar do Escritório e da Oficina de Projetos (`FLOR`, `ADD[10]`).
pub const FLOOR_OFFICE: u16 = 2;
pub const FLOOR_PROJECTS: u16 = 3;
/// Lado maior da miniatura (`THMB`): a página reduzida para caber em
/// 144 × 144 (ver [`thumb_size`]).
pub const THUMB_MAX: u16 = 144;
/// As 16 cores do Seleciona-cores e do Iluminador (recurso `CSCH`), na ordem
/// do original: branco, amarelo, laranja, marrom, rosa, vermelho, lavanda,
/// roxo, magenta, violeta, verde, verde-escuro, cinza-claro, cinza,
/// cinza-escuro e preto.
pub const TEXT_COLORS: [Rgb; 16] = [
    rgb(0xffffff),
    rgb(0xffee4c),
    rgb(0xff8c32),
    rgb(0xb96524),
    rgb(0xfa4a72),
    rgb(0xfa0039),
    rgb(0x8888e6),
    rgb(0x6141db),
    rgb(0xde38c7),
    rgb(0x932ca3),
    rgb(0x5ccb53),
    rgb(0x478c50),
    rgb(0xbebebe),
    rgb(0x8c8c8c),
    rgb(0x646464),
    rgb(0x000000),
];
/// Paleta das miniaturas de 4 bits gravadas pelo original (`TESTE1.MAX`).
pub const THUMB_PALETTE: [Rgb; 16] = [
    rgb(0xffffff),
    rgb(0x000000),
    rgb(0xffee4c),
    rgb(0xff8c32),
    rgb(0xb96524),
    rgb(0xfa4a72),
    rgb(0xfa0039),
    rgb(0x8888e6),
    rgb(0x6141db),
    rgb(0xd016de),
    rgb(0x932ca3),
    rgb(0x5ccb53),
    rgb(0x478c50),
    rgb(0xbebebe),
    rgb(0x8c8c8c),
    rgb(0x646464),
];

const fn rgb(c: u32) -> Rgb {
    Rgb {
        r: (c >> 16) as u8,
        g: (c >> 8) as u8,
        b: c as u8,
    }
}

pub(crate) fn err(offset: usize, kind: ErrorKind) -> FormatError {
    FormatError {
        format: Format::Max,
        offset,
        kind,
    }
}

/// Referência a um chunk de objeto: etiqueta e número (`PICT 2`, `SND 803`,
/// `BDR 0`, `SPLE 0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjRef {
    pub tag: Tag,
    pub id: u16,
}

impl ObjRef {
    pub const fn new(tag: &[u8; 4], id: u16) -> ObjRef {
        ObjRef { tag: Tag(*tag), id }
    }
}

/// Trecho `[start, end)` do texto (em cps) com as suas propriedades.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Run<P> {
    pub start: u32,
    pub end: u32,
    pub props: P,
}

/// Uma folha (tabela `PGD `): o trecho de texto que começa nela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Page {
    pub start: u32,
    pub end: u32,
    /// 0 nos documentos livres; nos cartões 1, 4, 0, 0 etc. (capa, páginas
    /// internas, contracapa; a mapear).
    pub flags: u16,
}

/// Som ligado a um trecho do texto (tabela `SNDT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WordSound {
    pub start: u32,
    pub end: u32,
    /// O som (`SND n`, cópia em [`Document::objects`]).
    pub sound: ObjRef,
    /// 4 bytes iniciais do elemento, sem uso aparente (lixo de memória num
    /// exemplo); regravados como vieram.
    pub reserved: [u8; 4],
}

/// Tipo de projeto da Oficina de Projetos (`WIZA`, `ADD[0..4]`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Project {
    /// Documento livre (zeros).
    #[default]
    None,
    /// `card`: cartão.
    Card,
    /// `banr`: faixa.
    Banner,
    /// `news`: jornal.
    Newspaper,
    /// Outro valor.
    Other([u8; 4]),
}

impl Project {
    pub fn from_bytes(b: [u8; 4]) -> Project {
        match &b {
            [0, 0, 0, 0] => Project::None,
            b"card" => Project::Card,
            b"banr" => Project::Banner,
            b"news" => Project::Newspaper,
            _ => Project::Other(b),
        }
    }

    pub fn to_bytes(self) -> [u8; 4] {
        match self {
            Project::None => [0; 4],
            Project::Card => *b"card",
            Project::Banner => *b"banr",
            Project::Newspaper => *b"news",
            Project::Other(b) => b,
        }
    }
}

const ADD_LEN: usize = 128;
const ADD_PASSWORD: usize = 0x1c;
const PASSWORD_LEN: usize = 32;
const ADD_TITLE: usize = 0x3c;
const ADD_AUTHOR: usize = 0x5e;
/// Maior título/autor que cabe no `ADD ` (Pascal em 34 bytes).
pub const ADD_TEXT_MAX: usize = 33;
/// Deslocamento das letras da senha no `ADD `.
const PASSWORD_SHIFT: u8 = 5;

/// Descritor do documento (chunk `ADD `, 128 bytes, também usado pela
/// biblioteca) junto com os chunks que o repetem (`TITL`, `AUTH`, `FLOR`,
/// `WIZA`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Add {
    /// `WIZA` / `ADD[0..4]`.
    pub project: Project,
    /// `ADD[5] = 1`: documento "embaralhado" pelo Agente secreto. O texto
    /// não é cifrado; só a exibição é embaralhada até digitar a senha.
    pub encrypted: bool,
    /// `FLOR` / `ADD[10]`: andar em que foi feito ([`FLOOR_OFFICE`] ou
    /// [`FLOOR_PROJECTS`]).
    pub floor: u16,
    /// `ADD[11]`: 1 a 3 (provável modo de visualização).
    pub view: u8,
    /// `ADD[12]` (`i16`): −2 nos documentos novos; −4, 0x19, 0x21… nos
    /// exemplos (a mapear).
    pub field_12: i16,
    /// Senha do Agente secreto, já decodificada (no arquivo cada letra vai
    /// com +5, completada com espaços até 32 bytes); vazia = sem senha.
    pub password: String,
    /// `TITL` (nome dado ao salvar; o arquivo tem o mesmo nome em 8.3). No
    /// `ADD ` vão só os primeiros [`ADD_TEXT_MAX`] bytes.
    pub title: String,
    /// `AUTH`: nome completo do usuário.
    pub author: String,
    /// Os 128 bytes do `ADD ` com os campos acima zerados: o que o port não
    /// interpreta (inclusive restos de títulos antigos depois do Pascal).
    /// O gravador parte deles e escreve os campos por cima.
    pub reserved: [u8; ADD_LEN],
}

impl Default for Add {
    /// Descritor de um documento novo do Escritório, como em `TESTE1.MAX`.
    fn default() -> Add {
        let mut reserved = [0u8; ADD_LEN];
        reserved[14] = 1;
        Add {
            project: Project::None,
            encrypted: false,
            floor: FLOOR_OFFICE,
            view: 1,
            field_12: -2,
            password: String::new(),
            title: String::new(),
            author: String::new(),
            reserved,
        }
    }
}

impl Add {
    /// Lê os 128 bytes do `ADD ` (completados com zeros se faltarem).
    pub fn from_bytes(data: &[u8]) -> Add {
        let mut b = [0u8; ADD_LEN];
        let n = data.len().min(ADD_LEN);
        b[..n].copy_from_slice(&data[..n]);
        let mut reserved = b;
        let mut take = |r: std::ops::Range<usize>| {
            reserved[r].fill(0);
        };
        take(0..4);
        take(5..6);
        take(10..14);
        take(ADD_PASSWORD..ADD_PASSWORD + PASSWORD_LEN);
        let pascal = |at: usize| {
            let len = (b[at] as usize).min(ADD_TEXT_MAX);
            (decode_text(&b[at + 1..at + 1 + len]), len)
        };
        let (title, tl) = pascal(ADD_TITLE);
        let (author, al) = pascal(ADD_AUTHOR);
        take(ADD_TITLE..ADD_TITLE + 1 + tl);
        take(ADD_AUTHOR..ADD_AUTHOR + 1 + al);
        let pw = &b[ADD_PASSWORD..ADD_PASSWORD + PASSWORD_LEN];
        let end = pw
            .iter()
            .rposition(|&c| c != b' ' && c != 0)
            .map_or(0, |i| i + 1);
        let password = decode_text(
            &pw[..end]
                .iter()
                .map(|c| c.wrapping_sub(PASSWORD_SHIFT))
                .collect::<Vec<u8>>(),
        );
        Add {
            project: Project::from_bytes([b[0], b[1], b[2], b[3]]),
            encrypted: b[5] != 0,
            floor: b[10] as u16,
            view: b[11],
            field_12: i16::from_le_bytes([b[12], b[13]]),
            password,
            title,
            author,
            reserved,
        }
    }

    /// Os 128 bytes do `ADD `: [`Add::reserved`] com os campos por cima.
    pub fn to_bytes(&self) -> [u8; ADD_LEN] {
        let mut b = self.reserved;
        b[0..4].copy_from_slice(&self.project.to_bytes());
        b[5] = self.encrypted as u8;
        b[10] = self.floor as u8;
        b[11] = self.view;
        b[12..14].copy_from_slice(&self.field_12.to_le_bytes());
        if !self.password.is_empty() {
            let mut pw = [b' '; PASSWORD_LEN];
            for (d, c) in pw.iter_mut().zip(encode_text(&self.password)) {
                *d = c.wrapping_add(PASSWORD_SHIFT);
            }
            b[ADD_PASSWORD..ADD_PASSWORD + PASSWORD_LEN].copy_from_slice(&pw);
        }
        for (at, s) in [(ADD_TITLE, &self.title), (ADD_AUTHOR, &self.author)] {
            let t = encode_text(s);
            let t = &t[..t.len().min(ADD_TEXT_MAX)];
            b[at] = t.len() as u8;
            b[at + 1..at + 1 + t.len()].copy_from_slice(t);
        }
        b
    }
}

/// Um documento do Creative Writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Texto corrido, com `\r` fechando cada parágrafo (não há `\n`). Os
    /// documentos do original terminam com `\r` (o do Wine com um `\r` a
    /// mais: "abc" vira `"abc\r\r"`). Caracteres com
    /// [`CharProps::anchor`] são âncoras de objetos na linha, não letras.
    pub text: String,
    /// Trechos de caractere, em ordem, cobrindo o texto sem buracos.
    pub char_runs: Vec<Run<CharProps>>,
    /// Trechos de parágrafo: um por parágrafo, terminando logo após o `\r`.
    pub para_runs: Vec<Run<ParaProps>>,
    /// Tabela de fontes (`FONT`); `ftc` 0 é sempre Lucida Sans.
    pub fonts: Vec<Font>,
    /// Folhas (`PGD `), em ordem.
    pub pages: Vec<Page>,
    /// Propriedades do documento (`DOP `).
    pub dop: DopProps,
    /// Objetos soltos (`FRAM`), na ordem de desenho.
    pub frames: Vec<Frame>,
    /// Sons em palavras (`SNDT`).
    pub word_sounds: Vec<WordSound>,
    /// Chunks dos objetos (`PICT`, `SPLE`, `BDR `, `SND ` e quaisquer outros
    /// que o leitor não interpreta), na ordem do arquivo.
    pub objects: Vec<OwnedChunk>,
    /// Seleção ao salvar (`SELN`): cps de início e fim. `None` = sem `SELN`.
    pub selection: Option<(u32, u32)>,
    /// Configuração da impressora (`PRNT`/`WNPR`), opaca. `None` = sem `PRNT`.
    pub printer: Option<Vec<u8>>,
    /// Miniatura (`THMB`) como `PKBM` cru; ver [`Document::thumbnail_bitmap`]
    /// e [`Document::set_thumbnail`]. `None` = sem `THMB`.
    pub thumbnail: Option<Vec<u8>>,
    /// Título, autor, projeto, andar, senha.
    pub add: Add,
}

/// `PRNT` gravado pelo original sem impressora (72 dpi, folha de 595 × 792).
pub const DEFAULT_PRINTER: [u8; 22] = [
    b'p', b'r', b'r', b'1', 0x48, 0x00, 0x48, 0x00, 0x53, 0x02, 0x18, 0x03, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0,
];

/// Tamanho da miniatura de uma página de `w` × `h` pontos, como o original:
/// o lado maior vale 144 e o outro é proporcional, arredondado para baixo
/// (595 × 792 → 108 × 144; 792 × 595 → 144 × 108; faixa 542 × 767 → 101 × 144).
pub fn thumb_size(w: u16, h: u16) -> (u16, u16) {
    let (w, h) = (w.max(1) as u32, h.max(1) as u32);
    let m = THUMB_MAX as u32;
    if w >= h {
        (m as u16, (h * m / w).max(1) as u16)
    } else {
        ((w * m / h).max(1) as u16, m as u16)
    }
}

/// Miniatura em branco para uma página de `w` × `h` pontos, 4 bits, paleta
/// [`THUMB_PALETTE`].
pub fn blank_thumbnail(w: u16, h: u16) -> pkbm::Bitmap {
    let (tw, th) = thumb_size(w, h);
    pkbm::Bitmap {
        width: tw,
        height: th,
        depth: 4,
        pixels: vec![0; tw as usize * th as usize],
        palette: Some(THUMB_PALETTE.to_vec()),
    }
}

impl Default for Document {
    fn default() -> Document {
        Document::new()
    }
}

impl Document {
    /// Documento novo e vazio do Escritório.
    pub fn new() -> Document {
        Document::from_text("")
    }

    /// Documento novo com o texto dado, sem formatação: as quebras de linha
    /// (`\r\n`, `\n`, `\r`) viram `\r` e o texto ganha o `\r` do último
    /// parágrafo e o `\r` final, como o original ("abc" → `"abc\r\r"`). Um
    /// trecho de caractere, um de parágrafo por parágrafo, uma folha, cursor
    /// no fim do texto digitado, a fonte padrão, o `DOP` padrão, a
    /// impressora padrão, miniatura em branco e o [`Add`] padrão.
    pub fn from_text(s: &str) -> Document {
        let mut text = s.replace("\r\n", "\r").replace('\n', "\r");
        text.push_str("\r\r");
        let len = text.chars().count() as u32;
        let mut doc = Document {
            text,
            char_runs: vec![Run {
                start: 0,
                end: len,
                props: CharProps::default(),
            }],
            para_runs: Vec::new(),
            fonts: vec![Font::new(DEFAULT_FONT, 0, 2)],
            pages: vec![Page {
                start: 0,
                end: len,
                flags: 0,
            }],
            dop: DopProps::default(),
            frames: Vec::new(),
            word_sounds: Vec::new(),
            objects: Vec::new(),
            selection: Some((len - 2, len - 2)),
            printer: Some(DEFAULT_PRINTER.to_vec()),
            thumbnail: None,
            add: Add::default(),
        };
        doc.para_runs = doc
            .paragraphs()
            .into_iter()
            .map(|(start, end)| Run {
                start,
                end,
                props: ParaProps::default(),
            })
            .collect();
        doc.set_thumbnail(&blank_thumbnail(doc.dop.page_width, doc.dop.page_height));
        doc
    }

    /// Número de cps do texto.
    pub fn len(&self) -> u32 {
        self.text.chars().count() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Trecho `[start, end)` do texto, em cps.
    pub fn text_range(&self, start: u32, end: u32) -> String {
        self.text
            .chars()
            .skip(start as usize)
            .take(end.saturating_sub(start) as usize)
            .collect()
    }

    /// Parágrafos `[início, fim)` em cps, cada um com o seu `\r`; um resto
    /// sem `\r` no fim conta como parágrafo.
    pub fn paragraphs(&self) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        let mut start = 0;
        let mut n = 0;
        for c in self.text.chars() {
            n += 1;
            if c == '\r' {
                out.push((start, n));
                start = n;
            }
        }
        if start < n {
            out.push((start, n));
        }
        out
    }

    /// Propriedades de caractere em `cp` (padrão fora dos trechos).
    pub fn char_props_at(&self, cp: u32) -> &CharProps {
        static DEFAULT: std::sync::OnceLock<CharProps> = std::sync::OnceLock::new();
        find_run(&self.char_runs, cp).unwrap_or_else(|| DEFAULT.get_or_init(CharProps::default))
    }

    /// Propriedades do parágrafo que contém `cp` (padrão fora dos trechos).
    pub fn para_props_at(&self, cp: u32) -> &ParaProps {
        static DEFAULT: std::sync::OnceLock<ParaProps> = std::sync::OnceLock::new();
        find_run(&self.para_runs, cp).unwrap_or_else(|| DEFAULT.get_or_init(ParaProps::default))
    }

    /// Texto para exibir ou exportar: parágrafos separados por `\n`, sem as
    /// âncoras de objetos e sem o parágrafo final vazio.
    pub fn plain_text(&self) -> String {
        let mut paras: Vec<String> = Vec::new();
        let mut cur = String::new();
        for (i, c) in self.text.chars().enumerate() {
            if c == '\r' {
                paras.push(std::mem::take(&mut cur));
            } else if !self.char_props_at(i as u32).anchor {
                cur.push(c);
            }
        }
        if !cur.is_empty() || paras.is_empty() {
            paras.push(cur);
        } else if paras.last().is_some_and(String::is_empty) {
            paras.pop();
        }
        paras.join("\n")
    }

    /// Nome da fonte de `ftc`; [`DEFAULT_FONT`] se não estiver na tabela.
    pub fn font_name(&self, ftc: u16) -> &str {
        self.fonts
            .iter()
            .find(|f| f.ftc == ftc)
            .map_or(DEFAULT_FONT, |f| f.name.as_str())
    }

    /// O chunk do objeto citado.
    pub fn object(&self, r: ObjRef) -> Option<&OwnedChunk> {
        self.objects.iter().find(|c| c.tag == r.tag && c.id == r.id)
    }

    /// Todas as referências a objetos (molduras, borda do `DOP`, sons em
    /// palavras), em ordem e sem repetição.
    pub fn object_refs(&self) -> Vec<ObjRef> {
        let mut out: Vec<ObjRef> = Vec::new();
        let all = self
            .frames
            .iter()
            .flat_map(|f| [f.object, f.sound])
            .chain([self.dop.border, self.dop.border_alt])
            .flatten()
            .chain(self.word_sounds.iter().map(|w| w.sound));
        for r in all {
            if !out.contains(&r) {
                out.push(r);
            }
        }
        out
    }

    /// A miniatura decodificada, se houver e for válida.
    pub fn thumbnail_bitmap(&self) -> Option<pkbm::Bitmap> {
        self.thumbnail.as_deref().and_then(|t| pkbm::decode(t).ok())
    }

    /// Troca a miniatura: a página inteira reduzida a [`thumb_size`], com 4
    /// ou 8 bits e paleta embutida. É gravada com o PackBits por linha, como
    /// o original (a prévia da pasta do Writer exige).
    pub fn set_thumbnail(&mut self, b: &pkbm::Bitmap) {
        self.thumbnail = Some(pkbm::encode_rows(b, (0, 0)));
    }
}

fn find_run<P>(runs: &[Run<P>], cp: u32) -> Option<&P> {
    let i = runs.partition_point(|r| r.end <= cp);
    runs.get(i)
        .filter(|r| r.start <= cp && cp < r.end)
        .map(|r| &r.props)
}

/// Decodifica Windows-1252 sem perdas (os 5 bytes indefinidos viram os
/// controles C1 de mesmo número).
fn decode_text(b: &[u8]) -> String {
    b.iter()
        .map(|&c| match c {
            0x81 | 0x8d | 0x8f | 0x90 | 0x9d => char::from(c),
            _ => cp1252::decode(&[c]).chars().next().unwrap_or('?'),
        })
        .collect()
}

/// Codifica em Windows-1252; o que não existir vira `?`.
fn encode_text(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c as u32 {
            v @ (0x81 | 0x8d | 0x8f | 0x90 | 0x9d) => v as u8,
            _ => cp1252::encode_char(c).unwrap_or(b'?'),
        })
        .collect()
}

/// Chunks interpretados pelo leitor; os outros vão para `objects`.
const KNOWN: [&[u8; 4]; 19] = [
    b"TEXT", b"FDPP", b"FDPC", b"DOP ", b"FRAM", b"BTEP", b"BTEC", b"PGD ", b"FONT", b"SNDT",
    b"PRNT", b"SELN", b"THMB", b"ADD ", b"AUTH", b"TITL", b"FLOR", b"WIZA", b"INK ",
];

/// Lê um documento `.MAX`.
///
/// Exige o criador `INK ` e o chunk `TEXT`; os demais são opcionais (sem
/// `BTEC`/`BTEP` não há trechos; sem `DOP ` vale [`DopProps::default`]; sem
/// `ADD ` vale [`Add::default`]). `TITL`, `AUTH`, `FLOR` e `WIZA`, quando
/// existem, prevalecem sobre os campos repetidos no `ADD `.
pub fn read(data: &[u8]) -> Result<Document> {
    let f = chnk::parse(data).map_err(|e| err(e.offset, e.kind))?;
    if f.creator != CREATOR {
        return Err(err(4, ErrorKind::BadMagic { expected: "INK " }));
    }
    let part = |tag: &[u8; 4]| {
        f.chunks
            .iter()
            .find(|c| c.tag.0 == *tag && c.id == 0)
            .and_then(|c| c.parts.first())
    };
    let text_part = part(b"TEXT").ok_or_else(|| err(0, ErrorKind::Invalid("no TEXT".into())))?;
    let text = decode_text(text_part.data);
    let len = text_part.data.len() as u32;
    let text_fc = text_part.offset as u32;

    let char_runs = runs(&f, b"BTEC", b"FDPC", text_fc, len, CharProps::from_grpprl)?;
    let para_runs = runs(&f, b"BTEP", b"FDPP", text_fc, len, ParaProps::from_grpprl)?;

    let fonts = match part(b"FONT") {
        Some(p) => font::read(p.data, p.offset)?,
        None => Vec::new(),
    };
    let pages = match part(b"PGD ") {
        Some(p) => {
            let t = plc::read(p.data, 2, p.offset)?;
            t.elems
                .iter()
                .enumerate()
                .map(|(k, e)| Page {
                    start: t.pos[k],
                    end: t.pos[k + 1],
                    flags: u16::from_le_bytes([e[0], e[1]]),
                })
                .collect()
        }
        None => Vec::new(),
    };
    let dop = match part(b"DOP ") {
        Some(p) => {
            let n = p.data.first().map_or(0, |&n| n as usize);
            let g = p.data.get(1..1 + n).ok_or_else(|| {
                err(
                    p.offset,
                    ErrorKind::Invalid("DOP shorter than its size".into()),
                )
            })?;
            DopProps::from_sprms(&sprm::parse_grpprl(g))
        }
        None => DopProps::default(),
    };
    let frames = match part(b"FRAM") {
        Some(p) => read_frames(p.data, p.offset)?,
        None => Vec::new(),
    };
    let word_sounds = match part(b"SNDT") {
        Some(p) => {
            let t = plc::read(p.data, 10, p.offset)?;
            t.elems
                .iter()
                .enumerate()
                .map(|(k, e)| WordSound {
                    start: t.pos[k],
                    end: t.pos[k + 1],
                    sound: ObjRef::from_bytes(&e[4..10]),
                    reserved: [e[0], e[1], e[2], e[3]],
                })
                .collect()
        }
        None => Vec::new(),
    };
    let objects = f
        .chunks
        .iter()
        .filter(|c| !KNOWN.contains(&&c.tag.0))
        .map(OwnedChunk::from)
        .collect();
    let selection = match part(b"SELN") {
        Some(p) if p.data.len() >= 12 => {
            let u = |at: usize| u32::from_le_bytes(p.data[at..at + 4].try_into().unwrap());
            Some((u(4), u(8)))
        }
        Some(p) => return Err(err(p.offset, ErrorKind::Invalid("SELN too short".into()))),
        None => None,
    };

    let mut add = part(b"ADD ").map_or_else(Add::default, |p| Add::from_bytes(p.data));
    if let Some(p) = part(b"TITL") {
        add.title = decode_text(p.data);
    }
    if let Some(p) = part(b"AUTH") {
        add.author = decode_text(p.data);
    }
    if let Some(p) = part(b"FLOR").filter(|p| p.data.len() >= 2) {
        add.floor = u16::from_le_bytes([p.data[0], p.data[1]]);
    }
    if let Some(p) = part(b"WIZA").filter(|p| p.data.len() >= 4) {
        add.project = Project::from_bytes([p.data[0], p.data[1], p.data[2], p.data[3]]);
    }

    Ok(Document {
        text,
        char_runs,
        para_runs,
        fonts,
        pages,
        dop,
        frames,
        word_sounds,
        objects,
        selection,
        printer: part(b"PRNT").map(|p| p.data.to_vec()),
        thumbnail: part(b"THMB").map(|p| p.data.to_vec()),
        add,
    })
}

pub(crate) fn read_frames(b: &[u8], base: usize) -> Result<Vec<Frame>> {
    let bad = |at: usize| {
        err(
            base + at,
            ErrorKind::Invalid("FRAM entry beyond part".into()),
        )
    };
    if b.len() < 2 {
        return Err(bad(0));
    }
    let n = u16::from_le_bytes([b[0], b[1]]) as usize;
    let mut at = 2;
    let mut frames = Vec::with_capacity(n.min(1024));
    for _ in 0..n {
        let len = *b.get(at).ok_or_else(|| bad(at))? as usize;
        let g = b.get(at + 1..at + 1 + len).ok_or_else(|| bad(at))?;
        frames.push(Frame::from_sprms(&sprm::parse_grpprl(g)));
        at += 1 + len;
    }
    Ok(frames)
}

/// Trechos de uma tabela BTE e das suas páginas FKP, em cps. Sem a tabela,
/// as páginas são lidas na ordem dos números dos chunks.
fn runs<P>(
    f: &chnk::ChunkFile<'_>,
    bte: &[u8; 4],
    fkp_tag: &[u8; 4],
    text_fc: u32,
    len: u32,
    props: impl Fn(&[u8]) -> P,
) -> Result<Vec<Run<P>>> {
    let mut pages: Vec<(u16, &chnk::Part<'_>)> = f
        .chunks
        .iter()
        .filter(|c| c.tag.0 == *fkp_tag)
        .filter_map(|c| c.parts.first().map(|p| (c.id, p)))
        .collect();
    let order: Vec<&chnk::Part<'_>> = match f
        .chunks
        .iter()
        .find(|c| c.tag.0 == *bte && c.id == 0)
        .and_then(|c| c.parts.first())
    {
        Some(p) => {
            let t = plc::read(p.data, 4, p.offset)?;
            t.elems
                .iter()
                .map(|e| {
                    let fc = u32::from_le_bytes([e[0], e[1], e[2], e[3]]) as usize;
                    pages
                        .iter()
                        .find(|(_, pp)| pp.offset == fc)
                        .map(|(_, pp)| *pp)
                        .ok_or_else(|| {
                            err(
                                p.offset,
                                ErrorKind::Invalid(format!(
                                    "{} page 0x{fc:x} not found",
                                    Tag(*bte)
                                )),
                            )
                        })
                })
                .collect::<Result<_>>()?
        }
        None => {
            pages.sort_by_key(|(id, _)| *id);
            pages.into_iter().map(|(_, p)| p).collect()
        }
    };
    let cp = |fc: u32| fc.saturating_sub(text_fc).min(len);
    let mut out = Vec::new();
    for p in order {
        for r in fkp::read_page(p.data, p.offset)? {
            let (start, end) = (cp(r.start), cp(r.end));
            if end > start {
                out.push(Run {
                    start,
                    end,
                    props: props(&r.grpprl),
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_decoding_is_lossless() {
        let all: Vec<u8> = (0..=255).collect();
        let s = decode_text(&all);
        assert_eq!(s.chars().count(), 256);
        assert_eq!(encode_text(&s), all);
        assert_eq!(decode_text(b"\xe9\x93"), "é“");
        assert_eq!(encode_text("中a"), b"?a");
    }

    #[test]
    fn from_text_matches_the_original_conventions() {
        let d = Document::from_text("abc");
        assert_eq!(d.text, "abc\r\r");
        assert_eq!(d.len(), 5);
        assert_eq!(d.paragraphs(), [(0, 4), (4, 5)]);
        assert_eq!(d.para_runs.len(), 2);
        assert_eq!(d.selection, Some((3, 3)));
        assert_eq!(d.plain_text(), "abc");
        let d = Document::from_text("um\r\ndois\nfim\n");
        assert_eq!(d.text, "um\rdois\rfim\r\r\r");
        assert_eq!(d.plain_text(), "um\ndois\nfim\n");
        let d = Document::new();
        assert_eq!(d.text, "\r\r");
        assert_eq!(d.plain_text(), "");
        let b = d.thumbnail_bitmap().unwrap();
        assert_eq!((b.width, b.height, b.depth), (108, 144, 4));
        // o mesmo tamanho e a mesma compressão de TESTE1.MAX (430 bytes,
        // menos o pixel cinza do cursor)
        assert_eq!(d.thumbnail.as_ref().unwrap().len(), 42 + 96 + 2 * 144);
    }

    #[test]
    fn thumb_sizes_follow_the_page() {
        assert_eq!(thumb_size(595, 792), (108, 144));
        assert_eq!(thumb_size(792, 595), (144, 108));
        assert_eq!(thumb_size(542, 767), (101, 144));
        assert_eq!(thumb_size(0, 0), (144, 144));
        assert_eq!(thumb_size(1, 9000), (1, 144));
    }

    #[test]
    fn plain_text_skips_anchors_and_lookups() {
        let mut d = Document::from_text("aXb");
        d.char_runs = vec![
            Run {
                start: 0,
                end: 1,
                props: CharProps::default(),
            },
            Run {
                start: 1,
                end: 2,
                props: CharProps {
                    anchor: true,
                    ..CharProps::default()
                },
            },
            Run {
                start: 2,
                end: 5,
                props: CharProps::default(),
            },
        ];
        assert_eq!(d.plain_text(), "ab");
        assert!(d.char_props_at(1).anchor);
        assert!(!d.char_props_at(2).anchor);
        assert!(d.char_props_at(99).is_default());
        assert!(d.para_props_at(99).is_default());
        assert_eq!(d.text_range(1, 3), "Xb");
        assert_eq!(d.font_name(0), "Lucida Sans");
        assert_eq!(d.font_name(7), DEFAULT_FONT);
        // texto sem \r final
        d.text = "ab".into();
        assert_eq!(d.paragraphs(), [(0, 2)]);
    }

    #[test]
    fn add_password_and_garbage() {
        // S5.MAX: Agente secreto com a senha "abc"
        let mut b = Add::default().to_bytes();
        b[5] = 1;
        b[0x1c..0x3c].copy_from_slice(b"fgh                             ");
        b[0x3c..0x3c + 15].copy_from_slice(b"\x02S5\x00umento Novo");
        let a = Add::from_bytes(&b);
        assert!(a.encrypted);
        assert_eq!(a.password, "abc");
        assert_eq!(a.title, "S5");
        // o resto do título antigo fica em `reserved` e volta na gravação
        assert_eq!(&a.reserved[0x3f..0x4b], b"\x00umento Novo");
        assert_eq!(a.to_bytes(), b);
        assert_eq!(Add::from_bytes(&a.to_bytes()), a);
        // título longo é cortado no ADD
        let a = Add {
            title: "x".repeat(40),
            ..Add::default()
        };
        let b = a.to_bytes();
        assert_eq!(b[0x3c], 33);
        assert_eq!(Add::from_bytes(&b).title, "x".repeat(33));
        assert_eq!(Add::from_bytes(&[]).floor, 0);
    }

    #[test]
    fn project_codes() {
        for p in [
            Project::None,
            Project::Card,
            Project::Banner,
            Project::Newspaper,
            Project::Other(*b"xyz!"),
        ] {
            assert_eq!(Project::from_bytes(p.to_bytes()), p);
        }
    }
}
