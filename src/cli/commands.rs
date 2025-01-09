use super::completion;
use clap::{value_parser, Arg, ArgAction, Command};
use clap_complete::ArgValueCompleter;

pub fn create_list_subcommand() -> Command {
    Command::new("list").about("list save states")
}

pub fn create_load_subcommand() -> Command {
    Command::new("load").about("load save file").arg(
        Arg::new("relative_path")
            .help("relative path to save file from profile")
            .value_name("RELATIVE_PATH")
            .add(ArgValueCompleter::new(completion::entry_completer)),
    )
}

pub fn create_import_subcommand() -> Command {
    Command::new("import").about("import save file")
}

pub fn create_rename_subcommand() -> Command {
    Command::new("rename")
        .about("rename save state")
        .arg(Arg::new("new_name").required(true).value_name("NEW_NAME"))
        .arg(
            Arg::new("relative_path")
                .help("relative path to save file from profile")
                .required(true)
                .value_name("RELATIVE_PATH")
                .add(ArgValueCompleter::new(completion::entry_completer)),
        )
}

pub fn create_delete_subcommand() -> Command {
    Command::new("delete").about("delete save file").arg(
        Arg::new("relative_path")
            .help("relative path to save file from profile")
            .required(true)
            .value_name("RELATIVE_PATH")
            .add(ArgValueCompleter::new(completion::entry_completer)),
    )
}

pub fn create_profile_subcommand() -> Command {
    let by_index = Arg::new("by_index")
        .short('i')
        .long("by-index")
        .conflicts_with("profile_name")
        .value_name("INDEX")
        .value_parser(value_parser!(usize));

    Command::new("profile")
        .about("manage profiles")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create")
                .about("create profile")
                .arg(Arg::new("profile_name").required(true).value_name("NAME")),
        )
        .subcommand(
            Command::new("delete")
                .about("delete profile")
                .arg(
                    Arg::new("profile_name")
                        .value_name("NAME")
                        .add(ArgValueCompleter::new(completion::profile_completer)),
                )
                .arg(by_index.clone().help("select profile by index")),
        )
        .subcommand(
            Command::new("rename")
                .about("rename profile")
                .arg(Arg::new("new_name").required(true).value_name("NEW_NAME"))
                .arg(
                    Arg::new("profile_name")
                        .value_name("NAME")
                        .add(ArgValueCompleter::new(completion::profile_completer)),
                )
                .arg(by_index.clone().help("select profile by index")),
        )
        .subcommand(
            Command::new("list")
                .about("list the available profiles")
                .arg(
                    Arg::new("no_index")
                        .help("don't show indices")
                        .long("no-index")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("set")
                .about("set profile")
                .arg(
                    Arg::new("profile_name")
                        .required_unless_present("by_index")
                        .value_name("NAME")
                        .add(ArgValueCompleter::new(completion::profile_completer)),
                )
                .arg(by_index.help("set profile by index")),
        )
}
