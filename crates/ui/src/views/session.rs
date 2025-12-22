use dioxus::prelude::*;

#[component]
pub fn SessionView() -> Element {
    rsx! {
        div { class: "page",
            h2 { "Practice" }
            p { "Session placeholder" }
        }
    }
}
