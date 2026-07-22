use leptos::prelude::*;

#[derive(Clone, Debug)]
struct ChatRoom {
    id: String,
    name: String,
    last_message: String,
    unread: u32,
}

#[derive(Clone, Debug)]
struct ChatMessage {
    id: String,
    sender: String,
    content: String,
    timestamp: String,
}

/// Chat page with rooms and messages.
#[component]
pub fn ChatPage() -> impl IntoView {
    let (rooms, set_rooms) = signal(Vec::<ChatRoom>::new());
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (selected_room, set_selected_room) = signal(None::<String>);
    let (new_message, set_new_message) = signal(String::new());
    let (loading, set_loading) = signal(true);

    Effect::new(move |_| {
        set_loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            let set_r = set_rooms;
            let set_l = set_loading;
            wasm_bindgen_futures::spawn_local(async move {
                let client = crate::api::ApiClient::new(crate::api::ApiClientConfig::default());
                match client.get::<serde_json::Value>("/api/v1/chat/rooms").await {
                    Ok(val) => {
                        if let Some(arr) = val.as_array() {
                            let items: Vec<ChatRoom> = arr.iter().filter_map(|v| {
                                Some(ChatRoom {
                                    id: v["id"].as_str()?.to_string(),
                                    name: v["name"].as_str().unwrap_or("Room").to_string(),
                                    last_message: v["last_message"].as_str().unwrap_or("").to_string(),
                                    unread: v["unread"].as_u64().unwrap_or(0) as u32,
                                })
                            }).collect();
                            set_r.set(items);
                        }
                        set_l.set(false);
                    }
                    Err(e) => { log::error!("Chat rooms load failed: {}", e); set_l.set(false); }
                }
            });
        }
    });

    view! {
        <div class="flex h-full">
            <aside class="w-72 border-r border-[var(--color-border)] overflow-y-auto flex-shrink-0">
                <div class="p-3 border-b border-[var(--color-border)]">
                    <h2 class="font-semibold">"Chat Rooms"</h2>
                </div>
                <nav class="px-2 py-2">
                    {move || rooms.get().into_iter().map(|room| {
                        let id = room.id.clone();
                        let name = room.name.clone();
                        let last = room.last_message.clone();
                        let unread = room.unread;
                        let id2 = id.clone();
                        let sel = move || selected_room.get() == Some(id.clone());
                        view! {
                            <button class=move || format!("w-full text-left px-3 py-2 rounded-md text-sm {}", if sel() { "bg-accent-subtle text-accent" } else { "hover:bg-sunken" })
                                on:click=move |_| set_selected_room.set(Some(id2.clone()))>
                                <div class="font-medium">{name}</div>
                                <div class="text-xs text-secondary truncate">{last}</div>
                                {if unread > 0 { view! { <span class="badge badge-accent ml-auto">{unread}</span> }.into_any() } else { view! { <></> }.into_any() }}
                            </button>
                        }
                    }).collect_view()}
                </nav>
            </aside>
            <main class="flex-1 flex flex-col">
                {move || {
                    match selected_room.get() {
                        Some(room_id) => {
                            let rid = room_id.clone();
                            view! {
                                <div class="flex-1 overflow-y-auto p-4 space-y-3">
                                    {move || messages.get().into_iter().filter(|m| m.sender != "system").map(|msg| {
                                        view! {
                                            <div class="flex gap-3">
                                                <div class="w-8 h-8 rounded-full bg-accent-subtle text-accent flex items-center justify-center text-xs font-bold shrink-0">
                                                    {msg.sender.chars().next().unwrap_or('?')}
                                                </div>
                                                <div>
                                                    <div class="flex items-baseline gap-2">
                                                        <span class="font-medium text-sm">{msg.sender}</span>
                                                        <span class="text-xs text-tertiary">{msg.timestamp}</span>
                                                    </div>
                                                    <p class="text-sm mt-0.5">{msg.content}</p>
                                                </div>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                                <div class="border-t border-[var(--color-border)] p-3 flex gap-2">
                                    <input class="input flex-1" type="text" placeholder="Type a message..." prop:value=move || new_message.get() />
                                    <button class="btn btn-primary">"Send"</button>
                                </div>
                            }.into_any()
                        }
                        None => view! { <div class="flex-1 flex items-center justify-center text-secondary"><p>"Select a chat room"</p></div> }.into_any(),
                    }
                }}
            </main>
        </div>
    }
}
