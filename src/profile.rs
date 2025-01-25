use crate::app::StatefulList;
use crate::entry::Entry;
use crate::utils;
use crate::watcher::HandleFileSystemEvent;
use anyhow::{Context, Result};
use std::fmt::Display;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

pub fn get_profiles() -> Result<Vec<Profile>> {
    Ok(utils::get_state_dir()?
        .read_dir()?
        .flatten()
        .filter(|dir_entry| dir_entry.file_type().unwrap().is_dir())
        .map(|dir_entry| Profile::new(dir_entry.path()))
        .collect())
}

pub fn get_active_profile_file() -> Result<PathBuf> {
    let state_dir = utils::get_state_dir()?;

    Ok(state_dir.join("active_profile"))
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

#[derive(Debug)]
pub struct Profile {
    pub folder: Entry,
    active_save_file: Option<PathBuf>,
}

impl Profile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            folder: Entry::Folder {
                path,
                entries: Vec::new(),
                is_fold_opened: true,
                depth: 0,
            },
            active_save_file: None,
        }
    }

    pub fn name(&self) -> std::borrow::Cow<'_, str> {
        self.folder.name().to_string_lossy()
    }

    pub fn path(&self) -> &Path {
        match &self.folder {
            Entry::Folder { path, .. } => path,
            Entry::File { .. } => unreachable!(),
        }
    }

    pub fn load_entries(&mut self) -> Result<()> {
        *self.folder.entries_mut() = Entry::entries_from_path(self.path(), 0)?;
        self.read_active_save_file();

        Ok(())
    }

    fn get_active_save_path(&self) -> PathBuf {
        self.abs_path_to("active_save_file")
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
        self.active_save_file = Some(path.to_owned());

        Ok(())
    }

    fn write_active_save_file(&self, path: &Path) -> Result<()> {
        let file_path = self.get_active_save_path();
        let mut file = File::create(file_path)?;
        Ok(writeln!(file, "{}", self.rel_path_to(path))?)
    }

    pub fn read_active_save_file(&mut self) {
        if let Ok(path) = fs::read_to_string(self.get_active_save_path())
            .map(|path| self.abs_path_to(path.trim()))
        {
            self.active_save_file = Some(path);
        }
    }

    pub fn abs_path_to<A: AsRef<Path>>(&self, path: A) -> PathBuf {
        self.path().join(path)
    }

    pub fn rel_path_to(&self, entry_path: &Path) -> String {
        utils::get_relative_path(self.path(), entry_path).unwrap()
    }

    pub fn get_file_rel_paths(&self, include_folders: bool) -> Vec<String> {
        let mut paths = Vec::new();

        for entry in self.folder.descendants(true) {
            let entry = entry.borrow();
            let is_file = entry.is_file();
            if include_folders || is_file {
                let mut path = self.rel_path_to(entry.path());

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
        write!(f, "{}", self.name())
    }
}

#[derive(Debug)]
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
            let mut new_path = profile.path().to_owned();
            new_path.set_file_name(new_name);
            utils::rename(profile.path(), &new_path)?;
        }

        Ok(())
    }

    pub fn delete_selected_profile(&mut self) -> Result<()> {
        if let Some(profile) = self.inner.get_selected() {
            std::fs::remove_dir_all(profile.path())?;
        }

        Ok(())
    }

    pub fn select_profile(&mut self) -> Result<bool> {
        if let Some(idx) = self.active_profile {
            if matches!(self.inner.state.selected(), Some(selected_idx) if idx == selected_idx) {
                return Ok(false);
            }

            self.inner.items[idx].folder.entries_mut().drain(..);
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

    pub fn get_mut_profile(&mut self) -> Option<&mut Profile> {
        self.inner.items.get_mut(self.active_profile?)
    }
}

impl HandleFileSystemEvent for Profiles {
    fn on_create(&mut self, path: &Path) -> Result<()> {
        self.inner.items.push(Profile::new(path.to_owned()));

        Ok(())
    }

    fn on_rename(&mut self, path: &Path, new_path: &Path) -> Result<()> {
        let profiles = &mut self.inner.items;
        let Some(idx) = profiles.iter().position(|profile| profile.path() == path) else {
            return Ok(());
        };

        let profile = &mut profiles[idx];
        let new_name = new_path.file_name().unwrap().to_string_lossy();

        if matches!(get_active_profile(), Ok(active_profile) if active_profile == profile.name()) {
            update_active_profile(&new_name)?;
        }

        if matches!(self.active_profile, Some(active_idx) if active_idx == idx) {
            profile.folder.rename(new_path);
        }

        Ok(())
    }

    fn on_delete(&mut self, path: &Path) -> Result<()> {
        let profiles = &self.inner.items;
        if let Some(idx) = profiles.iter().position(|profile| profile.path() == path) {
            self.inner.items.remove(idx);

            if matches!(self.active_profile, Some(active_idx) if active_idx == idx) {
                self.active_profile = None;
            }
        };

        Ok(())
    }
}
