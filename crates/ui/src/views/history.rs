use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use services::SessionSummaryListItem;

use crate::context::AppContext;
use crate::views::{ViewError, ViewState, view_state_from_resource};

#[derive(Clone, Debug, PartialEq)]
struct HistoryData {
    items: Vec<SessionSummaryListItem>,
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
            Ok(HistoryData { items })
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
                    if data.items.is_empty() {
                        p { "No recent sessions yet." }
                    } else {
                        ul {
                            for item in data.items {
                                SummaryCard { item }
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
fn SummaryCard(item: SessionSummaryListItem) -> Element {
    let completed_at_str = format_datetime(item.completed_at);

    rsx! {
        li {
            p { "{completed_at_str}" }
            p {
                "Total: {item.total} | Again: {item.again} | Hard: {item.hard} | Good: {item.good} | Easy: {item.easy}"
            }
        }
    }
}

fn format_datetime(value: DateTime<Utc>) -> String {
    // UI-level formatting. If you later want locale/relative time, change it here.
    value.to_rfc3339()
}
