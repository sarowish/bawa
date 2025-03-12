#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::return_self_not_must_use)]

pub mod app;
pub mod cli;
mod commands;
pub mod config;
mod entry;
mod event;
mod fuzzy_finder;
mod help;
mod input;
mod message;
mod profile;
pub mod search;
pub mod tree;
pub mod ui;
mod utils;
mod watcher;
