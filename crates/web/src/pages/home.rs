use leptos::prelude::*;

use crate::components::clipboard::provide_clipboard_state;
use crate::components::command_palette::provide_command_palette_state;
use crate::components::file_browser::FileBrowser;
use crate::components::header::{Header, provide_header_state};

#[component]
pub fn HomePage(initial_path: String) -> impl IntoView {
    provide_clipboard_state();
    provide_command_palette_state();
    provide_header_state();
    view! {
        <div style="display:grid; grid-template-rows:auto 1fr; height:100vh; overflow:hidden;">
            <Header />
            <div style="overflow:hidden; position:relative;">
                <FileBrowser initial_path=initial_path />
            </div>
        </div>
    }
}
