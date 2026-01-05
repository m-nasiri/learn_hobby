use dioxus::prelude::ReadableExt;
use learn_core::model::{CardId, DeckId};

use super::{CardListItemVm, DeckOptionVm, filter_card_list_items, strip_html_tags};
use crate::views::ViewState;
use crate::views::editor::state::{DeleteState, DuplicateCheckState, EditorState, SaveState};
use crate::views::editor::utils::build_tag_suggestions;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DailyLimitVm {
    pub limit: u32,
    pub created_today: u32,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug)]
pub struct EditorVm {
    pub deck_label: String,
    pub is_create_mode: bool,
    pub selected_card_id: Option<CardId>,
    pub can_edit: bool,
    pub can_submit: bool,
    pub can_cancel: bool,
    pub has_unsaved_changes: bool,
    pub prompt_invalid: bool,
    pub answer_invalid: bool,
    pub search_value: String,
    pub match_count: Option<usize>,
    pub deck_tags: Vec<String>,
    pub deck_tags_loading: bool,
    pub deck_tags_error: bool,
    pub selected_tag: Option<String>,
    pub tag_input_value: String,
    pub tag_suggestions: Vec<String>,
    pub card_tags: Vec<String>,
    pub prompt_toolbar_disabled: bool,
    pub answer_toolbar_disabled: bool,
    pub daily_limit_warning: Option<String>,
}

#[must_use]
pub fn build_editor_vm(
    state: &EditorState,
    decks_state: &ViewState<Vec<DeckOptionVm>>,
    cards_state: &ViewState<Vec<CardListItemVm>>,
    deck_tags_state: &ViewState<Vec<String>>,
    daily_limit_state: &ViewState<DailyLimitVm>,
) -> EditorVm {
    let selected_deck = *state.selected_deck.read();
    let deck_label = deck_label_from_state(decks_state, selected_deck);
    let is_create_mode = (state.is_create_mode)();
    let selected_card_id = (state.selected_card_id)();
    let has_unsaved_changes = (state.has_unsaved_changes)();
    let can_edit = is_create_mode || selected_card_id.is_some();
    let can_submit = can_edit
        && (state.save_state)() != SaveState::Saving
        && (state.delete_state)() != DeleteState::Deleting
        && (state.duplicate_check_state)() != DuplicateCheckState::Checking
        && has_unsaved_changes;
    let can_cancel = is_create_mode && (state.last_selected_card)().is_some();

    let prompt_plain = strip_html_tags(&state.prompt_text.read());
    let answer_plain = strip_html_tags(&state.answer_text.read());
    let prompt_invalid = (state.show_validation)() && prompt_plain.trim().is_empty();
    let answer_invalid = (state.show_validation)() && answer_plain.trim().is_empty();

    let search_value = state.search_query.read().to_string();
    let match_count = match cards_state {
        ViewState::Ready(items) if !search_value.trim().is_empty() => {
            Some(match_count_for_query(items, &search_value))
        }
        _ => None,
    };

    let deck_tags = match deck_tags_state {
        ViewState::Ready(tags) => tags.clone(),
        _ => Vec::new(),
    };
    let deck_tags_loading = matches!(deck_tags_state, ViewState::Loading);
    let deck_tags_error = matches!(deck_tags_state, ViewState::Error(_));

    let selected_filters = (state.selected_tag_filters)();
    let selected_tag = selected_filters.first().cloned();
    let tag_input_value = state.tag_input.read().to_string();
    let card_tags = state.card_tags.read().clone();
    let tag_suggestions =
        build_tag_suggestions(&deck_tags, &card_tags, &tag_input_value);

    let daily_limit_warning = match daily_limit_state {
        ViewState::Ready(limit)
            if is_create_mode && limit.limit > 0 && limit.created_today >= limit.limit =>
        {
            Some(format!(
                "Today's new card limit reached ({}/{}) — you can still save.",
                limit.created_today, limit.limit
            ))
        }
        _ => None,
    };

    EditorVm {
        deck_label,
        is_create_mode,
        selected_card_id,
        can_edit,
        can_submit,
        can_cancel,
        has_unsaved_changes,
        prompt_invalid,
        answer_invalid,
        search_value,
        match_count,
        deck_tags,
        deck_tags_loading,
        deck_tags_error,
        selected_tag,
        tag_input_value,
        tag_suggestions,
        card_tags,
        prompt_toolbar_disabled: !can_edit,
        answer_toolbar_disabled: !can_edit,
        daily_limit_warning,
    }
}

fn deck_label_from_state(
    decks_state: &ViewState<Vec<DeckOptionVm>>,
    selected_deck: DeckId,
) -> String {
    match decks_state {
        ViewState::Ready(options) => options
            .iter()
            .find(|opt| opt.id == selected_deck)
            .map_or_else(|| format!("{}", selected_deck.value()), |opt| opt.label.clone()),
        _ => "--".to_string(),
    }
}

fn match_count_for_query(items: &[CardListItemVm], query: &str) -> usize {
    filter_card_list_items(items, query.trim()).len()
}

#[cfg(test)]
mod tests {
    use learn_core::model::{CardId, DeckId};

    use super::{deck_label_from_state, match_count_for_query};
    use crate::vm::CardListItemVm;
    use crate::views::ViewState;

    fn list_item(id: u64, prompt: &str, answer: &str) -> CardListItemVm {
        CardListItemVm::new(
            CardId::new(id),
            prompt.to_string(),
            answer.to_string(),
            prompt.to_string(),
            answer.to_string(),
            prompt.to_string(),
            answer.to_string(),
        )
    }

    #[test]
    fn deck_label_from_state_falls_back() {
        let state: ViewState<Vec<crate::vm::DeckOptionVm>> = ViewState::Loading;
        let deck_id = DeckId::new(7);
        assert_eq!(deck_label_from_state(&state, deck_id), "--");
    }

    #[test]
    fn deck_label_from_state_uses_matching_deck() {
        let deck_id = DeckId::new(1);
        let options = vec![crate::vm::DeckOptionVm::new(deck_id, "Default".to_string())];
        let state = ViewState::Ready(options);
        assert_eq!(deck_label_from_state(&state, deck_id), "Default");
    }

    #[test]
    fn match_count_for_query_filters_items() {
        let items = vec![
            list_item(1, "Rust", "Language"),
            list_item(2, "Dioxus", "UI"),
        ];
        assert_eq!(match_count_for_query(&items, "rust"), 1);
        assert_eq!(match_count_for_query(&items, "ui"), 1);
        assert_eq!(match_count_for_query(&items, ""), 2);
    }
}
