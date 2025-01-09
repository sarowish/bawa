use crate::{
    profile::{get_profiles, Profiles},
    search::FuzzyFinder,
};
use clap_complete::CompletionCandidate;
use std::{ffi::OsStr, path::MAIN_SEPARATOR};

pub fn profile_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    let current = &current.to_lowercase();

    let profiles = get_profiles().unwrap_or_default();

    profiles
        .iter()
        .map(|profile| profile.name.clone())
        .filter(|name| name.to_lowercase().starts_with(current))
        .map(|name| CompletionCandidate::new(name).help(Some("Profile".into())))
        .collect()
}

pub fn entry_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let mut candidates = Vec::new();

    if let Some(pattern) = current.to_str() {
        let pattern = pattern.trim_start_matches(['\'', '"']);
        let profiles = Profiles::new().unwrap();
        if let Some(profile) = profiles.get_profile() {
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
    };

    candidates
}
