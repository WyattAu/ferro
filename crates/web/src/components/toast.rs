use leptos::*;

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
        self.push.call(msg);
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
    let (toasts, set_toasts) = create_signal::<Vec<ToastMessage>>(vec![]);

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

    view! {
        {children()}
        <div class="fixed top-4 right-4 z-[9999] flex flex-col gap-2 max-w-sm w-full pointer-events-none" role="region" aria-label="Notifications">
            <For
                each=move || toasts.get()
                key=|t| t.id
                let:toast
            >
                {
                    let toast_id = toast.id;
                    view! {
                        <ToastItem toast=toast on_dismiss=Callback::new(move |()| dismiss.call(toast_id)) />
                    }
                }
            </For>
        </div>
    }
}

#[component]
fn ToastItem(toast: ToastMessage, on_dismiss: Callback<()>) -> impl IntoView {
    let (visible, set_visible) = create_signal(true);
    let (dismissed, set_dismissed) = create_signal(false);

    let bg_class = match toast.toast_type {
        ToastType::Success => "bg-green-50 dark:bg-green-900/30 border-green-200 dark:border-green-800 text-green-800 dark:text-green-200",
        ToastType::Error => "bg-red-50 dark:bg-red-900/30 border-red-200 dark:border-red-800 text-red-800 dark:text-red-200",
        ToastType::Info => "bg-blue-50 dark:bg-blue-900/30 border-blue-200 dark:border-blue-800 text-blue-800 dark:text-blue-200",
        ToastType::Warning => "bg-yellow-50 dark:bg-yellow-900/30 border-yellow-200 dark:border-yellow-800 text-yellow-800 dark:text-yellow-200",
    };

    let icon_class = match toast.toast_type {
        ToastType::Success => "text-green-500 dark:text-green-400",
        ToastType::Error => "text-red-500 dark:text-red-400",
        ToastType::Info => "text-blue-500 dark:text-blue-400",
        ToastType::Warning => "text-yellow-500 dark:text-yellow-400",
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
        on_dismiss.call(());
    };

    create_effect(move |_| {
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

    view! {
        <div
            class=move || format!(
                "pointer-events-auto border rounded-lg shadow-lg px-4 py-3 flex items-start gap-3 transition-all duration-300 ease-in-out {} {}",
                bg_class,
                if dismissed.get() { "opacity-0 -translate-x-full scale-95" } else { "opacity-100 translate-x-0 scale-100" }
            )
            role="alert"
            aria-live="polite"
            style=move || if visible.get() || dismissed.get() { "display: flex" } else { "display: none" }
        >
            <span class={icon_class} aria-hidden="true">{icon}</span>
            <p class="flex-1 text-sm font-medium">{message_text}</p>
            <button
                class="p-0.5 rounded opacity-60 hover:opacity-100 transition-opacity focus:outline-none focus:ring-2 focus:ring-current"
                aria-label="Dismiss notification"
                on:click=handle_dismiss
            >
                <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
            </button>
        </div>
    }
}
