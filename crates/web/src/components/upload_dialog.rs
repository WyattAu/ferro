use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::focus_trap::FocusTrap;
use crate::components::toast::ToastContext;
use crate::t;

/// Dialog for uploading files via file picker.
#[component]
pub fn UploadDialog(
    /// Whether the dialog is visible.
    open: ReadSignal<bool>,
    /// Setter for dialog visibility.
    set_open: WriteSignal<bool>,
    /// Current directory path (where files will be uploaded).
    current_path: ReadSignal<String>,
    /// Callback invoked after successful upload(s).
    on_uploaded: Callback<()>,
) -> impl IntoView {
    let do_upload_files = move |file_list: web_sys::FileList| {
        let path = current_path.get();
        let count = file_list.length();
        for i in 0..count {
            let Some(file) = file_list.get(i) else {
                continue;
            };
            let file_name = file.name();
            let file_path = if path == "/" {
                format!("/{}", file_name)
            } else {
                format!("{}/{}", path, file_name)
            };
            spawn_local(async move {
                if let Ok(ab) = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                    let uint8 = js_sys::Uint8Array::new(&ab);
                    let mut bytes = vec![0u8; uint8.length() as usize];
                    uint8.copy_to(&mut bytes);
                    match api::upload_file(&file_path, &bytes).await {
                        Ok(()) => {
                            ToastContext::success(format!("File uploaded: {}", file_name));
                            api::show_notification("Upload Complete", &format!("{} uploaded successfully", file_name));
                            on_uploaded.run(());
                        }
                        Err(e) => {
                            ToastContext::error(format!("Upload failed: {}", e));
                        }
                    }
                }
            });
        }
    };

    let handle_file_input = move |ev: ev::Event| {
        set_open.set(false);
        let target = ev.target();
        let input: Option<web_sys::HtmlInputElement> = target.and_then(|t| {
            use wasm_bindgen::JsCast;
            t.dyn_into::<web_sys::HtmlInputElement>().ok()
        });
        if let Some(input) = input
            && let Some(files) = web_sys::HtmlInputElement::files(&input)
        {
            do_upload_files(files);
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
                    aria-labelledby="upload-title"
                    tabindex="-1"
                >
                    <h3 id="upload-title" class="text-section font-mono text-[var(--text-primary)] mb-4">{t!("dialog.upload.title")}</h3>
                    <label class="block w-full border-2 border-dashed border-[var(--border-default)] rounded p-8 text-center cursor-pointer hover:border-blue-400 transition-colors">
                        <svg class="w-12 h-12 text-[var(--text-tertiary)] mx-auto mb-3" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                        </svg>
                        <p id="upload-file-hint" class="text-sm text-[var(--text-secondary)]">{t!("dialog.upload.file_hint")}</p>
                        <input
                            type="file"
                            class="hidden"
                            multiple
                            aria-label=t!("dialog.upload.file_hint")
                            aria-describedby="upload-file-hint"
                            on:change=handle_file_input
                        />
                    </label>
                    <div class="flex justify-end mt-4">
                        <button
                            class="px-4 py-2 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 rounded min-h-[44px]"
                            on:click=move |_| set_open.set(false)
                        >
                            {t!("common.close")}
                        </button>
                    </div>
                </div>
                </FocusTrap>
            </div>
        })}
    }
}
