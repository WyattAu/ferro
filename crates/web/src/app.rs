use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::*;
use leptos_router::hooks::*;
use leptos_router::path;

use crate::api::BrandingConfig;
use crate::auth;
use crate::components::audio_player::AudioPlayer;
use crate::components::error_boundary::ErrorBoundary;
use crate::components::onboarding::OnboardingOverlay;
use crate::components::setup_wizard::SetupWizard;
use crate::components::theme_toggle::provide_theme_state;
use crate::components::toast::ProvideToastContext;
use crate::i18n::{I18nCtx, Locale};
use crate::pages::admin::AdminPage;
use crate::pages::analytics::AnalyticsPage;
use crate::pages::auth::AuthCallbackPage;
use crate::pages::calendar::CalendarPage;
use crate::pages::chat::ChatPage;
use crate::pages::contacts::ContactsPage;
use crate::pages::dashboard::DashboardPage;
use crate::pages::home::HomePage;
use crate::pages::login::LoginPage;
use crate::pages::mail::MailPage;
use crate::pages::notes::NotesPage;
use crate::pages::photos::PhotosPage;
use crate::pages::settings::SettingsPage;
use crate::pages::tasks::TasksPage;
use crate::pages::trash::TrashPage;
use crate::pages::whiteboard::WhiteboardPage;
use crate::t;

#[component]
pub fn App() -> impl IntoView {
    I18nCtx::provide(Locale::default());
    provide_theme_state();
    let auth_state = auth::provide_auth_state();
    let (branding, set_branding) = signal(None::<BrandingConfig>);
    provide_context(branding);

    Effect::new(move |_| {
        auth::init_auth(&auth_state);
    });

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(config) = crate::api::fetch_branding().await {
                set_branding.set(Some(config));
            }
        });
    });

    Effect::new(move |_| {
        let b = branding.get();
        if let Some(ref config) = b {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                if let Some(window) = web_sys::window() {
                    if let Some(doc) = window.document() {
                        doc.set_title(&config.title);
                        if let Some(el) = doc.document_element() {
                            if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
                                let _ = html_el.style().set_property("--accent", &config.primary_color);
                            }
                        }

                        if let Some(ref favicon_url) = config.favicon_url {
                            if let Some(head) = doc.head() {
                                let existing = doc.query_selector("link[rel~='icon']").ok().flatten();
                                if let Some(link) = existing {
                                    if let Ok(link) = link.dyn_into::<web_sys::HtmlLinkElement>() {
                                        link.set_href(favicon_url);
                                    }
                                } else if let Some(link) = doc.create_element("link").ok().and_then(|e| {
                                    use wasm_bindgen::JsCast;
                                    e.dyn_into::<web_sys::HtmlLinkElement>().ok()
                                }) {
                                    link.set_rel("icon");
                                    link.set_href(favicon_url);
                                    let _ = head.append_child(&link);
                                }
                            }
                        }

                        if let Some(ref css) = config.custom_css {
                            let existing = doc.query_selector("#ferro-branding-css").ok().flatten();
                            if let Some(el) = existing {
                                if let Ok(style) = el.dyn_into::<web_sys::HtmlStyleElement>() {
                                    let _ = style.set_text_content(Some(css));
                                }
                            } else if let Some(style) = doc.create_element("style").ok().and_then(|e| {
                                use wasm_bindgen::JsCast;
                                e.dyn_into::<web_sys::HtmlStyleElement>().ok()
                            }) {
                                style.set_id("ferro-branding-css");
                                let _ = style.set_text_content(Some(css));
                                if let Some(head) = doc.head() {
                                    let _ = head.append_child(&style);
                                }
                            }
                        }
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            let _ = config;
        }
    });

    view! {
        <ErrorBoundary>
            <ProvideToastContext>
                <Router base="/".to_string()>
                    <Routes fallback=|| t!("common.not_found")>
                        <Route path=path!("/") view=RootView />
                        <Route path=path!("/dashboard") view=DashboardPage />
                        <Route path=path!("/files/") view=RootView />
                        <Route path=path!("/files/*path") view=FileViewRoute />
                        <Route path=path!("/trash") view=TrashPage />
                        <Route path=path!("/settings") view=SettingsPage />
                        <Route path=path!("/admin") view=AdminPage />
                        <Route path=path!("/calendar") view=CalendarPage />
                        <Route path=path!("/contacts") view=ContactsPage />
                        <Route path=path!("/notes") view=NotesPage />
                        <Route path=path!("/tasks") view=TasksPage />
                        <Route path=path!("/chat") view=ChatPage />
                        <Route path=path!("/chat/*room_id") view=ChatPage />
                        <Route path=path!("/photos") view=PhotosPage />
                        <Route path=path!("/mail") view=MailPage />
                        <Route path=path!("/whiteboard") view=WhiteboardPage />
                        <Route path=path!("/whiteboard/:id") view=WhiteboardPage />
                        <Route path=path!("/analytics") view=AnalyticsPage />
                        <Route path=path!("/auth/callback") view=AuthCallbackPage />
                        <Route path=path!("/auth/login") view=LoginPage />
                    </Routes>
                </Router>
            </ProvideToastContext>
            <AudioPlayer />
            <OnboardingOverlay />
            <SetupWizard />
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
    let path = move || params.with(|p| p.get("path").map(|v| format!("/{}", v)).unwrap_or("/".to_string()));
    view! {
        <HomePage initial_path=path() />
    }
}
