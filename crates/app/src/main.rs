#![forbid(unsafe_code)]

mod config;

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
    println!("{cfg:?} browser={} frames={:?}", args.browser, args.frames);
}
