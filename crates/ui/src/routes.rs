use dioxus::prelude::*;
use dioxus_router::{Link, Outlet, Routable, use_navigator, use_route};

use crate::context::AppContext;
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
    let ctx = use_context::<AppContext>();
    let navigator = use_navigator();
    let route: Route = use_route();

    use_effect(move || {
        if ctx.take_open_editor_on_launch() && !matches!(route, Route::Editor { .. }) {
            navigator.push(Route::Editor {});
        }
    });

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

            nav { class: "sidebar__nav", aria_label: "Primary",
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
            Link {
                class: "sidebar__link",
                active_class: "sidebar__link--active",
                to,
                {label}
            }
        }
    }
}
