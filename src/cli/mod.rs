use clap::{builder::ValueParser, Arg, ArgAction, ArgMatches, Command};
pub use handlers::handle_subcommands;
use std::{env, sync::LazyLock};

mod commands;
mod completion;
mod handlers;

pub static CLAP_ARGS: LazyLock<ArgMatches> = LazyLock::new(get_matches);

pub fn build_command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Path to configuration file")
                .value_parser(ValueParser::path_buf())
                .value_name("FILE"),
        )
        .arg(
            Arg::new("no_config")
                .long("no-config")
                .help("Ignore configuration file")
                .conflicts_with("config")
                .action(ArgAction::SetTrue),
        )
        .subcommands(commands::create_entry_subcommands())
        .subcommand(commands::create_game_subcommand())
        .subcommand(commands::create_profile_subcommand())
}

pub fn get_matches() -> ArgMatches {
    build_command().get_matches()
}
