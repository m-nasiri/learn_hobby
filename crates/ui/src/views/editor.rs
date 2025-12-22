use dioxus::prelude::*;

#[component]
pub fn EditorView() -> Element {
    rsx! {
        div { class: "page",
            h2 { "Add / Edit Card" }
            p { "Editor placeholder" }
        }
    }
}
