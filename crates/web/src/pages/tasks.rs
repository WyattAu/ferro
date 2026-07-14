use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;
use crate::t;

use chrono::Datelike;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub assignee: String,
    pub due_date: Option<String>,
    pub priority: String,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
enum ViewMode {
    Kanban,
    Calendar,
}

#[derive(Debug, Clone, PartialEq)]
enum SortBy {
    CreatedAt,
    Priority,
    DueDate,
}

fn priority_color(priority: &str) -> &'static str {
    match priority {
        "urgent" => "bg-[var(--danger-subtle)] text-[var(--danger)] border-[var(--danger-subtle)] dark:border-red-700",
        "high" => {
            "bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-400 border-orange-300 dark:border-orange-700"
        }
        "medium" => {
            "bg-[var(--warning-subtle)] text-[var(--warning)] border-[var(--warning-subtle)] dark:border-yellow-700"
        }
        "low" => {
            "bg-[var(--success-subtle)] text-[var(--success)] border-[var(--success-subtle)] dark:border-green-700"
        }
        _ => {
            "bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)] text-[var(--text-secondary)] border-[var(--border-default)]"
        }
    }
}

fn status_icon(status: &str) -> &'static str {
    match status {
        "in_progress" => "blue",
        "done" => "green",
        _ => "gray",
    }
}

#[component]
pub fn TasksPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (loading, set_loading) = signal(true);
    let (tasks, set_tasks) = signal(Vec::<Task>::new());
    let (error_msg, set_error) = signal(String::new());
    let (view_mode, set_view_mode) = signal(ViewMode::Kanban);
    let (show_create_dialog, set_show_create_dialog) = signal(false);
    let (show_detail_modal, set_show_detail_modal) = signal(false);
    let (selected_task, set_selected_task) = signal(None::<Task>);
    let (filter_assignee, set_filter_assignee) = signal(String::new());
    let (filter_priority, set_filter_priority) = signal(String::new());
    let (filter_tag, _set_filter_tag) = signal(String::new());
    let (sort_by, set_sort_by) = signal(SortBy::CreatedAt);

    // Create dialog state
    let (create_title, set_create_title) = signal(String::new());
    let (create_description, set_create_description) = signal(String::new());
    let (create_status, set_create_status) = signal("todo".to_string());
    let (create_assignee, set_create_assignee) = signal(String::new());
    let (create_due_date, set_create_due_date) = signal(String::new());
    let (create_priority, set_create_priority) = signal("medium".to_string());
    let (create_tags, set_create_tags) = signal(String::new());

    // Detail modal state
    let (detail_title, set_detail_title) = signal(String::new());
    let (detail_description, set_detail_description) = signal(String::new());
    let (detail_status, set_detail_status) = signal(String::new());
    let (detail_assignee, set_detail_assignee) = signal(String::new());
    let (detail_due_date, set_detail_due_date) = signal(String::new());
    let (detail_priority, set_detail_priority) = signal(String::new());
    let (detail_tags, set_detail_tags) = signal(String::new());

    // Drag state
    let (dragging_task_id, set_dragging_task_id) = signal(None::<String>);

    let fetch_tasks = move || {
        set_loading.set(true);
        set_error.set(String::new());
        spawn_local(async move {
            let mut url = "/api/tasks".to_string();
            let mut params = Vec::new();
            let assignee = filter_assignee.get();
            if !assignee.is_empty() {
                params.push(format!("assignee={}", urlencoding(&assignee)));
            }
            let priority = filter_priority.get();
            if !priority.is_empty() {
                params.push(format!("priority={}", urlencoding(&priority)));
            }
            let tag = filter_tag.get();
            if !tag.is_empty() {
                params.push(format!("tag={}", urlencoding(&tag)));
            }
            let sort_str = match sort_by.get() {
                SortBy::Priority => "priority",
                SortBy::DueDate => "due_date",
                _ => "created_at",
            };
            params.push(format!("sort={}", sort_str));
            if !params.is_empty() {
                url.push('?');
                url.push_str(&params.join("&"));
            }

            match api::fetch_json(&url).await {
                Ok(val) => {
                    let tasks_list = val
                        .get("tasks")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    Some(Task {
                                        id: v.get("id")?.as_str()?.to_string(),
                                        title: v.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                                        description: v
                                            .get("description")
                                            .and_then(|d| d.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        status: v.get("status").and_then(|s| s.as_str()).unwrap_or("todo").to_string(),
                                        assignee: v.get("assignee").and_then(|a| a.as_str()).unwrap_or("").to_string(),
                                        due_date: v.get("due_date").and_then(|d| d.as_str()).map(String::from),
                                        priority: v
                                            .get("priority")
                                            .and_then(|p| p.as_str())
                                            .unwrap_or("medium")
                                            .to_string(),
                                        tags: v.get("tags").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                                        created_at: v
                                            .get("created_at")
                                            .and_then(|c| c.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        updated_at: v
                                            .get("updated_at")
                                            .and_then(|u| u.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    set_tasks.set(tasks_list);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(e);
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        let _ = filter_assignee.get();
        let _ = filter_priority.get();
        let _ = filter_tag.get();
        let _ = sort_by.get();
        fetch_tasks();
    });

    let columns = vec![("todo", "To Do"), ("in_progress", "In Progress"), ("done", "Done")];

    let tasks_for_column = move |status: &str| -> Vec<Task> {
        let status = status.to_string();
        tasks.get().into_iter().filter(|t| t.status == status).collect()
    };

    let open_detail = move |task: Task| {
        set_detail_title.set(task.title.clone());
        set_detail_description.set(task.description.clone());
        set_detail_status.set(task.status.clone());
        set_detail_assignee.set(task.assignee.clone());
        set_detail_due_date.set(task.due_date.clone().unwrap_or_default());
        set_detail_priority.set(task.priority.clone());
        set_detail_tags.set(task.tags.clone());
        set_selected_task.set(Some(task));
        set_show_detail_modal.set(true);
    };

    let create_task = move |_: ev::MouseEvent| {
        let title = create_title.get();
        let description = create_description.get();
        let status = create_status.get();
        let assignee = create_assignee.get();
        let due_date = create_due_date.get();
        let priority = create_priority.get();
        let tags = create_tags.get();
        set_show_create_dialog.set(false);

        spawn_local(async move {
            let body = serde_json::json!({
                "title": title,
                "description": description,
                "status": status,
                "assignee": assignee,
                "due_date": if due_date.is_empty() { None } else { Some(due_date) },
                "priority": priority,
                "tags": tags,
            });
            match api::fetch_json_with_method("/api/tasks", "POST", Some(&body.to_string())).await {
                Ok(_) => {
                    fetch_tasks();
                    // Reset form
                    set_create_title.set(String::new());
                    set_create_description.set(String::new());
                    set_create_status.set("todo".to_string());
                    set_create_assignee.set(String::new());
                    set_create_due_date.set(String::new());
                    set_create_priority.set("medium".to_string());
                    set_create_tags.set(String::new());
                }
                Err(e) => {
                    set_error.set(e);
                }
            }
        });
    };

    let save_task_detail = move |_: ev::MouseEvent| {
        if let Some(ref task) = selected_task.get() {
            let task_id = task.id.clone();
            let title = detail_title.get();
            let description = detail_description.get();
            let status = detail_status.get();
            let assignee = detail_assignee.get();
            let due_date = detail_due_date.get();
            let priority = detail_priority.get();
            let tags = detail_tags.get();
            set_show_detail_modal.set(false);

            spawn_local(async move {
                let body = serde_json::json!({
                    "title": title,
                    "description": description,
                    "status": status,
                    "assignee": assignee,
                    "due_date": if due_date.is_empty() { None } else { Some(due_date) },
                    "priority": priority,
                    "tags": tags,
                });
                let _ = api::fetch_json_with_method(&format!("/api/tasks/{}", task_id), "PUT", Some(&body.to_string()))
                    .await;
                fetch_tasks();
            });
        }
    };

    let delete_task = move |id: String| {
        spawn_local(async move {
            let _ = api::fetch_json_with_method(&format!("/api/tasks/{}", id), "DELETE", None).await;
            set_selected_task.set(None);
            set_show_detail_modal.set(false);
            fetch_tasks();
        });
    };

    let move_task = move |task_id: String, new_status: String| {
        spawn_local(async move {
            let body = serde_json::json!({ "status": new_status });
            let _ = api::fetch_json_with_method(
                &format!("/api/tasks/{}/status", task_id),
                "PATCH",
                Some(&body.to_string()),
            )
            .await;
            fetch_tasks();
        });
    };

    // Drag and drop handlers
    let on_drag_start = {
        move |task_id: String, ev: ev::DragEvent| {
            set_dragging_task_id.set(Some(task_id));
            if let Some(data_transfer) = ev.data_transfer() {
                let _ = data_transfer.set_data("text/plain", &dragging_task_id.get().unwrap_or_default());
                data_transfer.set_effect_allowed("move");
            }
        }
    };

    let on_drag_over = move |ev: ev::DragEvent| {
        ev.prevent_default();
        if let Some(data_transfer) = ev.data_transfer() {
            data_transfer.set_drop_effect("move");
        }
    };

    let on_drop = {
        move |status: String, ev: ev::DragEvent| {
            ev.prevent_default();
            if let Some(task_id) = dragging_task_id.get() {
                move_task(task_id, status);
            }
            set_dragging_task_id.set(None);
        }
    };

    view! {
        <div class="h-screen flex flex-col bg-[var(--bg-base)]">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-[var(--accent)] focus:text-[var(--text-on-accent)] focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 overflow-auto px-2 sm:px-4 pt-16">
                <main id="main-content" class="max-w-full w-full mx-auto p-6">
                    // Header
                    <div class="flex items-center justify-between mb-6">
                        <h1 class="text-2xl font-bold font-mono text-[var(--text-primary)]">{t!("tasks.title")}</h1>
                        <div class="flex items-center gap-3">
                            // View mode toggle
                            <div class="flex items-center gap-1 bg-[var(--border-subtle)] bg-[var(--bg-surface-raised)] rounded-lg p-1">
                                <button
                                    on:click=move |_| set_view_mode.set(ViewMode::Kanban)
                                    class=move || format!("px-3 py-1.5 text-sm font-medium rounded-md transition-colors {}",
                                        if view_mode.get() == ViewMode::Kanban {
                                            "bg-[var(--bg-surface)] dark:bg-[var(--text-tertiary)] text-[var(--text-primary)] shadow"
                                        } else {
                                            "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)]"
                                        }
                                    )
                                >
                                    "Board"
                                </button>
                                <button
                                    on:click=move |_| set_view_mode.set(ViewMode::Calendar)
                                    class=move || format!("px-3 py-1.5 text-sm font-medium rounded-md transition-colors {}",
                                        if view_mode.get() == ViewMode::Calendar {
                                            "bg-[var(--bg-surface)] dark:bg-[var(--text-tertiary)] text-[var(--text-primary)] shadow"
                                        } else {
                                            "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)]"
                                        }
                                    )
                                >
                                    "Calendar"
                                </button>
                            </div>
                            <button
                                on:click=move |_: ev::MouseEvent| {
                                    set_create_title.set(String::new());
                                    set_create_description.set(String::new());
                                    set_create_status.set("todo".to_string());
                                    set_create_assignee.set(String::new());
                                    set_create_due_date.set(String::new());
                                    set_create_priority.set("medium".to_string());
                                    set_create_tags.set(String::new());
                                    set_show_create_dialog.set(true);
                                }
                                class="inline-flex items-center gap-2 px-4 py-2 bg-[var(--accent)] text-[var(--text-on-accent)] text-sm font-bold rounded-lg hover:bg-[var(--accent-hover)] transition-colors"
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
                                {t!("tasks.new_task")}
                            </button>
                        </div>
                    </div>

                    // Filters
                    <div class="flex items-center gap-4 mb-6">
                        <div class="flex items-center gap-2">
                            <label class="text-sm text-[var(--text-secondary)]">Assignee:</label>
                            <input
                                type="text"
                                prop:value=move || filter_assignee.get()
                                on:input=move |ev| set_filter_assignee.set(event_target_value(&ev))
                                class="px-2 py-1 text-sm border border-[var(--border-default)] rounded bg-[var(--bg-surface)] text-[var(--text-primary)]"
                                placeholder="Filter..."
                            />
                        </div>
                        <div class="flex items-center gap-2">
                            <label class="text-sm text-[var(--text-secondary)]">Priority:</label>
                            <select
                                prop:value=move || filter_priority.get()
                                on:change=move |ev| set_filter_priority.set(event_target_value(&ev))
                                class="px-2 py-1 text-sm border border-[var(--border-default)] rounded bg-[var(--bg-surface)] text-[var(--text-primary)]"
                            >
                                <option value="">All</option>
                                <option value="urgent">Urgent</option>
                                <option value="high">High</option>
                                <option value="medium">Medium</option>
                                <option value="low">Low</option>
                            </select>
                        </div>
                        <div class="flex items-center gap-2">
                            <label class="text-sm text-[var(--text-secondary)]">Sort:</label>
                            <select
                                prop:value=move || match sort_by.get() {
                                    SortBy::Priority => "priority",
                                    SortBy::DueDate => "due_date",
                                    _ => "created_at",
                                }
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    set_sort_by.set(match val.as_str() {
                                        "priority" => SortBy::Priority,
                                        "due_date" => SortBy::DueDate,
                                        _ => SortBy::CreatedAt,
                                    });
                                }
                                class="px-2 py-1 text-sm border border-[var(--border-default)] rounded bg-[var(--bg-surface)] text-[var(--text-primary)]"
                            >
                                <option value="created_at">Created</option>
                                <option value="priority">Priority</option>
                                <option value="due_date">Due Date</option>
                            </select>
                        </div>
                    </div>

                    {move || loading.get().then(|| view! {
                        <div class="flex items-center justify-center py-12" role="status" aria-busy="true">
                            <div class="text-sm text-[var(--text-tertiary)] font-mono">{t!("common.loading")}</div>
                        </div>
                    })}

                    {move || (!error_msg.get().is_empty() && !loading.get()).then(|| view! {
                        <div class="p-4 bg-[var(--danger-subtle)] border-l-4 border-l-[var(--danger)] rounded text-sm text-[var(--danger)]" role="alert">
                            <span class="font-bold">{t!("error.prefix")}</span> {error_msg}
                        </div>
                    })}

                    // Kanban board
                    {move || (view_mode.get() == ViewMode::Kanban && !loading.get()).then(|| {
                        let cols = columns.clone();
                        view! {
                            <div class="flex gap-6 overflow-x-auto pb-4">
                                {cols.into_iter().map(|(status, label)| {
                                    let status_clone = status.to_string();
                                    let column_tasks = tasks_for_column(status);
                                                    let status_clone2 = status_clone.to_string();
                                    view! {
                                        <div
                                            class="flex-shrink-0 w-80 bg-[var(--bg-inset)] rounded-xl border border-[var(--border-default)]"
                                            on:dragover=on_drag_over
                                            on:drop={
                                                let status_clone3 = status_clone.clone();
                                                move |ev: ev::DragEvent| on_drop(status_clone3.clone(), ev)
                                            }
                                        >
                                            <div class="px-4 py-3 border-b border-[var(--border-default)]">
                                                <div class="flex items-center justify-between">
                                                    <h3 class="font-bold font-mono text-[var(--text-primary)]">{label}</h3>
                                                    <span class="text-xs text-[var(--text-tertiary)] bg-[var(--border-subtle)] bg-[var(--bg-surface-raised)] px-2 py-0.5 rounded-full">{column_tasks.len()}</span>
                                                </div>
                                            </div>
                                            <div class="p-3 space-y-3 min-h-[200px]">
                                                <For
                                                    each=move || column_tasks.clone()
                                                    key=|t| t.id.clone()
                                                    let:task
                                                >
                                                    {
                                                        let task_clone = task.clone();
                                                        let task_id = task.id.clone();
                                                        view! {
                                                            <div
                                                                class="bg-[var(--bg-surface)] rounded-lg border border-[var(--border-default)] p-3 cursor-grab active:cursor-grabbing hover:shadow-md transition-shadow"
                                                                draggable="true"
                                                                on:dragstart={
                                                                    let task_id_for_drag = task_id.clone();
                                                                    move |ev: ev::DragEvent| on_drag_start(task_id_for_drag.clone(), ev)
                                                                }
                                                                on:click=move |_: ev::MouseEvent| open_detail(task_clone.clone())
                                                            >
                                                                <div class="flex items-start justify-between gap-2">
                                                                    <h4 class="text-sm font-medium text-[var(--text-primary)] flex-1">{task.title.clone()}</h4>
                                                                    <span class=move || format!("text-xs px-1.5 py-0.5 rounded border {}", priority_color(&task.priority))>
                                                                        {task.priority.clone()}
                                                                    </span>
                                                                </div>
                                                                {if !task.description.is_empty() {
                                                                    view! {
                                                                        <p class="text-xs text-[var(--text-tertiary)] dark:text-[var(--text-tertiary)] mt-1 line-clamp-2">{task.description.clone()}</p>
                                                                    }.into_any()
                                                                } else {
                                                                    ().into_any()
                                                                }}
                                                                <div class="flex items-center justify-between mt-2">
                                                                    <div class="flex items-center gap-2">
                                                                    {if let Some(ref due) = task.due_date {
                                                                        let due_clone = due.clone();
                                                                        view! {
                                                                            <span class="text-xs text-[var(--text-tertiary)]">{due_clone}</span>
                                                                        }.into_any()
                                                                        } else {
                                                                            ().into_any()
                                                                        }}
                                                                    </div>
                                                                    {if !task.assignee.is_empty() {
                                                                        view! {
                                                                            <span class="text-xs text-[var(--accent)] dark:text-[var(--accent)]">{task.assignee.clone()}</span>
                                                                        }.into_any()
                                                                    } else {
                                                                        ().into_any()
                                                                    }}
                                                                </div>
                                                                {if !task.tags.is_empty() {
                                                                    view! {
                                                                        <div class="flex flex-wrap gap-1 mt-2">
                                                                            {task.tags.split(',').filter(|t| !t.trim().is_empty()).map(|tag| {
                                                                                view! { <span class="text-xs px-1.5 py-0.5 bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)] text-[var(--text-secondary)] rounded">{tag.trim()}</span> }
                                                                            }).collect::<Vec<_>>()}
                                                                        </div>
                                                                    }.into_any()
                                                                } else {
                                                                    ().into_any()
                                                                }}
                                                                // Quick move buttons
                                                                <div class="flex items-center gap-1 mt-2 pt-2 border-t border-[var(--border-subtle)] border-[var(--border-default)]">
                                                                    {if status_clone2 != "todo" {
                                                                        let task_id_back = task_id.clone();
                                                                        let status_back = status_clone2.clone();
                                                                        view! {
                                                                            <button
                                                                                on:click=move |ev: ev::MouseEvent| {
                                                                                    ev.stop_propagation();
                                                                                    let new_status = if status_back == "done" { "in_progress".to_string() } else { "todo".to_string() };
                                                                                    move_task(task_id_back.clone(), new_status);
                                                                                }
                                                                                class="text-xs px-2 py-0.5 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded"
                                                                            >
                                                                                {if status_clone2 == "done" { "Reopen" } else { "Back" }}
                                                                            </button>
                                                                        }.into_any()
                                                                    } else {
                                                                        ().into_any()
                                                                    }}
                                                                    {if status_clone2 != "done" {
                                                                        let task_id_forward = task_id.clone();
                                                                        let status_forward = status_clone2.clone();
                                                                        view! {
                                                                            <button
                                                                                on:click=move |ev: ev::MouseEvent| {
                                                                                    ev.stop_propagation();
                                                                                    let new_status = if status_forward == "todo" { "in_progress".to_string() } else { "done".to_string() };
                                                                                    move_task(task_id_forward.clone(), new_status);
                                                                                }
                                                                                class="text-xs px-2 py-0.5 text-[var(--accent)] hover:text-[var(--accent)] dark:hover:text-[var(--accent)] hover:bg-[var(--accent-subtle)] hover:bg-[var(--accent-subtle)] rounded"
                                                                            >
                                                                                {if status_clone2 == "todo" { "Start" } else { "Complete" }}
                                                                            </button>
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
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }
                    })}

                    // Calendar view
                    {move || (view_mode.get() == ViewMode::Calendar && !loading.get()).then(|| {
                        let all_tasks = tasks.get();
                        let today = chrono::Utc::now().date_naive();

                        // Group tasks by due date
                        let mut tasks_by_date: std::collections::HashMap<String, Vec<Task>> = std::collections::HashMap::new();
                        for task in &all_tasks {
                            if let Some(ref due) = task.due_date {
                                tasks_by_date.entry(due.clone()).or_default().push(task.clone());
                            }
                        }

                        // Generate calendar for next 30 days
                        let mut days = Vec::new();
                        for i in 0..30 {
                            let date = today + chrono::TimeDelta::days(i);
                            let date_str = date.format("%Y-%m-%d").to_string();
                            let day_tasks = tasks_by_date.get(&date_str).cloned().unwrap_or_default();
                            days.push((date_str, date, day_tasks));
                        }

                        view! {
                            <div class="bg-[var(--bg-surface)] rounded-xl shadow-sm brutal-border overflow-hidden">
                                <div class="px-4 py-3 border-b border-[var(--border-default)] bg-[var(--bg-inset)]">
                                    <h3 class="font-bold font-mono text-[var(--text-primary)]">Upcoming Tasks</h3>
                                </div>
                                <div class="divide-y divide-[var(--border-default)]">
                                    {days.into_iter().map(|(_date_str, date, day_tasks)| {
                                        let is_today = date == today;
                                        view! {
                                            <div class=move || format!("px-4 py-3 {}",
                                                if is_today { "bg-[var(--accent-subtle)]" } else { "" }
                                            )>
                                                <div class="flex items-center gap-3">
                                                    <div class=move || format!("w-16 text-center {}",
                                                        if is_today { "text-[var(--accent)] dark:text-[var(--accent)] font-bold" } else { "text-[var(--text-tertiary)]" }
                                                    )>
                                                        <div class="text-xs">{format!("{:?}", date.weekday())}</div>
                                                        <div class="text-lg font-mono">{date.day()}</div>
                                                    </div>
                                                    <div class="flex-1">
                                                        {if day_tasks.is_empty() {
                                                            view! { <div class="text-sm text-[var(--text-tertiary)]">No tasks</div> }.into_any()
                                                        } else {
                                                            view! {
                                                                <div class="space-y-1">
                                                                    {day_tasks.into_iter().map(|task| {
                                                                        let task_clone = task.clone();
                                                                        view! {
                                                                            <div
                                                                                class="flex items-center gap-2 px-2 py-1 rounded hover:bg-[var(--interactive-hover)] cursor-pointer"
                                                                                on:click=move |_: ev::MouseEvent| open_detail(task_clone.clone())
                                                                            >
                                                                                <span class=move || format!("w-2 h-2 rounded-full {}", status_icon(&task.status))></span>
                                                                                <span class="text-sm text-[var(--text-primary)] flex-1">{task.title.clone()}</span>
                                                                                <span class=move || format!("text-xs px-1.5 py-0.5 rounded {}", priority_color(&task.priority))>
                                                                                    {task.priority.clone()}
                                                                                </span>
                                                                            </div>
                                                                        }
                                                                    }).collect::<Vec<_>>()}
                                                                </div>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }
                    })}
                </main>
            </div>

            // Create task dialog
            {move || show_create_dialog.get().then(|| view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_create_dialog.set(false)>
                    <div class="bg-[var(--bg-surface)] rounded-xl shadow-xl max-w-lg w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                        <h3 class="text-lg font-bold font-mono text-[var(--text-primary)] mb-4">{t!("tasks.new_task")}</h3>
                        <div class="space-y-4">
                            <div>
                                <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.title")}</label>
                                <input
                                    type="text"
                                    prop:value=move || create_title.get()
                                    on:input=move |ev| set_create_title.set(event_target_value(&ev))
                                    class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                    placeholder="Task title"
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.description")}</label>
                                <textarea
                                    prop:value=move || create_description.get()
                                    on:input=move |ev| set_create_description.set(event_target_value(&ev))
                                    rows="3"
                                    class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                    placeholder="Description (optional)"
                                ></textarea>
                            </div>
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.status")}</label>
                                    <select
                                        prop:value=move || create_status.get()
                                        on:change=move |ev| set_create_status.set(event_target_value(&ev))
                                        class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                    >
                                        <option value="todo">To Do</option>
                                        <option value="in_progress">In Progress</option>
                                        <option value="done">Done</option>
                                    </select>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.priority")}</label>
                                    <select
                                        prop:value=move || create_priority.get()
                                        on:change=move |ev| set_create_priority.set(event_target_value(&ev))
                                        class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                    >
                                        <option value="low">Low</option>
                                        <option value="medium">Medium</option>
                                        <option value="high">High</option>
                                        <option value="urgent">Urgent</option>
                                    </select>
                                </div>
                            </div>
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.assignee")}</label>
                                    <input
                                        type="text"
                                        prop:value=move || create_assignee.get()
                                        on:input=move |ev| set_create_assignee.set(event_target_value(&ev))
                                        class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        placeholder="Assignee"
                                    />
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.due_date")}</label>
                                    <input
                                        type="date"
                                        prop:value=move || create_due_date.get()
                                        on:input=move |ev| set_create_due_date.set(event_target_value(&ev))
                                        class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                    />
                                </div>
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.tags")}</label>
                                <input
                                    type="text"
                                    prop:value=move || create_tags.get()
                                    on:input=move |ev| set_create_tags.set(event_target_value(&ev))
                                    class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                    placeholder="Comma-separated tags"
                                />
                            </div>
                        </div>
                        <div class="flex items-center justify-end gap-3 mt-6">
                            <button
                                on:click=move |_: ev::MouseEvent| set_show_create_dialog.set(false)
                                class="px-4 py-2 text-sm font-medium text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded-lg transition-colors"
                            >
                                {t!("common.cancel")}
                            </button>
                            <button
                                on:click=create_task
                                class="px-4 py-2 text-sm font-medium text-[var(--text-on-accent)] bg-[var(--accent)] hover:bg-[var(--accent-hover)] rounded-lg transition-colors"
                            >
                                {t!("common.save")}
                            </button>
                        </div>
                    </div>
                </div>
            })}

            // Task detail modal
            {move || show_detail_modal.get().then(|| {
                let task_id = selected_task.get().map(|t| t.id.clone()).unwrap_or_default();
                view! {
                    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_detail_modal.set(false)>
                        <div class="bg-[var(--bg-surface)] rounded-xl shadow-xl max-w-2xl w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                            <h3 class="text-lg font-bold font-mono text-[var(--text-primary)] mb-4">{t!("tasks.edit_task")}</h3>
                            <div class="space-y-4">
                                <div>
                                    <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.title")}</label>
                                    <input
                                        type="text"
                                        prop:value=move || detail_title.get()
                                        on:input=move |ev| set_detail_title.set(event_target_value(&ev))
                                        class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                    />
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.description")}</label>
                                    <textarea
                                        prop:value=move || detail_description.get()
                                        on:input=move |ev| set_detail_description.set(event_target_value(&ev))
                                        rows="4"
                                        class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                    ></textarea>
                                </div>
                                <div class="grid grid-cols-2 gap-4">
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.status")}</label>
                                        <select
                                            prop:value=move || detail_status.get()
                                            on:change=move |ev| set_detail_status.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        >
                                            <option value="todo">To Do</option>
                                            <option value="in_progress">In Progress</option>
                                            <option value="done">Done</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.priority")}</label>
                                        <select
                                            prop:value=move || detail_priority.get()
                                            on:change=move |ev| set_detail_priority.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        >
                                            <option value="low">Low</option>
                                            <option value="medium">Medium</option>
                                            <option value="high">High</option>
                                            <option value="urgent">Urgent</option>
                                        </select>
                                    </div>
                                </div>
                                <div class="grid grid-cols-2 gap-4">
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.assignee")}</label>
                                        <input
                                            type="text"
                                            prop:value=move || detail_assignee.get()
                                            on:input=move |ev| set_detail_assignee.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.due_date")}</label>
                                        <input
                                            type="date"
                                            prop:value=move || detail_due_date.get()
                                            on:input=move |ev| set_detail_due_date.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        />
                                    </div>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("tasks.tags")}</label>
                                    <input
                                        type="text"
                                        prop:value=move || detail_tags.get()
                                        on:input=move |ev| set_detail_tags.set(event_target_value(&ev))
                                        class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        placeholder="Comma-separated tags"
                                    />
                                </div>
                            </div>
                            <div class="flex items-center justify-between mt-6">
                                <button
                                    on:click=move |_: ev::MouseEvent| {
                                        set_show_detail_modal.set(false);
                                        delete_task(task_id.clone());
                                    }
                                    class="px-4 py-2 text-sm font-medium text-[var(--danger)] hover:text-[var(--danger)] hover:bg-[var(--danger-subtle)] rounded-lg transition-colors"
                                >
                                    {t!("tasks.delete")}
                                </button>
                                <div class="flex items-center gap-3">
                                    <button
                                        on:click=move |_: ev::MouseEvent| set_show_detail_modal.set(false)
                                        class="px-4 py-2 text-sm font-medium text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded-lg transition-colors"
                                    >
                                        {t!("common.cancel")}
                                    </button>
                                    <button
                                        on:click=save_task_detail
                                        class="px-4 py-2 text-sm font-medium text-[var(--text-on-accent)] bg-[var(--accent)] hover:bg-[var(--accent-hover)] rounded-lg transition-colors"
                                    >
                                        {t!("common.save")}
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('/', "%2F")
        .replace('&', "%26")
        .replace('?', "%3F")
        .replace('#', "%23")
}
