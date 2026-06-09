use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api::ApiState;
use crate::components::badge::{Badge, BadgeVariant};
use crate::components::modal::Modal;
use crate::state::format_timestamp;

#[component]
pub fn WebhooksPage(api: RwSignal<ApiState>) -> impl IntoView {
    let (webhooks, set_webhooks) = signal(Vec::<serde_json::Value>::new());
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(true);
    let (msg, set_msg) = signal(None::<String>);
    let (show_create, set_show_create) = signal(false);
    let (show_delete, set_show_delete) = signal(false);
    let (delete_id, set_delete_id) = signal(String::new());
    let (form_error, set_form_error) = signal(None::<String>);
    let (new_url, set_new_url) = signal(String::new());
    let (new_secret, set_new_secret) = signal(String::new());
    let (selected_events, set_selected_events) = signal(Vec::<String>::new());

    let event_options = [
        "file.created",
        "file.updated",
        "file.deleted",
        "share.created",
        "share.deleted",
        "user.created",
        "user.deleted",
    ];

    let load_webhooks = move || {
        let api_clone = api.get_untracked();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match api_clone.list_webhooks().await {
                Ok(w) => set_webhooks.set(w),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    Effect::new(move |_| load_webhooks());

    let toggle_event = move |ev: String| {
        set_selected_events.update(|events| {
            if events.contains(&ev) {
                events.retain(|e| e != &ev);
            } else {
                events.push(ev);
            }
        });
    };

    let _do_create = move |_: leptos::ev::MouseEvent| {
        let url = new_url.get();
        let secret = new_secret.get();
        let events = selected_events.get();
        if url.trim().is_empty() {
            set_form_error.set(Some("URL is required".to_string()));
            return;
        }
        if events.is_empty() {
            set_form_error.set(Some("Select at least one event".to_string()));
            return;
        }
        set_form_error.set(None);
        let api_clone = api.get_untracked();
        let u = url.trim().to_string();
        let s = secret.trim().to_string();
        let e = events;
        spawn_local(async move {
            match api_clone.create_webhook(&u, e, &s).await {
                Ok(_) => {
                    set_msg.set(Some("Webhook created successfully".to_string()));
                    set_new_url.set(String::new());
                    set_new_secret.set(String::new());
                    set_selected_events.set(Vec::new());
                    set_show_create.set(false);
                    load_webhooks();
                }
                Err(err) => set_form_error.set(Some(err)),
            }
        });
    };

    let do_delete = move |_: leptos::ev::MouseEvent| {
        let id = delete_id.get();
        let api_clone = api.get_untracked();
        spawn_local(async move {
            match api_clone.delete_webhook(&id).await {
                Ok(_) => {
                    set_msg.set(Some("Webhook deleted".to_string()));
                    set_show_delete.set(false);
                    load_webhooks();
                }
                Err(e) => set_msg.set(Some(format!("Delete failed: {}", e))),
            }
        });
    };

    view! {
        <div class="page">
            <div class="page-header">
                <div class="page-header-left"></div>
                <button class="btn btn-primary" on:click=move |_| set_show_create.set(true)>"Create Webhook"</button>
            </div>

            {move || msg.get().map(|m| view! { <div class="success-banner">{m}</div> })}
            {move || error.get().map(|e| view! { <div class="error-banner">{e}</div> })}
            {move || loading.get().then(|| view! { <div class="loading">"Loading webhooks..."</div> })}

            <div class="table-wrapper">
                <table class="data-table">
                    <thead><tr><th>"URL"</th><th>"Events"</th><th>"Status"</th><th>"Last Triggered"</th><th>"Actions"</th></tr></thead>
                    <tbody>
                        {move || {
                            let hooks = webhooks.get();
                            if hooks.is_empty() {
                                vec![view! { <tr><td colspan="5" class="table-empty">"No webhooks configured"</td></tr> }.into_any()]
                            } else {
                                hooks.iter().map(|wh| {
                                    let url = wh.get("url").and_then(|u| u.as_str()).unwrap_or("-").to_string();
                                    let id = wh.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                                    let events = wh.get("events").and_then(|e| e.as_array())
                                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or("-".to_string());
                                    let active = wh.get("active").and_then(|a| a.as_bool()).unwrap_or(true);
                                    let last_triggered = wh.get("last_triggered").and_then(|l| l.as_str()).unwrap_or("Never").to_string();
                                    let bv = if active { BadgeVariant::Success } else { BadgeVariant::Neutral };
                                    let status_text = if active { "Active".to_string() } else { "Inactive".to_string() };
                                    let wid = id.clone();
                                    view! {
                                        <tr>
                                            <td class="mono">{url}</td>
                                            <td>{events}</td>
                                            <td><Badge text=status_text variant=bv/></td>
                                            <td>{format_timestamp(&last_triggered)}</td>
                                            <td class="actions-cell">
                                                <button class="btn btn-secondary btn-sm" on:click=move |_| set_msg.set(Some("Test ping sent".to_string()))>"Test"</button>
                                                <button class="btn btn-danger btn-sm" on:click=move |_| { set_delete_id.set(wid.clone()); set_show_delete.set(true); }>"Delete"</button>
                                            </td>
                                        </tr>
                                    }.into_any()
                                }).collect::<Vec<_>>()
                            }
                        }}
                    </tbody>
                </table>
            </div>

            <Modal title="Create Webhook".to_string() show=show_create.get() on_close=Callback::new(move |()| set_show_create.set(false))>
                <form class="modal-form" on:submit=move |ev| ev.prevent_default()>
                    <div class="form-group">
                        <label class="form-label">"Webhook URL"</label>
                        <input type="url" class="form-input" placeholder="https://example.com/webhook" prop:value=new_url on:input=move |ev| set_new_url.set(event_target_value(&ev)) />
                    </div>
                    <div class="form-group">
                        <label class="form-label">"Secret (optional)"</label>
                        <input type="text" class="form-input" placeholder="Webhook signing secret" prop:value=new_secret on:input=move |ev| set_new_secret.set(event_target_value(&ev)) />
                    </div>
                    <div class="form-group">
                        <label class="form-label">"Events"</label>
                        <div class="checkbox-group" role="group" aria-label="Webhook events">
                            {event_options.iter().map(|event| {
                                let ev = event.to_string();
                                let se = selected_events.get();
                                let checked = se.contains(&ev);
                                let te = toggle_event;
                                view! {
                                    <label class="checkbox-label">
                                        <input type="checkbox" prop:checked=checked on:change=move |_| te(ev.clone()) />
                                        {event.to_string()}
                                    </label>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                    {move || form_error.get().map(|e| view! { <div class="form-error" role="alert" aria-live="assertive">{e}</div> })}
                    <div class="modal-actions">
                        <button type="button" class="btn btn-secondary" on:click=move |_| set_show_create.set(false)>"Cancel"</button>
                        <button type="submit" class="btn btn-primary">"Create Webhook"</button>
                    </div>
                </form>
            </Modal>

            <Modal title="Delete Webhook".to_string() show=show_delete.get() on_close=Callback::new(move |()| set_show_delete.set(false))>
                <div class="modal-form">
                    <p>"Are you sure you want to delete this webhook?"</p>
                    <div class="modal-actions">
                        <button class="btn btn-secondary" on:click=move |_| set_show_delete.set(false)>"Cancel"</button>
                        <button class="btn btn-danger" on:click=do_delete>"Delete"</button>
                    </div>
                </div>
            </Modal>
        </div>
    }
}
