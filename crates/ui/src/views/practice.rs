use dioxus::prelude::*;
use dioxus_router::use_navigator;

use crate::context::AppContext;
use crate::routes::Route;
use crate::views::{ViewError, ViewState, view_state_from_resource};
use crate::vm::{PracticeDeckCardVm, map_practice_deck_card};

#[derive(Clone, Debug, PartialEq)]
struct PracticeData {
    deck_cards: Vec<PracticeDeckCardVm>,
}

#[component]
pub fn PracticeView() -> Element {
    let ctx = use_context::<AppContext>();
    let navigator = use_navigator();
    let deck_service = ctx.deck_service();
    let card_service = ctx.card_service();
    let mut search = use_signal(String::new);

    let resource = use_resource(move || {
        let deck_service = deck_service.clone();
        let card_service = card_service.clone();
        async move {
            let decks = deck_service
                .list_decks(64)
                .await
                .map_err(|_| ViewError::Unknown)?;

            let mut deck_cards = Vec::new();
            for deck in &decks {
                let stats = card_service
                    .deck_practice_stats(deck.id())
                    .await
                    .map_err(|_| ViewError::Unknown)?;
                let tag_stats = card_service
                    .list_tag_practice_stats(deck.id())
                    .await
                    .map_err(|_| ViewError::Unknown)?;
                deck_cards.push(map_practice_deck_card(deck, stats, &tag_stats));
            }

            Ok::<_, ViewError>(PracticeData {
                deck_cards,
            })
        }
    });

    let state = view_state_from_resource(&resource);
    let query = search().trim().to_lowercase();
    rsx! {
        div { class: "page practice-page",
            header { class: "view-header",
                h2 { class: "view-title", "Practice" }
                p { class: "view-subtitle", "Pick a deck or tap a tag for a focused session." }
            }
            div { class: "view-divider" }
            match state {
                ViewState::Idle => rsx! {
                    p { "Idle" }
                },
                ViewState::Loading => rsx! {
                    p { "Loading..." }
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
                ViewState::Ready(data) => {
                    let visible_decks = data
                        .deck_cards
                        .iter()
                        .filter(|deck| deck.matches_query(&query))
                        .cloned()
                        .collect::<Vec<_>>();
                    let empty_decks_message = if data.deck_cards.is_empty() {
                        "No decks yet. Create a deck to start practicing."
                    } else {
                        "No decks match that search."
                    };
                    let deck_cards = visible_decks.iter().map(|deck| {
                        let nav = navigator;
                        let deck_id = deck.id.value();
                        let name = deck.name.clone();
                        let due_label = deck.due_label.clone();
                        let new_label = deck.new_label.clone();
                        let total_label = deck.total_label.clone();
                        let tag_pills = deck.tag_pills.clone();
                        let extra_tag = deck.extra_tag_label.clone();
                        let avatar = deck.avatar.clone();
                        let tag_buttons = tag_pills.iter().map(|tag| {
                            let tag_label = tag.name.clone();
                            let due_label = tag.due_label.clone();
                            rsx! {
                                button {
                                    class: "practice-tag-pill practice-tag-pill--button",
                                    r#type: "button",
                                    onclick: move |_| {
                                        let _ = nav.push(Route::SessionTag {
                                            deck_id,
                                            tag: tag_label.clone(),
                                        });
                                    },
                                    span { class: "practice-tag-pill-name", "{tag.name}" }
                                    if let Some(label) = due_label.as_ref() {
                                        span { class: "practice-tag-pill-count", "{label}" }
                                    }
                                }
                            }
                        });
                        rsx! {
                            div { class: "practice-deck-card",
                                div { class: "practice-deck-header",
                                    div { class: "practice-deck-meta",
                                        span { class: "practice-deck-avatar", "{avatar}" }
                                        div { class: "practice-deck-text",
                                            h4 { class: "practice-deck-name", "{name}" }
                                            div { class: "practice-deck-stats",
                                                span { class: "practice-stat practice-stat--due", "{due_label}" }
                                                span { class: "practice-stat practice-stat--new", "{new_label}" }
                                                span { class: "practice-stat", "{total_label}" }
                                            }
                                        }
                                    }
                                    button {
                                        class: "btn btn-primary practice-deck-action",
                                        r#type: "button",
                                        onclick: move |_| {
                                            let _ = nav.push(Route::Session { deck_id });
                                        },
                                        "Practice"
                                    }
                                }
                                if !tag_pills.is_empty() || extra_tag.is_some() {
                                    div { class: "practice-deck-tags",
                                        {tag_buttons}
                                        if let Some(extra) = extra_tag.as_ref() {
                                            span { class: "practice-tag-pill practice-tag-pill--extra", "{extra}" }
                                        }
                                    }
                                }
                            }
                        }
                    });
                    rsx! {
                        div { class: "practice-search",
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
                                placeholder: "Search decks...",
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

                        div { class: "practice-decks",
                            h3 { class: "practice-section-title", "Decks" }
                            if visible_decks.is_empty() {
                                p { class: "practice-empty", "{empty_decks_message}" }
                            } else {
                                div { class: "practice-deck-grid",
                                    {deck_cards}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
