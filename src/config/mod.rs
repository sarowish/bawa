pub mod keys;
pub mod options;
pub mod theme;

use self::{
    keys::{KeyBindings, UserKeyBindings},
    options::{Options, UserOptions},
};
use crate::{cli::CLAP_ARGS, utils};
use anyhow::Result;
use serde::Deserialize;
use std::{
    fs,
    path::PathBuf,
    sync::{LazyLock, Once},
};
use theme::{Theme, UserTheme};

static CONFIG: LazyLock<Config> = LazyLock::new(|| match Config::new() {
    Ok(config) => config,
    Err(e) => {
        eprintln!("{e:?}");
        std::process::exit(1);
    }
});
pub static OPTIONS: LazyLock<&Options> = LazyLock::new(|| &CONFIG.options);
pub static KEY_BINDINGS: LazyLock<&KeyBindings> = LazyLock::new(|| &CONFIG.key_bindings);
pub static THEME: LazyLock<&Theme> = LazyLock::new(|| &CONFIG.theme);
pub static SKIP_CONFIG: Once = Once::new();
const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize)]
struct UserConfig {
    #[serde(flatten)]
    options: UserOptions,
    theme: Option<UserTheme>,
    key_bindings: Option<UserKeyBindings>,
}

#[derive(Default)]
pub struct Config {
    pub options: Options,
    pub theme: Theme,
    pub key_bindings: KeyBindings,
}

impl Config {
    pub fn new() -> Result<Self> {
        let mut config = Self::default();

        if SKIP_CONFIG.is_completed() {
            return Ok(config);
        }

        let config_path = (!CLAP_ARGS.get_flag("no_config")).then_some(
            match CLAP_ARGS.get_one::<PathBuf>("config") {
                Some(path) => path.to_owned(),
                None => utils::get_config_dir()?.join(CONFIG_FILE),
            },
        );

        let config_str = config_path.as_ref().map(fs::read_to_string);

        if let Some(Ok(user_config)) = &config_str {
            let user_config = toml::from_str::<UserConfig>(user_config)?;
            config.merge(user_config)?;
        };

        Ok(config)
    }
}

impl MergeConfig for Config {
    type Other = UserConfig;

    fn merge(&mut self, user_config: Self::Other) -> Result<()> {
        self.options.merge(user_config.options)?;

        if let Some(theme) = user_config.theme {
            self.theme.merge(theme)?;
        }

        if let Some(key_bindings) = user_config.key_bindings {
            self.key_bindings.merge(key_bindings)?;
        }

        Ok(())
    }
}

trait MergeConfig {
    type Other;

    fn merge(&mut self, user_config: Self::Other) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::UserConfig;
    use std::{fs, path::PathBuf};

    pub fn read_example_config() -> UserConfig {
        let config_path = PathBuf::from("example/config.toml");
        let config_str = fs::read_to_string(config_path).unwrap();
        toml::from_str::<UserConfig>(&config_str).unwrap()
    }
}
