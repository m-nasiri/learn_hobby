use dioxus::prelude::*;

use crate::views::{ViewError, ViewState, view_state_from_resource};

#[derive(Clone, Debug, PartialEq)]
struct SessionData {
    status: &'static str,
}

#[component]
pub fn SessionView() -> Element {
    let resource = use_resource(move || async move {
        Ok::<_, ViewError>(SessionData {
            status: "Session placeholder",
        })
    });
    let state = view_state_from_resource(&resource);

    rsx! {
        div { class: "page",
            h2 { "Practice" }
            match state {
                ViewState::Idle => rsx! {
                    p { "Idle" }
                },
                ViewState::Loading => rsx! {
                    p { "Loading..." }
                },
                ViewState::Ready(data) => rsx! {
                    p { "{data.status}" }
                },
                ViewState::Error(err) => rsx! {
                    p { "{err.message()}" }
                },
            }
        }
    }
}
