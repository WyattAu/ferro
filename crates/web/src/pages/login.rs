use leptos::ev;
use leptos::prelude::*;
use leptos_router::components::*;

use crate::auth;
use crate::components::theme_toggle::{ThemeToggle, provide_theme_state};
use crate::t;

#[component]
pub fn LoginPage() -> impl IntoView {
    provide_theme_state();

    let on_login = move |_: ev::MouseEvent| {
        auth::start_login();
    };

    view! {
        <div class="min-h-screen bg-gray-100 dark:bg-gray-900 flex items-center justify-center relative">
            <div class="absolute top-4 right-4">
                <ThemeToggle />
            </div>
            <div class="brutal-block rounded-lg shadow-2xl p-8 max-w-md w-full text-center">
                <div class="flex items-center justify-center gap-2 mb-8">
                    <div class="w-10 h-10 bg-transparent brutal-border rounded flex items-center justify-center font-display text-display text-accent">{t!("brand.name")}</div>
                    <div class="text-left">
                        <h1 class="text-xl font-bold font-mono text-gray-900 leading-none">{t!("brand.name")}</h1>
                        <span class="text-label text-muted">{t!("brand.tagline")}</span>
                    </div>
                </div>

                <h2 class="text-section font-mono text-gray-900 mb-2">{t!("login.welcome")}</h2>
                <p class="text-muted font-mono mb-6">{t!("login.description")}</p>

                <button
                    class="w-full bg-blue-600 text-white px-4 py-3 rounded-sm hover:bg-blue-700 transition-colors font-bold uppercase tracking-widest brutal-border shadow-iron focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                    on:click=on_login
                >
                    {t!("common.sign_in")}
                </button>

                <A
                    href="/"
                    attr:class="block mt-4 text-sm text-muted font-mono hover:text-gray-700 no-underline focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 rounded text-label"
                >
                    {t!("common.skip_for_now")}
                </A>
            </div>
        </div>
    }
}
