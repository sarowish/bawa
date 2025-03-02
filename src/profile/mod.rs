use crate::app::StatefulList;
use crate::entry::Entry;
use crate::tree::{NodeId, Tree};
use crate::utils;
use crate::watcher::HandleFileSystemEvent;
use anyhow::Result;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

pub mod state;

pub fn get_profiles() -> Result<Vec<Profile>> {
    Ok(utils::get_state_dir()?
        .read_dir()?
        .flatten()
        .filter(|dir_entry| dir_entry.file_type().unwrap().is_dir())
        .map(|dir_entry| Profile::new(dir_entry.path()))
        .collect())
}

pub fn get_active_profile_file() -> Result<PathBuf> {
    Ok(utils::get_state_dir()?.join("active_profile"))
}

pub fn update_active_profile(profile_name: &str) -> Result<()> {
    let file_path = get_active_profile_file()?;
    let mut file = File::create(file_path)?;
    Ok(writeln!(file, "{profile_name}")?)
}

pub fn get_active_profile() -> Result<String> {
    Ok(fs::read_to_string(get_active_profile_file()?)?
        .trim()
        .to_owned())
}

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
        let state_file = self.abs_path_to("_state");
        let root = Entry::new(&self.path);
        if let Some(state) = fs::read(state_file)
            .ok()
            .and_then(|s| bincode::deserialize::<state::State>(&s).ok())
        {
            root.add_to_tree(&state.entries, &mut self.entries)?;
            self.active_save_file = state.active_save_file.map(|rel| self.abs_path_to(rel));
        } else {
            root.add_to_tree(&[], &mut self.entries)?;
        }

        Ok(())
    }

    pub fn write_state(&self) -> Result<()> {
        utils::write_atomic(&self.abs_path_to("_state"), &bincode::serialize(self)?)
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
        utils::get_relative_path(&self.path, entry_path).unwrap()
    }

    pub fn get_file_rel_paths(&self, include_folders: bool) -> Vec<String> {
        let mut paths = Vec::new();

        for id in self.entries.iter_ids() {
            let entry = &self.entries[id];
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

pub struct Profiles {
    pub inner: StatefulList<Profile>,
    pub active_profile: Option<usize>,
}

impl Profiles {
    pub fn new() -> Result<Self> {
        let active_profile_name = get_active_profile();
        let mut profiles = get_profiles()?;
        let mut active_profile = None;

        if let Ok(name) = active_profile_name {
            active_profile = profiles.iter().position(|profile| profile.name() == name);

            if let Some(idx) = active_profile {
                profiles[idx].load_entries()?;
            }
        }

        Ok(Self {
            inner: StatefulList::with_items(profiles),
            active_profile,
        })
    }

    pub fn create_profile(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        let path = utils::get_state_dir()?.join(name);
        utils::check_for_dup(&path)?;
        std::fs::create_dir(&path)?;

        Ok(())
    }

    pub fn rename_selected_profile(&mut self, new_name: &str) -> Result<()> {
        if new_name.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        if let Some(profile) = self.inner.get_selected() {
            let mut new_path = profile.path.clone();
            new_path.set_file_name(new_name);
            utils::rename(&profile.path, &new_path)?;
        }

        Ok(())
    }

    pub fn delete_selected_profile(&mut self) -> Result<()> {
        if let Some(profile) = self.inner.get_selected() {
            std::fs::remove_dir_all(&profile.path)?;
        }

        Ok(())
    }

    pub fn select_profile(&mut self) -> Result<bool> {
        if let Some(idx) = self.active_profile {
            if matches!(self.inner.state.selected(), Some(selected_idx) if idx == selected_idx) {
                return Ok(false);
            }

            self.inner.items[idx].entries.empty();
        }

        if let Some(profile) = self.inner.get_mut_selected() {
            profile.load_entries()?;
            update_active_profile(&profile.name())?;
            self.active_profile = self.inner.state.selected();
            Ok(true)
        } else {
            Err(anyhow::anyhow!("There aren't any profiles to select"))
        }
    }

    pub fn get_profile(&self) -> Option<&Profile> {
        self.inner.items.get(self.active_profile?)
    }

    pub fn get_profile_mut(&mut self) -> Option<&mut Profile> {
        self.inner.items.get_mut(self.active_profile?)
    }

    pub fn get_entries(&self) -> Option<&Tree<Entry>> {
        self.get_profile().map(|profile| &profile.entries)
    }

    pub fn get_entries_mut(&mut self) -> Option<&mut Tree<Entry>> {
        self.get_profile_mut().map(|profile| &mut profile.entries)
    }
}

impl HandleFileSystemEvent for Profiles {
    fn on_create(&mut self, path: &Path) -> Result<()> {
        self.inner.items.push(Profile::new(path.to_owned()));

        Ok(())
    }

    fn on_rename(&mut self, path: &Path, new_path: &Path) -> Result<()> {
        let profiles = &mut self.inner.items;
        let Some(idx) = profiles.iter().position(|profile| profile.path == path) else {
            return Ok(());
        };

        let profile = &mut profiles[idx];
        let new_name = new_path.file_name().unwrap().to_string_lossy();

        if matches!(get_active_profile(), Ok(active_profile) if active_profile == profile.name()) {
            update_active_profile(&new_name)?;
        }

        if matches!(self.active_profile, Some(active_idx) if active_idx == idx) {
            profile.entries.update_paths(NodeId::root(), new_path)?;
        }

        Ok(())
    }

    fn on_delete(&mut self, path: &Path) -> Result<()> {
        let profiles = &self.inner.items;
        if let Some(idx) = profiles.iter().position(|profile| profile.path == path) {
            self.inner.items.remove(idx);

            if matches!(self.active_profile, Some(active_idx) if active_idx == idx) {
                self.active_profile = None;
            }
        };

        Ok(())
    }
}
