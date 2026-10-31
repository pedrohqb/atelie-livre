//! Sons RIFF/WAVE: PCM de 8 e 16 bits e ADPCM da Microsoft (ver `docs/formats/sons.md`).

use crate::{ErrorKind, Format, Reader, Result};

const ADAPT: [i64; 16] = [
    230, 230, 230, 230, 307, 409, 512, 614, 768, 614, 512, 409, 307, 230, 230, 230,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm {
    pub sample_rate: u32,
    pub channels: u16,
    /// Amostras intercaladas por canal.
    pub samples: Vec<i16>,
}

pub fn is_riff_wave(data: &[u8]) -> bool {
    data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE"
}

struct Fmt {
    tag: u16,
    channels: u16,
    rate: u32,
    block_align: u16,
    bits: u16,
    samples_per_block: u16,
    coefs: Vec<(i64, i64)>,
}

pub fn decode(data: &[u8]) -> Result<Pcm> {
    if !is_riff_wave(data) {
        return Err(Reader::new(data, Format::Wave).error(ErrorKind::BadMagic {
            expected: "RIFF/WAVE",
        }));
    }
    let mut r = Reader::new(data, Format::Wave);
    r.seek(4)?;
    let riff_len = r.u32_le()? as usize;
    let end = riff_len.saturating_add(8).min(data.len());
    r.seek(12)?;
    let mut fmt = None;
    let mut fact = None;
    let mut body = None;
    while r.pos() + 8 <= end {
        let id = r.tag()?;
        let len = r.u32_le()? as usize;
        let avail = end - r.pos();
        let at = r.pos();
        match &id.0 {
            b"fmt " => {
                fmt = Some(parse_fmt(
                    &mut Reader::new(r.bytes(len.min(avail))?, Format::Wave),
                    at,
                )?)
            }
            b"fact" if len >= 4 => fact = Some(r.clone().u32_le()? as usize),
            b"data" => body = Some(&data[at..at + len.min(avail)]),
            _ => {}
        }
        r.seek(at + len.min(avail))?;
        if len % 2 == 1 && r.remaining() > 0 && r.pos() < end {
            r.skip(1)?;
        }
    }
    let fmt = fmt.ok_or_else(|| r.error(ErrorKind::Invalid("no fmt chunk".into())))?;
    let body = body.ok_or_else(|| r.error(ErrorKind::Invalid("no data chunk".into())))?;
    if fmt.channels == 0 || fmt.channels > 2 {
        return Err(r.error(ErrorKind::Unsupported(format!("{} channels", fmt.channels))));
    }
    let mut samples = match (fmt.tag, fmt.bits) {
        (1, 8) => body.iter().map(|&b| (b as i16 - 128) << 8).collect(),
        (1, 16) => body
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect(),
        (2, _) => decode_ms_adpcm(&fmt, body, data.len())?,
        (tag, bits) => {
            return Err(r.error(ErrorKind::Unsupported(format!(
                "format 0x{tag:04x} with {bits} bits"
            ))));
        }
    };
    if let Some(frames) = fact {
        samples.truncate(frames.saturating_mul(fmt.channels as usize));
    }
    Ok(Pcm {
        sample_rate: fmt.rate,
        channels: fmt.channels,
        samples,
    })
}

fn parse_fmt(r: &mut Reader<'_>, at: usize) -> Result<Fmt> {
    let shift = |mut e: crate::FormatError| {
        e.offset += at;
        e
    };
    let mut f = Fmt {
        tag: r.u16_le().map_err(shift)?,
        channels: r.u16_le().map_err(shift)?,
        rate: r.u32_le().map_err(shift)?,
        block_align: 0,
        bits: 0,
        samples_per_block: 0,
        coefs: Vec::new(),
    };
    r.skip(4).map_err(shift)?;
    f.block_align = r.u16_le().map_err(shift)?;
    f.bits = r.u16_le().map_err(shift)?;
    if f.tag == 2 {
        r.skip(2).map_err(shift)?;
        f.samples_per_block = r.u16_le().map_err(shift)?;
        let n = r.u16_le().map_err(shift)?;
        for _ in 0..n {
            let a = r.i16_le().map_err(shift)? as i64;
            let b = r.i16_le().map_err(shift)? as i64;
            f.coefs.push((a, b));
        }
    }
    Ok(f)
}

fn decode_ms_adpcm(f: &Fmt, body: &[u8], file_len: usize) -> Result<Vec<i16>> {
    let ch = f.channels as usize;
    let header = 7 * ch;
    let err = |msg: String| Reader::new(&[], Format::Wave).error(ErrorKind::Invalid(msg));
    if (f.block_align as usize) < header {
        return Err(err(format!(
            "block align {} smaller than header",
            f.block_align
        )));
    }
    let mut out = Vec::with_capacity(body.len().saturating_mul(2).min(file_len.saturating_mul(4)));
    for block in body.chunks(f.block_align as usize) {
        if block.len() < header {
            break;
        }
        let mut r = Reader::new(block, Format::Wave);
        let mut coef = [(0i64, 0i64); 2];
        let mut delta = [0i64; 2];
        let mut s1 = [0i64; 2];
        let mut s2 = [0i64; 2];
        for c in coef.iter_mut().take(ch) {
            let i = r.u8()? as usize;
            *c = *f
                .coefs
                .get(i)
                .ok_or_else(|| err(format!("predictor index {i}")))?;
        }
        for d in delta.iter_mut().take(ch) {
            *d = r.i16_le()? as i64;
        }
        for s in s1.iter_mut().take(ch) {
            *s = r.i16_le()? as i64;
        }
        for s in s2.iter_mut().take(ch) {
            *s = r.i16_le()? as i64;
        }
        out.extend((0..ch).map(|c| s2[c] as i16));
        out.extend((0..ch).map(|c| s1[c] as i16));
        let max_frames = (f.samples_per_block as usize).saturating_sub(2);
        let nibbles = block[header..].iter().flat_map(|&b| [b >> 4, b & 0x0f]);
        for (k, n) in nibbles.take(max_frames * ch).enumerate() {
            let c = k % ch;
            let signed = if n >= 8 { n as i64 - 16 } else { n as i64 };
            // divisão inteira (trunca em direção a zero), como na especificação RIFF da Microsoft
            let pred = (s1[c] * coef[c].0 + s2[c] * coef[c].1) / 256;
            let sample = (pred + signed * delta[c]).clamp(i16::MIN as i64, i16::MAX as i64);
            out.push(sample as i16);
            s2[c] = s1[c];
            s1[c] = sample;
            delta[c] = ((ADAPT[n as usize] * delta[c]) >> 8).clamp(16, i32::MAX as i64);
        }
    }
    Ok(out)
}

/// Grava PCM de 16 bits como WAV.
pub fn encode_pcm16(pcm: &Pcm) -> Vec<u8> {
    let data_len = pcm.samples.len() * 2;
    let align = pcm.channels as u32 * 2;
    let mut v = Vec::with_capacity(44 + data_len);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(
        &u32::try_from(data_len)
            .unwrap_or(u32::MAX)
            .saturating_add(36)
            .to_le_bytes(),
    );
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&pcm.channels.to_le_bytes());
    v.extend_from_slice(&pcm.sample_rate.to_le_bytes());
    v.extend_from_slice(&pcm.sample_rate.saturating_mul(align).to_le_bytes());
    v.extend_from_slice(&(align as u16).to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&u32::try_from(data_len).unwrap_or(u32::MAX).to_le_bytes());
    for s in &pcm.samples {
        v.extend_from_slice(&s.to_le_bytes());
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorKind;

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend_from_slice(body);
        if body.len() % 2 == 1 {
            v.push(0);
        }
        v
    }

    fn riff(chunks: &[Vec<u8>], padding: usize) -> Vec<u8> {
        let body: Vec<u8> = chunks.concat();
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&(body.len() as u32 + 4).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(&body);
        v.extend(std::iter::repeat_n(0xaa, padding)); // preenchimento do recurso NE
        v
    }

    fn fmt_pcm(channels: u16, rate: u32, bits: u16) -> Vec<u8> {
        let align = channels * bits / 8;
        let mut f = Vec::new();
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&channels.to_le_bytes());
        f.extend_from_slice(&rate.to_le_bytes());
        f.extend_from_slice(&(rate * align as u32).to_le_bytes());
        f.extend_from_slice(&align.to_le_bytes());
        f.extend_from_slice(&bits.to_le_bytes());
        chunk(b"fmt ", &f)
    }

    fn fmt_adpcm(
        channels: u16,
        block_align: u16,
        samples_per_block: u16,
        coefs: &[(i16, i16)],
    ) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(&2u16.to_le_bytes());
        f.extend_from_slice(&channels.to_le_bytes());
        f.extend_from_slice(&11025u32.to_le_bytes());
        f.extend_from_slice(&5622u32.to_le_bytes());
        f.extend_from_slice(&block_align.to_le_bytes());
        f.extend_from_slice(&4u16.to_le_bytes());
        f.extend_from_slice(&(4 + 4 * coefs.len() as u16).to_le_bytes());
        f.extend_from_slice(&samples_per_block.to_le_bytes());
        f.extend_from_slice(&(coefs.len() as u16).to_le_bytes());
        for (a, b) in coefs {
            f.extend_from_slice(&a.to_le_bytes());
            f.extend_from_slice(&b.to_le_bytes());
        }
        chunk(b"fmt ", &f)
    }

    fn mono_block(pred: u8, delta: i16, s1: i16, s2: i16, nibbles: &[u8]) -> Vec<u8> {
        let mut b = vec![pred];
        b.extend_from_slice(&delta.to_le_bytes());
        b.extend_from_slice(&s1.to_le_bytes());
        b.extend_from_slice(&s2.to_le_bytes());
        b.extend_from_slice(nibbles);
        b
    }

    #[test]
    fn ms_adpcm_single_block() {
        let wav = riff(
            &[
                fmt_adpcm(1, 8, 4, &[(256, 0)]),
                chunk(b"data", &mono_block(0, 16, 100, 50, &[0x10])),
            ],
            3,
        );
        let pcm = decode(&wav).unwrap();
        assert_eq!((pcm.sample_rate, pcm.channels), (11025, 1));
        assert_eq!(pcm.samples, [50, 100, 116, 116]);
    }

    #[test]
    fn ms_adpcm_stereo_interleaves() {
        let mut block = vec![0u8, 0];
        for v in [16i16, 16, 100, -100, 50, -50] {
            block.extend_from_slice(&v.to_le_bytes());
        }
        block.push(0x1f);
        let wav = riff(
            &[fmt_adpcm(2, 15, 3, &[(256, 0)]), chunk(b"data", &block)],
            0,
        );
        assert_eq!(
            decode(&wav).unwrap().samples,
            [50, -50, 100, -100, 116, -116]
        );
    }

    #[test]
    fn ms_adpcm_uses_second_coefficient_and_saturates() {
        // coef (512, -256): previsão 2·s1 − s2 = 32700·2 − 0 satura em 32767
        let wav = riff(
            &[
                fmt_adpcm(1, 8, 4, &[(256, 0), (512, -256)]),
                chunk(b"data", &mono_block(1, 16, 32700, 0, &[0x70])),
            ],
            0,
        );
        let s = decode(&wav).unwrap().samples;
        assert_eq!(&s[..3], &[0, 32700, 32767]);
    }

    #[test]
    fn ms_adpcm_prediction_truncates_toward_zero() {
        // caso real (ARTIST.SND 0x210): (−198·460 + −197·−208) / 256 = −195,7 → −195, não −196
        let wav = riff(
            &[
                fmt_adpcm(1, 8, 4, &[(256, 0), (460, -208)]),
                chunk(b"data", &mono_block(1, 16, -198, -197, &[0x00])),
            ],
            0,
        );
        assert_eq!(&decode(&wav).unwrap().samples[..3], &[-197, -198, -195]);
    }

    #[test]
    fn fact_chunk_truncates_output() {
        let wav = riff(
            &[
                fmt_adpcm(1, 8, 4, &[(256, 0)]),
                chunk(b"fact", &3u32.to_le_bytes()),
                chunk(b"data", &mono_block(0, 16, 100, 50, &[0x10])),
            ],
            0,
        );
        assert_eq!(decode(&wav).unwrap().samples, [50, 100, 116]);
    }

    #[test]
    fn partial_last_block_decodes_present_bytes() {
        let mut data = mono_block(0, 16, 100, 50, &[0x10]);
        data.extend_from_slice(&mono_block(0, 16, 7, 8, &[]));
        let wav = riff(&[fmt_adpcm(1, 8, 4, &[(256, 0)]), chunk(b"data", &data)], 0);
        assert_eq!(decode(&wav).unwrap().samples, [50, 100, 116, 116, 8, 7]);
    }

    #[test]
    fn incomplete_trailing_header_is_ignored() {
        let mut data = mono_block(0, 16, 100, 50, &[0x10]);
        data.extend_from_slice(&[0, 1, 2]);
        let wav = riff(&[fmt_adpcm(1, 8, 4, &[(256, 0)]), chunk(b"data", &data)], 0);
        assert_eq!(decode(&wav).unwrap().samples, [50, 100, 116, 116]);
    }

    #[test]
    fn bad_predictor_index_is_invalid() {
        let wav = riff(
            &[
                fmt_adpcm(1, 8, 4, &[(256, 0)]),
                chunk(b"data", &mono_block(5, 16, 0, 0, &[0])),
            ],
            0,
        );
        assert!(matches!(
            decode(&wav).unwrap_err().kind,
            ErrorKind::Invalid(_)
        ));
    }

    #[test]
    fn pcm8_converted_to_i16() {
        let wav = riff(&[fmt_pcm(1, 8000, 8), chunk(b"data", &[128, 255, 0])], 0);
        assert_eq!(decode(&wav).unwrap().samples, [0, 32512, -32768]);
    }

    #[test]
    fn skips_unknown_chunks_with_odd_size() {
        let wav = riff(
            &[
                chunk(b"LIST", &[1, 2, 3]),
                fmt_pcm(1, 8000, 8),
                chunk(b"data", &[128]),
            ],
            0,
        );
        assert_eq!(decode(&wav).unwrap().samples, [0]);
    }

    #[test]
    fn unsupported_format_tag() {
        let mut f = fmt_pcm(1, 8000, 4);
        f[8] = 0x11; // IMA ADPCM
        let wav = riff(&[f, chunk(b"data", &[0; 4])], 0);
        assert!(matches!(
            decode(&wav).unwrap_err().kind,
            ErrorKind::Unsupported(_)
        ));
    }

    #[test]
    fn encode_huge_rate_does_not_overflow() {
        let p = Pcm {
            sample_rate: u32::MAX,
            channels: 2,
            samples: vec![0],
        };
        let bytes = encode_pcm16(&p);
        assert_eq!(
            u32::from_le_bytes(bytes[28..32].try_into().unwrap()),
            u32::MAX
        );
    }

    #[test]
    fn encode_roundtrip() {
        let p = Pcm {
            sample_rate: 11025,
            channels: 2,
            samples: vec![0, -1, 32767, -32768],
        };
        let bytes = encode_pcm16(&p);
        assert!(is_riff_wave(&bytes));
        assert_eq!(decode(&bytes).unwrap(), p);
    }

    #[test]
    fn every_truncation_and_corruption_is_handled() {
        let wav = riff(
            &[
                fmt_adpcm(1, 8, 4, &[(256, 0), (512, -256)]),
                chunk(b"fact", &4u32.to_le_bytes()),
                chunk(b"data", &mono_block(1, 16, 100, 50, &[0x10])),
            ],
            0,
        );
        for n in 0..wav.len() {
            let _ = decode(&wav[..n]);
        }
        for i in 0..wav.len() {
            for x in [0x00, 0xff, 0x7f, 0x80] {
                let mut c = wav.clone();
                c[i] = x;
                let _ = decode(&c);
            }
        }
    }
}
