use dioxus::prelude::*;
use dioxus_router::Router;

use crate::routes::Route;

#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: asset!("/assets/style.css") }
        Router::<Route> {}
    }
}
