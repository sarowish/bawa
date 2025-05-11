use super::state;
use crate::entry::Entry;
use crate::tree::Tree;
use crate::utils;
use anyhow::Result;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

pub struct Profile {
    pub path: PathBuf,
    pub entries: Tree<Entry>,
    pub active_save_file: Option<PathBuf>,
}

impl Profile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            entries: Tree::default(),
            active_save_file: None,
        }
    }

    pub fn name(&self) -> std::borrow::Cow<'_, str> {
        self.path.file_name().unwrap().to_string_lossy()
    }

    pub fn load_entries(&mut self) -> Result<()> {
        if self.entries.root().is_some() {
            return Ok(());
        }

        let state_file = self.abs_path_to(".state");
        let root = Entry::new(&self.path);
        if let Some(state) = fs::read(state_file)
            .ok()
            .and_then(|s| bincode::deserialize::<state::ProfileState>(&s).ok())
        {
            root.add_to_tree(&state.entries, &mut self.entries)?;
            self.active_save_file = state.active_save_file.map(|rel| self.abs_path_to(rel));
        } else {
            root.add_to_tree(&[], &mut self.entries)?;
        }

        Ok(())
    }

    pub fn write_state(&self) -> Result<()> {
        utils::write_atomic(&self.abs_path_to(".state"), &bincode::serialize(self)?)
    }

    pub fn get_active_save_file(&self) -> Option<PathBuf> {
        self.active_save_file.clone()
    }

    pub fn update_active_save_file(&mut self, path: &Path) -> Result<()> {
        if matches!(&self.active_save_file, Some(active_path) if active_path == path) {
            return Ok(());
        }

        self.active_save_file = Some(path.to_owned());
        self.write_state()?;

        Ok(())
    }

    pub fn reset_active_save_file(&mut self) -> Result<()> {
        self.active_save_file = None;
        self.write_state()?;

        Ok(())
    }

    pub fn abs_path_to<A: AsRef<Path>>(&self, path: A) -> PathBuf {
        self.path.join(path)
    }

    pub fn rel_path_to(&self, entry_path: &Path) -> String {
        utils::get_relative_path(&self.path, entry_path)
            .unwrap()
            .to_string_lossy()
            .to_string()
    }

    pub fn get_file_rel_paths(&self, include_folders: bool) -> Vec<String> {
        let mut paths = Vec::new();

        for entry in self.entries.iter_nodes().skip(1) {
            if include_folders || entry.is_file() {
                let mut path = self.rel_path_to(&entry.path);

                if entry.is_folder() {
                    path.push(MAIN_SEPARATOR);
                }

                paths.push(path);
            }
        }

        paths
    }
}

impl Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
