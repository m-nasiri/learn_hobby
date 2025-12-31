use dioxus::document::eval;
use dioxus::prelude::*;

use crate::vm::{CardListItemVm, DeckOptionVm, MarkdownAction, MarkdownField, filter_card_list_items};
use crate::views::{ViewState, view_state_from_resource};

use super::super::state::{DeleteState, EditorState, SaveRequest, SaveState};
use super::super::scripts::exec_command_script;

fn current_deck_label(
    decks_state: &ViewState<Vec<DeckOptionVm>>,
    selected_deck: learn_core::model::DeckId,
) -> String {
    match decks_state {
        ViewState::Ready(options) => options
            .iter()
            .find(|opt| opt.id == selected_deck)
            .map_or_else(|| "Deck".to_string(), |opt| opt.label.clone()),
        _ => "Deck".to_string(),
    }
}

fn handle_undo_redo(state: &EditorState, evt: &KeyboardEvent) -> bool {
    let modifiers = evt.data.modifiers();
    if !modifiers.contains(Modifiers::CONTROL) && !modifiers.contains(Modifiers::META) {
        return false;
    }

    let wants_undo = matches!(
        evt.data.key(),
        Key::Character(value) if value.eq_ignore_ascii_case("z")
    );
    let wants_redo = matches!(
        evt.data.key(),
        Key::Character(value) if value.eq_ignore_ascii_case("y")
    );
    if !wants_undo && !wants_redo {
        return false;
    }

    let active_field = (state.last_focus_field)();
    let element_id = if active_field == MarkdownField::Front {
        "prompt"
    } else {
        "answer"
    };
    let redo = wants_redo || (wants_undo && modifiers.contains(Modifiers::SHIFT));
    let command = if redo { "redo" } else { "undo" };
    evt.prevent_default();
    spawn(async move {
        let script = exec_command_script(element_id, command, None);
        let _ = eval(&script).await;
    });
    true
}

fn handle_primary_meta_actions(
    state: &EditorState,
    evt: &KeyboardEvent,
    save_action: Callback<SaveRequest>,
    request_new_card_action: Callback<()>,
    open_delete_modal_action: Callback<()>,
) -> bool {
    if !evt.data.modifiers().contains(Modifiers::META) {
        return false;
    }

    if evt.data.key() == Key::Enter {
        evt.prevent_default();
        save_action.call(SaveRequest::new(false));
        return true;
    }

    if matches!(evt.data.key(), Key::Character(value) if value.eq_ignore_ascii_case("n")) {
        evt.prevent_default();
        request_new_card_action.call(());
        return true;
    }

    if evt.data.key() == Key::Backspace
        && (state.selected_card_id)().is_some()
        && !(state.is_create_mode)()
        && (state.delete_state)() != DeleteState::Deleting
    {
        evt.prevent_default();
        open_delete_modal_action.call(());
        return true;
    }

    false
}

fn handle_rename_shortcut(state: &EditorState, evt: &KeyboardEvent) -> bool {
    if !evt.data.modifiers().contains(Modifiers::META) {
        return false;
    }
    if !matches!(evt.data.key(), Key::Character(value) if value.eq_ignore_ascii_case("r")) {
        return false;
    }
    if !matches!(
        view_state_from_resource(&state.decks_resource),
        ViewState::Ready(_)
    ) {
        return false;
    }

    evt.prevent_default();
    let decks_state = view_state_from_resource(&state.decks_resource);
    let label = current_deck_label(&decks_state, *state.selected_deck.read());
    let mut rename_deck_name = state.rename_deck_name;
    let mut rename_deck_state = state.rename_deck_state;
    let mut rename_deck_error = state.rename_deck_error;
    let mut show_deck_menu = state.show_deck_menu;
    let mut show_delete_modal = state.show_delete_modal;
    let mut is_renaming_deck = state.is_renaming_deck;
    rename_deck_name.set(label);
    rename_deck_state.set(SaveState::Idle);
    rename_deck_error.set(None);
    show_deck_menu.set(false);
    show_delete_modal.set(false);
    is_renaming_deck.set(true);
    true
}

fn handle_format_shortcuts(
    state: &EditorState,
    evt: &KeyboardEvent,
    apply_format_action: Callback<(MarkdownField, MarkdownAction)>,
) -> bool {
    if !evt.data.modifiers().contains(Modifiers::META) {
        return false;
    }

    let can_edit = (state.is_create_mode)() || (state.selected_card_id)().is_some();
    if !can_edit {
        return false;
    }

    let active_field = (state.last_focus_field)();
    if !matches!(active_field, MarkdownField::Front | MarkdownField::Back) {
        return false;
    }

    let field = active_field;
    let shift = evt.data.modifiers().contains(Modifiers::SHIFT);
    match evt.data.key() {
        Key::Character(value) if value.eq_ignore_ascii_case("b") => {
            evt.prevent_default();
            apply_format_action.call((field, MarkdownAction::Bold));
            true
        }
        Key::Character(value) if value.eq_ignore_ascii_case("i") => {
            evt.prevent_default();
            apply_format_action.call((field, MarkdownAction::Italic));
            true
        }
        Key::Character(value) if value.eq_ignore_ascii_case("k") => {
            evt.prevent_default();
            apply_format_action.call((field, MarkdownAction::Link));
            true
        }
        Key::Character(value) if value == "7" && shift => {
            evt.prevent_default();
            apply_format_action.call((field, MarkdownAction::NumberedList));
            true
        }
        Key::Character(value) if value == "8" && shift => {
            evt.prevent_default();
            apply_format_action.call((field, MarkdownAction::BulletList));
            true
        }
        _ => false,
    }
}

pub(super) fn build_on_key_action(
    state: &EditorState,
    save_action: Callback<SaveRequest>,
    request_new_card_action: Callback<()>,
    open_delete_modal_action: Callback<()>,
    apply_format_action: Callback<(MarkdownField, MarkdownAction)>,
    cancel_new_action: Callback<()>,
) -> Callback<KeyboardEvent> {
    let state = state.clone();
    use_callback(move |evt: KeyboardEvent| {
        if evt.data.key() == Key::Tab {
            return;
        }

        if handle_undo_redo(&state, &evt) {
            return;
        }

        if handle_primary_meta_actions(
            &state,
            &evt,
            save_action,
            request_new_card_action,
            open_delete_modal_action,
        ) {
            return;
        }

        if handle_rename_shortcut(&state, &evt) {
            return;
        }

        if handle_format_shortcuts(&state, &evt, apply_format_action) {
            return;
        }

        if evt.data.key() == Key::Escape && (state.is_create_mode)() {
            evt.prevent_default();
            cancel_new_action.call(());
        }
    })
}

pub(super) fn build_list_on_key_action(
    state: &EditorState,
    request_select_card_action: Callback<CardListItemVm>,
) -> Callback<KeyboardEvent> {
    let state = state.clone();
    let cards_state = view_state_from_resource(&state.cards_resource);
    use_callback(move |evt: KeyboardEvent| {
        if !evt.data.modifiers().is_empty() {
            return;
        }

        let key = evt.data.key();
        if !matches!(key, Key::ArrowDown | Key::ArrowUp | Key::Enter) {
            return;
        }

        let ViewState::Ready(items) = &cards_state else {
            return;
        };
        let filtered = filter_card_list_items(items, state.search_query.read().trim());
        if filtered.is_empty() {
            return;
        }

        let current_id = (state.selected_card_id)();
        let current_index = current_id
            .and_then(|id| filtered.iter().position(|item| item.id == id));

        let next_index = match key {
            Key::ArrowDown => match current_index {
                Some(idx) => (idx + 1).min(filtered.len() - 1),
                None => 0,
            },
            Key::ArrowUp => match current_index {
                Some(idx) => idx.saturating_sub(1),
                None => filtered.len().saturating_sub(1),
            },
            Key::Enter => current_index.unwrap_or(0),
            _ => return,
        };

        if let Some(item) = filtered.get(next_index).cloned() {
            evt.prevent_default();
            request_select_card_action.call(item);
        }
    })
}
