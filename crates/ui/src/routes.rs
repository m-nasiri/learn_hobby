use dioxus::prelude::*;
use dioxus_router::{Link, Outlet, Routable};

use crate::views::{EditorView, HistoryView, HomeView, SessionView, SettingsView, SummaryView};

#[derive(Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
        #[route("/", HomeView)] Home {},
        #[route("/session", SessionView)] Session {},
        #[route("/editor", EditorView)] Editor {},
        #[route("/history", HistoryView)] History {},
        #[route("/history/:summary_id", SummaryView)] Summary { summary_id: i64 },
        #[route("/settings", SettingsView)] Settings {},
}

#[component]
fn Layout() -> Element {
    rsx! {
        div { class: "app",
            Sidebar {}

            main { class: "content", role: "main", Outlet::<Route> {} }
        }
    }
}

#[component]
fn Sidebar() -> Element {
    rsx! {
        aside { class: "sidebar",
            header { class: "sidebar__header",
                h1 { class: "sidebar__title", "Learn" }
            }

            nav { class: "sidebar__nav", "aria-label": "Primary",
                ul { class: "sidebar__list",
                    NavItem { to: Route::Home {}, label: "Home" }
                    NavItem { to: Route::Session {}, label: "Practice" }
                    NavItem { to: Route::Editor {}, label: "Add / Edit" }
                    NavItem { to: Route::History {}, label: "History" }
                    NavItem { to: Route::Settings {}, label: "Settings" }
                }
            }
        }
    }
}

#[component]
fn NavItem(to: Route, label: &'static str) -> Element {
    rsx! {
        li { class: "sidebar__item",
            Link { class: "sidebar__link", to, {label} }
        }
    }
}
