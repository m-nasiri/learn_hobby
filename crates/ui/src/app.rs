use dioxus::prelude::*;
use dioxus_router::Router;

use crate::routes::Route;

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: asset!("/assets/style.css") }

        // Stable OS/window title. Per-route titles are rendered inside the right pane.
        document::Title { "Learn" }

        // A single root container for global layout CSS hooks.
        div { class: "app-root",
            ErrorBoundary {
                handle_error: |errors: ErrorContext| rsx! {
                    div { class: "fatal",
                        h1 { "Something went wrong" }
                        pre { "{errors:?}" }
                    }
                },
                Router::<Route> {}
            }
        }
    }
}
