//! Codificador de GIF animado (GIF89a) com paleta global de 256 cores e
//! compressão LZW, sem dependências. Serve para gravar o desenho animado
//! do Ateliê Livre num arquivo que qualquer visualizador abre.
//!
//! Estrutura gerada: cabeçalho `GIF89a`, descritor lógico da tela com tabela
//! global de cores (256 entradas), extensão de aplicação `NETSCAPE2.0` (laço
//! infinito), e para cada quadro uma extensão de controle gráfico com o
//! atraso, um descritor de imagem e os dados LZW em blocos de até 255 bytes.

use crate::palette::Rgb;

/// Tamanho mínimo dos códigos LZW para índices de 8 bits.
const MIN_CODE_SIZE: u8 = 8;
const CLEAR: u16 = 256;
const END: u16 = 257;
const MAX_CODE: u16 = 4095;

/// Um quadro da animação: índices da paleta (`width × height` bytes) e o
/// atraso em centésimos de segundo.
pub struct Frame<'a> {
    pub pixels: &'a [u8],
    pub delay_cs: u16,
}

/// Monta o GIF animado em memória. `palette` é truncada ou completada com
/// preto até 256 cores; quadros com tamanho errado são ignorados.
pub fn encode(width: u16, height: u16, palette: &[Rgb], frames: &[Frame<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"GIF89a");
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    // tabela global presente, 8 bits de cor, 256 entradas
    out.push(0xF7);
    out.push(0); // cor de fundo
    out.push(0); // proporção do pixel
    for i in 0..256 {
        let p = palette.get(i).copied().unwrap_or(Rgb { r: 0, g: 0, b: 0 });
        out.extend_from_slice(&[p.r, p.g, p.b]);
    }
    // laço infinito (NETSCAPE 2.0)
    out.extend_from_slice(&[0x21, 0xFF, 0x0B]);
    out.extend_from_slice(b"NETSCAPE2.0");
    out.extend_from_slice(&[0x03, 0x01, 0x00, 0x00, 0x00]);
    let len = width as usize * height as usize;
    for f in frames.iter().filter(|f| f.pixels.len() == len) {
        // extensão de controle gráfico: sem transparência, descarte "não especificado"
        out.extend_from_slice(&[0x21, 0xF9, 0x04, 0x00]);
        out.extend_from_slice(&f.delay_cs.to_le_bytes());
        out.extend_from_slice(&[0x00, 0x00]);
        // descritor de imagem: quadro inteiro, sem tabela local, sem entrelaçar
        out.push(0x2C);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.push(0x00);
        out.push(MIN_CODE_SIZE);
        write_blocks(&mut out, &lzw_compress(f.pixels));
        out.push(0x00);
    }
    out.push(0x3B);
    out
}

/// Sub-blocos de dados de até 255 bytes cada.
fn write_blocks(out: &mut Vec<u8>, data: &[u8]) {
    for chunk in data.chunks(255) {
        out.push(chunk.len() as u8);
        out.extend_from_slice(chunk);
    }
}

/// Escritor de bits de ordem LSB-first, como o GIF pede.
struct BitWriter {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl BitWriter {
    fn write(&mut self, code: u16, bits: u32) {
        self.acc |= (code as u32) << self.n;
        self.n += bits;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

/// LZW variável (GIF) sobre índices de 8 bits; o dicionário é um mapa
/// `(prefixo, byte) → código` guardado numa tabela de 4096 × 256.
pub fn lzw_compress(data: &[u8]) -> Vec<u8> {
    let mut w = BitWriter {
        out: Vec::with_capacity(data.len() / 2),
        acc: 0,
        n: 0,
    };
    // tabela de filhos: índice = prefixo * 256 + byte; 0 = ausente
    let mut table = vec![0u16; (MAX_CODE as usize + 1) * 256];
    let mut next = END + 1;
    let mut size = MIN_CODE_SIZE as u32 + 1;
    w.write(CLEAR, size);
    let mut iter = data.iter();
    let Some(&first) = iter.next() else {
        w.write(END, size);
        return w.finish();
    };
    let mut prefix = first as u16;
    for &b in iter {
        let slot = prefix as usize * 256 + b as usize;
        if table[slot] != 0 {
            prefix = table[slot];
            continue;
        }
        w.write(prefix, size);
        if next <= MAX_CODE {
            table[slot] = next;
            if next == (1 << size) && size < 12 {
                size += 1;
            }
            next += 1;
        } else {
            w.write(CLEAR, size);
            table.fill(0);
            next = END + 1;
            size = MIN_CODE_SIZE as u32 + 1;
        }
        prefix = b as u16;
    }
    w.write(prefix, size);
    w.write(END, size);
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Leitor de bits LSB-first, só para o teste.
    struct BitReader<'a> {
        data: &'a [u8],
        pos: usize,
        acc: u32,
        n: u32,
    }

    impl BitReader<'_> {
        fn read(&mut self, bits: u32) -> Option<u16> {
            while self.n < bits {
                let b = *self.data.get(self.pos)?;
                self.pos += 1;
                self.acc |= (b as u32) << self.n;
                self.n += 8;
            }
            let v = (self.acc & ((1 << bits) - 1)) as u16;
            self.acc >>= bits;
            self.n -= bits;
            Some(v)
        }
    }

    /// Descompressor LZW mínimo, para conferir o codificador.
    fn lzw_decompress(data: &[u8]) -> Vec<u8> {
        let mut r = BitReader {
            data,
            pos: 0,
            acc: 0,
            n: 0,
        };
        let mut out = Vec::new();
        let mut dict: Vec<Vec<u8>> = Vec::new();
        let reset = |d: &mut Vec<Vec<u8>>| {
            d.clear();
            d.extend((0..256u16).map(|i| vec![i as u8]));
            d.push(Vec::new()); // CLEAR
            d.push(Vec::new()); // END
        };
        reset(&mut dict);
        let mut size = 9;
        let mut prev: Option<Vec<u8>> = None;
        while let Some(code) = r.read(size) {
            if code == CLEAR {
                reset(&mut dict);
                size = 9;
                prev = None;
                continue;
            }
            if code == END {
                break;
            }
            let entry = if (code as usize) < dict.len() {
                dict[code as usize].clone()
            } else {
                let p = prev.clone().expect("código fora do dicionário");
                let mut e = p.clone();
                e.push(p[0]);
                e
            };
            out.extend_from_slice(&entry);
            if let Some(p) = prev {
                let mut e = p;
                e.push(entry[0]);
                dict.push(e);
                if dict.len() == (1 << size) && size < 12 {
                    size += 1;
                }
            }
            prev = Some(entry);
        }
        out
    }

    fn pal() -> Vec<Rgb> {
        (0..256u32)
            .map(|i| Rgb {
                r: i as u8,
                g: (i * 3 % 256) as u8,
                b: (255 - i) as u8,
            })
            .collect()
    }

    #[test]
    fn lzw_roundtrip_small() {
        let data = b"abracadabra abracadabra".to_vec();
        assert_eq!(lzw_decompress(&lzw_compress(&data)), data);
        assert_eq!(lzw_decompress(&lzw_compress(&[])), Vec::<u8>::new());
        assert_eq!(lzw_decompress(&lzw_compress(&[7])), vec![7]);
    }

    #[test]
    fn lzw_roundtrip_large_with_dictionary_reset() {
        // mais de 4096 códigos novos: força o CLEAR no meio
        let mut data = Vec::new();
        let mut x: u32 = 1;
        for _ in 0..200_000 {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            data.push((x >> 16) as u8 % 7);
        }
        let packed = lzw_compress(&data);
        assert!(packed.len() < data.len());
        assert_eq!(lzw_decompress(&packed), data);
    }

    #[test]
    fn lzw_compresses_runs() {
        let data = vec![0u8; 100_000];
        assert!(lzw_compress(&data).len() < 1000);
    }

    /// Lê os blocos de um GIF gerado e devolve (largura, altura, [(atraso, pixels)]).
    fn parse(gif: &[u8]) -> (u16, u16, Vec<(u16, Vec<u8>)>) {
        assert_eq!(&gif[..6], b"GIF89a");
        let w = u16::from_le_bytes([gif[6], gif[7]]);
        let h = u16::from_le_bytes([gif[8], gif[9]]);
        assert_eq!(gif[10], 0xF7);
        let mut pos = 13 + 256 * 3;
        let mut frames = Vec::new();
        let mut delay = 0;
        loop {
            match gif[pos] {
                0x21 => {
                    let label = gif[pos + 1];
                    pos += 2;
                    if label == 0xF9 {
                        delay = u16::from_le_bytes([gif[pos + 2], gif[pos + 3]]);
                    }
                    while gif[pos] != 0 {
                        pos += gif[pos] as usize + 1;
                    }
                    pos += 1;
                }
                0x2C => {
                    assert_eq!(&gif[pos + 1..pos + 5], &[0, 0, 0, 0]);
                    assert_eq!(u16::from_le_bytes([gif[pos + 5], gif[pos + 6]]), w);
                    assert_eq!(u16::from_le_bytes([gif[pos + 7], gif[pos + 8]]), h);
                    assert_eq!(gif[pos + 10], MIN_CODE_SIZE);
                    pos += 11;
                    let mut data = Vec::new();
                    while gif[pos] != 0 {
                        let n = gif[pos] as usize;
                        data.extend_from_slice(&gif[pos + 1..pos + 1 + n]);
                        pos += n + 1;
                    }
                    pos += 1;
                    frames.push((delay, lzw_decompress(&data)));
                }
                0x3B => {
                    assert_eq!(pos + 1, gif.len());
                    return (w, h, frames);
                }
                b => panic!("bloco inesperado {b:#x} em {pos}"),
            }
        }
    }

    #[test]
    fn gif_has_palette_loop_and_frames() {
        let a: Vec<u8> = (0..60u8).collect();
        let b: Vec<u8> = (0..60u8).rev().collect();
        let gif = encode(
            10,
            6,
            &pal(),
            &[
                Frame {
                    pixels: &a,
                    delay_cs: 25,
                },
                Frame {
                    pixels: &b,
                    delay_cs: 100,
                },
                Frame {
                    pixels: &[1, 2, 3],
                    delay_cs: 1,
                },
            ],
        );
        // tabela de cores logo após o descritor da tela
        assert_eq!(&gif[13..16], &[0, 0, 255]);
        assert_eq!(&gif[13 + 3 * 255..13 + 3 * 256], &[255, 253, 0]);
        let netscape = gif.windows(11).position(|w| w == b"NETSCAPE2.0");
        assert!(netscape.is_some());
        let (w, h, frames) = parse(&gif);
        assert_eq!((w, h), (10, 6));
        assert_eq!(frames, vec![(25, a), (100, b)]);
    }

    #[test]
    fn gif_short_palette_is_padded() {
        let gif = encode(1, 1, &[Rgb { r: 9, g: 8, b: 7 }], &[]);
        assert_eq!(&gif[13..16], &[9, 8, 7]);
        assert_eq!(&gif[16..19], &[0, 0, 0]);
        assert_eq!(parse(&gif).2.len(), 0);
    }
}
