use dioxus::prelude::*;

use crate::context::AppContext;
use crate::views::{ViewError, ViewState, view_state_from_resource};

#[derive(Clone, Debug, PartialEq)]
struct EditorData {
    deck_id_label: String,
    status: &'static str,
}

#[component]
pub fn EditorView() -> Element {
    let ctx = use_context::<AppContext>();
    let deck_id = ctx.current_deck_id();
    let resource = use_resource(move || async move {
        Ok::<_, ViewError>(EditorData {
            deck_id_label: format!("{deck_id:?}"),
            status: "Editor placeholder",
        })
    });
    let state = view_state_from_resource(&resource);

    rsx! {
        div { class: "page",
            h2 { "Add / Edit Card" }
            match state {
                ViewState::Idle => rsx! {
                    p { "Idle" }
                },
                ViewState::Loading => rsx! {
                    p { "Loading..." }
                },
                ViewState::Ready(data) => rsx! {
                    p { "Deck: {data.deck_id_label}" }
                    p { "{data.status}" }
                },
                ViewState::Error(err) => rsx! {
                    p { "{err.message()}" }
                },
            }
        }
    }
}
