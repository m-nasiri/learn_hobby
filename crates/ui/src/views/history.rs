use dioxus::prelude::*;

use crate::views::{view_state_from_resource, ViewError, ViewState};

#[derive(Clone, Debug, PartialEq)]
struct HistoryData {
    status: &'static str,
}

#[component]
pub fn HistoryView() -> Element {
    let resource = use_resource(move || async move {
        Ok::<_, ViewError>(HistoryData {
            status: "History placeholder",
        })
    });
    let state = view_state_from_resource(resource);

    rsx! {
        div { class: "page",
            h2 { "History" }
            match state {
                ViewState::Idle => rsx! { p { "Idle" } },
                ViewState::Loading => rsx! { p { "Loading..." } },
                ViewState::Ready(data) => rsx! {
                    p { "{data.status}" }
                },
                ViewState::Error(_) => rsx! {
                    p { "{ViewError::message()}" }
                },
            }
        }
    }
}
