use dioxus::prelude::*;

use crate::context::AppContext;
use crate::views::{ViewError, ViewState, view_state_from_resource};
use crate::vm::{SessionSummaryDetailVm, map_session_summary_detail};

#[derive(Clone, Debug, PartialEq, Eq)]
struct SummaryData {
    summary: SessionSummaryDetailVm,
}

#[component]
pub fn SummaryView(summary_id: i64) -> Element {
    let ctx = use_context::<AppContext>();
    let summaries = ctx.session_summaries();

    let resource = use_resource(move || {
        let summaries = summaries.clone();
        let summary_id = summary_id;

        async move {
            let summary = summaries
                .get_summary(summary_id)
                .await
                // Keep error mapping in the UI boundary.
                // If you later add `ViewError::from_service(...)`, switch to that.
                .map_err(|_| ViewError::Unknown)?;

            Ok::<_, ViewError>(SummaryData {
                summary: map_session_summary_detail(&summary),
            })
        }
    });

    let state = view_state_from_resource(&resource);

    rsx! {
        div { class: "page",
            h2 { "Session Summary" }

            match state {
                ViewState::Idle => rsx! {
                    p { "Idle" }
                },
                ViewState::Loading => rsx! {
                    p { "Loading..." }
                },
                ViewState::Ready(data) => rsx! {
                    SummaryDetails { summary: data.summary.clone() }
                },
                ViewState::Error(err) => rsx! {
                    p { "{err.message()}" }
                },
            }
        }
    }
}

#[component]
fn SummaryDetails(summary: SessionSummaryDetailVm) -> Element {
    rsx! {
        // Definition list reads well for label/value pairs.
        dl { class: "summary",
            dt { "Started" }
            dd { "{summary.started_at_str}" }

            dt { "Completed" }
            dd { "{summary.completed_at_str}" }

            dt { "Total" }
            dd { "{summary.total}" }

            dt { "Again" }
            dd { "{summary.again}" }

            dt { "Hard" }
            dd { "{summary.hard}" }

            dt { "Good" }
            dd { "{summary.good}" }

            dt { "Easy" }
            dd { "{summary.easy}" }
        }
    }
}
