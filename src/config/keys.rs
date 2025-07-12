use super::MergeConfig;
use crate::commands::{
    Command, ConfirmationCommand, GameSelectionCommand, HelpCommand, ProfileSelectionCommand,
};
use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use indexmap::IndexMap;
use serde::Deserialize;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

#[derive(Deserialize)]
pub struct UserKeyBindings {
    #[serde(flatten)]
    general: Option<HashMap<String, String>>,
    game_selection: Option<HashMap<String, String>>,
    profile_selection: Option<HashMap<String, String>>,
    help: Option<HashMap<String, String>>,
    confirmation: Option<HashMap<String, String>>,
}

fn parse_binding(binding: &str) -> Result<KeyEvent> {
    let mut tokens = binding.rsplit('-');

    let code = if let Some(token) = tokens.next() {
        match token {
            "backspace" => KeyCode::Backspace,
            "space" => KeyCode::Char(' '),
            "enter" => KeyCode::Enter,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            "tab" => KeyCode::Tab,
            "backtab" => KeyCode::BackTab,
            "del" | "delete" => KeyCode::Delete,
            "insert" => KeyCode::Insert,
            "esc" | "escape" => KeyCode::Esc,
            _ => {
                if token.len() == 1 {
                    KeyCode::Char(token.chars().next().unwrap())
                } else if let Some(numbers) = token
                    .strip_prefix('f')
                    .and_then(|numbers| numbers.parse::<u8>().ok())
                {
                    KeyCode::F(numbers)
                } else {
                    anyhow::bail!("\"{}\" is not a valid key", token)
                }
            }
        }
    } else {
        anyhow::bail!("\"{}\" is not a valid binding", binding)
    };

    let mut modifiers = KeyModifiers::NONE;

    for token in tokens {
        match token {
            "ctrl" => modifiers.insert(KeyModifiers::CONTROL),
            "shift" => modifiers.insert(KeyModifiers::SHIFT),
            "alt" => modifiers.insert(KeyModifiers::ALT),
            _ => anyhow::bail!("\"{}\" is not a valid modifier", token),
        }
    }

    Ok(KeyEvent::new(code, modifiers))
}

#[derive(PartialEq, Eq, Debug)]
pub struct KeyBindings {
    pub general: IndexMap<KeyEvent, Command>,
    pub game_selection: IndexMap<KeyEvent, GameSelectionCommand>,
    pub profile_selection: IndexMap<KeyEvent, ProfileSelectionCommand>,
    pub help: IndexMap<KeyEvent, HelpCommand>,
    pub confirmation: IndexMap<KeyEvent, ConfirmationCommand>,
}

impl Default for KeyBindings {
    #[rustfmt::skip]
    fn default() -> Self {
        let mut general = IndexMap::new();
        let mut game_selection = IndexMap::new();
        let mut profile_selection = IndexMap::new();
        let mut help = IndexMap::new();
        let mut confirmation = IndexMap::new();

        macro_rules! insert_binding {
            ($map: expr, $key: expr, $command: expr) => {
                $map.insert(parse_binding($key).unwrap(), $command);
            };
        }

        insert_binding!(general, "j", Command::OnDown);
        insert_binding!(general, "down", Command::OnDown);
        insert_binding!(general, "k", Command::OnUp);
        insert_binding!(general, "up", Command::OnUp);
        insert_binding!(general, "h", Command::OnLeft);
        insert_binding!(general, "left", Command::OnLeft);
        insert_binding!(general, "l", Command::OnRight);
        insert_binding!(general, "right", Command::OnRight);
        insert_binding!(general, "g", Command::SelectFirst);
        insert_binding!(general, "G", Command::SelectLast);
        insert_binding!(general, "J", Command::DownDirectory);
        insert_binding!(general, "shift-down", Command::DownDirectory);
        insert_binding!(general, "K", Command::UpDirectory);
        insert_binding!(general, "shift-up", Command::UpDirectory);
        insert_binding!(general, "f", Command::LoadSaveFile);
        insert_binding!(general, "ctrl-f", Command::LoadActiveSaveFile);
        insert_binding!(general, "F", Command::MarkSaveFile);
        insert_binding!(general, "i", Command::ImportSaveFile);
        insert_binding!(general, "I", Command::ImportSaveFileTopLevel);
        insert_binding!(general, "R", Command::ReplaceSaveFile);
        insert_binding!(general, "d", Command::DeleteFile);
        insert_binding!(general, "c", Command::CreateFolder);
        insert_binding!(general, "C", Command::CreateFolderTopLevel);
        insert_binding!(general, "r", Command::Rename);
        insert_binding!(general, "p", Command::MoveEntries);
        insert_binding!(general, "P", Command::MoveEntriesTopLevel);
        insert_binding!(general, "M", Command::MoveUp);
        insert_binding!(general, "m", Command::MoveDown);
        insert_binding!(general, "a", Command::OpenAllFolds);
        insert_binding!(general, "z", Command::CloseAllFolds);
        insert_binding!(general, "W", Command::OpenGameWindow);
        insert_binding!(general, "w", Command::OpenProfileWindow);
        insert_binding!(general, "ctrl-h", Command::ToggleHelp);
        insert_binding!(general, "f1", Command::ToggleHelp);
        insert_binding!(general, "/", Command::EnterSearch);
        insert_binding!(general, "n", Command::RepeatLastSearch);
        insert_binding!(general, "N", Command::RepeatLastSearchBackward);
        insert_binding!(general, "s", Command::OpenFuzzyFinder);
        insert_binding!(general, "S", Command::OpenFuzzyFinderGlobal);
        insert_binding!(general, "space", Command::MarkEntry);
        insert_binding!(general, "esc", Command::Reset);
        insert_binding!(general, "q", Command::Quit);
        insert_binding!(general, "ctrl-c", Command::Quit);

        insert_binding!(game_selection, "c", GameSelectionCommand::Create);
        insert_binding!(game_selection, "r", GameSelectionCommand::Rename);
        insert_binding!(game_selection, "d", GameSelectionCommand::Delete);
        insert_binding!(game_selection, "enter", GameSelectionCommand::Select);
        insert_binding!(game_selection, "s", GameSelectionCommand::SetSavefile);
        insert_binding!(game_selection, "escape", GameSelectionCommand::Abort);

        insert_binding!(profile_selection, "c", ProfileSelectionCommand::Create);
        insert_binding!(profile_selection, "r", ProfileSelectionCommand::Rename);
        insert_binding!(profile_selection, "d", ProfileSelectionCommand::Delete);
        insert_binding!(profile_selection, "enter", ProfileSelectionCommand::Select);
        insert_binding!(profile_selection, "escape", ProfileSelectionCommand::Abort);

        insert_binding!(help, "ctrl-y", HelpCommand::ScrollUp);
        insert_binding!(help, "ctrl-e", HelpCommand::ScrollDown);
        insert_binding!(help, "g", HelpCommand::GoToTop);
        insert_binding!(help, "G", HelpCommand::GoToBottom);
        insert_binding!(help, "esc", HelpCommand::Abort);

        insert_binding!(confirmation, "y", ConfirmationCommand::Confirm);
        insert_binding!(confirmation, "enter", ConfirmationCommand::Confirm);
        insert_binding!(confirmation, "n", ConfirmationCommand::Cancel);
        insert_binding!(confirmation, "esc", ConfirmationCommand::Cancel);
        insert_binding!(confirmation, "ctrl-y", ConfirmationCommand::ScrollUp);
        insert_binding!(confirmation, "ctrl-e", ConfirmationCommand::ScrollDown);
        insert_binding!(confirmation, "g", ConfirmationCommand::GoToTop);
        insert_binding!(confirmation, "G", ConfirmationCommand::GoToBottom);

        Self {
            general,
            game_selection,
            profile_selection,
            help,
            confirmation,
        }
    }
}

fn set_bindings<'a, T, E>(
    key_bindings: &mut IndexMap<KeyEvent, T>,
    user_key_bindings: &'a HashMap<String, String>,
) -> Result<(), anyhow::Error>
where
    T: TryFrom<&'a str, Error = E>,
    E: Into<anyhow::Error>,
{
    for (bindings, command) in user_key_bindings {
        for binding in bindings.split_whitespace() {
            let binding = parse_binding(binding)
                .with_context(|| format!("Error: failed to parse binding \"{binding}\""))?;
            if command.is_empty() {
                key_bindings.swap_remove(&binding);
            } else {
                key_bindings.insert(
                    binding,
                    T::try_from(command.as_str())
                        .map_err(|e| anyhow::anyhow!(e))
                        .with_context(|| format!("Error: failed to parse command \"{command}\""))?,
                );
            }
        }
    }

    Ok(())
}

impl MergeConfig for KeyBindings {
    type Other = UserKeyBindings;

    fn merge(&mut self, user_key_bindings: Self::Other) -> Result<()> {
        if let Some(bindings) = user_key_bindings.general {
            set_bindings(&mut self.general, &bindings)?;
        }

        if let Some(bindings) = user_key_bindings.game_selection {
            set_bindings(&mut self.game_selection, &bindings)?;
        }

        if let Some(bindings) = user_key_bindings.profile_selection {
            set_bindings(&mut self.profile_selection, &bindings)?;
        }

        if let Some(bindings) = user_key_bindings.help {
            set_bindings(&mut self.help, &bindings)?;
        }

        if let Some(bindings) = user_key_bindings.confirmation {
            set_bindings(&mut self.confirmation, &bindings)?;
        }

        Ok(())
    }
}

impl Deref for KeyBindings {
    type Target = IndexMap<KeyEvent, Command>;

    fn deref(&self) -> &Self::Target {
        &self.general
    }
}

impl DerefMut for KeyBindings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.general
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, keys::UserKeyBindings, tests::read_example_config};

    #[test]
    fn example_up_to_date() {
        let default = Config::default().key_bindings;
        let user_config = read_example_config();

        let UserKeyBindings {
            general,
            game_selection,
            profile_selection,
            help,
            confirmation,
        } = user_config.key_bindings.unwrap();

        assert!(general.is_some_and(|keys| keys.len() == default.general.len()));
        assert!(game_selection.is_some_and(|keys| keys.len() == default.game_selection.len()));
        assert!(
            profile_selection.is_some_and(|keys| keys.len() == default.profile_selection.len())
        );
        assert!(help.is_some_and(|keys| keys.len() == default.help.len()));
        assert!(confirmation.is_some_and(|keys| keys.len() == default.confirmation.len()));
    }
}
