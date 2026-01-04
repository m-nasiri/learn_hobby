use dioxus::prelude::*;
use dioxus_router::{Link, Outlet, Routable, use_navigator, use_route};

use crate::context::AppContext;
use crate::views::{
    EditorView, HistoryView, HomeView, PracticeView, SessionView, SettingsView, SummaryView,
};

#[derive(Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
        #[route("/", HomeView)] Home {},
        #[route("/practice", PracticeView)] Practice {},
        #[route("/session/:deck_id", SessionDeckRoute)] Session { deck_id: u64 },
        #[route("/session/:deck_id/all", SessionAllRoute)]
        SessionAll { deck_id: u64 },
        #[route("/session/:deck_id/mistakes", SessionMistakesRoute)]
        SessionMistakes { deck_id: u64 },
        #[route("/session/:deck_id/tag/:tag", SessionTagRoute)]
        SessionTag { deck_id: u64, tag: String },
        #[route("/editor", EditorView)] Editor {},
        #[route("/history", HistoryView)] History {},
        #[route("/history/:summary_id", SummaryView)] Summary { summary_id: i64 },
        #[route("/settings", SettingsRoute)] Settings {},
        #[route("/settings/deck/:deck_id", DeckSettingsRoute)]
        SettingsDeck { deck_id: u64 },
}

#[derive(Clone, Copy, PartialEq)]
enum NavIcon {
    Home,
    Practice,
    Edit,
    History,
    Settings,
}

#[component]
fn Layout() -> Element {
    let ctx = use_context::<AppContext>();
    let navigator = use_navigator();
    let route: Route = use_route();
    let mut did_redirect = use_signal(|| false);

    use_effect(move || {
        // `use_effect` runs after render; ensure this redirect logic only fires once.
        if *did_redirect.read() {
            return;
        }

        if ctx.take_open_editor_on_launch() && !matches!(route, Route::Editor { .. }) {
            did_redirect.set(true);
            navigator.push(Route::Editor {});
        }
    });

    rsx! {
        div { class: "app-shell",
            Sidebar {}

            main { class: "content", role: "main", Outlet::<Route> {} }
        }
    }
}

#[component]
fn SessionDeckRoute(deck_id: u64) -> Element {
    rsx! { SessionView { deck_id, tag: None, mode: crate::vm::SessionStartMode::Due } }
}

#[component]
fn SessionAllRoute(deck_id: u64) -> Element {
    rsx! { SessionView { deck_id, tag: None, mode: crate::vm::SessionStartMode::All } }
}

#[component]
fn SessionMistakesRoute(deck_id: u64) -> Element {
    rsx! { SessionView { deck_id, tag: None, mode: crate::vm::SessionStartMode::Mistakes } }
}

#[component]
fn SessionTagRoute(deck_id: u64, tag: String) -> Element {
    rsx! { SessionView { deck_id, tag: Some(tag), mode: crate::vm::SessionStartMode::Due } }
}

#[component]
fn SettingsRoute() -> Element {
    rsx! { SettingsView { deck_id: None } }
}

#[component]
fn DeckSettingsRoute(deck_id: u64) -> Element {
    rsx! { SettingsView { deck_id: Some(deck_id) } }
}

#[component]
fn Sidebar() -> Element {
    rsx! {
        aside { class: "sidebar", aria_label: "Sidebar",
            header { class: "sidebar__header",
                h1 { class: "sidebar__title", "Learn" }
            }

            nav { class: "sidebar__nav", aria_label: "Primary",
                ul { class: "sidebar__list",
                    NavItem { to: Route::Home {}, label: "Home", icon: NavIcon::Home }
                    NavItem { to: Route::Practice {}, label: "Practice", icon: NavIcon::Practice }
                    NavItem { to: Route::Editor {}, label: "Add / Edit", icon: NavIcon::Edit }
                    NavItem { to: Route::History {}, label: "History", icon: NavIcon::History }
                    NavItem { to: Route::Settings {}, label: "Settings", icon: NavIcon::Settings }
                }
            }
        }
    }
}

#[component]
fn NavItem(to: Route, label: &'static str, icon: NavIcon) -> Element {
    let icon = match icon {
        NavIcon::Home => rsx! {
            svg {
                class: "sidebar__icon",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.6",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                path { d: "M3 10.5L12 3l9 7.5" }
                path { d: "M5 10v9a1 1 0 0 0 1 1h4v-5h4v5h4a1 1 0 0 0 1-1v-9" }
            }
        },
        NavIcon::Practice => rsx! {
            svg {
                class: "sidebar__icon",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.6",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                path { d: "M8 5l11 7-11 7z" }
            }
        },
        NavIcon::Edit => rsx! {
            svg {
                class: "sidebar__icon",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.6",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                path { d: "M4 20h4l10-10-4-4L4 16v4z" }
                path { d: "M14 6l4 4" }
            }
        },
        NavIcon::History => rsx! {
            svg {
                class: "sidebar__icon",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.6",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                circle { cx: "12", cy: "12", r: "8" }
                path { d: "M12 7v5l3 2" }
            }
        },
        NavIcon::Settings => rsx! {
            svg {
                class: "sidebar__icon",
                view_box: "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.6",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                circle { cx: "12", cy: "12", r: "3" }
                path { d: "M19 12a7 7 0 0 0-.1-1l2-1.6-2-3.4-2.4.8a7.5 7.5 0 0 0-1.6-1L14 2h-4l-.9 2.8a7.5 7.5 0 0 0-1.6 1l-2.4-.8-2 3.4 2 1.6a7 7 0 0 0 0 2l-2 1.6 2 3.4 2.4-.8a7.5 7.5 0 0 0 1.6 1L10 22h4l.9-2.8a7.5 7.5 0 0 0 1.6-1l2.4.8 2-3.4-2-1.6c.1-.3.1-.7.1-1z" }
            }
        },
    };

    rsx! {
        li { class: "sidebar__item",
            Link {
                class: "sidebar__link",
                active_class: "sidebar__link--active",
                to,
                span { class: "sidebar__icon-wrap", aria_hidden: "true",
                    {icon}
                }
                span { class: "sidebar__label", "{label}" }
            }
        }
    }
}
