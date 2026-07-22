//! Keyboard shortcuts manager.
//!
//! Registers global keyboard shortcuts and dispatches actions.
//! Follows defense-grade audit trail: every shortcut press logged.

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

/// Keyboard shortcut action.
pub type ShortcutAction = Box<dyn Fn()>;

/// Keyboard shortcut manager.
pub struct ShortcutManager {
    shortcuts: Rc<RefCell<HashMap<String, ShortcutAction>>>,
}

impl ShortcutManager {
    pub fn new() -> Self {
        Self {
            shortcuts: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Register a keyboard shortcut.
    pub fn register(&self, combo: &str, action: ShortcutAction) {
        self.shortcuts.borrow_mut().insert(combo.to_string(), action);
    }

    /// Handle a keydown event. Returns true if shortcut was consumed.
    pub fn handle_keydown(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        let combo = format!(
            "{}{}{}{}",
            if ctrl { "Ctrl+" } else { "" },
            if shift { "Shift+" } else { "" },
            if alt { "Alt+" } else { "" },
            key,
        );

        if let Some(action) = self.shortcuts.borrow().get(&combo) {
            log::debug!("Shortcut: {}", combo);
            action();
            true
        } else {
            false
        }
    }
}

/// Register default shortcuts for the application.
pub fn register_default_shortcuts(manager: &ShortcutManager) {
    // Global navigation
    manager.register("Ctrl+k", Box::new(|| {
        log::info!("Open command palette");
    }));

    manager.register("Ctrl+/", Box::new(|| {
        log::info!("Toggle keyboard shortcuts help");
    }));

    // File operations
    manager.register("Ctrl+n", Box::new(|| {
        log::info!("New file/folder");
    }));

    manager.register("Delete", Box::new(|| {
        log::info!("Delete selected");
    }));

    manager.register("Ctrl+a", Box::new(|| {
        log::info!("Select all");
    }));

    manager.register("Escape", Box::new(|| {
        log::info!("Clear selection / close dialog");
    }));

    // Navigation
    manager.register("g h", Box::new(|| {
        log::info!("Go to Home");
    }));

    manager.register("g n", Box::new(|| {
        log::info!("Go to Notes");
    }));

    manager.register("g t", Box::new(|| {
        log::info!("Go to Tasks");
    }));

    manager.register("g c", Box::new(|| {
        log::info!("Go to Calendar");
    }));
}
