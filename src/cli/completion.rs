use crate::{
    config::SKIP_CONFIG,
    fuzzy_finder::FuzzyFinder,
    game::{Game, Games, get_active_game, profile::Profile, read_games},
    utils,
};
use clap_complete::CompletionCandidate;
use std::{ffi::OsStr, path::MAIN_SEPARATOR};

pub fn game_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    let current = &current.to_lowercase();

    let games = read_games().unwrap_or_default();

    games
        .iter()
        .map(Game::name)
        .filter(|name| name.to_lowercase().starts_with(current))
        .map(|name| CompletionCandidate::new(name.into_owned()).help(Some("Game".into())))
        .collect()
}

pub fn profile_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    let current = &current.to_lowercase();

    let game = utils::get_state_dir()
        .and_then(|path| get_active_game().map(|name| path.join(name)))
        .map(Game::new);

    let profiles = game
        .and_then(|game| game.read_profiles())
        .unwrap_or_default();

    profiles
        .iter()
        .map(Profile::name)
        .filter(|name| name.to_lowercase().starts_with(current))
        .map(|name| CompletionCandidate::new(name.into_owned()).help(Some("Profile".into())))
        .collect()
}

pub fn entry_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    SKIP_CONFIG.call_once(|| {});
    let mut candidates = Vec::new();

    if let Some(pattern) = current.to_str() {
        let pattern = pattern.trim_start_matches(['\'', '"']);
        let games = Games::new().ok();
        if let Some(profile) = games.as_ref().and_then(Games::get_profile) {
            let paths = profile.get_file_rel_paths(true);
            let matched = FuzzyFinder::non_interactive(&paths, pattern);

            for item in matched {
                let depth = item
                    .trim_end_matches(MAIN_SEPARATOR)
                    .matches(MAIN_SEPARATOR)
                    .count();
                let help = if item.ends_with(MAIN_SEPARATOR) {
                    "Folder"
                } else {
                    "Save File"
                };

                candidates.push(
                    CompletionCandidate::new(item)
                        .help(Some(help.into()))
                        .display_order(Some(depth)),
                );
            }
        }
    }

    candidates
}
