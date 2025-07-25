use crate::{
    config::{OPTIONS, options},
    entry::Entry,
    event::Event,
    fuzzy_finder::{
        FuzzyFinder,
        picker::{Global, Local},
    },
    game::{
        Games,
        creation::{CreatingGame, Step},
        profile::Profile,
    },
    help::Help,
    input::{self, Input, Mode},
    message::{Message, set_msg_if_error},
    search::Search,
    tree::{Node, NodeId, Tree, TreeState},
    ui::{
        self,
        confirmation::{Context as ConfirmationContext, Prompt},
    },
    utils,
    watcher::{
        Context as EventContext, FileSystemEvent, HandleFileSystemEvent, Kind as EventKind, Watcher,
    },
};
use anyhow::{Context, Result, ensure};
use crossterm::event::{Event as CrosstermEvent, EventStream};
use futures::StreamExt;
use ratatui::widgets::ListState;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::{self, UnboundedReceiver};

pub struct App {
    pub games: Games,
    pub tree_state: TreeState,
    pub footer_input: Option<Input>,
    pub mode: Mode,
    pub help: Help,
    pub message: Message,
    pub search: Search,
    pub fuzzy_finder: FuzzyFinder,
    pub game_creation: CreatingGame,
    pub watcher: Watcher,
    pending_move: Option<HandleMove>,
    rx: UnboundedReceiver<Event>,
}

impl App {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut app = Self {
            message: Message::new(tx.clone()),
            games: Games::new()?,
            tree_state: TreeState::default(),
            footer_input: None,
            mode: Mode::Normal,
            help: Help::default(),
            search: Search::default(),
            fuzzy_finder: FuzzyFinder::default(),
            game_creation: CreatingGame::default(),
            watcher: Watcher::new(tx)?,
            pending_move: None,
            rx,
        };

        if app.games.get_profile().is_some() {
            app.setup_state();
        } else {
            app.open_game_window();
        }

        Ok(app)
    }

    pub async fn run(mut self) -> Result<()> {
        let mut terminal = ui::init();
        let mut term_events = EventStream::new();

        self.auto_mark_save_file();
        self.watcher.watch_non_recursive(&utils::get_state_dir()?);

        if let Some(game) = self.games.get_game() {
            self.watcher.watch_non_recursive(&game.path);

            if let Some(profile) = self.games.get_profile() {
                self.watcher.watch_recursive(&profile.path);
            }
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
                    let Some(event) = self.watcher.handle_event(event) else {
                        continue;
                    };

                    let res = match event.context {
                        EventContext::Game => self.on_game_event(&event),
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

    fn on_game_event(&mut self, event: &FileSystemEvent) -> Result<()> {
        self.games.handle_file_system_event(event)?;

        match self.games.get_game() {
            Some(game) => match &event.kind {
                EventKind::Rename(new_path) if *new_path == game.path => {
                    self.watcher.watch_non_recursive(new_path);

                    for profile in &game.profiles.items {
                        self.watcher.watch_non_recursive(&profile.path);
                    }
                }
                _ => (),
            },
            None => self.open_game_window(),
        }

        Ok(())
    }

    fn on_profile_event(&mut self, event: &FileSystemEvent) -> Result<()> {
        if let Some(game) = self.games.get_game_mut() {
            game.handle_file_system_event(event)?;

            match game.get_profile() {
                Some(profile) => match &event.kind {
                    EventKind::Rename(new_path) if *new_path == profile.path => {
                        self.watcher.watch_recursive(new_path);
                    }
                    _ => (),
                },
                None => self.open_profile_window(),
            }
        }

        Ok(())
    }

    fn setup_state(&mut self) {
        let profile = self.games.get_profile_mut();
        let active_path = profile.as_deref().and_then(Profile::get_active_save_file);

        if let Some(entries) = profile.map(|profile| &mut profile.entries) {
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

    pub fn open_game_window(&mut self) {
        self.footer_input = None;
        self.fuzzy_finder.reset();
        self.mode = Mode::GameSelection;
    }

    pub fn open_profile_window(&mut self) {
        self.footer_input = None;
        self.fuzzy_finder.reset();
        self.mode = Mode::ProfileSelection;
    }

    pub fn confirm_game_selection(&mut self) {
        let previous_game_path = self.games.get_game().map(|profile| profile.path.clone());
        let previous_profile_path = self.games.get_profile().map(|profile| profile.path.clone());

        if let Ok(selected_new_game) = self.games.select_game() {
            let profile_selected = self.games.get_profile().is_some();

            if selected_new_game {
                self.watcher
                    .watch_non_recursive(&self.games.get_game().unwrap().path);

                if let Some(path) = previous_game_path {
                    self.watcher
                        .unwatch(&path)
                        .expect("This path should've been watched.");
                }

                if profile_selected {
                    self.on_profile_change(previous_profile_path);
                }
            }

            if !profile_selected {
                return self.open_profile_window();
            }

            self.mode = Mode::Normal;
        }
    }

    pub fn handle_game_creation(&mut self) -> Result<()> {
        let state = &mut self.game_creation;

        let input = self.footer_input.take().map(|input| input.text);

        match &state.step {
            Step::EnterName => {
                state.name = input;
                state.step = Step::PresetOrManual(false);
            }
            Step::EnterPath => {
                let path = PathBuf::from(input.unwrap());
                Games::create_game(&mut self.games, state.name.as_ref().unwrap(), &path)?;
                self.mode.select_previous();
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    pub fn confirm_profile_selection(&mut self) {
        let old_path = self.games.get_profile().map(|profile| profile.path.clone());

        let Some(game) = self.games.get_game_mut() else {
            return;
        };

        if let Ok(selected_new_profile) = game.select_profile() {
            if selected_new_profile {
                self.on_profile_change(old_path);
            }
            self.mode = Mode::Normal;
        }
    }

    pub fn on_profile_change(&mut self, previous_profile_path: Option<PathBuf>) {
        self.setup_state();
        self.auto_mark_save_file();
        self.watcher
            .watch_recursive(&self.games.get_profile().unwrap().path);

        if let Some(path) = previous_profile_path {
            self.watcher
                .unwatch(&path)
                .expect("This path should've been watched.");
        }
    }

    pub fn on_confirmation(&mut self) {
        let res = match self.mode.confirmation_context() {
            ConfirmationContext::Deletion => self.delete_selected_entry(),
            ConfirmationContext::Replacing => self.replace_save_file(),
            ConfirmationContext::GameDeletion => self.games.delete_selected_game(),
            ConfirmationContext::ProfileDeletion => self
                .games
                .get_game_unchecked_mut()
                .delete_selected_profile(),
        };

        self.mode.select_previous();

        set_msg_if_error!(self.message, res);
    }

    pub fn prompt_for_confirmation(&mut self, context: ConfirmationContext) {
        match context {
            ConfirmationContext::Deletion if self.tree_state.selected.is_none() => {}
            ConfirmationContext::Replacing if matches!(self.selected_entry(), Some(entry) if entry.is_folder()) =>
                {}
            ConfirmationContext::GameDeletion if self.games.inner.state.selected().is_none() => {}
            ConfirmationContext::ProfileDeletion
                if self.games.get_profiles().state.selected().is_none() => {}
            _ => self.mode = Mode::Confirmation(Prompt::new(self, context)),
        }
    }

    pub fn selected_entry(&self) -> Option<&Node<Entry>> {
        self.tree_state
            .selected
            .and_then(|id| self.games.get_entries().and_then(|entries| entries.get(id)))
    }

    pub fn selected_entry_mut(&mut self) -> Option<&mut Node<Entry>> {
        self.tree_state.selected.and_then(|id| {
            self.games
                .get_entries_mut()
                .and_then(|entries| entries.get_mut(id))
        })
    }

    pub fn delete_selected_entry(&mut self) -> Result<()> {
        if !self.tree_state.marked.is_empty() {
            let entries = self.games.get_entries().unwrap();
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
        let entries = self.games.get_entries_mut().unwrap();

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
        if self.tree_state.marked.is_empty() {
            return;
        }

        let Some(selected) = self.tree_state.selected else {
            return;
        };

        let base_path = self.context_path(top_level);
        let mut moved_outside: u32 = 0;
        let mut moved_in = false;
        let mut fail = false;

        let profile = self.games.get_profile_mut().unwrap();
        let entries = &mut profile.entries;

        for id in self.tree_state.marked.drain() {
            let entry = &entries[id];
            let new_path = base_path.join(entry.name());

            if entry.path == new_path {
                moved_in = true;
                if entries[selected].is_folder() && !top_level {
                    entries.move_entry(Tree::prepend, selected, id);
                } else {
                    entries.move_entry(Tree::insert_after, selected, id);
                }
            } else if utils::check_for_dup(&new_path).is_err()
                || std::fs::rename(&entry.path, new_path).is_err()
            {
                fail = true;
            } else {
                moved_outside += 1;
            }
        }

        if moved_outside != 0 {
            let mut id = selected;
            let method: fn(&mut Tree<Entry>, NodeId, NodeId);

            if top_level {
                if let Some(ancestor) = entries
                    .ancestors(id)
                    .take_while(|id| *id != NodeId::root())
                    .last()
                {
                    id = ancestor;
                }
                method = Tree::insert_after;
            } else {
                method = if entries[id].is_folder() {
                    Tree::prepend
                } else {
                    Tree::insert_after
                }
            }

            self.pending_move = Some(HandleMove::new(moved_outside, id, method));
        }

        if moved_in {
            set_msg_if_error!(self.message, profile.write_state());
        }

        if fail {
            self.message
                .set_error_from_str("Couldn't move some of the files");
        }
    }

    pub fn move_up(&mut self) {
        let Some((id, profile)) = (self.tree_state.selected).zip(self.games.get_profile_mut())
        else {
            return;
        };

        let entries = &mut profile.entries;

        if let Some(swap_with) = entries[id].previous_sibling() {
            entries.move_entry(Tree::insert_before, swap_with, id);
        } else if let Some(swap_with) = entries.following_siblings(id).next_back() {
            entries.move_entry(Tree::insert_after, swap_with, id);
        } else {
            return;
        }

        set_msg_if_error!(self.message, profile.write_state());
    }

    pub fn move_down(&mut self) {
        let Some((id, profile)) = (self.tree_state.selected).zip(self.games.get_profile_mut())
        else {
            return;
        };

        let entries = &mut profile.entries;

        if let Some(swap_with) = entries[id].next_sibling() {
            entries.move_entry(Tree::insert_after, swap_with, id);
        } else if let Some(swap_with) = entries.preceding_siblings(id).next_back() {
            entries.move_entry(Tree::insert_before, swap_with, id);
        } else {
            return;
        }

        set_msg_if_error!(self.message, profile.write_state());
    }

    pub fn open_all_folds(&mut self) {
        if let Some(entries) = self.games.get_entries_mut() {
            entries.apply_to_nodes(|node| {
                if let Some(expanded) = node.expanded.as_mut() {
                    *expanded = true;
                }
            });
        }
    }

    pub fn close_all_folds(&mut self) {
        if let Some(entries) = self.games.get_entries_mut() {
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
        let Some(entries) = self.games.get_entries_mut() else {
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
        if let Some(entries) = self.games.get_entries() {
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
        if let Some(entries) = self.games.get_entries() {
            self.tree_state.select_next(entries);
            self.auto_mark_save_file();
        }
    }

    pub fn select_first(&mut self) {
        if let Some(entries) = self.games.get_entries() {
            self.tree_state.select_first(entries);
            self.auto_mark_save_file();
        }
    }

    pub fn select_last(&mut self) {
        if let Some(entries) = self.games.get_entries() {
            self.tree_state.select_last(entries);
            self.auto_mark_save_file();
        }
    }

    pub fn up_directory(&mut self) {
        if let Some(id) = self.tree_state.selected {
            if let Some(entries) = self.games.get_entries() {
                self.tree_state.select_unchecked(
                    entries
                        .predecessors(id)
                        .chain(entries.children(NodeId::root()).rev())
                        .find(|id| entries[*id].is_folder() && *id != NodeId::root()),
                );
            }
        } else {
            self.select_first();
        }
    }

    pub fn down_directory(&mut self) {
        if let Some(id) = self.tree_state.selected {
            if let Some(entries) = self.games.get_entries() {
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
        }
    }

    pub fn load_save_file(&mut self, path: &Path, mark_as_active: bool) -> Result<()> {
        let game = self.games.get_game_unchecked_mut();
        let Some(savefile_path) = &game.savefile_path else {
            self.message
                .set_warning("No savefile path is set for the game.");
            return Ok(());
        };

        std::fs::copy(path, savefile_path).context("couldn't load save file")?;

        let profile = game.get_profile_mut().unwrap();

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

    pub fn load_random_save_file(&mut self) {
        let Some(entries) = self.games.get_entries_mut() else {
            return;
        };

        let save_files = entries
            .iter_ids()
            .filter(|id| !entries.detached_from_root(*id) && entries[*id].is_file())
            .collect::<Vec<NodeId>>();

        let id = fastrand::choice(save_files);
        self.tree_state.select(id, entries);

        if let Some(entries) = self.games.get_entries()
            && let Some(entry) = id.map(|id| &entries[id])
        {
            set_msg_if_error!(self.message, self.load_save_file(&entry.path.clone(), true));
        }
    }

    pub fn load_active_save_file(&mut self) {
        if let Some(path) = self.games.get_profile().unwrap().get_active_save_file() {
            set_msg_if_error!(self.message, self.load_save_file(&path, false));
        } else {
            self.message
                .set_warning("No active save file exists for the selected profile.");
        }
    }

    pub fn mark_selected_save_file(&mut self) {
        if let Some(path) = self
            .selected_entry()
            .filter(|entry| entry.is_file())
            .map(|entry| entry.path.clone())
        {
            let profile = self.games.get_profile_mut().unwrap();
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
        let Some(savefile_path) = self.games.get_game_unchecked().savefile_path.clone() else {
            self.message
                .set_warning("No savefile path is set for the game.");
            return;
        };

        let mut path = self
            .context_path(top_level)
            .join(savefile_path.file_name().unwrap());
        utils::validate_name(&mut path);

        set_msg_if_error!(
            self.message,
            std::fs::copy(&savefile_path, &path).map_err(Into::into)
        );
    }

    pub fn replace_save_file(&mut self) -> Result<()> {
        if let Some(entry) = self.selected_entry() {
            if entry.is_file() {
                if let Some(savefile_path) = &self.games.get_game_unchecked().savefile_path {
                    std::fs::copy(savefile_path, &entry.path)?;
                } else {
                    self.message
                        .set_warning("No savefile path is set for the game.");
                }
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
        }

        self.fuzzy_finder.update_matches();
    }

    pub fn jump_to_entry(&mut self) {
        if let Some(idx) = self.fuzzy_finder.selected_idx() {
            if let Some(picker) = self.fuzzy_finder.picker.take() {
                picker.jump(idx, self);
            }
        }
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

struct HandleMove {
    count: u32,
    relative: NodeId,
    method: fn(&mut Tree<Entry>, NodeId, NodeId),
}

impl HandleMove {
    fn new(count: u32, relative: NodeId, method: fn(&mut Tree<Entry>, NodeId, NodeId)) -> Self {
        Self {
            count,
            relative,
            method,
        }
    }

    fn execute(&mut self, entries: &mut Tree<Entry>, id: NodeId) {
        entries.move_entry(self.method, self.relative, id);
        self.count = self.count.saturating_sub(1);
    }
}

impl HandleFileSystemEvent for App {
    fn on_create(&mut self, path: &Path) -> Result<()> {
        let Some(entries) = self.games.get_entries_mut() else {
            return Ok(());
        };

        if let Some(parent_id) = path.parent().and_then(|path| entries.find_by_path(path)) {
            let new = entries.add_value(Entry::new(path));
            entries.append(parent_id, new);

            let node = &mut entries[new];

            if node.is_folder() {
                node.expanded = Some(false);
            }

            if self
                .tree_state
                .selected
                .filter(|id| !entries.detached_from_root(*id))
                .is_none()
            {
                self.tree_state.select_unchecked(Some(new));
            }
        }

        Ok(())
    }

    fn on_rename(&mut self, path: &Path, new_path: &Path) -> Result<()> {
        let Some(profile) = self.games.get_profile_mut() else {
            return Ok(());
        };

        let Some(entry_id) = profile.entries.find_by_path(path) else {
            return Ok(());
        };

        if let Some(handle) = &mut self.pending_move {
            handle.execute(&mut profile.entries, entry_id);
            if handle.count == 0 {
                self.pending_move = None;
            }
        } else if let Some(new_parent) = new_path
            .parent()
            .filter(|parent| Some(*parent) != path.parent())
            .and_then(|path| profile.entries.find_by_path(path))
        {
            profile
                .entries
                .move_entry(Tree::append, new_parent, entry_id);
        }

        profile.entries.update_paths(entry_id, new_path)?;

        if matches!(profile.get_active_save_file(), Some(active_path) if active_path == path) {
            profile.update_active_save_file(new_path)?;
        } else {
            set_msg_if_error!(self.message, profile.write_state());
        }

        Ok(())
    }

    fn on_delete(&mut self, path: &Path) -> Result<()> {
        let Some(profile) = self.games.get_profile_mut().filter(|p| p.path.exists()) else {
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

    pub fn push(&mut self, item: T) {
        self.items.push(item);

        if self.state.selected().is_none() {
            self.select_first();
        }
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
