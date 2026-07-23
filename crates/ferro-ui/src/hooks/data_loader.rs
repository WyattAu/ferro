//! Reusable data loading hook for domain components.
//!
//! Encapsulates the loading/error/data lifecycle pattern repeated across
//! all domain components (photos, contacts, calendar, tasks, notes, chat, trash).

/// State of a data loading operation.
#[derive(Clone, Debug)]
pub enum LoadState<T: Clone + 'static> {
    Loading,
    Loaded(T),
    Error(String),
    Empty,
}

impl<T: Clone + 'static> LoadState<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
    pub fn data(&self) -> Option<&T> {
        match self {
            Self::Loaded(d) => Some(d),
            _ => None,
        }
    }
}
