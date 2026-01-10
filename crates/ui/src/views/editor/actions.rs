mod cards;
mod decks;
mod format;
mod intent;
mod keyboard;
mod menus;
mod save;
mod tags;

use dioxus::prelude::*;
use dioxus_router::use_navigator;

use crate::vm::{CardListItemVm, MarkdownAction, MarkdownField};

use super::state::{
    EditorServices, EditorState, SaveRequest, WritingToolsCommand, WritingToolsTone,
};

pub use intent::EditorIntent;

#[derive(Clone)]
pub struct EditorDispatcher {
    pub dispatch: Callback<EditorIntent>,
    pub on_key: Callback<KeyboardEvent>,
    pub list_on_key: Callback<KeyboardEvent>,
}

#[derive(Clone)]
struct EditorActionHandlers {
    save: Callback<SaveRequest>,
    create_deck: Callback<()>,
    cancel_rename: Callback<()>,
    commit_rename: Callback<()>,
    begin_rename: Callback<String>,
    request_select_deck: Callback<learn_core::model::DeckId>,
    request_select_card: Callback<CardListItemVm>,
    request_new_card: Callback<()>,
    add_tag: Callback<String>,
    remove_tag: Callback<String>,
    set_tag_filter: Callback<Option<String>>,
    handle_paste: Callback<MarkdownField>,
    apply_format: Callback<(MarkdownField, MarkdownAction)>,
    apply_block_dir: Callback<(MarkdownField, String)>,
    confirm_discard: Callback<()>,
    cancel_discard: Callback<()>,
    open_delete_modal: Callback<()>,
    toggle_save_menu: Callback<()>,
    close_save_menu: Callback<()>,
    toggle_writing_tools: Callback<MarkdownField>,
    close_writing_tools: Callback<()>,
    update_writing_tools_prompt: Callback<String>,
    select_writing_tools_tone: Callback<WritingToolsTone>,
    select_writing_tools_command: Callback<(MarkdownField, WritingToolsCommand)>,
    close_delete_modal: Callback<()>,
    open_reset_deck_modal: Callback<()>,
    close_reset_deck_modal: Callback<()>,
    confirm_reset_deck: Callback<()>,
    close_duplicate_modal: Callback<()>,
    confirm_duplicate: Callback<()>,
    delete: Callback<()>,
    cancel_new: Callback<()>,
}

pub fn use_editor_dispatcher(state: &EditorState, services: &EditorServices) -> EditorDispatcher {
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
    let open_reset_deck_modal_action = decks::build_open_reset_deck_modal_action(&state);
    let close_reset_deck_modal_action = decks::build_close_reset_deck_modal_action(&state);
    let confirm_reset_deck_action = decks::build_confirm_reset_deck_action(&state, &services);
    let toggle_save_menu_action = menus::build_toggle_save_menu_action(&state);
    let close_save_menu_action = menus::build_close_save_menu_action(&state);
    let toggle_writing_tools_action = menus::build_toggle_writing_tools_action(&state);
    let close_writing_tools_action = menus::build_close_writing_tools_action(&state);
    let update_writing_tools_prompt_action = menus::build_update_writing_tools_prompt_action(&state);
    let select_writing_tools_tone_action = menus::build_select_writing_tools_tone_action(&state);
    let select_writing_tools_command_action =
        menus::build_select_writing_tools_command_action(&state);
    let close_delete_modal_action = menus::build_close_delete_modal_action(&state);
    let close_duplicate_modal_action = menus::build_close_duplicate_modal_action(&state);
    let confirm_duplicate_action = menus::build_confirm_duplicate_action(&state, save_action);
    let delete_action = cards::build_delete_action(&state, &services);
    let cancel_new_action = cards::build_cancel_new_action(&state);

    cards::use_cards_resource_effect(&state, select_card_action);

    let handlers = EditorActionHandlers {
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
        open_reset_deck_modal: open_reset_deck_modal_action,
        close_reset_deck_modal: close_reset_deck_modal_action,
        confirm_reset_deck: confirm_reset_deck_action,
        toggle_save_menu: toggle_save_menu_action,
        close_save_menu: close_save_menu_action,
        toggle_writing_tools: toggle_writing_tools_action,
        close_writing_tools: close_writing_tools_action,
        update_writing_tools_prompt: update_writing_tools_prompt_action,
        select_writing_tools_tone: select_writing_tools_tone_action,
        select_writing_tools_command: select_writing_tools_command_action,
        close_delete_modal: close_delete_modal_action,
        close_duplicate_modal: close_duplicate_modal_action,
        confirm_duplicate: confirm_duplicate_action,
        delete: delete_action,
        cancel_new: cancel_new_action,
    };

    let dispatch = {
        let handlers = handlers.clone();
        use_callback(move |intent: EditorIntent| dispatch_intent(intent, &handlers))
    };

    let on_key = keyboard::build_on_key_action(&state, dispatch);
    let list_on_key = keyboard::build_list_on_key_action(&state, dispatch);

    EditorDispatcher {
        dispatch,
        on_key,
        list_on_key,
    }
}

fn dispatch_intent(intent: EditorIntent, handlers: &EditorActionHandlers) {
    match intent {
        EditorIntent::Save(request) => handlers.save.call(request),
        EditorIntent::CreateDeck => handlers.create_deck.call(()),
        EditorIntent::CancelRename => handlers.cancel_rename.call(()),
        EditorIntent::CommitRename => handlers.commit_rename.call(()),
        EditorIntent::BeginRename(label) => handlers.begin_rename.call(label),
        EditorIntent::RequestSelectDeck(deck_id) => handlers.request_select_deck.call(deck_id),
        EditorIntent::RequestSelectCard(card) => handlers.request_select_card.call(card),
        EditorIntent::RequestNewCard => handlers.request_new_card.call(()),
        EditorIntent::AddTag(tag) => handlers.add_tag.call(tag),
        EditorIntent::RemoveTag(tag) => handlers.remove_tag.call(tag),
        EditorIntent::SetTagFilter(tag) => handlers.set_tag_filter.call(tag),
        EditorIntent::HandlePaste(field) => handlers.handle_paste.call(field),
        EditorIntent::ApplyFormat(field, action) => {
            handlers.apply_format.call((field, action));
        }
        EditorIntent::ApplyBlockDir(field, dir) => {
            handlers.apply_block_dir.call((field, dir));
        }
        EditorIntent::ConfirmDiscard => handlers.confirm_discard.call(()),
        EditorIntent::CancelDiscard => handlers.cancel_discard.call(()),
        EditorIntent::OpenDeleteModal => handlers.open_delete_modal.call(()),
        EditorIntent::OpenResetDeckModal => handlers.open_reset_deck_modal.call(()),
        EditorIntent::CloseResetDeckModal => handlers.close_reset_deck_modal.call(()),
        EditorIntent::ConfirmResetDeck => handlers.confirm_reset_deck.call(()),
        EditorIntent::ToggleSaveMenu => handlers.toggle_save_menu.call(()),
        EditorIntent::CloseSaveMenu => handlers.close_save_menu.call(()),
        EditorIntent::ToggleWritingTools(field) => handlers.toggle_writing_tools.call(field),
        EditorIntent::CloseWritingTools => handlers.close_writing_tools.call(()),
        EditorIntent::UpdateWritingToolsPrompt(value) => {
            handlers.update_writing_tools_prompt.call(value);
        }
        EditorIntent::SelectWritingToolsTone(tone) => {
            handlers.select_writing_tools_tone.call(tone);
        }
        EditorIntent::SelectWritingToolsCommand(field, command) => {
            handlers.select_writing_tools_command.call((field, command));
        }
        EditorIntent::CloseDeleteModal => handlers.close_delete_modal.call(()),
        EditorIntent::CloseDuplicateModal => handlers.close_duplicate_modal.call(()),
        EditorIntent::ConfirmDuplicate => handlers.confirm_duplicate.call(()),
        EditorIntent::Delete => handlers.delete.call(()),
        EditorIntent::CancelNew => handlers.cancel_new.call(()),
    }
}
