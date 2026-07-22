use leptos::prelude::*;

/// Admin panel with tabs for enterprise features.
#[component]
pub fn AdminPage() -> impl IntoView {
    let (tab, set_tab) = signal("overview".to_string());

    let tabs = vec![
        ("overview", "Overview"),
        ("users", "Users"),
        ("dlp", "DLP Policies"),
        ("antivirus", "Antivirus"),
        ("watermarks", "Watermarks"),
        ("audit", "Audit Log"),
    ];

    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center gap-3 px-4 py-2 border-b border-[var(--color-border)]">
                <h1 class="text-lg font-semibold">"Admin Panel"</h1>
            </div>
            <div class="flex flex-1 overflow-hidden">
                <aside class="w-48 border-r border-[var(--color-border)] py-2 flex-shrink-0">
                    {tabs.into_iter().map(|(id, label)| {
                        let tab_id = id.to_string(); let tab_id2 = tab_id.clone();
                        let tab_label = label.to_string();
                        view! {
                            <button class=move || format!("w-full text-left px-4 py-2 text-sm {}", if tab.get() == tab_id { "bg-accent-subtle text-accent font-medium" } else { "hover:bg-sunken text-secondary" })
                                on:click=move |_| set_tab.set(tab_id2.clone())>
                                {tab_label}
                            </button>
                        }
                    }).collect_view()}
                </aside>
                <main class="flex-1 overflow-y-auto p-6">
                    {move || match tab.get().as_str() {
                        "overview" => view! {
                            <div class="space-y-6">
                                <h2 class="text-xl font-semibold">"Overview"</h2>
                                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
                                    <div class="card"><p class="text-sm text-secondary">"Total Files"</p><p class="text-2xl font-bold">"—"</p></div>
                                    <div class="card"><p class="text-sm text-secondary">"Total Size"</p><p class="text-2xl font-bold">"—"</p></div>
                                    <div class="card"><p class="text-sm text-secondary">"Users"</p><p class="text-2xl font-bold">"—"</p></div>
                                    <div class="card"><p class="text-sm text-secondary">"Share Links"</p><p class="text-2xl font-bold">"—"</p></div>
                                </div>
                            </div>
                        }.into_any(),
                        "users" => view! {
                            <div class="space-y-4">
                                <div class="flex items-center justify-between">
                                    <h2 class="text-xl font-semibold">"Users"</h2>
                                    <button class="btn btn-primary btn-sm">"+ Add User"</button>
                                </div>
                                <table class="table w-full">
                                    <thead><tr><th>"Name"</th><th>"Email"</th><th>"Role"</th><th>"Actions"</th></tr></thead>
                                    <tbody>
                                        <tr><td>"Admin"</td><td>"admin@ferro.local"</td><td><span class="badge badge-accent">"Admin"</span></td><td>"—"</td></tr>
                                    </tbody>
                                </table>
                            </div>
                        }.into_any(),
                        "dlp" => view! {
                            <div class="space-y-4">
                                <div class="flex items-center justify-between">
                                    <h2 class="text-xl font-semibold">"DLP Policies"</h2>
                                    <button class="btn btn-primary btn-sm">"+ New Policy"</button>
                                </div>
                                <p class="text-secondary">"No DLP policies configured"</p>
                            </div>
                        }.into_any(),
                        "antivirus" => view! {
                            <div class="space-y-4">
                                <h2 class="text-xl font-semibold">"Antivirus"</h2>
                                <div class="card">
                                    <p class="text-secondary">"ClamAV integration"</p>
                                    <button class="btn btn-primary btn-sm mt-3">"Scan All Files"</button>
                                </div>
                            </div>
                        }.into_any(),
                        "watermarks" => view! {
                            <div class="space-y-4">
                                <div class="flex items-center justify-between">
                                    <h2 class="text-xl font-semibold">"Watermarks"</h2>
                                    <button class="btn btn-primary btn-sm">"+ New Watermark"</button>
                                </div>
                                <p class="text-secondary">"No watermark policies configured"</p>
                            </div>
                        }.into_any(),
                        "audit" => view! {
                            <div class="space-y-4">
                                <h2 class="text-xl font-semibold">"Audit Log"</h2>
                                <table class="table w-full">
                                    <thead><tr><th>"Timestamp"</th><th>"Action"</th><th>"User"</th><th>"Path"</th></tr></thead>
                                    <tbody>
                                        <tr><td class="text-sm text-secondary">"—"</td><td>"—"</td><td>"—"</td><td>"—"</td></tr>
                                    </tbody>
                                </table>
                            </div>
                        }.into_any(),
                        _ => view! { <p>"Unknown tab"</p> }.into_any(),
                    }}
                </main>
            </div>
        </div>
    }
}
