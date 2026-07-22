use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::infrastructure::error_boundary::ErrorBoundary;
use crate::components::infrastructure::toast::ToastProvider;
use crate::stores::theme::provide_theme;
use crate::stores::auth::provide_auth;

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    // Global state providers
    provide_theme();
    provide_auth();

    view! {
        <ErrorBoundary>
            <ToastProvider>
                <Router>
                    <Routes fallback=|| "Page not found".into_view()>
                        <Route path=path!("/") view=HomePage />
                        <Route path=path!("/ui/") view=HomePage />
                        <Route path=path!("/ui/files") view=HomePage />
                        <Route path=path!("/ui/files/*path") view=HomePage />
                    </Routes>
                </Router>
            </ToastProvider>
        </ErrorBoundary>
    }
}

/// Home / file browser page.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div class="shell">
            <header class="shell-header">
                <h1>"Ferro"</h1>
            </header>
            <main class="shell-content">
                <p class="text-secondary">"File browser — coming in Phase 1"</p>
            </main>
        </div>
    }
}
