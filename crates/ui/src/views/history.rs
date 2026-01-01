use dioxus::prelude::*;
use dioxus_router::use_navigator;

use crate::context::AppContext;
use crate::routes::Route;
use crate::views::{ViewError, ViewState, view_state_from_resource};
use crate::vm::{SessionSummaryCardVm, map_session_summary_cards};

#[derive(Clone, Debug, PartialEq)]
struct HistoryData {
    cards: Vec<SessionSummaryCardVm>,
}

#[component]
pub fn HistoryView() -> Element {
    let ctx = use_context::<AppContext>();
    let summaries = ctx.session_summaries();
    let deck_id = ctx.current_deck_id();
    let deck_service = ctx.deck_service();
    let card_service = ctx.card_service();
    let mut search = use_signal(String::new);

    let resource = use_resource(move || {
        let summaries = summaries.clone();
        async move {
            let items = summaries
                .list_recent_summaries(deck_id, 7, 10)
                .await
                .map_err(|_| ViewError::Unknown)?;
            let cards = map_session_summary_cards(&items);
            Ok(HistoryData { cards })
        }
    });
    let deck_label_resource = use_resource(move || {
        let deck_service = deck_service.clone();
        async move {
            let deck = deck_service
                .get_deck(deck_id)
                .await
                .map_err(|_| ViewError::Unknown)?;
            Ok::<_, ViewError>(deck.map(|deck| deck.name().to_string()))
        }
    });
    let counts_resource = use_resource(move || {
        let card_service = card_service.clone();
        async move {
            let stats = card_service
                .deck_practice_stats(deck_id)
                .await
                .map_err(|_| ViewError::Unknown)?;
            Ok::<_, ViewError>(stats.due)
        }
    });

    let state = view_state_from_resource(&resource);
    let deck_label = deck_label_resource
        .value()
        .read()
        .as_ref()
        .and_then(|value| value.as_ref().ok())
        .and_then(Clone::clone)
        .unwrap_or_else(|| "Deck".to_string());
    let due_label = counts_resource
        .value()
        .read()
        .as_ref()
        .and_then(|value| value.as_ref().ok())
        .map_or_else(|| "Due: --".to_string(), |due| format!("Due: {due}"));
    let query = search().trim().to_lowercase();

    rsx! {
        div { class: "page history-page",
            header { class: "view-header",
                h2 { class: "view-title", "History" }
                p { class: "view-subtitle", "Recent sessions from the last week." }
            }
            div { class: "view-divider" }

            match state {
                ViewState::Idle => rsx! {
                    p { "Idle" }
                },
                ViewState::Loading => rsx! {
                    p { "Loading..." }
                },
                ViewState::Ready(data) => {
                    let matches_deck = !query.is_empty()
                        && deck_label.to_lowercase().contains(&query);
                    let visible_cards = data
                        .cards
                        .iter()
                        .filter(|card| {
                            query.is_empty()
                                || matches_deck
                                || card.completed_at_str.to_lowercase().contains(&query)
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    let empty_message = if data.cards.is_empty() {
                        "No recent sessions yet."
                    } else {
                        "No sessions match that search."
                    };
                    let deck_label = deck_label.clone();
                    let due_label = due_label.clone();
                    rsx! {
                        div { class: "practice-search history-search",
                            span { class: "practice-search-icon", aria_hidden: "true",
                                svg {
                                    view_box: "0 0 24 24",
                                    stroke: "currentColor",
                                    stroke_width: "1.8",
                                    fill: "none",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    circle { cx: "11", cy: "11", r: "7" }
                                    path { d: "M20 20l-3.5-3.5" }
                                }
                            }
                            input {
                                class: "practice-search-input",
                                r#type: "text",
                                placeholder: "Search history...",
                                value: "{search()}",
                                oninput: move |evt| search.set(evt.value()),
                            }
                            if !search().is_empty() {
                                button {
                                    class: "practice-search-clear",
                                    r#type: "button",
                                    onclick: move |_| search.set(String::new()),
                                    span { class: "practice-search-clear-icon", "×" }
                                }
                            }
                        }
                        if visible_cards.is_empty() {
                            p { class: "history-empty", "{empty_message}" }
                        } else {
                            ul { class: "history-list",
                                for card in visible_cards {
                SummaryCard {
                    key: "{card.id}",
                    card,
                    deck_label: deck_label.clone(),
                    due_label: due_label.clone(),
                    deck_id: deck_id.value(),
                }
                            }
                        }
                    }
                    }
                },
                ViewState::Error(err) => rsx! {
                    p { "{err.message()}" }
                    button {
                        class: "btn btn-secondary",
                        r#type: "button",
                        onclick: move |_| {
                            let mut resource = resource;
                            resource.restart();
                        },
                        "Retry"
                    }
                },
            }
        }
    }
}

#[component]
fn SummaryCard(
    card: SessionSummaryCardVm,
    deck_label: String,
    due_label: String,
    deck_id: u64,
) -> Element {
    let navigator = use_navigator();
    let avatar = deck_label.chars().next().unwrap_or('D').to_string();
    rsx! {
        li { class: "history-item",
            div { class: "history-item__main",
                div { class: "history-item__avatar", "{avatar}" }
                div { class: "history-item__content",
                    h3 { class: "history-item__title", "{deck_label}" }
                    div { class: "history-item__meta",
                        span { class: "history-item__count", "{card.cards_label}" }
                        span { class: "history-item__dot", "•" }
                        if card.total == 0 {
                            span { class: "history-item__breakdown", "No reviews" }
                        } else {
                            span { class: "history-metric history-metric--again", "{card.again_pct}% Again" }
                            span { class: "history-metric history-metric--hard", "{card.hard_pct}% Hard" }
                            span { class: "history-metric history-metric--good", "{card.good_pct}% Good" }
                            span { class: "history-metric history-metric--easy", "{card.easy_pct}% Easy" }
                        }
                    }
                }
            }
            div { class: "history-item__actions",
                span { class: "history-item__date", "{card.completed_at_str}" }
                span { class: "history-item__due", "{due_label}" }
                button {
                    class: "btn btn-primary history-item__action",
                    r#type: "button",
                    onclick: move |_| {
                        let _ = navigator.push(Route::Session { deck_id });
                    },
                    "Practice again"
                }
            }
        }
    }
}
