use crate::{
    search::Context as SearchContext,
    ui::confirmation::{Context as ConfirmationContext, Prompt},
};

#[derive(Default)]
pub enum Mode {
    #[default]
    Normal,
    Confirmation(Prompt),
    EntryRenaming,
    GameSelection,
    GameCreation,
    GameRenaming,
    ProfileSelection,
    ProfileCreation,
    ProfileRenaming,
    FolderCreation(bool),
    Search(SearchContext),
}

impl Mode {
    pub fn select_previous(&mut self) {
        *self = match self {
            Mode::Confirmation(prompt) => match prompt.context {
                ConfirmationContext::Deletion | ConfirmationContext::Replacing => Mode::Normal,
                ConfirmationContext::GameDeletion => Mode::GameSelection,
                ConfirmationContext::ProfileDeletion => Mode::ProfileSelection,
            },
            Mode::Search(search_context) => match &search_context {
                SearchContext::Normal => Mode::Normal,
                SearchContext::GameSelection => Mode::GameSelection,
                SearchContext::GameCreation => Mode::GameCreation,
                SearchContext::ProfileSelection => Mode::ProfileSelection,
            },
            Mode::EntryRenaming
            | Mode::FolderCreation(_)
            | Mode::GameSelection
            | Mode::ProfileSelection => Mode::Normal,
            Mode::GameCreation | Mode::GameRenaming => Mode::GameSelection,
            Mode::ProfileCreation | Mode::ProfileRenaming => Mode::ProfileSelection,
            Mode::Normal => unreachable!(),
        };
    }

    pub fn search_context(&self) -> SearchContext {
        match self {
            Mode::Normal => SearchContext::Normal,
            Mode::GameSelection => SearchContext::GameSelection,
            Mode::GameCreation => SearchContext::GameCreation,
            Mode::ProfileSelection => SearchContext::ProfileSelection,
            Mode::Search(context) => *context,
            _ => unreachable!(),
        }
    }

    pub fn confirmation_context(&self) -> ConfirmationContext {
        match self {
            Mode::Confirmation(prompt) => prompt.context,
            _ => unreachable!(),
        }
    }

    pub fn is_profile_selection(&self) -> bool {
        matches!(
            &self,
            Mode::ProfileSelection
                | Mode::ProfileCreation
                | Mode::ProfileRenaming
                | Mode::Search(SearchContext::ProfileSelection)
        )
    }

    pub fn is_game_selection(&self) -> bool {
        matches!(
            &self,
            Mode::GameSelection
                | Mode::GameCreation
                | Mode::GameRenaming
                | Mode::Search(SearchContext::GameSelection | SearchContext::GameCreation)
        )
    }

    pub fn is_game_creation(&self) -> bool {
        matches!(&self, |Mode::GameCreation| Mode::Search(
            SearchContext::GameCreation
        ))
    }
}
