use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use dioxus_router::Link;

use crate::context::AppContext;
use crate::routes::Route;
use crate::views::{ViewError, ViewState, view_state_from_resource};
use crate::vm::format_relative_datetime;
use learn_core::model::DeckId;

#[derive(Clone, Debug, PartialEq, Eq)]
struct HomePracticeNow {
    deck_id: DeckId,
    deck_name: String,
    due: u32,
    new: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HomeRecentSession {
    deck_id: DeckId,
    deck_name: String,
    completed_at: DateTime<Utc>,
    meta_label: String,
    again_pct: u32,
    hard_pct: u32,
    good_pct: u32,
    easy_pct: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HomeUpcomingDeck {
    deck_id: DeckId,
    deck_name: String,
    due: u32,
    new: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HomeData {
    practice_now: HomePracticeNow,
    recent_sessions: Vec<HomeRecentSession>,
    upcoming_decks: Vec<HomeUpcomingDeck>,
}

#[component]
pub fn HomeView() -> Element {
    let ctx = use_context::<AppContext>();

    let deck_id = ctx.current_deck_id();
    let summaries = ctx.session_summaries();
    let deck_service = ctx.deck_service();
    let card_service = ctx.card_service();

    let resource = use_resource(move || {
        let summaries = summaries.clone();
        let deck_service = deck_service.clone();
        let card_service = card_service.clone();

        async move {
            let now = summaries.now();
            let current_deck = deck_service
                .get_deck(deck_id)
                .await
                .map_err(|_| ViewError::Unknown)?
                .ok_or(ViewError::Unknown)?;
            let current_stats = card_service
                .deck_practice_stats(deck_id)
                .await
                .map_err(|_| ViewError::Unknown)?;

            let decks = deck_service
                .list_decks(8)
                .await
                .map_err(|_| ViewError::Unknown)?;

            let mut recent_sessions = Vec::new();
            for deck in &decks {
                let items = summaries
                    .list_recent_summaries(deck.id(), 7, 1)
                    .await
                    .map_err(|_| ViewError::Unknown)?;
                if let Some(item) = items.first() {
                    let total = item.total;
                    let pct = |count: u32| {
                        if total == 0 {
                            0
                        } else {
                            count.saturating_mul(100) / total
                        }
                    };
                    let date_label = format_relative_datetime(&item.completed_at, &now);
                    recent_sessions.push(HomeRecentSession {
                        deck_id: deck.id(),
                        deck_name: deck.name().to_string(),
                        completed_at: item.completed_at,
                        meta_label: format!("{date_label} \u{00b7} {total} Cards"),
                        again_pct: pct(item.again),
                        hard_pct: pct(item.hard),
                        good_pct: pct(item.good),
                        easy_pct: pct(item.easy),
                    });
                }
            }
            recent_sessions.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));
            recent_sessions.truncate(3);

            let mut upcoming_decks = Vec::new();
            for deck in &decks {
                let stats = card_service
                    .deck_practice_stats(deck.id())
                    .await
                    .map_err(|_| ViewError::Unknown)?;
                if stats.due > 0 || stats.new > 0 {
                    upcoming_decks.push(HomeUpcomingDeck {
                        deck_id: deck.id(),
                        deck_name: deck.name().to_string(),
                        due: stats.due,
                        new: stats.new,
                    });
                }
            }
            upcoming_decks.sort_by(|a, b| b.due.cmp(&a.due).then_with(|| b.new.cmp(&a.new)));
            upcoming_decks.truncate(3);

            Ok::<_, ViewError>(HomeData {
                practice_now: HomePracticeNow {
                    deck_id,
                    deck_name: current_deck.name().to_string(),
                    due: current_stats.due,
                    new: current_stats.new,
                },
                recent_sessions,
                upcoming_decks,
            })
        }
    });

    let state = view_state_from_resource(&resource);

    rsx! {
        div { class: "page home-page",
            match state {
                ViewState::Idle => rsx! {
                    p { "Loading..." }
                },
                ViewState::Loading => rsx! {
                    p { "Loading..." }
                },
                ViewState::Ready(data) => rsx! {
                    section { class: "home-hero",
                        h1 { class: "home-hero__title", "Welcome back!" }
                        p { class: "home-hero__subtitle", "Ready to start practicing?" }
                    }

                    section { class: "home-top-cards",
                        div { class: "home-card",
                            div { class: "home-card__icon home-card__icon--practice",
                                svg {
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "1.6",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    rect { x: "3.5", y: "5", width: "17", height: "14", rx: "3" }
                                    path { d: "M8.5 11h.01" }
                                    path { d: "M15.5 11h.01" }
                                    path { d: "M9 14.5c1 1 2.5 1 3.5 0" }
                                }
                            }
                            h3 { class: "home-card__title", "Practice Now" }
                            p { class: "home-card__meta",
                                "{data.practice_now.due} Due"
                                span { class: "home-card__dot", "\u{00b7}" }
                                "{data.practice_now.new} New"
                                span { class: "home-card__dot", "\u{00b7}" }
                                "{data.practice_now.deck_name}"
                            }
                            Link {
                                class: "btn btn-primary home-card__action",
                                to: Route::Session { deck_id: data.practice_now.deck_id.value() },
                                "Start"
                            }
                        }
                        div { class: "home-card",
                            div { class: "home-card__icon home-card__icon--manage",
                                svg {
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "1.6",
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    circle { cx: "12", cy: "12", r: "3.5" }
                                    path { d: "M19.4 15a1 1 0 0 0 .2 1.1l.1.1a2 2 0 0 1-2.8 2.8l-.1-.1a1 1 0 0 0-1.1-.2 1 1 0 0 0-.6.9V20a2 2 0 0 1-4 0v-.1a1 1 0 0 0-.6-.9 1 1 0 0 0-1.1.2l-.1.1a2 2 0 0 1-2.8-2.8l.1-.1a1 1 0 0 0 .2-1.1 1 1 0 0 0-.9-.6H4a2 2 0 0 1 0-4h.1a1 1 0 0 0 .9-.6 1 1 0 0 0-.2-1.1l-.1-.1a2 2 0 0 1 2.8-2.8l.1.1a1 1 0 0 0 1.1.2 1 1 0 0 0 .6-.9V4a2 2 0 0 1 4 0v.1a1 1 0 0 0 .6.9 1 1 0 0 0 1.1-.2l.1-.1a2 2 0 0 1 2.8 2.8l-.1.1a1 1 0 0 0-.2 1.1 1 1 0 0 0 .9.6H20a2 2 0 0 1 0 4h-.1a1 1 0 0 0-.5.6z" }
                                }
                            }
                            h3 { class: "home-card__title", "Manage Collection" }
                            p { class: "home-card__meta", "Edit decks & cards in the editor" }
                            Link { class: "btn btn-secondary home-card__action", to: Route::Editor {}, "Go to Editor" }
                        }
                    }

                    section { class: "home-columns",
                        div { class: "home-panel",
                            div { class: "home-panel__header",
                                h4 { class: "home-panel__title", "Recent Sessions" }
                            }
                            div { class: "home-panel__list",
                                if data.recent_sessions.is_empty() {
                                    p { class: "home-panel__empty", "No recent sessions yet." }
                                }
                                for item in data.recent_sessions {
                                    div { class: "home-session-row",
                                        div { class: "home-session-row__left",
                                            div { class: "home-session-avatar",
                                                "{deck_initial(&item.deck_name)}"
                                            }
                                            div { class: "home-session-text",
                                                h5 { class: "home-session-title", "{item.deck_name}" }
                                                p { class: "home-session-meta", "{item.meta_label}" }
                                                p { class: "home-session-breakdown",
                                                    span { class: "home-session-metric history-metric--again", "{item.again_pct}% Again" }
                                                    span { class: "home-session-dot", "\u{00b7}" }
                                                    span { class: "home-session-metric history-metric--hard", "{item.hard_pct}% Hard" }
                                                    span { class: "home-session-dot", "\u{00b7}" }
                                                    span { class: "home-session-metric history-metric--good", "{item.good_pct}% Good" }
                                                    span { class: "home-session-dot", "\u{00b7}" }
                                                    span { class: "home-session-metric history-metric--easy", "{item.easy_pct}% Easy" }
                                                }
                                            }
                                        }
                                        Link {
                                            class: "btn home-row__action",
                                            to: Route::Session { deck_id: item.deck_id.value() },
                                            "Start"
                                        }
                                    }
                                }
                            }
                            div { class: "home-panel__footer",
                                Link { class: "home-panel__link", to: Route::History {}, "View All Sessions \u{2192}" }
                            }
                        }

                        div { class: "home-panel",
                            div { class: "home-panel__header",
                                h4 { class: "home-panel__title", "Upcoming Review" }
                            }
                            div { class: "home-panel__list",
                                if data.upcoming_decks.is_empty() {
                                    div { class: "home-panel__empty",
                                        p { "No decks are ready yet." }
                                        Link { class: "btn home-row__action", to: Route::Practice {}, "Open Practice" }
                                    }
                                } else {
                                    for deck in data.upcoming_decks {
                                        div { class: "home-upcoming-row",
                                            div { class: "home-session-avatar home-session-avatar--muted",
                                                "{deck_initial(&deck.deck_name)}"
                                            }
                                            div { class: "home-session-text",
                                                h5 { class: "home-session-title", "{deck.deck_name}" }
                                                p { class: "home-session-meta",
                                                    "{deck.due} Due"
                                                    span { class: "home-session-dot", "\u{00b7}" }
                                                    "{deck.new} New"
                                                }
                                            }
                                            Link {
                                                class: "btn home-row__action",
                                                to: Route::PracticeDeck { deck_id: deck.deck_id.value() },
                                                "Practice"
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "home-panel__footer home-panel__footer--right",
                                Link { class: "home-panel__link", to: Route::Practice {}, "Find a Tag to Practice \u{2192}" }
                            }
                        }
                    }
                },
                ViewState::Error(err) => rsx! {
                    div { class: "home-error",
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
                    }
                },
            }
        }
    }
}

fn deck_initial(name: &str) -> String {
    name.chars().next().map_or_else(|| "?".to_string(), |ch| ch.to_string())
}
