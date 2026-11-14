//! Balão de ajuda do McZee na tela de pintura: um registro `WHAT` com a
//! formatação do original (ver [`crate::rich`]) — negrito, cores, recuo,
//! figuras embutidas (ícone da ferramenta) e ligações clicáveis (gema da
//! dica `21b`, chave da tecla de atalho `21c`, Maggie `21e`, palavra do
//! glossário `ffff`) que abrem o registro ligado no lugar ou ao lado. O
//! balão principal (com rabicho até o McZee) tem os cantos arredondados; os
//! das ligações, ao lado, são retos, como no original.

use crate::Frame;
use crate::assets::{Assets, Program};
use crate::balloon::{BOX_EXTRA, PAD, WRAP_LESS, draw_frame_in, draw_tail_box_in, pad_bottom};
use crate::rich::{self, Ctx, Laid};
use formats::fnt::Font;
use formats::what::Record;
use std::rc::Rc;

/// Centro da tela, onde se abrem as dicas.
const SCREEN_CENTER: (i32, i32) = (320, 240);

#[derive(Clone)]
pub struct HelpBalloon {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    /// Bloco `WHAT` e índice do registro mostrado.
    pub block: u16,
    pub index: usize,
    records: Rc<Vec<Record>>,
    /// Ponta do rabicho (quem fala), acima do balão; `None` = sem rabicho.
    pub tail: Option<(i32, i32)>,
    /// Coluna da borda direita, se o balão se alinha por ela (os da
    /// pintura, no original, encostam em x = 617 qualquer que seja a largura).
    pub right: Option<i32>,
    /// Centralizado na tela (as dicas das gemas, da chave e do glossário).
    pub centered: bool,
    /// Programa dos registros: de que tabela e de que `.PIC` vêm as figuras
    /// embutidas.
    pub program: Program,
}

impl HelpBalloon {
    /// Balão do registro `index` do bloco; `None` se o registro não existe
    /// ou está vazio. O balão fica em `(x, y)`, puxado para dentro da tela.
    pub fn new(records: Rc<Vec<Record>>, block: u16, index: usize, x: i32, y: i32) -> Option<Self> {
        let r = records.get(index)?;
        if r.plain().trim().is_empty() {
            return None;
        }
        let w = (r.width as i32).clamp(120, 400);
        Some(HelpBalloon {
            x: x.min(637 - w - BOX_EXTRA),
            y,
            w,
            block,
            index,
            records,
            tail: None,
            right: None,
            centered: false,
            program: Program::Artist,
        })
    }

    /// Mesmo balão, com as figuras embutidas de outro programa (o Writer).
    pub fn in_program(mut self, program: Program) -> HelpBalloon {
        self.program = program;
        self
    }

    /// O mesmo balão com a borda direita (inclusive) na coluna `right`.
    pub fn anchored_right(mut self, right: i32) -> HelpBalloon {
        self.right = Some(right);
        self.x = right + 1 - self.w - BOX_EXTRA;
        self
    }

    /// Mesmo balão com outro registro do bloco (ligação), se existir.
    pub fn follow(&self, index: usize) -> Option<HelpBalloon> {
        HelpBalloon::new(self.records.clone(), self.block, index, self.x, self.y).map(|mut h| {
            h.tail = self.tail;
            h.program = self.program;
            match self.right {
                Some(r) => h.anchored_right(r),
                None => h,
            }
        })
    }

    /// Balão de uma ligação (gema, tecla, glossário), que se abre por cima
    /// enquanto este continua aberto: centralizado na tela, como no
    /// original (medido em quatro dicas da pintura).
    pub fn follow_beside(&self, index: usize) -> Option<HelpBalloon> {
        let mut b = self.follow(index)?;
        // o rabicho é do balão principal (quem fala)
        b.tail = None;
        b.right = None;
        b.centered = true;
        b.x = SCREEN_CENTER.0 - (b.w + BOX_EXTRA) / 2;
        Some(b)
    }

    /// Identidade do balão (bloco, registro e lugar).
    pub fn key(&self) -> (u16, usize, i32, i32) {
        (self.block, self.index, self.x, self.y)
    }

    pub fn record(&self) -> &Record {
        &self.records[self.index]
    }

    pub fn title(&self) -> &str {
        &self.records[self.index].title
    }

    fn font(a: &Assets) -> Option<Rc<Font>> {
        a.font(8)
    }

    fn layout(&self, a: &Assets) -> Laid {
        let font = Self::font(a);
        let records = &self.records;
        let glossary = |k: usize| records.get(k).map(|r| r.title.trim().to_string());
        let ctx = Ctx {
            assets: a,
            program: self.program,
            font: font.as_deref(),
            name: "",
            glossary: &glossary,
            own: Some(self.index),
        };
        rich::layout(&self.record().items, self.w - WRAP_LESS, &ctx)
    }

    pub fn height(&self, a: &Assets) -> i32 {
        PAD + self.layout(a).height + pad_bottom(self.program)
    }

    /// Posição de tela do balão, puxado para cima se não couber.
    fn origin(&self, a: &Assets) -> (i32, i32, i32) {
        let h = self.height(a);
        if self.centered {
            return (self.x, (SCREEN_CENTER.1 - h / 2).max(5), h);
        }
        (self.x, self.y.min(455 - h).max(5), h)
    }

    pub fn contains(&self, a: &Assets, x: i32, y: i32) -> bool {
        let (bx, by, h) = self.origin(a);
        rich::inside((bx, by, self.w + BOX_EXTRA, h), x, y)
    }

    /// Registro ligado sob `(x, y)`, se o clique caiu numa ligação.
    pub fn link_at(&self, a: &Assets, x: i32, y: i32) -> Option<usize> {
        let (bx, by, _) = self.origin(a);
        self.layout(a)
            .hot_at(x - bx - PAD, y - by - PAD)
            .map(rich::Hot::target)
            .filter(|t| *t != self.index)
    }

    pub fn draw(&self, f: &mut Frame, a: &Assets) {
        let font = Self::font(a);
        let lay = self.layout(a);
        let (bx, by, h) = self.origin(a);
        let bw = self.w + BOX_EXTRA;
        let p = self.program;
        draw_frame_in(f, p, (bx, by, bw, h), self.tail.is_some());
        if let Some(tip) = self.tail.filter(|t| t.1 < by - 4) {
            draw_tail_box_in(f, p, (bx, by, bw, h), tip);
        }
        rich::draw(
            f,
            a,
            font.as_deref(),
            &lay,
            (bx + PAD, by + PAD),
            None,
            &mut |_, _, _| {},
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;
    use formats::what::{Item, PIC_GLOSSARY, PIC_TIP};

    fn t(text: &str, bold: bool) -> Item {
        Item::Text {
            text: text.into(),
            bold,
            color: 0xf,
            indent: 0,
            key: false,
            signature: false,
        }
    }

    fn records() -> Rc<Vec<Record>> {
        Rc::new(vec![
            Record {
                title: "Lata de tinta".into(),
                width: 150,
                height: 100,
                style: 2,
                items: vec![
                    Item::Picture {
                        id: 0x1001,
                        param: 0,
                    },
                    t("Lata de tinta", true),
                    Item::Newline,
                    t("Preencha uma", false),
                    Item::Picture {
                        id: PIC_GLOSSARY,
                        param: 2,
                    },
                    t("com cores e padrões que vão além.", false),
                    Item::Newline,
                    Item::Picture {
                        id: PIC_TIP,
                        param: 1 | 0x8000,
                    },
                    t("Sem quebras", false),
                ],
            },
            Record {
                title: "Sem quebras".into(),
                width: 100,
                height: 40,
                style: 2,
                items: vec![t("Dica.", false)],
            },
            Record {
                title: "área".into(),
                width: 100,
                height: 40,
                style: 2,
                items: vec![t("Uma região.", false)],
            },
            Record {
                title: " ".into(),
                width: 200,
                height: 1,
                style: 2,
                items: vec![],
            },
        ])
    }

    #[test]
    fn empty_record_is_none_and_balloon_stays_on_screen() {
        assert!(HelpBalloon::new(records(), 0x1003, 3, 445, 115).is_none());
        assert!(HelpBalloon::new(records(), 0x1003, 9, 445, 115).is_none());
        let b = HelpBalloon::new(records(), 0x1003, 0, 600, 115).unwrap();
        assert_eq!(b.x + b.w, 635);
        assert_eq!(b.title(), "Lata de tinta");
    }

    #[test]
    fn layout_wraps_links_and_draws_bold() {
        let (_d, a) = empty_assets();
        let b = HelpBalloon::new(records(), 0x1003, 0, 445, 115).unwrap();
        let lay = b.layout(&a);
        // sem .PIC a figura 1001 some; texto quebra em várias linhas
        assert!(
            !lay.runs
                .iter()
                .any(|p| matches!(p, rich::Run::Picture { .. }))
        );
        assert!(lay.height > 40);
        let ys: std::collections::BTreeSet<i32> = lay
            .runs
            .iter()
            .filter_map(|p| match p {
                rich::Run::Text { y, .. } => Some(*y),
                _ => None,
            })
            .collect();
        assert!(ys.len() >= 4, "{ys:?}");
        // ligações: a palavra do glossário e a dica (ícone + texto)
        let target = |t: usize| lay.hots.iter().find(|(_, h)| h.target() == t);
        assert!(target(2).is_some());
        assert!(target(1).is_some());
        let (r, _) = target(2).unwrap();
        let (bx, by, _) = b.origin(&a);
        assert_eq!(
            b.link_at(&a, bx + PAD + r.0 + 3, by + PAD + r.1 + 1),
            Some(2)
        );
        assert_eq!(b.link_at(&a, bx + 1, by + 1), None);
        assert!(b.contains(&a, bx + 1, by + 1));
        assert!(!b.contains(&a, bx - 1, by));
        let next = b.follow(2).unwrap();
        assert_eq!(next.title(), "área");
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        // sem rabicho: canto reto, borda preta
        assert_eq!(f.pixel(bx as u32 + 1, by as u32 + 1)[..3], [0, 0, 0]);
    }

    #[test]
    fn writer_balloons_take_their_figures_from_the_writer_pic() {
        let Some(a) = crate::scenes::testutil::assets() else {
            return;
        };
        // Seleciona-adesivos: ícone da ferramenta, gemas e o Cavalete
        let recs = Rc::new(a.dialogs(Program::Writer, 0x0fa1));
        let b = HelpBalloon::new(recs, 0x0fa1, 0, 0, 115)
            .unwrap()
            .in_program(Program::Writer);
        let pics = |b: &HelpBalloon| {
            b.layout(&a)
                .runs
                .iter()
                .filter_map(|r| match r {
                    rich::Run::Picture { id, .. } => Some(*id),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(pics(&b), [0x225, 0x21b, 0x80a, 0x21b]);
        assert_eq!(b.layout(&a).program, Program::Writer);
        // a ligação continua no Writer
        let tip = b.follow_beside(1).unwrap();
        assert_eq!(tip.program, Program::Writer);
        // o Cavalete (43×41) sai apagado, como no original
        let lay = b.layout(&a);
        let (ex, ey) = lay
            .runs
            .iter()
            .find_map(|r| match r {
                rich::Run::Picture {
                    id: 0x80a, x, y, ..
                } => Some((*x, *y)),
                _ => None,
            })
            .unwrap();
        let mut f = Frame::new(640, 480);
        b.draw(&mut f, &a);
        let (bx, by, _) = b.origin(&a);
        let (ox, oy) = (bx + PAD + ex, by + PAD + ey);
        let grey = (0..41)
            .flat_map(|dy| (0..43).map(move |dx| (dx, dy)))
            .filter(|(dx, dy)| f.pixel((ox + dx) as u32, (oy + dy) as u32)[..3] == [100, 100, 100])
            .count();
        assert!(grey > 100, "{grey}");
    }

    #[test]
    fn writer_balloons_are_one_pixel_taller_at_the_bottom() {
        // medido nas 11 capturas do Escritório: o mesmo texto, a borda de
        // baixo 1 px mais longe
        let (_d, a) = empty_assets();
        let b = HelpBalloon::new(records(), 0x1003, 0, 445, 115).unwrap();
        let w = b.clone().in_program(Program::Writer);
        assert_eq!(w.height(&a), b.height(&a) + 1);
    }
}
