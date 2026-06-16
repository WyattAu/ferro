use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;
use crate::components::navigation::NavigationSidebar;
use crate::t;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MailAccount {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub provider: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MailFolder {
    pub id: String,
    pub name: String,
    pub unread_count: u32,
    pub folder_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MailMessage {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub date: String,
    pub is_read: bool,
    pub has_attachments: bool,
    pub snippet: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MailMessageDetail {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub date: String,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
    pub attachments: Vec<MailAttachment>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MailAttachment {
    pub id: String,
    pub filename: String,
    pub size: u64,
    pub mime_type: String,
}

#[derive(Debug, Clone, PartialEq)]
enum MailView {
    List,
    Detail,
    Compose,
}

fn format_mail_date(date: &str) -> String {
    if date.len() >= 16 {
        format!("{} {}", &date[..10], &date[11..16])
    } else {
        date.to_string()
    }
}

#[component]
pub fn MailPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (loading, set_loading) = signal(true);
    let (accounts, set_accounts) = signal(Vec::<MailAccount>::new());
    let (selected_account, set_selected_account) = signal(None::<String>);
    let (folders, set_folders) = signal(Vec::<MailFolder>::new());
    let (selected_folder, set_selected_folder) = signal(None::<String>);
    let (messages, set_messages) = signal(Vec::<MailMessage>::new());
    let (selected_message, set_selected_message) = signal(None::<MailMessageDetail>);
    let (view, set_view) = signal(MailView::List);
    let (search_query, set_search_query) = signal(String::new());
    let (error_msg, set_error) = signal(String::new());

    let (show_add_account, set_show_add_account) = signal(false);
    let (new_account_email, set_new_account_email) = signal(String::new());
    let (new_account_provider, set_new_account_provider) = signal("imap".to_string());

    let (compose_to, set_compose_to) = signal(String::new());
    let (compose_cc, set_compose_cc) = signal(String::new());
    let (compose_subject, set_compose_subject) = signal(String::new());
    let (compose_body, set_compose_body) = signal(String::new());

    Effect::new(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            match api::fetch_json("/api/mail/accounts").await {
                Ok(val) => {
                    let list = val
                        .get("accounts")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    Some(MailAccount {
                                        id: v.get("id")?.as_str()?.to_string(),
                                        email: v.get("email").and_then(|e| e.as_str()).unwrap_or("").to_string(),
                                        display_name: v.get("display_name").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                                        provider: v.get("provider").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    set_accounts.set(list);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(e);
                    set_loading.set(false);
                }
            }
        });
    });

    let fetch_folders = move |account_id: &str| {
        let aid = account_id.to_string();
        spawn_local(async move {
            let url = format!("/api/mail/accounts/{}/folders", aid);
            match api::fetch_json(&url).await {
                Ok(val) => {
                    let list = val
                        .get("folders")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    Some(MailFolder {
                                        id: v.get("id")?.as_str()?.to_string(),
                                        name: v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                                        unread_count: v.get("unread_count").and_then(|u| u.as_u64()).unwrap_or(0) as u32,
                                        folder_type: v.get("folder_type").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    set_folders.set(list);
                }
                Err(_) => {}
            }
        });
    };

    let fetch_messages = move |account_id: &str, folder_id: &str| {
        let aid = account_id.to_string();
        let fid = folder_id.to_string();
        spawn_local(async move {
            let url = format!("/api/mail/accounts/{}/folders/{}/messages", aid, fid);
            match api::fetch_json(&url).await {
                Ok(val) => {
                    let list = val
                        .get("messages")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    Some(MailMessage {
                                        id: v.get("id")?.as_str()?.to_string(),
                                        subject: v.get("subject").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                                        from: v.get("from").and_then(|f| f.as_str()).unwrap_or("").to_string(),
                                        date: v.get("date").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                                        is_read: v.get("is_read").and_then(|r| r.as_bool()).unwrap_or(false),
                                        has_attachments: v.get("has_attachments").and_then(|a| a.as_bool()).unwrap_or(false),
                                        snippet: v.get("snippet").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    set_messages.set(list);
                }
                Err(_) => {}
            }
        });
    };

    let select_account = move |account_id: String| {
        set_selected_account.set(Some(account_id.clone()));
        set_selected_folder.set(None);
        set_messages.set(vec![]);
        set_selected_message.set(None);
        set_view.set(MailView::List);
        fetch_folders(&account_id);
    };

    let select_folder = move |folder_id: String| {
        set_selected_folder.set(Some(folder_id.clone()));
        set_selected_message.set(None);
        set_view.set(MailView::List);
        if let Some(aid) = selected_account.get() {
            fetch_messages(&aid, &folder_id);
        }
    };

    let select_message = move |message_id: String| {
        let aid = selected_account.get().unwrap_or_default();
        let fid = selected_folder.get().unwrap_or_default();
        let mid = message_id.clone();
        spawn_local(async move {
            let url = format!("/api/mail/accounts/{}/folders/{}/messages/{}", aid, fid, mid);
            match api::fetch_json(&url).await {
                Ok(val) => {
                    let detail = MailMessageDetail {
                        id: val.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
                        subject: val.get("subject").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                        from: val.get("from").and_then(|f| f.as_str()).unwrap_or("").to_string(),
                        to: val.get("to").and_then(|t| t.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
                        cc: val.get("cc").and_then(|c| c.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
                        date: val.get("date").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                        body_html: val.get("body_html").and_then(|h| h.as_str()).map(String::from),
                        body_text: val.get("body_text").and_then(|t| t.as_str()).map(String::from),
                        attachments: val.get("attachments").and_then(|a| a.as_array()).map(|arr| {
                            arr.iter().filter_map(|v| {
                                Some(MailAttachment {
                                    id: v.get("id")?.as_str()?.to_string(),
                                    filename: v.get("filename").and_then(|f| f.as_str()).unwrap_or("").to_string(),
                                    size: v.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
                                    mime_type: v.get("mime_type").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                                })
                            }).collect()
                        }).unwrap_or_default(),
                    };
                    set_selected_message.set(Some(detail));
                    set_view.set(MailView::Detail);
                }
                Err(_) => {}
            }
        });
    };

    let send_compose = move |_: ev::MouseEvent| {
        let to = compose_to.get();
        let cc = compose_cc.get();
        let subject = compose_subject.get();
        let body = compose_body.get();
        set_view.set(MailView::List);

        spawn_local(async move {
            let body_json = serde_json::json!({
                "to": to.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>(),
                "cc": cc.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>(),
                "subject": subject,
                "body": body,
            });
            let _ = api::fetch_json_with_method("/api/mail/send", "POST", Some(&body_json.to_string())).await;
        });
    };

    let delete_account = move |account_id: String| {
        let aid = account_id.clone();
        spawn_local(async move {
            let url = format!("/api/mail/accounts/{}", aid);
            let _ = api::fetch_json_with_method(&url, "DELETE", None).await;
        });
    };

    let format_bytes = |bytes: u64| -> String {
        if bytes == 0 { return "0 B".to_string(); }
        let units = ["B", "KB", "MB", "GB"];
        let mut val = bytes as f64;
        let mut idx = 0;
        while val >= 1024.0 && idx < units.len() - 1 { val /= 1024.0; idx += 1; }
        format!("{:.1} {}", val, units[idx])
    };

    view! {
        <div class="h-screen flex flex-col bg-gray-100 dark:bg-gray-900">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 flex overflow-hidden pt-16">
                <NavigationSidebar />
                <main id="main-content" class="flex-1 overflow-auto p-6">
                    <div class="flex items-center justify-between mb-6">
                        <h1 class="text-2xl font-bold font-mono text-gray-900 dark:text-white">{t!("mail.title")}</h1>
                        <div class="flex items-center gap-2">
                            <button
                                on:click=move |_| set_view.set(MailView::Compose)
                                class="inline-flex items-center gap-2 px-4 py-2 bg-blue-600 text-white text-sm font-bold rounded-lg hover:bg-blue-700 transition-colors"
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
                                {t!("mail.compose")}
                            </button>
                        </div>
                    </div>

                    <div class="flex gap-4 h-[calc(100vh-10rem)]">
                        {/* Account Sidebar */}
                        <div class="w-56 shrink-0 bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden flex flex-col">
                            <div class="p-3 border-b border-gray-200 dark:border-gray-700">
                                <div class="flex items-center justify-between">
                                    <h2 class="text-xs font-bold uppercase font-mono text-gray-500">{t!("mail.accounts")}</h2>
                                    <button
                                        on:click=move |_| set_show_add_account.set(true)
                                        class="p-1 text-gray-400 hover:text-blue-600 rounded transition-colors"
                                        aria-label=t!("mail.add_account")
                                    >
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
                                    </button>
                                </div>
                            </div>
                            <div class="flex-1 overflow-y-auto">
                                {move || loading.get().then(|| view! {
                                    <div class="p-4 text-sm text-gray-500">{t!("common.loading")}</div>
                                })}
                                <For
                                    each=move || accounts.get()
                                    key=|a| a.id.clone()
                                    let:account
                                >
                                    {
                                        let aid = account.id.clone();
                                        let aid2 = aid.clone();
                                        let email = account.email.clone();
                                        let selected = selected_account.clone();
                                        view! {
                                            <div
                                                class=move || format!("px-3 py-2.5 cursor-pointer border-b border-gray-100 dark:border-gray-700 transition-colors {}",
                                                    if selected.get() == Some(aid.clone()) { "bg-blue-50 dark:bg-blue-900/20 border-l-2 border-l-blue-600" } else { "hover:bg-gray-50 dark:hover:bg-gray-700" }
                                                )
                                                on:click=move |_: ev::MouseEvent| select_account(aid2.clone())
                                            >
                                                <div class="text-sm font-mono text-gray-900 dark:text-white truncate">{email}</div>
                                            </div>
                                        }
                                    }
                                </For>
                            </div>
                        </div>

                        {/* Folder List */}
                        <div class="w-44 shrink-0 bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden flex flex-col">
                            <div class="p-3 border-b border-gray-200 dark:border-gray-700">
                                <h2 class="text-xs font-bold uppercase font-mono text-gray-500">{t!("mail.folders")}</h2>
                            </div>
                            <div class="flex-1 overflow-y-auto">
                                <For
                                    each=move || folders.get()
                                    key=|f| f.id.clone()
                                    let:folder
                                >
                                    {
                                        let fid = folder.id.clone();
                                        let fid2 = fid.clone();
                                        let name = folder.name.clone();
                                        let unread = folder.unread_count;
                                        let selected = selected_folder.clone();
                                        view! {
                                            <div
                                                class=move || format!("px-3 py-2 cursor-pointer flex items-center justify-between transition-colors {}",
                                                    if selected.get() == Some(fid.clone()) { "bg-blue-50 dark:bg-blue-900/20" } else { "hover:bg-gray-50 dark:hover:bg-gray-700" }
                                                )
                                                on:click=move |_: ev::MouseEvent| select_folder(fid2.clone())
                                            >
                                                <span class="text-sm font-mono text-gray-700 dark:text-gray-300 truncate">{name}</span>
                                                {move || (unread > 0).then(|| view! {
                                                    <span class="text-xs font-bold bg-blue-600 text-white rounded-full px-1.5 py-0.5 min-w-[20px] text-center">{unread}</span>
                                                })}
                                            </div>
                                        }
                                    }
                                </For>
                            </div>
                        </div>

                        {/* Main Content Area */}
                        <div class="flex-1 bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden flex flex-col">
                            {move || match view.get() {
                                MailView::List => {
                                    let msgs = messages.get();
                                    let q = search_query.get();
                                    let filtered: Vec<_> = if q.is_empty() {
                                        msgs
                                    } else {
                                        msgs.into_iter().filter(|m| {
                                            m.subject.to_lowercase().contains(&q.to_lowercase()) ||
                                            m.from.to_lowercase().contains(&q.to_lowercase())
                                        }).collect()
                                    };

                                    view! {
                                        <>
                                            <div class="p-3 border-b border-gray-200 dark:border-gray-700">
                                                <div class="relative">
                                                    <input
                                                        type="text"
                                                        placeholder=t!("mail.search_placeholder")
                                                        prop:value=move || search_query.get()
                                                        on:input=move |ev| set_search_query.set(event_target_value(&ev))
                                                        class="w-full px-3 py-2 pl-8 text-sm font-mono border rounded bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                                                    />
                                                    <svg class="absolute left-2.5 top-2.5 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>
                                                </div>
                                            </div>
                                            <div class="flex-1 overflow-y-auto">
                                                {let f2 = filtered.clone(); move || f2.is_empty().then(|| view! {
                                                    <div class="p-8 text-center text-gray-500 text-sm">{t!("mail.no_messages")}</div>
                                                })}
                                                <For
                                                    each=move || filtered.clone()
                                                    key=|m| m.id.clone()
                                                    let:message
                                                >
                                                    {
                                                        let mid = message.id.clone();
                                                        let subject = message.subject.clone();
                                                        let from = message.from.clone();
                                                        let date = message.date.clone();
                                                        let is_read = message.is_read;
                                                        let has_attach = message.has_attachments;
                                                        let snippet = message.snippet.clone();
                                                        view! {
                                                            <div
                                                                class="px-4 py-3 border-b border-gray-100 dark:border-gray-700 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                                                                on:click=move |_: ev::MouseEvent| select_message(mid.clone())
                                                            >
                                                                <div class="flex items-center justify-between mb-1">
                                                                    <div class="flex items-center gap-2 min-w-0">
                                                                        {move || (!is_read).then(|| view! {
                                                                            <span class="w-2 h-2 bg-blue-600 rounded-full shrink-0"></span>
                                                                        })}
                                                                        <span class=move || format!("text-sm truncate {}", if is_read { "text-gray-600 dark:text-gray-400" } else { "font-bold text-gray-900 dark:text-white" })>{subject.clone()}</span>
                                                                    </div>
                                                                    <div class="flex items-center gap-2 shrink-0">
                                                                        {move || has_attach.then(|| view! {
                                                                            <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13" /></svg>
                                                                        })}
                                                                        <span class="text-xs text-gray-500 font-mono">{format_mail_date(&date)}</span>
                                                                    </div>
                                                                </div>
                                                                <div class="text-xs text-gray-500 mb-1">{from}</div>
                                                                <div class="text-xs text-gray-400 truncate">{snippet}</div>
                                                            </div>
                                                        }
                                                    }
                                                </For>
                                            </div>
                                        </>
                                    }.into_any()
                                }
                                MailView::Detail => {
                                    match selected_message.get() {
                                        Some(msg) => {
                                            view! {
                                                <div class="flex flex-col h-full">
                                                    <div class="p-4 border-b border-gray-200 dark:border-gray-700">
                                                        <div class="flex items-center gap-2 mb-2">
                                                            <button
                                                                on:click=move |_| set_view.set(MailView::List)
                                                                class="p-1 text-gray-500 hover:text-gray-700 rounded transition-colors"
                                                                aria-label=t!("common.back")
                                                            >
                                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" /></svg>
                                                            </button>
                                                            <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{msg.subject.clone()}</h2>
                                                        </div>
                                                        <div class="text-sm text-gray-600 dark:text-gray-400">
                                                            <span class="font-medium">{t!("mail.from")}</span> {msg.from.clone()} " · " {format_mail_date(&msg.date)}
                                                        </div>
                                                        {move || (!msg.to.is_empty()).then(|| view! {
                                                            <div class="text-xs text-gray-500 mt-1">
                                                                <span class="font-medium">{t!("mail.to")}</span> {msg.to.join(", ")}
                                                            </div>
                                                        })}
                                                        {move || (!msg.cc.is_empty()).then(|| view! {
                                                            <div class="text-xs text-gray-500 mt-1">
                                                                <span class="font-medium">{t!("mail.cc")}</span> {msg.cc.join(", ")}
                                                            </div>
                                                        })}
                                                    </div>
                                                    <div class="flex-1 overflow-y-auto p-4">
                                                        {move || msg.body_html.as_ref().map(|html| view! {
                                                            <div class="prose prose-sm dark:prose-invert max-w-none" inner_html=html.clone()></div>
                                                        })}
                                                        {move || msg.body_text.as_ref().map(|text| view! {
                                                            <pre class="text-sm font-mono text-gray-700 dark:text-gray-300 whitespace-pre-wrap">{text.clone()}</pre>
                                                        })}
                                                    </div>
                                                    {move || (!msg.attachments.is_empty()).then(|| view! {
                                                        <div class="p-4 border-t border-gray-200 dark:border-gray-700">
                                                            <h3 class="text-xs font-bold uppercase font-mono text-gray-500 mb-2">{t!("mail.attachments")}</h3>
                                                            <div class="flex flex-wrap gap-2">
                                                                {msg.attachments.iter().map(|att| {
                                                                    let filename = att.filename.clone();
                                                                    let size = att.size;
                                                                    let format_bytes_fn = format_bytes;
                                                                    view! {
                                                                        <div class="flex items-center gap-2 px-3 py-2 bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700">
                                                                            <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
                                                                            <div>
                                                                                <div class="text-sm font-mono text-gray-900 dark:text-white">{filename}</div>
                                                                                <div class="text-xs text-gray-500">{format_bytes_fn(size)}</div>
                                                                            </div>
                                                                        </div>
                                                                    }
                                                                }).collect::<Vec<_>>()}
                                                            </div>
                                                        </div>
                                                    })}
                                                </div>
                                            }.into_any()
                                        }
                                        None => view! {
                                            <div class="flex items-center justify-center h-full text-gray-500">{t!("mail.select_message")}</div>
                                        }.into_any(),
                                    }
                                }
                                MailView::Compose => {
                                    view! {
                                        <div class="flex flex-col h-full">
                                            <div class="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                                                <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{t!("mail.compose")}</h2>
                                                <button
                                                    on:click=move |_| set_view.set(MailView::List)
                                                    class="p-1 text-gray-500 hover:text-gray-700 rounded transition-colors"
                                                    aria-label=t!("common.close")
                                                >
                                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
                                                </button>
                                            </div>
                                            <div class="flex-1 overflow-y-auto p-4 space-y-4">
                                                <div>
                                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("mail.to")}</label>
                                                    <input
                                                        type="text"
                                                        prop:value=move || compose_to.get()
                                                        on:input=move |ev| set_compose_to.set(event_target_value(&ev))
                                                        placeholder="recipient@example.com"
                                                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500"
                                                    />
                                                </div>
                                                <div>
                                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("mail.cc")}</label>
                                                    <input
                                                        type="text"
                                                        prop:value=move || compose_cc.get()
                                                        on:input=move |ev| set_compose_cc.set(event_target_value(&ev))
                                                        placeholder="cc@example.com (optional)"
                                                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500"
                                                    />
                                                </div>
                                                <div>
                                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("mail.subject")}</label>
                                                    <input
                                                        type="text"
                                                        prop:value=move || compose_subject.get()
                                                        on:input=move |ev| set_compose_subject.set(event_target_value(&ev))
                                                        placeholder="Subject"
                                                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500"
                                                    />
                                                </div>
                                                <div class="flex-1">
                                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("mail.body")}</label>
                                                    <textarea
                                                        prop:value=move || compose_body.get()
                                                        on:input=move |ev| set_compose_body.set(event_target_value(&ev))
                                                        rows="12"
                                                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500"
                                                    ></textarea>
                                                </div>
                                                <div class="flex items-center gap-2">
                                                    <button
                                                        on:click=send_compose
                                                        class="px-4 py-2 bg-blue-600 text-white text-sm font-bold rounded-lg hover:bg-blue-700 transition-colors"
                                                    >
                                                        {t!("mail.send")}
                                                    </button>
                                                    <button
                                                        on:click=move |_| set_view.set(MailView::List)
                                                        class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                                                    >
                                                        {t!("common.cancel")}
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>

                    {/* Add Account Dialog */}
                    {move || show_add_account.get().then(|| view! {
                        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_add_account.set(false)>
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                                <h3 class="text-lg font-bold font-mono text-gray-900 dark:text-white mb-4">{t!("mail.add_account")}</h3>
                                <div class="space-y-4">
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("mail.email")}</label>
                                        <input
                                            type="email"
                                            prop:value=move || new_account_email.get()
                                            on:input=move |ev| set_new_account_email.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("mail.provider")}</label>
                                        <select
                                            prop:value=move || new_account_provider.get()
                                            on:change=move |ev| set_new_account_provider.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500"
                                        >
                                            <option value="imap">{t!("mail.provider_imap")}</option>
                                            <option value="exchange">{t!("mail.provider_exchange")}</option>
                                            <option value="google">{t!("mail.provider_google")}</option>
                                        </select>
                                    </div>
                                </div>
                                <div class="flex items-center justify-end gap-3 mt-6">
                                    <button
                                        on:click=move |_| set_show_add_account.set(false)
                                        class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                                    >
                                        {t!("common.cancel")}
                                    </button>
                                    <button
                                        on:click=move |_: ev::MouseEvent| {
                                            let email = new_account_email.get();
                                            let provider = new_account_provider.get();
                                            spawn_local(async move {
                                                let body = serde_json::json!({ "email": email, "provider": provider });
                                                let _ = api::fetch_json_with_method("/api/mail/accounts", "POST", Some(&body.to_string())).await;
                                            });
                                            set_show_add_account.set(false);
                                        }
                                        class="px-4 py-2 text-sm font-bold text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
                                    >
                                        {t!("mail.add_account")}
                                    </button>
                                </div>
                            </div>
                        </div>
                    })}
                </main>
            </div>
        </div>
    }
}
