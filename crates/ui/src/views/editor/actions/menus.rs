use std::rc::Rc;

use dioxus::prelude::*;

use super::super::state::{
    DuplicateCheckState, EditorState, PendingAction, SaveMenuState, SaveRequest, SaveState,
    WritingToolsCommand, WritingToolsMenuState, WritingToolsTone,
};
use crate::vm::{CardListItemVm, MarkdownField};

pub(super) fn build_discard_actions(
    state: &EditorState,
    select_card_action: Callback<CardListItemVm>,
    apply_select_deck_action: Callback<learn_core::model::DeckId>,
    new_card_action: Callback<()>,
) -> (Callback<()>, Callback<()>) {
    let state_for_confirm = state.clone();
    let confirm_discard_action = use_callback(move |()| {
        let mut show_unsaved_modal = state_for_confirm.show_unsaved_modal;
        let mut pending_action = state_for_confirm.pending_action;
        let mut save_menu_state = state_for_confirm.save_menu_state;
        let mut writing_tools_menu_state = state_for_confirm.writing_tools_menu_state;
        if let Some(action) = pending_action() {
            match action {
                PendingAction::SelectCard(item) => {
                    select_card_action.call(item);
                }
                PendingAction::SelectDeck(deck_id) => {
                    apply_select_deck_action.call(deck_id);
                }
                PendingAction::NewCard => {
                    new_card_action.call(());
                }
            }
        }
        show_unsaved_modal.set(false);
        pending_action.set(None);
        save_menu_state.set(SaveMenuState::Closed);
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
    });

    let state_for_cancel = state.clone();
    let cancel_discard_action = use_callback(move |()| {
        let mut show_unsaved_modal = state_for_cancel.show_unsaved_modal;
        let mut pending_action = state_for_cancel.pending_action;
        let mut save_menu_state = state_for_cancel.save_menu_state;
        let mut writing_tools_menu_state = state_for_cancel.writing_tools_menu_state;
        show_unsaved_modal.set(false);
        pending_action.set(None);
        save_menu_state.set(SaveMenuState::Closed);
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
    });

    (confirm_discard_action, cancel_discard_action)
}

pub(super) fn build_open_delete_modal_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    let reset_duplicate_state = Rc::clone(&state.reset_duplicate_state);
    use_callback(move |()| {
        let reset_duplicate_state = Rc::clone(&reset_duplicate_state);
        let mut show_delete_modal = state.show_delete_modal;
        let mut show_deck_menu = state.show_deck_menu;
        let mut show_deck_actions = state.show_deck_actions;
        let mut is_renaming_deck = state.is_renaming_deck;
        let mut rename_deck_state = state.rename_deck_state;
        let mut rename_deck_error = state.rename_deck_error;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut pending_action = state.pending_action;
        let mut save_menu_state = state.save_menu_state;
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        let selected_card_id = (state.selected_card_id)();
        if selected_card_id.is_some() {
            show_deck_menu.set(false);
            show_deck_actions.set(false);
            is_renaming_deck.set(false);
            rename_deck_state.set(SaveState::Idle);
            rename_deck_error.set(None);
            show_unsaved_modal.set(false);
            pending_action.set(None);
            save_menu_state.set(SaveMenuState::Closed);
            writing_tools_menu_state.set(WritingToolsMenuState::Closed);
            reset_duplicate_state.borrow_mut()();
            show_delete_modal.set(true);
        }
    })
}

pub(super) fn build_toggle_save_menu_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut save_menu_state = state.save_menu_state;
        let mut show_delete_modal = state.show_delete_modal;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        if save_menu_state() == SaveMenuState::Open {
            save_menu_state.set(SaveMenuState::Closed);
        } else {
            show_delete_modal.set(false);
            show_unsaved_modal.set(false);
            writing_tools_menu_state.set(WritingToolsMenuState::Closed);
            save_menu_state.set(SaveMenuState::Open);
        }
    })
}

pub(super) fn build_close_save_menu_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut save_menu_state = state.save_menu_state;
        save_menu_state.set(SaveMenuState::Closed);
    })
}

pub(super) fn build_toggle_writing_tools_action(state: &EditorState) -> Callback<MarkdownField> {
    let state = state.clone();
    use_callback(move |field: MarkdownField| {
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        let mut save_menu_state = state.save_menu_state;
        let mut show_delete_modal = state.show_delete_modal;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut show_deck_menu = state.show_deck_menu;
        let mut show_deck_actions = state.show_deck_actions;
        let mut is_renaming_deck = state.is_renaming_deck;
        let mut rename_deck_state = state.rename_deck_state;
        let mut rename_deck_error = state.rename_deck_error;
        let mut show_new_deck = state.show_new_deck;
        let mut new_deck_state = state.new_deck_state;
        match writing_tools_menu_state() {
            WritingToolsMenuState::Open(current) if current == field => {
                writing_tools_menu_state.set(WritingToolsMenuState::Closed);
            }
            _ => {
                save_menu_state.set(SaveMenuState::Closed);
                show_delete_modal.set(false);
                show_unsaved_modal.set(false);
                show_deck_menu.set(false);
                show_deck_actions.set(false);
                is_renaming_deck.set(false);
                rename_deck_state.set(SaveState::Idle);
                rename_deck_error.set(None);
                show_new_deck.set(false);
                new_deck_state.set(SaveState::Idle);
                writing_tools_menu_state.set(WritingToolsMenuState::Open(field));
            }
        }
    })
}

pub(super) fn build_close_writing_tools_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
    })
}

pub(super) fn build_update_writing_tools_prompt_action(state: &EditorState) -> Callback<String> {
    let state = state.clone();
    use_callback(move |value: String| {
        let mut writing_tools_prompt = state.writing_tools_prompt;
        writing_tools_prompt.set(value);
    })
}

pub(super) fn build_select_writing_tools_tone_action(
    state: &EditorState,
) -> Callback<WritingToolsTone> {
    let state = state.clone();
    use_callback(move |tone: WritingToolsTone| {
        let mut writing_tools_tone = state.writing_tools_tone;
        writing_tools_tone.set(tone);
    })
}

pub(super) fn build_select_writing_tools_command_action(
    state: &EditorState,
) -> Callback<(MarkdownField, WritingToolsCommand)> {
    let state = state.clone();
    use_callback(move |(field, command): (MarkdownField, WritingToolsCommand)| {
        let mut writing_tools_last_command = state.writing_tools_last_command;
        let mut writing_tools_menu_state = state.writing_tools_menu_state;
        let mut last_focus_field = state.last_focus_field;
        writing_tools_last_command.set(Some(command));
        last_focus_field.set(field);
        writing_tools_menu_state.set(WritingToolsMenuState::Closed);
    })
}

pub(super) fn build_close_delete_modal_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut show_delete_modal = state.show_delete_modal;
        show_delete_modal.set(false);
    })
}

pub(super) fn build_close_duplicate_modal_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut show_duplicate_modal = state.show_duplicate_modal;
        let mut pending_duplicate_practice = state.pending_duplicate_practice;
        let mut duplicate_check_state = state.duplicate_check_state;
        show_duplicate_modal.set(false);
        pending_duplicate_practice.set(false);
        duplicate_check_state.set(DuplicateCheckState::Idle);
    })
}

pub(super) fn build_confirm_duplicate_action(
    state: &EditorState,
    save_action: Callback<SaveRequest>,
) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut show_duplicate_modal = state.show_duplicate_modal;
        let mut pending_duplicate_practice = state.pending_duplicate_practice;
        let mut duplicate_check_state = state.duplicate_check_state;
        let practice = pending_duplicate_practice();
        show_duplicate_modal.set(false);
        pending_duplicate_practice.set(false);
        duplicate_check_state.set(DuplicateCheckState::Idle);
        save_action.call(SaveRequest::force(practice));
    })
}
