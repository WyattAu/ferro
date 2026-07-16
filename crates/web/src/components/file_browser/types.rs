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
    Graph,
    DualPane,
}

impl ViewMode {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ViewMode::List => "list",
            ViewMode::Grid => "grid",
            ViewMode::Graph => "graph",
            ViewMode::DualPane => "dual_pane",
        }
    }

    pub(crate) fn from_str(s: &str) -> Self {
        match s {
            "grid" => ViewMode::Grid,
            "graph" => ViewMode::Graph,
            "dual_pane" => ViewMode::DualPane,
            _ => ViewMode::List,
        }
    }
}
