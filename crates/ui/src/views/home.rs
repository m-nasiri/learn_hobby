use dioxus::prelude::*;

use crate::context::AppContext;
use crate::views::{ViewError, ViewState, view_state_from_resource};

#[derive(Clone, Debug, PartialEq)]
struct HomeData {
    deck_id: String,
    hint: &'static str,
}

#[component]
pub fn HomeView() -> Element {
    let ctx = use_context::<AppContext>();

    let deck_id = ctx.current_deck_id();

    let resource = use_resource(move || async move {
        Ok::<_, ViewError>(HomeData {
            deck_id: format!("{deck_id:?}"),
            hint: "Next: use SessionService to build a micro-session (Practice now).",
        })
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
                    p { "Current deck: {data.deck_id}" }
                    p { "{data.hint}" }
                },
                ViewState::Error(err) => rsx! {
                    p { "{err.message()}" }
                },
            }
        }
    }
}
