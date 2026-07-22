use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::infrastructure::error_boundary::ErrorBoundary;
use crate::components::infrastructure::toast::ToastProvider;
use crate::components::domain::file_browser::FileBrowser;
use crate::components::domain::notes::NotesPage;
use crate::components::domain::tasks::TasksPage;
use crate::components::domain::calendar::CalendarPage;
use crate::components::domain::contacts::ContactsPage;
use crate::components::domain::chat::ChatPage;
use crate::components::domain::photos::PhotosPage;
use crate::components::domain::settings::SettingsPage;
use crate::components::domain::admin::AdminPage;
use crate::components::domain::trash::TrashPage;
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
                        <Route path=path!("/ui/calendar") view=CalendarPage />
                        <Route path=path!("/ui/contacts") view=ContactsPage />
                        <Route path=path!("/ui/chat") view=ChatPage />
                        <Route path=path!("/ui/photos") view=PhotosPage />
                        <Route path=path!("/ui/settings") view=SettingsPage />
                        <Route path=path!("/ui/admin") view=AdminPage />
                        <Route path=path!("/ui/trash") view=TrashPage />
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
            <nav class="flex items-center gap-1 ml-6 overflow-x-auto">
                <a href="/ui/" class="nav-link">"Files"</a>
                <a href="/ui/notes" class="nav-link">"Notes"</a>
                <a href="/ui/tasks" class="nav-link">"Tasks"</a>
                <a href="/ui/calendar" class="nav-link">"Calendar"</a>
                <a href="/ui/contacts" class="nav-link">"Contacts"</a>
                <a href="/ui/chat" class="nav-link">"Chat"</a>
                <a href="/ui/photos" class="nav-link">"Photos"</a>
                <a href="/ui/trash" class="nav-link">"Trash"</a>
                <a href="/ui/admin" class="nav-link">"Admin"</a>
                <a href="/ui/settings" class="nav-link">"Settings"</a>
            </nav>
            <div class="ml-auto shrink-0">
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
