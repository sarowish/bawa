use crate::{
    entry::{Entry, RcEntry},
    event::Event,
    help::Help,
    input::{self, ConfirmationContext, Input, Mode},
    message::Message,
    profile::Profiles,
    term, ui, utils,
    watcher::{
        Context as EventContext, FileSystemEvent, HandleFileSystemEvent, Kind as EventKind, Watcher,
    },
    OPTIONS,
};
use anyhow::{Context, Result};
use crossterm::event::{Event as CrosstermEvent, EventStream};
use futures::StreamExt;
use ratatui::widgets::ListState;
use std::{cell::RefCell, path::Path, rc::Rc};
use tokio::sync::mpsc::{self, UnboundedReceiver};

pub struct App {
    pub profiles: Profiles,
    pub visible_entries: StatefulList<Rc<RefCell<Entry>>>,
    pub footer_input: Option<Input>,
    pub mode: Mode,
    pub help: Help,
    pub message: Message,
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
            footer_input: None,
            mode: Mode::Normal,
            help: Help::new(),
            watcher: Watcher::new(tx.clone())?,
            rx,
        };

        app.watcher.watch_profiles(&utils::get_data_dir()?);

        if let Some(profile) = app.profiles.get_profile() {
            app.watcher.watch_profile_entries(&profile.path);
            app.load_entries();
        } else {
            app.select_profile();
        }

        Ok(app)
    }

    pub async fn run(mut self) -> Result<()> {
        let mut terminal = ratatui::init();
        let mut term_events = EventStream::new();

        loop {
            terminal.draw(|f| ui::draw(f, &mut self))?;
            term::set_cursor(&self, &mut terminal);

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
                        self.message.set_error(e.to_string());
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
                (profile.path == event.path, &event.kind)
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
            self.visible_entries = StatefulList::from(&profile.entries);
        }
    }

    pub fn select_profile(&mut self) {
        self.mode = Mode::ProfileSelection;
    }

    pub fn confirm_profile_selection(&mut self) {
        let old_path = self
            .profiles
            .get_profile()
            .map(|profile| profile.path.clone());

        if let Ok(selected_new_profile) = self.profiles.select_profile() {
            if selected_new_profile {
                self.load_entries();
                self.watcher
                    .watch_profile_entries(&self.profiles.get_profile().unwrap().path);

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
            self.message.set_error(e.to_string());
        }
    }

    pub fn prompt_for_confirmation(&mut self, context: ConfirmationContext) {
        match context {
            ConfirmationContext::Deletion if self.visible_entries.state.selected().is_none() => {}
            ConfirmationContext::Replacing if matches!(self.visible_entries.get_selected(), Some(entry) if entry.borrow().is_folder()) =>
                {}
            ConfirmationContext::ProfileDeletion
                if self.profiles.profiles.state.selected().is_none() => {}
            _ => self.mode = Mode::Confirmation(context),
        }
    }

    pub fn delete_selected_entry(&mut self) -> Result<()> {
        let Some(selected_entry) = self.visible_entries.get_selected() else {
            return Ok(());
        };

        if selected_entry.borrow().is_folder() {
            std::fs::remove_dir_all(selected_entry.borrow().path())?;
        } else {
            std::fs::remove_file(selected_entry.borrow().path())?;
        }

        self.mode = Mode::Normal;
        Ok(())
    }

    pub fn take_input(&mut self, mode: Mode) {
        self.footer_input = Some(Input::new(&mode));
        self.mode = mode;
    }

    pub fn create_folder(&mut self) -> Result<()> {
        let text = std::mem::take(&mut self.footer_input.as_mut().unwrap().text);

        if matches!(self.mode, Mode::FolderCreation(true))
            || !self
                .visible_entries
                .get_selected()
                .is_some_and(|entry| !(entry.borrow().depth() == 0 && entry.borrow().is_file()))
        {
            let profile = self.profiles.get_mut_profile().unwrap();
            let path = profile.path.join(text);

            std::fs::create_dir(&path)?;
        } else {
            let Some(selected_idx) = self.visible_entries.state.selected() else {
                return Ok(());
            };

            let idx = self.find_context(selected_idx).unwrap();

            if let Some(entry) = self.visible_entries.items.get(idx) {
                let path = entry.borrow().path().join(text);
                std::fs::create_dir(&path)?;
            }

            self.open_fold_at_index(idx);
        }

        self.footer_input = None;
        self.mode = Mode::Normal;
        Ok(())
    }

    pub fn enter_renaming(&mut self) {
        let Some(entry) = self.visible_entries.get_selected() else {
            return;
        };

        self.mode = Mode::EntryRenaming;
        self.footer_input = Some(Input::with_text(&entry.borrow().file_name()));
    }

    pub fn rename_selected_entry(&mut self) -> Result<()> {
        let entry = self.visible_entries.get_selected().unwrap();
        let text = &self.footer_input.as_ref().unwrap().text;

        if text.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        let old_path = entry.borrow().path();

        let mut new_path = old_path.clone();
        new_path.set_file_name(text);

        utils::rename(&old_path, new_path)?;

        self.footer_input = None;
        self.mode = Mode::Normal;
        Ok(())
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
        let depth = if let Some(entry) = self.visible_entries.items.get(idx) {
            entry.borrow().depth()
        } else {
            return None;
        };

        if depth == 0 {
            return None;
        }

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

            entry.borrow().children()
        } else {
            return false;
        };

        for entry in &children {
            *entry.borrow_mut().last_item_mut() = false;
        }

        if let Some(entry) = children.last() {
            *entry.borrow_mut().last_item_mut() = true;
        }

        idx += 1;
        self.visible_entries.items.splice(idx..idx, children);

        true
    }

    fn close_fold_at_index(&mut self, mut idx: usize) -> bool {
        let children_len = if let Some(entry) = self.visible_entries.items.get(idx) {
            let children_len = entry.borrow().children_len();

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
                    .children_len();
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
                    .children_len();
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

        if OPTIONS.auto_mark_save_file {
            self.mark_selected_save_file();
        }
    }
    pub fn on_right(&mut self) {
        if let Some(idx) = self.visible_entries.state.selected() {
            self.open_fold_at_index(idx);
        }
    }

    pub fn on_down(&mut self) {
        self.visible_entries.next();

        if OPTIONS.auto_mark_save_file {
            self.mark_selected_save_file();
        }
    }

    pub fn select_first(&mut self) {
        self.visible_entries.select_first();

        if OPTIONS.auto_mark_save_file {
            self.mark_selected_save_file();
        }
    }

    pub fn select_last(&mut self) {
        self.visible_entries.select_last();

        if OPTIONS.auto_mark_save_file {
            self.mark_selected_save_file();
        }
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

    pub fn load_selected_save_file(&mut self) {
        if let Some(entry) = self.visible_entries.get_selected() {
            if let Entry::File { ref path, .. } = *entry.borrow() {
                if let Err(e) = self.load_save_file(path) {
                    self.message.set_error(e.to_string());
                    return;
                }

                self.message.set_message_with_timeout(
                    format!(
                        "Loaded {}",
                        utils::get_relative_path(&self.profiles.get_profile().unwrap().path, path)
                            .unwrap()
                    ),
                    5,
                );
            }
        }
    }

    pub fn mark_selected_save_file(&mut self) {
        if let Some(entry) = self.visible_entries.get_selected() {
            if let Entry::File { ref path, .. } = *entry.borrow() {
                let profile = self.profiles.get_profile().unwrap();
                if let Err(e) = profile.update_selected_save_file(path) {
                    self.message.set_error(e.to_string());
                }
            }
        }
    }

    pub fn load_previous_save_file(&self) -> Result<()> {
        match self.profiles.get_profile() {
            Some(profile) => profile
                .get_selected_save_file()
                .context("No previous save file exists for the selected profile.")
                .map(|path| self.load_save_file(&path))?,
            None => Err(anyhow::anyhow!("No profile is selected.")),
        }
    }

    fn load_save_file(&self, path: &Path) -> Result<()> {
        if let Some(profile) = self.profiles.get_profile() {
            std::fs::copy(path, &OPTIONS.save_file_path).context("couldn't load save file")?;
            profile
                .update_selected_save_file(path)
                .context("couldn't mark as selected file")?;
        }

        Ok(())
    }

    pub fn import_save_file(&mut self, top_level: bool) {
        let save_file_path = OPTIONS.save_file_path.clone();

        if top_level
            || !self
                .visible_entries
                .get_selected()
                .is_some_and(|entry| !(entry.borrow().depth() == 0 && entry.borrow().is_file()))
        {
            let profile = self.profiles.get_mut_profile().unwrap();
            let mut path = profile.path.join(save_file_path.file_name().unwrap());
            utils::validate_name(&mut path);

            if let Err(e) = std::fs::copy(&save_file_path, &path) {
                self.message.set_error(e.to_string());
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
                    self.message.set_error(e.to_string());
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
}

impl HandleFileSystemEvent for App {
    fn on_create(&mut self, path: &Path) -> Result<()> {
        let Some(profile) = self.profiles.get_mut_profile() else {
            return Ok(());
        };

        let path_to_entry = profile.find_entry(path.parent().unwrap());
        let depth = path_to_entry.len();

        let new_entry = Rc::new(RefCell::new(Entry::new(path.to_path_buf(), depth)?));

        if depth == 0 {
            profile.entries.push(new_entry.clone());
            self.visible_entries.items.push(new_entry);
        } else {
            let parent = &path_to_entry.last().unwrap().1;
            let is_fold_opened = parent.borrow().is_fold_opened();
            if let Some(false) = is_fold_opened {
                parent.borrow_mut().insert_to_folder(new_entry);
                return Ok(());
            }

            let visible_entries = &self.visible_entries.items;
            if let Some(idx) = visible_entries
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
        let Some(profile) = self.profiles.get_mut_profile() else {
            return Ok(());
        };

        let path_to_entry = profile.find_entry(path);
        let child = &path_to_entry.last().unwrap().1;
        child.borrow_mut().rename(new_path);

        if matches!(profile.get_selected_save_file(), Ok(selected_save_file) if selected_save_file == path)
        {
            profile.update_selected_save_file(new_path)?;
        }

        Ok(())
    }
    fn on_delete(&mut self, path: &Path) {
        let Some(profile) = self.profiles.get_mut_profile() else {
            return;
        };

        let mut path_to_entry = profile.find_entry(path);

        if path_to_entry.len() == 1 {
            profile.entries.remove(path_to_entry[0].0);
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
            self.visible_entries.items.remove(idx);
        }
    }
}

#[derive(Debug)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl From<&Vec<RcEntry>> for StatefulList<RcEntry> {
    fn from(value: &Vec<RcEntry>) -> Self {
        let mut entries = Vec::new();

        for entry in value {
            entries.push(entry.clone());
            entries.append(&mut entry.borrow().children());
        }

        Self::with_items(entries)
    }
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
