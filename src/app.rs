use crate::{
    entry::Entry,
    input::{ConfirmationContext, Input, InputMode},
    profile::Profiles,
    utils, OPTIONS,
};
use anyhow::{Context, Result};
use ratatui::widgets::ListState;
use std::{cell::RefCell, path::Path, rc::Rc};

#[derive(Debug)]
pub struct App {
    pub profiles: Profiles,
    pub visible_entries: StatefulList<Rc<RefCell<Entry>>>,
    pub footer_input: Option<Input>,
    pub input_mode: InputMode,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            profiles: Profiles::new().unwrap(),
            visible_entries: StatefulList::with_items(Vec::new()),
            footer_input: None,
            input_mode: InputMode::Normal,
        };

        if app.profiles.get_profile().is_some() {
            app.load_entries();
        } else {
            app.select_profile();
        }

        app
    }

    fn load_entries(&mut self) {
        if let Some(profile) = self.profiles.get_profile() {
            self.visible_entries = StatefulList::with_items(profile.entries.clone());
        }
    }

    pub fn select_profile(&mut self) {
        self.input_mode = InputMode::ProfileSelection;
    }

    pub fn confirm_profile_selection(&mut self) {
        if let Ok(()) = self.profiles.select_profile() {
            self.load_entries();
            self.input_mode = InputMode::Normal;
        }
    }

    pub fn on_confirmation(&mut self, context: ConfirmationContext) {
        match context {
            ConfirmationContext::Deletion => self.delete_selected_entry(),
            ConfirmationContext::Replacing => self.replace_save_file(),
            ConfirmationContext::ProfileDeletion => {
                self.profiles.delete_selected_profile();

                if self.profiles.get_profile().is_none() {
                    self.visible_entries = StatefulList::with_items(Vec::new());
                }

                self.input_mode = InputMode::ProfileSelection;
            }
        }
    }

    pub fn prompt_for_confirmation(&mut self, context: ConfirmationContext) {
        match context {
            ConfirmationContext::Deletion | ConfirmationContext::Replacing => {
                if self.visible_entries.state.selected().is_none() {
                    return;
                }
            }
            ConfirmationContext::ProfileDeletion => {
                if self.profiles.profiles.state.selected().is_none() {
                    return;
                }
            }
        }

        self.input_mode = InputMode::Confirmation(context);
    }

    pub fn delete_selected_entry(&mut self) {
        let Some(selected_entry) = self.visible_entries.get_selected() else {
            return;
        };

        if selected_entry.borrow().is_folder() {
            std::fs::remove_dir_all(selected_entry.borrow().path()).unwrap();
        } else {
            std::fs::remove_file(selected_entry.borrow().path()).unwrap();
        }

        let idx = self.visible_entries.state.selected().unwrap();

        if let Some(parent_idx) = self.find_parent(idx) {
            let idx = self.visible_entries.items[parent_idx]
                .borrow()
                .entries()
                .iter()
                .position(|entry| Rc::ptr_eq(entry, selected_entry))
                .unwrap();

            self.close_fold_at_index(parent_idx);

            self.visible_entries.items[parent_idx]
                .borrow_mut()
                .entries_mut()
                .remove(idx);

            self.open_fold_at_index(parent_idx);
        } else {
            if selected_entry.borrow().is_folder() {
                self.close_fold_at_index(idx);
            }

            let deleted = self.visible_entries.items.remove(idx);
            let profile_entries = &mut self.profiles.get_mut_profile().unwrap().entries;
            let idx = profile_entries
                .iter()
                .position(|entry| Rc::ptr_eq(entry, &deleted))
                .unwrap();
            profile_entries.remove(idx);
        }

        self.input_mode = InputMode::Normal;
    }

    pub fn take_input(&mut self, mode: InputMode) {
        self.footer_input = Some(Input::new(&mode));
        self.input_mode = mode;
    }

    pub fn create_folder(&mut self) {
        let text = std::mem::take(&mut self.footer_input.as_mut().unwrap().text);

        if matches!(self.input_mode, InputMode::FolderCreation(true))
            || !self
                .visible_entries
                .get_selected()
                .is_some_and(|entry| !(entry.borrow().depth() == 0 && entry.borrow().is_file()))
        {
            let profile = self.profiles.get_mut_profile().unwrap();
            let path = profile.path.join(text);

            std::fs::create_dir(&path).unwrap();
            let entry = Rc::new(RefCell::new(Entry::new(path, 0).unwrap()));
            profile.entries.push(entry.clone());
            self.visible_entries.items.push(entry);
        } else {
            let Some(selected_idx) = self.visible_entries.state.selected() else {
                return;
            };
            let idx = self.find_context(selected_idx).unwrap();
            self.close_fold_at_index(idx);

            if let Some(entry) = self.visible_entries.items.get(idx) {
                let path = entry.borrow().path().join(text);
                let depth = entry.borrow().depth();

                std::fs::create_dir(&path).unwrap();
                let child = Rc::new(RefCell::new(Entry::new(path, depth + 1).unwrap()));
                entry.borrow_mut().insert_to_folder(child);
            }

            self.open_fold_at_index(idx);
        }

        self.footer_input = None;
        self.input_mode = InputMode::Normal;
    }

    pub fn enter_renaming(&mut self) {
        let Some(entry) = self.visible_entries.get_selected() else {
            return;
        };

        self.input_mode = InputMode::EntryRenaming;
        self.footer_input = Some(Input::with_text(&entry.borrow().name()));
    }

    pub fn rename_selected_entry(&mut self) {
        let entry = self.visible_entries.get_selected().unwrap();
        let text = &self.footer_input.as_ref().unwrap().text;

        let old_path = entry.borrow().path();
        entry.borrow_mut().rename(text).unwrap();

        match self.profiles.get_profile() {
            Some(profile) if old_path == profile.get_selected_save_file().unwrap() => {
                profile
                    .update_selected_save_file(&entry.borrow().path())
                    .unwrap();
            }
            _ => {}
        };

        self.footer_input = None;
        self.input_mode = InputMode::Normal;
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

    pub fn load_selected_save_file(&self) {
        if let Some(entry) = self.visible_entries.get_selected() {
            if let Entry::File { ref path, .. } = *entry.borrow() {
                self.load_save_file(path);
            }
        }
    }

    pub fn mark_selected_save_file(&self) {
        if let Some(entry) = self.visible_entries.get_selected() {
            if let Entry::File { ref path, .. } = *entry.borrow() {
                let profile = self.profiles.get_profile().unwrap();
                profile.update_selected_save_file(path).unwrap();
            }
        }
    }

    pub fn load_previous_save_file(&self) -> Result<()> {
        match self.profiles.get_profile() {
            Some(profile) => profile
                .get_selected_save_file()
                .map(|path| self.load_save_file(&path))
                .context("No previous save file exists for the selected profile."),
            None => Err(anyhow::anyhow!("No profile is selected.")),
        }
    }

    pub fn load_save_file(&self, path: &Path) {
        if let Some(profile) = self.profiles.get_profile() {
            std::fs::copy(path, &OPTIONS.save_file_path).unwrap();
            profile.update_selected_save_file(path).unwrap();
        }
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
            utils::verify_name(&mut path);

            std::fs::copy(&save_file_path, &path).unwrap();
            let entry = Rc::new(RefCell::new(Entry::new(path, 0).unwrap()));
            profile.entries.push(entry.clone());
            self.visible_entries.items.push(entry);
        } else {
            let Some(selected_idx) = self.visible_entries.state.selected() else {
                return;
            };
            let idx = self.find_context(selected_idx).unwrap();
            self.close_fold_at_index(idx);

            if let Some(entry) = self.visible_entries.items.get_mut(idx) {
                let mut path = entry
                    .borrow()
                    .path()
                    .join(save_file_path.file_name().unwrap());
                utils::verify_name(&mut path);
                let depth = entry.borrow().depth();

                std::fs::copy(&save_file_path, &path).unwrap();
                let child = Rc::new(RefCell::new(Entry::new(path, depth + 1).unwrap()));
                entry.borrow_mut().insert_to_folder(child);
            }

            self.open_fold_at_index(idx);
        }
    }

    pub fn replace_save_file(&mut self) {
        if let Some(entry) = self.visible_entries.get_selected() {
            if entry.borrow().is_file() {
                std::fs::copy(&OPTIONS.save_file_path, entry.borrow().path()).unwrap();
            }
        }

        self.input_mode = InputMode::Normal;
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
