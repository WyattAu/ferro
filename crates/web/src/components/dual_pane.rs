use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::file_browser::FileBrowser;
use crate::components::toast::ToastContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct DualPaneState {
    pub left_path: String,
    pub right_path: String,
    pub active_pane: Pane,
    pub pane_ratio: f64,
    pub sync_scrolling: bool,
}

impl Default for DualPaneState {
    fn default() -> Self {
        Self::new()
    }
}

impl DualPaneState {
    pub fn new() -> Self {
        Self {
            left_path: "/".to_string(),
            right_path: "/".to_string(),
            active_pane: Pane::Left,
            pane_ratio: 0.5,
            sync_scrolling: false,
        }
    }
}

#[component]
pub fn DualPane(
    initial_left: Option<String>,
    initial_right: Option<String>,
) -> impl IntoView {
    let (state, set_state) = signal(DualPaneState {
        left_path: initial_left.unwrap_or_else(|| "/".to_string()),
        right_path: initial_right.unwrap_or_else(|| "/".to_string()),
        active_pane: Pane::Left,
        pane_ratio: 0.5,
        sync_scrolling: false,
    });

    let (dragging_divider, set_dragging_divider) = signal(false);
    let (container_width, _set_container_width) = signal(1200.0);

    let handle_divider_mouse_down = move |ev: ev::MouseEvent| {
        ev.prevent_default();
        set_dragging_divider.set(true);
    };

    let handle_divider_mouse_move = move |ev: ev::MouseEvent| {
        if dragging_divider.get() {
            let x = ev.client_x() as f64;
            let width = container_width.get();
            let ratio = (x / width).clamp(0.2, 0.8);
            set_state.update(|s| s.pane_ratio = ratio);
        }
    };

    let handle_divider_mouse_up = move |_: ev::MouseEvent| {
        set_dragging_divider.set(false);
    };

    let toggle_sync_scrolling = move |_: ev::MouseEvent| {
        set_state.update(|s| s.sync_scrolling = !s.sync_scrolling);
    };

    let switch_active_pane = move |pane: Pane| {
        set_state.update(|s| s.active_pane = pane);
    };

    let swap_panes = move |_: ev::MouseEvent| {
        set_state.update(|s| {
            let tmp = s.left_path.clone();
            s.left_path = s.right_path.clone();
            s.right_path = tmp;
        });
    };

    // On mobile, detect screen width and show stacked layout
    let _is_mobile = {
        #[cfg(target_arch = "wasm32")]
        {
            let width = web_sys::window()
                .map(|w| w.inner_width().unwrap_or_default().as_f64().unwrap_or(1024.0))
                .unwrap_or(1024.0);
            width < 768.0
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            false
        }
    };

    let copy_path_to_other = move |from: Pane| {
        let path = match from {
            Pane::Left => state.get().left_path.clone(),
            Pane::Right => state.get().right_path.clone(),
        };
        match from {
            Pane::Left => set_state.update(|s| s.right_path = path),
            Pane::Right => set_state.update(|s| s.left_path = path),
        }
    };

    let _handle_drag_drop = move |(_source_pane, file_path, dest_pane): (Pane, String, Pane)| {
        let file_name = file_path.rsplit('/').next().unwrap_or("").to_string();
        let dest_path = match dest_pane {
            Pane::Left => state.get().left_path.clone(),
            Pane::Right => state.get().right_path.clone(),
        };
        let dest = if dest_path == "/" {
            format!("/{}", file_name)
        } else {
            format!("{}/{}", dest_path, file_name)
        };

        spawn_local(async move {
            match api::move_file(&file_path, &dest).await {
                Ok(()) => {
                    ToastContext::success(format!("Moved {} to {}", file_name, dest));
                }
                Err(e) => {
                    ToastContext::error(format!("Move failed: {}", e));
                }
            }
        });
    };

    let left_pane_ref = NodeRef::<html::Div>::new();
    let right_pane_ref = NodeRef::<html::Div>::new();

    view! {
        <div class="flex flex-col h-full relative">
            // Mobile stacked layout (hidden on sm+)
            <div class="sm:hidden flex flex-col h-full">
                // Mobile pane switcher tabs
                <div class="flex items-center bg-[var(--bg-surface)] border-b border-[var(--border-default)]">
                    <button
                        class=move || format!(
                            "flex-1 px-3 py-2 text-xs font-bold uppercase text-center transition-colors {}",
                            if state.get().active_pane == Pane::Left { "bg-[var(--accent-subtle)] text-[var(--accent)] border-b-2 border-b-[var(--accent)]" } else { "text-[var(--text-tertiary)]" }
                        )
                        on:click=move |_| switch_active_pane(Pane::Left)
                    >
                        "Left"
                    </button>
                    <button
                        class=move || format!(
                            "flex-1 px-3 py-2 text-xs font-bold uppercase text-center transition-colors {}",
                            if state.get().active_pane == Pane::Right { "bg-[var(--accent-subtle)] text-[var(--accent)] border-b-2 border-b-[var(--accent)]" } else { "text-[var(--text-tertiary)]" }
                        )
                        on:click=move |_| switch_active_pane(Pane::Right)
                    >
                        "Right"
                    </button>
                </div>
                // Mobile pane content - show active pane full width
                {move || match state.get().active_pane {
                    Pane::Left => view! {
                        <div class="flex-1 overflow-hidden">
                            <FileBrowser initial_path=state.get().left_path />
                        </div>
                    }.into_any(),
                    Pane::Right => view! {
                        <div class="flex-1 overflow-hidden">
                            <FileBrowser initial_path=state.get().right_path />
                        </div>
                    }.into_any(),
                }}
            </div>

            // Desktop dual-pane layout (hidden on mobile)
            <div
                class="hidden sm:flex h-full relative flex-1"
                on:mousemove=handle_divider_mouse_move
                on:mouseup=handle_divider_mouse_up
                on:mouseleave=handle_divider_mouse_up
            >
                // Left pane
                <div
                    node_ref=left_pane_ref
                    class="flex-1 overflow-hidden border-r border-[var(--border-default)]"
                    style=move || format!("width: {}%; flex: none;", state.get().pane_ratio * 100.0)
                    on:click=move |_| switch_active_pane(Pane::Left)
                >
                    <div class="flex items-center justify-between px-3 py-1.5 bg-[var(--bg-surface)] border-b border-[var(--border-default)]">
                        <div class="flex items-center gap-2">
                            <span class="text-xs font-bold uppercase text-[var(--text-tertiary)]">"Left"</span>
                            <span class="text-xs text-[var(--text-tertiary)] font-mono">
                                {move || state.get().left_path}
                            </span>
                        </div>
                        <div class="flex items-center gap-1">
                            <button
                                on:click=move |_| copy_path_to_other(Pane::Right)
                                class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded"
                                title="Copy path to right"
                            >
                                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                                </svg>
                            </button>
                        </div>
                    </div>
                    <FileBrowser initial_path=state.get().left_path />
                </div>

                // Resizable divider
                <div
                    class="w-1.5 bg-[var(--bg-surface)] hover:bg-[var(--accent)] cursor-col-resize flex items-center justify-center transition-colors group"
                    on:mousedown=handle_divider_mouse_down
                >
                    <div class="w-0.5 h-8 bg-[var(--border-default)] group-hover:bg-[var(--accent)] rounded-full transition-colors"></div>
                </div>

                // Right pane
                <div
                    node_ref=right_pane_ref
                    class="flex-1 overflow-hidden"
                    style=move || format!("width: {}%; flex: none;", (1.0 - state.get().pane_ratio) * 100.0)
                    on:click=move |_| switch_active_pane(Pane::Right)
                >
                    <div class="flex items-center justify-between px-3 py-1.5 bg-[var(--bg-surface)] border-b border-[var(--border-default)]">
                        <div class="flex items-center gap-2">
                            <span class="text-xs font-bold uppercase text-[var(--text-tertiary)]">"Right"</span>
                            <span class="text-xs text-[var(--text-tertiary)] font-mono">
                                {move || state.get().right_path}
                            </span>
                        </div>
                        <div class="flex items-center gap-1">
                            <button
                                on:click=move |_| copy_path_to_other(Pane::Left)
                                class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded"
                                title="Copy path to left"
                            >
                                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                                </svg>
                            </button>
                        </div>
                    </div>
                    <FileBrowser initial_path=state.get().right_path />
                </div>
            </div>

            // Bottom toolbar (desktop only)
            <div class="hidden sm:flex items-center justify-between px-4 py-2 bg-[var(--bg-surface)] border-t border-[var(--border-default)]">
                <div class="flex items-center gap-3">
                    <button
                        on:click=swap_panes
                        class="px-3 py-1.5 text-xs font-medium bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)] text-[var(--text-secondary)] rounded hover:bg-[var(--interactive-hover)] transition-colors flex items-center gap-1"
                    >
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4" />
                        </svg>
                        "Swap Panes"
                    </button>
                    <button
                        on:click=toggle_sync_scrolling
                        class=move || format!(
                            "px-3 py-1.5 text-xs font-medium rounded transition-colors flex items-center gap-1 {}",
                            if state.get().sync_scrolling {
                                "bg-[var(--accent)] text-white"
                            } else {
                                "bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)] text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)]"
                            }
                        )
                    >
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                        </svg>
                        "Sync Scroll"
                    </button>
                </div>
                <div class="text-xs text-[var(--text-tertiary)]">
                    {move || if state.get().active_pane == Pane::Left { "Active: Left" } else { "Active: Right" }}
                </div>
            </div>
        </div>
    }
}
