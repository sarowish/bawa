use super::completion;
use clap::{value_parser, Arg, ArgAction, Command};
use clap_complete::ArgValueCompleter;

pub fn create_entry_subcommands() -> Vec<Command> {
    let fuzzy = Arg::new("fuzzy")
        .help("launch fuzzy finder to pick file")
        .short('f')
        .long("fuzzy")
        .action(ArgAction::SetTrue);

    let mut relative_path = Arg::new("relative_path")
        .help("relative path to save file from profile")
        .value_name("RELATIVE_PATH")
        .add(ArgValueCompleter::new(completion::entry_completer));

    vec![
        Command::new("list").about("list save states"),
        Command::new("load")
            .about("load save file")
            .arg(&relative_path)
            .arg(&fuzzy),
        Command::new("import").about("import save file"),
        Command::new("rename")
            .about("rename save state")
            .arg(Arg::new("new_name").required(true).value_name("NEW_NAME"))
            .arg({
                relative_path = relative_path.required_unless_present("fuzzy");
                &relative_path
            })
            .arg(&fuzzy),
        Command::new("delete")
            .about("delete save file")
            .arg(relative_path)
            .arg(fuzzy),
    ]
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
