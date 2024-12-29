use crate::app::StatefulList;
use crate::entry::{find_entry, Entry, RcEntry};
use crate::utils;
use crate::watcher::HandleFileSystemEvent;
use anyhow::Result;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn get_profiles() -> Result<Vec<Profile>> {
    Ok(utils::get_data_dir()?
        .read_dir()?
        .flatten()
        .filter(|dir_entry| dir_entry.file_type().unwrap().is_dir())
        .map(|dir_entry| Profile::new(dir_entry.path()))
        .collect())
}

pub fn get_selected_profile_file() -> Result<PathBuf> {
    let data_dir = utils::get_data_dir()?;

    Ok(data_dir.join("selected_profile"))
}

pub fn update_selected_profile(profile_name: &str) -> Result<()> {
    let file_path = get_selected_profile_file()?;
    let mut file = File::create(file_path)?;
    Ok(writeln!(file, "{profile_name}")?)
}

pub fn get_selected_profile() -> Result<String> {
    Ok(fs::read_to_string(get_selected_profile_file()?)?
        .trim()
        .to_string())
}

#[derive(Debug)]
pub struct Profile {
    pub name: String,
    pub path: PathBuf,
    pub entries: Vec<RcEntry>,
}

impl Profile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            entries: Vec::new(),
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            path,
        }
    }

    pub fn load_entries(&mut self) -> Result<()> {
        self.entries = Entry::entries_from_path(&self.path, 0)?;
        Ok(())
    }

    fn get_selected_save_path(&self) -> PathBuf {
        self.path.join("selected_save_file")
    }

    pub fn delete_selected_save(&self) -> Result<()> {
        let path = self.get_selected_save_path();
        fs::remove_file(path)?;
        Ok(())
    }

    pub fn update_selected_save_file(&self, path: &Path) -> Result<()> {
        let file_path = self.get_selected_save_path();
        let mut file = File::create(file_path)?;
        Ok(writeln!(
            file,
            "{}",
            utils::get_relative_path(&self.path, path)?
        )?)
    }

    pub fn get_selected_save_file(&self) -> Result<PathBuf> {
        Ok(self
            .path
            .join(fs::read_to_string(self.get_selected_save_path())?.trim()))
    }

    pub fn find_entry(&self, entry_path: &Path) -> Vec<(usize, RcEntry)> {
        let components = utils::get_relative_path_with_components(&self.path, entry_path).unwrap();

        if components.is_empty() {
            Vec::new()
        } else {
            find_entry(&self.entries, &components)
        }
    }

    pub fn get_entries(&self, flatten: bool) -> Vec<RcEntry> {
        let mut entries = Vec::new();

        for entry in &self.entries {
            entries.push(entry.clone());
            entries.append(&mut entry.borrow().children(flatten));
        }

        entries
    }

    pub fn get_file_rel_paths(&self) -> Vec<String> {
        self.get_entries(true)
            .iter()
            .filter(|entry| entry.borrow().is_file())
            .map(|entry| utils::get_relative_path(&self.path, &entry.borrow().path()).unwrap())
            .collect()
    }
}

impl Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug)]
pub struct Profiles {
    pub profiles: StatefulList<Profile>,
    pub active_profile: Option<usize>,
}

impl Profiles {
    pub fn new() -> Result<Self> {
        let selected_profile = get_selected_profile();
        let mut profiles = get_profiles()?;
        let mut active_profile = None;

        if let Ok(name) = selected_profile {
            active_profile = profiles.iter().position(|profile| profile.name == name);

            if let Some(idx) = active_profile {
                profiles[idx].load_entries()?;
            }
        }

        Ok(Self {
            profiles: StatefulList::with_items(profiles),
            active_profile,
        })
    }

    pub fn create_profile(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        let path = utils::get_data_dir()?.join(name);
        utils::check_for_dup(&path)?;
        std::fs::create_dir(&path)?;

        Ok(())
    }

    pub fn rename_selected_profile(&mut self, new_name: &str) -> Result<()> {
        if new_name.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        if let Some(profile) = self.profiles.get_selected() {
            let mut new_path = profile.path.clone();
            new_path.set_file_name(new_name);
            utils::rename(&profile.path, &new_path)?;
        }

        Ok(())
    }

    pub fn delete_selected_profile(&mut self) -> Result<()> {
        if let Some(profile) = self.profiles.get_selected() {
            std::fs::remove_dir_all(&profile.path)?;
        }

        Ok(())
    }

    pub fn select_profile(&mut self) -> Result<bool> {
        if let Some(idx) = self.active_profile {
            if matches!(self.profiles.state.selected(), Some(selected_idx) if idx == selected_idx) {
                return Ok(false);
            }

            self.profiles.items[idx].entries.drain(..);
        }

        if let Some(profile) = self.profiles.get_mut_selected() {
            profile.load_entries()?;
            update_selected_profile(&profile.name)?;
            self.active_profile = self.profiles.state.selected();
            Ok(true)
        } else {
            Err(anyhow::anyhow!("There aren't any profiles to select"))
        }
    }

    pub fn get_profile(&self) -> Option<&Profile> {
        self.profiles.items.get(self.active_profile?)
    }

    pub fn get_mut_profile(&mut self) -> Option<&mut Profile> {
        self.profiles.items.get_mut(self.active_profile?)
    }
}

impl HandleFileSystemEvent for Profiles {
    fn on_create(&mut self, path: &Path) -> Result<()> {
        self.profiles.items.push(Profile::new(path.to_path_buf()));

        Ok(())
    }

    fn on_rename(&mut self, path: &Path, new_path: &Path) -> Result<()> {
        let items = &self.profiles.items;
        let Some(idx) = items.iter().position(|profile| profile.path == path) else {
            return Ok(());
        };

        let profile = &mut self.profiles.items[idx];
        let new_name = new_path.file_name().unwrap().to_string_lossy();

        if matches!(get_selected_profile(), Ok(selected_profile) if selected_profile  == profile.name)
        {
            update_selected_profile(&new_name)?;
        }

        profile.name = new_name.to_string();
        profile.path = new_path.to_path_buf();

        if matches!(self.active_profile, Some(active_idx) if active_idx == idx) {
            for entry in &profile.entries {
                let entry_name = entry.borrow().name();
                *entry.borrow_mut().path_mut() = profile.path.join(entry_name);
                entry.borrow_mut().update_children_path();
            }
        }

        Ok(())
    }

    fn on_delete(&mut self, path: &Path) -> Result<()> {
        let items = &self.profiles.items;
        if let Some(idx) = items.iter().position(|profile| profile.path == path) {
            self.profiles.items.remove(idx);

            if matches!(self.active_profile, Some(active_idx) if active_idx == idx) {
                self.active_profile = None;
            }
        };

        Ok(())
    }
}
