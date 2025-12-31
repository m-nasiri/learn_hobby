use dioxus::prelude::*;
use learn_core::model::CardId;
use services::CardListSort;

use crate::vm::{CardListItemVm, filter_card_list_items};
use crate::views::ViewState;

use super::super::utils::{sort_from_value, sort_value};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HighlightSpan {
    text: String,
    is_match: bool,
}

fn highlight_spans(text: &str, query: &str) -> Vec<HighlightSpan> {
    let needle = query.trim();
    if needle.is_empty() || text.is_empty() {
        return vec![HighlightSpan {
            text: text.to_string(),
            is_match: false,
        }];
    }

    let mut lowered = Vec::new();
    let mut map_start = Vec::new();
    let mut map_end = Vec::new();

    for (idx, ch) in text.char_indices() {
        let end = idx + ch.len_utf8();
        for lower in ch.to_lowercase() {
            lowered.push(lower);
            map_start.push(idx);
            map_end.push(end);
        }
    }

    let needle_chars: Vec<char> = needle.to_lowercase().chars().collect();
    if needle_chars.is_empty() {
        return vec![HighlightSpan {
            text: text.to_string(),
            is_match: false,
        }];
    }

    let mut spans = Vec::new();
    let mut cursor = 0usize;
    let mut idx = 0usize;
    while idx + needle_chars.len() <= lowered.len() {
        if lowered[idx..idx + needle_chars.len()] == needle_chars[..] {
            let start = map_start[idx];
            let end = map_end[idx + needle_chars.len() - 1];
            if start > cursor {
                spans.push(HighlightSpan {
                    text: text[cursor..start].to_string(),
                    is_match: false,
                });
            }
            if start < end {
                spans.push(HighlightSpan {
                    text: text[start..end].to_string(),
                    is_match: true,
                });
            }
            cursor = end;
            idx += needle_chars.len();
        } else {
            idx += 1;
        }
    }

    if cursor < text.len() {
        spans.push(HighlightSpan {
            text: text[cursor..].to_string(),
            is_match: false,
        });
    }

    if spans.is_empty() {
        spans.push(HighlightSpan {
            text: text.to_string(),
            is_match: false,
        });
    }

    spans
}

fn render_highlighted(text: &str, query: &str) -> Vec<Element> {
    highlight_spans(text, query)
        .into_iter()
        .enumerate()
        .map(|(idx, span)| {
            rsx!(
                span {
                    key: "{idx}",
                    class: if span.is_match {
                        "editor-list-highlight"
                    } else {
                        "editor-list-text"
                    },
                    "{span.text}"
                }
            )
        })
        .collect()
}

#[component]
pub fn EditorListPane(
    cards_state: ViewState<Vec<CardListItemVm>>,
    selected_card_id: Option<CardId>,
    search_value: String,
    match_count: Option<usize>,
    sort_mode: CardListSort,
    selected_tag: Option<String>,
    deck_tags: Vec<String>,
    deck_tags_loading: bool,
    deck_tags_error: bool,
    on_search_change: Callback<String>,
    on_clear_search: Callback<()>,
    on_sort_change: Callback<CardListSort>,
    on_tag_filter_change: Callback<Option<String>>,
    on_select_card: Callback<CardListItemVm>,
    on_new_card: Callback<()>,
    on_list_key: Callback<KeyboardEvent>,
) -> Element {
    let has_search = !search_value.trim().is_empty();
    let query = search_value.trim();
    let selected_tag_value = selected_tag.clone().unwrap_or_default();

    rsx! {
        aside {
            class: "editor-list-pane",
            tabindex: "0",
            aria_label: "Card list",
            onkeydown: on_list_key,
            div { class: "editor-list-toolbar",
                div { class: "editor-list-header",
                    div { class: "editor-list-title-row",
                        h3 { class: "editor-list-title", "Cards" }
                        if let Some(count) = match_count {
                            span { class: "editor-list-count",
                                if count == 1 { "1 result" } else { "{count} results" }
                            }
                        }
                    }
                    div { class: "editor-list-search",
                        span { class: "editor-list-search-icon",
                            svg {
                                view_box: "0 0 16 16",
                                path {
                                    d: "M7 2.5a4.5 4.5 0 1 1-3.2 7.7l-2.1 2.1",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                }
                            }
                        }
                        input {
                            class: "editor-list-search-input",
                            r#type: "text",
                            placeholder: "Search",
                            value: "{search_value}",
                            oninput: move |evt| on_search_change.call(evt.value()),
                            onkeydown: move |evt| {
                                if matches!(evt.data.key(), Key::Escape) {
                                    evt.prevent_default();
                                    on_search_change.call(String::new());
                                }
                            },
                        }
                        if has_search {
                            button {
                                class: "editor-list-search-clear",
                                aria_label: "Clear search",
                                r#type: "button",
                                title: "Clear search",
                                onclick: move |_| on_clear_search.call(()),
                                svg {
                                    class: "editor-list-search-clear-icon",
                                    view_box: "0 0 12 12",
                                    path {
                                        d: "M3 3l6 6M9 3l-6 6",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                    }
                                }
                            }
                        }
                    }
                    div { class: "editor-list-controls",
                        span { class: "editor-list-control-label", "Sort by" }
                        select {
                            class: "editor-list-select",
                            title: "Sort cards",
                            value: "{sort_value(sort_mode)}",
                            onchange: move |evt| {
                                on_sort_change.call(sort_from_value(&evt.value()));
                            },
                            option { value: "recent", "Recent" }
                            option { value: "created", "Created" }
                            option { value: "alpha", "A–Z" }
                        }
                    }
                    div { class: "editor-list-controls",
                        span { class: "editor-list-control-label", "Filter tags" }
                        select {
                            class: "editor-list-select",
                            title: "Filter by tag",
                            disabled: deck_tags_loading || deck_tags_error || deck_tags.is_empty(),
                            value: "{selected_tag_value}",
                            onchange: move |evt| {
                                let value = evt.value();
                                if value.is_empty() {
                                    on_tag_filter_change.call(None);
                                } else {
                                    on_tag_filter_change.call(Some(value));
                                }
                            },
                            if deck_tags_loading {
                                option { value: "", "Loading tags..." }
                            } else if deck_tags_error {
                                option { value: "", "Tags unavailable" }
                            } else if deck_tags.is_empty() {
                                option { value: "", "No tags yet" }
                            } else {
                                option { value: "", "All tags" }
                                for tag in deck_tags.clone() {
                                    option { value: "{tag}", "{tag}" }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "editor-list-surface",
                div { class: "editor-list-body",
                match cards_state {
                    ViewState::Idle => rsx! {
                        p { class: "editor-list-empty", "Idle" }
                    },
                    ViewState::Loading => rsx! {
                        p { class: "editor-list-empty", "Loading cards..." }
                    },
                    ViewState::Error(err) => rsx! {
                        p { class: "editor-list-empty", "{err.message()}" }
                    },
                    ViewState::Ready(items) => {
                        if items.is_empty() {
                            rsx! {
                                p { class: "editor-list-empty", "No cards yet." }
                                button {
                                    class: "btn editor-list-cta",
                                    r#type: "button",
                                    onclick: move |_| on_new_card.call(()),
                                    "Create your first card"
                                }
                            }
                        } else {
                            let filtered_items = filter_card_list_items(&items, query);
                            if filtered_items.is_empty() {
                                rsx! {
                                    p { class: "editor-list-empty", "No matches." }
                                }
                            } else {
                                rsx! {
                                    ul { class: "editor-list-items",
                                        for item in filtered_items {
                                            li {
                                                class: if Some(item.id) == selected_card_id {
                                                    "editor-list-item editor-list-item--active"
                                                } else {
                                                    "editor-list-item"
                                                },
                                                key: "{item.id.value()}",
                                                onclick: move |_| on_select_card.call(item.clone()),
                                                div { class: "editor-list-front",
                                                    for node in render_highlighted(
                                                        &item.prompt_preview,
                                                        query,
                                                    ) {
                                                        {node}
                                                    }
                                                }
                                                div { class: "editor-list-back",
                                                    for node in render_highlighted(
                                                        &item.answer_preview,
                                                        query,
                                                    ) {
                                                        {node}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::highlight_spans;

    #[test]
    fn highlight_spans_marks_match_segments() {
        let spans = highlight_spans("Rust", "st");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "Ru");
        assert!(!spans[0].is_match);
        assert_eq!(spans[1].text, "st");
        assert!(spans[1].is_match);
    }

    #[test]
    fn highlight_spans_handles_no_match() {
        let spans = highlight_spans("Rust", "zz");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Rust");
        assert!(!spans[0].is_match);
    }
}
