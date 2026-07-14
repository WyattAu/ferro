#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserTab {
    Files,
    Favorites,
    Recent,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewMode {
    List,
    Grid,
}

impl ViewMode {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ViewMode::List => "list",
            ViewMode::Grid => "grid",
        }
    }

    pub(crate) fn from_str(s: &str) -> Self {
        match s {
            "grid" => ViewMode::Grid,
            _ => ViewMode::List,
        }
    }
}
