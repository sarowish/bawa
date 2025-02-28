use super::CLAP_ARGS;
use crate::{
    app::App,
    profile::Profiles,
    tree::{widget::Tree, TreeState},
    utils,
};
use anyhow::{Context, Result};
use clap::{parser::ValueSource, ArgMatches};
use crossterm::style::Stylize;
use std::path::PathBuf;

pub fn handle_subcommands(app: &mut App) -> bool {
    let res = match CLAP_ARGS.subcommand() {
        Some(("list", args)) => handle_list_subcommand(app, args),
        Some(("load", args)) => handle_load_subcommand(app, args),
        Some(("import", args)) => handle_import_subcommand(app, args),
        Some(("rename", args)) => handle_rename_subcommand(app, args),
        Some(("delete", args)) => handle_delete_subcommand(app, args),
        Some(("profile", args)) => handle_profile_subcommand(&mut app.profiles, args),
        _ => return false,
    };

    if let Err(e) = res {
        eprintln!("{e:?}");
    }

    true
}

pub fn handle_list_subcommand(app: &mut App, _args: &ArgMatches) -> Result<()> {
    app.open_all_folds();

    let Some(profile) = app.profiles.get_profile() else {
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
    } else {
        std::process::exit(1)
    }

    if !app.message.is_empty() {
        println!("{}", *app.message);
    }

    Ok(())
}

pub fn handle_import_subcommand(app: &mut App, _args: &ArgMatches) -> Result<()> {
    app.profiles
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
        let entries = app.profiles.get_entries().unwrap();
        let id = entries
            .find_by_path(&path)
            .context("There is no such entry.")?;
        entries[id].delete()?;
    } else {
        std::process::exit(1)
    }

    Ok(())
}

pub fn handle_profile_subcommand(profiles: &mut Profiles, args: &ArgMatches) -> Result<()> {
    match args.subcommand() {
        Some(("create", args)) => {
            Profiles::create_profile(args.get_one::<String>("profile_name").unwrap())?;
        }
        Some(("delete", args)) => {
            select_profile_by_idx_or_name(profiles, args)?;
            profiles.delete_selected_profile()?;
        }
        Some(("rename", args)) => {
            select_profile_by_idx_or_name(profiles, args)?;
            profiles.rename_selected_profile(args.get_one::<String>("new_name").unwrap())?;
        }
        Some(("list", args)) => {
            for (idx, profile) in profiles.inner.items.iter().enumerate() {
                println!(
                    "{}{}{}",
                    if args.get_flag("no_index") {
                        String::new()
                    } else {
                        format!("[{idx}] ").bold().to_string()
                    },
                    profile.name(),
                    if profiles
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
            select_profile_by_idx_or_name(profiles, args)?;
            profiles.select_profile()?;
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
    let profile = app
        .profiles
        .get_profile()
        .context("No profile is selected.")?;

    let mut relative_path = args.get_one::<String>("relative_path").map(String::as_str);

    if args.get_flag("fuzzy") {
        app.fuzzy_finder.input.set_text(relative_path.unwrap_or(""));
        let paths = profile.get_file_rel_paths(false);
        relative_path = app.fuzzy_finder.run_inline(&paths)?;
    }

    Ok(relative_path.map(|rel_path| profile.abs_path_to(rel_path)))
}

fn select_profile_by_idx_or_name(profiles: &mut Profiles, args: &ArgMatches) -> Result<()> {
    let mut idx = args.get_one::<usize>("by_index").map(ToOwned::to_owned);

    if idx.is_none() {
        if let Some(name) = args.get_one::<String>("profile_name") {
            idx = profiles
                .inner
                .items
                .iter()
                .position(|profile| profile.name() == *name);

            if idx.is_none() {
                return Err(anyhow::anyhow!("No profile with the name \"{}\".", name));
            }
        };
    }

    if idx.is_some() {
        profiles.inner.state.select(idx);
    } else {
        profiles.inner.state.select(profiles.active_profile);
    }

    Ok(())
}
