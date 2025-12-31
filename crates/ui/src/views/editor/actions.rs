mod cards;
mod decks;
mod format;
mod keyboard;
mod menus;
mod save;
mod tags;

use dioxus::prelude::*;
use dioxus_router::use_navigator;

use crate::vm::{CardListItemVm, MarkdownAction, MarkdownField};

use super::state::{EditorServices, EditorState, SaveRequest};

#[derive(Clone)]
pub struct EditorActions {
    pub save: Callback<SaveRequest>,
    pub create_deck: Callback<()>,
    pub cancel_rename: Callback<()>,
    pub commit_rename: Callback<()>,
    pub begin_rename: Callback<String>,
    pub request_select_deck: Callback<learn_core::model::DeckId>,
    pub request_select_card: Callback<CardListItemVm>,
    pub request_new_card: Callback<()>,
    pub add_tag: Callback<String>,
    pub remove_tag: Callback<String>,
    pub set_tag_filter: Callback<Option<String>>,
    pub handle_paste: Callback<MarkdownField>,
    pub apply_format: Callback<(MarkdownField, MarkdownAction)>,
    pub apply_block_dir: Callback<(MarkdownField, String)>,
    pub confirm_discard: Callback<()>,
    pub cancel_discard: Callback<()>,
    pub open_delete_modal: Callback<()>,
    pub toggle_save_menu: Callback<()>,
    pub close_save_menu: Callback<()>,
    pub close_delete_modal: Callback<()>,
    pub close_duplicate_modal: Callback<()>,
    pub confirm_duplicate: Callback<()>,
    pub delete: Callback<()>,
    pub cancel_new: Callback<()>,
    pub on_key: Callback<KeyboardEvent>,
    pub list_on_key: Callback<KeyboardEvent>,
}

pub fn use_editor_actions(state: &EditorState, services: &EditorServices) -> EditorActions {
    let navigator = use_navigator();
    let state = state.clone();
    let services = services.clone();

    let save_action = save::build_save_action(&state, &services, navigator);
    let create_deck_action = decks::build_create_deck_action(&state, &services);
    let (cancel_rename_action, commit_rename_action, begin_rename_action) =
        decks::build_rename_actions(&state, &services);
    let apply_select_deck_action = decks::build_apply_select_deck_action(&state);
    let select_card_action = cards::build_select_card_action(&state);
    let request_select_card_action =
        cards::build_request_select_card_action(&state, select_card_action);
    let request_select_deck_action =
        decks::build_request_select_deck_action(&state, apply_select_deck_action);
    let new_card_action = cards::build_new_card_action(&state);
    let request_new_card_action = cards::build_request_new_card_action(&state, new_card_action);
    let (add_tag_action, remove_tag_action, set_tag_filter_action) =
        tags::build_tag_actions(&state);
    let handle_paste_action = format::build_paste_action(&state);
    let (apply_format_action, apply_block_dir_action) = format::build_format_actions(&state);
    let (confirm_discard_action, cancel_discard_action) = menus::build_discard_actions(
        &state,
        select_card_action,
        apply_select_deck_action,
        new_card_action,
    );
    let open_delete_modal_action = menus::build_open_delete_modal_action(&state);
    let toggle_save_menu_action = menus::build_toggle_save_menu_action(&state);
    let close_save_menu_action = menus::build_close_save_menu_action(&state);
    let close_delete_modal_action = menus::build_close_delete_modal_action(&state);
    let close_duplicate_modal_action = menus::build_close_duplicate_modal_action(&state);
    let confirm_duplicate_action = menus::build_confirm_duplicate_action(&state, save_action);
    let delete_action = cards::build_delete_action(&state, &services);
    let cancel_new_action = cards::build_cancel_new_action(&state);

    cards::use_cards_resource_effect(&state, select_card_action);

    let on_key = keyboard::build_on_key_action(
        &state,
        save_action,
        request_new_card_action,
        open_delete_modal_action,
        apply_format_action,
        cancel_new_action,
    );
    let list_on_key = keyboard::build_list_on_key_action(&state, request_select_card_action);

    EditorActions {
        save: save_action,
        create_deck: create_deck_action,
        cancel_rename: cancel_rename_action,
        commit_rename: commit_rename_action,
        begin_rename: begin_rename_action,
        request_select_deck: request_select_deck_action,
        request_select_card: request_select_card_action,
        request_new_card: request_new_card_action,
        add_tag: add_tag_action,
        remove_tag: remove_tag_action,
        set_tag_filter: set_tag_filter_action,
        handle_paste: handle_paste_action,
        apply_format: apply_format_action,
        apply_block_dir: apply_block_dir_action,
        confirm_discard: confirm_discard_action,
        cancel_discard: cancel_discard_action,
        open_delete_modal: open_delete_modal_action,
        toggle_save_menu: toggle_save_menu_action,
        close_save_menu: close_save_menu_action,
        close_delete_modal: close_delete_modal_action,
        close_duplicate_modal: close_duplicate_modal_action,
        confirm_duplicate: confirm_duplicate_action,
        delete: delete_action,
        cancel_new: cancel_new_action,
        on_key,
        list_on_key,
    }
}
