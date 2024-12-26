use crate::{app::App, input::Mode};
use ratatui::{prelude::CrosstermBackend, Terminal};
use std::io::Stdout;

pub fn set_cursor(app: &App, terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
    if let Some(input) = &app.footer_input {
        const SEARCH_CURSOR_OFFSET: u16 = 1;
        const RENAMING_CURSOR_OFFSET: u16 = 8;
        const FOLDER_CREATION_CURSOR_OFFSET: u16 = 13;
        const PROFILE_CREATION_CURSOR_OFFSET: u16 = 14;

        let cursor_position = input.cursor_position;
        let offset = match app.mode {
            Mode::Search(_) => SEARCH_CURSOR_OFFSET,
            Mode::EntryRenaming | Mode::ProfileRenaming => RENAMING_CURSOR_OFFSET,
            Mode::FolderCreation(_) => FOLDER_CREATION_CURSOR_OFFSET,
            Mode::ProfileCreation => PROFILE_CREATION_CURSOR_OFFSET,
            _ => panic!(),
        };

        terminal
            .set_cursor_position((
                cursor_position + offset,
                terminal.size().unwrap().height - 1,
            ))
            .unwrap();
        terminal.show_cursor().unwrap();
    } else {
        terminal.hide_cursor().unwrap();
    }
}
