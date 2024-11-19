use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::{App, StatefulList};

#[derive(Debug)]
pub enum InputMode {
    Normal,
    Confirmation,
    EntryRenaming,
    ProfileSelection,
    ProfileCreation,
    ProfileRenaming,
    FolderCreation(bool),
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
    match app.input_mode {
        InputMode::Normal => return handle_key_normal_mode(key, app),
        InputMode::ProfileSelection => return handle_key_profile_selection_mode(key, app),
        InputMode::Confirmation => handle_key_confirmation_mode(key, app),
        _ => handle_key_editing_mode(key, app),
    }

    false
}

fn handle_key_normal_mode(key: KeyEvent, app: &mut App) -> bool {
    match key.code {
        KeyCode::Left | KeyCode::Char('h') => app.on_left(),
        KeyCode::Up | KeyCode::Char('k') => app.on_up(),
        KeyCode::Right | KeyCode::Char('l') => app.on_right(),
        KeyCode::Down | KeyCode::Char('j') => app.on_down(),
        KeyCode::Char('K') => app.up_directory(),
        KeyCode::Char('J') => app.down_directory(),
        KeyCode::Char('g') => app.select_first(),
        KeyCode::Char('G') => app.select_last(),
        KeyCode::Char('p') => app.jump_to_parent(),
        KeyCode::Char('f') => app.load_selected_save_file(),
        KeyCode::Char('i') => app.import_save_file(),
        KeyCode::Char('r') => app.replace_save_file(),
        KeyCode::Char('d') => app.prompt_for_deletion(),
        KeyCode::Char('c') => app.take_input(InputMode::FolderCreation(false)),
        KeyCode::Char('C') => app.take_input(InputMode::FolderCreation(true)),
        KeyCode::Char('s') => app.enter_renaming(),
        KeyCode::Char('a') => app.open_all_folds(),
        KeyCode::Char('z') => app.close_all_folds(),
        KeyCode::Char('w') => app.select_profile(),
        KeyCode::Char('q') => return true,
        _ => {}
    }

    false
}

fn handle_key_profile_selection_mode(key: KeyEvent, app: &mut App) -> bool {
    match key.code {
        KeyCode::Enter => app.confirm_profile_selection(),
        KeyCode::Esc => abort(app),
        KeyCode::Char('q') => return true,
        _ => {
            let profiles = &mut app.profiles.profiles;

            match key.code {
                KeyCode::Down | KeyCode::Char('j') => profiles.next(),
                KeyCode::Up | KeyCode::Char('k') => profiles.previous(),
                KeyCode::Char('g') => profiles.select_first(),
                KeyCode::Char('G') => profiles.select_last(),
                KeyCode::Char('c') => {
                    app.input_mode = InputMode::ProfileCreation;
                    app.footer_input = Some(Input::new(&InputMode::ProfileCreation));
                }
                KeyCode::Char('r') => {
                    app.input_mode = InputMode::ProfileRenaming;
                    app.footer_input =
                        Some(Input::with_text(&profiles.get_selected().unwrap().name));
                }
                KeyCode::Char('d') => {
                    app.profiles.delete_selected_profile();

                    if app.profiles.get_profile().is_none() {
                        app.visible_entries = StatefulList::with_items(Vec::new());
                    }
                }
                _ => {}
            }
        }
    }

    false
}

fn handle_key_confirmation_mode(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('y') => app.delete_selected_entry(),
        KeyCode::Char('n') => app.input_mode = InputMode::Normal,
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
