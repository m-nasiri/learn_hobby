use dioxus::prelude::*;

use crate::context::AppContext;

#[component]
pub fn HomeView() -> Element {
    let ctx = use_context::<AppContext>();

    rsx! {
        div { class: "page",
            h2 { "Home" }
            p { "App: {ctx.app().app_name()}" }
            p { "Next: wire real services into UiApp." }
        }
    }
}
