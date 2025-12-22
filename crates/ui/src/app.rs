use dioxus::prelude::*;
use dioxus_router::Router;

use crate::context::provide_app_context;
use crate::routes::Route;

#[component]
pub fn App() -> Element {
    provide_app_context();

    rsx! {
        document::Stylesheet { href: asset!("/assets/style.css") }
        Router::<Route> {}
    }
}
