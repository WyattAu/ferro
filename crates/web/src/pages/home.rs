use leptos::*;

use crate::components::clipboard::provide_clipboard_state;
use crate::components::command_palette::{CommandPalette, provide_command_palette_state};
use crate::components::file_browser::FileBrowser;
use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;

#[component]
pub fn HomePage(initial_path: String) -> impl IntoView {
    provide_theme_state();
    provide_clipboard_state();
    provide_command_palette_state();
    provide_header_state();
    view! {
        <div class="h-screen flex flex-col bg-gray-100 dark:bg-gray-900">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded">"Skip to main content"</a>
            <Header />
            <div class="flex-1 overflow-auto px-2 sm:px-4 pt-16">
                <main id="main-content" class="max-w-7xl w-full mx-auto bg-white dark:bg-gray-800 shadow-sm rounded-xl">
                    <FileBrowser initial_path=initial_path />
                </main>
            </div>
            <CommandPalette />
        </div>
    }
}
