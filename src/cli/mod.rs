use clap::{Arg, ArgAction, ArgMatches, Command, ValueHint};
pub use handlers::handle_subcommands;
use std::env;

mod commands;
mod completion;
mod handlers;

pub fn build_command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Path to configuration file")
                .value_hint(ValueHint::FilePath)
                .value_name("FILE"),
        )
        .arg(
            Arg::new("no_config")
                .long("no-config")
                .help("Ignore configuration file")
                .conflicts_with("config")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("save_file")
                .short('s')
                .long("save-file")
                .help("Path to save file")
                .value_hint(ValueHint::FilePath)
                .value_name("FILE"),
        )
        .subcommand(commands::create_list_subcommand())
        .subcommand(commands::create_load_subcommand())
        .subcommand(commands::create_import_subcommand())
        .subcommand(commands::create_rename_subcommand())
        .subcommand(commands::create_delete_subcommand())
        .subcommand(commands::create_profile_subcommand())
}

pub fn get_matches() -> ArgMatches {
    build_command().get_matches()
}
