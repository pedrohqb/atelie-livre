#![forbid(unsafe_code)]

mod detect;
mod info;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "atelie-tools",
    version,
    about = "Ferramentas de dados do Ateliê Livre"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Mostra o tipo e o inventário de um arquivo.
    Info { file: PathBuf },
    /// Extrai imagens, sons e recursos de uma pasta instalada.
    Extract { input: PathBuf, output: PathBuf },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Info { file } => {
            let data = match std::fs::read(&file) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{}: {e}", file.display());
                    return ExitCode::from(1);
                }
            };
            match info::report(&data) {
                Ok(text) => {
                    print!("{text}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}: {e}", file.display());
                    ExitCode::from(1)
                }
            }
        }
        Command::Extract { .. } => {
            eprintln!("extract: não implementado");
            ExitCode::from(2)
        }
    }
}
