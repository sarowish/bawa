use crate::app::StatefulList;
use crate::entry::Entry;
use crate::utils;
use anyhow::Result;
use std::cell::RefCell;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub fn get_profiles() -> Result<Vec<Profile>> {
    Ok(utils::get_data_dir()?
        .read_dir()?
        .flatten()
        .filter(|dir_entry| dir_entry.file_type().unwrap().is_dir())
        .map(|dir_entry| Profile::new(dir_entry.path()))
        .collect())
}

pub fn get_selected_profile_file() -> Result<PathBuf> {
    let data_dir = utils::get_data_dir();

    data_dir.map(|path| path.join("selected_profile"))
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
    pub entries: Vec<Rc<RefCell<Entry>>>,
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
}

impl Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug)]
pub struct Profiles {
    pub profiles: StatefulList<Profile>,
    active_profile: Option<usize>,
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

    pub fn create_profile(&mut self, name: &str) {
        let path = utils::get_data_dir().unwrap().join(name);

        std::fs::create_dir(&path).unwrap();

        let profile = Profile::new(path);
        self.profiles.items.push(profile);
    }

    pub fn rename_selected_profile(&mut self, name: &str) {
        if name.is_empty() {
            return;
        }

        let Some(selected_idx) = self.profiles.state.selected() else {
            return;
        };

        let Some(profile) = self.profiles.get_mut_selected() else {
            return;
        };

        let old_name = profile.name.clone();

        profile.name = name.to_string();
        profile.path.set_file_name(name);

        fs::rename(&old_name, &profile.path).unwrap();

        if old_name == get_selected_profile().unwrap() {
            update_selected_profile(&profile.name).unwrap();
        }

        if matches!(self.active_profile, Some(idx) if idx == selected_idx) {
            for entry in &profile.entries {
                let entry_name = entry.borrow().name();
                *entry.borrow_mut().path_mut() = profile.path.join(entry_name);
                entry.borrow_mut().update_children_path();
            }
        }
    }

    pub fn delete_selected_profile(&mut self) {
        let Some(profile) = self.profiles.get_selected() else {
            return;
        };

        std::fs::remove_dir_all(&profile.path).unwrap();

        if let Some(idx) = self.profiles.state.selected() {
            self.profiles.items.remove(idx);

            if matches!(self.active_profile, Some(active_idx) if active_idx == idx) {
                self.active_profile = None;
            }
        }
    }

    pub fn select_profile(&mut self) -> Result<()> {
        if let Some(idx) = self.active_profile {
            if matches!(self.profiles.state.selected(), Some(selected_idx) if idx == selected_idx) {
                return Ok(());
            }

            self.profiles.items[idx].entries.drain(..);
        }

        if let Some(profile) = self.profiles.get_mut_selected() {
            profile.load_entries()?;
            update_selected_profile(&profile.name)?;
            self.active_profile = self.profiles.state.selected();
            Ok(())
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
