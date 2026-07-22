use leptos::prelude::*;

#[derive(Clone, Debug)]
struct Contact {
    uid: String,
    name: String,
    emails: Vec<String>,
    phones: Vec<String>,
    organization: String,
    notes: String,
}

/// Contacts page with list and detail view.
#[component]
pub fn ContactsPage() -> impl IntoView {
    let (contacts, set_contacts) = signal(Vec::<Contact>::new());
    let (selected, set_selected) = signal(None::<String>);
    let (search, set_search) = signal(String::new());
    let (loading, set_loading) = signal(true);

    Effect::new(move |_| {
        set_loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            let set_c = set_contacts;
            let set_l = set_loading;
            wasm_bindgen_futures::spawn_local(async move {
                let client = crate::api::ApiClient::new(crate::api::ApiClientConfig::default());
                match client.get::<serde_json::Value>("/api/v1/contacts").await {
                    Ok(val) => {
                        if let Some(arr) = val.as_array() {
                            let items: Vec<Contact> = arr.iter().filter_map(|v| {
                                Some(Contact {
                                    uid: v["uid"].as_str()?.to_string(),
                                    name: v["name"].as_str().unwrap_or("Unknown").to_string(),
                                    emails: v["emails"].as_array().map(|a| a.iter().filter_map(|e| e.as_str().map(String::from)).collect()).unwrap_or_default(),
                                    phones: v["phones"].as_array().map(|a| a.iter().filter_map(|p| p.as_str().map(String::from)).collect()).unwrap_or_default(),
                                    organization: v["organization"].as_str().unwrap_or("").to_string(),
                                    notes: v["notes"].as_str().unwrap_or("").to_string(),
                                })
                            }).collect();
                            set_c.set(items);
                        }
                        set_l.set(false);
                    }
                    Err(e) => { log::error!("Contacts load failed: {}", e); set_l.set(false); }
                }
            });
        }
    });

    view! {
        <div class="flex h-full">
            <aside class="w-72 border-r border-[var(--color-border)] overflow-y-auto flex-shrink-0">
                <div class="p-3">
                    <input class="input w-full" type="text" placeholder="Search contacts..." prop:value=move || search.get() />
                </div>
                <nav class="px-2">
                    {move || {
                        let q = search.get().to_lowercase();
                        contacts.get().into_iter()
                            .filter(|c| q.is_empty() || c.name.to_lowercase().contains(&q))
                            .map(|c| {
                                let uid = c.uid.clone(); let uid2 = uid.clone();
                                let name = c.name.clone();
                                let sel = move || selected.get() == Some(uid2.clone());
                                let initials: String = name.split_whitespace().take(2).map(|w| w.chars().next().unwrap_or_default()).collect();
                                view! {
                                    <button class=move || format!("w-full text-left px-3 py-2 rounded-md text-sm flex items-center gap-2 {}", if sel() { "bg-accent-subtle text-accent" } else { "hover:bg-sunken" })
                                        on:click=move |_| set_selected.set(Some(uid.clone()))>
                                        <div class="w-8 h-8 rounded-full bg-accent-subtle text-accent flex items-center justify-center text-xs font-bold">{initials}</div>
                                        {name}
                                    </button>
                                }
                            }).collect_view()
                    }}
                </nav>
            </aside>
            <main class="flex-1 overflow-y-auto p-6">
                {move || {
                    let selected_uid = selected.get();
                    let contact_list = contacts.get();
                    if let Some(uid) = selected_uid {
                        if let Some(c) = contact_list.iter().find(|c| c.uid == uid).cloned() {
                            let initials: String = c.name.split_whitespace().take(2).map(|w| w.chars().next().unwrap_or_default()).collect();
                            view! {
                                <div>
                                    <div class="flex items-center gap-4 mb-6">
                                        <div class="w-16 h-16 rounded-full bg-accent-subtle text-accent flex items-center justify-center text-2xl font-bold">{initials}</div>
                                        <div>
                                            <h1 class="text-2xl font-bold">{c.name}</h1>
                                            {if !c.organization.is_empty() { view! { <p class="text-secondary">{c.organization}</p> }.into_any() } else { view! { <></> }.into_any() }}
                                        </div>
                                    </div>
                                    <div class="space-y-4">
                                        {if !c.emails.is_empty() { view! {
                                            <div><h3 class="text-sm font-semibold text-secondary uppercase mb-1">"Emails"</h3>
                                            {c.emails.into_iter().map(|e| view! { <p>{e}</p> }).collect_view()}</div>
                                        }.into_any() } else { view! { <></> }.into_any() }}
                                        {if !c.phones.is_empty() { view! {
                                            <div><h3 class="text-sm font-semibold text-secondary uppercase mb-1">"Phones"</h3>
                                            {c.phones.into_iter().map(|p| view! { <p>{p}</p> }).collect_view()}</div>
                                        }.into_any() } else { view! { <></> }.into_any() }}
                                        {if !c.notes.is_empty() { view! {
                                            <div><h3 class="text-sm font-semibold text-secondary uppercase mb-1">"Notes"</h3>
                                            <p>{c.notes}</p></div>
                                        }.into_any() } else { view! { <></> }.into_any() }}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <p class="text-secondary">"Contact not found"</p> }.into_any()
                        }
                    } else {
                        view! { <div class="text-center text-secondary py-12"><p>"Select a contact"</p></div> }.into_any()
                    }
                }}
            </main>
        </div>
    }
}
