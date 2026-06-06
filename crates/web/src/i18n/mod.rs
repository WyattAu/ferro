mod en;

pub use en::EN;

/// Supported locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    #[default]
    En,
}

impl Locale {
    /// BCP-47 language tag.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::En => "en",
        }
    }

    /// Display name in its own language.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::En => "English",
        }
    }
}

/// Lookup a translation key in the given locale.
/// Returns the key itself as fallback if missing.
#[inline]
pub fn translate(locale: Locale, key: &'static str) -> &'static str {
    match locale {
        Locale::En => en::get(key).unwrap_or(key),
    }
}

/// Leptos context for accessing the current locale.
#[derive(Debug, Clone, Copy)]
pub struct I18nCtx {
    pub locale: Locale,
}

impl I18nCtx {
    /// Provide the i18n context to the component tree.
    pub fn provide(locale: Locale) {
        leptos::provide_context(Self { locale });
    }

    /// Read the i18n context from the nearest provider.
    pub fn expect() -> Self {
        leptos::expect_context::<Self>()
    }

    /// Translate a key using the current locale.
    #[inline]
    pub fn t(self, key: &'static str) -> &'static str {
        translate(self.locale, key)
    }
}

/// Macro for translation lookups.
///
/// Usage:
///   t!("common.cancel")         -> static &str
///   t!(ctx, "common.cancel")    -> uses I18nCtx from context
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::I18nCtx::expect().t($key)
    };
}
