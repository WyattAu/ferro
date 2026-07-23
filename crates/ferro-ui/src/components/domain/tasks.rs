use leptos::prelude::*;

#[derive(Clone, Debug)]
struct TaskItem {
    id: String,
    title: String,
    status: String,
    priority: String,
    assignee: Option<String>,
    due_date: Option<String>,
}

/// Tasks page with Kanban board view.
#[component]
pub fn TasksPage() -> impl IntoView {
    let (tasks, set_tasks) = signal(Vec::<TaskItem>::new());
    let (_loading, set_loading) = signal(true);
    let (view_mode, set_view_mode) = signal("board".to_string());

    Effect::new(move |_| {
        set_loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            let set_t = set_tasks;
            let set_l = set_loading;
            wasm_bindgen_futures::spawn_local(async move {
                let client = crate::api::ApiClient::from_env();
                match client.get::<serde_json::Value>("/api/v1/tasks").await {
                    Ok(val) => {
                        if let Some(arr) = val.as_array() {
                            let items: Vec<TaskItem> = arr
                                .iter()
                                .filter_map(|v| {
                                    Some(TaskItem {
                                        id: v["id"].as_str()?.to_string(),
                                        title: v["title"].as_str().unwrap_or("Untitled").to_string(),
                                        status: v["status"].as_str().unwrap_or("todo").to_string(),
                                        priority: v["priority"].as_str().unwrap_or("medium").to_string(),
                                        assignee: v["assignee"].as_str().map(String::from),
                                        due_date: v["due_date"].as_str().map(String::from),
                                    })
                                })
                                .collect();
                            set_t.set(items);
                        }
                        set_l.set(false);
                    }
                    Err(e) => {
                        log::error!("Tasks load failed: {}", e);
                        set_l.set(false);
                    }
                }
            });
        }
    });

    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center gap-3 px-4 py-2 border-b border-[var(--color-border)]">
                <h1 class="text-lg font-semibold">"Tasks"</h1>
                <div class="ml-auto flex gap-2">
                    <button class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "board" { "btn-primary" } else { "" })
                        on:click=move |_| set_view_mode.set("board".to_string())>"Board"</button>
                    <button class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "list" { "btn-primary" } else { "" })
                        on:click=move |_| set_view_mode.set("list".to_string())>"List"</button>
                </div>
            </div>
            <div class="flex-1 overflow-x-auto p-4">
                <TaskBoard tasks=tasks />
            </div>
        </div>
    }
}

#[component]
fn TaskBoard(tasks: ReadSignal<Vec<TaskItem>>) -> impl IntoView {
    let columns = vec![("todo", "To Do"), ("in_progress", "In Progress"), ("done", "Done")];

    view! {
        <div class="flex gap-4 h-full">
            {columns.into_iter().map(|(col, label)| {
                let col_name = col.to_string();
                let col_label = label.to_string();
                view! {
                    <div class="flex-shrink-0 w-72">
                        <div class="flex items-center gap-2 mb-3">
                            <h3 class="text-sm font-semibold uppercase text-secondary">{col_label}</h3>
                        </div>
                        <div class="space-y-2">
                            {move || tasks.get().into_iter()
                                .filter(|t| t.status == col_name)
                                .map(|task| {
                                    let pc = match task.priority.as_str() {
                                        "high" => "text-danger",
                                        "medium" => "text-warning",
                                        _ => "text-secondary",
                                    }.to_string();
                                    let title = task.title.clone();
                                    let priority = task.priority.clone();
                                    let _pc = pc;
                                    view! {
                                        <div class="card p-3 cursor-pointer hover:shadow-md transition-shadow">
                                            <p class="font-medium text-sm mb-2">{title}</p>
                                            <div class="text-xs text-secondary">{priority}</div>
                                        </div>
                                    }
                                }).collect_view()
                            }
                        </div>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}
