use super::MergeConfig;
use crate::utils;
use anyhow::Result;
use crossterm::style::Stylize;
use serde::Deserialize;
use std::{
    io::{self, Write},
    path::PathBuf,
};

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum RenameEmpty {
    Stem,
    Ext,
    DotExt,
    All,
}

#[derive(Default, Deserialize)]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum RenameCursor {
    End,
    Start,
    #[default]
    BeforeExt,
}

#[derive(Default, Deserialize)]
pub struct RenameOptions {
    pub empty: Option<RenameEmpty>,
    #[serde(default)]
    pub cursor: RenameCursor,
}

#[derive(Deserialize)]
pub struct UserOptions {
    save_file_path: Option<PathBuf>,
    auto_mark_save_file: Option<bool>,
    hide_extensions: Option<bool>,
    rename: Option<RenameOptions>,
}

pub struct Options {
    pub save_file_path: PathBuf,
    pub auto_mark_save_file: bool,
    pub hide_extensions: bool,
    pub rename: RenameOptions,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            save_file_path: PathBuf::new(),
            auto_mark_save_file: false,
            hide_extensions: false,
            rename: RenameOptions::default(),
        }
    }
}

impl MergeConfig for Options {
    type Other = UserOptions;

    fn merge(&mut self, user_options: Self::Other) -> Result<()> {
        macro_rules! set_options_field {
            ($name: ident) => {
                if let Some(option) = user_options.$name {
                    self.$name = option;
                }
            };
        }

        set_options_field!(save_file_path);
        set_options_field!(auto_mark_save_file);
        set_options_field!(hide_extensions);
        set_options_field!(rename);

        Ok(())
    }
}

pub fn pick_save_file_path() -> Result<PathBuf> {
    let paths = utils::get_save_file_paths()?;
    let mut input = String::new();

    if paths.is_empty() {
        println!("Specify save file path in the configuration file");
        std::process::exit(0);
    } else if paths.len() == 1 {
        println!("No save file path is specified. You can specify it in the configuration file or use the path below:\n");
        println!("{}", paths[0].to_string_lossy().stylize().bold());
        print!("\nUse the path above? [Y/n] ");

        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();

        input = input.trim().to_lowercase();
        if input.is_empty() || input == "y" || input == "yes" {
            return Ok(paths[0].clone());
        }
        std::process::exit(0);
    } else {
        println!("No save file path is specified. You can specify it in the configuration file or pick one of the paths below:\n");
        for (idx, path) in paths.iter().enumerate() {
            println!(
                "{} {}",
                format!("[{}]", idx + 1).stylize().bold().blue(),
                path.to_string_lossy().stylize().bold()
            );
        }

        print!("\nEnter a number: ");

        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();

        let idx = input.trim().parse::<usize>()?;
        if (1..=paths.len()).contains(&idx) {
            return Ok(paths[idx.saturating_sub(1)].clone());
        }

        Err(anyhow::anyhow!("Didn't enter a valid number"))
    }
}
