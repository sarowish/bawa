use crate::{
    config::{options, OPTIONS},
    entry::Entry,
    event::Event,
    fuzzy_finder::{
        picker::{Global, Local},
        FuzzyFinder,
    },
    help::Help,
    input::{self, Input, Mode},
    message::{set_msg_if_error, Message},
    profile::{Profile, Profiles},
    search::Search,
    tree::{Node, NodeId, TreeState},
    ui::{
        self,
        confirmation::{Context as ConfirmationContext, Prompt},
    },
    utils,
    watcher::{
        Context as EventContext, FileSystemEvent, HandleFileSystemEvent, Kind as EventKind, Watcher,
    },
};
use anyhow::{ensure, Context, Result};
use crossterm::event::{Event as CrosstermEvent, EventStream};
use futures::StreamExt;
use ratatui::widgets::ListState;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::{self, UnboundedReceiver};

pub struct App {
    pub profiles: Profiles,
    pub tree_state: TreeState,
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
            tree_state: TreeState::default(),
            footer_input: None,
            mode: Mode::Normal,
            help: Help::default(),
            search: Search::default(),
            fuzzy_finder: FuzzyFinder::default(),
            watcher: Watcher::new(tx.clone())?,
            rx,
        };

        if app.profiles.get_profile().is_some() {
            app.setup_state();
        } else {
            app.select_profile();
        }

        Ok(app)
    }

    pub async fn run(mut self) -> Result<()> {
        let mut terminal = ui::init();
        let mut term_events = EventStream::new();

        self.auto_mark_save_file();
        self.watcher.watch_profiles(&utils::get_state_dir()?);

        if let Some(profile) = self.profiles.get_profile() {
            self.watcher.watch_profile_entries(&profile.path);
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

                    set_msg_if_error!(self.message, res);
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

    fn setup_state(&mut self) {
        let active_path = self
            .profiles
            .get_profile()
            .and_then(Profile::get_active_save_file);

        if let Some(entries) = self.profiles.get_entries_mut() {
            self.tree_state = TreeState::default();

            for id in entries.iter_ids() {
                let node = &mut entries[id];
                node.expanded = node.is_folder().then_some(false);
                if matches!(active_path, Some(ref path) if node.path == *path) {
                    self.tree_state.active = Some(id);
                }
            }

            if let Some(root) = entries.root_mut() {
                root.toggle_fold();
            }

            self.tree_state.select_first(entries);
        }
    }

    pub fn select_profile(&mut self) {
        self.footer_input = None;
        self.fuzzy_finder.reset();
        self.mode = Mode::ProfileSelection;
    }

    pub fn confirm_profile_selection(&mut self) {
        let old_path = self
            .profiles
            .get_profile()
            .map(|profile| profile.path.clone());

        if let Ok(selected_new_profile) = self.profiles.select_profile() {
            if selected_new_profile {
                self.setup_state();
                self.auto_mark_save_file();
                self.watcher
                    .watch_profile_entries(&self.profiles.get_profile().unwrap().path);

                if let Some(path) = old_path {
                    self.watcher.unwatch(&path).unwrap();
                }
            }
            self.mode = Mode::Normal;
        }
    }

    pub fn on_confirmation(&mut self) {
        let res = match self.mode.confirmation_context() {
            ConfirmationContext::Deletion => self.delete_selected_entry(),
            ConfirmationContext::Replacing => self.replace_save_file(),
            ConfirmationContext::ProfileDeletion => self.profiles.delete_selected_profile(),
        };

        self.mode.select_previous();

        set_msg_if_error!(self.message, res);
    }

    pub fn prompt_for_confirmation(&mut self, context: ConfirmationContext) {
        match context {
            ConfirmationContext::Deletion if self.tree_state.selected.is_none() => {}
            ConfirmationContext::Replacing if matches!(self.selected_entry(), Some(entry) if entry.is_folder()) =>
                {}
            ConfirmationContext::ProfileDeletion
                if self.profiles.inner.state.selected().is_none() => {}
            _ => self.mode = Mode::Confirmation(Prompt::new(self, context)),
        }
    }

    pub fn selected_entry(&self) -> Option<&Node<Entry>> {
        self.tree_state.selected.and_then(|id| {
            self.profiles
                .get_entries()
                .and_then(|entries| entries.get(id))
        })
    }

    pub fn selected_entry_mut(&mut self) -> Option<&mut Node<Entry>> {
        self.tree_state.selected.and_then(|id| {
            self.profiles
                .get_entries_mut()
                .and_then(|entries| entries.get_mut(id))
        })
    }

    pub fn delete_selected_entry(&mut self) -> Result<()> {
        if !self.tree_state.marked.is_empty() {
            let entries = self.profiles.get_entries().unwrap();
            for id in self.tree_state.marked.drain() {
                entries[id].delete()?;
            }
        } else if let Some(entry) = self.selected_entry() {
            entry.delete()?;
        }

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

    pub fn context_path(&mut self, top_level: bool) -> PathBuf {
        let entries = self.profiles.get_entries_mut().unwrap();

        let id = (!top_level)
            .then_some(self.tree_state.selected.and_then(|id| entries.context(id)))
            .flatten()
            .unwrap_or(NodeId::root());
        let node = &mut entries[id];

        node.expanded = Some(true);
        node.path.clone()
    }

    pub fn create_folder(&mut self, top_level: bool) -> Result<()> {
        let file_name = self.extract_input();
        ensure!(!file_name.is_empty(), "Name can't be empty.");
        let path = self.context_path(top_level).join(file_name);
        utils::check_for_dup(&path)?;
        Ok(std::fs::create_dir(&path)?)
    }

    pub fn enter_renaming(&mut self) {
        let Some(entry) = self.selected_entry() else {
            return;
        };

        let mut file_name = entry.name().to_string_lossy().into_owned();

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

        if new_name.is_empty() {
            return Err(anyhow::anyhow!("Name can't be empty."));
        }

        let entry = self.selected_entry().unwrap();
        let old_path = &entry.path;
        let mut new_path = old_path.to_owned();
        new_path.set_file_name(new_name);

        utils::rename(old_path, &new_path)
    }

    pub fn move_entries(&mut self, top_level: bool) {
        let base_path = self.context_path(top_level);
        let mut fail = false;

        let entries = self.profiles.get_entries().unwrap();

        for entry in self.tree_state.marked.drain().map(|id| &entries[id]) {
            let new_path = base_path.join(entry.name());
            if utils::check_for_dup(&new_path).is_err()
                || std::fs::rename(&entry.path, new_path).is_err()
            {
                fail = true;
            };
        }

        if fail {
            self.message
                .set_error_from_str("Couldn't move some of the files");
        }
    }

    pub fn move_up(&mut self) {
        let Some((id, profile)) = (self.tree_state.selected).zip(self.profiles.get_profile_mut())
        else {
            return;
        };

        let entries = &mut profile.entries;

        if let Some(swap_with) = entries[id].previous_sibling() {
            entries.detach(id);
            entries.insert_before(swap_with, id);
        } else if let Some(swap_with) = entries.following_siblings(id).last() {
            entries.detach(id);
            entries.insert_after(swap_with, id);
        } else {
            return;
        };
    }

    pub fn move_down(&mut self) {
        let Some((id, profile)) = (self.tree_state.selected).zip(self.profiles.get_profile_mut())
        else {
            return;
        };

        let entries = &mut profile.entries;

        if let Some(swap_with) = entries[id].next_sibling() {
            entries.detach(id);
            entries.insert_after(swap_with, id);
        } else if let Some(swap_with) = entries.preceding_siblings(id).last() {
            entries.detach(id);
            entries.insert_before(swap_with, id);
        } else {
            return;
        };

        set_msg_if_error!(self.message, profile.write_state());
    }

    pub fn open_all_folds(&mut self) {
        if let Some(entries) = self.profiles.get_entries_mut() {
            entries.apply_to_nodes(|node| {
                if let Some(expanded) = node.expanded.as_mut() {
                    *expanded = true;
                }
            });
        }
    }

    pub fn close_all_folds(&mut self) {
        if let Some(entries) = self.profiles.get_entries_mut() {
            entries.apply_to_nodes(|node| {
                if let Some(expanded) = node.expanded.as_mut() {
                    *expanded = false;
                }
            });

            if let Some(id) = self.tree_state.selected.and_then(|id| {
                entries
                    .ancestors(id)
                    .filter(|id| *id != NodeId::root())
                    .last()
            }) {
                self.tree_state.selected = Some(id);
            }
        }
    }

    pub fn jump_to_parent(&mut self) {
        self.tree_state
            .select_unchecked(self.selected_entry().and_then(Node::non_root_parent));
    }

    pub fn on_left(&mut self) {
        let Some(entries) = self.profiles.get_entries_mut() else {
            return;
        };

        if let Some(id) = self.tree_state.selected.and_then(|id| {
            let node = &entries[id];
            node.is_expanded().then_some(id).or(node.non_root_parent())
        }) {
            self.tree_state.select_unchecked(Some(id));
            entries[id].toggle_fold();
        }
    }

    pub fn on_up(&mut self) {
        if let Some(entries) = self.profiles.get_entries() {
            self.tree_state.select_prev(entries);
            self.auto_mark_save_file();
        }
    }
    pub fn on_right(&mut self) {
        if let Some(entry) = self.selected_entry_mut() {
            if entry.is_collapsed() {
                entry.toggle_fold();
            }
        }
    }

    pub fn on_down(&mut self) {
        if let Some(entries) = self.profiles.get_entries() {
            self.tree_state.select_next(entries);
            self.auto_mark_save_file();
        }
    }

    pub fn select_first(&mut self) {
        if let Some(entries) = self.profiles.get_entries() {
            self.tree_state.select_first(entries);
            self.auto_mark_save_file();
        }
    }

    pub fn select_last(&mut self) {
        if let Some(entries) = self.profiles.get_entries() {
            self.tree_state.select_last(entries);
            self.auto_mark_save_file();
        }
    }

    pub fn up_directory(&mut self) {
        if let Some(id) = self.tree_state.selected {
            if let Some(entries) = self.profiles.get_entries() {
                self.tree_state.select_unchecked(
                    entries
                        .predecessors(id)
                        .chain(entries.children(NodeId::root()).rev())
                        .find(|id| entries[*id].is_folder() && *id != NodeId::root()),
                );
            }
        } else {
            self.select_first();
        };
    }

    pub fn down_directory(&mut self) {
        if let Some(id) = self.tree_state.selected {
            if let Some(entries) = self.profiles.get_entries() {
                self.tree_state.select_unchecked(
                    entries
                        .following_siblings(id)
                        .chain(
                            entries
                                .ancestors(id)
                                .flat_map(|id| entries.following_siblings(id)),
                        )
                        .chain(entries.children(NodeId::root()))
                        .find(|id| entries[*id].is_folder() && *id != NodeId::root()),
                );
            }
        } else {
            self.select_last();
        };
    }

    pub fn load_save_file(&mut self, path: &Path, mark_as_active: bool) -> Result<()> {
        std::fs::copy(path, &OPTIONS.save_file_path).context("couldn't load save file")?;

        let profile = self.profiles.get_profile_mut().unwrap();

        self.message
            .set_message_with_timeout(&format!("Loaded {}", profile.rel_path_to(path)), 5);

        if mark_as_active {
            profile.update_active_save_file(path)?;
            self.tree_state.active = self.tree_state.selected;
        }

        Ok(())
    }

    pub fn load_selected_save_file(&mut self) {
        if let Some(entry) = self.selected_entry() {
            if entry.is_file() {
                let path = entry.path.clone();
                set_msg_if_error!(self.message, self.load_save_file(&path, true));
            }
        }
    }

    pub fn load_active_save_file(&mut self) {
        if let Some(path) = self.profiles.get_profile().unwrap().get_active_save_file() {
            set_msg_if_error!(self.message, self.load_save_file(&path, false));
        } else {
            self.message
                .set_warning("No active save file exists for the selected profile.");
        };
    }

    pub fn mark_selected_save_file(&mut self) {
        if let Some(path) = self
            .selected_entry()
            .filter(|entry| entry.is_file())
            .map(|entry| entry.path.clone())
        {
            let profile = self.profiles.get_profile_mut().unwrap();
            set_msg_if_error!(self.message, profile.update_active_save_file(&path));
            self.tree_state.active = self.tree_state.selected;
        }
    }

    pub fn auto_mark_save_file(&mut self) {
        if OPTIONS.auto_mark_save_file {
            self.mark_selected_save_file();
        }
    }

    pub fn import_save_file(&mut self, top_level: bool) {
        let save_file_path = OPTIONS.save_file_path.clone();
        let mut path = self
            .context_path(top_level)
            .join(save_file_path.file_name().unwrap());
        utils::validate_name(&mut path);

        set_msg_if_error!(
            self.message,
            std::fs::copy(&save_file_path, &path).map_err(|err| err.into())
        );
    }

    pub fn replace_save_file(&mut self) -> Result<()> {
        if let Some(entry) = self.selected_entry() {
            if entry.is_file() {
                std::fs::copy(&OPTIONS.save_file_path, &entry.path)?;
            }
        }

        Ok(())
    }

    pub fn open_fuzzy_finder(&mut self, global: bool) {
        if global {
            match Global::new(self) {
                Ok(picker) => self.fuzzy_finder.set_picker(picker),
                Err(e) => self.message.set_error(&e),
            }
        } else {
            self.fuzzy_finder.set_picker(Local::new(self));
        };

        self.fuzzy_finder.update_matches();
    }

    pub fn jump_to_entry(&mut self) {
        if let Some(idx) = self.fuzzy_finder.selected_idx() {
            if let Some(picker) = self.fuzzy_finder.picker.take() {
                picker.jump(idx, self);
            }
        };
    }

    pub fn mark_entry(&mut self) {
        if let Some(id) = self.tree_state.selected {
            if !self.tree_state.unmark(id) {
                self.tree_state.mark(id);
            }

            self.on_down();
        }
    }
}

impl HandleFileSystemEvent for App {
    fn on_create(&mut self, path: &Path) -> Result<()> {
        let Some(entries) = self.profiles.get_entries_mut() else {
            return Ok(());
        };

        if let Some(parent_id) = path.parent().and_then(|path| entries.find_by_path(path)) {
            let new = entries.add_value(Entry::new(path));
            entries.append(parent_id, new);

            let node = &mut entries[new];
            if node.is_folder() {
                node.expanded = Some(false);
            }
        }

        Ok(())
    }

    fn on_rename(&mut self, path: &Path, new_path: &Path) -> Result<()> {
        let Some(profile) = self.profiles.get_profile_mut() else {
            return Ok(());
        };

        let entries = &mut profile.entries;

        let Some(entry_id) = entries.find_by_path(path) else {
            return Ok(());
        };

        if path.parent() != new_path.parent() {
            entries.detach(entry_id);
            if let Some(new_parent) = new_path
                .parent()
                .and_then(|path| entries.find_by_path(path))
            {
                entries.append(new_parent, entry_id);
            }
        }

        entries.update_paths(entry_id, new_path)?;

        if matches!(profile.get_active_save_file(), Some(active_path) if active_path == path) {
            profile.update_active_save_file(new_path)?;
        }

        Ok(())
    }

    fn on_delete(&mut self, path: &Path) -> Result<()> {
        let Some(profile) = self.profiles.get_profile_mut() else {
            return Ok(());
        };

        if matches!(profile.get_active_save_file(), Some(active_path) if active_path == path) {
            profile.reset_active_save_file()?;
        }

        if let Some(entry_id) = profile.entries.find_by_path(path) {
            if matches!(self.tree_state.selected, Some(id) if id == entry_id) {
                self.tree_state.select_prev(&profile.entries);
            }
            self.tree_state.unmark(entry_id);
            profile.entries.detach(entry_id);
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
        self.state.selected().and_then(|idx| self.items.get(idx))
    }

    pub fn get_selected_mut(&mut self) -> Option<&mut T> {
        self.state
            .selected()
            .and_then(|idx| self.items.get_mut(idx))
    }
}
