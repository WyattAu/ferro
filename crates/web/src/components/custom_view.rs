use serde::{Deserialize, Serialize};

/// Column visibility configuration for custom views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnConfig {
    pub name: bool,
    pub size: bool,
    pub modified: bool,
    pub mime_type: bool,
    pub tags: bool,
    pub rating: bool,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            name: true,
            size: true,
            modified: true,
            mime_type: false,
            tags: false,
            rating: false,
        }
    }
}

/// Sort direction for custom views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

impl SortDirection {
    pub fn label(&self) -> &'static str {
        match self {
            SortDirection::Asc => "Ascending",
            SortDirection::Desc => "Descending",
        }
    }
}

/// Sort field for custom views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SortField {
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "size")]
    Size,
    #[serde(rename = "modified")]
    Modified,
    #[serde(rename = "type")]
    Type,
}

impl SortField {
    pub fn label(&self) -> &'static str {
        match self {
            SortField::Name => "Name",
            SortField::Size => "Size",
            SortField::Modified => "Modified",
            SortField::Type => "Type",
        }
    }
}

/// Group by options for custom views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GroupBy {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "type")]
    Type,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "size")]
    Size,
}

impl GroupBy {
    pub fn label(&self) -> &'static str {
        match self {
            GroupBy::None => "None",
            GroupBy::Type => "Type",
            GroupBy::Date => "Date",
            GroupBy::Size => "Size",
        }
    }
}

/// Filter configuration for custom views.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FilterConfig {
    pub file_type: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

/// A custom view configuration that can be saved and loaded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomView {
    pub id: String,
    pub name: String,
    pub columns: ColumnConfig,
    pub sort_field: SortField,
    pub sort_direction: SortDirection,
    pub group_by: GroupBy,
    pub filter: FilterConfig,
}

impl CustomView {
    pub fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            columns: ColumnConfig::default(),
            sort_field: SortField::Name,
            sort_direction: SortDirection::Asc,
            group_by: GroupBy::None,
            filter: FilterConfig::default(),
        }
    }
}

/// Preset view configurations.
#[derive(Clone)]
pub enum ViewPreset {
    Default,
    Compact,
    Detailed,
    Media,
    Documents,
}

impl ViewPreset {
    pub fn label(&self) -> &'static str {
        match self {
            ViewPreset::Default => "Default",
            ViewPreset::Compact => "Compact",
            ViewPreset::Detailed => "Detailed",
            ViewPreset::Media => "Media",
            ViewPreset::Documents => "Documents",
        }
    }

    pub fn to_custom_view(&self) -> CustomView {
        match self {
            ViewPreset::Default => CustomView::new("Default"),
            ViewPreset::Compact => {
                let mut view = CustomView::new("Compact");
                view.columns.modified = false;
                view.columns.mime_type = false;
                view
            }
            ViewPreset::Detailed => {
                let mut view = CustomView::new("Detailed");
                view.columns.tags = true;
                view.columns.rating = true;
                view.columns.mime_type = true;
                view
            }
            ViewPreset::Media => {
                let mut view = CustomView::new("Media");
                view.columns.size = false;
                view.columns.modified = false;
                view.columns.mime_type = true;
                view.group_by = GroupBy::Type;
                view.filter.file_type = Some("image/*|video/*|audio/*".to_string());
                view
            }
            ViewPreset::Documents => {
                let mut view = CustomView::new("Documents");
                view.columns.tags = true;
                view.sort_field = SortField::Modified;
                view.sort_direction = SortDirection::Desc;
                view.filter.file_type = Some("application/pdf|text/*".to_string());
                view
            }
        }
    }
}

/// localStorage key for persisting custom views.
#[allow(dead_code)]
const CUSTOM_VIEWS_KEY: &str = "ferro_custom_views";

/// Load all custom views from localStorage.
pub fn load_custom_views() -> Vec<CustomView> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window()
            && let Ok(Some(storage)) = window.local_storage()
            && let Ok(Some(data)) = storage.get_item(CUSTOM_VIEWS_KEY)
            && let Ok(views) = serde_json::from_str::<Vec<CustomView>>(&data)
        {
            return views;
        }
        vec![]
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        vec![]
    }
}

/// Save all custom views to localStorage.
pub fn save_custom_views(_views: &[CustomView]) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(json) = serde_json::to_string(_views)
            && let Some(window) = web_sys::window()
            && let Ok(Some(storage)) = window.local_storage()
        {
            let _ = storage.set_item(CUSTOM_VIEWS_KEY, &json);
        }
    }
}

/// Save a single custom view (add or update).
pub fn save_custom_view(view: &CustomView) {
    let mut views = load_custom_views();
    if let Some(existing) = views.iter_mut().find(|v| v.id == view.id) {
        *existing = view.clone();
    } else {
        views.push(view.clone());
    }
    save_custom_views(&views);
}

/// Delete a custom view by ID.
pub fn delete_custom_view(id: &str) {
    let mut views = load_custom_views();
    views.retain(|v| v.id != id);
    save_custom_views(&views);
}

/// Get a custom view by ID.
pub fn get_custom_view(id: &str) -> Option<CustomView> {
    load_custom_views().into_iter().find(|v| v.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_config_default() {
        let config = ColumnConfig::default();
        assert!(config.name);
        assert!(config.size);
        assert!(config.modified);
        assert!(!config.mime_type);
        assert!(!config.tags);
        assert!(!config.rating);
    }

    #[test]
    fn test_sort_direction_label() {
        assert_eq!(SortDirection::Asc.label(), "Ascending");
        assert_eq!(SortDirection::Desc.label(), "Descending");
    }

    #[test]
    fn test_sort_field_label() {
        assert_eq!(SortField::Name.label(), "Name");
        assert_eq!(SortField::Size.label(), "Size");
        assert_eq!(SortField::Modified.label(), "Modified");
        assert_eq!(SortField::Type.label(), "Type");
    }

    #[test]
    fn test_group_by_label() {
        assert_eq!(GroupBy::None.label(), "None");
        assert_eq!(GroupBy::Type.label(), "Type");
        assert_eq!(GroupBy::Date.label(), "Date");
        assert_eq!(GroupBy::Size.label(), "Size");
    }

    #[test]
    fn test_custom_view_new() {
        let view = CustomView::new("Test View");
        assert_eq!(view.name, "Test View");
        assert!(!view.id.is_empty());
        assert_eq!(view.sort_field, SortField::Name);
        assert_eq!(view.sort_direction, SortDirection::Asc);
    }

    #[test]
    fn test_view_preset_default() {
        let view = ViewPreset::Default.to_custom_view();
        assert_eq!(view.name, "Default");
        assert_eq!(view.columns, ColumnConfig::default());
    }

    #[test]
    fn test_view_preset_compact() {
        let view = ViewPreset::Compact.to_custom_view();
        assert_eq!(view.name, "Compact");
        assert!(!view.columns.modified);
    }

    #[test]
    fn test_view_preset_detailed() {
        let view = ViewPreset::Detailed.to_custom_view();
        assert!(view.columns.tags);
        assert!(view.columns.rating);
    }

    #[test]
    fn test_view_preset_media() {
        let view = ViewPreset::Media.to_custom_view();
        assert_eq!(view.group_by, GroupBy::Type);
        assert!(view.filter.file_type.is_some());
    }

    #[test]
    fn test_filter_config_default() {
        let config = FilterConfig::default();
        assert!(config.file_type.is_none());
        assert!(config.min_size.is_none());
    }

    #[test]
    fn test_custom_view_serde() {
        let view = CustomView::new("Test");
        let json = serde_json::to_string(&view).unwrap();
        let parsed: CustomView = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test");
        assert_eq!(parsed.id, view.id);
    }
}
