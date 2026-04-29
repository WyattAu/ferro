pub mod api;
pub mod app;
pub mod auth;
pub mod components;
pub mod pages;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(app::App);
}
