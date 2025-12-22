use dioxus::prelude::*;
use dioxus_router::{Link, Outlet, Routable};

use crate::views::{EditorView, HistoryView, HomeView, SessionView, SettingsView};

#[derive(Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
        #[route("/", HomeView)] Home {},
        #[route("/session", SessionView)] Session {},
        #[route("/editor", EditorView)] Editor {},
        #[route("/history", HistoryView)] History {},
        #[route("/settings", SettingsView)] Settings {},
}

#[component]
fn Layout() -> Element {
    rsx! {
        div { class: "app",
            Sidebar {}
            main { class: "content",
                Outlet::<Route> {}
            }
        }
    }
}

#[component]
fn Sidebar() -> Element {
    rsx! {
        nav { class: "sidebar",
            h1 { "Learn" }
            ul {
                li { Link { to: Route::Home {}, "Home" } }
                li { Link { to: Route::Session {}, "Practice" } }
                li { Link { to: Route::Editor {}, "Add / Edit" } }
                li { Link { to: Route::History {}, "History" } }
                li { Link { to: Route::Settings {}, "Settings" } }
            }
        }
    }
}
