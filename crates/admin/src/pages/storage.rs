use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api::ApiState;
use crate::components::chart::{BarChart, PieChart};
use crate::state::format_timestamp;
use ferro_common::format::format_size as format_bytes;

#[component]
pub fn StoragePage(api: RwSignal<ApiState>) -> impl IntoView {
    let (storage, set_storage) = signal(None::<serde_json::Value>);
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(true);
    let (msg, set_msg) = signal(None::<String>);

    Effect::new(move |_| {
        let api_clone = api.get_untracked();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match api_clone.storage_info().await {
                Ok(s) => set_storage.set(Some(s)),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    });

    let do_empty_trash = move |_: leptos::ev::MouseEvent| {
        let api_clone = api.get_untracked();
        spawn_local(async move {
            match api_clone.delete("/api/v1/trash/empty").await {
                Ok(_) => set_msg.set(Some("Trash emptied successfully".to_string())),
                Err(e) => set_msg.set(Some(format!("Failed: {}", e))),
            }
        });
    };

    view! {
        <div class="page">
            <div aria-live="polite">
                {move || loading.get().then(|| view! { <div class="loading" role="status">"Loading storage information..."</div> })}
            </div>
            <div aria-live="assertive">
                {move || error.get().map(|e| view! { <div class="error-banner" role="alert">{e}</div> })}
            </div>
            <div aria-live="polite">
                {move || msg.get().map(|m| view! { <div class="success-banner" role="status">{m}</div> })}
            </div>

            {move || {
                let s = storage.get()?;
                let total_bytes = s.get("total_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                let file_count = s.get("file_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let dir_count = s.get("directory_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let backend = s.get("backend").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                let recent = s.get("recent_files").and_then(|f| f.as_array()).cloned().unwrap_or_default();

                let mut chart_data: Vec<(String, f64)> = recent.iter().take(8).map(|f| {
                    let name = f.get("path").and_then(|p| p.as_str()).unwrap_or("?").rsplit('/').next().unwrap_or("?").chars().take(12).collect();
                    let size = f.get("size").and_then(|sz| sz.as_f64()).unwrap_or(0.0);
                    (name, size)
                }).collect();
                chart_data.reverse();

                let (mut docs, mut images, mut videos, mut other) = (0.0_f64, 0.0, 0.0, 0.0);
                for f in &recent {
                    let path = f.get("path").and_then(|p| p.as_str()).unwrap_or("").to_lowercase();
                    let size = f.get("size").and_then(|sz| sz.as_f64()).unwrap_or(0.0);
                    if path.ends_with(".txt") || path.ends_with(".md") || path.ends_with(".pdf") || path.ends_with(".doc") || path.ends_with(".docx") { docs += size; }
                    else if path.ends_with(".png") || path.ends_with(".jpg") || path.ends_with(".jpeg") || path.ends_with(".gif") || path.ends_with(".svg") { images += size; }
                    else if path.ends_with(".mp4") || path.ends_with(".mkv") || path.ends_with(".webm") { videos += size; }
                    else { other += size; }
                }
                let type_data = vec![("Documents".to_string(), docs), ("Images".to_string(), images), ("Videos".to_string(), videos), ("Other".to_string(), other)];

                let largest_path = s.get("largest_file").and_then(|f| f.get("path")).and_then(|p| p.as_str()).unwrap_or("-").to_string();
                let largest_size = format_bytes(s.get("largest_file").and_then(|f| f.get("size")).and_then(|sz| sz.as_u64()).unwrap_or(0));

                let file_rows: Vec<_> = recent.iter().map(|f| {
                    let path = f.get("path").and_then(|p| p.as_str()).unwrap_or("-").to_string();
                    let size = f.get("size").and_then(|sz| sz.as_u64()).map(format_bytes).unwrap_or("-".to_string());
                    let modified = f.get("modified_at").and_then(|m| m.as_str()).unwrap_or("-").to_string();
                    (path, size, modified)
                }).collect();

                Some(view! {
                    <>
                        <div class="stats-grid">
                            <div class="stats-card surface">
                                <div class="stats-card-header"><span class="stats-card-title">"Total Storage"</span></div>
                                <div class="stats-card-value">{format_bytes(total_bytes)}</div>
                            </div>
                            <div class="stats-card surface">
                                <div class="stats-card-header"><span class="stats-card-title">"Files"</span></div>
                                <div class="stats-card-value">{file_count}</div>
                            </div>
                            <div class="stats-card surface">
                                <div class="stats-card-header"><span class="stats-card-title">"Directories"</span></div>
                                <div class="stats-card-value">{dir_count}</div>
                            </div>
                            <div class="stats-card surface">
                                <div class="stats-card-header"><span class="stats-card-title">"Backend"</span></div>
                                <div class="stats-card-value">{backend}</div>
                            </div>
                        </div>

                        <div class="dashboard-panels">
                            <div class="panel surface">
                                <h2 class="panel-title font-display">"File Size Distribution"</h2>
                                <BarChart data=chart_data title="".to_string() color="#E85D04".to_string() />
                            </div>
                            <div class="panel surface">
                                <h2 class="panel-title font-display">"Files by Type"</h2>
                                <PieChart data=type_data title="".to_string() />
                            </div>
                        </div>

                        <div class="panel surface">
                            <h2 class="panel-title font-display">"Largest File"</h2>
                            <div class="detail-row">
                                <span class="detail-label">"Path"</span>
                                <span class="detail-value mono">{largest_path}</span>
                            </div>
                            <div class="detail-row">
                                <span class="detail-label">"Size"</span>
                                <span class="detail-value">{largest_size}</span>
                            </div>
                        </div>

                        <div class="panel surface brutal-border">
                            <div class="panel-header-row">
                                <h2 class="panel-title font-display">"Recent Files"</h2>
                                <button class="btn btn-secondary btn-sm" on:click=do_empty_trash aria-label="Empty trash">"Empty Trash"</button>
                            </div>
                            <div class="table-wrapper">
                                <table class="data-table" aria-label="Recent files">
                                    <thead><tr><th scope="col">"Path"</th><th scope="col">"Size"</th><th scope="col">"Modified"</th></tr></thead>
                                    <tbody>
                                        {if file_rows.is_empty() {
                                            vec![view! { <tr><td colspan="3" class="table-empty">"No files found"</td></tr> }.into_any()]
                                        } else {
                                            file_rows.iter().map(|(path, size, modified)| view! {
                                                <tr>
                                                    <td class="mono">{path.clone()}</td>
                                                    <td>{size.clone()}</td>
                                                    <td>{format_timestamp(modified)}</td>
                                                </tr>
                                            }.into_any()).collect::<Vec<_>>()
                                        }}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </>
                })
            }}
        </div>
    }
}
