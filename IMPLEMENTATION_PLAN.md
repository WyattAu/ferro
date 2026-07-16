# Implementation Plan: Real-Time Co-Editing & Tabs in File Manager

## Overview

This plan implements two features:
1. **Real-Time Co-Editing** - Enhanced collaborative editing with cursor presence, typing indicators, and conflict resolution UI
2. **Tabs in File Manager** - Browser-like tabs for multiple folder views

---

## Feature 1: Real-Time Co-Editing

### Current State Analysis

The codebase already has a solid CRDT foundation:

- **`crates/crdt/src/document.rs`**: `CrdtDocument` with `join()`, `leave()`, `insert_text()`, `delete_text()`, `apply_ops()` methods. Tracks participants via `HashMap<ParticipantId, ParticipantInfo>`.
- **`crates/crdt/src/text.rs`**: `RgaString` CRDT implementation with operation-based synchronization.
- **`crates/server-collaboration/src/collab_ws.rs`**: WebSocket handler with `CollabMessage` protocol (Join, Operations, State, Participants, Hello, DocumentState). Room-based architecture with broadcast.
- **`crates/web/src/components/collaboration.rs`**: Client-side `CollabEditor` component with `CollabStateHandle`, WebSocket connection, reconnection logic, and basic `PresenceIndicator`.

### What Needs to Be Built

The existing `CollabEditor` component (line 322 in collaboration.rs) already handles:
- WebSocket connection and reconnection
- CRDT document synchronization
- Basic presence (avatar initials)

**Missing features to implement:**
1. Cursor position indicators (colored cursors showing where other users are editing)
2. Typing indicators ("User is typing...")
3. Conflict resolution UI (visual feedback when CRDT conflicts occur)
4. Enhanced connection status indicator
5. Auto-save via CRDT sync (already handled by server's `idle_save_loop`)

### Implementation Steps

#### Step 1: Add Cursor Position Tracking to CRDT Protocol

**File: `crates/server-collaboration/src/collab_ws.rs`**

Add new message variant to `CollabMessage`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollabMessage {
    // ... existing variants ...
    CursorPosition {
        participant_id: u32,
        position: usize,
        selection_end: Option<usize>,
    },
    TypingIndicator {
        participant_id: u32,
        is_typing: bool,
    },
}
```

Update `Room` struct to track cursor positions:
```rust
struct Room {
    // ... existing fields ...
    cursor_positions: DashMap<u32, CursorPosition>,
}
```

#### Step 2: Update Client-Side SyncMessage Protocol

**File: `crates/web/src/components/collaboration.rs`**

Add matching variants to `SyncMessage`:
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SyncMessage {
    // ... existing variants ...
    CursorPosition {
        participant_id: u32,
        position: usize,
        selection_end: Option<usize>,
    },
    TypingIndicator {
        participant_id: u32,
        is_typing: bool,
    },
}
```

#### Step 3: Create CursorPresence Component

**File: `crates/web/src/components/collaboration.rs`** (add to existing file)

```rust
#[derive(Debug, Clone)]
pub struct CursorInfo {
    pub participant_id: ParticipantId,
    pub name: String,
    pub position: usize,
    pub selection_end: Option<usize>,
    pub color: String,
}

#[component]
pub fn CursorPresence(
    cursors: ReadSignal<Vec<CursorInfo>>,
    text_ref: NodeRef<html::Textarea>,
) -> impl IntoView {
    // Render colored cursor indicators overlaid on textarea
    // Each cursor shows: colored line | name label
}
```

#### Step 4: Create TypingIndicator Component

**File: `crates/web/src/components/collaboration.rs`**

```rust
#[component]
pub fn TypingIndicator(typing_users: ReadSignal<Vec<String>>) -> impl IntoView {
    view! {
        {move || {
            let users = typing_users.get();
            if users.is_empty() {
                view! { <span class="hidden"></span> }.into_any()
            } else {
                let text = match users.len() {
                    1 => format!("{} is typing...", users[0]),
                    2 => format!("{} and {} are typing...", users[0], users[1]),
                    _ => format!("{} users are typing...", users.len()),
                };
                view! {
                    <div class="flex items-center gap-2 px-4 py-1 text-xs text-[var(--text-tertiary)]">
                        <div class="flex gap-1">
                            <span class="w-1.5 h-1.5 bg-[var(--accent)] rounded-full animate-bounce" style="animation-delay: 0ms"></span>
                            <span class="w-1.5 h-1.5 bg-[var(--accent)] rounded-full animate-bounce" style="animation-delay: 150ms"></span>
                            <span class="w-1.5 h-1.5 bg-[var(--accent)] rounded-full animate-bounce" style="animation-delay: 300ms"></span>
                        </div>
                        <span>{text}</span>
                    </div>
                }.into_any()
            }
        }}
    }
}
```

#### Step 5: Create ConflictIndicator Component

**File: `crates/web/src/components/collaboration.rs`**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictState {
    None,
    Resolving,
    Resolved,
}

#[component]
pub fn ConflictIndicator(state: ReadSignal<ConflictState>) -> impl IntoView {
    view! {
        {move || match state.get() {
            ConflictState::None => view! { <span class="hidden"></span> }.into_any(),
            ConflictState::Resolving => view! {
                <div class="flex items-center gap-2 px-3 py-1 bg-[var(--warning-subtle)] text-[var(--warning)] text-xs rounded">
                    <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                    </svg>
                    <span>Syncing changes...</span>
                </div>
            }.into_any(),
            ConflictState::Resolved => view! {
                <div class="flex items-center gap-2 px-3 py-1 bg-[var(--success-subtle)] text-[var(--success)] text-xs rounded">
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                    </svg>
                    <span>Changes synced</span>
                </div>
            }.into_any(),
        }}
    }
}
```

#### Step 6: Enhance CollabEditor Component

**File: `crates/web/src/components/collaboration.rs`**

Update the existing `CollabEditor` component to integrate new features:

```rust
#[component]
pub fn CollabEditor(document_id: String, participant_name: String) -> impl IntoView {
    // ... existing signals ...
    let (cursors, set_cursors) = signal(Vec::<CursorInfo>::new());
    let (typing_users, set_typing_users) = signal(Vec::<String>::new());
    let (conflict_state, set_conflict_state) = signal(ConflictState::None);
    let (last_cursor_position, set_last_cursor_position) = signal(0usize);
    
    // Debounced typing indicator
    let typing_timeout: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
    
    // ... existing setup ...
    
    // In on_input handler, add cursor position broadcast:
    let handle_for_input = handle.clone();
    let on_input = move |ev: web_sys::Event| {
        let new_value = event_target_value(&ev);
        let old_value = handle_for_input.get_text();
        if old_value != new_value {
            handle_for_input.apply_local_edit(&old_value, &new_value);
            
            // Broadcast cursor position
            let target: web_sys::HtmlTextAreaElement = ev.target_unchecked_into();
            let position = target.selection_start().unwrap_or(Some(0)).unwrap_or(0) as usize;
            handle_for_input.send_cursor_position(position, None);
            
            // Broadcast typing indicator
            handle_for_input.send_typing_indicator(true);
            // Reset typing timeout
        }
    };
    
    // In WebSocket message handler, handle CursorPosition and TypingIndicator messages
    
    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center justify-between px-4 py-2 border-b bg-[var(--bg-base)]">
                <div class="flex items-center gap-2">
                    <ConnectionBadge state=connection_state />
                    <span class="text-xs font-mono text-[var(--text-tertiary)]">
                        {move || format!("v{}", version.get())}
                    </span>
                    <ConflictIndicator state=conflict_state />
                </div>
                <PresenceIndicator />
            </div>
            <div class="relative flex-1">
                <textarea
                    class="flex-1 w-full p-4 font-mono text-sm resize-none focus:outline-none bg-[var(--bg-surface)] dark:text-gray-100"
                    prop:value=text
                    on:input=on_input
                    on:keydown=on_keydown
                    prop:disabled=is_read_only
                    spellcheck="false"
                />
                <CursorPresence cursors cursors=text_ref />
            </div>
            <TypingIndicator typing_users />
        </div>
    }
}
```

#### Step 7: Add Cursor Color Assignment

**File: `crates/web/src/components/collaboration.rs`**

```rust
const CURSOR_COLORS: &[&str] = &[
    "#3B82F6", // blue
    "#10B981", // green
    "#F59E0B", // amber
    "#EF4444", // red
    "#8B5CF6", // violet
    "#EC4899", // pink
    "#06B6D4", // cyan
    "#84CC16", // lime
];

fn assign_color(participant_id: u32) -> String {
    let index = (participant_id as usize) % CURSOR_COLORS.len();
    CURSOR_COLORS[index].to_string()
}
```

---

## Feature 2: Tabs in File Manager

### Current State Analysis

The file browser (`crates/web/src/components/file_browser/mod.rs`) currently:
- Uses a single `current_path` signal
- Has tab switching for Files/Favorites/Recent views (via `BrowserTab` enum)
- Supports list and grid view modes
- Has toolbar with navigation controls

**The `BrowserTab` enum in `types.rs`** is for view modes (Files/Favorites/Recent), not browser tabs.

### What Needs to Be Built

1. **TabState struct** - Represents a single browser tab
2. **TabBar component** - Visual tab bar with add/close/switch
3. **Tab state management** - Signals for multiple tabs
4. **localStorage persistence** - Save/restore tabs across sessions
5. **Integration with FileBrowser** - Use active tab's path instead of single path

### Implementation Steps

#### Step 1: Create TabState Types

**File: `crates/web/src/components/file_browser/types.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TabState {
    pub id: String,
    pub path: String,
    pub title: String,
    pub is_active: bool,
}

impl TabState {
    pub fn new(path: &str) -> Self {
        let title = path
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("Home")
            .to_string();
        
        TabState {
            id: uuid::Uuid::new_v4().to_string(),
            path: path.to_string(),
            title,
            is_active: false,
        }
    }
    
    pub fn title_from_path(path: &str) -> String {
        path.rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("Home")
            .to_string()
    }
}

// Keep existing BrowserTab and ViewMode enums
```

#### Step 2: Create TabBar Component

**File: `crates/web/src/components/file_browser/tab_bar.rs`**

```rust
use leptos::prelude::*;
use super::types::TabState;

#[component]
pub fn TabBar(
    tabs: ReadSignal<Vec<TabState>>,
    on_switch: Callback<String>,
    on_close: Callback<String>,
    on_add: Callback<()>,
    on_reorder: Callback<(String, String)>,
) -> impl IntoView {
    let (dragging_tab, set_dragging_tab) = signal(None::<String>);
    let (drag_over_tab, set_drag_over_tab) = signal(None::<String>);
    
    view! {
        <div class="flex items-center bg-[var(--bg-surface-sunken)] border-b border-[var(--border-default)] overflow-x-auto">
            <For
                each=move || tabs.get()
                key=|tab| tab.id.clone()
                let:tab
            >
                {
                    let tab_id = tab.id.clone();
                    let tab_title = tab.title.clone();
                    let is_active = tab.is_active;
                    
                    view! {
                        <div
                            class=move || format!(
                                "group flex items-center gap-2 px-4 py-2 border-r border-[var(--border-default)] cursor-pointer min-w-[120px] max-w-[200px] {}",
                                if is_active { "bg-[var(--bg-surface)] border-b-2 border-b-[var(--accent)]" } else { "hover:bg-[var(--interactive-hover)]" }
                            )
                            draggable="true"
                            on:dragstart=move |ev| {
                                ev.data_transfer().unwrap().set_data("text/plain", &tab_id).ok();
                                set_dragging_tab.set(Some(tab_id.clone()));
                            }
                            on:dragover=move |ev| {
                                ev.prevent_default();
                                set_drag_over_tab.set(Some(tab_id.clone()));
                            }
                            on:drop=move |ev| {
                                ev.prevent_default();
                                if let Some(source_id) = dragging_tab.get() {
                                    on_reorder.call((source_id, tab_id.clone()));
                                }
                                set_dragging_tab.set(None);
                                set_drag_over_tab.set(None);
                            }
                            on:click=move |_| on_switch.call(tab_id.clone())
                        >
                            <span class="text-sm truncate">{tab_title}</span>
                            <button
                                class="opacity-0 group-hover:opacity-100 p-0.5 hover:bg-[var(--danger-subtle)] hover:text-[var(--danger)] rounded transition-opacity"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    on_close.call(tab_id.clone());
                                }
                                aria-label="Close tab"
                            >
                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                </svg>
                            </button>
                        </div>
                    }
                }
            </For>
            
            <button
                class="flex items-center justify-center px-3 py-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] transition-colors"
                on:click=move |_| on_add.call(())
                aria-label="New tab"
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                </svg>
            </button>
        </div>
    }
}
```

#### Step 3: Add localStorage Persistence

**File: `crates/web/src/components/file_browser/tab Persistence`**

```rust
use wasm_bindgen::prelude::*;

const TABS_STORAGE_KEY: &str = "ferro_file_browser_tabs";

pub fn save_tabs_to_storage(tabs: &[TabState]) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(json) = serde_json::to_string(tabs) {
            if let Some(window) = web_sys::window() {
                if let Ok(storage) = window.local_storage() {
                    let _ = storage.set_item(TABS_STORAGE_KEY, &json);
                }
            }
        }
    }
}

pub fn load_tabs_from_storage() -> Vec<TabState> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(storage) = window.local_storage() {
                if let Ok(Some(json)) = storage.get_item(TABS_STORAGE_KEY) {
                    if let Ok(tabs) = serde_json::from_str::<Vec<TabState>>(&json) {
                        if !tabs.is_empty() {
                            return tabs;
                        }
                    }
                }
            }
        }
    }
    
    // Default: single tab at home
    vec![TabState::new("/")]
}
```

#### Step 4: Update FileBrowser Component

**File: `crates/web/src/components/file_browser/mod.rs`**

Major changes:

1. Replace single `current_path` signal with tabs array:
```rust
// OLD:
let (current_path, set_current_path) = signal(initial_path);

// NEW:
let (tabs, set_tabs) = signal(load_tabs_from_storage());
let (active_tab_id, set_active_tab_id) = signal(String::new());

// Derive current_path from active tab
let current_path = move || {
    tabs.with(|tabs| {
        tabs.iter()
            .find(|t| t.is_active)
            .map(|t| t.path.clone())
            .unwrap_or_else(|| "/".to_string())
    })
};
```

2. Add tab operations:
```rust
let add_tab = move |path: String| {
    set_tabs.update(|tabs| {
        // Deactivate all tabs
        for tab in tabs.iter_mut() {
            tab.is_active = false;
        }
        // Add new active tab
        let mut new_tab = TabState::new(&path);
        new_tab.is_active = true;
        tabs.push(new_tab);
        set_active_tab_id.set(new_tab.id.clone());
    });
    save_tabs_to_storage(&tabs.get());
};

let close_tab = move |tab_id: String| {
    set_tabs.update(|tabs| {
        if let Some(idx) = tabs.iter().position(|t| t.id == tab_id) {
            tabs.remove(idx);
            // If closed tab was active, activate adjacent tab
            if tabs.iter().all(|t| !t.is_active) {
                let new_idx = idx.min(tabs.len().saturating_sub(1));
                if let Some(new_active) = tabs.get_mut(new_idx) {
                    new_active.is_active = true;
                    set_active_tab_id.set(new_active.id.clone());
                }
            }
        }
        // Always have at least one tab
        if tabs.is_empty() {
            let mut default_tab = TabState::new("/");
            default_tab.is_active = true;
            tabs.push(default_tab);
            set_active_tab_id.set(tabs[0].id.clone());
        }
    });
    save_tabs_to_storage(&tabs.get());
};

let switch_tab = move |tab_id: String| {
    set_tabs.update(|tabs| {
        for tab in tabs.iter_mut() {
            tab.is_active = tab.id == tab_id;
        }
    });
    set_active_tab_id.set(tab_id.clone());
    save_tabs_to_storage(&tabs.get());
};
```

3. Update `navigate` function to update current tab:
```rust
let navigate = move |path: String| {
    let tab_id = active_tab_id.get();
    set_tabs.update(|tabs| {
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.path = path.clone();
            tab.title = TabState::title_from_path(&path);
        }
    });
    load_directory(path);
    save_tabs_to_storage(&tabs.get());
};
```

4. Add TabBar to view:
```rust
view! {
    <div class="flex flex-col h-full">
        // Tab bar
        <TabBar
            tabs
            on_switch=switch_tab
            on_close=close_tab
            on_add=add_tab_for_current
            on_reorder=reorder_tabs
        />
        
        // Existing toolbar and content
        <Toolbar ... />
        // ... rest of component
    }
}
```

#### Step 5: Register TabBar Module

**File: `crates/web/src/components/file_browser/mod.rs`**

```rust
mod tab_bar;
```

---

## Verification

### cargo check --workspace

Run after implementation to verify compilation:
```bash
cargo check --workspace
```

### Manual Testing Checklist

**Feature 1 (Co-Editing):**
- [ ] Open same document in two browser windows
- [ ] Verify cursors appear with different colors
- [ ] Verify typing indicators show when other user types
- [ ] Verify changes sync in real-time
- [ ] Disconnect one client, verify reconnection
- [ ] Check conflict resolution UI appears during concurrent edits

**Feature 2 (Tabs):**
- [ ] Click "+" to add new tab
- [ ] Navigate to different folder in new tab
- [ ] Switch between tabs
- [ ] Close tab (verify adjacent tab activates)
- [ ] Refresh page (verify tabs persist)
- [ ] Drag to reorder tabs
- [ ] Verify only one tab active at a time

---

## Files to Create/Modify

### New Files
1. `crates/web/src/components/file_browser/tab_bar.rs` - TabBar component
2. `crates/web/src/components/file_browser/tab Persistence.rs` (or inline in types.rs)

### Modified Files
1. `crates/server-collaboration/src/collab_ws.rs` - Add cursor/typing messages
2. `crates/web/src/components/collaboration.rs` - Add CursorPresence, TypingIndicator, ConflictIndicator components
3. `crates/web/src/components/file_browser/types.rs` - Add TabState struct
4. `crates/web/src/components/file_browser/mod.rs` - Integrate TabBar, tab state management
5. `crates/web/src/components/mod.rs` - May need to expose new modules

---

## Dependencies

- `uuid` crate for generating tab IDs (already in workspace)
- `serde` / `serde_json` for localStorage serialization (already in use)
- `wasm-bindgen` for web APIs (already in use)
