use crate::{
    config::options,
    entry::{Entry, RcEntry},
    event::Event,
    help::Help,
    input::{self, ConfirmationContext, Input, Mode},
    message::Message,
    profile::Profiles,
    search::{FuzzyFinder, Search},
    ui, utils,
    watcher::{
        Context as EventContext, FileSystemEvent, HandleFileSystemEvent, Kind as EventKind, Watcher,
    },
    OPTIONS,
};
use anyhow::{Context, Result};
use crossterm::event::{Event as CrosstermEvent, EventStream};
use futures::StreamExt;
use ratatui::widgets::ListState;
use std::{
    collections::HashMap,
    path::{self, Path, PathBuf},
    rc::Rc,
};
use tokio::sync::mpsc::{self, UnboundedReceiver};

pub struct App {
    pub profiles: Profiles,
    pub visible_entries: StatefulList<RcEntry>,
    pub marked_entries: HashMap<PathBuf, RcEntry>,
    pub footer_input: Option<Input>,
    pub mode: Mode,
    pub help: Help,
    pub message: Message,
    pub search: Search,
    pub fuzzy_finder: FuzzyFinder,
    pub watcher: Watcher,
    rx: UnboundedReceiver<Event>,
}

impl App {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut app = Self {
            message: Message::new(tx.clone()),
            profiles: Profiles::new()?,
            visible_entries: StatefulList::with_items(Vec::new()),
            marked_entries: HashMap::new(),
            footer_input: None,
            mode: Mode::Normal,
            help: Help::new(),
            search: Search::default(),
            fuzzy_finder: FuzzyFinder::new(),
            watcher: Watcher::new(tx.clone())?,
            rx,
        };

        if app.profiles.get_profile().is_some() {
            app.load_entries();
        } else {
            app.select_profile();
        }

        Ok(app)
    }

    pub async fn run(mut self) -> Result<()> {
        let mut terminal = ui::init();
        let mut term_events = EventStream::new();

        self.auto_mark_save_file();
        self.watcher.watch_profiles(&utils::get_data_dir()?);

        if let Some(profile) = self.profiles.get_profile() {
            self.watcher.watch_profile_entries(profile.path());
        }

        loop {
            terminal.draw(|f| ui::draw(f, &mut self))?;

            let event = tokio::select! {
                Some(Ok(term_event)) = term_events.next() => Event::Crossterm(term_event),
                Some(event) = self.rx.recv() => event,
            };

            match event {
                Event::Crossterm(term_event) => {
                    if let CrosstermEvent::Key(key) = term_event {
                        if input::handle_event(key, &mut self) {
                            break;
                        }
                    }
                }
                Event::FileSystem(event) => {
                    let res = match event.context {
                        EventContext::Profile => self.on_profile_event(&event),
                        EventContext::Entry => self.handle_file_system_event(&event),
                    };

                    if let Err(e) = res {
                        self.message.set_error(&e);
                    }
                }
                Event::ClearMessage => self.message.clear(),
            }
        }

        Ok(())
    }

    fn on_profile_event(&mut self, event: &FileSystemEvent) -> Result<()> {
        self.profiles.handle_file_system_event(event)?;

        if let Some(profile) = self.profiles.get_profile() {
            if let (true, EventKind::Rename(ref new_path)) =
                (profile.path() == event.path, &event.kind)
            {
                self.watcher.watch_profile_entries(new_path);
            }
        } else if self.profiles.get_profile().is_none() {
            self.select_profile();
        }

        Ok(())
    }

    fn load_entries(&mut self) {
        if let Some(profile) = self.profiles.get_profile() {
            self.visible_entries = StatefulList::with_items(profile.folder.descendants(false));
        }
    }

    pub fn select_profile(&mut self) {
        self.mode = Mode::ProfileSelection;
    }

    pub fn confirm_profile_selection(&mut self) {
        let old_path = self
            .profiles
            .get_profile()
            .map(|profile| profile.path().to_owned());

        if let Ok(selected_new_profile) = self.profiles.select_profile() {
            if selected_new_profile {
                self.load_entries();
                self.auto_mark_save_file();
                self.watcher
                    .watch_profile_entries(self.profiles.get_profile().unwrap().path());

                if let Some(path) = old_path {
                    self.watcher.unwatch(&path).unwrap();
                }
            }
            self.mode = Mode::Normal;
        }
    }

    pub fn on_confirmation(&mut self, context: ConfirmationContext) {
        let res = match context {
            ConfirmationContext::Deletion => self.delete_selected_entry(),
            ConfirmationContext::Replacing => self.replace_save_file(),
            ConfirmationContext::ProfileDeletion => {
                let res = self.profiles.delete_selected_profile();

                if res.is_ok() && self.profiles.get_profile().is_none() {
                    self.visible_entries = StatefulList::with_items(Vec::new());
                }

                self.mode = Mode::ProfileSelection;
                res
            }
        };

        if let Err(e) = res {
            self.message.set_error(&e);
        }
    }

    pub fn prompt_for_confirmation(&mut self, context: ConfirmationContext) {
        match context {
            ConfirmationContext::Deletion if self.visible_entries.state.selected().is_none() => {}
            ConfirmationContext::Replacing if matches!(self.visible_entries.get_selected(), Some(entry) if entry.borrow().is_folder()) =>
                {}
            ConfirmationContext::ProfileDeletion
                if self.profiles.inner.state.selected().is_none() => {}
            _ => self.mode = Mode::Confirmation(context),
        }
    }

    pub fn delete_selected_entry(&mut self) -> Result<()> {
        if !self.marked_entries.is_empty() {
            for (_, entry) in self.marked_entries.drain() {
                entry.borrow().delete()?;
            }
        } else if let Some(selected_entry) = self.visible_entries.get_selected() {
            selected_entry.borrow().delete()?;
        } else {
            return Ok(());
        };

        self.mode = Mode::Normal;
        Ok(())
    }

    pub fn take_input(&mut self, mode: Mode) {
        self.footer_input = Some(Input::from(&mode));
        self.mode = mode;
        self.message.clear();
    }

    pub fn extract_input(&mut self) -> String {
        self.mode.select_previous();
        self.footer_input.take().unwrap().text
    }

    pub fn abort_input(&mut self) {
        self.mode.select_previous();
        self.footer_input = None;
    }

    pub fn create_folder(&mut self, top_level: bool) -> Result<()> {
        let file_name = self.extract_input();

        if file_name.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        if top_level
            || self
                .visible_entries
                .get_selected()
                .is_none_or(|entry| (entry.borrow().depth() == 0 && entry.borrow().is_file()))
        {
            let profile = self.profiles.get_mut_profile().unwrap();
            let path = profile.abs_path_to(&file_name);
            utils::check_for_dup(&path)?;
            std::fs::create_dir(&path)?;
        } else {
            let Some(selected_idx) = self.visible_entries.state.selected() else {
                return Ok(());
            };

            let idx = self.find_context(selected_idx).unwrap();

            if let Some(entry) = self.visible_entries.items.get(idx) {
                let path = entry.borrow().path().join(file_name);
                utils::check_for_dup(&path)?;
                std::fs::create_dir(&path)?;
            }

            self.open_fold_at_index(idx);
        }

        Ok(())
    }

    pub fn enter_renaming(&mut self) {
        let Some(entry) = self.visible_entries.get_selected() else {
            return;
        };

        let mut file_name = entry.borrow().file_name();

        if let Some(empty_opt) = &OPTIONS.rename.empty {
            if let options::RenameEmpty::All = empty_opt {
                file_name = String::new();
            } else if let Some(dot_idx) = file_name.rfind('.') {
                match empty_opt {
                    options::RenameEmpty::Stem => file_name.drain(..dot_idx),
                    options::RenameEmpty::Ext => file_name.drain((dot_idx + 1)..),
                    options::RenameEmpty::DotExt => file_name.drain(dot_idx..),
                    options::RenameEmpty::All => unreachable!(),
                };
            }
        }

        self.take_input(Mode::EntryRenaming);
        let input = self.footer_input.as_mut().unwrap();
        input.set_text(&file_name);

        match OPTIONS.rename.cursor {
            options::RenameCursor::End => (),
            options::RenameCursor::Start => input.set_idx(0),
            options::RenameCursor::BeforeExt => {
                if let Some(dot_idx) = file_name.rfind('.') {
                    input.set_idx(dot_idx);
                }
            }
        }
    }

    pub fn rename_selected_entry(&mut self) -> Result<()> {
        let new_name = self.extract_input();
        let entry = self.visible_entries.get_selected().unwrap();

        if new_name.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        let old_path = entry.borrow().path();

        let mut new_path = old_path.clone();
        new_path.set_file_name(new_name);

        utils::rename(&old_path, &new_path)
    }

    pub fn move_entries(&mut self, top_level: bool) {
        let base_path = if top_level
            || self
                .visible_entries
                .get_selected()
                .is_none_or(|entry| (entry.borrow().depth() == 0 && entry.borrow().is_file()))
        {
            self.profiles.get_profile().unwrap().path()
        } else {
            let selected_idx = self.visible_entries.state.selected().unwrap();
            let idx = self.find_context(selected_idx).unwrap();
            &self.visible_entries.items[idx].borrow().path()
        };

        let mut fail = false;

        for (_, entry) in self.marked_entries.drain() {
            let new_path = base_path.join(entry.borrow().file_name());
            if utils::check_for_dup(&new_path).is_err()
                || std::fs::rename(entry.borrow().path(), new_path).is_err()
            {
                fail = true;
            };
        }

        if fail {
            self.message
                .set_error_from_str("Couldn't move some of the files");
        }
    }

    fn find_context(&self, idx: usize) -> Option<usize> {
        let entry = self.visible_entries.items.get(idx)?;

        if entry.borrow().is_folder() {
            Some(idx)
        } else {
            self.find_parent(idx)
        }
    }

    fn find_parent(&self, mut idx: usize) -> Option<usize> {
        let depth = self
            .visible_entries
            .items
            .get(idx)
            .map(|entry| entry.borrow().depth())
            .filter(|depth| *depth > 0)?;

        idx -= 1;

        while let Some(entry) = self.visible_entries.items.get(idx) {
            if entry.borrow().depth() < depth {
                return Some(idx);
            }

            idx -= 1;
        }

        None
    }

    fn open_fold_at_index(&mut self, mut idx: usize) -> bool {
        let children = if let Some(entry) = self.visible_entries.items.get(idx) {
            if let Entry::Folder {
                ref mut is_fold_opened,
                ..
            } = *entry.borrow_mut()
            {
                if *is_fold_opened {
                    return false;
                }

                *is_fold_opened = true;
            }

            entry.borrow().descendants(false)
        } else {
            return false;
        };

        idx += 1;
        self.visible_entries.items.splice(idx..idx, children);

        true
    }

    fn close_fold_at_index(&mut self, mut idx: usize) -> bool {
        let children_len = if let Some(entry) = self.visible_entries.items.get(idx) {
            let children_len = entry.borrow().descendants_len();

            if let Entry::Folder {
                ref mut is_fold_opened,
                ..
            } = *entry.borrow_mut()
            {
                if !*is_fold_opened {
                    return false;
                }

                *is_fold_opened = false;
            }

            children_len
        } else {
            return false;
        };

        idx += 1;
        self.visible_entries.items.drain(idx..(idx + children_len));

        true
    }

    pub fn open_all_folds(&mut self) {
        let Some(mut selected_idx) = self.visible_entries.state.selected() else {
            // if no entry is selected, assume that the profile is empty
            return;
        };

        let mut idx = 0;

        while self.visible_entries.items.get(idx).is_some() {
            if self.open_fold_at_index(idx) && selected_idx > idx {
                selected_idx += self
                    .visible_entries
                    .items
                    .get(idx)
                    .unwrap()
                    .borrow()
                    .descendants_len();
            }

            idx += 1;
        }

        self.visible_entries.select_with_index(selected_idx);
    }

    pub fn close_all_folds(&mut self) {
        let Some(mut selected_idx) = self.visible_entries.state.selected() else {
            // if no entry is selected, assume that the profile is empty
            return;
        };

        while let Some(idx) = self.find_parent(selected_idx) {
            selected_idx = idx;
        }

        // TODO: rewrite this so that open but invisible folders get closed too
        for idx in (0..self.visible_entries.items.len()).rev() {
            if selected_idx > idx {
                selected_idx -= self
                    .visible_entries
                    .items
                    .get(idx)
                    .unwrap()
                    .borrow()
                    .descendants_len();
            }

            self.close_fold_at_index(idx);
        }

        self.visible_entries.select_with_index(selected_idx);
    }

    pub fn jump_to_parent(&mut self) {
        let Some(idx) = self.visible_entries.state.selected() else {
            return;
        };

        if let Some(parent_idx) = self.find_parent(idx) {
            self.visible_entries.select_with_index(parent_idx);
        }
    }

    pub fn on_left(&mut self) {
        let Some(entry) = self.visible_entries.get_selected() else {
            return;
        };

        let Some(idx) = (match *entry.borrow() {
            Entry::Folder { is_fold_opened, .. } if is_fold_opened => {
                self.visible_entries.state.selected()
            }
            _ => self.find_parent(self.visible_entries.state.selected().unwrap()),
        }) else {
            return;
        };

        self.visible_entries.select_with_index(idx);
        self.close_fold_at_index(idx);
    }

    pub fn on_up(&mut self) {
        self.visible_entries.previous();
        self.auto_mark_save_file();
    }
    pub fn on_right(&mut self) {
        if let Some(idx) = self.visible_entries.state.selected() {
            self.open_fold_at_index(idx);
        }
    }

    pub fn on_down(&mut self) {
        self.visible_entries.next();
        self.auto_mark_save_file();
    }

    pub fn select_first(&mut self) {
        self.visible_entries.select_first();
        self.auto_mark_save_file();
    }

    pub fn select_last(&mut self) {
        self.visible_entries.select_last();
        self.auto_mark_save_file();
    }

    pub fn up_directory(&mut self) {
        let Some(mut idx) = self.visible_entries.state.selected() else {
            self.select_first();
            return;
        };

        if matches!(
            *self.visible_entries.get_selected().unwrap().borrow(),
            Entry::File { depth, .. } if depth != 0
        ) {
            self.visible_entries
                .select_with_index(self.find_parent(idx).unwrap());
            return;
        }

        let selected_depth = self
            .visible_entries
            .get_selected()
            .unwrap()
            .borrow()
            .depth();

        loop {
            idx = idx.checked_sub(1).unwrap_or(
                self.visible_entries
                    .items
                    .len()
                    .checked_sub(1)
                    .unwrap_or_default(),
            );

            if let Some(entry) = self.visible_entries.items.get(idx) {
                if matches!(*entry.borrow(), Entry::Folder { depth, .. } if selected_depth >= depth)
                {
                    break;
                }
            }
        }

        self.visible_entries.select_with_index(idx);
    }

    pub fn down_directory(&mut self) {
        let Some(mut idx) = self.visible_entries.state.selected() else {
            self.select_first();
            return;
        };

        if matches!(
            *self.visible_entries.get_selected().unwrap().borrow(),
            Entry::File { depth, .. } if depth != 0
        ) {
            self.visible_entries
                .select_with_index(self.find_parent(idx).unwrap());
            self.down_directory();
            return;
        }

        let selected_depth = self
            .visible_entries
            .get_selected()
            .unwrap()
            .borrow()
            .depth();

        loop {
            idx += 1;

            if idx == self.visible_entries.items.len() {
                idx = 0;
            }

            if let Some(entry) = self.visible_entries.items.get(idx) {
                if matches!(*entry.borrow(), Entry::Folder { depth, .. } if selected_depth >= depth)
                {
                    break;
                }
            }
        }

        self.visible_entries.select_with_index(idx);
    }

    fn get_index_of_active_list(&mut self) -> Option<usize> {
        match self.mode {
            Mode::Normal => self.visible_entries.state.selected(),
            Mode::ProfileSelection => self.profiles.inner.state.selected(),
            _ => unreachable!(),
        }
    }

    fn set_index_of_active_list(&mut self, new_idx: Option<usize>) {
        if new_idx.is_none() {
            return;
        }

        match self.mode {
            Mode::Normal => {
                self.visible_entries.state.select(new_idx);
                self.auto_mark_save_file();
            }
            Mode::ProfileSelection => self.profiles.inner.state.select(new_idx),
            _ => unreachable!(),
        };
    }

    pub fn load_save_file(&mut self, path: &Path, mark_as_active: bool) -> Result<()> {
        std::fs::copy(path, &OPTIONS.save_file_path).context("couldn't load save file")?;

        let profile = self.profiles.get_mut_profile().unwrap();

        self.message
            .set_message_with_timeout(&format!("Loaded {}", profile.rel_path_to(path)), 5);

        if mark_as_active {
            profile.update_active_save_file(path)?;
        }

        Ok(())
    }

    pub fn load_selected_save_file(&mut self) {
        if let Some(entry) = self.visible_entries.get_selected() {
            if !entry.borrow().is_folder() {
                let path = entry.borrow().path();
                if let Err(e) = self.load_save_file(&path, true) {
                    self.message.set_error(&e);
                }
            }
        }
    }

    pub fn load_active_save_file(&mut self) {
        if let Some(path) = self.profiles.get_profile().unwrap().get_active_save_file() {
            if let Err(e) = self.load_save_file(&path, false) {
                self.message.set_error(&e);
            }
        } else {
            self.message
                .set_warning("No active save file exists for the selected profile.");
        };
    }

    pub fn mark_selected_save_file(&mut self) {
        if let Some(entry) = self.visible_entries.get_selected() {
            if let Entry::File { ref path, .. } = *entry.borrow() {
                let profile = self.profiles.get_mut_profile().unwrap();
                if let Err(e) = profile.update_active_save_file(path) {
                    self.message.set_error(&e);
                }
            }
        }
    }

    pub fn auto_mark_save_file(&mut self) {
        if OPTIONS.auto_mark_save_file {
            self.mark_selected_save_file();
        }
    }

    pub fn import_save_file(&mut self, top_level: bool) {
        let save_file_path = OPTIONS.save_file_path.clone();

        if top_level
            || self
                .visible_entries
                .get_selected()
                .is_none_or(|entry| (entry.borrow().depth() == 0 && entry.borrow().is_file()))
        {
            let profile = self.profiles.get_mut_profile().unwrap();
            let mut path = profile.abs_path_to(save_file_path.file_name().unwrap());
            utils::validate_name(&mut path);

            if let Err(e) = std::fs::copy(&save_file_path, &path) {
                self.message.set_error(&e.into());
            }
        } else {
            let Some(selected_idx) = self.visible_entries.state.selected() else {
                return;
            };
            let idx = self.find_context(selected_idx).unwrap();

            if let Some(entry) = self.visible_entries.items.get_mut(idx) {
                let mut path = entry
                    .borrow()
                    .path()
                    .join(save_file_path.file_name().unwrap());
                utils::validate_name(&mut path);

                if let Err(e) = std::fs::copy(&save_file_path, &path) {
                    self.message.set_error(&e.into());
                }
            }

            self.open_fold_at_index(idx);
        }
    }

    pub fn replace_save_file(&mut self) -> Result<()> {
        if let Some(entry) = self.visible_entries.get_selected() {
            if entry.borrow().is_file() {
                std::fs::copy(&OPTIONS.save_file_path, entry.borrow().path())?;
            }
        }

        self.mode = Mode::Normal;
        Ok(())
    }

    pub fn search_new_pattern(&mut self) {
        let pattern = self.extract_input();
        self.search = Search::new(&pattern);
        self.repeat_search();
    }

    fn run_search(&mut self) {
        let list = match self.mode {
            Mode::Normal => &self
                .visible_entries
                .items
                .iter()
                .map(|entry| entry.borrow().name())
                .collect::<Vec<String>>(),
            Mode::ProfileSelection => &self
                .profiles
                .inner
                .items
                .iter()
                .map(|profile| profile.name().to_owned())
                .collect::<Vec<String>>(),
            _ => unreachable!(),
        };

        self.search.search(list);

        if self.search.matches.is_empty() {
            self.message
                .set_error_from_str(&format!("Pattern not found: {}", self.search.pattern));
        } else {
            self.message.clear();
        }
    }

    pub fn repeat_search(&mut self) {
        self.run_search();

        let new_idx = if let Some(selected_idx) = self.get_index_of_active_list() {
            self.search
                .matches
                .iter()
                .find(|index| **index > selected_idx)
                .or_else(|| self.search.matches.first())
        } else {
            self.search.matches.first()
        };

        self.set_index_of_active_list(new_idx.copied());
    }

    pub fn repeat_search_backwards(&mut self) {
        self.run_search();

        let new_idx = if let Some(selected_idx) = self.get_index_of_active_list() {
            self.search
                .matches
                .iter()
                .rev()
                .find(|index| **index < selected_idx)
                .or_else(|| self.search.matches.last())
        } else {
            self.search.matches.last()
        };

        self.set_index_of_active_list(new_idx.copied());
    }

    pub fn open_fuzzy_finder(&mut self) {
        if let Some(profile) = self.profiles.get_profile() {
            self.fuzzy_finder
                .fill_paths(&profile.get_file_rel_paths(false));
            self.fuzzy_finder.update_matches();
        }
    }

    pub fn jump_to_entry(&mut self) {
        let selected_item = self.fuzzy_finder.matched_items.get_selected();
        let rel_path = selected_item.as_ref().unwrap().text.clone();
        let components = rel_path.split(path::MAIN_SEPARATOR).collect::<Vec<&str>>();
        let mut idx = 0;

        for component in components {
            while let Some(entry) = self.visible_entries.items.get(idx) {
                if entry.borrow().file_name() == component {
                    if entry.borrow().is_file() {
                        self.visible_entries.state.select(Some(idx));
                        self.auto_mark_save_file();
                        return;
                    }

                    self.open_fold_at_index(idx);
                    idx += 1;
                    break;
                }

                idx += 1;
            }
        }
    }

    pub fn mark_entry(&mut self) {
        if let Some(entry) = self.visible_entries.get_selected() {
            let path = entry.borrow().path();
            if self.marked_entries.remove(&path).is_none() {
                self.marked_entries.insert(path, entry.clone());
            }

            self.visible_entries.next();
        };
    }
}

impl HandleFileSystemEvent for App {
    fn on_create(&mut self, path: &Path) -> Result<()> {
        let Some(profile) = self.profiles.get_mut_profile() else {
            return Ok(());
        };

        let path_to_entry = profile.folder.find_entry(path.parent().unwrap());
        let depth = path_to_entry.len();

        let new_entry = Entry::new_rc(path.to_path_buf(), depth)?;

        if depth == 0 {
            profile.folder.insert_to_folder(new_entry.clone());
            self.visible_entries.items.push(new_entry);
        } else {
            let parent = &path_to_entry.last().unwrap().1;
            let folds_opened = path_to_entry
                .iter()
                .all(|(_, entry)| entry.borrow().is_fold_opened().unwrap_or_default());

            if !folds_opened {
                parent.borrow_mut().insert_to_folder(new_entry);
            } else if let Some(idx) = (self.visible_entries.items)
                .iter()
                .position(|entry| Rc::ptr_eq(entry, parent))
            {
                self.close_fold_at_index(idx);
                parent.borrow_mut().insert_to_folder(new_entry);
                self.open_fold_at_index(idx);
            }
        }

        Ok(())
    }

    fn on_rename(&mut self, path: &Path, new_path: &Path) -> Result<()> {
        let moved = path.parent() != new_path.parent();

        if let Some(entry) = self.marked_entries.remove(path) {
            self.marked_entries.insert(new_path.to_path_buf(), entry);
        }

        if moved {
            self.on_delete(path)?;
            self.on_create(new_path)?;
        }

        let Some(profile) = self.profiles.get_mut_profile() else {
            return Ok(());
        };

        if !moved {
            let path_to_entry = profile.folder.find_entry(path);
            let child = &path_to_entry.last().unwrap().1;
            child.borrow_mut().rename(new_path);
        }

        if matches!(profile.get_active_save_file(), Some(selected_save_file) if selected_save_file == path)
        {
            profile.update_active_save_file(new_path)?;
        }

        Ok(())
    }
    fn on_delete(&mut self, path: &Path) -> Result<()> {
        let Some(profile) = self.profiles.get_mut_profile() else {
            return Ok(());
        };

        if matches!(profile.get_active_save_file(), Some(active_save_file) if active_save_file == path)
        {
            profile.delete_active_save()?;
        }

        self.marked_entries.remove(path);

        let mut path_to_entry = profile.folder.find_entry(path);

        if path_to_entry.len() == 1 {
            profile.folder.entries_mut().remove(path_to_entry[0].0);
        } else {
            let (parent, child) = path_to_entry
                .last_chunk_mut::<2>()
                .map(|chunk| (&chunk[0], &chunk[1]))
                .unwrap();

            parent.1.borrow_mut().entries_mut().remove(child.0);
        }

        let items = &self.visible_entries.items;
        if let Some(idx) = items
            .iter()
            .position(|entry| Rc::ptr_eq(entry, &path_to_entry.last().unwrap().1))
        {
            self.close_fold_at_index(idx);
            self.visible_entries.items.remove(idx);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> Self {
        let mut stateful_list = StatefulList {
            state: ListState::default(),
            items,
        };

        stateful_list.select_first();

        stateful_list
    }

    fn select_with_index(&mut self, index: usize) {
        self.state.select(if self.items.is_empty() {
            None
        } else {
            Some(index)
        });
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.select_with_index(i);
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.select_with_index(i);
    }

    pub fn select_first(&mut self) {
        self.select_with_index(0);
    }

    pub fn select_last(&mut self) {
        self.select_with_index(self.items.len().checked_sub(1).unwrap_or_default());
    }

    pub fn get_selected(&self) -> Option<&T> {
        match self.state.selected() {
            Some(i) => Some(&self.items[i]),
            None => None,
        }
    }

    pub fn get_mut_selected(&mut self) -> Option<&mut T> {
        match self.state.selected() {
            Some(i) => Some(&mut self.items[i]),
            None => None,
        }
    }
}
