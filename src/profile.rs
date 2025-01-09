use crate::app::StatefulList;
use crate::entry::{find_entry, Entry, RcEntry};
use crate::utils;
use crate::watcher::HandleFileSystemEvent;
use anyhow::{Context, Result};
use std::fmt::Display;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

pub fn get_profiles() -> Result<Vec<Profile>> {
    Ok(utils::get_data_dir()?
        .read_dir()?
        .flatten()
        .filter(|dir_entry| dir_entry.file_type().unwrap().is_dir())
        .map(|dir_entry| Profile::new(dir_entry.path()))
        .collect())
}

pub fn get_active_profile_file() -> Result<PathBuf> {
    let data_dir = utils::get_data_dir()?;

    Ok(data_dir.join("active_profile"))
}

pub fn update_active_profile(profile_name: &str) -> Result<()> {
    let file_path = get_active_profile_file()?;
    let mut file = File::create(file_path)?;
    Ok(writeln!(file, "{profile_name}")?)
}

pub fn get_active_profile() -> Result<String> {
    Ok(fs::read_to_string(get_active_profile_file()?)?
        .trim()
        .to_string())
}

#[derive(Debug)]
pub struct Profile {
    pub name: String,
    pub path: PathBuf,
    pub entries: Vec<RcEntry>,
    pub active_save_file: Option<PathBuf>,
}

impl Profile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            entries: Vec::new(),
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            path,
            active_save_file: None,
        }
    }

    pub fn load_entries(&mut self) -> Result<()> {
        self.entries = Entry::entries_from_path(&self.path, 0)?;
        self.read_active_save_file();

        Ok(())
    }

    fn get_active_save_path(&self) -> PathBuf {
        self.path.join("active_save_file")
    }

    pub fn get_active_save_file(&self) -> Option<PathBuf> {
        self.active_save_file.clone()
    }

    pub fn delete_active_save(&self) -> Result<()> {
        let path = self.get_active_save_path();
        Ok(fs::remove_file(path)?)
    }

    pub fn update_active_save_file(&mut self, path: &Path) -> Result<()> {
        if matches!(&self.active_save_file, Some(active_path) if active_path == path) {
            return Ok(());
        }

        self.write_active_save_file(path)
            .context("Couldn't mark as active save file")?;
        self.active_save_file = Some(path.to_path_buf());

        Ok(())
    }

    fn write_active_save_file(&self, path: &Path) -> Result<()> {
        let file_path = self.get_active_save_path();
        let mut file = File::create(file_path)?;
        Ok(writeln!(
            file,
            "{}",
            utils::get_relative_path(&self.path, path)?
        )?)
    }

    pub fn read_active_save_file(&mut self) {
        if let Ok(path) =
            fs::read_to_string(self.get_active_save_path()).map(|path| self.path.join(path.trim()))
        {
            self.active_save_file = Some(path);
        }
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

    pub fn get_file_rel_paths(&self, include_folders: bool) -> Vec<String> {
        let mut paths = Vec::new();

        for entry in self.get_entries(true) {
            let entry = entry.borrow();
            let is_file = entry.is_file();
            if include_folders || is_file {
                let mut path = utils::get_relative_path(&self.path, &entry.path()).unwrap();

                if !is_file {
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
        let selected_profile = get_active_profile();
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
            update_active_profile(&profile.name)?;
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

        if matches!(get_active_profile(), Ok(selected_profile) if selected_profile  == profile.name)
        {
            update_active_profile(&new_name)?;
        }

        profile.name = new_name.to_string();
        profile.path = new_path.to_path_buf();

        if matches!(self.active_profile, Some(active_idx) if active_idx == idx) {
            for entry in &profile.entries {
                let entry_name = entry.borrow().file_name();
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
