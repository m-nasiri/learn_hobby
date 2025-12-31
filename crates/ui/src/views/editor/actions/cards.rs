use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::*;

use crate::vm::CardListItemVm;
use crate::views::{ViewError, ViewState, view_state_from_resource};

use super::super::state::{
    DeleteState, EditorServices, EditorState, PendingAction, SaveMenuState, SaveState,
};

pub(super) fn build_select_card_action(state: &EditorState) -> Callback<CardListItemVm> {
    let state = state.clone();
    let set_editor_fields = Rc::clone(&state.set_editor_fields);
    let reset_duplicate_state = Rc::clone(&state.reset_duplicate_state);
    use_callback(move |item: CardListItemVm| {
        let set_editor_fields = Rc::clone(&set_editor_fields);
        let reset_duplicate_state = Rc::clone(&reset_duplicate_state);
        let mut selected_card_id = state.selected_card_id;
        let mut last_selected_card = state.last_selected_card;
        let mut is_create_mode = state.is_create_mode;
        let mut card_tags = state.card_tags;
        let mut last_selected_tags = state.last_selected_tags;
        let mut tag_input = state.tag_input;
        let mut save_state = state.save_state;
        let mut show_new_deck = state.show_new_deck;
        let mut new_deck_state = state.new_deck_state;
        let mut show_deck_menu = state.show_deck_menu;
        let mut is_renaming_deck = state.is_renaming_deck;
        let mut rename_deck_state = state.rename_deck_state;
        let mut rename_deck_error = state.rename_deck_error;
        let mut delete_state = state.delete_state;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut pending_action = state.pending_action;
        let mut show_validation = state.show_validation;
        let mut show_delete_modal = state.show_delete_modal;
        let mut focus_prompt = state.focus_prompt;

        selected_card_id.set(Some(item.id));
        last_selected_card.set(Some(item.clone()));
        is_create_mode.set(false);
        let prompt_html = item.prompt_html;
        let answer_html = item.answer_html;
        set_editor_fields.borrow_mut()(prompt_html, answer_html);
        card_tags.set(Vec::new());
        last_selected_tags.set(Vec::new());
        tag_input.set(String::new());
        save_state.set(SaveState::Idle);
        delete_state.set(DeleteState::Idle);
        show_validation.set(false);
        show_delete_modal.set(false);
        show_unsaved_modal.set(false);
        pending_action.set(None);
        reset_duplicate_state.borrow_mut()();
        focus_prompt.set(false);
        show_new_deck.set(false);
        new_deck_state.set(SaveState::Idle);
        show_deck_menu.set(false);
        is_renaming_deck.set(false);
        rename_deck_state.set(SaveState::Idle);
        rename_deck_error.set(None);
    })
}

pub(super) fn build_request_select_card_action(
    state: &EditorState,
    select_card_action: Callback<CardListItemVm>,
) -> Callback<CardListItemVm> {
    let state = state.clone();
    let has_unsaved_changes = Rc::clone(&state.has_unsaved_changes);
    use_callback(move |item: CardListItemVm| {
        let mut pending_action = state.pending_action;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut show_deck_menu = state.show_deck_menu;
        if has_unsaved_changes() {
            pending_action.set(Some(PendingAction::SelectCard(item)));
            show_unsaved_modal.set(true);
            show_deck_menu.set(false);
            return;
        }
        select_card_action.call(item);
    })
}

pub(super) fn build_new_card_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    let clear_editor_fields = Rc::clone(&state.clear_editor_fields);
    let reset_duplicate_state = Rc::clone(&state.reset_duplicate_state);
    use_callback(move |()| {
        let clear_editor_fields = Rc::clone(&clear_editor_fields);
        let reset_duplicate_state = Rc::clone(&reset_duplicate_state);
        let mut selected_card_id = state.selected_card_id;
        let mut is_create_mode = state.is_create_mode;
        let mut card_tags = state.card_tags;
        let mut tag_input = state.tag_input;
        let mut save_state = state.save_state;
        let mut show_new_deck = state.show_new_deck;
        let mut new_deck_state = state.new_deck_state;
        let mut show_deck_menu = state.show_deck_menu;
        let mut new_deck_name = state.new_deck_name;
        let mut is_renaming_deck = state.is_renaming_deck;
        let mut rename_deck_state = state.rename_deck_state;
        let mut rename_deck_error = state.rename_deck_error;
        let mut delete_state = state.delete_state;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut pending_action = state.pending_action;
        let mut last_focus_field = state.last_focus_field;
        let mut show_validation = state.show_validation;
        let mut show_delete_modal = state.show_delete_modal;
        let mut save_menu_state = state.save_menu_state;
        let mut focus_prompt = state.focus_prompt;

        selected_card_id.set(None);
        is_create_mode.set(true);
        clear_editor_fields.borrow_mut()();
        card_tags.set(Vec::new());
        tag_input.set(String::new());
        save_state.set(SaveState::Idle);
        delete_state.set(DeleteState::Idle);
        show_validation.set(false);
        show_delete_modal.set(false);
        show_unsaved_modal.set(false);
        pending_action.set(None);
        reset_duplicate_state.borrow_mut()();
        save_menu_state.set(SaveMenuState::Closed);
        focus_prompt.set(true);
        last_focus_field.set(crate::vm::MarkdownField::Front);
        show_new_deck.set(false);
        new_deck_state.set(SaveState::Idle);
        new_deck_name.set(String::new());
        show_deck_menu.set(false);
        is_renaming_deck.set(false);
        rename_deck_state.set(SaveState::Idle);
        rename_deck_error.set(None);
    })
}

pub(super) fn build_request_new_card_action(
    state: &EditorState,
    new_card_action: Callback<()>,
) -> Callback<()> {
    let state = state.clone();
    let has_unsaved_changes = Rc::clone(&state.has_unsaved_changes);
    use_callback(move |()| {
        let mut pending_action = state.pending_action;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut show_deck_menu = state.show_deck_menu;
        let mut save_menu_state = state.save_menu_state;
        if has_unsaved_changes() {
            pending_action.set(Some(PendingAction::NewCard));
            show_unsaved_modal.set(true);
            show_deck_menu.set(false);
            save_menu_state.set(SaveMenuState::Closed);
            return;
        }
        new_card_action.call(());
    })
}

pub(super) fn build_delete_action(
    state: &EditorState,
    services: &EditorServices,
) -> Callback<()> {
    let state = state.clone();
    let clear_editor_fields = Rc::clone(&state.clear_editor_fields);
    let card_service = services.card_service.clone();
    let selected_deck = state.selected_deck;
    use_callback(move |()| {
        let clear_editor_fields = Rc::clone(&clear_editor_fields);
        let card_service = card_service.clone();
        let mut delete_state = state.delete_state;
        let mut save_state = state.save_state;
        let mut cards_resource = state.cards_resource;
        let mut selected_card_id = state.selected_card_id;
        let mut last_selected_card = state.last_selected_card;
        let mut is_create_mode = state.is_create_mode;
        let mut card_tags = state.card_tags;
        let mut last_selected_tags = state.last_selected_tags;
        let mut tag_input = state.tag_input;
        let mut show_delete_modal = state.show_delete_modal;
        let mut save_menu_state = state.save_menu_state;
        let deck_id = *selected_deck.read();
        let Some(card_id) = selected_card_id() else {
            return;
        };

        if delete_state() == DeleteState::Deleting {
            return;
        }

        spawn(async move {
            delete_state.set(DeleteState::Deleting);
            save_state.set(SaveState::Idle);
            show_delete_modal.set(false);
            save_menu_state.set(SaveMenuState::Closed);
            let result = card_service.delete_card(deck_id, card_id).await;
            match result {
                Ok(()) => {
                    delete_state.set(DeleteState::Success);
                    selected_card_id.set(None);
                    last_selected_card.set(None);
                    is_create_mode.set(false);
                    clear_editor_fields.borrow_mut()();
                    card_tags.set(Vec::new());
                    last_selected_tags.set(Vec::new());
                    tag_input.set(String::new());
                    cards_resource.restart();
                    let mut delete_state = delete_state;
                    spawn(async move {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        if delete_state() == DeleteState::Success {
                            delete_state.set(DeleteState::Idle);
                        }
                    });
                }
                Err(_) => {
                    delete_state.set(DeleteState::Error(ViewError::Unknown));
                }
            }
        });
    })
}

pub(super) fn build_cancel_new_action(state: &EditorState) -> Callback<()> {
    let state = state.clone();
    let clear_editor_fields = Rc::clone(&state.clear_editor_fields);
    let set_editor_fields = Rc::clone(&state.set_editor_fields);
    let reset_duplicate_state = Rc::clone(&state.reset_duplicate_state);
    use_callback(move |()| {
        let clear_editor_fields = Rc::clone(&clear_editor_fields);
        let set_editor_fields = Rc::clone(&set_editor_fields);
        let reset_duplicate_state = Rc::clone(&reset_duplicate_state);
        let mut selected_card_id = state.selected_card_id;
        let last_selected_card = state.last_selected_card;
        let mut is_create_mode = state.is_create_mode;
        let mut card_tags = state.card_tags;
        let mut tag_input = state.tag_input;
        let mut save_state = state.save_state;
        let mut show_deck_menu = state.show_deck_menu;
        let mut delete_state = state.delete_state;
        let mut show_delete_modal = state.show_delete_modal;
        let mut show_validation = state.show_validation;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut pending_action = state.pending_action;
        let last_selected_tags = state.last_selected_tags;

        if !is_create_mode() {
            return;
        }

        if let Some(card) = last_selected_card() {
            selected_card_id.set(Some(card.id));
            set_editor_fields.borrow_mut()(card.prompt_html.clone(), card.answer_html.clone());
            card_tags.set(last_selected_tags());
            is_create_mode.set(false);
        } else {
            selected_card_id.set(None);
            clear_editor_fields.borrow_mut()();
            card_tags.set(Vec::new());
            is_create_mode.set(true);
        }

        tag_input.set(String::new());
        save_state.set(SaveState::Idle);
        delete_state.set(DeleteState::Idle);
        show_delete_modal.set(false);
        show_validation.set(false);
        show_unsaved_modal.set(false);
        pending_action.set(None);
        reset_duplicate_state.borrow_mut()();
        show_deck_menu.set(false);
    })
}

pub(super) fn use_cards_resource_effect(
    state: &EditorState,
    select_card_action: Callback<CardListItemVm>,
) {
    let state = state.clone();
    let clear_editor_fields = Rc::clone(&state.clear_editor_fields);
    use_effect(move || {
        let mut selected_card_id = state.selected_card_id;
        let mut last_selected_card = state.last_selected_card;
        let mut is_create_mode = state.is_create_mode;
        let mut save_state = state.save_state;
        let mut delete_state = state.delete_state;
        let mut show_delete_modal = state.show_delete_modal;
        let mut show_validation = state.show_validation;
        let mut show_unsaved_modal = state.show_unsaved_modal;
        let mut pending_action = state.pending_action;
        let mut focus_prompt = state.focus_prompt;
        let cards_state = view_state_from_resource(&state.cards_resource);
        if let ViewState::Ready(items) = &cards_state {
            if items.is_empty() {
                if !is_create_mode() {
                    selected_card_id.set(None);
                    last_selected_card.set(None);
                    is_create_mode.set(true);
                    clear_editor_fields.borrow_mut()();
                    save_state.set(SaveState::Idle);
                    delete_state.set(DeleteState::Idle);
                    show_delete_modal.set(false);
                    show_validation.set(false);
                    show_unsaved_modal.set(false);
                    pending_action.set(None);
                    focus_prompt.set(true);
                }
            } else if selected_card_id().is_none()
                && !is_create_mode()
                && let Some(first) = items.first()
            {
                select_card_action.call(first.clone());
            }
        }
    });
}
