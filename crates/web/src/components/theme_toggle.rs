use leptos::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    #[allow(dead_code)] // Used by WASM runtime
    fn as_str(&self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    #[allow(dead_code)] // Used by WASM runtime
    fn from_str(s: &str) -> Self {
        match s {
            "dark" => Theme::Dark,
            _ => Theme::Light,
        }
    }
}

#[derive(Clone)]
pub struct ThemeState {
    theme: ReadSignal<Theme>,
    set_theme: WriteSignal<Theme>,
}

impl ThemeState {
    pub fn theme(&self) -> ReadSignal<Theme> {
        self.theme
    }

    pub fn set_theme(&self, theme: Theme) {
        self.set_theme.set(theme);
    }

    pub fn is_dark(&self) -> bool {
        self.theme.get() == Theme::Dark
    }
}

pub fn provide_theme_state() -> ThemeState {
    let (theme, set_theme) = create_signal(Theme::default());

    let state = ThemeState { theme, set_theme };
    provide_context(state.clone());

    #[cfg(target_arch = "wasm32")]
    {
        let init_state = state.clone();
        create_effect(move |_| {
            let window = web_sys::window();
            let document = window.as_ref().and_then(|w| w.document());

            let Some(document) = document else {
                return;
            };
            let html = document.document_element();

            if let Some(html) = html {
                let stored = window
                    .as_ref()
                    .and_then(|w| w.local_storage().ok())
                    .and_then(|s| s.get_item("ferro_theme").ok())
                    .flatten();

                let initial = stored.map(|s| Theme::from_str(&s)).unwrap_or_else(|| {
                    let prefers_dark = window
                        .as_ref()
                        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok())
                        .flatten()
                        .map(|mql| mql.matches())
                        .unwrap_or(false);
                    if prefers_dark {
                        Theme::Dark
                    } else {
                        Theme::Light
                    }
                });

                init_state.set_theme(initial);

                if initial == Theme::Dark {
                    let _ = html.class_list().add_1("dark");
                } else {
                    let _ = html.class_list().remove_1("dark");
                }
            }
        });

        let listen_state = state.clone();
        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                if let Ok(mql) = window.match_media("(prefers-color-scheme: dark)") {
                    let cb = wasm_bindgen::closure::Closure::wrap(Box::new(
                        move |_e: web_sys::MediaQueryListEvent| {
                            let prefers_dark = mql.matches();
                            let theme = if prefers_dark {
                                Theme::Dark
                            } else {
                                Theme::Light
                            };
                            listen_state.set_theme(theme);
                        },
                    )
                        as Box<dyn Fn(web_sys::MediaQueryListEvent)>);
                    let _ =
                        mql.add_event_listener_with_callback("change", cb.as_ref().unchecked_ref());
                    cb.forget();
                }
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let sync_state = state.clone();
        create_effect(move |_| {
            let current = sync_state.theme.get();
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(html) = document.document_element() {
                        if current == Theme::Dark {
                            let _ = html.class_list().add_1("dark");
                        } else {
                            let _ = html.class_list().remove_1("dark");
                        }
                    }
                }
                if let Ok(storage) = window.local_storage() {
                    let _ = storage.set_item("ferro_theme", current.as_str());
                }
            }
        });
    }

    state
}

pub fn use_theme_state() -> ThemeState {
    use_context::<ThemeState>().expect("ThemeState not provided")
}

#[component]
pub fn ThemeToggle() -> impl IntoView {
    let theme_state = use_theme_state();
    let ts_for_view = theme_state.clone();

    let toggle = move |_: ev::MouseEvent| {
        let current = theme_state.theme.get();
        let next = match current {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        };
        theme_state.set_theme(next);
    };

    view! {
        <button
            class="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded surface brutal-border shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
            on:click=toggle
            aria-label="Toggle theme"
        >
            {move || match ts_for_view.theme.get() {
                Theme::Light => view! {
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                    </svg>
                }.into_any(),
                Theme::Dark => view! {
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                    </svg>
                }.into_any(),
            }}
        </button>
    }
}
