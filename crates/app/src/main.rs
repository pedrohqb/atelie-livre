#![forbid(unsafe_code)]

mod audio;
mod config;
mod platform;
mod run;

use clap::Parser;

fn main() {
    let args = config::Args::parse();
    let path = config::config_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let (cfg, warnings) = config::parse_config(&text);
    for w in warnings {
        eprintln!("aviso: {w}");
    }
    let cfg = config::effective(cfg, &args);
    std::process::exit(run::run(cfg, &args));
}
