//! Páginas FKP de 512 bytes (`FDPC`, `FDPP`): trechos de texto e o grpprl
//! de cada um.
//!
//! ```text
//! u16 crun, u16 1, (crun+1) × u32 fc, crun × u16 offset do grpprl (0 = nenhum)
//! ... grpprls no fim da página: u8 tamanho + sprms
//! ```

use super::err;
use crate::{ErrorKind, Result};

/// Tamanho de uma página FKP.
pub const PAGE: usize = 512;

/// Trecho `[start, end)` em fc (offset no arquivo) com o grpprl (sem o byte
/// de tamanho; vazio = sem propriedades).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FkpRun {
    pub start: u32,
    pub end: u32,
    pub grpprl: Vec<u8>,
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Lê uma página; `base` é o offset da página no arquivo (para os erros).
pub fn read_page(page: &[u8], base: usize) -> Result<Vec<FkpRun>> {
    if page.len() < 4 {
        return Err(err(base, ErrorKind::Invalid("FKP page too short".into())));
    }
    let crun = u16_at(page, 0) as usize;
    let header = 4 + 4 * (crun + 1) + 2 * crun;
    if header > page.len() {
        return Err(err(
            base,
            ErrorKind::Invalid(format!("FKP with {crun} runs does not fit its page")),
        ));
    }
    let offs = 4 + 4 * (crun + 1);
    let mut runs = Vec::with_capacity(crun);
    for k in 0..crun {
        let start = u32_at(page, 4 + 4 * k);
        let end = u32_at(page, 8 + 4 * k);
        let o = u16_at(page, offs + 2 * k) as usize;
        let grpprl = if o == 0 {
            Vec::new()
        } else {
            let cb = *page.get(o).ok_or_else(|| {
                err(
                    base + offs + 2 * k,
                    ErrorKind::Invalid(format!("grpprl offset 0x{o:x} outside page")),
                )
            })? as usize;
            page.get(o + 1..o + 1 + cb)
                .ok_or_else(|| {
                    err(
                        base + o,
                        ErrorKind::Invalid(format!("grpprl of {cb} bytes outside page")),
                    )
                })?
                .to_vec()
        };
        runs.push(FkpRun { start, end, grpprl });
    }
    Ok(runs)
}

/// Uma página em montagem.
#[derive(Default)]
struct Builder<'a> {
    runs: Vec<&'a FkpRun>,
    /// grpprls distintos, na ordem do primeiro uso.
    grpprls: Vec<&'a [u8]>,
    grp_bytes: usize,
}

impl<'a> Builder<'a> {
    /// Cabe mais este trecho?
    fn fits(&self, run: &FkpRun) -> bool {
        let n = self.runs.len() + 1;
        let header = 4 + 4 * (n + 1) + 2 * n;
        let new_grp = if run.grpprl.is_empty() || self.grpprls.contains(&run.grpprl.as_slice()) {
            0
        } else {
            1 + run.grpprl.len()
        };
        header + self.grp_bytes + new_grp <= PAGE
    }

    fn push(&mut self, run: &'a FkpRun) {
        if !run.grpprl.is_empty() && !self.grpprls.contains(&run.grpprl.as_slice()) {
            self.grpprls.push(&run.grpprl);
            self.grp_bytes += 1 + run.grpprl.len();
        }
        self.runs.push(run);
    }

    fn page(&self, zero_first_fc: bool) -> [u8; PAGE] {
        let mut p = [0u8; PAGE];
        let crun = self.runs.len();
        p[0..2].copy_from_slice(&(crun as u16).to_le_bytes());
        p[2..4].copy_from_slice(&1u16.to_le_bytes());
        for (k, r) in self.runs.iter().enumerate() {
            let fc = if k == 0 && zero_first_fc { 0 } else { r.start };
            p[4 + 4 * k..8 + 4 * k].copy_from_slice(&fc.to_le_bytes());
        }
        if let Some(last) = self.runs.last() {
            p[4 + 4 * crun..8 + 4 * crun].copy_from_slice(&last.end.to_le_bytes());
        }
        // grpprls do fim para o começo, na ordem do primeiro uso
        let mut at = PAGE;
        let mut placed: Vec<usize> = Vec::with_capacity(self.grpprls.len());
        for g in &self.grpprls {
            at -= 1 + g.len();
            p[at] = g.len() as u8;
            p[at + 1..at + 1 + g.len()].copy_from_slice(g);
            placed.push(at);
        }
        let offs = 4 + 4 * (crun + 1);
        for (k, r) in self.runs.iter().enumerate() {
            let o = if r.grpprl.is_empty() {
                0
            } else {
                let i = self
                    .grpprls
                    .iter()
                    .position(|g| *g == r.grpprl.as_slice())
                    .expect("grpprl placed");
                placed[i] as u16
            };
            p[offs + 2 * k..offs + 2 * k + 2].copy_from_slice(&o.to_le_bytes());
        }
        p
    }
}

/// Distribui os trechos em páginas, como o original: cada página recebe
/// trechos até não caber o próximo (6 bytes de índice mais o grpprl, se for
/// novo na página); os grpprls iguais de uma página são gravados uma vez, do
/// fim para o começo, na ordem do primeiro uso. O primeiro fc da primeira
/// página é gravado como 0. Grpprls de mais de 255 bytes são cortados.
pub fn write_pages(runs: &[FkpRun]) -> Vec<[u8; PAGE]> {
    let clipped: Vec<FkpRun> = runs
        .iter()
        .map(|r| FkpRun {
            start: r.start,
            end: r.end,
            grpprl: r.grpprl[..r.grpprl.len().min(255)].to_vec(),
        })
        .collect();
    let mut pages = Vec::new();
    let mut b = Builder::default();
    for r in &clipped {
        if !b.runs.is_empty() && !b.fits(r) {
            pages.push(b.page(pages.is_empty()));
            b = Builder::default();
        }
        b.push(r);
    }
    if !b.runs.is_empty() || pages.is_empty() {
        pages.push(b.page(pages.is_empty()));
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(start: u32, end: u32, g: &[u8]) -> FkpRun {
        FkpRun {
            start,
            end,
            grpprl: g.to_vec(),
        }
    }

    #[test]
    fn teste1_pages() {
        // "abc\r\r" em 0x200: dois parágrafos e um trecho de caractere sem propriedades
        let pp = write_pages(&[run(0x200, 0x204, &[]), run(0x204, 0x205, &[])]);
        assert_eq!(pp.len(), 1);
        assert_eq!(
            &pp[0][..20],
            &[2, 0, 1, 0, 0, 0, 0, 0, 4, 2, 0, 0, 5, 2, 0, 0, 0, 0, 0, 0]
        );
        assert!(pp[0][20..].iter().all(|&b| b == 0));
        let back = read_page(&pp[0], 0).unwrap();
        assert_eq!(back[0], run(0, 0x204, &[]));
        assert_eq!(back[1], run(0x204, 0x205, &[]));
    }

    #[test]
    fn f1_grpprls_are_shared_and_placed_at_the_end() {
        let g = [0x0e, 1, 0];
        let runs = [
            run(0x200, 0x207, &[]),
            run(0x207, 0x208, &g),
            run(0x208, 0x209, &[]),
            run(0x209, 0x212, &g),
            run(0x212, 0x213, &[]),
        ];
        let p = &write_pages(&runs)[0];
        assert_eq!(&p[0x1fc..], &[3, 0x0e, 1, 0]);
        let offs: Vec<u16> = (0..5).map(|k| u16_at(p, 4 + 24 + 2 * k)).collect();
        assert_eq!(offs, [0, 0x1fc, 0, 0x1fc, 0]);
        let back = read_page(p, 0).unwrap();
        assert_eq!(back[1].grpprl, g);
        assert_eq!(back[3].grpprl, g);
    }

    #[test]
    fn pages_split_when_full() {
        // 100 trechos com grpprls distintos de 5 bytes
        let runs: Vec<FkpRun> = (0..100u32)
            .map(|k| run(0x200 + k, 0x201 + k, &[0x0f, k as u8, 0, 0x04, 0xff]))
            .collect();
        let pp = write_pages(&runs);
        assert!(pp.len() > 1);
        let mut all = Vec::new();
        for (i, p) in pp.iter().enumerate() {
            let rr = read_page(p, 0).unwrap();
            if i > 0 {
                assert_eq!(rr[0].start, all.last().map(|r: &FkpRun| r.end).unwrap());
            }
            all.extend(rr);
        }
        all[0].start = 0x200;
        assert_eq!(all, runs);
    }

    #[test]
    fn bad_pages_are_errors() {
        assert!(read_page(&[1, 0], 0).is_err());
        let mut p = [0u8; PAGE];
        p[0] = 0xff;
        p[1] = 0xff;
        assert!(read_page(&p, 0).is_err());
        let mut p = write_pages(&[run(0x200, 0x201, &[0x04, 0xff])])[0];
        p[12] = 0xff; // offset do grpprl fora da página
        p[13] = 0x01;
        assert!(read_page(&p, 0).is_err());
        let mut p = write_pages(&[run(0x200, 0x201, &[0x04, 0xff])])[0];
        p[PAGE - 3] = 200; // tamanho maior que o resto da página
        assert!(read_page(&p, 0).is_err());
    }

    #[test]
    fn empty_input_gives_one_empty_page() {
        let pp = write_pages(&[]);
        assert_eq!(pp.len(), 1);
        assert!(read_page(&pp[0], 0).unwrap().is_empty());
    }
}
