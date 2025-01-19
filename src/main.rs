use clap_complete::CompleteEnv;
use cli::handle_subcommands;
use config::OPTIONS;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    CompleteEnv::with_factory(cli::build_command).complete();

    if !OPTIONS.save_file_path.exists() {
        return Ok(());
    }

    let mut app = app::App::new()?;

    if handle_subcommands(&mut app) {
        return Ok(());
    }

    let res = app.run().await;
    ui::restore();

    if let Err(e) = res {
        eprintln!("{e:?}");
    }

    Ok(())
}
