use dioxus::prelude::*;

#[component]
pub fn HistoryView() -> Element {
    rsx! {
        div { class: "page",
            h2 { "History" }
            p { "History placeholder" }
        }
    }
}
