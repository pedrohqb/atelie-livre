//! Desenha um balão do port sobre uma imagem de fundo (em geral a captura do
//! original na mesma tela) e grava o resultado, para comparar pixel a pixel
//! com o original.
//!
//! ```text
//! balloon_shot DADOS FUNDO.png SAIDA.png TIPO BLOCO REGISTRO X Y [opções]
//! ```
//!
//! `TIPO`: `help` (ajuda da pintura), `sub` (ligação ao lado, cantos retos),
//! `balloon` (balão de fala), `text` (cadastro e avisos), `alert` (aviso com
//! cantos retos). Opções: `tail=X,Y` (ponta do rabicho), `name=Nome`,
//! `buttons=bsn` (Volta, Para, Segue), `select=K` (bicho marcado).

use engine::assets::{Assets, Program};
use engine::balloon::Balloon;
use engine::help::HelpBalloon;
use engine::text_balloon::{Button, TextBalloon};
use engine::{Frame, Image};
use std::path::Path;
use std::rc::Rc;

fn load_png(path: &Path) -> Image {
    let mut dec = png::Decoder::new(std::fs::File::open(path).expect("fundo"));
    dec.set_transformations(png::Transformations::EXPAND);
    let mut reader = dec.read_info().expect("png");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("png");
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb | png::ColorType::Indexed => buf[..info.buffer_size()]
            .chunks(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => panic!("cor {other:?}"),
    };
    Image {
        width: info.width,
        height: info.height,
        rgba,
    }
}

fn save_png(f: &Frame, path: &Path) {
    let file = std::fs::File::create(path).expect("saída");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), f.width(), f.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png")
        .write_image_data(f.pixels())
        .expect("png");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 9 {
        eprintln!("uso: balloon_shot DADOS FUNDO SAIDA TIPO BLOCO REGISTRO X Y [opções]");
        std::process::exit(2);
    }
    let a = Assets::open(Path::new(&args[1])).expect("dados");
    let mut f = Frame::new(640, 480);
    f.blit(&load_png(Path::new(&args[2])), 0, 0);
    let kind = args[4].as_str();
    let block = u16::from_str_radix(&args[5], 16).expect("bloco hex");
    let k: usize = args[6].parse().expect("registro");
    let (x, y): (i32, i32) = (args[7].parse().unwrap(), args[8].parse().unwrap());
    let mut tail = None;
    let mut name = String::from("Ana");
    let mut buttons = Vec::new();
    let mut select = 0;
    for o in &args[9..] {
        if let Some(t) = o.strip_prefix("tail=") {
            let (tx, ty) = t.split_once(',').expect("tail=X,Y");
            tail = Some((tx.parse().unwrap(), ty.parse().unwrap()));
        } else if let Some(n) = o.strip_prefix("name=") {
            name = n.to_string();
        } else if let Some(b) = o.strip_prefix("buttons=") {
            for c in b.chars() {
                buttons.push(match c {
                    'b' => Button::Back,
                    's' => Button::Stop,
                    _ => Button::Next,
                });
            }
        } else if let Some(s) = o.strip_prefix("select=") {
            select = s.parse().unwrap();
        }
    }
    let records = a.dialogs(Program::Artist, block);
    match kind {
        "help" | "sub" => {
            let mut h = HelpBalloon::new(Rc::new(records), block, k, x, y).expect("registro");
            h.x = x;
            h.tail = if kind == "help" { tail } else { None };
            h.draw(&mut f, &a);
        }
        "balloon" => {
            let mut b = Balloon::from_block(&records, k, x, y, &name).expect("registro");
            b.tail = tail;
            b.draw(&mut f, &a);
        }
        _ => {
            let mut b = TextBalloon::from_record(&records[k], x, y, &name, buttons);
            b.selected = select;
            if kind == "alert" {
                b = b.as_alert();
            }
            b.draw(&mut f, &a);
        }
    }
    save_png(&f, Path::new(&args[3]));
}
