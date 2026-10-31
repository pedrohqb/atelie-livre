//! Roteiros de animação `ATAG` (ver `docs/formats/atag.md`).
//!
//! Aparecem em dois lugares: como recursos `ATAG` dos executáveis
//! (`ARTIST.EXE`, `WRITER.EXE`), onde as imagens são ids dos `.PIC`, e como
//! a parte `ATAG` dos `CHNK` aninhados de `A-MAG100.ANI` (adesivos
//! animados), onde as imagens são números de chunks `PKBM` do mesmo `CHNK`.
//! O leitor aceita as duas variantes; a única diferença de layout é um
//! prefixo de 4 bytes (`u16 1`, `u16 tamanho`) na variante dos `CHNK`.

use crate::{ErrorKind, Format, Reader, Result};

const OP_BEGIN: u16 = 1;
const OP_CELLS: u16 = 2;
const OP_CELL: u16 = 3;
const OP_PATH_EXE: u16 = 4;
const OP_STEPS: u16 = 7;
const OP_PATH_CHNK: u16 = 8;
const OP_END: u16 = 9;

const CELL_SIZE: usize = 18;
const STEP_SIZE: usize = 30;
const MOVE_SIZE: usize = 8;
/// Marca que abre cada passo (`00 01`).
const STEP_MARK: u16 = 0x0100;

/// Onde o roteiro estava guardado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Recurso `ATAG` de um executável; `Cell::image` é id do `.PIC`.
    Resource,
    /// Parte `ATAG` de um `CHNK` aninhado; `Cell::image` é número de chunk `PKBM`.
    Chunk,
}

/// Uma imagem da animação com a duração que ela fica na tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Id da imagem (`.PIC`) ou número do chunk `PKBM`.
    pub image: u16,
    /// Segundo id gravado: nos `CHNK` é a máscara (`image + 1`); nos
    /// executáveis é sempre `image + 3`, que no `.PIC` não corresponde à
    /// máscara (a máscara real é `image + 1`).
    pub mask: u16,
    /// Deslocamento gravado no registro; sempre (0, 0) nos dados conhecidos.
    pub offset: (i16, i16),
    /// Duração em milissegundos.
    pub duration_ms: u32,
    /// Id do som em `ARTIST.SND`/`WRITER.SND`, ou 0.
    pub sound: u16,
}

/// Um passo da animação: qual imagem mostrar e onde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// Índice em [`Script::cells`].
    pub cell: u16,
    /// Posição do canto superior esquerdo em relação à posição inicial
    /// (soma dos `delta` dos passos anteriores).
    pub x: i16,
    pub y: i16,
    /// Canto inferior direito (`x + largura`, `y + altura` da imagem).
    pub right: i16,
    pub bottom: i16,
    /// Deslocamento até o passo seguinte.
    pub delta: (i16, i16),
}

/// Um trecho do caminho: deslocamento aplicado após um passo ou, com
/// `tween > 0`, movimento interpolado em `tween` quadros.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub dx: i16,
    pub dy: i16,
    /// Número de quadros de interpolação (0 = salto direto ao fim do passo).
    pub tween: u8,
}

/// Roteiro de animação decodificado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub source: Source,
    /// Campo de flags do cabeçalho (0, 0x10 nos executáveis; 0x800 nos `CHNK`).
    pub flags: u32,
    /// Imagens com duração e som.
    pub cells: Vec<Cell>,
    /// Passos explícitos (só nos `CHNK`). Vazio nos executáveis, onde cada
    /// célula é um passo, na ordem.
    pub steps: Vec<Step>,
    /// Tipo do caminho: 5 = deslocamento por passo; 9 = movimento interpolado.
    pub path_kind: u16,
    /// Posição inicial na tela de 640×480 em que o roteiro foi gravado.
    pub origin: (i16, i16),
    /// Caminho: um deslocamento por passo (o último é zero).
    pub moves: Vec<Move>,
}

impl Script {
    /// Sequência de passos a executar: os explícitos ou, sem eles, as
    /// células em ordem com as posições acumuladas do caminho.
    pub fn playback(&self) -> Vec<Step> {
        if !self.steps.is_empty() {
            return self.steps.clone();
        }
        let (mut x, mut y) = (0i16, 0i16);
        self.cells
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let m = self.moves.get(i).copied().unwrap_or(Move {
                    dx: 0,
                    dy: 0,
                    tween: 0,
                });
                let s = Step {
                    cell: i as u16,
                    x,
                    y,
                    right: x,
                    bottom: y,
                    delta: (m.dx, m.dy),
                };
                x = x.wrapping_add(m.dx);
                y = y.wrapping_add(m.dy);
                s
            })
            .collect()
    }
}

/// Lê um roteiro `ATAG` (recurso de executável ou parte de `CHNK`).
pub fn parse(data: &[u8]) -> Result<Script> {
    let mut r = Reader::new(data, Format::Atag);
    let invalid = |r: &Reader, msg: String| r.error(ErrorKind::Invalid(msg));

    // Variante dos CHNK: `u16 1, u16 tamanho` antes do cabeçalho comum.
    // Nos recursos o segundo u16 é a metade alta do `u32 1`, logo zero.
    let (source, limit) = if r.remaining() >= 4 && data[0..2] == [1, 0] && data[2..4] != [0, 0] {
        r.skip(2)?;
        let len = r.u16_le()? as usize;
        if len > data.len() {
            return Err(invalid(
                &r,
                format!("tamanho {len} maior que os dados ({})", data.len()),
            ));
        }
        (Source::Chunk, len)
    } else {
        (Source::Resource, data.len())
    };

    let version = r.u32_le()?;
    if version != 1 {
        return Err(invalid(&r, format!("versão {version}, esperada 1")));
    }
    let flags = r.u32_le()?;
    let _reserved = r.u32_le()?;

    let mut script = Script {
        source,
        flags,
        cells: Vec::new(),
        steps: Vec::new(),
        path_kind: 0,
        origin: (0, 0),
        moves: Vec::new(),
    };
    let mut seen_begin = false;

    while r.pos() + 2 <= limit {
        let op = r.u16_le()?;
        match op {
            // Recursos terminam sem marca de fim: o resto é preenchimento.
            0 if source == Source::Resource => break,
            OP_BEGIN => seen_begin = true,
            OP_CELLS => {
                r.skip(6)?;
                let n = r.u16_le()? as usize;
                script.cells.reserve(n);
                for _ in 0..n {
                    script.cells.push(read_cell(&mut r)?);
                }
            }
            OP_STEPS => {
                r.skip(12)?;
                let n = r.u16_le()? as usize;
                script.steps.reserve(n);
                for _ in 0..n {
                    let s = read_step(&mut r)?;
                    if (s.cell as usize) >= script.cells.len() {
                        return Err(invalid(
                            &r,
                            format!("passo usa célula {} de {}", s.cell, script.cells.len()),
                        ));
                    }
                    script.steps.push(s);
                }
            }
            OP_PATH_EXE | OP_PATH_CHNK => {
                r.skip(2)?;
                script.path_kind = r.u16_le()?;
                script.origin = (r.i16_le()?, r.i16_le()?);
                r.skip(4)?;
                let n = r.u16_le()? as usize;
                script.moves.reserve(n);
                for _ in 0..n {
                    script.moves.push(read_move(&mut r)?);
                }
            }
            OP_END => {
                r.skip(2)?;
                break;
            }
            other => {
                return Err(invalid(&r, format!("código {other} desconhecido")));
            }
        }
    }
    if !seen_begin {
        return Err(invalid(&r, "sem marca de início (código 1)".into()));
    }
    if script.cells.is_empty() {
        return Err(invalid(&r, "sem células (código 2)".into()));
    }
    Ok(script)
}

fn read_cell(r: &mut Reader) -> Result<Cell> {
    let start = r.pos();
    let op = r.u16_le()?;
    if op != OP_CELL {
        return Err(r.error(ErrorKind::Invalid(format!(
            "célula com código {op}, esperado {OP_CELL}"
        ))));
    }
    r.skip(2)?;
    let image = r.u16_le()?;
    let mask = r.u16_le()?;
    let offset = (r.i16_le()?, r.i16_le()?);
    let duration_ms = r.u32_le()?;
    let sound = r.u16_le()?;
    debug_assert_eq!(r.pos() - start, CELL_SIZE);
    Ok(Cell {
        image,
        mask,
        offset,
        duration_ms,
        sound,
    })
}

fn read_step(r: &mut Reader) -> Result<Step> {
    let start = r.pos();
    let mark = r.u16_le()?;
    if mark != STEP_MARK {
        return Err(r.error(ErrorKind::Invalid(format!(
            "passo com marca 0x{mark:04x}, esperada 0x{STEP_MARK:04x}"
        ))));
    }
    let delta = (r.i16_le()?, r.i16_le()?);
    r.skip(6)?;
    let cell = r.u16_le()?;
    let x = r.i16_le()?;
    let y = r.i16_le()?;
    let right = r.i16_le()?;
    let bottom = r.i16_le()?;
    r.skip(8)?;
    debug_assert_eq!(r.pos() - start, STEP_SIZE);
    Ok(Step {
        cell,
        x,
        y,
        right,
        bottom,
        delta,
    })
}

fn read_move(r: &mut Reader) -> Result<Move> {
    let start = r.pos();
    let _zero = r.u8()?;
    let tween = r.u8()?;
    let dx = r.i16_le()?;
    let dy = r.i16_le()?;
    r.skip(2)?;
    debug_assert_eq!(r.pos() - start, MOVE_SIZE);
    Ok(Move { dx, dy, tween })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le16(v: &mut Vec<u8>, x: i32) {
        v.extend_from_slice(&(x as i16).to_le_bytes());
    }
    fn le32(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_le_bytes());
    }
    fn cell(v: &mut Vec<u8>, image: u16, dur: u32, snd: u16) {
        le16(v, 3);
        le16(v, 0);
        le16(v, image as i32);
        le16(v, image as i32 + 3);
        le32(v, 0);
        le32(v, dur);
        le16(v, snd as i32);
    }
    fn mv(v: &mut Vec<u8>, tween: u8, dx: i32, dy: i32) {
        v.push(0);
        v.push(tween);
        le16(v, dx);
        le16(v, dy);
        le16(v, 0);
    }

    /// Recurso de executável: duas células, caminho tipo 5, sem marca de fim.
    fn resource_bytes() -> Vec<u8> {
        let mut v = Vec::new();
        le32(&mut v, 1);
        le32(&mut v, 0x10);
        le32(&mut v, 0);
        le16(&mut v, 1);
        le16(&mut v, 2);
        v.extend_from_slice(&[0; 6]);
        le16(&mut v, 2);
        cell(&mut v, 0x64dc, 200, 0);
        cell(&mut v, 0x64e4, 400, 0x239);
        le16(&mut v, 4);
        le16(&mut v, 0);
        le16(&mut v, 5);
        le16(&mut v, 601);
        le16(&mut v, 289);
        le32(&mut v, 0);
        le16(&mut v, 2);
        mv(&mut v, 0, -104, -18);
        mv(&mut v, 0, 0, 0);
        v.extend_from_slice(&[0; 16]);
        v
    }

    #[test]
    fn parses_resource_variant() {
        let s = parse(&resource_bytes()).unwrap();
        assert_eq!(s.source, Source::Resource);
        assert_eq!(s.flags, 0x10);
        assert_eq!(s.cells.len(), 2);
        assert_eq!(
            s.cells[1],
            Cell {
                image: 0x64e4,
                mask: 0x64e7,
                offset: (0, 0),
                duration_ms: 400,
                sound: 0x239
            }
        );
        assert!(s.steps.is_empty());
        assert_eq!(s.path_kind, 5);
        assert_eq!(s.origin, (601, 289));
        assert_eq!(
            s.moves,
            vec![
                Move {
                    dx: -104,
                    dy: -18,
                    tween: 0
                },
                Move {
                    dx: 0,
                    dy: 0,
                    tween: 0
                }
            ]
        );
        let p = s.playback();
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].cell, p[0].x, p[0].y), (0, 0, 0));
        assert_eq!((p[1].cell, p[1].x, p[1].y), (1, -104, -18));
    }

    #[test]
    fn parses_tween_moves() {
        let mut v = resource_bytes();
        // troca o primeiro movimento por um interpolado em 20 quadros
        let pos = v.len() - 16 - 16;
        v[pos + 1] = 20;
        let s = parse(&v).unwrap();
        assert_eq!(s.moves[0].tween, 20);
        assert_eq!(s.moves[0].dx, -104);
    }

    /// Parte de CHNK: prefixo de tamanho, passos explícitos e marca de fim.
    fn chunk_bytes() -> Vec<u8> {
        let mut v = Vec::new();
        le16(&mut v, 1);
        le16(&mut v, 0); // tamanho, preenchido abaixo
        le32(&mut v, 1);
        le32(&mut v, 0x800);
        le32(&mut v, 1);
        le16(&mut v, 1);
        le16(&mut v, 2);
        v.extend_from_slice(&[0; 6]);
        le16(&mut v, 2);
        cell(&mut v, 2, 300, 0);
        cell(&mut v, 4, 300, 0);
        le16(&mut v, 7);
        v.extend_from_slice(&[0; 12]);
        le16(&mut v, 3);
        for (dx, dy, c, x, y, w, h) in [
            (-1, 5, 0, 0, 0, 216, 77),
            (56, -8, 1, -1, 5, 280, 66),
            (0, 0, 0, 55, -3, 216, 77),
        ] {
            le16(&mut v, STEP_MARK as i32);
            le16(&mut v, dx);
            le16(&mut v, dy);
            v.extend_from_slice(&[0; 6]);
            le16(&mut v, c);
            le16(&mut v, x);
            le16(&mut v, y);
            le16(&mut v, x + w);
            le16(&mut v, y + h);
            v.extend_from_slice(&[0; 8]);
        }
        le16(&mut v, 8);
        le16(&mut v, 0);
        le16(&mut v, 5);
        le16(&mut v, -215);
        le16(&mut v, 253);
        le32(&mut v, 0);
        le16(&mut v, 3);
        mv(&mut v, 0, -1, 5);
        mv(&mut v, 0, 56, -8);
        mv(&mut v, 0, 0, 0);
        le16(&mut v, 9);
        le16(&mut v, 0);
        let len = v.len() as u16;
        v[2..4].copy_from_slice(&len.to_le_bytes());
        // lixo após o fim, como nas partes reais
        v.extend_from_slice(&[0xfe, 0xf7, 0x00, 0x00, 0x0f]);
        v
    }

    #[test]
    fn parses_chunk_variant() {
        let s = parse(&chunk_bytes()).unwrap();
        assert_eq!(s.source, Source::Chunk);
        assert_eq!(s.flags, 0x800);
        assert_eq!(s.cells.len(), 2);
        assert_eq!(s.cells[0].image, 2);
        assert_eq!(s.cells[0].mask, 5);
        assert_eq!(s.steps.len(), 3);
        assert_eq!(
            s.steps[1],
            Step {
                cell: 1,
                x: -1,
                y: 5,
                right: 279,
                bottom: 71,
                delta: (56, -8)
            }
        );
        assert_eq!(s.origin, (-215, 253));
        assert_eq!(s.moves.len(), 3);
        assert_eq!(s.playback(), s.steps);
    }

    #[test]
    fn rejects_bad_version() {
        let mut v = resource_bytes();
        v[0] = 2;
        let e = parse(&v).unwrap_err();
        assert_eq!(e.format, Format::Atag);
        assert!(matches!(e.kind, ErrorKind::Invalid(_)));
    }

    #[test]
    fn rejects_unknown_op_and_truncation() {
        let mut v = resource_bytes();
        v[12] = 6;
        assert!(parse(&v).is_err());
        let v = resource_bytes();
        assert!(parse(&v[..40]).is_err());
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn rejects_step_out_of_range() {
        let mut v = chunk_bytes();
        // célula do primeiro passo: 4 + 12 + 2 + 10 + 36 + 16 + 12 = 92
        v[92] = 7;
        assert!(parse(&v).is_err());
    }

    #[test]
    fn chunk_length_must_fit() {
        let mut v = chunk_bytes();
        v[2..4].copy_from_slice(&0xffffu16.to_le_bytes());
        assert!(parse(&v).is_err());
    }
}
