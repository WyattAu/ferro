use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::t;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    Success,
    Error,
    Info,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ToastMessage {
    pub id: u32,
    pub message: String,
    pub toast_type: ToastType,
}

#[derive(Debug, Clone, Copy)]
pub struct ToastContext {
    push: Callback<ToastMessage>,
}

static TOAST_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

impl ToastContext {
    fn push(&self, msg: ToastMessage) {
        self.push.run(msg);
    }

    pub fn success(message: impl Into<String>) {
        if let Some(ctx) = use_context::<ToastContext>() {
            ctx.push(ToastMessage {
                id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                message: message.into(),
                toast_type: ToastType::Success,
            });
        }
    }

    pub fn error(message: impl Into<String>) {
        if let Some(ctx) = use_context::<ToastContext>() {
            ctx.push(ToastMessage {
                id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                message: message.into(),
                toast_type: ToastType::Error,
            });
        }
    }

    pub fn info(message: impl Into<String>) {
        if let Some(ctx) = use_context::<ToastContext>() {
            ctx.push(ToastMessage {
                id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                message: message.into(),
                toast_type: ToastType::Info,
            });
        }
    }

    pub fn warning(message: impl Into<String>) {
        if let Some(ctx) = use_context::<ToastContext>() {
            ctx.push(ToastMessage {
                id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                message: message.into(),
                toast_type: ToastType::Warning,
            });
        }
    }
}

#[component]
pub fn ProvideToastContext(children: Children) -> impl IntoView {
    let (toasts, set_toasts) = signal::<Vec<ToastMessage>>(vec![]);

    let push = Callback::new(move |msg: ToastMessage| {
        set_toasts.update(|t| {
            t.insert(0, msg);
            if t.len() > 10 {
                t.truncate(10);
            }
        });
    });

    provide_context(ToastContext { push });

    let dismiss = Callback::new(move |id: u32| {
        set_toasts.update(|t| {
            t.retain(|m| m.id != id);
        });
    });

    #[cfg(target_arch = "wasm32")]
    {
        let dismiss_clone = dismiss;
        let toasts_clone = toasts;
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Escape" {
                        let current = toasts_clone.get();
                        if let Some(last) = current.last() {
                            dismiss_clone.run(last.id);
                        }
                    }
                })
                    as Box<dyn Fn(web_sys::KeyboardEvent)>);
                let _ = document.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
                std::mem::forget(cb);
            }
        }
    }

    view! {
        {children()}
        <div class="fixed top-4 right-4 z-[9999] flex flex-col gap-2 max-w-sm w-full pointer-events-none" role="status" aria-label={t!("toast.aria_notifications")}>
            <For
                each=move || toasts.get()
                key=|t| t.id
                let:toast
            >
                {
                    let toast_id = toast.id;
                    view! {
                        <ToastItem toast=toast on_dismiss=Callback::new(move |()| dismiss.run(toast_id)) />
                    }
                }
            </For>
        </div>
    }
}

#[component]
fn ToastItem(toast: ToastMessage, on_dismiss: Callback<()>) -> impl IntoView {
    let (visible, set_visible) = signal(true);
    let (dismissed, set_dismissed) = signal(false);

    let bg_class = match toast.toast_type {
        ToastType::Success => {
            "bg-[var(--bg-surface)] border border-[var(--border-default)] border-l-4 border-l-[var(--success)] shadow-lg"
        }
        ToastType::Error => {
            "bg-[var(--bg-surface)] border border-[var(--border-default)] border-l-4 border-l-[var(--danger)] shadow-lg"
        }
        ToastType::Info => {
            "bg-[var(--bg-surface)] border border-[var(--border-default)] border-l-4 border-l-[var(--accent)] shadow-lg"
        }
        ToastType::Warning => {
            "bg-[var(--bg-surface)] border border-[var(--border-default)] border-l-4 border-l-[var(--warning)] shadow-lg"
        }
    };

    let icon_class = match toast.toast_type {
        ToastType::Success => "text-[var(--success)]",
        ToastType::Error => "text-[var(--danger)]",
        ToastType::Info => "text-[var(--accent)]",
        ToastType::Warning => "text-[var(--warning)]",
    };

    let icon = match toast.toast_type {
        ToastType::Success => view! {
            <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
        }.into_any(),
        ToastType::Error => view! {
            <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
        }.into_any(),
        ToastType::Info => view! {
            <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
        }.into_any(),
        ToastType::Warning => view! {
            <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4.832c-.77-.833-2.694-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
        }.into_any(),
    };

    let message_text = toast.message.clone();

    let handle_dismiss = move |_| {
        set_dismissed.set(true);
        set_visible.set(false);
        on_dismiss.run(());
    };

    Effect::new(move |_| {
        let dismiss_handle = set_timeout_with_handle(
            move || {
                set_dismissed.set(true);
                set_visible.set(false);
            },
            std::time::Duration::from_secs(5),
        );
        move || {
            if let Ok(handle) = dismiss_handle {
                handle.clear();
            }
        }
    });

    #[cfg(target_arch = "wasm32")]
    {
        let on_dismiss_esc = on_dismiss;
        let dismissed_esc = set_dismissed;
        let visible_esc = set_visible;
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Escape" {
                        dismissed_esc.set(true);
                        visible_esc.set(false);
                        on_dismiss_esc.run(());
                    }
                })
                    as Box<dyn Fn(web_sys::KeyboardEvent)>);
                let _ = document.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
                std::mem::forget(cb);
            }
        }
    }

    view! {
        <div
            class=move || format!(
                "pointer-events-auto rounded-lg px-4 py-3 flex items-start gap-3 transition-all duration-300 ease-in-out relative overflow-hidden {} {}",
                bg_class,
                if dismissed.get() { "opacity-0 translate-x-full scale-95" } else { "opacity-100 translate-x-0 scale-100" }
            )
            aria-live="polite"
            aria-atomic="true"
            style=move || if visible.get() || dismissed.get() { "display: flex" } else { "display: none" }
        >
            <span class={icon_class} aria-hidden="true">{icon}</span>
            <p class="flex-1 text-sm font-medium text-[var(--text-primary)]">{message_text}</p>
            <button
                class="min-w-[44px] min-h-[44px] flex items-center justify-center p-0.5 rounded-md text-[var(--text-tertiary)] hover:text-[var(--text-primary)] hover:bg-[var(--interactive-hover)] transition-colors focus-ring"
                aria-label=t!("toast.aria_dismiss")
                on:click=handle_dismiss
            >
                <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
            </button>
            <div class="absolute bottom-0 left-0 h-0.5 bg-[var(--accent)] opacity-30" style="animation: toast-progress 5s linear forwards;" aria-hidden="true"></div>
        </div>
    }
}
