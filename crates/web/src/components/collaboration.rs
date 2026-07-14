use std::cell::{Cell, RefCell};
use std::rc::Rc;

use leptos::prelude::*;
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
    pub document: Rc<RefCell<CrdtDocument>>,
    pub participant_id: ParticipantId,
    pub participant_name: String,
    pub pending_ops: Rc<RefCell<Vec<TextOperation>>>,
    pub set_text: Callback<String>,
    pub set_version: Callback<u64>,
    pub set_remote_participants: Callback<Vec<ParticipantInfo>>,
    pub set_connection_state: Callback<CollabConnectionState>,
    pub ws: Rc<RefCell<Option<web_sys::WebSocket>>>,
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

        self.set_version.run(new_version);
        self.set_text.run(self.document.borrow().get_text());

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

        self.set_version.run(new_version);
        self.set_text.run(new_text);
    }

    fn send_ops(&self, ops: &[TextOperation]) {
        if let Some(ref ws) = *self.ws.borrow() {
            if ws.ready_state() == web_sys::WebSocket::OPEN {
                if let Ok(payload) = serde_json::to_string(&SyncMessage::Operations { ops: ops.to_vec() }) {
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
#[serde(tag = "type", rename_all = "snake_case")]
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
    DocumentState {
        document_id: String,
        serialized_state: String,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticipantEntry {
    pub participant_id: u32,
    pub name: String,
}

fn ws_url_for_document(document_id: &str) -> String {
    let location = web_sys::window().expect("must be in browser context").location();
    let protocol = if location.protocol().unwrap_or_default() == "https:" {
        "wss:"
    } else {
        "ws:"
    };
    let host = location.host().unwrap_or_default();
    format!("{protocol}//{host}/ws/collab/{document_id}")
}

struct ReconnectData {
    handle: CollabStateHandle,
    ws_url: String,
    document_id: String,
    participant_id: ParticipantId,
    participant_name: String,
    backoff_ms: Cell<u32>,
}

fn setup_websocket(data: &Rc<ReconnectData>) {
    match web_sys::WebSocket::new(&data.ws_url) {
        Ok(ws) => {
            *data.handle.ws.borrow_mut() = Some(ws.clone());
            data.handle.set_connection_state.run(CollabConnectionState::Connecting);

            let d = data.clone();
            let ws_for_open = ws.clone();
            let onopen_closure = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
                d.handle.set_connection_state.run(CollabConnectionState::Connected);
                d.backoff_ms.set(1000);

                let join_msg = SyncMessage::Join {
                    document_id: d.document_id.clone(),
                    participant_id: d.participant_id.0,
                    name: d.participant_name.clone(),
                };
                if let Ok(payload) = serde_json::to_string(&join_msg) {
                    let _ = ws_for_open.send_with_str(&payload);
                }
                d.handle.flush_pending();
            });
            ws.set_onopen(Some(onopen_closure.as_ref().unchecked_ref()));
            onopen_closure.forget();

            let d = data.clone();
            let onmessage_closure = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MessageEvent)>::new(
                move |ev: web_sys::MessageEvent| {
                    let data_str = ev.data().as_string().unwrap_or_default();
                    if let Ok(msg) = serde_json::from_str::<SyncMessage>(&data_str) {
                        match msg {
                            SyncMessage::Operations { ops } => {
                                let my_site = d.handle.participant_id.0;
                                let remote_ops: Vec<_> = ops
                                    .into_iter()
                                    .filter(|op| match op {
                                        TextOperation::Insert { id, .. } => id.site_id != my_site,
                                        TextOperation::Delete { id, .. } => id.site_id != my_site,
                                    })
                                    .collect();
                                if !remote_ops.is_empty() {
                                    d.handle.apply_remote_ops(&remote_ops);
                                }
                            }
                            SyncMessage::DocumentState { serialized_state, .. } => {
                                if let Ok(server_doc) = serde_json::from_str::<CrdtDocument>(&serialized_state) {
                                    let mut local_doc = d.handle.document.borrow_mut();
                                    *local_doc = server_doc;
                                    local_doc.join(d.handle.participant_id, &d.handle.participant_name);
                                    let text = local_doc.get_text();
                                    let version = local_doc.version;
                                    drop(local_doc);
                                    d.handle.set_text.run(text);
                                    d.handle.set_version.run(version);
                                }
                            }
                            SyncMessage::Participants { participants } => {
                                let infos: Vec<ParticipantInfo> = participants
                                    .iter()
                                    .map(|p| ParticipantInfo {
                                        id: ParticipantId(p.participant_id),
                                        name: p.name.clone(),
                                    })
                                    .collect();
                                d.handle.set_remote_participants.run(infos);
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

            let d = data.clone();
            let onerror_closure =
                wasm_bindgen::closure::Closure::<dyn Fn(web_sys::Event)>::new(move |_ev: web_sys::Event| {
                    d.handle.set_connection_state.run(CollabConnectionState::Disconnected);
                    schedule_reconnect(&d);
                });
            ws.set_onerror(Some(onerror_closure.as_ref().unchecked_ref()));
            onerror_closure.forget();

            let d = data.clone();
            let onclose_closure =
                wasm_bindgen::closure::Closure::<dyn Fn(web_sys::CloseEvent)>::new(move |_ev: web_sys::CloseEvent| {
                    d.handle.set_connection_state.run(CollabConnectionState::Disconnected);
                    schedule_reconnect(&d);
                });
            ws.set_onclose(Some(onclose_closure.as_ref().unchecked_ref()));
            onclose_closure.forget();
        }
        Err(_) => {
            schedule_reconnect(data);
        }
    }
}

fn schedule_reconnect(data: &Rc<ReconnectData>) {
    let delay = data.backoff_ms.get();
    data.backoff_ms.set((delay * 2).min(30000));

    let d = data.clone();
    let timer_closure = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
        setup_websocket(&d);
    });

    let _ = web_sys::window()
        .expect("window")
        .set_timeout_with_callback_and_timeout_and_arguments_0(timer_closure.as_ref().unchecked_ref(), delay as i32);
    timer_closure.forget();
}

#[component]
pub fn CollabEditor(document_id: String, participant_name: String) -> impl IntoView {
    let (text, set_text) = signal(String::new());
    let (version, set_version) = signal(0u64);
    let (connection_state, set_connection_state) = signal(CollabConnectionState::Disconnected);
    let (remote_participants, set_remote_participants) = signal::<Vec<ParticipantInfo>>(vec![]);

    let doc_id = DocumentId(document_id.clone());
    let participant_id = ParticipantId(js_sys::Math::random() as u32 * 1000000 + 1);

    let mut doc = CrdtDocument::new(doc_id.clone());
    doc.join(participant_id, &participant_name);
    let initial_text = doc.get_text();

    let state_handle = CollabStateHandle {
        document: Rc::new(RefCell::new(doc)),
        participant_id,
        participant_name: participant_name.clone(),
        pending_ops: Rc::new(RefCell::new(Vec::new())),
        set_text: Callback::new(move |v: String| set_text.set(v)),
        set_version: Callback::new(move |v: u64| set_version.set(v)),
        set_remote_participants: Callback::new(move |v: Vec<ParticipantInfo>| set_remote_participants.set(v)),
        set_connection_state: Callback::new(move |v: CollabConnectionState| set_connection_state.set(v)),
        ws: Rc::new(RefCell::new(None)),
    };

    let handle = state_handle.clone();
    set_text.set(initial_text);

    let ws_url = ws_url_for_document(&document_id);

    let reconnect_data = Rc::new(ReconnectData {
        handle: state_handle.clone(),
        ws_url,
        document_id: document_id.clone(),
        participant_id,
        participant_name: participant_name.clone(),
        backoff_ms: Cell::new(1000),
    });

    setup_websocket(&reconnect_data);

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
            <div class="flex items-center justify-between px-4 py-2 border-b bg-[var(--bg-base)]">
                <div class="flex items-center gap-2">
                    <ConnectionBadge state=connection_state />
                    <span class="text-xs font-mono text-[var(--text-tertiary)]">
                        {move || format!("v{}", version.get())}
                    </span>
                </div>
                <PresenceIndicator />
            </div>
            <textarea
                class="flex-1 w-full p-4 font-mono text-sm resize-none focus:outline-none bg-[var(--bg-surface)] dark:text-gray-100"
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
        CollabConnectionState::Connected => "bg-[var(--success-subtle)] text-[var(--success)] dark:bg-green-900 dark:text-[var(--success)]",
        CollabConnectionState::Connecting => "bg-[var(--warning-subtle)] text-[var(--warning)] bg-[var(--warning-subtle)] text-[var(--warning)]",
        CollabConnectionState::ReadOnly => "bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200",
        CollabConnectionState::Disconnected => "bg-[var(--bg-inset)] text-[var(--text-primary)] bg-[var(--bg-surface-raised)] dark:text-gray-200",
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
                                    class="w-6 h-6 rounded-full bg-[var(--accent)] border-2 border-white dark:border-[var(--border-strong)] flex items-center justify-center"
                                    title=user_name
                                >
                                    <span class="text-[10px] font-bold text-[var(--text-on-accent)]">
                                        {user_initial}
                                    </span>
                                </div>
                                {participants.into_iter().map(|p| {
                                    let initial = p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".to_string());
                                    let name = p.name.clone();
                                    view! {
                                        <div
                                            class="w-6 h-6 rounded-full bg-[var(--text-tertiary)] border-2 border-white dark:border-[var(--border-strong)] flex items-center justify-center"
                                            title=name
                                        >
                                            <span class="text-[10px] font-bold text-[var(--text-on-accent)]">{initial}</span>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                            <span class="text-xs font-mono text-[var(--text-tertiary)] ml-1">
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
        <div class="flex items-center gap-3 px-4 py-2 text-xs font-mono border-t bg-[var(--bg-base)] border-[var(--border-default)]">
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
                                <span class="text-[var(--text-tertiary)]">
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
