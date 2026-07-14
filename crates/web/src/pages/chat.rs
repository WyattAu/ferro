use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;
use crate::t;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatRoom {
    pub id: String,
    pub name: String,
    pub room_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub room_id: String,
    pub user_id: String,
    pub content: String,
    pub timestamp: String,
    pub reply_to: Option<String>,
    pub attachment_path: Option<String>,
}

#[component]
pub fn ChatPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (loading, set_loading) = signal(true);
    let (rooms, set_rooms) = signal(Vec::<ChatRoom>::new());
    let (selected_room_id, set_selected_room_id) = signal(None::<String>);
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (new_message, set_new_message) = signal(String::new());
    let (typing_users, _set_typing_users) = signal(Vec::<String>::new());
    let (show_create_room, set_show_create_room) = signal(false);
    let (new_room_name, set_new_room_name) = signal(String::new());
    let (_error_msg, set_error) = signal(String::new());
    let (ws_connected, _set_ws_connected) = signal(false);

    let fetch_rooms = move || {
        spawn_local(async move {
            match api::fetch_json("/api/chat/rooms").await {
                Ok(val) => {
                    let rooms_list = val
                        .get("rooms")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    Some(ChatRoom {
                                        id: v.get("id")?.as_str()?.to_string(),
                                        name: v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                                        room_type: v
                                            .get("room_type")
                                            .and_then(|t| t.as_str())
                                            .unwrap_or("global")
                                            .to_string(),
                                        created_at: v
                                            .get("created_at")
                                            .and_then(|c| c.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    set_rooms.set(rooms_list);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(e);
                    set_loading.set(false);
                }
            }
        });
    };

    let fetch_messages = move |room_id: String| {
        spawn_local(async move {
            match api::fetch_json(&format!("/api/chat/rooms/{}/messages?limit=50", room_id)).await {
                Ok(val) => {
                    let msgs = val
                        .get("messages")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    Some(ChatMessage {
                                        id: v.get("id")?.as_str()?.to_string(),
                                        room_id: v.get("room_id").and_then(|r| r.as_str()).unwrap_or("").to_string(),
                                        user_id: v.get("user_id").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                                        content: v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                                        timestamp: v
                                            .get("timestamp")
                                            .and_then(|t| t.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        reply_to: v.get("reply_to").and_then(|r| r.as_str()).map(|s| s.to_string()),
                                        attachment_path: v
                                            .get("attachment_path")
                                            .and_then(|a| a.as_str())
                                            .map(|s| s.to_string()),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    set_messages.set(msgs);
                }
                Err(e) => {
                    set_error.set(e);
                }
            }
        });
    };

    let select_room = move |room_id: String| {
        set_selected_room_id.set(Some(room_id.clone()));
        fetch_messages(room_id);
    };

    let send_message = move |_: ev::MouseEvent| {
        let content = new_message.get();
        if content.trim().is_empty() {
            return;
        }
        if let Some(room_id) = selected_room_id.get() {
            let room_id_clone = room_id.clone();
            let content_clone = content.clone();
            set_new_message.set(String::new());

            spawn_local(async move {
                let body = serde_json::json!({
                    "content": content_clone,
                });
                match api::fetch_json_with_method(
                    &format!("/api/chat/rooms/{}/messages", room_id_clone),
                    "POST",
                    Some(&body.to_string()),
                )
                .await
                {
                    Ok(_) => {
                        fetch_messages(room_id_clone);
                    }
                    Err(e) => {
                        set_error.set(e);
                    }
                }
            });
        }
    };

    let handle_keydown = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            let content = new_message.get();
            if content.trim().is_empty() {
                return;
            }
            if let Some(room_id) = selected_room_id.get() {
                let room_id_clone = room_id.clone();
                let content_clone = content.clone();
                set_new_message.set(String::new());

                spawn_local(async move {
                    let body = serde_json::json!({
                        "content": content_clone,
                    });
                    match api::fetch_json_with_method(
                        &format!("/api/chat/rooms/{}/messages", room_id_clone),
                        "POST",
                        Some(&body.to_string()),
                    )
                    .await
                    {
                        Ok(_) => {
                            fetch_messages(room_id_clone);
                        }
                        Err(e) => {
                            set_error.set(e);
                        }
                    }
                });
            }
        }
    };

    let create_room = move |_: ev::MouseEvent| {
        let name = new_room_name.get();
        if name.trim().is_empty() {
            return;
        }
        set_show_create_room.set(false);
        spawn_local(async move {
            let body = serde_json::json!({
                "name": name,
                "room_type": "global",
            });
            match api::fetch_json_with_method("/api/chat/rooms", "POST", Some(&body.to_string())).await {
                Ok(_) => {
                    fetch_rooms();
                }
                Err(e) => {
                    set_error.set(e);
                }
            }
        });
    };

    fetch_rooms();

    let highlight_mentions = move |content: String| {
        let mut result = String::new();
        let mut remaining = content.as_str();
        while let Some(at_pos) = remaining.find('@') {
            let before = &remaining[..at_pos];
            result.push_str(before);
            let after_at = &remaining[at_pos + 1..];
            let word_end = after_at
                .find(|c: char| c.is_whitespace() || c == ',' || c == '.')
                .unwrap_or(after_at.len());
            let username = &after_at[..word_end];
            result.push_str(&format!(
                "<span class=\"text-blue-500 font-semibold\">@{}</span>",
                username
            ));
            remaining = &after_at[word_end..];
        }
        result.push_str(remaining);
        result
    };

    view! {
        <div class="h-screen flex flex-col bg-gray-100 dark:bg-gray-900">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 overflow-hidden px-2 sm:px-4 pt-16">
                <main id="main-content" class="h-full flex">
                    // Sidebar - Room list
                    <div class="w-72 flex-shrink-0 flex flex-col border-r border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
                        <div class="p-3 border-b border-gray-200 dark:border-gray-700">
                            <div class="flex items-center justify-between mb-3">
                                <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">"Chat"</h2>
                                <button
                                    on:click=move |_: ev::MouseEvent| {
                                        set_new_room_name.set(String::new());
                                        set_show_create_room.set(true);
                                    }
                                    class="p-1.5 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                                    title="New Room"
                                >
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
                                </button>
                            </div>
                            <div class="flex items-center gap-2 text-xs text-gray-500">
                                <div class=move || format!("w-2 h-2 rounded-full {}", if ws_connected.get() { "bg-green-500" } else { "bg-gray-400" })></div>
                                <span>{move || if ws_connected.get() { "Connected" } else { "Disconnected" }}</span>
                            </div>
                        </div>

                        <div class="flex-1 overflow-y-auto">
                            {move || loading.get().then(|| view! {
                                <div class="flex items-center justify-center py-8">
                                    <div class="text-sm text-gray-500 font-mono">{t!("common.loading")}</div>
                                </div>
                            })}

                            <For
                                each=move || rooms.get()
                                key=|r| r.id.clone()
                                let:room
                            >
                                {
                                    let room_id = room.id.clone();
                                    let is_selected = move || selected_room_id.get() == Some(room_id.clone());
                                    let room_clone = room.clone();
                                    view! {
                                        <div
                                            class=move || format!("px-3 py-2 cursor-pointer border-b border-gray-100 dark:border-gray-700/50 transition-colors {}",
                                                if is_selected() { "bg-blue-50 dark:bg-blue-900/20" } else { "hover:bg-gray-50 dark:hover:bg-gray-700/50" }
                                            )
                                            on:click=move |_: ev::MouseEvent| select_room(room_clone.id.clone())
                                        >
                                            <div class="flex items-center gap-2">
                                                <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" /></svg>
                                                <div class="text-sm font-medium text-gray-900 dark:text-white truncate">{room.name}</div>
                                            </div>
                                            <div class="text-xs text-gray-400 mt-1">{room.room_type}</div>
                                        </div>
                                    }
                                }
                            </For>
                        </div>
                    </div>

                    // Main content area - Messages
                    <div class="flex-1 flex flex-col overflow-hidden">
                        {move || {
                            if let Some(room_id) = selected_room_id.get() {
                                let room_name = rooms.get().iter().find(|r| r.id == room_id).map(|r| r.name.clone()).unwrap_or_default();
                                view! {
                                    <div class="flex-1 flex flex-col overflow-hidden">
                                        // Room header
                                        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
                                            <div class="flex items-center gap-3">
                                                <svg class="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" /></svg>
                                                <h3 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{room_name}</h3>
                                            </div>
                                        </div>

                                        // Messages list
                                        <div class="flex-1 overflow-y-auto px-6 py-4 space-y-4">
                                            <For
                                                each=move || messages.get()
                                                key=|m| m.id.clone()
                                                let:msg
                                            >
                                                {
                                                    view! {
                                                        <div class="flex gap-3">
                                                            <div class="w-8 h-8 rounded-full bg-blue-500 flex items-center justify-center text-white text-sm font-bold flex-shrink-0">
                                                                {msg.user_id.chars().next().unwrap_or('U').to_uppercase().collect::<String>()}
                                                            </div>
                                                            <div class="flex-1 min-w-0">
                                                                <div class="flex items-baseline gap-2">
                                                                    <span class="text-sm font-medium text-gray-900 dark:text-white">{msg.user_id.clone()}</span>
                                                                    <span class="text-xs text-gray-400">{msg.timestamp[..19.min(msg.timestamp.len())].to_string()}</span>
                                                                </div>
                                                                {if let Some(ref reply_to) = msg.reply_to {
                                                                    view! {
                                                                        <div class="text-xs text-gray-400 mb-1">"Replying to "{reply_to.clone()}</div>
                                                                    }.into_any()
                                                                } else {
                                                                    ().into_any()
                                                                }}
                                                                <div class="text-sm text-gray-700 dark:text-gray-300 mt-0.5" inner_html=highlight_mentions(msg.content.clone())></div>
                                                                {if let Some(ref attachment) = msg.attachment_path {
                                                                    let attachment_clone = attachment.clone();
                                                                    view! {
                                                                        <div class="mt-2 p-2 bg-gray-50 dark:bg-gray-700 rounded-lg border border-gray-200 dark:border-gray-600 inline-flex items-center gap-2">
                                                                            <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13" /></svg>
                                                                            <span class="text-xs text-gray-600 dark:text-gray-400">{attachment_clone}</span>
                                                                        </div>
                                                                    }.into_any()
                                                                } else {
                                                                    ().into_any()
                                                                }}
                                                            </div>
                                                        </div>
                                                    }
                                                }
                                            </For>
                                        </div>

                                        // Typing indicator
                                        {move || {
                                            let typing = typing_users.get();
                                            if !typing.is_empty() {
                                                view! {
                                                    <div class="px-6 py-1 text-xs text-gray-400">
                                                        {typing.join(", ")}

                                                        {if typing.len() == 1 { " is typing..." } else { " are typing..." }}
                                                    </div>
                                                }.into_any()
                                            } else {
                                                ().into_any()
                                            }
                                        }}

                                        // Message input
                                        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
                                            <div class="flex gap-3">
                                                <input
                                                    type="text"
                                                    prop:value=move || new_message.get()
                                                    on:input=move |ev| set_new_message.set(event_target_value(&ev))
                                                    on:keydown=handle_keydown
                                                    class="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                                    placeholder="Type a message... (use @ to mention)"
                                                />
                                                <button
                                                    on:click=send_message
                                                    class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium"
                                                >
                                                    "Send"
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="flex-1 flex items-center justify-center bg-white dark:bg-gray-800">
                                        <div class="text-center">
                                            <svg class="w-16 h-16 mx-auto text-gray-300 dark:text-gray-600 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" /></svg>
                                            <p class="text-gray-500">Select a chat room</p>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                </main>
            </div>

            // Create room dialog
            {move || show_create_room.get().then(|| view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_create_room.set(false)>
                    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                        <h3 class="text-lg font-bold font-mono text-gray-900 dark:text-white mb-4">"New Chat Room"</h3>
                        <div class="space-y-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Room Name"</label>
                                <input
                                    type="text"
                                    prop:value=move || new_room_name.get()
                                    on:input=move |ev| set_new_room_name.set(event_target_value(&ev))
                                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                    placeholder="Enter room name"
                                />
                            </div>
                        </div>
                        <div class="flex items-center justify-end gap-3 mt-6">
                            <button
                                on:click=move |_: ev::MouseEvent| set_show_create_room.set(false)
                                class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                            >
                                {t!("common.cancel")}
                            </button>
                            <button
                                on:click=create_room
                                class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
                            >
                                {t!("common.save")}
                            </button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}
