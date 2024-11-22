pub mod keys;
pub mod options;

use self::{
    keys::{KeyBindings, UserKeyBindings},
    options::{Options, UserOptions},
};
use crate::{utils, CLAP_ARGS};
use anyhow::Result;
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize)]
struct UserConfig {
    #[serde(flatten)]
    options: UserOptions,
    key_bindings: Option<UserKeyBindings>,
}

#[derive(Default)]
pub struct Config {
    pub options: Options,
    pub key_bindings: KeyBindings,
}

impl Config {
    pub fn new() -> Result<Self> {
        let config_file = match CLAP_ARGS.get_one::<PathBuf>("config") {
            Some(path) => path.to_owned(),
            None => utils::get_config_dir()?.join(CONFIG_FILE),
        };

        let mut config = match fs::read_to_string(&config_file) {
            Ok(config_str) if !CLAP_ARGS.get_flag("no_config") => {
                Self::try_from(toml::from_str::<UserConfig>(&config_str)?)?
            }
            _ => Self::default(),
        };

        config.options.override_with_clap_args();

        if !config.options.save_file_path.exists() {
            config.options.save_file_path = options::pick_save_file_path()?;

            if !CLAP_ARGS.get_flag("no_config") && !config_file.exists() {
                let config_dir = config_file.parent().unwrap();

                if !config_dir.exists() {
                    std::fs::create_dir_all(config_dir).unwrap();
                }

                let mut file = File::create(config_file)?;
                writeln!(file, "save_file_path = {:?}", config.options.save_file_path)?;
            }
        }

        Ok(config)
    }
}

impl TryFrom<UserConfig> for Config {
    type Error = anyhow::Error;

    fn try_from(user_config: UserConfig) -> Result<Self, Self::Error> {
        let mut config = Self {
            options: user_config.options.into(),
            ..Default::default()
        };

        if let Some(key_bindings) = user_config.key_bindings {
            config.key_bindings = key_bindings.try_into()?;
        }

        Ok(config)
    }
}
