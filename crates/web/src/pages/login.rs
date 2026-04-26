use leptos::*;
use leptos_router::*;

use crate::auth;
use crate::components::theme_toggle::{provide_theme_state, ThemeToggle};

#[component]
pub fn LoginPage() -> impl IntoView {
    let _state = auth::use_auth_state();
    provide_theme_state();

    let on_login = move |_: ev::MouseEvent| {
        auth::start_login();
    };

    view! {
        <div class="min-h-screen bg-gray-100 dark:bg-gray-900 flex items-center justify-center relative">
            <div class="absolute top-4 right-4">
                <ThemeToggle />
            </div>
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-8 max-w-md w-full text-center">
                <div class="flex items-center justify-center gap-2 mb-8">
                    <div class="w-10 h-10 bg-blue-600 rounded-lg flex items-center justify-center">
                        <span class="text-white font-bold text-lg">"F"</span>
                    </div>
                    <div class="text-left">
                        <h1 class="text-xl font-bold text-gray-900 dark:text-gray-100 leading-none">"Ferro"</h1>
                        <span class="text-xs text-gray-500 dark:text-gray-400">"Storage Orchestrator"</span>
                    </div>
                </div>

                <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">"Welcome to Ferro"</h2>
                <p class="text-gray-500 dark:text-gray-400 mb-6">"Sign in with your organization to continue"</p>

                <button
                    class="w-full bg-blue-600 text-white px-4 py-3 rounded-lg hover:bg-blue-700 transition-colors font-medium focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                    on:click=on_login
                >
                    "Sign in"
                </button>

                <A
                    href="/"
                    class="block mt-4 text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 no-underline focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 rounded"
                >
                    "Skip for now"
                </A>
            </div>
        </div>
    }
}
