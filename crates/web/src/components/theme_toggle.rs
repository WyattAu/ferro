use leptos::ev;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;

#[cfg(target_arch = "wasm32")]
use crate::styles::dark_mode;
use crate::styles::dark_mode::Theme;

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
        self.theme.get().is_dark()
    }

    pub fn effective_is_dark(&self) -> bool {
        let t = self.theme.get();
        match t {
            Theme::System => {
                #[cfg(target_arch = "wasm32")]
                {
                    dark_mode::detect_system_theme() == "dark"
                }
                #[cfg(not(target_arch = "wasm32"))]
                false
            }
            other => other.is_dark(),
        }
    }
}

pub fn provide_theme_state() -> ThemeState {
    let (theme, set_theme) = signal(Theme::default());

    let state = ThemeState { theme, set_theme };
    provide_context(state.clone());

    #[cfg(target_arch = "wasm32")]
    {
        // Inject theme CSS
        dark_mode::inject_theme_css();

        // Resolve initial theme from localStorage > system preference
        let initial = dark_mode::resolve_initial_theme();
        state.set_theme(initial);

        // Apply initial theme
        let effective = dark_mode::resolve_effective_theme(initial);
        dark_mode::apply_theme(effective);
        dark_mode::persist_theme(initial.as_str());

        // Listen for system theme changes
        let _listen_state = state.clone();
        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                use wasm_bindgen::JsCast;
                if let Ok(Some(mql)) = window.match_media("(prefers-color-scheme: dark)") {
                    let mql = std::rc::Rc::new(mql);
                    let mql_ref = mql.clone();
                    let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MediaQueryListEvent| {
                        // When system theme changes and user has "system" selected,
                        // re-apply the effective theme
                        let _prefers_dark = mql_ref.matches();
                        // The Effect below handles re-application
                    })
                        as Box<dyn Fn(web_sys::MediaQueryListEvent)>);
                    let _ = mql.add_event_listener_with_callback("change", cb.as_ref().unchecked_ref());
                    cb.forget();
                }
            }
        });

        // Sync theme state changes to DOM
        let sync_state = state.clone();
        Effect::new(move |_| {
            let current = sync_state.theme.get();
            let effective = dark_mode::resolve_effective_theme(current);
            dark_mode::apply_theme(effective);
            dark_mode::persist_theme(current.as_str());
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
    let ts_for_cycle = theme_state.clone();

    // Cycle through themes
    let cycle_theme = move |_: ev::MouseEvent| {
        let current = ts_for_cycle.theme.get();
        let next = match current {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Midnight,
            Theme::Midnight => Theme::SolarizedLight,
            Theme::SolarizedLight => Theme::SolarizedDark,
            Theme::SolarizedDark => Theme::Nord,
            Theme::Nord => Theme::TokyoNight,
            Theme::TokyoNight => Theme::Dracula,
            Theme::Dracula => Theme::HighContrast,
            Theme::HighContrast => Theme::Sepia,
            Theme::Sepia => Theme::Forest,
            Theme::Forest => Theme::Ocean,
            Theme::Ocean => Theme::System,
            Theme::System => Theme::Light,
            Theme::Custom => Theme::Light,
        };
        ts_for_cycle.set_theme(next);
    };

    view! {
        <button
            class="p-2 rounded-md text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--interactive-hover)] transition-colors focus-ring min-w-[44px] min-h-[44px] flex items-center justify-center"
            on:click=cycle_theme
            aria-label=move || format!("Theme: {}", ts_for_view.theme.get().display_name())
            title=move || format!("Current theme: {}. Click to cycle.", ts_for_view.theme.get().display_name())
        >
            {move || match ts_for_view.theme.get() {
                Theme::Light => view! {
                    <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                    </svg>
                }.into_any(),
                Theme::Sepia => view! {
                    <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                    </svg>
                }.into_any(),
                Theme::SolarizedLight => view! {
                    <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                    </svg>
                }.into_any(),
                Theme::Dark | Theme::Midnight | Theme::SolarizedDark | Theme::Nord | Theme::TokyoNight | Theme::Dracula | Theme::HighContrast | Theme::Forest | Theme::Ocean | Theme::Custom => view! {
                    <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                    </svg>
                }.into_any(),
                Theme::System => view! {
                    <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                    </svg>
                }.into_any(),
            }}
        </button>
    }
}

/// Full theme picker with all options visible.
#[component]
pub fn ThemePicker() -> impl IntoView {
    let theme_state = use_theme_state();

    view! {
        <div class="flex gap-2" role="radiogroup" aria-label="Select theme">
            {Theme::all().iter().map(|t| {
                let theme = *t;
                let ts = theme_state.clone();
                let is_active = move || ts.theme.get() == theme;
                view! {
                    <button
                        class=move || {
                            let base = "px-3 py-2 rounded-md text-sm font-medium transition-colors min-w-[44px] min-h-[44px] focus-ring";
                            if is_active() {
                                format!("{} bg-[var(--accent)] text-[var(--text-on-accent)]", base)
                            } else {
                                format!("{} bg-[var(--bg-surface-raised)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--interactive-hover)]", base)
                            }
                        }
                        on:click=move |_| ts.set_theme(theme)
                        aria-checked=is_active
                        role="radio"
                    >
                        {theme.display_name()}
                    </button>
                }
            }).collect_view()}
        </div>
    }
}
