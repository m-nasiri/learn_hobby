use dioxus::prelude::*;

use super::super::state::EditorState;

pub(super) fn build_tag_actions(
    state: &EditorState,
) -> (
    Callback<String>,
    Callback<String>,
    Callback<Option<String>>,
) {
    let state_for_add = state.clone();
    let add_tag_action = use_callback(move |tag: String| {
        let mut card_tags = state_for_add.card_tags;
        let mut tag_input = state_for_add.tag_input;
        let tag = tag.trim();
        if tag.is_empty() {
            return;
        }
        let mut tags = card_tags();
        if tags.iter().any(|existing| existing == tag) {
            tag_input.set(String::new());
            return;
        }
        tags.push(tag.to_string());
        card_tags.set(tags);
        tag_input.set(String::new());
    });

    let state_for_remove = state.clone();
    let remove_tag_action = use_callback(move |tag: String| {
        let mut card_tags = state_for_remove.card_tags;
        let mut tags = card_tags();
        tags.retain(|existing| existing != &tag);
        card_tags.set(tags);
    });

    let state_for_filter = state.clone();
    let set_tag_filter_action = use_callback(move |tag: Option<String>| {
        let mut selected_tag_filters = state_for_filter.selected_tag_filters;
        if let Some(tag) = tag {
            selected_tag_filters.set(vec![tag]);
        } else {
            selected_tag_filters.set(Vec::new());
        }
    });

    (add_tag_action, remove_tag_action, set_tag_filter_action)
}
