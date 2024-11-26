use clap::ArgMatches;
use config::{keys::KeyBindings, options::Options, Config};
use std::sync::LazyLock;

mod app;
mod cli;
mod commands;
mod config;
mod entry;
mod event;
mod help;
mod input;
mod message;
mod profile;
mod term;
mod ui;
mod utils;

static CLAP_ARGS: LazyLock<ArgMatches> = LazyLock::new(cli::get_matches);
static CONFIG: LazyLock<Config> = LazyLock::new(|| match Config::new() {
    Ok(config) => config,
    Err(e) => {
        eprintln!("{e:?}");
        std::process::exit(1);
    }
});
static OPTIONS: LazyLock<&'static Options> = LazyLock::new(|| &CONFIG.options);
static KEY_BINDINGS: LazyLock<&'static KeyBindings> = LazyLock::new(|| &CONFIG.key_bindings);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !OPTIONS.save_file_path.exists() {
        return Ok(());
    }

    let app = app::App::new()?;

    if let Some(("load", _)) = CLAP_ARGS.subcommand() {
        if let Err(e) = app.load_previous_save_file() {
            eprintln!("{e:?}");
        }
        return Ok(());
    }

    let res = app.run().await;
    ratatui::restore();

    if let Err(e) = res {
        eprintln!("{e:?}");
    }

    Ok(())
}
