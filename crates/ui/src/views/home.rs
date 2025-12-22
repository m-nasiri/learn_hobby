use dioxus::prelude::*;
use dioxus_router::Link;

use crate::context::AppContext;
use crate::routes::Route;
use crate::views::{ViewError, ViewState, view_state_from_resource};

#[derive(Clone, Debug, PartialEq, Eq)]
struct HomeData {
    deck_id_label: String,
    recent_count: u32,
}

#[component]
pub fn HomeView() -> Element {
    let ctx = use_context::<AppContext>();

    // Keep IDs as domain types in-flight; only format at the UI boundary.
    let deck_id = ctx.current_deck_id();
    let summaries = ctx.session_summaries();

    let resource = use_resource(move || {
        let summaries = summaries.clone();
        let deck_id = deck_id;

        async move {
            let items = summaries
                .list_recent_summaries(deck_id, 7, 5)
                .await
                .map_err(|_| ViewError::Unknown)?;

            // `limit` is tiny, but keep the conversion explicit and safe.
            let capped = items.len().min(u32::MAX as usize);
            let recent_count = u32::try_from(capped).unwrap_or(u32::MAX);

            Ok::<_, ViewError>(HomeData {
                deck_id_label: format!("{deck_id:?}"),
                recent_count,
            })
        }
    });

    let state = view_state_from_resource(&resource);

    rsx! {
        div { class: "page",
            h2 { "Home" }

            match state {
                ViewState::Idle => rsx! {
                    p { "Idle" }
                },
                ViewState::Loading => rsx! {
                    p { "Loading..." }
                },
                ViewState::Ready(data) => rsx! {
                    p { "Current deck: {data.deck_id_label}" }
                    p { "Recent sessions (7d): {data.recent_count}" }
                    Link { class: "btn btn-primary", to: Route::Session {}, "Practice now" }
                },
                ViewState::Error(err) => rsx! {
                    p { "{err.message()}" }
                },
            }
        }
    }
}
