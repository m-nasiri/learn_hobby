use dioxus::prelude::*;
use dioxus_router::Link;

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

    let state = view_state_from_resource(&resource);

    rsx! {
        div { class: "page",
            h2 { "History" }

            match state {
                ViewState::Idle => rsx! {
                    p { "Idle" }
                },
                ViewState::Loading => rsx! {
                    p { "Loading..." }
                },
                ViewState::Ready(data) => rsx! {
                    if data.cards.is_empty() {
                        p { "No recent sessions yet." }
                    } else {
                        ul {
                            for card in data.cards {
                                SummaryCard { card }
                            }
                        }
                    }
                },
                ViewState::Error(err) => rsx! {
                    p { "{err.message()}" }
                },
            }
        }
    }
}

#[component]
fn SummaryCard(card: SessionSummaryCardVm) -> Element {
    rsx! {
        li {
            Link { class: "summary-link", to: Route::Summary { summary_id: card.id },
                span { class: "summary-date", "{card.completed_at_str}" }
                span { class: "summary-cta", "View" }
            }
            p {
                "Total: {card.total} | Again: {card.again} | Hard: {card.hard} | Good: {card.good} | Easy: {card.easy}"
            }
        }
    }
}
