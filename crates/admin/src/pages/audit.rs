use leptos::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::api::ApiState;
use crate::components::badge::{Badge, BadgeVariant};
use crate::state::format_timestamp;

#[component]
pub fn AuditPage(api: RwSignal<ApiState>) -> impl IntoView {
    let (entries, set_entries) = create_signal(Vec::<serde_json::Value>::new());
    let (total, set_total) = create_signal(0_usize);
    let (error, set_error) = create_signal(None::<String>);
    let (loading, set_loading) = create_signal(true);
    let (page, set_page) = create_signal(0_usize);
    let (filter_user, set_filter_user) = create_signal(String::new());
    let (filter_action, set_filter_action) = create_signal(String::new());
    let page_size: usize = 50;

    let load_audit = move || {
        let p = page.get();
        let api_clone = api.get_untracked();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match api_clone.audit_log(page_size, p * page_size).await {
                Ok(data) => {
                    if let Some(e) = data.get("entries").and_then(|e| e.as_array()) {
                        set_entries.set(e.clone());
                    }
                    if let Some(t) = data.get("total").and_then(|t| t.as_u64()) {
                        set_total.set(t as usize);
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    create_effect(move |_| load_audit());

    let current_page = page.get();
    let total_pages = total.get().div_ceil(page_size);

    let do_prev = move |_: leptos::ev::MouseEvent| {
        if page.get() > 0 {
            set_page.update(|p| *p -= 1);
            load_audit();
        }
    };

    let do_next = move |_: leptos::ev::MouseEvent| {
        if page.get() + 1 < total_pages {
            set_page.update(|p| *p += 1);
            load_audit();
        }
    };

    let do_export = move |_: leptos::ev::MouseEvent| {
        let data = entries.get();
        let mut csv = String::from("Timestamp,User,Action,Resource,Status\n");
        for entry in &data {
            let ts = entry
                .get("timestamp")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            let user = entry.get("user").and_then(|u| u.as_str()).unwrap_or("");
            let action = entry.get("action").and_then(|a| a.as_str()).unwrap_or("");
            let resource = entry.get("resource").and_then(|r| r.as_str()).unwrap_or("");
            let status = entry.get("status").and_then(|s| s.as_str()).unwrap_or("");
            csv.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                ts, user, action, resource, status
            ));
        }
        if let Some(window) = web_sys::window() {
            let arr = js_sys::Array::new();
            arr.push(&JsValue::from_str(&csv));
            if let Ok(blob) = web_sys::Blob::new_with_str_sequence(&arr)
                && let Ok(blob_url) = web_sys::Url::create_object_url_with_blob(&blob)
                && let Some(doc) = window.document()
            {
                let el = doc.create_element("a");
                let el2 = el
                    .ok()
                    .and_then(|e| e.dyn_into::<web_sys::HtmlAnchorElement>().ok());
                if let Some(el2) = el2 {
                    el2.set_href(&blob_url);
                    el2.set_download("audit_log.csv");
                    el2.click();
                    let _ = web_sys::Url::revoke_object_url(&blob_url);
                }
            }
        }
    };

    view! {
        <div class="page">
            <div class="page-header">
                <div class="page-header-left">
                    <input type="text" class="search-input" placeholder="Filter by user..." prop:value=filter_user on:input=move |ev| set_filter_user.set(event_target_value(&ev)) aria-label="Filter by user" />
                    <input type="text" class="search-input" placeholder="Filter by action..." prop:value=filter_action on:input=move |ev| set_filter_action.set(event_target_value(&ev)) aria-label="Filter by action" />
                </div>
                <button class="btn btn-secondary" on:click=do_export>"Export CSV"</button>
            </div>

            {move || error.get().map(|e| view! { <div class="error-banner">{e}</div> })}
            {move || loading.get().then(|| view! { <div class="loading">"Loading audit log..."</div> })}

            <div class="table-wrapper">
                <table class="data-table">
                    <thead><tr><th>"Timestamp"</th><th>"User"</th><th>"Action"</th><th>"Resource"</th><th>"Status"</th></tr></thead>
                    <tbody>
                        {move || {
                            let user_filter = filter_user.get().to_lowercase();
                            let action_filter = filter_action.get().to_lowercase();
                            let all = entries.get();
                            let filtered: Vec<_> = all.iter().filter(|e| {
                                let u = e.get("user").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                                let a = e.get("action").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                                (user_filter.is_empty() || u.contains(&user_filter))
                                    && (action_filter.is_empty() || a.contains(&action_filter))
                            }).collect();
                            if filtered.is_empty() {
                                vec![view! { <tr><td colspan="5" class="table-empty">"No audit entries found"</td></tr> }]
                            } else {
                                filtered.iter().map(|entry| {
                                    let ts = entry.get("timestamp").and_then(|t| t.as_str()).unwrap_or("-").to_string();
                                    let user = entry.get("user").and_then(|u| u.as_str()).unwrap_or("-").to_string();
                                    let action = entry.get("action").and_then(|a| a.as_str()).unwrap_or("-").to_string();
                                    let resource = entry.get("resource").and_then(|r| r.as_str()).unwrap_or("-").to_string();
                                    let status = entry.get("status").and_then(|s| s.as_str()).unwrap_or("success").to_string();
                                    let bv = if status == "error" { BadgeVariant::Danger } else { BadgeVariant::Success };
                                    view! {
                                        <tr>
                                            <td class="mono">{format_timestamp(&ts)}</td>
                                            <td>{user}</td>
                                            <td><Badge text=action variant=BadgeVariant::Neutral/></td>
                                            <td class="mono">{resource}</td>
                                            <td><Badge text=status variant=bv/></td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()
                            }
                        }}
                    </tbody>
                </table>
            </div>

            <div class="pagination">
                <span class="pagination-info">
                    {format!("Page {} of {} ({} total entries)", current_page + 1, total_pages.max(1), total.get())}
                </span>
                <div class="pagination-controls">
                    <button class="btn btn-secondary btn-sm" on:click=do_prev disabled=current_page == 0>"Previous"</button>
                    <button class="btn btn-secondary btn-sm" on:click=do_next disabled=current_page + 1 >= total_pages>"Next"</button>
                </div>
            </div>
        </div>
    }
}
