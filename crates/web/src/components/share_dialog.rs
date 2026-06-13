use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::focus_trap::FocusTrap;
use crate::components::toast::ToastContext;
use crate::t;

/// Share dialog: password-protected link creation with expiration options.
/// Fully self-contained -- owns all share_* state internally.
#[component]
pub fn ShareDialog(
    /// Whether the dialog is visible.
    open: ReadSignal<bool>,
    /// Setter for dialog visibility.
    set_open: WriteSignal<bool>,
) -> impl IntoView {
    let (share_path, set_share_path) = signal(String::new());
    let (share_password, set_share_password) = signal(String::new());
    let (share_expires, set_share_expires) = signal(String::from("168"));
    let (share_url, set_share_url) = signal(String::new());
    let (share_creating, set_share_creating) = signal(false);
    let (share_error, set_share_error) = signal(String::new());
    let (share_copied, set_share_copied) = signal(false);

    // Public API: open the dialog for a given path.
    // Provided via Leptos context so parent components can call it.
    let open_for = Callback::new(move |path: String| {
        set_share_path.set(path);
        set_share_password.set(String::new());
        set_share_expires.set(String::from("168"));
        set_share_url.set(String::new());
        set_share_error.set(String::new());
        set_share_copied.set(false);
        set_open.set(true);
    });

    // Expose via context so file_browser can call share_dialog.open_for(path)
    provide_context(ShareDialogHandle { open_for });

    let do_create_share = move |_: ev::MouseEvent| {
        let path = share_path.get();
        let password = share_password.get();
        let expires_str = share_expires.get();
        let pw = if password.is_empty() {
            None
        } else {
            Some(password)
        };
        let expires: u32 = expires_str.parse().unwrap_or(168);
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
                    let err_msg = e.clone();
                    set_share_error.set(e);
                    set_share_creating.set(false);
                    ToastContext::error(format!("Share creation failed: {}", err_msg));
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

    view! {
        {move || open.get().then(|| view! {
            <div class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center transition-opacity duration-200"
                on:keydown=move |ev: ev::KeyboardEvent| {
                    if ev.key() == "Escape" {
                        set_open.set(false);
                    }
                }
            >
                <FocusTrap>
                <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="share-title"
                    tabindex="-1"
                >
                    <div class="flex items-center justify-between mb-4">
                        <h3 id="share-title" class="text-lg font-semibold text-gray-900">{t!("dialog.share.title")}</h3>
                        <button
                            class="p-1 text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 rounded min-w-[44px] min-h-[44px] flex items-center justify-center"
                            aria-label=t!("aria.close_dialog")
                            on:click=move |_| set_open.set(false)
                        >
                            <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>

                    <div class="mb-4">
                        <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("dialog.share.path_label")}</label>
                        <div class="px-3 py-2 bg-gray-50 dark:bg-gray-900 border rounded text-sm text-gray-600 truncate">
                            {share_path}
                        </div>
                    </div>

                    <div class="mb-4">
                        <label for="share-password" class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("dialog.share.password_label")}</label>
                        <input
                            id="share-password"
                            type="password"
                            placeholder=t!("dialog.share.password_placeholder")
                            class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                            prop:value=share_password
                            on:input=move |ev| set_share_password.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="mb-4">
                        <label for="share-expires" class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("dialog.share.expires_label")}</label>
                        <select
                            id="share-expires"
                            class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                            on:change=move |ev| set_share_expires.set(event_target_value(&ev))
                        >
                            <option value="1" selected=move || share_expires.get() == "1">{t!("dialog.share.1h")}</option>
                            <option value="24" selected=move || share_expires.get() == "24">{t!("dialog.share.24h")}</option>
                            <option value="168" selected=move || share_expires.get() == "168">{t!("dialog.share.7d")}</option>
                            <option value="720" selected=move || share_expires.get() == "720">{t!("dialog.share.30d")}</option>
                        </select>
                    </div>

                    {move || (!share_error.get().is_empty()).then(|| view! {
                        <div class="mb-4 p-2 bg-red-50 border-l-4 border-l-red-500 rounded text-sm text-red-700" role="alert">
                            {share_error}
                        </div>
                    })}

                    {move || (!share_url.get().is_empty()).then(|| view! {
                        <div class="mb-4">
                            <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">{t!("dialog.share.url_label")}</label>
                            <div class="flex items-center gap-2">
                                <input
                                    type="text"
                                    readonly
                                    aria-label=t!("dialog.share.url_label")
                                    class="flex-1 px-3 py-2 bg-gray-50 dark:bg-gray-900 border rounded text-sm text-gray-600 font-mono"
                                    prop:value=share_url
                                />
                                <button
                                    class="px-3 py-2 text-sm bg-green-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-green-700 transition-colors whitespace-nowrap focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                    on:click=do_copy_share_url
                                >
                                    {move || if share_copied.get() { t!("dialog.share.copied") } else { t!("common.copy") }}
                                </button>
                            </div>
                        </div>
                    })}

                    <div class="flex justify-end gap-2">
                        <button
                            class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded min-h-[44px]"
                            on:click=move |_| set_open.set(false)
                        >
                            {t!("common.close")}
                        </button>
                        {move || share_url.get().is_empty().then(|| view! {
                            <button
                                class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                disabled=share_creating
                                on:click=do_create_share
                            >
                                {move || if share_creating.get() { t!("dialog.share.creating") } else { t!("dialog.share.create_share") }}
                            </button>
                        })}
                    </div>
                </div>
                </FocusTrap>
            </div>
        })}
    }
}

/// Handle to programmatically open the share dialog from parent components.
/// Provided via Leptos context so any child can call `handle.open_for(path)`.
#[derive(Clone)]
pub struct ShareDialogHandle {
    pub open_for: Callback<String>,
}
