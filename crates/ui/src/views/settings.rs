use dioxus::prelude::*;

#[component]
pub fn SettingsView() -> Element {
    rsx! {
        div { class: "page",
            h2 { "Settings" }
            p { "Settings placeholder" }
        }
    }
}
