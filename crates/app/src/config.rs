//! Configuração: `config.toml` + argumentos da linha de comando.

use engine::assets::{Program, SoundRef};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Scale {
    /// Maior fator inteiro que cabe, pixels nítidos.
    Integer,
    /// Preenche mantendo 4:3, com filtro suave.
    Fit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub data_dir: Option<PathBuf>,
    pub scale: Scale,
    pub fullscreen: bool,
    pub volume: u8,
    pub click_sound: Option<SoundRef>,
    pub dev: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            data_dir: None,
            scale: Scale::Integer,
            fullscreen: true,
            volume: 80,
            click_sound: None,
            dev: false,
        }
    }
}

#[derive(Debug, clap::Parser)]
#[command(name = "atelie-livre", version, about = "Ateliê Livre")]
pub struct Args {
    /// Pasta MSKIDS com os arquivos originais.
    #[arg(long)]
    pub data: Option<PathBuf>,
    /// Abre em janela em vez de tela cheia.
    #[arg(long)]
    pub window: bool,
    /// Modo de ampliação.
    #[arg(long, value_enum)]
    pub scale: Option<Scale>,
    /// Começa no navegador de recursos.
    #[arg(long)]
    pub browser: bool,
    /// Encerra após N quadros (testes).
    #[arg(long, hide = true)]
    pub frames: Option<u32>,
}

pub fn config_path_from(xdg: Option<PathBuf>, home: Option<PathBuf>) -> PathBuf {
    let base = xdg.unwrap_or_else(|| home.unwrap_or_default().join(".config"));
    base.join("atelie-livre").join("config.toml")
}

pub fn config_path() -> PathBuf {
    let env = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    config_path_from(env("XDG_CONFIG_HOME"), env("HOME"))
}

/// Lê o TOML; problemas viram avisos e valores padrão, nunca erro.
pub fn parse_config(text: &str) -> (Config, Vec<String>) {
    let mut cfg = Config::default();
    let mut warnings = Vec::new();
    let table = match text.parse::<toml::Table>() {
        Ok(t) => t,
        Err(e) => {
            warnings.push(format!(
                "config.toml inválido, usando padrões: {}",
                e.message()
            ));
            return (cfg, warnings);
        }
    };
    for (key, value) in &table {
        let ok = match key.as_str() {
            "data_dir" => value
                .as_str()
                .map(|s| cfg.data_dir = Some(PathBuf::from(s)))
                .is_some(),
            "scale" => match value.as_str() {
                Some("integer") => {
                    cfg.scale = Scale::Integer;
                    true
                }
                Some("fit") => {
                    cfg.scale = Scale::Fit;
                    true
                }
                _ => false,
            },
            "fullscreen" => value.as_bool().map(|b| cfg.fullscreen = b).is_some(),
            "dev" => value.as_bool().map(|b| cfg.dev = b).is_some(),
            "volume" => value
                .as_integer()
                .map(|v| cfg.volume = v.clamp(0, 100) as u8)
                .is_some(),
            "click_sound" => match parse_sound(value) {
                Some(s) => {
                    cfg.click_sound = Some(s);
                    true
                }
                None => false,
            },
            _ => {
                warnings.push(format!(
                    "config.toml: chave desconhecida \"{key}\" ignorada"
                ));
                continue;
            }
        };
        if !ok {
            warnings.push(format!(
                "config.toml: valor inválido para \"{key}\", usando o padrão"
            ));
        }
    }
    (cfg, warnings)
}

fn parse_sound(v: &toml::Value) -> Option<SoundRef> {
    let t = v.as_table()?;
    let program = match t.get("program")?.as_str()? {
        "artist" => Program::Artist,
        "writer" => Program::Writer,
        _ => return None,
    };
    let id = u16::try_from(t.get("id")?.as_integer()?).ok()?;
    Some(SoundRef::Snd(program, id))
}

/// Troca um `~` inicial pela pasta pessoal (o arquivo de configuração não passa pelo shell).
pub fn expand_home(path: PathBuf, home: Option<PathBuf>) -> PathBuf {
    let Some(home) = home else { return path };
    match path.strip_prefix("~") {
        Ok(rest) if path.starts_with("~") => home.join(rest),
        _ => path,
    }
}

/// Argumentos têm prioridade sobre o arquivo.
pub fn effective(mut cfg: Config, args: &Args) -> Config {
    if let Some(d) = &args.data {
        cfg.data_dir = Some(d.clone());
    }
    if args.window {
        cfg.fullscreen = false;
    }
    if let Some(s) = args.scale {
        cfg.scale = s;
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use engine::assets::{Program, SoundRef};

    #[test]
    fn empty_file_gives_defaults() {
        let (c, w) = parse_config("");
        assert_eq!(c, Config::default());
        assert!(w.is_empty());
        assert_eq!(c.scale, Scale::Integer);
        assert!(c.fullscreen);
        assert_eq!(c.volume, 80);
    }

    #[test]
    fn invalid_toml_gives_defaults_and_warning() {
        let (c, w) = parse_config("scale = = integer");
        assert_eq!(c, Config::default());
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn unknown_key_warns_but_keeps_rest() {
        let (c, w) = parse_config("fullscreen = false\ncores = 3\n");
        assert!(!c.fullscreen);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("cores"));
    }

    #[test]
    fn wrong_type_warns_and_uses_default_for_that_key() {
        let (c, w) = parse_config("volume = \"alto\"\nfullscreen = false\n");
        assert_eq!(c.volume, 80);
        assert!(!c.fullscreen);
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn volume_is_clamped() {
        assert_eq!(parse_config("volume = 250").0.volume, 100);
        assert_eq!(parse_config("volume = -5").0.volume, 0);
    }

    #[test]
    fn click_sound_and_others_parse() {
        let text = "data_dir = \"/dados/MSKIDS\"\nscale = \"fit\"\ndev = true\n\
                    click_sound = { program = \"artist\", id = 0x209 }\n";
        let (c, w) = parse_config(text);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(c.data_dir, Some(PathBuf::from("/dados/MSKIDS")));
        assert_eq!(c.scale, Scale::Fit);
        assert!(c.dev);
        assert_eq!(c.click_sound, Some(SoundRef::Snd(Program::Artist, 0x209)));
    }

    #[test]
    fn args_override_config() {
        let args = Args::parse_from(["atelie-livre", "--window", "--scale", "fit", "--data", "/x"]);
        let c = effective(Config::default(), &args);
        assert!(!c.fullscreen);
        assert_eq!(c.scale, Scale::Fit);
        assert_eq!(c.data_dir, Some(PathBuf::from("/x")));
        let none = Args::parse_from(["atelie-livre"]);
        assert_eq!(effective(Config::default(), &none), Config::default());
    }

    #[test]
    fn expand_home_handles_tilde() {
        let home = Some(PathBuf::from("/home/p"));
        assert_eq!(
            expand_home(PathBuf::from("~/MSKIDS"), home.clone()),
            PathBuf::from("/home/p/MSKIDS")
        );
        assert_eq!(
            expand_home(PathBuf::from("~"), home.clone()),
            PathBuf::from("/home/p")
        );
        assert_eq!(
            expand_home(PathBuf::from("/abs/~x"), home.clone()),
            PathBuf::from("/abs/~x")
        );
        assert_eq!(
            expand_home(PathBuf::from("~outro/x"), home),
            PathBuf::from("~outro/x")
        );
        assert_eq!(
            expand_home(PathBuf::from("~/x"), None),
            PathBuf::from("~/x")
        );
    }

    #[test]
    fn config_path_respects_xdg() {
        assert_eq!(
            config_path_from(Some("/cfg".into()), Some("/home/p".into())),
            PathBuf::from("/cfg/atelie-livre/config.toml")
        );
        assert_eq!(
            config_path_from(None, Some("/home/p".into())),
            PathBuf::from("/home/p/.config/atelie-livre/config.toml")
        );
    }
}
