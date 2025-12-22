#![allow(non_snake_case)]

mod app;
mod context;
mod routes;
mod views;

fn main() {
    dioxus::LaunchBuilder::desktop().launch(app::App);
}
