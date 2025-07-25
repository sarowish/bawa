use super::CLAP_ARGS;
use crate::{
    app::App,
    fuzzy_finder::picker::Local,
    game::{Game, Games},
    tree::{TreeState, widget::Tree},
    utils,
};
use anyhow::{Context, Result};
use clap::{ArgMatches, parser::ValueSource};
use crossterm::style::Stylize;
use std::path::PathBuf;

pub fn handle_subcommands(app: &mut App) -> bool {
    let res = match CLAP_ARGS.subcommand() {
        Some(("list", args)) => handle_list_subcommand(app, args),
        Some(("load", args)) => handle_load_subcommand(app, args),
        Some(("import", args)) => handle_import_subcommand(app, args),
        Some(("rename", args)) => handle_rename_subcommand(app, args),
        Some(("delete", args)) => handle_delete_subcommand(app, args),
        Some(("game", args)) => handle_game_subcommand(app, args),
        Some(("profile", args)) => handle_profile_subcommand(app, args),
        _ => return false,
    };

    if let Err(e) = res {
        eprintln!("{e:?}");
    }

    true
}

pub fn handle_list_subcommand(app: &mut App, _args: &ArgMatches) -> Result<()> {
    app.open_all_folds();

    let Some(profile) = app.games.get_profile() else {
        return Err(anyhow::anyhow!("No Profile is selected"));
    };

    let mut tree_state = TreeState::default();
    let entries = &profile.entries;

    if let Some(path) = profile.get_active_save_file() {
        tree_state.active = entries.iter_ids().find(|id| entries[*id].path == path);
    }

    for item in Tree::from(entries).items {
        let spans = &item.content.iter().next().unwrap().spans;

        print!("{}", spans[0].content.dark_grey());
        print!("{}{}", spans[1], spans[2]);
        if let Some(span) = spans.get(3) {
            println!("{}", span.content.yellow());
        } else {
            println!();
        }
    }

    Ok(())
}

pub fn handle_load_subcommand(app: &mut App, args: &ArgMatches) -> Result<()> {
    if let Some(path) = get_entry_path(args, app)? {
        app.load_save_file(&path, true)?;
    } else if !any_args(args) {
        app.load_active_save_file();
    } else if args.get_flag("random") {
        app.load_random_save_file();
    } else {
        std::process::exit(1)
    }

    if !app.message.is_empty() {
        println!("{}", *app.message);
    }

    Ok(())
}

pub fn handle_import_subcommand(app: &mut App, _args: &ArgMatches) -> Result<()> {
    app.games
        .get_game()
        .context("No game is selected.")?
        .active_profile
        .context("No profile is selected.")?;
    app.import_save_file(true);
    Ok(())
}

fn handle_rename_subcommand(app: &mut App, args: &ArgMatches) -> Result<()> {
    if let Some(entry_path) = get_entry_path(args, app)? {
        let new_name = args.get_one::<String>("new_name").unwrap();
        let mut new_path = entry_path.clone();
        new_path.set_file_name(new_name);

        utils::rename(&entry_path, &new_path)?;
    } else {
        std::process::exit(1)
    }

    Ok(())
}

fn handle_delete_subcommand(app: &mut App, args: &ArgMatches) -> Result<()> {
    if let Some(path) = get_entry_path(args, app)? {
        let entries = app.games.get_entries().unwrap();
        let id = entries
            .find_by_path(&path)
            .context("There is no such entry.")?;
        entries[id].delete()?;
    } else {
        std::process::exit(1)
    }

    Ok(())
}

pub fn handle_game_subcommand(app: &mut App, args: &ArgMatches) -> Result<()> {
    let games = &mut app.games;

    match args.subcommand() {
        Some(("create", args)) => {
            Games::create_game(
                &mut app.games,
                args.get_one::<String>("game_name").unwrap(),
                args.get_one::<PathBuf>("savefile_path").unwrap(),
            )?;
        }
        Some(("delete", args)) => {
            select_game_by_idx_or_name(games, args)?;
            games.delete_selected_game()?;
        }
        Some(("rename", args)) => {
            select_game_by_idx_or_name(games, args)?;
            games.rename_selected_game(args.get_one::<String>("new_name").unwrap())?;
        }
        Some(("list", args)) => {
            for (idx, profile) in games.inner.items.iter().enumerate() {
                println!(
                    "{}{}{}",
                    if args.get_flag("no_index") {
                        String::new()
                    } else {
                        format!("[{idx}] ").bold().to_string()
                    },
                    profile.name(),
                    if games
                        .active_game
                        .is_some_and(|active_idx| active_idx == idx)
                    {
                        " (*)".yellow().bold().to_string()
                    } else {
                        String::new()
                    },
                );
            }
        }
        Some(("set", args)) => {
            select_game_by_idx_or_name(games, args)?;
            games.select_game()?;
        }
        _ => return Ok(()),
    }

    Ok(())
}

pub fn handle_profile_subcommand(app: &mut App, args: &ArgMatches) -> Result<()> {
    let game = app.games.get_game_mut().context("No game is selected.")?;

    match args.subcommand() {
        Some(("create", args)) => {
            game.create_profile(args.get_one::<String>("profile_name").unwrap())?;
        }
        Some(("delete", args)) => {
            select_profile_by_idx_or_name(game, args)?;
            game.delete_selected_profile()?;
        }
        Some(("rename", args)) => {
            select_profile_by_idx_or_name(game, args)?;
            game.rename_selected_profile(args.get_one::<String>("new_name").unwrap())?;
        }
        Some(("list", args)) => {
            for (idx, profile) in game.profiles.items.iter().enumerate() {
                println!(
                    "{}{}{}",
                    if args.get_flag("no_index") {
                        String::new()
                    } else {
                        format!("[{idx}] ").bold().to_string()
                    },
                    profile.name(),
                    if game
                        .active_profile
                        .is_some_and(|active_idx| active_idx == idx)
                    {
                        " (*)".yellow().bold().to_string()
                    } else {
                        String::new()
                    },
                );
            }
        }
        Some(("set", args)) => {
            select_profile_by_idx_or_name(game, args)?;
            game.select_profile()?;
        }
        _ => return Ok(()),
    }

    Ok(())
}

fn any_args(args: &ArgMatches) -> bool {
    args.ids()
        .filter_map(|id| args.value_source(id.as_str()))
        .any(|value| value == ValueSource::CommandLine)
}

fn get_entry_path(args: &ArgMatches, app: &mut App) -> Result<Option<PathBuf>> {
    let profile = app.games.get_profile().context("No profile is selected.")?;

    let mut relative_path = args.get_one::<String>("relative_path").map(String::as_str);

    if args.get_flag("fuzzy") {
        app.fuzzy_finder.input.set_text(relative_path.unwrap_or(""));
        app.fuzzy_finder.set_picker(Local::new(app));
        relative_path = app.fuzzy_finder.run_inline()?;
    }

    Ok(relative_path.map(|rel_path| profile.abs_path_to(rel_path)))
}

fn select_game_by_idx_or_name(games: &mut Games, args: &ArgMatches) -> Result<()> {
    let mut idx = args.get_one::<usize>("by_index").map(ToOwned::to_owned);

    if idx.is_none() {
        if let Some(name) = args.get_one::<String>("game_name") {
            idx = games
                .inner
                .items
                .iter()
                .position(|game| game.name() == *name);

            if idx.is_none() {
                return Err(anyhow::anyhow!("No game with the name \"{}\".", name));
            }
        }
    }

    if idx.is_some() {
        games.inner.state.select(idx);
    } else {
        games.inner.state.select(games.active_game);
    }

    Ok(())
}

fn select_profile_by_idx_or_name(game: &mut Game, args: &ArgMatches) -> Result<()> {
    let mut idx = args.get_one::<usize>("by_index").map(ToOwned::to_owned);
    let profiles = &mut game.profiles;

    if idx.is_none() {
        if let Some(name) = args.get_one::<String>("profile_name") {
            idx = profiles
                .items
                .iter()
                .position(|profile| profile.name() == *name);

            if idx.is_none() {
                return Err(anyhow::anyhow!("No profile with the name \"{}\".", name));
            }
        }
    }

    if idx.is_some() {
        profiles.state.select(idx);
    } else {
        profiles.state.select(game.active_profile);
    }

    Ok(())
}
