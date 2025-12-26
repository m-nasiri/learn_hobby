use dioxus::prelude::*;

use crate::views::{ViewError, ViewState, view_state_from_resource};

#[derive(Clone, Debug, PartialEq)]
struct SettingsData {
    status: &'static str,
}

#[component]
pub fn SettingsView() -> Element {
    let resource = use_resource(move || async move {
        Ok::<_, ViewError>(SettingsData {
            status: "Settings placeholder",
        })
    });
    let state = view_state_from_resource(&resource);

    rsx! {
        div { class: "page",
            header { class: "view-header",
                h2 { class: "view-title", "Settings" }
                p { class: "view-subtitle", "Preferences and keyboard shortcuts." }
            }
            div { class: "view-divider" }
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
            }
        }
    }
}
