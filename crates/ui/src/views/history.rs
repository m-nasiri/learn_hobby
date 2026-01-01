use dioxus::prelude::*;
use dioxus_router::use_navigator;
use learn_core::model::DeckId;

use crate::context::AppContext;
use crate::routes::Route;
use crate::views::{ViewError, ViewState, view_state_from_resource};
use crate::vm::{SessionSummaryCardVm, map_session_summary_cards};

#[derive(Clone, Debug, PartialEq)]
struct HistoryData {
    cards: Vec<SessionSummaryCardVm>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResetState {
    Idle,
    Resetting,
    Error(ViewError),
}

#[component]
pub fn HistoryView() -> Element {
    let ctx = use_context::<AppContext>();
    let summaries = ctx.session_summaries();
    let deck_id = ctx.current_deck_id();
    let deck_service = ctx.deck_service();
    let card_service = ctx.card_service();
    let mut search = use_signal(String::new);
    let mut open_menu = use_signal(|| None::<i64>);
    let mut reset_target = use_signal(|| None::<u64>);
    let mut reset_state = use_signal(|| ResetState::Idle);
    let mut show_mistakes_only = use_signal(|| false);

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
    let card_service_for_counts = card_service.clone();
    let counts_resource = use_resource(move || {
        let card_service = card_service_for_counts.clone();
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
                        .filter(|card| {
                            !show_mistakes_only() || (card.again + card.hard) > 0
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
                    let on_reset = {
                        let mut reset_target = reset_target;
                        let mut reset_state = reset_state;
                        let mut open_menu = open_menu;
                        use_callback(move |deck_id| {
                            open_menu.set(None);
                            reset_state.set(ResetState::Idle);
                            reset_target.set(Some(deck_id));
                        })
                    };
                    rsx! {
                        div { class: "history-controls",
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
                            button {
                                class: if show_mistakes_only() {
                                    "history-filter history-filter--active"
                                } else {
                                    "history-filter"
                                },
                                r#type: "button",
                                onclick: move |_| show_mistakes_only.set(!show_mistakes_only()),
                                "Mistakes only"
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
                                        open_menu,
                                        on_reset,
                                    }
                                }
                            }
                        }
                        if open_menu().is_some() {
                            div {
                                class: "history-menu-overlay",
                                onclick: move |_| open_menu.set(None),
                            }
                        }
                        if let Some(deck_id) = reset_target() {
                            div {
                                class: "history-modal-overlay",
                                onclick: move |_| {
                                    reset_target.set(None);
                                    reset_state.set(ResetState::Idle);
                                },
                                div {
                                    class: "history-modal",
                                    onclick: move |evt| evt.stop_propagation(),
                                    h3 { class: "history-modal-title", "Reset deck learning?" }
                                    p { class: "history-modal-body",
                                        "This resets scheduling for every card in this deck."
                                    }
                                    if let ResetState::Error(err) = reset_state() {
                                        p { class: "history-modal-error", "{err.message()}" }
                                    }
                                    div { class: "history-modal-actions",
                                        button {
                                            class: "btn editor-modal-cancel",
                                            r#type: "button",
                                            onclick: move |_| {
                                                reset_target.set(None);
                                                reset_state.set(ResetState::Idle);
                                            },
                                            "Cancel"
                                        }
                                        button {
                                            class: "btn editor-modal-confirm",
                                            r#type: "button",
                                            disabled: reset_state() == ResetState::Resetting,
                                            onclick: move |_| {
                                                let mut reset_state = reset_state;
                                                let mut reset_target = reset_target;
                                                let mut resource = resource;
                                                let card_service = card_service.clone();
                                                spawn(async move {
                                                    reset_state.set(ResetState::Resetting);
                                                    match card_service
                                                        .reset_deck_learning(DeckId::new(deck_id))
                                                        .await
                                                    {
                                                        Ok(_) => {
                                                            reset_state.set(ResetState::Idle);
                                                            reset_target.set(None);
                                                            resource.restart();
                                                        }
                                                        Err(_) => {
                                                            reset_state.set(ResetState::Error(ViewError::Unknown));
                                                        }
                                                    }
                                                });
                                            },
                                            "Reset"
                                        }
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
    open_menu: Signal<Option<i64>>,
    on_reset: EventHandler<u64>,
) -> Element {
    let navigator = use_navigator();
    let avatar = deck_label.chars().next().unwrap_or('D').to_string();
    let is_open = open_menu() == Some(card.id);
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
                div { class: "history-action",
                    button {
                        class: "btn btn-primary history-item__action",
                        r#type: "button",
                        onclick: move |_| {
                            if is_open {
                                open_menu.set(None);
                            } else {
                                open_menu.set(Some(card.id));
                            }
                        },
                        span { "Practice again" }
                        span { class: "history-action-caret" }
                    }
                    if is_open {
                        div { class: "history-action-menu",
                            button {
                                class: "history-action-item",
                                r#type: "button",
                                onclick: move |_| {
                                    open_menu.set(None);
                                    let _ = navigator.push(Route::Session { deck_id });
                                },
                                "Practice Due Cards"
                            }
                            button {
                                class: "history-action-item",
                                r#type: "button",
                                onclick: move |_| {
                                    open_menu.set(None);
                                    let _ = navigator.push(Route::SessionAll { deck_id });
                                },
                                "Practice All Cards"
                            }
                            button {
                                class: "history-action-item history-action-item--danger",
                                r#type: "button",
                                onclick: move |_| {
                                    open_menu.set(None);
                                    on_reset.call(deck_id);
                                },
                                "Reset Learning Progress"
                            }
                        }
                    }
                }
            }
        }
    }
}
