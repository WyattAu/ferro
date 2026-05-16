use leptos::*;
use leptos_router::*;

use crate::auth;
use crate::components::error_boundary::ErrorBoundary;
use crate::components::onboarding::OnboardingOverlay;
use crate::components::toast::ProvideToastContext;
use crate::pages::admin::AdminPage;
use crate::pages::auth::AuthCallbackPage;
use crate::pages::home::HomePage;
use crate::pages::login::LoginPage;
use crate::pages::settings::SettingsPage;
use crate::pages::trash::TrashPage;

#[component]
pub fn App() -> impl IntoView {
    let auth_state = auth::provide_auth_state();

    create_effect(move |_| {
        auth::init_auth(&auth_state);
    });

    view! {
        <ErrorBoundary>
            <ProvideToastContext>
                <Router base="/ui">
                    <Routes base="/ui">
                        <Route path="/" view=RootView />
                        <Route path="/files" view=RootView />
                        <Route path="/files/*path" view=FileViewRoute />
                        <Route path="/trash" view=TrashPage />
                        <Route path="/settings" view=SettingsPage />
                        <Route path="/admin" view=AdminPage />
                        <Route path="/auth/callback" view=AuthCallbackPage />
                        <Route path="/auth/login" view=LoginPage />
                    </Routes>
                </Router>
            </ProvideToastContext>
            <OnboardingOverlay />
        </ErrorBoundary>
    }
}

#[component]
fn RootView() -> impl IntoView {
    view! {
        <HomePage initial_path="/".to_string() />
    }
}

#[component]
fn FileViewRoute() -> impl IntoView {
    let params = use_params_map();
    let path = move || {
        params.with(|p| {
            p.get("path")
                .map(|v| format!("/{}", v))
                .unwrap_or("/".to_string())
        })
    };
    view! {
        <HomePage initial_path=path() />
    }
}
