use leptos::prelude::*;

use crate::components::clipboard::provide_clipboard_state;
use crate::components::command_palette::{CommandPalette, provide_command_palette_state};
use crate::components::file_browser::FileBrowser;
use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;
use crate::t;

#[component]
pub fn HomePage(initial_path: String) -> impl IntoView {
    provide_theme_state();
    provide_clipboard_state();
    provide_command_palette_state();
    provide_header_state();
    view! {
        <div class="h-screen flex flex-col bg-[var(--bg-base)]">
            <div style="position:fixed;top:0;left:0;z-index:9999;background:red;color:white;padding:8px;font-size:16px;">DEBUG: HomePage rendered</div>
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-[var(--accent)] focus:text-[var(--text-on-accent)] focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 overflow-auto px-2 sm:px-4 pt-16">
                <main id="main-content" class="max-w-7xl w-full mx-auto bg-[var(--bg-surface)] shadow-sm rounded-xl">
                    <FileBrowser initial_path=initial_path />
                </main>
            </div>
            <CommandPalette />
        </div>
    }
}
