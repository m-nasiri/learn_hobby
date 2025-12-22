use dioxus::prelude::*;

use crate::context::AppContext;
use crate::views::{view_state_from_resource, ViewError, ViewState};

#[derive(Clone, Debug, PartialEq)]
struct HomeData {
    app_name: &'static str,
    hint: &'static str,
}

#[component]
pub fn HomeView() -> Element {
    let ctx = use_context::<AppContext>();
    let app_name = ctx.app().app_name();
    let resource = use_resource(move || async move {
        Ok::<_, ViewError>(HomeData {
            app_name,
            hint: "Next: wire real services into UiApp.",
        })
    });
    let state = view_state_from_resource(resource);

    rsx! {
        div { class: "page",
            h2 { "Home" }
            match state {
                ViewState::Idle => rsx! { p { "Idle" } },
                ViewState::Loading => rsx! { p { "Loading..." } },
                ViewState::Ready(data) => rsx! {
                    p { "App: {data.app_name}" }
                    p { "{data.hint}" }
                },
                ViewState::Error(_) => rsx! {
                    p { "{ViewError::message()}" }
                },
            }
        }
    }
}
