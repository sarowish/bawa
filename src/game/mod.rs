use crate::{
    app::StatefulList,
    entry::Entry,
    tree::{NodeId, Tree},
    utils,
    watcher::HandleFileSystemEvent,
};
use anyhow::Result;
use profile::Profile;
use std::{
    fmt::Display,
    fs::{self, File},
    path::PathBuf,
};
use std::{io::Write, path::Path};

pub mod creation;
pub mod profile;
pub mod state;

pub fn read_games() -> Result<Vec<Game>> {
    Ok(utils::get_state_dir()?
        .read_dir()?
        .flatten()
        .filter(|dir_entry| dir_entry.file_type().unwrap().is_dir())
        .map(|dir_entry| Game::new(dir_entry.path()))
        .collect())
}

pub fn get_active_game_file() -> Result<PathBuf> {
    Ok(utils::get_state_dir()?.join("active_game"))
}

pub fn update_active_game(game_name: &str) -> Result<()> {
    let file_path = get_active_game_file()?;
    let mut file = File::create(file_path)?;
    Ok(writeln!(file, "{game_name}")?)
}
pub fn get_active_game() -> Result<String> {
    Ok(fs::read_to_string(get_active_game_file()?)?
        .trim()
        .to_owned())
}

pub struct Game {
    pub path: PathBuf,
    pub savefile_path: Option<PathBuf>,
    pub profiles: StatefulList<Profile>,
    pub active_profile: Option<usize>,
}

impl Game {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            savefile_path: None,
            profiles: StatefulList::with_items(Vec::new()),
            active_profile: None,
        }
    }

    pub fn name(&self) -> std::borrow::Cow<'_, str> {
        self.path.file_name().unwrap().to_string_lossy()
    }

    pub fn read_profiles(&self) -> Result<Vec<Profile>> {
        Ok(self
            .path
            .read_dir()?
            .flatten()
            .filter(|dir_entry| dir_entry.file_type().unwrap().is_dir())
            .map(|dir_entry| Profile::new(dir_entry.path()))
            .collect())
    }

    fn load_profiles(&mut self) -> Result<()> {
        let mut profiles = self.read_profiles()?;
        let state_file = self.path.join("_state");

        if let Some(state) = fs::read(state_file)
            .ok()
            .and_then(|s| bincode::deserialize::<state::GameState>(&s).ok())
        {
            self.savefile_path = state.savefile_path.map(PathBuf::from);
            if let Some(name) = state.active_profile {
                self.active_profile = profiles.iter().position(|profile| profile.name() == name);

                if let Some(idx) = self.active_profile {
                    profiles[idx].load_entries()?;
                }
            }
        }

        self.profiles = StatefulList::with_items(profiles);

        Ok(())
    }

    pub fn write_state(&self) -> Result<()> {
        utils::write_atomic(&self.path.join("_state"), &bincode::serialize(self)?)
    }

    pub fn create_profile(&self, name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        let path = self.path.join(name);
        utils::check_for_dup(&path)?;
        std::fs::create_dir(&path)?;

        Ok(())
    }

    pub fn rename_selected_profile(&self, new_name: &str) -> Result<()> {
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

    pub fn delete_selected_profile(&self) -> Result<()> {
        if let Some(profile) = self.profiles.get_selected() {
            std::fs::remove_dir_all(&profile.path)?;
        }

        Ok(())
    }

    pub fn select_profile(&mut self) -> Result<bool> {
        if self.profiles.get_selected().is_none() {
            return Err(anyhow::anyhow!("Can't select profile"));
        }

        if let Some(idx) = self.active_profile {
            if matches!(self.profiles.state.selected(), Some(selected_idx) if idx == selected_idx) {
                return Ok(false);
            }

            self.profiles.items[idx].entries.empty();
        }

        let profile = self.profiles.get_selected_mut().unwrap();
        profile.load_entries()?;
        self.update_active_profile(self.profiles.state.selected())?;

        Ok(true)
    }

    pub fn set_savefile_path(&mut self, savefile_path: &str) -> Result<()> {
        self.savefile_path = Some(PathBuf::from(savefile_path));
        self.write_state()
    }

    pub fn update_active_profile(&mut self, profile_idx: Option<usize>) -> Result<()> {
        self.active_profile = profile_idx;
        self.write_state()
    }

    pub fn get_profile(&self) -> Option<&Profile> {
        self.profiles.items.get(self.active_profile?)
    }

    pub fn get_profile_mut(&mut self) -> Option<&mut Profile> {
        self.profiles.items.get_mut(self.active_profile?)
    }

    pub fn get_entries(&self) -> Option<&Tree<Entry>> {
        self.get_profile().map(|profile| &profile.entries)
    }

    pub fn get_entries_mut(&mut self) -> Option<&mut Tree<Entry>> {
        self.get_profile_mut().map(|profile| &mut profile.entries)
    }
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl HandleFileSystemEvent for Game {
    fn on_create(&mut self, path: &Path) -> Result<()> {
        self.profiles.items.push(Profile::new(path.to_owned()));

        Ok(())
    }

    fn on_rename(&mut self, path: &Path, new_path: &Path) -> Result<()> {
        let profiles = &mut self.profiles.items;

        let Some(idx) = profiles.iter().position(|profile| profile.path == path) else {
            return Ok(());
        };

        let profile = &mut profiles[idx];
        new_path.clone_into(&mut profile.path);

        if matches!(self.active_profile, Some(active_idx) if active_idx == idx) {
            profile.entries.update_paths(NodeId::root(), new_path)?;
            self.write_state()?;
        }

        Ok(())
    }

    fn on_delete(&mut self, path: &Path) -> Result<()> {
        if !self.path.exists() {
            return Ok(());
        }

        let profiles = &self.profiles.items;

        if let Some(idx) = profiles.iter().position(|profile| profile.path == path) {
            self.profiles.items.remove(idx);

            if matches!(self.active_profile, Some(active_idx) if active_idx == idx) {
                self.update_active_profile(None)?;
            }
        }

        Ok(())
    }
}

pub struct Games {
    pub inner: StatefulList<Game>,
    pub active_game: Option<usize>,
}

impl Games {
    pub fn new() -> Result<Self> {
        let mut games = read_games()?;
        let mut active_game = None;

        if let Ok(name) = get_active_game() {
            active_game = games.iter().position(|game| game.name() == name);

            if let Some(idx) = active_game {
                games[idx].load_profiles()?;
            }
        }

        Ok(Self {
            inner: StatefulList::with_items(games),
            active_game,
        })
    }

    pub fn create_game(name: &str, savefile_path: &Path) -> Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        let path = utils::get_state_dir()?.join(name);
        utils::check_for_dup(&path)?;
        std::fs::create_dir(&path)?;

        let mut game = Game::new(path);
        game.savefile_path = Some(savefile_path.to_owned());
        game.write_state()?;

        Ok(())
    }

    pub fn rename_selected_game(&mut self, new_name: &str) -> Result<()> {
        if new_name.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        if let Some(game) = self.inner.get_selected() {
            let mut new_path = game.path.clone();
            new_path.set_file_name(new_name);
            utils::rename(&game.path, &new_path)?;
        }

        Ok(())
    }

    pub fn delete_selected_game(&mut self) -> Result<()> {
        if let Some(game) = self.inner.get_selected() {
            std::fs::remove_dir_all(&game.path)?;
        }

        Ok(())
    }

    pub fn select_game(&mut self) -> Result<bool> {
        if self.inner.get_selected().is_none() {
            return Err(anyhow::anyhow!("Can't select game"));
        }

        if let Some(idx) = self.active_game {
            if matches!(self.inner.state.selected(), Some(selected_idx) if idx == selected_idx) {
                return Ok(false);
            }

            self.inner.items[idx].profiles.items.drain(..);
        }

        let game = self.inner.get_selected_mut().unwrap();
        game.load_profiles()?;
        update_active_game(&game.name())?;
        self.active_game = self.inner.state.selected();

        Ok(true)
    }

    pub fn get_game(&self) -> Option<&Game> {
        self.inner.items.get(self.active_game?)
    }

    pub fn get_game_mut(&mut self) -> Option<&mut Game> {
        self.inner.items.get_mut(self.active_game?)
    }

    pub fn get_game_unchecked(&self) -> &Game {
        &self.inner.items[self.active_game.unwrap()]
    }

    pub fn get_game_unchecked_mut(&mut self) -> &mut Game {
        &mut self.inner.items[self.active_game.unwrap()]
    }

    pub fn get_profiles(&self) -> &StatefulList<Profile> {
        &self.inner.items[self.active_game.unwrap()].profiles
    }

    pub fn get_profiles_mut(&mut self) -> &mut StatefulList<Profile> {
        &mut self.inner.items[self.active_game.unwrap()].profiles
    }

    pub fn get_profile(&self) -> Option<&Profile> {
        self.get_game().and_then(Game::get_profile)
    }

    pub fn get_profile_mut(&mut self) -> Option<&mut Profile> {
        self.get_game_mut().and_then(Game::get_profile_mut)
    }

    pub fn get_entries(&self) -> Option<&Tree<Entry>> {
        self.get_game().and_then(Game::get_entries)
    }

    pub fn get_entries_mut(&mut self) -> Option<&mut Tree<Entry>> {
        self.get_game_mut().and_then(Game::get_entries_mut)
    }
}

impl HandleFileSystemEvent for Games {
    fn on_create(&mut self, path: &Path) -> Result<()> {
        self.inner.items.push(Game::new(path.to_owned()));

        Ok(())
    }

    fn on_rename(&mut self, path: &Path, new_path: &Path) -> Result<()> {
        let games = &mut self.inner.items;

        let Some(idx) = games.iter().position(|game| game.path == path) else {
            return Ok(());
        };

        let game = &mut games[idx];
        new_path.clone_into(&mut game.path);

        if matches!(self.active_game, Some(active_idx) if active_idx == idx) {
            for profile in &mut game.profiles.items {
                profile.path = new_path.join(profile.name().as_ref());
                profile
                    .entries
                    .update_paths(NodeId::root(), &profile.path)?;
            }

            let new_name = new_path.file_name().unwrap().to_string_lossy();
            update_active_game(&new_name)?;
        }

        Ok(())
    }

    fn on_delete(&mut self, path: &Path) -> Result<()> {
        let games = &self.inner.items;
        if let Some(idx) = games.iter().position(|game| game.path == path) {
            self.inner.items.remove(idx);

            if matches!(self.active_game, Some(active_idx) if active_idx == idx) {
                self.active_game = None;
            }
        }

        Ok(())
    }
}
