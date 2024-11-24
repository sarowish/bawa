use anyhow::Result;
use app::App;
use clap::ArgMatches;
use config::{keys::KeyBindings, options::Options, Config};
use crossterm::event::{self, Event};
use input::Mode;
use ratatui::DefaultTerminal;
use std::sync::LazyLock;

mod app;
mod cli;
mod commands;
mod config;
mod entry;
mod help;
mod input;
mod profile;
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

fn main() {
    if !OPTIONS.save_file_path.exists() {
        return;
    }

    let app = app::App::new();

    if let Some(("load", _)) = CLAP_ARGS.subcommand() {
        if let Err(e) = app.load_previous_save_file() {
            eprintln!("{e}");
        }
        return;
    }

    let terminal = ratatui::init();
    run_tui(terminal, app).unwrap();
    ratatui::restore();
}

fn run_tui(mut terminal: DefaultTerminal, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if let Some(input) = &app.footer_input {
            const RENAMING_CURSOR_OFFSET: u16 = 8;
            const FOLDER_CREATION_CURSOR_OFFSET: u16 = 13;
            const PROFILE_CREATION_CURSOR_OFFSET: u16 = 14;

            let cursor_position = input.cursor_position;
            let offset = match app.mode {
                Mode::EntryRenaming | Mode::ProfileRenaming => RENAMING_CURSOR_OFFSET,
                Mode::FolderCreation(_) => FOLDER_CREATION_CURSOR_OFFSET,
                Mode::ProfileCreation => PROFILE_CREATION_CURSOR_OFFSET,
                _ => panic!(),
            };
            terminal
                .set_cursor_position((cursor_position + offset, terminal.size()?.height - 1))?;
            terminal.show_cursor()?;
        } else {
            terminal.hide_cursor()?;
        }

        if let Event::Key(key) = event::read()? {
            if input::handle_event(key, &mut app) {
                break;
            }
        }
    }

    Ok(())
}
