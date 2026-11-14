//! `atelie-tools dct`: lista e consulta os dicionários do Creative Writer
//! (`SPELL.DCT`/`THES.DCT`) ou os livres do sistema.

use clap::Subcommand;
use engine::writer::dict::{Dictionary, Variant};
use formats::dct::{SpellDct, Thesaurus, pos_name};
use std::path::Path;

#[derive(Subcommand)]
pub enum Action {
    /// Cabeçalhos e contagens dos dois arquivos originais.
    Info,
    /// Lista todas as palavras (uma por linha).
    Palavras,
    /// Verifica palavras (cada linha: OK ou ERRO e sugestões).
    Verificar { palavras: Vec<String> },
    /// Sugestões para uma palavra.
    Sugerir {
        palavra: String,
        #[arg(long, default_value_t = 10)]
        max: usize,
    },
    /// Sinônimos ("Palavras parecidas").
    Sinonimos { palavra: String },
    /// Anagramas ("Palavras embaralhadas").
    Anagramas { palavra: String },
    /// Lista as entradas do thesaurus original (título, classe, sinônimos).
    Entradas,
}

fn variant(v: &str) -> Result<Variant, String> {
    match v {
        "br" => Ok(Variant::Brasil),
        "pt" => Ok(Variant::Portugal),
        _ => Err(format!("variante {v:?}: use br ou pt")),
    }
}

fn read(data: Option<&Path>, name: &str) -> Result<Vec<u8>, String> {
    let d = data.ok_or("indique a pasta MSKIDS com --data ou ATELIE_DATA")?;
    let p = d.join("WRITER").join(name);
    std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))
}

pub fn run(action: &Action, data: Option<&Path>, free: bool, v: &str) -> Result<(), String> {
    let v = variant(v)?;
    match action {
        Action::Info => return info(data),
        Action::Entradas => {
            let t = Thesaurus::parse(&read(data, "THES.DCT")?).map_err(|e| e.to_string())?;
            for e in t.entries() {
                println!(
                    "{}\t{} ({})\t{}",
                    e.headword,
                    e.pos,
                    pos_name(e.pos),
                    e.synonyms.join(", ")
                );
            }
            return Ok(());
        }
        _ => {}
    }
    let dict = if free {
        Dictionary::free_system(v)
            .ok_or("dicionário livre pt_BR não encontrado (ATELIE_DICT, /usr/share/hunspell)")?
    } else {
        let d = data.ok_or("indique a pasta MSKIDS com --data ou ATELIE_DATA (ou use --livre)")?;
        Dictionary::from_original(d, v)?
    };
    match action {
        Action::Palavras => {
            use std::io::Write;
            let mut out = std::io::BufWriter::new(std::io::stdout().lock());
            for w in dict.words() {
                if writeln!(out, "{w}").is_err() {
                    break; // saída fechada (ex.: | head)
                }
            }
        }
        Action::Verificar { palavras } => {
            for w in palavras {
                if dict.contains(w) {
                    println!("{w}\tOK");
                } else {
                    println!("{w}\tERRO\t{}", dict.suggestions(w, 5).join(", "));
                }
            }
        }
        Action::Sugerir { palavra, max } => {
            for s in dict.suggestions(palavra, *max) {
                println!("{s}");
            }
        }
        Action::Sinonimos { palavra } => {
            for s in dict.synonyms(palavra) {
                println!("{s}");
            }
        }
        Action::Anagramas { palavra } => {
            for s in dict.anagrams(palavra) {
                println!("{s}");
            }
        }
        Action::Info | Action::Entradas => unreachable!(),
    }
    Ok(())
}

fn info(data: Option<&Path>) -> Result<(), String> {
    let s = SpellDct::parse(&read(data, "SPELL.DCT")?).map_err(|e| e.to_string())?;
    let h = &s.header;
    println!("SPELL.DCT: {}", h.notice);
    println!(
        "  idioma {} (código {}), data {}, {} blocos de {} bytes, códigos de {} bits, terminais 1..{}",
        h.language, h.language_code, h.date, h.blocks, h.block_size, h.code_bits, h.last_terminal
    );
    let by = |m: u8| s.entries().iter().filter(|w| w.variants & m != 0).count();
    println!(
        "  {} formas ({} Brasil, {} Portugal); {} equivalências fonéticas",
        s.entries().len(),
        by(formats::dct::variant::BRASIL),
        by(formats::dct::variant::PORTUGAL),
        s.phonetic.len()
    );
    if let Ok(d) = read(data, "THES.DCT") {
        let t = Thesaurus::parse(&d).map_err(|e| e.to_string())?;
        println!(
            "THES.DCT: {} entradas, {} formas no índice",
            t.entries().len(),
            t.forms().len()
        );
    }
    Ok(())
}
