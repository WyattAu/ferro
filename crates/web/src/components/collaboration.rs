use std::cell::RefCell;
use std::rc::Rc;

use leptos::*;
use wasm_bindgen::JsCast;

use crate::t;
use ferro_crdt::document::{CrdtDocument, DocumentId, ParticipantId};
use ferro_crdt::text::TextOperation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabConnectionState {
    Disconnected,
    Connecting,
    Connected,
    ReadOnly,
}

#[derive(Debug, Clone)]
pub struct CollabContext {
    pub document_id: DocumentId,
    pub participant_id: ParticipantId,
    pub participant_name: String,
    pub connection_state: ReadSignal<CollabConnectionState>,
    pub remote_participants: ReadSignal<Vec<ParticipantInfo>>,
    pub version: ReadSignal<u64>,
}

#[derive(Debug, Clone)]
pub struct ParticipantInfo {
    pub id: ParticipantId,
    pub name: String,
}

impl CollabContext {
    pub fn is_read_only(&self) -> bool {
        self.connection_state.get() == CollabConnectionState::ReadOnly
            || self.connection_state.get() == CollabConnectionState::Disconnected
    }
}

#[derive(Clone)]
pub struct CollabStateHandle {
    document: Rc<RefCell<CrdtDocument>>,
    participant_id: ParticipantId,
    pending_ops: Rc<RefCell<Vec<TextOperation>>>,
    set_text: Callback<String>,
    set_version: Callback<u64>,
    set_remote_participants: Callback<Vec<ParticipantInfo>>,
    set_connection_state: Callback<CollabConnectionState>,
    ws: Rc<RefCell<Option<web_sys::WebSocket>>>,
}

impl CollabStateHandle {
    pub fn apply_local_edit(&self, old_text: &str, new_text: &str) {
        let mut doc = self.document.borrow_mut();
        let participant_id = self.participant_id;

        let common_prefix_len = old_text
            .chars()
            .zip(new_text.chars())
            .take_while(|(a, b)| a == b)
            .count();
        let common_suffix_len = old_text
            .chars()
            .rev()
            .zip(new_text.chars().rev())
            .take_while(|(a, b)| a == b)
            .count();

        let delete_start = common_prefix_len;
        let delete_len = old_text.len() - common_prefix_len - common_suffix_len;
        let insert_text: String = new_text
            .chars()
            .skip(common_prefix_len)
            .take(new_text.len() - common_prefix_len - common_suffix_len)
            .collect();

        let mut all_ops = Vec::new();

        if delete_len > 0 {
            let (ops, _) = doc.delete_text(participant_id, delete_start, delete_len);
            all_ops.extend(ops);
        }

        if !insert_text.is_empty() {
            let (ops, _) = doc.insert_text(participant_id, delete_start, &insert_text);
            all_ops.extend(ops);
        }

        let new_version = doc.version;
        drop(doc);

        self.set_version.call(new_version);
        self.set_text.call(self.document.borrow().get_text());

        if !all_ops.is_empty() {
            self.send_ops(&all_ops);
        }
    }

    pub fn apply_remote_ops(&self, ops: &[TextOperation]) {
        let mut doc = self.document.borrow_mut();
        doc.apply_ops(ops);
        let new_version = doc.version;
        let new_text = doc.get_text();
        drop(doc);

        self.set_version.call(new_version);
        self.set_text.call(new_text);
    }

    fn send_ops(&self, ops: &[TextOperation]) {
        if let Some(ref ws) = *self.ws.borrow() {
            if ws.ready_state() == web_sys::WebSocket::OPEN {
                if let Ok(payload) =
                    serde_json::to_string(&SyncMessage::Operations { ops: ops.to_vec() })
                {
                    let _ = ws.send_with_str(&payload);
                }
            } else {
                self.pending_ops.borrow_mut().extend_from_slice(ops);
            }
        } else {
            self.pending_ops.borrow_mut().extend_from_slice(ops);
        }
    }

    pub fn flush_pending(&self) {
        let pending: Vec<TextOperation> = self.pending_ops.borrow_mut().drain(..).collect();
        if !pending.is_empty() {
            self.send_ops(&pending);
        }
    }

    pub fn get_text(&self) -> String {
        self.document.borrow().get_text()
    }

    #[allow(dead_code)]
    pub fn participants(&self) -> Vec<ParticipantInfo> {
        self.document
            .borrow()
            .participants
            .iter()
            .map(|(&id, info)| ParticipantInfo {
                id,
                name: info.name.clone(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SyncMessage {
    Join {
        document_id: String,
        participant_id: u32,
        name: String,
    },
    Operations {
        ops: Vec<TextOperation>,
    },
    State {
        document_id: String,
        version: u64,
    },
    Participants {
        participants: Vec<ParticipantEntry>,
    },
    Hello {
        participant_id: u32,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticipantEntry {
    pub participant_id: u32,
    pub name: String,
}

fn ws_url_for_document(document_id: &str) -> String {
    let location = web_sys::window()
        .expect("must be in browser context")
        .location();
    let protocol = if location.protocol().unwrap_or_default() == "https:" {
        "wss:"
    } else {
        "ws:"
    };
    let host = location.host().unwrap_or_default();
    format!("{protocol}//{host}/ws/collab/{document_id}")
}

#[component]
pub fn CollabEditor(document_id: String, participant_name: String) -> impl IntoView {
    let (text, set_text) = create_signal(String::new());
    let (version, set_version) = create_signal(0u64);
    let (connection_state, set_connection_state) =
        create_signal(CollabConnectionState::Disconnected);
    let (remote_participants, set_remote_participants) =
        create_signal::<Vec<ParticipantInfo>>(vec![]);

    let doc_id = DocumentId(document_id.clone());
    let participant_id = ParticipantId(js_sys::Math::random() as u32 * 1000000 + 1);

    let mut doc = CrdtDocument::new(doc_id.clone());
    doc.join(participant_id, &participant_name);
    let initial_text = doc.get_text();

    let state_handle = CollabStateHandle {
        document: Rc::new(RefCell::new(doc)),
        participant_id,
        pending_ops: Rc::new(RefCell::new(Vec::new())),
        set_text: Callback::new(move |v: String| set_text.set(v)),
        set_version: Callback::new(move |v: u64| set_version.set(v)),
        set_remote_participants: Callback::new(move |v: Vec<ParticipantInfo>| {
            set_remote_participants.set(v)
        }),
        set_connection_state: Callback::new(move |v: CollabConnectionState| {
            set_connection_state.set(v)
        }),
        ws: Rc::new(RefCell::new(None)),
    };

    let handle = state_handle.clone();
    set_text.set(initial_text);

    let ws_url = ws_url_for_document(&document_id);
    let handle_for_ws = handle.clone();
    let document_id_for_ws = document_id.clone();
    let pid_for_ws = participant_id;

    set_connection_state.set(CollabConnectionState::Connecting);

    let ws_result = web_sys::WebSocket::new(&ws_url);
    match ws_result {
        Ok(ws) => {
            {
                let mut ws_cell = handle_for_ws.ws.borrow_mut();
                *ws_cell = Some(ws.clone());
            }

            let handle_onopen = handle_for_ws.clone();
            let ws_for_open = ws.clone();
            let join_msg = SyncMessage::Join {
                document_id: document_id_for_ws,
                participant_id: pid_for_ws.0,
                name: participant_name.clone(),
            };
            let onopen_closure = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
                handle_onopen
                    .set_connection_state
                    .call(CollabConnectionState::Connected);
                if let Ok(payload) = serde_json::to_string(&join_msg) {
                    let _ = ws_for_open.send_with_str(&payload);
                }
                handle_onopen.flush_pending();
            });
            ws.set_onopen(Some(onopen_closure.as_ref().unchecked_ref()));
            onopen_closure.forget();

            let handle_onmessage = handle_for_ws.clone();
            let onmessage_closure =
                wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MessageEvent)>::new(
                    move |ev: web_sys::MessageEvent| {
                        let data_str = ev.data().as_string().unwrap_or_default();
                        if let Ok(msg) = serde_json::from_str::<SyncMessage>(&data_str) {
                            match msg {
                                SyncMessage::Operations { ops } => {
                                    handle_onmessage.apply_remote_ops(&ops);
                                }
                                SyncMessage::Participants { participants } => {
                                    let infos: Vec<ParticipantInfo> = participants
                                        .iter()
                                        .map(|p| ParticipantInfo {
                                            id: ParticipantId(p.participant_id),
                                            name: p.name.clone(),
                                        })
                                        .collect();
                                    handle_onmessage.set_remote_participants.call(infos);
                                }
                                SyncMessage::Hello { .. } => {}
                                SyncMessage::Join { .. } => {}
                                SyncMessage::State { .. } => {}
                            }
                        }
                    },
                );
            ws.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
            onmessage_closure.forget();

            let handle_onerror = handle_for_ws.clone();
            let onerror_closure = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(
                move |_ev: web_sys::Event| {
                    handle_onerror
                        .set_connection_state
                        .call(CollabConnectionState::ReadOnly);
                },
            );
            ws.set_onerror(Some(onerror_closure.as_ref().unchecked_ref()));
            onerror_closure.forget();

            let handle_onclose = handle_for_ws.clone();
            let onclose_closure =
                wasm_bindgen::closure::Closure::<dyn Fn(web_sys::CloseEvent)>::new(
                    move |_ev: web_sys::CloseEvent| {
                        handle_onclose
                            .set_connection_state
                            .call(CollabConnectionState::Disconnected);
                    },
                );
            ws.set_onclose(Some(onclose_closure.as_ref().unchecked_ref()));
            onclose_closure.forget();
        }
        Err(_) => {
            set_connection_state.set(CollabConnectionState::ReadOnly);
        }
    }

    let context = CollabContext {
        document_id: doc_id,
        participant_id,
        participant_name,
        connection_state,
        remote_participants,
        version,
    };
    provide_context(context);

    let is_read_only = move || {
        matches!(
            connection_state.get(),
            CollabConnectionState::ReadOnly | CollabConnectionState::Disconnected
        )
    };

    let handle_for_input = handle.clone();
    let on_input = move |ev: web_sys::Event| {
        let new_value = event_target_value(&ev);
        let old_value = handle_for_input.get_text();
        if old_value != new_value {
            handle_for_input.apply_local_edit(&old_value, &new_value);
        }
    };

    let handle_for_keydown = handle.clone();
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Tab" {
            ev.prevent_default();
            let target: Option<web_sys::HtmlTextAreaElement> = ev
                .target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok());
            if let Some(textarea) = target {
                let start = textarea.selection_start().unwrap_or_default().unwrap_or(0);
                let end = textarea.selection_end().unwrap_or_default().unwrap_or(0);
                let val = textarea.value();
                let new_val = format!("{}    {}", &val[..start as usize], &val[end as usize..]);
                textarea.set_value(&new_val);
                let _ = textarea.set_selection_start(Some(start + 4));
                let _ = textarea.set_selection_end(Some(start + 4));

                let old_value = handle_for_keydown.get_text();
                handle_for_keydown.apply_local_edit(&old_value, &new_val);
            }
        }
    };

    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center justify-between px-4 py-2 border-b bg-gray-50 dark:bg-gray-800">
                <div class="flex items-center gap-2">
                    <ConnectionBadge state=connection_state />
                    <span class="text-xs font-mono text-gray-500">
                        {move || format!("v{}", version.get())}
                    </span>
                </div>
                <PresenceIndicator />
            </div>
            <textarea
                class="flex-1 w-full p-4 font-mono text-sm resize-none focus:outline-none bg-white dark:bg-gray-900 dark:text-gray-100"
                prop:value=text
                on:input=on_input
                on:keydown=on_keydown
                prop:disabled=is_read_only
                placeholder=if is_read_only() {
                    t!("collab.read_only_placeholder").to_string()
                } else {
                    t!("collab.editor_placeholder").to_string()
                }
                spellcheck="false"
            />
        </div>
    }
}

#[component]
fn ConnectionBadge(state: ReadSignal<CollabConnectionState>) -> impl IntoView {
    let label = move || match state.get() {
        CollabConnectionState::Connected => "Connected",
        CollabConnectionState::Connecting => "Connecting...",
        CollabConnectionState::ReadOnly => "Read-only",
        CollabConnectionState::Disconnected => "Offline",
    };

    let color_class = move || match state.get() {
        CollabConnectionState::Connected => {
            "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
        }
        CollabConnectionState::Connecting => {
            "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200"
        }
        CollabConnectionState::ReadOnly => {
            "bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200"
        }
        CollabConnectionState::Disconnected => {
            "bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200"
        }
    };

    view! {
        <span class=move || format!("px-2 py-0.5 text-xs font-mono font-medium rounded {}", color_class())>
            {label}
        </span>
    }
}

#[component]
pub fn PresenceIndicator() -> impl IntoView {
    let ctx = use_context::<CollabContext>();

    view! {
        {move || {
            match ctx {
                Some(ref context) => {
                    let participants = context.remote_participants.get();
                    let total = participants.len() + 1;
                    let user_initial = context
                        .participant_name
                        .chars()
                        .next()
                        .map(|c| c.to_uppercase().to_string())
                        .unwrap_or_else(|| "?".to_string());
                    let user_name = context.participant_name.clone();
                    view! {
                        <div class="flex items-center gap-1">
                            <div class="flex -space-x-1">
                                <div
                                    class="w-6 h-6 rounded-full bg-blue-500 border-2 border-white dark:border-gray-800 flex items-center justify-center"
                                    title=user_name
                                >
                                    <span class="text-[10px] font-bold text-white">
                                        {user_initial}
                                    </span>
                                </div>
                                {participants.into_iter().map(|p| {
                                    let initial = p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".to_string());
                                    let name = p.name.clone();
                                    view! {
                                        <div
                                            class="w-6 h-6 rounded-full bg-gray-400 border-2 border-white dark:border-gray-800 flex items-center justify-center"
                                            title=name
                                        >
                                            <span class="text-[10px] font-bold text-white">{initial}</span>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                            <span class="text-xs font-mono text-gray-500 ml-1">
                                {move || format!("{}", total)}
                            </span>
                        </div>
                    }.into_any()
                }
                None => view! { <div></div> }.into_any(),
            }
        }}
    }
}

#[component]
pub fn CollabAwarenessBar() -> impl IntoView {
    let ctx = use_context::<CollabContext>();

    view! {
        <div class="flex items-center gap-3 px-4 py-2 text-xs font-mono border-t bg-gray-50 dark:bg-gray-800 dark:border-gray-700">
            <PresenceIndicator />
            {move || {
                match ctx {
                    Some(ref context) => {
                        let ver = context.version.get();
                        let read_only = matches!(
                            context.connection_state.get(),
                            CollabConnectionState::ReadOnly | CollabConnectionState::Disconnected
                        );
                        view! {
                            <div class="flex items-center gap-3">
                                <span class="text-gray-500">
                                    {format!("Document v{}", ver)}
                                </span>
                                {if read_only {
                                    view! {
                                        <span class="text-orange-600 dark:text-orange-400">
                                            {t!("collab.server_unavailable")}
                                        </span>
                                    }.into_any()
                                } else {
                                    view! { <span class="hidden"></span> }.into_any()
                                }}
                            </div>
                        }.into_any()
                    }
                    None => view! { <span class="hidden"></span> }.into_any(),
                }
            }}
        </div>
    }
}
