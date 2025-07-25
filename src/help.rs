use crate::{config::KEY_BINDINGS, ui::Scroller};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::ops::Deref;

const DESCRIPTIONS_LEN: usize = 37;
const DESCRIPTIONS: [&str; DESCRIPTIONS_LEN] = [
    "Go one line downward",                                 // On Down
    "Go one line upward",                                   // On Up
    "Close fold",                                           // On Left
    "Open fold",                                            // On Right
    "Jump to the first line",                               // Select First
    "Jump to the last line",                                // Select Last
    "Jump to the folder below",                             // Down Directory
    "Jump to the folder above",                             // Up Directory
    "Jump to the parent folder",                            // Jump To Parent
    "Load the selected save file",                          // Load Save File
    "Load a random save file",                              // Load Random Save File
    "Load the active save file",                            // Load Active Save File
    "Mark the selected save file as active",                // Mark Save File
    "Import save file into the current folder",             // Import Save File
    "Import save file to the top level",                    // Import Save File Top Level
    "Import new save file and overwrite the selected file", // Replace Save File
    "Delete the selected file/folder",                      // Delete File
    "Create folder",                                        // Create Folder
    "Create folder in the top level",                       // Create Folder Top Level
    "Rename the selected file/folder",                      // Rename
    "Move the marked entries into the current folder",      // Move Entries
    "Move the marked entries to the top level",             // Move Entries Top Level
    "Swap the selected entry with its above sibling",       // Move Up
    "Swap the selected entry with its below sibling",       // Move Below
    "Open all folds",                                       // Open All Folds
    "Close all folds",                                      // Close All Folds
    "Open game selection window",                           // Open Game Window
    "Open profile selection window",                        // Open Profile Window
    "Open help window",                                     // Toggle Help
    "Enter search pattern",                                 // Enter Search
    "Repeat the latest search",                             // Repeat Last Search
    "Repeat the latest search backward",                    // Repeat Last Search Backward
    "Open fuzzy finder",                                    // Open Fuzzy Finder
    "Open global fuzzy finder",                             // Open Fuzzy Finder Global
    "Mark the selected entry",                              // Mark Entry
    "Unmark all marked entries",                            // Reset
    "Quit application",                                     // Quit
];

const GAME_SELECTION_DESCRIPTIONS_LEN: usize = 6;
const GAME_SELECTION_DESCRIPTIONS: [&str; GAME_SELECTION_DESCRIPTIONS_LEN] = [
    " - Create, ",
    " - Rename, ",
    " - Delete, ",
    " - Set savefile path, ",
    " - Select, ",
    " - Abort",
];

const PROFILE_SELECTION_DESCRIPTIONS_LEN: usize = 5;
const PROFILE_SELECTION_DESCRIPTIONS: [&str; PROFILE_SELECTION_DESCRIPTIONS_LEN] = [
    " - Create, ",
    " - Rename, ",
    " - Delete, ",
    " - Select, ",
    " - Abort",
];

fn key_event_to_string(key_event: &KeyEvent) -> String {
    let key_code = match key_event.code {
        KeyCode::Backspace => "backspace",
        KeyCode::Enter => "enter",
        KeyCode::Left => "left",
        KeyCode::Right => "right",
        KeyCode::Up => "up",
        KeyCode::Down => "down",
        KeyCode::Home => "home",
        KeyCode::End => "end",
        KeyCode::PageUp => "pageup",
        KeyCode::PageDown => "pagedown",
        KeyCode::Tab => "tab",
        KeyCode::BackTab => "backtab",
        KeyCode::Delete => "delete",
        KeyCode::Insert => "insert",
        KeyCode::F(num) => &format!("f{num}"),
        KeyCode::Char(' ') => "space",
        KeyCode::Char(c) => &c.to_string(),
        KeyCode::Esc => "esc",
        _ => "",
    };

    let mut modifiers = Vec::with_capacity(3);

    if key_event.modifiers.intersects(KeyModifiers::CONTROL) {
        modifiers.push("ctrl");
    }

    if key_event.modifiers.intersects(KeyModifiers::SHIFT) {
        modifiers.push("shift");
    }

    if key_event.modifiers.intersects(KeyModifiers::ALT) {
        modifiers.push("alt");
    }

    let mut key = modifiers.join("-");

    if !key.is_empty() {
        key.push('-');
    }
    key.push_str(key_code);

    key
}

const HELP_ENTRY: (String, &str) = (String::new(), "");

pub struct Bindings {
    pub general: [(String, &'static str); DESCRIPTIONS_LEN],
    pub game_selection: [(String, &'static str); GAME_SELECTION_DESCRIPTIONS_LEN],
    pub profile_selection: [(String, &'static str); PROFILE_SELECTION_DESCRIPTIONS_LEN],
}

impl Default for Bindings {
    fn default() -> Self {
        let mut help = Self {
            general: [HELP_ENTRY; DESCRIPTIONS_LEN],
            game_selection: [HELP_ENTRY; GAME_SELECTION_DESCRIPTIONS_LEN],
            profile_selection: [HELP_ENTRY; PROFILE_SELECTION_DESCRIPTIONS_LEN],
        };

        macro_rules! generate_entries {
            ($entries: expr, $bindings: expr, $descriptions: ident) => {
                for (key, command) in &$bindings {
                    let idx = *command as usize;

                    if !$entries[idx].0.is_empty() {
                        $entries[idx].0.push_str(", ");
                    }
                    $entries[idx].0.push_str(&key_event_to_string(key));
                }

                for (idx, (_, desc)) in $entries.iter_mut().enumerate() {
                    *desc = $descriptions[idx];
                }
            };
        }

        generate_entries!(help.general, KEY_BINDINGS.general, DESCRIPTIONS);
        generate_entries!(
            help.game_selection,
            KEY_BINDINGS.game_selection,
            GAME_SELECTION_DESCRIPTIONS
        );
        generate_entries!(
            help.profile_selection,
            KEY_BINDINGS.profile_selection,
            PROFILE_SELECTION_DESCRIPTIONS
        );

        for (keys, _) in &mut help.general {
            *keys = format!("{keys:14}  ");
        }

        help
    }
}

impl Deref for Bindings {
    type Target = [(String, &'static str); DESCRIPTIONS_LEN];

    fn deref(&self) -> &Self::Target {
        &self.general
    }
}

#[derive(Default)]
pub struct Help {
    pub bindings: Bindings,
    pub visible: bool,
    pub scroller: Scroller,
}

impl Help {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}
