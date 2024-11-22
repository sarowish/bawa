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
    MarkSaveFile,
    ImportSaveFile,
    ImportSaveFileTopLevel,
    ReplaceSaveFile,
    DeleteFile,
    CreateFolder,
    CreateFolderTopLevel,
    Rename,
    OpenAllFolds,
    CloseAllFolds,
    SelectProfile,
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
            "mark_save_file" => Command::MarkSaveFile,
            "import_save_file" => Command::ImportSaveFile,
            "import_save_file_top_level" => Command::ImportSaveFileTopLevel,
            "replace_save_file" => Command::ReplaceSaveFile,
            "delete_file" => Command::DeleteFile,
            "create_folder" => Command::CreateFolder,
            "create_folder_top_level" => Command::CreateFolderTopLevel,
            "rename" => Command::Rename,
            "open_all_folds" => Command::OpenAllFolds,
            "close_all_folds" => Command::CloseAllFolds,
            "select_profile" => Command::SelectProfile,
            "quit" => Command::Quit,
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
