use super::Mode;
use crate::{
    app::App,
    commands::{Command, ConfirmationCommand, HelpCommand, ProfileSelectionCommand},
    config::{KEY_BINDINGS, OPTIONS},
    fuzzy_finder::FuzzyFinder,
    help::Help,
    message::set_msg_if_error,
    profile::Profiles,
    search::Direction,
    ui::confirmation::Context as ConfirmationContext,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn handle_event(key: KeyEvent, app: &mut App) -> bool {
    if key.kind == KeyEventKind::Release {
        return false;
    }

    if app.help.visible {
        return handle_key_help_mode(key, &mut app.help);
    }

    match &app.mode {
        Mode::Normal if !app.fuzzy_finder.is_active() => return handle_key_normal_mode(key, app),
        Mode::ProfileSelection => return handle_key_profile_selection_mode(key, app),
        Mode::Confirmation(_) => return handle_key_confirmation_mode(key, app),
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
            Command::MoveUp => app.move_up(),
            Command::MoveDown => app.move_down(),
            Command::OpenAllFolds => app.open_all_folds(),
            Command::CloseAllFolds => app.close_all_folds(),
            Command::SelectProfile => app.select_profile(),
            Command::ToggleHelp => app.help.toggle(),
            Command::EnterSearch => app.search_new_pattern(),
            Command::RepeatLastSearch => app.repeat_search(),
            Command::RepeatLastSearchBackward => app.repeat_search_reverse(),
            Command::OpenFuzzyFinder => app.open_fuzzy_finder(false),
            Command::OpenFuzzyFinderGlobal => app.open_fuzzy_finder(true),
            Command::MarkEntry => app.mark_entry(),
            Command::Reset => app.tree_state.marked.clear(),
            Command::Quit => return true,
        }
    }

    false
}

fn handle_key_profile_selection_mode(key: KeyEvent, app: &mut App) -> bool {
    let profiles = &mut app.profiles.inner;

    if let Some(command) = KEY_BINDINGS.profile_selection.get(&key) {
        match command {
            ProfileSelectionCommand::Create => app.take_input(Mode::ProfileCreation),
            ProfileSelectionCommand::Rename => {
                let text = profiles.get_selected().unwrap().name().into_owned();
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
            Command::EnterSearch => app.search_new_pattern(),
            Command::RepeatLastSearch => app.repeat_search(),
            Command::RepeatLastSearchBackward => app.repeat_search_reverse(),
            Command::Quit => return true,
            _ => (),
        }
    }

    false
}

fn handle_key_help_mode(key: KeyEvent, help: &mut Help) -> bool {
    if let Some(command) = KEY_BINDINGS.help.get(&key) {
        match command {
            HelpCommand::ScrollUp => help.scroller.scroll_up(),
            HelpCommand::ScrollDown => help.scroller.scroll_down(),
            HelpCommand::GoToTop => help.scroller.scroll_top(),
            HelpCommand::GoToBottom => help.scroller.scroll_bottom(),
            HelpCommand::Abort => help.toggle(),
        }
    } else if let Some(command) = KEY_BINDINGS.get(&key) {
        match command {
            Command::OnDown => help.scroller.scroll_down(),
            Command::OnUp => help.scroller.scroll_up(),
            Command::SelectFirst => help.scroller.scroll_top(),
            Command::SelectLast => help.scroller.scroll_bottom(),
            Command::ToggleHelp => help.toggle(),
            Command::Quit => return true,
            _ => (),
        }
    }

    false
}

fn handle_key_confirmation_mode(key: KeyEvent, app: &mut App) -> bool {
    let Mode::Confirmation(prompt) = &mut app.mode else {
        unreachable!();
    };

    if let Some(command) = KEY_BINDINGS.confirmation.get(&key) {
        match command {
            ConfirmationCommand::Confirm => app.on_confirmation(),
            ConfirmationCommand::Cancel => app.mode.select_previous(),
            ConfirmationCommand::ScrollUp => prompt.scroller.scroll_up(),
            ConfirmationCommand::ScrollDown => prompt.scroller.scroll_down(),
            ConfirmationCommand::GoToTop => prompt.scroller.scroll_top(),
            ConfirmationCommand::GoToBottom => prompt.scroller.scroll_bottom(),
        }
    } else if let Some(command) = KEY_BINDINGS.get(&key) {
        match command {
            Command::OnDown => prompt.scroller.scroll_down(),
            Command::OnUp => prompt.scroller.scroll_up(),
            Command::SelectFirst => prompt.scroller.scroll_top(),
            Command::SelectLast => prompt.scroller.scroll_bottom(),
            Command::Quit => return true,
            _ => (),
        }
    }

    false
}

pub fn handle_key_fuzzy_mode(key: KeyEvent, fuzzy_finder: &mut FuzzyFinder) {
    let input = &mut fuzzy_finder.input;

    match (key.code, key.modifiers) {
        (KeyCode::Down | KeyCode::Tab, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            fuzzy_finder.matched.next();
        }
        (KeyCode::Up | KeyCode::BackTab, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            fuzzy_finder.matched.previous();
        }
        _ if input.update(key) => fuzzy_finder.update_matches(),
        _ => {}
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
                let changed = input.update(key);

                if changed && matches!(app.mode, Mode::Search(_)) && OPTIONS.incremental_search {
                    app.search.pattern = app.footer_input.as_ref().unwrap().text.clone();
                    app.run_search(Direction::Forward);
                }
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
        Mode::Search(..) => app.complete_search(),
        Mode::Normal
            if app.fuzzy_finder.is_active() && !app.fuzzy_finder.matched.items.is_empty() =>
        {
            app.jump_to_entry();
            app.fuzzy_finder.reset();
            Ok(())
        }
        _ => Ok(()),
    };

    set_msg_if_error!(app.message, res);
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
        | Mode::ProfileRenaming => app.abort_input(),
        Mode::Search(_) => app.abort_search(),
        Mode::Normal => app.fuzzy_finder.reset(),
        Mode::Confirmation(_) => (),
    }
}
