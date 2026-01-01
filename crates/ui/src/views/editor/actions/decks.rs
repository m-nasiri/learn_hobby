use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::*;
use learn_core::model::DeckSettings;

use crate::views::ViewError;

use super::super::state::{
    DeleteState, EditorServices, EditorState, PendingAction, ResetDeckState, SaveMenuState,
    SaveState,
};

pub(super) fn build_create_deck_action(
    state: &EditorState,
    services: &EditorServices,
) -> Callback<()> {
    let state = state.clone();
    let clear_editor_fields = Rc::clone(&state.clear_editor_fields);
    let deck_service = services.deck_service.clone();
    use_callback(move |()| {
        let clear_editor_fields = Rc::clone(&clear_editor_fields);
        let deck_service = deck_service.clone();
        let mut show_new_deck = state.show_new_deck;
        let mut new_deck_state = state.new_deck_state;
        let mut new_deck_name = state.new_deck_name;
        let mut selected_deck = state.selected_deck;
        let mut decks_resource = state.decks_resource;
        let mut cards_resource = state.cards_resource;
        let mut selected_card_id = state.selected_card_id;
        let mut last_selected_card = state.last_selected_card;
        let mut is_create_mode = state.is_create_mode;
        let mut show_deck_menu = state.show_deck_menu;
        let mut show_deck_actions = state.show_deck_actions;
        let mut is_renaming_deck = state.is_renaming_deck;
        let mut rename_deck_state = state.rename_deck_state;
        let mut rename_deck_error = state.rename_deck_error;
        let mut delete_state = state.delete_state;
        let mut show_validation = state.show_validation;
        let mut show_delete_modal = state.show_delete_modal;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut pending_action = state.pending_action;
        let mut save_menu_state = state.save_menu_state;
        let mut focus_prompt = state.focus_prompt;

        let name = new_deck_name.read().to_string();
        if !is_valid_deck_name(&name) || new_deck_state() == SaveState::Saving {
            return;
        }
        let name = name.trim().to_owned();

        spawn(async move {
            new_deck_state.set(SaveState::Saving);
            let result = deck_service
                .create_deck(name.clone(), None, DeckSettings::default_for_adhd())
                .await;

            match result {
                Ok(deck_id) => {
                    selected_deck.set(deck_id);
                    new_deck_name.set(String::new());
                    show_new_deck.set(false);
                    show_deck_menu.set(false);
                    show_deck_actions.set(false);
                    is_renaming_deck.set(false);
                    rename_deck_state.set(SaveState::Idle);
                    rename_deck_error.set(None);
                    delete_state.set(DeleteState::Idle);
                    show_delete_modal.set(false);
                    show_validation.set(false);
                    show_unsaved_modal.set(false);
                    pending_action.set(None);
                    focus_prompt.set(false);
                    save_menu_state.set(SaveMenuState::Closed);
                    new_deck_state.set(SaveState::Success);
                    decks_resource.restart();
                    cards_resource.restart();
                    selected_card_id.set(None);
                    last_selected_card.set(None);
                    is_create_mode.set(true);
                    clear_editor_fields.borrow_mut()();
                }
                Err(_) => {
                    new_deck_state.set(SaveState::Error(ViewError::Unknown));
                }
            }
        });
    })
}

pub(super) fn build_rename_actions(
    state: &EditorState,
    services: &EditorServices,
) -> (Callback<()>, Callback<()>, Callback<String>) {
    let state_for_cancel = state.clone();
    let cancel_rename_action = use_callback(move |()| {
        let mut is_renaming_deck = state_for_cancel.is_renaming_deck;
        let mut rename_deck_state = state_for_cancel.rename_deck_state;
        let mut rename_deck_error = state_for_cancel.rename_deck_error;
        let mut rename_deck_name = state_for_cancel.rename_deck_name;

        is_renaming_deck.set(false);
        rename_deck_state.set(SaveState::Idle);
        rename_deck_error.set(None);
        rename_deck_name.set(String::new());
    });

    let state_for_commit = state.clone();
    let deck_service = services.deck_service.clone();
    let commit_rename_action = use_callback(move |()| {
        let deck_service = deck_service.clone();
        let mut rename_deck_state = state_for_commit.rename_deck_state;
        let mut rename_deck_error = state_for_commit.rename_deck_error;
        let mut is_renaming_deck = state_for_commit.is_renaming_deck;
        let mut decks_resource = state_for_commit.decks_resource;
        let deck_id = *state_for_commit.selected_deck.read();
        let name = state_for_commit.rename_deck_name.read().to_string();

        if !is_valid_deck_name(&name) || rename_deck_state() == SaveState::Saving {
            rename_deck_error.set(Some("Name cannot be empty.".to_string()));
            return;
        }
        let name = name.trim().to_owned();

        spawn(async move {
            rename_deck_state.set(SaveState::Saving);
            rename_deck_error.set(None);

            if deck_service.rename_deck(deck_id, name).await.is_ok() {
                rename_deck_state.set(SaveState::Success);
                is_renaming_deck.set(false);
                decks_resource.restart();
            } else {
                rename_deck_state.set(SaveState::Error(ViewError::Unknown));
                let message = "Rename failed. Please try again.".to_string();
                rename_deck_error.set(Some(message.clone()));
                let mut rename_deck_error = rename_deck_error;
                spawn(async move {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    if rename_deck_error.read().as_ref() == Some(&message) {
                        rename_deck_error.set(None);
                    }
                });
            }
        });
    });

    let state_for_begin = state.clone();
    let begin_rename_action = use_callback(move |label: String| {
        let mut is_renaming_deck = state_for_begin.is_renaming_deck;
        let mut rename_deck_name = state_for_begin.rename_deck_name;
        let mut rename_deck_state = state_for_begin.rename_deck_state;
        let mut rename_deck_error = state_for_begin.rename_deck_error;
        let mut show_deck_menu = state_for_begin.show_deck_menu;
        let mut show_deck_actions = state_for_begin.show_deck_actions;
        let mut show_new_deck = state_for_begin.show_new_deck;
        let mut new_deck_state = state_for_begin.new_deck_state;

        rename_deck_name.set(label);
        rename_deck_state.set(SaveState::Idle);
        rename_deck_error.set(None);
        show_deck_menu.set(false);
        show_deck_actions.set(false);
        show_new_deck.set(false);
        new_deck_state.set(SaveState::Idle);
        is_renaming_deck.set(true);
    });

    (cancel_rename_action, commit_rename_action, begin_rename_action)
}

pub(super) fn build_apply_select_deck_action(state: &EditorState) -> Callback<learn_core::model::DeckId> {
    let state = state.clone();
    let clear_editor_fields = Rc::clone(&state.clear_editor_fields);
    let reset_duplicate_state = Rc::clone(&state.reset_duplicate_state);
    use_callback(move |deck_id| {
        let clear_editor_fields = Rc::clone(&clear_editor_fields);
        let reset_duplicate_state = Rc::clone(&reset_duplicate_state);
        let mut selected_deck = state.selected_deck;
        let mut show_new_deck = state.show_new_deck;
        let mut new_deck_state = state.new_deck_state;
        let mut selected_card_id = state.selected_card_id;
        let mut last_selected_card = state.last_selected_card;
        let mut is_create_mode = state.is_create_mode;
        let mut card_tags = state.card_tags;
        let mut last_selected_tags = state.last_selected_tags;
        let mut tag_input = state.tag_input;
        let mut selected_tag_filters = state.selected_tag_filters;
        let mut save_state = state.save_state;
        let mut delete_state = state.delete_state;
        let mut show_delete_modal = state.show_delete_modal;
        let mut show_validation = state.show_validation;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut pending_action = state.pending_action;
        let mut focus_prompt = state.focus_prompt;
        let mut show_deck_menu = state.show_deck_menu;
        let mut show_deck_actions = state.show_deck_actions;
        let mut new_deck_name = state.new_deck_name;
        let mut is_renaming_deck = state.is_renaming_deck;
        let mut rename_deck_state = state.rename_deck_state;
        let mut rename_deck_error = state.rename_deck_error;

        selected_deck.set(deck_id);
        show_new_deck.set(false);
        new_deck_state.set(SaveState::Idle);
        selected_card_id.set(None);
        last_selected_card.set(None);
        is_create_mode.set(false);
        clear_editor_fields.borrow_mut()();
        card_tags.set(Vec::new());
        last_selected_tags.set(Vec::new());
        tag_input.set(String::new());
        selected_tag_filters.set(Vec::new());
        save_state.set(SaveState::Idle);
        delete_state.set(DeleteState::Idle);
        show_delete_modal.set(false);
        show_validation.set(false);
        show_unsaved_modal.set(false);
        pending_action.set(None);
        reset_duplicate_state.borrow_mut()();
        focus_prompt.set(false);
        show_deck_menu.set(false);
        show_deck_actions.set(false);
        new_deck_name.set(String::new());
        is_renaming_deck.set(false);
        rename_deck_state.set(SaveState::Idle);
        rename_deck_error.set(None);
    })
}

pub(super) fn build_request_select_deck_action(
    state: &EditorState,
    apply_select_deck_action: Callback<learn_core::model::DeckId>,
) -> Callback<learn_core::model::DeckId> {
    let state = state.clone();
    let has_unsaved_changes = Rc::clone(&state.has_unsaved_changes);
    use_callback(move |deck_id| {
        let mut pending_action = state.pending_action;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut show_deck_menu = state.show_deck_menu;
        if has_unsaved_changes() {
            pending_action.set(Some(PendingAction::SelectDeck(deck_id)));
            show_unsaved_modal.set(true);
            show_deck_menu.set(false);
            return;
        }
        apply_select_deck_action.call(deck_id);
    })
}

pub(super) fn build_open_reset_deck_modal_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut show_reset_deck_modal = state.show_reset_deck_modal;
        let mut reset_deck_state = state.reset_deck_state;
        let mut show_deck_actions = state.show_deck_actions;
        let mut show_deck_menu = state.show_deck_menu;
        let mut is_renaming_deck = state.is_renaming_deck;
        let mut rename_deck_state = state.rename_deck_state;
        let mut rename_deck_error = state.rename_deck_error;
        show_deck_actions.set(false);
        show_deck_menu.set(false);
        is_renaming_deck.set(false);
        rename_deck_state.set(SaveState::Idle);
        rename_deck_error.set(None);
        reset_deck_state.set(ResetDeckState::Idle);
        show_reset_deck_modal.set(true);
    })
}

pub(super) fn build_close_reset_deck_modal_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    use_callback(move |()| {
        let mut show_reset_deck_modal = state.show_reset_deck_modal;
        let mut reset_deck_state = state.reset_deck_state;
        show_reset_deck_modal.set(false);
        reset_deck_state.set(ResetDeckState::Idle);
    })
}

pub(super) fn build_confirm_reset_deck_action(
    state: &EditorState,
    services: &EditorServices,
) -> Callback<()> {
    let state = state.clone();
    let card_service = services.card_service.clone();
    use_callback(move |()| {
        let card_service = card_service.clone();
        let mut show_reset_deck_modal = state.show_reset_deck_modal;
        let mut reset_deck_state = state.reset_deck_state;
        let mut cards_resource = state.cards_resource;
        let deck_id = *state.selected_deck.read();
        spawn(async move {
            reset_deck_state.set(ResetDeckState::Resetting);
            match card_service.reset_deck_learning(deck_id).await {
                Ok(_) => {
                    reset_deck_state.set(ResetDeckState::Success);
                    show_reset_deck_modal.set(false);
                    cards_resource.restart();
                }
                Err(_) => {
                    reset_deck_state.set(ResetDeckState::Error(ViewError::Unknown));
                }
            }
        });
    })
}

fn is_valid_deck_name(name: &str) -> bool {
    !name.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::is_valid_deck_name;

    #[test]
    fn valid_deck_name_rejects_empty() {
        assert!(!is_valid_deck_name(""));
        assert!(!is_valid_deck_name("   "));
    }

    #[test]
    fn valid_deck_name_accepts_non_empty() {
        assert!(is_valid_deck_name("Default"));
        assert!(is_valid_deck_name("  Deck  "));
    }
}
