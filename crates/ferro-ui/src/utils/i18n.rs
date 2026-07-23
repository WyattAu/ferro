//! Internationalization (i18n) scaffold.
//!
//! Currently English only. Structure in place for multi-language support.
//! Uses simple key-value lookup with fallback to key.

use std::collections::HashMap;

/// Translation map for a locale.
pub struct Translations {
    map: HashMap<String, String>,
}

impl Translations {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.map.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> String {
        self.map.get(key).cloned().unwrap_or_else(|| key.to_string())
    }
}

/// Load English translations.
pub fn load_english() -> Translations {
    let mut t = Translations::new();

    // Navigation
    t.insert("nav.files", "Files");
    t.insert("nav.notes", "Notes");
    t.insert("nav.tasks", "Tasks");
    t.insert("nav.calendar", "Calendar");
    t.insert("nav.contacts", "Contacts");
    t.insert("nav.chat", "Chat");
    t.insert("nav.photos", "Photos");
    t.insert("nav.trash", "Trash");
    t.insert("nav.admin", "Admin");
    t.insert("nav.settings", "Settings");

    // File browser
    t.insert("files.empty", "This folder is empty");
    t.insert("files.drop_hint", "Drop files here to upload");
    t.insert("files.upload_btn", "Choose files");
    t.insert("files.search", "Search files...");
    t.insert("files.items", "{} items");
    t.insert("files.selected", "{} selected");
    t.insert("files.select_all", "Select all");
    t.insert("files.clear", "Clear");

    // Common
    t.insert("common.loading", "Loading...");
    t.insert("common.error", "Error");
    t.insert("common.reload", "Reload");
    t.insert("common.save", "Save");
    t.insert("common.cancel", "Cancel");
    t.insert("common.delete", "Delete");
    t.insert("common.edit", "Edit");
    t.insert("common.create", "Create");
    t.insert("common.name", "Name");
    t.insert("common.size", "Size");
    t.insert("common.modified", "Modified");
    t.insert("common.actions", "Actions");

    // Trash
    t.insert("trash.empty", "Empty Trash");
    t.insert("trash.restore", "Restore");
    t.insert("trash.permanent", "Delete");

    // Settings
    t.insert("settings.account", "Account");
    t.insert("settings.preferences", "Preferences");
    t.insert("settings.appearance", "Appearance");
    t.insert("settings.notifications", "Notifications");

    t
}

/// Global translation context.
pub struct I18n {
    current: Translations,
}

impl I18n {
    pub fn new() -> Self {
        Self {
            current: load_english(),
        }
    }

    pub fn t(&self, key: &str) -> String {
        self.current.get(key)
    }
}

/// Get translation for key (global).
pub fn t(key: &str) -> String {
    // For now, return key as-is (English)
    key.to_string()
}
