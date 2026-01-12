use dioxus::document::eval;
use dioxus::prelude::*;

use crate::vm::{DeckOptionVm, MarkdownAction, MarkdownField, filter_card_list_items};
use crate::views::{ViewState, view_state_from_resource};

use super::super::state::{DeleteState, EditorState, SaveRequest};
use super::super::scripts::exec_command_script;
use super::intent::EditorIntent;

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
    dispatch: &Callback<EditorIntent>,
) -> bool {
    if !evt.data.modifiers().contains(Modifiers::META) {
        return false;
    }

    if evt.data.key() == Key::Enter {
        evt.prevent_default();
        dispatch.call(EditorIntent::Save(SaveRequest::new(false)));
        return true;
    }

    if matches!(evt.data.key(), Key::Character(value) if value.eq_ignore_ascii_case("n")) {
        evt.prevent_default();
        dispatch.call(EditorIntent::RequestNewCard);
        return true;
    }

    if evt.data.key() == Key::Backspace
        && (state.selected_card_id)().is_some()
        && !(state.is_create_mode)()
        && (state.delete_state)() != DeleteState::Deleting
    {
        evt.prevent_default();
        dispatch.call(EditorIntent::OpenDeleteModal);
        return true;
    }

    false
}

fn handle_rename_shortcut(
    state: &EditorState,
    evt: &KeyboardEvent,
    dispatch: &Callback<EditorIntent>,
) -> bool {
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
    dispatch.call(EditorIntent::BeginRename(label));
    true
}

fn handle_format_shortcuts(
    state: &EditorState,
    evt: &KeyboardEvent,
    dispatch: &Callback<EditorIntent>,
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
            dispatch.call(EditorIntent::ApplyFormat(field, MarkdownAction::Bold));
            true
        }
        Key::Character(value) if value.eq_ignore_ascii_case("i") => {
            evt.prevent_default();
            dispatch.call(EditorIntent::ApplyFormat(field, MarkdownAction::Italic));
            true
        }
        Key::Character(value) if value.eq_ignore_ascii_case("k") => {
            evt.prevent_default();
            dispatch.call(EditorIntent::OpenLinkEditor(field));
            true
        }
        Key::Character(value) if value == "7" && shift => {
            evt.prevent_default();
            dispatch.call(EditorIntent::ApplyFormat(field, MarkdownAction::NumberedList));
            true
        }
        Key::Character(value) if value == "8" && shift => {
            evt.prevent_default();
            dispatch.call(EditorIntent::ApplyFormat(field, MarkdownAction::BulletList));
            true
        }
        _ => false,
    }
}

pub(super) fn build_on_key_action(
    state: &EditorState,
    dispatch: Callback<EditorIntent>,
) -> Callback<KeyboardEvent> {
    let state = state.clone();
    use_callback(move |evt: KeyboardEvent| {
        if evt.data.key() == Key::Tab {
            return;
        }

        if handle_undo_redo(&state, &evt) {
            return;
        }

        if handle_primary_meta_actions(&state, &evt, &dispatch) {
            return;
        }

        if handle_rename_shortcut(&state, &evt, &dispatch) {
            return;
        }

        if handle_format_shortcuts(&state, &evt, &dispatch) {
            return;
        }

        if evt.data.key() == Key::Escape && (state.is_create_mode)() {
            evt.prevent_default();
            dispatch.call(EditorIntent::CancelNew);
        }
    })
}

pub(super) fn build_list_on_key_action(
    state: &EditorState,
    dispatch: Callback<EditorIntent>,
) -> Callback<KeyboardEvent> {
    let state = state.clone();
    let cards_state = view_state_from_resource(&state.cards_resource);
    use_callback(move |evt: KeyboardEvent| {
        if !evt.data.modifiers().is_empty() {
            return;
        }

        let key = evt.data.key();
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
        let Some(next_index) = next_list_index(current_index, filtered.len(), &key) else {
            return;
        };

        if let Some(item) = filtered.get(next_index).cloned() {
            evt.prevent_default();
            dispatch.call(EditorIntent::RequestSelectCard(item));
        }
    })
}

fn next_list_index(current_index: Option<usize>, len: usize, key: &Key) -> Option<usize> {
    if len == 0 {
        return None;
    }

    match key {
        Key::ArrowDown => Some(match current_index {
            Some(idx) => (idx + 1).min(len - 1),
            None => 0,
        }),
        Key::ArrowUp => Some(match current_index {
            Some(idx) => idx.saturating_sub(1),
            None => len.saturating_sub(1),
        }),
        Key::Enter => Some(current_index.unwrap_or(0)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::next_list_index;
    use dioxus::prelude::Key;

    #[test]
    fn next_list_index_handles_empty_list() {
        assert_eq!(next_list_index(None, 0, &Key::ArrowDown), None);
    }

    #[test]
    fn next_list_index_moves_down_from_none() {
        assert_eq!(next_list_index(None, 3, &Key::ArrowDown), Some(0));
    }

    #[test]
    fn next_list_index_moves_up_from_none() {
        assert_eq!(next_list_index(None, 3, &Key::ArrowUp), Some(2));
    }

    #[test]
    fn next_list_index_moves_between_items() {
        assert_eq!(next_list_index(Some(1), 3, &Key::ArrowDown), Some(2));
        assert_eq!(next_list_index(Some(1), 3, &Key::ArrowUp), Some(0));
    }

    #[test]
    fn next_list_index_caps_at_edges() {
        assert_eq!(next_list_index(Some(2), 3, &Key::ArrowDown), Some(2));
        assert_eq!(next_list_index(Some(0), 3, &Key::ArrowUp), Some(0));
    }

    #[test]
    fn next_list_index_uses_enter_selection() {
        assert_eq!(next_list_index(Some(2), 3, &Key::Enter), Some(2));
        assert_eq!(next_list_index(None, 3, &Key::Enter), Some(0));
    }
}
