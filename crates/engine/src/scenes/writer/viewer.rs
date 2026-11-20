//! Visor de páginas (`office-ferramentas.md` §8): página 1 com Página
//! inteira, Aumentar visão, MENOS ZOOM, a régua de zoom com o bichinho e
//! MAIS ZOOM; página 2 com Visão normal, Duas páginas, Todas as páginas e
//! Lado a lado. Cada botão troca a vista do editor ([`Editor::set_view`];
//! geometria medida em `view.rs`), toca `0233` e abre o seu balão; o
//! bichinho `092c` anda sobre a régua conforme o zoom.

use super::editor::Editor;
use super::tools::{
    Cx, Pic, SND_OPTION, SND_SWITCH, SWITCH, SWITCH_RECT, StripButton, ToolId, ToolStrip,
    WHAT_VIEWER, inside, strip_button,
};
use super::view::{Arrange, NORMAL_LEVEL, View, Zoom};
use crate::Frame;
use crate::assets::Assets;

const PAGE1: [StripButton; 4] = [
    strip_button(
        0x08a9,
        None,
        (35, 58, 60, 39),
        (WHAT_VIEWER, 5),
        "Página inteira",
    ),
    strip_button(
        0x08ad,
        None,
        (122, 58, 60, 39),
        (WHAT_VIEWER, 6),
        "Aumentar visão",
    ),
    strip_button(
        0x0891,
        Some(0x0893),
        (209, 64, 61, 28),
        (WHAT_VIEWER, 7),
        "Menos zoom",
    ),
    strip_button(
        0x0895,
        Some(0x0897),
        (434, 63, 44, 29),
        (WHAT_VIEWER, 8),
        "Mais zoom",
    ),
];

/// Régua de zoom ("a a a a a a") e o bichinho que marca o nível (y 57; o
/// x vem de [`View::bug_x`]).
const RULER: Pic = Pic::opaque(0x0885, 278, 73);
const BUG: Pic = Pic::masked(0x092c, 0x092d, 364, 57);

const PAGE2: [StripButton; 4] = [
    strip_button(
        0x0899,
        Some(0x089b),
        (55, 58, 66, 39),
        (WHAT_VIEWER, 1),
        "Visão normal",
    ),
    strip_button(
        0x089d,
        None,
        (168, 58, 66, 39),
        (WHAT_VIEWER, 2),
        "Duas páginas",
    ),
    strip_button(
        0x08a1,
        None,
        (281, 58, 66, 39),
        (WHAT_VIEWER, 3),
        "Todas as páginas",
    ),
    strip_button(
        0x08a5,
        None,
        (394, 58, 66, 39),
        (WHAT_VIEWER, 4),
        "Lado a lado",
    ),
];

#[derive(Default)]
pub struct ViewerStrip {
    more: bool,
}

/// A vista depois do botão `k` da página `more` da faixa.
fn after(mut v: View, scale: f64, more: bool, k: usize) -> View {
    v.overview = false;
    if more {
        v.arrange = [Arrange::Normal, Arrange::Two, Arrange::All, Arrange::Side][k];
        if k == 0 {
            // "com tamanho e largura totais"
            v.zoom = Zoom::Level(NORMAL_LEVEL);
        }
        return v;
    }
    if k < 2 && matches!(v.arrange, Arrange::All | Arrange::Side) {
        v.arrange = Arrange::Normal;
    }
    match k {
        0 => v.zoom = Zoom::FitPage,
        1 => v.zoom = Zoom::FitWidth,
        _ => {
            v.step_zoom(scale, k == 3);
        }
    }
    v
}

impl ToolStrip for ViewerStrip {
    fn id(&self) -> ToolId {
        ToolId::Viewer
    }

    fn press(&mut self, ed: &mut Editor, cx: &mut Cx, x: i32, y: i32) -> bool {
        if inside(SWITCH_RECT, x, y) {
            cx.sound(SND_SWITCH);
            self.more = !self.more;
            return true;
        }
        let buttons = if self.more { &PAGE2 } else { &PAGE1 };
        let Some(k) = StripButton::find(buttons, x, y) else {
            return false;
        };
        cx.sound(SND_OPTION);
        if let Some(h) = buttons[k].help {
            cx.help(h);
        }
        let v = after(ed.view, ed.scale(), self.more, k);
        ed.set_view(v);
        true
    }

    fn draw_strip(&self, ed: &Editor, f: &mut Frame, a: &Assets) {
        if self.more {
            StripButton::draw_all(&PAGE2, f, a);
        } else {
            StripButton::draw_all(&PAGE1, f, a);
            RULER.draw(f, a);
            BUG.draw_at(f, a, View::bug_x(ed.scale()), BUG.y);
        }
        SWITCH.draw(f, a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::testutil::empty_assets;
    use crate::scenes::writer::tools::{WHAT_VIEWER, snd};

    fn press(
        s: &mut ViewerStrip,
        ed: &mut Editor,
        a: &Assets,
        x: i32,
        y: i32,
    ) -> Vec<crate::scene::Effect> {
        let mut fx = Vec::new();
        let mut cx = Cx::new(&mut fx, a);
        assert!(s.press(ed, &mut cx, x, y));
        let _ = cx.take_help();
        fx
    }

    #[test]
    fn zoom_buttons_move_the_bug_and_change_the_view() {
        let (_d, a) = empty_assets();
        let mut ed = Editor::blank(None, &a);
        let mut s = ViewerStrip::default();
        assert_eq!(View::bug_x(ed.scale()), 364);
        let fx = press(&mut s, &mut ed, &a, 220, 70);
        assert!(fx.contains(&snd(SND_OPTION)));
        assert_eq!(ed.view.zoom, Zoom::Level(4));
        assert_eq!(View::bug_x(ed.scale()), 347);
        press(&mut s, &mut ed, &a, 440, 70);
        press(&mut s, &mut ed, &a, 440, 70);
        assert_eq!(ed.view.zoom, Zoom::Level(6));
        // Página inteira: 44 %, a folha inteira na vista
        press(&mut s, &mut ed, &a, 40, 70);
        assert_eq!(ed.view.zoom, Zoom::FitPage);
        assert!((ed.scale() - 350.0 / 792.0).abs() < 1e-9);
        // Aumentar visão volta a encher a largura
        press(&mut s, &mut ed, &a, 130, 70);
        assert_eq!(ed.scale(), 1.0);
    }

    #[test]
    fn second_page_picks_the_arrangement() {
        let (_d, a) = empty_assets();
        let mut ed = Editor::blank(None, &a);
        let mut s = ViewerStrip::default();
        press(&mut s, &mut ed, &a, 515, 70);
        press(&mut s, &mut ed, &a, 290, 70);
        assert_eq!(ed.view.arrange, Arrange::All);
        press(&mut s, &mut ed, &a, 400, 70);
        assert_eq!(ed.view.arrange, Arrange::Side);
        press(&mut s, &mut ed, &a, 170, 70);
        assert_eq!(ed.view.arrange, Arrange::Two);
        press(&mut s, &mut ed, &a, 60, 70);
        assert!(ed.view.is_normal());
        let _ = WHAT_VIEWER;
    }

    #[test]
    fn clicks_on_a_zoomed_page_reach_the_text() {
        let (_d, a) = empty_assets();
        let mut ed = Editor::blank(None, &a);
        ed.type_text("abc");
        ed.set_view(View {
            arrange: Arrange::Normal,
            zoom: Zoom::Level(2),
            overview: false,
        });
        // 50 %: folha em x 171, texto na margem de 72 pt = 36 px
        let (x, y) = ed.to_screen(0, 72, 72);
        assert_eq!((x, y), (171 + 36, 107 + 36));
        let p = ed.to_page(x + 1, y + 1).unwrap();
        assert_eq!((p.page, p.x, p.y), (0, 74, 74));
    }
}
