use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::api;
use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;
use crate::t;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Contact {
    pub uid: String,
    pub address_book_id: String,
    pub vcard_data: String,
    pub etag: String,
    pub created_at: String,
    pub updated_at: String,
}

fn parse_vcard_fn(vcard: &str) -> String {
    for line in vcard.lines() {
        if let Some(rest) = line.strip_prefix("FN:") {
            return rest.trim().to_string();
        }
    }
    "Unknown".to_string()
}

fn parse_vcard_emails(vcard: &str) -> Vec<String> {
    let mut emails = Vec::new();
    for line in vcard.lines() {
        if line.starts_with("EMAIL")
            && let Some(colon_pos) = line.find(':')
        {
            emails.push(line[colon_pos + 1..].trim().to_string());
        }
    }
    emails
}

fn parse_vcard_phones(vcard: &str) -> Vec<String> {
    let mut phones = Vec::new();
    for line in vcard.lines() {
        if line.starts_with("TEL")
            && let Some(colon_pos) = line.find(':')
        {
            phones.push(line[colon_pos + 1..].trim().to_string());
        }
    }
    phones
}

fn parse_vcard_org(vcard: &str) -> String {
    for line in vcard.lines() {
        if let Some(rest) = line.strip_prefix("ORG:") {
            return rest.trim().to_string();
        }
    }
    String::new()
}

fn parse_vcard_note(vcard: &str) -> String {
    for line in vcard.lines() {
        if let Some(rest) = line.strip_prefix("NOTE:") {
            return rest.trim().to_string();
        }
    }
    String::new()
}

fn parse_vcard_photo(vcard: &str) -> Option<String> {
    for line in vcard.lines() {
        if line.starts_with("PHOTO")
            && let Some(colon_pos) = line.find(':')
        {
            let data = line[colon_pos + 1..].trim();
            if !data.is_empty() {
                let mime = if line.contains("PNG") {
                    "image/png"
                } else if line.contains("JPEG") || line.contains("JPG") {
                    "image/jpeg"
                } else {
                    "image/png"
                };
                return Some(format!("data:{};base64,{}", mime, data));
            }
        }
    }
    None
}

fn build_vcard(fn_name: &str, emails: &[String], phones: &[String], org: &str, note: &str, uid: &str) -> String {
    let mut vcard = format!(
        "BEGIN:VCARD\r\n\
         VERSION:3.0\r\n\
         UID:{}\r\n\
         FN:{}\r\n",
        uid, fn_name
    );
    for email in emails {
        if !email.is_empty() {
            vcard.push_str(&format!("EMAIL:{}\r\n", email));
        }
    }
    for phone in phones {
        if !phone.is_empty() {
            vcard.push_str(&format!("TEL:{}\r\n", phone));
        }
    }
    if !org.is_empty() {
        vcard.push_str(&format!("ORG:{}\r\n", org));
    }
    if !note.is_empty() {
        vcard.push_str(&format!("NOTE:{}\r\n", note));
    }
    vcard.push_str("END:VCARD\r\n");
    vcard
}

fn initials_from_name(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() >= 2 {
        format!("{}{}", &parts[0][..1], &parts[parts.len() - 1][..1])
    } else if !parts.is_empty() && !parts[0].is_empty() {
        parts[0][..1].to_uppercase()
    } else {
        "?".to_string()
    }
}

#[component]
pub fn ContactsPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (loading, set_loading) = signal(true);
    let (contacts, set_contacts) = signal(Vec::<Contact>::new());
    let (error_msg, set_error) = signal(String::new());
    let (search_query, set_search_query) = signal(String::new());
    let (selected_contact, set_selected_contact) = signal(None::<Contact>);
    let (show_dialog, set_show_dialog) = signal(false);
    let (editing_contact, set_editing_contact) = signal(None::<Contact>);

    let (dialog_fn, set_dialog_fn) = signal(String::new());
    let (dialog_emails, set_dialog_emails) = signal(String::new());
    let (dialog_phones, set_dialog_phones) = signal(String::new());
    let (dialog_org, set_dialog_org) = signal(String::new());
    let (dialog_note, set_dialog_note) = signal(String::new());

    let fetch_contacts = move || {
        set_loading.set(true);
        set_error.set(String::new());
        spawn_local(async move {
            match api::fetch_json("/api/contacts").await {
                Ok(val) => {
                    let ctrs = val
                        .get("contacts")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    Some(Contact {
                                        uid: v.get("uid")?.as_str()?.to_string(),
                                        address_book_id: v
                                            .get("address_book_id")
                                            .and_then(|a| a.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        vcard_data: v
                                            .get("vcard_data")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        etag: v.get("etag").and_then(|e| e.as_str()).unwrap_or("").to_string(),
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
                    set_contacts.set(ctrs);
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
        fetch_contacts();
    });

    let filtered_contacts = move || {
        let q = search_query.get().to_lowercase();
        let ctrs = contacts.get();
        if q.is_empty() {
            ctrs
        } else {
            ctrs.into_iter()
                .filter(|c| {
                    let name = parse_vcard_fn(&c.vcard_data).to_lowercase();
                    let emails = parse_vcard_emails(&c.vcard_data);
                    let org = parse_vcard_org(&c.vcard_data).to_lowercase();
                    name.contains(&q) || org.contains(&q) || emails.iter().any(|e| e.to_lowercase().contains(&q))
                })
                .collect()
        }
    };

    let open_create_dialog = move |_: ev::MouseEvent| {
        set_editing_contact.set(None);
        set_dialog_fn.set(String::new());
        set_dialog_emails.set(String::new());
        set_dialog_phones.set(String::new());
        set_dialog_org.set(String::new());
        set_dialog_note.set(String::new());
        set_show_dialog.set(true);
    };

    let open_edit_dialog = move |contact: Contact| {
        set_editing_contact.set(Some(contact.clone()));
        set_dialog_fn.set(parse_vcard_fn(&contact.vcard_data));
        set_dialog_emails.set(parse_vcard_emails(&contact.vcard_data).join(", "));
        set_dialog_phones.set(parse_vcard_phones(&contact.vcard_data).join(", "));
        set_dialog_org.set(parse_vcard_org(&contact.vcard_data));
        set_dialog_note.set(parse_vcard_note(&contact.vcard_data));
        set_show_dialog.set(true);
    };

    let save_contact = move |_: ev::MouseEvent| {
        let fn_name = dialog_fn.get();
        let emails: Vec<String> = dialog_emails
            .get()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let phones: Vec<String> = dialog_phones
            .get()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let org = dialog_org.get();
        let note = dialog_note.get();
        let editing = editing_contact.get();

        let uid = editing
            .as_ref()
            .map(|c| c.uid.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let vcard = build_vcard(&fn_name, &emails, &phones, &org, &note, &uid);

        set_show_dialog.set(false);

        spawn_local(async move {
            if let Some(ref contact) = editing {
                let body = serde_json::json!({ "vcard_data": vcard });
                let _ = api::fetch_json_with_method(
                    &format!("/api/contacts/{}", contact.uid),
                    "PUT",
                    Some(&body.to_string()),
                )
                .await;
            } else {
                let body = serde_json::json!({
                    "address_book_id": "",
                    "vcard_data": vcard
                });
                let _ = api::fetch_json_with_method("/api/contacts", "POST", Some(&body.to_string())).await;
            }
            fetch_contacts();
        });
    };

    let delete_contact = move |uid: String| {
        spawn_local(async move {
            let _ = api::fetch_json_with_method(&format!("/api/contacts/{}", uid), "DELETE", None).await;
            set_selected_contact.set(None);
            fetch_contacts();
        });
    };

    let export_contacts = move |_: ev::MouseEvent| {
        spawn_local(async move {
            let _ = api::fetch_json("/api/contacts/export").await;
        });
    };

    let import_contacts = move |_: ev::MouseEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().expect("no window");
            let document = window.document().expect("no document");
            let input: web_sys::HtmlInputElement = document
                .create_element("input")
                .expect("failed to create input")
                .dyn_into()
                .expect("failed to cast");
            input.set_type("file");
            input.set_accept(".vcf");
            let _set_contacts_clone = set_contacts;
            let input_clone = input.clone();
            let on_change = Closure::wrap(Box::new(move |_: web_sys::Event| {
                if let Some(files) = input_clone.files() {
                    if let Some(file) = files.get(0) {
                        let reader = web_sys::FileReader::new().expect("failed to create reader");
                        let reader_clone = reader.clone();
                        let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                            if let Ok(text) = reader_clone.result() {
                                if let Some(text_str) = text.as_string() {
                                    spawn_local(async move {
                                        let body = text_str;
                                        let _ =
                                            api::fetch_json_with_method("/api/contacts/import", "POST", Some(&body))
                                                .await;
                                    });
                                }
                            }
                        }) as Box<dyn FnMut(_)>);
                        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                        let _ = onload.forget();
                        let _ = reader.read_as_text(&file);
                    }
                }
            }) as Box<dyn FnMut(_)>);
            input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
            let _ = on_change.forget();
            input.click();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = set_contacts;
        }
    };

    view! {
        <div class="h-screen flex flex-col bg-[var(--bg-base)]">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-[var(--accent)] focus:text-[var(--text-on-accent)] focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 overflow-auto px-2 sm:px-4 pt-16">
                <main id="main-content" class="max-w-7xl w-full mx-auto p-6">
                    <div class="flex items-center justify-between mb-6">
                        <h1 class="text-2xl font-bold font-mono text-[var(--text-primary)]">{t!("contacts.title")}</h1>
                        <div class="flex items-center gap-2">
                            <button
                                on:click=import_contacts
                                class="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium text-[var(--text-secondary)] bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg hover:bg-[var(--interactive-hover)] transition-colors"
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" /></svg>
                                {t!("contacts.import")}
                            </button>
                            <button
                                on:click=export_contacts
                                class="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium text-[var(--text-secondary)] bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg hover:bg-[var(--interactive-hover)] transition-colors"
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" /></svg>
                                {t!("contacts.export")}
                            </button>
                            <button
                                on:click=open_create_dialog
                                class="inline-flex items-center gap-2 px-4 py-2 bg-[var(--accent)] text-[var(--text-on-accent)] text-sm font-bold rounded-lg hover:bg-[var(--accent-hover)] transition-colors"
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
                                {t!("contacts.new_contact")}
                            </button>
                        </div>
                    </div>

                    // Search bar
                    <div class="mb-6">
                        <div class="relative">
                            <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--text-tertiary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>
                            <input
                                type="text"
                                placeholder={t!("contacts.search_placeholder")}
                                prop:value=move || search_query.get()
                                on:input=move |ev| set_search_query.set(event_target_value(&ev))
                                class="w-full pl-10 pr-4 py-2.5 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                            />
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

                    <div class="flex gap-6">
                        // Contact list
                        <div class="w-1/3">
                            <div class="bg-[var(--bg-surface)] rounded-xl shadow-sm brutal-border overflow-hidden">
                                <div class="divide-y divide-[var(--border-default)]">
                                    <For
                                        each=move || filtered_contacts()
                                        key=|c| c.uid.clone()
                                        let:contact
                                    >
                                        {
                                            let name = parse_vcard_fn(&contact.vcard_data);
                                            let emails = parse_vcard_emails(&contact.vcard_data);
                                            let primary_email = emails.first().cloned().unwrap_or_default();
                                            let initials = initials_from_name(&name);
                                            let uid_clone = contact.uid.clone();
                                            let is_selected = move || selected_contact.get().as_ref().map(|c| c.uid.clone()) == Some(uid_clone.clone());
                                            let contact_clone = contact.clone();
                                            view! {
                                                <div
                                                    class=move || format!("px-4 py-3 cursor-pointer transition-colors {}",
                                                        if is_selected() { "bg-[var(--accent-subtle)]" } else { "hover:bg-[var(--interactive-hover)]/50" }
                                                    )
                                                    on:click=move |_: ev::MouseEvent| set_selected_contact.set(Some(contact_clone.clone()))
                                                >
                                                    <div class="flex items-center gap-3">
                                                        <div class="w-10 h-10 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center text-sm font-bold text-[var(--accent)] dark:text-[var(--accent)] shrink-0">
                                                            {initials}
                                                        </div>
                                                        <div class="min-w-0">
                                                            <div class="text-sm font-medium text-[var(--text-primary)] truncate">{name}</div>
                                                            <div class="text-xs text-[var(--text-tertiary)] truncate">{primary_email}</div>
                                                        </div>
                                                    </div>
                                                </div>
                                            }
                                        }
                                    </For>
                                </div>
                            </div>
                        </div>

                        // Contact detail
                        <div class="flex-1">
                            {move || {
                                if let Some(ref contact) = selected_contact.get() {
                                    let name = parse_vcard_fn(&contact.vcard_data);
                                    let emails = parse_vcard_emails(&contact.vcard_data);
                                    let phones = parse_vcard_phones(&contact.vcard_data);
                                    let org = parse_vcard_org(&contact.vcard_data);
                                    let note = parse_vcard_note(&contact.vcard_data);
                                    let photo = parse_vcard_photo(&contact.vcard_data);
                                    let initials = initials_from_name(&name);
                                    let contact_clone = contact.clone();
                                    let uid_for_delete = contact.uid.clone();
                                    let org_clone = org.clone();
                                    view! {
                                        <div class="bg-[var(--bg-surface)] rounded-xl shadow-sm brutal-border p-6">
                                            <div class="flex items-start justify-between mb-6">
                                                <div class="flex items-center gap-4">
                                                    {if let Some(ref photo_url) = photo {
                                                        view! {
                                                            <img src=photo_url class="w-16 h-16 rounded-full object-cover" alt="Contact photo" />
                                                        }.into_any()
                                                    } else {
                                                        view! {
                                                            <div class="w-16 h-16 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center text-xl font-bold text-[var(--accent)] dark:text-[var(--accent)]">
                                                                {initials}
                                                            </div>
                                                        }.into_any()
                                                    }}
                                                    <div>
                                                        <h2 class="text-xl font-bold font-mono text-[var(--text-primary)]">{name}</h2>
                                                        {if !org_clone.is_empty() {
                                                            view! { <div class="text-sm text-[var(--text-tertiary)]">{org_clone}</div> }.into_any()
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                    </div>
                                                </div>
                                                <div class="flex items-center gap-2">
                                                    <button
                                                        on:click=move |_: ev::MouseEvent| open_edit_dialog(contact_clone.clone())
                                                        class="p-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded-lg transition-colors"
                                                        aria-label="Edit"
                                                    >
                                                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" /></svg>
                                                    </button>
                                                    <button
                                                        on:click=move |_: ev::MouseEvent| delete_contact(uid_for_delete.clone())
                                                        class="p-2 text-[var(--danger)] hover:text-[var(--danger)] hover:bg-[var(--danger-subtle)] rounded-lg transition-colors"
                                                        aria-label="Delete"
                                                    >
                                                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
                                                    </button>
                                                </div>
                                            </div>

                                            <div class="space-y-4">
                                                {if !emails.is_empty() {
                                                    view! {
                                                        <div>
                                                            <h3 class="text-xs font-bold uppercase text-[var(--text-tertiary)] mb-2">{t!("contacts.email")}</h3>
                                                            <div class="space-y-1">
                                                                {emails.into_iter().map(|email| view! {
                                                                    <div class="text-sm text-[var(--text-primary)] font-mono">{email}</div>
                                                                }).collect::<Vec<_>>()}
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}

                                                {if !phones.is_empty() {
                                                    view! {
                                                        <div>
                                                            <h3 class="text-xs font-bold uppercase text-[var(--text-tertiary)] mb-2">{t!("contacts.phone")}</h3>
                                                            <div class="space-y-1">
                                                                {phones.into_iter().map(|phone| view! {
                                                                    <div class="text-sm text-[var(--text-primary)] font-mono">{phone}</div>
                                                                }).collect::<Vec<_>>()}
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}

                                                {if !org.is_empty() {
                                                    view! {
                                                        <div>
                                                            <h3 class="text-xs font-bold uppercase text-[var(--text-tertiary)] mb-2">{t!("contacts.organization")}</h3>
                                                            <div class="text-sm text-[var(--text-primary)] font-mono">{org}</div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}

                                                {if !note.is_empty() {
                                                    view! {
                                                        <div>
                                                            <h3 class="text-xs font-bold uppercase text-[var(--text-tertiary)] mb-2">{t!("contacts.notes")}</h3>
                                                            <div class="text-sm text-[var(--text-primary)] whitespace-pre-wrap">{note}</div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                            </div>

                                            <div class="mt-6 pt-4 border-t border-[var(--border-default)]">
                                                <div class="text-xs text-[var(--text-tertiary)] font-mono">
                                                    {t!("contacts.created")} ": " {contact.created_at.as_str()}
                                                    " · " {t!("contacts.updated")} ": " {contact.updated_at.as_str()}
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="bg-[var(--bg-surface)] rounded-xl shadow-sm brutal-border p-12 text-center">
                                            <svg class="w-12 h-12 mx-auto text-[var(--text-tertiary)] dark:text-[var(--text-secondary)] mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" /></svg>
                                            <p class="text-sm text-[var(--text-tertiary)]">{t!("contacts.select_contact")}</p>
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>

                    // Contact creation/editing dialog
                    {move || show_dialog.get().then(|| view! {
                        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_dialog.set(false)>
                            <div class="bg-[var(--bg-surface)] rounded-xl shadow-xl max-w-lg w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                                <h3 class="text-lg font-bold font-mono text-[var(--text-primary)] mb-4">
                                    {move || if editing_contact.get().is_some() { t!("contacts.edit_contact") } else { t!("contacts.new_contact") }}
                                </h3>
                                <div class="space-y-4">
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("contacts.full_name")}</label>
                                        <input
                                            type="text"
                                            prop:value=move || dialog_fn.get()
                                            on:input=move |ev| set_dialog_fn.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("contacts.emails")}</label>
                                        <input
                                            type="text"
                                            placeholder="email1@example.com, email2@example.com"
                                            prop:value=move || dialog_emails.get()
                                            on:input=move |ev| set_dialog_emails.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("contacts.phones")}</label>
                                        <input
                                            type="text"
                                            placeholder="+1 555-123-4567, +1 555-987-6543"
                                            prop:value=move || dialog_phones.get()
                                            on:input=move |ev| set_dialog_phones.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("contacts.organization")}</label>
                                        <input
                                            type="text"
                                            prop:value=move || dialog_org.get()
                                            on:input=move |ev| set_dialog_org.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">{t!("contacts.notes")}</label>
                                        <textarea
                                            prop:value=move || dialog_note.get()
                                            on:input=move |ev| set_dialog_note.set(event_target_value(&ev))
                                            rows="3"
                                            class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                        ></textarea>
                                    </div>
                                </div>
                                <div class="flex items-center justify-between mt-6">
                                    <div>
                                        {move || editing_contact.get().is_some().then(|| {
                                            let uid = editing_contact.get().map(|c| c.uid.clone()).unwrap_or_default();
                                            view! {
                                                <button
                                                    on:click=move |_: ev::MouseEvent| {
                                                        set_show_dialog.set(false);
                                                        delete_contact(uid.clone());
                                                    }
                                                    class="px-4 py-2 text-sm font-medium text-[var(--danger)] hover:text-[var(--danger)] hover:bg-[var(--danger-subtle)] rounded-lg transition-colors"
                                                >
                                                    {t!("contacts.delete")}
                                                </button>
                                            }
                                        })}
                                    </div>
                                    <div class="flex items-center gap-3">
                                        <button
                                            on:click=move |_: ev::MouseEvent| set_show_dialog.set(false)
                                            class="px-4 py-2 text-sm font-medium text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded-lg transition-colors"
                                        >
                                            {t!("common.cancel")}
                                        </button>
                                        <button
                                            on:click=save_contact
                                            class="px-4 py-2 text-sm font-medium text-[var(--text-on-accent)] bg-[var(--accent)] hover:bg-[var(--accent-hover)] rounded-lg transition-colors"
                                        >
                                            {t!("common.save")}
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    })}
                </main>
            </div>
        </div>
    }
}
