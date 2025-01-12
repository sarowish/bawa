use std::ops::RangeBounds;

use crate::{
    app::App,
    commands::{Command, HelpCommand, ProfileSelectionCommand},
    config::KEY_BINDINGS,
    help::Help,
    profile::Profiles,
    search::FuzzyFinder,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Default)]
pub enum Mode {
    #[default]
    Normal,
    Confirmation(ConfirmationContext),
    EntryRenaming,
    ProfileSelection,
    ProfileCreation,
    ProfileRenaming,
    FolderCreation(bool),
    Search(SearchContext),
}

impl Mode {
    pub fn select_previous(&mut self) {
        *self = match self {
            Mode::Confirmation(confirmation_context) => match confirmation_context {
                ConfirmationContext::Deletion | ConfirmationContext::Replacing => Mode::Normal,
                ConfirmationContext::ProfileDeletion => Mode::ProfileSelection,
            },
            Mode::Search(search_context) => match &search_context {
                SearchContext::Normal => Mode::Normal,
                SearchContext::ProfileSelection => Mode::ProfileSelection,
            },
            Mode::EntryRenaming | Mode::ProfileSelection | Mode::FolderCreation(_) => Mode::Normal,
            Mode::ProfileCreation | Mode::ProfileRenaming => Mode::ProfileSelection,
            Mode::Normal => unreachable!(),
        };
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ConfirmationContext {
    Deletion,
    Replacing,
    ProfileDeletion,
}

#[derive(Default)]
pub enum SearchContext {
    #[default]
    Normal,
    ProfileSelection,
}

enum InputChange {
    Insert,
    Append,
    Delete,
}

pub struct Input {
    pub text: String,
    pub prompt: String,
    idx: usize,
    cursor_position: u16,
    cursor_offset: u16,
    change: Option<InputChange>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            prompt: String::new(),
            idx: 0,
            cursor_position: 0,
            cursor_offset: 0,
            change: None,
        }
    }

    pub fn with_prompt(mode: &Mode) -> Self {
        let prompt = match mode {
            Mode::Search(_) => "/",
            Mode::ProfileCreation => "Profile Name: ",
            Mode::EntryRenaming | Mode::ProfileRenaming => "Rename: ",
            Mode::FolderCreation(_) => "Folder Name: ",
            Mode::Normal => "",
            _ => panic!(),
        };

        Self {
            text: String::new(),
            prompt: prompt.to_string(),
            idx: 0,
            cursor_position: 0,
            cursor_offset: prompt.len() as u16,
            change: None,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.idx = text.len();
        self.cursor_position = text.width() as u16;
    }

    pub fn cursor_position(&self) -> u16 {
        self.cursor_position + self.cursor_offset
    }

    pub fn set_idx(&mut self, idx: usize) {
        if idx > self.text.len() {
            self.idx = self.text.len();
        } else {
            self.idx = idx;
        }

        self.cursor_position = self.text[..self.idx].width() as u16;
    }

    fn clear_range<R: RangeBounds<usize>>(&mut self, range: R) {
        if self.text.drain(range).next().is_some() {
            self.change = Some(InputChange::Delete);
        }
    }

    fn insert_key(&mut self, ch: char) {
        if self.idx == self.text.len() {
            self.text.push(ch);
            self.change = Some(InputChange::Append);
        } else {
            self.text.insert(self.idx, ch);
            self.change = Some(InputChange::Insert);
        }

        self.idx += ch.len_utf8();
        self.cursor_position += ch.width().unwrap() as u16;
    }

    fn pop_key(&mut self) {
        if self.idx == 0 {
            return;
        }

        let (offset, ch) = self.text[..self.idx].grapheme_indices(true).last().unwrap();
        self.cursor_position -= ch.width() as u16;
        self.clear_range(offset..self.idx);
        self.idx = offset;
    }

    fn move_cursor_left(&mut self) {
        if self.idx == 0 {
            return;
        }

        let (offset, ch) = self.text[..self.idx].grapheme_indices(true).last().unwrap();
        self.cursor_position -= ch.width() as u16;
        self.idx = offset;
    }

    fn move_cursor_right(&mut self) {
        if self.idx == self.text.len() {
            return;
        }

        let (offset, ch) = self.text[self.idx..]
            .grapheme_indices(true)
            .next()
            .map(|(offset, ch)| (self.idx + offset + ch.len(), ch))
            .unwrap();
        self.cursor_position += ch.width() as u16;
        self.idx = offset;
    }

    fn move_cursor_one_word_left(&mut self) {
        let idx = self.text[..self.idx]
            .unicode_word_indices()
            .last()
            .map_or(0, |(offset, _)| offset);
        self.cursor_position -= self.text[idx..self.idx].width() as u16;
        self.idx = idx;
    }

    fn move_cursor_one_word_right(&mut self) {
        let old_idx = self.idx;
        self.idx = self.text[self.idx..]
            .unicode_word_indices()
            .nth(1)
            .map_or(self.text.len(), |(offset, _)| self.idx + offset);
        self.cursor_position += self.text[old_idx..self.idx].width() as u16;
    }

    fn move_cursor_to_beginning_of_line(&mut self) {
        self.idx = 0;
        self.cursor_position = 0;
    }

    fn move_cursor_to_end_of_line(&mut self) {
        self.idx = self.text.len();
        self.cursor_position = self.text.width() as u16;
    }

    fn delete_word_before_cursor(&mut self) {
        let old_idx = self.idx;
        self.move_cursor_one_word_left();
        self.clear_range(self.idx..old_idx);
    }

    fn clear_line(&mut self) {
        if !self.text.is_empty() {
            self.text.clear();
            self.idx = 0;
            self.cursor_position = 0;
            self.change = Some(InputChange::Delete);
        }
    }

    fn clear_to_right(&mut self) {
        self.clear_range(self.idx..);
    }

    fn update(&mut self, key: KeyEvent) {
        self.change = None;

        match (key.code, key.modifiers) {
            (KeyCode::Left, KeyModifiers::CONTROL) => self.move_cursor_one_word_left(),
            (KeyCode::Right, KeyModifiers::CONTROL) => self.move_cursor_one_word_right(),
            (KeyCode::Left, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.move_cursor_left();
            }
            (KeyCode::Right, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.move_cursor_right();
            }
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.move_cursor_to_beginning_of_line();
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => self.move_cursor_to_end_of_line(),
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => self.delete_word_before_cursor(),
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => self.clear_line(),
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => self.clear_to_right(),
            (KeyCode::Backspace, _) | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                self.pop_key();
            }
            (KeyCode::Char(c), _) => self.insert_key(c),
            _ => {}
        }
    }
}

pub fn handle_event(key: KeyEvent, app: &mut App) -> bool {
    if key.kind == KeyEventKind::Release {
        return false;
    }

    if app.help.visible {
        return handle_key_help_mode(key, &mut app.help);
    }

    match app.mode {
        Mode::Normal if !app.fuzzy_finder.is_active() => return handle_key_normal_mode(key, app),
        Mode::ProfileSelection => return handle_key_profile_selection_mode(key, app),
        Mode::Confirmation(context) => handle_key_confirmation_mode(key, app, context),
        _ => handle_key_editing_mode(key, app),
    }

    false
}

fn handle_key_normal_mode(key: KeyEvent, app: &mut App) -> bool {
    if let Some(command) = KEY_BINDINGS.get(&key) {
        match command {
            Command::OnDown => app.on_down(),
            Command::OnUp => app.on_up(),
            Command::OnLeft => app.on_left(),
            Command::OnRight => app.on_right(),
            Command::SelectFirst => app.select_first(),
            Command::SelectLast => app.select_last(),
            Command::DownDirectory => app.down_directory(),
            Command::UpDirectory => app.up_directory(),
            Command::JumpToParent => app.jump_to_parent(),
            Command::LoadSaveFile => app.load_selected_save_file(),
            Command::LoadActiveSaveFile => app.load_active_save_file(),
            Command::MarkSaveFile => app.mark_selected_save_file(),
            Command::ImportSaveFile => app.import_save_file(false),
            Command::ImportSaveFileTopLevel => app.import_save_file(true),
            Command::ReplaceSaveFile => app.prompt_for_confirmation(ConfirmationContext::Replacing),
            Command::DeleteFile => app.prompt_for_confirmation(ConfirmationContext::Deletion),
            Command::CreateFolder => app.take_input(Mode::FolderCreation(false)),
            Command::CreateFolderTopLevel => app.take_input(Mode::FolderCreation(true)),
            Command::Rename => app.enter_renaming(),
            Command::MoveEntries => app.move_entries(false),
            Command::MoveEntriesTopLevel => app.move_entries(true),
            Command::OpenAllFolds => app.open_all_folds(),
            Command::CloseAllFolds => app.close_all_folds(),
            Command::SelectProfile => app.select_profile(),
            Command::ToggleHelp => app.help.toggle(),
            Command::EnterSearch => app.take_input(Mode::Search(SearchContext::Normal)),
            Command::RepeatLastSearch => app.repeat_search(),
            Command::RepeatLastSearchBackward => app.repeat_search_backwards(),
            Command::OpenFuzzyFinder => app.open_fuzzy_finder(),
            Command::MarkEntry => app.mark_entry(),
            Command::Reset => app.marked_entries.clear(),
            Command::Quit => return true,
        }
    }

    false
}

fn handle_key_profile_selection_mode(key: KeyEvent, app: &mut App) -> bool {
    let profiles = &mut app.profiles.profiles;

    if let Some(command) = KEY_BINDINGS.profile_selection.get(&key) {
        match command {
            ProfileSelectionCommand::Create => app.take_input(Mode::ProfileCreation),
            ProfileSelectionCommand::Rename => {
                let text = profiles.get_selected().unwrap().name.clone();
                app.take_input(Mode::ProfileRenaming);
                app.footer_input.as_mut().unwrap().set_text(&text);
            }
            ProfileSelectionCommand::Delete => {
                app.prompt_for_confirmation(ConfirmationContext::ProfileDeletion);
            }
            ProfileSelectionCommand::Select => app.confirm_profile_selection(),
            ProfileSelectionCommand::Abort => abort(app),
        }
    } else if let Some(command) = KEY_BINDINGS.get(&key) {
        match command {
            Command::OnDown => profiles.next(),
            Command::OnUp => profiles.previous(),
            Command::SelectFirst => profiles.select_first(),
            Command::SelectLast => profiles.select_last(),
            Command::EnterSearch => app.take_input(Mode::Search(SearchContext::ProfileSelection)),
            Command::RepeatLastSearch => app.repeat_search(),
            Command::RepeatLastSearchBackward => app.repeat_search_backwards(),
            Command::Quit => return true,
            _ => (),
        }
    }

    false
}

fn handle_key_help_mode(key: KeyEvent, help_window_state: &mut Help) -> bool {
    if let Some(command) = KEY_BINDINGS.help.get(&key) {
        match command {
            HelpCommand::ScrollUp => help_window_state.scroll_up(),
            HelpCommand::ScrollDown => help_window_state.scroll_down(),
            HelpCommand::GoToTop => help_window_state.scroll_top(),
            HelpCommand::GoToBottom => help_window_state.scroll_bottom(),
            HelpCommand::Abort => help_window_state.toggle(),
        }
    } else if let Some(command) = KEY_BINDINGS.get(&key) {
        match command {
            Command::OnDown => help_window_state.scroll_down(),
            Command::OnUp => help_window_state.scroll_up(),
            Command::SelectFirst => help_window_state.scroll_top(),
            Command::SelectLast => help_window_state.scroll_bottom(),
            Command::ToggleHelp => help_window_state.toggle(),
            Command::Quit => return true,
            _ => (),
        }
    }

    false
}

fn handle_key_confirmation_mode(key: KeyEvent, app: &mut App, context: ConfirmationContext) {
    match key.code {
        KeyCode::Char('y') => app.on_confirmation(context),
        KeyCode::Char('n') => app.mode.select_previous(),
        _ => (),
    }
}

pub fn handle_key_fuzzy_mode(key: KeyEvent, fuzzy_finder: &mut FuzzyFinder) {
    let input = &mut fuzzy_finder.input;

    match (key.code, key.modifiers) {
        (KeyCode::Down | KeyCode::Tab, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            fuzzy_finder.matched_items.next();
        }
        (KeyCode::Up | KeyCode::BackTab, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            fuzzy_finder.matched_items.previous();
        }
        _ => {
            input.update(key);

            if input.change.is_some() {
                fuzzy_finder.update_matches();
            }
        }
    }
}

fn handle_key_editing_mode(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Enter => complete(app),
        KeyCode::Esc => abort(app),
        _ => {
            if app.fuzzy_finder.is_active() {
                handle_key_fuzzy_mode(key, &mut app.fuzzy_finder);
            } else if let Some(input) = &mut app.footer_input {
                input.update(key);
            };
        }
    }
}

fn complete(app: &mut App) {
    let res = match &app.mode {
        Mode::EntryRenaming => app.rename_selected_entry(),
        Mode::FolderCreation(top_level) => app.create_folder(*top_level),
        Mode::ProfileCreation => Profiles::create_profile(&app.extract_input()),
        Mode::ProfileRenaming => {
            let new_name = app.extract_input();
            app.profiles.rename_selected_profile(&new_name)
        }
        Mode::Search(..) => {
            app.search_new_pattern();
            Ok(())
        }
        Mode::Normal
            if app.fuzzy_finder.is_active() && !app.fuzzy_finder.matched_items.items.is_empty() =>
        {
            app.jump_to_entry();
            app.fuzzy_finder.reset();
            Ok(())
        }
        _ => Ok(()),
    };

    if let Err(e) = res {
        app.message.set_error(&e);
    }
}

fn abort(app: &mut App) {
    match &app.mode {
        Mode::ProfileSelection => {
            if app.profiles.get_profile().is_some() {
                app.mode.select_previous();
            } else {
                app.message
                    .set_warning("Can't abort while no profile is selected");
            }
        }
        Mode::EntryRenaming
        | Mode::FolderCreation(..)
        | Mode::ProfileCreation
        | Mode::ProfileRenaming
        | Mode::Search(..) => {
            app.abort_input();
        }
        Mode::Normal if app.fuzzy_finder.is_active() => {
            app.fuzzy_finder.reset();
        }
        _ => (),
    }
}
