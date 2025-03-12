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
                ConfirmationContext::ProfileDeletion => Mode::ProfileSelection,
            },
            Mode::Search(search_context) => match &search_context {
                SearchContext::Normal => Mode::Normal,
                SearchContext::ProfileSelection => Mode::ProfileSelection,
            },
            Mode::EntryRenaming | Mode::ProfileSelection | Mode::FolderCreation(_) => Mode::Normal,
            Mode::ProfileCreation | Mode::ProfileRenaming => Mode::ProfileSelection,
            Mode::Normal => unreachable!(),
        };
    }

    pub fn search_context(&self) -> SearchContext {
        match self {
            Mode::Normal => SearchContext::Normal,
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
}
