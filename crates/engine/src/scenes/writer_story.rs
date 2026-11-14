//! História do McZee no Creative Writer (tarefa T2;
//! `cw/report/building.md` §5): bloco `WHAT 4000`, álbum `4001` (máscara
//! `4009`) sobre o céu da Entrada (`3c41`), 9 falas (`[0]`, `[1]`,
//! `[4]`–`[10]`) em 7 páginas duplas.
//!
//! O `WRITER.PIC` guarda o álbum na mesma ordem do `ARTIST.PIC`, 0x200
//! abaixo (`4201` → `4001`, `420b` → `400b`…), com as mesmas medidas; as
//! posições das legendas são as do Fine Artist (conferidas nas capturas
//! `cw/shots/building_11_story0` … `13_story8`: "Nossa casa na árvore" em
//! (123,376), "A biblioteca?!" em (362,229)…). O que muda são as fotos e
//! as legendas das páginas 5–7 (o prédio com os andares, a Oficina de
//! Ideias, o McZee na biblioteca e no carrinho) e os textos.

use super::story::{Page, SPEECHES, StorySpec};
use super::writer_entry::WRITER_ENTRY;
use crate::assets::Program;

/// Bloco dos balões da história.
pub const WHAT: u16 = 0x4000;
/// Álbum aberto com o McZee espiando (495×314; máscara `4009`).
pub const ALBUM: u16 = 0x4001;

/// As 7 páginas duplas (foto da esquerda, da direita, legendas por cima,
/// em coordenadas do álbum).
pub const PAGES: [Page; 7] = [
    Page {
        left: 0x400b,
        right: 0x4011,
        // Maggie na rua, "Maggie" e "Max"
        figures: &[(0x402b, 161, 152), (0x4019, 77, 260), (0x4023, 336, 122)],
    },
    Page {
        left: 0x403b,
        right: 0x4053,
        // "Nossa casa na árvore", "Os contadores de histórias"
        figures: &[(0x4041, 47, 262), (0x404b, 278, 117)],
    },
    Page {
        left: 0x4073,
        right: 0x4059,
        // "A chegada do McZee", "Nosso primeiro encontro"
        figures: &[(0x406b, 26, 257), (0x4061, 262, 114)],
    },
    Page {
        left: 0x4093,
        right: 0x4079,
        // "Imaginópolis", "A biblioteca?!"
        figures: &[(0x408b, 42, 253), (0x4081, 286, 115)],
    },
    Page {
        // o prédio com o quadro dos andares; o McZee na biblioteca
        left: 0x40b3,
        right: 0x4099,
        // "McZee explica" dos dois lados
        figures: &[(0x40a1, 44, 261), (0x40a1, 284, 119)],
    },
    Page {
        // a Oficina de Ideias; o McZee dançando na biblioteca
        left: 0x40d3,
        right: 0x40b9,
        // "Oficina de Idéias", "McZee explica" (o `40c1`, no lugar do
        // "mais" do FA, é um ponto de 5×5 que o original não mostra)
        figures: &[(0x40cb, 32, 256), (0x40a1, 284, 119)],
    },
    Page {
        // a mesma foto da biblioteca; o McZee no carrinho
        left: 0x40f3,
        right: 0x40d9,
        // "Oficina de projetos", "Te vejo em Imaginópolis"
        figures: &[(0x40eb, 37, 259), (0x40e1, 277, 113)],
    },
];

/// Textos do `WHAT 4000` (por registro), usados sem o `WRITER.EXE`.
pub const FALLBACK: [&str; 11] = [
    "Oi, eu sou o McZee! Seja bem-vindo ao meu mundo! Você provavelmente quer saber alguma coisa sobre mim e por que motivo eu estou aqui. A minha história começa com um garoto e uma garota como você: um escritor chamado Max e uma artista plástica chamada Maggie.",
    "Eles se mudaram no mesmo dia para casas vizinhas e se tornaram amigos inseparáveis!",
    "",
    "",
    "Não demorou muito para brincarem juntos todo dia. Eles construíram uma casa na árvore e passaram muito tempo juntos ali, escrevendo e desenhando para contar suas aventuras.",
    "Um dia, quando eles estavam brincando na casa na árvore, eu entrei como um furacão. Eles não perceberam que o nosso encontro ia mudar suas vidas para sempre.",
    "Eu levei os dois para a minha casa em Imaginópolis — a Biblioteca Maluca. Logo de cara, o Max e a Maggie perceberam que aquele era um lugar um pouco diferente.",
    "A minha biblioteca era um lugar bem maluco. Grandes autores do mundo inteiro já vieram buscar inspiração aqui. Você também pode fazer isso!",
    "— Eu sou a centelha criativa. Adoro inspirar as pessoas! — disse eu.\nEntão, mostrei a eles aonde deveriam ir se estivessem sem idéias para escrever — para as minhas maravilhosas máquinas de idéias que ficam no quarto andar. Será que você consegue achá-las?",
    "Eu mostrei a eles o terceiro andar, onde os meus amigos bichos estão esperando para ajudar a fazer cartões, faixas e jornais!",
    "Eu dei a eles muitas ferramentas e idéias para criar coisas chocantes! Venha você também para o meu mundo em Imaginópolis. Explore e veja quantas ferramentas diferentes você vai encontrar para brincar!",
];

/// A história do Writer: as falas e as páginas são as mesmas do FA (`[2]` e
/// `[3]` são "blank" e ficam de fora).
pub const WRITER_STORY: StorySpec = StorySpec {
    program: Program::Writer,
    background: WRITER_ENTRY.background,
    album: ALBUM,
    what: WHAT,
    pages: &PAGES,
    speeches: &SPEECHES,
    fallback: &FALLBACK,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;
    use crate::assets::ImageRef;
    use crate::scene::{Effect, Input, Key, Scene, SceneId};
    use crate::scenes::story::{ALBUM_POS, Story};
    use crate::scenes::testutil::{assets, empty_assets};
    use crate::text_balloon::Part;

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            ctrl: false,
            alt: false,
        }
    }

    /// Sem dados: os textos reserva do Writer, os sons do `WRITER.SND` e o
    /// fim leva à Imaginópolis do Writer.
    #[test]
    fn writer_story_ends_in_writer_imaginopolis() {
        let (_d, a) = empty_assets();
        let mut s = Story::for_program(Program::Writer, None, &a);
        let mut f = Frame::new(640, 480);
        s.draw(&mut f, &a);
        let mut all = Vec::new();
        for _ in 0..SPEECHES.len() {
            all.extend(s.update(&[key(Key::Right)], 0.0, &a));
        }
        assert!(all.contains(&Effect::ChangeScene(
            SceneId::Imaginopolis.in_program(Program::Writer)
        )));
        assert!(all.iter().all(|e| !matches!(
            e,
            Effect::PlaySound(crate::assets::SoundRef::Snd(Program::Artist, _))
        )));
        // a primeira fala vem do texto reserva do Writer
        let s = Story::for_program(Program::Writer, None, &a);
        assert!(
            matches!(&s.balloon_parts()[0], Part::Text(t) if t.contains("um escritor chamado Max"))
        );
    }

    #[test]
    fn fallback_texts_skip_the_blank_records() {
        for (k, _) in SPEECHES {
            assert!(!FALLBACK[k].is_empty(), "registro {k}");
        }
        assert!(FALLBACK[6].contains("Biblioteca Maluca"));
    }

    /// Com os dados reais: as figuras existem no `WRITER.PIC` com as medidas
    /// das do Fine Artist (fotos 169×119 ou 168×118), os textos vêm do `4000`
    /// e o desenho bate com a captura do original na foto da página 5.
    #[test]
    fn writer_album_with_real_data() {
        let Some(a) = assets() else { return };
        let ids = a.pic_ids(Program::Writer);
        for p in &PAGES {
            for id in [p.left, p.right] {
                assert!(ids.binary_search(&id).is_ok(), "{id:04x}");
                let img = a.image(&ImageRef::Pic(Program::Writer, id));
                assert!(img.width >= 168 && img.height >= 118, "{id:04x}");
            }
            for &(id, ..) in p.figures {
                assert!(ids.binary_search(&id).is_ok(), "{id:04x}");
            }
        }
        let album = a.image(&ImageRef::Pic(Program::Writer, ALBUM));
        assert_eq!((album.width, album.height), (495, 314));
        let mut s = Story::for_program(Program::Writer, None, &a);
        assert!(
            matches!(&s.balloon_parts()[0], Part::Text(t) if t.starts_with("Oi, eu sou o McZee!"))
        );
        for _ in 0..5 {
            s.update(&[key(Key::Right)], 0.0, &a);
        }
        assert_eq!(s.spread(), 4);
        assert!(
            matches!(&s.balloon_parts()[0], Part::Text(t) if t.starts_with("A minha biblioteca"))
        );
        let mut f = Frame::new(640, 480);
        s.draw(&mut f, &a);
        crate::scenes::testutil::dump(&f, "writer_story_5");
        // a foto da esquerda é o prédio com os andares (`40b3`), no lugar
        let (ax, ay) = ALBUM_POS;
        let photo = a.image(&ImageRef::Pic(Program::Writer, 0x40b3));
        let (px, py) = (ax + 37 + 60, ay + 122 + 30);
        let k = ((30 * photo.width + 60) * 4) as usize;
        assert_eq!(f.pixel(px as u32, py as u32)[..3], photo.rgba[k..k + 3]);
    }
}
