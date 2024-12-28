use clap::ArgMatches;
use cli::handle_subcommands;
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
mod search;
mod ui;
mod utils;
mod watcher;

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

    let mut app = app::App::new()?;

    if handle_subcommands(&mut app) {
        return Ok(());
    }

    let res = app.run().await;
    ratatui::restore();

    if let Err(e) = res {
        eprintln!("{e:?}");
    }

    Ok(())
}
