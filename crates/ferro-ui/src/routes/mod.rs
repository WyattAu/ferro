use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::infrastructure::error_boundary::ErrorBoundary;
use crate::components::infrastructure::toast::ToastProvider;
use crate::components::domain::file_browser::FileBrowser;
use crate::components::domain::notes::NotesPage;
use crate::components::domain::tasks::TasksPage;
use crate::stores::theme::provide_theme;
use crate::stores::auth::provide_auth;
use crate::styles::inject_styles;

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    inject_styles();
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
                        <Route path=path!("/ui/notes") view=NotesPage />
                        <Route path=path!("/ui/tasks") view=TasksPage />
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
            <AppHeader />
            <main class="shell-content" style="padding:0;">
                <FileBrowser />
            </main>
        </div>
    }
}

/// Shared app header with navigation.
#[component]
fn AppHeader() -> impl IntoView {
    view! {
        <header class="shell-header">
            <a href="/ui/" class="text-xl font-bold tracking-tight">"⚡ Ferro"</a>
            <nav class="flex items-center gap-2 ml-6">
                <a href="/ui/" class="nav-link">"Files"</a>
                <a href="/ui/notes" class="nav-link">"Notes"</a>
                <a href="/ui/tasks" class="nav-link">"Tasks"</a>
            </nav>
            <div class="ml-auto">
                <ThemeToggle />
            </div>
        </header>
    }
}

/// Theme toggle button.
#[component]
fn ThemeToggle() -> impl IntoView {
    let theme = crate::stores::theme::use_theme();

    view! {
        <button
            class="btn btn-ghost btn-sm"
            on:click=move |_| theme.toggle()
            aria-label="Toggle theme"
        >
            {move || if theme.theme.get().is_dark() { "🌙" } else { "☀️" }}
        </button>
    }
}
