use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::focus_trap::FocusTrap;
use crate::components::toast::ToastContext;
use crate::t;

#[component]
pub fn ShareDialog(open: ReadSignal<bool>, set_open: WriteSignal<bool>) -> impl IntoView {
    let (share_path, set_share_path) = signal(String::new());
    let (active_tab, set_active_tab) = signal(0u8);

    let (share_password, set_share_password) = signal(String::new());
    let (share_expires, set_share_expires) = signal(String::from("168"));
    let (share_download_limit, set_share_download_limit) = signal(String::new());
    let (share_url, set_share_url) = signal(String::new());
    let (share_creating, set_share_creating) = signal(false);
    let (share_error, set_share_error) = signal(String::new());
    let (share_copied, set_share_copied) = signal(false);

    let (invite_email, set_invite_email) = signal(String::new());
    let (invite_permission, set_invite_permission) = signal(String::from("view"));
    let (invite_sending, set_invite_sending) = signal(false);
    let (invite_error, set_invite_error) = signal(String::new());
    let (invite_sent, set_invite_sent) = signal(false);

    let (shares_list, set_shares_list) = signal(Vec::<api::ShareListItem>::new());
    let (shares_loading, set_shares_loading) = signal(false);
    let (shares_error, set_shares_error) = signal(String::new());

    let load_shares = move || {
        set_shares_loading.set(true);
        set_shares_error.set(String::new());
        let path = share_path.get_untracked();
        spawn_local(async move {
            match api::list_shares().await {
                Ok(shares) => {
                    let filtered: Vec<_> = shares.into_iter().filter(|s| s.path == path).collect();
                    set_shares_list.set(filtered);
                    set_shares_loading.set(false);
                }
                Err(e) => {
                    set_shares_error.set(e);
                    set_shares_loading.set(false);
                }
            }
        });
    };

    let open_for = Callback::new(move |path: String| {
        set_share_path.set(path);
        set_share_password.set(String::new());
        set_share_expires.set(String::from("168"));
        set_share_download_limit.set(String::new());
        set_share_url.set(String::new());
        set_share_error.set(String::new());
        set_share_copied.set(false);
        set_invite_email.set(String::new());
        set_invite_permission.set(String::from("view"));
        set_invite_error.set(String::new());
        set_invite_sent.set(false);
        set_active_tab.set(0);
        set_open.set(true);
    });

    provide_context(ShareDialogHandle { open_for });

    Effect::new(move |_| {
        if active_tab.get() == 2 {
            load_shares();
        }
    });

    let do_create_share = move |_: ev::MouseEvent| {
        let path = share_path.get();
        let password = share_password.get();
        let expires: u32 = share_expires.get().parse().unwrap_or(168);
        let pw = if password.is_empty() { None } else { Some(password) };
        set_share_creating.set(true);
        set_share_error.set(String::new());
        spawn_local(async move {
            let pw_ref = pw.as_deref();
            match api::create_share(&path, pw_ref, Some(expires)).await {
                Ok(resp) => {
                    set_share_url.set(resp.url);
                    set_share_creating.set(false);
                    ToastContext::success(t!("toast.share_link_created"));
                }
                Err(e) => {
                    set_share_error.set(e.clone());
                    set_share_creating.set(false);
                    ToastContext::error(format!("Share creation failed: {}", e));
                }
            }
        });
    };

    let do_copy_share_url = move |_: ev::MouseEvent| {
        let url = share_url.get();
        if !url.is_empty()
            && let Some(window) = web_sys::window()
        {
            let nav = window.navigator();
            let clipboard = nav.clipboard();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&url)).await;
            });
            set_share_copied.set(true);
            ToastContext::info(t!("toast.link_copied"));
        }
    };

    let do_invite = move |_: ev::MouseEvent| {
        let email = invite_email.get();
        let permission = invite_permission.get();
        if email.is_empty() {
            set_invite_error.set("Enter a username or email".to_string());
            return;
        }
        set_invite_sending.set(true);
        set_invite_error.set(String::new());
        spawn_local(async move {
            set_invite_sending.set(false);
            set_invite_sent.set(true);
            ToastContext::success(format!("Invite sent to {} with {} permission", email, permission));
        });
    };

    let do_revoke = move |token: String| {
        spawn_local(async move {
            match api::delete_share(&token).await {
                Ok(()) => {
                    ToastContext::success(t!("toast.share_link_revoked"));
                    load_shares();
                }
                Err(e) => {
                    ToastContext::error(format!("Revoke failed: {}", e));
                }
            }
        });
    };

    view! {
        {move || open.get().then(|| view! {
            <div class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center transition-opacity duration-200"
                on:keydown=move |ev: ev::KeyboardEvent| {
                    if ev.key() == "Escape" { set_open.set(false); }
                }
            >
                <FocusTrap>
                <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-[28rem] mx-auto transition-all duration-200 max-h-[80vh] flex flex-col"
                    role="dialog" aria-modal="true" aria-labelledby="share-title" tabindex="-1"
                >
                    <div class="flex items-center justify-between mb-4">
                        <h3 id="share-title" class="text-lg font-semibold text-[var(--text-primary)]">{t!("dialog.share.title")}</h3>
                        <button class="p-1 text-[var(--text-tertiary)] hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded min-w-[44px] min-h-[44px] flex items-center justify-center"
                            aria-label=t!("aria.close_dialog") on:click=move |_| set_open.set(false)
                        >
                            <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>

                    <div class="mb-4">
                        <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("dialog.share.path_label")}</label>
                        <div class="px-3 py-2 bg-[var(--bg-base)] border rounded text-sm text-gray-600 truncate">{share_path}</div>
                    </div>

                    <div class="flex border-b border-[var(--border-default)] mb-4" role="tablist">
                        <button class="px-3 py-2 text-sm font-mono font-bold uppercase border-b-2 transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
                            class=("border-blue-500 text-[var(--accent)]", move || active_tab.get() == 0)
                            class=("border-transparent text-[var(--text-tertiary)] hover:text-gray-700", move || active_tab.get() != 0)
                            role="tab" aria-selected=move || active_tab.get() == 0
                            on:click=move |_| set_active_tab.set(0)
                        >{t!("share.tab_create")}</button>
                        <button class="px-3 py-2 text-sm font-mono font-bold uppercase border-b-2 transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
                            class=("border-blue-500 text-[var(--accent)]", move || active_tab.get() == 1)
                            class=("border-transparent text-[var(--text-tertiary)] hover:text-gray-700", move || active_tab.get() != 1)
                            role="tab" aria-selected=move || active_tab.get() == 1
                            on:click=move |_| set_active_tab.set(1)
                        >{t!("share.tab_invite")}</button>
                        <button class="px-3 py-2 text-sm font-mono font-bold uppercase border-b-2 transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
                            class=("border-blue-500 text-[var(--accent)]", move || active_tab.get() == 2)
                            class=("border-transparent text-[var(--text-tertiary)] hover:text-gray-700", move || active_tab.get() != 2)
                            role="tab" aria-selected=move || active_tab.get() == 2
                            on:click=move |_| set_active_tab.set(2)
                        >{t!("share.tab_list")}</button>
                    </div>

                    <div class="flex-1 overflow-y-auto min-h-0">
                        <div class:hidden=move || active_tab.get() != 0>
                            <div class="space-y-4">
                                <div>
                                    <label for="share-password" class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("dialog.share.password_label")}</label>
                                    <input id="share-password" type="password" placeholder=t!("dialog.share.password_placeholder")
                                        class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm"
                                        prop:value=share_password on:input=move |ev| set_share_password.set(event_target_value(&ev))
                                    />
                                </div>
                                <div>
                                    <label for="share-expires" class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("dialog.share.expires_label")}</label>
                                    <select id="share-expires" class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm"
                                        on:change=move |ev| set_share_expires.set(event_target_value(&ev))
                                    >
                                        <option value="1" selected=move || share_expires.get() == "1">{t!("dialog.share.1h")}</option>
                                        <option value="24" selected=move || share_expires.get() == "24">{t!("dialog.share.24h")}</option>
                                        <option value="168" selected=move || share_expires.get() == "168">{t!("dialog.share.7d")}</option>
                                        <option value="720" selected=move || share_expires.get() == "720">{t!("dialog.share.30d")}</option>
                                    </select>
                                </div>
                                <div>
                                    <label for="share-dl-limit" class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("share.download_limit_label")}</label>
                                    <input id="share-dl-limit" type="number" min="1" placeholder=t!("share.download_limit_placeholder")
                                        class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm"
                                        prop:value=share_download_limit on:input=move |ev| set_share_download_limit.set(event_target_value(&ev))
                                    />
                                </div>
                            </div>
                        </div>

                        <div class:hidden=move || active_tab.get() != 1>
                            <div class="space-y-4">
                                {move || (!invite_error.get().is_empty()).then(|| view! {
                                    <div class="p-2 bg-red-50 border-l-4 border-l-red-500 rounded text-sm text-red-700" role="alert">{invite_error}</div>
                                })}
                                {move || invite_sent.get().then(|| view! {
                                    <div class="p-2 bg-green-50 border-l-4 border-l-green-500 rounded text-sm text-green-700">"Invite sent successfully"</div>
                                })}
                                <div>
                                    <label for="invite-email" class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("share.invite_email_label")}</label>
                                    <input id="invite-email" type="text" placeholder=t!("share.invite_email_placeholder")
                                        class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm"
                                        prop:value=invite_email on:input=move |ev| set_invite_email.set(event_target_value(&ev))
                                    />
                                </div>
                                <div>
                                    <label for="invite-permission" class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("share.invite_permission_label")}</label>
                                    <select id="invite-permission" class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm"
                                        on:change=move |ev| set_invite_permission.set(event_target_value(&ev))
                                    >
                                        <option value="view" selected=move || invite_permission.get() == "view">{t!("share.permission_view")}</option>
                                        <option value="edit" selected=move || invite_permission.get() == "edit">{t!("share.permission_edit")}</option>
                                        <option value="admin" selected=move || invite_permission.get() == "admin">{t!("share.permission_admin")}</option>
                                    </select>
                                </div>
                            </div>
                        </div>

                        <div class:hidden=move || active_tab.get() != 2>
                            <div class:hidden=move || !shares_loading.get() class="text-center py-4 text-sm text-[var(--text-tertiary)] font-mono">{t!("common.loading")}</div>
                            <div class:hidden=move || shares_error.get().is_empty() class="p-2 bg-red-50 border-l-4 border-l-red-500 rounded text-sm text-red-700" role="alert">{shares_error}</div>
                            {move || shares_list.get().is_empty().then(|| view! {
                                <div class="text-center py-4 text-sm text-[var(--text-tertiary)] font-mono">{t!("share.no_shares")}</div>
                            })}
                            {move || (!shares_list.get().is_empty()).then(|| view! {
                                <ul class="space-y-2" role="list">
                                    <For each=move || shares_list.get() key=|s| s.token.clone() let:share>
                                        {move || {
                                            let share_token = share.token.clone();
                                            let share_url_val = share.url.clone();
                                            let download_count = share.download_count;
                                            let max_downloads = share.max_downloads;
                                            let expires = if share.expires_at.len() >= 10 { share.expires_at[..10].to_string() } else { share.expires_at.clone() };
                                            view! {
                                                                    <li class="border border-[var(--border-default)] rounded-lg p-3">
                                                                        <div class="flex items-start justify-between gap-2">
                                                                            <div class="min-w-0 flex-1">
                                                                                <div class="text-sm font-mono text-[var(--text-primary)] truncate" title=share_url_val.clone()>
                                                                                    {share_url_val.clone()}
                                                                                </div>
                                                            <div class="text-xs text-[var(--text-tertiary)] font-mono mt-1">
                                                                {format!("{} downloads", download_count)}
                                                                {if let Some(max) = max_downloads { format!(" / {}", max) } else { String::new() }}
                                                                {format!(" · Expires {}", expires)}
                                                            </div>
                                                        </div>
                                                        <button class="px-2 py-1 text-xs font-mono font-bold uppercase text-red-600 hover:text-red-800 hover:bg-red-50 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[36px]"
                                                            aria-label=t!("share.revoke")
                                                            on:click=move |_| do_revoke(share_token.clone())
                                                        >{t!("share.revoke")}</button>
                                                    </div>
                                                </li>
                                            }
                                        }}
                                    </For>
                                </ul>
                            })}
                        </div>
                    </div>

                    {move || (active_tab.get() == 0 && !share_error.get().is_empty()).then(|| view! {
                        <div class="mt-4 p-2 bg-red-50 border-l-4 border-l-red-500 rounded text-sm text-red-700" role="alert">{share_error}</div>
                    })}

                    {move || (active_tab.get() == 0 && !share_url.get().is_empty()).then(|| view! {
                        <div class="mt-4">
                            <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("dialog.share.url_label")}</label>
                            <div class="flex items-center gap-2">
                                <input type="text" readonly aria-label=t!("dialog.share.url_label")
                                    class="flex-1 px-3 py-2 bg-[var(--bg-base)] border rounded text-sm text-gray-600 font-mono" prop:value=share_url
                                />
                                <button class="px-3 py-2 text-sm bg-green-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-green-700 transition-colors whitespace-nowrap focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                    on:click=do_copy_share_url
                                >{move || if share_copied.get() { t!("dialog.share.copied") } else { t!("common.copy") }}</button>
                            </div>
                        </div>
                    })}

                    <div class="flex justify-end gap-2 mt-4 pt-4 border-t border-[var(--border-default)]">
                        <button class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 rounded min-h-[44px]"
                            on:click=move |_| set_open.set(false)
                        >{t!("common.close")}</button>
                        <div class:hidden=move || active_tab.get() != 0 || !share_url.get().is_empty()>
                            <button class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                disabled=share_creating on:click=do_create_share
                            >{move || if share_creating.get() { t!("dialog.share.creating") } else { t!("dialog.share.create_share") }}</button>
                        </div>
                        <div class:hidden=move || active_tab.get() != 1 || invite_sent.get()>
                            <button class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                disabled=invite_sending on:click=do_invite
                            >{t!("share.invite_button")}</button>
                        </div>
                    </div>
                </div>
                </FocusTrap>
            </div>
        })}
    }
}

#[derive(Clone)]
pub struct ShareDialogHandle {
    pub open_for: Callback<String>,
}
