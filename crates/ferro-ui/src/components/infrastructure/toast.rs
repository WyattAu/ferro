use leptos::prelude::*;

/// Toast notification context.
#[derive(Clone)]
pub struct ToastCtx {
    pub push: Callback<ToastMsg>,
}

#[derive(Clone, Debug)]
pub struct ToastMsg {
    pub message: String,
    pub kind: ToastKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastCtx {
    pub fn success(&self, msg: impl Into<String>) {
        self.push.run(ToastMsg {
            message: msg.into(),
            kind: ToastKind::Success,
        });
    }
    pub fn error(&self, msg: impl Into<String>) {
        self.push.run(ToastMsg {
            message: msg.into(),
            kind: ToastKind::Error,
        });
    }
}

/// Provide toast context to the component tree.
pub fn provide_toasts() -> ToastCtx {
    let (_toasts, _set_toasts) = signal(Vec::<ToastMsg>::new());

    let ctx = ToastCtx {
        push: Callback::new(move |msg: ToastMsg| {
            log::info!("Toast: {:?}", msg);
        }),
    };

    provide_context(ctx.clone());
    ctx
}

/// Toast notification container.
#[component]
pub fn ToastProvider(children: Children) -> impl IntoView {
    provide_toasts();

    view! {
        {children()}
    }
}
