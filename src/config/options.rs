use crate::{utils, CLAP_ARGS};
use anyhow::Result;
use crossterm::style::Stylize;
use serde::Deserialize;
use std::{
    io::{self, Write},
    path::PathBuf,
};

#[derive(Deserialize)]
pub struct UserOptions {
    save_file_path: Option<PathBuf>,
    auto_mark_save_file: Option<bool>,
    hide_extensions: Option<bool>,
}

pub struct Options {
    pub save_file_path: PathBuf,
    pub auto_mark_save_file: bool,
    pub hide_extensions: bool,
}

impl Options {
    pub fn override_with_clap_args(&mut self) {
        if let Some(save_file_path) = CLAP_ARGS.get_one::<PathBuf>("save_file") {
            save_file_path.clone_into(&mut self.save_file_path);
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            save_file_path: PathBuf::new(),
            auto_mark_save_file: false,
            hide_extensions: false,
        }
    }
}

impl From<UserOptions> for Options {
    fn from(user_options: UserOptions) -> Self {
        let mut options = Options::default();

        macro_rules! set_options_field {
            ($name: ident) => {
                if let Some(option) = user_options.$name {
                    options.$name = option;
                }
            };
        }

        set_options_field!(save_file_path);
        set_options_field!(auto_mark_save_file);
        set_options_field!(hide_extensions);

        options
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
    } else {
        println!("No save file path is specified. You can specify it in the configuration file or pick one of the paths below:\n");
        for (idx, path) in paths.iter().enumerate() {
            println!(
                "{} {}",
                format!("[{}]", idx + 1).stylize().bold().blue(),
                path.to_string_lossy().stylize().bold()
            );
        }

        print!("\nEnter a number (you can type something invalid to cancel): ");

        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();

        let idx = input.trim().parse::<usize>()?;
        if (1..=paths.len()).contains(&idx) {
            return Ok(paths[idx.saturating_sub(1)].clone());
        }
    }

    Err(anyhow::anyhow!("Didn't enter a valid number"))
}
