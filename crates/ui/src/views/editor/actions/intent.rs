use learn_core::model::DeckId;

use crate::vm::{CardListItemVm, MarkdownAction, MarkdownField};

use super::super::state::SaveRequest;

#[derive(Clone, Debug)]
pub enum EditorIntent {
    Save(SaveRequest),
    CreateDeck,
    CancelRename,
    CommitRename,
    BeginRename(String),
    RequestSelectDeck(DeckId),
    RequestSelectCard(CardListItemVm),
    RequestNewCard,
    AddTag(String),
    RemoveTag(String),
    SetTagFilter(Option<String>),
    HandlePaste(MarkdownField),
    ApplyFormat(MarkdownField, MarkdownAction),
    ApplyBlockDir(MarkdownField, String),
    ConfirmDiscard,
    CancelDiscard,
    OpenDeleteModal,
    ToggleSaveMenu,
    CloseSaveMenu,
    CloseDeleteModal,
    OpenResetDeckModal,
    CloseResetDeckModal,
    ConfirmResetDeck,
    CloseDuplicateModal,
    ConfirmDuplicate,
    Delete,
    CancelNew,
}
