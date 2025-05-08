#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Command {
    OnDown,
    OnUp,
    OnLeft,
    OnRight,
    SelectFirst,
    SelectLast,
    DownDirectory,
    UpDirectory,
    JumpToParent,
    LoadSaveFile,
    LoadActiveSaveFile,
    MarkSaveFile,
    ImportSaveFile,
    ImportSaveFileTopLevel,
    ReplaceSaveFile,
    DeleteFile,
    CreateFolder,
    CreateFolderTopLevel,
    Rename,
    MoveEntries,
    MoveEntriesTopLevel,
    MoveUp,
    MoveDown,
    OpenAllFolds,
    CloseAllFolds,
    OpenGameWindow,
    OpenProfileWindow,
    ToggleHelp,
    EnterSearch,
    RepeatLastSearch,
    RepeatLastSearchBackward,
    OpenFuzzyFinder,
    OpenFuzzyFinderGlobal,
    MarkEntry,
    Reset,
    Quit,
}

impl TryFrom<&str> for Command {
    type Error = anyhow::Error;

    fn try_from(command: &str) -> Result<Self, Self::Error> {
        let command = match command {
            "on_down" => Command::OnDown,
            "on_up" => Command::OnUp,
            "on_left" => Command::OnLeft,
            "on_right" => Command::OnRight,
            "select_first" => Command::SelectFirst,
            "select_last" => Command::SelectLast,
            "down_directory" => Command::DownDirectory,
            "up_directory" => Command::UpDirectory,
            "jump_to_parent" => Command::JumpToParent,
            "load_save_file" => Command::LoadSaveFile,
            "load_active_save_file" => Command::LoadActiveSaveFile,
            "mark_save_file" => Command::MarkSaveFile,
            "import_save_file" => Command::ImportSaveFile,
            "import_save_file_top_level" => Command::ImportSaveFileTopLevel,
            "replace_save_file" => Command::ReplaceSaveFile,
            "delete_file" => Command::DeleteFile,
            "create_folder" => Command::CreateFolder,
            "create_folder_top_level" => Command::CreateFolderTopLevel,
            "rename" => Command::Rename,
            "move_entries" => Command::MoveEntries,
            "move_entries_top_level" => Command::MoveEntriesTopLevel,
            "move_up" => Command::MoveUp,
            "move_down" => Command::MoveDown,
            "open_all_folds" => Command::OpenAllFolds,
            "close_all_folds" => Command::CloseAllFolds,
            "open_game_window" => Command::OpenGameWindow,
            "open_profile_window" => Command::OpenProfileWindow,
            "toggle_help" => Command::ToggleHelp,
            "enter_search" => Command::EnterSearch,
            "repeat_last_search" => Command::RepeatLastSearch,
            "repeat_last_search_backward" => Command::RepeatLastSearchBackward,
            "open_fuzzy_finder" => Command::OpenFuzzyFinder,
            "open_fuzzy_finder_global" => Command::OpenFuzzyFinderGlobal,
            "mark_entry" => Command::MarkEntry,
            "reset" => Command::Reset,
            "quit" => Command::Quit,
            _ => anyhow::bail!("\"{}\" is an invalid command", command),
        };

        Ok(command)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GameSelectionCommand {
    Create,
    Rename,
    Delete,
    SetSavefile,
    Select,
    Abort,
}

impl TryFrom<&str> for GameSelectionCommand {
    type Error = anyhow::Error;

    fn try_from(command: &str) -> Result<Self, Self::Error> {
        let command = match command {
            "create" => GameSelectionCommand::Create,
            "rename" => GameSelectionCommand::Rename,
            "delete" => GameSelectionCommand::Delete,
            "set_savefile" => GameSelectionCommand::SetSavefile,
            "select" => GameSelectionCommand::Select,
            "abort" => GameSelectionCommand::Abort,
            _ => anyhow::bail!("\"{}\" is an invalid command", command),
        };

        Ok(command)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ProfileSelectionCommand {
    Create,
    Rename,
    Delete,
    Select,
    Abort,
}

impl TryFrom<&str> for ProfileSelectionCommand {
    type Error = anyhow::Error;

    fn try_from(command: &str) -> Result<Self, Self::Error> {
        let command = match command {
            "create" => ProfileSelectionCommand::Create,
            "rename" => ProfileSelectionCommand::Rename,
            "delete" => ProfileSelectionCommand::Delete,
            "select" => ProfileSelectionCommand::Select,
            "abort" => ProfileSelectionCommand::Abort,
            _ => anyhow::bail!("\"{}\" is an invalid command", command),
        };

        Ok(command)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum HelpCommand {
    ScrollUp,
    ScrollDown,
    GoToTop,
    GoToBottom,
    Abort,
}

impl TryFrom<&str> for HelpCommand {
    type Error = anyhow::Error;

    fn try_from(command: &str) -> Result<Self, Self::Error> {
        let command = match command {
            "scroll_up" => HelpCommand::ScrollUp,
            "scroll_down" => HelpCommand::ScrollDown,
            "go_to_top" => HelpCommand::GoToTop,
            "go_to_bottom" => HelpCommand::GoToBottom,
            "abort" => HelpCommand::Abort,
            _ => anyhow::bail!("\"{}\" is an invalid command", command),
        };

        Ok(command)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConfirmationCommand {
    Confirm,
    Cancel,
    ScrollUp,
    ScrollDown,
    GoToTop,
    GoToBottom,
}

impl TryFrom<&str> for ConfirmationCommand {
    type Error = anyhow::Error;

    fn try_from(command: &str) -> Result<Self, Self::Error> {
        let command = match command {
            "confirm" => ConfirmationCommand::Confirm,
            "cancel" => ConfirmationCommand::Cancel,
            "scroll_up" => ConfirmationCommand::ScrollUp,
            "scroll_down" => ConfirmationCommand::ScrollDown,
            "go_to_top" => ConfirmationCommand::GoToTop,
            "go_to_bottom" => ConfirmationCommand::GoToBottom,
            _ => anyhow::bail!("\"{}\" is an invalid command", command),
        };

        Ok(command)
    }
}
