use leptos::prelude::*;

/// Available themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Theme {
    #[default]
    Light,
    Dark,
    System,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
            Theme::System => "system",
        }
    }

    pub fn is_dark(&self) -> bool {
        matches!(self, Theme::Dark)
    }
}

/// Theme state provided to the component tree.
#[derive(Clone)]
pub struct ThemeState {
    pub theme: ReadSignal<Theme>,
    set_theme: WriteSignal<Theme>,
}

impl ThemeState {
    pub fn toggle(&self) {
        let next = match self.theme.get() {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
            Theme::System => Theme::Light,
        };
        self.set_theme.set(next);
    }
}

/// Provide theme state to the component tree.
pub fn provide_theme() -> ThemeState {
    let initial = read_persisted_theme();
    let (theme, set_theme) = signal(initial);
    let state = ThemeState { theme, set_theme };
    provide_context(state.clone());

    state
}

/// Get theme state from context.
pub fn use_theme() -> ThemeState {
    use_context::<ThemeState>().expect("ThemeState not provided")
}

#[cfg(target_arch = "wasm32")]
fn read_persisted_theme() -> Theme {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|s| s.get_item("ferro_theme").ok())
        .flatten()
        .map(|v| match v.as_str() {
            "dark" => Theme::Dark,
            _ => Theme::Light,
        })
        .unwrap_or(Theme::Light)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_persisted_theme() -> Theme {
    Theme::Light
}
