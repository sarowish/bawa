use crate::{app::App, entry::entries_to_spans, profile::Profiles, utils, CLAP_ARGS};
use anyhow::{Context, Result};
use clap::ArgMatches;
use crossterm::style::Stylize;

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
    app.profiles
        .active_profile
        .context("No Profile is selected.")?;

    app.open_all_folds();

    for spans in entries_to_spans(&app.visible_entries.items, &Default::default()) {
        print!("{}", spans[0].content.dark_grey());
        println!("{}{}", spans[1], spans[2]);
    }

    Ok(())
}

pub fn handle_load_subcommand(app: &mut App, args: &ArgMatches) -> Result<()> {
    if let Some(relative_path) = args.get_one::<String>("relative_path") {
        let profile = app
            .profiles
            .get_profile()
            .context("No profile is selected.")?;
        app.load_save_file(&profile.path.join(relative_path))?;
        Ok(())
    } else {
        app.load_active_save_file()
    }
}

pub fn handle_import_subcommand(app: &mut App, _args: &ArgMatches) -> Result<()> {
    app.profiles
        .active_profile
        .context("No profile is selected.")?;
    app.import_save_file(true);
    Ok(())
}

fn handle_rename_subcommand(app: &mut App, args: &ArgMatches) -> Result<()> {
    let new_name = args.get_one::<String>("new_name").unwrap();
    let relative_path = args.get_one::<String>("relative_path").unwrap();
    let profile_path = &app
        .profiles
        .get_profile()
        .context("No profile is selected.")?
        .path;

    let entry_path = profile_path.join(relative_path);
    let mut new_path = entry_path.clone();
    new_path.set_file_name(new_name);

    utils::rename(&entry_path, &new_path)?;

    Ok(())
}

fn handle_delete_subcommand(app: &mut App, args: &ArgMatches) -> Result<()> {
    let relative_path = args.get_one::<String>("relative_path").unwrap();
    let profile = app
        .profiles
        .get_profile()
        .context("No profile is selected.")?;

    let path_to_entry = profile.find_entry(&profile.path.join(relative_path));
    let entry = &path_to_entry.last().context("There is no such entry.")?.1;
    entry.borrow().delete()?;

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
            for (idx, profile) in profiles.profiles.items.iter().enumerate() {
                println!(
                    "{}{}{}",
                    if args.get_flag("no_index") {
                        String::new()
                    } else {
                        format!("[{idx}] ").bold().to_string()
                    },
                    profile.name,
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

fn select_profile_by_idx_or_name(profiles: &mut Profiles, args: &ArgMatches) -> Result<()> {
    let mut idx = args.get_one::<usize>("by_index").map(ToOwned::to_owned);

    if idx.is_none() {
        if let Some(name) = args.get_one::<String>("profile_name") {
            idx = profiles
                .profiles
                .items
                .iter()
                .position(|profile| profile.name == *name);

            if idx.is_none() {
                return Err(anyhow::anyhow!("No profile with the name \"{}\".", name));
            }
        };
    }

    if idx.is_some() {
        profiles.profiles.state.select(idx);
    } else {
        profiles.profiles.state.select(profiles.active_profile);
    }

    Ok(())
}
