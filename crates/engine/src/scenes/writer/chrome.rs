//! Moldura do Escritório: bordas, faixa, página, barra de baixo e a barra
//! de ferramentas com o McZee e a porta.

use super::*;

impl Writer {
    pub(super) fn draw_chrome(&self, f: &mut Frame, a: &Assets) {
        f.fill(DARK);
        // bordas laranja em relevo, como na pintura
        for y0 in [0, 104, 456, 475] {
            for (dy, c) in [(0, BLACK), (1, ORANGE), (2, ORANGE), (3, BROWN), (4, BLACK)] {
                f.rect(0, y0 + dy, 640, 1, c);
            }
        }
        for (x, c) in [(0, BLACK), (1, BROWN), (2, ORANGE), (3, ORANGE), (4, BLACK)] {
            f.rect(x, 0, 1, 480, c);
        }
        for (x, c) in [
            (635, BLACK),
            (636, BROWN),
            (637, ORANGE),
            (638, ORANGE),
            (639, BLACK),
        ] {
            f.rect(x, 0, 1, 480, c);
        }
        // faixa de opções e linha sob a barra de ferramentas
        f.rect(5, 56, 630, 48, LIGHT);
        f.rect(5, 55, 630, 1, BLACK);
        // nicho escuro do McZee
        f.rect(532, 55, 1, 49, BLACK);
        f.rect(533, 56, 47, 48, DARK);
        // página branca com borda preta de 1 px por fora
        f.rect(PAGE.0 - 1, PAGE.1, 1, PAGE.3, BLACK);
        f.rect(PAGE.0 + PAGE.2, PAGE.1, 1, PAGE.3, BLACK);
        f.rect(PAGE.0, PAGE.1, PAGE.2, PAGE.3, WHITE);
        // barra inferior
        for (x0, x1) in [(22, 137), (155, 486), (518, 616)] {
            f.rect(x0, 461, 1, 14, BLACK);
            f.rect(x1, 461, 1, 14, BLACK);
            f.rect(x0 + 1, 461, x1 - x0 - 1, 14, WHITE);
        }
        let pages = format!("Pág. {}", self.page + 1);
        match a.font(8) {
            Some(font) => {
                f.text_font(&font, 69, 461, "Escritório", BLACK);
                let name = "Documento Novo";
                let w = font.width(name) as i32;
                f.text_font(&font, 320 - w / 2, 462, name, BLACK);
                f.text_font(&font, 537, 462, &pages, BLACK);
            }
            None => {
                f.text(44, 464, "Escritório", BLACK);
                f.text(264, 464, "Documento Novo", BLACK);
                f.text(536, 464, &pages, BLACK);
            }
        }
        f.blit(&a.image(&pic(0x03c3)), 27, 463);
        f.blit(&a.image(&pic(0x03c3)), 523, 463);
        f.blit_masked(
            &a.image(&pic(0x02db)),
            &a.image(&pic(0x02dd)),
            PAGE_PREV.0,
            PAGE_PREV.1,
        );
        f.blit_masked(
            &a.image(&pic(0x02df)),
            &a.image(&pic(0x02e1)),
            PAGE_NEXT.0,
            PAGE_NEXT.1,
        );
    }

    pub(super) fn draw_tools(&self, f: &mut Frame, a: &Assets) {
        for t in &TOOLS {
            let active = t.action == Action::Select(self.tool);
            let (img, mask) = if active { t.active } else { t.normal };
            if active {
                let w = a.image(&pic(img)).width as i32;
                f.rect(t.x - 5, 5, w + 11, 51, LIGHT);
                f.rect(t.x - 6, 5, 1, 50, BLACK);
                f.rect(t.x + w + 6, 5, 1, 50, BLACK);
            }
            f.blit_masked(&a.image(&pic(img)), &a.image(&pic(mask)), t.x, t.y);
        }
        f.blit_masked(&a.image(&pic(0x0235)), &a.image(&pic(0x0237)), 536, 47);
        f.blit_masked(&a.image(&pic(0x023d)), &a.image(&pic(0x023f)), 581, 16);
    }
}
