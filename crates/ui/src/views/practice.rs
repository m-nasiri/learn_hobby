use dioxus::prelude::*;
use dioxus_router::use_navigator;
use learn_core::model::DeckId;

use crate::context::AppContext;
use crate::routes::Route;
use crate::views::{ViewError, ViewState, view_state_from_resource};
use crate::vm::{PracticeDeckCardVm, map_practice_deck_card};

#[derive(Clone, Debug, PartialEq)]
struct PracticeData {
    deck_cards: Vec<PracticeDeckCardVm>,
    deck_scope_name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResetState {
    Idle,
    Resetting,
    Error(ViewError),
}

#[component]
pub fn PracticeView(deck_id: Option<u64>) -> Element {
    let ctx = use_context::<AppContext>();
    let navigator = use_navigator();
    let deck_service = ctx.deck_service();
    let card_service = ctx.card_service();
    let deck_scope_id = deck_id.map(DeckId::new);
    let mut search = use_signal(String::new);
    let mut open_menu = use_signal(|| None::<u64>);
    let mut reset_target = use_signal(|| None::<u64>);
    let mut reset_state = use_signal(|| ResetState::Idle);

    let card_service_for_resource = card_service.clone();
    let resource = use_resource(move || {
        let deck_service = deck_service.clone();
        let card_service = card_service_for_resource.clone();
        let deck_scope_id = deck_scope_id;
        async move {
            let decks = if let Some(deck_id) = deck_scope_id {
                match deck_service
                    .get_deck(deck_id)
                    .await
                    .map_err(|_| ViewError::Unknown)?
                {
                    Some(deck) => vec![deck],
                    None => Vec::new(),
                }
            } else {
                deck_service
                    .list_decks(64)
                    .await
                    .map_err(|_| ViewError::Unknown)?
            };

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

            let deck_scope_name = deck_scope_id
                .and_then(|id| decks.iter().find(|deck| deck.id() == id))
                .map(|deck| deck.name().to_string());

            Ok::<_, ViewError>(PracticeData {
                deck_cards,
                deck_scope_name,
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
                        let mut open_menu = open_menu;
                        let mut reset_target = reset_target;
                        let mut reset_state = reset_state;
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
                                    div { class: "practice-action",
                                        button {
                                            class: "btn btn-primary practice-deck-action",
                                            r#type: "button",
                                            onclick: move |_| {
                                                if open_menu() == Some(deck_id) {
                                                    open_menu.set(None);
                                                } else {
                                                    open_menu.set(Some(deck_id));
                                                }
                                            },
                                            span { "Practice" }
                                            span { class: "practice-deck-action-caret" }
                                        }
                                        if open_menu() == Some(deck_id) {
                                            div { class: "practice-action-menu",
                                                button {
                                                    class: "practice-action-item",
                                                    r#type: "button",
                                                    onclick: move |_| {
                                                        open_menu.set(None);
                                                        let _ = nav.push(Route::Session { deck_id });
                                                    },
                                                    "Practice Due Cards"
                                                }
                                            button {
                                                class: "practice-action-item",
                                                r#type: "button",
                                                onclick: move |_| {
                                                    open_menu.set(None);
                                                    let _ = nav.push(Route::SessionAll { deck_id });
                                                },
                                                "Practice All Cards"
                                            }
                                            button {
                                                class: "practice-action-item",
                                                r#type: "button",
                                                onclick: move |_| {
                                                    open_menu.set(None);
                                                    let _ = nav.push(Route::SessionMistakes { deck_id });
                                                },
                                                "Re-practice Mistakes"
                                            }
                                            button {
                                                class: "practice-action-item",
                                                r#type: "button",
                                                onclick: move |_| {
                                                    open_menu.set(None);
                                                    let _ = nav.push(Route::SettingsDeck { deck_id });
                                                },
                                                "Deck settings..."
                                            }
                                            button {
                                                class: "practice-action-item practice-action-item--danger",
                                                r#type: "button",
                                                onclick: move |_| {
                                                    open_menu.set(None);
                                                        reset_state.set(ResetState::Idle);
                                                        reset_target.set(Some(deck_id));
                                                    },
                                                    "Reset Learning Progress"
                                                }
                                            }
                                        }
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
                    let subtitle = data.deck_scope_name.as_ref().map(|name| {
                        format!("Focused on {name}. Pick a practice option.")
                    });
                    rsx! {
                        if let Some(label) = subtitle {
                            p { class: "view-hint", "{label}" }
                        }
                        if data.deck_scope_name.is_none() {
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
                        if open_menu().is_some() {
                            div {
                                class: "practice-menu-overlay",
                                onclick: move |_| open_menu.set(None),
                            }
                        }
                        if let Some(deck_id) = reset_target() {
                            div {
                                class: "practice-modal-overlay",
                                onclick: move |_| {
                                    reset_target.set(None);
                                    reset_state.set(ResetState::Idle);
                                },
                                div {
                                    class: "practice-modal",
                                    onclick: move |evt| evt.stop_propagation(),
                                    h3 { class: "practice-modal-title", "Reset deck learning?" }
                                    p { class: "practice-modal-body",
                                        "This resets scheduling for every card in this deck."
                                    }
                                    if let ResetState::Error(err) = reset_state() {
                                        p { class: "practice-modal-error", "{err.message()}" }
                                    }
                                    div { class: "practice-modal-actions",
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
                                                    match card_service.reset_deck_learning(DeckId::new(deck_id)).await {
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
                }
            }
        }
    }
}
