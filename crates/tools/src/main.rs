#![forbid(unsafe_code)]

mod detect;
mod extract;
mod html;
mod info;
mod mag;

mod report;

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
    /// Converte um desenho .MAG do Fine Artist em PNG.
    Mag2png { input: PathBuf, output: PathBuf },
    /// Converte um PNG (de preferência 594×347) em desenho .MAG.
    Png2mag {
        input: PathBuf,
        output: PathBuf,
        /// Título do desenho.
        #[arg(long, default_value = "Quadro Novo")]
        title: String,
        /// Nome do autor.
        #[arg(long, default_value = "")]
        author: String,
        /// Pasta MSKIDS, para usar a paleta do ARTIST.EXE (padrão: ATELIE_DATA).
        #[arg(long, env = "ATELIE_DATA")]
        data: Option<PathBuf>,
    },
}

fn simple(r: Result<(), String>) -> ExitCode {
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
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
        Command::Extract { input, output } => match extract::run(&input, &output) {
            Ok(report) => {
                let code = report.exit_code();
                eprintln!(
                    "{} arquivo(s) processado(s), {} falha(s), {} ausente(s); relatório em {}",
                    report.processed.len(),
                    report.failures.len(),
                    report.missing.len(),
                    output.join("report.json").display()
                );
                if code == 2 {
                    eprintln!(
                        "{}: a pasta não parece uma instalação compatível",
                        input.display()
                    );
                }
                ExitCode::from(code)
            }
            Err(e) => {
                eprintln!("erro de E/S: {e}");
                ExitCode::from(1)
            }
        },
        Command::Mag2png { input, output } => simple(mag::mag2png(&input, &output)),
        Command::Png2mag {
            input,
            output,
            title,
            author,
            data,
        } => simple(mag::png2mag(
            &input,
            &output,
            &title,
            &author,
            data.as_deref(),
        )),
    }
}
