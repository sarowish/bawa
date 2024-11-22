use crate::{
    app::App,
    commands::{Command, ProfileSelectionCommand},
    KEY_BINDINGS,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug)]
pub enum InputMode {
    Normal,
    Confirmation(ConfirmationContext),
    EntryRenaming,
    ProfileSelection,
    ProfileCreation,
    ProfileRenaming,
    FolderCreation(bool),
}

#[derive(Clone, Copy, Debug)]
pub enum ConfirmationContext {
    Deletion,
    Replacing,
    ProfileDeletion,
}

impl ConfirmationContext {
    pub fn previous_input_mode(self) -> InputMode {
        match self {
            ConfirmationContext::Deletion | ConfirmationContext::Replacing => InputMode::Normal,
            ConfirmationContext::ProfileDeletion => InputMode::ProfileSelection,
        }
    }
}

#[derive(Debug)]
pub struct Input {
    pub text: String,
    pub prompt: String,
    idx: usize,
    pub cursor_position: u16,
}

impl Input {
    pub fn new(mode: &InputMode) -> Self {
        let prompt = match mode {
            InputMode::ProfileCreation => "Profile Name",
            InputMode::EntryRenaming | InputMode::ProfileRenaming => "Rename",
            InputMode::FolderCreation(_) => "Folder Name",
            _ => panic!(),
        };

        Self {
            text: String::new(),
            idx: 0,
            cursor_position: 0,
            prompt: format!("{prompt}: "),
        }
    }

    pub fn with_text(text: &str) -> Self {
        Self {
            text: text.to_string(),
            idx: text.len(),
            cursor_position: text.width() as u16,
            prompt: String::from("Rename: "),
        }
    }

    pub fn insert_key(&mut self, ch: char) {
        if self.idx == self.text.len() {
            self.text.push(ch);
        } else {
            self.text.insert(self.idx, ch);
        }

        self.idx += ch.len_utf8();
        self.cursor_position += ch.width().unwrap() as u16;
    }

    pub fn pop_key(&mut self) {
        if self.idx == 0 {
            return;
        }

        let (offset, ch) = self.text[..self.idx].grapheme_indices(true).last().unwrap();
        self.cursor_position -= ch.width() as u16;
        self.text.drain(offset..self.idx);
        self.idx = offset;
    }

    pub fn move_cursor_left(&mut self) {
        if self.idx == 0 {
            return;
        }

        let (offset, ch) = self.text[..self.idx].grapheme_indices(true).last().unwrap();
        self.cursor_position -= ch.width() as u16;
        self.idx = offset;
    }

    pub fn move_cursor_right(&mut self) {
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

    pub fn move_cursor_one_word_left(&mut self) {
        let idx = self.text[..self.idx]
            .unicode_word_indices()
            .last()
            .map_or(0, |(offset, _)| offset);
        self.cursor_position -= self.text[idx..self.idx].width() as u16;
        self.idx = idx;
    }

    pub fn move_cursor_one_word_right(&mut self) {
        let old_idx = self.idx;
        self.idx = self.text[self.idx..]
            .unicode_word_indices()
            .nth(1)
            .map_or(self.text.len(), |(offset, _)| self.idx + offset);
        self.cursor_position += self.text[old_idx..self.idx].width() as u16;
    }

    pub fn move_cursor_to_beginning_of_line(&mut self) {
        self.idx = 0;
        self.cursor_position = 0;
    }

    pub fn move_cursor_to_end_of_line(&mut self) {
        self.idx = self.text.len();
        self.cursor_position = self.text.width() as u16;
    }

    pub fn delete_word_before_cursor(&mut self) {
        let old_idx = self.idx;
        self.move_cursor_one_word_left();
        self.text.drain(self.idx..old_idx);
    }

    pub fn clear_line(&mut self) {
        self.text.clear();
        self.idx = 0;
        self.cursor_position = 0;
    }

    pub fn clear_to_right(&mut self) {
        self.text.drain(self.idx..);
    }
}

pub fn handle_event(key: KeyEvent, app: &mut App) -> bool {
    if key.kind == KeyEventKind::Release {
        return false;
    }

    match app.input_mode {
        InputMode::Normal => return handle_key_normal_mode(key, app),
        InputMode::ProfileSelection => return handle_key_profile_selection_mode(key, app),
        InputMode::Confirmation(context) => handle_key_confirmation_mode(key, app, context),
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
            Command::MarkSaveFile => app.mark_selected_save_file(),
            Command::ImportSaveFile => app.import_save_file(false),
            Command::ImportSaveFileTopLevel => app.import_save_file(true),
            Command::ReplaceSaveFile => app.prompt_for_confirmation(ConfirmationContext::Replacing),
            Command::DeleteFile => app.prompt_for_confirmation(ConfirmationContext::Deletion),
            Command::CreateFolder => app.take_input(InputMode::FolderCreation(false)),
            Command::CreateFolderTopLevel => app.take_input(InputMode::FolderCreation(true)),
            Command::Rename => app.enter_renaming(),
            Command::OpenAllFolds => app.open_all_folds(),
            Command::CloseAllFolds => app.close_all_folds(),
            Command::SelectProfile => app.select_profile(),
            Command::Quit => return true,
        }
    }

    false
}

fn handle_key_profile_selection_mode(key: KeyEvent, app: &mut App) -> bool {
    let profiles = &mut app.profiles.profiles;

    if let Some(command) = KEY_BINDINGS.profile_selection.get(&key) {
        match command {
            ProfileSelectionCommand::Create => {
                app.input_mode = InputMode::ProfileCreation;
                app.footer_input = Some(Input::new(&InputMode::ProfileCreation));
            }
            ProfileSelectionCommand::Rename => {
                app.input_mode = InputMode::ProfileRenaming;
                app.footer_input = Some(Input::with_text(&profiles.get_selected().unwrap().name));
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
            Command::Quit => return true,
            _ => (),
        }
    }

    false
}

fn handle_key_confirmation_mode(key: KeyEvent, app: &mut App, context: ConfirmationContext) {
    match key.code {
        KeyCode::Char('y') => app.on_confirmation(context),
        KeyCode::Char('n') => app.input_mode = context.previous_input_mode(),
        _ => (),
    }
}

fn handle_key_editing_mode(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Enter => complete(app),
        KeyCode::Esc => abort(app),
        _ => {
            let input = app.footer_input.as_mut().unwrap();

            match (key.code, key.modifiers) {
                (KeyCode::Left, KeyModifiers::CONTROL) => input.move_cursor_one_word_left(),
                (KeyCode::Right, KeyModifiers::CONTROL) => input.move_cursor_one_word_right(),
                (KeyCode::Left, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                    input.move_cursor_left();
                }
                (KeyCode::Right, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                    input.move_cursor_right();
                }
                (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                    input.move_cursor_to_beginning_of_line();
                }
                (KeyCode::Char('e'), KeyModifiers::CONTROL) => input.move_cursor_to_end_of_line(),
                (KeyCode::Char('w'), KeyModifiers::CONTROL) => input.delete_word_before_cursor(),
                (KeyCode::Char('u'), KeyModifiers::CONTROL) => input.clear_line(),
                (KeyCode::Char('k'), KeyModifiers::CONTROL) => input.clear_to_right(),
                (KeyCode::Backspace, _) | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                    input.pop_key();
                }
                (KeyCode::Char(c), _) => input.insert_key(c),
                _ => {}
            }
        }
    }
}

fn complete(app: &mut App) {
    match app.input_mode {
        InputMode::EntryRenaming => app.rename_selected_entry(),
        InputMode::FolderCreation(..) => app.create_folder(),
        InputMode::ProfileCreation => {
            app.profiles
                .create_profile(&app.footer_input.as_ref().unwrap().text);
            app.input_mode = InputMode::ProfileSelection;
            app.footer_input = None;
        }
        InputMode::ProfileRenaming => {
            app.profiles
                .rename_selected_profile(&app.footer_input.as_ref().unwrap().text);
            app.input_mode = InputMode::ProfileSelection;
            app.footer_input = None;
        }
        _ => (),
    }
}

fn abort(app: &mut App) {
    match app.input_mode {
        InputMode::EntryRenaming | InputMode::FolderCreation(..) => {
            app.input_mode = InputMode::Normal;
            app.footer_input = None;
        }
        InputMode::ProfileSelection => {
            // notify user that they can't abort if no profile is active
            if app.profiles.get_profile().is_some() {
                app.input_mode = InputMode::Normal;
            }
        }
        InputMode::ProfileCreation | InputMode::ProfileRenaming => {
            app.input_mode = InputMode::ProfileSelection;
            app.footer_input = None;
        }
        _ => (),
    }
}
