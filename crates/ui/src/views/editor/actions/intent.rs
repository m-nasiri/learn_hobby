use learn_core::model::DeckId;

use crate::vm::{CardListItemVm, MarkdownAction, MarkdownField};

use super::super::state::{SaveRequest, WritingToolsCommand, WritingToolsTone};

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
    ApplyFormat(MarkdownField, MarkdownAction),
    ApplyBlockDir(MarkdownField, String),
    ConfirmDiscard,
    CancelDiscard,
    OpenDeleteModal,
    ToggleSaveMenu,
    CloseSaveMenu,
    ToggleWritingTools(MarkdownField),
    CloseWritingTools,
    UpdateWritingToolsPrompt(String),
    SelectWritingToolsTone(WritingToolsTone),
    SelectWritingToolsCommand(MarkdownField, WritingToolsCommand),
    WritingToolsReplace(MarkdownField),
    WritingToolsCopy(MarkdownField),
    Indent(MarkdownField, bool),
    CloseDeleteModal,
    OpenResetDeckModal,
    CloseResetDeckModal,
    ConfirmResetDeck,
    CloseDuplicateModal,
    ConfirmDuplicate,
    Delete,
    CancelNew,
}
