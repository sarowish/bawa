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
    path::{Path, PathBuf},
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

        if let Some(save_file_path) = CLAP_ARGS.get_one::<PathBuf>("save_file") {
            save_file_path.clone_into(&mut config.options.save_file_path);
        } else if config.options.save_file_path.as_os_str().is_empty() {
            config.options.save_file_path = options::pick_save_file_path()?;

            if let Some(Ok(config_str)) = config_str {
                insert_save_path(
                    config_path.as_ref().unwrap(),
                    &config_str,
                    &config.options.save_file_path,
                )?;
            } else if let Some(path) = config_path {
                create_new_config(&path, &config.options.save_file_path)?;
            }
        }

        if config.options.save_file_path.exists() {
            Ok(config)
        } else {
            Err(anyhow::anyhow!(
                "There is no file at {:?}",
                config.options.save_file_path
            ))
        }
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

fn create_new_config(config_path: &Path, save_file_path: &Path) -> Result<()> {
    let config_dir = config_path.parent().unwrap();
    let mut config = toml_edit::DocumentMut::new();
    config["save_file_path"] = toml_edit::value(save_file_path.to_str().unwrap());

    if !config_dir.exists() {
        std::fs::create_dir_all(config_dir).unwrap();
    }

    Ok(fs::write(config_path, config.to_string())?)
}

fn insert_save_path(path: &Path, config_str: &str, save_file: &Path) -> Result<()> {
    let mut document: toml_edit::DocumentMut = config_str.parse()?;
    overwrite_value(
        document.as_table_mut(),
        "save_file_path",
        save_file.to_str().unwrap(),
    );
    utils::write_atomic(path, document.to_string().as_bytes())
}

fn overwrite_value(table: &mut toml_edit::Table, key: &str, value: impl Into<toml_edit::Value>) {
    let mut value = value.into();

    let existing = table.entry(key).or_insert_with(Default::default);
    if let Some(existing_value) = existing.as_value() {
        *value.decor_mut() = existing_value.decor().clone();
    }

    *existing = toml_edit::Item::Value(value);
}
