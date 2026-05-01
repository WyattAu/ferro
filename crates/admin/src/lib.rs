use wasm_bindgen::prelude::*;

pub mod api;
pub mod app;
pub mod components;
pub mod pages;
pub mod state;

pub use app::App;

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
