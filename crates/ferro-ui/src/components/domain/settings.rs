use leptos::prelude::*;

/// Settings page with tabs.
#[component]
pub fn SettingsPage() -> impl IntoView {
    let (tab, set_tab) = signal("account".to_string());

    let tabs = vec![
        ("account", "Account"),
        ("preferences", "Preferences"),
        ("appearance", "Appearance"),
        ("notifications", "Notifications"),
    ];

    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center gap-3 px-4 py-2 border-b border-[var(--color-border)]">
                <h1 class="text-lg font-semibold">"Settings"</h1>
            </div>
            <div class="flex flex-1 overflow-hidden">
                <aside class="w-48 border-r border-[var(--color-border)] py-2 flex-shrink-0">
                    {tabs.into_iter().map(|(id, label)| {
                        let tab_id = id.to_string();
                        let tab_label = label.to_string();
                        let tab_id2 = tab_id.clone();
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
                        "account" => view! {
                            <div class="max-w-2xl space-y-6">
                                <h2 class="text-xl font-semibold">"Account Settings"</h2>
                                <div class="card space-y-4">
                                    <div><label class="text-sm font-medium text-secondary">"Display Name"</label>
                                    <input class="input mt-1" type="text" value="Admin" /></div>
                                    <div><label class="text-sm font-medium text-secondary">"Email"</label>
                                    <input class="input mt-1" type="text" value="admin@ferro.local" /></div>
                                    <button class="btn btn-primary">"Save Changes"</button>
                                </div>
                            </div>
                        }.into_any(),
                        "preferences" => view! {
                            <div class="max-w-2xl space-y-6">
                                <h2 class="text-xl font-semibold">"Preferences"</h2>
                                <div class="card space-y-4">
                                    <div class="flex items-center justify-between">
                                        <span class="text-sm">"Show hidden files"</span>
                                        <input type="checkbox" class="w-5 h-5" />
                                    </div>
                                    <div class="flex items-center justify-between">
                                        <span class="text-sm">"Items per page"</span>
                                        <select class="input w-32"><option>"50"</option><option>"100"</option><option>"200"</option></select>
                                    </div>
                                </div>
                            </div>
                        }.into_any(),
                        "appearance" => view! {
                            <div class="max-w-2xl space-y-6">
                                <h2 class="text-xl font-semibold">"Appearance"</h2>
                                <div class="card space-y-4">
                                    <div><label class="text-sm font-medium text-secondary">"Theme"</label>
                                    <div class="flex gap-3 mt-2">
                                        <button class="btn btn-secondary">"Light"</button>
                                        <button class="btn btn-primary">"Dark"</button>
                                        <button class="btn btn-secondary">"System"</button>
                                    </div></div>
                                </div>
                            </div>
                        }.into_any(),
                        "notifications" => view! {
                            <div class="max-w-2xl space-y-6">
                                <h2 class="text-xl font-semibold">"Notifications"</h2>
                                <div class="card space-y-4">
                                    <div class="flex items-center justify-between"><span class="text-sm">"Share notifications"</span><input type="checkbox" class="w-5 h-5" checked /></div>
                                    <div class="flex items-center justify-between"><span class="text-sm">"Upload notifications"</span><input type="checkbox" class="w-5 h-5" checked /></div>
                                    <div class="flex items-center justify-between"><span class="text-sm">"Comment notifications"</span><input type="checkbox" class="w-5 h-5" /></div>
                                </div>
                            </div>
                        }.into_any(),
                        _ => view! { <p>"Unknown tab"</p> }.into_any(),
                    }}
                </main>
            </div>
        </div>
    }
}
