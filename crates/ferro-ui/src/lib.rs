//! Ferro UI — Rewritten frontend to FAANG/Defense/HFT/ECN standards.
//!
//! Architecture:
//! - Components: primitives → layout → domain → infrastructure
//! - State: global → feature → component → server
//! - API: type-safe generated client from TOML schema
//! - Real-time: WebSocket manager with auto-reconnect
//! - Offline: IndexedDB local cache with conflict resolution

pub mod api;
pub mod components;
pub mod hooks;
pub mod routes;
pub mod stores;
pub mod styles;
pub mod utils;

use wasm_bindgen::prelude::*;

/// Entry point — called when WASM module initializes.
#[wasm_bindgen(start)]
pub fn main() {
    // Set panic hook for better error messages in console
    console_error_panic_hook::set_once();

    // Mount the app
    leptos::mount::mount_to_body(|| leptos::view! { <crate::routes::App/> });
}
