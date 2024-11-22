use crate::commands::{Command, HelpCommand, ProfileSelectionCommand};
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
    profile_selection: Option<HashMap<String, String>>,
    help: Option<HashMap<String, String>>,
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
    pub profile_selection: IndexMap<KeyEvent, ProfileSelectionCommand>,
    pub help: IndexMap<KeyEvent, HelpCommand>,
}

impl Default for KeyBindings {
    #[rustfmt::skip]
    fn default() -> Self {
        let mut general = IndexMap::new();
        let mut profile_selection = IndexMap::new();
        let mut help = IndexMap::new();

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
        insert_binding!(general, "F", Command::MarkSaveFile);
        insert_binding!(general, "i", Command::ImportSaveFile);
        insert_binding!(general, "I", Command::ImportSaveFileTopLevel);
        insert_binding!(general, "r", Command::ReplaceSaveFile);
        insert_binding!(general, "d", Command::DeleteFile);
        insert_binding!(general, "c", Command::CreateFolder);
        insert_binding!(general, "C", Command::CreateFolderTopLevel);
        insert_binding!(general, "s", Command::Rename);
        insert_binding!(general, "a", Command::OpenAllFolds);
        insert_binding!(general, "z", Command::CloseAllFolds);
        insert_binding!(general, "p", Command::SelectProfile);
        insert_binding!(general, "ctrl-h", Command::ToggleHelp);
        insert_binding!(general, "f1", Command::ToggleHelp);
        insert_binding!(general, "q", Command::Quit);
        insert_binding!(general, "ctrl-c", Command::Quit);

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

        Self {
            general,
            profile_selection,
            help,
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

impl TryFrom<UserKeyBindings> for KeyBindings {
    type Error = anyhow::Error;

    fn try_from(user_key_bindings: UserKeyBindings) -> Result<Self, Self::Error> {
        let mut key_bindings = KeyBindings::default();

        if let Some(bindings) = user_key_bindings.general {
            set_bindings(&mut key_bindings, &bindings)?;
        }

        if let Some(bindings) = user_key_bindings.profile_selection {
            set_bindings(&mut key_bindings.profile_selection, &bindings)?;
        }

        if let Some(bindings) = user_key_bindings.help {
            set_bindings(&mut key_bindings.help, &bindings)?;
        }

        Ok(key_bindings)
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
