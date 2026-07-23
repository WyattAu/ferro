use crate::components::primitives::Spinner;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// File upload zone with drag-drop and progress.
#[component]
pub fn UploadZone(#[prop(into)] path: String, #[prop(optional)] on_complete: Option<Callback<()>>) -> impl IntoView {
    let (dragging, set_dragging) = signal(false);
    let (uploading, set_uploading) = signal(false);
    let (progress, set_progress) = signal(0u32);
    let (error, set_error) = signal(None::<String>);

    let handle_drop = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_dragging.set(false);

        if let Some(dt) = ev.data_transfer() {
            let files = dt.files();
            if let Some(files) = files {
                let len = files.length();
                let path = path.clone();
                let set_up = set_uploading;
                let set_prog = set_progress;
                let set_err = set_error;
                let cb = on_complete.clone();

                set_up.set(true);
                set_prog.set(0);

                #[cfg(target_arch = "wasm32")]
                wasm_bindgen_futures::spawn_local(async move {
                    for i in 0..len {
                        if let Some(file) = files.get(i) {
                            let name = file.name();
                            let file_path = if path == "/" {
                                format!("/{}", name)
                            } else {
                                format!("{}/{}", path, name)
                            };

                            let array_buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await;
                            match array_buffer {
                                Ok(ab) => {
                                    let bytes = js_sys::Uint8Array::new(&ab).to_vec();
                                    let client = crate::api::ApiClient::from_env();
                                    // TODO: implement actual upload via PUT /api/v1/files/{path}
                                    log::info!("Upload: {} ({} bytes)", file_path, bytes.len());
                                }
                                Err(e) => {
                                    set_err.set(Some(format!("Read failed: {:?}", e)));
                                }
                            }
                            set_prog.set(((i + 1) as f32 / len as f32 * 100.0) as u32);
                        }
                    }
                    set_up.set(false);
                    if let Some(cb) = cb {
                        cb.run(());
                    }
                });
            }
        }
    };

    let handle_dragover = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_dragging.set(true);
    };

    let handle_dragleave = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_dragging.set(false);
    };

    let open_file_dialog = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let doc = web_sys::window().unwrap().document().unwrap();
            let input = doc.create_element("input").unwrap();
            let input: web_sys::HtmlInputElement = input.dyn_into().unwrap();
            let _ = input.set_attribute("type", "file");
            let _ = input.set_attribute("multiple", "true");
            input.click();
        }
    };

    view! {
        <div
            class=move || format!(
                "border-2 border-dashed rounded-lg p-8 text-center transition-colors {}",
                if dragging.get() { "border-accent bg-accent-subtle" } else { "border-[var(--color-border)]" }
            )
            on:drop=handle_drop
            on:dragover=handle_dragover
            on:dragleave=handle_dragleave
            role="region"
            aria-label="Upload zone"
        >
            {move || if uploading.get() {
                view! {
                    <div class="flex flex-col items-center gap-3">
                        <Spinner />
                        <p class="text-secondary">{format!("Uploading... {}%", progress.get())}</p>
                        <div class="w-full h-2 bg-[var(--color-bg-sunken)] rounded-full overflow-hidden">
                            <div
                                class="h-full bg-accent transition-all"
                                style:width=move || format!("{}%", progress.get())
                            ></div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="flex flex-col items-center gap-3">
                        <div class="text-4xl">"📤"</div>
                        <p class="font-medium">"Drop files here to upload"</p>
                        <p class="text-sm text-secondary">"or"</p>
                        <button class="btn btn-primary btn-sm" on:click=open_file_dialog>
                            "Choose files"
                        </button>
                    </div>
                }.into_any()
            }}

            {move || error.get().map(|e| view! {
                <div class="mt-3 text-sm text-danger">{e}</div>
            })}
        </div>
    }
}
